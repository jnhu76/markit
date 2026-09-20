//! G0 lane adapter — BENCH-GRAMMAR-v1 through the frozen repository
//! reference parse.
//!
//! The profiler does not re-implement BENCH-GRAMMAR-v1: it calls
//! `markit_mdbench_shared_grammar::parse_full` (the same code every horse
//! parses with, extracted line-for-line in R5) and gates the result with
//! `markit_mdbench_oracle::validate_normalized`, exactly as H0 does. A
//! second, profiler-owned G0 parser would be a second semantic authority
//! and is deliberately avoided.

use markit_mdbench_common::NoopWorkSink;
use markit_mdbench_oracle::normalized::{Node, NodeKind};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_shared_grammar::parse_full;

use crate::facts::{Span, SyntaxKind, TableFacts};
use crate::lanes::G0_GRAMMAR_ID;
use crate::parse::{
    fenced_content_interval, LaneExtras, LaneNode, LaneParse,
};

/// Frozen G0 leaf-block counting rule (`block_kinds`).
pub const G0_BLOCK_KINDS: &[SyntaxKind] = &[
    SyntaxKind::Paragraph,
    SyntaxKind::HeadingAtx,
    SyntaxKind::CodeBlockFenced,
    SyntaxKind::ReferenceDefinition,
];

/// Frozen G0 container kinds (`container_kinds`).
pub const G0_CONTAINER_KINDS: &[SyntaxKind] = &[
    SyntaxKind::BlockQuote,
    SyntaxKind::List,
    SyntaxKind::ListItem,
];

/// Parse `source` under G0. Total by construction: BENCH-GRAMMAR-v1 has no
/// error nodes, so this cannot fail on valid UTF-8 input.
pub fn parse_g0(source: &str) -> LaneParse {
    let mut noop = NoopWorkSink;
    let document = parse_full(source.as_bytes(), &mut noop);
    // NORMALIZED-RESULT-v1 rejects zero-length node spans, and the zero-byte
    // document is exactly one zero-length span. Skipping validation there
    // keeps BENCH-GRAMMAR-v1 total over UTF-8 (the empty document parses to
    // no nodes) instead of panicking on a legitimate input. The frozen
    // validator is not modified.
    if !source.is_empty() {
        validate_normalized(&document, Some(source.as_bytes()))
            .expect("G0 reference parse violates NORMALIZED-RESULT-v1");
    }

    let mut non_host_spans = Vec::new();
    let mut literal_spans = Vec::new();
    let mut fenced_code_content_bytes = 0u64;
    let mut reference_definition_count = 0u64;
    let mut reference_use_count = 0u64;
    let mut children = Vec::new();

    for node in &document.root.children {
        children.push(convert(
            source,
            node,
            &mut non_host_spans,
            &mut literal_spans,
            &mut fenced_code_content_bytes,
            &mut reference_definition_count,
            &mut reference_use_count,
        ));
    }

    LaneParse {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        root: LaneNode {
            kind: SyntaxKind::Document,
            span: Span::new(0, source.len()),
            detail: None,
            children,
        },
        non_host_spans,
        literal_spans,
        extras: LaneExtras {
            fenced_code_content_bytes,
            reference_definition_count,
            reference_definition_counting:
                "ALL_DEFINITION_NODES: ReferenceDefinition nodes in source order, duplicates counted"
                    .to_string(),
            reference_use_count,
            reference_use_counting:
                "RESOLVED_REFERENCE_LINKS: ReferenceLink nodes only; an unresolved candidate is \
                 literal text under BENCH-GRAMMAR-v1 §9.3"
                    .to_string(),
            table: None::<TableFacts>,
            math_occupancy_note:
                "not applicable: BENCH-GRAMMAR-v1 has no math construct; math-looking bytes are \
                 ordinary text"
                    .to_string(),
            unexpected_oracle_tags: Vec::new(),
        },
        out_of_band_facts: Vec::new(),
    }
}

// The synthetic root kind is irrelevant (it is never emitted as a fact);
// the tree below it is the document body.

fn convert(
    source: &str,
    node: &Node,
    non_host_spans: &mut Vec<Span>,
    literal_spans: &mut Vec<Span>,
    fenced_code_content_bytes: &mut u64,
    reference_definition_count: &mut u64,
    reference_use_count: &mut u64,
) -> LaneNode {
    let kind = match node.kind {
        NodeKind::Document => SyntaxKind::Paragraph,
        NodeKind::Paragraph => SyntaxKind::Paragraph,
        NodeKind::Heading => SyntaxKind::HeadingAtx,
        NodeKind::BlockQuote => SyntaxKind::BlockQuote,
        NodeKind::List => SyntaxKind::List,
        NodeKind::ListItem => SyntaxKind::ListItem,
        NodeKind::FencedCode => SyntaxKind::CodeBlockFenced,
        NodeKind::Text => SyntaxKind::Text,
        NodeKind::Emphasis => SyntaxKind::Emphasis,
        NodeKind::CodeSpan => SyntaxKind::CodeSpan,
        NodeKind::Link => SyntaxKind::LinkInline,
        NodeKind::ReferenceLink => SyntaxKind::LinkReference,
        NodeKind::ReferenceDefinition => SyntaxKind::ReferenceDefinition,
    };

    let span = Span::new(node.start, node.end);
    let mut lane_node = LaneNode::new(kind, span);

    match node.kind {
        NodeKind::FencedCode => {
            // The frozen vocabulary already owns the raw content interval.
            if let Some((start, end)) = node.content {
                lane_node.detail = Some(format!("info={}", node.info.clone().unwrap_or_default()));
                non_host_spans.push(Span::new(start, end));
                *fenced_code_content_bytes += (end - start) as u64;
            }
            // The whole block, start of the opener line through the closing
            // fence, is the interval a *candidate* is resolved against.
            literal_spans.push(Span::new(
                crate::parse::line_start(source, span.start),
                span.end,
            ));
        }
        NodeKind::CodeSpan => {
            non_host_spans.push(span);
            literal_spans.push(span);
        }
        NodeKind::ReferenceDefinition => {
            *reference_definition_count += 1;
        }
        NodeKind::ReferenceLink => {
            *reference_use_count += 1;
        }
        _ => {}
    }

    // Guard: the content-interval rule must agree with the frozen field.
    if node.kind == NodeKind::FencedCode {
        if let Some((start, end)) = node.content {
            let derived = fenced_content_interval(source, span, '`');
            debug_assert_eq!(
                (derived.start, derived.end),
                (start, end),
                "fenced-content byte rule disagrees with the frozen G0 content field"
            );
        }
    }

    for child in &node.children {
        lane_node.children.push(convert(
            source,
            child,
            non_host_spans,
            literal_spans,
            fenced_code_content_bytes,
            reference_definition_count,
            reference_use_count,
        ));
    }
    lane_node
}
