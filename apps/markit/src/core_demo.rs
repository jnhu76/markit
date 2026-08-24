//! markit-core seam demo (P0-01 product-workspace smoke; headless
//! diagnostic, kept alive via `markit --core-demo`).
//!
//! Exercises the exact seams the GPUI editor slice uses: transaction
//! mutation, mutation-time change propagation, revision-gated derived work.
//! The real window/editor path is `editor_slice.rs` (P0-03).

use markit_core::{
    ByteOffset, Document, EditTransaction, LineNumber, Revisioned, Selection, TextEdit,
};

pub fn run() {
    println!("Markit product app (core seam demo; editor UI awaits P0-03)");

    let mut doc = Document::new("# Markit\n中文编辑器 🙂\n");

    // A derived view (here: line count) is tagged with its document
    // version: identity + revision, never a bare revision number.
    let mut derived = Revisioned::new(doc.version(), doc.line_count());
    println!(
        "loaded document {:?} at revision {} — {} lines, {} bytes",
        doc.id(),
        doc.revision().as_u64(),
        doc.line_count(),
        doc.len_bytes(),
    );

    // Typing goes through the transaction seam.
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(2), "产品 "))
        .apply(&mut doc)
        .expect("skeleton edit is valid");
    let edit = &applied.result.edits[0];
    println!(
        "typed edit: kind {:?}, old [{}, {}), new [{}, {}), byte_delta {}, scanned {} bytes, {} line entries touched",
        applied.result.kind,
        edit.old_range.start.as_usize(),
        edit.old_range.end.as_usize(),
        edit.new_range.start.as_usize(),
        edit.new_range.end.as_usize(),
        edit.byte_delta,
        applied.result.work.bytes_scanned,
        applied.result.work.line_entries_touched,
    );

    // The old derived result is stale and must be rejected, not applied.
    match derived.commit(doc.version()) {
        Ok(_) => unreachable!("skeleton: stale result must not commit"),
        Err(stale) => {
            println!(
                "stale derived result rejected: base revision {} < current {}",
                stale.base_version.revision().as_u64(),
                stale.current_version.revision().as_u64(),
            );
        }
    }
    derived = Revisioned::new(doc.version(), doc.line_count());
    let lines = derived
        .commit(doc.version())
        .expect("current version commits");
    println!(
        "derived line count committed at revision {}: {lines}",
        doc.revision().as_u64()
    );

    // Selection transform over the same edit coordinates.
    let caret = Selection::caret(ByteOffset(20)).map_over_edit(edit.old_range, "产品 ");
    println!("caret mapped to byte {}", caret.caret_offset().as_usize());
    println!("line 0 is now: {:?}", doc.line_str(LineNumber(0)));

    // The undo seam restores the text as a new revision.
    applied.inverse.apply(&mut doc).expect("inverse applies");
    println!(
        "after inverse: revision {}, line 0 = {:?}",
        doc.revision().as_u64(),
        doc.line_str(LineNumber(0)),
    );

    println!("core seam OK");
}
