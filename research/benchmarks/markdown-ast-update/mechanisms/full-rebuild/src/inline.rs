//! H0 inline scanning — BENCH-GRAMMAR-v1 §4, §9, §10.
//!
//! One scanner per content SEGMENT: a segment is a maximal byte range of
//! one block's content between container prefixes (NORMALIZED-RESULT-v1
//! §2: a container prefix and the LF before it are trivia BETWEEN text
//! runs). Inline constructs are matched within a single segment; this is
//! the H0 reference reading of "maximal runs within the parent block's
//! content region" and is documented in the R4 stage record.
//!
//! Frozen rules implemented here:
//!
//! - code spans: exact run-length match, raw content, exact bytes (D10);
//! - links: `[text](dest)` with the frozen failure rules (SPACE / '[' /
//!   no `)` on the same line -> literal), `[text][label]` with no
//!   whitespace between the bracket groups, collapsed `[]` form, NO
//!   shortcut links (D8);
//! - reference resolution: document-global, position-independent,
//!   first definition of a normalized label wins (§9.3); unresolved
//!   references decompose to literal text and scanning resumes one byte
//!   after the `[`;
//! - emphasis: single `*` delimiter, LIFO with close-preferred
//!   tie-break and the empty-emphasis adjacency ban (§10.2); emphasis
//!   content is re-scanned recursively.

use markit_mdbench_oracle::normalized::{Node, NodeKind};

use crate::parser::Skel;

/// Document-global reference-definition table, in source order; first
/// definition of a normalized label wins lookup (§9.3). Later duplicates
/// remain visible as ReferenceDefinition nodes — only lookup skips them.
#[derive(Debug, Default, Clone)]
pub struct RefTable {
    entries: Vec<(String, String)>,
}

impl RefTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a definition fact. Lookup keeps the FIRST entry per label.
    pub fn define(&mut self, label: String, destination: String) {
        self.entries.push((label, destination));
    }

    /// Frozen first-wins lookup.
    pub fn resolve(&self, label: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, d)| d.as_str())
    }
}

/// Label normalization (§9.1): collapse space runs to one space, trim,
/// ASCII-case-fold only (D7).
pub fn norm_label(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut pending_space = false;
    for &b in bytes {
        if b == b' ' {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(b.to_ascii_lowercase() as char);
        }
    }
    out
}

/// Convert block skeletons into normalized nodes, running the inline
/// pass with the completed definition table (§13: blocks -> table ->
/// inlines).
pub fn materialize(src: &[u8], skels: Vec<Skel>, defs: &RefTable) -> Vec<Node> {
    let mut out = Vec::with_capacity(skels.len());
    for s in skels {
        out.push(materialize_one(src, s, defs));
    }
    out
}

fn materialize_one(src: &[u8], s: Skel, defs: &RefTable) -> Node {
    match s {
        Skel::Para {
            start,
            end,
            segments,
        } => {
            let mut n = Node::new(NodeKind::Paragraph, start, end);
            n.children = scan_inlines(src, &segments, true, defs);
            n
        }
        Skel::Heading {
            start,
            end,
            level,
            content,
        } => {
            let mut n = Node::new(NodeKind::Heading, start, end);
            n.level = Some(level);
            n.children = scan_inlines(src, &[content], true, defs);
            n
        }
        Skel::Quote {
            start,
            end,
            children,
        } => {
            let mut n = Node::new(NodeKind::BlockQuote, start, end);
            n.children = materialize(src, children, defs);
            n
        }
        Skel::List { start, end, items } => {
            let mut n = Node::new(NodeKind::List, start, end);
            n.children = materialize(src, items, defs);
            n
        }
        Skel::Item {
            start,
            end,
            marker,
            children,
        } => {
            let mut n = Node::new(NodeKind::ListItem, start, end);
            n.marker = Some(if marker == b'-' { "-" } else { "*" }.to_string());
            n.children = materialize(src, children, defs);
            n
        }
        Skel::Fence {
            start,
            end,
            info,
            content,
        } => {
            let mut n = Node::new(NodeKind::FencedCode, start, end);
            n.info = Some(info);
            n.content = Some(content);
            n
        }
        Skel::Def {
            start,
            end,
            label,
            destination,
        } => {
            let mut n = Node::new(NodeKind::ReferenceDefinition, start, end);
            n.label = Some(label);
            n.destination = Some(destination);
            n
        }
    }
}

/// Scan a block's content segments (links enabled at block level).
pub fn scan_inlines(
    src: &[u8],
    segments: &[(usize, usize)],
    links: bool,
    defs: &RefTable,
) -> Vec<Node> {
    let mut out = Vec::new();
    for &(ss, se) in segments {
        out.extend(scan_region(src, ss, se, links, defs));
    }
    out
}

