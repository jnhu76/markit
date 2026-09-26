//! `split` at Owner record rank (I3 task contract §19–§21, §39–§40; spec
//! §12.3, #59 §7.4): rank-based semantics, one search spine, join_with_pivot
//! reconstruction, the accepted `h(output) <= h(input)` proof core, and the
//! withdrawn output-height window deliberately left unfalsified-then-regressed.

use super::*;
use crate::sequence::{join, split};

/// Uneven, deterministic weight/certificate patterns (I3 task contract §37:
/// not every Owner weighs the same).
fn patterns(n: usize) -> (Vec<usize>, Vec<bool>) {
    let weights: Vec<usize> = (0..n)
        .map(|i| match i % 4 {
            0 => 1,
            1 => 7,
            2 => 2,
            _ => 31,
        })
        .collect();
    let certs: Vec<bool> = (0..n).map(|i| i % 3 == 0).collect();
    (weights, certs)
}

#[test]
fn split_is_exact_at_every_rank_for_n_zero_through_64() {
    for n in 0..=64usize {
        let (weights, certs) = patterns(n);
        let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
        let total_bytes: usize = weights.iter().sum();
        let reference = make(&weights, &certs);
        let tree_height = reference.root.as_ref().map_or(0, |r| r.height);

        for k in 0..=n {
            let fresh = make(&weights, &certs);
            let original_addrs = addresses(&fresh);
            let (a, b) = split(fresh.root, k);
            let seq_a = OwnerSeq { root: a };
            let seq_b = OwnerSeq { root: b };

            // Order, records, bytes (I3 task contract §39).
            assert_seq(&seq_a, &model[..k]);
            assert_seq(&seq_b, &model[k..]);
            assert_eq!(seq_a.total_bytes() + seq_b.total_bytes(), total_bytes);

            // Accepted proof core only: h(A) <= h(T), h(B) <= h(T). The
            // withdrawn window (h in [h-2, h+1]) is NOT asserted.
            let ha = seq_a.root.as_ref().map_or(0, |r| r.height);
            let hb = seq_b.root.as_ref().map_or(0, |r| r.height);
            assert!(
                ha <= tree_height,
                "h(A)={ha} <= h(T)={tree_height} at n={n}, k={k}"
            );
            assert!(
                hb <= tree_height,
                "h(B)={hb} <= h(T)={tree_height} at n={n}, k={k}"
            );

            // No node was cloned or rebuilt (I3 task contract §25).
            let mut conserved = addresses(&seq_a);
            conserved.extend(addresses(&seq_b));
            let mut sorted_conserved = conserved.clone();
            sorted_conserved.sort_unstable();
            let mut sorted_original = original_addrs.clone();
            sorted_original.sort_unstable();
            assert_eq!(
                sorted_conserved, sorted_original,
                "address conservation at n={n}, k={k}"
            );

            // join(A, B) reproduces the original Owner order and
            // aggregates (§39). The rejoined height is a join outcome,
            // not part of the identity — the frozen #59 §7.2 window
            // governs it and is asserted in the structural suite.
            let rejoined = OwnerSeq {
                root: join(seq_a.root, seq_b.root),
            };
            assert_seq(&rejoined, &model);
        }
    }
}

#[test]
fn adversarial_split_heights_regist_the_withdrawn_window() {
    // I3 task contract §40: an output height far below h(T)−2 is legal.
    // The withdrawn lemma ("every split output lies in [h−2, h+1]") would
    // reject this; asserting it here is regression evidence against
    // resurrecting that false statement (spec §12.3.1).
    let weights = [1usize; 64];
    let tree = make(&weights, &[false; 64]);
    let tree_height = tree.root.as_ref().unwrap().height; // 7 for 64 unit records
    let (a, b) = split(tree.root, 1);
    let seq_a = OwnerSeq { root: a };
    let seq_b = OwnerSeq { root: b };
    assert_eq!(
        seq_a.root.as_ref().unwrap().height,
        1,
        "single-Owner output"
    );
    assert_eq!(
        tree_height - seq_a.root.as_ref().unwrap().height,
        6,
        "far outside the withdrawn [h−2, h+1] window"
    );
    assert!(seq_b.root.as_ref().unwrap().height <= tree_height);
}

#[test]
fn split_precondition_violations_are_not_fallbacks() {
    let tree = make(&[1, 2, 3], &[false; 3]);
    // k > records is a precondition error, not an algorithmic fallback
    // (I3 task contract §30).
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = split(tree.root, 4);
    }));
    assert!(result.is_err(), "k > records must fail loudly");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = split(None, 1);
    }));
    assert!(
        result.is_err(),
        "k > 0 on an empty sequence must fail loudly"
    );
}
