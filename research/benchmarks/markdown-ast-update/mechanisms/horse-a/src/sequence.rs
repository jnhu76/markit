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
