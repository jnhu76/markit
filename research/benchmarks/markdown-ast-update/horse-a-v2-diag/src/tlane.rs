//! T-LANE (L1.7): minimally instrumented end-to-end comparator timing.
//!
//! Policy (recorded in every artifact):
//!
//! ```text
//! process model      one process, cells in manifest order
//! arm order          fixed per round (A, B, C, D as present), rounds
//!                    interleaved so slow drift affects all arms alike
//! warmup             30 full rounds, discarded
//! measured           ROUNDS (default 200) full rounds, every sample kept
//! timed window       the complete arm call: for A the whole frozen
//!                    update (stage + commit, incl. retirement); for B
//!                    validation + full build + retirement; for C/D the
//!                    frozen Mechanism phases prepare_update + update +
//!                    complete (T_total)
//! per-rep state      the pre-state is cloned OUTSIDE the timed window;
//!                    the produced state is black_boxed and dropped
//!                    OUTSIDE the timed window
//! statistics         median / p25 / p75 / min / mean over all samples
//! ```
//!
//! This is a small diagnostic panel, not a campaign: the numbers answer
//! only whether the selected legal comparator paths show meaningful
//! headroom on these cells.

use std::hint::black_box;

use markit_mdbench_common::{
    CanonicalEdit, Mechanism, MechanismContext, MechanismId, NoopWorkSink, Source, SourceId,
};
use markit_mdbench_fragment_reuse::FragmentReuseMechanism;
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_horse_a::direct_ready::direct_ready_rebuild;
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::state::ReadyDocument;
use markit_mdbench_horse_a::structural::NoopHorseAStructuralSink;
use markit_mdbench_horse_a::update::update;
use markit_mdbench_instrumentation::clock::{Clock, InstantClock};

use crate::cells::{frozen_cells, ARM_A, ARM_B, ARM_C, ARM_D};

pub const WARMUP_ROUNDS: u32 = 30;
pub const MEASURED_ROUNDS: u32 = 200;

#[derive(serde::Serialize)]
pub struct ArmTiming {
    pub arm: String,
    pub samples: u32,
    pub median_ns: u64,
    pub p25_ns: u64,
    pub p75_ns: u64,
    pub min_ns: u64,
    pub mean_ns: u64,
}

#[derive(serde::Serialize)]
pub struct CellTiming {
    pub cell_id: String,
    pub family: String,
    pub role: String,
    pub arms: Vec<ArmTiming>,
    /// median ratios vs arm A (same cell)
    pub vs_a: std::collections::BTreeMap<String, f64>,
}

