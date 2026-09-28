//! OwnerSeq materialization and the I3 weighted-AVL substrate (spec
//! §12; #59 §7.2–§7.4): the O(M) `bulk_build`, the weighted
//! `locate_by_byte` navigation, and the relink-only structural mutation
//! operators (`remove_max`, `join_with_pivot`, `join`, `split` —
//! `replace_range`, `safe_predecessor` and the cursor land with their
//! own slices). One record per node; W-A3 is preserved, not optimized
//! away.
//!
//! I5 structural accounting (#59 §20 frozen ledger): every operator
//! charges its own work to the caller's [`HorseAStructuralSink`] at the
//! point the work occurs — one visit per logical processing of one
//! non-empty node by the named operator (revisits count again), one link
//! write per persistent-slot mutation (with the frozen flat rotation
//! convention: single = 3 slot writes charged at the rotation primitive,
//! superseding the primitive's raw slot operations), per-field aggregate
//! reads/writes. Split-internal `join_with_pivot` work routes to
//! `split_node_visits`; top-level-join work routes to `join_node_visits`;
//! `remove_max` always charges `pivot_extract_node_visits`. No count is
//! ever derived after the fact.

use crate::certificate::RestartCertificate;
use crate::state::{Aggregate, AvlNode, Owner, OwnerSeq};
use crate::structural::{HorseAStructuralSink, StructuralOp};
use crate::workspace::{CommitWorkspace, ExtractFrame, JoinFrame, SplitFrame, SplitSide};

/// The visit route of an internal `join_with_pivot`: split-internal joins
/// charge `split_node_visits`, top-level-join joins charge
/// `join_node_visits` — never both (#59 §20 routing).
pub(crate) type JoinRoute = StructuralOp;

impl OwnerSeq {
    /// Bulk-build a balanced OwnerSeq from source-ordered Owners
    /// (spec §12.5: O(M), one pass — never a repeated O(log M)
    /// insertion loop). Middle-split construction keeps sibling heights
    /// within one, so the AVL balance holds by construction.
    ///
    /// Returns the built sequence AND its height as transient
    /// construction metadata: the height each recursion level derives
    /// from its children while assembling is carried out of the seam
    /// instead of being re-read from the persistent root afterwards
    /// (which would be an unaccounted aggregate read). Workspace sizing
    /// consumes this already-derived scalar.
    pub(crate) fn bulk_build(
        owners: Vec<Owner>,
        sink: &mut dyn HorseAStructuralSink,
    ) -> (OwnerSeq, u32) {
        // Each slot is taken exactly once; total element movement is O(M).
        let mut slots: Vec<Option<Owner>> = owners.into_iter().map(Some).collect();
        let slot_count = slots.len();
        let (root, height) = build_range(&mut slots, 0, slot_count, sink);
        (OwnerSeq { root }, height)
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Total covered bytes: the root aggregate (O(1)).
    pub fn total_bytes(&self) -> usize {
        self.root.as_ref().map_or(0, |n| n.agg.subtree_bytes)
    }

    /// Number of retained Owner records: the root aggregate (O(1)).
    pub fn records(&self) -> usize {
        self.root.as_ref().map_or(0, |n| n.agg.subtree_records)
    }

    /// Height of the retained sequence (`h(empty) = 0`; O(1) root read).
    pub fn height(&self) -> u32 {
        self.root.as_ref().map_or(0, |n| n.height)
    }

    /// Whether the sequence contains at least one persistent eligible
    /// outgoing RestartCertificate (the root `subtree_has_safe`).
    pub fn has_safe(&self) -> bool {
        self.root.as_ref().is_some_and(|n| n.agg.subtree_has_safe)
    }

    /// Read-only in-order traversal with the running byte-weight base:
    /// `f(base, owner)` receives each Owner's document-absolute coverage
    /// base (`base(Owner_i) = sum of coverage_len_j for j < i`). This is
    /// the whole-export traversal shape (data-model §6) — a sequential
    /// cursor over the tree, not a per-Owner root seek.
    pub fn for_each_in_order<'s, F>(&'s self, mut f: F)
    where
        F: FnMut(usize, &'s Owner),
    {
        fn walk<'s, F: FnMut(usize, &'s Owner)>(node: &'s AvlNode, base: &mut usize, f: &mut F) {
            if let Some(left) = &node.left {
                walk(left, base, f);
            }
            f(*base, &node.owner);
            *base += node.owner.coverage_len;
            if let Some(right) = &node.right {
                walk(right, base, f);
            }
        }
        if let Some(root) = &self.root {
            let mut base = 0usize;
            walk(root, &mut base, &mut f);
        }
    }

    /// Collect the Owners in source order (read-only inspection helper
    /// for tests and validators; the I3 slice owns the real navigation
    /// substrate).
    pub fn owners_in_order(&self) -> Vec<&Owner> {
        let mut out = Vec::with_capacity(self.records());
        self.for_each_in_order(|_, owner| out.push(owner));
        out
    }

