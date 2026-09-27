//! I5 primitive exactness tests (#59 §20 frozen event ledger; task
//! contract §13/§34): every structural operator's charging is pinned on
//! known tiny shapes whose visit/link-write/rotation/aggregate counts are
//! hand-derived from the algorithms, never from the recorded output.
//!
//! Convention: a field the operator never had occasion to charge stays
//! `Unknown` — `Known(0)` appears only where the frozen ledger charges a
//! measured zero (e.g. `join`'s empty-side fast path legitimately performs
//! no work of its own, and the sentinel sites assert measured absence).

use crate::certificate::{RestartCertificate, RestartSupport};
use crate::sequence::{join, join_with_pivot, rebalance, recompute, remove_max, split};
use crate::workspace::CommitWorkspace;
use crate::state::{Aggregate, AvlNode, Owner, OwnerPayload, OwnerSeq};
use crate::structural::{Observed, RecordingHorseAStructuralSink, StructuralOp};

/// A synthetic Owner with the given coverage length and optional
/// certificate (blank-line interval, Owner-relative).
pub(crate) fn owner(len: usize, blank_line: Option<std::ops::Range<usize>>) -> Owner {
    Owner {
        coverage_len: len,
        payload: OwnerPayload::TriviaOnly,
        outgoing_restart: blank_line.map(|b| RestartCertificate {
            support: RestartSupport {
                preceding_lf: Some(b.start - 1),
                blank_line: b,
            },
        }),
    }
}

/// A balanced 3-record sequence: left (20), root (20), right (20).
pub(crate) fn three_node_sequence(certs: [bool; 3]) -> OwnerSeq {
    let blank = |certified: bool| certified.then_some(17..20);
    OwnerSeq::bulk_build(
        vec![
            owner(20, blank(certs[0])),
            owner(20, blank(certs[1])),
            owner(20, blank(certs[2])),
        ],
        &mut RecordingHorseAStructuralSink::new(),
    )
}

/// One fresh node with exact metadata (children already exact).
fn node(left: Option<Box<AvlNode>>, right: Option<Box<AvlNode>>, coverage: usize) -> AvlNode {
    let mut n = AvlNode {
        left,
        right,
        height: 0,
        agg: Aggregate {
            subtree_bytes: 0,
            subtree_records: 0,
            subtree_has_safe: false,
        },
        owner: owner(coverage, None),
    };
    recompute(&mut n, &mut RecordingHorseAStructuralSink::new());
    n
}

// ------------------------------------------------ recompute (§19/§34)

#[test]
fn recompute_charges_four_writes_and_exactly_four_reads_per_non_empty_child() {
    // Leaf: no children, no reads, 4 writes.
    let mut leaf = node(None, None, 10);
    let mut sink = RecordingHorseAStructuralSink::new();
    recompute(&mut leaf, &mut sink);
    let c = sink.counters();
    assert_eq!(c.aggregate_reads, Observed::Unknown, "no child, no reads");
    assert_eq!(c.aggregate_writes, Observed::known(4));

    // One child: 4 reads + 4 writes.
    let mut one_child = node(Some(Box::new(node(None, None, 10))), None, 10);
    let mut sink = RecordingHorseAStructuralSink::new();
    recompute(&mut one_child, &mut sink);
    let c = sink.counters();
    assert_eq!(c.aggregate_reads, Observed::known(4));
    assert_eq!(c.aggregate_writes, Observed::known(4));

    // Two children: 8 reads + 4 writes.
    let mut two = node(
        Some(Box::new(node(None, None, 10))),
        Some(Box::new(node(None, None, 10))),
        10,
    );
    let mut sink = RecordingHorseAStructuralSink::new();
    recompute(&mut two, &mut sink);
    let c = sink.counters();
    assert_eq!(c.aggregate_reads, Observed::known(8));
    assert_eq!(c.aggregate_writes, Observed::known(4));
}

// ------------------------------------------------ rotations (§18/§34)

