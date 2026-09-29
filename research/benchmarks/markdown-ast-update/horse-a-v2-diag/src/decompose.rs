//! L1 responsibility-decomposition probes (H-B discrimination).
//!
//! The A-LANE counters show arm B (direct READY rebuild) and arm C (H0)
//! inspect the SAME bytes and build the SAME number of semantic nodes,
//! yet B's wall latency is ~3.4x C on the E6 cells. The frozen counters
//! cannot split that delta into its responsibilities. These probes
//! answer the split by timing frozen PUBLIC operations in isolation:
//!
//! ```text
//! P1_BUILD      horse_a::full_build(post) alone
//!               (parse + payload materialization + span rebase +
//!                coverage + certificates + RefTable + AVL bulk build;
//!                NO old-state retirement, NO validation)
//!
//! P0_H0_PARSE   FullRebuildMechanism::full_parse(post) alone
//!               (parse + NormalizedDocument construction; NO
//!                old-state drop)
//!
//! then, combined with the T-LANE arms:
//!   B - P1          = validation + old-state retirement
//!   P1 - P0         = Horse-A retained-READY construction over H0's
//!                     document construction (state-shape difference)
//!   C - P0          = H0's old-state drop + wrapper (context)
//! ```
//!
//! These are diagnostic probes on unmodified frozen public APIs, run
//! with the same warmup/round policy as the T-LANE. They are NOT new
//! comparator arms and NOT a mechanism change.

use markit_mdbench_common::{
    Mechanism, MechanismContext, NoopWorkSink, Source, SourceId,
};
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::structural::NoopHorseAStructuralSink;
use markit_mdbench_instrumentation::clock::{Clock, InstantClock};

use crate::cells::frozen_cells;
use crate::tlane::{quantile, ArmTiming, MEASURED_ROUNDS, WARMUP_ROUNDS};

pub const P1_BUILD: &str = "P1_FULL_BUILD_ONLY";
pub const P0_H0_PARSE: &str = "P0_H0_FULL_PARSE_ONLY";

#[derive(serde::Serialize)]
pub struct CellDecompose {
    pub cell_id: String,
    pub probes: Vec<ArmTiming>,
    /// derived responsibility split (nanoseconds, medians)
    pub derived_ns: std::collections::BTreeMap<String, u64>,
}

fn summarize(arm: &str, mut samples: Vec<u64>) -> ArmTiming {
    samples.sort_unstable();
    let n = samples.len() as u32;
    ArmTiming {
        arm: arm.to_string(),
        samples: n,
        median_ns: quantile(&samples, 0.5),
        p25_ns: quantile(&samples, 0.25),
        p75_ns: quantile(&samples, 0.75),
        min_ns: samples[0],
        mean_ns: samples.iter().sum::<u64>() / (n as u64).max(1),
    }
}

pub fn run() -> Result<Vec<CellDecompose>, String> {
    let clock = InstantClock::new();
    let h0 = FullRebuildMechanism::new();
    let mut out = Vec::new();

    for cell in frozen_cells() {
        let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
        let edit = cell.edit();
        let post_source = edit
            .apply(&pre_source, SourceId(1))
            .map_err(|e| format!("cell {}: post derivation: {e}", cell.id))?;

        let mut samples: std::collections::BTreeMap<&str, Vec<u64>> = std::collections::BTreeMap::new();
        samples.insert(P1_BUILD, Vec::new());
        samples.insert(P0_H0_PARSE, Vec::new());

        for round in 0..(WARMUP_ROUNDS + MEASURED_ROUNDS) {
            // P1 — Horse-A full build alone (no old state involved).
            let t0 = clock.now_nanos();
            let mut sink = NoopWorkSink;
            let built = full_build(&post_source, &mut sink, &mut NoopHorseAStructuralSink)
                .map_err(|e| format!("cell {}: P1 failed: {e}", cell.id))?;
            let t1 = clock.now_nanos();
            std::hint::black_box(&built);
            drop(built);
            if round >= WARMUP_ROUNDS {
                samples.get_mut(P1_BUILD).unwrap().push(t1 - t0);
            }

            // P0 — H0 full parse alone (no old state involved).
            let t0 = clock.now_nanos();
            let mut sink = NoopWorkSink;
            let mut cx = MechanismContext::new(&mut sink);
            let pending = h0
                .full_parse(&post_source, &mut cx)
                .map_err(|f| format!("cell {}: P0 failed: {f:?}", cell.id))?;
            let completed = h0.complete(pending).map_err(|f| format!("{f:?}"))?;
            let t1 = clock.now_nanos();
            std::hint::black_box(&completed.state);
            drop(completed);
            if round >= WARMUP_ROUNDS {
                samples.get_mut(P0_H0_PARSE).unwrap().push(t1 - t0);
            }
        }

        let p1_timing = summarize(P1_BUILD, samples.remove(P1_BUILD).unwrap());
        let p0_timing = summarize(P0_H0_PARSE, samples.remove(P0_H0_PARSE).unwrap());
        let mut derived = std::collections::BTreeMap::new();
        derived.insert("p1_minus_p0_horse_state_over_h0_doc".to_string(), p1_timing.median_ns.saturating_sub(p0_timing.median_ns));

        out.push(CellDecompose {
            cell_id: cell.id.to_string(),
            probes: vec![p1_timing, p0_timing],
            derived_ns: derived,
        });
    }
    Ok(out)
}