/// One literal byte (an unmatched emphasis opener) interleaved into the
/// node buffer by position at end of scan.
#[derive(Debug)]
enum Elem {
    Node(Node),
    Byte(usize),
}

/// Scan one segment region `[ss, se)`. `links` is false inside link
/// text (nested links are literal; §9.2).
fn scan_region(src: &[u8], ss: usize, se: usize, links: bool, defs: &RefTable) -> Vec<Node> {
    let mut out: Vec<Elem> = Vec::new();
    // pending maximal text run [text_start, i)
    let mut text_start: Option<usize> = None;
    // emphasis opener stack: (opener position, out.len() after flush)
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut i = ss;

    macro_rules! flush_text {
        ($upto:expr) => {
            if let Some(ts) = text_start.take() {
                if ts < $upto {
                    out.push(Elem::Node(Node::new(NodeKind::Text, ts, $upto)));
                }
            }
        };
    }

    while i < se {
        let b = src[i];
        match b {
            b'`' => {
                let run = run_len(src, i, se, b'`');
                match find_run_exact(src, i + run, se, run) {
                    Some(closer) => {
                        flush_text!(i);
                        let mut n = Node::new(NodeKind::CodeSpan, i, closer + run);
                        n.content = Some((i + run, closer));
                        out.push(Elem::Node(n));
                        i = closer + run;
                    }
                    None => {
                        // unclosed backtick run is literal text (§10.1)
                        text_start.get_or_insert(i);
                        i += run;
                    }
                }
            }
            b'[' if links => {
                match scan_link_candidate(src, ss, se, i, defs) {
                    LinkOutcome::None => {
                        // failed candidate: '[' is literal, resume one
                        // byte forward (§9.2)
                        text_start.get_or_insert(i);
                        i += 1;
                    }
                    LinkOutcome::Link {
                        end,
                        destination,
                        text: (ts, te),
                    } => {
                        flush_text!(i);
                        let mut n = Node::new(NodeKind::Link, i, end);
                        n.destination = Some(destination);
                        n.children = scan_region(src, ts, te, false, defs);
                        out.push(Elem::Node(n));
                        i = end;
                    }
                    LinkOutcome::Reference {
                        end,
                        label,
                        destination,
                        text: (ts, te),
                    } => {
                        flush_text!(i);
                        let mut n = Node::new(NodeKind::ReferenceLink, i, end);
                        n.label = Some(label);
                        n.destination = Some(destination);
                        n.children = scan_region(src, ts, te, false, defs);
                        out.push(Elem::Node(n));
                        i = end;
                    }
                }
            }
            b'*' => {
                let can_open = i + 1 < se && src[i + 1] != b' ' && src[i + 1] != b'\n';
                let can_close = i > ss && src[i - 1] != b' ' && src[i - 1] != b'\n';
                // close-preferred tie-break; empty-emphasis adjacency
                // ban: a '*' is never closable against an opener
                // immediately next to it — only the top opener can be
                // adjacent, so when the top is banned the opener below
                // it (if any) is the close target (§10.2)
                let top_adjacent = stack.last().is_some_and(|&(pos, _)| pos + 1 == i);
                let closable = can_close
                    && match stack.len() {
                        0 => false,
                        1 => !top_adjacent,
                        _ => true,
                    };
                if closable {
                    if top_adjacent {
                        // discard the adjacent-banned top: its byte lies
                        // inside the closed content region and is
                        // re-scanned there
                        stack.pop();
                    }
                    let (opener, out_len) = stack.pop().expect("closable implies opener");
                    flush_text!(i);
                    out.truncate(out_len);
                    let children = scan_region(src, opener + 1, i, links, defs);
                    let mut n = Node::new(NodeKind::Emphasis, opener, i + 1);
                    n.children = children;
                    out.push(Elem::Node(n));
                    text_start = None;
                    i += 1;
                } else if can_open {
                    flush_text!(i);
                    stack.push((i, out.len()));
                    i += 1;
                } else {
                    text_start.get_or_insert(i);
                    i += 1;
                }
            }
            _ => {
                text_start.get_or_insert(i);
                i += 1;
            }
        }
    }
    flush_text!(se);
    // unmatched openers become literal text bytes at their positions,
    // interleaved in source order (recorded buffer indices are
    // non-decreasing bottom-to-top)
    while let Some((pos, out_len)) = stack.pop() {
        out.insert(out_len.min(out.len()), Elem::Byte(pos));
    }
    // coalesce adjacent text bytes / text nodes into maximal runs
    let mut final_nodes: Vec<Node> = Vec::new();
    // pending run [run_start, run_end) merged from Text nodes and bytes
    let mut run: Option<(usize, usize)> = None;
    for elem in out {
        match elem {
            Elem::Byte(p) => match run {
                Some((rs, re)) if re == p => run = Some((rs, p + 1)),
                Some((rs, re)) => {
                    final_nodes.push(Node::new(NodeKind::Text, rs, re));
                    run = Some((p, p + 1));
                }
                None => run = Some((p, p + 1)),
            },
            Elem::Node(n) => {
                if n.kind == NodeKind::Text {
                    match run {
                        Some((rs, re)) if re == n.start => {
                            run = Some((rs, n.end));
                        }
                        Some((rs, re)) => {
                            final_nodes.push(Node::new(NodeKind::Text, rs, re));
                            run = Some((n.start, n.end));
                        }
                        None => run = Some((n.start, n.end)),
                    }
                } else {
                    if let Some((rs, re)) = run.take() {
                        final_nodes.push(Node::new(NodeKind::Text, rs, re));
                    }
                    final_nodes.push(n);
                }
            }
        }
    }
    if let Some((rs, re)) = run {
        final_nodes.push(Node::new(NodeKind::Text, rs, re));
    }
    final_nodes
}

