//! Canonical UTF-8 byte-range edit (R0 §6) and operation taxonomy.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::source::Source;

/// The R0 first-round operation set (`protocol/R0-METHODOLOGY.md` §8).
///
/// Serialized as lowercase snake_case; the same spelling is used in the
/// canonical [`crate::case::CaseKeyV1`] encoding, so renaming is a
/// case-identity break and must never happen silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    FullParse,
    Insert,
    Delete,
    ReplaceEq,
    ReplaceGrow,
    ReplaceShrink,
    StructuralEdit,
    Query,
}

impl OperationKind {
    /// Canonical spelling used in the versioned CaseKey byte encoding.
    pub fn canonical_name(self) -> &'static str {
        match self {
            OperationKind::FullParse => "full_parse",
            OperationKind::Insert => "insert",
            OperationKind::Delete => "delete",
            OperationKind::ReplaceEq => "replace_eq",
            OperationKind::ReplaceGrow => "replace_grow",
            OperationKind::ReplaceShrink => "replace_shrink",
            OperationKind::StructuralEdit => "structural_edit",
            OperationKind::Query => "query",
        }
    }

    /// Whether this operation carries an edit range at all.
    pub fn has_edit(self) -> bool {
        !matches!(self, OperationKind::FullParse | OperationKind::Query)
    }

    /// Whether this operation carries inserted text (possibly empty).
    pub fn has_inserted_text(self) -> bool {
        matches!(
            self,
            OperationKind::Insert
                | OperationKind::ReplaceEq
                | OperationKind::ReplaceGrow
                | OperationKind::ReplaceShrink
                | OperationKind::StructuralEdit
        )
    }

    /// Classify a pure byte-level edit. `StructuralEdit`, `Query` and
    /// `FullParse` are semantic choices made by the generator and cannot be
    /// inferred from bytes alone.
    pub fn classify(edit: &CanonicalEdit) -> OperationKind {
        let removed = edit.end_byte - edit.start_byte;
        let inserted = edit.inserted_text_len_bytes();
        match (removed, inserted) {
            (0, 0) => OperationKind::ReplaceEq,
            (0, _) => OperationKind::Insert,
            (_, 0) => OperationKind::Delete,
            (r, i) if r == i => OperationKind::ReplaceEq,
            (r, i) if i > r => OperationKind::ReplaceGrow,
            (_, _) => OperationKind::ReplaceShrink,
        }
    }
}

/// Errors produced by canonical-edit construction/validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// `start > end`.
    StartAfterEnd,
    /// Range end (or start) exceeds source length.
    OutOfRange,
    /// Range boundary is not a UTF-8 char boundary (Unicode/CJK/emoji safe).
    NotCharBoundary,
}

impl core::fmt::Display for EditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EditError::StartAfterEnd => write!(f, "edit start_byte > end_byte"),
            EditError::OutOfRange => write!(f, "edit range out of source bounds"),
            EditError::NotCharBoundary => {
                write!(f, "edit range boundary is not a UTF-8 char boundary")
            }
        }
    }
}

impl std::error::Error for EditError {}

/// Canonical edit: `[start_byte, end_byte)` byte range + inserted UTF-8
/// text (R0 §6).
///
/// Invariants (validated at construction and against a source):
///
/// - `start_byte <= end_byte <= source.len_bytes()`
/// - both boundaries are UTF-8 char boundaries of the old source
/// - inserted text is valid UTF-8 by construction (`String`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEdit {
    start_byte: u64,
    end_byte: u64,
    inserted: String,
}

impl CanonicalEdit {
    /// Byte-range only checks that do not need a source.
    pub fn new(start: usize, end: usize, inserted: impl Into<String>) -> Result<Self, EditError> {
        if start > end {
            return Err(EditError::StartAfterEnd);
        }
        Ok(Self {
            start_byte: start as u64,
            end_byte: end as u64,
            inserted: inserted.into(),
        })
    }

