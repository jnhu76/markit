//! Explicit source-coordinate vocabulary (AGENTS.md §8).
//!
//! The canonical source coordinate of markit-core is the **byte offset**
//! into the document's UTF-8 text. Every other coordinate space — Unicode
//! scalars, grapheme boundaries, logical positions, display positions,
//! platform UTF-16 — is deliberately absent from this module:
//!
//! - UTF-16 belongs to the platform edge (GPUI/Windows input handling)
//!   and must not spread through the core;
//! - grapheme/display coordinates get their own explicit types when
//!   those layers exist, instead of hiding inside an ambiguous
//!   `char_offset: usize` API.
//!
//! Byte offsets are always UTF-8 **character boundaries** when they cross
//! a document API; mutations that would split a UTF-8 sequence are
//! rejected (see [`crate::EditError::NotOnCharBoundary`]).

use std::ops::Range;

/// Byte offset into a document's UTF-8 text. The canonical source
/// coordinate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ByteOffset(pub usize);

impl ByteOffset {
    /// The document start.
    pub const ZERO: Self = Self(0);

    /// Numeric form (internal computations and diagnostics).
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Zero-based line ordinal. The first line is `LineNumber(0)`.
///
/// Not to be confused with [`crate::line_index::LineIndex`], which is the
/// incremental data structure mapping byte offsets to lines.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LineNumber(pub usize);

impl LineNumber {
    /// The first line of a document.
    pub const FIRST: Self = Self(0);

    /// Numeric form (internal computations and diagnostics).
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Half-open byte range `[start, end)` in document source coordinates.
///
/// Preconditions (validated where a range enters a document API):
/// `start <= end <= document length`, and both endpoints on UTF-8
/// character boundaries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct SourceRange {
    /// First byte of the range.
    pub start: ByteOffset,
    /// One past the last byte of the range.
    pub end: ByteOffset,
}

impl SourceRange {
    /// Builds a range from its endpoints. Permissive: invalid ranges are
    /// rejected by document APIs, not here.
    pub fn new(start: ByteOffset, end: ByteOffset) -> Self {
        Self { start, end }
    }

    /// Length in bytes. Requires an ordered range
    /// (`start <= end` — all document-accepted ranges are ordered).
    pub fn len(&self) -> usize {
        debug_assert!(self.start <= self.end, "unordered source range");
        self.end.as_usize() - self.start.as_usize()
    }

    /// Whether the range covers no bytes.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Whether `offset` lies in `[start, end)`.
    pub fn contains(&self, offset: ByteOffset) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Smallest range covering both `self` and `other`.
    pub fn covering(self, other: SourceRange) -> SourceRange {
        SourceRange {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Equivalent raw range for internal indexing. Crate-private on
    /// purpose: public APIs stay in explicit coordinate types.
    pub(crate) fn as_usize_range(&self) -> Range<usize> {
        self.start.as_usize()..self.end.as_usize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_basics() {
        let r = SourceRange::new(ByteOffset(2), ByteOffset(5));
        assert_eq!(r.len(), 3);
        assert!(!r.is_empty());
        assert!(r.contains(ByteOffset(2)));
        assert!(r.contains(ByteOffset(4)));
        assert!(!r.contains(ByteOffset(5)));
        assert_eq!(r.as_usize_range(), 2..5);
    }

    #[test]
    fn empty_range_contains_nothing() {
        let r = SourceRange::new(ByteOffset(3), ByteOffset(3));
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(!r.contains(ByteOffset(3)));
    }

    #[test]
    fn covering_spans_both() {
        let a = SourceRange::new(ByteOffset(4), ByteOffset(6));
        let b = SourceRange::new(ByteOffset(10), ByteOffset(12));
        let c = a.covering(b);
        assert_eq!(c, SourceRange::new(ByteOffset(4), ByteOffset(12)));
        assert_eq!(b.covering(a), c);
        assert_eq!(a.covering(a), a);
    }

    #[test]
    fn offsets_order_naturally() {
        assert!(ByteOffset(1) < ByteOffset(2));
        assert_eq!(ByteOffset::ZERO, ByteOffset(0));
        assert!(LineNumber::FIRST < LineNumber(1));
    }
}