    /// Weighted byte locate (spec §12; I3 task contract §11): the unique
    /// Owner whose coverage `[base, base + coverage_len)` contains `x`,
    /// with rank, absolute base, and the offset inside the Owner — enough
    /// transient information for later composition. No Owner enumeration
    /// and no Vec materialization: one root-to-leaf weighted descent
    /// (`O(H)`, I3 task contract §12).
    ///
    /// `x == L` is the explicit logical EOF position (an empty sequence
    /// has `L = 0` and is EOF at `x = 0`); `x > L` is a precondition
    /// violation, not a fallback (I3 task contract §30). Deliberately no
    /// edit-damage policy: no deletion-endpoint view, no left guard, no
    /// restart choice (I3 task contract §11).
    pub(crate) fn locate_by_byte(
        &self,
        x: usize,
        sink: &mut dyn HorseAStructuralSink,
    ) -> Located<'_> {
        if self.root.is_some() {
            // The O(1) root aggregate read this operator performs first.
            sink.aggregate_reads(1);
        }
        let total = self.total_bytes();
        assert!(x <= total, "locate_by_byte({x}) out of range 0..={total}");
        if x == total {
            return Located::Eof;
        }
        let mut node = self
            .root
            .as_deref()
            .expect("non-total locate requires a node");
        let mut base = 0usize;
        let mut rank = 0usize;
        loop {
            // One logical processing of this non-empty node by locate.
            sink.node_visit(StructuralOp::Locate);
            // The mechanism-required descent reads only: bytes + records of
            // the left child (locate reads ≤ 2H + 1, spec §16 as corrected
            // by ACCOUNTING-CORRECTION-1 §6.1).
            let (lb, lr) = child_bytes_records(&node.left, sink);
            let owner_end = base + lb + node.owner.coverage_len;
            if x < base + lb {
                node = node
                    .left
                    .as_deref()
                    .expect("weighted descent stays on a node");
            } else if x < owner_end {
                return Located::Owner(LocatedOwner {
                    owner: &node.owner,
                    rank: rank + lr,
                    base: base + lb,
                    offset: x - base - lb,
                });
            } else {
                base = owner_end;
                rank += lr + 1;
                node = node
                    .right
                    .as_deref()
                    .expect("weighted descent stays on a node");
            }
        }
    }
}

/// The unique Owner whose coverage contains the located byte (I3 task
/// contract §11). Transient navigation view; nothing here is persistent
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocatedOwner<'s> {
    pub owner: &'s Owner,
    /// Source-order rank of this Owner (0-based).
    pub rank: usize,
    /// Absolute byte base of this Owner (`sum of coverage_len_j, j < rank`).
    pub base: usize,
    /// `x - base`, inside `[0, coverage_len)`.
    pub offset: usize,
}

/// Result of a weighted byte locate: the containing Owner, or the
/// explicit logical EOF position at `x == L` (I3 task contract §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Located<'s> {
    Owner(LocatedOwner<'s>),
    Eof,
}

fn build_range(
    slots: &mut [Option<Owner>],
    lo: usize,
    hi: usize,
    sink: &mut dyn HorseAStructuralSink,
) -> (Option<Box<AvlNode>>, u32) {
    if lo >= hi {
        return (None, 0);
    }
    let mid = lo + (hi - lo) / 2;
    let (left, left_height) = build_range(slots, lo, mid, sink);
    let (right, right_height) = build_range(slots, mid + 1, hi, sink);
    let owner = slots[mid]
        .take()
        .expect("each Owner slot is taken exactly once");
    // One created node = one bulk-build visit; its metadata recompute is
    // charged by the shared seam. The children move into the fresh node's
    // fields at construction — no persistent slot is reassigned, so the
    // frozen link-write ledger charges 0 here.
    sink.node_visit(StructuralOp::BulkBuild);
    // The subtree height handed out of the construction recursion — the
    // same value the recompute seam installs (`1 + max(child heights)`),
    // derived here from the recursion's own returns so it leaves the seam
    // as transient construction metadata without a persistent re-read.
    let height = 1 + left_height.max(right_height);
    (Some(Box::new(assemble(left, right, owner, sink))), height)
}

/// The one authoritative local metadata recomputation (spec §12, §15.4;
/// I3 task contract §8): `height` and all three aggregates are derived
/// from the children's metadata and the Owner-local fields, and every
/// metadata write on the structural path routes through this seam —
/// `bulk_build`, rotations/rebalance, `join_with_pivot`, `remove_max`,
/// `split`, and `replace_range` all share it. No operator duplicates the
/// aggregate arithmetic. Charged exactly as #59 §7.1 defines the unit:
/// 4 aggregate field reads per non-empty child (empty children contribute
/// no reads) + 4 aggregate field writes.
pub(crate) fn recompute(node: &mut AvlNode, sink: &mut dyn HorseAStructuralSink) {
    let (lh, lb, lr, ls) = child_meta(&node.left, sink);
    let (rh, rb, rr, rs) = child_meta(&node.right, sink);
    node.height = 1 + lh.max(rh);
    node.agg = Aggregate {
        subtree_bytes: lb + node.owner.coverage_len + rb,
        subtree_records: lr + 1 + rr,
        subtree_has_safe: ls || node.owner.outgoing_restart.is_some() || rs,
    };
    sink.aggregate_writes(4);
}

/// `(height, bytes, records, has_safe)` of one child slot — four
/// aggregate field reads per non-empty child. Reserved for the shared
/// `recompute` seam, which genuinely derives all four persistent fields
/// (spec §15.4). An empty child has no node and contributes no
/// aggregate-field reads (`h(empty) = 0` comes from the null check).
#[inline]
fn child_meta(
    child: &Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
) -> (u32, usize, usize, bool) {
    match child {
        Some(n) => {
            sink.aggregate_reads(4);
            (
                n.height,
                n.agg.subtree_bytes,
                n.agg.subtree_records,
                n.agg.subtree_has_safe,
            )
        }
        None => (0, 0, 0, false),
    }
}

/// `(subtree_bytes, subtree_records)` of one child slot — the two fields a
/// weighted byte/rank descent's decision needs. Two aggregate field reads
/// per non-empty child (the mechanism-required reads of the corrected
/// locate/safe-predecessor/fact-range read authority,
/// ACCOUNTING-CORRECTION-1 §6.1: `subtree_bytes` and `subtree_records` of
/// each descent node's left child; `height`/`subtree_has_safe` are read
/// only where they steer — `height_of`/`has_safe_of`).
#[inline]
fn child_bytes_records(
    child: &Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
) -> (usize, usize) {
    match child {
        Some(n) => {
            sink.aggregate_reads(2);
            (n.agg.subtree_bytes, n.agg.subtree_records)
        }
        None => (0, 0),
    }
}