#[test]
fn a_single_rotation_charges_one_rotation_and_three_link_writes() {
    // Left-heavy root whose left child leans left: one single rotation.
    let ll = Box::new(node(None, None, 10));
    let l = Box::new(node(Some(ll), None, 10));
    let root = node(Some(l), None, 10);
    let mut sink = RecordingHorseAStructuralSink::new();
    let mut slot = Some(Box::new(root));
    rebalance(&mut slot, &mut sink, StructuralOp::PivotExtract);
    let c = sink.counters();
    assert_eq!(c.avl_rotations, Observed::known(1));
    assert_eq!(c.sequence_link_writes, Observed::known(3));
    assert_eq!(
        c.pivot_extract_node_visits,
        Observed::known(1),
        "participant"
    );
    assert_eq!(c.aggregate_writes, Observed::known(8), "2 recomputes");
}

#[test]
fn a_double_rotation_charges_two_rotations_and_six_link_writes() {
    // Left-heavy root whose left child leans right: LR double rotation.
    let lr = Box::new(node(None, None, 10));
    let l = Box::new(node(None, Some(lr), 10));
    let root = node(Some(l), None, 10);
    let mut sink = RecordingHorseAStructuralSink::new();
    let mut slot = Some(Box::new(root));
    rebalance(&mut slot, &mut sink, StructuralOp::PivotExtract);
    let c = sink.counters();
    assert_eq!(c.avl_rotations, Observed::known(2));
    assert_eq!(c.sequence_link_writes, Observed::known(6));
    assert_eq!(
        c.pivot_extract_node_visits,
        Observed::known(2),
        "participants"
    );
    assert_eq!(
        c.aggregate_writes,
        Observed::known(16),
        "4 recomputes: both primitives recompute 2 nodes each"
    );
}

// ------------------------------------------------ locate (§14/§34)

#[test]
fn locate_charges_one_visit_per_descent_node_on_known_shapes() {
    let seq = three_node_sequence([false; 3]);
    // Hit at the root: exactly one processed node.
    let mut sink = RecordingHorseAStructuralSink::new();
    let _ = seq.locate_by_byte(25, &mut sink);
    let c = sink.counters();
    assert_eq!(c.locate_node_visits, Observed::known(1));
    assert_eq!(
        c.aggregate_reads,
        Observed::known(1 + 2),
        "root read + left child bytes/records (narrowed descent reads; \
         ACCOUNTING-CORRECTION-1 §6.1: locate reads ≤ 2H + 1)"
    );
    assert_eq!(c.sequence_link_writes, Observed::Unknown);
    assert_eq!(c.aggregate_writes, Observed::Unknown);

    // Hit in the leftmost leaf: root + leaf, two processed nodes.
    let mut sink = RecordingHorseAStructuralSink::new();
    let _ = seq.locate_by_byte(5, &mut sink);
    assert_eq!(sink.counters().locate_node_visits, Observed::known(2));

    // EOF position: no node is processed at all.
    let mut sink = RecordingHorseAStructuralSink::new();
    let _ = seq.locate_by_byte(60, &mut sink);
    let c = sink.counters();
    assert_eq!(c.locate_node_visits, Observed::Unknown);
    assert_eq!(c.aggregate_reads, Observed::known(1), "root read only");
}

// ------------------------------------------------ safe_predecessor (§15/§34)

