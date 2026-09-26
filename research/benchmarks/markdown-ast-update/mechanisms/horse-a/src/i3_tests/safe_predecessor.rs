//! `safe_predecessor` (I3 task contract §13–§15, §38; spec §7.1, §12,
//! #59 §7.6): the nearest preceding Owner boundary strictly before an
//! exclusive byte bound that carries a persistent outgoing
//! RestartCertificate — found by aggregate-pruned backtracking, never a
//! linear scan toward BOF, with position eligibility established before
//! certificate inspection.

use super::*;

/// Vec-model expectation: the maximum rank whose Owner is certified and
/// whose outgoing boundary cut is strictly below `before`. Test-side
/// linear scan (allowed in tests; forbidden in the mechanism).
fn expected_safe(model: &Model, before: usize) -> Option<(usize, usize, usize)> {
    let mut base = 0usize;
    let mut best: Option<(usize, usize, usize)> = None;
    for (rank, (w, c)) in model.iter().enumerate() {
        let cut = base + w;
        if *c && cut < before {
            best = Some((rank, base, cut));
        }
        base = cut;
    }
    best
}

fn assert_nearest(seq: &OwnerSeq, model: &Model, before: usize) {
    let owners = seq.owners_in_order();
    match (seq.safe_predecessor(before), expected_safe(model, before)) {
        (None, None) => {}
        (Some(found), Some((rank, base, boundary))) => {
            assert_eq!(found.rank, rank, "rank at before={before}");
            assert_eq!(found.base, base, "base at before={before}");
            assert_eq!(found.boundary, boundary, "boundary at before={before}");
            assert_eq!(found.boundary, found.base + found.owner.coverage_len);
            assert!(
                std::ptr::eq(found.owner, owners[rank]),
                "the returned Owner is the model's certified Owner at rank {rank}"
            );
            assert_eq!(
                found.cert.support.blank_line,
                owners[rank]
                    .outgoing_restart
                    .as_ref()
                    .unwrap()
                    .support
                    .blank_line,
                "the returned certificate is the Owner's own"
            );
        }
        (found, expected) => {
            panic!("safe_predecessor({before}) = {found:?}, model expected {expected:?}")
        }
    }
}

#[test]
fn no_certified_boundaries_yield_none() {
    let empty = OwnerSeq::default();
    assert!(empty.safe_predecessor(0).is_none());

    let seq = make(&[2, 3, 4], &[false; 3]);
    for before in [0usize, 1, 2, 5, 9] {
        assert!(seq.safe_predecessor(before).is_none(), "before={before}");
    }
}

#[test]
fn certificate_patterns_return_the_nearest_prior_eligible() {
    // I3 task contract §38: exercise the frozen pattern list and query
    // every relevant boundary — the result must be the NEAREST prior
    // eligible certificate, never merely some certificate.
    let weights = [2usize, 3, 2, 4, 1, 3];
    let total: usize = weights.iter().sum();
    let patterns: &[&[bool]] = &[
        &[false; 6],                                // no Owners certified
        &[true, false, false, false, false, false], // only first
        &[false, false, true, false, false, false], // only middle
        &[false, false, false, false, true, false], // only last interior
        &[true, false, true, false, true, false],   // alternating
        &[true, true, true, true, true, false],     // all eligible interior
    ];
    for certs in patterns {
        let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
        let seq = make(&weights, certs);
        assert_seq(&seq, &model);
        for before in 0..=total {
            assert_nearest(&seq, &model, before);
        }
    }
}

#[test]
fn a_boundary_at_the_exclusive_bound_yields_the_earlier_certified() {
    // Position eligibility precedes certificate inspection (I3 task
    // contract §14). Owners: [2, 3, 2], owners 0 and 1 certified.
    // before = 5 = cut(owner 1) = base(owner 2): owner 1's own boundary
    // sits AT the exclusive bound (ineligible) even though it carries a
    // certificate — the answer must come from owner 0 via the guided
    // descent into the abandoned left region.
    let weights = [2usize, 3, 2];
    let certs = [true, true, false];
    let model: Model = weights.iter().copied().zip(certs).collect();
    let seq = make(&weights, &certs);

    let found = seq
        .safe_predecessor(5)
        .expect("owner 0 is certified at cut 2");
    assert_eq!(
        found.rank, 0,
        "the boundary at the exclusive bound is skipped"
    );
    assert_eq!(found.boundary, 2);

    // One byte further, owner 1's boundary is strictly below the bound.
    let found = seq
        .safe_predecessor(6)
        .expect("owner 1 is certified at cut 5");
    assert_eq!(found.rank, 1);
    assert_eq!(found.boundary, 5);
    assert_nearest(&seq, &model, 5);
    assert_nearest(&seq, &model, 6);
}

#[test]
fn the_eof_boundary_is_never_eligible_below_the_total() {
    // Even where a synthetic certificate sits on the final Owner, its
    // cut == L can never be strictly below before = L; earlier certified
    // boundaries win.
    let weights = [2usize, 3];
    let certs = [false, true];
    let model: Model = weights.iter().copied().zip(certs).collect();
    let seq = make(&weights, &certs);
    for before in 0..=5usize {
        assert!(seq.safe_predecessor(before).is_none(), "before={before}");
    }
    assert_nearest(&seq, &model, 5);
}

#[test]
fn nearest_prior_eligible_holds_on_a_deep_uneven_tree() {
    let weights: Vec<usize> = (0..32).map(|i| if i % 7 == 0 { 31 } else { 1 }).collect();
    let certs: Vec<bool> = (0..32).map(|i| i % 3 == 1).collect();
    let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
    let seq = make(&weights, &certs);
    assert_seq(&seq, &model);
    let total: usize = weights.iter().sum();
    for before in 0..=total {
        assert_nearest(&seq, &model, before);
    }
}

#[test]
fn safe_predecessor_bounds_are_preconditions() {
    let seq = make(&[2, 3], &[true, false]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = seq.safe_predecessor(6);
    }));
    assert!(result.is_err(), "before > total must fail loudly");
}
