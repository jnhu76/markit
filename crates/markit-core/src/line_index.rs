//! Incremental line index (ADR-003).
//!
//! One full scan at load; local updates per edit: the newline entries
//! inside the changed range are dropped, the new newline entries are
//! inserted, and the entries after the edit are shifted by the byte
//! delta. A normal local edit never rebuilds the index from scratch.
//!
//! ## Representation
//!
//! `line_starts[i]` is the byte offset of line `i`'s first byte. Entries
//! are strictly increasing, `line_starts[0] == 0`, and
//! `line_starts.len() == line_count`. There is **no duplicate EOF
//! sentinel**: the position `total_len` belongs to the last line, which
//! removes the prototype's out-of-range last-line bug class by
//! construction (a document ending in `'\n'` has a final empty line,
//! e.g. `"ab\n"` → starts `[0, 3]`, `line_of(3) == 1`).
//!
//! ## Known residual cost
//!
//! The per-edit suffix shift is O(lines after the edit) — position
//! dependent, instrumented via [`LineIndexCounters`], and an accepted
//! known cost until real workloads justify a buffer redesign
//! (ADR-003; `docs/product/performance-invariants.md` Notes).

use std::ops::Range;

/// Work counters for one line-index update.
///
/// Structural, not wall-clock: the regression battery asserts these
/// instead of timing (INV-01/INV-08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineIndexCounters {
    /// Document bytes examined for this update. A local edit scans only
    /// the bytes of its own new text; the binary searches over line
    /// starts are O(log lines) and counted as entry touches instead.
    pub bytes_scanned: u64,
    /// Line-start entries removed + inserted + shifted.
    pub line_entries_touched: u64,
    /// 1 for a full rebuild (load / ReplaceDocument), else 0.
    pub full_rebuilds: u64,
}

impl LineIndexCounters {
    fn absorb(&mut self, other: &Self) {
        self.bytes_scanned += other.bytes_scanned;
        self.line_entries_touched += other.line_entries_touched;
        self.full_rebuilds += other.full_rebuilds;
    }
}

/// Incrementally maintained line index over a UTF-8 document.
///
/// The index is text-less by design: it stores only line starts and the
/// total byte length, so [`Document`](crate::Document) keeps single
/// ownership of the text. Internal representation (`Vec<usize>`) is
/// private and replaceable.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of each line's first byte; strictly increasing.
    line_starts: Vec<usize>,
    /// Total document byte length.
    total_len: usize,
    last: LineIndexCounters,
    cumulative: LineIndexCounters,
}

impl LineIndex {
    /// Builds the index with one full scan — the only full scan allowed
    /// (load time and ReplaceDocument).
    pub fn from_text(text: &str) -> Self {
        let mut line_starts = Vec::with_capacity(text.len() / 32 + 1);
        line_starts.push(0);
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        let counters = LineIndexCounters {
            bytes_scanned: text.len() as u64,
            line_entries_touched: line_starts.len() as u64,
            full_rebuilds: 1,
        };
        Self {
            line_starts,
            total_len: text.len(),
            last: counters,
            cumulative: counters,
        }
    }

    /// Applies `old_range -> new_text` incrementally.
    ///
    /// Precondition (debug-asserted; callers validate before mutating):
    /// `old_range` is ordered, in bounds for the indexed text, and on
    /// UTF-8 character boundaries.
    pub(crate) fn apply_edit(&mut self, old_range: Range<usize>, new_text: &str) {
        debug_assert!(old_range.start <= old_range.end && old_range.end <= self.total_len);

        let byte_delta = new_text.len() as i64 - (old_range.end - old_range.start) as i64;

        // Line starts strictly inside (start, end] are the successors of
        // newlines covered by the edit; they are removed. The start of
        // the line containing `old_range.start` survives untouched.
        let lo = self.line_starts.partition_point(|&s| s <= old_range.start);
        let hi = self.line_starts.partition_point(|&s| s <= old_range.end);
        let removed = (hi - lo) as u64;

        // New line starts from the newlines in the inserted text, in new
        // coordinates (only the changed region is scanned).
        let inserted_starts: Vec<usize> = new_text
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(i, _)| old_range.start + i + 1)
            .collect();
        let inserted = inserted_starts.len() as u64;

        let suffix_start = lo + inserted_starts.len();
        self.line_starts.splice(lo..hi, inserted_starts);

        // Shift the surviving suffix (old entries after the edit region)
        // into new coordinates. O(lines after the edit): known,
        // instrumented, position-dependent cost.
        let shifted = if byte_delta != 0 {
            let suffix = &mut self.line_starts[suffix_start..];
            for start in suffix.iter_mut() {
                *start = (*start as i64 + byte_delta) as usize;
            }
            suffix.len() as u64
        } else {
            0
        };

