//! Structural mutation substrate: `remove_max`, `join_with_pivot`,
//! `join`, and the rotation/rebalance primitives (I3 task contract
//! §9, §16–§19, §41–§43; spec §12.1–§12.2, #59 §7.2–§7.3).
//!
//! There is no production insertion API to build adversarial shapes
//! with, so the rotation tests assemble exact unbalanced trees with the
//! test-side `n()` constructor (I3 task contract §42) and drive the
//! shared `rebalance` primitive directly.

use super::*;
use crate::sequence::{join, join_with_pivot, rebalance, remove_max};

fn addr(node: &AvlNode) -> usize {
    node as *const AvlNode as usize
}

fn sorted(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v
}

// ---------------------------------------------------------------- remove_max

#[test]
fn remove_max_on_a_single_owner_yields_an_isolated_pivot() {
    let seq = make(&[4], &[true]);
    let root = seq.root.expect("non-empty");
    let root_addr = addr(&root);
    let (rest, pivot) = remove_max(root);
    assert!(
        rest.is_none(),
        "nothing remains after removing the only Owner"
    );
    assert_eq!(addr(&pivot), root_addr, "pivot is the original node");
    assert!(
        pivot.left.is_none() && pivot.right.is_none(),
        "pivot keeps no retained child ownership"
    );
    assert_eq!(pivot.owner.coverage_len, 4);
    assert!(pivot.owner.outgoing_restart.is_some());
}

#[test]
fn remove_max_on_balanced_trees_keeps_order_and_validity() {
    for n_owners in 1..=8usize {
        let weights: Vec<usize> = (1..=n_owners).map(|i| i * 3).collect();
        let certs: Vec<bool> = (0..n_owners).map(|i| i % 2 == 0).collect();
        let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
        let seq = make(&weights, &certs);
        let original_addrs = addresses(&seq);
        let (rest, pivot) = remove_max(seq.root.expect("non-empty"));

        // The pivot is exactly the original final Owner node.
        assert_eq!(addr(&pivot), original_addrs[n_owners - 1]);
        assert!(pivot.left.is_none() && pivot.right.is_none());
        assert_eq!(pivot.owner.coverage_len, weights[n_owners - 1]);
        assert_eq!(
            pivot.owner.outgoing_restart.is_some(),
            model[n_owners - 1].1
        );
        // Detached pivot metadata is stale by contract (§17) until it is
        // attached again — join_with_pivot recomputes it; nothing else may
        // inspect it.

        let rest_seq = OwnerSeq { root: rest };
        assert_seq(&rest_seq, &model[..n_owners - 1]);
        let mut conserved = addresses(&rest_seq);
        conserved.push(addr(&pivot));
        assert_eq!(sorted(conserved), sorted(original_addrs), "no allocation");
    }
}

#[test]
fn remove_max_rebalances_after_a_right_spine_removal() {
    // AVL root: left height 3 (7 unit Owners), right height 2 (a
    // right-leaning chain). Removing the rightmost Owner drops the right
    // spine to height 1 and leaves the root left-heavy by 2 — the unwind
    // must rebalance (I3 task contract §43 "right-heavy path cases").
    let left_tree = make(&[1; 7], &[false; 7]);
    let chain = n(None, 1, false, n(None, 1, false, None));
    let root = n(left_tree.root, 2, false, chain).expect("root");
    let original_addrs = {
        let seq = OwnerSeq { root: Some(root) };
        let a = addresses(&seq);
        (seq, a)
    };
    let (rest, pivot) = remove_max(original_addrs.0.root.expect("non-empty"));
    assert_eq!(
        pivot.owner.coverage_len, 1,
        "pivot is the chain's last leaf"
    );

    let rest_seq = OwnerSeq { root: rest };
    let mut expected: Model = vec![(1, false); 7];
    expected.push((2, false));
    expected.push((1, false));
    assert_seq(&rest_seq, &expected);
    let mut conserved = addresses(&rest_seq);
    conserved.push(addr(&pivot));
    assert_eq!(sorted(conserved), sorted(original_addrs.1), "no allocation");
}

