//! Non-measuring campaign preflight (task §28, §47).
//!
//! Checks, fail-closed, before any session data is collected:
//!
//! ```text
//! campaign receipt valid           (artifact hashes + spec recompute)
//! machine matches frozen manifest  (exact stable-field equality)
//! binary/build identity matches    (profile, rustc, Cargo.lock digest)
//! runner commit available          (never "unknown")
//! Cargo.lock matches
//! CPU affinity can be applied      (probe + restore)
//! source materialized              (hash-verified workload load)
//! workload receipt valid           (#35 freeze receipt artifacts)
//! schedule valid                   (cardinalities + orders + spec id)
//! schema v2 + envelope schema      (generated == checked-in)
//! no duplicate ObservationIds      (per requested SCOPE, see below)
//! expected cardinalities           (cells / rows, per scope)
//! ```
//!
//! The ObservationId enumeration is scope-explicit ([`PreflightScope`]):
//! a timing session, an attribution lane, and the whole campaign
//! enumerate DIFFERENT id sets. Deriving the scope from "was a session
//! ordinal supplied?" conflated them (an attribution lane passed a
//! timing-only check), so the scope is now a required, explicit
//! argument and `All` unions every lane of the campaign into one id set.
//!
//! Transient environment (timestamp, load average, available memory,
//! temperature where readable) is recorded as DIAGNOSTICS ONLY. There
//! is no adaptive inclusion/exclusion of data based on whether timing
//! "looks good"; if the host is clearly busy or the policy mismatches,
//! the preflight aborts BEFORE collecting session data.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::json;

use crate::manifest::{CampaignManifest, ENVELOPE_SCHEMA_PATH};
use crate::Surface;

/// Result of a preflight run.
pub struct PreflightReport {
    pub pass: bool,
    pub blockers: Vec<String>,
    pub diagnostics: serde_json::Value,
}

/// Which host-binding checks to run. Real sessions and the real
/// preflight command run [`HostBinding::Enforce`]; the NON_RESEARCH
/// smoke runs [`HostBinding::SkipForNonResearch`] so plumbing can be
/// validated on any dev machine (explicitly recorded, never silent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostBinding {
    Enforce,
    SkipForNonResearch,
}

/// Which observation identities a preflight enumerates and guards.
///
/// The three scopes are structurally distinct — they enumerate different
/// id sets with different cardinalities — so each execution path states
/// its own:
///
/// ```text
/// All                             whole campaign (both lanes, all sessions)
/// Timing  { surface, session }    one timing session
/// Attribution { surface }         one attribution lane
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightScope {
    /// Every timing session of both surfaces PLUS both attribution
    /// lanes, unioned into one campaign-wide id set.
    All,
    /// Exactly one timing session of one surface.
    Timing { surface: Surface, session: u32 },
    /// Exactly one attribution lane (no timing session).
    Attribution { surface: Surface },
}

impl PreflightScope {
    /// Stable label used in diagnostics and blocker messages.
    pub fn label(self) -> String {
        match self {
            PreflightScope::All => "all".to_string(),
            PreflightScope::Timing { surface, session } => {
                format!("timing:{}:{session}", surface.as_str())
            }
            PreflightScope::Attribution { surface } => {
                format!("attribution:{}", surface.as_str())
            }
        }
    }
}

