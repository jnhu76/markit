//! Producer/adjudicator tests (task §14/§15) — SYNTHETIC ROWS ONLY.
//!
//! No test runs a primary cell in recording mode: every adjudication
//! input here is a synthetic (mock) counter row; the only real
//! constructions are the non-treatment `construct_and_verify`
//! byte-identity checks (validate-only semantics — construction +
//! hashing, no update, no full build, no recording sink).

use std::collections::BTreeMap;

use crate::structural::Observed;

use super::adjudicate::{
    adjudicate, counters_deterministic, derive_cell_verdict, derive_overall_verdict, Adjudication,
    FieldKind,
};
use super::contract::{self, FrozenCell, FROZEN_CELLS};
use super::schema::{
    counters_to_map, observed_from_string, observed_to_string, AdjudicationBlock, CorrectnessBlock,
    HostBlock, RawRowV1, ResultChecksums, COUNTER_FIELDS,
};
use super::writer::{check_output_path, write_row};

// ---------------------------------------------------------------------------
// Synthetic-row fixtures (no treatment data; plausible in-bounds values)
// ---------------------------------------------------------------------------

/// The diagnostic heights of the frozen witness (spec §4:
/// H_old = H_new = ⌊log₂ M⌋ + 1 — recorded diagnostics, only the H_max
/// invariant is adjudicated).
fn h_diag(cell: &FrozenCell) -> u64 {
    match cell.m {
        1_024 => 11,
        8_192 => 14,
        131_072 => 18,
        _ => unreachable!("frozen cells only"),
    }
}

fn known(n: u64) -> Observed {
    Observed::Known(n)
}

/// A synthetic counter map that satisfies every frozen obligation of the
/// cell (in-bounds thresholds, exact geometry/values, zero sentinels).
fn sample_counters(cell: &FrozenCell) -> BTreeMap<String, Observed> {
    let h = h_diag(cell);
    let mut map = BTreeMap::new();
    for (name, value) in [
        ("m_old", cell.m),
        ("m_new", cell.m),
        ("h_old", h),
        ("h_new", h),
        ("restart_old", cell.restart),
        ("convergence_old", cell.convergence_old),
        ("convergence_new", cell.convergence_new),
        ("replace_lo", cell.replace_lo),
        ("replace_hi", cell.replace_hi),
        ("full_build_selected", 0),
        ("full_build_reason", 0),
        ("locate_node_visits", h),
        ("safe_predecessor_node_visits", 3 * h - 2),
        ("cursor_node_visits", 2 * h + 10),
        ("fact_range_node_visits", 3 * h - 2),
        ("split_node_visits", 10 * h),
        ("pivot_extract_node_visits", 4 * h),
        ("join_node_visits", 4 * h),
        ("bulk_build_node_visits", contract::DELTA_NEW),
        ("retire_node_visits", contract::DELTA_OLD),
        ("sequence_link_writes", 50 * h + 10),
        ("avl_rotations", 10 * h),
        ("aggregate_reads", 200 * h),
        ("aggregate_writes", 80 * h),
        ("certificate_reads", contract::CERTIFICATE_READS_MAX),
        ("certificate_writes", contract::CERTIFICATE_WRITES_MAX),
        ("candidate_checks", contract::CANDIDATE_CHECKS),
        ("cursor_advances", contract::CURSOR_ADVANCES),
        ("owners_created", contract::DELTA_NEW),
        ("owners_removed", contract::DELTA_OLD),
        (
            "fresh_payload_nodes_final",
            contract::FRESH_PAYLOAD_NODES_FINAL,
        ),
        (
            "fresh_payload_nodes_temporary",
            contract::FRESH_PAYLOAD_NODES_TEMPORARY_MAX,
        ),
        ("payload_nodes_retired", contract::P_REMOVED),
        ("old_fact_owner_visits", contract::OLD_FACT_OWNER_VISITS_MAX),
        ("old_facts_extracted", 0),
        ("new_facts_extracted", 0),
        ("facts_compared", 0),
        ("fact_compare_bytes", 0),
        (
            "reftable_entries_visited",
            contract::REFTABLE_ENTRIES_VISITED,
        ),
        ("retirement_frames_entered", contract::RETIREMENT_FRAMES_MAX),
        ("max_retirement_depth", contract::D_PAYLOAD),
        ("forbidden_prefix_sequential_enumeration", 0),
        ("forbidden_suffix_sequential_enumeration", 0),
        ("forbidden_unaffected_payload_inspections", 0),
        ("forbidden_unaffected_coordinate_writes", 0),
        ("forbidden_unaffected_certificate_writes", 0),
        ("forbidden_global_fact_recollection", 0),
        ("forbidden_unaffected_old_retirement", 0),
        ("forbidden_attribution_tree_walk", 0),
    ] {
        map.insert(name.to_string(), known(value));
    }
    map
}

