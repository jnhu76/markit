//! Markit product application — skeleton.
//!
//! G0 (GPUI baseline selection) is not complete, so this binary has no
//! UI substrate yet and must not invent one: the window/editor shell
//! lands with the pinned GPUI baseline (roadmap G0 → P1). What exists
//! here proves the product app crate consumes `markit-core` through the
//! product workspace and exercises the exact seams the editor shell
//! will use (transaction mutation, mutation-time change propagation,
//! revision-gated derived work).

use markit_core::{
    ByteOffset, Document, EditTransaction, LineNumber, Revisioned, Selection, TextEdit,
};

fn main() {
    println!("Markit product app skeleton (editor UI awaits the G0 GPUI baseline)");

    let mut doc = Document::new("# Markit\n中文编辑器 🙂\n");

    // A derived view (here: line count) is tagged with its revision.
    let mut derived = Revisioned::new(doc.revision(), doc.line_count());
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
    println!(
        "typed edit: kind {:?}, old [{}, {}), new [{}, {}), byte_delta {}, scanned {} bytes, {} line entries touched",
        applied.result.kind,
        applied.result.old_range.start.as_usize(),
        applied.result.old_range.end.as_usize(),
        applied.result.new_range.start.as_usize(),
        applied.result.new_range.end.as_usize(),
        applied.result.byte_delta,
        applied.result.work.bytes_scanned,
        applied.result.work.line_entries_touched,
    );

    // The old derived result is stale and must be rejected, not applied.
    match derived.commit(doc.revision()) {
        Ok(_) => unreachable!("skeleton: stale result must not commit"),
        Err(stale) => {
            println!(
                "stale derived result rejected: base revision {} < current {}",
                stale.base_revision.as_u64(),
                stale.current_revision.as_u64(),
            );
        }
    }
    derived = Revisioned::new(doc.revision(), doc.line_count());
    let lines = derived
        .commit(doc.revision())
        .expect("current revision commits");
    println!(
        "derived line count committed at revision {}: {lines}",
        doc.revision().as_u64()
    );

    // Selection transform over the same edit coordinates.
    let caret = Selection::caret(ByteOffset(20)).map_over_edit(applied.result.old_range, "产品 ");
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
