//! Pure lexical seam: line classification and delimiter scanning.
//!
//! Small, side-effect-free functions from `&str` to shapes. The block
//! parser and inline parser are built entirely on these, which keeps the
//! classification rules in one place and leaves a clean seam for a future
//! SIMD line classifier.
//!
//! SIMD status: **DEFERRED — no profiler evidence yet.** Replacing these
//! functions with SIMD is a measured intervention, not a default
//! (`AGENTS.md` §3; the forbidden list in the P0-02 task explicitly
//! excludes SIMD until evidence exists).

use std::ops::Range;

use crate::markdown::block::ListSignature;
use crate::markdown::state::FenceChar;

/// Count of leading spaces. Tabs are not spaces and never part of an
/// opener prefix (contract D4).
pub(crate) fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

/// Whether a line's content is empty or only spaces/tabs (contract §2).
pub(crate) fn is_blank_line(line: &str) -> bool {
    line.bytes().all(|b| b == b' ' || b == b'\t')
}

fn skip_spaces_tabs(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

/// ATX heading shape (contract §6.3). Ranges are line-relative bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AtxShape {
    /// Heading level, 1–6.
    pub level: u8,
    /// Content range: after the opener and leading whitespace, before an
    /// optional closing `#` sequence. May be empty.
    pub content: Range<usize>,
}

/// Whether `line` opens an ATX heading, and its shape (contract §6.3).
pub(crate) fn atx_heading(line: &str) -> Option<AtxShape> {
    let indent = leading_spaces(line);
    if indent > 3 {
        return None;
    }
    let bytes = line.as_bytes();
    let mut i = indent;
    let mut hashes = 0usize;
    while i < bytes.len() && bytes[i] == b'#' {
        hashes += 1;
        i += 1;
    }
    // 1–6 `#`, then a space, tab, or end of line. The run is maximal, so
    // content can never begin with `#` (this is what makes the closing-
    // sequence rule in the contract well-defined).
    if !(1..=6).contains(&hashes) {
        return None;
    }
    match bytes.get(i) {
        None | Some(b' ') | Some(b'\t') => {}
        _ => return None,
    }

    let content_start = skip_spaces_tabs(bytes, i);
    let mut content_end = bytes.len();
    while content_end > content_start
        && (bytes[content_end - 1] == b' ' || bytes[content_end - 1] == b'\t')
    {
        content_end -= 1;
    }
    // Closing sequence: a trailing `#` run inside the content, preceded
    // by a space or tab — explicitly, or implicitly because the run is
    // the whole content and leading whitespace was stripped after the
    // opener (`# #`). An escaped `\#` is preceded by `\` and therefore
    // never a closer. (The opener run is maximal, so content starting
    // with `#` can only arise through that stripped-whitespace case.)
    let mut closer_start = content_end;
    while closer_start > content_start && bytes[closer_start - 1] == b'#' {
        closer_start -= 1;
    }
    let closer_valid = closer_start < content_end
        && (closer_start == content_start && content_start > i
            || closer_start > content_start
                && (bytes[closer_start - 1] == b' ' || bytes[closer_start - 1] == b'\t'));
    let content_end = if closer_valid {
        // The closer run — and any whitespace before it — are not content.
        let mut end = closer_start;
        while end > content_start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
            end -= 1;
        }
        end
    } else {
        content_end
    };
    Some(AtxShape {
        level: hashes as u8,
        content: content_start..content_end,
    })
}

/// Fence opener shape (contract §6.7). Ranges are line-relative bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FenceOpenShape {
    /// Backtick or tilde.
    pub fence_char: FenceChar,
    /// Length of the opening run (minimum closing length).
    pub fence_len: usize,
    /// Trimmed info-string range, or None when empty. A backtick fence
    /// whose info contains a backtick is not an opener at all.
    pub info: Option<Range<usize>>,
}

