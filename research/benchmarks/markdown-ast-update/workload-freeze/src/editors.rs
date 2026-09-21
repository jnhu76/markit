//! Deterministic edit constructors for the frozen transition registry
//! (CORRECTIVE-C A6/A7).
//!
//! Absolute rule (task §34): every edit derives from EXISTING real syntax
//! (`existing real syntax -> canonical mutation`). No constructor inserts
//! syntax that was not already there; a constructor that cannot produce
//! its declared transition from an anchor returns a recorded rejection,
//! never a fabricated edit.
//!
//! Constructor shape: given the pre source and one profiler-recognized
//! real anchor (never a blind regex offset — task §31), build the
//! canonical edit, then DERIVE the payload's document-specific predicates
//! from the reference parses of the pre and post sources. The oracle
//! re-proves every derived predicate at validation time. An anchor whose
//! edit produces no provable transition (`kind`-count flip, topology
//! change with content change, or text-content change) is rejected, so a
//! one-byte mutation that does not change the parsed state can never
//! become a payload (task §17, §35).

use markit_mdbench_semantics::facts::{RecognitionStatus, SyntaxFact, SyntaxKind};
use markit_mdbench_semantics::parse::{fenced_content_interval, LaneNode, LaneParse};
use markit_mdbench_semantics::transition::{topology, text_fingerprint, EditSpec, PredicateV1};
use markit_mdbench_semantics::{g0::parse_g0, g1::parse_g1};

/// Kinds whose recognized counts are derived as payload-specific
/// predicates whenever pre != post (document-relative proof of exactly
/// what the edit did to the parse).
pub const DERIVED_KINDS: &[SyntaxKind] = &[
    SyntaxKind::Paragraph,
    SyntaxKind::HeadingAtx,
    SyntaxKind::HeadingSetext,
    SyntaxKind::BlockQuote,
    SyntaxKind::List,
    SyntaxKind::ListItem,
    SyntaxKind::CodeBlockFenced,
    SyntaxKind::CodeBlockIndented,
    SyntaxKind::HtmlBlock,
    SyntaxKind::ThematicBreak,
    SyntaxKind::ReferenceDefinition,
    SyntaxKind::Table,
    SyntaxKind::TableHeaderRow,
    SyntaxKind::TableRow,
    SyntaxKind::TableCell,
    SyntaxKind::Text,
    SyntaxKind::Emphasis,
    SyntaxKind::Strong,
    SyntaxKind::CodeSpan,
    SyntaxKind::LinkInline,
    SyntaxKind::LinkReference,
    SyntaxKind::LinkAutolink,
    SyntaxKind::Image,
    SyntaxKind::RawHtmlInline,
];

/// Parse a source under a lane id ("G0" / "G1").
pub fn parse_lane(source: &str, lane_id: &str) -> LaneParse {
    match lane_id {
        "G1" => parse_g1(source),
        _ => parse_g0(source),
    }
}

/// Recognized occurrences of `kind`: parse nodes plus recognized
/// out-of-band facts not already covered by a node (the frozen
/// TRANSITION-ORACLE-v1 counting rule).
pub fn recognized_count(parse: &LaneParse, facts: &[SyntaxFact], kind: SyntaxKind) -> u64 {
    let nodes: Vec<&LaneNode> = parse.nodes().into_iter().filter(|n| n.kind == kind).collect();
    let extra = facts
        .iter()
        .filter(|fact| {
            fact.syntax_kind == kind
                && fact.recognition_status == RecognitionStatus::Recognized
                && !nodes
                    .iter()
                    .any(|node| node.span.intersects(&fact.span()))
        })
        .count() as u64;
    nodes.len() as u64 + extra
}

/// Everything derived from comparing the pre and post reference parses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derivation {
    /// Pre-side assertions: registry pre floors + exact before-counts for
    /// every flipped kind.
    pub pre_predicates: Vec<PredicateV1>,
    /// Post-side assertions: registry post floors + exact after-counts
    /// for every flipped kind + the satisfied cross-source predicates.
    pub post_predicates: Vec<PredicateV1>,
    /// Topology (position-independent) is unchanged pre->post.
    pub topology_equal: bool,
    /// At least one text/code-span byte sequence differs pre->post.
    pub text_differs: bool,
    /// At least one derived kind count flipped.
    pub kind_flip: bool,
    pub pre_topology_sha256: String,
    pub post_topology_sha256: String,
}