/// Assemble one node from already-built children and recompute its
/// metadata through the shared seam (spec §12: `h(empty)=0`,
/// `h(leaf)=1`; aggregates combine).
fn assemble(
    left: Option<Box<AvlNode>>,
    right: Option<Box<AvlNode>>,
    owner: Owner,
    sink: &mut dyn HorseAStructuralSink,
) -> AvlNode {
    let mut node = AvlNode {
        left,
        right,
        height: 0,
        agg: Aggregate {
            subtree_bytes: 0,
            subtree_records: 0,
            subtree_has_safe: false,
        },
        owner,
    };
    recompute(&mut node, sink);
    node
}

// ------------------------------------------------------- structural substrate
//
// Relink-only structural mutation primitives (spec §12.1–§12.2, #59
// §7.2–§7.3; I3 task contract §9/§10/§16–§19). Every operator moves
// existing `Box<AvlNode>`s between slots — a moved Box never relocates
// its allocation (Rust reference: memory-allocation-and-lifetime), so
// retained-node identity survives every operation. No subtree clone, no
// node reconstruction, no fresh pivot allocation. All metadata writes
// route through the shared `recompute` seam.

#[inline]
fn height_of(child: &Option<Box<AvlNode>>, sink: &mut dyn HorseAStructuralSink) -> u32 {
    match child {
        Some(n) => {
            sink.aggregate_reads(1);
            n.height
        }
        None => 0,
    }
}

#[inline]
fn records_of(child: &Option<Box<AvlNode>>, sink: &mut dyn HorseAStructuralSink) -> usize {
    match child {
        Some(n) => {
            sink.aggregate_reads(1);
            n.agg.subtree_records
        }
        None => 0,
    }
}

#[inline]
fn has_safe_of(child: &Option<Box<AvlNode>>, sink: &mut dyn HorseAStructuralSink) -> bool {
    match child {
        Some(n) => {
            sink.aggregate_reads(1);
            n.agg.subtree_has_safe
        }
        None => false,
    }
}

/// Balance factor `h(left) − h(right)`; within `{−1, 0, +1}` whenever
/// the AVL invariant holds.
fn balance_factor(node: &AvlNode, sink: &mut dyn HorseAStructuralSink) -> i32 {
    height_of(&node.left, sink) as i32 - height_of(&node.right, sink) as i32
}

/// Rotate the subtree rooted at `slot` to the left: its right child
/// becomes the subtree root. Single rotation = 1 rotation unit / 3
/// structural link writes under the frozen §15.2–§15.3 conventions,
/// charged HERE at the primitive (the flat convention supersedes the
/// primitive's raw slot operations). The rotation participant is one
/// extra logical processing of the new subtree root by the enclosing
/// operator (`op` routes it). Owner in-order sequence unchanged; no
/// Owner or payload is cloned; no new retained node is allocated.
fn rotate_left(
    slot: &mut Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    op: StructuralOp,
) {
    sink.rotations(1);
    sink.link_writes(3);
    sink.node_visit(op);
    let mut node = slot.take().expect("rotate_left requires a node");
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    recompute(&mut node, sink);
    pivot.left = Some(node);
    recompute(&mut pivot, sink);
    *slot = Some(pivot);
}

/// Rotate the subtree rooted at `slot` to the right (mirror of
/// [`rotate_left`]).
fn rotate_right(
    slot: &mut Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    op: StructuralOp,
) {
    sink.rotations(1);
    sink.link_writes(3);
    sink.node_visit(op);
    let mut node = slot.take().expect("rotate_right requires a node");
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    recompute(&mut node, sink);
    pivot.right = Some(node);
    recompute(&mut pivot, sink);
    *slot = Some(pivot);
}

/// Restore the AVL invariant at `slot` after one child subtree's height
/// changed by at most one. Double rotations compose the primitive
/// rotations — no second restructuring path exists (I3 task contract
/// §9). Metadata must be exact on entry and stays exact. Rotation events
/// and their participant visits route to `op` (the enclosing operator).
pub(crate) fn rebalance(
    slot: &mut Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    op: StructuralOp,
) {
    let Some(node) = slot.as_deref() else {
        return;
    };
    let bf = balance_factor(node, sink);
    if bf > 1 {
        // Left-heavy. If the left child leans right, pre-rotate it left
        // (LR → LL) before the single right rotation.
        let left_bf = balance_factor(
            node.left.as_deref().expect("bf > 1 implies a left child"),
            sink,
        );
        if left_bf < 0 {
            rotate_left(&mut slot.as_mut().expect("populated").left, sink, op);
        }
        rotate_right(slot, sink, op);
    } else if bf < -1 {
        let right_bf = balance_factor(
            node.right
                .as_deref()
                .expect("bf < −1 implies a right child"),
            sink,
        );
        if right_bf > 0 {
            rotate_right(&mut slot.as_mut().expect("populated").right, sink, op);
        }
        rotate_left(slot, sink, op);
    }
}

/// Rebalance a subtree root held outside a persistent slot (helper for
/// the recursive operators). Moving the Box through the local slot is 0
/// link writes (§15.2).
fn rebalance_box(
    node: Box<AvlNode>,
    sink: &mut dyn HorseAStructuralSink,
    op: StructuralOp,
) -> Box<AvlNode> {
    let mut slot = Some(node);
    rebalance(&mut slot, sink, op);
    slot.expect("rebalance never empties a populated slot")
}

