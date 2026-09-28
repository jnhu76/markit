//! Post-merge corrective regressions (#62, pre-frontier workspace
//! formation semantics): the two defects the focused post-PR-#70 review
//! found are pinned here so they cannot silently return.
//!
//! 1. OLD READY PRESERVATION — a workspace reservation refusal is an
//!    ordinary PRE-FRONTIER error: it must occur while the caller still
//!    owns the old READY document (staging borrows it), and the old
//!    state must remain valid and USABLE afterwards — meaningful use
//!    (validation, export, a subsequent legal update), not merely "did
//!    not crash while dropping".
//!
//! 2. LOGICAL STACK BOUND — the workspace guard is the pre-derived
//!    LOGICAL limit, never `Vec::capacity()` (an allocator/runtime
//!    fact). Pushes up to the limit succeed; one push past the limit
//!    trips the bounded-stack invariant even when the allocator holds
//!    spare capacity.
//!
//! Deterministic allocation refusal is induced through a TEST-ONLY
//! one-shot thread-local seam compiled out of every non-test build
//! (`crate::workspace::arm_reservation_failure_for_tests`) — there is no
//! production fault-injection mechanism. A genuinely refusing global
//! allocator cannot be used here: staging legitimately performs many
//! infallible `Vec` allocations before reaching the workspace
//! reservation, and those abort rather than error.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_shared_grammar::parse_full;

use crate::full_build::full_build;
use crate::structural::NoopHorseAStructuralSink;
use crate::update::{stage, UpdateError};
use crate::workspace::{arm_reservation_failure_for_tests, BoundedStack};
use crate::NormalizeV1;

fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

/// The #60 witness-shape fixture (three paragraph Owners, local route).
const WITNESS_SOURCE: &str = "alpha\n\nbeta\n\ngamma\n";

/// P1-1 regression: a workspace formation refusal happens BEFORE the
/// frontier — staging (which reserves the workspace) only borrows the old
/// READY document, returns `UpdateError::ResourceRefused`, and the old
/// state remains valid and usable.
#[test]
fn workspace_reservation_failure_preserves_old_ready() {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build");
    let e = edit(13, 13, "X");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");

    // Arm the TEST-ONLY one-shot seam: the NEXT workspace reservation on
    // this thread refuses exactly where the fallible try_reserve_exact
    // runs. The refusal must surface as an ordinary pre-frontier error.
    arm_reservation_failure_for_tests();
    match stage(
        &old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    ) {
        Err(UpdateError::ResourceRefused { .. }) => {}
        other => panic!(
            "a refused workspace reservation must be a pre-frontier \
             UpdateError::ResourceRefused, got: {other:?}"
        ),
    }

    // The old READY state was never consumed: it is still OWNED here and
    // provably usable — meaningful use, not just non-crash.
    // (1) it still satisfies the READY invariants;
    crate::validate_ready(&old).expect("old READY still validates");
    // (2) it still exports the correct normalized result;
    let exported = old.normalize_v1();
    let clean = parse_full(old_source.as_bytes(), &mut NoopWorkSink);
    assert_eq!(
        exported, clean,
        "old READY still normalizes to the clean parse of its source"
    );
    // (3) a subsequent legal update operation still completes from it
    // (the seam is one-shot, so this staging's reservation succeeds).
    let next = crate::update::update_with_structural(
        old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("the same edit completes once the reservation is serviced");
    assert_eq!(next.source_id, SourceId(2));
    assert_eq!(next.owners.records(), 3);
    let next_clean = parse_full(post.as_bytes(), &mut NoopWorkSink);
    assert_eq!(
        next.normalize_v1(),
        next_clean,
        "the retry produces the correct next READY state"
    );
}

/// P2 regression, part 1: pushes up to the LOGICAL limit succeed while
/// the underlying allocator capacity is strictly larger — the guard is
/// the limit, not the capacity, and the test never assumes
/// `Vec::capacity() == requested` (it only requires the guaranteed
/// `capacity >= requested`, which the over-reservation makes strictly
/// greater than the limit here).
#[test]
fn bounded_stack_accepts_exactly_the_logical_limit_of_pushes() {
    let limit = 3usize;
    let mut stack: BoundedStack<u8> = BoundedStack::with_over_reservation_for_tests(limit);
    assert!(
        stack.allocator_capacity() >= limit,
        "the underlying capacity must cover the logical limit"
    );
    assert!(
        stack.allocator_capacity() > limit,
        "this fixture deliberately holds allocator room PAST the limit"
    );
    for i in 0..limit as u8 {
        stack.push_bounded(i);
    }
    assert_eq!(stack.len(), limit);
    assert!(!stack.is_empty());
    // Drain works and preserves LIFO order.
    let mut drained = Vec::new();
    while let Some(f) = stack.pop() {
        drained.push(f);
    }
    assert_eq!(drained, vec![2, 1, 0]);
    assert!(stack.is_empty());
}

/// P2 regression, part 2: ONE push past the logical limit trips the
/// bounded-stack invariant — deterministically, across allocators, and
/// even though the allocator still holds spare capacity (the push would
/// NOT have reallocated).
#[test]
#[should_panic(
    expected = "Horse-A workspace logical bound exceeded: push at depth 3 would pass the pre-derived limit 3"
)]
fn bounded_stack_refuses_one_push_past_the_logical_limit() {
    let limit = 3usize;
    let mut stack: BoundedStack<u8> = BoundedStack::with_over_reservation_for_tests(limit);
    assert!(stack.allocator_capacity() > limit);
    for i in 0..limit as u8 {
        stack.push_bounded(i);
    }
    // The (limit+1)-th push must panic on the logical bound.
    stack.push_bounded(99);
}
