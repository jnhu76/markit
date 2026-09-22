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
//!   preflight [--timing S --session N | --attribution S]
//!                           non-measuring session preflight (fail-closed);
//!                           no flags = the whole campaign (All scope)
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
//! them over the campaign (task §50). Both bind the executable SHA256
//! into the `RunId` and verify the finalized raw file before writing a
//! run receipt or declaring the session complete.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign::finalize::{RawFileSummary, RawLane};
use markit_mdbench_campaign::manifest::{
    CampaignManifest, ENVELOPE_SCHEMA_PATH, MACHINE_MANIFEST_PATH, SCHEDULE_MANIFEST_PATH,
};
use markit_mdbench_campaign::preflight::{HostBinding, PreflightScope};
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
    println!("SCHEDULE_DETERMINISM_PASS bytes={}", first.len());
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
    let scope = parse_preflight_scope(flags)?;
    let report = markit_mdbench_campaign::preflight::preflight(root, HostBinding::Enforce, scope);
    println!(
        "{}",
        markit_mdbench_campaign::preflight::preflight_json(&report, HostBinding::Enforce)
    );
    Ok(report.pass)
}

/// The preflight scope is EXPLICIT: a timing session, an attribution
/// lane, and the whole campaign check different identity sets, so an
/// unstated scope must never be guessed (it used to be inferred from
/// which flags happened to be present).
fn parse_preflight_scope(flags: &[String]) -> Result<PreflightScope, String> {
    let timing = flag_value(flags, "--timing");
    let attribution = flag_value(flags, "--attribution");
    let session = flag_value(flags, "--session");
    if flag_value(flags, "--surface").is_some() {
        return Err(
            "--surface is not a preflight scope flag; use --timing <surface> --session <n>, \
             --attribution <surface>, or no flags for the whole campaign"
                .to_string(),
        );
    }
    match (timing, attribution, session) {
        (None, None, None) => Ok(PreflightScope::All),
        (Some(_), Some(_), _) => {
            Err("--timing and --attribution are mutually exclusive".to_string())
        }
        (Some(timing), None, None) => Err(format!(
            "--timing {timing} needs --session <n>: one timing session is preflighted at a time"
        )),
        (Some(timing), None, Some(session)) => Ok(PreflightScope::Timing {
            surface: Surface::parse(&timing)?,
            session: session
                .parse::<u32>()
                .map_err(|e| format!("--session: {e}"))?,
        }),
        (None, Some(attribution), None) => Ok(PreflightScope::Attribution {
            surface: Surface::parse(&attribution)?,
        }),
        (None, Some(_), Some(_)) => Err(
            "--session does not apply to --attribution (the lane has no timing session)"
                .to_string(),
        ),
        (None, None, Some(_)) => Err("--session requires --timing <surface>".to_string()),
    }
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
    // Guard: smoke output must never land under the primary raw path
    // (the library-level guard in run_smoke enforces the same rule).
    if markit_mdbench_campaign::smoke::is_under_primary_raw_root(&out_path, root) {
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
        PreflightScope::Timing { surface, session },
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
    // The exact binary is part of the run identity: a rebuilt executable
    // (same commit/toolchain/profile) must not continue this run.
    let executable_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    let run_id = markit_mdbench_campaign::identity::run_id(
        &spec_id,
        &build.runner_git_commit,
        &machine_digest,
        &build,
        &executable_sha256,
    );
    // Raw layout (task §39): append/create-only; an existing file for
    // this session is a hard error, and one spec never spans two runs.
    let spec_root = root.join("results/raw").join(&spec_id);
    ensure_single_run_identity(&spec_root, &run_id)?;
    let out_path = spec_root
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

    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!("{}#{}", MACHINE_MANIFEST_PATH, machine.machine_id),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_TIMING,
        non_research: false,
    };
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
    // The expectation is built from the SAME schedule rows the executor
    // consumes, before anything runs.
    let expectation = markit_mdbench_campaign::finalize::expectation_from_schedule(
        &subset,
        &spec_id,
        &run_id,
        &session_id,
        surface,
        RawLane::Timing {
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
    let clock = markit_mdbench_campaign::execute::CampaignClock::Real(
        markit_mdbench_instrumentation::InstantClock::new(),
    );
    let outcome = executor.run(&cases, &clock, &mut file)?;
    use std::io::Write;
    file.flush().map_err(|e| format!("flush: {e}"))?;
    drop(file);
    // Finalization (task §26, §39): the raw file is re-read and checked
    // against the frozen contract; only a passing file gets a receipt.
    let finalized = finalize_raw_file(&out_path, &expectation, &executable_sha256)?;
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Completed { observations, .. }
            if finalized =>
        {
            println!(
                "SESSION_COMPLETE observations={observations} lane=timing surface={} session={session}",
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

/// One `CampaignSpecId` carries exactly ONE `RunId`.
///
/// A rebuilt executable, a changed machine manifest, or a different
/// runner commit all produce a different `RunId`. A campaign run must
/// never be silently split across binaries — the operator either keeps
/// the frozen binary or archives the previous run directory
/// deliberately.
fn ensure_single_run_identity(spec_root: &Path, run_id: &str) -> Result<(), String> {
    if !spec_root.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(spec_root).map_err(|e| format!("read {}: {e}", spec_root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", spec_root.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name != run_id {
            return Err(format!(
                "{} already holds raw data for run {name}, but this binary/commit/machine binds run \
                 {run_id}: a rebuilt executable is a NEW RunId, and one campaign run is never split \
                 across two binaries. Archive the existing run directory (a deliberate act) before \
                 starting a new campaign run.",
                spec_root.display()
            ));
        }
    }
    Ok(())
}

/// Read a finalized raw file back and check it against the frozen
/// contract. On success the verified facts enter the run receipt; on
/// failure the file is retained as evidence and marked invalid — it
/// NEVER gets a success receipt.
fn finalize_raw_file(
    raw_path: &Path,
    expectation: &markit_mdbench_campaign::finalize::RawFileExpectation,
    run_executable_sha256: &str,
) -> Result<bool, String> {
    match markit_mdbench_campaign::finalize::verify_raw_file(raw_path, expectation) {
        Ok(summary) => {
            write_run_receipt(raw_path, &summary, run_executable_sha256)?;
            println!(
                "FINAL_RAW_FILE_PASS file={} rows={} sha256={}",
                raw_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default(),
                summary.rows,
                summary.sha256
            );
            Ok(true)
        }
        Err(blockers) => {
            let marker = write_invalid_marker(raw_path, &blockers)?;
            eprintln!(
                "FINAL_RAW_FILE_INVALID file={} detail={}",
                raw_path.display(),
                marker.display()
            );
            for blocker in &blockers {
                eprintln!("RAW_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

/// Write `<raw file>.invalid.json`: the failure evidence marker for a raw
/// file that did not pass finalization. Written once, never overwritten.
fn write_invalid_marker(raw_path: &Path, blockers: &[String]) -> Result<PathBuf, String> {
    let marker_path = raw_path.with_extension("jsonl.invalid.json");
    if marker_path.exists() {
        return Ok(marker_path);
    }
    let marker = serde_json::json!({
        "schema": "run-file-invalid-v1",
        "verdict": "PRIMARY_CAMPAIGN_INVALID",
        "file": raw_path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "producer_pid": std::process::id(),
        "blockers": blockers,
    });
    std::fs::write(&marker_path, format!("{marker}\n"))
        .map_err(|e| format!("write {}: {e}", marker_path.display()))?;
    Ok(marker_path)
}

/// Write `<raw file>.receipt.json` binding the VERIFIED finalized raw
/// file: SHA256, row counts, first/last observation id, campaign/run/
/// session identity (task §26/§39).
fn write_run_receipt(
    raw_path: &Path,
    summary: &RawFileSummary,
    run_executable_sha256: &str,
) -> Result<(), String> {
    // Process identity: the frozen session semantics require a FRESH
    // worker process per session (task §11); recording pid + kernel
    // start-time makes that auditable after the fact.
    let producer_pid = std::process::id();
    let producer_start_ticks = std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|text| {
            // Field 22 (1-indexed) is starttime; the comm field may
            // contain spaces, so split after the last ')'.
            let after_comm = text.rsplit_once(')').map(|(_, rest)| rest.to_string())?;
            after_comm
                .split_whitespace()
                .nth(19)
                .map(|value| value.to_string())
        });
    // The receipt is written by the binary that produced the rows; if
    // that binary changed mid-run the RunId would name a different
    // executable, which is a hard error rather than a recorded footnote.
    let executable_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    if executable_sha256 != run_executable_sha256 {
        return Err(format!(
            "the receipt-writing executable {executable_sha256} is not the executable bound into the \
             RunId ({run_executable_sha256})"
        ));
    }
    let receipt = serde_json::json!({
        "schema": "run-file-receipt-v1",
        "verdict": summary.verdict,
        "file": raw_path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "sha256": summary.sha256,
        "row_count": summary.rows,
        "warmup_rows": summary.warmup_rows,
        "measured_rows": summary.measured_rows,
        "attribution_rows": summary.attribution_rows,
        "expected_rows": summary.expected_rows,
        "unique_observation_ids": summary.unique_observation_ids,
        "case_count": summary.case_count,
        "lane": summary.lane,
        "surface": summary.surface,
        "session_ordinal": summary.session_ordinal,
        "campaign_spec_id": summary.campaign_spec_id,
        "run_id": summary.run_id,
        "session_id": summary.session_id,
        "first_observation_id": summary.first_observation_id,
        "last_observation_id": summary.last_observation_id,
        "producer_pid": producer_pid,
        "producer_start_ticks": producer_start_ticks,
        "executable_sha256": executable_sha256,
    });
    let receipt_path = raw_path.with_extension("jsonl.receipt.json");
    if receipt_path.exists() {
        return Err(format!(
            "{} already exists: run receipts are written once",
            receipt_path.display()
        ));
    }
    std::fs::write(&receipt_path, format!("{receipt}\n"))
        .map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
    Ok(())
}

fn cmd_run_attribution(root: &Path, flags: &[String]) -> Result<bool, String> {
    require_release_profile()?;
    let surface = Surface::parse(
        &flag_value(flags, "--surface").ok_or("run-attribution requires --surface")?,
    )?;
    // The attribution lane has no timing session; the scope says so
    // explicitly instead of being inferred from a missing --session.
    let report = markit_mdbench_campaign::preflight::preflight(
        root,
        HostBinding::Enforce,
        PreflightScope::Attribution { surface },
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
    let executable_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    let run_id = markit_mdbench_campaign::identity::run_id(
        &spec_id,
        &build.runner_git_commit,
        &machine_digest,
        &build,
        &executable_sha256,
    );
    let spec_root = root.join("results/raw").join(&spec_id);
    ensure_single_run_identity(&spec_root, &run_id)?;
    let out_path = spec_root
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
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: run_id.clone(),
        machine_environment_ref: format!("{}#{}", MACHINE_MANIFEST_PATH, machine.machine_id),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_ATTRIBUTION,
        non_research: false,
    };
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
    let expectation = markit_mdbench_campaign::finalize::expectation_from_schedule(
        &subset,
        &spec_id,
        &run_id,
        &session_id,
        surface,
        RawLane::Attribution,
        manifest.sessions.warmup_iterations,
        manifest.sessions.measured_iterations,
    )?;
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
    drop(file);
    // Finalization (task §26, §39): verify before any completion verdict.
    let finalized = finalize_raw_file(&out_path, &expectation, &executable_sha256)?;
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Completed { observations, .. }
            if finalized =>
        {
            println!(
                "ATTRIBUTION_COMPLETE observations={observations} surface={}",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    /// The preflight scope is never inferred from whichever flags happen
    /// to be present: an unattributable request must not resolve to a
    /// scope whose check it does not want.
    #[test]
    fn preflight_scope_must_be_stated_explicitly() {
        assert_eq!(parse_preflight_scope(&[]).unwrap(), PreflightScope::All);
        assert_eq!(
            parse_preflight_scope(&flags(&["--timing", "edit_write", "--session", "2"])).unwrap(),
            PreflightScope::Timing {
                surface: Surface::EditWrite,
                session: 2
            }
        );
        assert_eq!(
            parse_preflight_scope(&flags(&["--attribution", "clean_state"])).unwrap(),
            PreflightScope::Attribution {
                surface: Surface::CleanState
            }
        );
        for ambiguous in [
            &["--surface", "clean_state"][..],
            &["--timing", "clean_state"][..],
            &["--session", "1"][..],
            &["--attribution", "edit_write", "--session", "0"][..],
            &[
                "--timing",
                "edit_write",
                "--session",
                "0",
                "--attribution",
                "clean_state",
            ][..],
            &["--timing", "edit_write", "--session", "not-a-number"][..],
        ] {
            assert!(
                parse_preflight_scope(&flags(ambiguous)).is_err(),
                "{ambiguous:?} must not silently resolve to a scope"
            );
        }
    }

    /// One CampaignSpecId carries exactly one RunId, so a rebuilt binary
    /// cannot quietly continue an existing run.
    #[test]
    fn one_spec_carries_one_run_identity() {
        let dir = std::env::temp_dir().join(format!("mdbench-runid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // A spec with no raw data yet has nothing to contradict.
        assert!(ensure_single_run_identity(&dir, "run-a").is_ok());
        std::fs::create_dir_all(dir.join("run-a")).unwrap();
        assert!(ensure_single_run_identity(&dir, "run-a").is_ok());
        let error = ensure_single_run_identity(&dir, "run-b").expect_err("run-b must be refused");
        assert!(error.contains("never split"), "got {error}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
