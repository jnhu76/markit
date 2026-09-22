//! Finalized raw-file verification (task §10, §26, §39, §47).
//!
//! A raw file is not "finalized" because the writer stopped writing to
//! it: it is finalized when its CONTENT has been re-read from disk and
//! checked against the frozen contract that produced it. This module is
//! that check, and it runs BEFORE the file's run receipt is written and
//! before any completion verdict is printed.
//!
//! ```text
//! raw file on disk
//!   -> identity:  schema, result schema v2, CampaignSpecId, RunId,
//!                 SessionId, surface, session ordinal, ObservationId
//!                 (re-derived, not trusted)
//!   -> cardinality: exact rows, exact warmup/measured/attribution split
//!   -> coverage:  exact (case × horse × sample_kind × iteration) set
//!                 against the frozen schedule, no missing, no extra
//!   -> FINAL_RAW_FILE_PASS  -> run receipt -> SESSION_COMPLETE
//!      otherwise            -> PRIMARY_CAMPAIGN_INVALID (no receipt)
//! ```
//!
//! A raw file that fails any check is never summarized and never gets a
//! "success" receipt; the file itself is retained as failure evidence
//! (task §36: nothing is deleted, replaced, retried, or imputed).

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use serde::Serialize;

use crate::execute::CampaignObservationV1;
use crate::schedule::ScheduleRow;
use crate::{SampleKind, Surface, ENVELOPE_SCHEMA_ID};

/// Number of individual blockers reported before the rest are counted
/// only: a 72,400-row file can produce thousands of messages, and the
/// verdict must stay readable.
const MAX_BLOCKER_DETAIL: usize = 20;

/// Which lane a finalized raw file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawLane {
    /// A timing session: warmup + measured iterations per case × horse.
    Timing { session_ordinal: u32 },
    /// The attribution lane: one dispatch per case × horse.
    Attribution,
}

impl RawLane {
    pub fn as_str(self) -> &'static str {
        match self {
            RawLane::Timing { .. } => "timing",
            RawLane::Attribution => "attribution",
        }
    }

    pub fn session_ordinal(self) -> Option<u32> {
        match self {
            RawLane::Timing { session_ordinal } => Some(session_ordinal),
            RawLane::Attribution => None,
        }
    }
}

/// One scheduled case reduced to the identity facts finalization checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCaseIdentity {
    pub case_id: String,
    pub order_ordinal: u32,
    /// Exact horse execution order recorded in the frozen schedule.
    pub horse_order: Vec<String>,
}

/// What a finalized raw file MUST contain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFileExpectation {
    pub campaign_spec_id: String,
    pub run_id: String,
    pub session_id: String,
    pub surface: Surface,
    pub lane: RawLane,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    pub cases: Vec<ScheduledCaseIdentity>,
}

impl RawFileExpectation {
    /// Exact total rows (timing: cases × horses × (warmup + measured);
    /// attribution: cases × horses).
    pub fn expected_rows(&self) -> usize {
        match self.lane {
            RawLane::Timing { .. } => self.expected_warmup_rows() + self.expected_measured_rows(),
            RawLane::Attribution => self.expected_attribution_rows(),
        }
    }

    pub fn expected_warmup_rows(&self) -> usize {
        if matches!(self.lane, RawLane::Attribution) {
            return 0;
        }
        self.cases
            .iter()
            .map(|case| case.horse_order.len() * self.warmup_iterations as usize)
            .sum()
    }

    pub fn expected_measured_rows(&self) -> usize {
        if matches!(self.lane, RawLane::Attribution) {
            return 0;
        }
        self.cases
            .iter()
            .map(|case| case.horse_order.len() * self.measured_iterations as usize)
            .sum()
    }

    pub fn expected_attribution_rows(&self) -> usize {
        if !matches!(self.lane, RawLane::Attribution) {
            return 0;
        }
        self.cases.iter().map(|case| case.horse_order.len()).sum()
    }
}

