//! Coherent read view of a document — the plugin-boundary seam shape.
//!
//! A "snapshot" is a **coherent version boundary**, not a copy
//! (`docs/product/architecture.md` §7). This P0-01 implementation
//! borrows the document: while the snapshot exists, the borrow checker
//! prevents mutation, so `id + revision + content` are coherent by
//! construction and the snapshot costs O(1).
//!
//! The borrowing form is deliberately minimal:
//!
//! - when deferred/background consumers arrive (P1), snapshots that must
//!   outlive edits will need versioned or copied storage — that is a
//!   real-workload decision to make then;
//! - this type is **not** the future plugin ABI. Plugins consume a
//!   versioned adapter view
//!   (`docs/product/plugin-compatibility-contract.md` §7); the point of
//!   the seam is that they never see `&mut Document`, internal indexes,
//!   or storage pointers, and the adapter will be built against exactly
//!   these read-only semantics.

use std::borrow::Cow;
use std::ops::Range;

use crate::document::Document;
use crate::id::DocumentId;
use crate::position::{ByteOffset, LineNumber, SourceRange};
use crate::revision::DocumentRevision;

/// O(1) coherent read view of a [`Document`].
///
/// Coherence: the snapshot borrows the document immutably; no edit can
/// intervene while it is alive, so the recorded revision always
/// describes the visible content.
#[derive(Clone, Copy, Debug)]
pub struct DocumentSnapshot<'a> {
    document: &'a Document,
}

impl<'a> DocumentSnapshot<'a> {
    pub(crate) fn new(document: &'a Document) -> Self {
        Self { document }
    }

    /// Identity of the snapshotted document.
    pub fn id(&self) -> DocumentId {
        self.document.id()
    }

    /// Revision of the snapshotted state.
    pub fn revision(&self) -> DocumentRevision {
        self.document.revision()
    }

    /// Total byte length of the snapshotted text.
    pub fn len_bytes(&self) -> usize {
        self.document.len_bytes()
    }

    /// Whether the snapshotted text is empty.
    pub fn is_empty(&self) -> bool {
        self.document.is_empty()
    }

    /// Number of lines (always >= 1).
    pub fn line_count(&self) -> usize {
        self.document.line_count()
    }

    /// The line containing `offset` in the snapshotted state.
    pub fn line_of(&self, offset: ByteOffset) -> LineNumber {
        self.document.line_of(offset)
    }

    /// Content byte range of `line` (excluding the `'\n'` terminator).
    pub fn line_range(&self, line: LineNumber) -> SourceRange {
        self.document.line_range(line)
    }

    /// Content of `line` (excluding the `'\n'` terminator).
    pub fn line_str(&self, line: LineNumber) -> Cow<'a, str> {
        self.document.line_str(line)
    }

    /// Line span covering the byte `range`.
    pub fn line_span(&self, range: SourceRange) -> Range<LineNumber> {
        self.document.line_span(range)
    }

    /// Reads a byte range. Same contract as [`Document::slice`].
    pub fn slice(&self, range: SourceRange) -> Cow<'a, str> {
        self.document.slice(range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::TextEdit;

    #[test]
    fn snapshot_is_coherent_and_cheap() {
        let mut doc = Document::new("alpha\nbeta");
        let snapshot = doc.snapshot();
        assert_eq!(snapshot.revision().as_u64(), 0);
        assert_eq!(snapshot.line_count(), 2);
        assert_eq!(
            snapshot
                .slice(SourceRange::new(ByteOffset(0), ByteOffset(5)))
                .as_ref(),
            "alpha"
        );
        assert_eq!(snapshot.line_str(LineNumber(1)).as_ref(), "beta");
        assert_eq!(snapshot.id(), doc.id());

        // The snapshot's revision/content describe the borrowed state.
        // (The borrow ends at the last use above; the document is only
        // mutable again afterwards.)
        let rev = snapshot.revision();

        doc.apply_edit(TextEdit::insert(ByteOffset(0), "# "))
            .unwrap();
        assert!(doc.revision() > rev);
        let after = doc.snapshot();
        assert_eq!(after.line_str(LineNumber(0)).as_ref(), "# alpha");
        assert!(after.revision() > rev);
    }

    #[test]
    fn snapshot_line_queries_match_document() {
        let doc = Document::new("l0\nl1\nl2");
        let snapshot = doc.snapshot();
        for (line, expected) in ["l0", "l1", "l2"].iter().enumerate() {
            assert_eq!(snapshot.line_str(LineNumber(line)).as_ref(), *expected);
        }
        assert_eq!(snapshot.len_bytes(), 8);
        assert!(!snapshot.is_empty());
        let span = snapshot.line_span(SourceRange::new(ByteOffset(1), ByteOffset(7)));
        assert_eq!((span.start, span.end), (LineNumber(0), LineNumber(3)));
    }
}