/// Remove the rightmost Owner node from a non-empty AVL tree (I3 task
/// contract §18): returns the remaining tree (rebalanced, metadata
/// exact) plus the existing maximum node as an isolated pivot. The
/// maximum Owner is removed exactly once, no retained node is
/// allocated, and the pivot keeps no child ownership. The pivot's own
/// metadata is stale until it is attached again — `join_with_pivot`
/// recomputes it (contract §17); nothing else may inspect a detached
/// pivot.
///
/// Realization (#59 §9.1, ACCOUNTING-CORRECTION-1 §9 resolution A): an
/// iterative right-spine descent over the pre-staged bounded extract
/// stack (`ws`), then a reverse unwind — no recursive call stack as the
/// structural workspace. The push/pop mechanics are resource operations
/// and charge no counter; only the frozen logical work units are
/// charged.
///
/// Frozen charging (spec §12.1.1/§16 as corrected by
/// ACCOUNTING-CORRECTION-1 §6.2.1, under the uniform §15.2 link-write
/// ledger): one descent visit per node entered; the terminal pivot
/// isolation is ONE link write (rule L2 — the detached child handed out
/// of the frame); each unwind level is ONE link write (rule L1 — the
/// `node.right` take/reinstall pair of that frame) plus one reprocessing
/// visit, one recompute and the rotation primitives' own charges.
/// Totals for descent length `d ≥ 1`: visits ≤ 4d − 3, rotations ≤
/// 2(d−1), links ≤ 7d − 6, recomputes ≤ 5(d−1), reads ≤ 44(d−1),
/// writes ≤ 20(d−1). This is the same pairing convention
/// `join_with_pivot` uses — one unit for every operator.
pub(crate) fn remove_max(
    root: Box<AvlNode>,
    sink: &mut dyn HorseAStructuralSink,
    ws: &mut CommitWorkspace,
) -> (Option<Box<AvlNode>>, Box<AvlNode>) {
    debug_assert!(
        ws.extract.is_empty(),
        "the extract stack drains completely in every operator"
    );
    // Phase 1 — right-spine descent: one visit per node entered (the
    // right-spine decision); each frame's right slot is taken exactly
    // once and waits in the bounded workspace.
    let mut node = root;
    let pivot;
    let mut rest: Option<Box<AvlNode>>;
    loop {
        // One descent visit: this level logically processes the node (the
        // right-spine decision).
        sink.node_visit(StructuralOp::PivotExtract);
        match node.right.take() {
            None => {
                // Terminal: this node IS the maximum. Isolating the pivot
                // detaches its left subtree — the frozen 1-link-write
                // logical operation (rule L2).
                let mut p = node;
                rest = p.left.take();
                sink.link_writes(1);
                pivot = p;
                break;
            }
            Some(right) => {
                ws.extract.push_bounded(ExtractFrame { node });
                node = right;
            }
        }
    }
    // Phase 2 — reverse unwind: one reprocessing visit, one take/reinstall
    // pair link write (rule L1) and one recompute per ancestor, with the
    // rotation primitives charging their own participants.
    while let Some(frame) = ws.extract.pop() {
        let mut n = frame.node;
        n.right = rest;
        sink.node_visit(StructuralOp::PivotExtract);
        sink.link_writes(1);
        recompute(&mut n, sink);
        rest = Some(rebalance_box(n, sink, StructuralOp::PivotExtract));
    }
    (rest, pivot)
}

/// Height-aware AVL join with an existing detached pivot (spec §12.1;
/// #59 §7.2): output order is `all(left) · pivot · all(right)`. The
/// pivot must arrive structurally isolated (no child ownership, contract
/// §17) and is never converted to an Owner or re-allocated. Result
/// height stays in the frozen window `[max(h(L), h(R)), max + 1]`.
/// Visits route to `route` (`Split` when called inside split, `Join`
/// when called by a top-level join).
///
/// Charging follows the frozen piecewise operator lemma (spec §12.1.1 /
/// §16, as corrected by ACCOUNTING-CORRECTION-1 §6.2.2) under the
/// uniform §15.2 link-write ledger: compatible `δ ≤ 1` charges ≤ 1
/// visit / 0 rotations / 2 link writes / 1 recompute / ≤ 10 reads /
/// 4 writes; recursive `δ ≥ 2` with spine depth `t ≤ δ − 1` charges
/// ≤ 4t+1 ≤ 4δ−3 visits, ≤ 2t ≤ 2δ−2 rotations, ≤ 7t+2 ≤ 7δ−5 link
/// writes, ≤ 5t+1 ≤ 5δ−4 recomputes, ≤ 46t+10 ≤ 46δ−36 reads and
/// ≤ 20t+4 ≤ 20δ−16 writes. The terminal pivot attach is 3 link writes
/// total (two pivot-slot installs + the `node.<inner>` same-frame pair);
/// every non-terminal unwind level is 1.
///
/// Realization (#59 §9.1, ACCOUNTING-CORRECTION-1 §9 resolution A): the
/// `δ ≥ 2` inner-spine descent/unwind runs iteratively over the
/// pre-staged bounded join stack (`ws`) — no recursive call stack as the
/// structural workspace, and no new join algorithm (the same descent,
/// pivot attach, reverse unwind, recompute and rebalance).
pub(crate) fn join_with_pivot(
    left: Option<Box<AvlNode>>,
    pivot: Box<AvlNode>,
    right: Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    route: JoinRoute,
    ws: &mut CommitWorkspace,
) -> Box<AvlNode> {
    debug_assert!(
        pivot.left.is_none() && pivot.right.is_none(),
        "join_with_pivot pivot must be structurally isolated"
    );
    let lh = height_of(&left, sink);
    let rh = height_of(&right, sink);
    if (lh as i32 - rh as i32).abs() <= 1 {
        // Compatible heights: attach both sides directly under the pivot.
        let mut x = pivot;
        x.left = left;
        x.right = right;
        // The pivot attach: one visit for the pivot, two persistent
        // slot installations.
        sink.node_visit(route);
        sink.link_writes(2);
        recompute(&mut x, sink);
        return x;
    }
    if lh > rh {
        join_right(
            left.expect("left taller than an empty right"),
            pivot,
            right,
            sink,
            route,
            ws,
        )
    } else {
        join_left(
            left,
            pivot,
            right.expect("right taller than an empty left"),
            sink,
            route,
            ws,
        )
    }
}

