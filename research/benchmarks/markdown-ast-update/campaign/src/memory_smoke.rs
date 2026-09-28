//! NON_RESEARCH memory-lane smoke (#76 Gate B instrumentation validation).
//!
//! Proves, without collecting ANY comparative latency results, that the
//! actual campaign measurement wiring can measure the four frozen metric
//! families on all SIX horses:
//!
//! ```text
//! incremental latency   run_update_timed: T_prepare + T_native with
//!                       completion/seal + black_box inside T_native
//! initial build         run_full_parse_timed + build_initial_state
//! allocation count/bytes  run_update_memory + AllocReporter over the
//!                         process CountingAllocator (dedicated binary)
//! retained memory       AllocReporter window closing strictly after the
//!                       sealed state is black_boxed and still alive
//! ```
//!
//! The values printed here are NON_RESEARCH smoke observations carrying
//! the `CAMPAIGN_MEMORY_SMOKE/NON_RESEARCH_RESULT` provenance: they
//! validate instruments on one predetermined micro case, never
//! mechanisms, and must never be quoted as horse performance. Nothing
//! here writes campaign raw rows.

use std::hint::black_box;

use markit_mdbench_common::{
    CanonicalEdit, CorrectnessStatus, ExecutionStatus, Mechanism, Observed, ResultChecksum, Source,
    SourceId,
};
use markit_mdbench_instrumentation::{
    AllocReporter, InstantClock, LaneMeasurement, MemoryReporter, TimingRecord,
};
use markit_mdbench_oracle::{validate_normalized, NormalizeV1, ReferenceOracle};
use markit_mdbench_runner::orchestrate::{
    build_initial_state, run_full_parse_timed, run_update_memory, run_update_timed,
};

pub const MEMORY_SMOKE_PROVENANCE: &str = "CAMPAIGN_MEMORY_SMOKE/NON_RESEARCH_RESULT";

/// One predetermined micro source (small valid BENCH-GRAMMAR-v1 input;
/// the smoke validates instruments, never mechanisms).
const MICRO_SOURCE: &str = concat!(
    "# Alpha heading\n",
    "\n",
    "Leading paragraph with *emphasis* and `code`.\n",
    "\n",
    "- first item\n",
    "- second item\n",
    "\n",
    "```rust\n",
    "fn fenced() {}\n",
    "```\n",
    "\n",
    "Trailing paragraph.\n",
);

/// The one canonical edit: replace the two list items with three. The
/// post source is materialized through the frozen `edit.apply` host
/// precondition, exactly like the campaign's `edit_case_sources`.
fn micro_case() -> (Source, Source, CanonicalEdit) {
    let old = Source::new(SourceId(0), MICRO_SOURCE.to_string());
    let region = "- first item\n- second item";
    let start = MICRO_SOURCE
        .find(region)
        .expect("micro edit anchor present");
    let end = start + region.len();
    let edit = CanonicalEdit::new(start, end, "- one\n- two\n- three\n".to_string())
        .expect("micro edit is well-formed");
    let post = edit.apply(&old, SourceId(1)).expect("micro edit applies");
    (old, post, edit)
}

pub struct SmokeFacts {
    pub label: &'static str,
    initial_build_ns: u64,
    prepare_ns: u64,
    native_ns: u64,
    build_alloc_count: u64,
    build_allocated_bytes: u64,
    build_retained_bytes: u64,
    update_alloc_count: u64,
    update_allocated_bytes: u64,
    update_peak_bytes: u64,
    update_retained_bytes: u64,
    execution: ExecutionStatus,
    correctness: CorrectnessStatus,
}

fn known_ns(slot: &str, v: Observed<u64>) -> Result<u64, String> {
    match v {
        Observed::Known(v) => Ok(v),
        Observed::NotApplicable => Ok(0), // FULL_PARSE T_prepare is NotApplicable by identity
        Observed::Unknown => Err(format!("{slot} not measured")),
    }
}

fn known_mem(label: &str, slot: &str, v: Observed<u64>) -> Result<u64, String> {
    match v {
        Observed::Known(v) => Ok(v),
        _ => Err(format!("{label}: {slot} not measured")),
    }
}

fn extract(record: &TimingRecord) -> Result<(u64, u64), String> {
    Ok((
        known_ns("T_prepare", record.prepare_ns)?,
        known_ns("T_native", record.native_ns)?,
    ))
}

