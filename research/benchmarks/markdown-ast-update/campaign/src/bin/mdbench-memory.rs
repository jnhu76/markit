//! `mdbench-memory` — the formal M-LANE collector binary (#76
//! M-COLLECTOR-1).
//!
//! The DEDICATED instrumented binary of the frozen six-horse campaign:
//! it installs the [`CountingAllocator`] as the process allocator, so
//! the per-case [`AllocReporter`] windows are backed by real allocation
//! counting. The timing/attribution campaign binary
//! (`mdbench-campaign`) NEVER installs it — timing sessions always run
//! under the plain system allocator (Gate-B §2 lane separation).
//!
//! ```text
//! mdbench-memory run-memory [benchmark_root] [--surface S --session N]
//! ```
//!
//! Cadence and order are INHERITED, never chosen here: the frozen
//! campaign manifest's `[sessions]` policy (3 sessions × (10 warmup +
//! 30 measured) per case × horse per surface — campaign-wide, no
//! memory-lane exception) and the SAME frozen schedule manifest the
//! timing sessions consume. Formal memory rows carry
//! `PRIMARY-PERFORMANCE-CAMPAIGN-v1/MEMORY` provenance; the
//! NON_RESEARCH `mdbench-memory-smoke` instrument-validation binary is
//! a separate process whose outputs can never pass this lane's
//! finalization path. This entry point exists so the frozen campaign
//! has a formal execution path; it is NOT run by the M-COLLECTOR-1
//! task itself — formal collection is a separate, later task.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign::finalize::RawLane;
use markit_mdbench_campaign::manifest::{
    CampaignManifest, MACHINE_MANIFEST_PATH, SCHEDULE_MANIFEST_PATH,
};
use markit_mdbench_campaign::preflight::{HostBinding, PreflightScope};
use markit_mdbench_campaign::runsupport;
use markit_mdbench_campaign::Surface;

/// The M-LANE process allocator (and nothing else in this binary may
/// change allocation semantics).
#[global_allocator]
static COUNTING: markit_mdbench_instrumentation::CountingAllocator =
    markit_mdbench_instrumentation::CountingAllocator;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(subcommand) = args.get(1).cloned() else {
        eprintln!("usage: mdbench-memory run-memory [benchmark_root] [--surface S --session N]");
        return ExitCode::from(2);
    };
    let root = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
    let root = PathBuf::from(root);
    let flags = &args[3.min(args.len())..];

    let result: Result<bool, String> = match subcommand.as_str() {
        "run-memory" => cmd_run_memory(&root, flags),
        "--help" | "help" => {
            println!("mdbench-memory — formal M-LANE collector (#76); subcommand: run-memory");
            Ok(true)
        }
        other => Err(format!("unknown subcommand {other:?}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("mdbench-memory: {error}");
            ExitCode::from(1)
        }
    }
}

fn require_release_profile() -> Result<(), String> {
    let build = markit_mdbench_runner::current_build_identity();
    if build.build_profile_id != markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID {
        return Err(format!(
            "formal M-LANE execution requires the frozen release profile {}; this binary was built with {}",
            markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID,
            build.build_profile_id
        ));
    }
    Ok(())
}

