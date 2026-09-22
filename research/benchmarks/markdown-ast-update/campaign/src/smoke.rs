//! End-to-end NON-RESEARCH campaign plumbing smoke (task §49).
//!
//! Executes the full campaign pipeline — manifest load, schedule load,
//! FULL_READ dispatch, EDIT_WRITE dispatch, horse ordering, fresh-state
//! reset, post-timer correctness, campaign envelope emission, raw-row
//! serialization, failure propagation, receipt validation — on a TINY
//! fixed subset of the frozen workload with a FAKE deterministic
//! clock. No wall-clock value from this path is performance evidence;
//! every row carries `CAMPAIGN_SMOKE/NON_RESEARCH_RESULT` and output is
//! written OUTSIDE the primary raw result path.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::execute::{
    CampaignClock, ExecutionIdentity, ScheduledCase, SessionExecutor, SessionOutcome,
    PROVENANCE_NON_RESEARCH_SMOKE,
};
use crate::manifest::CampaignManifest;
use crate::preflight::HostBinding;
use crate::Surface;

#[derive(Debug, Clone)]
pub struct SmokeOptions {
    /// Tiny fixed subset size per surface (default 1).
    pub cases_per_surface: usize,
    pub warmup: u32,
    pub measured: u32,
    /// Also prove failure propagation (poisoned correctness hook on one
    /// extra mini-run).
    pub inject_failure: bool,
}

impl Default for SmokeOptions {
    fn default() -> Self {
        Self {
            cases_per_surface: 1,
            warmup: 2,
            measured: 2,
            inject_failure: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmokeReport {
    pub schema: String,
    pub non_research: bool,
    pub verdict: String,
    pub checks: Vec<String>,
    pub timing_rows: u64,
    pub attribution_rows: u64,
    pub failure_propagated: bool,
}

/// Run the fake-clock smoke. `out_dir` must be OUTSIDE the primary raw
/// result path; the caller enforces that (CLI + tests).
pub fn run_smoke(
    benchmark_root: &Path,
    out_dir: &Path,
    options: &SmokeOptions,
) -> Result<SmokeReport, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let mut checks = Vec::new();

    // Receipt + workload + schedule validation on the REAL frozen
    // artifacts (task §49 "receipt validation").
    let preflight =
        crate::preflight::preflight(benchmark_root, HostBinding::SkipForNonResearch, None, None);
    if !preflight.pass {
        return Err(format!(
            "smoke preflight failed (frozen artifacts invalid): {:?}",
            preflight.blockers
        ));
    }
    checks.push("preflight-receipt-schedule-workload".to_string());

    let manifest = CampaignManifest::load(benchmark_root)?;
    let workload = crate::workload::load_campaign_workload(benchmark_root)?;
    checks.push("manifest-load".to_string());
    checks.push("workload-materialization".to_string());

    // Load the FROZEN schedule and take the tiny fixed subset.
    let schedule_bytes =
        std::fs::read(benchmark_root.join(crate::manifest::SCHEDULE_MANIFEST_PATH))
            .map_err(|e| format!("read schedule: {e}"))?;
    let schedule = crate::schedule::schedule_from_jsonl(&schedule_bytes)?;
    checks.push("schedule-load".to_string());

    let binding = crate::receipt::build_spec_binding(benchmark_root, &manifest)?;
    let spec_id = crate::identity::campaign_spec_id(&binding);
    let machine_digest =
        crate::sha256_file(&benchmark_root.join(crate::manifest::MACHINE_MANIFEST_PATH))?;
    let build = markit_mdbench_runner::current_build_identity();
    // NON-RESEARCH run id: deterministic, explicitly not the research
    // RunId (which binds the approved runner commit of a real run).
    let run_id = crate::identity::run_id(&spec_id, "NON_RESEARCH_SMOKE", &machine_digest, &build);
    let identity = ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!(
            "{}#non-research-smoke",
            crate::manifest::MACHINE_MANIFEST_PATH
        ),
        provenance: PROVENANCE_NON_RESEARCH_SMOKE,
        non_research: true,
    };

    let mut total_timing_rows = 0u64;
    let mut total_attribution_rows = 0u64;

    // ---- timing smoke: both surfaces, session 0 ----------------------
    for surface in [Surface::CleanState, Surface::EditWrite] {
        let session_ordinal = 0u32;
        let session_id = crate::identity::session_id(&spec_id, surface.as_str(), session_ordinal);
        let session_seed =
            crate::identity::session_seed(manifest.seed.value, surface.as_str(), session_ordinal);
        let subset: Vec<&crate::schedule::ScheduleRow> = schedule
            .iter()
            .filter(|row| row.surface == surface.as_str() && row.session_ordinal == session_ordinal)
            .take(options.cases_per_surface)
            .collect();
        if subset.is_empty() {
            return Err(format!("smoke subset for {} is empty", surface.as_str()));
        }
        let cases = build_scheduled_cases(&workload, &subset)?;
        let out_path = out_dir.join(format!("smoke-timing-{}.jsonl", surface.as_str()));
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("create {}: {e}", out_path.display()))?;
        let executor = SessionExecutor {
            identity: &identity,
            surface,
            session_ordinal: Some(session_ordinal),
            session_id: session_id.clone(),
            session_seed,
            build_identity: build.clone(),
            warmup: options.warmup,
            measured: options.measured,
            observations: 0,
            warmup_rows: 0,
            measured_rows: 0,
            poison_correctness: false,
        };
        let clock = CampaignClock::Fake(markit_mdbench_instrumentation::ManualClock::new());
        let outcome = executor.run(&cases, &clock, &mut file)?;
        file.flush().map_err(|e| format!("flush: {e}"))?;
        match outcome {
            SessionOutcome::Completed { observations, .. } => {
                total_timing_rows += observations;
            }
            SessionOutcome::Invalid { reason, .. } => {
                return Err(format!("smoke timing run failed: {reason}"));
            }
        }
        checks.push(format!("timing-dispatch-{}", surface.as_str()));
    }

