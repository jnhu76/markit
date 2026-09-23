//! `mdbench-h4diag` — Issue #50 diagnostic CLI.
//!
//! ```text
//! mdbench-h4diag cells            [--out F]
//! mdbench-h4diag preflight        [--out F]
//! mdbench-h4diag verify-equivalence [--out F]
//!
//! mdbench-h4diag run-plain      --session N [--out F] [--reps R] [--warmup W] [--schedule-out F]
//! mdbench-h4diag run-phase      --session N [--out F] [--reps R] [--warmup W] [--schedule-out F]
//! mdbench-h4diag run-counters   --session N [--out F] [--schedule-out F]
//! mdbench-h4diag run-ablations  --session N [--out F] [--reps R] [--warmup W] [--schedule-out F]
//! mdbench-h4diag run-alloc      --session N [--out F] [--schedule-out F]
//! mdbench-h4diag receipt        --lane L [--out F]
//! ```
//!
//! Every lane writes JSONL rows plus a `receipt` object recording the
//! **actual producing executable SHA256**, the build identity, and the
//! schedule seed. Lane separation is enforced by construction: the
//! `run-plain` path never touches a phase timer, a diagnostic counter or
//! the allocator accounting (they are not compiled into that binary).

use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use markit_mdbench_common::{
    CanonicalEdit, CorrectnessStatus, ExecutionStatus, FailureStatus, Observed, Source, SourceId,
    WorkCounters,
};
use markit_mdbench_instrumentation::{InstantClock, LaneMeasurement};
use markit_mdbench_oracle::ReferenceOracle;
use markit_mdbench_restart_convergence::RestartConvergenceMechanism;
use markit_mdbench_runner::{build_initial_state, run_update_attributed, run_update_timed};

use crate::alg::{H4Diag, Variant};
use crate::cells::{all_cells, Cell};
use crate::deferred;
use crate::schedule;

/// Frozen sampling (Issue #50 §3).
pub const WARMUP_ITERATIONS: u32 = 10;
pub const MEASURED_ITERATIONS: u32 = 30;

/// Labels of the two identical A0 control slots (Issue #50 §3).
pub const CONTROL_LABELS: [&str; 2] = ["A0", "A0-dup"];

/// Ablation labels, in report order.
pub const ABLATION_LABELS: [&str; 5] = ["A0", "A0-dup", "Adefs", "Adrop", "Acapacity"];

// ---------------------------------------------------------------------------
// Flag parsing
// ---------------------------------------------------------------------------

fn flag_value(flags: &[String], name: &str) -> Option<String> {
    flags
        .iter()
        .position(|f| f == name)
        .and_then(|i| flags.get(i + 1).cloned())
}

fn flag_u32(flags: &[String], name: &str, default: u32) -> u32 {
    flag_value(flags, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn out_writer(flags: &[String], default_name: &str) -> Result<Box<dyn Write>, String> {
    match flag_value(flags, "--out") {
        Some(path) => {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("create {}: {e}", parent.display()))?;
            }
            let file =
                std::fs::File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
            Ok(Box::new(std::io::BufWriter::new(file)))
        }
        None => {
            let _ = default_name;
            Ok(Box::new(std::io::stdout()))
        }
    }
}

fn write_schedule(flags: &[String], slots: &[schedule::Slot]) -> Result<(), String> {
    if let Some(path) = flag_value(flags, "--schedule-out") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, schedule::to_jsonl(slots))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

fn emit(out: &mut dyn Write, value: &serde_json::Value) -> Result<(), String> {
    serde_json::to_writer(&mut *out, value).map_err(|e| format!("serialize row: {e}"))?;
    out.write_all(b"\n").map_err(|e| format!("write row: {e}"))?;
    Ok(())
}

fn obs_u64(v: &Observed<u64>) -> serde_json::Value {
    match v {
        Observed::Known(x) => serde_json::Value::from(*x),
        Observed::Unknown => serde_json::Value::from("UNKNOWN"),
        Observed::NotApplicable => serde_json::Value::from("NOT_APPLICABLE"),
    }
}