/// Derive the payload predicate set for one edit. The registry floors are
/// carried verbatim into their sides; every derived predicate is proven
/// independently by the oracle at validation time.
pub fn derive_predicates(
    grammar_id: &str,
    pre_parse: &LaneParse,
    pre_source: &str,
    pre_facts: &[SyntaxFact],
    post_parse: &LaneParse,
    post_source: &str,
    post_facts: &[SyntaxFact],
    floor_pre: Vec<PredicateV1>,
    floor_post: Vec<PredicateV1>,
) -> Derivation {
    let mut pre_predicates = floor_pre;
    let mut post_predicates = floor_post;
    let mut kind_flip = false;
    for kind in DERIVED_KINDS {
        let before = recognized_count(pre_parse, pre_facts, *kind);
        let after = recognized_count(post_parse, post_facts, *kind);
        if before != after {
            kind_flip = true;
            pre_predicates.push(PredicateV1::KindCount {
                grammar_id: grammar_id.to_string(),
                syntax_kind: *kind,
                op: markit_mdbench_semantics::transition::CmpOp::Eq,
                count: before,
            });
            post_predicates.push(PredicateV1::KindCount {
                grammar_id: grammar_id.to_string(),
                syntax_kind: *kind,
                op: markit_mdbench_semantics::transition::CmpOp::Eq,
                count: after,
            });
        }
    }
    let topology_equal = topology(pre_parse) == topology(post_parse);
    let text_differs =
        text_fingerprint(pre_parse, pre_source) != text_fingerprint(post_parse, post_source);
    if topology_equal {
        post_predicates.push(PredicateV1::TopologyEqual);
    }
    if text_differs {
        post_predicates.push(PredicateV1::TextContentDiffers);
    }

    Derivation {
        pre_predicates,
        post_predicates,
        topology_equal,
        text_differs,
        kind_flip,
        pre_topology_sha256: markit_mdbench_semantics::sha256_hex(
            topology(pre_parse).as_bytes(),
        ),
        post_topology_sha256: markit_mdbench_semantics::sha256_hex(
            topology(post_parse).as_bytes(),
        ),
    }
}

/// Transition truth gate: a derivation is provable when the edit actually
/// did something the frozen vocabulary can see — a kind-count flip, or a
/// text-content change. A mutation whose parse state is observably
/// identical is not a transition (task §17, §35).
pub fn derivation_is_provable(derivation: &Derivation) -> bool {
    derivation.kind_flip || derivation.text_differs
}

// ---------------------------------------------------------------------------
// Line helpers (mechanical byte extraction from authoritative spans)
// ---------------------------------------------------------------------------

/// Snap an offset down to the nearest UTF-8 char boundary (UTF-8 rule,
/// task §35: deterministic and boundary-safe).
fn floor_boundary(source: &str, mut offset: usize) -> usize {
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// Start of the line containing `offset`.
fn line_start_of(source: &str, offset: usize) -> usize {
    let offset = floor_boundary(source, offset);
    source[..offset]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0)
}

/// End of the line containing `offset` (exclusive, LF included when the
/// line has one).
fn line_end_of(source: &str, offset: usize) -> usize {
    match source[offset..].find('\n') {
        Some(rel) => offset + rel + 1,
        None => source.len(),
    }
}

fn is_ascii_alnum(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

/// One constructed (not yet floor-checked) edit with its context label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructedEdit {
    pub edit: EditSpec,
    pub context: String,
}

fn reject<T>(reason: &str) -> Result<T, String> {
    Err(reason.to_string())
}