/// Run the preflight for one explicit observation scope.
pub fn preflight(
    benchmark_root: &std::path::Path,
    host_binding: HostBinding,
    scope: PreflightScope,
) -> PreflightReport {
    let mut blockers: Vec<String> = Vec::new();

    // 1. Campaign manifest verifies against live state.
    let manifest = match CampaignManifest::load(benchmark_root) {
        Ok(manifest) => manifest,
        Err(error) => {
            blockers.push(format!("campaign manifest: {error}"));
            return report(blockers, scope, serde_json::Value::Null);
        }
    };
    if let Err(mut manifest_blockers) = manifest.verify(benchmark_root) {
        blockers.append(&mut manifest_blockers);
    }

    // 2. Workload freeze receipt artifacts (WORKLOAD_IDENTITY_CHANGED).
    if let Err(mut workload_blockers) =
        crate::receipt::verify_workload_freeze_receipt(benchmark_root)
    {
        blockers.append(&mut workload_blockers);
    }

    // 3. Campaign receipt valid (includes schedule regeneration check).
    if let Err(mut receipt_blockers) = crate::receipt::verify_receipt(benchmark_root) {
        blockers.append(&mut receipt_blockers);
    }

    // 4. Schedule valid (explicit re-verification; verify_receipt also
    //    byte-compares a regeneration).
    if let Err(mut schedule_blockers) = crate::receipt::verify_schedule_on_disk(benchmark_root) {
        blockers.append(&mut schedule_blockers);
    }

    // 5. Result schema v2 + envelope schema have not drifted from the
    //    Rust models (checked-in JSON == generated JSON).
    let result_schema_generated =
        serde_json::to_value(schemars::schema_for!(markit_mdbench_runner::ResultRowV1))
            .expect("result schema is serializable");
    match read_json_value(&benchmark_root.join("protocol/result-schema-v2.json")) {
        Ok(checked) if checked == result_schema_generated => {}
        Ok(_) => blockers.push(
            "protocol/result-schema-v2.json drifted from the Rust ResultRowV1 model".to_string(),
        ),
        Err(error) => blockers.push(format!("result-schema-v2.json: {error}")),
    }
    let envelope_generated =
        serde_json::to_value(schemars::schema_for!(crate::execute::CampaignObservationV1))
            .expect("envelope schema is serializable");
    match read_json_value(&benchmark_root.join(ENVELOPE_SCHEMA_PATH)) {
        Ok(checked) if checked == envelope_generated => {}
        Ok(_) => blockers.push(format!(
            "{ENVELOPE_SCHEMA_PATH} drifted from the Rust CampaignObservationV1 model"
        )),
        Err(error) => blockers.push(format!("{ENVELOPE_SCHEMA_PATH}: {error}")),
    }

    // 6. Cardinalities + duplicate ObservationIds for the requested scope.
    let workload = match crate::workload::load_campaign_workload(benchmark_root) {
        Ok(workload) => workload,
        Err(error) => {
            blockers.push(format!("source materialization: {error}"));
            return report(blockers, scope, serde_json::Value::Null);
        }
    };
    let enumeration = check_observation_uniqueness(&manifest, &workload, scope, &mut blockers);

    // 7. Host binding: machine match + build identity + affinity.
    let build = markit_mdbench_runner::current_build_identity();
    match host_binding {
        HostBinding::Enforce => {
            if build.build_profile_id
                != markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID
            {
                blockers.push(format!(
                    "build profile {:?} != release-primary-v1; primary sessions must run the frozen release profile",
                    build.build_profile_id
                ));
            }
            if build.runner_git_commit == "unknown" {
                blockers.push(
                    "runner git commit unavailable (built outside a git workspace?)".to_string(),
                );
            }
            match crate::manifest::MachineManifest::load(benchmark_root) {
                Ok(machine) => {
                    if let Err(mut machine_blockers) =
                        crate::machine::match_current_host(&machine, benchmark_root)
                    {
                        blockers.append(&mut machine_blockers);
                    }
                }
                Err(error) => blockers.push(format!("machine manifest: {error}")),
            }
        }
        HostBinding::SkipForNonResearch => {
            // NON_RESEARCH smoke: host-binding checks are deliberately
            // skipped and recorded as such; the smoke's output can never
            // be used as performance evidence.
        }
    }

    report(
        blockers,
        scope,
        serde_json::to_value(&enumeration).unwrap_or(serde_json::Value::Null),
    )
}

fn report(
    blockers: Vec<String>,
    scope: PreflightScope,
    enumeration: serde_json::Value,
) -> PreflightReport {
    PreflightReport {
        pass: blockers.is_empty(),
        blockers,
        diagnostics: json!({
            "transient": crate::machine::transient_diagnostics(),
            "scope": scope.label(),
            "observation_enumeration": enumeration,
        }),
    }
}

fn read_json_value(path: &std::path::Path) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Which iterations a lane carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaneKind {
    /// Warmup + measured iterations per case × horse.
    Timing { warmup: u32, measured: u32 },
    /// Exactly one dispatch per case × horse.
    Attribution,
}

