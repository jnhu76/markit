//! `mdbench-campaign` — the #31 campaign-freeze CLI.
//!
//! ```text
//! mdbench-campaign <subcommand> [benchmark_root]
//!
//!   manifest-verify        verify the campaign manifest against live state
//!   machine-capture <out>  capture the CURRENT host as a machine manifest
//!   machine-verify         compare the current host to the frozen manifest
//!   schedule-generate      (re)generate the frozen schedule deterministically
//!   schedule-verify        verify the on-disk schedule
//!   schedule-determinism   regenerate twice + byte-compare (and vs disk)
//!   receipt-generate       write the campaign freeze receipt (freeze action)
//!   receipt-verify         verify every bound hash + spec id + schedule
//!   preflight [--surface S] [--session N]
//!                           non-measuring session preflight (fail-closed)
//!   gen-envelope-schema <out>
//!                           write protocol/campaign-observation-schema-v1.json
//!   smoke <out_dir> [--cases N] [--warmup N] [--measured N] [--inject-failure]
//!                           NON_RESEARCH fake-clock end-to-end plumbing smoke
//!   run-session --surface S --session N      (FUTURE primary timing; release
//!                                             profile only — NOT this task)
//!   run-attribution --surface S              (FUTURE attribution lane)
//! ```
//!
//! `run-session` / `run-attribution` exist so MARKIT-31-PRIMARY-
//! PERFORMANCE-RUN-1 has a frozen execution path; THIS task never runs
//! them over the campaign (task §50).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign::manifest::{
    CampaignManifest, ENVELOPE_SCHEMA_PATH, MACHINE_MANIFEST_PATH, SCHEDULE_MANIFEST_PATH,
};
use markit_mdbench_campaign::preflight::HostBinding;
use markit_mdbench_campaign::Surface;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(subcommand) = args.get(1).cloned() else {
        eprintln!("usage: mdbench-campaign <subcommand> [benchmark_root] (see --help)");
        return ExitCode::from(2);
    };
    let root = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
    let root = PathBuf::from(root);
    // Flags after the root.
    let flags = &args[3.min(args.len())..];

    let result: Result<bool, String> = match subcommand.as_str() {
        "manifest-verify" => cmd_manifest_verify(&root),
        "machine-capture" => cmd_machine_capture(&root, flags),
        "machine-verify" => cmd_machine_verify(&root),
        "schedule-generate" => cmd_schedule_generate(&root, flags),
        "schedule-verify" => cmd_schedule_verify(&root),
        "schedule-determinism" => cmd_schedule_determinism(&root),
        "receipt-generate" => cmd_receipt_generate(&root),
        "receipt-verify" => cmd_receipt_verify(&root),
        "preflight" => cmd_preflight(&root, flags),
        "gen-envelope-schema" => cmd_gen_envelope_schema(&root, flags),
        "smoke" => cmd_smoke(&root, flags),
        "run-session" => cmd_run_session(&root, flags),
        "run-attribution" => cmd_run_attribution(&root, flags),
        "--help" | "help" => {
            print_help();
            Ok(true)
        }
        other => Err(format!("unknown subcommand {other:?}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("mdbench-campaign: {error}");
            ExitCode::from(1)
        }
    }
}

fn print_help() {
    println!(
        "mdbench-campaign — #31 primary performance campaign freeze\n\
         subcommands: manifest-verify machine-capture machine-verify\n\
         schedule-generate schedule-verify schedule-determinism\n\
         receipt-generate receipt-verify preflight gen-envelope-schema\n\
         smoke run-session run-attribution"
    );
}

fn flag_value(flags: &[String], name: &str) -> Option<String> {
    flags
        .iter()
        .position(|flag| flag == name)
        .and_then(|index| flags.get(index + 1))
        .cloned()
}

fn has_flag(flags: &[String], name: &str) -> bool {
    flags.iter().any(|flag| flag == name)
}