fn run_len(src: &[u8], from: usize, to: usize, byte: u8) -> usize {
    let mut n = from;
    while n < to && src[n] == byte {
        n += 1;
    }
    n - from
}

/// The next run of EXACTLY `n` backticks at/after `from` (§10.1): a
/// shorter run is content; a longer run does not close an n-run.
fn find_run_exact(src: &[u8], from: usize, to: usize, n: usize) -> Option<usize> {
    let mut i = from;
    while i < to {
        if src[i] == b'`' {
            let run = run_len(src, i, to, b'`');
            if run == n {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

enum LinkOutcome {
    /// candidate failed: literal text
    None,
    Link {
        end: usize,
        destination: String,
        text: (usize, usize),
    },
    Reference {
        end: usize,
        label: String,
        destination: String,
        text: (usize, usize),
    },
}

/// Link candidate at `at` (src[at] == '[') (§9.2).
fn scan_link_candidate(
    src: &[u8],
    _ss: usize,
    se: usize,
    at: usize,
    defs: &RefTable,
) -> LinkOutcome {
    // text region: up to the first ']' not inside an open code span
    let text_end = match find_text_region_end(src, at + 1, se) {
        Some(e) => e,
        None => return LinkOutcome::None,
    };
    let text = (at + 1, text_end);
    let after = text_end + 1;
    if after >= se {
        return LinkOutcome::None; // shortcut form "[text]" is not a link (D8)
    }
    match src[after] {
        b'(' => {
            // inline link: destination = bytes up to the FIRST ')';
            // fails on SPACE, '[', or no ')' before end of line (§9.2)
            let mut p = after + 1;
            let mut failed = false;
            while p < se {
                let b = src[p];
                if b == b')' {
                    break;
                }
                if b == b' ' || b == b'[' || b == b'\n' {
                    failed = true;
                    break;
                }
                p += 1;
            }
            if failed || p >= se || src[p] != b')' {
                return LinkOutcome::None;
            }
            let destination = String::from_utf8_lossy(&src[after + 1..p]).into_owned();
            LinkOutcome::Link {
                end: p + 1,
                destination,
                text,
            }
        }
        b'[' => {
            // reference link: '[' label ']' with no whitespace between
            // the two bracket groups
            let mut p = after + 1;
            while p < se && src[p] != b']' {
                p += 1;
            }
            if p >= se {
                return LinkOutcome::None;
            }
            let label_bytes = &src[after + 1..p];
            // empty label = collapsed form: the label is the TEXT bytes
            let key = if label_bytes.is_empty() {
                norm_label(&src[text.0..text.1])
            } else {
                norm_label(label_bytes)
            };
            match defs.resolve(&key) {
                Some(destination) => LinkOutcome::Reference {
                    end: p + 1,
                    label: key,
                    destination: destination.to_string(),
                    text,
                },
                // unresolved: NOT a ReferenceLink — the construct is
                // literal text; scanning resumes after the '[' (§9.3)
                None => LinkOutcome::None,
            }
        }
        _ => LinkOutcome::None,
    }
}

/// Find the first `]` at/after `from` that is not inside an open code
/// span (code spans win; §9.2).
fn find_text_region_end(src: &[u8], from: usize, to: usize) -> Option<usize> {
    let mut i = from;
    while i < to {
        match src[i] {
            b'`' => {
                let run = run_len(src, i, to, b'`');
                match find_run_exact(src, i + run, to, run) {
                    Some(closer) => i = closer + run,
                    None => i += run,
                }
            }
            b']' => return Some(i),
            _ => i += 1,
        }
    }
    None
}
