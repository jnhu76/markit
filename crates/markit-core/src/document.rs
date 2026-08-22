//! The document model: private storage + incremental line index +
//! monotonic revision.
//!
//! [`Document`] is the canonical owner of text in markit-core. Its
//! storage representation is private (a plain `String` today — allowed
//! for P0-01; a rope/piece table is a real-workload decision, not a
//! pre-emptive one, ADR-003). All reads go through byte/line queries
//! returning borrowed views, so the representation can change without
//! breaking consumers.
//!
//! Every mutation funnels through [`Document::apply_edit`] /
//! [`Document::apply_transaction`]: validate everything before touching
//! storage (all-or-nothing), splice the storage, update the line index
//! incrementally, advance the revision exactly once, and return an
//! [`EditResult`] whose changed ranges were computed at mutation time.

use std::borrow::Cow;
use std::ops::Range;

use crate::change::{AppliedEdit, ChangeKind, EditError, EditResult, EditWork, TextEdit};
use crate::id::DocumentId;
use crate::line_index::{LineIndex, LineIndexCounters};
use crate::position::{ByteOffset, LineNumber, SourceRange};
use crate::revision::{DocumentRevision, DocumentVersion};
use crate::snapshot::DocumentSnapshot;
use crate::transaction::EditTransaction;

/// A UTF-8 text document with incremental line indexing and version
/// identity.
///
/// Storage is private by design. The known costs of the current
/// representation, documented rather than hidden:
///
/// - `String` splicing is O(document bytes) per edit (memmove of the
///   suffix) — acceptable at current workloads, revisited only with
///   measurement;
/// - the line-index suffix shift is O(lines after the edit),
///   position-dependent and instrumented
///   ([`EditWork::line_entries_touched`]).
///
/// ## Not cloneable, on purpose
///
/// [`Document`] deliberately does **not** implement `Clone`: a clone would
/// duplicate the [`DocumentId`] and revision while letting the copies
/// diverge independently, so `(DocumentId, DocumentRevision)` would stop
/// naming exactly one coherent state — the invariant the whole
/// version/staleness model is built on. If document duplication ever
/// becomes a real workload, it gets an explicit `fork`/`duplicate` API
/// that mints a fresh `DocumentId` and defines its revision semantics;
/// identity-preserving duplication stays a compile error until then.
///
/// ```compile_fail
/// let doc = markit_core::Document::new("x");
/// let _copy = doc.clone();
/// ```
#[derive(Debug)]
pub struct Document {
    id: DocumentId,
    storage: String,
    lines: LineIndex,
    revision: DocumentRevision,
}

impl Document {
    /// Loads `text` with one full line scan — the only full scan the
    /// model ever performs for this document (ADR-003).
    pub fn new(text: impl Into<String>) -> Self {
        let storage = text.into();
        let lines = LineIndex::from_text(&storage);
        Self {
            id: DocumentId::new(),
            storage,
            lines,
            revision: DocumentRevision::INITIAL,
        }
    }

    /// Stable identity of this document.
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// Current revision. Advances by exactly one per successful mutation.
    /// Only meaningful together with [`Document::id`]; prefer
    /// [`Document::version`] when the value crosses a seam.
    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    /// The current coherent version: identity + revision. This is the unit
    /// of validity for derived results ([`Revisioned`]) — a bare revision
    /// from a different document must never validate against this one.
    pub fn version(&self) -> DocumentVersion {
        DocumentVersion::new(self.id, self.revision)
    }

    /// Total byte length.
    pub fn len_bytes(&self) -> usize {
        self.storage.len()
    }

    /// Whether the document has no bytes (it still has one empty line).
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Whether `offset` lies on a UTF-8 character boundary.
    pub fn is_char_boundary(&self, offset: ByteOffset) -> bool {
        self.storage.is_char_boundary(offset.as_usize())
    }

    // ---- line queries ------------------------------------------------------

    /// Number of lines (always >= 1).
    pub fn line_count(&self) -> usize {
        self.lines.line_count()
    }

    /// The line containing `offset`; `offset == len_bytes()` (EOF)
    /// addresses the last line. Panics if `offset > len_bytes()`.
    pub fn line_of(&self, offset: ByteOffset) -> LineNumber {
        LineNumber(self.lines.line_of(offset.as_usize()))
    }

    /// Content byte range of `line` (excluding its `'\n'` terminator).
    /// Panics if `line >= line_count()`.
    pub fn line_range(&self, line: LineNumber) -> SourceRange {
        let line = line.as_usize();
        SourceRange::new(
            ByteOffset(self.lines.line_start(line)),
            ByteOffset(self.lines.line_content_end(line)),
        )
    }

