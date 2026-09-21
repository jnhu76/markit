//! Deterministic normalized-result divergence locator (CORRECTNESS
//! DIAGNOSIS ONLY).
//!
//! [`first_divergence`] answers exactly one question: where do two
//! NORMALIZED-RESULT-v1 documents first differ, in document order? It
//! returns the divergent node path, both sides' node summaries, the
//! divergent attribute, and the source span when one is meaningful.
//!
//! It is NOT a research metric. It deliberately does NOT report
//! `changed_nodes`, damage size, propagation distance, byte counts, or
//! any other reuse/performance quantity — those belong to the #35/#33
//! semantic-interference instrumentation and must never be derived from
//! this module. Nothing here times anything and nothing here mutates a
//! mechanism's state.
//!
//! The traversal is depth-first in child order with a fixed attribute
//! order, so the same input pair always yields the same report.

use crate::normalized::{Node, NodeKind, NormalizedDocument};

/// Which part of a node differed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceKind {
    /// No corresponding node on one side (shape differs).
    MissingNode,
    /// Node kind differs.
    Kind,
    /// Byte span differs.
    Span,
    /// A value field (level/marker/info/content/destination/label)
    /// differs.
    Field,
    /// Same kind, same span, same fields, but a different number of
    /// children.
    ChildCount,
}

/// One node rendered as comparable plain data (used for both sides, so a
/// report can never compare apples against a differently-shaped
/// summarizer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    pub kind: NodeKind,
    pub start: usize,
    pub end: usize,
    pub level: Option<u8>,
    pub marker: Option<String>,
    pub info: Option<String>,
    pub content: Option<(usize, usize)>,
    pub destination: Option<String>,
    pub label: Option<String>,
    pub child_count: usize,
}

impl NodeSummary {
    fn of(node: &Node) -> Self {
        Self {
            kind: node.kind,
            start: node.start,
            end: node.end,
            level: node.level,
            marker: node.marker.clone(),
            info: node.info.clone(),
            content: node.content,
            destination: node.destination.clone(),
            label: node.label.clone(),
            child_count: node.children.len(),
        }
    }
}

/// The first divergence between two normalized results, in document
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// Child indices from the root (`[]` = the root itself).
    pub path: Vec<usize>,
    pub kind: DivergenceKind,
    /// The field name for [`DivergenceKind::Field`] (`"level"`,
    /// `"content"`, ...), else `""`.
    pub field: &'static str,
    pub expected: Option<NodeSummary>,
    pub actual: Option<NodeSummary>,
    /// Source span where the divergence is meaningful (the expected
    /// node's span; the actual node's span when only the actual side
    /// exists). `None` when neither side has a node.
    pub source_span: Option<(usize, usize)>,
}

/// First divergence in document order, or `None` when the documents are
/// structurally equal.
pub fn first_divergence(
    expected: &NormalizedDocument,
    actual: &NormalizedDocument,
) -> Option<Divergence> {
    compare(&[], &expected.root, &actual.root)
}

/// `true` when `first_divergence` finds nothing and the documents are
/// equal (equality is checked directly, so this never disagrees with
/// `PartialEq`).
pub fn is_equal(expected: &NormalizedDocument, actual: &NormalizedDocument) -> bool {
    expected == actual
}

fn compare(path: &[usize], expected: &Node, actual: &Node) -> Option<Divergence> {
    let mut out = path.to_vec();
    if expected.kind != actual.kind {
        return Some(divergence(out, DivergenceKind::Kind, "", expected, actual));
    }
    if (expected.start, expected.end) != (actual.start, actual.end) {
        return Some(divergence(out, DivergenceKind::Span, "", expected, actual));
    }
    for (field, differs) in [
        ("level", expected.level != actual.level),
        ("marker", expected.marker != actual.marker),
        ("info", expected.info != actual.info),
        ("content", expected.content != actual.content),
        ("destination", expected.destination != actual.destination),
        ("label", expected.label != actual.label),
    ] {
        if differs {
            return Some(divergence(
                out,
                DivergenceKind::Field,
                field,
                expected,
                actual,
            ));
        }
    }
    // Shape differences are located at the first child index that exists
    // on only one side — the common prefix is compared first so the
    // report points at the deepest first difference, not at the parent.
    for (index, (e, a)) in expected
        .children
        .iter()
        .zip(actual.children.iter())
        .enumerate()
    {
        out.push(index);
        if let Some(found) = compare(&out, e, a) {
            return Some(found);
        }
        out.pop();
    }
    let common = expected.children.len().min(actual.children.len());
    if expected.children.len() != actual.children.len() {
        out.push(common);
        return Some(Divergence {
            path: out,
            kind: DivergenceKind::MissingNode,
            field: "",
            expected: expected.children.get(common).map(NodeSummary::of),
            actual: actual.children.get(common).map(NodeSummary::of),
            source_span: expected
                .children
                .get(common)
                .map(|node| (node.start, node.end)),
        });
    }
    None
}

fn divergence(
    path: Vec<usize>,
    kind: DivergenceKind,
    field: &'static str,
    expected: &Node,
    actual: &Node,
) -> Divergence {
    Divergence {
        path,
        kind,
        field,
        expected: Some(NodeSummary::of(expected)),
        actual: Some(NodeSummary::of(actual)),
        source_span: Some((expected.start, expected.end)),
    }
}

