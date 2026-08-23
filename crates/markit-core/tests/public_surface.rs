//! Integration test: pins the intended public surface of markit-core.
//!
//! Everything used here is the public API an editor layer (and, later,
//! a versioned plugin adapter) is meant to consume: no storage access,
//! no internal indexes, no GPUI types, no scheduler machinery. If an
//! internal refactor breaks this file, the public seam moved — that
//! must be a deliberate decision, not an accident.

use std::borrow::Cow;

use markit_core::{
    ByteOffset, ChangeKind, Document, DocumentId, DocumentSnapshot, DocumentVersion, EditIntent,
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
    let initial: DocumentVersion = doc.version();
    assert_eq!(initial.revision().as_u64(), 0);
    assert_eq!(doc.line_count(), 3);
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# Markit");
    assert_eq!(doc.line_str(LineNumber(1)).as_ref(), "中文段落 🙂");

    // Coherent read view.
    let snapshot: DocumentSnapshot<'_> = doc.snapshot();
    assert_eq!(snapshot.id(), id);
    assert_eq!(snapshot.version(), initial);
    let first_line: Cow<'_, str> = snapshot.line_str(LineNumber(0));
    assert_eq!(first_line.as_ref(), "# Markit");

    // Mutation through the transaction seam, with mutation-time change
    // propagation. Canonical regions are per edit.
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(2), "产品 "))
        .apply(&mut doc)
        .expect("valid edit applies");
    assert_eq!(applied.result.kind, ChangeKind::Insert);
    assert_eq!(applied.result.byte_delta, 7);
    assert_eq!(applied.result.work.bytes_scanned, 7);
    assert_eq!(applied.result.work.full_rebuilds, 0);
    assert_eq!(applied.result.edits.len(), 1);
    assert_eq!(
        applied.result.edits[0].old_range,
        range(2, 2),
        "single edit: canonical per-edit region"
    );
    assert!(doc.revision() > initial.revision());
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# 产品 Markit");

    // Selection transform uses the same edit coordinates.
    let selection = Selection::caret(ByteOffset(20));
    let mapped = selection.map_over_edit(applied.result.edits[0].old_range, "产品 ");
    assert_eq!(mapped.caret_offset(), ByteOffset(27));

    // Derived work is version-gated: results from the old version are
    // rejected once the document moved on.
    let derived = Revisioned::new(initial, doc.line_count());
    assert!(derived.commit(doc.version()).is_err());
    let derived = Revisioned::new(doc.version(), doc.line_count());
    assert_eq!(derived.commit(doc.version()), Ok(3));

    // Undo seam: the inverse restores the text as a NEW revision.
    let rev_before_undo = doc.revision();
    applied.inverse.apply(&mut doc).expect("inverse applies");
    assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# Markit");
    assert!(doc.revision() > rev_before_undo);

    // Whole-document replacement stays a distinct semantic path.
    let result = doc.replace_all("reset");
    assert_eq!(result.kind, ChangeKind::ReplaceDocument);
    assert_eq!(result.work.full_rebuilds, 1);
    assert_eq!(result.edits.len(), 1);
    assert_eq!(doc.slice(range(0, 5)).as_ref(), "reset");
}

#[test]
fn version_identity_binds_document_and_revision() {
    // A revision number alone is not a version: equal numeric revisions
    // from different documents are unrelated states and must not
    // cross-commit.
    let a = Document::new("alpha");
    let b = Document::new("beta");
    assert_ne!(a.id(), b.id());
    assert_eq!(a.revision(), b.revision());

    let same_document_same_revision = Revisioned::new(a.version(), 1);
    assert_eq!(same_document_same_revision.commit(a.version()), Ok(1));

    let from_a = Revisioned::new(a.version(), a.line_count());
    assert!(
        from_a.commit(b.version()).is_err(),
        "different document, same revision number: must reject"
    );

    // Newer revision of the same document: reject.
    let mut a = a;
    markit_core::EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(5), "!"))
        .apply(&mut a)
        .unwrap();
    let stale = from_a.commit(a.version()).unwrap_err();
    assert_eq!(stale.base_version.document_id(), a.id());
    assert!(stale.current_version.revision() > stale.base_version.revision());
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

