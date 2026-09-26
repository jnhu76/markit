//! Focused I3 weighted-AVL sequence tests (HORSE-A-IMPL-1 #62, slice I3;
//! I3 task contract §35: "similarly scoped files", independent of Markdown
//! update composition).
//!
//! The I3 operators live at `pub(crate)`/private visibility (I3 task
//! contract §54: narrowest visibility, no general-purpose AVL library
//! API), so the suites are in-crate `#[cfg(test)]` modules instead of
//! `tests/` integration binaries. They exercise the frozen operator seams
//! (`locate_by_byte`, `safe_predecessor`, `split`, `join`,
//! `join_with_pivot`, `remove_max`, `replace_range`, cursor) — the same
//! seam I4 will compose — plus the §36 invariant matrix and the §25
//! pointer-identity probes.
//!
//! Every expected value is derived from the Vec reference model built
//! here (I3 task contract §36: the sequence-level test model may use a
//! Vec; production code may not), never read back from the
//! implementation.

mod cursor;
mod locate;
mod property;
mod replace_range;
mod safe_predecessor;
mod split;
mod structural;

use crate::certificate::{RestartCertificate, RestartSupport};
use crate::state::{Aggregate, AvlNode, Owner, OwnerPayload, OwnerSeq};
use crate::validate::validate_full_build_tree;

/// The Vec reference model: per-Owner `(byte weight, certified flag)` in
/// source order.
pub type Model = Vec<(usize, bool)>;

/// A synthetic Owner: only `coverage_len` and certificate presence are
/// mechanically significant for sequence invariants. The payload is the
/// cheapest legal shape; the certificate shell is syntactically coherent
/// (full certificate semantics are an I2 validator concern — the I3
/// sequence mechanics only ever inspect `outgoing_restart.is_some()`).
pub fn owner(len: usize, certified: bool) -> Owner {
    Owner {
        coverage_len: len,
        payload: OwnerPayload::TriviaOnly,
        outgoing_restart: certified.then(|| RestartCertificate {
            support: RestartSupport {
                preceding_lf: Some(0),
                blank_line: 1..2,
            },
        }),
    }
}

/// Build a balanced sequence from per-Owner byte weights + certificate
/// flags via the O(M) `bulk_build` (which the property suites also keep
/// exercised on every call).
pub fn make(weights: &[usize], certs: &[bool]) -> OwnerSeq {
    assert_eq!(weights.len(), certs.len(), "test model must be square");
    let owners: Vec<Owner> = weights
        .iter()
        .zip(certs)
        .map(|(&w, &c)| owner(w, c))
        .collect();
    crate::state::OwnerSeq::bulk_build(owners)
}

/// Read the current in-order model back through the read-only traversal.
pub fn model_of(seq: &OwnerSeq) -> Model {
    seq.owners_in_order()
        .iter()
        .map(|o| (o.coverage_len, o.outgoing_restart.is_some()))
        .collect()
}

/// The full §36 invariant matrix after an operation: in-order Owner
/// order, height metadata, AVL balance, `subtree_bytes`,
/// `subtree_records`, `subtree_has_safe`, and the coverage byte total.
pub fn assert_seq(seq: &OwnerSeq, expected: &Model) {
    assert_eq!(
        model_of(seq),
        *expected,
        "in-order Owner order / weights / certificate flags"
    );
    assert_eq!(
        seq.records(),
        expected.len(),
        "subtree_records total vs model"
    );
    let bytes = expected.iter().map(|(w, _)| *w).sum::<usize>();
    assert_eq!(seq.total_bytes(), bytes, "subtree_bytes total vs model");
    assert_eq!(
        seq.has_safe(),
        expected.iter().any(|(_, c)| *c),
        "subtree_has_safe total vs model"
    );
    validate_full_build_tree(seq).unwrap_or_else(|e| panic!("tree invariants broken: {e}"));
}

#[allow(dead_code)] // used by later TDD slices
/// In-order allocation addresses of every retained node. Test-only
/// identity probe (I3 task contract §25): raw addresses, never stable
/// production IDs. Multiset equality across an operation proves no
/// retained node was cloned, rebuilt, or reinserted.
pub fn addresses(seq: &OwnerSeq) -> Vec<usize> {
    fn walk(node: &AvlNode, out: &mut Vec<usize>) {
        if let Some(l) = &node.left {
            walk(l, out);
        }
        out.push(node as *const AvlNode as usize);
        if let Some(r) = &node.right {
            walk(r, out);
        }
    }
    let mut out = Vec::new();
    if let Some(root) = &seq.root {
        walk(root, &mut out);
    }
    out
}

#[allow(dead_code)] // used by later TDD slices
/// Manually assemble one (possibly unbalanced) node from child trees —
/// focused internal tests for rotations/rebalance build their exact
/// adversarial shapes with this (I3 task contract §42; there is no
/// production insertion API to abuse).
pub fn n(
    left: Option<Box<AvlNode>>,
    len: usize,
    certified: bool,
    right: Option<Box<AvlNode>>,
) -> Option<Box<AvlNode>> {
    let mut node = Box::new(AvlNode {
        left,
        right,
        height: 0,
        agg: Aggregate {
            subtree_bytes: 0,
            subtree_records: 0,
            subtree_has_safe: false,
        },
        owner: owner(len, certified),
    });
    crate::sequence::recompute(&mut node);
    Some(node)
}