/// G0-LOCAL-TEXT-REPLACE-EQ: same-length ASCII replacement inside one
/// real Text run. Chooses the longest ASCII-letter run in the span
/// (`max_by_key`, so on equal lengths the LATEST such run wins), then
/// replaces its middle `min(8, len)` bytes with `z`×L.
pub fn construct_local_text_replace_eq(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    let mut runs: Vec<(usize, usize)> = Vec::new(); // (start, len)
    let mut index = span.start;
    while index < span.end {
        if bytes[index].is_ascii_alphabetic() {
            let start = index;
            while index < span.end && bytes[index].is_ascii_alphabetic() {
                index += 1;
            }
            runs.push((start, index - start));
        } else {
            index += 1;
        }
    }
    runs.retain(|(_, len)| *len >= 4);
    let Some(best) = runs.iter().max_by_key(|(_, len)| *len) else {
        return reject("NO_ASCII_LETTER_RUN_GE4");
    };
    let (run_start, run_len) = *best;
    let replace_len = run_len.min(8);
    let start = run_start + (run_len - replace_len) / 2;
    let inserted = "z".repeat(replace_len);
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: start as u64,
            edit_end: (start + replace_len) as u64,
            inserted_text: inserted,
        },
        context: "paragraph_text".into(),
    })
}

/// G0-PARAGRAPH-SPLIT: replace one interior space of the real paragraph
/// with a blank line. Ranking key = distance to the paragraph midpoint
/// quantized to 1/1000 of a byte, with the byte offset as a secondary
/// term (`min_by_key`, so within one quantized bucket the EARLIEST
/// position wins). Interior = both neighbors exist on the same line and
/// are non-space bytes.
pub fn construct_paragraph_split(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    let mut interior_spaces: Vec<usize> = Vec::new();
    let mut index = span.start;
    while index < span.end {
        if bytes[index] == b' ' {
            let prev_ok = index > span.start
                && bytes[index - 1] != b' '
                && bytes[index - 1] != b'\n';
            let next_ok = index + 1 < span.end
                && bytes[index + 1] != b' '
                && bytes[index + 1] != b'\n';
            if prev_ok && next_ok {
                interior_spaces.push(index);
            }
        }
        index += 1;
    }
    if interior_spaces.is_empty() {
        return reject("NO_INTERIOR_SPACE");
    }
    let mid = (span.start + span.end) as f64 / 2.0;
    let pos = *interior_spaces
        .iter()
        .min_by_key(|&&pos| ((pos as f64 - mid).abs() * 1000.0) as u64 * 2 + (pos as u64))
        .expect("non-empty");
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: pos as u64,
            edit_end: pos as u64 + 1,
            inserted_text: "\n\n".into(),
        },
        context: "paragraph_boundary".into(),
    })
}

/// One paragraph-merge boundary: `right` is a real paragraph whose span
/// starts exactly one blank line after another paragraph's end.
/// G0-PARAGRAPH-MERGE: the two LF bytes of that blank line are replaced
/// by one LF (soft-break join; the R0 M-BB-PARA-MERGE shape adapted to a
/// content-preserving join).
pub fn construct_paragraph_merge(
    source: &str,
    right: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let start = right.span().start;
    if start < 2 {
        return reject("NO_BLANK_LINE_BEFORE");
    }
    if &source.as_bytes()[start - 2..start] != b"\n\n" {
        return reject("NO_BLANK_LINE_BEFORE");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: (start - 2) as u64,
            edit_end: start as u64,
            inserted_text: "\n".into(),
        },
        context: "paragraph_boundary".into(),
    })
}

/// G0-ATX-TO-PARAGRAPH: remove the real heading marker (`#`+ + one
/// space). G0-PARAGRAPH-TO-ATX (restore): insert the same bytes back.
pub fn construct_atx_marker_remove(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    let mut marker_len = 0usize;
    while span.start + marker_len < span.end && bytes[span.start + marker_len] == b'#' {
        marker_len += 1;
    }
    if marker_len == 0 {
        return reject("NO_MARKER_RUN");
    }
    let mut delete_end = span.start + marker_len;
    if delete_end < span.end && bytes[delete_end] == b' ' {
        delete_end += 1;
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: span.start as u64,
            edit_end: delete_end as u64,
            inserted_text: String::new(),
        },
        context: "document_block".into(),
    })
}

pub fn construct_atx_marker_insert(removed_bytes: &str, at: u64) -> ConstructedEdit {
    ConstructedEdit {
        edit: EditSpec {
            edit_start: at,
            edit_end: at,
            inserted_text: removed_bytes.to_string(),
        },
        context: "document_block".into(),
    }
}

