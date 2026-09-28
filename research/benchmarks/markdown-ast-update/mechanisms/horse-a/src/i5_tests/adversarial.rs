//! Adversarial operator-boundary tests (task §39/§40 of the I5
//! implementation-conformance repair): the join lemma's piecewise edges
//! (δ = 0 / 1 / 2 / 3), remove_max depths (d = 1 / 2), split edge ranks,
//! and the link-write unit invariants — single rotation = 3, double = 6,
//! the same-frame take/reinstall pair = ONE charge, and the final root
//! installation = ONE charge (no separate temporary root-take).
//!
//! Expected values are hand-derived from the frozen algorithms and the
//! §15.2/§12.1.1 ledger — event-level expectations, never opaque totals
//! read back from the implementation.

use crate::i5_tests::primitives::{node, three_node_sequence};
use crate::sequence::{join, join_with_pivot, remove_max, split};
use crate::state::{AvlNode, OwnerSeq};
use crate::structural::{Observed, RecordingHorseAStructuralSink, StructuralOp};
use crate::workspace::CommitWorkspace;

fn ws() -> CommitWorkspace {
    CommitWorkspace::for_tests(16)
}

/// An `n`-record trivia-only sequence (no certificates).
fn seq_of(n: usize) -> OwnerSeq {
    let (seq, _) = crate::state::OwnerSeq::bulk_build(
        (0..n)
            .map(|_| crate::i5_tests::primitives::owner(20, None))
            .collect(),
        &mut RecordingHorseAStructuralSink::new(),
    );
    seq
}

// ------------------------------------------------ join δ edges (§12.1.1)

/// δ = 1 (compatible): a height-2 left tree beside a height-1 right
/// tree attaches directly under the pivot — exactly the frozen
/// compatible row: 1 visit, 0 rotations, 2 link writes, 1 recompute,
/// 10 reads, 4 writes.
#[test]
fn join_delta1_compatible_attaches_directly() {
    let l = Box::new(node(Some(Box::new(node(None, None, 10))), None, 10)); // h2
    let pivot = Box::new(node(None, None, 10));
    let r = Box::new(node(None, None, 10)); // h1
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join_with_pivot(
        Some(l),
        pivot,
        Some(r),
        &mut sink,
        StructuralOp::Join,
        &mut ws(),
    );
    assert_eq!(joined.agg.subtree_records, 4, "2 left + pivot + right");
    let c = sink.counters();
    assert_eq!(c.join_node_visits, Observed::known(1), "pivot attach only");
    assert_eq!(c.avl_rotations, Observed::Unknown);
    assert_eq!(c.sequence_link_writes, Observed::known(2), "two installs");
    assert_eq!(
        c.aggregate_reads,
        Observed::known(10),
        "2 prologue heights + 8 attach recompute"
    );
    assert_eq!(c.aggregate_writes, Observed::known(4));
}

/// δ = 2 (first recursive case, spine depth t = 1): one descent level,
/// the terminal attach (2 installs) + the frame's take/reinstall pair —
/// 3 visits, 3 link writes, no rotation (the frozen lemma bounds this
/// by 4δ−3 = 5 visits / 7δ−5 = 9 links).
#[test]
fn join_delta2_recursive_terminal_frame_charges_three_links() {
    // A balanced height-3 left tree: both children height 2.
    let leaf = || Box::new(node(None, None, 10));
    let l2a = Box::new(node(Some(leaf()), Some(leaf()), 10));
    let l2b = Box::new(node(Some(leaf()), Some(leaf()), 10));
    let root = Box::new(node(Some(l2a), Some(l2b), 10)); // h3
    let pivot = Box::new(node(None, None, 10));
    let right = Box::new(node(None, None, 10)); // h1; δ = 3 − 1 = 2
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join_with_pivot(
        Some(root),
        pivot,
        Some(right),
        &mut sink,
        StructuralOp::Join,
        &mut ws(),
    );
    assert_eq!(joined.agg.subtree_records, 9, "3+3 left + pivot + right");
    let c = sink.counters();
    assert_eq!(
        c.join_node_visits,
        Observed::known(3),
        "1 descent + 1 pivot attach + 1 unwind (t = 1)"
    );
    assert_eq!(c.avl_rotations, Observed::Unknown);
    assert_eq!(
        c.sequence_link_writes,
        Observed::known(3),
        "2 attach installs + the frame's single take/reinstall pair"
    );
}

