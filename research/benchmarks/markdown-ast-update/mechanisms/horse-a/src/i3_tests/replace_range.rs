//! `replace_range` (I3 task contract §22–§25, §44; spec §12.4, #59 §7.5):
//! the frozen split/split/join/join formula with Owner record ranks, the
//! removed range B returned to the caller (never retired here), retained
//! P/S structurally transferred by ownership, and pointer identity for
//! every surviving node.

use super::*;

fn sorted(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v
}

#[test]
fn replace_range_is_exact_for_every_lo_hi_and_middle_shape() {
    // I3 task contract §44: for small N, every lo in 0..=N and every
    // hi in lo..=N, with an empty / one-Owner / multi-Owner middle,
    // against a Vec model — order, aggregates, AVL validity, and the
    // allocation identity of retained P/S and the removed range.
    for n in 0..=9usize {
        let weights: Vec<usize> = (0..n).map(|i| i * 3 + 1).collect();
        let certs: Vec<bool> = (0..n).map(|i| i % 2 == 0).collect();
        let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();

        for lo in 0..=n {
            for hi in lo..=n {
                for (mid_weights, mid_certs) in [
                    (vec![], vec![]),
                    (vec![100], vec![true]),
                    (vec![101, 7, 102], vec![false, true, false]),
                ] {
                    let mut seq = make(&weights, &certs);
                    let original_addrs = addresses(&seq);
                    let middle = make(&mid_weights, &mid_certs);
                    let middle_addrs = addresses(&middle);

                    let removed = seq.replace_range(lo, hi, middle);
                    let removed_seq = removed;

                    // New sequence = old prefix + middle + old suffix.
                    let mut expected: Model = model[..lo].to_vec();
                    expected.extend(mid_weights.iter().copied().zip(mid_certs.iter().copied()));
                    expected.extend(model[hi..].iter().copied());
                    assert_seq(&seq, &expected);

                    // Removed = exactly the old [lo, hi) — returned, never
                    // dropped inside replace_range (I3 task contract §22).
                    assert_seq(&removed_seq, model[lo..hi].to_vec().as_ref());

                    // Pointer identity: retained P/S nodes keep their
                    // allocation; the removed subtree keeps its own; the
                    // middle was spliced, not rebuilt (I3 task contract
                    // §24/§25).
                    let new_addrs = addresses(&seq);
                    let removed_addrs = addresses(&removed_seq);
                    let mut conserved = new_addrs.clone();
                    conserved.extend(removed_addrs.iter().copied());
                    let mut expected_conserved = original_addrs.clone();
                    expected_conserved.extend(middle_addrs.iter().copied());
                    assert_eq!(
                        sorted(conserved),
                        sorted(expected_conserved),
                        "allocation identity at n={n}, lo={lo}, hi={hi}, mid={mid_weights:?}"
                    );
                    for addr in middle_addrs {
                        assert!(
                            new_addrs.contains(&addr),
                            "middle node spliced at n={n}, lo={lo}, hi={hi}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn replace_range_named_edge_cases_match_the_frozen_list() {
    // I3 task contract §23, each named case spelled out once.
    let weights = [1, 2, 3, 4, 5];
    let certs = [false, true, false, true, false];
    let model: Model = weights.iter().copied().zip(certs).collect();

    // Insertion into an empty sequence (lo == hi == 0).
    let mut empty = OwnerSeq::default();
    let removed = empty.replace_range(0, 0, make(&[9, 9], &[true, false]));
    assert_seq(&empty, &[(9, true), (9, false)]);
    assert!(removed.is_empty());

    // lo == hi inside a sequence: pure insertion.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(2, 2, make(&[42], &[true]));
    let mut expected = model.clone();
    expected.insert(2, (42, true));
    assert_seq(&seq, &expected);
    assert!(removed.is_empty());

    // lo = 0: replace the prefix.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(0, 2, make(&[50], &[false]));
    assert_seq(&seq, &[(50, false), (3, false), (4, true), (5, false)]);
    assert_seq(&removed, &model[..2]);

    // hi = records: replace the suffix.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(3, 5, OwnerSeq::default());
    assert_seq(&seq, &model[..3]);
    assert_seq(&removed, &model[3..]);

    // lo = 0, hi = records: replace everything.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(0, 5, make(&[60, 61], &[false, false]));
    assert_seq(&seq, &[(60, false), (61, false)]);
    assert_seq(&removed, &model);

    // middle empty: pure deletion of exactly one Owner.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(1, 2, OwnerSeq::default());
    assert_seq(&seq, &[model[0], model[2], model[3], model[4]]);
    assert_seq(&removed, &model[1..2]);

    // middle non-empty: replace exactly one Owner.
    let mut seq = make(&weights, &certs);
    let removed = seq.replace_range(4, 5, make(&[70, 71, 72], &[true, false, true]));
    let mut expected = model[..4].to_vec();
    expected.extend([(70, true), (71, false), (72, true)]);
    assert_seq(&seq, &expected);
    assert_seq(&removed, &model[4..]);
}

#[test]
fn replace_range_precondition_violations_are_not_fallbacks() {
    let mut seq = make(&[1, 2, 3], &[false; 3]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = seq.replace_range(2, 1, OwnerSeq::default());
    }));
    assert!(result.is_err(), "lo > hi must fail loudly");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = seq.replace_range(0, 4, OwnerSeq::default());
    }));
    assert!(result.is_err(), "hi > records must fail loudly");
}
