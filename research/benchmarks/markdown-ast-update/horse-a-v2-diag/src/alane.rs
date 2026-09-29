//! A-LANE (L1.3): one un-timed attributed pass per cell/arm, reusing the
//! frozen counter surfaces:
//!
//! - every arm: common `WorkCounters` via `CounterSink` (the frozen
//!   ATTRIBUTION-SCHEMA-v2 — blocks reparsed, nodes rebuilt/reused,
//!   source-inspection unions and cumulative effort);
//! - Horse-A arms A/B additionally: `HORSE-A-STRUCTURAL-COUNTERS-v1`
//!   via the recording structural sink (the frozen v1 record; no new
//!   Horse-A counter was invented for #98).
//!
//! Derived quantities (computed here, from those frozen counters, never
//! charged inside the mechanism):
//!
//! - `repeated_post_bytes` — cumulative post-source inspection effort
//!   minus its unique union: bytes re-read after a first read (the E6
//!   forward-parse-then-full-build overlap when the full build entered);
//! - `full_build_entered` — arm A's own frozen route record;
//! - `arm_a_minus_b_*` — per-counter work excess of the normal path
//!   over the direct rebuild (both contain exactly one full build when
//!   A entered it, so the difference is the discarded incremental
//!   attempt — direct WORK evidence, not a wall-time delta).

use std::collections::BTreeMap;

use markit_mdbench_common::{
    CounterSink, Mechanism, MechanismContext, NoopWorkSink, Source, SourceId, WorkCounters,
};
use markit_mdbench_fragment_reuse::FragmentReuseMechanism;
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_horse_a::direct_ready::direct_ready_rebuild;
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::state::ReadyDocument;
use markit_mdbench_horse_a::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, RecordingHorseAStructuralSink,
};
use markit_mdbench_horse_a::update::update_with_structural;

use crate::cells::{frozen_cells, ARM_A, ARM_B, ARM_C, ARM_D};

/// Flatten the frozen structural record to JSON (the record itself is
/// not serde; its fields are public and fixed-size).
pub fn structural_to_json(c: &HorseAStructuralCountersV1) -> serde_json::Value {
    use markit_mdbench_horse_a::structural::Observed;
    let o = |v: Observed| match v {
        Observed::Unknown => serde_json::json!("UNKNOWN"),
        Observed::Known(n) => serde_json::json!(n),
    };
    serde_json::json!({
        "schema": c.schema,
        "m_old": o(c.m_old),
        "m_new": o(c.m_new),
        "h_old": o(c.h_old),
        "h_new": o(c.h_new),
        "restart_old": o(c.restart_old),
        "convergence_old": o(c.convergence_old),
        "convergence_new": o(c.convergence_new),
        "replace_lo": o(c.replace_lo),
        "replace_hi": o(c.replace_hi),
        "full_build_selected": o(c.full_build_selected),
        "full_build_reason": o(c.full_build_reason),
        "locate_node_visits": o(c.locate_node_visits),
        "safe_predecessor_node_visits": o(c.safe_predecessor_node_visits),
        "cursor_node_visits": o(c.cursor_node_visits),
        "fact_range_node_visits": o(c.fact_range_node_visits),
        "split_node_visits": o(c.split_node_visits),
        "pivot_extract_node_visits": o(c.pivot_extract_node_visits),
        "join_node_visits": o(c.join_node_visits),
        "bulk_build_node_visits": o(c.bulk_build_node_visits),
        "retire_node_visits": o(c.retire_node_visits),
        "sequence_link_writes": o(c.sequence_link_writes),
        "avl_rotations": o(c.avl_rotations),
        "aggregate_reads": o(c.aggregate_reads),
        "aggregate_writes": o(c.aggregate_writes),
        "certificate_reads": o(c.certificate_reads),
        "certificate_writes": o(c.certificate_writes),
        "candidate_checks": o(c.candidate_checks),
        "cursor_advances": o(c.cursor_advances),
        "owners_created": o(c.owners_created),
        "owners_removed": o(c.owners_removed),
        "fresh_payload_nodes_final": o(c.fresh_payload_nodes_final),
        "fresh_payload_nodes_temporary": o(c.fresh_payload_nodes_temporary),
        "payload_nodes_retired": o(c.payload_nodes_retired),
        "old_fact_owner_visits": o(c.old_fact_owner_visits),
        "old_facts_extracted": o(c.old_facts_extracted),
        "new_facts_extracted": o(c.new_facts_extracted),
        "facts_compared": o(c.facts_compared),
        "fact_compare_bytes": o(c.fact_compare_bytes),
        "reftable_entries_visited": o(c.reftable_entries_visited),
        "retirement_frames_entered": o(c.retirement_frames_entered),
        "max_retirement_depth": o(c.max_retirement_depth),
        "forbidden_prefix_sequential_enumeration": o(c.forbidden_prefix_sequential_enumeration),
        "forbidden_suffix_sequential_enumeration": o(c.forbidden_suffix_sequential_enumeration),
        "forbidden_unaffected_payload_inspections": o(c.forbidden_unaffected_payload_inspections),
        "forbidden_unaffected_coordinate_writes": o(c.forbidden_unaffected_coordinate_writes),
        "forbidden_unaffected_certificate_writes": o(c.forbidden_unaffected_certificate_writes),
        "forbidden_global_fact_recollection": o(c.forbidden_global_fact_recollection),
        "forbidden_unaffected_old_retirement": o(c.forbidden_unaffected_old_retirement),
        "forbidden_attribution_tree_walk": o(c.forbidden_attribution_tree_walk),
    })
}