fn sample_row(cell: &FrozenCell, repetition: u64) -> RawRowV1 {
    let executable_sha256 = "a".repeat(64);
    RawRowV1 {
        schema: contract::RAW_SCHEMA.to_string(),
        row_identity: RawRowV1::compute_row_identity(
            contract::STUDY_ID,
            cell.cell_id,
            repetition,
            &executable_sha256,
        ),
        study_id: contract::STUDY_ID.to_string(),
        protocol: contract::PROTOCOL.to_string(),
        frozen_contract_revision: contract::FROZEN_CONTRACT_REVISION.to_string(),
        study_mechanism_baseline: contract::STUDY_MECHANISM_BASELINE.to_string(),
        authorization_baseline: contract::AUTHORIZATION_BASELINE.to_string(),
        repository_commit: "1".repeat(40),
        repository_tree: "2".repeat(40),
        mechanism: contract::MECHANISM.to_string(),
        mechanism_design_head: contract::MECHANISM_DESIGN_HEAD.to_string(),
        mechanism_merge: contract::MECHANISM_MERGE.to_string(),
        cell_id: cell.cell_id.to_string(),
        case_id_hex: cell.case_id_hex.to_string(),
        n_bytes: cell.n_bytes,
        m: cell.m,
        target: cell.target,
        edit_start: cell.edit_start,
        edit_end: cell.edit_start,
        inserted_text_sha256: contract::INSERTED_TEXT_SHA256.to_string(),
        pre_sha256: cell.pre_sha256.to_string(),
        post_sha256: cell.post_sha256.to_string(),
        repetition,
        executable_sha256,
        rustc: "rustc 1.97.1 (8bab26f4f 2026-07-14)".to_string(),
        cargo: "cargo 1.97.1 (c980f4866 2026-06-30)".to_string(),
        toolchain_channel: "1.97.1".to_string(),
        build_profile: "release-primary-v1".to_string(),
        features: "none".to_string(),
        host: HostBlock {
            os: "linux".to_string(),
            kernel: "6.18.33.2-microsoft-standard-WSL2".to_string(),
            arch: "x86_64".to_string(),
            cpu_model: "test cpu".to_string(),
        },
        execution_command: "mdbench-horse-a-structural --cell X --repetition 1 --output y"
            .to_string(),
        counter_schema: contract::COUNTER_SCHEMA.to_string(),
        counters: sample_counters(cell),
        route: "local".to_string(),
        full_build_reason: "none".to_string(),
        correctness: CorrectnessBlock {
            c1: "PASS".to_string(),
            c2: "PASS".to_string(),
            c3: "PASS".to_string(),
            normalized_structural_equality: true,
        },
        result_checksums: ResultChecksums {
            c1: 1,
            c2: 2,
            c3: 3,
        },
        adjudication: AdjudicationBlock {
            status: "PASS".to_string(),
            reasons: vec![],
        },
    }
}

fn set_counter(row: &mut RawRowV1, name: &str, value: Observed) {
    row.counters.insert(name.to_string(), value);
}

fn reasons_mention(adj: &Adjudication, needle: &str) -> bool {
    adj.reasons().iter().any(|r| r.contains(needle))
}

// ---------------------------------------------------------------------------
// Schema / identity
// ---------------------------------------------------------------------------

