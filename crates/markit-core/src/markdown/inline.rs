//! Inline IR: source-referenced inline nodes for a block's text runs
//! (contract §7).
//!
//! Parsing is block-local, single-pass, no frameworks, no SIMD: a scan
//! that recognizes escapes, code spans, and links, then a simplified
//! CommonMark emphasis pairing (contract §7.3; deviations D10/D11), then
//! an iterative tree assembly. Nodes reference source bytes only — text
//! materialization is `snapshot.slice(range)`; presentation rules
//! (code-span space stripping, newline conversion) belong downstream.

use crate::markdown::block::BlockDetail;
use crate::markdown::block::BlockKind;
use crate::markdown::block::BlockRecord;
use crate::markdown::lex::{
    backtick_run_at, delimiter_run_at, find_backtick_string, is_ascii_punctuation,
};
use crate::position::SourceRange;
use crate::snapshot::DocumentSnapshot;

/// The inline IR of one block: the parsed nodes of each independent
/// inline run, with the run's source range.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InlineIr {
    /// One entry per inline run (contract §7.6); empty for blank and
    /// fenced-code blocks.
    pub runs: Vec<InlineRun>,
}

/// One inline run: an independent parse unit (a paragraph, a heading's
/// content, a quote line, a list item). Delimiters never pair across
/// runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineRun {
    /// The run's source range (interior newlines included).
    pub range: SourceRange,
    /// Parsed nodes, in order; node ranges tile the run except for
    /// structural gaps (none today — markers live outside runs).
    pub nodes: Vec<InlineNode>,
}

/// A source-referenced inline node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineNode {
    /// Literal text (escape sequences included: `\*` is a two-byte Text
    /// covering backslash and `*`; presentation strips the backslash).
    Text {
        /// Source bytes of this text.
        range: SourceRange,
    },
    /// Inline code: the full span including both delimiter runs.
    Code {
        /// Source bytes of the whole code span.
        range: SourceRange,
    },
    /// `*…*` / `_…_`.
    Emphasis {
        /// Source bytes from opener to closer.
        range: SourceRange,
        /// Parsed content.
        children: Vec<InlineNode>,
    },
    /// `**…**` / `__…__`.
    Strong {
        /// Source bytes from opener to closer.
        range: SourceRange,
        /// Parsed content.
        children: Vec<InlineNode>,
    },
    /// Inline link `[text](dest)` with optional title.
    Link {
        /// The whole link, brackets and parentheses included.
        range: SourceRange,
        /// Parsed link text.
        children: Vec<InlineNode>,
        /// The destination bytes (excluding `<>` wrappers).
        destination: SourceRange,
        /// The title bytes (excluding quotes), when present.
        title: Option<SourceRange>,
    },
}

impl InlineIr {
    /// The same IR moved by `delta` bytes — used when untouched blocks
    /// shift; their runs' bytes are unchanged by construction, so the
    /// tree shape is preserved as-is.
    pub(crate) fn shifted(&self, delta: i64) -> Self {
        Self {
            runs: self
                .runs
                .iter()
                .map(|r| InlineRun {
                    range: shift(r.range, delta),
                    nodes: shift_nodes(&r.nodes, delta),
                })
                .collect(),
        }
    }
}

impl InlineNode {
    /// Stable node-kind name (oracle comparisons, diagnostics).
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Code { .. } => "code",
            Self::Emphasis { .. } => "em",
            Self::Strong { .. } => "strong",
            Self::Link { .. } => "link",
        }
    }
}

/// Parses `text` (one inline run, at absolute byte offset `base`) into
/// nodes. Every returned range is absolute.
pub(crate) fn parse_run(text: &str, base: usize) -> Vec<InlineNode> {
    let toks = scan_tokens(text);
    let mut delims: Vec<DelimRun> = toks
        .iter()
        .filter_map(|t| match t {
            Tok::Delim(d) => Some(d.clone()),
            _ => None,
        })
        .collect();
    for delim in &mut delims {
        classify_flanking(text, delim);
    }
    let pairs = pair_emphasis(&mut delims);
    assemble(base, &toks, &delims, &pairs)
}