#[derive(serde::Serialize)]
pub struct ArmAttribution {
    pub arm: String,
    pub work: WorkCounters,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_state_ok: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct CellAttribution {
    pub cell_id: String,
    pub family: String,
    pub role: String,
    pub arms: Vec<ArmAttribution>,
    /// Derived (A-minus-B work deltas; only where both arms ran and A
    /// entered the frozen full-build branch — see the module header).
    pub derived: serde_json::Value,
}

fn observed_u64(v: markit_mdbench_common::Observed<u64>) -> Option<u64> {
    match v {
        markit_mdbench_common::Observed::Known(n) => Some(n),
        _ => None,
    }
}

fn derived_block(a: &ArmAttribution, b: Option<&ArmAttribution>) -> serde_json::Value {
    let fb = a
        .structural
        .as_ref()
        .and_then(|s| s.get("full_build_selected"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let wa = &a.work;
    let mut derived = serde_json::json!({
        "arm_a_full_build_entered": fb == 1,
        "arm_a_full_build_reason": a.structural
            .as_ref()
            .and_then(|s| s.get("full_build_reason"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        "arm_a_repeated_post_bytes": observed_u64(wa.source_bytes_inspected_total)
            .zip(observed_u64(wa.unique_post_source_bytes))
            .map(|(t, u)| t.saturating_sub(u)),
        "arm_a_unique_post_bytes": observed_u64(wa.unique_post_source_bytes),
        "arm_a_total_inspection_bytes": observed_u64(wa.source_bytes_inspected_total),
    });
    if let Some(b) = b {
        let wb = &b.work;
        let d = |x: Option<u64>, y: Option<u64>| x.zip(y).map(|(x, y)| x.saturating_sub(y));
        derived["arm_a_minus_b"] = serde_json::json!({
            "blocks_reparsed": d(observed_u64(wa.blocks_reparsed), observed_u64(wb.blocks_reparsed)),
            "nodes_rebuilt": d(observed_u64(wa.nodes_rebuilt), observed_u64(wb.nodes_rebuilt)),
            "total_inspection_bytes": d(
                observed_u64(wa.source_bytes_inspected_total),
                observed_u64(wb.source_bytes_inspected_total),
            ),
            "owners_created": d(
                a.structural.as_ref().and_then(|s| s.get("owners_created")).and_then(|v| v.as_u64()),
                b.structural.as_ref().and_then(|s| s.get("owners_created")).and_then(|v| v.as_u64()),
            ),
        });
    }
    derived
}

pub fn run() -> Result<Vec<CellAttribution>, String> {
    let h0 = FullRebuildMechanism::new();
    let h2 = FragmentReuseMechanism::new();
    let mut out = Vec::new();

    for cell in frozen_cells() {
        let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
        let edit = cell.edit();
        let post_source = edit
            .apply(&pre_source, SourceId(1))
            .map_err(|e| format!("cell {}: post derivation: {e}", cell.id))?;

        let mut sink = NoopWorkSink;
        let pre_horse = full_build(&pre_source, &mut sink, &mut NoopHorseAStructuralSink)
            .map_err(|e| format!("cell {}: pre full_build failed: {e}", cell.id))?;
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let h0_pre = h0
            .full_parse(&pre_source, &mut cx)
            .and_then(|p| h0.complete(p))
            .map_err(|f| format!("cell {}: H0 pre: {f:?}", cell.id))?
            .state;
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let h2_pre = h2
            .full_parse(&pre_source, &mut cx)
            .and_then(|p| h2.complete(p))
            .map_err(|f| format!("cell {}: H2 pre: {f:?}", cell.id))?
            .state;

        let mut arms: Vec<ArmAttribution> = Vec::new();

        // Arm A — normal path with BOTH frozen recording surfaces.
        {
            let mut work = WorkCounters::all_unknown();
            let mut counter = CounterSink::new(&mut work);
            let mut structural = RecordingHorseAStructuralSink::new();
            let result: Result<ReadyDocument, String> =
                update_with_structural(
                    pre_horse.clone(),
                    &pre_source,
                    &post_source,
                    &edit,
                    &mut counter,
                    &mut structural,
                )
                .map_err(|e| e.to_string());
            counter.finalize_derived();
            let counters = structural.into_counters();
            let ok = result.is_ok();
            drop(result);
            arms.push(ArmAttribution {
                arm: ARM_A.to_string(),
                work,
                structural: Some(structural_to_json(&counters)),
                result_state_ok: Some(ok),
            });
        }

        // Arm B — direct READY rebuild control, same recording surfaces.
        {
            let mut work = WorkCounters::all_unknown();
            let mut counter = CounterSink::new(&mut work);
            let mut structural = RecordingHorseAStructuralSink::new();
            let result: Result<ReadyDocument, String> = direct_ready_rebuild(
                pre_horse.clone(),
                &pre_source,
                &post_source,
                &edit,
                &mut counter,
                &mut structural,
            )
            .map_err(|e| e.to_string());
            counter.finalize_derived();
            let counters = structural.into_counters();
            let ok = result.is_ok();
            drop(result);
            arms.push(ArmAttribution {
                arm: ARM_B.to_string(),
                work,
                structural: Some(structural_to_json(&counters)),
                result_state_ok: Some(ok),
            });
        }

        // Arm C — H0 reference (common counters only).
        if cell.arms.contains(&ARM_C) {
            let mut work = WorkCounters::all_unknown();
            let mut counter = CounterSink::new(&mut work);
            {
                let mut cx = MechanismContext::new(&mut counter);
                let prepared = h0
                    .prepare_update(&pre_source, &post_source, &edit, &h0_pre, &mut cx)
                    .map_err(|f| format!("{f:?}"))?;
                let pending = h0
                    .update(&pre_source, &post_source, &edit, h0_pre.clone(), prepared, &mut cx)
                    .map_err(|f| format!("{f:?}"))?;
                let completed = h0.complete(pending).map_err(|f| format!("{f:?}"))?;
                counter.finalize_derived();
                drop(completed);
            }
            arms.push(ArmAttribution {
                arm: ARM_C.to_string(),
                work,
                structural: None,
                result_state_ok: None,
            });
        }

        // Arm D — H2 reference (common counters only).
        if cell.arms.contains(&ARM_D) {
            let mut work = WorkCounters::all_unknown();
            let mut counter = CounterSink::new(&mut work);
            {
                let mut cx = MechanismContext::new(&mut counter);
                let prepared = h2
                    .prepare_update(&pre_source, &post_source, &edit, &h2_pre, &mut cx)
                    .map_err(|f| format!("{f:?}"))?;
                let pending = h2
                    .update(&pre_source, &post_source, &edit, h2_pre.clone(), prepared, &mut cx)
                    .map_err(|f| format!("{f:?}"))?;
                let completed = h2.complete(pending).map_err(|f| format!("{f:?}"))?;
                counter.finalize_derived();
                drop(completed);
            }
            arms.push(ArmAttribution {
                arm: ARM_D.to_string(),
                work,
                structural: None,
                result_state_ok: None,
            });
        }

        let a_ref = arms.iter().find(|t| t.arm == ARM_A);
        let b_ref = arms.iter().find(|t| t.arm == ARM_B);
        let derived = match (a_ref, b_ref) {
            (Some(a), Some(b)) => derived_block(a, Some(b)),
            (Some(a), None) => derived_block(a, None),
            _ => serde_json::json!({}),
        };

        out.push(CellAttribution {
            cell_id: cell.id.to_string(),
            family: cell.family.to_string(),
            role: cell.role.to_string(),
            arms,
            derived,
        });
    }
    Ok(out)
}

/// Compact per-cell work summary for the report (kept machine-complete
/// in the artifact; this is the readable projection).
pub fn compact_table(rows: &[CellAttribution]) -> BTreeMap<String, serde_json::Value> {
    let mut out = BTreeMap::new();
    for row in rows {
        let a = row.arms.iter().find(|t| t.arm == ARM_A);
        let b = row.arms.iter().find(|t| t.arm == ARM_B);
        let get = |arm: Option<&ArmAttribution>, f: fn(&WorkCounters) -> markit_mdbench_common::Observed<u64>| {
            arm.map(|x| observed_u64(f(&x.work))).flatten()
        };
        let _ = get;
        out.insert(
            row.cell_id.clone(),
            serde_json::json!({
                "a_blocks_reparsed": a.and_then(|x| observed_u64(x.work.blocks_reparsed)),
                "a_nodes_rebuilt": a.and_then(|x| observed_u64(x.work.nodes_rebuilt)),
                "a_total_inspected": a.and_then(|x| observed_u64(x.work.source_bytes_inspected_total)),
                "b_blocks_reparsed": b.and_then(|x| observed_u64(x.work.blocks_reparsed)),
                "b_nodes_rebuilt": b.and_then(|x| observed_u64(x.work.nodes_rebuilt)),
                "b_total_inspected": b.and_then(|x| observed_u64(x.work.source_bytes_inspected_total)),
                "derived": row.derived,
            }),
        );
    }
    out
}