/// Whether `line` opens a fenced code block, and the opener's shape
/// (contract §6.7, CommonMark 0.31.2 rules adopted verbatim).
pub(crate) fn fence_opener(line: &str) -> Option<FenceOpenShape> {
    let indent = leading_spaces(line);
    if indent > 3 {
        return None;
    }
    let bytes = line.as_bytes();
    let fence_char = match bytes.get(indent) {
        Some(b'`') => FenceChar::Backtick,
        Some(b'~') => FenceChar::Tilde,
        _ => return None,
    };
    let want = fence_char.as_char() as u8;
    let mut i = indent;
    let mut fence_len = 0usize;
    while i < bytes.len() && bytes[i] == want {
        fence_len += 1;
        i += 1;
    }
    if fence_len < 3 {
        return None;
    }
    let info_start = skip_spaces_tabs(bytes, i);
    let mut info_end = bytes.len();
    while info_end > info_start && (bytes[info_end - 1] == b' ' || bytes[info_end - 1] == b'\t') {
        info_end -= 1;
    }
    if fence_char == FenceChar::Backtick && line[info_start..info_end].contains('`') {
        return None;
    }
    let info = (info_start < info_end).then_some(info_start..info_end);
    Some(FenceOpenShape {
        fence_char,
        fence_len,
        info,
    })
}

/// Whether `line` closes a fence opened with `fence_char`/`fence_len`:
/// indent <= 3, same character, length >= `fence_len`, then only spaces
/// and tabs to end of line (contract §6.7).
pub(crate) fn is_fence_closer(line: &str, fence_char: FenceChar, fence_len: usize) -> bool {
    let indent = leading_spaces(line);
    if indent > 3 {
        return false;
    }
    let bytes = line.as_bytes();
    let want = fence_char.as_char() as u8;
    let mut i = indent;
    let mut len = 0usize;
    while i < bytes.len() && bytes[i] == want {
        len += 1;
        i += 1;
    }
    if len < fence_len {
        return false;
    }
    skip_spaces_tabs(bytes, i) == bytes.len()
}

/// Blockquote line shape: the content after `>` and at most one space
/// (contract §6.4). Ranges are line-relative bytes; empty content is
/// possible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuoteShape {
    /// Content range on this line; may be empty.
    pub content: Range<usize>,
}

/// Whether `line` is a `>` line (opener or continuation), and where its
/// content starts (contract §6.4).
pub(crate) fn blockquote_line(line: &str) -> Option<QuoteShape> {
    let indent = leading_spaces(line);
    if indent > 3 {
        return None;
    }
    let bytes = line.as_bytes();
    if bytes.get(indent) != Some(&b'>') {
        return None;
    }
    let mut content_start = indent + 1;
    if bytes.get(content_start) == Some(&b' ') {
        content_start += 1;
    }
    Some(QuoteShape {
        content: content_start..bytes.len(),
    })
}

/// List marker line shape (contract §6.5). Ranges are line-relative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListMarkerShape {
    /// Bullet char or ordered delimiter.
    pub signature: ListSignature,
    /// The item's number, ordered lists only.
    pub number: Option<u32>,
    /// The marker bytes (`-`, `1.`, …), excluding following spaces.
    pub marker: Range<usize>,
    /// Byte index where item content starts (== line length for an
    /// empty item).
    pub content_start: usize,
}

/// Whether `line` starts with a list marker, and the marker's shape
/// (contract §6.5): bullet `-`/`+`/`*` or 1–9 digits plus `.`/`)`,
/// followed by a space, tab, or end of line; all spaces/tabs after the
/// marker are stripped from the content.
pub(crate) fn list_marker(line: &str) -> Option<ListMarkerShape> {
    let indent = leading_spaces(line);
    if indent > 3 {
        return None;
    }
    let bytes = line.as_bytes();
    let marker_start = indent;

    let bullet = matches!(bytes.get(indent), Some(b'-') | Some(b'+') | Some(b'*'));
    if bullet {
        let marker_end = indent + 1;
        match bytes.get(marker_end) {
            None | Some(b' ') | Some(b'\t') => {}
            _ => return None, // `-text` is not a marker
        }
        let content_start = skip_spaces_tabs(bytes, marker_end);
        return Some(ListMarkerShape {
            signature: ListSignature::Bullet {
                marker: bytes[indent] as char,
            },
            number: None,
            marker: marker_start..marker_end,
            content_start,
        });
    }

    let mut digits = 0usize;
    while digits < 9
        && bytes
            .get(marker_start + digits)
            .is_some_and(u8::is_ascii_digit)
    {
        digits += 1;
    }
    // 10+ digits is not an ordered marker (CommonMark caps markers at 9).
    if digits == 0
        || bytes
            .get(marker_start + digits)
            .is_some_and(u8::is_ascii_digit)
    {
        return None;
    }
    let delimiter = match bytes.get(marker_start + digits) {
        Some(b'.') => '.',
        Some(b')') => ')',
        _ => return None, // `1-x` or bare digits are not markers
    };
    let marker_end = marker_start + digits + 1;
    match bytes.get(marker_end) {
        None | Some(b' ') | Some(b'\t') => {}
        _ => return None,
    }
    let number: u32 = line[marker_start..marker_start + digits].parse().ok()?;
    let content_start = skip_spaces_tabs(bytes, marker_end);
    Some(ListMarkerShape {
        signature: ListSignature::Ordered { delimiter },
        number: Some(number),
        marker: marker_start..marker_end,
        content_start,
    })
}