/// Computes and stores a block's inline IR from its detail runs.
/// Returns whether the block has inline runs at all (the
/// `inline_blocks_reparsed` counter input).
pub(crate) fn attach_inline(record: &mut BlockRecord, snapshot: &DocumentSnapshot<'_>) -> bool {
    let runs = runs_for(record);
    if runs.is_empty() {
        record.inline = InlineIr::default();
        return false;
    }
    let mut parsed = Vec::with_capacity(runs.len());
    for range in runs {
        let text = snapshot.slice(range).into_owned();
        parsed.push(InlineRun {
            range,
            nodes: parse_run(&text, range.start.as_usize()),
        });
    }
    record.inline = InlineIr { runs: parsed };
    true
}

/// The independent inline runs of a block (contract §7.6).
pub(crate) fn runs_for(record: &BlockRecord) -> Vec<SourceRange> {
    match (&record.kind, &record.detail) {
        (BlockKind::Paragraph, _) => vec![record.source_range],
        (BlockKind::Heading, BlockDetail::Heading { content, .. }) => {
            if content.is_empty() {
                Vec::new()
            } else {
                vec![*content]
            }
        }
        (BlockKind::BlockQuote, BlockDetail::BlockQuote { content }) => {
            content.iter().copied().filter(|r| !r.is_empty()).collect()
        }
        (BlockKind::UnorderedList | BlockKind::OrderedList, BlockDetail::List { items, .. }) => {
            items.iter().map(|item| item.content).collect()
        }
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Phase 1: token scan
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Tok {
    /// Literal text (relative range).
    Text(std::ops::Range<usize>),
    /// Inline code span, delimiters included (relative range).
    Code(std::ops::Range<usize>),
    Link(LinkTok),
    Delim(DelimRun),
}

#[derive(Clone, Debug)]
struct LinkTok {
    /// Whole construct, `[` through `)`.
    full: std::ops::Range<usize>,
    /// Destination bytes (no `<>` wrapper).
    dest: std::ops::Range<usize>,
    /// Title bytes (no quotes).
    title: Option<std::ops::Range<usize>>,
    /// Parsed link text (absolute ranges).
    children: Vec<InlineNode>,
}

#[derive(Clone, Debug)]
struct DelimRun {
    ch: u8,
    start: usize,
    len: usize,
    can_open: bool,
    can_close: bool,
    /// Bytes taken from the left side (closer use).
    used_left: usize,
    /// Bytes taken from the right side (opener use).
    used_right: usize,
}

impl DelimRun {
    fn remaining(&self) -> usize {
        self.len - self.used_left - self.used_right
    }
}

fn is_ws(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn scan_tokens(text: &str) -> Vec<Tok> {
    let bytes = text.as_bytes();
    let mut toks = Vec::new();
    let mut text_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let flush_text = |text_start: &mut usize, end: usize, toks: &mut Vec<Tok>| {
            if end > *text_start {
                toks.push(Tok::Text(*text_start..end));
            }
            *text_start = end;
        };
        match bytes[i] {
            b'\\' => {
                // Escapes stay inside text; an escaped delimiter is never
                // scanned as one (contract §7.1).
                i += if i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1] as char) {
                    2
                } else {
                    1
                };
            }
            b'`' => {
                let run = backtick_run_at(text, i);
                match find_backtick_string(text, i + run, run) {
                    Some(close) => {
                        flush_text(&mut text_start, i, &mut toks);
                        toks.push(Tok::Code(i..close + run));
                        i = close + run;
                        text_start = i;
                    }
                    None => i += run, // unmatched: literal text
                }
            }
            b'[' => match try_link(text, i) {
                Some(link) => {
                    flush_text(&mut text_start, i, &mut toks);
                    i = link.full.end;
                    toks.push(Tok::Link(link));
                    text_start = i;
                }
                None => i += 1, // literal '['; rescan finds inner constructs
            },
            b'*' | b'_' => {
                let (_, len) = delimiter_run_at(text, i).expect("delimiter run");
                flush_text(&mut text_start, i, &mut toks);
                toks.push(Tok::Delim(DelimRun {
                    ch: bytes[i],
                    start: i,
                    len,
                    can_open: false,
                    can_close: false,
                    used_left: 0,
                    used_right: 0,
                }));
                i += len;
                text_start = i;
            }
            _ => i += 1,
        }
    }
    if bytes.len() > text_start {
        toks.push(Tok::Text(text_start..bytes.len()));
    }
    toks
}

