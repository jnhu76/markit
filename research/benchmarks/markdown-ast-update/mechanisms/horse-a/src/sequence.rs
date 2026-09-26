//! OwnerSeq materialization and the I3 weighted-AVL substrate (spec
//! §12; #59 §7.2–§7.4): the O(M) `bulk_build`, the weighted
//! `locate_by_byte` navigation, and the relink-only structural mutation
//! operators (`remove_max`, `join_with_pivot`, `join`, `split` —
//! `replace_range`, `safe_predecessor` and the cursor land with their
//! own slices). One record per node; W-A3 is preserved, not optimized
//! away.

use crate::certificate::RestartCertificate;
use crate::state::{Aggregate, AvlNode, Owner, OwnerSeq};

impl OwnerSeq {
    /// Bulk-build a balanced OwnerSeq from source-ordered Owners
    /// (spec §12.5: O(M), one pass — never a repeated O(log M)
    /// insertion loop). Middle-split construction keeps sibling heights
    /// within one, so the AVL balance holds by construction.
    pub(crate) fn bulk_build(owners: Vec<Owner>) -> OwnerSeq {
        // Each slot is taken exactly once; total element movement is O(M).
        let mut slots: Vec<Option<Owner>> = owners.into_iter().map(Some).collect();
        let slot_count = slots.len();
        OwnerSeq {
            root: build_range(&mut slots, 0, slot_count),
        }
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
    #[allow(dead_code)] // I4 composes the navigation surface (slice staging)
    pub(crate) fn locate_by_byte(&self, x: usize) -> Located<'_> {
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
            let (_, lb, lr, _) = child_meta(&node.left);
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
#[allow(dead_code)] // I4 composes the navigation surface (slice staging)
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
#[allow(dead_code)] // I4 composes the navigation surface (slice staging)
pub(crate) enum Located<'s> {
    Owner(LocatedOwner<'s>),
    Eof,
}

fn build_range(slots: &mut [Option<Owner>], lo: usize, hi: usize) -> Option<Box<AvlNode>> {
    if lo >= hi {
        return None;
    }
    let mid = lo + (hi - lo) / 2;
    let left = build_range(slots, lo, mid);
    let right = build_range(slots, mid + 1, hi);
    let owner = slots[mid]
        .take()
        .expect("each Owner slot is taken exactly once");
    Some(Box::new(assemble(left, right, owner)))
}

/// The one authoritative local metadata recomputation (spec §12, §15.4;
/// I3 task contract §8): `height` and all three aggregates are derived
/// from the children's metadata and the Owner-local fields, and every
/// metadata write on the structural path routes through this seam —
/// `bulk_build`, rotations/rebalance, `join_with_pivot`, `remove_max`,
/// `split`, and `replace_range` all share it. No operator duplicates the
/// aggregate arithmetic.
pub(crate) fn recompute(node: &mut AvlNode) {
    let (lh, lb, lr, ls) = child_meta(&node.left);
    let (rh, rb, rr, rs) = child_meta(&node.right);
    node.height = 1 + lh.max(rh);
    node.agg = Aggregate {
        subtree_bytes: lb + node.owner.coverage_len + rb,
        subtree_records: lr + 1 + rr,
        subtree_has_safe: ls || node.owner.outgoing_restart.is_some() || rs,
    };
}

/// `(height, bytes, records, has_safe)` of one child slot. An empty child
/// contributes no aggregate-field reads (`h(empty) = 0` comes from the
/// null check, spec §15.4).
#[inline]
fn child_meta(child: &Option<Box<AvlNode>>) -> (u32, usize, usize, bool) {
    match child {
        Some(n) => (
            n.height,
            n.agg.subtree_bytes,
            n.agg.subtree_records,
            n.agg.subtree_has_safe,
        ),
        None => (0, 0, 0, false),
    }
}

/// Assemble one node from already-built children and recompute its
/// metadata through the shared seam (spec §12: `h(empty)=0`,
/// `h(leaf)=1`; aggregates combine).
fn assemble(left: Option<Box<AvlNode>>, right: Option<Box<AvlNode>>, owner: Owner) -> AvlNode {
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
    recompute(&mut node);
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
fn height_of(child: &Option<Box<AvlNode>>) -> u32 {
    child.as_ref().map_or(0, |n| n.height)
}

#[inline]
fn records_of(child: &Option<Box<AvlNode>>) -> usize {
    child.as_ref().map_or(0, |n| n.agg.subtree_records)
}

#[inline]
fn has_safe_of(child: &Option<Box<AvlNode>>) -> bool {
    child.as_ref().is_some_and(|n| n.agg.subtree_has_safe)
}

/// Balance factor `h(left) − h(right)`; within `{−1, 0, +1}` whenever
/// the AVL invariant holds.
fn balance_factor(node: &AvlNode) -> i32 {
    height_of(&node.left) as i32 - height_of(&node.right) as i32
}

/// Rotate the subtree rooted at `slot` to the left: its right child
/// becomes the subtree root. Single rotation = 1 rotation unit / 3
/// structural link writes under the frozen §15.2–§15.3 conventions —
/// the relinks stay explicit and countable for the later I5 accounting
/// (I3 task contract §9/§32). Owner in-order sequence unchanged; no
/// Owner or payload is cloned; no new retained node is allocated.
fn rotate_left(slot: &mut Option<Box<AvlNode>>) {
    let mut node = slot.take().expect("rotate_left requires a node");
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    recompute(&mut node);
    pivot.left = Some(node);
    recompute(&mut pivot);
    *slot = Some(pivot);
}

/// Rotate the subtree rooted at `slot` to the right (mirror of
/// [`rotate_left`]).
fn rotate_right(slot: &mut Option<Box<AvlNode>>) {
    let mut node = slot.take().expect("rotate_right requires a node");
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    recompute(&mut node);
    pivot.right = Some(node);
    recompute(&mut pivot);
    *slot = Some(pivot);
}

/// Restore the AVL invariant at `slot` after one child subtree's height
/// changed by at most one. Double rotations compose the primitive
/// rotations — no second restructuring path exists (I3 task contract
/// §9). Metadata must be exact on entry and stays exact.
pub(crate) fn rebalance(slot: &mut Option<Box<AvlNode>>) {
    let Some(node) = slot.as_deref() else {
        return;
    };
    let bf = balance_factor(node);
    if bf > 1 {
        // Left-heavy. If the left child leans right, pre-rotate it left
        // (LR → LL) before the single right rotation.
        let left_bf = balance_factor(node.left.as_deref().expect("bf > 1 implies a left child"));
        if left_bf < 0 {
            rotate_left(&mut slot.as_mut().expect("populated").left);
        }
        rotate_right(slot);
    } else if bf < -1 {
        let right_bf = balance_factor(
            node.right
                .as_deref()
                .expect("bf < −1 implies a right child"),
        );
        if right_bf > 0 {
            rotate_right(&mut slot.as_mut().expect("populated").right);
        }
        rotate_left(slot);
    }
}

/// Rebalance a subtree root held outside a persistent slot (helper for
/// the recursive operators). Moving the Box through the local slot is 0
/// link writes (§15.2).
fn rebalance_box(node: Box<AvlNode>) -> Box<AvlNode> {
    let mut slot = Some(node);
    rebalance(&mut slot);
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
pub(crate) fn remove_max(root: Box<AvlNode>) -> (Option<Box<AvlNode>>, Box<AvlNode>) {
    let mut node = root;
    match node.right.take() {
        None => {
            let mut pivot = node;
            let left = pivot.left.take();
            (left, pivot)
        }
        Some(right) => {
            let (rest, pivot) = remove_max(right);
            node.right = rest;
            recompute(&mut node);
            (Some(rebalance_box(node)), pivot)
        }
    }
}

/// Height-aware AVL join with an existing detached pivot (spec §12.1;
/// #59 §7.2): output order is `all(left) · pivot · all(right)`. The
/// pivot must arrive structurally isolated (no child ownership, contract
/// §17) and is never converted to an Owner or re-allocated. Result
/// height stays in the frozen window `[max(h(L), h(R)), max + 1]`.
pub(crate) fn join_with_pivot(
    left: Option<Box<AvlNode>>,
    pivot: Box<AvlNode>,
    right: Option<Box<AvlNode>>,
) -> Box<AvlNode> {
    debug_assert!(
        pivot.left.is_none() && pivot.right.is_none(),
        "join_with_pivot pivot must be structurally isolated"
    );
    let lh = height_of(&left);
    let rh = height_of(&right);
    if (lh as i32 - rh as i32).abs() <= 1 {
        // Compatible heights: attach both sides directly under the pivot.
        let mut x = pivot;
        x.left = left;
        x.right = right;
        recompute(&mut x);
        return x;
    }
    if lh > rh {
        join_right(left.expect("left taller than an empty right"), pivot, right)
    } else {
        join_left(left, pivot, right.expect("right taller than an empty left"))
    }
}

/// `h(node) > h(right) + 1`: descend the inner (right) spine to the
/// first node whose right child fits beside `right`, attach the pivot
/// between them, and recompute/rebalance while unwinding. The AVL
/// invariant at each spine node bounds the new right-subtree height by
/// `old + 1`, so one rebalance event per unwind level suffices.
fn join_right(
    mut node: Box<AvlNode>,
    pivot: Box<AvlNode>,
    right: Option<Box<AvlNode>>,
) -> Box<AvlNode> {
    let right_height = height_of(&right);
    if height_of(&node.right) <= right_height + 1 {
        let c = node.right.take();
        let mut mid = pivot;
        mid.left = c;
        mid.right = right;
        recompute(&mut mid);
        node.right = Some(mid);
    } else {
        let c = node.right.take().expect("the inner spine continues");
        let joined = join_right(c, pivot, right);
        node.right = Some(joined);
    }
    recompute(&mut node);
    rebalance_box(node)
}

/// `h(node) > h(left) + 1`: mirror of [`join_right`] down the inner
/// (left) spine.
fn join_left(
    left: Option<Box<AvlNode>>,
    pivot: Box<AvlNode>,
    mut node: Box<AvlNode>,
) -> Box<AvlNode> {
    let left_height = height_of(&left);
    if height_of(&node.left) <= left_height + 1 {
        let c = node.left.take();
        let mut mid = pivot;
        mid.left = left;
        mid.right = c;
        recompute(&mut mid);
        node.left = Some(mid);
    } else {
        let c = node.left.take().expect("the inner spine continues");
        let joined = join_left(left, pivot, c);
        node.left = Some(joined);
    }
    recompute(&mut node);
    rebalance_box(node)
}

/// Frozen deterministic join (spec §12.2; #59 §7.3): one side empty →
/// the other; otherwise `remove_max(left)` exactly once supplies the
/// pivot for [`join_with_pivot`]. No `remove_min(right)` alternative, no
/// repeated insertion, no fresh pivot, no alternating strategy — the
/// policy is part of the mechanism identity.
pub(crate) fn join(
    left: Option<Box<AvlNode>>,
    right: Option<Box<AvlNode>>,
) -> Option<Box<AvlNode>> {
    match (left, right) {
        (None, right) => right,
        (left, None) => left,
        (Some(left), right) => {
            let (remaining, pivot) = remove_max(left);
            Some(join_with_pivot(remaining, pivot, right))
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
/// deliberately NOT assumed anywhere).
pub(crate) fn split(
    root: Option<Box<AvlNode>>,
    k: usize,
) -> (Option<Box<AvlNode>>, Option<Box<AvlNode>>) {
    let Some(mut node) = root else {
        assert_eq!(k, 0, "split rank {k} out of range for an empty sequence");
        return (None, None);
    };
    let total = node.agg.subtree_records;
    assert!(k <= total, "split rank {k} out of range 0..={total}");
    if k == 0 {
        return (None, Some(node));
    }
    if k == total {
        return (Some(node), None);
    }
    let left = node.left.take();
    let right = node.right.take();
    let left_records = records_of(&left);
    if k < left_records {
        // k lands inside the left subtree: the pivot joins the right
        // output between the abandoned left part and the old right subtree.
        let (a, b) = split(left, k);
        let right_out = join_with_pivot(b, node, right);
        (a, Some(right_out))
    } else if k == left_records {
        // The pivot becomes the first Owner of the right output.
        let right_out = join_with_pivot(None, node, right);
        (left, Some(right_out))
    } else if k == left_records + 1 {
        // The pivot becomes the final Owner of the left output.
        let left_out = join_with_pivot(left, node, None);
        (Some(left_out), right)
    } else {
        // k lands inside the right subtree: descend with the adjusted rank.
        let (c, d) = split(right, k - left_records - 1);
        let left_out = join_with_pivot(left, node, c);
        (Some(left_out), d)
    }
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
    /// clone, no rebuild (I3 task contract §24). Retirement of B is I5's
    /// concern; here it is returned intact to the caller.
    #[allow(dead_code)] // I4 composes the mutation surface (slice staging)
    pub(crate) fn replace_range(&mut self, lo: usize, hi: usize, middle: OwnerSeq) -> OwnerSeq {
        let records = self.records();
        assert!(
            lo <= hi && hi <= records,
            "replace_range [{lo}, {hi}) out of range 0..={records}"
        );
        let (a, bc) = split(self.root.take(), lo);
        let (b, c) = split(bc, hi - lo);
        let with_middle = join(a, middle.root);
        self.root = join(with_middle, c);
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
    /// The structural shape matches the frozen conservative bound
    /// `safe_predecessor_node_visits <= 3H - 2`; the formal counter is
    /// I5's, not this slice's (I3 task contract §15/§32).
    #[allow(dead_code)] // I4 composes the navigation surface (slice staging)
    pub(crate) fn safe_predecessor(&self, before: usize) -> Option<SafeBoundary<'_>> {
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
            let (_, lb, lr, _) = child_meta(&node.left);
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
        if has_safe_of(&located.left) {
            let left = located.left.as_deref().expect("has_safe implies a node");
            return Some(rightmost_certified(left, located_base, located_rank));
        }
        for step in path[..path.len() - 1].iter().rev() {
            if !step.went_right {
                continue;
            }
            let p = step.node;
            let (_, lb, lr, _) = child_meta(&p.left);
            let p_base = step.base + lb;
            let p_rank = step.rank + lr;
            if let Some(cert) = &p.owner.outgoing_restart {
                return Some(SafeBoundary {
                    owner: &p.owner,
                    cert,
                    rank: p_rank,
                    base: p_base,
                    boundary: p_base + p.owner.coverage_len,
                });
            }
            if has_safe_of(&p.left) {
                let left = p.left.as_deref().expect("has_safe implies a node");
                return Some(rightmost_certified(left, step.base, step.rank));
            }
        }
        None
    }
}

/// The rightmost certified boundary of a subtree whose `subtree_has_safe`
/// is true — one guided descent (≤ H − 1 visits). Every boundary in the
/// region is position-eligible by the caller's descent-direction proof.
fn rightmost_certified<'a>(node: &'a AvlNode, base: usize, rank: usize) -> SafeBoundary<'a> {
    let mut n = node;
    let mut b = base;
    let mut r = rank;
    loop {
        let (_, lb, lr, _) = child_meta(&n.left);
        if has_safe_of(&n.right) {
            b += lb + n.owner.coverage_len;
            r += lr + 1;
            n = n.right.as_deref().expect("guided descent stays on a node");
        } else if n.owner.outgoing_restart.is_some() {
            return SafeBoundary {
                owner: &n.owner,
                cert: n.owner.outgoing_restart.as_ref().expect("checked above"),
                rank: r + lr,
                base: b + lb,
                boundary: b + lb + n.owner.coverage_len,
            };
        } else {
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
#[allow(dead_code)] // I4 composes the navigation surface (slice staging)
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