        self.total_len = (self.total_len as i64 + byte_delta) as usize;

        let counters = LineIndexCounters {
            bytes_scanned: new_text.len() as u64,
            line_entries_touched: removed + inserted + shifted,
            full_rebuilds: 0,
        };
        self.last = counters;
        self.cumulative.absorb(&counters);
    }

    /// Number of lines (a document always has at least one, possibly
    /// empty, line).
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Total document byte length.
    pub fn total_len(&self) -> usize {
        self.total_len
    }

    /// The line containing `offset`. `offset == total_len()` (EOF) maps
    /// to the last line. Panics if `offset > total_len()`.
    pub fn line_of(&self, offset: usize) -> usize {
        assert!(
            offset <= self.total_len,
            "offset {offset} beyond end of document ({} bytes)",
            self.total_len
        );
        self.line_starts.partition_point(|&s| s <= offset) - 1
    }

    /// First byte of `line`. Panics if `line >= line_count()`.
    pub fn line_start(&self, line: usize) -> usize {
        assert!(line < self.line_starts.len(), "line {line} out of range");
        self.line_starts[line]
    }

    /// End of `line` **including** its `'\n'` terminator when present —
    /// the next line's start, or the document end for the last line.
    /// Panics if `line >= line_count()`.
    pub fn line_end(&self, line: usize) -> usize {
        assert!(line < self.line_starts.len(), "line {line} out of range");
        if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            self.total_len
        }
    }

    /// End of `line`'s content, **excluding** the `'\n'` terminator. A
    /// non-last line always ends with exactly one `'\n'`, so this needs
    /// no text access. Panics if `line >= line_count()`.
    pub fn line_content_end(&self, line: usize) -> usize {
        assert!(line < self.line_starts.len(), "line {line} out of range");
        if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1] - 1
        } else {
            self.total_len
        }
    }

    /// Inclusive line span covering the byte `range`: the lines touched
    /// by any byte in `[start, end)`. An empty range maps to the single
    /// line containing its start. Panics on an out-of-bounds range.
    pub fn line_span_of_range(&self, range: Range<usize>) -> Range<usize> {
        assert!(
            range.start <= range.end && range.end <= self.total_len,
            "range {range:?} out of bounds for {} bytes",
            self.total_len
        );
        let first = self.line_of(range.start);
        let last = if range.end > range.start {
            self.line_of(range.end - 1)
        } else {
            first
        };
        first..last + 1
    }

    /// Counters for the most recent update.
    pub fn last_update_counters(&self) -> LineIndexCounters {
        self.last
    }

    /// Counters accumulated over the index's lifetime.
    pub fn cumulative_counters(&self) -> LineIndexCounters {
        self.cumulative
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full-scan oracle: the representation the prototype rebuilt on
    /// every edit. The differential tests must hold
    /// `incremental == full scan` after every operation.
    fn oracle_starts(text: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    fn oracle_line_of(starts: &[usize], offset: usize) -> usize {
        starts.partition_point(|&s| s <= offset) - 1
    }

    fn assert_same_as_oracle(index: &LineIndex, text: &str) {
        assert_eq!(
            index.starts(),
            &oracle_starts(text),
            "line starts diverge from full-scan oracle for {text:?}"
        );
        assert_eq!(index.total_len(), text.len());
        assert_eq!(index.line_count(), oracle_starts(text).len());
        for offset in 0..=text.len() {
            assert_eq!(
                index.line_of(offset),
                oracle_line_of(index.starts(), offset),
                "line_of({offset}) diverges for {text:?}"
            );
        }
        let expected_lines: Vec<&str> = text.split('\n').collect();
        for (line, expected) in expected_lines.iter().enumerate() {
            let start = index.line_start(line);
            let content_end = index.line_content_end(line);
            assert_eq!(
                &text[start..content_end],
                *expected,
                "line {line} content diverges for {text:?}"
            );
        }
        index.assert_invariants();
    }

    /// SplitMix64: deterministic, dependency-free randomness for the
    /// differential batteries.
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
    fn load_basics() {
        let cases: [(&str, Vec<usize>); 6] = [
            ("", vec![0]),
            ("a", vec![0]),
            ("a\n", vec![0, 2]),
            ("a\nb", vec![0, 2]),
            ("a\nb\n", vec![0, 2, 4]),
            ("\n\n", vec![0, 1, 2]),
        ];
        for (text, starts) in cases {
            let index = LineIndex::from_text(text);
            assert_eq!(index.starts(), &starts[..], "for {text:?}");
            assert_eq!(index.line_count(), starts.len());
            assert_eq!(index.total_len(), text.len());
            assert_eq!(index.last_update_counters().full_rebuilds, 1);
            assert_eq!(
                index.last_update_counters().bytes_scanned,
                text.len() as u64
            );
        }
    }

    #[test]
    fn eof_belongs_to_last_line() {
        // The prototype's sentinel bug class: line_of(len) must stay in
        // range for trailing-newline documents.
        for text in ["ab\ncd", "ab\n", "\n", "a", ""] {
            let index = LineIndex::from_text(text);
            let last = index.line_of(text.len());
            assert!(last < index.line_count(), "for {text:?}");
            // The EOF position always addresses the last line.
            assert_eq!(last, index.line_count() - 1);
        }
        // "ab\n" → lines "ab" and "" ; EOF is the empty last line.
        let index = LineIndex::from_text("ab\n");
        assert_eq!(index.line_count(), 2);
        assert_eq!(index.line_of(3), 1);
        assert_eq!(index.line_start(1), 3);
        assert_eq!(index.line_content_end(1), 3);
    }

    #[test]
    fn insert_without_newline_is_pure_counter_case() {
        let mut index = LineIndex::from_text("ab\ncd");
        index.apply_edit(2..3, "X");
        assert_same_as_oracle(&index, "abXcd");
        let c = index.last_update_counters();
        assert_eq!(c.bytes_scanned, 1);
        assert_eq!(c.full_rebuilds, 0);
        assert_eq!(
            c.line_entries_touched, 1,
            "removed the covered newline successor"
        );
    }

    #[test]
    fn insert_newlines_mid_document() {
        let mut index = LineIndex::from_text("ab\ncd");
        index.apply_edit(3..3, "X\nY\nZ");
        assert_same_as_oracle(&index, "ab\nX\nY\nZcd");
    }

    #[test]
    fn delete_through_line_boundary_merges_lines() {
        // Removing a '\n' merges two lines: the successor entry is removed.
        let mut index = LineIndex::from_text("ab\ncd");
        index.apply_edit(2..3, "");
        assert_same_as_oracle(&index, "abcd");
        assert_eq!(index.line_count(), 1);
    }

    #[test]
    fn delete_multiline_range() {
        let mut index = LineIndex::from_text("a\nb\nc\nd");
        index.apply_edit(0..6, "");
        assert_same_as_oracle(&index, "d");
        assert_eq!(index.line_count(), 1);
    }

    #[test]
    fn replace_multiline_with_multiline() {
        let mut index = LineIndex::from_text("l1\nl2\nl3\nl4");
        index.apply_edit(3..9, "X\nY\nZ");
        assert_same_as_oracle(&index, "l1\nX\nY\nZl4");
    }

    #[test]
    fn replace_with_empty_at_document_end() {
        let mut index = LineIndex::from_text("a\nb\n");
        // Delete the final '\n'.
        index.apply_edit(3..4, "");
        assert_same_as_oracle(&index, "a\nb");
    }

    #[test]
    fn edits_at_document_edges() {
        let mut index = LineIndex::from_text("");
        index.apply_edit(0..0, "hello");
        assert_same_as_oracle(&index, "hello");
        index.apply_edit(5..5, "\n");
        assert_same_as_oracle(&index, "hello\n");
        index.apply_edit(6..6, "world");
        assert_same_as_oracle(&index, "hello\nworld");
        index.apply_edit(0..0, "X\n");
        assert_same_as_oracle(&index, "X\nhello\nworld");
    }

    #[test]
    fn unicode_edits_stay_consistent() {
        let mut index = LineIndex::from_text("中文🙂\n文本");
        index.apply_edit(0..0, "头\n");
        assert_same_as_oracle(&index, "头\n中文🙂\n文本");
        // Delete whole scalars at the end.
        let text = "头\n中文🙂\n文本".to_string();
        index.apply_edit(text.len() - 6..text.len(), "");
        assert_same_as_oracle(&index, "头\n中文🙂\n");
    }

    #[test]
    fn line_span_of_range_cases() {
        let index = LineIndex::from_text("aa\nbb\ncc");
        assert_eq!(index.line_span_of_range(0..1), 0..1);
        assert_eq!(
            index.line_span_of_range(2..5),
            0..2,
            "\\n belongs to its line"
        );
        assert_eq!(
            index.line_span_of_range(3..3),
            1..2,
            "empty range at line start"
        );
        assert_eq!(index.line_span_of_range(3..7), 1..3);
        assert_eq!(index.line_span_of_range(7..7), 2..3, "EOF empty range");
    }

    #[test]
    fn counters_accumulate() {
        let mut index = LineIndex::from_text("a\nb");
        let after_load = index.cumulative_counters();
        index.apply_edit(0..0, "z");
        assert_eq!(
            index.cumulative_counters().bytes_scanned,
            after_load.bytes_scanned + 1
        );
        assert_eq!(index.cumulative_counters().full_rebuilds, 1);
        assert_eq!(index.last_update_counters().full_rebuilds, 0);
    }

    #[test]
    fn local_edit_never_scans_document() {
        // INV-01, structural: a local edit scans only its own new text.
        let big = "l\n".repeat(50_000);
        let mut index = LineIndex::from_text(&big);
        let mid = big.len() / 2 + 1; // inside the second half
        index.apply_edit(mid..mid, "NEW");
        assert_eq!(index.last_update_counters().bytes_scanned, 3);
        assert_eq!(index.last_update_counters().full_rebuilds, 0);
        assert_same_as_oracle(&index, &format!("{}NEW{}", &big[..mid], &big[mid..]));
    }

    #[test]
    fn randomized_differential_against_full_scan_oracle() {
        for seed in 1..=8u64 {
            let mut rng = Rng::new(seed);
            let initial_len = 1 + rng.below(120);
            let mut text = random_string(&mut rng, initial_len);
            let mut index = LineIndex::from_text(&text);
            assert_same_as_oracle(&index, &text);

            for _ in 0..400 {
                let bounds = char_boundaries(&text);
                let start_idx = rng.below(bounds.len());
                let start = bounds[start_idx];
                let end_idx = start_idx + rng.below(bounds.len() - start_idx);
                let end = bounds[end_idx];
                let new_len = rng.below(8);
                let new_text = random_string(&mut rng, new_len);

                let expected: String = format!("{}{}{}", &text[..start], new_text, &text[end..]);
                index.apply_edit(start..end, &new_text);
                text = expected;

                assert_same_as_oracle(&index, &text);
                let c = index.last_update_counters();
                assert_eq!(c.bytes_scanned, new_text.len() as u64);
                assert_eq!(c.full_rebuilds, 0);
            }
        }
    }

    #[test]
    fn randomized_medium_documents() {
        for seed in 100..=101u64 {
            let mut rng = Rng::new(seed);
            let mut text = random_string(&mut rng, 5_000);
            let mut index = LineIndex::from_text(&text);
            for _ in 0..200 {
                let bounds = char_boundaries(&text);
                let start_idx = rng.below(bounds.len());
                let start = bounds[start_idx];
                let end_idx = start_idx + rng.below(bounds.len() - start_idx);
                let end = bounds[end_idx];
                let new_len = rng.below(6);
                let new_text = random_string(&mut rng, new_len);
                let expected: String = format!("{}{}{}", &text[..start], new_text, &text[end..]);
                index.apply_edit(start..end, &new_text);
                text = expected;
            }
            // Full oracle equality at the end (medium docs: equality per
            // op would dominate runtime; the small-doc battery above
            // already checks per-op).
            assert_eq!(index.starts(), &oracle_starts(&text)[..]);
            assert_eq!(index.total_len(), text.len());
            index.assert_invariants();
        }
    }

    #[test]
    fn suffix_shift_is_the_known_position_dependent_cost() {
        // Documented residual: begin-position edits shift O(lines after).
        // Three-quarter-position edits shift fewer entries. Both are
        // allowed; the test pins the asymmetry so a future regression is
        // visible.
        let text = "l\n".repeat(10_000);
        let mut index = LineIndex::from_text(&text);
        let late = text.len() * 3 / 4 + 1;

        index.apply_edit(0..0, "z");
        let begin_touched = index.last_update_counters().line_entries_touched;
        assert!(begin_touched >= 9_000, "begin edit shifts ~all entries");

        index.apply_edit(late..late, "z");
        let late_touched = index.last_update_counters().line_entries_touched;
        assert!(late_touched < begin_touched / 2);
        assert!(late_touched < index.line_count() as u64);
    }
}

#[cfg(test)]
impl LineIndex {
    fn starts(&self) -> &[usize] {
        &self.line_starts
    }

    fn assert_invariants(&self) {
        assert!(!self.line_starts.is_empty());
        assert_eq!(self.line_starts[0], 0);
        for w in self.line_starts.windows(2) {
            assert!(w[0] < w[1], "line starts must be strictly increasing");
        }
        assert!(*self.line_starts.last().unwrap() <= self.total_len);
    }
}
