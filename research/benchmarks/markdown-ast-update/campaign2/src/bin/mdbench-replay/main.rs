//! `mdbench-replay` — the dedicated, region-scoped replay command used by
//! the PROFILING lane (task §38).
//!
//! It exists so profiling never has to subtract a build-only run from a
//! whole-process run:
//!
//! - the process does ONLY the region of interest, in a loop;
//! - hardware PMU counters are opened with `perf_event_open`, scoped with
//!   `exclude_kernel = 1`, DISABLED at open time, enabled immediately
//!   before the measured phase and disabled immediately after it, so the
//!   counts describe the region itself and not the process;
//! - a `--setup-only` mode runs the identical preparation WITHOUT the
//!   measured phase, so any whole-process (`perf stat`) number can be
//!   validated rather than assumed;
//! - the in-process timing reported here is the SAME frozen timer
//!   boundary the primary surfaces use.
//!
//! Output is one JSON object per line: a header row, then one row per
//! repetition.
//!
//! ```text
//! mdbench-replay <root> --surface construction|resident-update|controlled|lifecycle
//!                [--axis A --cell L] [--case-index N] [--trace F] [--step S]
//!                --horse H3 --reps 10 [--setup-only] [--json]
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use markit_mdbench_campaign2::exec::{reference_for, CaseSpec};
use markit_mdbench_campaign2::generators::{self, Axis};
use markit_mdbench_campaign2::lifecycle::{self, LifecycleTraceV1};
use markit_mdbench_common::{
    CanonicalEdit, CaseId, CaseKeyV1, Observed, OperationKind, PayloadShape, Source, SourceId,
};
use markit_mdbench_instrumentation::{InstantClock, LaneMeasurement};
use markit_mdbench_oracle::ReferenceOracle;

mod perfcount;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let root = match args.get(1) {
        Some(root) => PathBuf::from(root),
        None => {
            eprintln!("usage: mdbench-replay <benchmark_root> --surface S --horse Hx --reps N");
            return ExitCode::from(2);
        }
    };
    let flags: Vec<String> = args[2..].to_vec();
    match run(&root, &flags) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mdbench-replay: {error}");
            ExitCode::from(1)
        }
    }
}

fn flag_value(flags: &[String], name: &str) -> Option<String> {
    flags
        .iter()
        .position(|f| f == name)
        .and_then(|i| flags.get(i + 1))
        .cloned()
}

fn has_flag(flags: &[String], name: &str) -> bool {
    flags.iter().any(|f| f == name)
}

