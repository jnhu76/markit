//! Document-authority behavior tests. Every test exercises the public
//! Document contract described by `docs/product/architecture.md` §2.1:
//! one document-content root with explicit revision, base-revision
//! validation, undo/redo as new commits, pinned reads, and the saved
//! baseline / dirty relation.

use markit_document::{
    Document, DocumentError, SourceEdit, SourceOffset, SourceRange, SourceRevision,
};

/// Tracer bullet — a successful commit produces a new revision and the
/// source reflects the replacement.
#[test]
fn commit_increments_revision_and_updates_source() {
    let mut doc = Document::new("# Hello\n");
    assert_eq!(doc.revision().get(), 0);

    let rev = doc
        .commit(SourceEdit::replace(
            doc.id(),
            doc.revision(),
            SourceRange::new(SourceOffset::new(2), SourceOffset::new(7)),
            "Goodbye".to_owned(),
        ))
        .expect("valid base revision");

    assert_eq!(rev.get(), 1);
    assert_eq!(doc.revision(), rev);
    assert_eq!(doc.source(), "# Goodbye\n");
}

/// A stale base revision is rejected explicitly and the document stays
/// exactly as it was — no silent rebase (architecture §2.1 rule 4).
#[test]
fn stale_base_revision_is_rejected_without_mutating_the_document() {
    let mut doc = Document::new("# Hello\n");
    let stale = doc.revision();
    let len = doc.source().len();
    doc.commit(SourceEdit::replace(
        doc.id(),
        stale,
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(len)),
        "# Changed\n".to_owned(),
    ))
    .expect("fresh base");

    let error = doc
        .commit(SourceEdit::replace(
            doc.id(),
            stale,
            SourceRange::new(SourceOffset::new(0), SourceOffset::new(0)),
            "# Stale\n".to_owned(),
        ))
        .expect_err("stale base must be rejected");

    assert_eq!(
        error,
        DocumentError::StaleBase {
            current: doc.revision(),
            edit_base: stale,
        }
    );
    assert_eq!(
        doc.source(),
        "# Changed\n",
        "a rejected edit mutates nothing"
    );
}

/// An edit addressed to one document cannot be committed by another, so
/// same-shaped revision numbers can never smuggle edits across documents.
#[test]
fn edit_for_another_document_is_rejected() {
    let mut doc = Document::new("# Hello\n");
    let other = Document::new("# Other\n");

    let error = doc
        .commit(SourceEdit::replace(
            other.id(),
            other.revision(),
            SourceRange::new(SourceOffset::new(0), SourceOffset::new(0)),
            "x".to_owned(),
        ))
        .expect_err("cross-document edit must be rejected");

    assert_eq!(
        error,
        DocumentError::DocumentMismatch {
            expected: doc.id(),
            actual: other.id(),
        }
    );
    assert_eq!(doc.source(), "# Hello\n");
}

/// A range that is unordered or runs past the source is rejected instead
/// of clamped or wrapped.
#[test]
fn out_of_bounds_or_unordered_range_is_rejected() {
    let mut doc = Document::new("# Hello\n");
    let len = doc.source().len() as u32;

    let past_end = doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(len as usize + 1)),
        "x".to_owned(),
    ));
    assert!(matches!(past_end, Err(DocumentError::RangeInvalid { .. })));

    let unordered = doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(4), SourceOffset::new(2)),
        "x".to_owned(),
    ));
    assert!(matches!(unordered, Err(DocumentError::RangeInvalid { .. })));

    assert_eq!(doc.source(), "# Hello\n", "rejected edits mutate nothing");
}

/// An edit boundary inside a multi-byte scalar value is rejected; CJK and
/// emoji content survives intact edits byte-for-byte.
#[test]
fn cjk_emoji_edits_reject_split_scalars_and_preserve_valid_source() {
    //                      bytes: #=0 ' '=1 世=2..5 界=5..8 ' '=8 🌍=9..13 \n=13
    let mut doc = Document::new("# 世界 🌍\n");

    let split = doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(3), SourceOffset::new(5)),
        "x".to_owned(),
    ));
    assert_eq!(
        split,
        Err(DocumentError::NotCharBoundary {
            offset: SourceOffset::new(3)
        })
    );

    let len = doc.source().len();
    let rev = doc
        .commit(SourceEdit::replace(
            doc.id(),
            doc.revision(),
            SourceRange::new(SourceOffset::new(len - 1), SourceOffset::new(len)),
            "\n\n## 地球 🗺️ note\n".to_owned(),
        ))
        .expect("boundary-aligned edit");

    assert_eq!(rev.get(), 1);
    assert_eq!(doc.source(), "# 世界 🌍\n\n## 地球 🗺️ note\n");
    assert!(doc.source().chars().all(|c| c != '\u{FFFD}'));
}