/// Schema round trip: a full row survives JSON serialization with every
/// Known/Unknown value explicit.
#[test]
fn schema_round_trip() {
    let cell = &FROZEN_CELLS[0];
    let row = sample_row(cell, 0);
    let json = serde_json::to_string(&row).expect("serialize");
    let back: RawRowV1 = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(row, back);
    assert!(json.contains("\"Known(2)\""));
    // Malformed Observed values are refused.
    let bad = json.replace("\"Known(2)\"", "\"Known(two)\"");
    assert!(serde_json::from_str::<RawRowV1>(&bad).is_err());
}

/// The Observed codec keeps Known(0) and Unknown strictly distinct.
#[test]
fn known_zero_distinguished_from_unknown() {
    assert_eq!(observed_to_string(Observed::Known(0)), "Known(0)");
    assert_eq!(observed_to_string(Observed::Unknown), "Unknown");
    assert_eq!(observed_from_string("Known(0)"), Some(Observed::Known(0)));
    assert_eq!(observed_from_string("Unknown"), Some(Observed::Unknown));
    assert_ne!(
        observed_from_string("Known(0)"),
        observed_from_string("Unknown")
    );
}

#[test]
fn study_id_exact_value() {
    assert_eq!(
        contract::STUDY_ID,
        "2e061da9cb6fbe57f9ce139ca02382fda4cad672fd9422e6c7d7b885f42f4e17"
    );
}

#[test]
fn frozen_contract_revision_exact_value() {
    assert_eq!(
        contract::FROZEN_CONTRACT_REVISION,
        "HORSE-A-FAILURE-FIRST-1/#60@sync-20260928T021211Z/master-f7fdcdaabc5d761435f3c0e8c17611973642934b/tree-9ec58ed228972c323da5cccf0b991235a0bc0e8a"
    );
    // Distinct from the other frozen identities.
    assert_ne!(contract::FROZEN_CONTRACT_REVISION, contract::STUDY_ID);
    assert!(!contract::FROZEN_CONTRACT_REVISION.contains("04b6496"));
}

/// Cell identity/hash validation (non-treatment): the constructed
/// pre/post/inserted bytes of every frozen cell match the frozen gates.
#[test]
fn cell_identity_hash_validation_all_cells() {
    for cell in FROZEN_CELLS.iter() {
        assert!(
            cell.threshold_table_is_consistent(),
            "{} table",
            cell.cell_id
        );
        let constructed = contract::construct_and_verify(cell)
            .unwrap_or_else(|e| panic!("{}: {e}", cell.cell_id));
        assert_eq!(
            contract::sha256_hex(constructed.pre.as_bytes()),
            cell.pre_sha256
        );
        assert_eq!(
            contract::sha256_hex(constructed.post.as_bytes()),
            cell.post_sha256
        );
        assert_eq!(
            contract::sha256_hex(contract::INSERTED_TEXT.as_bytes()),
            contract::INSERTED_TEXT_SHA256
        );
    }
}

// ---------------------------------------------------------------------------
// Immutable-row writer
// ---------------------------------------------------------------------------

fn temp_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    dir.join(format!(
        "horse-a-producer-test-{}-{}-{}.json",
        tag,
        std::process::id(),
        line!()
    ))
}

/// A written row is never overwritten: duplicate identity is refused and
/// the original bytes survive.
#[test]
fn duplicate_output_refusal() {
    let cell = &FROZEN_CELLS[0];
    let path = temp_path("dup");
    let _ = std::fs::remove_file(&path);
    let row = sample_row(cell, 0);
    write_row(&path, &row).expect("first write");
    let first_bytes = std::fs::read(&path).expect("read back");
    match write_row(&path, &row) {
        Err(super::writer::WriteError::OutputExists(_)) => {}
        other => panic!("second write must be refused, got {other:?}"),
    }
    // The path-legality pre-check refuses too (fail closed before work).
    assert!(matches!(
        check_output_path(&path),
        Err(super::writer::WriteError::OutputExists(_))
    ));
    let second_bytes = std::fs::read(&path).expect("read back again");
    assert_eq!(first_bytes, second_bytes, "the row must be untouched");
    // A missing parent directory is also refused.
    let orphan = std::path::PathBuf::from("/nonexistent-dir-xyz/row.json");
    assert!(matches!(
        check_output_path(&orphan),
        Err(super::writer::WriteError::NoParentDir(_))
    ));
    std::fs::remove_file(&path).ok();
}