pub fn quantile(sorted: &[u64], q: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize(arm: &str, mut samples: Vec<u64>) -> ArmTiming {
    samples.sort_unstable();
    let n = samples.len() as u32;
    let median = quantile(&samples, 0.5);
    let p25 = quantile(&samples, 0.25);
    let p75 = quantile(&samples, 0.75);
    let min = samples[0];
    let mean = samples.iter().sum::<u64>() / (n as u64).max(1);
    ArmTiming {
        arm: arm.to_string(),
        samples: n,
        median_ns: median,
        p25_ns: p25,
        p75_ns: p75,
        min_ns: min,
        mean_ns: mean,
    }
}

/// One full arm execution (untimed internals; caller times the call).
enum ArmRun {
    A,
    B,
    C,
    D,
}

fn run_arm_a(
    pre_state: ReadyDocument,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> Result<ReadyDocument, String> {
    let mut sink = NoopWorkSink;
    update(pre_state, pre, post, edit, &mut sink).map_err(|e| e.to_string())
}

fn run_arm_b(
    pre_state: ReadyDocument,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> Result<ReadyDocument, String> {
    let mut sink = NoopWorkSink;
    direct_ready_rebuild(pre_state, pre, post, edit, &mut sink, &mut NoopHorseAStructuralSink)
        .map_err(|e| e.to_string())
}

fn run_arm_c(
    mech: &FullRebuildMechanism,
    pre_state: <FullRebuildMechanism as Mechanism>::State,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> Result<<FullRebuildMechanism as Mechanism>::State, String> {
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let prepared = mech
        .prepare_update(pre, post, edit, &pre_state, &mut cx)
        .map_err(|f| format!("{f:?}"))?;
    let pending = mech
        .update(pre, post, edit, pre_state, prepared, &mut cx)
        .map_err(|f| format!("{f:?}"))?;
    let completed = mech.complete(pending).map_err(|f| format!("{f:?}"))?;
    Ok(completed.state)
}

fn run_arm_d(
    mech: &FragmentReuseMechanism,
    pre_state: <FragmentReuseMechanism as Mechanism>::State,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> Result<<FragmentReuseMechanism as Mechanism>::State, String> {
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let prepared = mech
        .prepare_update(pre, post, edit, &pre_state, &mut cx)
        .map_err(|f| format!("{f:?}"))?;
    let pending = mech
        .update(pre, post, edit, pre_state, prepared, &mut cx)
        .map_err(|f| format!("{f:?}"))?;
    let completed = mech.complete(pending).map_err(|f| format!("{f:?}"))?;
    Ok(completed.state)
}

pub fn run() -> Result<Vec<CellTiming>, String> {
    let clock = InstantClock::new();
    let mut out = Vec::new();

    for cell in frozen_cells() {
        let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
        let edit = cell.edit();
        let post_source = edit
            .apply(&pre_source, SourceId(1))
            .map_err(|e| format!("cell {}: post derivation: {e}", cell.id))?;

        // Identical fresh pre-states per mechanism (built once, cloned
        // per rep OUTSIDE the timed window).
        let mut sink = NoopWorkSink;
        let pre_horse = full_build(&pre_source, &mut sink, &mut NoopHorseAStructuralSink)
            .map_err(|e| format!("cell {}: pre full_build failed: {e}", cell.id))?;

        let h0 = FullRebuildMechanism::new();
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let h0_pre = h0
            .full_parse(&pre_source, &mut cx)
            .and_then(|p| h0.complete(p))
            .map_err(|f| format!("cell {}: H0 pre-state failed: {f:?}", cell.id))?
            .state;

        let h2 = FragmentReuseMechanism::new();
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let h2_pre = h2
            .full_parse(&pre_source, &mut cx)
            .and_then(|p| h2.complete(p))
            .map_err(|f| format!("cell {}: H2 pre-state failed: {f:?}", cell.id))?
            .state;

        let want_c = cell.arms.contains(&ARM_C);
        let want_d = cell.arms.contains(&ARM_D);

        let mut samples: std::collections::BTreeMap<&str, Vec<u64>> = std::collections::BTreeMap::new();
        for arm in cell.arms {
            samples.insert(arm, Vec::with_capacity(MEASURED_ROUNDS as usize));
        }

        let total_rounds = WARMUP_ROUNDS + MEASURED_ROUNDS;
        for round in 0..total_rounds {
            // fixed arm order per round; all arms in one round are
            // adjacent so thermal/allocator drift hits them alike
            for arm_name in cell.arms {
                let t0 = clock.now_nanos();
                let result: Result<(), String> = match *arm_name {
                    ARM_A => run_arm_a(pre_horse.clone(), &pre_source, &post_source, &edit)
                        .map(|s| {
                            black_box(&s);
                        }),
                    ARM_B => run_arm_b(pre_horse.clone(), &pre_source, &post_source, &edit)
                        .map(|s| {
                            black_box(&s);
                        }),
                    ARM_C if want_c => {
                        run_arm_c(&h0, h0_pre.clone(), &pre_source, &post_source, &edit).map(|s| {
                            black_box(&s);
                        })
                    }
                    ARM_D if want_d => {
                        run_arm_d(&h2, h2_pre.clone(), &pre_source, &post_source, &edit).map(|s| {
                            black_box(&s);
                        })
                    }
                    other => Err(format!("unknown arm {other}")),
                };
                let t1 = clock.now_nanos();
                result.map_err(|e| format!("cell {} arm {arm_name}: {e}", cell.id))?;
                if round >= WARMUP_ROUNDS {
                    samples.get_mut(arm_name).unwrap().push(t1 - t0);
                }
            }
        }

        let mut arms = Vec::new();
        for arm_name in cell.arms {
            arms.push(summarize(arm_name, samples.remove(arm_name).unwrap()));
        }
        let mut vs_a = std::collections::BTreeMap::new();
        if let Some(a) = arms.iter().find(|t| t.arm == ARM_A) {
            for t in &arms {
                if t.arm != ARM_A {
                    vs_a.insert(
                        t.arm.clone(),
                        a.median_ns as f64 / t.median_ns.max(1) as f64,
                    );
                }
            }
        }
        out.push(CellTiming {
            cell_id: cell.id.to_string(),
            family: cell.family.to_string(),
            role: cell.role.to_string(),
            arms,
            vs_a,
        });
    }
    Ok(out)
}

/// Mechanism ids of the arms (for the artifact header).
pub fn arm_mechanism_ids() -> std::collections::BTreeMap<String, MechanismId> {
    let mut m = std::collections::BTreeMap::new();
    m.insert(ARM_A.to_string(), MechanismId("horse-a-v1".into()));
    m.insert(
        ARM_B.to_string(),
        MechanismId("horse-a-v1-direct-ready-rebuild".into()),
    );
    m.insert(ARM_C.to_string(), FullRebuildMechanism::new().id());
    m.insert(ARM_D.to_string(), FragmentReuseMechanism::new().id());
    m
}