#[test]
fn safe_predecessor_aggregate_pruning_charges_no_linear_enumeration() {
    // Every boundary certified: the guided descent finds the rightmost
    // certified node of the located Owner's left subtree without examining
    // any ancestor beyond it.
    let seq = three_node_sequence([true, true, true]);
    let mut sink = RecordingHorseAStructuralSink::new();
    let found = seq.safe_predecessor(60, &mut sink);
    let c = sink.counters();
    let found = found.expect("a certified predecessor exists");
    assert_eq!(found.boundary, 40, "the rightmost certified boundary < 60");
    // Phase 1 descent: root + right leaf = 2 visits. Phase 2: one
    // examined abandoned right-descent ancestor (the root) = 1 visit.
    assert_eq!(c.safe_predecessor_node_visits, Observed::known(3));
    // Exactly one certificate inspection: the finally selected node's —
    // the located leaf's left subtree is empty, so the answer comes from
    // the nearest abandoned right-descent region (the root's own
    // certificate), not from a guided descent.
    assert_eq!(c.certificate_reads, Observed::known(1));
    // Narrowed to the mechanism-required reads (ACCOUNTING-CORRECTION-1
    // §6.1, safe_predecessor reads ≤ 6H): 2 root reads (bytes bound +
    // has_safe prune) + 2 phase-1 descent reads (root's left child
    // bytes/records; the leaf's empty left child reads nothing) + 2 reads
    // for the selected boundary's base/rank. The examined-but-rejected
    // ancestors would add one has_safe_of(left) each — none run here.
    assert_eq!(c.aggregate_reads, Observed::known(6));
}

#[test]
fn safe_predecessor_examines_only_abandoned_right_descent_regions() {
    // No certificates anywhere: the sequence-level prune refuses before
    // any descent, with exactly the two root reads charged.
    let seq = three_node_sequence([false; 3]);
    let mut sink = RecordingHorseAStructuralSink::new();
    assert!(seq.safe_predecessor(60, &mut sink).is_none());
    let c = sink.counters();
    assert_eq!(c.aggregate_reads, Observed::known(2));
    assert_eq!(c.safe_predecessor_node_visits, Observed::Unknown);
    assert_eq!(c.certificate_reads, Observed::Unknown);

    // `before == 0` refuses before any region work: one root read only.
    let seq = three_node_sequence([true; 3]);
    let mut sink = RecordingHorseAStructuralSink::new();
    assert!(seq.safe_predecessor(0, &mut sink).is_none());
    let c = sink.counters();
    assert_eq!(c.aggregate_reads, Observed::known(1));
    assert_eq!(c.certificate_reads, Observed::Unknown);
}

// ------------------------------------------------ remove_max (§34)