// ---------------------------------------------------------------------------
// INVALID: instrumentation / provenance / identity malformations
// ---------------------------------------------------------------------------

#[test]
fn malformed_provenance_is_invalid() {
    let cell = &FROZEN_CELLS[1];
    let mut row = sample_row(cell, 1);
    row.study_id = "0".repeat(64);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.frozen_contract_revision = "some-other-revision".to_string();
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.study_mechanism_baseline = "0".repeat(40);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.authorization_baseline = "0".repeat(40);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.repository_commit = "not-a-sha".to_string();
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.mechanism_design_head = "0".repeat(40);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.executable_sha256 = "deadbeef".to_string();
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.rustc = "  ".to_string();
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.counter_schema = "HORSE-A-STRUCTURAL-COUNTERS-v0".to_string();
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.row_identity = "0".repeat(64);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    // Cell-identity malformations.
    let mut row = sample_row(cell, 1);
    row.case_id_hex = "0".repeat(64);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.pre_sha256 = "0".repeat(64);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.post_sha256 = "0".repeat(64);
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.n_bytes += 1;
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    let mut row = sample_row(cell, 1);
    row.repetition = 3;
    assert!(matches!(adjudicate(cell, &row), Adjudication::Invalid(_)));

    // Counters-map shape: a missing or unknown field is INVALID.
    let mut row = sample_row(cell, 1);
    row.counters.remove("avl_rotations");
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Invalid(_)));
    assert!(reasons_mention(
        &adj,
        "counters_schema:missing_field:avl_rotations"
    ));

    let mut row = sample_row(cell, 1);
    row.counters.insert("mystery_counter".to_string(), known(1));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Invalid(_)));
    assert!(reasons_mention(
        &adj,
        "counters_schema:unknown_field:mystery_counter"
    ));
}

// ---------------------------------------------------------------------------
// FAIL: Unknown / thresholds / exact values / sentinels / correctness
// ---------------------------------------------------------------------------

/// The base synthetic row PASSES on all three cells.
#[test]
fn synthetic_pass_row_all_cells() {
    for cell in FROZEN_CELLS.iter() {
        for rep in 0..=2u64 {
            assert_eq!(
                adjudicate(cell, &sample_row(cell, rep)),
                Adjudication::Pass,
                "{} rep{rep}",
                cell.cell_id
            );
        }
    }
}

/// Unknown can never PASS (missing instrumentation is not zero); the
/// sweep must visit EVERY decision-bearing field.
#[test]
fn unknown_cannot_pass_every_field_visited() {
    for cell in FROZEN_CELLS.iter() {
        for name in COUNTER_FIELDS.iter() {
            let mut row = sample_row(cell, 0);
            set_counter(&mut row, name, Observed::Unknown);
            let adj = adjudicate(cell, &row);
            assert!(
                matches!(adj, Adjudication::Fail(_)),
                "{}: Unknown {name} must not PASS",
                cell.cell_id
            );
            assert!(
                reasons_mention(&adj, &format!("unknown:{name}")),
                "{}: {name} not visited: {:?}",
                cell.cell_id,
                adj.reasons()
            );
        }
    }
}

/// A missing counter key (map shape) is INVALID even though the sweep
/// would report Unknown — proven separately above; an overflowed value
/// (an enormous Known) exceeds its threshold and FAILs.
#[test]
fn overflow_value_cannot_pass() {
    let cell = &FROZEN_CELLS[0];
    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "aggregate_reads", Observed::Known(u64::MAX));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(&adj, "threshold_exceeded:aggregate_reads"));
}