/// The enumerated identity facts of one lane of one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaneEnumeration {
    pub lane: String,
    pub rows: usize,
    pub unique_ids: usize,
    /// `rows - unique_ids`: any non-zero value is a collision.
    pub duplicate_ids: usize,
    pub warmup_rows: usize,
    pub measured_rows: usize,
    pub attribution_rows: usize,
}

/// One lane's ids plus the facts derived from them.
struct LaneEnumerationIds {
    summary: LaneEnumeration,
    ids: BTreeSet<String>,
}

/// What a preflight scope actually enumerated (recorded in diagnostics,
/// so a run receipt shows WHICH identities were checked).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ObservationEnumeration {
    pub scope: String,
    pub lanes: Vec<LaneEnumeration>,
    /// Rows enumerated by this scope (all lanes).
    pub rows: usize,
    /// Distinct ids across every lane of this scope.
    pub unique_ids: usize,
    /// `rows - unique_ids`: collisions, including cross-lane collisions
    /// (the union is campaign-wide for [`PreflightScope::All`]).
    pub duplicate_ids: usize,
}

/// Enumerate the deterministic ObservationIds of the requested scope and
/// fail on any duplicate or cardinality mismatch (task §10, §22).
///
/// The expectations come from the FROZEN surface cardinalities, not from
/// the materialized case list: a missing or extra case changes the row
/// count, a repeated case changes the unique-id count.
fn check_observation_uniqueness(
    manifest: &CampaignManifest,
    workload: &crate::workload::CampaignWorkload,
    scope: PreflightScope,
    blockers: &mut Vec<String>,
) -> ObservationEnumeration {
    // The identity binding that observations are derived from is
    // deterministic; duplicate detection runs over the derived ids. The
    // run id is a placeholder: collisions in the (run, session, surface,
    // case, horse, kind, iteration) tuple are what is being checked.
    let fake_run = "preflight-observation-enumeration";
    let case_ids = |surface: Surface| -> Vec<String> {
        match surface {
            Surface::CleanState => workload
                .clean_state
                .iter()
                .map(|case| case.case_id_hex.clone())
                .collect(),
            Surface::EditWrite => workload
                .edit_write
                .iter()
                .map(|case| case.case_id_hex.clone())
                .collect(),
        }
    };

    let mut enumeration = ObservationEnumeration {
        scope: scope.label(),
        ..ObservationEnumeration::default()
    };
    // Campaign-wide id union: for `All` this catches cross-lane
    // collisions (e.g. an attribution id colliding with a timing id).
    let mut union: BTreeSet<String> = BTreeSet::new();

    fn record(
        lane: LaneEnumerationIds,
        union: &mut BTreeSet<String>,
        enumeration: &mut ObservationEnumeration,
        blockers: &mut Vec<String>,
    ) {
        for id in &lane.ids {
            if !union.insert(id.clone()) {
                blockers.push(format!(
                    "duplicate ObservationId across lanes in scope {}: {id}",
                    enumeration.scope
                ));
            }
        }
        enumeration.rows += lane.summary.rows;
        enumeration.lanes.push(lane.summary);
    }

    match scope {
        PreflightScope::All => {
            for surface in [Surface::CleanState, Surface::EditWrite] {
                let cases = case_ids(surface);
                check_frozen_case_count(surface, cases.len(), blockers);
                for session in 0..manifest.sessions.count {
                    let lane = enumerate_timing_session(
                        fake_run,
                        surface,
                        &cases,
                        session,
                        manifest.sessions.warmup_iterations,
                        manifest.sessions.measured_iterations,
                    );
                    check_timing_cardinality(manifest, surface, session, &lane, blockers);
                    record(lane, &mut union, &mut enumeration, blockers);
                }
                let lane = enumerate_attribution(fake_run, surface, &cases);
                check_attribution_cardinality(surface, &lane, blockers);
                record(lane, &mut union, &mut enumeration, blockers);
            }
            // Campaign-wide totals (task §10): 230,400 timing rows across
            // 3 sessions x 2 surfaces + 1,920 attribution rows = 232,320.
            check_campaign_totals(manifest, &enumeration, blockers);
        }
        PreflightScope::Timing { surface, session } => {
            if session >= manifest.sessions.count {
                blockers.push(format!(
                    "timing scope session {session} is outside the frozen session count {}",
                    manifest.sessions.count
                ));
                return enumeration;
            }
            let cases = case_ids(surface);
            check_frozen_case_count(surface, cases.len(), blockers);
            let lane = enumerate_timing_session(
                fake_run,
                surface,
                &cases,
                session,
                manifest.sessions.warmup_iterations,
                manifest.sessions.measured_iterations,
            );
            check_timing_cardinality(manifest, surface, session, &lane, blockers);
            record(lane, &mut union, &mut enumeration, blockers);
        }
        PreflightScope::Attribution { surface } => {
            let cases = case_ids(surface);
            check_frozen_case_count(surface, cases.len(), blockers);
            let lane = enumerate_attribution(fake_run, surface, &cases);
            check_attribution_cardinality(surface, &lane, blockers);
            record(lane, &mut union, &mut enumeration, blockers);
        }
    }

    enumeration.unique_ids = union.len();
    enumeration.duplicate_ids = enumeration.rows.saturating_sub(union.len());
    enumeration
}