fn known(v: &Observed<u64>) -> Option<u64> {
    match v {
        Observed::Known(x) => Some(*x),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Executable / build identity
// ---------------------------------------------------------------------------

/// SHA256 of the currently running executable (fails closed).
pub fn current_executable_sha256() -> Result<String, String> {
    let path = std::env::current_exe()
        .map_err(|e| format!("resolve current executable: {e}"))?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(crate::cells::sha256_hex(&bytes))
}

/// Identity block emitted with every lane.
pub fn receipt(lane: &str, extra: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "record": "receipt",
        "lane": lane,
        "executable_path": std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default(),
        "executable_sha256": current_executable_sha256()?,
        "build": {
            "cargo_pkg_version": env!("CARGO_PKG_VERSION"),
            "profile": "release",
            "lto": "thin",
            "codegen_units": 1,
            "panic": "unwind",
        },
        "features": {
            "phases": cfg!(feature = "phases"),
            "counters": cfg!(feature = "counters"),
            "allocator": cfg!(feature = "allocator"),
        },
        "authority": {
            "base_sha": "334eea6201fc0258e35a7c5b21feb722641ddcbd",
            "mechanism_authority_sha": "3762b7a42e1c284a4c2c2e0ebac8496e70c63431",
            "workload": "issue-50-local-text-regime",
        },
        "sampling": {
            "warmup_iterations": WARMUP_ITERATIONS,
            "measured_iterations": MEASURED_ITERATIONS,
        },
        "extra": extra,
    }))
}

// ---------------------------------------------------------------------------
// Cells / preflight
// ---------------------------------------------------------------------------