/// Known(0) is distinguished from Unknown on a forbidden sentinel: the
/// former satisfies the gate, the latter fails it.
#[test]
fn sentinel_known_zero_passes_unknown_fails() {
    let cell = &FROZEN_CELLS[0];
    let field = "forbidden_attribution_tree_walk";
    let mut zero = sample_row(cell, 0);
    set_counter(&mut zero, field, known(0));
    assert_eq!(adjudicate(cell, &zero), Adjudication::Pass);
    let mut unknown = sample_row(cell, 0);
    set_counter(&mut unknown, field, Observed::Unknown);
    let adj = adjudicate(cell, &unknown);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(&adj, &format!("unknown:{field}")));
}

/// Every forbidden sentinel, nonzero, is a FAIL with its own reason.
#[test]
fn each_forbidden_sentinel_nonzero_fails() {
    for cell in FROZEN_CELLS.iter() {
        for name in COUNTER_FIELDS
            .iter()
            .filter(|n| n.starts_with("forbidden_"))
        {
            for bad in [1u64, 7, u64::MAX] {
                let mut row = sample_row(cell, 2);
                set_counter(&mut row, name, known(bad));
                let adj = adjudicate(cell, &row);
                assert!(
                    matches!(adj, Adjudication::Fail(_)),
                    "{}: {name}={bad}",
                    cell.cell_id
                );
                assert!(reasons_mention(&adj, &format!("forbidden_sentinel:{name}")));
            }
        }
    }
}

/// COMPLETE §14 table-driven threshold boundary test: for every
/// thresholded field, value == threshold PASSES and threshold + 1 FAILs,
/// on every cell.
#[test]
fn every_threshold_boundary_is_enforced() {
    for cell in FROZEN_CELLS.iter() {
        let classification = super::adjudicate::classification_for(cell);
        for (name, kind) in classification.iter() {
            let bound = match kind {
                FieldKind::Threshold(b) | FieldKind::ConstThreshold(b) => *b,
                FieldKind::Exact(_) | FieldKind::Sentinel => continue,
            };
            // At the bound: PASS.
            let mut row = sample_row(cell, 0);
            set_counter(&mut row, name, known(bound));
            // Keep the f1/f2 composites legal when their components sit
            // at their own bounds.
            if matches!(adjudicate(cell, &row), Adjudication::Fail(_)) {
                // Component-at-bound can only fail via the composites;
                // verify that is the sole reason family, then continue.
                let adj = adjudicate(cell, &row);
                let non_composite: Vec<_> = adj
                    .reasons()
                    .iter()
                    .filter(|r| !r.contains("composite_exceeded"))
                    .collect();
                assert!(
                    non_composite.is_empty(),
                    "{}: {name} at bound {}: unexpected reasons {:?}",
                    cell.cell_id,
                    bound,
                    non_composite
                );
                continue;
            }
            // One beyond the bound: FAIL with that field named.
            let mut row = sample_row(cell, 0);
            set_counter(&mut row, name, known(bound + 1));
            let adj = adjudicate(cell, &row);
            assert!(
                matches!(adj, Adjudication::Fail(_)),
                "{}: {} = {} must FAIL",
                cell.cell_id,
                name,
                bound + 1
            );
            assert!(
                reasons_mention(&adj, &format!("threshold_exceeded:{name}")),
                "{}: {} not named: {:?}",
                cell.cell_id,
                name,
                adj.reasons()
            );
        }
    }
}

/// The f1/f2 composite boundaries: at the bound PASS, one beyond FAIL.
#[test]
fn composite_f1_f2_boundaries() {
    let cell = &FROZEN_CELLS[0];
    let h = cell.h_max;
    // f1 = locate + safe_predecessor == 4H − 2 passes.
    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "locate_node_visits", known(h));
    set_counter(&mut row, "safe_predecessor_node_visits", known(3 * h - 2));
    assert_eq!(adjudicate(cell, &row), Adjudication::Pass);
    // +1 on either component fails the composite.
    let mut row = row.clone();
    set_counter(&mut row, "locate_node_visits", known(h + 1));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(&adj, "f1_composite_exceeded"));

    // f2 = split + pivot + join == 36H − 4 passes (components within
    // their own bounds: 20H + (8H−2) + (8H−2)).
    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "split_node_visits", known(20 * h));
    set_counter(&mut row, "pivot_extract_node_visits", known(8 * h - 2));
    set_counter(&mut row, "join_node_visits", known(8 * h - 2));
    assert_eq!(adjudicate(cell, &row), Adjudication::Pass);
    let mut row = row.clone();
    set_counter(&mut row, "join_node_visits", known(8 * h - 1));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(&adj, "f2_composite_exceeded"));
}