/// Revision numbers are per-document counters: two documents at the same
/// numeric revision are distinct states and their snapshots never alias.
#[test]
fn same_numeric_revision_in_two_documents_never_aliases() {
    let mut a = Document::new("# A\n");
    let mut b = Document::new("# B\n");

    let edit_for = |doc: &Document, text: &str| {
        SourceEdit::replace(
            doc.id(),
            doc.revision(),
            SourceRange::new(SourceOffset::new(2), SourceOffset::new(3)),
            text.to_owned(),
        )
    };
    a.commit(edit_for(&a, "AAA")).expect("a commit");
    b.commit(edit_for(&b, "BBB")).expect("b commit");

    assert_eq!(
        a.revision(),
        b.revision(),
        "same number, different documents"
    );

    let pin_a = a.snapshot();
    assert_eq!(pin_a.document_id(), a.id());
    assert_ne!(pin_a.document_id(), b.id());
    assert_eq!(pin_a.source(), "# AAA\n");
    assert_eq!(b.snapshot().source(), "# BBB\n");
}

/// A pinned snapshot for revision N keeps reading N — same identity, same
/// bytes — after the document advances to N+1 (architecture ARCH-04).
#[test]
fn pin_n_remains_n_after_commit_n_plus_one() {
    let mut doc = Document::new("# Hello\n");
    let pin = doc.snapshot();
    assert_eq!(pin.revision().get(), 0);

    let len = doc.source().len();
    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(len)),
        "# Goodbye\n".to_owned(),
    ))
    .expect("commit past the pin");

    assert_eq!(doc.revision().get(), 1);
    assert_eq!(doc.source(), "# Goodbye\n");
    assert_eq!(
        pin.revision().get(),
        0,
        "the pin must not upgrade to latest"
    );
    assert_eq!(pin.source(), "# Hello\n");
    assert_eq!(pin.document_id(), doc.id());
}

/// Closing (dropping) the active document does not invalidate an
/// explicitly retained pin; release is dropping the pin itself.
#[test]
fn closing_the_document_leaves_a_retained_pin_readable() {
    let doc = Document::new("# Persisted\n");
    let pin = doc.snapshot();
    let id = doc.id();
    drop(doc);

    assert_eq!(pin.document_id(), id);
    assert_eq!(pin.revision().get(), 0);
    assert_eq!(pin.source(), "# Persisted\n");
}

/// Undo is a new commit: revision advances, content returns to the prior
/// state; redo replays forward the same way. Revision 4 holding the same
/// bytes as revision 0 is a different state — identities never ABA-reuse.
#[test]
fn undo_and_redo_produce_new_revisions_and_restore_content() {
    let mut doc = Document::new("# Hello\n");

    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(2), SourceOffset::new(7)),
        "Goodbye".to_owned(),
    ))
    .expect("e1");
    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(1)),
        "&".to_owned(),
    ))
    .expect("e2");
    assert_eq!(doc.source(), "& Goodbye\n");

    let undo1 = doc.undo().expect("undo e2");
    assert_eq!(undo1.get(), 3);
    assert_eq!(doc.source(), "# Goodbye\n");

    let undo2 = doc.undo().expect("undo e1");
    assert_eq!(undo2.get(), 4);
    assert_eq!(doc.source(), "# Hello\n");
    assert_ne!(doc.revision(), SourceRevision::INITIAL, "no ABA reuse");

    let redo1 = doc.redo().expect("redo e1");
    assert_eq!(redo1.get(), 5);
    assert_eq!(doc.source(), "# Goodbye\n");

    let redo2 = doc.redo().expect("redo e2");
    assert_eq!(redo2.get(), 6);
    assert_eq!(doc.source(), "& Goodbye\n");
}