/// What a verified raw file actually contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawFileSummary {
    pub verdict: String,
    pub lane: String,
    pub surface: String,
    pub session_ordinal: Option<u32>,
    pub campaign_spec_id: String,
    pub run_id: String,
    pub session_id: String,
    pub sha256: String,
    pub rows: usize,
    pub warmup_rows: usize,
    pub measured_rows: usize,
    pub attribution_rows: usize,
    pub expected_rows: usize,
    pub unique_observation_ids: usize,
    pub case_count: usize,
    pub first_observation_id: String,
    pub last_observation_id: String,
}

/// Build the expectation of one lane from the FROZEN schedule rows the
/// execution consumed. The row set is re-checked here (surface, session,
/// exact case set, horse orders) so a truncated or mixed schedule cannot
/// silently lower the expected cardinality.
#[allow(clippy::too_many_arguments)]
pub fn expectation_from_schedule(
    schedule_rows: &[&ScheduleRow],
    campaign_spec_id: &str,
    run_id: &str,
    session_id: &str,
    surface: Surface,
    lane: RawLane,
    warmup_iterations: u32,
    measured_iterations: u32,
) -> Result<RawFileExpectation, String> {
    let mut cases = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for row in schedule_rows {
        if row.surface != surface.as_str() {
            return Err(format!(
                "schedule row {} belongs to surface {:?}, not {}",
                row.case_id,
                row.surface,
                surface.as_str()
            ));
        }
        let expected_session = match lane {
            RawLane::Timing { session_ordinal } => session_ordinal,
            // The attribution lane always derives its case set from
            // session 0 of the frozen schedule (task §18).
            RawLane::Attribution => 0,
        };
        if row.session_ordinal != expected_session {
            return Err(format!(
                "schedule row {} is session {}, not {expected_session}",
                row.case_id, row.session_ordinal
            ));
        }
        if !seen.insert(row.case_id.clone()) {
            return Err(format!("schedule repeats case {}", row.case_id));
        }
        if row.horse_order.len() != crate::HORSE_IDS.len() {
            return Err(format!(
                "schedule row {} has {} horses, not {}",
                row.case_id,
                row.horse_order.len(),
                crate::HORSE_IDS.len()
            ));
        }
        let mut ordered = row.horse_order.clone();
        ordered.sort();
        let mut frozen = crate::HORSE_IDS.to_vec();
        frozen.sort();
        if ordered != frozen {
            return Err(format!(
                "schedule row {} does not dispatch exactly H0-H4",
                row.case_id
            ));
        }
        cases.push(ScheduledCaseIdentity {
            case_id: row.case_id.clone(),
            order_ordinal: row.order_ordinal,
            horse_order: row.horse_order.clone(),
        });
    }
    if cases.len() != surface.frozen_case_count() {
        return Err(format!(
            "{} schedule carries {} cases, not the frozen {}",
            surface.as_str(),
            cases.len(),
            surface.frozen_case_count()
        ));
    }
    // Frozen per-lane cardinalities (task §10): 22/362 cases.
    let expectation = RawFileExpectation {
        campaign_spec_id: campaign_spec_id.to_string(),
        run_id: run_id.to_string(),
        session_id: session_id.to_string(),
        surface,
        lane,
        warmup_iterations,
        measured_iterations,
        cases,
    };
    let rows = expectation.expected_rows();
    let frozen_rows = match lane {
        RawLane::Timing { .. } => {
            surface.frozen_case_count()
                * crate::HORSE_IDS.len()
                * (warmup_iterations + measured_iterations) as usize
        }
        RawLane::Attribution => surface.frozen_case_count() * crate::HORSE_IDS.len(),
    };
    if rows != frozen_rows {
        return Err(format!(
            "{} {}: expected rows {rows} != frozen {frozen_rows}",
            surface.as_str(),
            lane.as_str()
        ));
    }
    Ok(expectation)
}

/// Horse label of a mechanism id (the frozen roster is the only mapping).
fn horse_for_mechanism(mechanism_id: &str) -> Option<&'static str> {
    crate::HORSE_ROSTER
        .iter()
        .find(|entry| entry.mechanism_id == mechanism_id)
        .map(|entry| entry.id)
}

/// A bounded blocker list: the first [`MAX_BLOCKER_DETAIL`] messages are
/// kept verbatim, the rest are counted.
struct BlockerSink {
    items: Vec<String>,
    suppressed: usize,
}