#[test]
fn remove_max_charges_descent_detach_and_unwind_separately() {
    // Two-node right chain: descent (root + max), the detach write, one
    // unwind (reprocessing + relink + recompute of 4 writes), no rotation.
    let tree = Box::new(node(None, Some(Box::new(node(None, None, 10))), 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let (rest, pivot) = remove_max(tree, &mut sink, &mut CommitWorkspace::for_tests(16));
    let c = sink.counters();
    assert_eq!(rest.as_ref().map(|r| r.owner.coverage_len), Some(10));
    assert_eq!(pivot.owner.coverage_len, 10);
    assert!(pivot.left.is_none() && pivot.right.is_none());
    assert_eq!(
        c.pivot_extract_node_visits,
        Observed::known(3),
        "2 descent + 1 unwind"
    );
    assert_eq!(
        c.sequence_link_writes,
        Observed::known(2),
        "detach + unwind relink"
    );
    assert_eq!(c.aggregate_writes, Observed::known(4));
    assert_eq!(c.avl_rotations, Observed::Unknown, "no rotation occurred");
}

// ------------------------------------------------ split (§13/§34)

#[test]
fn split_charges_internal_joins_to_split_only() {
    let mut seq = three_node_sequence([false; 3]);
    let mut sink = RecordingHorseAStructuralSink::new();
    let (a, b) = split(seq.root.take(), 1, &mut sink, &mut CommitWorkspace::for_tests(16));
    let c = sink.counters();
    // Spine: the root (rank compare, k == left_records terminal) — 1 visit;
    // the internal join_with_pivot attach — 1 visit routed to Split.
    assert_eq!(c.split_node_visits, Observed::known(2));
    assert_eq!(c.join_node_visits, Observed::Unknown, "routing: never both");
    assert_eq!(c.pivot_extract_node_visits, Observed::Unknown);
    assert_eq!(c.avl_rotations, Observed::Unknown, "balanced 3-node split");
    // Two child-slot takes + the pivot's two attach writes.
    assert_eq!(c.sequence_link_writes, Observed::known(4));
    // Outputs: (leftmost Owner, remaining two).
    assert_eq!(a.as_ref().map(|n| n.agg.subtree_records), Some(1));
    assert_eq!(b.as_ref().map(|n| n.agg.subtree_records), Some(2));
}

// ------------------------------------------------ join (§34)

#[test]
fn join_charges_extraction_and_pivot_work_separately() {
    // Empty-side fast paths: no visits, no writes, no rotations — and no
    // fabricated Known(0) either.
    let r = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join(None, Some(r), &mut sink, &mut CommitWorkspace::for_tests(16)).expect("right side");
    let c = sink.counters();
    assert_eq!(joined.owner.coverage_len, 10);
    assert_eq!(c.pivot_extract_node_visits, Observed::Unknown);
    assert_eq!(c.join_node_visits, Observed::Unknown);
    assert_eq!(c.sequence_link_writes, Observed::Unknown);

    // Both sides single nodes: one pivot extraction (1 descent visit +
    // 1 detach write) and one compatible-height attach (1 visit + 2
    // writes + recompute of 4 reads / 4 writes).
    let l = Box::new(node(None, None, 10));
    let r = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined =
        join(Some(l), Some(r), &mut sink, &mut CommitWorkspace::for_tests(16)).expect("both sides");
    let c = sink.counters();
    assert_eq!(joined.agg.subtree_records, 2);
    assert_eq!(c.pivot_extract_node_visits, Observed::known(1));
    assert_eq!(c.join_node_visits, Observed::known(1));
    assert_eq!(c.sequence_link_writes, Observed::known(3));
    // join_with_pivot's right-child height read (1) + its attach
    // recompute over the non-empty right child (4).
    assert_eq!(c.aggregate_reads, Observed::known(5));
    assert_eq!(c.aggregate_writes, Observed::known(4));
    assert_eq!(c.avl_rotations, Observed::Unknown);
}

#[test]
fn join_with_pivot_routes_its_visits_to_the_callers_counter() {
    let l = Box::new(node(None, None, 10));
    let pivot = Box::new(node(None, None, 10));
    let r = Box::new(node(None, None, 10));
    let mut sink = RecordingHorseAStructuralSink::new();
    let joined = join_with_pivot(
        Some(l),
        pivot,
        Some(r),
        &mut sink,
        StructuralOp::Split,
        &mut CommitWorkspace::for_tests(16),
    );
    let c = sink.counters();
    assert_eq!(joined.agg.subtree_records, 3);
    assert_eq!(c.split_node_visits, Observed::known(1), "routed to split");
    assert_eq!(c.join_node_visits, Observed::Unknown);
    assert_eq!(c.sequence_link_writes, Observed::known(2));
    // Two operand height reads + the attach recompute over two non-empty
    // children (8).
    assert_eq!(c.aggregate_reads, Observed::known(10));
    assert_eq!(c.aggregate_writes, Observed::known(4));
}

// ------------------------------------------------ bulk build (§21)

#[test]
fn bulk_build_charges_one_visit_per_created_node() {
    let mut sink = RecordingHorseAStructuralSink::new();
    let seq = OwnerSeq::bulk_build(
        vec![owner(10, None), owner(10, None), owner(10, None)],
        &mut sink,
    );
    let c = sink.counters();
    assert_eq!(seq.records(), 3);
    assert_eq!(c.bulk_build_node_visits, Observed::known(3));
    // n recomputes: leaf recomputes read nothing, the root reads 8.
    assert_eq!(c.aggregate_writes, Observed::known(12));
    assert_eq!(c.aggregate_reads, Observed::known(8));
    assert_eq!(
        c.avl_rotations,
        Observed::Unknown,
        "construction is rotation-free"
    );
    // Construction-time field initialization installs no persistent slot.
    assert_eq!(c.sequence_link_writes, Observed::Unknown);
}
