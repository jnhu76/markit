//! Lane-independent parse view used to derive fact classes.
//!
//! Each lane produces a [`LaneParse`]: a node tree in the shared
//! [`SyntaxKind`] vocabulary, the lane's *non-host* byte intervals (code,
//! fence bodies, raw HTML) where Markdown-looking bytes are literal, and
//! the lane-specific extras the structural record needs.
//!
//! Nothing in this module parses Markdown: G0 fills it from the frozen
//! repository reference parse, G1 from the pinned independent oracle.

use crate::facts::{
    Span, StructuralFacts, CONTAINER_DEPTH_CONVENTION, LARGEST_BLOCK_RULE, ZERO_DENOMINATOR_RULE,
};
use crate::facts::{SyntaxFact, SyntaxKind, TableFacts};

/// One node of a lane parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneNode {
    pub kind: SyntaxKind,
    pub span: Span,
    /// Frozen kind-specific detail (fence character, table alignment, …).
    pub detail: Option<String>,
    pub children: Vec<LaneNode>,
}

impl LaneNode {
    pub fn new(kind: SyntaxKind, span: Span) -> Self {
        Self {
            kind,
            span,
            detail: None,
            children: Vec::new(),
        }
    }
}

/// Lane-specific quantities the structural record needs and that cannot be
/// recomputed from the node tree alone.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneExtras {
    pub fenced_code_content_bytes: u64,
    pub reference_definition_count: u64,
    pub reference_definition_counting: String,
    pub reference_use_count: u64,
    pub reference_use_counting: String,
    pub table: Option<TableFacts>,
    pub math_occupancy_note: String,
    /// Constructs the lane oracle emitted that this lane's frozen
    /// configuration cannot produce. Always empty in practice; recorded
    /// rather than silently relabeled.
    pub unexpected_oracle_tags: Vec<String>,
}

/// Byte offset where the line containing `offset` starts.
///
/// Used to widen a literal construct to its full extent: an oracle reports
/// an indented code block from its *content*, but the four spaces of
/// indentation are part of the construct, and a lexical probe can anchor on
/// them.
pub fn line_start(source: &str, offset: usize) -> usize {
    let end = offset.min(source.len());
    source[..end].rfind('\n').map_or(0, |index| index + 1)
}

/// A complete lane parse.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneParse {
    pub grammar_id: String,
    pub root: LaneNode,
    /// Intervals whose bytes are literal *content* for this lane: a node
    /// found strictly inside one is not host syntax.
    pub non_host_spans: Vec<Span>,
    /// The full span of every literal construct (fence opener through
    /// closing fence, an indented code block including its indentation, a
    /// code span including its backticks, a raw HTML span).
    ///
    /// A *candidate* is resolved against this list rather than against
    /// [`Self::non_host_spans`], because a lexical probe can anchor on the
    /// construct's own delimiter line: the setext probe takes the line
    /// above the underline, so on `"```\n=====\n```\n"` its candidate
    /// starts at the fence opener and is not contained by the content
    /// interval even though no byte of it is host syntax.
    pub literal_spans: Vec<Span>,
    pub extras: LaneExtras,
    /// Extra lane-specific facts that are not nodes of the tree (e.g.
    /// G1 reference definitions, which its oracle exposes out of band).
    pub out_of_band_facts: Vec<SyntaxFact>,
}

impl LaneParse {
    /// Depth-first iteration over every node except the synthetic root.
    pub fn nodes(&self) -> Vec<&LaneNode> {
        let mut out = Vec::new();
        for child in &self.root.children {
            collect(child, &mut out);
        }
        out
    }

    /// True when `span` is wholly inside a literal construct (candidate
    /// rule).
    ///
    /// A candidate is literal content only when ALL of its bytes are
    /// literal: a table-shaped line inside a fence is body text, but a table
    /// whose header row happens to contain a code span is host syntax (the
    /// pipes and the row structure are outside the code span). Partial
    /// overlap is therefore not disqualifying, and equality IS: a candidate
    /// that exactly fills a literal construct is that construct's bytes.
    ///
    /// The intervals are the constructs' *full* spans, so a candidate that
    /// anchors on a fence opener is literal too.
    pub fn is_inside_non_host(&self, span: Span) -> bool {
        self.literal_spans
            .iter()
            .any(|interval| interval.contains(&span))
    }

    /// True when `span` lies strictly inside a *different* non-host
    /// interval (recognized-node rule).
    ///
    /// The construct that owns a literal region (a fenced block, a code
    /// span, an HTML block) is itself host syntax; only what sits inside it
    /// is not. Equality therefore does not count as containment.
    pub fn is_strictly_inside_non_host(&self, span: Span) -> bool {
        self.non_host_spans.iter().any(|interval| {
            interval.contains(&span) && !(interval.start == span.start && interval.end == span.end)
        })
    }
}

fn collect<'a>(node: &'a LaneNode, out: &mut Vec<&'a LaneNode>) {
    out.push(node);
    for child in &node.children {
        collect(child, out);
    }
}

