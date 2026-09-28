//! `mdbench-pmu` — the minimum mechanism-neutral PMU diagnostic driver
//! for MARKIT-76-SIX-HORSE-PMU-EXPLANATION-v1 (#76 PMU campaign order).
//!
//! EXPLANATORY_DIAGNOSTIC_ONLY: not a new horse, not a replacement
//! campaign runner, not a primary timing runner. Horse-level isolation
//! is structural: one `run` invocation executes exactly ONE
//! `case × horse × event group × repetition` from the frozen PMU
//! schedule — one fresh process, one counted region — and the executor
//! NEVER wraps `mdbench-campaign run-session` with anything.
//!
//! ```text
//! mdbench-pmu probe-events                    (NON_RESEARCH capability probe)
//! mdbench-pmu schedule <root> --panel <path> --out <path>
//! mdbench-pmu run <root> --panel <path> --schedule <path> --entry <n>
//!                [--raw-root <dir>]
//! mdbench-pmu finalize <root> --panel <path> --schedule <path>
//!                [--raw-root <dir>]
//! ```
//!
//! Evidence namespace: `results/diagnostics/pmu-v1/raw/<pmu-run-id>/`
//! — PMU evidence can never be written into the primary `results/raw/`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign::pmu::events;
use markit_mdbench_campaign::pmu::panel::{self, PMU_PANEL_MANIFEST_SHA256};
use markit_mdbench_campaign::pmu::panel::{PMU_SEED, PMU_STUDY_ID};
use markit_mdbench_campaign::pmu::perfcount::EventCounterGroup;
use markit_mdbench_campaign::pmu::region;
use markit_mdbench_campaign::pmu::schedule::{self, PmuScheduleEntry, PmuScheduleHeader};
use markit_mdbench_campaign::pmu::schema::{
    self, CorrectnessV1, EventCountV1, HostIdentityV1, PmuObservationV1, ToolchainV1,
    PMU_OBSERVATION_SCHEMA, QUAL_COUNTER_UNAVAILABLE, QUAL_INVALID, QUAL_QUALIFIED,
    QUAL_UNQUALIFIED_MULTIPLEXED,
};
use markit_mdbench_campaign::workload::{
    load_campaign_workload_filtered, CleanStateCase, EditWriteCase,
};
use markit_mdbench_campaign::Surface;
use markit_mdbench_common::{Mechanism, ResultChecksum};
use markit_mdbench_oracle::{validate_normalized, NormalizeV1, ReferenceOracle};

/// Default PMU evidence root (diagnostics namespace, never primary).
const DEFAULT_RAW_ROOT: &str = "results/diagnostics/pmu-v1/raw";

/// Frozen LOW_COUNT threshold for metric derivation flags.
const LOW_COUNT_THRESHOLD: u64 = 100;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(subcommand) = args.get(1).cloned() else {
        eprintln!("usage: mdbench-pmu probe-events | schedule | run | finalize (see header docs)");
        return ExitCode::from(2);
    };
    let root = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".to_string()));
    let flags = &args[3.min(args.len())..];
    let result: Result<bool, String> = match subcommand.as_str() {
        "probe-events" => cmd_probe_events(),
        "schedule" => cmd_schedule(&root, flags),
        "run" => cmd_run(&root, flags),
        "finalize" => cmd_finalize(&root, flags),
        "--help" | "help" => {
            println!("mdbench-pmu — PMU explanatory diagnostic driver (#76)");
            Ok(true)
        }
        other => Err(format!("unknown subcommand {other:?}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("PMU_DRIVER_ERROR {message}");
            ExitCode::from(2)
        }
    }
}

fn flag_value(flags: &[String], name: &str) -> Option<String> {
    flags
        .iter()
        .position(|flag| flag == name)
        .and_then(|index| flags.get(index + 1))
        .cloned()
}

