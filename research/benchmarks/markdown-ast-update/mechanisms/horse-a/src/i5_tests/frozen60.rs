//! IC-5 — the EXACT frozen #60 geometry conformance test
//! (CORRECTNESS_AND_ACCOUNTING_CONFORMANCE_ONLY — **NOT** collection).
//!
//! Instantiates the three frozen primary cells (128 KiB / 1 MiB /
//! 16 MiB) byte-identically to the frozen workload anchor
//! (`results/h4-large-n-cause-1/cells.jsonl`, git blob 98d04bc9; the
//! frozen C-N construction `filler(126, b) + "\\n\\n"` per 128-byte
//! block, edit = insert "zzzzzzzz" at `target*128 + 63`, #60 §9.1 /
//! spec §17) and adjudicates the implementation against the
//! ALREADY-FROZEN static authority (spec §16 thresholds as corrected by
//! ACCOUNTING-CORRECTION-1 §6.4): correctness (C1), READY (C2), exact
//! geometry (C3), upper-bound containment (C4) and forbidden-sentinel
//! absence (C5).
//!
//! This module writes no treatment JSON, no benchmark CSV, no research
//! result file, registers no campaign, stores no timing, and compares
//! no latency: pre-frozen upper bounds are adjudication instruments,
//! and using them to verify implementation conformance is not
//! collection. The test prints nothing beyond failure diagnostics.
//! STRUCTURAL_COLLECTION_RUN = NO; PERFORMANCE_COLLECTION_RUN = NO.
//!
//! Every expected value below is hand-derived from the frozen
//! authorities — never read back from the implementation.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_corpusgen::filler;
use markit_mdbench_shared_grammar::parse_full;

use crate::full_build::full_build;
use crate::i5_tests::alloc_probe::{attempts, reset_attempts, DenyGuard};
use crate::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, Observed, RecordingHorseAStructuralSink,
};
use crate::update::stage;
use crate::NormalizeV1;

/// The frozen local geometry (spec §17.2): Δ_old = Δ_new = 2, Q = 2,
/// k = 2, P_removed = 4, D_payload = 2, no definitions/references/fences.
const INSERTED: &str = "zzzzzzzz";
/// sha256("zzzzzzzz") — the frozen `edit_sha256` of every cell.
const EDIT_SHA256: &str = "c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429";

/// One frozen primary cell: identity hashes, exact geometry and the
/// frozen #60 §9.5 thresholds (as corrected by ACCOUNTING-CORRECTION-1
/// §6.4 — fact-range 3H−2 with Δ_old = 2; f2 36H−4 / 18H−4 / 72H−3;
/// f3 406H−34 / 168H−24).
struct Cell {
    label: &'static str,
    /// N in bytes (a multiple of 128).
    n: usize,
    /// Frozen Hmax (spec §16): 14 / 18 / 24.
    h_max: u64,
    /// Frozen `pre_sha256` (byte identity gate).
    pre_sha256: &'static str,
    /// Frozen `post_sha256` (byte identity gate).
    post_sha256: &'static str,
    restart: u64,
    convergence_old: u64,
    convergence_new: u64,
    replace_lo: u64,
    replace_hi: u64,
    /// fact_range_node_visits ≤ H + Δ_old(H−1) = 3H − 2.
    fact_range: u64,
    /// split visits ×2 ≤ 20H.
    split_visits: u64,
    /// pivot extraction visits ×2 ≤ 8H − 2.
    pivot_visits: u64,
    /// top-level join visits ×2 ≤ 8H − 2.
    join_visits: u64,
    /// f2 visits ≤ 36H − 4.
    f2_visits: u64,
    /// rotations ≤ 18H − 4.
    rotations: u64,
    /// link writes ≤ 72H − 3.
    links: u64,
    /// aggregate reads ≤ 406H − 34.
    reads: u64,
    /// aggregate writes ≤ 168H − 24.
    writes: u64,
}

