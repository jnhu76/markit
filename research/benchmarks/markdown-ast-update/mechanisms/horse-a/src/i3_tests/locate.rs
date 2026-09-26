//! `locate_by_byte` (I3 task contract §11–§12, spec §12): weighted
//! one-root-to-leaf descent over `subtree_bytes`, explicit EOF at
//! `x == L`, out-of-range precondition failure, exact rank/base/offset.

use super::*;

/// The Vec-model expectation for byte `x`: `(rank, base, offset)`.
fn expected_at(model: &Model, x: usize) -> (usize, usize, usize) {
    let mut base = 0;
    for (rank, (w, _)) in model.iter().enumerate() {
        if x < base + w {
            return (rank, base, x - base);
        }
        base += w;
    }
    unreachable!("byte {x} is outside the model (expected EOF)");
}

#[test]
fn locate_hits_every_byte_and_boundary_on_uneven_trees() {
    // Varied, deliberately uneven weights (I3 task contract §37): the
    // weighted navigation is the reason the tree exists.
    for weights in [
        vec![1, 2, 3, 7, 31, 2],
        vec![1, 50, 2, 1000, 3],
        vec![5],
        vec![1, 1, 1, 1, 1, 1, 1, 1],
        vec![31, 7, 3, 2, 1],
    ] {
        let model: Model = weights.iter().map(|&w| (w, false)).collect();
        let certs = vec![false; weights.len()];
        let seq = make(&weights, &certs);
        assert_seq(&seq, &model);
        let total: usize = weights.iter().sum();
        for x in 0..total {
            let (rank, base, offset) = expected_at(&model, x);
            match seq.locate_by_byte(x) {
                crate::sequence::Located::Owner(loc) => {
                    assert_eq!(loc.rank, rank, "rank at byte {x} of {weights:?}");
                    assert_eq!(loc.base, base, "base at byte {x} of {weights:?}");
                    assert_eq!(loc.offset, offset, "offset at byte {x} of {weights:?}");
                    assert_eq!(
                        loc.owner.coverage_len, model[rank].0,
                        "coverage_len at byte {x} of {weights:?}"
                    );
                }
                crate::sequence::Located::Eof => {
                    panic!("byte {x} of {total} located as EOF in {weights:?}")
                }
            }
        }
        assert!(
            matches!(seq.locate_by_byte(total), crate::sequence::Located::Eof),
            "x == L must be EOF in {weights:?}"
        );
    }
}

#[test]
fn locate_owner_boundaries_resolve_to_the_next_owner_at_offset_zero() {
    // A boundary byte belongs to the Owner whose coverage starts there —
    // the previous Owner's coverage is half-open [base, base+len).
    let weights = [2, 3, 4];
    let seq = make(&weights, &[false; 3]);
    let mut base = 0;
    for (rank, &w) in weights.iter().enumerate() {
        match seq.locate_by_byte(base) {
            crate::sequence::Located::Owner(loc) => {
                assert_eq!(loc.rank, rank);
                assert_eq!(loc.base, base);
                assert_eq!(loc.offset, 0, "boundary byte starts its own Owner");
            }
            crate::sequence::Located::Eof => panic!("boundary {base} located as EOF"),
        }
        base += w;
    }
}

#[test]
fn locate_eof_is_explicit_including_the_empty_sequence() {
    // Empty sequence: L = 0, x = 0 → EOF (I3 task contract §11).
    let empty = OwnerSeq::default();
    assert!(matches!(
        empty.locate_by_byte(0),
        crate::sequence::Located::Eof
    ));

    let seq = make(&[3, 9], &[false; 2]);
    assert!(matches!(
        seq.locate_by_byte(12),
        crate::sequence::Located::Eof
    ));
}

#[test]
#[should_panic(expected = "out of range")]
fn locate_beyond_total_is_a_precondition_error() {
    let seq = make(&[3, 9], &[false; 2]);
    let _ = seq.locate_by_byte(13);
}

#[test]
fn locate_rank_and_base_stay_exact_on_a_deep_tree() {
    // 64 unit-weight Owners (the deep case of I3 task contract §46): every
    // boundary must resolve to rank = byte, offset 0; EOF exactly at 64.
    let weights = [1usize; 64];
    let seq = make(&weights, &[false; 64]);
    for x in 0..64usize {
        match seq.locate_by_byte(x) {
            crate::sequence::Located::Owner(loc) => {
                assert_eq!(loc.rank, x, "rank at byte {x}");
                assert_eq!(loc.base, x, "base at byte {x}");
                assert_eq!(loc.offset, 0, "offset at byte {x}");
            }
            crate::sequence::Located::Eof => panic!("byte {x} located as EOF"),
        }
    }
    assert!(matches!(
        seq.locate_by_byte(64),
        crate::sequence::Located::Eof
    ));
}
