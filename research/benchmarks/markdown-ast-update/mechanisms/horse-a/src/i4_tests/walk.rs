//! I4 candidate-walk unit tests (spec §7.3, §12–§14; #59 §8; task contract
//! §12–§15/§33). The walk is exercised directly on synthetic sequences
//! whose boundaries, bases and certificate supports are exact, so every
//! offered position and every predicate clause is pinned independently of
//! Markdown parsing: restart-cut exclusion, damage crossing, exact
//! old↔new mapping, support-touch invalidation, coverage closure, and
//! exhaustion.
//!
//! The synthetic support convention (mirrors `RestartSupport`): a
//! certificate is built from a *blank line* interval `b` and its
//! establishing LF, so its support byte set is
//! `{b.start - 1} ∪ [b.start, b.end)` — the frozen §6 support.
//! Coordinates here are Owner-relative; the walk applies each candidate's
//! own absolute base.

use std::ops::Range;

use crate::candidate::{CandidateWalk, ConvergencePolicy, WalkOutcome};
use crate::certificate::{RestartCertificate, RestartSupport};
use crate::state::{Owner, OwnerPayload, OwnerSeq};

/// A synthetic Owner: only coverage length and certificate presence/support
/// are mechanically significant to the walk (payload is TriviaOnly).
fn owner(len: usize, blank_line: Option<Range<usize>>) -> Owner {
    Owner {
        coverage_len: len,
        payload: OwnerPayload::TriviaOnly,
        outgoing_restart: blank_line.map(|b| RestartCertificate {
            support: RestartSupport {
                preceding_lf: Some(b.start - 1),
                blank_line: b,
            },
        }),
    }
}

/// Three 20-byte Owners (bases 0, 20, 40; boundaries 20, 40, 60) with the
/// given certificate blank-line intervals, all near the end of their own
/// coverage so the synthetic supports are distinct in absolute space.
fn sequence(certs: [Option<Range<usize>>; 3]) -> OwnerSeq {
    OwnerSeq::bulk_build(vec![
        owner(20, certs[0].clone()),
        owner(20, certs[1].clone()),
        owner(20, certs[2].clone()),
    ])
}

/// The default fixture: every boundary certified, support at relative
/// bytes `17..20` → absolute supports {17,18,19} / {37,38,39} / {57,58,59}.
fn certified_sequence() -> OwnerSeq {
    sequence([Some(17..20), Some(17..20), Some(17..20)])
}

fn policy(restart_cut: usize, edit: (usize, usize), delta: i128) -> ConvergencePolicy {
    ConvergencePolicy {
        restart_cut,
        edit_start: edit.0,
        edit_end: edit.1,
        delta,
    }
}

#[test]
fn the_restart_cut_is_never_offered_as_a_convergence_candidate() {
    let owners = certified_sequence();
    // Deliberately wrong initialization: `first_rank` includes the Owner
    // whose outgoing cut IS the restart cut. The walk's own guard must
    // still refuse to offer it, even though every other clause of the
    // predicate would hold for it.
    let mut walk = CandidateWalk::begin(&owners, 0, policy(20, (5, 5), 0));

    // A barrier exactly at the restart cut: the guard must not converge.
    assert!(
        !matches!(walk.on_sealed_barrier(20, true), WalkOutcome::Converged(_)),
        "the restart cut may never be offered as a convergence candidate"
    );

    // The next certified boundary strictly after the restart is offered
    // and accepted.
    match walk.on_sealed_barrier(40, true) {
        WalkOutcome::Converged(accepted) => {
            assert_eq!(accepted.old_cut, 40);
            assert_eq!(accepted.certified_rank, 1);
            assert!(
                accepted.old_cut > 20,
                "the first candidate must begin strictly after the restart cut"
            );
        }
        other => panic!("expected convergence strictly after the restart cut, got {other:?}"),
    }
}

#[test]
fn boundaries_at_or_before_the_restart_cut_are_never_offered() {
    let owners = certified_sequence();
    // Restart at 40: the certified boundaries 20 and 40 are both excluded.
    let mut walk = CandidateWalk::begin(&owners, 0, policy(40, (0, 0), 0));
    assert!(
        !matches!(walk.on_sealed_barrier(20, true), WalkOutcome::Converged(_)),
        "a boundary below the restart cut must not be offered"
    );
    assert!(
        !matches!(walk.on_sealed_barrier(40, true), WalkOutcome::Converged(_)),
        "the restart cut itself must not be offered"
    );
    match walk.on_sealed_barrier(60, true) {
        WalkOutcome::Converged(accepted) => assert_eq!(accepted.old_cut, 60),
        other => panic!("expected the boundary strictly after the restart, got {other:?}"),
    }
}

#[test]
fn damage_must_be_fully_crossed_before_a_candidate_is_eligible() {
    let owners = certified_sequence();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 25), 0));
    // The first candidate (20) is reached but the deleted range is not
    // fully behind it yet.
    assert_eq!(walk.on_sealed_barrier(20, true), WalkOutcome::Continue);
    match walk.on_sealed_barrier(40, true) {
        WalkOutcome::Converged(accepted) => {
            assert_eq!(accepted.old_cut, 40);
            assert_eq!(accepted.certified_rank, 1);
        }
        other => panic!("expected the first fully-crossed candidate to win, got {other:?}"),
    }
}