// ---------------------------------------------------------------------------
// Inline lexical seam (contract §7)
// ---------------------------------------------------------------------------

/// Length of the backtick run starting exactly at `at`, or 0.
pub(crate) fn backtick_run_at(text: &str, at: usize) -> usize {
    let bytes = text.as_bytes();
    let mut len = 0usize;
    while bytes.get(at + len) == Some(&b'`') {
        len += 1;
    }
    len
}

/// Pre-scanned index of backtick runs for O(log n) closer lookup.
///
/// Built once per inline run; eliminates the quadratic repeated-suffix
/// scan that naive scanning performs when many runs of different lengths
/// have no matching closer. Construction is currently sort-based
/// (`entries.sort()`), so build is O(R log R) in the number of runs R —
/// not linear by construction. If a real profiler ever shows the sort
/// dominating, switch to per-length position buckets or next-same-length
/// links; not worth the complexity until measured.
pub(crate) struct BacktickIndex {
    /// `(run_length, start_position)` entries, sorted by
    /// `(run_length, start_position)`.
    entries: Vec<(usize, usize)>,
}

impl BacktickIndex {
    /// Scans `text` once, recording every backtick run's length and
    /// position, then sorts by `(len, pos)` for binary-search lookup —
    /// O(R log R) in the number of runs.
    pub(crate) fn build(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut entries = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'`' {
                let start = i;
                let mut len = 0;
                while i < bytes.len() && bytes[i] == b'`' {
                    len += 1;
                    i += 1;
                }
                entries.push((len, start));
            } else {
                i += 1;
            }
        }
        entries.sort();
        Self { entries }
    }

    /// Finds the start of the first backtick run of exactly `len` at or
    /// after `from`. O(log n) via binary search over the pre-scanned
    /// entries.
    pub(crate) fn find(&self, from: usize, len: usize) -> Option<usize> {
        // Binary search for the first entry with (len, pos >= from).
        let idx = self
            .entries
            .partition_point(|&(l, p)| l < len || (l == len && p < from));
        self.entries
            .get(idx)
            .filter(|&&(l, _)| l == len)
            .map(|&(_, p)| p)
    }
}

/// An unescaped emphasis delimiter run starting exactly at `at`:
/// `(character, length)`, or `None` when `at` does not start one.
pub(crate) fn delimiter_run_at(text: &str, at: usize) -> Option<(u8, usize)> {
    let bytes = text.as_bytes();
    let &ch = bytes.get(at)?;
    if ch != b'*' && ch != b'_' {
        return None;
    }
    let mut len = 1usize;
    while bytes.get(at + len) == Some(&ch) {
        len += 1;
    }
    Some((ch, len))
}

