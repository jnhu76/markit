//! Sequential monotone cursor (I3 task contract §26–§28, §46; #59 §8):
//! one O(H) positioning descent, then source-order advance with no fresh
//! root seek per Owner — a transient path stack borrowed from the
//! sequence, never persisted, never materializing the Owners.

use super::*;

#[test]
fn a_full_cursor_walk_matches_the_model_from_rank_zero() {
    for (weights, certs) in [
        (vec![], vec![]),
        (vec![5], vec![true]),
        (
            vec![1, 2, 3, 7, 31, 2],
            vec![true, false, true, false, true, false],
        ),
        (vec![1; 64], vec![false; 64]),
    ] {
        let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
        let seq = make(&weights, &certs);
        assert_seq(&seq, &model);

        let mut cursor = seq.cursor_at_rank(0);
        let mut seen: Model = Vec::new();
        let mut prev_rank = None;
        while let Some(item) = cursor.next() {
            // rank monotone, no duplicates, no skips
            assert_eq!(item.rank, seen.len(), "rank at position {}", seen.len());
            assert!(prev_rank.is_none_or(|p| item.rank > p), "rank monotone");
            prev_rank = Some(item.rank);
            // absolute Owner base and outgoing boundary cut exact
            let base: usize = seen.iter().map(|(w, _)| *w).sum();
            assert_eq!(item.base, base, "base at rank {}", item.rank);
            assert_eq!(item.boundary_cut, base + item.owner.coverage_len);
            assert_eq!(
                item.owner.outgoing_restart.is_some(),
                model[item.rank].1,
                "certificate presence at rank {}",
                item.rank
            );
            seen.push((
                item.owner.coverage_len,
                item.owner.outgoing_restart.is_some(),
            ));
        }
        assert_eq!(
            seen.as_slice(),
            model.as_slice(),
            "no duplicate, no skipped Owner"
        );
    }
}

#[test]
fn a_cursor_started_at_every_rank_yields_the_exact_suffix() {
    let weights = [2usize, 3, 1, 7, 2, 4, 1];
    let certs = [true, false, false, true, false, true, false];
    let model: Model = weights.iter().copied().zip(certs).collect();
    let seq = make(&weights, &certs);

    for start in 0..=model.len() {
        let mut cursor = seq.cursor_at_rank(start);
        let mut seen: Model = Vec::new();
        while let Some(item) = cursor.next() {
            seen.push((
                item.owner.coverage_len,
                item.owner.outgoing_restart.is_some(),
            ));
        }
        assert_eq!(seen, model[start..], "start rank {start}");
    }

    // Starting at records == N is the exact EOF: nothing follows.
    let mut exhausted = seq.cursor_at_rank(model.len());
    assert!(exhausted.next().is_none(), "EOF start yields nothing");
}

#[test]
fn a_byte_positioned_cursor_starts_at_the_containing_owner() {
    let weights = [2usize, 3, 1, 7];
    let certs = [false, true, false, true];
    let model: Model = weights.iter().copied().zip(certs).collect();
    let seq = make(&weights, &certs);
    let total: usize = weights.iter().sum();

    for x in 0..=total {
        let mut cursor = seq.cursor_at_byte(x);
        let expected_start = expected_at_rank(&model, x);
        match cursor.next() {
            None => assert_eq!(expected_start, model.len(), "x == L is EOF at x={x}"),
            Some(item) => {
                assert_eq!(item.rank, expected_start, "first Owner at x={x}");
                let base: usize = model[..expected_start].iter().map(|(w, _)| *w).sum();
                assert_eq!(item.base, base, "base of the first Owner at x={x}");
            }
        }
        // The remainder is the exact suffix after the starting Owner.
        let mut rest: Model = Vec::new();
        while let Some(item) = cursor.next() {
            rest.push((
                item.owner.coverage_len,
                item.owner.outgoing_restart.is_some(),
            ));
        }
        if expected_start < model.len() {
            assert_eq!(rest, model[expected_start + 1..], "suffix at x={x}");
        } else {
            assert!(rest.is_empty(), "no suffix after EOF at x={x}");
        }
    }
}

fn expected_at_rank(model: &Model, x: usize) -> usize {
    let mut base = 0;
    for (rank, (w, _)) in model.iter().enumerate() {
        if x < base + w {
            return rank;
        }
        base += w;
    }
    model.len()
}

#[test]
fn cursor_preconditions_are_loud() {
    let seq = make(&[1, 2], &[false; 2]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = seq.cursor_at_rank(3);
    }));
    assert!(result.is_err(), "rank > records must fail loudly");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = seq.cursor_at_byte(4);
    }));
    assert!(result.is_err(), "byte > total must fail loudly");
}

#[test]
fn an_empty_sequence_cursors_directly_to_eof() {
    let empty = OwnerSeq::default();
    assert!(empty.cursor_at_rank(0).next().is_none());
    assert!(empty.cursor_at_byte(0).next().is_none());
}
