//! Finalized raw-file verification (task §48, §58).
//!
//! A raw file is finalized when its CONTENT has been re-read from disk and
//! checked against the frozen contract that produced it:
//!
//! ```text
//! identity    schema / study / campaign / sub-campaign / run / surface
//! cardinality exact rows and exact warmup-vs-measured split
//! coverage    exact (case x horse x sample_kind x iteration) set against
//!             the frozen schedule: no missing row, no extra row
//! integrity   no duplicate observation id, no mixed identity
//! FINAL_RAW_FILE_PASS -> receipt -> lane complete marker
//! ```

use std::collections::BTreeSet;

use crate::envelope::Campaign2ObservationV1;

/// Which lane a raw file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane2 {
    Timing,
    Attribution,
    Lifecycle,
    Memory,
}

impl Lane2 {
    pub fn as_str(self) -> &'static str {
        match self {
            Lane2::Timing => "timing",
            Lane2::Attribution => "attribution",
            Lane2::Lifecycle => "lifecycle",
            Lane2::Memory => "memory",
        }
    }
}

/// What a finalized raw file MUST contain.
#[derive(Debug, Clone)]
pub struct RawExpectation2 {
    pub schema: String,
    pub study_id: String,
    pub campaign_spec_id: String,
    pub sub_campaign_spec_id: String,
    pub run_id: String,
    pub surface: String,
    pub lane: Lane2,
    /// Expected rows keyed by `(case_id, horse, sample_kind, iteration)`.
    pub expected_observations: BTreeSet<(String, String, String, u32)>,
    pub expected_warmup_rows: u64,
    pub expected_measured_rows: u64,
}

/// The identity key of one observation.
pub fn key_of(row: &Campaign2ObservationV1) -> (String, String, String, u32) {
    (
        row.result_row_v2.case_id.clone(),
        row.horse_id.clone(),
        row.sample_kind.clone(),
        row.iteration_ordinal,
    )
}

/// Verify one raw file against its expectation. Returns every blocker
/// found (bounded detail is the caller's job).
pub fn verify(rows: &[Campaign2ObservationV1], expected: &RawExpectation2) -> Result<(), Vec<String>> {
    let mut blockers = Vec::new();
    let mut seen: BTreeSet<(String, String, String, u32)> = BTreeSet::new();
    let mut duplicates: Vec<(String, String, String, u32)> = Vec::new();
    let mut warmup = 0u64;
    let mut measured = 0u64;

    if rows.len() != expected.expected_observations.len() {
        blockers.push(format!(
            "row count {} != expected {}",
            rows.len(),
            expected.expected_observations.len()
        ));
    }

    let mut identity_problems = 0usize;
    let mut order_problems = 0usize;
    let mut previous_ordinal: Option<u32> = None;
    for (index, row) in rows.iter().enumerate() {
        if row.schema != expected.schema {
            identity_problems += 1;
        }
        if row.study_id != expected.study_id
            || row.campaign_spec_id != expected.campaign_spec_id
            || row.sub_campaign_spec_id != expected.sub_campaign_spec_id
            || row.run_id != expected.run_id
            || row.surface != expected.surface
        {
            identity_problems += 1;
        }
        if row.lane != expected.lane.as_str() {
            identity_problems += 1;
        }
        if let Some(previous) = previous_ordinal {
            if row.case_order_ordinal < previous {
                order_problems += 1;
            }
        }
        previous_ordinal = Some(row.case_order_ordinal);
        let key = key_of(row);
        if !seen.insert(key.clone()) {
            duplicates.push(key);
        }
        match row.sample_kind.as_str() {
            "warmup" => warmup += 1,
            "measured" => measured += 1,
            _ => {}
        }
        if index == 0 && row.schema != crate::envelope::SCHEMA {
            blockers.push(format!("first row schema {:?} is not the campaign-2 schema", row.schema));
        }
    }
    if identity_problems > 0 {
        blockers.push(format!(
            "{identity_problems} rows carry a foreign study/campaign/sub-campaign/run/surface/lane identity"
        ));
    }
    if order_problems > 0 {
        blockers.push(format!(
            "{order_problems} rows break the frozen case-order ordinal monotonicity"
        ));
    }
    if !duplicates.is_empty() {
        blockers.push(format!(
            "{} duplicate (case, horse, sample_kind, iteration) observations, first: {:?}",
            duplicates.len(),
            duplicates.first()
        ));
    }
    let missing: Vec<_> = expected
        .expected_observations
        .difference(&seen)
        .take(5)
        .cloned()
        .collect();
    if !missing.is_empty() {
        blockers.push(format!(
            "{} expected observations missing, first: {:?}",
            expected.expected_observations.difference(&seen).count(),
            missing
        ));
    }
    let extra: Vec<_> = seen
        .difference(&expected.expected_observations)
        .take(5)
        .cloned()
        .collect();
    if !extra.is_empty() {
        blockers.push(format!(
            "{} unexpected observations present, first: {:?}",
            seen.difference(&expected.expected_observations).count(),
            extra
        ));
    }
    if warmup != expected.expected_warmup_rows {
        blockers.push(format!(
            "warmup rows {warmup} != expected {}",
            expected.expected_warmup_rows
        ));
    }
    if measured != expected.expected_measured_rows {
        blockers.push(format!(
            "measured rows {measured} != expected {}",
            expected.expected_measured_rows
        ));
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// Every correctness fact in a raw file must be `pass` with no failure.
pub fn verify_all_correct(rows: &[Campaign2ObservationV1]) -> Result<(), Vec<String>> {
    let mut bad = Vec::new();
    for row in rows {
        let execution = row.result_row_v2.execution_status;
        let correctness = row.result_row_v2.correctness_status;
        if execution != markit_mdbench_common::ExecutionStatus::Pass
            || correctness != markit_mdbench_common::CorrectnessStatus::Pass
        {
            bad.push(format!(
                "case {} horse {} kind {} iter {}: execution={execution:?} correctness={correctness:?}",
                row.result_row_v2.case_id,
                row.horse_id,
                row.sample_kind,
                row.iteration_ordinal
            ));
            if bad.len() >= 10 {
                break;
            }
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(bad)
    }
}