    // ---- attribution smoke: both surfaces, tiny subset ----------------
    for surface in [Surface::CleanState, Surface::EditWrite] {
        let session_id = crate::execute::attribution_session_id(&spec_id, surface.as_str());
        let subset: Vec<&crate::schedule::ScheduleRow> = schedule
            .iter()
            .filter(|row| row.surface == surface.as_str() && row.session_ordinal == 0)
            .take(options.cases_per_surface)
            .collect();
        let cases = build_scheduled_cases(&workload, &subset)?;
        let out_path = out_dir.join(format!("smoke-attribution-{}.jsonl", surface.as_str()));
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("create {}: {e}", out_path.display()))?;
        let executor = SessionExecutor {
            identity: &identity,
            surface,
            session_ordinal: None,
            session_id,
            session_seed: manifest.seed.value,
            build_identity: build.clone(),
            warmup: 0,
            measured: 0,
            observations: 0,
            warmup_rows: 0,
            measured_rows: 0,
            poison_correctness: false,
        };
        let outcome = executor.run_attribution(&cases, &mut file)?;
        file.flush().map_err(|e| format!("flush: {e}"))?;
        match outcome {
            SessionOutcome::Completed { observations, .. } => {
                total_attribution_rows += observations;
            }
            SessionOutcome::Invalid { reason, .. } => {
                return Err(format!("smoke attribution run failed: {reason}"));
            }
        }
        checks.push(format!("attribution-dispatch-{}", surface.as_str()));
    }

    // ---- output validation: envelopes parse, v2 rows, unique ids -----
    for surface in [Surface::CleanState, Surface::EditWrite] {
        validate_output(
            &out_dir.join(format!("smoke-timing-{}.jsonl", surface.as_str())),
            &spec_id,
            options,
            true,
        )?;
        validate_output(
            &out_dir.join(format!("smoke-attribution-{}.jsonl", surface.as_str())),
            &spec_id,
            options,
            false,
        )?;
    }
    checks.push("envelope-serialization-validation".to_string());

    // ---- failure propagation ------------------------------------------
    let mut failure_propagated = false;
    if options.inject_failure {
        let surface = Surface::CleanState;
        let session_id = crate::identity::session_id(&spec_id, surface.as_str(), 99);
        let subset: Vec<&crate::schedule::ScheduleRow> = schedule
            .iter()
            .filter(|row| row.surface == surface.as_str() && row.session_ordinal == 0)
            .take(1)
            .collect();
        let cases = build_scheduled_cases(&workload, &subset)?;
        let out_path = out_dir.join("smoke-injected-failure.jsonl");
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("create {}: {e}", out_path.display()))?;
        let executor = SessionExecutor {
            identity: &identity,
            surface,
            session_ordinal: Some(99),
            session_id,
            session_seed: manifest.seed.value,
            build_identity: build.clone(),
            warmup: 1,
            measured: 1,
            observations: 0,
            warmup_rows: 0,
            measured_rows: 0,
            poison_correctness: true,
        };
        let clock = CampaignClock::Fake(markit_mdbench_instrumentation::ManualClock::new());
        let outcome = executor.run(&cases, &clock, &mut file)?;
        file.flush().map_err(|e| format!("flush: {e}"))?;
        match outcome {
            SessionOutcome::Invalid { observations, .. } => {
                // The failing row must be RETAINED in the raw output
                // (task §36) — exactly one wrong-result row then a clean
                // stop.
                let text = std::fs::read_to_string(&out_path)
                    .map_err(|e| format!("read {}: {e}", out_path.display()))?;
                let rows: Vec<crate::execute::CampaignObservationV1> = text
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| serde_json::from_str(line).map_err(|e| format!("parse row: {e}")))
                    .collect::<Result<_, _>>()?;
                if rows.len() == 1
                    && observations == 1
                    && rows[0].result_row_v2.correctness_status
                        == markit_mdbench_common::CorrectnessStatus::WrongResult
                {
                    failure_propagated = true;
                    checks.push("failure-propagation".to_string());
                } else {
                    return Err(format!(
                        "failure propagation mismatch: {} rows, observations {observations}",
                        rows.len()
                    ));
                }
            }
            SessionOutcome::Completed { .. } => {
                return Err("poisoned smoke unexpectedly completed".to_string());
            }
        }
    }

    Ok(SmokeReport {
        schema: "campaign-smoke-report-v1".to_string(),
        non_research: true,
        verdict: "NON_RESEARCH_SMOKE_PASS".to_string(),
        checks,
        timing_rows: total_timing_rows,
        attribution_rows: total_attribution_rows,
        failure_propagated,
    })
}