fn flag_u32(flags: &[String], name: &str, default: u32) -> u32 {
    flag_value(flags, name).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn run(root: &Path, flags: &[String]) -> Result<(), String> {
    let surface = flag_value(flags, "--surface")
        .ok_or_else(|| "--surface is required".to_string())?;
    let horse = flag_value(flags, "--horse").ok_or_else(|| "--horse is required".to_string())?;
    if !markit_mdbench_campaign2::HORSE_IDS.contains(&horse.as_str()) {
        return Err(format!("unknown horse {horse:?}"));
    }
    let reps = flag_u32(flags, "--reps", 10);
    let setup_only = has_flag(flags, "--setup-only");
    let clock = InstantClock::new();

    // Materialize the case OUTSIDE every measured region.
    let case = resolve_case(root, &surface, flags)?;
    let reference = reference_for(&case.spec.post_source)?;
    let build = markit_mdbench_runner::current_build_identity();
    let facts = case
        .spec
        .facts(
            markit_mdbench_campaign2::horse_mechanism_id(&horse)?,
            0,
        );

    println!(
        "{}",
        serde_json::json!({
            "schema": "campaign2-replay-header-v1",
            "surface": surface,
            "case_id": case.spec.case_id_hex,
            "cell_id": case.cell_id,
            "horse": horse,
            "reps": reps,
            "setup_only": setup_only,
            "source_bytes": case.spec.pre_source.len(),
            "post_bytes": case.spec.post_source.len(),
            "executable_sha256": markit_mdbench_campaign2::current_executable_sha256()
                .unwrap_or_else(|_| "UNAVAILABLE".to_string()),
            "build_identity": {
                "runner_git_commit": build.runner_git_commit,
                "rustc": build.rustc,
                "target": build.target,
                "build_profile_id": build.build_profile_id,
                "cargo_lock_sha256": build.cargo_lock_sha256,
            },
            "region_scope": if setup_only { "setup_only" } else { "measured_region" },
        })
    );

    let perf = perfcount::PerfCounters::open()?;
    let mut totals = perfcount::Counts::default();
    let label = markit_mdbench_campaign2::horse_mechanism_id(&horse)?.to_string();

    markit_mdbench_campaign2::with_horse!(horse.as_str(), |mech| {
        // Warm the code paths once, outside the measured region.
        let _ = markit_mdbench_campaign2::exec::resident_update_row(
            &mech,
            &case.spec,
            &clock,
            &ReferenceOracle::new(reference.clone()),
            &facts,
            &probe_identity(),
            &build,
        );
        for rep in 0..reps {
            // EVERYTHING that is not the measured region happens with the
            // counters DISABLED: the fresh pre-state build (Surface B's
            // frozen "outside every timer" rule) and, in `--setup-only`
            // mode, the build alone.
            let edit = case.spec.edit.clone();
            let prepared_state = if edit.is_some() || setup_only {
                match markit_mdbench_runner::orchestrate::build_initial_state(
                    &mech,
                    &Source::new(SourceId(0), case.spec.pre_source.clone()),
                ) {
                    Ok(state) => Some(state),
                    Err(failure) => return Err(format!("fresh pre-state build failed: {failure:?}")),
                }
            } else {
                None
            };
            let counts = perf.begin();
            let (total_ns, correctness_pass) = if setup_only {
                // The measured region is EMPTY by construction: this run
                // exists only so a whole-process number can be validated.
                (0u64, true)
            } else if let (Some(edit), Some(state)) = (edit.as_ref(), prepared_state) {
                let pre = Source::new(SourceId(0), case.spec.pre_source.clone());
                let post = Source::new(SourceId(1), case.spec.post_source.clone());
                let hook = ReferenceOracle::new(reference.clone());
                let outcome = markit_mdbench_campaign2::exec::run_update_chain_step(
                    &mech, &pre, &post, edit, state, &clock, &hook,
                );
                let total = match &outcome.report.measurement {
                    LaneMeasurement::Timing(t) => match t.total_ns {
                        Observed::Known(ns) => ns,
                        _ => 0,
                    },
                    _ => 0,
                };
                (
                    total,
                    outcome.report.correctness_status
                        == markit_mdbench_common::CorrectnessStatus::Pass,
                )
            } else {
                let source = Source::new(SourceId(0), case.spec.pre_source.clone());
                let hook = ReferenceOracle::new(reference.clone());
                let report = markit_mdbench_runner::orchestrate::run_full_parse_timed(
                    &mech, &source, &clock, &hook,
                );
                let total = match &report.measurement {
                    LaneMeasurement::Timing(t) => match t.total_ns {
                        Observed::Known(ns) => ns,
                        _ => 0,
                    },
                    _ => 0,
                };
                (
                    total,
                    report.correctness_status == markit_mdbench_common::CorrectnessStatus::Pass,
                )
            };
            let counts = perf.end(counts);
            totals.add(&counts);
            println!(
                "{}",
                serde_json::json!({
                    "schema": "campaign2-replay-row-v1",
                    "horse": horse,
                    "mechanism_id": label,
                    "rep": rep,
                    "total_ns": total_ns,
                    "correctness_pass": correctness_pass,
                    "task_clock_ns": counts.task_clock_ns,
                    "cycles": counts.cycles,
                    "instructions": counts.instructions,
                    "branches": counts.branches,
                    "branch_misses": counts.branch_misses,
                    "cache_references": counts.cache_references,
                    "cache_misses": counts.cache_misses,
                })
            );
        }
        println!(
            "{}",
            serde_json::json!({
                "schema": "campaign2-replay-total-v1",
                "horse": horse,
                "reps": reps,
                "region": if setup_only { "setup_only" } else { "measured_region" },
                "task_clock_ns": totals.task_clock_ns,
                "cycles": totals.cycles,
                "instructions": totals.instructions,
                "branches": totals.branches,
                "branch_misses": totals.branch_misses,
                "cache_references": totals.cache_references,
                "cache_misses": totals.cache_misses,
                "perf_available": perf.available(),
            })
        );
        Ok::<(), String>(())
    })
}

fn probe_identity() -> markit_mdbench_campaign2::envelope::ExecutionIdentity2 {
    markit_mdbench_campaign2::envelope::ExecutionIdentity2 {
        study_id: "replay".to_string(),
        campaign_spec_id: "replay".to_string(),
        sub_campaign_spec_id: "profiling".to_string(),
        run_id: "replay".to_string(),
        evidence_class: "PROFILE_PERF",
        machine_environment_ref: "replay".to_string(),
        provenance: "CAMPAIGN-2/REPLAY",
        non_research: true,
    }
}

/// The resolved replay target.
struct ReplayCase {
    spec: CaseSpec,
    cell_id: Option<String>,
}

fn resolve_case(root: &Path, surface: &str, flags: &[String]) -> Result<ReplayCase, String> {
    match surface {
        "construction" | "resident-update" | "resident_update" => {
            let index = flag_u32(flags, "--case-index", 0) as usize;
            let workload = markit_mdbench_campaign::workload::load_campaign_workload(root)?;
            if surface == "construction" {
                let case = workload
                    .clean_state
                    .get(index)
                    .ok_or_else(|| format!("clean_state case index {index} out of range"))?;
                Ok(ReplayCase {
                    spec: CaseSpec::full_parse(
                        case.case_id,
                        case.case_id_hex.clone(),
                        case.payload_id.clone(),
                        case.source_text.clone(),
                    ),
                    cell_id: None,
                })
            } else {
                let case = workload
                    .edit_write
                    .get(index)
                    .ok_or_else(|| format!("edit_write case index {index} out of range"))?;
                Ok(ReplayCase {
                    spec: CaseSpec::update(
                        case.case_id,
                        case.case_id_hex.clone(),
                        case.payload_id.clone(),
                        case.pre_source_text.clone(),
                        case.post_source_text.clone(),
                        case.edit.clone(),
                    ),
                    cell_id: Some(case.trace_id.clone()),
                })
            }
        }
        "controlled" => {
            let axis = Axis::parse(
                &flag_value(flags, "--axis").ok_or_else(|| "--axis is required".to_string())?,
            )?;
            let label = flag_value(flags, "--cell")
                .ok_or_else(|| "--cell is required".to_string())?;
            let cell = generators::generate_cell(axis, &label)?;
            generators::verify_case(&cell)?;
            Ok(ReplayCase {
                spec: markit_mdbench_campaign2::controlled::case_spec(&cell),
                cell_id: Some(cell.cell_id),
            })
        }
        "lifecycle" => {
            let trace_filter = flag_value(flags, "--trace")
                .ok_or_else(|| "--trace is required".to_string())?;
            let step_index = flag_u32(flags, "--step", 0) as usize;
            let traces = load_traces(root)?;
            let trace = traces
                .iter()
                .find(|t| t.trace_id == trace_filter || t.family == trace_filter)
                .ok_or_else(|| format!("no frozen trace matches {trace_filter:?}"))?;
            let workload = markit_mdbench_campaign::workload::load_campaign_workload(root)?;
            let initial = trace_initial(trace, &workload)?;
            let mut current = initial;
            for step in trace.steps.iter().take(step_index + 1) {
                let edit = step.edit();
                let pre = Source::new(SourceId(0), current.clone());
                let post = edit.apply(&pre, SourceId(1)).map_err(|e| format!("{e:?}"))?.as_str().to_string();
                if step.step as usize == step_index {
                    let spec = lifecycle_spec(&trace.trace_id, &current, &post, &edit, step)?;
                    return Ok(ReplayCase { spec, cell_id: Some(format!("{}#{}", trace.trace_id, step_index)) });
                }
                current = post;
            }
            Err(format!("trace {} has no step {step_index}", trace.trace_id))
        }
        other => Err(format!("unknown replay surface {other:?}")),
    }
}

fn load_traces(root: &Path) -> Result<Vec<LifecycleTraceV1>, String> {
    let path = markit_mdbench_campaign2::store::campaign_root(root)
        .join("manifests/lifecycle-traces-v1.jsonl");
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        out.push(serde_json::from_slice(line).map_err(|e| format!("parse trace: {e}"))?);
    }
    Ok(out)
}