/// One-line human-readable rendering of a divergence (stable wording —
/// diagnostic reports quote it).
pub fn describe(divergence: &Divergence) -> String {
    let path = if divergence.path.is_empty() {
        "root".to_string()
    } else {
        divergence
            .path
            .iter()
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join("/")
    };
    let what = match divergence.kind {
        DivergenceKind::MissingNode => "missing node",
        DivergenceKind::Kind => "kind",
        DivergenceKind::Span => "span",
        DivergenceKind::Field => divergence.field,
        DivergenceKind::ChildCount => "child_count",
    };
    format!(
        "path={path} field={what} expected={} actual={}",
        divergence
            .expected
            .as_ref()
            .map_or_else(|| "<absent>".to_string(), summarize),
        divergence
            .actual
            .as_ref()
            .map_or_else(|| "<absent>".to_string(), summarize)
    )
}

fn summarize(node: &NodeSummary) -> String {
    let mut text = format!("{}({}..{})", node.kind.name(), node.start, node.end);
    if let Some(level) = node.level {
        text.push_str(&format!(" level={level}"));
    }
    if let Some(marker) = &node.marker {
        text.push_str(&format!(" marker={marker:?}"));
    }
    if let Some(info) = &node.info {
        text.push_str(&format!(" info={info:?}"));
    }
    if let Some((start, end)) = node.content {
        text.push_str(&format!(" content={start}..{end}"));
    }
    if let Some(destination) = &node.destination {
        text.push_str(&format!(" destination={destination:?}"));
    }
    if let Some(label) = &node.label {
        text.push_str(&format!(" label={label:?}"));
    }
    text.push_str(&format!(" children={}", node.child_count));
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(children: Vec<Node>, len: usize) -> NormalizedDocument {
        let mut root = Node::new(NodeKind::Document, 0, len);
        root.children = children;
        NormalizedDocument::new(root)
    }

    #[test]
    fn equal_documents_have_no_divergence() {
        let a = doc(vec![Node::new(NodeKind::Paragraph, 0, 5)], 6);
        let b = doc(vec![Node::new(NodeKind::Paragraph, 0, 5)], 6);
        assert!(first_divergence(&a, &b).is_none());
        assert!(is_equal(&a, &b));
    }

    #[test]
    fn reports_first_divergence_in_document_order() {
        // Two differences: a child-count difference at the root's second
        // child and a kind difference later. The FIRST (document order)
        // must be reported.
        let mut heading = Node::new(NodeKind::Heading, 0, 4);
        heading.level = Some(2);
        let a = doc(
            vec![heading.clone(), Node::new(NodeKind::Paragraph, 4, 9)],
            10,
        );
        let mut heading_wrong_level = Node::new(NodeKind::Heading, 0, 4);
        heading_wrong_level.level = Some(3);
        // Same document length on both sides: the difference is inside
        // the tree, not in the root span.
        let b = doc(
            vec![
                heading_wrong_level,
                Node::new(NodeKind::Paragraph, 4, 9),
                Node::new(NodeKind::FencedCode, 9, 10),
            ],
            10,
        );
        let found = first_divergence(&a, &b).expect("divergence");
        assert_eq!(found.path, vec![0]);
        assert_eq!(found.kind, DivergenceKind::Field);
        assert_eq!(found.field, "level");
        assert_eq!(found.source_span, Some((0, 4)));
    }

    #[test]
    fn reports_missing_node_at_the_first_absent_child() {
        let mut quote_expected = Node::new(NodeKind::BlockQuote, 0, 8);
        quote_expected
            .children
            .push(Node::new(NodeKind::Paragraph, 2, 8));
        let quote_actual = Node::new(NodeKind::BlockQuote, 0, 8);
        let a = doc(vec![quote_expected], 9);
        let b = doc(vec![quote_actual], 9);
        let found = first_divergence(&a, &b).expect("divergence");
        assert_eq!(found.path, vec![0, 0]);
        assert_eq!(found.kind, DivergenceKind::MissingNode);
        assert!(found.expected.is_some());
        assert!(found.actual.is_none());
        assert_eq!(found.source_span, Some((2, 8)));
    }

    #[test]
    fn descends_the_common_prefix_before_reporting_shape() {
        // Both roots have two children; the shape difference lives in the
        // second one. The report must point there, not at the root.
        let mut expected_quote = Node::new(NodeKind::BlockQuote, 0, 4);
        expected_quote
            .children
            .push(Node::new(NodeKind::Paragraph, 2, 4));
        let actual_quote = Node::new(NodeKind::BlockQuote, 0, 4);
        let a = doc(
            vec![Node::new(NodeKind::Paragraph, 0, 0), expected_quote],
            5,
        );
        let b = doc(vec![Node::new(NodeKind::Paragraph, 0, 0), actual_quote], 5);
        let found = first_divergence(&a, &b).expect("divergence");
        assert_eq!(found.path, vec![1, 0]);
        assert_eq!(found.kind, DivergenceKind::MissingNode);
    }

    #[test]
    fn span_difference_is_reported_before_children() {
        let mut expected_quote = Node::new(NodeKind::BlockQuote, 0, 8);
        expected_quote
            .children
            .push(Node::new(NodeKind::Paragraph, 2, 8));
        let mut actual_quote = Node::new(NodeKind::BlockQuote, 0, 12);
        actual_quote
            .children
            .push(Node::new(NodeKind::Paragraph, 2, 8));
        let a = doc(vec![expected_quote], 13);
        let b = doc(vec![actual_quote], 13);
        let found = first_divergence(&a, &b).expect("divergence");
        assert_eq!(found.kind, DivergenceKind::Span);
        assert_eq!(found.source_span, Some((0, 8)));
        assert!(describe(&found).contains("path=0 field=span"));
    }
}