/// Enumerate one timing session of one surface.
fn enumerate_timing_session(
    run: &str,
    surface: Surface,
    case_ids: &[String],
    session_ordinal: u32,
    warmup_iterations: u32,
    measured_iterations: u32,
) -> LaneEnumerationIds {
    let session_id = crate::identity::session_id(run, surface.as_str(), session_ordinal);
    enumerate_lane(
        run,
        surface,
        &session_id,
        case_ids,
        LaneKind::Timing {
            warmup: warmup_iterations,
            measured: measured_iterations,
        },
        format!("timing:{}:{session_ordinal}", surface.as_str()),
    )
}

/// Enumerate one attribution lane of one surface (no timing session).
fn enumerate_attribution(run: &str, surface: Surface, case_ids: &[String]) -> LaneEnumerationIds {
    let session_id = crate::execute::attribution_session_id(run, surface.as_str());
    enumerate_lane(
        run,
        surface,
        &session_id,
        case_ids,
        LaneKind::Attribution,
        format!("attribution:{}", surface.as_str()),
    )
}

/// The single enumeration primitive both lanes go through: every
/// `case × horse` of the frozen roster, with the lane's iterations.
fn enumerate_lane(
    run: &str,
    surface: Surface,
    session_id: &str,
    case_ids: &[String],
    kind: LaneKind,
    lane_label: String,
) -> LaneEnumerationIds {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let (mut rows, mut warmup_rows, mut measured_rows, mut attribution_rows) = (0, 0, 0, 0);
    let mut push = |case_id: &str, horse: &str, sample_kind: &str, iteration: u32| {
        ids.insert(crate::identity::observation_id(
            run,
            session_id,
            surface.as_str(),
            case_id,
            horse,
            sample_kind,
            iteration,
        ));
    };
    for case_id in case_ids {
        for horse in crate::HORSE_IDS {
            match kind {
                LaneKind::Timing { warmup, measured } => {
                    for iteration in 0..warmup {
                        push(case_id, horse, "warmup", iteration);
                        warmup_rows += 1;
                        rows += 1;
                    }
                    for iteration in 0..measured {
                        push(case_id, horse, "measured", iteration);
                        measured_rows += 1;
                        rows += 1;
                    }
                }
                LaneKind::Attribution => {
                    push(case_id, horse, "attribution", 0);
                    attribution_rows += 1;
                    rows += 1;
                }
            }
        }
    }
    let unique_ids = ids.len();
    LaneEnumerationIds {
        summary: LaneEnumeration {
            lane: lane_label,
            rows,
            unique_ids,
            duplicate_ids: rows.saturating_sub(unique_ids),
            warmup_rows,
            measured_rows,
            attribution_rows,
        },
        ids,
    }
}