fn build_scheduled_cases<'a>(
    workload: &'a crate::workload::CampaignWorkload,
    subset: &[&crate::schedule::ScheduleRow],
) -> Result<Vec<ScheduledCase<'a>>, String> {
    let mut cases = Vec::new();
    for row in subset {
        match row.surface.as_str() {
            "clean_state" => {
                let case = workload
                    .clean_state
                    .iter()
                    .find(|case| case.case_id_hex == row.case_id)
                    .ok_or_else(|| format!("smoke case {} not in workload", row.case_id))?;
                cases.push(ScheduledCase::CleanState {
                    order_ordinal: row.order_ordinal,
                    horse_order: row.horse_order.clone(),
                    case,
                });
            }
            "edit_write" => {
                let case = workload
                    .edit_write
                    .iter()
                    .find(|case| case.case_id_hex == row.case_id)
                    .ok_or_else(|| format!("smoke case {} not in workload", row.case_id))?;
                cases.push(ScheduledCase::EditWrite {
                    order_ordinal: row.order_ordinal,
                    horse_order: row.horse_order.clone(),
                    case,
                });
            }
            other => return Err(format!("unknown surface {other:?}")),
        }
    }
    Ok(cases)
}

fn validate_output(
    path: &Path,
    expected_spec_id: &str,
    options: &SmokeOptions,
    timing: bool,
) -> Result<(), String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut seen = BTreeSet::new();
    let mut count = 0u64;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let observation: crate::execute::CampaignObservationV1 =
            serde_json::from_str(line).map_err(|e| format!("{} row: {e}", path.display()))?;
        if observation.schema != crate::ENVELOPE_SCHEMA_ID {
            return Err(format!("{}: envelope schema drift", path.display()));
        }
        if observation.campaign_spec_id != expected_spec_id {
            return Err(format!("{}: spec id drift", path.display()));
        }
        if observation.result_row_v2.schema_version
            != markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2
        {
            return Err(format!("{}: result row is not schema v2", path.display()));
        }
        if observation.result_row_v2.provenance_ref != PROVENANCE_NON_RESEARCH_SMOKE {
            return Err(format!(
                "{}: provenance {:?} is not the NON_RESEARCH smoke tag",
                path.display(),
                observation.result_row_v2.provenance_ref
            ));
        }
        if !seen.insert(observation.observation_id.clone()) {
            return Err(format!("{}: duplicate observation id", path.display()));
        }
        if timing {
            let lane = &observation.result_row_v2.measurement;
            match lane {
                markit_mdbench_runner::MeasurementV1::Timing(_) => {}
                _ => {
                    return Err(format!(
                        "{}: timing row carries a non-timing lane",
                        path.display()
                    ))
                }
            }
        }
        count += 1;
    }
    let expected = if timing {
        (options.cases_per_surface * 5 * (options.warmup + options.measured) as usize) as u64
    } else {
        (options.cases_per_surface * 5) as u64
    };
    if count != expected {
        return Err(format!(
            "{}: {count} rows != expected {expected}",
            path.display()
        ));
    }
    Ok(())
}