/// Compute STRUCTURAL_FACT from a lane parse under the frozen counting
/// rules carried in [`StructuralFacts`].
pub fn structural_facts(
    parse: &LaneParse,
    block_kinds: &[SyntaxKind],
    container_kinds: &[SyntaxKind],
) -> StructuralFacts {
    let source_bytes = parse.root.span.len() as u64;
    let mut block_count = 0u64;
    let mut largest_block_bytes = 0u64;
    let mut largest_block_span: Option<Span> = None;
    let mut max_container_depth = 0u32;
    let mut fence_count = 0u64;

    for child in &parse.root.children {
        walk_depth(child, 0, container_kinds, &mut |node, depth| {
            if block_kinds.contains(&node.kind) {
                block_count += 1;
                let bytes = node.span.len() as u64;
                if bytes > largest_block_bytes
                    || (bytes == largest_block_bytes
                        && largest_block_span
                            .map(|span| node.span.start < span.start)
                            .unwrap_or(true))
                {
                    largest_block_bytes = bytes;
                    largest_block_span = Some(node.span);
                }
            }
            if node.kind == SyntaxKind::CodeBlockFenced {
                fence_count += 1;
            }
            if container_kinds.contains(&node.kind) {
                max_container_depth = max_container_depth.max(depth + 1);
            }
        });
    }

    let kib = source_bytes as f64 / 1024.0;
    let fence_density_per_kib = ratio(fence_count as f64, kib);
    let code_occupancy = ratio(
        parse.extras.fenced_code_content_bytes as f64,
        source_bytes as f64,
    );
    let reference_total =
        parse.extras.reference_definition_count + parse.extras.reference_use_count;
    let reference_density_per_kib = ratio(reference_total as f64, kib);

    StructuralFacts {
        grammar_id: parse.grammar_id.clone(),
        block_kinds: block_kinds.to_vec(),
        container_kinds: container_kinds.to_vec(),
        block_count,
        largest_block_span,
        largest_block_bytes,
        max_container_depth,
        container_depth_convention: CONTAINER_DEPTH_CONVENTION.to_string(),
        largest_block_rule: LARGEST_BLOCK_RULE.to_string(),
        zero_denominator_rule: ZERO_DENOMINATOR_RULE.to_string(),
        fence_density_per_kib,
        code_occupancy,
        fenced_code_content_bytes: parse.extras.fenced_code_content_bytes,
        reference_definition_count: parse.extras.reference_definition_count,
        reference_definition_counting: parse.extras.reference_definition_counting.clone(),
        reference_use_count: parse.extras.reference_use_count,
        reference_density_per_kib,
        math_occupancy: None,
        math_occupancy_note: Some(parse.extras.math_occupancy_note.clone()),
        table: parse.extras.table.clone(),
    }
}

fn walk_depth<'a>(
    node: &'a LaneNode,
    depth: u32,
    container_kinds: &[SyntaxKind],
    visit: &mut impl FnMut(&'a LaneNode, u32),
) {
    visit(node, depth);
    for child in &node.children {
        let child_depth = if container_kinds.contains(&node.kind) {
            depth + 1
        } else {
            depth
        };
        walk_depth(child, child_depth, container_kinds, visit);
    }
}

/// Frozen zero-denominator rule: a ratio with a zero denominator is 0.0.
pub fn ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

/// Frozen content-interval rule for a fenced code block given only its
/// span: the opener line's terminator through the start of the closing
/// fence line (or to the block end when unclosed).
///
/// This is a byte rule applied *inside* an oracle-recognized span, not a
/// second parser: it never decides whether the construct exists.
///
/// Closing-line rule (mirrors the oracle state machines): the span's last
/// line closes the fence when it ends in a run of `fence_char` of length
/// >= max(3, opener run), preceded only by container-prefix bytes (space,
/// tab, `>`, `-`, `+`, `*`, `.`, `)`, digits) and followed only by spaces.
/// The prefix allowance is what makes a quoted closer (`> ```` `) close
/// the fence exactly as the oracle's per-line container stripping does;
/// ordinary body text before the run (`x```` `) never qualifies.
pub fn fenced_content_interval(source: &str, span: Span, fence_char: char) -> Span {
    let opener_end = match source[span.start..span.end].find('\n') {
        Some(offset) => span.start + offset + 1,
        None => return Span::new(span.end, span.end),
    };
    if opener_end >= span.end {
        return Span::new(span.end, span.end);
    }
    // Opener run length: skip to the opener's first fence char (the span
    // starts there in both oracle configurations; tolerating a small lead
    // keeps the rule total), then count the run.
    let opener_head = &source[span.start..opener_end];
    let opener_run = match opener_head.find(fence_char) {
        Some(first) => opener_head[first..]
            .chars()
            .take_while(|c| *c == fence_char)
            .count(),
        None => 3,
    };
    // Last line of the span; it closes the fence only if it is a valid
    // closing fence line under the rule above.
    let body = &source[opener_end..span.end];
    let last_line_start = match body.rfind('\n') {
        Some(offset) => opener_end + offset + 1,
        None => opener_end,
    };
    let last_line = source[last_line_start..span.end].trim_end_matches(['\n', ' ']);
    if is_closing_fence_line(last_line, fence_char, opener_run.max(3)) {
        Span::new(opener_end, last_line_start)
    } else {
        Span::new(opener_end, span.end)
    }
}

fn is_closing_fence_line(line: &str, fence_char: char, min_run: usize) -> bool {
    let line = line.trim_end_matches(' ');
    let run_len = line.chars().rev().take_while(|c| *c == fence_char).count();
    if run_len < min_run {
        return false;
    }
    let prefix = &line[..line.len() - run_len];
    prefix.chars().all(|c| {
        matches!(
            c,
            ' ' | '\t' | '>' | '-' | '+' | '*' | '.' | ')' | '0'..='9'
        )
    })
}

/// The fence character of a fenced block at `span.start`, derived from the
/// source bytes inside the recognized span.
pub fn fence_char_at(source: &str, span: Span) -> char {
    for byte in source.as_bytes()[span.start..span.end].iter() {
        match *byte {
            b'`' => return '`',
            b'~' => return '~',
            b' ' => continue,
            _ => break,
        }
    }
    '`'
}