/// The materialized case list must be exactly the frozen surface size
/// before any per-lane expectation is meaningful.
fn check_frozen_case_count(surface: Surface, materialized: usize, blockers: &mut Vec<String>) {
    if materialized != surface.frozen_case_count() {
        blockers.push(format!(
            "surface {}: {} materialized cases != frozen {}",
            surface.as_str(),
            materialized,
            surface.frozen_case_count()
        ));
    }
}

/// Timing expectations come from the FROZEN surface cardinality, never
/// from the materialized case list: a missing or extra case must move
/// the row count, not the target.
fn check_timing_cardinality(
    manifest: &CampaignManifest,
    surface: Surface,
    session: u32,
    lane: &LaneEnumerationIds,
    blockers: &mut Vec<String>,
) {
    let horses = crate::HORSE_IDS.len();
    let cases = surface.frozen_case_count();
    let warmup = cases * horses * manifest.sessions.warmup_iterations as usize;
    let measured = cases * horses * manifest.sessions.measured_iterations as usize;
    let rows = warmup + measured;
    if lane.summary.duplicate_ids != 0 {
        blockers.push(format!(
            "duplicate ObservationId in timing {} session {session}: {} rows, {} distinct ids",
            surface.as_str(),
            lane.summary.rows,
            lane.summary.unique_ids
        ));
    }
    if lane.summary.rows != rows
        || lane.summary.warmup_rows != warmup
        || lane.summary.measured_rows != measured
        || lane.summary.attribution_rows != 0
    {
        blockers.push(format!(
            "timing {} session {session}: enumerated {} rows ({} warmup / {} measured / {} attribution) != expected {rows} ({warmup} / {measured} / 0)",
            surface.as_str(),
            lane.summary.rows,
            lane.summary.warmup_rows,
            lane.summary.measured_rows,
            lane.summary.attribution_rows
        ));
    }
}

fn check_attribution_cardinality(
    surface: Surface,
    lane: &LaneEnumerationIds,
    blockers: &mut Vec<String>,
) {
    let rows = surface.frozen_case_count() * crate::HORSE_IDS.len();
    if lane.summary.duplicate_ids != 0 {
        blockers.push(format!(
            "duplicate attribution ObservationId in {}: {} rows, {} distinct ids",
            surface.as_str(),
            lane.summary.rows,
            lane.summary.unique_ids
        ));
    }
    if lane.summary.rows != rows
        || lane.summary.attribution_rows != rows
        || lane.summary.warmup_rows != 0
        || lane.summary.measured_rows != 0
    {
        blockers.push(format!(
            "attribution {}: enumerated {} rows ({} attribution / {} warmup / {} measured) != expected {rows}",
            surface.as_str(),
            lane.summary.rows,
            lane.summary.attribution_rows,
            lane.summary.warmup_rows,
            lane.summary.measured_rows
        ));
    }
}

/// Whole-campaign totals for [`PreflightScope::All`]: the frozen
/// cardinalities AND a duplicate-free union across every lane.
fn check_campaign_totals(
    manifest: &CampaignManifest,
    enumeration: &ObservationEnumeration,
    blockers: &mut Vec<String>,
) {
    let cells = manifest.cells_per_session();
    let per_session = manifest.timing_rows_per_session();
    let expected_frozen = crate::schedule::expected_frozen_schedule_rows(manifest.sessions.count);
    if cells != 1920 || per_session != 76_800 || expected_frozen != 1152 {
        blockers.push(format!(
            "cardinality guard: cells/session {cells} != 1920 or rows/session {per_session} != 76800 or schedule rows {expected_frozen} != 1152"
        ));
    }
    let timing_rows = per_session * manifest.sessions.count as usize;
    let attribution_rows = cells;
    let rows = timing_rows + attribution_rows;
    if enumeration.rows != rows {
        blockers.push(format!(
            "campaign-wide enumeration: {} rows != expected {rows} ({timing_rows} timing + {attribution_rows} attribution)",
            enumeration.rows
        ));
    }
    if enumeration.duplicate_ids != 0 {
        blockers.push(format!(
            "campaign-wide enumeration: {} duplicate ObservationIds over {} rows ({} distinct)",
            enumeration.duplicate_ids, enumeration.rows, enumeration.unique_ids
        ));
    }
}