impl BlockerSink {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            suppressed: 0,
        }
    }

    fn push(&mut self, message: String) {
        if self.items.len() < MAX_BLOCKER_DETAIL {
            self.items.push(message);
        } else {
            self.suppressed += 1;
        }
    }

    fn finish(mut self) -> Vec<String> {
        if self.suppressed > 0 {
            self.items.push(format!("... and {} more", self.suppressed));
        }
        self.items
    }
}

/// Verify a finalized raw file against its expectation. Returns the
/// verified summary, or the blocker list (never both).
pub fn verify_raw_file(
    path: &Path,
    expectation: &RawFileExpectation,
) -> Result<RawFileSummary, Vec<String>> {
    let bytes = std::fs::read(path).map_err(|e| vec![format!("read {}: {e}", path.display())])?;
    let sha256 = crate::sha256_hex(&bytes);
    let text = String::from_utf8(bytes)
        .map_err(|e| vec![format!("{}: not UTF-8: {e}", path.display())])?;

    let mut blockers = BlockerSink::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    let mut seen_keys: BTreeSet<(String, String, String, u32)> = BTreeSet::new();
    let mut expected_keys: BTreeSet<(String, String, String, u32)> = BTreeSet::new();
    let by_case: HashMap<&str, &ScheduledCaseIdentity> = expectation
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    for case in &expectation.cases {
        for horse in &case.horse_order {
            match expectation.lane {
                RawLane::Timing { .. } => {
                    for iteration in 0..expectation.warmup_iterations {
                        expected_keys.insert((
                            case.case_id.clone(),
                            horse.clone(),
                            SampleKind::Warmup.as_str().to_string(),
                            iteration,
                        ));
                    }
                    for iteration in 0..expectation.measured_iterations {
                        expected_keys.insert((
                            case.case_id.clone(),
                            horse.clone(),
                            SampleKind::Measured.as_str().to_string(),
                            iteration,
                        ));
                    }
                }
                RawLane::Attribution => {
                    expected_keys.insert((
                        case.case_id.clone(),
                        horse.clone(),
                        SampleKind::Attribution.as_str().to_string(),
                        0,
                    ));
                }
            }
        }
    }

    let (mut rows, mut warmup_rows, mut measured_rows, mut attribution_rows) = (0, 0, 0, 0);
    let mut first_observation_id = String::new();
    let mut last_observation_id = String::new();

    for (line_index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let observation: CampaignObservationV1 = match serde_json::from_str(line) {
            Ok(observation) => observation,
            Err(error) => {
                blockers.push(format!(
                    "{}: row {}: {error}",
                    path.display(),
                    line_index + 1
                ));
                continue;
            }
        };
        rows += 1;
        if first_observation_id.is_empty() {
            first_observation_id = observation.observation_id.clone();
        }
        last_observation_id = observation.observation_id.clone();

        // 1. Envelope + result schema identity.
        if observation.schema != ENVELOPE_SCHEMA_ID {
            blockers.push(format!(
                "row {}: envelope schema {:?} != {ENVELOPE_SCHEMA_ID}",
                line_index + 1,
                observation.schema
            ));
        }
        if observation.result_row_v2.schema_version
            != markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2
        {
            blockers.push(format!(
                "row {}: result schema v{} != v{}",
                line_index + 1,
                observation.result_row_v2.schema_version,
                markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2
            ));
        }

        // 2. Campaign identity binding.
        if observation.campaign_spec_id != expectation.campaign_spec_id {
            blockers.push(format!(
                "row {}: campaign spec id {} != {}",
                line_index + 1,
                observation.campaign_spec_id,
                expectation.campaign_spec_id
            ));
        }
        if observation.run_id != expectation.run_id {
            blockers.push(format!(
                "row {}: run id {} != {}",
                line_index + 1,
                observation.run_id,
                expectation.run_id
            ));
        }
        if observation.session_id != expectation.session_id {
            blockers.push(format!(
                "row {}: session id {} != {}",
                line_index + 1,
                observation.session_id,
                expectation.session_id
            ));
        }
        if observation.surface != expectation.surface.as_str() {
            blockers.push(format!(
                "row {}: surface {:?} != {}",
                line_index + 1,
                observation.surface,
                expectation.surface.as_str()
            ));
        }
        if observation.session_ordinal != expectation.lane.session_ordinal() {
            blockers.push(format!(
                "row {}: session ordinal {:?} != {:?}",
                line_index + 1,
                observation.session_ordinal,
                expectation.lane.session_ordinal()
            ));
        }

        // 3. Sample kind + iteration belonging to this lane.
        let lane_ok = match expectation.lane {
            RawLane::Timing { .. } => {
                observation.sample_kind == SampleKind::Warmup.as_str()
                    || observation.sample_kind == SampleKind::Measured.as_str()
            }
            RawLane::Attribution => observation.sample_kind == SampleKind::Attribution.as_str(),
        };
        if !lane_ok {
            blockers.push(format!(
                "row {}: sample kind {:?} does not belong to the {} lane",
                line_index + 1,
                observation.sample_kind,
                expectation.lane.as_str()
            ));
        }
        match observation.sample_kind.as_str() {
            "warmup" => warmup_rows += 1,
            "measured" => measured_rows += 1,
            "attribution" => attribution_rows += 1,
            _ => {}
        }

        // 4. Case identity: the row must belong to a scheduled case, in
        //    the scheduled position, with the scheduled horse order.
        let case_id = observation.result_row_v2.case_id.clone();
        match by_case.get(case_id.as_str()) {
            Some(scheduled) => {
                if observation.case_order_ordinal != scheduled.order_ordinal {
                    blockers.push(format!(
                        "row {}: case order ordinal {} != scheduled {}",
                        line_index + 1,
                        observation.case_order_ordinal,
                        scheduled.order_ordinal
                    ));
                }
            }
            None => blockers.push(format!(
                "row {}: case {case_id} is not in the frozen schedule",
                line_index + 1
            )),
        }
        // Every row counts for duplicate detection, including a row that
        // does not belong to this lane at all.
        if !seen_ids.insert(observation.observation_id.clone()) {
            blockers.push(format!(
                "row {}: duplicate observation id {}",
                line_index + 1,
                observation.observation_id
            ));
        }
        let Some(horse) = horse_for_mechanism(&observation.result_row_v2.mechanism_id) else {
            blockers.push(format!(
                "row {}: mechanism {:?} is not in the frozen roster",
                line_index + 1,
                observation.result_row_v2.mechanism_id
            ));
            continue;
        };
        if let Some(scheduled) = by_case.get(case_id.as_str()) {
            let position = observation.horse_order_ordinal as usize;
            if scheduled.horse_order.get(position).map(String::as_str) != Some(horse) {
                blockers.push(format!(
                    "row {}: horse {horse} at position {position} contradicts the scheduled horse order {:?}",
                    line_index + 1, scheduled.horse_order
                ));
            }
        }

        // 5. ObservationId: re-derived from the immutable fields, never
        //    trusted as stored.
        let derived = crate::identity::observation_id(
            &observation.run_id,
            &observation.session_id,
            &observation.surface,
            &case_id,
            horse,
            &observation.sample_kind,
            observation.iteration_ordinal,
        );
        if derived != observation.observation_id {
            blockers.push(format!(
                "row {}: observation id {} does not derive from its own identity fields ({derived})",
                line_index + 1,
                observation.observation_id
            ));
        }
        seen_keys.insert((
            case_id,
            horse.to_string(),
            observation.sample_kind.clone(),
            observation.iteration_ordinal,
        ));
    }

    // 6. Exact coverage: no missing and no extra (case × horse × kind ×
    //    iteration) identities.
    let missing: Vec<&(String, String, String, u32)> =
        expected_keys.difference(&seen_keys).collect();
    for key in missing.iter().take(MAX_BLOCKER_DETAIL) {
        blockers.push(format!(
            "missing observation: case {} horse {} {} iteration {}",
            key.0, key.1, key.2, key.3
        ));
    }
    if missing.len() > MAX_BLOCKER_DETAIL {
        blockers.suppressed += missing.len() - MAX_BLOCKER_DETAIL;
    }
    let extra: Vec<&(String, String, String, u32)> = seen_keys.difference(&expected_keys).collect();
    for key in extra.iter().take(MAX_BLOCKER_DETAIL) {
        blockers.push(format!(
            "unexpected observation: case {} horse {} {} iteration {}",
            key.0, key.1, key.2, key.3
        ));
    }
    if extra.len() > MAX_BLOCKER_DETAIL {
        blockers.suppressed += extra.len() - MAX_BLOCKER_DETAIL;
    }

    // 7. Exact cardinalities.
    let expected_rows = expectation.expected_rows();
    if rows != expected_rows {
        blockers.push(format!("{} rows != expected {expected_rows}", rows));
    }
    if warmup_rows != expectation.expected_warmup_rows() {
        blockers.push(format!(
            "{} warmup rows != expected {}",
            warmup_rows,
            expectation.expected_warmup_rows()
        ));
    }
    if measured_rows != expectation.expected_measured_rows() {
        blockers.push(format!(
            "{} measured rows != expected {}",
            measured_rows,
            expectation.expected_measured_rows()
        ));
    }
    if attribution_rows != expectation.expected_attribution_rows() {
        blockers.push(format!(
            "{} attribution rows != expected {}",
            attribution_rows,
            expectation.expected_attribution_rows()
        ));
    }
    if seen_ids.len() != rows {
        blockers.push(format!(
            "{} distinct observation ids over {rows} rows",
            seen_ids.len()
        ));
    }

    let blockers = blockers.finish();
    if !blockers.is_empty() {
        return Err(blockers);
    }
    Ok(RawFileSummary {
        verdict: "FINAL_RAW_FILE_PASS".to_string(),
        lane: expectation.lane.as_str().to_string(),
        surface: expectation.surface.as_str().to_string(),
        session_ordinal: expectation.lane.session_ordinal(),
        campaign_spec_id: expectation.campaign_spec_id.clone(),
        run_id: expectation.run_id.clone(),
        session_id: expectation.session_id.clone(),
        sha256,
        rows,
        warmup_rows,
        measured_rows,
        attribution_rows,
        expected_rows,
        unique_observation_ids: seen_ids.len(),
        case_count: expectation.cases.len(),
        first_observation_id,
        last_observation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use markit_mdbench_runner::{MeasurementV1, PayloadMetaV1, ResultRowV1, TimingMetricsV1};

    const RUN: &str = "run-id-fixture";
    const SPEC: &str = "spec-id-fixture";

    fn surface_cases(surface: Surface) -> Vec<ScheduledCaseIdentity> {
        let horses: Vec<String> = crate::HORSE_IDS.iter().map(|h| h.to_string()).collect();
        (0..surface.frozen_case_count().min(3))
            .map(|index| ScheduledCaseIdentity {
                case_id: format!("{index:064x}"),
                order_ordinal: index as u32,
                horse_order: horses.clone(),
            })
            .collect()
    }

    fn expectation(surface: Surface, lane: RawLane) -> RawFileExpectation {
        let session_id = match lane {
            RawLane::Timing { session_ordinal } => {
                crate::identity::session_id(SPEC, surface.as_str(), session_ordinal)
            }
            RawLane::Attribution => crate::execute::attribution_session_id(SPEC, surface.as_str()),
        };
        RawFileExpectation {
            campaign_spec_id: SPEC.to_string(),
            run_id: RUN.to_string(),
            session_id,
            surface,
            lane,
            warmup_iterations: 2,
            measured_iterations: 2,
            cases: surface_cases(surface),
        }
    }

    fn row_for(
        expectation: &RawFileExpectation,
        case: &ScheduledCaseIdentity,
        horse: &str,
        kind: &str,
        iteration: u32,
    ) -> CampaignObservationV1 {
        let mechanism_id = crate::execute::horse_id_to_mechanism(horse)
            .unwrap()
            .to_string();
        let position = case
            .horse_order
            .iter()
            .position(|h| h == horse)
            .expect("horse is scheduled");
        CampaignObservationV1 {
            schema: ENVELOPE_SCHEMA_ID.to_string(),
            campaign_spec_id: expectation.campaign_spec_id.clone(),
            run_id: expectation.run_id.clone(),
            session_id: expectation.session_id.clone(),
            surface: expectation.surface.as_str().to_string(),
            sample_kind: kind.to_string(),
            session_ordinal: expectation.lane.session_ordinal(),
            case_order_ordinal: case.order_ordinal,
            horse_order_ordinal: position as u32,
            iteration_ordinal: iteration,
            observation_id: crate::identity::observation_id(
                &expectation.run_id,
                &expectation.session_id,
                expectation.surface.as_str(),
                &case.case_id,
                horse,
                kind,
                iteration,
            ),
            result_row_v2: ResultRowV1 {
                schema_version: markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2,
                protocol_version: markit_mdbench_runner::PROTOCOL_VERSION.to_string(),
                build_identity: markit_mdbench_runner::current_build_identity().into(),
                case_id: case.case_id.clone(),
                seed: 1,
                mechanism_id,
                operation: markit_mdbench_common::OperationKind::FullParse,
                payload: PayloadMetaV1 {
                    payload_id: "fixture".to_string(),
                    shape: markit_mdbench_common::PayloadShape::Mixed,
                    size_bytes: 1,
                },
                edit: markit_mdbench_runner::EditMetaV1 {
                    start_byte: None,
                    end_byte: None,
                    inserted_sha256: None,
                },
                execution_status: markit_mdbench_common::ExecutionStatus::Pass,
                correctness_status: markit_mdbench_common::CorrectnessStatus::Pass,
                result_checksum: None,
                environment_ref: "fixture".to_string(),
                provenance_ref: crate::execute::PROVENANCE_TIMING.to_string(),
                measurement: MeasurementV1::Timing(TimingMetricsV1 {
                    prepare_ns: markit_mdbench_common::Observed::Known(1),
                    native_ns: markit_mdbench_common::Observed::Known(2),
                    total_ns: markit_mdbench_common::Observed::Known(3),
                }),
            },
        }
    }

    /// A complete, valid raw file for the expectation's lane.
    fn complete_rows(expectation: &RawFileExpectation) -> Vec<CampaignObservationV1> {
        let mut rows = Vec::new();
        for case in &expectation.cases {
            for horse in &case.horse_order {
                match expectation.lane {
                    RawLane::Timing { .. } => {
                        for iteration in 0..expectation.warmup_iterations {
                            rows.push(row_for(expectation, case, horse, "warmup", iteration));
                        }
                        for iteration in 0..expectation.measured_iterations {
                            rows.push(row_for(expectation, case, horse, "measured", iteration));
                        }
                    }
                    RawLane::Attribution => {
                        rows.push(row_for(expectation, case, horse, "attribution", 0));
                    }
                }
            }
        }
        rows
    }

    fn write_rows(name: &str, rows: &[CampaignObservationV1]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mdbench-finalize-{}-{}-{name}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut text = String::new();
        for row in rows {
            text.push_str(&serde_json::to_string(row).unwrap());
            text.push('\n');
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn a_complete_timing_session_passes() {
        let expectation = expectation(Surface::CleanState, RawLane::Timing { session_ordinal: 1 });
        let path = write_rows("timing", &complete_rows(&expectation));
        let summary = verify_raw_file(&path, &expectation).expect("complete session verifies");
        assert_eq!(summary.verdict, "FINAL_RAW_FILE_PASS");
        assert_eq!(summary.rows, expectation.expected_rows());
        assert_eq!(summary.warmup_rows, 3 * 5 * 2);
        assert_eq!(summary.measured_rows, 3 * 5 * 2);
        assert_eq!(summary.attribution_rows, 0);
        assert_eq!(summary.unique_observation_ids, summary.rows);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_complete_attribution_lane_passes() {
        let expectation = expectation(Surface::EditWrite, RawLane::Attribution);
        let path = write_rows("attribution", &complete_rows(&expectation));
        let summary = verify_raw_file(&path, &expectation).expect("complete lane verifies");
        assert_eq!(summary.attribution_rows, 3 * 5);
        assert_eq!(summary.session_ordinal, None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_duplicate_row_is_blocked() {
        let expectation = expectation(Surface::CleanState, RawLane::Attribution);
        let mut rows = complete_rows(&expectation);
        rows.push(rows[0].clone());
        let path = write_rows("duplicate", &rows);
        let blockers = verify_raw_file(&path, &expectation).unwrap_err();
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("duplicate observation id")),
            "duplicate row must block; got {blockers:?}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_row_is_blocked() {
        let expectation = expectation(Surface::CleanState, RawLane::Attribution);
        let mut rows = complete_rows(&expectation);
        rows.pop();
        let path = write_rows("missing", &rows);
        let blockers = verify_raw_file(&path, &expectation).unwrap_err();
        assert!(
            blockers.iter().any(|b| b.contains("missing observation")),
            "missing row must block; got {blockers:?}"
        );
        assert!(
            blockers.iter().any(|b| b.contains("rows != expected")),
            "short file must block on cardinality; got {blockers:?}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_extra_unscheduled_row_is_blocked() {
        let expectation = expectation(Surface::CleanState, RawLane::Attribution);
        let mut rows = complete_rows(&expectation);
        let mut alien = rows[0].clone();
        alien.result_row_v2.case_id = "ff".repeat(32);
        alien.observation_id = crate::identity::observation_id(
            &expectation.run_id,
            &expectation.session_id,
            expectation.surface.as_str(),
            &alien.result_row_v2.case_id,
            "H0",
            "attribution",
            0,
        );
        rows.push(alien);
        let path = write_rows("extra", &rows);
        let blockers = verify_raw_file(&path, &expectation).unwrap_err();
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("is not in the frozen schedule")),
            "unscheduled case must block; got {blockers:?}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn identity_and_schema_drift_are_blocked() {
        for (name, mutate) in [
            (
                "spec",
                (|row: &mut CampaignObservationV1| row.campaign_spec_id = "other".to_string())
                    as fn(&mut CampaignObservationV1),
            ),
            ("run", |row: &mut CampaignObservationV1| {
                row.run_id = "other".to_string()
            }),
            ("session", |row: &mut CampaignObservationV1| {
                row.session_id = "other".to_string()
            }),
            ("surface", |row: &mut CampaignObservationV1| {
                row.surface = "edit_write".to_string()
            }),
            ("ordinal", |row: &mut CampaignObservationV1| {
                row.session_ordinal = Some(9)
            }),
            ("envelope", |row: &mut CampaignObservationV1| {
                row.schema = "other-v1".to_string()
            }),
            ("result-schema", |row: &mut CampaignObservationV1| {
                row.result_row_v2.schema_version = 1
            }),
            ("reordered", |row: &mut CampaignObservationV1| {
                row.horse_order_ordinal = (row.horse_order_ordinal + 1) % 5
            }),
        ] {
            let expectation =
                expectation(Surface::CleanState, RawLane::Timing { session_ordinal: 0 });
            let mut rows = complete_rows(&expectation);
            mutate(&mut rows[0]);
            let path = write_rows(name, &rows);
            let blockers = verify_raw_file(&path, &expectation)
                .err()
                .unwrap_or_else(|| panic!("{name} drift must block"));
            assert!(!blockers.is_empty(), "{name} drift must produce a blocker");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn expectation_from_schedule_rejects_a_truncated_schedule() {
        let horses: Vec<String> = crate::HORSE_IDS.iter().map(|h| h.to_string()).collect();
        let rows: Vec<crate::schedule::ScheduleRow> = (0..2)
            .map(|index| crate::schedule::ScheduleRow {
                schema: crate::SCHEDULE_SCHEMA.to_string(),
                campaign_spec_id: SPEC.to_string(),
                surface: Surface::CleanState.as_str().to_string(),
                session_ordinal: 0,
                order_ordinal: index,
                case_id: format!("{index:064x}"),
                payload_id: "p".to_string(),
                source_key: "s".to_string(),
                trace_id: None,
                horse_order: horses.clone(),
            })
            .collect();
        let refs: Vec<&crate::schedule::ScheduleRow> = rows.iter().collect();
        let error = expectation_from_schedule(
            &refs,
            SPEC,
            RUN,
            "session",
            Surface::CleanState,
            RawLane::Attribution,
            10,
            30,
        )
        .expect_err("2 cases are not the frozen 22");
        assert!(error.contains("not the frozen 22"), "got {error}");
    }
}