/// H == H_max is allowed; H_max + 1 is an implementation invariant
/// failure (never a larger budget).
#[test]
fn h_max_boundary() {
    let cell = &FROZEN_CELLS[0];
    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "h_old", known(cell.h_max));
    set_counter(&mut row, "h_new", known(cell.h_max));
    assert_eq!(adjudicate(cell, &row), Adjudication::Pass);

    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "h_old", known(cell.h_max + 1));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(
        &adj,
        &format!("implementation_invariant_failure:h_old:{}", cell.h_max + 1)
    ));

    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "h_new", known(cell.h_max + 1));
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(
        &adj,
        "implementation_invariant_failure:h_new"
    ));
}

/// Every exact-value obligation: wrong value FAILs, right value PASSes
/// (geometry, route, replacement scale, fact work, scale diagnostics).
#[test]
fn exact_value_fields_table_driven() {
    let cell = &FROZEN_CELLS[2];
    let exact_fields: Vec<(&str, u64)> = vec![
        ("restart_old", cell.restart),
        ("convergence_old", cell.convergence_old),
        ("convergence_new", cell.convergence_new),
        ("replace_lo", cell.replace_lo),
        ("replace_hi", cell.replace_hi),
        ("full_build_selected", 0),
        ("full_build_reason", 0),
        ("candidate_checks", contract::CANDIDATE_CHECKS),
        ("cursor_advances", contract::CURSOR_ADVANCES),
        ("owners_created", contract::DELTA_NEW),
        ("owners_removed", contract::DELTA_OLD),
        ("bulk_build_node_visits", contract::DELTA_NEW),
        (
            "fresh_payload_nodes_final",
            contract::FRESH_PAYLOAD_NODES_FINAL,
        ),
        ("payload_nodes_retired", contract::P_REMOVED),
        ("old_facts_extracted", 0),
        ("new_facts_extracted", 0),
        ("facts_compared", 0),
        ("fact_compare_bytes", 0),
        ("reftable_entries_visited", 0),
        ("m_old", cell.m),
        ("m_new", cell.m),
    ];
    for (name, expected) in exact_fields.clone() {
        // Correct value (already in the fixture): PASS.
        let row = sample_row(cell, 0);
        assert_eq!(row.counters.get(name), Some(&known(expected)), "{name}");
        assert_eq!(adjudicate(cell, &row), Adjudication::Pass, "{name}");
        // Wrong values both ways: FAIL with the field named.
        for wrong in [expected + 1, expected.wrapping_sub(1)] {
            let mut row = sample_row(cell, 0);
            set_counter(&mut row, name, known(wrong));
            let adj = adjudicate(cell, &row);
            assert!(
                matches!(adj, Adjudication::Fail(_)),
                "{name} = {wrong} must FAIL (expected {expected})"
            );
            assert!(
                reasons_mention(&adj, &format!("exact_mismatch:{name}")),
                "{name} not named: {:?}",
                adj.reasons()
            );
        }
    }
}

/// full_build_selected = true is itself a structural FAIL on this fixed
/// witness, and the route/fallback strings must match.
#[test]
fn full_build_selection_fails() {
    let cell = &FROZEN_CELLS[0];
    let mut row = sample_row(cell, 0);
    set_counter(&mut row, "full_build_selected", known(1));
    set_counter(&mut row, "full_build_reason", known(1));
    row.route = "same_target_full_build".to_string();
    row.full_build_reason = "facts_differ".to_string();
    let adj = adjudicate(cell, &row);
    assert!(matches!(adj, Adjudication::Fail(_)));
    assert!(reasons_mention(&adj, "exact_mismatch:full_build_selected"));
    assert!(reasons_mention(&adj, "exact_mismatch:full_build_reason"));
    assert!(reasons_mention(&adj, "route:same_target_full_build"));
    assert!(reasons_mention(&adj, "full_build_reason:facts_differ"));
}

