//! I4 phase-1 tests (task contract §7/§13): the update establishes that the
//! old READY state, the canonical edit and the supplied post source are
//! mutually consistent BEFORE any restart/convergence work, and a rejected
//! association leaves the old state coherent (the pre-frontier abort path).
//!
//! The association authority is the shared canonical-edit contract: the
//! update validates identity, byte-range geometry, UTF-8 char boundaries
//! and the length arithmetic. It never re-derives a different edit by
//! diffing old and new source, and it never repairs a malformed edit
//! description.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};

use crate::state::InterpretationId;
use crate::update::{stage, UpdateError};

use super::{edit, Fixture};

fn stage_with(
    old: &crate::state::ReadyDocument,
    old_source: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> Result<crate::update::StagedUpdate, UpdateError> {
    stage(old, old_source, post, edit, &mut NoopWorkSink)
}

#[test]
fn an_edit_that_splits_a_utf8_character_is_rejected() {
    // "中文 text": '文' occupies bytes 3..6, so a range boundary at 4 is not
    // a char boundary. The edit must be rejected, never silently repaired.
    let fixture = Fixture::new("中文 text\n");
    let bad = CanonicalEdit::new(4, 6, "x").expect("construction is range-only");
    let post = Source::new(SourceId(2), "中文 text\n");
    assert!(matches!(
        stage_with(&fixture.old, &fixture.old_source, &post, &bad),
        Err(UpdateError::InvalidAssociation { .. })
    ));
    crate::validate::validate_ready(&fixture.old).expect("the old state stays coherent");
}

#[test]
fn a_post_source_that_does_not_match_the_edit_arithmetic_is_rejected() {
    let fixture = Fixture::new("alpha\n\nbeta\n\ngamma\n");
    let insertion = edit(3, 3, "X");
    // The canonical edit says the post source must be 20 bytes; this one is
    // not, so the edit does not describe the supplied transition.
    let post = Source::new(SourceId(2), "alpha\n\nbeta\n");
    assert!(matches!(
        stage_with(&fixture.old, &fixture.old_source, &post, &insertion),
        Err(UpdateError::InvalidAssociation { .. })
    ));

    // The correct post source for the same edit is accepted.
    let good = insertion
        .apply(&fixture.old_source, SourceId(2))
        .expect("edit applies");
    assert!(stage_with(&fixture.old, &fixture.old_source, &good, &insertion).is_ok());
}

#[test]
fn a_source_identity_mismatch_is_rejected() {
    let fixture = Fixture::new("alpha\n\nbeta\n\ngamma\n");
    let insertion = edit(3, 3, "X");
    let post = insertion
        .apply(&fixture.old_source, SourceId(2))
        .expect("edit applies");
    // Same bytes, different SourceId: the old state does not describe this
    // source object.
    let impostor = Source::new(SourceId(7), "alpha\n\nbeta\n\ngamma\n");
    assert!(matches!(
        stage_with(&fixture.old, &impostor, &post, &insertion),
        Err(UpdateError::InvalidAssociation { .. })
    ));
}

#[test]
fn an_old_state_that_does_not_describe_the_old_source_is_rejected() {
    let fixture = Fixture::new("alpha\n");
    let longer = Source::new(SourceId(1), "alpha\n\nbeta\n");
    let insertion = edit(3, 3, "X");
    let post = insertion.apply(&longer, SourceId(2)).expect("edit applies");
    assert!(matches!(
        stage_with(&fixture.old, &longer, &post, &insertion),
        Err(UpdateError::InvalidAssociation { .. })
    ));
}

#[test]
fn a_state_from_another_interpretation_is_rejected() {
    let fixture = Fixture::new("alpha\n\nbeta\n\ngamma\n");
    let mut foreign = fixture.old.clone();
    foreign.interpretation = InterpretationId {
        grammar: "OTHER-GRAMMAR",
        ..InterpretationId::HORSE_A_V1
    };
    let insertion = edit(3, 3, "X");
    let post = insertion
        .apply(&fixture.old_source, SourceId(2))
        .expect("edit applies");
    assert!(matches!(
        stage_with(&foreign, &fixture.old_source, &post, &insertion),
        Err(UpdateError::InvalidAssociation { .. })
    ));
}

#[test]
fn an_edit_range_past_the_end_of_the_old_source_is_rejected() {
    let fixture = Fixture::new("alpha\n");
    let past_end = CanonicalEdit::new(4, 99, "x").expect("construction is range-only");
    let post = Source::new(SourceId(2), "alphax\n");
    assert!(matches!(
        stage_with(&fixture.old, &fixture.old_source, &post, &past_end),
        Err(UpdateError::InvalidAssociation { .. })
    ));
}