const CELLS: &[Cell] = &[
    Cell {
        label: "128 KiB",
        n: 131_072,
        h_max: 14,
        pre_sha256: "58b39c389bfe5d5cbcfdbbd74150d276c7996e1b25a84a467fa4deb3cec29a30",
        post_sha256: "1ca6bb1f138507758230ba868c6a55bb9bc824b94fe0b9c75ff3d39c8fb777ea",
        restart: 65_408,
        convergence_old: 65_664,
        convergence_new: 65_672,
        replace_lo: 511,
        replace_hi: 513,
        fact_range: 40,
        split_visits: 280,
        pivot_visits: 110,
        join_visits: 110,
        f2_visits: 500,
        rotations: 248,
        links: 1_005,
        reads: 5_650,
        writes: 2_328,
    },
    Cell {
        label: "1 MiB",
        n: 1_048_576,
        h_max: 18,
        pre_sha256: "681c2c032bd1585deef29cf2ce9f9c2a3b8c4bf163ca54c8001d45309462b294",
        post_sha256: "e3e0d2ddf4717cc9bbd0e60848ada7b542500d143134b39609cd2537da8c174c",
        restart: 524_160,
        convergence_old: 524_416,
        convergence_new: 524_424,
        replace_lo: 4_095,
        replace_hi: 4_097,
        fact_range: 52,
        split_visits: 360,
        pivot_visits: 142,
        join_visits: 142,
        f2_visits: 644,
        rotations: 320,
        links: 1_293,
        reads: 7_274,
        writes: 3_000,
    },
    Cell {
        label: "16 MiB",
        n: 16_777_216,
        h_max: 24,
        pre_sha256: "0fdca64e71df4386d4407afa1dd5e72aa0faaf60d6854a1fab730d580be6c78d",
        post_sha256: "c8014b5ecae8ca489cb083e6a1a1f8170b3fd2d8d115a85ffa4c58ab4bb29eb0",
        restart: 8_388_480,
        convergence_old: 8_388_736,
        convergence_new: 8_388_744,
        replace_lo: 65_535,
        replace_hi: 65_537,
        fact_range: 70,
        split_visits: 480,
        pivot_visits: 190,
        join_visits: 190,
        f2_visits: 860,
        rotations: 428,
        links: 1_725,
        reads: 9_710,
        writes: 4_008,
    },
];

/// The frozen C-N pre source, built with the frozen corpus generator
/// (`filler` from markit-mdbench-corpusgen — the same crate the frozen
/// Campaign-2 generator draws it from): `N/128` blocks of
/// `filler(126, b) + "\\n\\n"`. Byte identity is then PROVEN against
/// the frozen anchor hashes below, not assumed.
pub(crate) fn frozen_pre_source(n: usize) -> String {
    assert_eq!(n % 128, 0, "frozen C-N sizes are multiples of 128");
    let blocks = n / 128;
    let mut pre = String::with_capacity(n);
    for b in 0..blocks {
        pre.push_str(&filler(126, b));
        pre.push_str("\n\n");
    }
    assert_eq!(pre.len(), n, "the frozen construction is byte exact");
    pre
}

/// The frozen local-text edit: insert "zzzzzzzz" at `target*128 + 63`
/// (the middle of the middle block's 126-byte line).
pub(crate) fn frozen_edit(n: usize) -> CanonicalEdit {
    let target = n / 128 / 2;
    let edit_start = target * 128 + 63;
    CanonicalEdit::new(edit_start, edit_start, INSERTED).expect("frozen edit geometry")
}

/// Assert `field == Known(expected)` with the cell/field named in the
/// failure message.
fn known_eq(cell: &str, field: &str, o: &Observed, expected: u64) {
    assert_eq!(
        *o,
        Observed::known(expected),
        "{cell}: {field} must be Known({expected}) — a required counter may \
         not be Unknown, and an exact frozen value may not differ"
    );
}

/// Assert `field` is `Known(n)` with `n <= bound` (pre-frozen upper
/// bound; never an equality — the frozen thresholds are containment
/// instruments, not expected observed values).
fn known_le(cell: &str, field: &str, o: &Observed, bound: u64) {
    match *o {
        Observed::Known(n) => assert!(
            n <= bound,
            "{cell}: {field} = {n} exceeds the frozen bound {bound}"
        ),
        Observed::Unknown => {
            panic!("{cell}: {field} is Unknown — a #60-required counter must be Known")
        }
    }
}

