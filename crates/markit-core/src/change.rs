//! Explicit edit/change vocabulary.
//!
//! A mutation never collapses into a vague "document changed" event:
//! every successful edit produces its changed ranges **at mutation time**
//! ([`EditResult`]), and downstream layers consume those ranges instead of
//! rescanning the document to guess what changed (ADR-003,
//! `docs/product/realtime-execution-model.md` §4).
//!
//! The **canonical** invalidation regions of a mutation are the per-edit
//! [`AppliedEdit`] entries. Sparsity must survive the change
//! representation: two distant one-line edits are two one-line regions —
//! never one region covering everything between them.

use std::fmt;
use std::ops::Range;

use crate::position::{ByteOffset, LineNumber, SourceRange};
use crate::revision::DocumentRevision;

/// Structural classification of an applied edit.
///
/// The classification is computed from the edit's shape at mutation time;
/// higher-level causes (paste, command) are carried by
/// [`EditIntent`](crate::EditIntent) on the transaction, not by smearing
/// more variants into this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// Text inserted at an empty range before the document end.
    Insert,
    /// Text inserted at the (pre-edit) document end.
    Append,
    /// A non-empty range removed.
    Delete,
    /// A non-empty range replaced by non-empty text.
    Replace,
    /// Whole-document replacement (load / reload / external rewrite).
    ReplaceDocument,
}

impl ChangeKind {
    /// Classifies an edit from its shape. Returns `None` for a no-op
    /// (empty range replaced by empty text), which callers reject.
    pub(crate) fn classify(
        range_len: usize,
        new_text_len: usize,
        at_document_end: bool,
    ) -> Option<Self> {
        match (range_len == 0, new_text_len == 0) {
            (true, true) => None,
            (true, false) => Some(if at_document_end {
                Self::Append
            } else {
                Self::Insert
            }),
            (false, true) => Some(Self::Delete),
            (false, false) => Some(Self::Replace),
        }
    }
}

/// A single requested mutation: replace `range` with `new_text`.
///
/// This is the command vocabulary every future input path (keyboard, IME,
/// paste, commands, plugins-through-the-adapter) funnels through; none of
/// them touch document storage directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Byte range to replace (empty range = pure insertion).
    pub range: SourceRange,
    /// Replacement text (empty = pure deletion).
    pub new_text: String,
}

impl TextEdit {
    /// Inserts `text` at `at`.
    pub fn insert(at: ByteOffset, text: impl Into<String>) -> Self {
        Self {
            range: SourceRange::new(at, at),
            new_text: text.into(),
        }
    }

    /// Deletes `range`.
    pub fn delete(range: SourceRange) -> Self {
        Self {
            range,
            new_text: String::new(),
        }
    }

    /// Replaces `range` with `text`.
    pub fn replace(range: SourceRange, text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: text.into(),
        }
    }

    /// Whether this edit would change nothing.
    pub fn is_noop(&self) -> bool {
        self.range.is_empty() && self.new_text.is_empty()
    }
}

/// One applied edit, with coordinates in both the pre-edit and the
/// post-edit document. **This is the canonical invalidation region** for
/// downstream consumers (future BlockIndex, view model): per edit, small,
/// and never merged across distant edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedEdit {
    /// Structural classification of this edit.
    pub kind: ChangeKind,
    /// Replaced range in the **pre-edit** document.
    pub old_range: SourceRange,
    /// Range of the inserted text in the **post-edit** document.
    pub new_range: SourceRange,
    /// `new_text.len() - old_range.len()` in bytes.
    pub byte_delta: i64,
    /// Lines covering [`AppliedEdit::old_range`] in the **pre-edit**
    /// document (line-granular dirty region for structures keyed by old
    /// coordinates; provided so consumers need not rescan or guess).
    pub old_line_span: Range<LineNumber>,
    /// Lines covering [`AppliedEdit::new_range`] in the **post-edit**
    /// document (line-granular dirty region for structures keyed by new
    /// coordinates).
    pub new_line_span: Range<LineNumber>,
}

/// Structural work counters for one mutation — the work-amplification
/// seam (INV-01/INV-08). Counters are structural, not wall-clock, so CI
/// can assert algorithmic behavior without timing flakes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditWork {
    /// Old bytes removed plus new bytes inserted (summed per edit).
    pub changed_bytes: u64,
    /// Post-edit lines covered by the changed regions: the **disjoint
    /// union of the per-edit new line spans**, so the measure stays
    /// proportional to what actually changed. It is never the span
    /// between the first and last edit — two distant one-line edits
    /// report 2, not the document size (INV-05/INV-08).
    pub changed_lines: u64,
    /// Document bytes actually scanned while updating the line index.
    /// A local edit scans only the bytes of its own new text (INV-01:
    /// full-document scans per edit must be 0).
    pub bytes_scanned: u64,
    /// Line-index entries removed + inserted + shifted.
    pub line_entries_touched: u64,
    /// Full line-index rebuilds performed (1 for load/ReplaceDocument, 0
    /// for normal edits).
    pub full_rebuilds: u64,
}