/// `h(node) > h(right) + 1`: descend the inner (right) spine to the
/// first node whose right child fits beside `right`, attach the pivot
/// between them, and recompute/rebalance while unwinding. The AVL
/// invariant at each spine node bounds the new right-subtree height by
/// `old + 1`, so one rebalance event per unwind level suffices.
/// Iterative over the bounded join stack; the unwind tail (visit, pair
/// link write, recompute, rebalance) runs for the terminal level too,
/// exactly as the frozen recursion's common tail did.
fn join_right(
    mut node: Box<AvlNode>,
    pivot: Box<AvlNode>,
    right: Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    route: JoinRoute,
    ws: &mut CommitWorkspace,
) -> Box<AvlNode> {
    debug_assert!(
        ws.join.is_empty(),
        "the join stack drains completely in every operator"
    );
    let held_pivot = pivot;
    let held_right = right;
    // Phase 1 — inner-spine descent.
    loop {
        // One descent visit: the height decision on this node.
        sink.node_visit(route);
        let right_height = height_of(&held_right, sink);
        if height_of(&node.right, sink) <= right_height + 1 {
            // Terminal: attach the pivot here. The attach is one pivot
            // visit + two persistent slot installations into it; the
            // `node.right` take/`Some(mid)` reinstall pair is charged by
            // the unwind tail below (rule L1) — 3 link writes total for
            // this frame, per the frozen lemma.
            let c = node.right.take();
            let mut mid = held_pivot;
            mid.left = c;
            mid.right = held_right;
            sink.node_visit(route);
            sink.link_writes(2);
            recompute(&mut mid, sink);
            node.right = Some(mid);
            break;
        }
        let child = node.right.take().expect("the inner spine continues");
        // The take here and the `parent.right = Some(joined)` reinstall in
        // the unwind below are ONE logical persistent-slot reassignment of
        // that frame's `right` slot (the §15.2 same-frame pairing rule
        // L1): the unwind charges the pair's single link write — the take
        // itself is not an independent charge (ACCOUNTING-CORRECTION-1
        // §10.6a; links ≤ 7t + 2 ≤ 7δ − 5, spec §12.1.1).
        ws.join.push_bounded(JoinFrame { node });
        node = child;
    }
    // Phase 2 — reverse unwind, terminal frame included: one
    // reprocessing visit + one pair link write + one recompute + one
    // rebalance per level; each popped parent reinstalls the joined
    // result into its taken inner slot.
    loop {
        sink.node_visit(route);
        sink.link_writes(1);
        recompute(&mut node, sink);
        let joined = rebalance_box(node, sink, route);
        match ws.join.pop() {
            Some(frame) => {
                let mut parent = frame.node;
                parent.right = Some(joined);
                node = parent;
            }
            None => return joined,
        }
    }
}

/// `h(node) > h(left) + 1`: mirror of [`join_right`] down the inner
/// (left) spine — iterative over the same bounded join stack.
fn join_left(
    left: Option<Box<AvlNode>>,
    pivot: Box<AvlNode>,
    mut node: Box<AvlNode>,
    sink: &mut dyn HorseAStructuralSink,
    route: JoinRoute,
    ws: &mut CommitWorkspace,
) -> Box<AvlNode> {
    debug_assert!(
        ws.join.is_empty(),
        "the join stack drains completely in every operator"
    );
    let held_pivot = pivot;
    let held_left = left;
    // Phase 1 — inner-spine descent.
    loop {
        // One descent visit: the height decision on this node.
        sink.node_visit(route);
        let left_height = height_of(&held_left, sink);
        if height_of(&node.left, sink) <= left_height + 1 {
            // Terminal attach (mirror): pivot visit + two installs; the
            // `node.left` pair is charged by the unwind tail (rule L1).
            let c = node.left.take();
            let mut mid = held_pivot;
            mid.left = held_left;
            mid.right = c;
            sink.node_visit(route);
            sink.link_writes(2);
            recompute(&mut mid, sink);
            node.left = Some(mid);
            break;
        }
        let child = node.left.take().expect("the inner spine continues");
        // Same-frame take/reinstall pair of that frame's `left` slot
        // (§15.2 rule L1): the unwind tail's single charge covers both —
        // the take is not an independent link write
        // (ACCOUNTING-CORRECTION-1 §10.6a).
        ws.join.push_bounded(JoinFrame { node });
        node = child;
    }
    // Phase 2 — reverse unwind, terminal frame included.
    loop {
        sink.node_visit(route);
        sink.link_writes(1);
        recompute(&mut node, sink);
        let joined = rebalance_box(node, sink, route);
        match ws.join.pop() {
            Some(frame) => {
                let mut parent = frame.node;
                parent.left = Some(joined);
                node = parent;
            }
            None => return joined,
        }
    }
}