#[test]
fn the_mapped_cut_must_equal_the_sealed_barrier() {
    let owners = certified_sequence();
    // delta = +5, so the candidate at 20 may only converge on a new-side
    // barrier at 25.
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 0), 5));
    assert_eq!(
        walk.on_sealed_barrier(24, true),
        WalkOutcome::Continue,
        "the parser has not reached the candidate's mapped cut"
    );
    match walk.on_sealed_barrier(25, true) {
        WalkOutcome::Converged(accepted) => {
            assert_eq!(accepted.old_cut, 20);
            assert_eq!(accepted.new_cut, 25);
            assert_eq!(accepted.certified_rank, 0);
        }
        other => panic!("expected convergence at the mapped cut, got {other:?}"),
    }
}

#[test]
fn a_crossed_candidate_without_its_barrier_is_rejected_and_the_walk_moves_on() {
    let owners = certified_sequence();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 0), 0));
    // A barrier past the first candidate's cut: the first candidate is
    // crossed but has no barrier of its own, so it is rejected once and
    // the walk stays on the still-unreached second candidate.
    assert_eq!(walk.on_sealed_barrier(30, true), WalkOutcome::Continue);
    match walk.on_sealed_barrier(40, true) {
        WalkOutcome::Converged(accepted) => assert_eq!(accepted.old_cut, 40),
        other => panic!("expected the later candidate to converge, got {other:?}"),
    }
}

#[test]
fn first_valid_convergence_wins_when_a_later_candidate_would_also_match() {
    let owners = certified_sequence();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 0), 0));
    // Both candidates at 20 and 40 map exactly onto their own barriers and
    // are otherwise eligible. There is no cost selector: the first valid
    // convergence is accepted and nothing is searched further.
    match walk.on_sealed_barrier(20, true) {
        WalkOutcome::Converged(accepted) => assert_eq!(accepted.old_cut, 20),
        other => panic!("expected the first valid convergence to win, got {other:?}"),
    }
}

#[test]
fn support_touch_invalidates_an_otherwise_valid_candidate() {
    let owners = certified_sequence();
    // A zero-length insertion at absolute byte 38, inside the second
    // Owner's support set {37,38,39} (base 20 + relative {17,18,19}).
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (38, 38), 0));
    assert_eq!(walk.on_sealed_barrier(20, true), WalkOutcome::Continue);
    assert_eq!(
        walk.on_sealed_barrier(40, true),
        WalkOutcome::Continue,
        "a candidate whose certificate support was touched must be skipped"
    );
    match walk.on_sealed_barrier(60, true) {
        WalkOutcome::Converged(accepted) => assert_eq!(accepted.old_cut, 60),
        other => panic!("expected the untouched later certificate to converge, got {other:?}"),
    }
}

#[test]
fn replacement_prefix_must_already_contain_a_top_level_block() {
    let owners = certified_sequence();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 0), 0));
    // Everything else holds for the candidate at 20, but the replacement
    // prefix has no sealed top-level block yet: cutting the coverage there
    // would strand those bytes in front of the retained suffix.
    assert_eq!(
        walk.on_sealed_barrier(20, false),
        WalkOutcome::Continue,
        "a blockless replacement prefix is not a legal coverage cut"
    );
    match walk.on_sealed_barrier(40, true) {
        WalkOutcome::Converged(accepted) => assert_eq!(accepted.old_cut, 40),
        other => panic!("expected convergence once a block exists, got {other:?}"),
    }
}

#[test]
fn without_a_certified_boundary_the_walk_is_exhausted() {
    let owners = sequence([None, None, None]);
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 0), 0));
    assert_eq!(walk.on_sealed_barrier(20, true), WalkOutcome::Exhausted);
    assert_eq!(walk.on_sealed_barrier(60, true), WalkOutcome::Exhausted);
}

#[test]
fn an_old_boundary_without_a_new_side_image_is_skipped() {
    // A deletion large enough that the early candidates have no new-side
    // image at all (delta = -25): they can never map onto a barrier.
    let owners = certified_sequence();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 30), -25));
    match walk.on_sealed_barrier(5, true) {
        WalkOutcome::Converged(accepted) => {
            // Only the last boundary (60 -> 35) can still match; it is not
            // reached by a barrier at 5.
            panic!("no candidate may converge before its mapped cut: {accepted:?}");
        }
        WalkOutcome::Continue | WalkOutcome::Exhausted => {}
    }
    match walk.on_sealed_barrier(35, true) {
        WalkOutcome::Converged(accepted) => {
            assert_eq!(accepted.old_cut, 60);
            assert_eq!(accepted.new_cut, 35);
        }
        // The candidate at 40 (mapped 15) and 20 (mapped -5) must have been
        // skipped: 40 >= edit_end (30) holds, but its mapped cut is behind
        // the barrier, so it is rejected by the mapping clause.
        other => panic!("expected the last candidate to converge, got {other:?}"),
    }
}