    /// Content of `line` (excluding its `'\n'` terminator), borrowed from
    /// storage. Panics if `line >= line_count()`.
    pub fn line_str(&self, line: LineNumber) -> Cow<'_, str> {
        self.slice(self.line_range(line))
    }

    /// Inclusive line span covering the byte `range` — the
    /// line-granular view of a changed range for downstream
    /// invalidation. Panics on an out-of-bounds range.
    pub fn line_span(&self, range: SourceRange) -> Range<LineNumber> {
        let span = self.lines.line_span_of_range(range.as_usize_range());
        LineNumber(span.start)..LineNumber(span.end)
    }

    // ---- reads --------------------------------------------------------------

    /// Reads a byte range, borrowed from storage.
    ///
    /// Panics if the range is unordered, out of bounds, or not on UTF-8
    /// character boundaries (same contract as `str` slicing, with a
    /// clearer message). Use [`Document::try_slice`] for the fallible
    /// form.
    pub fn slice(&self, range: SourceRange) -> Cow<'_, str> {
        match self.try_slice(range) {
            Some(text) => text,
            None => panic!(
                "invalid source range [{}, {}) for document of {} bytes",
                range.start.as_usize(),
                range.end.as_usize(),
                self.storage.len()
            ),
        }
    }

    /// Fallible range read. Returns `None` for unordered, out-of-bounds,
    /// or non-boundary ranges.
    pub fn try_slice(&self, range: SourceRange) -> Option<Cow<'_, str>> {
        if range.start > range.end || range.end.as_usize() > self.storage.len() {
            return None;
        }
        if !self.storage.is_char_boundary(range.start.as_usize())
            || !self.storage.is_char_boundary(range.end.as_usize())
        {
            return None;
        }
        Some(Cow::Borrowed(&self.storage[range.as_usize_range()]))
    }

    // ---- mutation -----------------------------------------------------------

    /// Applies a single edit. One revision bump; all-or-nothing
    /// validation.
    pub fn apply_edit(&mut self, edit: TextEdit) -> Result<EditResult, EditError> {
        self.apply_edits(vec![edit]).map(|(result, _)| result)
    }

    /// Applies a transaction atomically: every edit is validated before
    /// any mutation (all-or-nothing), edits apply at non-overlapping
    /// offsets, and the revision advances exactly once. Prefer
    /// [`EditTransaction::apply`], which also returns the inverse.
    pub fn apply_transaction(
        &mut self,
        transaction: EditTransaction,
    ) -> Result<EditResult, EditError> {
        self.apply_edits(transaction.into_edits())
            .map(|(result, _)| result)
    }

    /// A coherent O(1) read view of this document. See
    /// [`DocumentSnapshot`] for the coherence and boundary contract.
    pub fn snapshot(&self) -> DocumentSnapshot<'_> {
        DocumentSnapshot::new(self)
    }

    /// Shared mutation path. Returns the result plus the inverse edits
    /// (post-edit coordinates, original text) — the undo seam consumed
    /// by [`EditTransaction::apply`].
    pub(crate) fn apply_transaction_with_inverse(
        &mut self,
        transaction: EditTransaction,
    ) -> Result<(EditResult, Vec<TextEdit>), EditError> {
        self.apply_edits(transaction.into_edits())
    }

    /// Replaces the whole document (load / reload / external rewrite).
    /// Full line rebuild; classified as
    /// [`ChangeKind::ReplaceDocument`] so downstream layers never
    /// confuse it with a local edit.
    pub fn replace_all(&mut self, text: impl Into<String>) -> EditResult {
        let base_revision = self.revision;
        let old_len = self.storage.len();
        let old_line_count = self.lines.line_count();

        self.storage = text.into();
        self.lines = LineIndex::from_text(&self.storage);
        self.revision = base_revision.next();

        let new_len = self.storage.len();
        let byte_delta = new_len as i64 - old_len as i64;
        let old_range = SourceRange::new(ByteOffset::ZERO, ByteOffset(old_len));
        let new_range = SourceRange::new(ByteOffset::ZERO, ByteOffset(new_len));
        let counters = self.lines.last_update_counters();
        let new_line_count = self.lines.line_count();
        let old_line_span = LineNumber(0)..LineNumber(old_line_count);
        let new_line_span = LineNumber(0)..LineNumber(new_line_count);

        EditResult {
            base_revision,
            new_revision: self.revision,
            kind: ChangeKind::ReplaceDocument,
            covering_old_range: old_range,
            covering_new_range: new_range,
            byte_delta,
            line_delta: new_line_count as i64 - old_line_count as i64,
            edits: vec![AppliedEdit {
                kind: ChangeKind::ReplaceDocument,
                old_range,
                new_range,
                byte_delta,
                old_line_span,
                new_line_span,
            }],
            work: EditWork {
                changed_bytes: (old_len + new_len) as u64,
                changed_lines: new_line_count as u64,
                bytes_scanned: counters.bytes_scanned,
                line_entries_touched: counters.line_entries_touched,
                full_rebuilds: counters.full_rebuilds,
            },
        }
    }

    /// Shared mutation path. Returns the result plus the inverse edits
    /// (post-edit coordinates, original text) — the undo seam consumed
    /// by [`EditTransaction::apply`].
    pub(crate) fn apply_edits(
        &mut self,
        edits: Vec<TextEdit>,
    ) -> Result<(EditResult, Vec<TextEdit>), EditError> {
        // 1. Filter no-ops, sort by position, and validate EVERYTHING
        //    before touching storage: rejected mutations leave the
        //    document (text, line index, revision) untouched.
        let mut edits: Vec<TextEdit> = edits.into_iter().filter(|e| !e.is_noop()).collect();
        if edits.is_empty() {
            return Err(EditError::EmptyTransaction);
        }
        edits.sort_by_key(|e| e.range.start);
        for pair in edits.windows(2) {
            if pair[1].range.start < pair[0].range.end {
                return Err(EditError::OverlappingEdits {
                    first: pair[0].range,
                    second: pair[1].range,
                });
            }
        }
        for edit in &edits {
            if edit.range.start > edit.range.end || edit.range.end.as_usize() > self.storage.len() {
                return Err(EditError::InvalidRange {
                    start: edit.range.start,
                    end: edit.range.end,
                    document_len: self.storage.len(),
                });
            }
            if !self.storage.is_char_boundary(edit.range.start.as_usize()) {
                return Err(EditError::NotOnCharBoundary {
                    offset: edit.range.start,
                });
            }
            if !self.storage.is_char_boundary(edit.range.end.as_usize()) {
                return Err(EditError::NotOnCharBoundary {
                    offset: edit.range.end,
                });
            }
        }

        let base_revision = self.revision;
        let old_doc_len = self.storage.len();
        let old_line_count = self.lines.line_count();

        // 2. Capture inverse material, per-edit final coordinates, and
        //    per-edit PRE-EDIT line spans in one ascending pass (the line
        //    index still describes the pre-edit document here). Because
        //    edits are non-overlapping and position-sorted, an edit's text
        //    in the FINAL document starts at its original start plus the
        //    deltas of all earlier edits.
        let mut inverse: Vec<TextEdit> = Vec::with_capacity(edits.len());
        let mut final_new_ranges: Vec<SourceRange> = Vec::with_capacity(edits.len());
        let mut old_line_spans: Vec<Range<LineNumber>> = Vec::with_capacity(edits.len());
        let mut shift: i64 = 0;
        for edit in &edits {
            let old_text = self.storage[edit.range.as_usize_range()].to_string();
            let final_start = (edit.range.start.as_usize() as i64 + shift) as usize;
            let final_end = final_start + edit.new_text.len();
            final_new_ranges.push(SourceRange::new(
                ByteOffset(final_start),
                ByteOffset(final_end),
            ));
            old_line_spans.push(self.line_span(edit.range));
            inverse.push(TextEdit::replace(
                SourceRange::new(ByteOffset(final_start), ByteOffset(final_end)),
                old_text,
            ));
            shift += edit.new_text.len() as i64 - edit.range.len() as i64;
        }

        // 3. Apply back-to-front so earlier coordinates stay valid for
        //    each subsequent edit, updating the line index incrementally.
        let mut counters = LineIndexCounters::default();
        let mut covering_old: Option<SourceRange> = None;
        let mut covering_new: Option<SourceRange> = None;
        let mut changed_bytes: u64 = 0;

        for i in (0..edits.len()).rev() {
            let edit = &edits[i];
            self.storage
                .replace_range(edit.range.as_usize_range(), &edit.new_text);
            self.lines
                .apply_edit(edit.range.as_usize_range(), &edit.new_text);
            counters.absorb(&self.lines.last_update_counters());

            covering_old = Some(match covering_old {
                None => edit.range,
                Some(covering) => covering.covering(edit.range),
            });
            covering_new = Some(match covering_new {
                None => final_new_ranges[i],
                Some(covering) => covering.covering(final_new_ranges[i]),
            });
            changed_bytes += (edit.range.len() + edit.new_text.len()) as u64;
        }

        // 4. Per-edit POST-EDIT line spans from the final line index, and
        //    the canonical changed-line total: the disjoint union of those
        //    spans. Two distant edits stay two small regions — the union
        //    must NOT collapse them into one covering span (a multi-cursor
        //    or format transaction on a large document would otherwise
        //    report a document-wide invalidation).
        let applied: Vec<AppliedEdit> = edits
            .iter()
            .enumerate()
            .map(|(i, edit)| AppliedEdit {
                kind: ChangeKind::classify(
                    edit.range.len(),
                    edit.new_text.len(),
                    edit.range.start.as_usize() == old_doc_len,
                )
                .expect("no-op edits are filtered before application"),
                old_range: edit.range,
                new_range: final_new_ranges[i],
                byte_delta: edit.new_text.len() as i64 - edit.range.len() as i64,
                old_line_span: old_line_spans[i].clone(),
                new_line_span: self.line_span(final_new_ranges[i]),
            })
            .collect();

        let mut changed_lines: u64 = 0;
        let mut merged: Option<Range<LineNumber>> = None;
        for span in applied.iter().map(|edit| edit.new_line_span.clone()) {
            match merged {
                // Spans arrive in ascending order; overlap merges, gap sums.
                Some(open) if span.start <= open.end => {
                    merged = Some(open.start..open.end.max(span.end));
                }
                Some(done) => {
                    changed_lines += (done.end.as_usize() - done.start.as_usize()) as u64;
                    merged = Some(span);
                }
                None => merged = Some(span),
            }
        }
        if let Some(last) = merged {
            changed_lines += (last.end.as_usize() - last.start.as_usize()) as u64;
        }

        let covering_old_range = covering_old.expect("at least one effective edit");
        let covering_new_range = covering_new.expect("at least one effective edit");
        let kind = ChangeKind::classify(
            covering_old_range.len(),
            covering_new_range.len(),
            covering_old_range.start.as_usize() == old_doc_len,
        )
        .expect("covering range of effective edits is effective");

        self.revision = base_revision.next();

        let result = EditResult {
            base_revision,
            new_revision: self.revision,
            kind,
            covering_old_range,
            covering_new_range,
            byte_delta: self.storage.len() as i64 - old_doc_len as i64,
            line_delta: self.lines.line_count() as i64 - old_line_count as i64,
            edits: applied,
            work: EditWork {
                changed_bytes,
                changed_lines,
                bytes_scanned: counters.bytes_scanned,
                line_entries_touched: counters.line_entries_touched,
                full_rebuilds: counters.full_rebuilds,
            },
        };
        Ok((result, inverse))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision::Revisioned;

    fn offset(n: usize) -> ByteOffset {
        ByteOffset(n)
    }

    fn range(start: usize, end: usize) -> SourceRange {
        SourceRange::new(offset(start), offset(end))
    }

    fn text_of(doc: &Document) -> String {
        doc.slice(range(0, doc.len_bytes())).into_owned()
    }

    /// Full-scan oracle, identical in spirit to the prototype's
    /// per-edit rebuild: the differential tests must hold
    /// `incremental document == full scan` after every operation.
    fn oracle_starts(text: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    fn assert_matches_oracle(doc: &Document, text: &str) {
        assert_eq!(&text_of(doc), text, "document text diverges");
        assert_eq!(doc.len_bytes(), text.len());
        let oracle = oracle_starts(text);
        assert_eq!(doc.line_count(), oracle.len());
        assert_eq!(doc.lines.starts(), &oracle[..]);
        for byte in 0..=text.len() {
            let oracle_line = oracle.partition_point(|&s| s <= byte) - 1;
            assert_eq!(
                doc.line_of(offset(byte)).as_usize(),
                oracle_line,
                "line_of({byte}) diverges for {text:?}"
            );
        }
        let expected_lines: Vec<&str> = text.split('\n').collect();
        for (line, expected) in expected_lines.iter().enumerate() {
            let r = doc.line_range(LineNumber(line));
            assert_eq!(doc.line_str(LineNumber(line)).as_ref(), *expected);
            assert_eq!(doc.slice(r).as_ref(), *expected);
        }
        doc.lines.assert_invariants();
    }

    // ---- A. document edits ------------------------------------------------

    #[test]
    fn insert_begin_middle_end() {
        let mut doc = Document::new("hello world");
        doc.apply_edit(TextEdit::insert(offset(0), "X")).unwrap();
        assert_matches_oracle(&doc, "Xhello world");

        doc.apply_edit(TextEdit::insert(offset(6), "Y")).unwrap();
        assert_matches_oracle(&doc, "XhelloY world");

        doc.apply_edit(TextEdit::insert(offset(doc.len_bytes()), "!"))
            .unwrap();
        assert_matches_oracle(&doc, "XhelloY world!");
    }

    #[test]
    fn delete_and_replace() {
        let mut doc = Document::new("hello world");
        doc.apply_edit(TextEdit::delete(range(0, 1))).unwrap();
        assert_matches_oracle(&doc, "ello world");

        // "llo " ([1,5)) includes the space.
        doc.apply_edit(TextEdit::replace(range(1, 5), "EL"))
            .unwrap();
        assert_matches_oracle(&doc, "eELworld");

        doc.apply_edit(TextEdit::delete(range(
            doc.len_bytes() - 5,
            doc.len_bytes(),
        )))
        .unwrap();
        assert_matches_oracle(&doc, "eEL");
    }

    #[test]
    fn newline_insert_and_delete() {
        let mut doc = Document::new("ab");
        doc.apply_edit(TextEdit::insert(offset(1), "\n")).unwrap();
        assert_matches_oracle(&doc, "a\nb");
        assert_eq!(doc.line_count(), 2);

        doc.apply_edit(TextEdit::insert(doc.line_range(LineNumber(1)).start, "\n"))
            .unwrap();
        assert_matches_oracle(&doc, "a\n\nb");
        assert_eq!(doc.line_count(), 3);

        // Deleting the middle empty line merges around it.
        doc.apply_edit(TextEdit::delete(range(2, 3))).unwrap();
        assert_matches_oracle(&doc, "a\nb");
        assert_eq!(doc.line_count(), 2);
    }

    #[test]
    fn empty_document_edits() {
        let mut doc = Document::new("");
        assert!(doc.is_empty());
        assert_eq!(doc.line_count(), 1);
        assert_matches_oracle(&doc, "");

        doc.apply_edit(TextEdit::insert(offset(0), "a")).unwrap();
        assert_matches_oracle(&doc, "a");

        doc.apply_edit(TextEdit::delete(range(0, 1))).unwrap();
        assert_matches_oracle(&doc, "");
    }

    #[test]
    fn eof_edits_on_trailing_newline_document() {
        // Sentinel-class regression: the EOF position must address the
        // last (empty) line, never an out-of-range line.
        let mut doc = Document::new("ab\n");
        assert_eq!(doc.line_count(), 2);
        assert_eq!(doc.line_of(offset(3)).as_usize(), 1);
        assert_eq!(doc.line_range(LineNumber(1)), range(3, 3));

        doc.apply_edit(TextEdit::insert(offset(3), "X")).unwrap();
        assert_matches_oracle(&doc, "ab\nX");
        assert_eq!(doc.line_str(LineNumber(1)).as_ref(), "X");

        // Delete the trailing newline: two lines merge into one.
        doc.apply_edit(TextEdit::delete(range(2, 3))).unwrap();
        assert_matches_oracle(&doc, "abX");
        assert_eq!(doc.line_count(), 1);
    }

    #[test]
    fn delete_range_ending_at_eof() {
        let mut doc = Document::new("a\nbc");
        doc.apply_edit(TextEdit::delete(range(3, 4))).unwrap();
        assert_matches_oracle(&doc, "a\nb");
        // The '\n' is at byte 1.
        doc.apply_edit(TextEdit::delete(range(1, 2))).unwrap();
        assert_matches_oracle(&doc, "ab");
    }

    #[test]
    fn edit_result_reports_exact_ranges() {
        let mut doc = Document::new("hello world");
        let r = doc
            .apply_edit(TextEdit::replace(range(6, 11), "markit"))
            .unwrap();
        assert_eq!(r.base_revision, DocumentRevision::INITIAL);
        assert_eq!(r.new_revision, DocumentRevision::INITIAL.next());
        assert_eq!(r.kind, ChangeKind::Replace);
        assert_eq!(r.covering_old_range, range(6, 11));
        assert_eq!(r.covering_new_range, range(6, 12));
        assert_eq!(r.byte_delta, 1, "markit(6) - world(5)");
        assert_eq!(r.line_delta, 0);
        assert_eq!(r.edits.len(), 1);
        assert_eq!(r.edits[0].kind, ChangeKind::Replace);
        assert_eq!(r.edits[0].old_range, range(6, 11));
        assert_eq!(r.edits[0].new_range, range(6, 12));
        assert_eq!(r.edits[0].old_line_span, LineNumber(0)..LineNumber(1));
        assert_eq!(r.edits[0].new_line_span, LineNumber(0)..LineNumber(1));
        assert_eq!(r.work.changed_bytes, 5 + 6);
        assert_eq!(r.work.changed_lines, 1);
        assert_eq!(text_of(&doc), "hello markit");
    }

    #[test]
    fn append_vs_insert_classification() {
        let mut doc = Document::new("ab");
        let r = doc.apply_edit(TextEdit::insert(offset(1), "X")).unwrap();
        assert_eq!(r.kind, ChangeKind::Insert);
        let r = doc
            .apply_edit(TextEdit::insert(offset(doc.len_bytes()), "c"))
            .unwrap();
        assert_eq!(r.kind, ChangeKind::Append);
        let r = doc.apply_edit(TextEdit::delete(range(0, 1))).unwrap();
        assert_eq!(r.kind, ChangeKind::Delete);
    }

    // ---- invalid input leaves the document untouched ----------------------

    #[test]
    fn invalid_edits_are_rejected_atomically() {
        let mut doc = Document::new("中文");
        let before = text_of(&doc);
        let rev = doc.revision();

        let cases = [
            TextEdit::insert(offset(9), "x"),    // out of bounds (len 6)
            TextEdit::delete(range(4, 2)),       // inverted
            TextEdit::insert(offset(1), "x"),    // splits 中 (3-byte scalar)
            TextEdit::replace(range(2, 4), "x"), // splits both scalars
        ];
        for edit in cases {
            assert!(doc.apply_edit(edit).is_err(), "edit must be rejected");
            assert_eq!(&text_of(&doc), &before);
            assert_eq!(doc.revision(), rev);
        }
    }

    #[test]
    fn emoji_boundaries_are_enforced() {
        // 🙂 is a 4-byte scalar: interior offsets must be rejected.
        let mut doc = Document::new("a🙂b");
        let before = text_of(&doc);
        for bad in [2usize, 3, 4] {
            assert!(doc.apply_edit(TextEdit::insert(offset(bad), "x")).is_err());
        }
        assert_eq!(&text_of(&doc), &before);
        doc.apply_edit(TextEdit::insert(offset(1), "🙂")).unwrap();
        assert_matches_oracle(&doc, "a🙂🙂b");
    }

    #[test]
    fn noop_edit_is_rejected() {
        let mut doc = Document::new("abc");
        assert_eq!(
            doc.apply_edit(TextEdit::insert(offset(1), "")),
            Err(EditError::EmptyTransaction)
        );
        assert_eq!(doc.revision(), DocumentRevision::INITIAL);
    }

    #[test]
    fn invalid_slice_is_rejected() {
        let doc = Document::new("中文");
        assert!(doc.try_slice(range(1, 2)).is_none());
        assert!(doc.try_slice(range(0, 7)).is_none());
        assert!(doc.try_slice(range(2, 1)).is_none());
        assert_eq!(doc.try_slice(range(0, 3)).unwrap().as_ref(), "中");
    }

    // ---- C. revision semantics ---------------------------------------------

    #[test]
    fn revision_advances_exactly_once_per_mutation() {
        let mut doc = Document::new("a");
        assert_eq!(doc.revision(), DocumentRevision::INITIAL);
        for i in 1..=5 {
            let r = doc.apply_edit(TextEdit::insert(offset(0), "x")).unwrap();
            assert_eq!(r.base_revision.as_u64(), i - 1);
            assert_eq!(r.new_revision.as_u64(), i);
            assert_eq!(doc.revision().as_u64(), i);
        }
        // Rejected mutation: no bump.
        doc.apply_edit(TextEdit::insert(offset(999), "x"))
            .unwrap_err();
        assert_eq!(doc.revision().as_u64(), 5);
    }

    #[test]
    fn stale_derived_result_cannot_commit_over_newer_revision() {
        // INV-10 seam: a derived result captured at version (id, N) must
        // be rejected once the document has moved on.
        let mut doc = Document::new("line1\nline2");
        let derived = Revisioned::new(doc.version(), doc.line_count());
        assert_eq!(derived.commit(doc.version()), Ok(2));

        doc.apply_edit(TextEdit::insert(offset(0), "\n")).unwrap();
        assert!(doc.revision() > derived.base_version().revision());

        let stale = derived.commit(doc.version()).unwrap_err();
        assert_eq!(stale.value, 2);
        assert_eq!(stale.current_version.document_id(), doc.id());
        assert!(stale.current_version.revision() > stale.base_version.revision());
    }

    #[test]
    fn derived_result_from_a_different_document_is_rejected() {
        // Equal numeric revisions from different documents are unrelated
        // states: the version seam must reject the cross-document commit.
        let a = Document::new("alpha");
        let b = Document::new("beta");
        assert_ne!(a.id(), b.id());
        assert_eq!(a.revision(), b.revision());

        let derived_from_a = Revisioned::new(a.version(), a.line_count());
        assert_eq!(derived_from_a.commit(a.version()), Ok(1));
        assert!(
            derived_from_a.commit(b.version()).is_err(),
            "same revision number, different document: must reject"
        );
    }

    #[test]
    fn replace_all_semantics() {
        let mut doc = Document::new("a\nb\nc");
        let r = doc.replace_all("x\ny");
        assert_eq!(r.kind, ChangeKind::ReplaceDocument);
        assert_eq!(r.base_revision, DocumentRevision::INITIAL);
        assert_eq!(r.new_revision, DocumentRevision::INITIAL.next());
        assert_eq!(r.byte_delta, -2, "x\\ny(3) - a\\nb\\nc(5)");
        assert_eq!(r.line_delta, -1);
        assert_eq!(r.work.full_rebuilds, 1);
        assert_eq!(r.work.bytes_scanned, 3, "scans the replacement text");
        assert_eq!(r.work.changed_lines, 2, "replacement touches all lines");
        assert_matches_oracle(&doc, "x\ny");
    }

    // ---- Unicode content correctness ----------------------------------------

    #[test]
    fn multibyte_edit_coordinates() {
        let mut doc = Document::new("中文文本");
        // Replace 文本 (bytes 6..12) with 字.
        let r = doc
            .apply_edit(TextEdit::replace(range(6, 12), "字"))
            .unwrap();
        assert_eq!(r.byte_delta, -3, "字(3) - 文本(6)");
        assert_matches_oracle(&doc, "中文字");
        assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "中文字");

        doc.apply_edit(TextEdit::insert(offset(9), "🙂")).unwrap();
        assert_matches_oracle(&doc, "中文字🙂");
    }

    #[test]
    fn mixed_cjk_newline_document() {
        let text = "标题\n中文段落。\n\nEnglish tail 🙂\n";
        let mut doc = Document::new(text);
        assert_matches_oracle(&doc, text);
        doc.apply_edit(TextEdit::insert(offset(0), "# ")).unwrap();
        let edited = format!("# {text}");
        assert_matches_oracle(&doc, &edited);
        assert_eq!(doc.line_str(LineNumber(0)).as_ref(), "# 标题");
        // The trailing empty line remains addressable.
        let last = doc.line_count() - 1;
        assert_eq!(
            doc.line_range(LineNumber(last)),
            range(edited.len(), edited.len())
        );
    }

    // ---- work amplification (structural, not wall-clock) -------------------

    #[test]
    fn single_char_edit_on_large_document_is_structurally_bounded() {
        // INV-01/INV-08: a normal local edit scans only its own new
        // text — never the document — and touches line entries only
        // around/after the edit.
        let text = "l\n".repeat(50_000);
        let mut doc = Document::new(&text);
        let total_lines = doc.line_count();

        let r = doc
            .apply_edit(TextEdit::insert(offset(text.len() - 1), "Z"))
            .unwrap();
        assert_eq!(r.work.bytes_scanned, 1);
        assert_eq!(r.work.full_rebuilds, 0);
        assert_eq!(r.work.changed_bytes, 1);
        assert_eq!(r.work.changed_lines, 1);
        assert!(r.work.line_entries_touched < 3);
        assert!(total_lines >= 50_000);
    }

    #[test]
    fn large_document_positions_and_differential() {
        // 10K+ line corpus; edits at begin / q1 / mid / q3 / end; the
        // incremental index must equal the full-scan oracle after all
        // edits (checked once at the end — per-op equality is covered by
        // the randomized battery on small documents).
        let text = "l\n".repeat(25_000);
        let mut doc = Document::new(&text);
        let len = text.len();
        let positions = [0, len / 4, len / 2, len * 3 / 4, len];
        let mut expected = text.clone();
        for (i, &p) in positions.iter().enumerate() {
            let marker = format!("M{i}");
            doc.apply_edit(TextEdit::insert(offset(p + i), &marker))
                .unwrap();
            expected.insert_str(p + i, &marker);
        }
        // Delete one character and replace a slice as well.
        doc.apply_edit(TextEdit::delete(range(len / 2, len / 2 + 1)))
            .unwrap();
        expected.remove(len / 2);
        doc.apply_edit(TextEdit::replace(range(1, 3), "xx"))
            .unwrap();
        expected.replace_range(1..3, "xx");

        assert_matches_oracle(&doc, &expected);
    }

    #[test]
    fn million_line_correctness_case() {
        // 1M-line synthetic document (~2MB): the assertions below are
        // structural (work counters + oracle equality), so they cannot
        // pass "by being fast on this machine".
        let text = "l\n".repeat(1_000_000);
        let mut doc = Document::new(&text);
        let len = text.len();

        let r = doc
            .apply_edit(TextEdit::insert(offset(len / 2), "中"))
            .unwrap();
        assert_eq!(r.work.bytes_scanned, 3, "scans only the inserted text");
        assert_eq!(r.work.full_rebuilds, 0);
        assert_eq!(r.work.changed_bytes, 3);
        assert_eq!(r.work.changed_lines, 1);

        let r = doc
            .apply_edit(TextEdit::delete(range(len / 3, len / 3 + 1)))
            .unwrap();
        assert_eq!(r.work.bytes_scanned, 0, "deletion scans nothing");
        assert_eq!(r.work.full_rebuilds, 0);

        let mut expected = text.clone();
        expected.insert(len / 2, '中');
        expected.remove(len / 3);
        assert_matches_oracle(&doc, &expected);
    }

    // ---- sparse multi-edit invalidation (canonical dirty regions) ---------

    #[test]
    fn distant_edits_on_million_line_document_stay_sparse() {
        // Regression for the covering-range failure mode: two tiny edits
        // at opposite ends of a 1M-line document must remain two small
        // canonical dirty regions — never a ~1M-line invalidation. The
        // covering range is convenience-only precisely because here it
        // spans nearly the whole document.
        let text = "l\n".repeat(1_000_000);
        let mut doc = Document::new(&text);
        let len = text.len();

        let applied = crate::EditTransaction::command()
            .with_edit(TextEdit::insert(offset(0), "X")) // line 0
            .with_edit(TextEdit::insert(offset(len), "Y")) // final empty line
            .apply(&mut doc)
            .unwrap();

        let result = applied.result;
        assert_eq!(result.edits.len(), 2, "canonical regions: one per edit");

        let spans: Vec<Range<LineNumber>> = result
            .edits
            .iter()
            .map(|e| e.new_line_span.clone())
            .collect();
        for span in &spans {
            assert_eq!(
                span.end.as_usize() - span.start.as_usize(),
                1,
                "each region stays one line"
            );
        }
        assert!(spans[0].end <= spans[1].start, "regions stay disjoint");
        assert_eq!(spans[0].start, LineNumber(0));
        assert_eq!(spans[1].end, LineNumber(doc.line_count()));

        // changed_lines is the union of the actual spans, not the distance
        // between the edits.
        assert_eq!(result.work.changed_lines, 2);

        // And the covering range really does span the document — which is
        // why it must never be used as the invalidation region.
        assert_eq!(result.covering_new_range, range(0, len + 2));

        let mut expected = text.clone();
        expected.insert(0, 'X');
        expected.push('Y');
        assert_matches_oracle(&doc, &expected);
    }

    #[test]
    fn newline_deleting_edit_reports_the_merged_line() {
        let mut doc = Document::new("aa\nbb\ncc");
        let r = doc
            .apply_edits(vec![TextEdit::delete(range(2, 3))])
            .unwrap()
            .0;
        assert_eq!(text_of(&doc), "aabb\ncc");
        // Pre-edit the '\n' belonged to line 0; post-edit the disturbed
        // region is the single merged line, which holds the former line-1
        // content too — one line, not two, and never the document.
        assert_eq!(r.edits[0].old_line_span, LineNumber(0)..LineNumber(1));
        assert_eq!(r.edits[0].new_line_span, LineNumber(0)..LineNumber(1));
        assert_eq!(r.work.changed_lines, 1);
    }

    #[test]
    fn same_line_double_edit_counts_one_line() {
        let mut doc = Document::new("aa\nbb\ncc");
        let r = doc
            .apply_edits(vec![
                TextEdit::replace(range(0, 1), "X"),
                TextEdit::replace(range(1, 2), "Y"),
            ])
            .unwrap()
            .0;
        assert_eq!(text_of(&doc), "XY\nbb\ncc");
        assert_eq!(r.work.changed_lines, 1, "both edits share line 0");
        assert_eq!(r.edits.len(), 2);
        assert_eq!(r.edits[0].new_line_span, LineNumber(0)..LineNumber(1));
        assert_eq!(r.edits[1].new_line_span, LineNumber(0)..LineNumber(1));
    }

    // ---- randomized differential vs mirror string ---------------------------

    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Self(seed.wrapping_add(0x9E3779B97F4A7C15))
        }
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next_u64() % n as u64) as usize
        }
    }

    const ALPHABET: [&str; 9] = ["a", "b", "x", "中", "文", "🙂", "é", "\n", "\n"];

    fn random_string(rng: &mut Rng, len: usize) -> String {
        (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect()
    }

    fn char_boundaries(text: &str) -> Vec<usize> {
        text.char_indices()
            .map(|(i, _)| i)
            .chain([text.len()])
            .collect()
    }

    #[test]
    fn randomized_document_differential() {
        for seed in 1..=6u64 {
            let mut rng = Rng::new(seed);
            let initial_len = 1 + rng.below(100);
            let mut mirror = random_string(&mut rng, initial_len);
            let mut doc = Document::new(mirror.clone());
            assert_matches_oracle(&doc, &mirror);

            for _ in 0..300 {
                let bounds = char_boundaries(&mirror);
                let start_idx = rng.below(bounds.len());
                let start = bounds[start_idx];
                let end_idx = start_idx + rng.below(bounds.len() - start_idx);
                let end = bounds[end_idx];
                let new_len = rng.below(7);
                let new_text = random_string(&mut rng, new_len);
                if start == end && new_text.is_empty() {
                    continue; // no-op by contract; documents reject these
                }

                let result = doc
                    .apply_edit(TextEdit::replace(range(start, end), &new_text))
                    .unwrap();
                mirror = format!("{}{}{}", &mirror[..start], new_text, &mirror[end..]);

                // Mutation-time propagation is exact, per edit.
                assert_eq!(result.covering_old_range, range(start, end));
                assert_eq!(
                    result.covering_new_range,
                    range(start, start + new_text.len())
                );
                assert_eq!(
                    result.byte_delta,
                    new_text.len() as i64 - (end - start) as i64
                );
                assert_eq!(result.work.bytes_scanned, new_text.len() as u64);
                assert_eq!(result.work.full_rebuilds, 0);

                assert_matches_oracle(&doc, &mirror);
            }
        }
    }
}