fn cmd_run_memory(root: &Path, flags: &[String]) -> Result<bool, String> {
    require_release_profile()?;
    // Lane guard (fail-closed): prove the process allocator is the
    // counting one BEFORE any window opens. A binary that lost its
    // #[global_allocator] installation would emit all-zero rows.
    if !markit_mdbench_instrumentation::counting_allocator_active() {
        return Err(
            "CountingAllocator is not the process allocator: the formal M-LANE runs only in the \
             dedicated memory binary"
                .to_string(),
        );
    }
    let surface = Surface::parse(
        flags
            .iter()
            .position(|flag| flag == "--surface")
            .and_then(|index| flags.get(index + 1))
            .ok_or("run-memory requires --surface")?,
    )?;
    let session: u32 = flags
        .iter()
        .position(|flag| flag == "--session")
        .and_then(|index| flags.get(index + 1))
        .and_then(|value| value.parse().ok())
        .ok_or("run-memory requires --session N")?;

    // Full fail-closed preflight BEFORE any session data is collected
    // (scope-explicit: this is the memory session's identity set).
    let report = markit_mdbench_campaign::preflight::preflight(
        root,
        HostBinding::Enforce,
        PreflightScope::Memory { surface, session },
    );
    if !report.pass {
        return Err(format!("preflight blocked: {:?}", report.blockers));
    }
    // Pin the worker to the frozen single CPU, exactly like a timing
    // session (same machine contract, same worker discipline).
    let machine = markit_mdbench_campaign::manifest::MachineManifest::load(root)?;
    markit_mdbench_campaign::machine::apply_affinity(machine.selected_cpu)?;

    let manifest = CampaignManifest::load(root)?;
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(root)?;
    let schedule_bytes = std::fs::read(root.join(SCHEDULE_MANIFEST_PATH))
        .map_err(|e| format!("read schedule: {e}"))?;
    let schedule = markit_mdbench_campaign::schedule::schedule_from_jsonl(&schedule_bytes)?;

    let binding = markit_mdbench_campaign::receipt::build_spec_binding(root, &manifest)?;
    let spec_id = markit_mdbench_campaign::identity::campaign_spec_id(&binding);
    let machine_digest = markit_mdbench_campaign::sha256_file(&root.join(MACHINE_MANIFEST_PATH))?;
    let build = markit_mdbench_runner::current_build_identity();
    // The exact binary is part of the run identity (R7 §11): the RunId
    // binds THIS executable's SHA256, so the memory RunId is distinct
    // from the timing binary's RunId by construction — one RunId per
    // execution binary, never split.
    let executable_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    let run_id = markit_mdbench_campaign::identity::run_id(
        &spec_id,
        &build.runner_git_commit,
        &machine_digest,
        &build,
        &executable_sha256,
    );
    // Raw layout: append/create-only under the memory lane; an existing
    // file for this session is a hard error, and the memory lane is
    // never split across two memory binaries.
    let spec_root = root.join("results/raw").join(&spec_id);
    runsupport::ensure_single_run_identity(&spec_root, &run_id, &runsupport::MEMORY_BINARY_LANES)?;
    let out_path = spec_root
        .join(&run_id)
        .join("memory")
        .join(format!("session-{session}-{}.jsonl", surface.as_str()));
    if out_path.exists() {
        return Err(format!(
            "{} already exists: finalized raw files are never overwritten",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }

    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!("{}#{}", MACHINE_MANIFEST_PATH, machine.machine_id),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_MEMORY,
        non_research: false,
    };
    // Memory sessions have their OWN session-id derivation (cross-lane
    // collision-free) and otherwise inherit the frozen per-(surface,
    // session) seed derivation of the timing sessions.
    let session_id =
        markit_mdbench_campaign::execute::memory_session_id(&spec_id, surface.as_str(), session);
    let session_seed = markit_mdbench_campaign::manifest::parse_seed_value(&manifest.seed.value)?;
    let session_seed =
        markit_mdbench_campaign::identity::session_seed(session_seed, surface.as_str(), session);
    let subset: Vec<&markit_mdbench_campaign::schedule::ScheduleRow> = schedule
        .iter()
        .filter(|row| row.surface == surface.as_str() && row.session_ordinal == session)
        .collect();
    let cases = schedule_rows_to_cases(&workload, &subset)?;
    // The expectation is built from the SAME schedule rows the executor
    // consumes, before anything runs.
    let expectation = markit_mdbench_campaign::finalize::expectation_from_schedule(
        &subset,
        &spec_id,
        &run_id,
        &session_id,
        surface,
        RawLane::Memory {
            session_ordinal: session,
        },
        manifest.sessions.warmup_iterations,
        manifest.sessions.measured_iterations,
    )?;
    let executor = markit_mdbench_campaign::execute::SessionExecutor {
        identity: &identity,
        surface,
        session_ordinal: Some(session),
        session_id,
        session_seed,
        build_identity: build,
        warmup: manifest.sessions.warmup_iterations,
        measured: manifest.sessions.measured_iterations,
        observations: 0,
        warmup_rows: 0,
        measured_rows: 0,
        poison_correctness: false,
    };
    let mut file = std::fs::File::create(&out_path)
        .map_err(|e| format!("create {}: {e}", out_path.display()))?;
    let reporter = markit_mdbench_instrumentation::AllocReporter::new();
    let outcome = executor.run_memory(&cases, &reporter, &mut file)?;
    use std::io::Write;
    file.flush().map_err(|e| format!("flush: {e}"))?;
    drop(file);
    // Finalization: the raw file is re-read and checked against the
    // frozen contract (identity, coverage, cardinality, memory-lane
    // provenance and window facts); only a passing file gets a receipt.
    let finalized = runsupport::finalize_raw_file(&out_path, &expectation, &executable_sha256)?;
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Completed { observations, .. }
            if finalized =>
        {
            println!(
                "MEMORY_SESSION_COMPLETE observations={observations} lane=memory surface={} session={session}",
                surface.as_str()
            );
            Ok(true)
        }
        markit_mdbench_campaign::execute::SessionOutcome::Completed { .. } => {
            eprintln!("PRIMARY_CAMPAIGN_INVALID raw file failed finalization");
            Ok(false)
        }
        markit_mdbench_campaign::execute::SessionOutcome::Invalid { reason, .. } => {
            eprintln!("PRIMARY_CAMPAIGN_INVALID {reason}");
            Ok(false)
        }
    }
}

fn schedule_rows_to_cases<'a>(
    workload: &'a markit_mdbench_campaign::workload::CampaignWorkload,
    subset: &[&markit_mdbench_campaign::schedule::ScheduleRow],
) -> Result<Vec<markit_mdbench_campaign::execute::ScheduledCase<'a>>, String> {
    let mut cases = Vec::new();
    for row in subset {
        match row.surface.as_str() {
            "clean_state" => {
                let case = workload
                    .clean_state
                    .iter()
                    .find(|case| case.case_id_hex == row.case_id)
                    .ok_or_else(|| format!("case {} not in workload", row.case_id))?;
                cases.push(
                    markit_mdbench_campaign::execute::ScheduledCase::CleanState {
                        order_ordinal: row.order_ordinal,
                        horse_order: row.horse_order.clone(),
                        case,
                    },
                );
            }
            "edit_write" => {
                let case = workload
                    .edit_write
                    .iter()
                    .find(|case| case.case_id_hex == row.case_id)
                    .ok_or_else(|| format!("case {} not in workload", row.case_id))?;
                cases.push(markit_mdbench_campaign::execute::ScheduledCase::EditWrite {
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