// ------------------------------------------------------------- join_with_pivot

#[test]
fn join_with_pivot_attaches_directly_at_compatible_heights() {
    let l = make(&[1, 2], &[false; 2]);
    let r = make(&[3, 4, 5], &[false; 3]);
    let (lh, rh) = (
        l.root.as_ref().unwrap().height,
        r.root.as_ref().unwrap().height,
    );
    let pivot = n(None, 9, true, None).expect("pivot");
    let joined = join_with_pivot(l.root, pivot, r.root);
    // Compatible heights: the pivot IS the subtree root, attached under it.
    assert_eq!(joined.owner.coverage_len, 9);
    assert_eq!(
        joined.height,
        lh.max(rh) + 1,
        "#59 §7.2: result height ∈ [max(hL,hR), max+1]"
    );
    let seq = OwnerSeq { root: Some(joined) };
    let expected: Model = vec![
        (1, false),
        (2, false),
        (9, true),
        (3, false),
        (4, false),
        (5, false),
    ];
    assert_seq(&seq, &expected);
}

#[test]
fn join_with_pivot_preserves_every_node_across_uneven_joins() {
    // Left much taller, and the mirrored right-much-taller case. The
    // frozen #59 §7.2 height window holds, order is exact, and the
    // address multiset is conserved — no clone, no fresh retained node
    // (I3 task contract §10).
    for (lw, rw) in [(vec![1; 32], vec![2; 2]), (vec![2; 2], vec![1; 32])] {
        let certs = |k: usize| vec![false; k];
        let l = make(&lw, &certs(lw.len()));
        let r = make(&rw, &certs(rw.len()));
        let l_addrs = addresses(&l);
        let r_addrs = addresses(&r);
        let (lh, rh) = (
            l.root.as_ref().unwrap().height,
            r.root.as_ref().unwrap().height,
        );

        let pivot = n(None, 9, true, None).expect("pivot");
        let pivot_addr = addr(&pivot);
        let joined = join_with_pivot(l.root, pivot, r.root);

        assert!(
            (lh.max(rh)..=lh.max(rh) + 1).contains(&joined.height),
            "#59 §7.2 result-height window for {lw:?}+{rw:?}"
        );
        let seq = OwnerSeq { root: Some(joined) };
        let mut expected: Model = lw.iter().map(|&w| (w, false)).collect();
        expected.push((9, true));
        expected.extend(rw.iter().map(|&w| (w, false)));
        assert_seq(&seq, &expected);

        let mut conserved = l_addrs;
        conserved.push(pivot_addr);
        conserved.extend(r_addrs);
        assert_eq!(sorted(addresses(&seq)), sorted(conserved), "no allocation");
    }
}

#[test]
#[should_panic(expected = "structurally isolated")]
#[cfg(debug_assertions)]
fn join_with_pivot_requires_a_structurally_isolated_pivot() {
    // §17: the pivot must arrive with no child ownership; a detached node
    // that still owns children is a contract violation (debug-gated: the
    // debug_assert is compiled out of release builds).
    let l = make(&[1, 2], &[false; 2]);
    let r = make(&[3, 4], &[false; 2]);
    let dirty_pivot = n(None, 9, false, n(None, 8, false, None)).expect("pivot with a child");
    let _ = join_with_pivot(l.root, dirty_pivot, r.root);
}

// ----------------------------------------------------------------------- join

#[test]
fn join_empty_variants_return_the_other_side() {
    assert!(join(None, None).is_none());

    let r = make(&[1, 2, 3], &[false; 3]);
    let r_addrs = addresses(&r);
    let joined = join(None, r.root).expect("right side");
    assert_eq!(addresses(&OwnerSeq { root: Some(joined) }), r_addrs);

    let l = make(&[1, 2, 3], &[false; 3]);
    let l_addrs = addresses(&l);
    let joined = join(l.root, None).expect("left side");
    assert_eq!(addresses(&OwnerSeq { root: Some(joined) }), l_addrs);
}