/// Frozen deterministic join (spec §12.2; #59 §7.3): one side empty →
/// the other; otherwise `remove_max(left)` exactly once supplies the
/// pivot for [`join_with_pivot`]. No `remove_min(right)` alternative, no
/// repeated insertion, no fresh pivot, no alternating strategy — the
/// policy is part of the mechanism identity. `join` charges no visits of
/// its own: `remove_max` charges pivot extraction, `join_with_pivot`
/// charges `route` (the top-level-join `Join` route).
pub(crate) fn join(
    left: Option<Box<AvlNode>>,
    right: Option<Box<AvlNode>>,
    sink: &mut dyn HorseAStructuralSink,
    ws: &mut CommitWorkspace,
) -> Option<Box<AvlNode>> {
    match (left, right) {
        (None, right) => right,
        (left, None) => left,
        (Some(left), right) => {
            let (remaining, pivot) = remove_max(left, sink, ws);
            Some(join_with_pivot(
                remaining,
                pivot,
                right,
                sink,
                StructuralOp::Join,
                ws,
            ))
        }
    }
}

/// Split a tree at Owner record rank `k` (spec §12.3; #59 §7.4):
/// `0 <= k <= records(root)` and the result is
/// `(first k Owners, remaining Owners)`. Rank-based, never byte-based.
/// One search spine; outputs are reconstructed from existing nodes and
/// `join_with_pivot` only — no Vec flattening, no record reinsertion.
/// Both outputs are AVL-balanced with `h(output) <= h(input)` (the
/// accepted §12.3.1 proof core; the withdrawn output-height window is
/// deliberately NOT assumed anywhere). All work — including the internal
/// `join_with_pivot` joins — charges `split_node_visits` (#59 §20).
///
/// Spine aggregate reads: the entry reads the root's `subtree_records`
/// once for the precondition; every deeper frame already holds its
/// subtree's record count from the parent frame (`left_records`, or
/// `total - left_records - 1`), so no frame re-reads it
/// (ACCOUNTING-CORRECTION-1 §10.3: the per-spine-node `total` re-read was
/// an implementation convenience over-read, not mechanism work).
///
/// Realization (#59 §9.1, ACCOUNTING-CORRECTION-1 §9 resolution A): the
/// one search spine runs iteratively over the pre-staged bounded split
/// stack (`ws`); the reconstruction joins run in reverse unwind order,
/// calling the same frozen `join_with_pivot` — no recursive call stack
/// as the structural workspace, no Vec flattening, no bulk rebuild.
pub(crate) fn split(
    root: Option<Box<AvlNode>>,
    k: usize,
    sink: &mut dyn HorseAStructuralSink,
    ws: &mut CommitWorkspace,
) -> (Option<Box<AvlNode>>, Option<Box<AvlNode>>) {
    let Some(node) = root else {
        assert_eq!(k, 0, "split rank {k} out of range for an empty sequence");
        return (None, None);
    };
    // The spine entry's single aggregate read: the precondition rank bound.
    sink.aggregate_reads(1);
    let total = node.agg.subtree_records;
    split_known_total(node, k, total, sink, ws)
}

/// The split spine with each frame's subtree record count already known
/// (`total` = `subtree_records` of the entering node, established by the
/// caller without a re-read). One logical processing per spine node; the
/// child takes hand the subtrees to the continued descent / the
/// reconstruction join (rule L2, spec §15.2); the terminal cases and the
/// reconstruction `join_with_pivot` calls (their identity and order) are
/// exactly the frozen recursion's.
fn split_known_total(
    mut node: Box<AvlNode>,
    mut k: usize,
    mut total: usize,
    sink: &mut dyn HorseAStructuralSink,
    ws: &mut CommitWorkspace,
) -> (Option<Box<AvlNode>>, Option<Box<AvlNode>>) {
    debug_assert!(
        ws.split.is_empty(),
        "the split stack drains completely in every operator"
    );
    let mut a: Option<Box<AvlNode>>;
    let mut b: Option<Box<AvlNode>>;
    // Phase 1 — the one search spine, iteratively: dissect each spine
    // node, push its reconstruction frame, continue into the child that
    // contains the rank; the terminal cases produce the base outputs.
    loop {
        // One logical processing of this spine node (the rank
        // comparisons, including the terminal cases).
        sink.node_visit(StructuralOp::Split);
        assert!(k <= total, "split rank {k} out of range 0..={total}");
        if k == 0 {
            a = None;
            b = Some(node);
            break;
        }
        if k == total {
            a = Some(node);
            b = None;
            break;
        }
        let left = node.left.take();
        let right = node.right.take();
        // The two child-slot takes dissect this node for reconstruction.
        sink.link_writes(2);
        let left_records = records_of(&left, sink);
        if k < left_records {
            // k lands inside the left subtree: the pivot will join the
            // right output between the abandoned left part and the old
            // right subtree. The child's record count IS `left_records`.
            ws.split.push_bounded(SplitFrame {
                node,
                side: SplitSide::Left { right },
            });
            total = left_records;
            node = left.expect("k < left_records implies a left child");
        } else if k == left_records {
            // The pivot becomes the first Owner of the right output.
            let right_out = join_with_pivot(None, node, right, sink, StructuralOp::Split, ws);
            a = left;
            b = Some(right_out);
            break;
        } else if k == left_records + 1 {
            // The pivot becomes the final Owner of the left output.
            let left_out = join_with_pivot(left, node, None, sink, StructuralOp::Split, ws);
            a = Some(left_out);
            b = right;
            break;
        } else {
            // k lands inside the right subtree: descend with the adjusted
            // rank; the child's record count is `total - left_records -
            // 1` — this frame already holds it.
            ws.split.push_bounded(SplitFrame {
                node,
                side: SplitSide::Right { left },
            });
            k -= left_records + 1;
            total -= left_records + 1;
            node = right.expect("k > left_records + 1 implies a right child");
        }
    }
    // Phase 2 — reverse unwind: each dissected spine node rejoins its
    // kept subtree around the outputs produced below it, exactly the
    // frozen recursion's post-order reconstruction.
    while let Some(SplitFrame { node: fnode, side }) = ws.split.pop() {
        match side {
            SplitSide::Left { right } => {
                let right_out = join_with_pivot(b, fnode, right, sink, StructuralOp::Split, ws);
                b = Some(right_out);
            }
            SplitSide::Right { left } => {
                let left_out = join_with_pivot(left, fnode, a, sink, StructuralOp::Split, ws);
                a = Some(left_out);
            }
        }
    }
    (a, b)
}