fn try_link(text: &str, open: usize) -> Option<LinkTok> {
    let bytes = text.as_bytes();
    // Matching ']' with code spans skipped and escapes honored; balanced
    // brackets nest.
    let mut j = open + 1;
    let mut depth = 1usize;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' if j + 1 < bytes.len() && is_ascii_punctuation(bytes[j + 1] as char) => j += 2,
            b'`' => {
                let run = backtick_run_at(text, j);
                j = match find_backtick_string(text, j + run, run) {
                    Some(close) => close + run,
                    None => j + run,
                };
            }
            b'[' => {
                depth += 1;
                j += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            _ => j += 1,
        }
    }
    if j >= bytes.len() || bytes.get(j + 1) != Some(&b'(') {
        return None;
    }

    let mut k = j + 2;
    while k < bytes.len() && is_ws(bytes[k]) {
        k += 1;
    }
    // Destination.
    let dest = if bytes.get(k) == Some(&b'<') {
        let mut m = k + 1;
        while m < bytes.len() {
            match bytes[m] {
                b'\\' if m + 1 < bytes.len() && is_ascii_punctuation(bytes[m + 1] as char) => {
                    m += 2
                }
                b'>' => break,
                b'<' | b'\n' => return None,
                _ => m += 1,
            }
        }
        if m >= bytes.len() {
            return None;
        }
        let range = k + 1..m;
        k = m + 1;
        range
    } else {
        let mut m = k;
        let mut parens = 0i32;
        while m < bytes.len() {
            let b = bytes[m];
            if is_ws(b) {
                break; // no whitespace in a bare destination, ever
            }
            if b == b'\\' && m + 1 < bytes.len() && is_ascii_punctuation(bytes[m + 1] as char) {
                m += 2;
                continue;
            }
            if b == b')' && parens == 0 {
                break; // the link's closing paren
            }
            if b == b'(' {
                parens += 1;
            } else if b == b')' {
                parens -= 1;
            }
            m += 1;
        }
        if parens != 0 || m == k {
            return None; // unbalanced or empty bare destination
        }
        let range = k..m;
        k = m;
        range
    };

    // Optional title, separated by whitespace (newlines allowed).
    let title_start = k;
    while k < bytes.len() && is_ws(bytes[k]) {
        k += 1;
    }
    let title = if k > title_start && k < bytes.len() {
        match bytes[k] {
            b'"' | b'\'' => {
                let quote = bytes[k];
                let mut m = k + 1;
                while m < bytes.len() && bytes[m] != quote {
                    m += if bytes[m] == b'\\'
                        && m + 1 < bytes.len()
                        && is_ascii_punctuation(bytes[m + 1] as char)
                    {
                        2
                    } else {
                        1
                    };
                }
                if m >= bytes.len() {
                    return None;
                }
                let range = k + 1..m;
                k = m + 1;
                Some(range)
            }
            b'(' => {
                let mut m = k + 1;
                let mut parens = 0i32;
                while m < bytes.len() {
                    match bytes[m] {
                        b'\\'
                            if m + 1 < bytes.len()
                                && is_ascii_punctuation(bytes[m + 1] as char) =>
                        {
                            m += 2
                        }
                        b'(' => {
                            parens += 1;
                            m += 1;
                        }
                        b')' => {
                            if parens == 0 {
                                break;
                            }
                            parens -= 1;
                            m += 1;
                        }
                        _ => m += 1,
                    }
                }
                if m >= bytes.len() {
                    return None;
                }
                let range = k + 1..m;
                k = m + 1;
                Some(range)
            }
            _ => None,
        }
    } else {
        None
    };

    while k < bytes.len() && is_ws(bytes[k]) {
        k += 1;
    }
    if bytes.get(k) != Some(&b')') {
        return None;
    }

    let full = open..k + 1;
    let inner = open + 1..j;
    let children = parse_run(&text[inner.clone()], open + 1);
    if children.iter().any(contains_link) {
        return None; // links do not nest at any level (contract §7.4)
    }
    Some(LinkTok {
        full,
        dest,
        title,
        children,
    })
}

