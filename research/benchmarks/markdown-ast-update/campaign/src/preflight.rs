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
//! no duplicate ObservationIds      (per-session enumeration)
//! expected cardinalities           (cells / rows)
//! ```
//!
//! Transient environment (timestamp, load average, available memory,
//! temperature where readable) is recorded as DIAGNOSTICS ONLY. There
//! is no adaptive inclusion/exclusion of data based on whether timing
//! "looks good"; if the host is clearly busy or the policy mismatches,
//! the preflight aborts BEFORE collecting session data.

use serde_json::json;

use crate::manifest::{CampaignManifest, ENVELOPE_SCHEMA_PATH};

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

/// Run the preflight. `surface`/`session_ordinal` scope the duplicate
/// ObservationId enumeration (the full-campaign enumeration is the
/// union over all sessions).
pub fn preflight(
    benchmark_root: &std::path::Path,
    host_binding: HostBinding,
    surface: Option<crate::Surface>,
    session_ordinal: Option<u32>,
) -> PreflightReport {
    let mut blockers: Vec<String> = Vec::new();

    // 1. Campaign manifest verifies against live state.
    let manifest = match CampaignManifest::load(benchmark_root) {
        Ok(manifest) => manifest,
        Err(error) => {
            blockers.push(format!("campaign manifest: {error}"));
            return report(blockers);
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

    // 6. Cardinalities + duplicate ObservationIds per requested scope.
    let workload = match crate::workload::load_campaign_workload(benchmark_root) {
        Ok(workload) => workload,
        Err(error) => {
            blockers.push(format!("source materialization: {error}"));
            return report(blockers);
        }
    };
    check_observation_uniqueness(
        &manifest,
        &workload,
        surface,
        session_ordinal,
        &mut blockers,
    );

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

    report(blockers)
}

fn report(blockers: Vec<String>) -> PreflightReport {
    PreflightReport {
        pass: blockers.is_empty(),
        blockers,
        diagnostics: crate::machine::transient_diagnostics(),
    }
}

fn read_json_value(path: &std::path::Path) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Enumerate the deterministic ObservationIds of the requested scope
/// and fail on any duplicate or cardinality mismatch (task §10, §22).
fn check_observation_uniqueness(
    manifest: &CampaignManifest,
    workload: &crate::workload::CampaignWorkload,
    surface: Option<crate::Surface>,
    session_ordinal: Option<u32>,
    blockers: &mut Vec<String>,
) {
    let surfaces: Vec<crate::Surface> = match surface {
        Some(surface) => vec![surface],
        None => vec![crate::Surface::CleanState, crate::Surface::EditWrite],
    };
    // The identity binding that observations are derived from is
    // deterministic; duplicate detection runs over the derived ids.
    let fake_run = "preflight-observation-enumeration";
    for surface in surfaces {
        let case_ids: Vec<String> = match surface {
            crate::Surface::CleanState => workload
                .clean_state
                .iter()
                .map(|case| case.case_id_hex.clone())
                .collect(),
            crate::Surface::EditWrite => workload
                .edit_write
                .iter()
                .map(|case| case.case_id_hex.clone())
                .collect(),
        };
        if case_ids.len() != surface.frozen_case_count() {
            blockers.push(format!(
                "surface {}: {} materialized cases != frozen {}",
                surface.as_str(),
                case_ids.len(),
                surface.frozen_case_count()
            ));
        }
        let sessions: Vec<Option<u32>> = match session_ordinal {
            Some(ordinal) => vec![Some(ordinal)],
            None => (0..manifest.sessions.count).map(Some).collect(),
        };
        for session in sessions {
            let session_id = match session {
                Some(ordinal) => crate::identity::session_id(fake_run, surface.as_str(), ordinal),
                None => crate::execute::attribution_session_id(fake_run, surface.as_str()),
            };
            let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            let mut total = 0usize;
            for case_id in &case_ids {
                for horse in crate::HORSE_IDS {
                    if session.is_some() {
                        // Timing: warmup + measured iterations.
                        for kind in ["warmup", "measured"] {
                            let iterations = if kind == "warmup" {
                                manifest.sessions.warmup_iterations
                            } else {
                                manifest.sessions.measured_iterations
                            };
                            for iteration in 0..iterations {
                                let id = crate::identity::observation_id(
                                    fake_run,
                                    &session_id,
                                    surface.as_str(),
                                    case_id,
                                    horse,
                                    kind,
                                    iteration,
                                );
                                if !seen.insert(id) {
                                    blockers.push(format!(
                                        "duplicate ObservationId in {} session {:?}",
                                        surface.as_str(),
                                        session
                                    ));
                                }
                                total += 1;
                            }
                        }
                    } else {
                        // Attribution: one dispatch per case × horse.
                        let id = crate::identity::observation_id(
                            fake_run,
                            &session_id,
                            surface.as_str(),
                            case_id,
                            horse,
                            "attribution",
                            0,
                        );
                        if !seen.insert(id) {
                            blockers.push(format!(
                                "duplicate attribution ObservationId in {}",
                                surface.as_str()
                            ));
                        }
                        total += 1;
                    }
                }
            }
            let expected = match session {
                Some(_) => {
                    case_ids.len()
                        * 5
                        * (manifest.sessions.warmup_iterations
                            + manifest.sessions.measured_iterations)
                            as usize
                }
                None => case_ids.len() * 5,
            };
            if total != expected {
                blockers.push(format!(
                    "{} session {:?}: {} enumerated observations != expected {expected}",
                    surface.as_str(),
                    session,
                    total
                ));
            }
        }
    }
    // Full-campaign cardinality guards (task §10): derived from the
    // frozen surface counts, so they cannot silently drift from 1920 /
    // 76,800 / 1,152.
    let cells = manifest.cells_per_session();
    let per_session = manifest.timing_rows_per_session();
    let expected_frozen = crate::schedule::expected_frozen_schedule_rows(manifest.sessions.count);
    if cells != 1920 || per_session != 76_800 || expected_frozen != 1152 {
        blockers.push(format!(
            "cardinality guard: cells/session {cells} != 1920 or rows/session {per_session} != 76800 or schedule rows {expected_frozen} != 1152"
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
