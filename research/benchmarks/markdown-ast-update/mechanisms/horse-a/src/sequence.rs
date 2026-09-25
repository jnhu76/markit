//! Construction-only OwnerSeq materialization (spec §12.5, §13.1; task
//! #31): the frozen same-target full build requires a balanced
//! one-record-per-node sequence, so I2 converts ordered Owners to a
//! balanced tree in O(M) by consuming the ordered input once.
//!
//! This module deliberately implements ONLY construction and read-only
//! in-order traversal. The I3 slice owns the mutable operators
//! (`locate_by_byte`, `safe_predecessor`, `split`, `join`,
//! `join_with_pivot`, `remove_max`, `replace_range`, monotone cursor,
//! rebalancing mutation API); none of them exist here.

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

/// Recompute the node-local metadata from the assembled children
/// (spec §12: `h(empty)=0`, `h(leaf)=1`; aggregates combine).
fn assemble(left: Option<Box<AvlNode>>, right: Option<Box<AvlNode>>, owner: Owner) -> AvlNode {
    let (lh, lb, lr, ls) = left.as_ref().map_or((0, 0, 0, false), |n| {
        (
            n.height,
            n.agg.subtree_bytes,
            n.agg.subtree_records,
            n.agg.subtree_has_safe,
        )
    });
    let (rh, rb, rr, rs) = right.as_ref().map_or((0, 0, 0, false), |n| {
        (
            n.height,
            n.agg.subtree_bytes,
            n.agg.subtree_records,
            n.agg.subtree_has_safe,
        )
    });
    AvlNode {
        agg: Aggregate {
            subtree_bytes: lb + owner.coverage_len + rb,
            subtree_records: lr + 1 + rr,
            subtree_has_safe: ls || owner.outgoing_restart.is_some() || rs,
        },
        height: 1 + lh.max(rh),
        left,
        right,
        owner,
    }
}
