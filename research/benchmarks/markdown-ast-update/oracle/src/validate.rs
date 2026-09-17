//! NORMALIZED-RESULT-v1 conformance gate (R4-H0-REFERENCE-CORRECTIVE-1).
//!
//! ONE shared validator for ANY normalized result: the 43 golden-fixture
//! expected trees (enforced at load time by [`crate::fixture`]), every H0
//! result (enforced inside `markit-mdbench-full-rebuild`), and — in later
//! stages — every horse's normalized result before correctness
//! comparison. The Python twin in `scripts/verify_r3.py` independently
//! enforces the same frozen table on the static artifacts, so the R3
//! freeze gate also catches drift.
//!
//! Enforced (NORMALIZED-RESULT-v1 §1–§2):
//!
//! - exact field-kind legality: every field a kind allows is REQUIRED,
//!   every field it does not allow is FORBIDDEN (not merely "the required
//!   subset exists");
//! - `start < end` for EVERY node — a normalized node span is never
//!   zero-length (nor inverted). The only zero-length interval in the
//!   vocabulary is the `FencedCode.content` FIELD, validated separately;
//! - `FencedCode.content`: `node.start <= cs <= ce <= node.end`; `cs ==
//!   ce` (empty body) is legal;
//! - parent containment, non-decreasing sibling starts, root is the
//!   `Document` span `[0, len)`;
//! - with `source` given, every span bound (node spans and the content
//!   interval) sits on a UTF-8 char boundary.

use crate::normalized::{Node, NodeKind, NormalizedDocument};

/// The frozen field-kind legality table (NORMALIZED-RESULT-v1 §1): the
/// ONLY fields each kind may carry — and every listed field is required.
/// This table must stay byte-identical to `FIELD_TABLE` in
/// `scripts/verify_r3.py` and to §1 of the authority document.
fn allowed_fields(kind: NodeKind) -> &'static [&'static str] {
    match kind {
        NodeKind::Document
        | NodeKind::Paragraph
        | NodeKind::BlockQuote
        | NodeKind::List
        | NodeKind::Text
        | NodeKind::Emphasis
        | NodeKind::CodeSpan => &[],
        NodeKind::Heading => &["level"],
        NodeKind::ListItem => &["marker"],
        NodeKind::FencedCode => &["info", "content"],
        NodeKind::Link => &["destination"],
        NodeKind::ReferenceLink | NodeKind::ReferenceDefinition => &["label", "destination"],
    }
}

const ALL_FIELDS: [&str; 6] = ["level", "marker", "info", "content", "destination", "label"];

fn field_is_set(n: &Node, field: &str) -> bool {
    match field {
        "level" => n.level.is_some(),
        "marker" => n.marker.is_some(),
        "info" => n.info.is_some(),
        "content" => n.content.is_some(),
        "destination" => n.destination.is_some(),
        "label" => n.label.is_some(),
        _ => false,
    }
}

fn is_boundary(src: &[u8], pos: usize) -> bool {
    pos == 0 || pos >= src.len() || src[pos] & 0xC0 != 0x80
}

/// Validate a complete normalized result. `source` (when given) enables
/// the UTF-8 char-boundary checks and the `Document == [0, len)` rule.
pub fn validate_normalized(doc: &NormalizedDocument, source: Option<&[u8]>) -> Result<(), String> {
    validate_root(&doc.root, source)
}

/// Validate any node as the root of a normalized result.
pub fn validate_root(root: &Node, source: Option<&[u8]>) -> Result<(), String> {
    if root.kind != NodeKind::Document {
        return Err("root node is not a Document".to_string());
    }
    if root.start != 0 {
        return Err(format!("Document span starts at {} != 0", root.start));
    }
    if let Some(src) = source {
        if root.end != src.len() {
            return Err(format!(
                "Document span [0,{}) != [0,{})",
                root.end,
                src.len()
            ));
        }
    }
    walk(root, None, source)
}