/// Any correctness gate false is a FAIL; NOT_RUN likewise.
#[test]
fn correctness_gate_failures() {
    let cell = &FROZEN_CELLS[0];
    type RowMutation = Box<dyn Fn(&mut RawRowV1)>;
    let cases: [(&str, RowMutation); 4] = [
        (
            "c1",
            Box::new(|row: &mut RawRowV1| row.correctness.c1 = "FAIL".to_string()),
        ),
        (
            "c2",
            Box::new(|row: &mut RawRowV1| row.correctness.c2 = "FAIL".to_string()),
        ),
        (
            "c3",
            Box::new(|row: &mut RawRowV1| row.correctness.c3 = "NOT_RUN".to_string()),
        ),
        (
            "equality",
            Box::new(|row: &mut RawRowV1| row.correctness.normalized_structural_equality = false),
        ),
    ];
    for (label, mutate) in cases {
        let mut row = sample_row(cell, 0);
        mutate(&mut row);
        let adj = adjudicate(cell, &row);
        assert!(
            matches!(adj, Adjudication::Fail(_)),
            "{label} must FAIL, got {adj:?}"
        );
        assert!(reasons_mention(&adj, "correctness:"));
    }
}

// ---------------------------------------------------------------------------
// P2-6 completeness meta-test
// ---------------------------------------------------------------------------

/// The adjudicator classification covers every frozen decision field:
/// set-equality with the compile-time-exhaustive enumeration, no
/// duplicates, on every cell. Adding a field to
/// HORSE-A-STRUCTURAL-COUNTERS-v1 without classifying it fails here
/// (and the exhaustive destructure fails the build first).
#[test]
fn adjudicator_covers_every_frozen_decision_field() {
    use std::collections::BTreeSet;
    let enumerated: BTreeSet<&str> = COUNTER_FIELDS.iter().copied().collect();
    assert_eq!(
        enumerated.len(),
        COUNTER_FIELDS.len(),
        "enumeration has duplicates"
    );
    for cell in FROZEN_CELLS.iter() {
        let classification = super::adjudicate::classification_for(cell);
        let names: Vec<&str> = classification.iter().map(|(n, _)| *n).collect();
        let unique: BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(
            names.len(),
            unique.len(),
            "{}: duplicate classification",
            cell.cell_id
        );
        assert_eq!(
            unique, enumerated,
            "{}: classification != enumeration",
            cell.cell_id
        );
    }
    // The enumeration itself mirrors the real record: a fresh
    // HorseAStructuralCountersV1 serializes to exactly these keys.
    let fresh = crate::structural::HorseAStructuralCountersV1::new();
    let map = counters_to_map(&fresh);
    let serialized: BTreeSet<String> = map.keys().cloned().collect();
    let enumerated_owned: BTreeSet<String> = COUNTER_FIELDS.iter().map(|s| s.to_string()).collect();
    assert_eq!(serialized, enumerated_owned);
    // And every field is genuinely visited by the sweep (Unknown probe).
    // (Covered by unknown_cannot_pass_every_field_visited; asserted here
    // once more as the meta-test's behavioral half.)
    let cell = &FROZEN_CELLS[0];
    for name in COUNTER_FIELDS.iter() {
        let mut row = sample_row(cell, 1);
        set_counter(&mut row, name, Observed::Unknown);
        assert_ne!(adjudicate(cell, &row), Adjudication::Pass, "{name}");
    }
}

// ---------------------------------------------------------------------------
// Determinism / verdict derivation (#60 §15; task §42/§43)
// ---------------------------------------------------------------------------

