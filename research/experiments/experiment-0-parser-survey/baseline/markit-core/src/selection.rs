//! Minimal selection model: anchor/head over byte offsets, GPUI-free.
//!
//! A [`Selection`] is a value type, not document state: the document owns
//! text and revision; the editor layer owns selections. The one piece of
//! selection semantics that IS core policy is how a selection survives an
//! edit ([`Selection::map_over_edit`]) — every input path needs the same
//! transform, so it lives here, tested, once.

use crate::position::{ByteOffset, SourceRange};

/// An anchor/head selection. The caret is the head; the selection is
/// collapsed when anchor == head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Selection {
    /// The fixed end of the selection.
    pub anchor: ByteOffset,
    /// The moving end (caret).
    pub head: ByteOffset,
}

impl Selection {
    /// A collapsed caret at `offset`.
    pub fn caret(offset: ByteOffset) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    /// An explicit anchor/head selection.
    pub fn new(anchor: ByteOffset, head: ByteOffset) -> Self {
        Self { anchor, head }
    }

    /// The caret position (moving end).
    pub fn caret_offset(&self) -> ByteOffset {
        self.head
    }

    /// Whether anchor == head (a plain caret).
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    /// Whether the head precedes the anchor.
    pub fn is_reversed(&self) -> bool {
        self.anchor > self.head
    }

    /// The smaller endpoint.
    pub fn start(&self) -> ByteOffset {
        self.anchor.min(self.head)
    }

    /// The larger endpoint.
    pub fn end(&self) -> ByteOffset {
        self.anchor.max(self.head)
    }

    /// The selection as an ordered byte range.
    pub fn to_range(&self) -> SourceRange {
        SourceRange::new(self.start(), self.end())
    }

    /// Whether `offset` lies inside the (non-collapsed) selection:
    /// `start <= offset < end`.
    pub fn contains(&self, offset: ByteOffset) -> bool {
        self.to_range().contains(offset)
    }

    /// Maps the selection over an edit `replaced -> new_text`, endpoint by
    /// endpoint:
    ///
    /// - points at/before the region start are unchanged (**left
    ///   gravity**: a caret at the insertion point stays before the
    ///   inserted text);
    /// - points at/after the region end shift by the byte delta;
    /// - points strictly inside the replaced region collapse to the
    ///   region start.
    ///
    /// This is the documented default transform; an input path that needs
    /// different gravity (e.g. caret to the end of an IME commit) applies
    /// its policy on top of the mapped result.
    pub fn map_over_edit(&self, replaced: SourceRange, new_text: &str) -> Selection {
        Selection {
            anchor: map_point(self.anchor, replaced, new_text),
            head: map_point(self.head, replaced, new_text),
        }
    }
}

fn map_point(point: ByteOffset, replaced: SourceRange, new_text: &str) -> ByteOffset {
    if point <= replaced.start {
        point
    } else if point >= replaced.end {
        let delta = new_text.len() as i64 - replaced.len() as i64;
        ByteOffset((point.as_usize() as i64 + delta) as usize)
    } else {
        replaced.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(anchor: usize, head: usize) -> Selection {
        Selection::new(ByteOffset(anchor), ByteOffset(head))
    }

    fn region(start: usize, end: usize) -> SourceRange {
        SourceRange::new(ByteOffset(start), ByteOffset(end))
    }

    #[test]
    fn collapsed_caret_basics() {
        let c = Selection::caret(ByteOffset(5));
        assert!(c.is_collapsed());
        assert!(!c.is_reversed());
        assert_eq!(c.caret_offset(), ByteOffset(5));
        assert_eq!(c.to_range(), region(5, 5));
        assert!(!c.contains(ByteOffset(5)));
    }

    #[test]
    fn forward_and_reversed_selections() {
        let f = sel(2, 7);
        assert!(!f.is_reversed());
        assert_eq!(f.to_range(), region(2, 7));
        assert!(f.contains(ByteOffset(2)));
        assert!(f.contains(ByteOffset(6)));
        assert!(!f.contains(ByteOffset(7)));

        let r = sel(7, 2);
        assert!(r.is_reversed());
        assert_eq!(r.caret_offset(), ByteOffset(2));
        assert_eq!(r.to_range(), region(2, 7), "range is direction-agnostic");
    }

    #[test]
    fn edit_before_selection_shifts_both_endpoints() {
        let s = sel(10, 14);
        let mapped = s.map_over_edit(region(2, 4), "XYZ");
        assert_eq!(mapped, sel(11, 15));
        // Deletion before: shrink.
        let mapped = s.map_over_edit(region(0, 3), "");
        assert_eq!(mapped, sel(7, 11));
    }

    #[test]
    fn edit_after_selection_is_noop() {
        let s = sel(2, 5);
        let mapped = s.map_over_edit(region(9, 9), "hello");
        assert_eq!(mapped, s);
    }

    #[test]
    fn edit_inside_selection_collapses_it() {
        // Replace a subrange of the selection: the selection collapses to
        // the region start (both endpoints strictly inside map there).
        let s = sel(4, 12);
        let mapped = s.map_over_edit(region(6, 10), "ab");
        assert_eq!(mapped.anchor, ByteOffset(4));
        assert!(mapped.head >= ByteOffset(4));
        // anchor 4 <= start 6 stays; head 12 > end 10 shifts by delta -2
        // -> 10... head is at the region end, not inside.
        assert_eq!(mapped.head, ByteOffset(10));
        assert!(!mapped.is_collapsed());

        // Replace the entire selection: left gravity pins the anchor at
        // the region start and the head lands at start + new_len, so the
        // selection covers exactly the inserted text.
        let s = sel(6, 10);
        let mapped = s.map_over_edit(region(6, 10), "ab");
        assert_eq!(mapped, sel(6, 8));
        // Deleting the whole selection collapses it to the region start.
        let mapped = s.map_over_edit(region(6, 10), "");
        assert_eq!(mapped, Selection::caret(ByteOffset(6)));
    }

    #[test]
    fn delete_around_caret_pulls_caret_to_region_start() {
        // Caret at region end; deleting the region pulls it left.
        let c = Selection::caret(ByteOffset(10));
        let mapped = c.map_over_edit(region(4, 10), "");
        assert_eq!(mapped, Selection::caret(ByteOffset(4)));

        // Caret at region start: left gravity keeps it put.
        let c = Selection::caret(ByteOffset(4));
        let mapped = c.map_over_edit(region(4, 10), "");
        assert_eq!(mapped, Selection::caret(ByteOffset(4)));
    }

    #[test]
    fn insertion_at_caret_has_left_gravity() {
        let c = Selection::caret(ByteOffset(7));
        let mapped = c.map_over_edit(region(7, 7), "插入");
        assert_eq!(mapped, Selection::caret(ByteOffset(7)));
    }

    #[test]
    fn multibyte_deltas_are_in_bytes() {
        // Inserting a 6-byte CJK pair shifts later points by 6.
        let s = sel(3, 9);
        let mapped = s.map_over_edit(region(0, 0), "中文");
        assert_eq!(mapped, sel(9, 15));
    }

    #[test]
    fn reversed_selection_survives_mapping() {
        let s = sel(12, 6);
        let mapped = s.map_over_edit(region(2, 3), "");
        assert!(mapped.is_reversed());
        assert_eq!(mapped, sel(11, 5));
    }
}