/// δ = 3 (spine depth t = 2): the NON-TERMINAL level's take/reinstall
/// pair is ONE link write (§15.2 rule L1) — the IC-2 repair. Total links
/// = terminal 3 + (t − 1) = 4; a double-charging implementation would
/// report 5.
#[test]
fn join_delta3_non_terminal_pair_is_one_link_write() {
    // A height-4 left tree whose right spine descends h3 → h2, each
    // spine level balanced enough that no rotation fires.
    let leaf = || Box::new(node(None, None, 10));
    let h2_left = Box::new(node(Some(leaf()), None, 10)); // h2, bf +1
    let spine2 = Box::new(node(None, Some(leaf()), 10)); // h2, bf −1
    let h2b = Box::new(node(Some(leaf()), None, 10));
    let h3_left = Box::new(node(Some(h2_left.clone()), Some(leaf()), 10)); // h3
    let spine3 = Box::new(node(Some(h2b), Some(spine2), 10)); // h3, bf 0
    let root = Box::new(node(Some(h3_left), Some(spine3), 10)); // h4
    let pivot = Box::new(node(None, None, 10));
    let right = Box::new(node(None, None, 10)); // h1; δ = 4 − 1 = 3
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join_with_pivot(
        Some(root),
        pivot,
        Some(right),
        &mut sink,
        StructuralOp::Join,
        &mut ws(),
    );
    assert_eq!(joined.agg.subtree_records, 12, "4+5 left + pivot + right");
    let c = sink.counters();
    assert_eq!(
        c.join_node_visits,
        Observed::known(5),
        "2 descent + 1 pivot attach + 2 unwind (t = 2)"
    );
    assert_eq!(
        c.sequence_link_writes,
        Observed::known(4),
        "terminal 3 + ONE pair link for the single non-terminal level"
    );
    assert_eq!(c.avl_rotations, Observed::Unknown, "balanced unwind");
}

/// δ = 0 uses the compatible row — the closed forms are never applied
/// there (they would be negative; the piecewise lemma's edge rule).
/// (Pinned by `join_with_pivot_routes_its_visits_to_the_callers_counter`
/// in `primitives`; restated here as the explicit δ-edge witness.)
#[test]
fn join_delta0_compatible_row_applies() {
    let l = Box::new(node(None, None, 10));
    let pivot = Box::new(node(None, None, 10));
    let r = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join_with_pivot(
        Some(l),
        pivot,
        Some(r),
        &mut sink,
        StructuralOp::Join,
        &mut ws(),
    );
    assert_eq!(joined.agg.subtree_records, 3, "1 + pivot + 1");
    let c = sink.counters();
    assert_eq!(c.join_node_visits, Observed::known(1));
    assert_eq!(c.sequence_link_writes, Observed::known(2));
}

// ------------------------------------------------ remove_max depths

/// d = 1 (the lone detach): one descent visit, one link write, nothing
/// else (frozen d ≥ 1 row: visits ≤ 4d−3 = 1, links ≤ 7d−6 = 1).
#[test]
fn remove_max_d1_is_the_lone_detach() {
    let tree = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let (rest, pivot) = remove_max(tree, &mut sink, &mut ws());
    assert!(rest.is_none());
    assert_eq!(pivot.owner.coverage_len, 10);
    let c = sink.counters();
    assert_eq!(c.pivot_extract_node_visits, Observed::known(1));
    assert_eq!(c.sequence_link_writes, Observed::known(1));
    assert_eq!(c.aggregate_writes, Observed::Unknown);
    assert_eq!(c.avl_rotations, Observed::Unknown);
}