fn cmd_manifest_verify(root: &Path) -> Result<bool, String> {
    let manifest = CampaignManifest::load(root)?;
    match manifest.verify(root) {
        Ok(()) => {
            println!("CAMPAIGN_MANIFEST_OK campaign_id={}", manifest.campaign_id);
            Ok(true)
        }
        Err(blockers) => {
            for blocker in blockers {
                eprintln!("MANIFEST_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

fn cmd_machine_capture(root: &Path, flags: &[String]) -> Result<bool, String> {
    let machine_id =
        flag_value(flags, "--machine-id").unwrap_or_else(|| "primary-bench-1".to_string());
    let notes = flag_value(flags, "--notes").unwrap_or_else(|| {
        "Governor/turbo/SMT inspected and recorded, never modified by tooling. Frequency policy is schedutil-based; turbo is not software-controllable on this host and is recorded as such. Affinity is applied per session; SMT and NUMA are recorded topology facts.".to_string()
    });
    let manifest = markit_mdbench_campaign::machine::capture_machine(&machine_id, root, &notes)?;
    let out_path = root.join(MACHINE_MANIFEST_PATH);
    if out_path.exists() && !has_flag(flags, "--force") {
        return Err(format!(
            "{} already exists (machine manifests are frozen; pass --force to overwrite during the freeze window)",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let text = toml::to_string_pretty(&manifest)
        .map_err(|e| format!("serialize machine manifest: {e}"))?;
    std::fs::write(&out_path, text).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "MACHINE_CAPTURE_OK machine_id={} cpu{} cores={} logical={} governor={}",
        manifest.machine_id,
        manifest.selected_cpu,
        manifest.physical_cores,
        manifest.logical_cpus,
        manifest.frequency_governor
    );
    Ok(true)
}

fn cmd_machine_verify(root: &Path) -> Result<bool, String> {
    let manifest = markit_mdbench_campaign::manifest::MachineManifest::load(root)?;
    match markit_mdbench_campaign::machine::match_current_host(&manifest, root) {
        Ok(()) => {
            println!("MACHINE_MATCH_OK machine_id={}", manifest.machine_id);
            Ok(true)
        }
        Err(blockers) => {
            for blocker in blockers {
                eprintln!("MACHINE_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

fn cmd_schedule_generate(root: &Path, flags: &[String]) -> Result<bool, String> {
    let manifest = CampaignManifest::load(root)?;
    manifest
        .verify(root)
        .map_err(|blockers| blockers.join("; "))?;
    let bytes = markit_mdbench_campaign::receipt::regenerate_schedule(root, &manifest)?;
    let out_path = root.join(SCHEDULE_MANIFEST_PATH);
    if out_path.exists() && !has_flag(flags, "--force") {
        return Err(format!(
            "{} already exists (schedules are frozen before timing; pass --force to overwrite during the freeze window)",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(&out_path, &bytes).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "SCHEDULE_GENERATE_OK rows={} bytes={}",
        bytes.iter().filter(|b| **b == b'\n').count(),
        bytes.len()
    );
    Ok(true)
}

fn cmd_schedule_verify(root: &Path) -> Result<bool, String> {
    match markit_mdbench_campaign::receipt::verify_schedule_on_disk(root) {
        Ok(()) => {
            println!("SCHEDULE_VERIFY_OK");
            Ok(true)
        }
        Err(blockers) => {
            for blocker in blockers {
                eprintln!("SCHEDULE_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

fn cmd_schedule_determinism(root: &Path) -> Result<bool, String> {
    let manifest = CampaignManifest::load(root)?;
    let first = markit_mdbench_campaign::receipt::regenerate_schedule(root, &manifest)?;
    let second = markit_mdbench_campaign::receipt::regenerate_schedule(root, &manifest)?;
    if first != second {
        return Err("two regenerations from the same frozen spec differ".to_string());
    }
    let on_disk = std::fs::read(root.join(SCHEDULE_MANIFEST_PATH))
        .map_err(|e| format!("read schedule: {e}"))?;
    if on_disk != first {
        return Err("on-disk schedule differs from deterministic regeneration".to_string());
    }
    let rows = markit_mdbench_campaign::receipt::verify_schedule_on_disk(root);
    if let Err(blockers) = rows {
        for blocker in blockers {
            eprintln!("SCHEDULE_BLOCKER {blocker}");
        }
        return Ok(false);
    }
    println!("SCHEDULE_DETERMINISM_PASS rows={}", first.len());
    Ok(true)
}

fn cmd_receipt_generate(root: &Path) -> Result<bool, String> {
    let receipt = markit_mdbench_campaign::receipt::generate_receipt(root)?;
    let out_path = root.join(markit_mdbench_campaign::manifest::CAMPAIGN_RECEIPT_PATH);
    if out_path.exists() {
        return Err(format!(
            "{} already exists (receipts are frozen; regenerate deliberately during the freeze window by removing it first)",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let text =
        serde_json::to_string_pretty(&receipt).map_err(|e| format!("serialize receipt: {e}"))?;
    std::fs::write(&out_path, text + "\n")
        .map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "CAMPAIGN_RECEIPT_GENERATED spec_id={} bound_artifacts={}",
        receipt.campaign_spec_id,
        receipt.artifacts.len()
    );
    Ok(true)
}

fn cmd_receipt_verify(root: &Path) -> Result<bool, String> {
    match markit_mdbench_campaign::receipt::verify_receipt(root) {
        Ok(()) => {
            let receipt = markit_mdbench_campaign::receipt::CampaignReceipt::load(root)?;
            println!(
                "CAMPAIGN_RECEIPT_OK spec_id={} artifacts={}",
                receipt.campaign_spec_id,
                receipt.artifacts.len()
            );
            Ok(true)
        }
        Err(blockers) => {
            for blocker in blockers {
                eprintln!("RECEIPT_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

fn cmd_preflight(root: &Path, flags: &[String]) -> Result<bool, String> {
    let surface = match flag_value(flags, "--surface") {
        Some(value) => Some(Surface::parse(&value)?),
        None => None,
    };
    let session = match flag_value(flags, "--session") {
        Some(value) => Some(
            value
                .parse::<u32>()
                .map_err(|e| format!("--session: {e}"))?,
        ),
        None => None,
    };
    let report =
        markit_mdbench_campaign::preflight::preflight(root, HostBinding::Enforce, surface, session);
    println!(
        "{}",
        markit_mdbench_campaign::preflight::preflight_json(&report, HostBinding::Enforce)
    );
    Ok(report.pass)
}

fn cmd_gen_envelope_schema(root: &Path, flags: &[String]) -> Result<bool, String> {
    let out = flag_value(flags, "--out").unwrap_or_else(|| ENVELOPE_SCHEMA_PATH.to_string());
    let schema = schemars::schema_for!(markit_mdbench_campaign::execute::CampaignObservationV1);
    let mut text = serde_json::to_string_pretty(&schema)
        .map_err(|e| format!("serialize envelope schema: {e}"))?;
    text.push('\n');
    let out_path = root.join(&out);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(&out_path, text).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!("wrote {}", out_path.display());
    Ok(true)
}

fn cmd_smoke(root: &Path, flags: &[String]) -> Result<bool, String> {
    let Some(out_dir) = flag_value(flags, "--out") else {
        return Err("smoke requires --out <dir> (must be OUTSIDE results/raw)".to_string());
    };
    let out_path = PathBuf::from(&out_dir);
    // Guard: smoke output must never land under the primary raw path.
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let raw_root = canonical_root.join("results/raw");
    let canonical_out = out_path.canonicalize().unwrap_or_else(|_| out_path.clone());
    if canonical_out.starts_with(&raw_root) {
        return Err(format!(
            "smoke output {} is under the primary raw result path",
            out_path.display()
        ));
    }
    let options = markit_mdbench_campaign::smoke::SmokeOptions {
        cases_per_surface: flag_value(flags, "--cases")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        warmup: flag_value(flags, "--warmup")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),
        measured: flag_value(flags, "--measured")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),
        inject_failure: has_flag(flags, "--inject-failure"),
    };
    let report = markit_mdbench_campaign::smoke::run_smoke(root, &out_path, &options)?;
    println!(
        "{} timing_rows={} attribution_rows={} failure_propagated={}",
        report.verdict, report.timing_rows, report.attribution_rows, report.failure_propagated
    );
    println!("NON_RESEARCH_RESULT");
    Ok(true)
}

// ---------------------------------------------------------------------------
// FUTURE primary-timing entry points (MARKIT-31-PRIMARY-PERFORMANCE-RUN-1).
// Frozen now, NOT executed by this task (task §50).
// ---------------------------------------------------------------------------

fn require_release_profile() -> Result<(), String> {
    let build = markit_mdbench_runner::current_build_identity();
    if build.build_profile_id != markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID {
        return Err(format!(
            "primary campaign execution requires the frozen release profile {}; this binary was built with {}",
            markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID,
            build.build_profile_id
        ));
    }
    Ok(())
}

fn cmd_run_session(root: &Path, flags: &[String]) -> Result<bool, String> {
    require_release_profile()?;
    let surface =
        Surface::parse(&flag_value(flags, "--surface").ok_or("run-session requires --surface")?)?;
    let session = flag_value(flags, "--session")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or("run-session requires --session N")?;

    // Full fail-closed preflight BEFORE any session data is collected.
    let report = markit_mdbench_campaign::preflight::preflight(
        root,
        HostBinding::Enforce,
        Some(surface),
        Some(session),
    );
    if !report.pass {
        return Err(format!("preflight blocked: {:?}", report.blockers));
    }
    // Pin the worker to the frozen single CPU (task §25).
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
    let run_id = markit_mdbench_campaign::identity::run_id(
        &spec_id,
        &build.runner_git_commit,
        &machine_digest,
        &build,
    );
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!("{}#{}", MACHINE_MANIFEST_PATH, machine.machine_id),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_TIMING,
        non_research: false,
    };

    // Raw layout (task §39): append/create-only; an existing file for
    // this session is a hard error.
    let out_path = root
        .join("results/raw")
        .join(&spec_id)
        .join(&run_id)
        .join("timing")
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

    let session_id =
        markit_mdbench_campaign::identity::session_id(&spec_id, surface.as_str(), session);
    let session_seed = markit_mdbench_campaign::identity::session_seed(
        manifest.seed.value,
        surface.as_str(),
        session,
    );
    let subset: Vec<&markit_mdbench_campaign::schedule::ScheduleRow> = schedule
        .iter()
        .filter(|row| row.surface == surface.as_str() && row.session_ordinal == session)
        .collect();
    let cases = schedule_rows_to_cases(&workload, &subset)?;
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
    let clock = markit_mdbench_campaign::execute::CampaignClock::Real(
        markit_mdbench_instrumentation::InstantClock::new(),
    );
    let outcome = executor.run(&cases, &clock, &mut file)?;
    use std::io::Write;
    file.flush().map_err(|e| format!("flush: {e}"))?;
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Completed { observations, .. } => {
            println!("SESSION_COMPLETE observations={observations}");
            Ok(true)
        }
        markit_mdbench_campaign::execute::SessionOutcome::Invalid { reason, .. } => {
            eprintln!("PRIMARY_CAMPAIGN_INVALID {reason}");
            Ok(false)
        }
    }
}

fn cmd_run_attribution(root: &Path, flags: &[String]) -> Result<bool, String> {
    require_release_profile()?;
    let surface = Surface::parse(
        &flag_value(flags, "--surface").ok_or("run-attribution requires --surface")?,
    )?;
    let report = markit_mdbench_campaign::preflight::preflight(
        root,
        HostBinding::Enforce,
        Some(surface),
        None,
    );
    if !report.pass {
        return Err(format!("preflight blocked: {:?}", report.blockers));
    }
    let machine = markit_mdbench_campaign::manifest::MachineManifest::load(root)?;
    markit_mdbench_campaign::machine::apply_affinity(machine.selected_cpu)?;

    let manifest = CampaignManifest::load(root)?;
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(root)?;
    let binding = markit_mdbench_campaign::receipt::build_spec_binding(root, &manifest)?;
    let spec_id = markit_mdbench_campaign::identity::campaign_spec_id(&binding);
    let machine_digest = markit_mdbench_campaign::sha256_file(&root.join(MACHINE_MANIFEST_PATH))?;
    let build = markit_mdbench_runner::current_build_identity();
    let run_id = markit_mdbench_campaign::identity::run_id(
        &spec_id,
        &build.runner_git_commit,
        &machine_digest,
        &build,
    );
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!("{}#{}", MACHINE_MANIFEST_PATH, machine.machine_id),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_ATTRIBUTION,
        non_research: false,
    };
    let out_path = root
        .join("results/raw")
        .join(&spec_id)
        .join(&run_id)
        .join("attribution")
        .join(format!("{}.jsonl", surface.as_str()));
    if out_path.exists() {
        return Err(format!(
            "{} already exists: finalized raw files are never overwritten",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    // Deterministic attribution order: sorted by (case id, horse).
    let session_id =
        markit_mdbench_campaign::execute::attribution_session_id(&spec_id, surface.as_str());
    let mut rows: Vec<markit_mdbench_campaign::schedule::ScheduleRow> = {
        let schedule_bytes = std::fs::read(root.join(SCHEDULE_MANIFEST_PATH))
            .map_err(|e| format!("read schedule: {e}"))?;
        markit_mdbench_campaign::schedule::schedule_from_jsonl(&schedule_bytes)?
            .into_iter()
            .filter(|row| row.surface == surface.as_str() && row.session_ordinal == 0)
            .collect()
    };
    rows.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    let subset: Vec<&markit_mdbench_campaign::schedule::ScheduleRow> = rows.iter().collect();
    let cases = schedule_rows_to_cases(&workload, &subset)?;
    let executor = markit_mdbench_campaign::execute::SessionExecutor {
        identity: &identity,
        surface,
        session_ordinal: None,
        session_id,
        session_seed: manifest.seed.value,
        build_identity: build,
        warmup: 0,
        measured: 0,
        observations: 0,
        warmup_rows: 0,
        measured_rows: 0,
        poison_correctness: false,
    };
    let mut file = std::fs::File::create(&out_path)
        .map_err(|e| format!("create {}: {e}", out_path.display()))?;
    let outcome = executor.run_attribution(&cases, &mut file)?;
    use std::io::Write;
    file.flush().map_err(|e| format!("flush: {e}"))?;
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Completed { observations, .. } => {
            println!("ATTRIBUTION_COMPLETE observations={observations}");
            Ok(true)
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
