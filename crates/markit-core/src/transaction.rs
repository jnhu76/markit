//! Edit transaction seam.
//!
//! [`EditTransaction`] is the single mutation entry point future input
//! paths — keyboard handlers, IME composition commit, paste, commands,
//! and (through the adapter) plugins — funnel through instead of
//! touching document storage. It is the natural boundary for undo/redo
//! grouping and IME commit grouping (ADR-006/ADR-007): the intent travels
//! with the edits, and applying a transaction returns its inverse.
//!
//! The undo **stack** (coalescing, memory bounds) is deliberately a later
//! phase; this module only guarantees that the data needed to invert a
//! transaction is produced correctly now.

use crate::change::{EditError, EditResult, TextEdit};
use crate::document::Document;

/// Why a transaction exists — the metadata future undo grouping and IME
/// semantics key off. Carried alongside the edits, not smeared into the
/// change classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditIntent {
    /// Ordinary typing or deletion (coalescing candidate).
    Typing,
    /// Clipboard paste (one undo step, never coalesced with typing).
    Paste,
    /// IME composition commit — exactly one transaction per commit
    /// (ADR-007; composition itself never enters undo as keystrokes).
    ImeCommit,
    /// Explicit editor command.
    Command,
}

/// An atomic group of edits plus the intent that caused them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransaction {
    intent: EditIntent,
    edits: Vec<TextEdit>,
}

impl EditTransaction {
    /// An empty transaction with the given intent. Push edits before
    /// applying; an all-no-op transaction is rejected at apply time.
    pub fn new(intent: EditIntent) -> Self {
        Self {
            intent,
            edits: Vec::new(),
        }
    }

    /// A [`EditIntent::Typing`] transaction.
    pub fn typing() -> Self {
        Self::new(EditIntent::Typing)
    }

    /// A [`EditIntent::Paste`] transaction.
    pub fn paste() -> Self {
        Self::new(EditIntent::Paste)
    }

    /// A [`EditIntent::ImeCommit`] transaction.
    pub fn ime_commit() -> Self {
        Self::new(EditIntent::ImeCommit)
    }

    /// A [`EditIntent::Command`] transaction.
    pub fn command() -> Self {
        Self::new(EditIntent::Command)
    }

    /// Adds an edit (builder style).
    pub fn push(&mut self, edit: TextEdit) -> &mut Self {
        self.edits.push(edit);
        self
    }

    /// Adds an edit (consuming builder style).
    pub fn with_edit(mut self, edit: TextEdit) -> Self {
        self.edits.push(edit);
        self
    }

    /// The intent that caused this transaction.
    pub fn intent(&self) -> EditIntent {
        self.intent
    }

    /// The edits, in the order they were added.
    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    /// Consumes the transaction into its edits (document-side seam).
    pub(crate) fn into_edits(self) -> Vec<TextEdit> {
        self.edits
    }

    /// Applies atomically to `document`:
    ///
    /// - every edit is validated before any mutation (all-or-nothing);
    /// - the revision advances exactly once;
    /// - the returned [`AppliedTransaction`] carries both the edit
    ///   result and the inverse transaction (undo seam).
    pub fn apply(self, document: &mut Document) -> Result<AppliedTransaction, EditError> {
        let intent = self.intent;
        let (result, inverse_edits) = document.apply_transaction_with_inverse(self)?;
        Ok(AppliedTransaction {
            result,
            inverse: EditTransaction {
                intent,
                edits: inverse_edits,
            },
        })
    }
}

/// The outcome of applying a transaction: the forward result plus the
/// ready-to-apply inverse.
#[derive(Clone, Debug)]
pub struct AppliedTransaction {
    /// The forward [`EditResult`] (changed ranges, revision, work).
    pub result: EditResult,
    /// The transaction that undoes [`Self::result`] when applied next.
    pub inverse: EditTransaction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{ByteOffset, SourceRange};

    fn offset(n: usize) -> ByteOffset {
        ByteOffset(n)
    }

    fn text_of(doc: &Document) -> String {
        doc.slice(SourceRange::new(ByteOffset(0), ByteOffset(doc.len_bytes())))
            .into_owned()
    }

    #[test]
    fn single_edit_transaction_bumps_revision_once() {
        let mut doc = Document::new("hello");
        let applied = EditTransaction::typing()
            .with_edit(TextEdit::insert(offset(5), " world"))
            .apply(&mut doc)
            .unwrap();
        assert_eq!(text_of(&doc), "hello world");
        assert_eq!(applied.result.base_revision.as_u64(), 0);
        assert_eq!(applied.result.new_revision.as_u64(), 1);
        assert_eq!(applied.result.kind, crate::ChangeKind::Append);
        assert_eq!(applied.inverse.intent(), EditIntent::Typing);
    }