#[test]
fn join_takes_its_pivot_from_the_left_maximum() {
    // §41: for nonempty/nonempty join, the pivot source is exactly the
    // maximum node of the left tree (address identity pins it: the old
    // left-max node reappears at rank len(left) − 1 of the joined
    // sequence).
    let l = make(&[1, 2, 3, 4, 5, 6, 7], &[false; 7]);
    let r = make(&[8, 9], &[false; 2]);
    let l_addrs = addresses(&l);
    let r_addrs = addresses(&r);
    let left_max_addr = l_addrs[l_addrs.len() - 1];

    let joined = join(l.root, r.root).expect("both sides non-empty");
    let seq = OwnerSeq { root: Some(joined) };
    let result_addrs = addresses(&seq);
    assert_eq!(result_addrs.len(), l_addrs.len() + r_addrs.len());
    assert_eq!(result_addrs[..l_addrs.len()], l_addrs, "left order kept");
    assert_eq!(result_addrs[l_addrs.len()..], r_addrs, "right order kept");
    assert_eq!(
        result_addrs[l_addrs.len() - 1],
        left_max_addr,
        "the joined pivot node is the old left maximum"
    );

    let expected: Model = (1..=7)
        .map(|w| (w, false))
        .chain((8..=9).map(|w| (w, false)))
        .collect();
    assert_seq(&seq, &expected);
}

#[test]
fn join_covers_similar_and_uneven_heights() {
    for (lw, rw) in [
        (vec![1; 4], vec![2; 4]),  // similar heights
        (vec![1; 16], vec![2; 2]), // left much taller
        (vec![1; 2], vec![2; 16]), // right much taller
    ] {
        let certs = |k: usize| vec![false; k];
        let l = make(&lw, &certs(lw.len()));
        let r = make(&rw, &certs(rw.len()));
        let l_addrs = addresses(&l);
        let r_addrs = addresses(&r);
        let joined = join(l.root, r.root).expect("both non-empty");
        let seq = OwnerSeq { root: Some(joined) };
        let mut expected: Model = lw.iter().map(|&w| (w, false)).collect();
        expected.extend(rw.iter().map(|&w| (w, false)));
        assert_seq(&seq, &expected);
        let mut conserved = l_addrs;
        conserved.extend(r_addrs);
        assert_eq!(sorted(addresses(&seq)), sorted(conserved), "no allocation");
    }
}

// ------------------------------------------------------------------ rotations

/// Rebalance at the root slot of a sequence and return (in-order node
/// addresses before, rebalanced sequence).
fn rebalanced(seq: OwnerSeq) -> (Vec<usize>, OwnerSeq) {
    let before = addresses(&seq);
    let mut slot = seq.root;
    rebalance(&mut slot);
    let after_seq = OwnerSeq { root: slot };
    (before, after_seq)
}

#[test]
fn single_right_rotation_restores_an_ll_shape() {
    // LL: root bf=+2 with a left-heavy left child → one single right
    // rotation; the old left child becomes the subtree root.
    let a = n(None, 1, false, None);
    let l = n(a, 2, false, None);
    let root = n(l, 3, false, None).expect("root bf=+2");
    let (before, after) = rebalanced(OwnerSeq { root: Some(root) });

    let new_root = after.root.as_ref().expect("still populated");
    assert_eq!(
        new_root.owner.coverage_len, 2,
        "old left child is the new root"
    );
    assert_eq!(new_root.left.as_ref().unwrap().owner.coverage_len, 1);
    assert_eq!(new_root.right.as_ref().unwrap().owner.coverage_len, 3);
    let expected: Model = vec![(1, false), (2, false), (3, false)];
    assert_seq(&after, &expected);
    assert_eq!(
        addresses(&after),
        before,
        "in-order node identity unchanged"
    );
}

#[test]
fn single_left_rotation_restores_an_rr_shape() {
    // RR: mirror of LL.
    let d = n(None, 4, false, None);
    let r = n(None, 3, false, d);
    let root = n(None, 2, false, r).expect("root bf=−2");
    let (before, after) = rebalanced(OwnerSeq { root: Some(root) });

    let new_root = after.root.as_ref().expect("still populated");
    assert_eq!(
        new_root.owner.coverage_len, 3,
        "old right child is the new root"
    );
    assert_eq!(new_root.left.as_ref().unwrap().owner.coverage_len, 2);
    assert_eq!(new_root.right.as_ref().unwrap().owner.coverage_len, 4);
    let expected: Model = vec![(2, false), (3, false), (4, false)];
    assert_seq(&after, &expected);
    assert_eq!(
        addresses(&after),
        before,
        "in-order node identity unchanged"
    );
}

