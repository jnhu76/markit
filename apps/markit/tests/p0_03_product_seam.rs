//! P0-03 headless product-seam regression (issue #16).
//!
//! Pins the edit→Markdown half of the vertical slice, GPUI-free — exactly
//! the sequence `apps/markit/src/editor_slice.rs` performs per keystroke:
//!
//! ```text
//! initial Document + MarkdownState
//!   -> EditTransaction
//!   -> MarkdownState::update(snapshot, &EditResult)
//!   -> matching DocumentVersion
//!   -> expected Markdown semantic result via the BlockView query surface
//! ```
//!
//! Locality assertions are for THIS fixture's ordinary local edit, per
//! issue #12 R6: "one block reparsed" is expected evidence, never a
//! universal invariant (structural edits honestly propagate further).

use markit_core::markdown::{BlockDetail, BlockKind, MarkdownState, MarkdownStateError};
use markit_core::InlineNode;
use markit_core::{
    ByteOffset, ChangeKind, Document, EditTransaction, LineNumber, SourceRange, TextEdit,
};

/// The P0-03 product fixture document (see editor_slice.rs).
const DOC: &str = "# Markit\n\nEdit **me**.\n";

fn setup() -> (Document, MarkdownState) {
    let doc = Document::new(DOC);
    let state = MarkdownState::build(&doc.snapshot());
    assert_eq!(
        state.version(),
        doc.version(),
        "initial build must describe the loaded document version"
    );
    (doc, state)
}

/// The full per-keystroke seam: apply, snapshot, update, verify versions.
fn keystroke(
    doc: &mut Document,
    state: &mut MarkdownState,
    at: usize,
    text: &str,
) -> markit_core::EditResult {
    let base = doc.version();
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(at), text))
        .apply(doc)
        .expect("fixture edit is valid");
    let result = applied.result;

    // One keystroke = exactly one revision step, identity included.
    assert_eq!(result.base_version, base);
    assert_eq!(
        result.new_version.revision().as_u64(),
        base.revision().as_u64() + 1,
        "revision advances exactly once per transaction"
    );
    assert_eq!(
        result.new_version.document_id(),
        base.document_id(),
        "same document identity across the edit"
    );

    // The exact canonical EditResult regions drive the Markdown update.
    assert_eq!(result.edits.len(), 1, "one edit -> one canonical region");

    let snapshot = doc.snapshot();
    assert_eq!(
        snapshot.version(),
        result.new_version,
        "post-edit snapshot is at the edit's new version"
    );
    state
        .update(&snapshot, &result)
        .expect("coherent triple: state@N + result N->N+1 + snapshot@N+1");

    // DocumentVersion and MarkdownState::version() stay equal.
    assert_eq!(state.version(), doc.version());
    assert_eq!(state.version(), result.new_version);
    result
}

#[test]
fn paragraph_keystroke_is_local_and_semantically_visible() {
    let (mut doc, mut state) = setup();

    // Fixture layout: heading line 0, blank line 1, paragraph line 2.
    let edit_at = DOC.find("Edit").expect("fixture contains Edit") + "Edit".len();
    let result = keystroke(&mut doc, &mut state, edit_at, "x");

    // P0-01 counters: an ordinary local insert never rebuilds, scans only
    // its own bytes, and touches one changed line.
    assert_eq!(result.work.full_rebuilds, 0);
    assert_eq!(result.work.bytes_scanned, 1);
    assert_eq!(result.work.changed_lines, 1);
    assert_eq!(result.edits[0].new_line_span, LineNumber(2)..LineNumber(3));

    // P0-02 counters, fixture locality (NOT a universal law): the island is
    // the paragraph plus the blank-run boundary neighbor above it.
    let work = state.last_work();
    assert_eq!(work.dirty_regions, 1);
    assert_eq!(work.blocks_reparsed, 2, "blank neighbor + the paragraph");
    assert_eq!(work.lines_scanned, 2, "blank line + paragraph line");
    assert!(work.convergence_line < doc.line_count() as u64);

    // Semantic result through the same query surface the projection uses:
    // the heading kept its level, the paragraph grew, strong survived.
    let snapshot = doc.snapshot();
    let heading = state
        .blocks_in_lines(LineNumber(0)..LineNumber(1))
        .next()
        .expect("heading on line 0");
    assert_eq!(heading.kind(), BlockKind::Heading);
    match heading.detail() {
        BlockDetail::Heading { level, content } => {
            assert_eq!(*level, 1);
            assert_eq!(snapshot.slice(*content).as_ref(), "Markit");
        }
        other => panic!("heading detail, got {other:?}"),
    }

    let paragraph = state
        .blocks_in_lines(LineNumber(2)..LineNumber(3))
        .next()
        .expect("paragraph on line 2");
    assert_eq!(paragraph.kind(), BlockKind::Paragraph);
    let text = snapshot.slice(paragraph.source_range());
    assert_eq!(text.trim_end(), "Editx **me**.");
    assert!(
        paragraph.inline().runs.iter().any(|run| run
            .nodes
            .iter()
            .any(|node| matches!(node, InlineNode::Strong { .. }))),
        "strong emphasis survives the neighboring insert"
    );
}