fn trace_initial(
    trace: &LifecycleTraceV1,
    workload: &markit_mdbench_campaign::workload::CampaignWorkload,
) -> Result<String, String> {
    match &trace.initial_source_origin {
        lifecycle::TraceOriginV1::RealPayloadChain { workload_trace_id, .. } => workload
            .edit_write
            .iter()
            .find(|c| &c.trace_id == workload_trace_id)
            .map(|c| c.pre_source_text.clone())
            .ok_or_else(|| format!("real trace {workload_trace_id} not in the workload")),
        lifecycle::TraceOriginV1::Controlled { family, .. } => lifecycle::controlled_plans()
            .into_iter()
            .find(|p| p.family() == family)
            .map(|p| p.initial_source())
            .ok_or_else(|| format!("no controlled plan for family {family}")),
    }
}

fn lifecycle_spec(
    trace_id: &str,
    pre: &str,
    post: &str,
    edit: &CanonicalEdit,
    step: &lifecycle::TraceStepV1,
) -> Result<CaseSpec, String> {
    let operation = OperationKind::classify(edit);
    let key = CaseKeyV1 {
        payload_id: format!("c2-lifecycle:{trace_id}:{}", step.step),
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: pre.len() as u64,
        old_source_sha256: hex32(&markit_mdbench_campaign2::sha256_hex(pre.as_bytes()))?,
        operation,
        edit_start_byte: Some(edit.start_byte()),
        edit_end_byte: Some(edit.end_byte()),
        inserted_text_sha256: if operation.has_inserted_text() {
            Some(hex32(&markit_mdbench_campaign2::sha256_hex(
                edit.inserted_text().as_bytes(),
            ))?)
        } else {
            None
        },
        generator_id: Some("CAMPAIGN-2-REPLAY".to_string()),
        generator_seed: Some(0),
    }
    .validated()
    .map_err(|e| format!("replay case key: {e:?}"))?;
    Ok(CaseSpec::update(
        CaseId::from_key(&key),
        key.payload_id.clone(),
        key.payload_id.clone(),
        pre.to_string(),
        post.to_string(),
        edit.clone(),
    ))
}

fn hex32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("digest {hex:?} is not 64 hex chars"));
    }
    (0..32)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "digest length".to_string())
}