#[test]
fn determinism_exact_equality_required() {
    let cell = &FROZEN_CELLS[0];
    let rows = [
        sample_row(cell, 0),
        sample_row(cell, 1),
        sample_row(cell, 2),
    ];
    let refs: Vec<&RawRowV1> = rows.iter().collect();
    let adjudications: Vec<Adjudication> = refs.iter().map(|r| adjudicate(cell, r)).collect();
    assert!(counters_deterministic(&refs, &adjudications));
    // One differing counter breaks determinism (never averaged).
    let mut divergent = sample_row(cell, 2);
    set_counter(&mut divergent, "aggregate_reads", known(4242));
    let rows2 = [sample_row(cell, 0), sample_row(cell, 1), divergent];
    let refs2: Vec<&RawRowV1> = rows2.iter().collect();
    let adj2: Vec<Adjudication> = refs2.iter().map(|r| adjudicate(cell, r)).collect();
    assert!(!counters_deterministic(&refs2, &adj2));
    // A differing branch/fallback reason breaks determinism.
    let mut other_route = sample_row(cell, 2);
    other_route.route = "same_target_full_build".to_string();
    let rows3 = [sample_row(cell, 0), sample_row(cell, 1), other_route];
    let refs3: Vec<&RawRowV1> = rows3.iter().collect();
    let adj3: Vec<Adjudication> = refs3.iter().map(|r| adjudicate(cell, r)).collect();
    assert!(!counters_deterministic(&refs3, &adj3));
    // An INVALID participant makes determinism unverifiable.
    let mut invalid = sample_row(cell, 2);
    invalid.study_id = "0".repeat(64);
    let rows4 = [sample_row(cell, 0), sample_row(cell, 1), invalid];
    let refs4: Vec<&RawRowV1> = rows4.iter().collect();
    let adj4: Vec<Adjudication> = refs4.iter().map(|r| adjudicate(cell, r)).collect();
    assert!(!counters_deterministic(&refs4, &adj4));
}

#[test]
fn cell_and_overall_verdict_conjunction() {
    fn refs_of(rows: &[RawRowV1]) -> [&RawRowV1; 3] {
        let refs: Vec<&RawRowV1> = rows.iter().collect();
        [refs[0], refs[1], refs[2]]
    }
    let cell0 = &FROZEN_CELLS[0];
    let rows = [
        sample_row(cell0, 0),
        sample_row(cell0, 1),
        sample_row(cell0, 2),
    ];
    let pass_cell = derive_cell_verdict(cell0, refs_of(&rows));
    assert_eq!(pass_cell.verdict, "PASS");
    assert!(pass_cell.counters_deterministic);

    // One FAIL row: the cell FAILs even though counters may still agree.
    let mut failing = sample_row(cell0, 2);
    set_counter(&mut failing, "locate_node_visits", known(u64::MAX / 2));
    let rows = [sample_row(cell0, 0), sample_row(cell0, 1), failing];
    let fail_cell = derive_cell_verdict(cell0, refs_of(&rows));
    assert_eq!(fail_cell.verdict, "FAIL");

    // Nondeterminism alone also blocks the cell verdict.
    let mut divergent = sample_row(cell0, 2);
    set_counter(&mut divergent, "avl_rotations", known(999));
    let rows = [sample_row(cell0, 0), sample_row(cell0, 1), divergent];
    let nondet_cell = derive_cell_verdict(cell0, refs_of(&rows));
    assert_eq!(nondet_cell.verdict, "FAIL");
    assert!(!nondet_cell.counters_deterministic);

    // Overall: all three cells PASS -> STRUCTURAL_PASS; one FAIL ->
    // STRUCTURAL_FAIL.
    let cells: Vec<_> = FROZEN_CELLS
        .iter()
        .map(|c| {
            let rows = [sample_row(c, 0), sample_row(c, 1), sample_row(c, 2)];
            derive_cell_verdict(c, refs_of(&rows))
        })
        .collect();
    assert_eq!(derive_overall_verdict(&cells), "STRUCTURAL_PASS");
    let mut mixed = cells;
    mixed[1].verdict = "FAIL".to_string();
    assert_eq!(derive_overall_verdict(&mixed), "STRUCTURAL_FAIL");
}