fn contains_link(node: &InlineNode) -> bool {
    match node {
        InlineNode::Link { .. } => true,
        InlineNode::Emphasis { children, .. } | InlineNode::Strong { children, .. } => {
            children.iter().any(contains_link)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Phase 2: flanking (contract §7.3, deviation D11: whitespace classes only)
// ---------------------------------------------------------------------------

fn prev_char(text: &str, byte_idx: usize) -> Option<char> {
    if byte_idx == 0 {
        None
    } else {
        text[..byte_idx].chars().next_back()
    }
}

fn next_char(text: &str, byte_idx: usize) -> Option<char> {
    text.get(byte_idx..).and_then(|rest| rest.chars().next())
}

fn classify_flanking(text: &str, delim: &mut DelimRun) {
    let after = next_char(text, delim.start + delim.len);
    let before = prev_char(text, delim.start);
    let left_flanking = after.is_some_and(|c| !c.is_whitespace());
    let right_flanking = before.is_some_and(|c| !c.is_whitespace());
    if delim.ch == b'*' {
        delim.can_open = left_flanking;
        delim.can_close = right_flanking;
    } else {
        // '_': no intraword emphasis — open only at a (virtual) start
        // boundary, close only at a (virtual) end boundary.
        delim.can_open = left_flanking && before.is_none_or(|c| c.is_whitespace());
        delim.can_close = right_flanking && after.is_none_or(|c| c.is_whitespace());
    }
}

// ---------------------------------------------------------------------------
// Phase 3: emphasis pairing (CommonMark's algorithm minus the rule of 3)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Pairing {
    /// Opener bytes consumed by this pair (relative).
    open: std::ops::Range<usize>,
    /// Closer bytes consumed by this pair (relative).
    close: std::ops::Range<usize>,
    strong: bool,
}

fn pair_emphasis(delims: &mut [DelimRun]) -> Vec<Pairing> {
    let mut pairs = Vec::new();
    for ci in 0..delims.len() {
        loop {
            if delims[ci].remaining() == 0 || !delims[ci].can_close {
                break;
            }
            let Some(oi) = (0..ci).rev().find(|&o| {
                delims[o].ch == delims[ci].ch && delims[o].can_open && delims[o].remaining() > 0
            }) else {
                break;
            };
            let use_len = if delims[oi].remaining() >= 2 && delims[ci].remaining() >= 2 {
                2
            } else {
                1
            };
            let strong = use_len == 2;
            // Openers allocate from the right of their run (content
            // side), closers from the left — inner pairs therefore take
            // the bytes adjacent to the content (`***x***` = em > strong).
            let open_start = delims[oi].start + delims[oi].len - delims[oi].used_right - use_len;
            delims[oi].used_right += use_len;
            let close_start = delims[ci].start + delims[ci].used_left;
            delims[ci].used_left += use_len;
            pairs.push(Pairing {
                open: open_start..open_start + use_len,
                close: close_start..close_start + use_len,
                strong,
            });
        }
    }
    // Outer pairs were recorded after their inner pairs (later closers
    // wrap earlier ones); sort by (start asc, end desc) for assembly.
    pairs.sort_by(|a, b| {
        a.open
            .start
            .cmp(&b.open.start)
            .then(b.close.end.cmp(&a.close.end))
    });
    pairs
}

// ---------------------------------------------------------------------------
// Phase 4: assembly (iterative — delimiter nesting is unbounded)
// ---------------------------------------------------------------------------

fn assemble(base: usize, toks: &[Tok], delims: &[DelimRun], pairs: &[Pairing]) -> Vec<InlineNode> {
    // Leaves: text/code/link tokens plus each run's unclaimed middle
    // bytes (leftover delimiters are literal text).
    let mut leaves: Vec<InlineNode> = Vec::new();
    for tok in toks {
        match tok {
            Tok::Text(r) => push_text(&mut leaves, base + r.start, base + r.end),
            Tok::Code(r) => leaves.push(InlineNode::Code {
                range: abs(base, r.clone()),
            }),
            Tok::Link(link) => leaves.push(InlineNode::Link {
                range: abs(base, link.full.clone()),
                children: link.children.clone(),
                destination: abs(base, link.dest.clone()),
                title: link.title.as_ref().map(|t| abs(base, t.clone())),
            }),
            Tok::Delim(_) => {}
        }
    }
    for delim in delims {
        let leftover_start = delim.start + delim.used_left;
        let leftover_end = delim.start + delim.len - delim.used_right;
        if leftover_end > leftover_start {
            push_text(&mut leaves, base + leftover_start, base + leftover_end);
        }
    }
    leaves.sort_by_key(|node| node_range(node).start.as_usize());
    merge_adjacent_text(&mut leaves);

    // Pairs are relative to the run; everything below is absolute.
    let pairs: Vec<Pairing> = pairs
        .iter()
        .map(|p| Pairing {
            open: p.open.start + base..p.open.end + base,
            close: p.close.start + base..p.close.end + base,
            strong: p.strong,
        })
        .collect();
    let pairs = &pairs[..];

    // Frame stack. `pairs` is sorted (start asc, end desc) and pair byte
    // ranges are disjoint, so pairs nest or sit side by side. Opening a
    // pair first closes every frame that ended at or before the pair's
    // opener (a finished sibling, not a wrapper). Iterative on purpose:
    // delimiter nesting is unbounded.
    struct Frame {
        end: usize,
        pair: Pairing,
        children: Vec<InlineNode>,
    }
    fn close_finished(
        stack: &mut Vec<Frame>,
        root: &mut Vec<InlineNode>,
        before: usize,
        _base: usize,
    ) {
        while stack.last().is_some_and(|f| f.end <= before) {
            let frame = stack.pop().expect("checked non-empty");
            let node = emphasis_node(frame.pair, frame.children);
            match stack.last_mut() {
                Some(parent) => parent.children.push(node),
                None => root.push(node),
            }
        }
    }

    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Vec<InlineNode> = Vec::new();
    let mut pair_idx = 0usize;
    for leaf in leaves {
        let leaf_start = node_range(&leaf).start.as_usize();
        while pair_idx < pairs.len() && pairs[pair_idx].open.start <= leaf_start {
            let pair = pairs[pair_idx].clone();
            pair_idx += 1;
            close_finished(&mut stack, &mut root, pair.open.start, base);
            stack.push(Frame {
                end: pair.close.end,
                pair,
                children: Vec::new(),
            });
        }
        close_finished(&mut stack, &mut root, leaf_start, base);
        match stack.last_mut() {
            Some(frame) => frame.children.push(leaf),
            None => root.push(leaf),
        }
    }
    while let Some(frame) = stack.pop() {
        let node = emphasis_node(frame.pair, frame.children);
        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => root.push(node),
        }
    }
    root
}

fn emphasis_node(pair: Pairing, children: Vec<InlineNode>) -> InlineNode {
    let range = SourceRange::new(
        crate::position::ByteOffset(pair.open.start),
        crate::position::ByteOffset(pair.close.end),
    );
    if pair.strong {
        InlineNode::Strong { range, children }
    } else {
        InlineNode::Emphasis { range, children }
    }
}

fn node_range(node: &InlineNode) -> SourceRange {
    match node {
        InlineNode::Text { range }
        | InlineNode::Code { range }
        | InlineNode::Emphasis { range, .. }
        | InlineNode::Strong { range, .. }
        | InlineNode::Link { range, .. } => *range,
    }
}

fn abs(base: usize, relative: std::ops::Range<usize>) -> SourceRange {
    SourceRange::new(
        crate::position::ByteOffset(base + relative.start),
        crate::position::ByteOffset(base + relative.end),
    )
}

fn push_text(nodes: &mut Vec<InlineNode>, start: usize, end: usize) {
    nodes.push(InlineNode::Text {
        range: SourceRange::new(
            crate::position::ByteOffset(start),
            crate::position::ByteOffset(end),
        ),
    });
}

fn merge_adjacent_text(nodes: &mut Vec<InlineNode>) {
    let mut merged: Vec<InlineNode> = Vec::with_capacity(nodes.len());
    for node in nodes.drain(..) {
        if let InlineNode::Text { range } = &node {
            if let Some(InlineNode::Text { range: last, .. }) = merged.last_mut() {
                if last.end == range.start {
                    last.end = range.end;
                    continue;
                }
            }
        }
        merged.push(node);
    }
    *nodes = merged;
}

fn shift(range: SourceRange, delta: i64) -> SourceRange {
    let move_offset = |offset: crate::position::ByteOffset| {
        crate::position::ByteOffset((offset.as_usize() as i64 + delta).max(0) as usize)
    };
    SourceRange::new(move_offset(range.start), move_offset(range.end))
}

fn shift_nodes(nodes: &[InlineNode], delta: i64) -> Vec<InlineNode> {
    nodes
        .iter()
        .map(|node| match node {
            InlineNode::Text { range } => InlineNode::Text {
                range: shift(*range, delta),
            },
            InlineNode::Code { range } => InlineNode::Code {
                range: shift(*range, delta),
            },
            InlineNode::Emphasis { range, children } => InlineNode::Emphasis {
                range: shift(*range, delta),
                children: shift_nodes(children, delta),
            },
            InlineNode::Strong { range, children } => InlineNode::Strong {
                range: shift(*range, delta),
                children: shift_nodes(children, delta),
            },
            InlineNode::Link {
                range,
                children,
                destination,
                title,
            } => InlineNode::Link {
                range: shift(*range, delta),
                children: shift_nodes(children, delta),
                destination: shift(*destination, delta),
                title: title.map(|t| shift(t, delta)),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::ByteOffset;

    fn parse(text: &str) -> Vec<InlineNode> {
        parse_run(text, 0)
    }

    /// `(kind, start, end)` summary for compact assertions.
    fn shape(nodes: &[InlineNode]) -> Vec<(&'static str, usize, usize)> {
        nodes
            .iter()
            .map(|n| {
                (
                    n.kind_name(),
                    node_range(n).start.as_usize(),
                    node_range(n).end.as_usize(),
                )
            })
            .collect()
    }

    fn text_of(nodes: &[InlineNode], i: usize, text: &str) -> String {
        let range = node_range(&nodes[i]);
        text[range.start.as_usize()..range.end.as_usize()].to_string()
    }

    #[test]
    fn plain_text_is_one_node() {
        let nodes = parse("hello 世界 🙂");
        assert_eq!(shape(&nodes), vec![("text", 0, 17)]);
        // Absolute offsets survive a nonzero base.
        let shifted = parse_run("ab", 100);
        assert_eq!(shape(&shifted), vec![("text", 100, 102)]);
    }

    #[test]
    fn escapes_cover_both_bytes() {
        let nodes = parse(r"a \* b \\ c");
        assert_eq!(nodes.len(), 1, "all text");
        assert_eq!(text_of(&nodes, 0, r"a \* b \\ c"), r"a \* b \\ c");
        // An escaped delimiter never pairs.
        assert_eq!(shape(&parse(r"\*x*")), vec![("text", 0, 4)]);
    }

    #[test]
    fn code_spans_match_equal_lengths() {
        assert_eq!(shape(&parse("`x`")), vec![("code", 0, 3)]);
        assert_eq!(shape(&parse("``x` ``")), vec![("code", 0, 7)]);
        // No closer of equal length -> literal.
        assert_eq!(shape(&parse("``x`")), vec![("text", 0, 4)]);
        // Content is opaque to emphasis.
        assert_eq!(shape(&parse("``*x*``")), vec![("code", 0, 7)]);
        // Between two spans, text resumes.
        assert_eq!(
            shape(&parse("`a` and `b`")),
            vec![("code", 0, 3), ("text", 3, 8), ("code", 8, 11)]
        );
    }

    #[test]
    fn emphasis_basics() {
        assert_eq!(shape(&parse("*x*")), vec![("em", 0, 3)]);
        assert_eq!(shape(&parse("**x**")), vec![("strong", 0, 5)]);
        // Intraword '*' works, intraword '_' does not (contract §7.3).
        assert_eq!(
            shape(&parse("a*b*c")),
            vec![("text", 0, 1), ("em", 1, 4), ("text", 4, 5)]
        );
        assert_eq!(shape(&parse("snake_case_name")).len(), 1);
        assert_eq!(shape(&parse("_x_")), vec![("em", 0, 3)]);
        // Unmatched stays literal.
        assert_eq!(shape(&parse("*x")).len(), 1);
    }

    #[test]
    fn emphasis_nesting_and_runs() {
        // ***x*** = em > strong (CommonMark shape).
        let nodes = parse("***x***");
        assert_eq!(shape(&nodes), vec![("em", 0, 7)]);
        let InlineNode::Emphasis { children, .. } = &nodes[0] else {
            panic!()
        };
        assert_eq!(shape(children), vec![("strong", 1, 6)]);
        // Strong with inner em.
        let nodes = parse("**a *b* c**");
        let InlineNode::Strong { children, .. } = &nodes[0] else {
            panic!()
        };
        assert_eq!(
            shape(children),
            vec![("text", 2, 4), ("em", 4, 7), ("text", 7, 9)]
        );
        // em spanning a line inside one run.
        assert_eq!(shape(&parse("*foo\nbar*")), vec![("em", 0, 9)]);
        // Literal leftovers between pairs: a**b -> em, text.
        let nodes = parse("*a**b*");
        assert_eq!(shape(&nodes), vec![("em", 0, 3), ("em", 3, 6)]);
    }

    #[test]
    fn links_parse_all_forms() {
        let nodes = parse("[a](http://x)");
        let InlineNode::Link {
            destination,
            title,
            children,
            ..
        } = &nodes[0]
        else {
            panic!()
        };
        assert_eq!(shape(children), vec![("text", 1, 2)]);
        assert_eq!(destination.start.as_usize(), 4);
        assert_eq!(destination.end.as_usize(), 12);
        assert_eq!(*title, None);

        // Title forms.
        assert!(matches!(
            parse("[a](u \"t\")")[0],
            InlineNode::Link { title: Some(_), .. }
        ));
        assert!(matches!(
            parse("[a](u 't')")[0],
            InlineNode::Link { title: Some(_), .. }
        ));
        assert!(matches!(
            parse("[a](u (t))")[0],
            InlineNode::Link { title: Some(_), .. }
        ));

        // Angle destination and escaped punctuation inside.
        assert!(matches!(parse("[a](<u\\>x>)")[0], InlineNode::Link { .. }));

        // Balanced parens in a bare destination.
        assert!(matches!(parse("[a](u(1))")[0], InlineNode::Link { .. }));

        // Link text carries inline structure.
        let nodes = parse("[*a* `b`](u)");
        let InlineNode::Link { children, .. } = &nodes[0] else {
            panic!()
        };
        assert_eq!(
            shape(children),
            vec![("em", 1, 4), ("text", 4, 5), ("code", 5, 8)]
        );
    }

    #[test]
    fn malformed_links_stay_literal() {
        assert_eq!(shape(&parse("[a] x")).len(), 1); // no '('
        assert_eq!(shape(&parse("[a](u")).len(), 1); // no ')'
        assert_eq!(shape(&parse("[a](  )")).len(), 1); // empty dest
        assert_eq!(shape(&parse("[a] (u)")).len(), 1); // gap before '('
                                                       // Nested link: outer fails, inner survives (CommonMark shape).
        let nodes = parse("[x [b](u)](v)");
        assert_eq!(
            shape(&nodes),
            vec![("text", 0, 3), ("link", 3, 9), ("text", 9, 13)]
        );
        // Emphasis around a link still pairs.
        let nodes = parse("*[a](u)*");
        assert_eq!(shape(&nodes), vec![("em", 0, 8)]);
    }

    #[test]
    fn unsupported_inline_constructs_degrade_to_text() {
        // Image: '!' is text, the link still parses (D12).
        let nodes = parse("![alt](u)");
        assert_eq!(shape(&nodes), vec![("text", 0, 1), ("link", 1, 9)]);
        // Reference links, autolinks, raw HTML, entities: text (D12).
        for text in ["[a][b]", "<http://x>", "<b>", "&amp;"] {
            assert_eq!(parse(text).len(), 1, "{text} stays one text node");
        }
    }

    #[test]
    fn attach_inline_per_block_kind() {
        use crate::document::Document;
        let doc = Document::new("# h *x*\npara _y_\n> q `z`\n- [a](u)\n```\n*x*\n```\n");
        let snap = doc.snapshot();
        let state = crate::markdown::MarkdownState::build(&snap);
        let kinds: Vec<_> = state
            .blocks()
            .iter()
            .map(|b| (b.kind, b.inline.runs.len(), has_nodes(b)))
            .collect();
        // heading(1 run), paragraph(1), quote(1), list(1), fence(0), trailing blank(0)
        assert_eq!(kinds[0], (crate::markdown::BlockKind::Heading, 1, true));
        assert_eq!(kinds[1], (crate::markdown::BlockKind::Paragraph, 1, true));
        assert_eq!(kinds[2], (crate::markdown::BlockKind::BlockQuote, 1, true));
        assert_eq!(
            kinds[3],
            (crate::markdown::BlockKind::UnorderedList, 1, true)
        );
        assert_eq!(kinds[4], (crate::markdown::BlockKind::FencedCode, 0, false));
        let em = state.blocks()[0].inline.runs[0].nodes[0].clone();
        assert_eq!(em.kind_name(), "text");
    }

    fn has_nodes(record: &BlockRecord) -> bool {
        record.inline.runs.iter().any(|r| !r.nodes.is_empty())
    }

    #[test]
    fn inline_query_view() {
        use crate::document::Document;
        let doc = Document::new("*a* b");
        let snap = doc.snapshot();
        let state = crate::markdown::MarkdownState::build(&snap);
        let id = state.blocks()[0].id;
        let ir = state.inline_ir(id).expect("paragraph has inline IR");
        assert_eq!(ir.runs.len(), 1);
        assert_eq!(shape(&ir.runs[0].nodes), vec![("em", 0, 3), ("text", 3, 5)]);
        let _ = ByteOffset(0);
    }
}