fn require_release_profile() -> Result<(), String> {
    let build = markit_mdbench_runner::current_build_identity();
    if build.build_profile_id != markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID {
        return Err(format!(
            "PMU horse observation requires the frozen release profile {}; this binary was built \
             with {}",
            markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID,
            build.build_profile_id
        ));
    }
    Ok(())
}

/// Why one observation failed before producing a qualified row.
/// Setup failures are MECHANISM-LEVEL events (reference gate, fresh
/// pre-state construction) and must be recorded as INVALID; counter
/// failures are instrumentation events. Never silently dropped.
enum RunError {
    Setup(String),
    Counter(String),
}

impl RunError {
    fn counter(message: String) -> Self {
        RunError::Counter(message)
    }
}

fn hostname_only() -> String {
    let mut hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_else(|_| "unavailable".to_string());
    hostname.retain(|c| c != '\n');
    hostname
}

fn host_identity(pinned_cpu: u32) -> HostIdentityV1 {
    let read =
        |path: &str| std::fs::read_to_string(path).unwrap_or_else(|_| "unavailable".to_string());
    let mut hostname = read("/proc/sys/kernel/hostname");
    hostname.retain(|c| c != '\n');
    let mut kernel = read("/proc/sys/kernel/osrelease");
    kernel.retain(|c| c != '\n');
    let mut paranoid = read("/proc/sys/kernel/perf_event_paranoid");
    paranoid.retain(|c| c != '\n');
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("model name")
                    .map(|rest| rest.trim_start_matches(":\t").to_string())
            })
        })
        .unwrap_or_else(|| "unavailable".to_string());
    HostIdentityV1 {
        hostname,
        kernel,
        cpu,
        perf_event_paranoid: paranoid,
        pinned_cpu,
    }
}

fn toolchain_identity() -> ToolchainV1 {
    let build = markit_mdbench_runner::current_build_identity();
    ToolchainV1 {
        rustc: build.rustc,
        cargo: format!("cargo-{}", build.build_profile_id),
        build_profile: build.build_profile_id,
    }
}

// ---------------------------------------------------------------------------
// probe-events: NON_RESEARCH capability probe of the frozen groups.
// ---------------------------------------------------------------------------

fn cmd_probe_events() -> Result<bool, String> {
    events::table_invariants()?;
    println!("PMU_PROBE_NON_RESEARCH study={PMU_STUDY_ID}");
    for group in events::all_groups() {
        let mut counters = match EventCounterGroup::open(group.events, group.standalone) {
            Ok(counters) => counters,
            Err(error) => {
                println!("PMU_PROBE_GROUP {} OPEN_FAILED {}", group.id, error);
                continue;
            }
        };
        counters
            .begin()
            .map_err(|e| format!("begin {}: {e}", group.id))?;
        // Deterministic in-process busy region (black_boxed so it is not
        // optimized away).
        let mut accumulator: u64 = 0;
        for i in 0u64..2_000_000 {
            accumulator = accumulator.wrapping_add(i.wrapping_mul(2654435761));
        }
        std::hint::black_box(&mut accumulator);
        let readings = counters
            .end()
            .map_err(|e| format!("end {}: {e}", group.id))?;
        let parts: Vec<String> = group
            .events
            .iter()
            .zip(readings)
            .map(|(def, reading)| {
                format!(
                    "{}={}(enabled={} running={} frac={:.4})",
                    def.alias,
                    reading.value,
                    reading.time_enabled,
                    reading.time_running,
                    reading.running_fraction().unwrap_or(f64::NAN)
                )
            })
            .collect();
        println!("PMU_PROBE_GROUP {} {}", group.id, parts.join(" "));
    }
    println!("PMU_PROBE_COMPLETE NON_RESEARCH_RESULT");
    Ok(true)
}

// ---------------------------------------------------------------------------
// schedule: materialize + verify + write the frozen PMU schedule.
// ---------------------------------------------------------------------------