fn walk(n: &Node, parent: Option<&Node>, source: Option<&[u8]>) -> Result<(), String> {
    let label = format!("{:?} {}..{}", n.kind, n.start, n.end);
    let allowed = allowed_fields(n.kind);

    // required fields exist
    let missing: Vec<&str> = allowed
        .iter()
        .copied()
        .filter(|f| !field_is_set(n, f))
        .collect();
    if !missing.is_empty() {
        return Err(format!("{label}: missing required field(s) {}", missing.join(", ")));
    }
    // forbidden fields absent
    let forbidden: Vec<&str> = ALL_FIELDS
        .iter()
        .copied()
        .filter(|f| field_is_set(n, f) && !allowed.contains(f))
        .collect();
    if !forbidden.is_empty() {
        return Err(format!(
            "{label}: forbidden field(s) {} — kind allows only [{}]",
            forbidden.join(", "),
            allowed.join(", ")
        ));
    }
    // zero-length contract: start < end for EVERY node
    if n.start >= n.end {
        return Err(format!(
            "{label}: node span is zero-length or inverted; start < end is required for every normalized node"
        ));
    }
    // parent containment
    if let Some(p) = parent {
        if n.start < p.start || n.end > p.end {
            return Err(format!(
                "{label}: escapes parent {:?} {}..{}",
                p.kind, p.start, p.end
            ));
        }
    }
    // sibling order + children
    let mut cursor = n.start;
    for c in &n.children {
        if c.start < cursor {
            return Err(format!(
                "{:?} {}..{} starts before the previous sibling ends at {}",
                c.kind, c.start, c.end, cursor
            ));
        }
        cursor = cursor.max(c.end);
        walk(c, Some(n), source)?;
    }
    // UTF-8 char boundaries (when the source is available)
    if let Some(src) = source {
        for pos in [n.start, n.end] {
            if !is_boundary(src, pos) {
                return Err(format!("{label}: bound {pos} is not a UTF-8 char boundary"));
            }
        }
    }
    // FencedCode.content interval: containment, order, empty legal
    if n.kind == NodeKind::FencedCode {
        if let Some((cs, ce)) = n.content {
            if !(n.start <= cs && cs <= ce && ce <= n.end) {
                return Err(format!(
                    "{label}: content interval {cs}..{ce} outside/unordered vs node span"
                ));
            }
            if let Some(src) = source {
                for pos in [cs, ce] {
                    if !is_boundary(src, pos) {
                        return Err(format!(
                            "{label}: content bound {pos} is not a UTF-8 char boundary"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with(child: Node) -> NormalizedDocument {
        let mut root = Node::new(NodeKind::Document, 0, child.end.max(child.start) + 1);
        root.children.push(child);
        NormalizedDocument::new(root)
    }

    fn fenced(info: bool, content: Option<(usize, usize)>) -> Node {
        let mut n = Node::new(NodeKind::FencedCode, 0, 10);
        if info {
            n.info = Some(String::new());
        }
        n.content = content;
        n
    }

    #[test]
    fn forbidden_fields_are_rejected() {
        // CodeSpan content=... (the corrected MAJOR — must never return)
        let mut cs = Node::new(NodeKind::CodeSpan, 0, 5);
        cs.content = Some((1, 4));
        assert!(validate_normalized(&doc_with(cs), None)
            .unwrap_err()
            .contains("forbidden field(s) content"));
        // Text destination=...
        let mut t = Node::new(NodeKind::Text, 0, 5);
        t.destination = Some("/p".into());
        assert!(validate_normalized(&doc_with(t), None)
            .unwrap_err()
            .contains("forbidden field(s) destination"));
        // Paragraph level=...
        let mut p = Node::new(NodeKind::Paragraph, 0, 5);
        p.level = Some(1);
        assert!(validate_normalized(&doc_with(p), None)
            .unwrap_err()
            .contains("forbidden field(s) level"));
    }

    #[test]
    fn missing_required_fields_are_rejected() {
        // FencedCode missing content
        let f = fenced(true, None);
        assert!(validate_normalized(&doc_with(f), None)
            .unwrap_err()
            .contains("missing required field(s) content"));
        // FencedCode missing info
        let f = fenced(false, Some((3, 7)));
        assert!(validate_normalized(&doc_with(f), None)
            .unwrap_err()
            .contains("missing required field(s) info"));
        // ReferenceLink missing destination
        let mut rl = Node::new(NodeKind::ReferenceLink, 0, 5);
        rl.label = Some("r".into());
        assert!(validate_normalized(&doc_with(rl), None)
            .unwrap_err()
            .contains("missing required field(s) destination"));
        // Heading missing level
        let h = Node::new(NodeKind::Heading, 0, 5);
        assert!(validate_normalized(&doc_with(h), None)
            .unwrap_err()
            .contains("missing required field(s) level"));
    }

    #[test]
    fn legal_shapes_pass() {
        // CodeSpan with no fields
        let cs = Node::new(NodeKind::CodeSpan, 0, 5);
        assert!(validate_normalized(&doc_with(cs), None).is_ok());
        // FencedCode with info + content
        let f = fenced(true, Some((3, 7)));
        assert!(validate_normalized(&doc_with(f), None).is_ok());
        // ReferenceLink with label + destination
        let mut rl = Node::new(NodeKind::ReferenceLink, 0, 5);
        rl.label = Some("r".into());
        rl.destination = Some("/p".into());
        assert!(validate_normalized(&doc_with(rl), None).is_ok());
    }

    #[test]
    fn zero_length_nodes_are_rejected() {
        // zero-length FencedCode NODE: the node is otherwise COMPLETE
        // (info + content fields set), so the ONLY violation is its span
        let mut f = fenced(true, Some((5, 5)));
        f.start = 5;
        f.end = 5;
        let mut root = Node::new(NodeKind::Document, 0, 11);
        root.children.push(f);
        assert!(validate_normalized(&NormalizedDocument::new(root), None)
            .unwrap_err()
            .contains("zero-length or inverted"));
        // zero-length Text
        let mut root = Node::new(NodeKind::Document, 0, 6);
        root.children.push(Node::new(NodeKind::Text, 2, 2));
        assert!(validate_normalized(&NormalizedDocument::new(root), None)
            .unwrap_err()
            .contains("zero-length or inverted"));
    }

    #[test]
    fn empty_content_interval_is_legal() {
        // non-empty fenced node, empty body: content 5..5 inside 0..10
        let f = fenced(true, Some((5, 5)));
        assert!(validate_normalized(&doc_with(f), None).is_ok());
    }

    #[test]
    fn content_interval_must_sit_inside_the_node() {
        let mut f = fenced(true, Some((5, 5)));
        f.end = 4; // node smaller than its content interval
        assert!(validate_normalized(&doc_with(f), None).is_err());
    }

    #[test]
    fn boundary_checks_need_the_source() {
        // "中文" = 6 bytes; a Text ending mid-char is rejected only
        // when the source is provided
        let src = "中文\n".as_bytes();
        let mut root = Node::new(NodeKind::Document, 0, src.len());
        let mut p = Node::new(NodeKind::Paragraph, 0, 5); // mid-char end
        p.children.push(Node::new(NodeKind::Text, 0, 5));
        root.children.push(p);
        let doc = NormalizedDocument::new(root);
        assert!(validate_normalized(&doc, Some(src))
            .unwrap_err()
            .contains("char boundary"));
    }
}