#[test]
fn eof_keystrokes_stay_in_lockstep() {
    // The Windows smoke path: the caret starts at EOF on the final empty
    // line, and each keystroke appends there. The first one is a structural
    // change (trailing blank becomes a paragraph) and must stay honest AND
    // in lockstep — version equality is the law, not the block count.
    let (mut doc, mut state) = setup();
    let snapshot = doc.snapshot();
    assert_eq!(snapshot.line_count(), 4, "heading/blank/paragraph/final");

    let at_eof = DOC.len();
    let first = keystroke(&mut doc, &mut state, at_eof, "X");
    assert_eq!(first.kind, ChangeKind::Append);
    // The work stays local to the final line's island (identity pairing
    // internals — which old id the paragraph keeps — are core's business).
    assert_eq!(state.last_work().dirty_regions, 1);
    assert_eq!(state.last_work().blocks_reparsed, 1);

    // The second append grows the same paragraph.
    let second = keystroke(&mut doc, &mut state, at_eof + 1, "Y");
    assert_eq!(second.kind, ChangeKind::Append);
    assert_eq!(
        state.last_work().blocks_reparsed,
        1,
        "the grown paragraph only (fixture evidence)"
    );
    assert_eq!(state.version(), doc.version());

    let snapshot = doc.snapshot();
    let last_line = snapshot.line_count() - 1;
    let block = state
        .blocks_in_lines(LineNumber(last_line)..LineNumber(last_line + 1))
        .next()
        .expect("block on the final line");
    // Markdown semantics: the typed lines join the paragraph above (no
    // blank line separates them) — one paragraph, soft line break inside.
    assert_eq!(block.kind(), BlockKind::Paragraph);
    assert_eq!(
        snapshot.slice(block.source_range()).trim_end(),
        "Edit **me**.\nXY"
    );

    // The undo seam inverts a keystroke as a new revision, still lockstep.
    let inverse = EditTransaction::typing()
        .with_edit(TextEdit::delete(SourceRange::new(
            ByteOffset(at_eof),
            ByteOffset(at_eof + 2),
        )))
        .apply(&mut doc)
        .expect("inverse edit applies");
    state
        .update(&doc.snapshot(), &inverse.result)
        .expect("state follows the inverse");
    assert_eq!(state.version(), doc.version());
    assert_eq!(doc.version().revision().as_u64(), 3);
}

#[test]
fn version_mismatch_fails_closed_at_the_product_seam() {
    // The app must not be able to hide a mismatch with a rebuild-shaped
    // update: a result that does not describe exactly (state@N -> N+1,
    // snapshot@N+1) is rejected and the state stays where it was.
    let (mut doc, mut state) = setup();
    let first = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(0), "x"))
        .apply(&mut doc)
        .unwrap();
    let second = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(1), "y"))
        .apply(&mut doc)
        .unwrap();
    // State is still at revision 0 while the document is at revision 2.

    // A result whose new revision is not the snapshot's revision.
    assert_eq!(
        state.update(&doc.snapshot(), &first.result),
        Err(MarkdownStateError::SnapshotNotAtNewRevision)
    );
    // A result whose base revision is not the state's revision.
    assert_eq!(
        state.update(&doc.snapshot(), &second.result),
        Err(MarkdownStateError::StaleBase)
    );
    // Every rejection left the state exactly where it was.
    assert_eq!(state.version().revision().as_u64(), 0);
    assert_eq!(state.version().document_id(), doc.id());
    assert_eq!(doc.version().revision().as_u64(), 2);

    // `MarkdownState::build` stays valid ONLY for explicit ownership
    // moments — initial load, and a future explicit reset/reload action —
    // never as automatic recovery inside the transaction hot path
    // (`apply_transaction` keeps Document@N+1 + stale MarkdownState@N
    // and renders INCOHERENT; it never rebuilds). This build only proves
    // the document itself is still fully recoverable:
    let rebuilt = MarkdownState::build(&doc.snapshot());
    assert_eq!(rebuilt.version(), doc.version());
    assert_eq!(rebuilt.block_count(), state.block_count());
    let _ = &mut state;
}