/// Linear history edges: undo stops at the initial revision, redo stops at
/// the tip, and a fresh commit after undo discards the redo path.
#[test]
fn undo_redo_edges_and_new_commit_truncates_redo() {
    let mut doc = Document::new("# Hello\n");
    assert_eq!(doc.undo(), None, "nothing before the initial revision");

    let len = doc.source().len();
    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(len)),
        "# Second\n".to_owned(),
    ))
    .expect("e1");
    assert_eq!(doc.redo(), None, "nothing to redo at the tip");

    doc.undo().expect("back to initial");
    assert_eq!(doc.source(), "# Hello\n");

    let len = doc.source().len();
    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(len)),
        "# Diverged\n".to_owned(),
    ))
    .expect("e2 after undo");
    assert_eq!(doc.source(), "# Diverged\n");
    assert_eq!(
        doc.redo(),
        None,
        "a fresh commit discards the discarded branch's redo path"
    );
}

/// Undo/redo history is byte-coordinate based, so multi-byte replacements
/// replay exactly over CJK/emoji content.
#[test]
fn undo_redo_replays_multibyte_replacements_exactly() {
    let mut doc = Document::new("# 世界\n");

    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(0)),
        "## 标题 🌍\n".to_owned(),
    ))
    .expect("insert heading");

    doc.undo().expect("undo the insertion");
    assert_eq!(doc.source(), "# 世界\n");

    doc.redo().expect("redo the insertion");
    assert_eq!(doc.source(), "## 标题 🌍\n# 世界\n");
}

/// Every successful commit produces a new revision — including a
/// zero-length replacement, which changes no bytes but is still a state
/// transition. "Successful" never means "revision unchanged".
#[test]
fn an_empty_replacement_is_a_valid_commit_to_a_new_revision() {
    let mut doc = Document::new("# Hello\n");
    let base = doc.revision();

    let rev = doc
        .commit(SourceEdit::replace(
            doc.id(),
            base,
            SourceRange::new(SourceOffset::new(4), SourceOffset::new(4)),
            "",
        ))
        .expect("an empty replacement is a valid edit");

    assert_eq!(rev.get(), 1);
    assert_eq!(doc.source(), "# Hello\n", "no bytes changed");
    doc.undo().expect("the no-op is still one undo step");
    assert_eq!(doc.revision().get(), 2);
}

/// The saved baseline tracks which revision the file holds: edits make
/// the document dirty again, and a late save of revision N does not clean
/// a document that already advanced to N+1 (the save race).
#[test]
fn saved_baseline_tracks_exactly_the_revision_that_was_saved() {
    let mut doc = Document::new("# Hello\n");
    assert!(doc.is_dirty(), "never-saved content is dirty");

    let opened = doc.snapshot();
    doc.mark_saved(&opened)
        .expect("the opened content is on disk");
    assert!(!doc.is_dirty());

    doc.commit(SourceEdit::replace(
        doc.id(),
        doc.revision(),
        SourceRange::new(SourceOffset::new(2), SourceOffset::new(7)),
        "Bye".to_owned(),
    ))
    .expect("edit past the save");
    assert!(doc.is_dirty());

    // The save of the OLD revision completes after the edit committed:
    // marking the opened snapshot saved must not clean the new revision.
    doc.mark_saved(&opened)
        .expect("a late save of an older revision is still a save");
    assert_eq!(doc.saved_revision(), Some(opened.revision()));
    assert!(doc.is_dirty(), "the edited revision was never saved");

    let edited = doc.snapshot();
    doc.mark_saved(&edited).expect("save current");
    assert!(!doc.is_dirty());

    doc.undo().expect("undo past the save");
    assert!(
        doc.is_dirty(),
        "undo is a new revision, not a return to the saved one"
    );
}

/// The saved baseline carries document identity: another document's
/// snapshot — even at the same numeric revision — can never mark this
/// document's unsaved bytes clean.
#[test]
fn marking_another_documents_revision_saved_is_rejected() {
    let mut doc = Document::new("# Hello\n");
    let other = Document::new("# Other\n");
    let foreign = other.snapshot();
    assert_eq!(
        foreign.revision(),
        doc.revision(),
        "same number, different document"
    );

    let error = doc.mark_saved(&foreign);
    assert_eq!(
        error,
        Err(DocumentError::DocumentMismatch {
            expected: doc.id(),
            actual: foreign.document_id(),
        })
    );
    assert_eq!(doc.saved_revision(), None, "a refused mark saves nothing");
    assert!(doc.is_dirty(), "unsaved bytes stay dirty");
}