    #[test]
    fn multi_edit_transaction_is_atomic_and_exact() {
        let mut doc = Document::new("one two three");
        let applied = EditTransaction::command()
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(0), offset(3)),
                "1",
            ))
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(4), offset(7)),
                "2",
            ))
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(8), offset(13)),
                "3!",
            ))
            .apply(&mut doc)
            .unwrap();

        assert_eq!(text_of(&doc), "1 2 3!");
        assert_eq!(applied.result.new_revision.as_u64(), 1);
        assert_eq!(applied.result.edits.len(), 3);
        // Covering range spans first..last in old coordinates
        // (convenience-only; canonical regions are the per-edit entries).
        assert_eq!(
            applied.result.covering_old_range,
            SourceRange::new(offset(0), offset(13))
        );
        // Per-edit new ranges are in FINAL coordinates (3 inserts shift
        // later positions left/right accordingly: 1 at [0,1), 2 at [2,3),
        // 3! at [4,6)).
        assert_eq!(
            applied.result.edits[0].new_range,
            SourceRange::new(offset(0), offset(1))
        );
        assert_eq!(
            applied.result.edits[1].new_range,
            SourceRange::new(offset(2), offset(3))
        );
        assert_eq!(
            applied.result.edits[2].new_range,
            SourceRange::new(offset(4), offset(6))
        );
    }

    #[test]
    fn overlapping_edits_reject_without_mutation() {
        let mut doc = Document::new("abcdef");
        let rev = doc.revision();
        let err = EditTransaction::command()
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(1), offset(4)),
                "X",
            ))
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(3), offset(5)),
                "Y",
            ))
            .apply(&mut doc)
            .unwrap_err();
        assert!(matches!(err, EditError::OverlappingEdits { .. }));
        assert_eq!(text_of(&doc), "abcdef");
        assert_eq!(doc.revision(), rev);
    }

    #[test]
    fn invalid_edit_rejects_whole_transaction() {
        let mut doc = Document::new("abcdef");
        let rev = doc.revision();
        let err = EditTransaction::paste()
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(0), offset(2)),
                "X",
            ))
            .with_edit(TextEdit::insert(offset(99), "Y")) // out of bounds
            .apply(&mut doc)
            .unwrap_err();
        assert!(matches!(err, EditError::InvalidRange { .. }));
        assert_eq!(text_of(&doc), "abcdef", "no partial application");
        assert_eq!(doc.revision(), rev);
    }

    #[test]
    fn empty_transaction_is_rejected() {
        let mut doc = Document::new("abc");
        let err = EditTransaction::typing().apply(&mut doc).unwrap_err();
        assert_eq!(err, EditError::EmptyTransaction);
        let err = EditTransaction::typing()
            .with_edit(TextEdit::insert(offset(1), ""))
            .apply(&mut doc)
            .unwrap_err();
        assert_eq!(err, EditError::EmptyTransaction);
    }

    #[test]
    fn inverse_restores_the_document() {
        let original = "first\nsecond\nthird";
        let mut doc = Document::new(original);
        let applied = EditTransaction::ime_commit()
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(6), offset(12)),
                "第二个",
            ))
            .apply(&mut doc)
            .unwrap();
        let rev_after_forward = doc.revision();
        assert_eq!(text_of(&doc), "first\n第二个\nthird");

        applied
            .inverse
            .apply(&mut doc)
            .expect("inverse applies cleanly");
        assert_eq!(text_of(&doc), original);
        assert!(
            doc.revision() > rev_after_forward,
            "undo is a new revision, not a rollback"
        );
    }

    #[test]
    fn multi_edit_inverse_restores_the_document() {
        let original = "one two three four";
        let mut doc = Document::new(original);
        let applied = EditTransaction::command()
            .with_edit(TextEdit::delete(SourceRange::new(offset(0), offset(4))))
            .with_edit(TextEdit::insert(offset(9), "3 "))
            .with_edit(TextEdit::replace(
                SourceRange::new(offset(14), offset(18)),
                "4",
            ))
            .apply(&mut doc)
            .unwrap();
        assert_ne!(&text_of(&doc), original);

        applied.inverse.apply(&mut doc).unwrap();
        assert_eq!(text_of(&doc), original);
    }

    #[test]
    fn intent_travels_with_the_transaction() {
        let mut doc = Document::new("x");
        let tx = EditTransaction::ime_commit().with_edit(TextEdit::insert(offset(1), "y"));
        assert_eq!(tx.intent(), EditIntent::ImeCommit);
        let applied = tx.apply(&mut doc).unwrap();
        assert_eq!(applied.inverse.intent(), EditIntent::ImeCommit);
    }

    #[test]
    fn ime_commit_grouping_smoke() {
        // ADR-007 shape: composition replaces a marked range and moves
        // the caret — ONE transaction, one revision bump, invertible.
        // (ASCII stand-in keeps the arithmetic obvious.)
        let mut doc = Document::new("hello, world");
        let marked = SourceRange::new(offset(5), offset(12));
        let applied = EditTransaction::ime_commit()
            .with_edit(TextEdit::replace(marked, "，世界"))
            .apply(&mut doc)
            .unwrap();
        assert_eq!(applied.result.new_revision.as_u64(), 1);
        applied.inverse.apply(&mut doc).unwrap();
        assert_eq!(text_of(&doc), "hello, world");
    }
}