/// G0-LIST-ITEM-INDENT: insert two spaces after the item line's existing
/// indent. Requires a marker line (`-`/`*`) at the anchor whose
/// immediately preceding line is a sibling marker line at the same
/// indent: under G0 §7 (tight-only, blank line ends the list) only such
/// an item can nest under its predecessor when indented.
pub fn construct_list_item_indent(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let line_start = line_start_of(source, fact.span().start);
    let bytes = source.as_bytes();
    let mut indent = 0usize;
    while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
        indent += 1;
    }
    let Some(&marker) = bytes.get(line_start + indent) else {
        return reject("NO_MARKER_LINE");
    };
    if marker != b'-' && marker != b'*' {
        return reject("NO_MARKER_LINE");
    }
    // Adjacent-sibling precondition: the line immediately before the
    // item line (the line ending at prev_lf) must be a sibling marker
    // line at the same indent.
    let Some(prev_lf) = line_start.checked_sub(1) else {
        return reject("NO_ADJACENT_SIBLING");
    };
    if bytes.get(prev_lf) != Some(&b'\n') {
        return reject("NO_ADJACENT_SIBLING");
    }
    let prev_start = match source[..prev_lf].rfind('\n') {
        Some(index) => index + 1,
        None => 0,
    };
    let mut prev_indent = 0usize;
    while prev_start + prev_indent < prev_lf && bytes[prev_start + prev_indent] == b' ' {
        prev_indent += 1;
    }
    let Some(&prev_marker) = bytes.get(prev_start + prev_indent) else {
        return reject("NO_ADJACENT_SIBLING");
    };
    if prev_indent != indent || (prev_marker != b'-' && prev_marker != b'*') {
        return reject("NO_ADJACENT_SIBLING");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: (line_start + indent) as u64,
            edit_end: (line_start + indent) as u64,
            inserted_text: "  ".into(),
        },
        context: "list_item".into(),
    })
}

/// G0-BQ-NEST-LINE: insert `>` immediately before the quote line's own
/// `>` marker.
pub fn construct_bq_nest(source: &str, fact: &SyntaxFact) -> Result<ConstructedEdit, String> {
    let line_start = line_start_of(source, fact.span().start);
    let bytes = source.as_bytes();
    let mut index = line_start;
    while index < bytes.len() && bytes[index] == b' ' {
        index += 1;
    }
    if bytes.get(index) != Some(&b'>') {
        return reject("NO_QUOTE_MARKER_LINE");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: index as u64,
            edit_end: index as u64,
            inserted_text: ">".into(),
        },
        context: "blockquote_line".into(),
    })
}

/// Locate the closer's backtick run of a real fenced block: scan forward
/// from the frozen body-interval end over container/trivia bytes until
/// the run. Returns (run_start, run_len).
fn closer_backtick_run(
    source: &str,
    fact: &SyntaxFact,
) -> Result<(usize, usize), String> {
    let span = fact.span();
    let fence_char = markit_mdbench_semantics::parse::fence_char_at(source, span);
    let body = fenced_content_interval(source, span, fence_char);
    let bytes = source.as_bytes();
    let mut index = body.end;
    // Container prefixes / indentation before the closer run.
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'>' | b'-' | b'*' | b'.' => index += 1,
            b'`' => break,
            b'\n' => return reject("NO_CLOSER_LINE"),
            _ => return reject("NO_CLOSER_RUN"),
        }
    }
    let run_start = index;
    let mut run_len = 0usize;
    while index < bytes.len() && bytes[index] == b'`' {
        run_len += 1;
        index += 1;
    }
    if run_len == 0 {
        return reject("NO_CLOSER_RUN");
    }
    Ok((run_start, run_len))
}

/// G0-FENCE-CLOSER-REMOVE: delete the real closer's backtick run; the
/// fence becomes unclosed and runs to EOF (G0 §8).
pub fn construct_fence_closer_remove(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let (run_start, run_len) = closer_backtick_run(source, fact)?;
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: run_start as u64,
            edit_end: (run_start + run_len) as u64,
            inserted_text: String::new(),
        },
        context: "top_level_fence".into(),
    })
}

