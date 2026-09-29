//! Issue #100 L2 profiling-only repeated-execution harness.
//!
//! ```text
//! mdbench-horse-a-v2-l2 --cell E6-1 --arm P1 --reps 100 --warmup 20
//! ```
//!
//! Repeats the exact frozen #98 decomposition probe inside a
//! `#[inline(never)]` marker region so perf sees enough samples:
//!
//! ```text
//! P1  horse_a::full_build(post) alone        (no old state, no retirement)
//! P0  H0 full_parse(post) + complete alone   (no old state involved)
//! ```
//!
//! Operation semantics match `horse-a-v2-diag::decompose` exactly
//! (build -> black_box -> drop per iteration). The drop stays INSIDE the
//! marker region because a sampler cannot exclude it by time; the
//! analysis stage separates teardown contexts by call path instead, so
//! the construction attribution stays comparable to the untimed-drop
//! probe semantics. Wall time printed here is informational only.
//!
//! The frame hierarchy `mdbench_l2_region -> l2_p1_build|l2_p0_parse ->
//! full_build|full_parse` is a frozen analysis contract: the folding
//! stage detects frame order and filters setup samples by these names.

use std::hint::black_box;

use markit_mdbench_common::{Mechanism, MechanismContext, NoopWorkSink, Source, SourceId};
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::structural::NoopHorseAStructuralSink;
use markit_mdbench_horse_a_v2_diag::cells::frozen_cells;

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
enum Arm {
    P1,
    P0,
}

impl Arm {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "P1" => Ok(Arm::P1),
            "P0" => Ok(Arm::P0),
            other => Err(format!("unknown arm {other:?} (expected P1|P0)")),
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Arm::P1 => "P1_FULL_BUILD_ONLY",
            Arm::P0 => "P0_H0_FULL_PARSE_ONLY",
        }
    }
}

#[derive(serde::Serialize)]
struct Receipt {
    cell_id: String,
    arm: &'static str,
    warmup_ops: u64,
    region_ops: u64,
    /// Informational only — NOT benchmark evidence (P-LANE, see crate docs).
    region_wall_ns: u128,
    mean_ns_per_op_info_only: u128,
}

fn main() {
    let mut cell_arg: Option<String> = None;
    let mut arm_arg: Option<String> = None;
    let mut reps: u64 = 0;
    let mut warmup: u64 = 20;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--cell" => cell_arg = Some(args.next().expect("--cell value")),
            "--arm" => arm_arg = Some(args.next().expect("--arm value")),
            "--reps" => reps = args.next().expect("--reps value").parse().expect("--reps integer"),
            "--warmup" => warmup = args.next().expect("--warmup value").parse().expect("--warmup integer"),
            other => panic!("unknown argument {other:?}"),
        }
    }
    let cell_arg = cell_arg.expect("--cell is required");
    let arm = Arm::parse(&arm_arg.expect("--arm is required")).unwrap();
    assert!(reps > 0, "--reps must be > 0");

    // The frozen #98 manifest, constructed exactly as sealed (byte-exact).
    let mut cells = frozen_cells();
    let idx = cells
        .iter()
        .position(|c| c.id.starts_with(&cell_arg))
        .unwrap_or_else(|| panic!("cell {cell_arg:?} not in the frozen #98 manifest"));
    let cell = cells.swap_remove(idx);
    let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
    let post_source = cell
        .edit()
        .apply(&pre_source, SourceId(1))
        .expect("frozen cell edit must apply");

    // Warmup runs OUTSIDE the marker region (excluded from attribution by
    // the marker-frame filter in the analysis stage).
    match arm {
        Arm::P1 => l2_p1_build(black_box(&post_source), warmup),
        Arm::P0 => l2_p0_parse(black_box(&post_source), warmup),
    }

    let t0 = std::time::Instant::now();
    profile_region(black_box(&post_source), arm, reps);
    let region_wall_ns = t0.elapsed().as_nanos();

    let receipt = Receipt {
        cell_id: cell.id.to_string(),
        arm: arm.as_str(),
        warmup_ops: warmup,
        region_ops: reps,
        region_wall_ns,
        mean_ns_per_op_info_only: region_wall_ns / u128::from(reps),
    };
    println!("{}", serde_json::to_string(&receipt).expect("receipt serializes"));
}

/// The sampled marker region. All in-region frames root at this symbol,
/// which is how the analysis stage excludes setup/warmup samples.
#[inline(never)]
fn profile_region(post_source: &Source, arm: Arm, reps: u64) {
    match arm {
        Arm::P1 => l2_p1_build(post_source, reps),
        Arm::P0 => l2_p0_parse(post_source, reps),
    }
}

/// P1_FULL_BUILD_ONLY, verbatim probe semantics from #98 decompose:
/// parse + payload materialization + span rebase + coverage +
/// certificates + RefTable + AVL bulk build; NO old-state retirement,
/// NO validation.
#[inline(never)]
fn l2_p1_build(post_source: &Source, reps: u64) {
    for _ in 0..reps {
        let mut sink = NoopWorkSink;
        let built = full_build(post_source, &mut sink, &mut NoopHorseAStructuralSink)
            .expect("P1 full_build failed");
        black_box(&built);
        drop(built);
    }
}

/// P0_H0_FULL_PARSE_ONLY, verbatim probe semantics from #98 decompose:
/// parse + NormalizedDocument construction + the trivial completion
/// seal; NO old-state drop.
#[inline(never)]
fn l2_p0_parse(post_source: &Source, reps: u64) {
    let h0 = FullRebuildMechanism::new();
    for _ in 0..reps {
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let pending = h0.full_parse(post_source, &mut cx).expect("P0 full_parse failed");
        let completed = h0.complete(pending).expect("P0 complete failed");
        black_box(&completed.state);
        drop(completed);
    }
}