/// Run the complete C1–C5 adjudication over one frozen cell.
fn run_cell(cell: &Cell) {
    let cell_id = cell.label;

    // ---- byte identity against the frozen workload anchor ----
    let old_source = Source::new(SourceId(1), frozen_pre_source(cell.n));
    assert_eq!(
        old_source.sha256_hex(),
        cell.pre_sha256,
        "{cell_id}: the generated pre source is not byte-identical to the frozen anchor"
    );
    let edit = frozen_edit(cell.n);
    assert_eq!(
        Source::new(SourceId(0), INSERTED).sha256_hex(),
        EDIT_SHA256,
        "{cell_id}: the inserted text is not the frozen 8-byte run"
    );
    let post = edit
        .apply(&old_source, SourceId(2))
        .expect("the frozen edit applies");
    assert_eq!(
        post.sha256_hex(),
        cell.post_sha256,
        "{cell_id}: the post source is not byte-identical to the frozen anchor"
    );

    // ---- the initial READY state (whole-document fresh construction —
    // excluded from the primary update structural verdict, spec §18) ----
    let old = full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("the frozen pre source full-builds");

    // ---- the frozen update through stage → prepare → commit, with the
    // commit frontier inside the allocation-denial guard (the frozen
    // #60 shape exercises split, remove_max, join_with_pivot, rotations,
    // retirement and the final installation post-frontier) ----
    let mut recording = RecordingHorseAStructuralSink::new();
    let staged = stage(
        &old,
        &old_source,
        &post,
        &edit,
        &mut NoopWorkSink,
        &mut recording,
    )
    .expect("the frozen update stages on the local path");
    let prepared = staged.prepare(old);
    reset_attempts();
    let next = {
        let _guard = DenyGuard::deny();
        prepared.commit(&mut recording)
    };
    assert_eq!(
        attempts(),
        0,
        "{cell_id}: the post-frontier region attempted {} allocation(s)",
        attempts()
    );

    // ---- C1 — correctness: full normalized structural equality
    // (never a hash) against the clean H0 parse of the post source ----
    let exported = next.normalize_v1();
    let clean = parse_full(post.as_bytes(), &mut NoopWorkSink);
    assert_eq!(
        exported, clean,
        "{cell_id}: normalize(Horse-A result) != normalize(clean H0 parse)"
    );

    // ---- C2 — READY validator ----
    crate::validate_ready(&next).expect("{cell_id}: READY invariants");

    let c: HorseAStructuralCountersV1 = recording.into_counters();
    assert_eq!(c.schema, HorseAStructuralCountersV1::SCHEMA);

    // ---- C3 — exact frozen geometry and Known values ----
    known_eq(cell_id, "restart_old", &c.restart_old, cell.restart);
    known_eq(
        cell_id,
        "convergence_old",
        &c.convergence_old,
        cell.convergence_old,
    );
    known_eq(
        cell_id,
        "convergence_new",
        &c.convergence_new,
        cell.convergence_new,
    );
    known_eq(cell_id, "replace_lo", &c.replace_lo, cell.replace_lo);
    known_eq(cell_id, "replace_hi", &c.replace_hi, cell.replace_hi);
    // The frozen local route: facts equal, no full build, convergence
    // strictly before EOF.
    known_eq(cell_id, "full_build_selected", &c.full_build_selected, 0);
    known_eq(cell_id, "full_build_reason", &c.full_build_reason, 0);
    // Candidate/cursor work: exactly the frozen Q = 2 / k = 2.
    known_eq(cell_id, "candidate_checks", &c.candidate_checks, 2);
    known_eq(cell_id, "cursor_advances", &c.cursor_advances, 2);
    // Replacement scale: Δ_old = Δ_new = 2.
    known_eq(cell_id, "owners_created", &c.owners_created, 2);
    known_eq(cell_id, "owners_removed", &c.owners_removed, 2);
    known_eq(
        cell_id,
        "bulk_build_node_visits",
        &c.bulk_build_node_visits,
        2,
    );
    known_eq(
        cell_id,
        "fresh_payload_nodes_final",
        &c.fresh_payload_nodes_final,
        4,
    );
    known_eq(
        cell_id,
        "fresh_payload_nodes_temporary",
        &c.fresh_payload_nodes_temporary,
        4,
    );
    // Facts: 0/0 and an applied-but-empty comparison.
    known_eq(cell_id, "old_facts_extracted", &c.old_facts_extracted, 0);
    known_eq(cell_id, "new_facts_extracted", &c.new_facts_extracted, 0);
    known_eq(cell_id, "facts_compared", &c.facts_compared, 0);
    known_eq(
        cell_id,
        "reftable_entries_visited",
        &c.reftable_entries_visited,
        0,
    );
    // Retirement (f5): local route, O only.
    known_eq(cell_id, "retire_node_visits", &c.retire_node_visits, 2);
    known_eq(
        cell_id,
        "payload_nodes_retired",
        &c.payload_nodes_retired,
        4,
    );
    known_eq(
        cell_id,
        "retirement_frames_entered",
        &c.retirement_frames_entered,
        6,
    );
    known_eq(cell_id, "max_retirement_depth", &c.max_retirement_depth, 2);
    // Scale diagnostics: M unchanged (2 removed, 2 created).
    known_eq(cell_id, "m_old", &c.m_old, (cell.n / 128) as u64);
    known_eq(cell_id, "m_new", &c.m_new, (cell.n / 128) as u64);
    assert!(matches!(c.h_old, Observed::Known(_)));
    assert!(matches!(c.h_new, Observed::Known(_)));

    // ---- C4 — upper-bound containment (pre-frozen #60 §9.5 rows as
    // corrected by ACCOUNTING-CORRECTION-1 §6.4) ----
    known_le(cell_id, "locate visits", &c.locate_node_visits, cell.h_max);
    known_le(
        cell_id,
        "safe_predecessor visits",
        &c.safe_predecessor_node_visits,
        3 * cell.h_max - 2,
    );
    // f1 = locate + safe predecessor ≤ 4H − 2.
    if let (Observed::Known(l), Observed::Known(s)) =
        (c.locate_node_visits, c.safe_predecessor_node_visits)
    {
        assert!(
            l + s <= 4 * cell.h_max - 2,
            "{cell_id}: f1 = {l} + {s} exceeds 4H − 2 = {}",
            4 * cell.h_max - 2
        );
    } else {
        panic!("{cell_id}: f1 components must be Known");
    }
    known_le(
        cell_id,
        "cursor visits",
        &c.cursor_node_visits,
        2 * cell.h_max + 10,
    );
    known_le(
        cell_id,
        "fact_range visits",
        &c.fact_range_node_visits,
        cell.fact_range,
    );
    known_le(
        cell_id,
        "split visits (×2)",
        &c.split_node_visits,
        cell.split_visits,
    );
    known_le(
        cell_id,
        "pivot extraction visits (×2)",
        &c.pivot_extract_node_visits,
        cell.pivot_visits,
    );
    known_le(
        cell_id,
        "top-level join visits (×2)",
        &c.join_node_visits,
        cell.join_visits,
    );
    if let (Observed::Known(sp), Observed::Known(pv), Observed::Known(jv)) = (
        c.split_node_visits,
        c.pivot_extract_node_visits,
        c.join_node_visits,
    ) {
        assert!(
            sp + pv + jv <= cell.f2_visits,
            "{cell_id}: f2 visits = {sp} + {pv} + {jv} exceeds the frozen bound {}",
            cell.f2_visits
        );
    } else {
        panic!("{cell_id}: f2 components must be Known");
    }
    known_le(cell_id, "avl_rotations", &c.avl_rotations, cell.rotations);
    known_le(
        cell_id,
        "sequence_link_writes",
        &c.sequence_link_writes,
        cell.links,
    );
    known_le(cell_id, "aggregate_reads", &c.aggregate_reads, cell.reads);
    known_le(
        cell_id,
        "aggregate_writes",
        &c.aggregate_writes,
        cell.writes,
    );
    known_le(cell_id, "certificate_reads", &c.certificate_reads, 5);
    known_le(cell_id, "certificate_writes", &c.certificate_writes, 2);
    known_le(
        cell_id,
        "old_fact_owner_visits",
        &c.old_fact_owner_visits,
        4,
    );

    // ---- C5 — every forbidden sentinel measured Known(0) ----
    for (name, field) in [
        (
            "prefix enumeration",
            &c.forbidden_prefix_sequential_enumeration,
        ),
        (
            "suffix enumeration",
            &c.forbidden_suffix_sequential_enumeration,
        ),
        (
            "unaffected payload inspections",
            &c.forbidden_unaffected_payload_inspections,
        ),
        (
            "unaffected coordinate writes",
            &c.forbidden_unaffected_coordinate_writes,
        ),
        (
            "unaffected certificate writes",
            &c.forbidden_unaffected_certificate_writes,
        ),
        (
            "global fact recollection",
            &c.forbidden_global_fact_recollection,
        ),
        (
            "unaffected old retirement",
            &c.forbidden_unaffected_old_retirement,
        ),
        ("attribution tree walk", &c.forbidden_attribution_tree_walk),
    ] {
        assert_eq!(
            *field,
            Observed::known(0),
            "{cell_id}: forbidden {name} must be Known(0)"
        );
    }
}

/// The 128 KiB frozen cell.
#[test]
fn frozen60_128k_conforms() {
    run_cell(&CELLS[0]);
}

/// The 1 MiB frozen cell.
#[test]
fn frozen60_1m_conforms() {
    run_cell(&CELLS[1]);
}

/// The 16 MiB frozen cell.
#[test]
fn frozen60_16m_conforms() {
    run_cell(&CELLS[2]);
}