/// d = 2: descent + terminal + one unwind — the structurally identical
/// take/reinstall pair charged ONCE (the same convention join uses).
#[test]
fn remove_max_d2_pair_is_one_link_per_level() {
    let tree = Box::new(node(None, Some(Box::new(node(None, None, 10))), 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let (rest, pivot) = remove_max(tree, &mut sink, &mut ws());
    assert!(rest.is_some());
    assert!(pivot.left.is_none() && pivot.right.is_none());
    let c = sink.counters();
    assert_eq!(c.pivot_extract_node_visits, Observed::known(3));
    assert_eq!(
        c.sequence_link_writes,
        Observed::known(2),
        "terminal detach + ONE unwind pair link"
    );
    assert_eq!(c.aggregate_writes, Observed::known(4));
}

// ------------------------------------------------ split edge ranks

/// k = 0 and k = records: the whole tree passes to one side — one spine
/// visit, one aggregate read (the entry's precondition), no takes, no
/// links, no joins.
#[test]
fn split_edge_ranks_return_whole_trees_without_dissection() {
    for (k, left_is_empty) in [(0, true), (3, false)] {
        let mut sink = RecordingHorseAStructuralSink::new();
        let fresh = three_node_sequence([false; 3]);
        let (a, b) = split(fresh.root, k, &mut sink, &mut ws());
        let c = sink.counters();
        assert_eq!(c.split_node_visits, Observed::known(1), "k = {k}");
        assert_eq!(c.aggregate_reads, Observed::known(1), "entry read, k = {k}");
        assert_eq!(c.sequence_link_writes, Observed::Unknown, "k = {k}");
        assert_eq!(c.pivot_extract_node_visits, Observed::Unknown, "k = {k}");
        assert_eq!(c.join_node_visits, Observed::Unknown, "k = {k}");
        assert_eq!(
            a.as_ref().map(|n| n.agg.subtree_records),
            if left_is_empty { None } else { Some(3) }
        );
        assert_eq!(
            b.as_ref().map(|n| n.agg.subtree_records),
            if left_is_empty { Some(3) } else { None }
        );
    }
}

/// Near-left edge (k = 1) and near-right edge (k = records − 1) on the
/// balanced three-node fixture: one dissection level (2 take links) +
/// one compatible-δ reconstruction join (2 attach installs) — the
/// outputs keep the exact record split.
#[test]
fn split_near_edges_dissect_once_and_join_compatible() {
    for (k, a_records, b_records) in [(1usize, 1usize, 2usize), (2, 2, 1)] {
        let fresh = three_node_sequence([false; 3]);
        let mut sink = RecordingHorseAStructuralSink::new();
        let (a, b) = split(fresh.root, k, &mut sink, &mut ws());
        let c = sink.counters();
        assert_eq!(
            a.as_ref().map(|n| n.agg.subtree_records),
            Some(a_records),
            "k = {k}"
        );
        assert_eq!(
            b.as_ref().map(|n| n.agg.subtree_records),
            Some(b_records),
            "k = {k}"
        );
        assert_eq!(
            c.split_node_visits,
            Observed::known(2),
            "spine + join, k = {k}"
        );
        assert_eq!(
            c.sequence_link_writes,
            Observed::known(4),
            "2 child takes + 2 pivot installs, k = {k}"
        );
        assert_eq!(c.pivot_extract_node_visits, Observed::Unknown, "k = {k}");
    }
}

// ------------------------------------------------ root installation unit

/// Root replacement (lo = 0, hi = records): both splits hit the k=0/k=total
/// early exits, both joins hit empty-side fast paths — the ONLY
/// persistent root-slot charge is the final installation (exactly 1;
/// the frozen §20 rule the IC-3 repair restored).
#[test]
fn root_replacement_charges_exactly_one_root_installation() {
    let mut seq = three_node_sequence([false; 3]);
    let middle = seq_of(2);
    let mut sink = RecordingHorseAStructuralSink::new();
    let removed = seq.replace_range(0, 3, middle, &mut sink, &mut ws());
    let c = sink.counters();
    assert_eq!(seq.records(), 2);
    assert_eq!(removed.records(), 3);
    assert_eq!(
        c.sequence_link_writes,
        Observed::known(1),
        "the final root installation is the single root-slot write"
    );
    assert_eq!(c.split_node_visits, Observed::known(2));
}

/// Local middle replacement keeps the splice work proportional: the
/// two splits + two joins + one installation all charge, and the
/// detached middle comes back intact (the i3 semantic suite pins the
/// full shape matrix; this pins the event-level presence).
#[test]
fn local_middle_replacement_splices_with_events_only() {
    let mut seq = three_node_sequence([false; 3]);
    let middle = seq_of(1);
    let mut sink = RecordingHorseAStructuralSink::new();
    let removed = seq.replace_range(1, 2, middle, &mut sink, &mut ws());
    let c = sink.counters();
    assert_eq!(seq.records(), 3);
    assert_eq!(removed.records(), 1);
    for (name, field) in [
        ("split", &c.split_node_visits),
        ("pivot_extract", &c.pivot_extract_node_visits),
        ("link_writes", &c.sequence_link_writes),
        ("aggregate_writes", &c.aggregate_writes),
    ] {
        assert!(
            matches!(field, Observed::Known(v) if *v > 0),
            "{name} must be measured positive on the splice path"
        );
    }
}

// ------------------------------------------------ join empty-side identity

/// `join`'s empty-side fast paths perform none of their own work — and
/// fabricate no `Known(0)` either (Unknown, never measured).
#[test]
fn join_empty_sides_stay_unmeasured() {
    let r = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined: Box<AvlNode> = join(None, Some(r), &mut sink, &mut ws()).expect("right side");
    assert_eq!(joined.owner.coverage_len, 10);
    let c = sink.counters();
    assert_eq!(c.join_node_visits, Observed::Unknown);
    assert_eq!(c.pivot_extract_node_visits, Observed::Unknown);
    assert_eq!(c.sequence_link_writes, Observed::Unknown);

    // And an empty OwnerSeq result of a whole-replacement splice stays
    // a valid empty sequence.
    let mut empty: OwnerSeq = OwnerSeq::default();
    let removed = empty.replace_range(0, 0, OwnerSeq::default(), &mut sink, &mut ws());
    assert!(empty.is_empty() && removed.is_empty());
}