/// G0-EMPH-DELIM-BREAK: delete the emphasis span's closing `*`.
pub fn construct_emph_delim_break(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    if source.as_bytes().get(span.end - 1) != Some(&b'*') {
        return reject("NO_CLOSING_DELIMITER");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: (span.end - 1) as u64,
            edit_end: span.end as u64,
            inserted_text: String::new(),
        },
        context: "plain_paragraph".into(),
    })
}

/// G0-CODESPAN-DELIM-BREAK: delete the first backtick of the code span's
/// opening run (the R0 M-IDS-CODE-DELIM shape on real syntax).
pub fn construct_codespan_delim_break(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    let mut run_len = 0usize;
    while span.start + run_len < span.end && bytes[span.start + run_len] == b'`' {
        run_len += 1;
    }
    if run_len == 0 {
        return reject("NO_BACKTICK_RUN");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: span.start as u64,
            edit_end: span.start as u64 + 1,
            inserted_text: String::new(),
        },
        context: "plain_paragraph".into(),
    })
}

/// G0-REFDEF-REMOVE: delete the real definition's whole line.
pub fn construct_refdef_remove(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let start = line_start_of(source, span.start);
    let end = line_end_of(source, span.end);
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: start as u64,
            edit_end: end as u64,
            inserted_text: String::new(),
        },
        context: "reference_dependency".into(),
    })
}

/// G0-LINK-DEST-BREAK: insert a SPACE at the start of the real inline
/// link's destination. G0 §9.2: a destination containing a space makes
/// the construct fail — the link decomposes into literal text, so the
/// edit is a real inline-structure break (a pure destination byte
/// replacement leaves the link a link and is invisible to every
/// predicate in PREDICATE-v1).
pub fn construct_link_dest_break(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    // Locate "](...)" inside the span (the G0 inline-link shape).
    let Some(close_text_rel) = source[span.start..span.end].find(']') else {
        return reject("NO_LINK_SHAPE");
    };
    let paren = span.start + close_text_rel + 1;
    if bytes.get(paren) != Some(&b'(') {
        return reject("NO_LINK_SHAPE");
    }
    let dest_start = paren + 1;
    let Some(close_rel) = source[dest_start..span.end].find(')') else {
        return reject("NO_LINK_SHAPE");
    };
    if dest_start + close_rel == dest_start {
        return reject("EMPTY_DESTINATION");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: dest_start as u64,
            edit_end: dest_start as u64,
            inserted_text: " ".into(),
        },
        context: "inline_link".into(),
    })
}

// ---------------------------------------------------------------------------
// G1 table constructors (SEMANTIC_ONLY; NOT_HORSE_QUALIFIED)
// ---------------------------------------------------------------------------

/// Byte range of the delimiter row (the table span's second line).
fn table_delimiter_row(source: &str, table_span: markit_mdbench_semantics::Span) -> Result<(usize, usize), String> {
    let header_start = line_start_of(source, table_span.start);
    let header_end = line_end_of(source, header_start);
    if header_end >= table_span.end {
        return reject("NO_DELIMITER_ROW");
    }
    let delim_start = header_end;
    let delim_end = line_end_of(source, delim_start);
    Ok((delim_start, delim_end))
}

fn table_header_row_span(source: &str, table_span: markit_mdbench_semantics::Span) -> (usize, usize) {
    let header_start = line_start_of(source, table_span.start);
    (header_start, line_end_of(source, header_start))
}

/// G1-TABLE-DELIM-BREAK: replace the delimiter row's first `-` with `n`
/// so the row stops being a delimiter row (valid Table -> non-Table).
pub fn construct_table_delim_break(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let (delim_start, delim_end) = table_delimiter_row(source, fact.span())?;
    let bytes = source.as_bytes();
    let mut pos = delim_start;
    while pos < delim_end && bytes[pos] != b'-' {
        pos += 1;
    }
    if pos == delim_end {
        return reject("NO_DELIMITER_DASH");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: pos as u64,
            edit_end: pos as u64 + 1,
            inserted_text: "n".into(),
        },
        context: "ordinary_table".into(),
    })
}

