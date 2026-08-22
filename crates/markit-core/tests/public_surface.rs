//! Integration test: pins the intended public surface of markit-core.
//!
//! Everything used here is the public API an editor layer (and, later,
//! a versioned plugin adapter) is meant to consume: no storage access,
//! no internal indexes, no GPUI types, no scheduler machinery. If an
//! internal refactor breaks this file, the public seam moved — that
//! must be a deliberate decision, not an accident.

use std::borrow::Cow;

use markit_core::{
    ByteOffset, ChangeKind, Document, DocumentId, DocumentRevision, DocumentSnapshot, EditIntent,
    EditTransaction, LineNumber, Revisioned, Selection, SourceRange, TextEdit,
};

fn range(start: usize, end: usize) -> SourceRange {
    SourceRange::new(ByteOffset(start), ByteOffset(end))
}

#[test]
fn end_to_end_public_flow() {
    // Load: one full scan, stable identity, initial revision.
    let mut doc = Document::new("# Markit\n中文段落 🙂\n");
    let id: DocumentId = doc.id();
    let initial: DocumentRevision = doc.revision();
    assert_eq!(initial.as_u64(), 0);
    assert_eq!(doc.line_count(), 3);
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# Markit");
    assert_eq!(doc.line_str(LineNumber(1)).as_ref(), "中文段落 🙂");

    // Coherent read view.
    let snapshot: DocumentSnapshot<'_> = doc.snapshot();
    assert_eq!(snapshot.id(), id);
    assert_eq!(snapshot.revision(), initial);
    let first_line: Cow<'_, str> = snapshot.line_str(LineNumber(0));
    assert_eq!(first_line.as_ref(), "# Markit");

    // Mutation through the transaction seam, with mutation-time change
    // propagation.
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(2), "产品 "))
        .apply(&mut doc)
        .expect("valid edit applies");
    assert_eq!(applied.result.kind, ChangeKind::Insert);
    assert_eq!(applied.result.byte_delta, 7);
    assert_eq!(applied.result.work.bytes_scanned, 7);
    assert_eq!(applied.result.work.full_rebuilds, 0);
    assert!(doc.revision() > initial);
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# 产品 Markit");

    // Selection transform uses the same edit coordinates.
    let selection = Selection::caret(ByteOffset(20));
    let mapped = selection.map_over_edit(applied.result.old_range, "产品 ");
    assert_eq!(mapped.caret_offset(), ByteOffset(27));

    // Derived work is revision-gated: results from the old revision are
    // rejected once the document moved on.
    let derived = Revisioned::new(initial, doc.line_count());
    assert!(derived.commit(doc.revision()).is_err());
    let derived = Revisioned::new(doc.revision(), doc.line_count());
    assert_eq!(derived.commit(doc.revision()), Ok(3));

    // Undo seam: the inverse restores the text as a NEW revision.
    let rev_before_undo = doc.revision();
    applied.inverse.apply(&mut doc).expect("inverse applies");
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# Markit");
    assert!(doc.revision() > rev_before_undo);

    // Whole-document replacement stays a distinct semantic path.
    let result = doc.replace_all("reset");
    assert_eq!(result.kind, ChangeKind::ReplaceDocument);
    assert_eq!(result.work.full_rebuilds, 1);
    assert_eq!(doc.slice(range(0, 5)).as_ref(), "reset");
}

#[test]
fn intent_metadata_survives_the_boundary() {
    let mut doc = Document::new("text");
    let tx = EditTransaction::ime_commit().with_edit(TextEdit::insert(ByteOffset(4), "!"));
    assert_eq!(tx.intent(), EditIntent::ImeCommit);
    let applied = tx.apply(&mut doc).expect("applies");
    assert_eq!(applied.inverse.intent(), EditIntent::ImeCommit);
}

#[test]
fn invalid_input_cannot_corrupt_state() {
    let mut doc = Document::new("边界");
    let revision = doc.revision();
    // Splits the 3-byte 中 scalar.
    let rejected = TextEdit::insert(ByteOffset(1), "x");
    assert!(doc.apply_edit(rejected).is_err());
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.slice(range(0, 3)).as_ref(), "边");
    assert_eq!(doc.try_slice(range(1, 2)), None);
}
