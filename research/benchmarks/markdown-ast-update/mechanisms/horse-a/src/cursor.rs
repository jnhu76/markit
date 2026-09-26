//! The sequential monotone cursor substrate (spec §12 operator list;
//! #59 §8; I3 task contract §26–§28): one `O(H)` positioning descent,
//! then monotone source-order advance with an explicit ancestor path —
//! no fresh root seek per next Owner, no parent pointers, no persisted
//! cursor state, no materialized Owner vector. Pure traversal substrate:
//! restart, convergence, and candidate eligibility stay in I4.

use crate::state::{AvlNode, Owner, OwnerSeq};

/// One path entry: an ancestor whose right subtree is still pending,
/// with the byte base and rank at which its Owner begins.
#[derive(Debug)]
struct Frame<'s> {
    node: &'s AvlNode,
    base: usize,
    rank: usize,
}

/// A monotone source-order cursor over an [`OwnerSeq`]. Transient and
/// borrowed — any structural mutation outlives the borrow.
#[derive(Debug)]
#[allow(dead_code)] // I4 composes the navigation surface (slice staging)
pub(crate) struct OwnerCursor<'s> {
    /// Path of ancestors with pending right subtrees; the top of the
    /// stack is the next Owner to yield. Empty == EOF.
    stack: Vec<Frame<'s>>,
}

/// One yielded Owner: its reference, source-order rank, absolute byte
/// base, and outgoing boundary cut (`base + coverage_len`).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // I4 composes the navigation surface (slice staging)
pub(crate) struct CursorItem<'s> {
    pub owner: &'s Owner,
    /// Source-order rank (0-based, strictly increasing per cursor).
    pub rank: usize,
    /// Absolute byte base of the Owner.
    pub base: usize,
    /// Absolute cut of the Owner's outgoing boundary.
    pub boundary_cut: usize,
}

impl OwnerSeq {
    /// Position a cursor at the Owner with source-order rank `rank`
    /// (`0 <= rank <= records`; `rank == records` is the exact EOF
    /// start). One weighted rank descent — never a full traversal.
    #[allow(dead_code)] // I4 composes the navigation surface (slice staging)
    pub(crate) fn cursor_at_rank(&self, rank: usize) -> OwnerCursor<'_> {
        let records = self.records();
        assert!(
            rank <= records,
            "cursor rank {rank} out of range 0..={records}"
        );
        let mut stack = Vec::new();
        let mut node = self.root.as_deref();
        let mut base = 0usize;
        let mut r = 0usize;
        while let Some(n) = node {
            let (lb, lr) = child_sums(&n.left);
            if rank < r + lr {
                // The target sits inside the left subtree; this node is a
                // pending successor of everything yielded there.
                stack.push(Frame {
                    node: n,
                    base: base + lb,
                    rank: r + lr,
                });
                node = n.left.as_deref();
            } else if rank == r + lr {
                stack.push(Frame {
                    node: n,
                    base: base + lb,
                    rank: r + lr,
                });
                break;
            } else {
                // Target is in the right subtree; this node's Owner was
                // already passed by the start rank.
                base += lb + n.owner.coverage_len;
                r += lr + 1;
                node = n.right.as_deref();
            }
        }
        OwnerCursor { stack }
    }

    /// Position a cursor at the Owner whose coverage contains byte `x`
    /// (`0 <= x <= total_bytes`; `x == total` is the exact EOF start,
    /// including the empty sequence). One weighted byte descent.
    #[allow(dead_code)] // I4 composes the navigation surface (slice staging)
    pub(crate) fn cursor_at_byte(&self, x: usize) -> OwnerCursor<'_> {
        let total = self.total_bytes();
        assert!(x <= total, "cursor byte {x} out of range 0..={total}");
        let mut stack = Vec::new();
        let mut node = self.root.as_deref();
        let mut base = 0usize;
        let mut rank = 0usize;
        while let Some(n) = node {
            let (lb, lr) = child_sums(&n.left);
            let owner_end = base + lb + n.owner.coverage_len;
            if x < base + lb {
                stack.push(Frame {
                    node: n,
                    base: base + lb,
                    rank: rank + lr,
                });
                node = n.left.as_deref();
            } else if x < owner_end {
                stack.push(Frame {
                    node: n,
                    base: base + lb,
                    rank: rank + lr,
                });
                break;
            } else {
                base = owner_end;
                rank += lr + 1;
                node = n.right.as_deref();
            }
        }
        OwnerCursor { stack }
    }
}

impl<'s> OwnerCursor<'s> {
    /// Advance to and yield the next Owner in source order. Amortized
    /// O(1): each node enters the path stack at most once per walk
    /// (monotone forward, no revisits — #59 §8).
    #[allow(dead_code)] // I4 composes the navigation surface (slice staging)
    pub(crate) fn next(&mut self) -> Option<CursorItem<'s>> {
        let frame = self.stack.pop()?;
        let item = CursorItem {
            owner: &frame.node.owner,
            rank: frame.rank,
            base: frame.base,
            boundary_cut: frame.base + frame.node.owner.coverage_len,
        };
        // Push the leftmost spine of the yielded node's right subtree;
        // each pushed ancestor carries the base/rank its Owner begins at.
        let mut base = frame.base + frame.node.owner.coverage_len;
        let mut rank = frame.rank + 1;
        let mut child = frame.node.right.as_deref();
        while let Some(n) = child {
            // Each pushed ancestor's Owner begins only after its whole
            // left subtree; descending left keeps the running position
            // (the next un-yielded Owner's position) unchanged.
            let (lb, lr) = child_sums(&n.left);
            self.stack.push(Frame {
                node: n,
                base: base + lb,
                rank: rank + lr,
            });
            child = n.left.as_deref();
        }
        Some(item)
    }
}

/// `(subtree_bytes, subtree_records)` of one child slot.
#[inline]
fn child_sums(child: &Option<Box<AvlNode>>) -> (usize, usize) {
    match child {
        Some(n) => (n.agg.subtree_bytes, n.agg.subtree_records),
        None => (0, 0),
    }
}