#[test]
fn double_rotation_restores_an_lr_shape() {
    // LR: root bf=+2 whose left child is right-heavy (bf=−1) → the double
    // rotation composes rotate_left(left) + rotate_right(root); the old
    // left child's right child becomes the subtree root.
    let a = n(None, 1, false, None);
    let c = n(None, 3, false, None);
    let b = n(c, 4, false, None);
    let l = n(a, 2, false, b); // bf = −1
    let root = n(l, 9, false, n(None, 8, false, None)).expect("root bf=+2");
    let (before, after) = rebalanced(OwnerSeq { root: Some(root) });

    let new_root = after.root.as_ref().expect("still populated");
    assert_eq!(
        new_root.owner.coverage_len, 4,
        "old inner child is the new root"
    );
    assert_eq!(new_root.left.as_ref().unwrap().owner.coverage_len, 2);
    assert_eq!(new_root.right.as_ref().unwrap().owner.coverage_len, 9);
    let expected: Model = vec![
        (1, false),
        (2, false),
        (3, false),
        (4, false),
        (9, false),
        (8, false),
    ];
    assert_seq(&after, &expected);
    assert_eq!(
        addresses(&after),
        before,
        "in-order node identity unchanged"
    );
}

#[test]
fn double_rotation_restores_an_rl_shape() {
    // RL: mirror of LR.
    let d = n(None, 8, false, None);
    let a = n(None, 6, false, None);
    let b = n(None, 7, false, a);
    let r = n(b, 5, false, d); // bf = +1
    let root = n(n(None, 1, false, None), 9, false, r).expect("root bf=−2");
    let (before, after) = rebalanced(OwnerSeq { root: Some(root) });

    let new_root = after.root.as_ref().expect("still populated");
    assert_eq!(
        new_root.owner.coverage_len, 7,
        "old inner child is the new root"
    );
    assert_eq!(new_root.left.as_ref().unwrap().owner.coverage_len, 9);
    assert_eq!(new_root.right.as_ref().unwrap().owner.coverage_len, 5);
    let expected: Model = vec![
        (1, false),
        (9, false),
        (7, false),
        (6, false),
        (5, false),
        (8, false),
    ];
    assert_seq(&after, &expected);
    assert_eq!(
        addresses(&after),
        before,
        "in-order node identity unchanged"
    );
}

#[test]
fn single_rotation_with_a_balanced_child_is_valid() {
    // The deletion-style case: root bf=+2 whose left child has bf=0. One
    // single rotation is the correct restructuring; every node stays
    // AVL-valid afterwards.
    let ll = n(n(None, 1, false, None), 2, false, n(None, 3, false, None));
    let lr = n(n(None, 4, false, None), 5, false, n(None, 6, false, None));
    let l = n(ll, 2, false, lr); // h3, bf 0
    let root = n(l, 9, false, n(None, 8, false, None)).expect("root bf=+2");
    let (before, after) = rebalanced(OwnerSeq { root: Some(root) });

    let expected: Model = vec![
        (1, false),
        (2, false),
        (3, false),
        (2, false),
        (4, false),
        (5, false),
        (6, false),
        (9, false),
        (8, false),
    ];
    assert_seq(&after, &expected);
    assert_eq!(
        addresses(&after),
        before,
        "in-order node identity unchanged"
    );
}

#[test]
fn rebalancing_an_already_valid_tree_is_an_identity() {
    for size in [1usize, 2, 3, 7, 8] {
        let weights: Vec<usize> = (1..=size).collect();
        let certs = vec![false; size];
        let seq = make(&weights, &certs);
        let (before, after) = rebalanced(seq);
        assert_eq!(addresses(&after), before, "no restructuring at size {size}");
    }
}