/// Whether `c` is ASCII punctuation (the escapable set, contract §7.1).
pub(crate) fn is_ascii_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_and_indent() {
        assert_eq!(leading_spaces("  - x"), 2);
        assert_eq!(leading_spaces("\t- x"), 0, "tab is not indent (D4)");
        assert!(is_blank_line(""));
        assert!(is_blank_line(" \t "));
        assert!(!is_blank_line(" x"));
    }

    #[test]
    fn atx_shapes() {
        let h = atx_heading("# foo").unwrap();
        assert_eq!((h.level, h.content.clone()), (1, 2..5));
        let h = atx_heading("  ### foo ###").unwrap();
        assert_eq!((h.level, h.content.clone()), (3, 6..9));
        let h = atx_heading("######").unwrap();
        assert_eq!((h.level, h.content.clone()), (6, 6..6));
        let h = atx_heading("# #").unwrap();
        assert_eq!(h.content, 2..2, "lone closing # empties the heading");
        assert!(
            atx_heading("####### x").is_none(),
            "7 hashes is a paragraph"
        );
        assert!(
            atx_heading("#5 bolt").is_none(),
            "# must be followed by space"
        );
        assert!(atx_heading("    # x").is_none(), "indent 4 never opens");
        assert!(atx_heading("\t# x").is_none(), "leading tab never opens");
        let h = atx_heading("# foo \\#").unwrap();
        assert_eq!(h.content, 2..8, "escaped # is not a closer");
        let h = atx_heading("# foo#").unwrap();
        assert_eq!(h.content, 2..6, "closer needs a preceding space");
    }

    #[test]
    fn fence_openers() {
        let f = fence_opener("```rust").unwrap();
        assert_eq!(f.fence_char, FenceChar::Backtick);
        assert_eq!(f.fence_len, 3);
        assert_eq!(f.info, Some(3..7));
        assert!(
            fence_opener("``` foo ` bar").is_none(),
            "backtick in backtick info"
        );
        assert!(
            fence_opener("~~~ a ` b").is_some(),
            "tilde info is unrestricted"
        );
        assert_eq!(fence_opener("````").unwrap().fence_len, 4);
        assert!(fence_opener("``x").is_none());
        assert!(fence_opener("  ~~~").is_some());
        assert!(fence_opener("    ```").is_none());
    }

    #[test]
    fn fence_closers() {
        assert!(is_fence_closer("```", FenceChar::Backtick, 3));
        assert!(is_fence_closer("  `````", FenceChar::Backtick, 4));
        assert!(is_fence_closer("~~~ ", FenceChar::Tilde, 3));
        assert!(!is_fence_closer("``x", FenceChar::Backtick, 3));
        assert!(!is_fence_closer("~~", FenceChar::Tilde, 3), "too short");
        assert!(
            !is_fence_closer("```x", FenceChar::Backtick, 3),
            "trailing text"
        );
        assert!(
            !is_fence_closer("~~~", FenceChar::Backtick, 3),
            "wrong char"
        );
        assert!(
            !is_fence_closer("    ```", FenceChar::Backtick, 3),
            "indent 4"
        );
    }

    #[test]
    fn quote_lines() {
        assert_eq!(blockquote_line("> a").unwrap().content, 2..3);
        assert_eq!(blockquote_line(">a").unwrap().content, 1..2);
        assert_eq!(
            blockquote_line(">   a").unwrap().content,
            2..5,
            "one space max"
        );
        assert_eq!(blockquote_line(">").unwrap().content, 1..1);
        assert_eq!(blockquote_line("  > x").unwrap().content, 4..5);
        assert!(blockquote_line("    > x").is_none());
        assert!(blockquote_line("x > y").is_none());
    }

    #[test]
    fn list_markers() {
        let m = list_marker("- x").unwrap();
        assert_eq!(m.signature, ListSignature::Bullet { marker: '-' });
        assert_eq!(m.marker, 0..1);
        assert_eq!(m.content_start, 2);
        assert_eq!(
            list_marker("-      x").unwrap().content_start,
            7,
            "strip all spaces"
        );
        assert_eq!(list_marker("-").unwrap().content_start, 1, "empty item");
        assert!(list_marker("-x").is_none());
        assert!(list_marker("+ x").is_some());
        assert!(list_marker("* x").is_some());

        let m = list_marker("1. x").unwrap();
        assert_eq!(m.signature, ListSignature::Ordered { delimiter: '.' });
        assert_eq!(m.number, Some(1));
        assert_eq!(m.marker, 0..2);
        let m = list_marker("  42) y").unwrap();
        assert_eq!(m.number, Some(42));
        assert_eq!(m.marker, 2..5);
        assert!(list_marker("1.x").is_none());
        assert!(
            list_marker("1234567890. x").is_none(),
            "10 digits is not a marker"
        );
        assert!(list_marker("123456789. x").is_some(), "9 digits is");
        assert!(list_marker("    - x").is_none(), "indent 4 never opens");
    }

    #[test]
    fn inline_scans() {
        assert_eq!(backtick_run_at("a``b", 1), 2);
        let idx = BacktickIndex::build("a` `` b``c");
        assert_eq!(idx.find(0, 2), Some(3), "first run of exactly 2");
        let idx = BacktickIndex::build("```x``");
        assert_eq!(idx.find(0, 2), Some(4));
        let idx = BacktickIndex::build("no ticks");
        assert_eq!(idx.find(0, 1), None);
        assert_eq!(delimiter_run_at("**a", 0), Some((b'*', 2)));
        assert_eq!(delimiter_run_at("a", 0), None);
    }
}