impl OwnerSeq {
    /// Frozen `replace_range` (spec §12.4; #59 §7.5; I3 task contract
    /// §22): `lo`/`hi` are Owner record ranks with
    /// `0 <= lo <= hi <= records`.
    ///
    /// ```text
    /// (A,BC) = split(root, lo)
    /// (B,C)  = split(BC, hi - lo)
    /// self   = join(join(A, middle), C)
    /// return = B            // owned detached structure — never dropped
    /// ```
    ///
    /// Retained P/S and the fresh `middle` tree are transferred
    /// structurally by ownership — no record-by-record reinsertion, no
    /// clone, no rebuild (I3 task contract §24). Retirement of B is the
    /// caller's (commit's) concern; here it is returned intact.
    ///
    /// The two splits and two joins run on the caller's pre-staged
    /// bounded workspace (`ws`, #59 §9.1) — post-frontier this operator
    /// performs no allocation.
    pub(crate) fn replace_range(
        &mut self,
        lo: usize,
        hi: usize,
        middle: OwnerSeq,
        sink: &mut dyn HorseAStructuralSink,
        ws: &mut CommitWorkspace,
    ) -> OwnerSeq {
        ws.clear();
        if self.root.is_some() {
            // The O(1) root aggregate read for the range precondition.
            sink.aggregate_reads(1);
        }
        let records = self.records();
        assert!(
            lo <= hi && hi <= records,
            "replace_range [{lo}, {hi}) out of range 0..={records}"
        );
        // Emptying the persistent root slot to dissect the sequence: the
        // take here and the final `self.root = root` installation below are
        // ONE logical root-slot reassignment (the §15.2 same-frame pairing
        // rule; #59 §20 freezes the final root installation as the single
        // root-slot write — the temporary by-value move is not an
        // independent charge, ACCOUNTING-CORRECTION-1 §10.6b).
        let (a, bc) = split(self.root.take(), lo, sink, ws);
        let (b, c) = split(bc, hi - lo, sink, ws);
        let with_middle = join(a, middle.root, sink, ws);
        let root = join(with_middle, c, sink, ws);
        // The final root installation: one persistent root-slot write
        // (#59 §20).
        self.root = root;
        sink.link_writes(1);
        OwnerSeq { root: b }
    }

    /// Aggregate-pruned nearest-safe-boundary predecessor (spec §7.1,
    /// #59 §7.6; I3 task contract §13): the nearest Owner boundary whose
    /// absolute cut is **strictly below** the exclusive byte bound
    /// `before` and whose Owner carries a persistent outgoing
    /// RestartCertificate. The later I4 call site derives its
    /// "strictly before the relevant Owner-start boundary" requirement
    /// (and #59's "rightmost safe boundary ≤ r₀" via `before = r₀ + 1`);
    /// no edit-damage policy lives here (I3 task contract §14).
    ///
    /// Frozen search shape — no linear scan toward BOF, no Owner
    /// enumeration:
    ///
    /// 1. one weighted descent for `before - 1` with the ancestor stack
    ///    (≤ H visits). Every Owner strictly left of the located Owner
    ///    has its cut ≤ base(located) ≤ before − 1 < before, and the
    ///    located Owner plus everything at or right of it does not — so
    ///    the position eligibility of every candidate region is a
    ///    descent-direction fact, established before any certificate is
    ///    inspected (§14 ordering);
    /// 2. the located Owner's own left subtree is the nearest abandoned
    ///    predecessor region, then the backtracking ancestors — deepest
    ///    right-descent first, each an abandoned `left ∪ self` region.
    ///    At each region: the ancestor's own eligible certificate is the
    ///    nearest candidate of its region, else `subtree_has_safe`
    ///    prunes the region or guides one final descent into its
    ///    rightmost safe candidate (≤ H − 1 visits).
    ///
    /// Charging (#59 §20): one visit per phase-1 descent node, one visit
    /// per examined phase-2 ancestor (reprocessing counts again), one
    /// visit per guided second-descent node; `subtree_has_safe` reads are
    /// aggregate reads, the actual certificate presence inspections that
    /// drive the selection are certificate reads.
    pub(crate) fn safe_predecessor(
        &self,
        before: usize,
        sink: &mut dyn HorseAStructuralSink,
    ) -> Option<SafeBoundary<'_>> {
        if self.root.is_some() {
            // The O(1) root aggregate read for the range bound.
            sink.aggregate_reads(1);
        }
        let total = self.total_bytes();
        assert!(
            before <= total,
            "safe_predecessor bound {before} out of range 0..={total}"
        );
        // No cut is strictly below 0.
        if before == 0 {
            return None;
        }
        // Sequence-level prune: without a certified boundary anywhere, no
        // region can qualify (aggregate read, never a certificate read).
        if self.root.is_some() {
            sink.aggregate_reads(1);
        }
        if !self.has_safe() {
            return None;
        }