/// G1-TABLE-HEADER-PIPE-REMOVE: delete one real header cell-separator
/// pipe (column-boundary edit). A pipe is a separator — not a
/// removable leading/trailing boundary — only when it is neither the
/// row's leading boundary pipe nor its trailing boundary pipe; removing
/// a separator drops the header cell count below the delimiter cell
/// count and destroys the table (GFM 0.29).
pub fn construct_table_header_pipe_remove(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let (header_start, header_end_inclusive) = table_header_row_span(source, fact.span());
    let bytes = source.as_bytes();
    // Exclude the line terminator: only row content participates in the
    // leading/trailing boundary decision.
    let header_end = if header_end_inclusive > header_start
        && bytes[header_end_inclusive - 1] == b'\n'
    {
        header_end_inclusive - 1
    } else {
        header_end_inclusive
    };
    let mut pipes: Vec<usize> = Vec::new();
    for pos in header_start..header_end {
        if bytes[pos] == b'|' {
            pipes.push(pos);
        }
    }
    if pipes.is_empty() {
        return reject("NO_INTERIOR_PIPE");
    }
    let first_non_space = header_start
        + bytes[header_start..header_end]
            .iter()
            .position(|&b| b != b' ')
            .ok_or_else(|| "NO_INTERIOR_PIPE".to_string())?;
    let last_non_space = header_start
        + bytes[header_start..header_end]
            .iter()
            .rposition(|&b| b != b' ')
            .ok_or_else(|| "NO_INTERIOR_PIPE".to_string())?;
    let is_leading = |pos: usize| pos == first_non_space;
    let is_trailing = |pos: usize| pos == last_non_space;
    let separator = pipes
        .iter()
        .find(|&&pos| !is_leading(pos) && !is_trailing(pos));
    let Some(&pos) = separator else {
        return reject("NO_INTERIOR_PIPE");
    };
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: pos as u64,
            edit_end: pos as u64 + 1,
            inserted_text: String::new(),
        },
        context: "ordinary_table".into(),
    })
}

/// G1-TABLE-CELL-EDIT: same-length `z` replacement of the first
/// ASCII-alphanumeric byte inside one real cell.
pub fn construct_table_cell_edit(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let bytes = source.as_bytes();
    let mut pos = span.start;
    while pos < span.end && !is_ascii_alnum(bytes[pos]) {
        pos += 1;
    }
    if pos == span.end {
        return reject("NO_ALNUM_CELL_BYTE");
    }
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: pos as u64,
            edit_end: pos as u64 + 1,
            inserted_text: "z".into(),
        },
        context: "ordinary_table".into(),
    })
}

/// G1-TABLE-ROW-DELETE: delete one real body-row line.
pub fn construct_table_row_delete(
    source: &str,
    fact: &SyntaxFact,
) -> Result<ConstructedEdit, String> {
    let span = fact.span();
    let start = line_start_of(source, span.start);
    let end = line_end_of(source, span.start);
    Ok(ConstructedEdit {
        edit: EditSpec {
            edit_start: start as u64,
            edit_end: end as u64,
            inserted_text: String::new(),
        },
        context: "ordinary_table".into(),
    })
}

// ---------------------------------------------------------------------------
// Restore legs (BREAK/RESTORE pairs; RESTORE starts from the frozen
// broken state S1, never from S0 — task §36, PAYLOAD-LIFECYCLE-v1 §5)
// ---------------------------------------------------------------------------

/// Exact RESTORE inverse over S1: remove `[at_start, at_end)` (the
/// BREAK edit's inserted bytes at their S1 offsets) and re-insert the
/// BREAK edit's removed bytes. Covers pure insert (empty range),
/// pure delete (empty inserted_text) and replace.
pub fn construct_exact_reinsert(
    broken: &str,
    removed: &str,
    at_start: u64,
    at_end: u64,
    context: &str,
) -> ConstructedEdit {
    let _ = broken;
    ConstructedEdit {
        edit: EditSpec {
            edit_start: at_start,
            edit_end: at_end,
            inserted_text: removed.to_string(),
        },
        context: context.to_string(),
    }
}

/// Extract the byte range `source[start..end]` (UTF-8-safe by
/// construction: constructors only pick char boundaries).
pub fn slice(source: &str, start: u64, end: u64) -> String {
    source[start as usize..end as usize].to_string()
}