/// Serialize a preflight report as the CLI's JSON output.
pub fn preflight_json(report: &PreflightReport, host_binding: HostBinding) -> String {
    let verdict = if report.pass {
        "CAMPAIGN_PREFLIGHT_PASS"
    } else {
        "CAMPAIGN_PREFLIGHT_BLOCKED"
    };
    let host_binding = match host_binding {
        HostBinding::Enforce => "enforced",
        HostBinding::SkipForNonResearch => "skipped-non-research",
    };
    json!({
        "verdict": verdict,
        "host_binding": host_binding,
        "blockers": report.blockers,
        "diagnostics": report.diagnostics,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn benchmark_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn frozen_manifest() -> CampaignManifest {
        CampaignManifest::load(&benchmark_root()).expect("frozen campaign manifest loads")
    }

    /// Synthetic case ids shaped like the frozen ones (64 lowercase hex).
    fn cases(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("{index:064x}")).collect()
    }

    fn attribution_blockers(surface: Surface, case_ids: &[String]) -> Vec<String> {
        let mut blockers = Vec::new();
        check_frozen_case_count(surface, case_ids.len(), &mut blockers);
        let lane = enumerate_attribution("test-run", surface, case_ids);
        check_attribution_cardinality(surface, &lane, &mut blockers);
        blockers
    }

    fn timing_blockers(surface: Surface, case_ids: &[String], session: u32) -> Vec<String> {
        let manifest = frozen_manifest();
        let mut blockers = Vec::new();
        check_frozen_case_count(surface, case_ids.len(), &mut blockers);
        let lane = enumerate_timing_session(
            "test-run",
            surface,
            case_ids,
            session,
            manifest.sessions.warmup_iterations,
            manifest.sessions.measured_iterations,
        );
        check_timing_cardinality(&manifest, surface, session, &lane, &mut blockers);
        blockers
    }

    #[test]
    fn attribution_lane_passes_the_frozen_shape_without_blockers() {
        // 22 x 5 = 110 and 362 x 5 = 1810 (task §10).
        for (surface, count, rows) in [
            (Surface::CleanState, 22, 110),
            (Surface::EditWrite, 362, 1810),
        ] {
            let case_ids = cases(count);
            assert!(attribution_blockers(surface, &case_ids).is_empty());
            let lane = enumerate_attribution("test-run", surface, &case_ids);
            assert_eq!(lane.summary.rows, rows);
            assert_eq!(lane.summary.attribution_rows, rows);
            assert_eq!(lane.summary.duplicate_ids, 0);
            assert_eq!(lane.summary.unique_ids, rows);
        }
    }

    #[test]
    fn attribution_scope_blocks_a_duplicate_case() {
        // A repeated case dispatches twice under one identity: the second
        // row collides, which must abort the attribution lane.
        let mut case_ids = cases(22);
        case_ids[7] = case_ids[3].clone();
        let blockers = attribution_blockers(Surface::CleanState, &case_ids);
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("duplicate attribution ObservationId")),
            "duplicate attribution case must block; got {blockers:?}"
        );
    }

    #[test]
    fn attribution_scope_blocks_a_missing_case() {
        // 21 materialized cases against the frozen 22: missing rows, and
        // the count guard fires before any row is dispatched.
        let blockers = attribution_blockers(Surface::CleanState, &cases(21));
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("21 materialized cases != frozen 22")),
            "missing case must block; got {blockers:?}"
        );
        assert!(
            blockers.iter().any(|b| b.contains("!= expected 110")),
            "short attribution lane must block on cardinality; got {blockers:?}"
        );
    }

    #[test]
    fn attribution_scope_blocks_an_extra_case() {
        let blockers = attribution_blockers(Surface::EditWrite, &cases(363));
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("363 materialized cases != frozen 362")),
            "extra case must block; got {blockers:?}"
        );
    }

    #[test]
    fn timing_session_passes_the_frozen_shape_and_blocks_duplicates() {
        // 22 x 5 x (10 + 30) = 4,400 rows per CLEAN_STATE session.
        let case_ids = cases(22);
        assert!(timing_blockers(Surface::CleanState, &case_ids, 0).is_empty());
        let lane = enumerate_timing_session("test-run", Surface::CleanState, &case_ids, 0, 10, 30);
        assert_eq!(lane.summary.rows, 4_400);
        assert_eq!(lane.summary.warmup_rows, 1_100);
        assert_eq!(lane.summary.measured_rows, 3_300);
        assert_eq!(lane.summary.attribution_rows, 0);

        let mut duplicated = case_ids.clone();
        duplicated[1] = duplicated[0].clone();
        let blockers = timing_blockers(Surface::CleanState, &duplicated, 0);
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("duplicate ObservationId in timing clean_state session 0")),
            "duplicate timing case must block; got {blockers:?}"
        );
    }

    #[test]
    fn attribution_ids_never_collide_with_timing_ids_or_each_other() {
        // The attribution lane must not reuse a timing session id space
        // (the defect this scope split fixes) and the two surfaces must
        // not collide either.
        let mut union = BTreeSet::new();
        let mut total = 0usize;
        for (surface, count, rows) in [
            (Surface::CleanState, 22, 110),
            (Surface::EditWrite, 362, 1810),
        ] {
            let case_ids = cases(count);
            for session in 0..3u32 {
                let lane =
                    enumerate_timing_session("test-run", surface, &case_ids, session, 10, 30);
                for id in &lane.ids {
                    assert!(union.insert(id.clone()), "timing id collided: {id}");
                }
                total += lane.summary.rows;
            }
            let lane = enumerate_attribution("test-run", surface, &case_ids);
            for id in &lane.ids {
                assert!(union.insert(id.clone()), "attribution id collided: {id}");
            }
            total += rows;
        }
        assert_eq!(total, 3 * 76_800 + 1_920);
        assert_eq!(union.len(), total);
    }

    #[test]
    fn timing_scope_rejects_a_session_outside_the_frozen_count() {
        let manifest = frozen_manifest();
        let workload =
            crate::workload::load_campaign_workload(&benchmark_root()).expect("workload loads");
        let mut blockers = Vec::new();
        let enumeration = check_observation_uniqueness(
            &manifest,
            &workload,
            PreflightScope::Timing {
                surface: Surface::CleanState,
                session: 3,
            },
            &mut blockers,
        );
        assert_eq!(enumeration.rows, 0);
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("outside the frozen session count")),
            "out-of-range session must block; got {blockers:?}"
        );
    }

    #[test]
    fn real_scopes_enumerate_the_frozen_campaign_cardinalities() {
        let manifest = frozen_manifest();
        let workload =
            crate::workload::load_campaign_workload(&benchmark_root()).expect("workload loads");

        let mut all_blockers = Vec::new();
        let all = check_observation_uniqueness(
            &manifest,
            &workload,
            PreflightScope::All,
            &mut all_blockers,
        );
        assert!(
            all_blockers.is_empty(),
            "All scope blockers: {all_blockers:?}"
        );
        assert_eq!(all.rows, 230_400 + 1_920);
        assert_eq!(all.unique_ids, all.rows);
        assert_eq!(all.duplicate_ids, 0);
        // 3 sessions x 2 surfaces + 2 attribution lanes.
        assert_eq!(all.lanes.len(), 8);

        let mut attribution_blockers = Vec::new();
        let attribution = check_observation_uniqueness(
            &manifest,
            &workload,
            PreflightScope::Attribution {
                surface: Surface::EditWrite,
            },
            &mut attribution_blockers,
        );
        assert!(attribution_blockers.is_empty());
        assert_eq!(attribution.rows, 1_810);
        assert_eq!(attribution.lanes[0].attribution_rows, 1_810);
        assert_eq!(attribution.lanes[0].warmup_rows, 0);
        assert_eq!(attribution.lanes[0].measured_rows, 0);

        let mut timing_blockers = Vec::new();
        let timing = check_observation_uniqueness(
            &manifest,
            &workload,
            PreflightScope::Timing {
                surface: Surface::CleanState,
                session: 2,
            },
            &mut timing_blockers,
        );
        assert!(timing_blockers.is_empty());
        assert_eq!(timing.rows, 4_400);
        assert_eq!(timing.lanes[0].warmup_rows, 1_100);
        assert_eq!(timing.lanes[0].measured_rows, 3_300);
    }
}