fn smoke_horse<M>(
    label: &'static str,
    mechanism: &M,
    clock: &InstantClock,
    reporter: &AllocReporter,
) -> Result<SmokeFacts, String>
where
    M: Mechanism,
    M::State: NormalizeV1 + ResultChecksum,
{
    let (old, post, edit) = micro_case();

    // Correctness authority: H0 clean parse, exactly like the campaign
    // cells (once, outside every timer). FULL_PARSE verifies against the
    // OLD source; UPDATE verifies against the POST source.
    let reference_old = markit_mdbench_full_rebuild::parse_document(old.as_bytes());
    validate_normalized(&reference_old, None).map_err(|e| format!("reference gate: {e:?}"))?;
    let hook_old = ReferenceOracle::new(reference_old);
    let reference_post = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
    validate_normalized(&reference_post, None).map_err(|e| format!("post gate: {e:?}"))?;
    let hook = ReferenceOracle::new(reference_post);

    // Family 2 — INITIAL BUILD timing through the real FULL_PARSE lane
    // (parse + complete/seal + black_box inside the timer).
    let full = run_full_parse_timed(mechanism, &old, clock, &hook_old);
    if full.execution_status != ExecutionStatus::Pass {
        return Err(format!("full parse: {:?}", full.execution_status));
    }
    if full.correctness_status != CorrectnessStatus::Pass {
        return Err(format!(
            "full parse correctness: {:?}",
            full.correctness_status
        ));
    }
    let (_, initial_build_ns) = match &full.measurement {
        LaneMeasurement::Timing(t) => extract(t)?,
        other => return Err(format!("expected timing lane, got {other:?}")),
    };

    // Families 3+4 — allocation/retained during the real initial-state
    // construction (the untimed build the campaign performs per
    // iteration; windowed here for the M-LANE proof).
    let build_probe = reporter.begin_case();
    let built =
        build_initial_state(mechanism, &old).map_err(|f| format!("initial build: {f:?}"))?;
    black_box(&built);
    let build_record = reporter.end_case(build_probe);

    // Family 1 — incremental latency through the real timing lane.
    let timing = run_update_timed(mechanism, &old, &post, &edit, built, clock, &hook);
    if timing.execution_status != ExecutionStatus::Pass {
        return Err(format!("timed update: {:?}", timing.execution_status));
    }
    if timing.correctness_status != CorrectnessStatus::Pass {
        return Err(format!(
            "timed correctness: {:?}",
            timing.correctness_status
        ));
    }
    let (prepare_ns, native_ns) = match &timing.measurement {
        LaneMeasurement::Timing(t) => extract(t)?,
        other => return Err(format!("expected timing lane, got {other:?}")),
    };

    // Families 3+4 — allocation/retained during the UPDATE. The M-LANE
    // lane function opens its own per-case window and closes it strictly
    // after the sealed new state was black_boxed and still alive: the
    // frozen retained-memory lifecycle point.
    let fresh = build_initial_state(mechanism, &old).map_err(|f| format!("pre-state: {f:?}"))?;
    let memory = run_update_memory(mechanism, &old, &post, &edit, fresh, reporter, &hook);
    let update_record = match &memory.measurement {
        LaneMeasurement::Memory(r) => *r,
        other => return Err(format!("expected memory lane, got {other:?}")),
    };
    if memory.execution_status != ExecutionStatus::Pass {
        return Err(format!("memory update: {:?}", memory.execution_status));
    }
    if memory.correctness_status != CorrectnessStatus::Pass {
        return Err(format!(
            "memory correctness: {:?}",
            memory.correctness_status
        ));
    }

    Ok(SmokeFacts {
        label,
        initial_build_ns,
        prepare_ns,
        native_ns,
        build_alloc_count: known_mem(
            label,
            "build allocation_count",
            build_record.allocation_count,
        )?,
        build_allocated_bytes: known_mem(
            label,
            "build allocated_bytes",
            build_record.allocated_bytes,
        )?,
        build_retained_bytes: known_mem(
            label,
            "build retained_bytes",
            build_record.retained_bytes,
        )?,
        update_alloc_count: known_mem(
            label,
            "update allocation_count",
            update_record.allocation_count,
        )?,
        update_allocated_bytes: known_mem(
            label,
            "update allocated_bytes",
            update_record.allocated_bytes,
        )?,
        update_peak_bytes: known_mem(label, "update peak_bytes", update_record.peak_bytes)?,
        update_retained_bytes: known_mem(
            label,
            "update retained_bytes",
            update_record.retained_bytes,
        )?,
        execution: memory.execution_status,
        correctness: memory.correctness_status,
    })
}