/// Result of a successful mutation.
///
/// Produced **by the mutation itself**; downstream consumers (future
/// BlockIndex, view model) read [`EditResult::edits`] — the canonical,
/// per-edit dirty regions — and never re-derive the changed region by
/// diffing or rescanning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditResult {
    /// Revision before the mutation.
    pub base_revision: DocumentRevision,
    /// Revision after the mutation (always `base_revision.next()`).
    pub new_revision: DocumentRevision,
    /// Classification of the covering change. Per-edit classifications
    /// live in [`EditResult::edits`].
    pub kind: ChangeKind,
    /// Smallest range covering all edits in the **pre-edit** document.
    ///
    /// **Convenience only.** For a multi-edit transaction with distant
    /// edits this spans everything between the first and last edit; using
    /// it as an invalidation region reintroduces O(document) work and is
    /// exactly the failure the per-edit regions exist to prevent.
    /// Canonical invalidation: [`EditResult::edits`].
    pub covering_old_range: SourceRange,
    /// Smallest range covering all edits in the **post-edit** document.
    ///
    /// **Convenience only** — same warning as
    /// [`EditResult::covering_old_range`].
    pub covering_new_range: SourceRange,
    /// Document byte-length change.
    pub byte_delta: i64,
    /// Document line-count change.
    pub line_delta: i64,
    /// Per-edit details, in ascending document order — the **canonical**
    /// changed regions for downstream invalidation.
    pub edits: Vec<AppliedEdit>,
    /// Structural work counters for this mutation.
    pub work: EditWork,
}

/// Why a mutation was rejected. Rejected mutations change nothing: no
/// revision bump, no partial application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// Range out of bounds or inverted.
    InvalidRange {
        /// The offending range start.
        start: ByteOffset,
        /// The offending range end.
        end: ByteOffset,
        /// Document byte length at validation time.
        document_len: usize,
    },
    /// Offset does not lie on a UTF-8 character boundary and the edit
    /// would split a multi-byte sequence.
    NotOnCharBoundary {
        /// The offending offset.
        offset: ByteOffset,
    },
    /// A transaction contains overlapping edits (ranges shown sorted by
    /// position).
    OverlappingEdits {
        /// First of the overlapping ranges.
        first: SourceRange,
        /// Second of the overlapping ranges.
        second: SourceRange,
    },
    /// A transaction contains no effective (non-no-op) edits.
    EmptyTransaction,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange {
                start,
                end,
                document_len,
            } => write!(
                f,
                "invalid source range [{}, {}) for document of {} bytes",
                start.as_usize(),
                end.as_usize(),
                document_len
            ),
            Self::NotOnCharBoundary { offset } => write!(
                f,
                "offset {} does not lie on a UTF-8 character boundary",
                offset.as_usize()
            ),
            Self::OverlappingEdits { first, second } => write!(
                f,
                "overlapping edits [{}, {}) and [{}, {})",
                first.start.as_usize(),
                first.end.as_usize(),
                second.start.as_usize(),
                second.end.as_usize()
            ),
            Self::EmptyTransaction => write!(f, "transaction has no effective edits"),
        }
    }
}

impl std::error::Error for EditError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_shapes() {
        use ChangeKind::*;
        assert_eq!(ChangeKind::classify(0, 3, false), Some(Insert));
        assert_eq!(ChangeKind::classify(0, 3, true), Some(Append));
        assert_eq!(ChangeKind::classify(2, 0, false), Some(Delete));
        assert_eq!(ChangeKind::classify(2, 4, false), Some(Replace));
        assert_eq!(ChangeKind::classify(0, 0, false), None);
    }

    #[test]
    fn edit_constructors() {
        let ins = TextEdit::insert(ByteOffset(3), "ab");
        assert!(ins.range.is_empty());
        assert_eq!(ins.new_text, "ab");
        assert!(!ins.is_noop());

        let del = TextEdit::delete(SourceRange::new(ByteOffset(1), ByteOffset(4)));
        assert!(del.new_text.is_empty());
        assert!(!del.is_noop());

        let noop = TextEdit::insert(ByteOffset(5), "");
        assert!(noop.is_noop());
    }

    #[test]
    fn error_display_is_actionable() {
        let e = EditError::InvalidRange {
            start: ByteOffset(9),
            end: ByteOffset(2),
            document_len: 5,
        };
        assert!(e.to_string().contains("[9, 2)"));
        assert!(e.to_string().contains("5 bytes"));
        let e = EditError::NotOnCharBoundary {
            offset: ByteOffset(1),
        };
        assert!(e.to_string().contains("character boundary"));
    }
}