#[test]
fn markdown_surface_is_query_views_not_parser_internals() {
    use markit_core::markdown::{
        BlockDetail, BlockFingerprint, BlockKind, BlockParseState, FenceChar, FenceInfo, InlineIr,
        InlineNode, InlineRun, ListItem, ListSignature, MarkdownState, MarkdownStateError,
        MarkdownWork,
    };
    use markit_core::{BlockRecord, InternalBlockId};

    // The consumer workflow: build from a snapshot, query by position and
    // id, read inline IR, gate on version — with no access to parsers,
    // indexes, or lexical machinery.
    let mut doc = Document::new("# 标题\n\n段落 *强调* `code` [链接](u)\n");
    let snapshot = doc.snapshot();
    let mut state = MarkdownState::build(&snapshot);

    let version: markit_core::DocumentVersion = state.version();
    assert_eq!(version, snapshot.version());
    let count = state.block_count();
    assert!(count >= 2);
    let blocks: &[BlockRecord] = state.blocks();
    let _work: MarkdownWork = state.last_work();
    let _cumulative: MarkdownWork = state.cumulative_work();

    let heading = state.block_at_offset(ByteOffset(0)).expect("heading at 0");
    assert_eq!(heading.kind, BlockKind::Heading);
    let BlockDetail::Heading { level, content } = &heading.detail else {
        panic!("heading detail");
    };
    assert_eq!(*level, 1);
    assert!(!content.is_empty());

    let mid = ByteOffset(doc.len_bytes() / 2);
    let _slice: &[BlockRecord] = state.blocks_in_range(range(0, mid.as_usize()));
    let _lines: &[BlockRecord] = state.blocks_in_lines(LineNumber(0)..LineNumber(2));

    let id: InternalBlockId = blocks[0].id;
    let _again: Option<&BlockRecord> = state.block_by_id(id);
    let ir: Option<&InlineIr> = state.inline_ir(id);
    let heading_ir = ir.expect("headings have inline IR");
    assert_eq!(heading_ir.runs.len(), 1);
    let _run: &InlineRun = &heading_ir.runs[0];
    let paragraph = blocks
        .iter()
        .find(|b| b.kind == BlockKind::Paragraph)
        .unwrap();
    let InlineIr { runs }: &InlineIr = &paragraph.inline;
    assert!(!runs.is_empty());
    let InlineRun { range: _, nodes } = &runs[0];
    assert!(nodes
        .iter()
        .any(|n| matches!(n, InlineNode::Emphasis { .. }) || matches!(n, InlineNode::Text { .. })));

    let _fingerprint: BlockFingerprint = paragraph.fingerprint;
    let _state_after: BlockParseState = paragraph.state_after;
    let BlockDetail::Paragraph = paragraph.detail else {
        panic!("paragraph detail");
    };
    let _ = (
        FenceChar::Backtick.as_char(),
        ListSignature::Bullet { marker: '-' },
        MarkdownStateError::StaleBase.to_string(),
    );
    let _item = ListItem {
        marker_range: range(0, 1),
        content: range(0, 1),
        number: Some(1),
    };
    let _fence = FenceInfo {
        fence_char: FenceChar::Tilde,
        fence_len: 3,
        info: None,
    };

    // The update seam gates on the whole version triple. Borrowing
    // rules force the same discipline the runtime enforces: a snapshot
    // cannot outlive an edit. The pre-edit view is reconstructed as a
    // new revision (undo), which the update must reject because its
    // revision is not the result's new revision.
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(2), "X"))
        .apply(&mut doc)
        .expect("applies");
    applied.inverse.apply(&mut doc).expect("undo applies");
    let reverted = doc.snapshot();
    assert_eq!(
        state.update(&reverted, &applied.result),
        Err(MarkdownStateError::SnapshotNotAtNewRevision)
    );
    // Skipped revisions cannot be absorbed either: rebuild at the
    // reverted version and take one coherent step from there.
    let mut rebuilt = MarkdownState::build(&reverted);
    let redo = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(2), "X"))
        .apply(&mut doc)
        .expect("redo applies");
    rebuilt
        .update(&doc.snapshot(), &redo.result)
        .expect("one coherent step from the matching base");
}