fn cmd_cells(flags: &[String]) -> Result<(), String> {
    let mut out = out_writer(flags, "cells")?;
    for cell in all_cells()? {
        emit(&mut out, &cell.to_json())?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

/// Named slot differences between two frozen work-counter records.
fn work_counter_diff(a: &WorkCounters, b: &WorkCounters) -> Vec<String> {
    let ja = serde_json::to_value(a).unwrap_or(serde_json::Value::Null);
    let jb = serde_json::to_value(b).unwrap_or(serde_json::Value::Null);
    let mut out = Vec::new();
    if let (serde_json::Value::Object(ma), serde_json::Value::Object(mb)) = (ja, jb) {
        for (k, va) in ma {
            if let Some(vb) = mb.get(&k) {
                if va != *vb {
                    out.push(format!("{k}: original={va} diag={vb}"));
                }
            }
        }
    }
    out.sort();
    out
}

/// One preflight observation: the frozen workload invariants, measured
/// through the frozen `CounterSink` on the diagnostic A0 copy.
struct Preflight {
    cell: String,
    checksum_matches_original: bool,
    counters_match_original: bool,
    restart_distance: Option<u64>,
    convergence_distance: Option<u64>,
    fresh_blocks: Option<u64>,
    fresh_nodes: Option<u64>,
    nodes_reused: Option<u64>,
    unique_post_bytes: Option<u64>,
    source_bytes_inspected_total: Option<u64>,
    correctness: String,
    problems: Vec<String>,
    counter_diff: Vec<String>,
}

fn preflight_one(cell: &Cell) -> Result<Preflight, String> {
    let old = Source::new(SourceId(0), cell.pre_source.clone());
    let post = Source::new(SourceId(1), cell.post_source.clone());
    let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;

    let diag = H4Diag::new(Variant::A0);
    let original = RestartConvergenceMechanism::new();

    // Original H4, attribution lane (frozen counters).
    let mut orig_counters = WorkCounters::all_unknown();
    let orig_state = build_initial_state(&original, &old).map_err(|f| format!("{f:?}"))?;
    let orig_hook = ReferenceOracle::new(reference.clone());
    let orig_report = run_update_attributed(
        &original,
        &old,
        &post,
        &cell.edit,
        orig_state,
        &mut orig_counters,
        &orig_hook,
    );

    // Diagnostic copy, attribution lane.
    let mut diag_counters = WorkCounters::all_unknown();
    let diag_state = build_initial_state(&diag, &old).map_err(|f| format!("{f:?}"))?;
    let diag_hook = ReferenceOracle::new(reference);
    let diag_report = run_update_attributed(
        &diag,
        &old,
        &post,
        &cell.edit,
        diag_state,
        &mut diag_counters,
        &diag_hook,
    );

    let mut problems = Vec::new();
    let checksum_matches_original = orig_report.result_checksum == diag_report.result_checksum;
    if !checksum_matches_original {
        problems.push("diagnostic copy checksum differs from the original H4".to_string());
    }
    let counter_diff = work_counter_diff(&orig_counters, &diag_counters);
    let counters_match_original = counter_diff.is_empty();
    if !counters_match_original {
        problems.push(format!(
            "diagnostic copy work counters differ from the original H4: {}",
            counter_diff.join("; ")
        ));
    }

    let restart_distance = known(&diag_counters.restart_distance);
    let convergence_distance = known(&diag_counters.convergence_distance);
    let fresh_blocks = known(&diag_counters.blocks_reparsed);
    let fresh_nodes = known(&diag_counters.nodes_rebuilt);
    let nodes_reused = known(&diag_counters.nodes_reused);
    let unique_post_bytes = known(&diag_counters.unique_post_source_bytes);
    let source_bytes_inspected_total = known(&diag_counters.source_bytes_inspected_total);

    if restart_distance != Some(Cell::EXPECTED_RESTART_DISTANCE) {
        problems.push(format!(
            "restart distance {restart_distance:?} != expected {}",
            Cell::EXPECTED_RESTART_DISTANCE
        ));
    }
    if convergence_distance != Some(Cell::EXPECTED_CONVERGENCE_DISTANCE) {
        problems.push(format!(
            "convergence distance {convergence_distance:?} != expected {}",
            Cell::EXPECTED_CONVERGENCE_DISTANCE
        ));
    }
    if fresh_blocks != Some(Cell::EXPECTED_FRESH_BLOCKS) {
        problems.push(format!(
            "fresh blocks {fresh_blocks:?} != expected {}",
            Cell::EXPECTED_FRESH_BLOCKS
        ));
    }
    if fresh_nodes != Some(Cell::EXPECTED_FRESH_NODES) {
        problems.push(format!(
            "fresh nodes {fresh_nodes:?} != expected {}",
            Cell::EXPECTED_FRESH_NODES
        ));
    }
    if unique_post_bytes != Some(Cell::EXPECTED_UNIQUE_POST_BYTES) {
        problems.push(format!(
            "unique post bytes {unique_post_bytes:?} != expected {}",
            Cell::EXPECTED_UNIQUE_POST_BYTES
        ));
    }
    if convergence_distance.map(|d| d < cell.n_bytes as u64) != Some(true) {
        problems.push("convergence is not before EOF".to_string());
    }
    if diag_report.correctness_status != CorrectnessStatus::Pass {
        problems.push(format!(
            "correctness {:?}",
            diag_report.correctness_status
        ));
    }
    if diag_report.execution_status != ExecutionStatus::Pass {
        problems.push(format!("execution {:?}", diag_report.execution_status));
    }

    Ok(Preflight {
        cell: cell.label.clone(),
        checksum_matches_original,
        counters_match_original,
        restart_distance,
        convergence_distance,
        fresh_blocks,
        fresh_nodes,
        nodes_reused,
        unique_post_bytes,
        source_bytes_inspected_total,
        correctness: format!("{:?}", diag_report.correctness_status),
        problems,
        counter_diff,
    })
}

fn cmd_preflight(flags: &[String]) -> Result<(), String> {
    let mut out = out_writer(flags, "preflight")?;
    let mut failures = 0usize;
    for cell in all_cells()? {
        let p = preflight_one(&cell)?;
        if !p.problems.is_empty() {
            failures += 1;
        }
        emit(
            &mut out,
            &serde_json::json!({
                "record": "preflight",
                "cell": p.cell,
                "n_bytes": cell.n_bytes,
                "m_blocks": cell.m_blocks,
                "case_id_hex": cell.case_id_hex,
                "campaign2_overlap": cell.campaign2_overlap,
                "checksum_matches_original_h4": p.checksum_matches_original,
                "work_counters_match_original_h4": p.counters_match_original,
                "work_counter_diff": p.counter_diff,
                "restart_distance": p.restart_distance,
                "convergence_distance": p.convergence_distance,
                "fresh_blocks": p.fresh_blocks,
                "fresh_nodes": p.fresh_nodes,
                "nodes_reused": p.nodes_reused,
                "unique_post_bytes": p.unique_post_bytes,
                "source_bytes_inspected_total": p.source_bytes_inspected_total,
                "correctness": p.correctness,
                "problems": p.problems,
                "status": if p.problems.is_empty() { "PASS" } else { "FAIL" },
            }),
        )?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    if failures > 0 {
        return Err(format!("{failures} cell(s) failed preflight"));
    }
    Ok(())
}

fn cmd_verify_equivalence(flags: &[String]) -> Result<(), String> {
    let mut out = out_writer(flags, "verify-equivalence")?;
    emit(&mut out, &receipt("verify-equivalence", serde_json::json!({}))?)?;
    let mut failures = 0usize;
    for cell in all_cells()? {
        let p = preflight_one(&cell)?;
        if !(p.checksum_matches_original && p.counters_match_original && p.problems.is_empty()) {
            failures += 1;
        }
        emit(
            &mut out,
            &serde_json::json!({
                "record": "equivalence",
                "cell": p.cell,
                "checksum_matches_original_h4": p.checksum_matches_original,
                "work_counters_match_original_h4": p.counters_match_original,
                "work_counter_diff": p.counter_diff,
                "problems": p.problems,
            }),
        )?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    if failures > 0 {
        return Err(format!("{failures} cell(s) failed equivalence"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lane A — U_PLAIN (original H4, frozen timer boundary)
// ---------------------------------------------------------------------------

fn cell_labels() -> Vec<String> {
    crate::CELL_LABELS.iter().map(|s| s.to_string()).collect()
}

fn cmd_run_plain(flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let reps = flag_u32(flags, "--reps", MEASURED_ITERATIONS);
    let warmup = flag_u32(flags, "--warmup", WARMUP_ITERATIONS);
    let mut out = out_writer(flags, "u-plain")?;
    emit(
        &mut out,
        &receipt(
            "u-plain",
            serde_json::json!({
                "session": session,
                "schedule_seed": schedule::lane_seed("u-plain", session),
                "labels": CONTROL_LABELS,
                "mechanism": markit_mdbench_restart_convergence::H4_MECHANISM_ID,
                "instrumentation": "none (original H4, frozen runner timers, NoopWorkSink)",
            }),
        )?,
    )?;

    let cells = all_cells()?;
    let labels: Vec<String> = CONTROL_LABELS.iter().map(|s| s.to_string()).collect();
    let slots = schedule::build(
        &cell_labels(),
        &labels,
        warmup + reps,
        schedule::lane_seed("u-plain", session),
    );
    write_schedule(flags, &slots)?;

    let mechanism = RestartConvergenceMechanism::new();
    let clock = InstantClock::new();
    let by_label: std::collections::HashMap<&str, &Cell> =
        cells.iter().map(|c| (c.label.as_str(), c)).collect();

    for slot in &slots {
        let cell = by_label
            .get(slot.cell.as_str())
            .ok_or_else(|| format!("unknown cell {}", slot.cell))?;
        let old = Source::new(SourceId(0), cell.pre_source.clone());
        let post = Source::new(SourceId(1), cell.post_source.clone());
        let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;
        let hook = ReferenceOracle::new(reference);
        let old_state = build_initial_state(&mechanism, &old).map_err(|f| format!("build_initial_state: {f:?}"))?;
        let report = run_update_timed(
            &mechanism,
            &old,
            &post,
            &cell.edit,
            old_state,
            &clock,
            &hook,
        );
        let timing = match &report.measurement {
            LaneMeasurement::Timing(t) => t,
            other => return Err(format!("unexpected lane {other:?}")),
        };
        emit(
            &mut out,
            &serde_json::json!({
                "record": "observation",
                "lane": "u-plain",
                "session": session,
                "round": slot.round,
                "ordinal": slot.ordinal,
                "label": slot.label,
                "cell": cell.label,
                "n_bytes": cell.n_bytes,
                "m_blocks": cell.m_blocks,
                "case_id_hex": cell.case_id_hex,
                "sample_kind": if slot.round < warmup { "warmup" } else { "measured" },
                "measured_ordinal": slot.round.saturating_sub(warmup),
                "execution_status": format!("{:?}", report.execution_status),
                "correctness_status": format!("{:?}", report.correctness_status),
                "failure": report.failure.as_ref().map(|f| format!("{f:?}")),
                "prepare_ns": obs_u64(&timing.prepare_ns),
                "native_ns": obs_u64(&timing.native_ns),
                "total_ns": obs_u64(&timing.total_ns),
                "result_checksum": report.result_checksum,
            }),
        )?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Lane B — U_PHASE
// ---------------------------------------------------------------------------

#[cfg(feature = "phases")]
fn cmd_run_phase(flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let reps = flag_u32(flags, "--reps", MEASURED_ITERATIONS);
    let warmup = flag_u32(flags, "--warmup", WARMUP_ITERATIONS);
    let mut out = out_writer(flags, "u-phase")?;
    emit(
        &mut out,
        &receipt(
            "u-phase",
            serde_json::json!({
                "session": session,
                "schedule_seed": schedule::lane_seed("u-phase", session),
                "mechanism": crate::alg::H4DIAG_MECHANISM_ID,
                "phase_map": crate::phases::Phase::ALL.iter().map(|p| p.name()).collect::<Vec<_>>(),
                "hook_sub_measure": "inclusive_of_P2_never_summed",
            }),
        )?,
    )?;

    let cells = all_cells()?;
    let labels: Vec<String> = vec!["A0".to_string()];
    let slots = schedule::build(
        &cell_labels(),
        &labels,
        warmup + reps,
        schedule::lane_seed("u-phase", session),
    );
    write_schedule(flags, &slots)?;

    let mechanism = H4Diag::new(Variant::A0);
    let clock = InstantClock::new();
    let by_label: std::collections::HashMap<&str, &Cell> =
        cells.iter().map(|c| (c.label.as_str(), c)).collect();

    for slot in &slots {
        let cell = by_label
            .get(slot.cell.as_str())
            .ok_or_else(|| format!("unknown cell {}", slot.cell))?;
        let old = Source::new(SourceId(0), cell.pre_source.clone());
        let post = Source::new(SourceId(1), cell.post_source.clone());
        let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;
        let hook = ReferenceOracle::new(reference);
        let old_state = build_initial_state(&mechanism, &old).map_err(|f| format!("build_initial_state: {f:?}"))?;
        crate::phases::arm();
        let report = run_update_timed(
            &mechanism,
            &old,
            &post,
            &cell.edit,
            old_state,
            &clock,
            &hook,
        );
        let phases = crate::phases::disarm();
        let timing = match &report.measurement {
            LaneMeasurement::Timing(t) => t,
            other => return Err(format!("unexpected lane {other:?}")),
        };
        let total = known(&timing.total_ns);
        let disjoint = phases.disjoint_sum();
        let mut row = serde_json::json!({
            "record": "observation",
            "lane": "u-phase",
            "session": session,
            "round": slot.round,
            "ordinal": slot.ordinal,
            "label": slot.label,
            "cell": cell.label,
            "n_bytes": cell.n_bytes,
            "m_blocks": cell.m_blocks,
            "case_id_hex": cell.case_id_hex,
            "sample_kind": if slot.round < warmup { "warmup" } else { "measured" },
            "measured_ordinal": slot.round.saturating_sub(warmup),
            "execution_status": format!("{:?}", report.execution_status),
            "correctness_status": format!("{:?}", report.correctness_status),
            "prepare_ns": obs_u64(&timing.prepare_ns),
            "native_ns": obs_u64(&timing.native_ns),
            "total_ns": obs_u64(&timing.total_ns),
            "result_checksum": report.result_checksum,
            "phase_disjoint_sum_ns": disjoint,
            "phase_residual_ns": total.map(|t| t as i64 - disjoint as i64),
            "hook_inclusive_ns": phases.hook(),
            "phase_clock_reads": phases.clock_reads,
        });
        for phase in crate::phases::Phase::ALL {
            row[phase.name()] = serde_json::Value::from(phases.get(phase));
        }
        emit(&mut out, &row)?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

#[cfg(not(feature = "phases"))]
fn cmd_run_phase(_flags: &[String]) -> Result<(), String> {
    Err("run-phase requires a binary built with --features phases".to_string())
}

// ---------------------------------------------------------------------------
// Lane C — W_COUNTERS
// ---------------------------------------------------------------------------

#[cfg(feature = "counters")]
fn cmd_run_counters(flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let mut out = out_writer(flags, "work-counters")?;
    emit(
        &mut out,
        &receipt(
            "work-counters",
            serde_json::json!({
                "session": session,
                "mechanism": crate::alg::H4DIAG_MECHANISM_ID,
                "note": "no headline timing in this lane; the counters explain work growth, not U_PLAIN time",
            }),
        )?,
    )?;

    let mechanism = H4Diag::new(Variant::A0);
    for cell in all_cells()? {
        let old = Source::new(SourceId(0), cell.pre_source.clone());
        let post = Source::new(SourceId(1), cell.post_source.clone());
        let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;
        let hook = ReferenceOracle::new(reference);
        let mut frozen = WorkCounters::all_unknown();
        let old_state = build_initial_state(&mechanism, &old).map_err(|f| format!("build_initial_state: {f:?}"))?;
        crate::counters::reset();
        let report = run_update_attributed(
            &mechanism,
            &old,
            &post,
            &cell.edit,
            old_state,
            &mut frozen,
            &hook,
        );
        let diag = crate::counters::snapshot();
        let mut row = serde_json::json!({
            "record": "observation",
            "lane": "work-counters",
            "session": session,
            "cell": cell.label,
            "n_bytes": cell.n_bytes,
            "m_blocks": cell.m_blocks,
            "case_id_hex": cell.case_id_hex,
            "execution_status": format!("{:?}", report.execution_status),
            "correctness_status": format!("{:?}", report.correctness_status),
            "result_checksum": report.result_checksum,
            "frozen_work_counters": {
                "blocks_reparsed": obs_u64(&frozen.blocks_reparsed),
                "nodes_rebuilt": obs_u64(&frozen.nodes_rebuilt),
                "nodes_reused": obs_u64(&frozen.nodes_reused),
                "metadata_records_touched": obs_u64(&frozen.metadata_records_touched),
                "unique_old_source_bytes": obs_u64(&frozen.unique_old_source_bytes),
                "unique_post_source_bytes": obs_u64(&frozen.unique_post_source_bytes),
                "unique_source_bytes": obs_u64(&frozen.unique_source_bytes),
                "source_bytes_inspected_total": obs_u64(&frozen.source_bytes_inspected_total),
                "restart_distance": obs_u64(&frozen.restart_distance),
                "convergence_distance": obs_u64(&frozen.convergence_distance),
            },
            "diag_counters": diag.to_json(),
        });
        row["counters_row"] = serde_json::Value::from(diag.row());
        emit(&mut out, &row)?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

#[cfg(not(feature = "counters"))]
fn cmd_run_counters(_flags: &[String]) -> Result<(), String> {
    Err("run-counters requires a binary built with --features counters".to_string())
}

// ---------------------------------------------------------------------------
// Ablations (A0 / A0-dup / Adefs / Adrop / Acapacity), one interleaved schedule
// ---------------------------------------------------------------------------

fn variant_for_label(label: &str) -> Result<Variant, String> {
    match label {
        "A0" | "A0-dup" => Ok(Variant::A0),
        "Adefs" => Ok(Variant::ADefs),
        "Adrop" => Ok(Variant::ADrop),
        "Acapacity" => Ok(Variant::ACapacity),
        other => Err(format!("unknown ablation label {other:?}")),
    }
}

fn cmd_run_ablations(flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let reps = flag_u32(flags, "--reps", MEASURED_ITERATIONS);
    let warmup = flag_u32(flags, "--warmup", WARMUP_ITERATIONS);
    let mut out = out_writer(flags, "ablations")?;
    emit(
        &mut out,
        &receipt(
            "ablations",
            serde_json::json!({
                "session": session,
                "schedule_seed": schedule::lane_seed("ablations", session),
                "labels": ABLATION_LABELS,
                "note": "A0, the identical A0-duplicate control, and every ablation are interleaved inside each round",
            }),
        )?,
    )?;

    let cells = all_cells()?;
    let labels: Vec<String> = ABLATION_LABELS.iter().map(|s| s.to_string()).collect();
    let slots = schedule::build(
        &cell_labels(),
        &labels,
        warmup + reps,
        schedule::lane_seed("ablations", session),
    );
    write_schedule(flags, &slots)?;

    let clock = InstantClock::new();
    let by_label: std::collections::HashMap<&str, &Cell> =
        cells.iter().map(|c| (c.label.as_str(), c)).collect();

    for slot in &slots {
        let cell = by_label
            .get(slot.cell.as_str())
            .ok_or_else(|| format!("unknown cell {}", slot.cell))?;
        let variant = variant_for_label(&slot.label)?;
        let mechanism = H4Diag::new(variant);
        let old = Source::new(SourceId(0), cell.pre_source.clone());
        let post = Source::new(SourceId(1), cell.post_source.clone());
        let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;
        let hook = ReferenceOracle::new(reference);
        let old_state = build_initial_state(&mechanism, &old).map_err(|f| format!("build_initial_state: {f:?}"))?;
        let report = run_update_timed(
            &mechanism,
            &old,
            &post,
            &cell.edit,
            old_state,
            &clock,
            &hook,
        );
        let timing = match &report.measurement {
            LaneMeasurement::Timing(t) => t,
            other => return Err(format!("unexpected lane {other:?}")),
        };

        // Post-timer drain: `Adrop` destroys the parked old state here; every
        // other variant runs the identical code path with nothing parked
        // (the empty-drain control, Issue #50 §10).
        let drain_start = Instant::now();
        let drain = deferred::drain_counted();
        let drain_ns = u64::try_from(drain_start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        black_box(&drain);
        let inventory = deferred::inventory();

        emit(
            &mut out,
            &serde_json::json!({
                "record": "observation",
                "lane": "ablations",
                "session": session,
                "round": slot.round,
                "ordinal": slot.ordinal,
                "label": slot.label,
                "variant": variant.label(),
                "cell": cell.label,
                "n_bytes": cell.n_bytes,
                "m_blocks": cell.m_blocks,
                "case_id_hex": cell.case_id_hex,
                "sample_kind": if slot.round < warmup { "warmup" } else { "measured" },
                "measured_ordinal": slot.round.saturating_sub(warmup),
                "execution_status": format!("{:?}", report.execution_status),
                "correctness_status": format!("{:?}", report.correctness_status),
                "failure": report.failure.as_ref().map(|f| format!("{f:?}")),
                "prepare_ns": obs_u64(&timing.prepare_ns),
                "native_ns": obs_u64(&timing.native_ns),
                "total_ns": obs_u64(&timing.total_ns),
                "result_checksum": report.result_checksum,
                "drain_ns": drain_ns,
                "drain": drain.to_json(),
                "retirement_inventory": inventory.to_json(),
            }),
        )?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Lane D — A_ALLOCATOR
// ---------------------------------------------------------------------------

#[cfg(feature = "allocator")]
fn cmd_run_alloc(flags: &[String]) -> Result<(), String> {
    let session = flag_u32(flags, "--session", 0);
    let reps = flag_u32(flags, "--reps", MEASURED_ITERATIONS);
    let warmup = flag_u32(flags, "--warmup", WARMUP_ITERATIONS);
    let mut out = out_writer(flags, "allocator")?;
    emit(
        &mut out,
        &receipt(
            "allocator",
            serde_json::json!({
                "session": session,
                "schedule_seed": schedule::lane_seed("allocator", session),
                "mechanism": crate::alg::H4DIAG_MECHANISM_ID,
                "allocator": "System wrapped by CountingAllocator (requested payload bytes)",
                "note": "allocator-lane wall time is NOT U_PLAIN and is never mixed into it",
            }),
        )?,
    )?;

    let cells = all_cells()?;
    let labels: Vec<String> = vec!["A0".to_string()];
    let slots = schedule::build(
        &cell_labels(),
        &labels,
        warmup + reps,
        schedule::lane_seed("allocator", session),
    );
    write_schedule(flags, &slots)?;

    let mechanism = H4Diag::new(Variant::A0);
    let clock = InstantClock::new();
    let by_label: std::collections::HashMap<&str, &Cell> =
        cells.iter().map(|c| (c.label.as_str(), c)).collect();

    for slot in &slots {
        let cell = by_label
            .get(slot.cell.as_str())
            .ok_or_else(|| format!("unknown cell {}", slot.cell))?;
        let old = Source::new(SourceId(0), cell.pre_source.clone());
        let post = Source::new(SourceId(1), cell.post_source.clone());
        let reference = markit_mdbench_campaign2::exec::reference_for(&cell.post_source)?;
        let hook = ReferenceOracle::new(reference);
        let old_state = build_initial_state(&mechanism, &old).map_err(|f| format!("build_initial_state: {f:?}"))?;
        let wall_start = Instant::now();
        crate::allocstat::arm();
        let report = run_update_timed(
            &mechanism,
            &old,
            &post,
            &cell.edit,
            old_state,
            &clock,
            &hook,
        );
        let window = crate::allocstat::disarm();
        let wall_ns = u64::try_from(wall_start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let timing = match &report.measurement {
            LaneMeasurement::Timing(t) => t,
            other => return Err(format!("unexpected lane {other:?}")),
        };
        emit(
            &mut out,
            &serde_json::json!({
                "record": "observation",
                "lane": "allocator",
                "session": session,
                "round": slot.round,
                "ordinal": slot.ordinal,
                "label": slot.label,
                "cell": cell.label,
                "n_bytes": cell.n_bytes,
                "m_blocks": cell.m_blocks,
                "case_id_hex": cell.case_id_hex,
                "sample_kind": if slot.round < warmup { "warmup" } else { "measured" },
                "measured_ordinal": slot.round.saturating_sub(warmup),
                "execution_status": format!("{:?}", report.execution_status),
                "correctness_status": format!("{:?}", report.correctness_status),
                "result_checksum": report.result_checksum,
                "alloc_lane_wall_ns": wall_ns,
                "alloc_lane_runner_total_ns": obs_u64(&timing.total_ns),
                "allocator": window.to_json(),
                "allocator_row": window.row(),
            }),
        )?;
    }
    out.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

#[cfg(not(feature = "allocator"))]
fn cmd_run_alloc(_flags: &[String]) -> Result<(), String> {
    Err("run-alloc requires a binary built with --features allocator".to_string())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn usage() {
    eprintln!(
        "usage: mdbench-h4diag <subcommand> [flags]\n\
         \n\
         subcommands:\n\
         \x20 cells               write the frozen #50 cell facts\n\
         \x20 preflight           verify cells + frozen workload invariants\n\
         \x20 verify-equivalence  prove the diagnostic copy == original H4\n\
         \x20 run-plain           U_PLAIN  (original H4, frozen timers)\n\
         \x20 run-phase           U_PHASE  (needs --features phases)\n\
         \x20 run-counters        W_COUNTERS (needs --features counters)\n\
         \x20 run-ablations       A0 / A0-dup / Adefs / Adrop / Acapacity\n\
         \x20 run-alloc           A_ALLOCATOR (needs --features allocator)\n\
         \n\
         flags: --session N --out F --reps R --warmup W --schedule-out F"
    );
}

/// CLI entry point shared by every diagnostic binary.
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(subcommand) = args.get(1).cloned() else {
        usage();
        return ExitCode::from(2);
    };
    let flags: Vec<String> = args[2..].to_vec();
    let result = match subcommand.as_str() {
        "cells" => cmd_cells(&flags),
        "preflight" => cmd_preflight(&flags),
        "verify-equivalence" => cmd_verify_equivalence(&flags),
        "run-plain" => cmd_run_plain(&flags),
        "run-phase" => cmd_run_phase(&flags),
        "run-counters" => cmd_run_counters(&flags),
        "run-ablations" => cmd_run_ablations(&flags),
        "run-alloc" => cmd_run_alloc(&flags),
        "receipt" => {
            let lane = flag_value(&flags, "--lane").unwrap_or_else(|| "unknown".to_string());
            let mut out = match out_writer(&flags, "receipt") {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(1);
                }
            };
            receipt(&lane, serde_json::json!({})).and_then(|r| emit(&mut out, &r))
        }
        other => {
            eprintln!("unknown subcommand {other:?}");
            usage();
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

/// Helper used by the binaries to locate the benchmark root (unused by
/// the current subcommands, kept for the run scripts' convenience).
pub fn benchmark_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Marker type so `CanonicalEdit` stays referenced in every feature
/// configuration (it appears only in lane-specific signatures otherwise).
pub type EditArg = CanonicalEdit;

/// Marker so the failure type stays referenced in every configuration.
pub type FailureArg = FailureStatus;