    /// Validate against a concrete old source: range bounds and UTF-8
    /// char boundaries.
    pub fn validate_against(&self, source: &Source) -> Result<(), EditError> {
        let len = source.len_bytes();
        if self.end_byte as usize > len {
            return Err(EditError::OutOfRange);
        }
        let s = source.as_str();
        if !s.is_char_boundary(self.start_byte as usize)
            || !s.is_char_boundary(self.end_byte as usize)
        {
            return Err(EditError::NotCharBoundary);
        }
        Ok(())
    }

    pub fn start_byte(&self) -> u64 {
        self.start_byte
    }

    pub fn end_byte(&self) -> u64 {
        self.end_byte
    }

    pub fn inserted_text(&self) -> &str {
        &self.inserted
    }

    pub fn inserted_text_len_bytes(&self) -> u64 {
        self.inserted.len() as u64
    }

    /// Length removed from the old source by this edit.
    pub fn removed_len_bytes(&self) -> u64 {
        self.end_byte - self.start_byte
    }

    /// Host-side post-edit source materialization. By the frozen timer
    /// contract this happens OUTSIDE every mechanism timer; the runner
    /// calls this before any timing starts.
    pub fn apply(
        &self,
        source: &Source,
        new_id: crate::source::SourceId,
    ) -> Result<Source, EditError> {
        self.validate_against(source)?;
        let s = source.as_str();
        let start = self.start_byte as usize;
        let end = self.end_byte as usize;
        let mut post = String::with_capacity(len_plus(
            s.len(),
            self.inserted.len(),
            self.removed_len_bytes(),
        ));
        post.push_str(&s[..start]);
        post.push_str(&self.inserted);
        post.push_str(&s[end..]);
        Ok(Source::new(new_id, post))
    }
}

fn len_plus(source_len: usize, inserted: usize, removed: u64) -> usize {
    source_len + inserted - removed as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    const SID: SourceId = SourceId(1);

    #[test]
    fn rejects_start_after_end() {
        assert_eq!(CanonicalEdit::new(5, 4, ""), Err(EditError::StartAfterEnd));
    }

    #[test]
    fn rejects_out_of_range_and_bad_boundaries() {
        let src = Source::new(SID, "中文 text");
        // '文' is bytes 3..6; byte 4 splits it.
        assert_eq!(
            CanonicalEdit::new(4, 6, "x").and_then(|e| e.validate_against(&src).map(|_| e)),
            Err(EditError::NotCharBoundary)
        );
        assert_eq!(
            CanonicalEdit::new(0, 99, "x").and_then(|e| e.validate_against(&src).map(|_| e)),
            Err(EditError::OutOfRange)
        );
        // Char-aligned ranges are fine, including CJK/emoji content.
        let ok = CanonicalEdit::new(3, 6, "🐉").expect("edit");
        ok.validate_against(&src).expect("valid");
    }

    #[test]
    fn apply_produces_expected_post_source() {
        let src = Source::new(SID, "hello world");
        let edit = CanonicalEdit::new(5, 5, ",").expect("insert");
        let post = edit.apply(&src, SourceId(2)).expect("apply");
        assert_eq!(post.as_str(), "hello, world");
        assert_eq!(post.id(), SourceId(2));

        let del = CanonicalEdit::new(5, 11, "").expect("delete");
        let post = del.apply(&src, SourceId(3)).expect("apply");
        assert_eq!(post.as_str(), "hello");
    }

    #[test]
    fn classification_matches_byte_facts() {
        let src = Source::new(SID, "0123456789");
        let cases = [
            (
                CanonicalEdit::new(2, 2, "ab").unwrap(),
                OperationKind::Insert,
            ),
            (CanonicalEdit::new(2, 4, "").unwrap(), OperationKind::Delete),
            (
                CanonicalEdit::new(2, 4, "xy").unwrap(),
                OperationKind::ReplaceEq,
            ),
            (
                CanonicalEdit::new(2, 4, "xyz").unwrap(),
                OperationKind::ReplaceGrow,
            ),
            (
                CanonicalEdit::new(2, 4, "x").unwrap(),
                OperationKind::ReplaceShrink,
            ),
        ];
        for (edit, expected) in cases {
            edit.validate_against(&src).expect("valid");
            assert_eq!(OperationKind::classify(&edit), expected);
        }
    }
}