fn cmd_schedule(root: &Path, flags: &[String]) -> Result<bool, String> {
    let panel_path =
        PathBuf::from(flag_value(flags, "--panel").ok_or("schedule requires --panel <path>")?);
    let out_path =
        PathBuf::from(flag_value(flags, "--out").ok_or("schedule requires --out <path>")?);
    let panel = panel::load_verified_panel(&panel_path, PMU_PANEL_MANIFEST_SHA256)?;
    let (header, entries) = schedule::materialize(&panel)?;
    let expected = schedule::expected_entries(panel.cells.len());
    if entries.len() as u32 != expected {
        return Err(format!(
            "schedule entries {} != expected {expected}",
            entries.len()
        ));
    }
    schedule::entries_match_panel(&entries, &panel)?;
    let bytes = schedule::schedule_file_bytes(&header, &entries);
    // Round-trip verification BEFORE the file exists.
    let (parsed_header, parsed_entries) = schedule::verify_schedule_file(&bytes)?;
    if parsed_header != header || parsed_entries != entries {
        return Err("schedule round-trip mismatch".to_string());
    }
    if out_path.exists() {
        return Err(format!(
            "refusing to overwrite existing schedule {}",
            out_path.display()
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::write(&out_path, &bytes).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "PMU_SCHEDULE_WRITTEN file={} entries={} sha256={} seed={:#x} raw_root_note={}",
        out_path.display(),
        entries.len(),
        header.schedule_sha256,
        PMU_SEED,
        root.display()
    );
    Ok(true)
}

// ---------------------------------------------------------------------------
// run: ONE schedule entry = one fresh process = one counted region.
// ---------------------------------------------------------------------------

struct LoadedEntry {
    header: PmuScheduleHeader,
    entry: PmuScheduleEntry,
    clean: Option<CleanStateCase>,
    edit: Option<EditWriteCase>,
}

fn load_entry(
    root: &Path,
    panel_path: &str,
    schedule_path: &str,
    ordinal: u32,
) -> Result<LoadedEntry, String> {
    let panel = panel::load_verified_panel(Path::new(panel_path), PMU_PANEL_MANIFEST_SHA256)?;
    let bytes =
        std::fs::read(schedule_path).map_err(|e| format!("read schedule {schedule_path}: {e}"))?;
    let (header, entries) = schedule::verify_schedule_file(&bytes)?;
    schedule::entries_match_panel(&entries, &panel)?;
    // The schedule file must BE the frozen materialization of this panel
    // under the frozen seed — not merely a self-consistent file. A
    // trimmed or rebuilt schedule can never pass this gate.
    let (expected_header, expected_entries) = schedule::materialize(&panel)?;
    if header != expected_header || entries != expected_entries {
        return Err(
            "schedule file is not the frozen materialization of the verified panel".to_string(),
        );
    }
    let entry = schedule::entry_by_ordinal(&entries, ordinal)?.clone();
    // Materialize ONLY this entry's case (filtered loader; identical
    // per-case identity semantics as the full load).
    let mut filter = BTreeSet::new();
    filter.insert(entry.payload_id.clone());
    let workload = load_campaign_workload_filtered(root, &filter)?;
    let clean = workload
        .clean_state
        .into_iter()
        .find(|case| case.payload_id == entry.payload_id)
        .map(|case| {
            if case.case_id_hex != entry.case_id {
                return Err(format!(
                    "clean case id drift for {}: workload {} != schedule {}",
                    entry.slot, case.case_id_hex, entry.case_id
                ));
            }
            if entry.surface != Surface::CleanState.as_str() {
                return Err(format!(
                    "surface drift for {}: schedule {:?} but case is CLEAN_STATE",
                    entry.slot, entry.surface
                ));
            }
            Ok(case)
        })
        .transpose()?;
    let edit = workload
        .edit_write
        .into_iter()
        .find(|case| case.payload_id == entry.payload_id)
        .map(|case| {
            if case.case_id_hex != entry.case_id {
                return Err(format!(
                    "edit case id drift for {}: workload {} != schedule {}",
                    entry.slot, case.case_id_hex, entry.case_id
                ));
            }
            if entry.surface != Surface::EditWrite.as_str() {
                return Err(format!(
                    "surface drift for {}: schedule {:?} but case is EDIT_WRITE",
                    entry.slot, entry.surface
                ));
            }
            Ok(case)
        })
        .transpose()?;
    if clean.is_none() && edit.is_none() {
        return Err(format!(
            "panel case {} ({}) not materialized by the filtered workload loader",
            entry.slot, entry.payload_id
        ));
    }
    Ok(LoadedEntry {
        header,
        entry,
        clean,
        edit,
    })
}

fn cmd_run(root: &Path, flags: &[String]) -> Result<bool, String> {
    require_release_profile()?;
    let panel_path = flag_value(flags, "--panel").ok_or("run requires --panel")?;
    let schedule_path = flag_value(flags, "--schedule").ok_or("run requires --schedule")?;
    let ordinal: u32 = flag_value(flags, "--entry")
        .and_then(|v| v.parse().ok())
        .ok_or("run requires --entry <n>")?;
    let raw_root = PathBuf::from(
        flag_value(flags, "--raw-root").unwrap_or_else(|| DEFAULT_RAW_ROOT.to_string()),
    );
    let loaded = load_entry(root, &panel_path, &schedule_path, ordinal)?;
    let entry = &loaded.entry;

    // Frozen single-CPU affinity (same benchmark-free selection rule as
    // the primary campaign).
    let (cpu, _core, _siblings) = markit_mdbench_campaign::machine::select_primary_cpu()?;
    markit_mdbench_campaign::machine::apply_affinity(cpu)?;

    let driver_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    let mut hostname = hostname_only();
    hostname.truncate(64);
    let run_id = schema::pmu_run_id(
        &driver_sha256,
        PMU_PANEL_MANIFEST_SHA256,
        &loaded.header.schedule_sha256,
        &hostname,
    );
    let observation_id = schema::pmu_observation_id(
        &run_id,
        ordinal,
        &entry.horse,
        &entry.event_group,
        entry.repetition,
    );

    let pinned_cpu = cpu;
    let outcome: Result<PmuObservationV1, RunError> = dispatch_region(
        &loaded,
        entry,
        &run_id,
        &observation_id,
        &driver_sha256,
        &loaded.header,
        pinned_cpu,
    );

    let observation = match outcome {
        Ok(observation) => observation,
        Err(error) => {
            let (qualification, failure_label, execution, correctness_label) = match &error {
                RunError::Setup(message) => (
                    QUAL_INVALID,
                    format!("setup failed: {message}"),
                    "failed",
                    "not_checked",
                ),
                RunError::Counter(message) => (
                    QUAL_COUNTER_UNAVAILABLE,
                    format!("counter instrumentation unavailable: {message}"),
                    "instrumentation_unavailable",
                    "not_checked",
                ),
            };
            // The observation is retained (never silently dropped); a
            // mechanism-level setup failure is INVALID, an instrumentation
            // failure is COUNTER_UNAVAILABLE.
            PmuObservationV1 {
                schema: PMU_OBSERVATION_SCHEMA.to_string(),
                pmu_study_id: PMU_STUDY_ID.to_string(),
                observation_id,
                pmu_run_id: run_id.clone(),
                panel_manifest_sha256: PMU_PANEL_MANIFEST_SHA256.to_string(),
                pmu_schedule_sha256: loaded.header.schedule_sha256.clone(),
                driver_sha256,
                primary_mechanism_authority: panel::PRIMARY_MECHANISM_AUTHORITY.to_string(),
                entry_ordinal: ordinal,
                slot: entry.slot.clone(),
                case_id: entry.case_id.clone(),
                surface: entry.surface.clone(),
                payload_id: entry.payload_id.clone(),
                frozen_regime: entry.frozen_regime.clone(),
                selection_class: entry.selection_class.clone(),
                selection_reason: entry.selection_reason.clone(),
                horse: entry.horse.clone(),
                mechanism_id: markit_mdbench_campaign::execute::horse_id_to_mechanism(
                    &entry.horse,
                )?
                .to_string(),
                event_group: entry.event_group.clone(),
                repetition: entry.repetition,
                events: Vec::new(),
                low_count_events: Vec::new(),
                qualification: qualification.to_string(),
                correctness: CorrectnessV1 {
                    execution_status: execution.to_string(),
                    correctness_status: correctness_label.to_string(),
                    result_checksum: None,
                    failure: Some(failure_label),
                },
                host: host_identity(pinned_cpu),
                toolchain: toolchain_identity(),
            }
        }
    };

    let run_dir = raw_root.join(&run_id);
    std::fs::create_dir_all(&run_dir).map_err(|e| format!("create {}: {e}", run_dir.display()))?;
    let raw_file = run_dir.join("observations.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&raw_file)
        .map_err(|e| format!("open {}: {e}", raw_file.display()))?;
    let line =
        serde_json::to_string(&observation).map_err(|e| format!("serialize observation: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("append observation: {e}"))?;

    let healthy = observation.qualification == QUAL_QUALIFIED
        && observation.correctness.execution_status == "pass"
        && observation.correctness.correctness_status == "pass";
    println!(
        "PMU_OBSERVATION_WRITTEN entry={ordinal} slot={} horse={} group={} rep={} qual={} \
         correctness={}/{} file={}",
        observation.slot,
        observation.horse,
        observation.event_group,
        observation.repetition,
        observation.qualification,
        observation.correctness.execution_status,
        observation.correctness.correctness_status,
        raw_file.display()
    );
    Ok(healthy)
}

#[allow(clippy::too_many_arguments)]
fn dispatch_region(
    loaded: &LoadedEntry,
    entry: &PmuScheduleEntry,
    run_id: &str,
    observation_id: &str,
    driver_sha256: &str,
    header: &PmuScheduleHeader,
    pinned_cpu: u32,
) -> Result<PmuObservationV1, RunError> {
    match entry.horse.as_str() {
        "H0" => build_observation(
            markit_mdbench_full_rebuild::H0_MECHANISM_ID,
            &markit_mdbench_full_rebuild::FullRebuildMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        "H1" => build_observation(
            markit_mdbench_block_local::H1_MECHANISM_ID,
            &markit_mdbench_block_local::BlockLocalMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        "H2" => build_observation(
            markit_mdbench_fragment_reuse::H2_MECHANISM_ID,
            &markit_mdbench_fragment_reuse::FragmentReuseMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        "H3" => build_observation(
            markit_mdbench_old_tree_subtree_reuse::H3_MECHANISM_ID,
            &markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        "H4" => build_observation(
            markit_mdbench_restart_convergence::H4_MECHANISM_ID,
            &markit_mdbench_restart_convergence::RestartConvergenceMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        "HorseA" => build_observation(
            markit_mdbench_horse_a::HORSE_A_MECHANISM_ID,
            &markit_mdbench_horse_a::HorseAMechanism::new(),
            loaded,
            entry,
            run_id,
            observation_id,
            driver_sha256,
            header,
            pinned_cpu,
        ),
        other => Err(RunError::Setup(format!(
            "unknown horse {other:?} in PMU schedule entry"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_observation<M>(
    mechanism_id: &str,
    mechanism: &M,
    loaded: &LoadedEntry,
    entry: &PmuScheduleEntry,
    run_id: &str,
    observation_id: &str,
    driver_sha256: &str,
    header: &PmuScheduleHeader,
    pinned_cpu: u32,
) -> Result<PmuObservationV1, RunError>
where
    M: Mechanism,
    M::State: NormalizeV1 + ResultChecksum,
{
    // ---- setup stage: OUTSIDE the counters, MECHANISM-LEVEL semantics ---
    // Everything here mirrors the campaign cell loop: source construction,
    // the H0 reference oracle (built once per case), and SINGLE_RESET
    // fresh pre-state construction. Failures are Setup errors -> INVALID.
    enum Region<M: Mechanism> {
        Clean {
            source: markit_mdbench_common::Source,
        },
        Edit {
            pre: markit_mdbench_common::Source,
            post: markit_mdbench_common::Source,
            edit: markit_mdbench_common::CanonicalEdit,
            old_state: M::State,
        },
    }
    let setup: Result<Region<M>, RunError> = if let Some(clean) = loaded.clean.as_ref() {
        let source = markit_mdbench_common::Source::new(
            markit_mdbench_common::SourceId(0),
            clean.source_text.clone(),
        );
        Ok(Region::Clean { source })
    } else if let Some(edit_case) = loaded.edit.as_ref() {
        let (pre, post) = markit_mdbench_campaign::workload::edit_case_sources(edit_case);
        // SINGLE_RESET: fresh pre-edit state OUTSIDE the counter window,
        // never retained across processes (there is only one region per
        // process on this path).
        let old_state = markit_mdbench_runner::orchestrate::build_initial_state(mechanism, &pre)
            .map_err(|failure| {
                RunError::Setup(format!("fresh pre-state construction failed: {failure:?}"))
            })?;
        Ok(Region::Edit {
            pre,
            post,
            edit: edit_case.edit.clone(),
            old_state,
        })
    } else {
        Err(RunError::Setup(
            "entry has neither a clean nor an edit case".to_string(),
        ))
    };
    let setup = setup?;

    // ---- counter stage -------------------------------------------------
    let group_def = events::group_by_id(&entry.event_group).map_err(RunError::counter)?;
    let mut counters = EventCounterGroup::open(group_def.events, group_def.standalone)
        .map_err(|error| RunError::counter(format!("counter open failed: {error}")))?;
    let group_defs = counters.defs();
    let outcome = match setup {
        Region::Clean { source } => {
            // Correctness authority: H0 clean parse of the same source, ONCE,
            // outside the counter window (same authority as the campaign
            // cell).
            let reference = markit_mdbench_full_rebuild::parse_document(source.as_bytes());
            validate_normalized(&reference, None)
                .map_err(|e| RunError::Setup(format!("clean-state reference gate: {e:?}")))?;
            let hook = ReferenceOracle::new(reference);
            region::run_clean_state_region(mechanism, &source, &mut counters, &hook)
        }
        Region::Edit {
            pre,
            post,
            edit,
            old_state,
        } => {
            let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
            validate_normalized(&reference, None)
                .map_err(|e| RunError::Setup(format!("edit reference gate: {e:?}")))?;
            let hook = ReferenceOracle::new(reference);
            region::run_edit_write_region(
                mechanism,
                &pre,
                &post,
                &edit,
                old_state,
                &mut counters,
                &hook,
            )
        }
    };

    // A failed counter READ keeps the row (correctness facts preserved);
    // only the qualification degrades — never the other way around.
    let counts = outcome.counts;
    let (counts_error, readings) = match counts {
        Ok(readings) => (None, readings),
        Err(error) => (Some(error), Vec::new()),
    };
    let event_counts: Vec<EventCountV1> = group_defs
        .iter()
        .zip(readings)
        .map(|(def, reading)| EventCountV1::from_reading(def, reading))
        .collect();
    let low_count_events: Vec<String> = event_counts
        .iter()
        .filter(|count| count.raw_count < LOW_COUNT_THRESHOLD)
        .map(|count| count.alias.clone())
        .collect();
    // Frozen multiplexing qualification: EVERY event must show
    // time_running == time_enabled > 0. No scaling of multiplexed values.
    let multiplexed = event_counts
        .iter()
        .any(|count| count.time_enabled == 0 || count.time_running != count.time_enabled);
    let correctness_pass = outcome.execution_status == markit_mdbench_common::ExecutionStatus::Pass
        && outcome.correctness_status == markit_mdbench_common::CorrectnessStatus::Pass;
    let qualification = if !correctness_pass {
        QUAL_INVALID
    } else if counts_error.is_some() {
        QUAL_COUNTER_UNAVAILABLE
    } else if multiplexed {
        QUAL_UNQUALIFIED_MULTIPLEXED
    } else {
        QUAL_QUALIFIED
    };
    let mut failure = outcome.failure.map(|f| format!("{f:?}"));
    if failure.is_none() {
        if let Some(error) = counts_error {
            failure = Some(format!("counter read failed: {error}"));
        }
    }
    let correctness_block = CorrectnessV1 {
        execution_status: format!("{:?}", outcome.execution_status).to_lowercase(),
        correctness_status: format!("{:?}", outcome.correctness_status).to_lowercase(),
        result_checksum: outcome.result_checksum,
        failure,
    };
    Ok(PmuObservationV1 {
        schema: PMU_OBSERVATION_SCHEMA.to_string(),
        pmu_study_id: PMU_STUDY_ID.to_string(),
        observation_id: observation_id.to_string(),
        pmu_run_id: run_id.to_string(),
        panel_manifest_sha256: PMU_PANEL_MANIFEST_SHA256.to_string(),
        pmu_schedule_sha256: header.schedule_sha256.clone(),
        driver_sha256: driver_sha256.to_string(),
        primary_mechanism_authority: panel::PRIMARY_MECHANISM_AUTHORITY.to_string(),
        entry_ordinal: entry.entry_ordinal,
        slot: entry.slot.clone(),
        case_id: entry.case_id.clone(),
        surface: entry.surface.clone(),
        payload_id: entry.payload_id.clone(),
        frozen_regime: entry.frozen_regime.clone(),
        selection_class: entry.selection_class.clone(),
        selection_reason: entry.selection_reason.clone(),
        horse: entry.horse.clone(),
        mechanism_id: mechanism_id.to_string(),
        event_group: entry.event_group.clone(),
        repetition: entry.repetition,
        events: event_counts,
        low_count_events,
        qualification: qualification.to_string(),
        correctness: correctness_block,
        host: host_identity(pinned_cpu),
        toolchain: toolchain_identity(),
    })
}

// ---------------------------------------------------------------------------
// finalize: fail-closed verification of the collected raw evidence.
// ---------------------------------------------------------------------------

fn cmd_finalize(root: &Path, flags: &[String]) -> Result<bool, String> {
    let _ = root;
    let panel_path = flag_value(flags, "--panel").ok_or("finalize requires --panel")?;
    let schedule_path = flag_value(flags, "--schedule").ok_or("finalize requires --schedule")?;
    let raw_root = PathBuf::from(
        flag_value(flags, "--raw-root").unwrap_or_else(|| DEFAULT_RAW_ROOT.to_string()),
    );
    let panel = panel::load_verified_panel(Path::new(&panel_path), PMU_PANEL_MANIFEST_SHA256)?;
    let bytes =
        std::fs::read(&schedule_path).map_err(|e| format!("read schedule {schedule_path}: {e}"))?;
    let (header, entries) = schedule::verify_schedule_file(&bytes)?;
    schedule::entries_match_panel(&entries, &panel)?;
    // Same gate as `run`: the file must BE the frozen materialization.
    let (expected_header, expected_entries) = schedule::materialize(&panel)?;
    if header != expected_header || entries != expected_entries {
        return Err(
            "schedule file is not the frozen materialization of the verified panel".to_string(),
        );
    }

    let driver_sha256 = markit_mdbench_campaign::identity::current_executable_sha256()?;
    let mut hostname = hostname_only();
    hostname.truncate(64);
    let run_id = schema::pmu_run_id(
        &driver_sha256,
        PMU_PANEL_MANIFEST_SHA256,
        &header.schedule_sha256,
        &hostname,
    );
    let run_dir = raw_root.join(&run_id);
    let raw_file = run_dir.join("observations.jsonl");
    let raw_bytes =
        std::fs::read(&raw_file).map_err(|e| format!("read {}: {e}", raw_file.display()))?;
    let mut observations: Vec<PmuObservationV1> = Vec::new();
    for line in raw_bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let row: PmuObservationV1 =
            serde_json::from_slice(line).map_err(|e| format!("parse observation: {e}"))?;
        observations.push(row);
    }

    let mut blockers: Vec<String> = Vec::new();
    if observations.len() != entries.len() {
        blockers.push(format!(
            "observations {} != schedule entries {}",
            observations.len(),
            entries.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut covered = BTreeSet::new();
    let mut invalid = 0usize;
    let mut unqualified = 0usize;
    for row in &observations {
        if !ids.insert(row.observation_id.clone()) {
            blockers.push(format!("duplicate observation id {}", row.observation_id));
        }
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.entry_ordinal == row.entry_ordinal)
        else {
            blockers.push(format!(
                "row references unknown entry ordinal {}",
                row.entry_ordinal
            ));
            continue;
        };
        if !covered.insert(row.entry_ordinal) {
            blockers.push(format!("entry ordinal {} covered twice", row.entry_ordinal));
        }
        if row.horse != entry.horse
            || row.event_group != entry.event_group
            || row.repetition != entry.repetition
            || row.slot != entry.slot
            || row.case_id != entry.case_id
            || row.surface != entry.surface
            || row.selection_class != entry.selection_class
            || row.pmu_run_id != run_id
            || row.panel_manifest_sha256 != PMU_PANEL_MANIFEST_SHA256
            || row.pmu_schedule_sha256 != header.schedule_sha256
        {
            blockers.push(format!("row identity drift at entry {}", row.entry_ordinal));
        }
        if row.correctness.execution_status != "pass"
            || row.correctness.correctness_status != "pass"
        {
            invalid += 1;
        } else if row.qualification == QUAL_UNQUALIFIED_MULTIPLEXED
            || row.qualification == QUAL_COUNTER_UNAVAILABLE
        {
            unqualified += 1;
        }
    }
    let missing: Vec<String> = entries
        .iter()
        .filter(|entry| !covered.contains(&entry.entry_ordinal))
        .map(|entry| entry.slot.clone())
        .collect();
    if !missing.is_empty() {
        blockers.push(format!(
            "{} schedule entries have no observation (e.g. {:?})",
            missing.len(),
            missing.first()
        ));
    }

    let file_sha256 = markit_mdbench_campaign::sha256_file(&raw_file)?;
    if blockers.is_empty() {
        let receipt = serde_json::json!({
            "schema": "pmu-raw-receipt-v1",
            "verdict": "PMU_RAW_FILE_PASS",
            "study": PMU_STUDY_ID,
            "pmu_run_id": run_id,
            "driver_sha256": driver_sha256,
            "panel_manifest_sha256": PMU_PANEL_MANIFEST_SHA256,
            "pmu_schedule_sha256": header.schedule_sha256,
            "rows": observations.len(),
            "file": raw_file.display().to_string(),
            "file_sha256": file_sha256,
            "invalid_observations": invalid,
            "unqualified_observations": unqualified,
        });
        let receipt_path = run_dir.join("observations.receipt.json");
        std::fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt).unwrap(),
        )
        .map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
        println!(
            "PMU_RAW_FILE_PASS file={} rows={} sha256={file_sha256} invalid={invalid} \
             unqualified={unqualified}",
            raw_file.display(),
            observations.len()
        );
        Ok(true)
    } else {
        let marker = serde_json::json!({
            "schema": "pmu-raw-invalid-v1",
            "verdict": "PMU_STUDY_INVALID",
            "blockers": blockers,
            "rows": observations.len(),
            "file_sha256": file_sha256,
            "invalid_observations": invalid,
            "unqualified_observations": unqualified,
        });
        let marker_path = run_dir.join("observations.jsonl.invalid.json");
        std::fs::write(&marker_path, serde_json::to_string_pretty(&marker).unwrap())
            .map_err(|e| format!("write {}: {e}", marker_path.display()))?;
        eprintln!("PMU_RAW_FILE_INVALID file={}", raw_file.display());
        for blocker in &blockers {
            eprintln!("PMU_BLOCKER {blocker}");
        }
        Ok(false)
    }
}