/// Run the memory smoke over all six horses through the real lane
/// functions. Returns the per-horse facts.
pub fn run_memory_smoke() -> Result<Vec<SmokeFacts>, String> {
    let clock = InstantClock::new();
    let reporter = AllocReporter::new();

    macro_rules! arm {
        ($label:literal, $mech:expr) => {
            smoke_horse($label, &$mech, &clock, &reporter)?
        };
    }

    Ok(vec![
        arm!(
            "H0",
            markit_mdbench_full_rebuild::FullRebuildMechanism::new()
        ),
        arm!("H1", markit_mdbench_block_local::BlockLocalMechanism::new()),
        arm!(
            "H2",
            markit_mdbench_fragment_reuse::FragmentReuseMechanism::new()
        ),
        arm!(
            "H3",
            markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new()
        ),
        arm!(
            "H4",
            markit_mdbench_restart_convergence::RestartConvergenceMechanism::new()
        ),
        arm!("HorseA", markit_mdbench_horse_a::HorseAMechanism::new()),
    ])
}

/// Validate the smoke facts: instruments must have measured non-trivial
/// values for every horse, and windows must be internally consistent.
pub fn check_facts(facts: &[SmokeFacts]) -> Result<(), String> {
    for f in facts {
        if f.execution != ExecutionStatus::Pass || f.correctness != CorrectnessStatus::Pass {
            return Err(format!("{}: run did not pass", f.label));
        }
        if f.native_ns == 0 {
            return Err(format!("{}: T_native not measured", f.label));
        }
        if f.build_alloc_count == 0 || f.build_allocated_bytes == 0 {
            return Err(format!(
                "{}: initial-build window measured no allocations",
                f.label
            ));
        }
        if f.build_retained_bytes == 0 {
            return Err(format!(
                "{}: initial-build window retained nothing",
                f.label
            ));
        }
        if f.update_alloc_count == 0 || f.update_allocated_bytes == 0 {
            return Err(format!(
                "{}: update window measured no allocations",
                f.label
            ));
        }
        if f.update_peak_bytes < f.update_retained_bytes {
            return Err(format!("{}: peak < retained is impossible", f.label));
        }
    }
    Ok(())
}

/// Print the NON_RESEARCH smoke table + verdict. Never machine-raw rows.
pub fn report(facts: &[SmokeFacts]) {
    println!("provenance: {MEMORY_SMOKE_PROVENANCE}");
    println!(
        "case: one predetermined micro source + one canonical edit; NOT a frozen campaign case"
    );
    println!("these numbers validate INSTRUMENTS only and are never decision-bearing");
    println!(
        "{:<8} {:>12} {:>12} {:>12} {:>15} {:>14} {:>12} {:>14} {:>14} {:>12}",
        "horse",
        "build_ns",
        "prepare_ns",
        "native_ns",
        "bld_alloc_cnt",
        "bld_alloc_B",
        "bld_ret_B",
        "upd_alloc_B",
        "upd_peak_B",
        "upd_ret_B"
    );
    for f in facts {
        println!(
            "{:<8} {:>12} {:>12} {:>12} {:>15} {:>14} {:>12} {:>14} {:>14} {:>12}",
            f.label,
            f.initial_build_ns,
            f.prepare_ns,
            f.native_ns,
            f.build_alloc_count,
            f.build_allocated_bytes,
            f.build_retained_bytes,
            f.update_allocated_bytes,
            f.update_peak_bytes,
            f.update_retained_bytes,
        );
    }
}

/// The binary entry contract: run, validate, print, verdict.
pub fn run_and_report() -> Result<(), String> {
    let facts = run_memory_smoke()?;
    check_facts(&facts)?;
    report(&facts);
    println!("MEMORY_SMOKE_PASS (NON_RESEARCH instrument validation only)");
    Ok(())
}