        /// One ancestor-stack entry from the phase-1 descent.
        struct Step<'a> {
            node: &'a AvlNode,
            /// Absolute byte base of this node's Owner.
            base: usize,
            /// Source-order rank of this node's Owner.
            rank: usize,
            /// Whether the descent continued into this node's right
            /// subtree — making `node.left ∪ node` an abandoned eligible
            /// predecessor region.
            went_right: bool,
        }

        // Phase 1: weighted descent for `before - 1` (< total, so it
        // always lands inside an Owner).
        let mut node = self.root.as_deref().expect("has_safe implies a node");
        let mut base = 0usize;
        let mut rank = 0usize;
        let mut path: Vec<Step> = Vec::new();
        let target = before - 1;
        loop {
            // One phase-1 descent visit per processed node. The descent
            // steers on bytes + records only (safe_predecessor reads ≤ 6H,
            // spec §16 as corrected by ACCOUNTING-CORRECTION-1 §6.1).
            sink.node_visit(StructuralOp::SafePredecessor);
            let (lb, lr) = child_bytes_records(&node.left, sink);
            let owner_end = base + lb + node.owner.coverage_len;
            if target < base + lb {
                path.push(Step {
                    node,
                    base,
                    rank,
                    went_right: false,
                });
                node = node.left.as_deref().expect("descent stays on a node");
            } else if target < owner_end {
                path.push(Step {
                    node,
                    base,
                    rank,
                    went_right: false,
                });
                break;
            } else {
                path.push(Step {
                    node,
                    base,
                    rank,
                    went_right: true,
                });
                base = owner_end;
                rank += lr + 1;
                node = node.right.as_deref().expect("descent stays on a node");
            }
        }

        // Phase 2, nearest region first: the located Owner's left subtree
        // (its cuts are all strictly below `before`), then the abandoned
        // `left ∪ self` regions backtracked deepest-first — each region's
        // maximum rank is strictly below the previous one's, so the first
        // region holding a certified boundary provides the answer.
        let located = path[path.len() - 1].node;
        let (located_base, located_rank) = {
            let last = &path[path.len() - 1];
            (last.base, last.rank)
        };
        if has_safe_of(&located.left, sink) {
            let left = located.left.as_deref().expect("has_safe implies a node");
            return Some(rightmost_certified(left, located_base, located_rank, sink));
        }
        for step in path[..path.len() - 1].iter().rev() {
            if !step.went_right {
                continue;
            }
            // One phase-2 examination visit (reprocessing counts again).
            sink.node_visit(StructuralOp::SafePredecessor);
            let p = step.node;
            // The ancestor's own certificate presence/content inspection
            // drives the selection — one certificate read. The base/rank
            // aggregate reads happen only for the boundary actually
            // selected (the corrected phase-2 read decomposition: one
            // `has_safe_of(left)` per examined ancestor, two reads for the
            // selected boundary; ACCOUNTING-CORRECTION-1 §6.1).
            sink.certificate_read();
            if let Some(cert) = &p.owner.outgoing_restart {
                let (lb, lr) = child_bytes_records(&p.left, sink);
                return Some(SafeBoundary {
                    owner: &p.owner,
                    cert,
                    rank: step.rank + lr,
                    base: step.base + lb,
                    boundary: step.base + lb + p.owner.coverage_len,
                });
            }
            if has_safe_of(&p.left, sink) {
                let left = p.left.as_deref().expect("has_safe implies a node");
                return Some(rightmost_certified(left, step.base, step.rank, sink));
            }
        }
        None
    }
}

/// The rightmost certified boundary of a subtree whose `subtree_has_safe`
/// is true — one guided descent (≤ H − 1 visits). Every boundary in the
/// region is position-eligible by the caller's descent-direction proof.
/// The `subtree_has_safe` checks are aggregate reads; each node whose
/// certificate presence is actually inspected for the selection charges
/// one certificate read.
fn rightmost_certified<'a>(
    node: &'a AvlNode,
    base: usize,
    rank: usize,
    sink: &mut dyn HorseAStructuralSink,
) -> SafeBoundary<'a> {
    let mut n = node;
    let mut b = base;
    let mut r = rank;
    loop {
        // One guided-descent visit per processed node. The steering reads
        // are exactly `has_safe_of(right)` + bytes/records of the left
        // child (3 per node; ACCOUNTING-CORRECTION-1 §6.1 guided-descent
        // decomposition).
        sink.node_visit(StructuralOp::SafePredecessor);
        let (lb, lr) = child_bytes_records(&n.left, sink);
        if has_safe_of(&n.right, sink) {
            b += lb + n.owner.coverage_len;
            r += lr + 1;
            n = n.right.as_deref().expect("guided descent stays on a node");
        } else {
            // This node's own certificate presence decides the selection.
            sink.certificate_read();
            if let Some(cert) = &n.owner.outgoing_restart {
                return SafeBoundary {
                    owner: &n.owner,
                    cert,
                    rank: r + lr,
                    base: b + lb,
                    boundary: b + lb + n.owner.coverage_len,
                };
            }
            n = n
                .left
                .as_deref()
                .expect("subtree_has_safe guarantees a certified node below");
        }
    }
}

/// A certified boundary found by [`OwnerSeq::safe_predecessor`]: the
/// Owner whose outgoing boundary carries the persistent
/// RestartCertificate, its rank and absolute base, and the absolute
/// boundary cut (`base + coverage_len`, strictly below the queried
/// bound). Transient navigation view; nothing here is persistent state.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // the full navigation view stays part of the frozen result contract
pub(crate) struct SafeBoundary<'s> {
    pub owner: &'s Owner,
    pub cert: &'s RestartCertificate,
    /// Source-order rank of the certified Owner (0-based).
    pub rank: usize,
    /// Absolute byte base of the certified Owner.
    pub base: usize,
    /// Absolute cut of the certified boundary (`base + coverage_len`).
    pub boundary: usize,
}
