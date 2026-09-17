//! NORMALIZED-RESULT-v1 — the frozen semantic vocabulary (R3 freeze).
//!
//! Authority: `grammar/NORMALIZED-RESULT-v1.md` (single owner) +
//! `protocol/R0-METHODOLOGY.md` §5 (normalized result contract). This
//! module implements that vocabulary EXACTLY: exactly the 13 frozen node
//! kinds, exactly the frozen fields, UTF-8 byte half-open spans relative
//! to the whole document, resolved reference destinations as required
//! facts, and no identity/mechanism state of any kind.
//!
//! Nothing in this file knows about mechanisms, horses, timers, or
//! allocation identity. Structural equality is derived ([`PartialEq`]);
//! correctness comparison compares the actual normalized structure.

use sha2::{Digest, Sha256};

/// The closed node vocabulary of NORMALIZED-RESULT-v1 §1. No other kinds
/// exist; no error nodes; no trivia nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Document,
    Paragraph,
    Heading,
    BlockQuote,
    List,
    ListItem,
    FencedCode,
    Text,
    Emphasis,
    CodeSpan,
    Link,
    ReferenceLink,
    ReferenceDefinition,
}

impl NodeKind {
    /// Stable kind tag used by the canonical checksum encoding (§7 of the
    /// R4 stage record). Values are frozen with `corpus-gen`-style
    /// discipline: renumbering changes every checksum and requires a
    /// documented version bump.
    pub fn tag(self) -> u8 {
        match self {
            NodeKind::Document => 0,
            NodeKind::Paragraph => 1,
            NodeKind::Heading => 2,
            NodeKind::BlockQuote => 3,
            NodeKind::List => 4,
            NodeKind::ListItem => 5,
            NodeKind::FencedCode => 6,
            NodeKind::Text => 7,
            NodeKind::Emphasis => 8,
            NodeKind::CodeSpan => 9,
            NodeKind::Link => 10,
            NodeKind::ReferenceLink => 11,
            NodeKind::ReferenceDefinition => 12,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            NodeKind::Document => "Document",
            NodeKind::Paragraph => "Paragraph",
            NodeKind::Heading => "Heading",
            NodeKind::BlockQuote => "BlockQuote",
            NodeKind::List => "List",
            NodeKind::ListItem => "ListItem",
            NodeKind::FencedCode => "FencedCode",
            NodeKind::Text => "Text",
            NodeKind::Emphasis => "Emphasis",
            NodeKind::CodeSpan => "CodeSpan",
            NodeKind::Link => "Link",
            NodeKind::ReferenceLink => "ReferenceLink",
            NodeKind::ReferenceDefinition => "ReferenceDefinition",
        }
    }
}

/// One normalized node: kind, span, frozen value fields, ordered children.
///
/// Value fields are `None` unless the node kind pins them
/// (NORMALIZED-RESULT-v1 §1). A `Some` on any other kind is a vocabulary
/// violation; structural equality compares them, so a stray field fails
/// equality rather than hiding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    /// UTF-8 byte span `[start, end)` relative to the complete document.
    pub start: usize,
    pub end: usize,
    /// `Heading.level` (1..=6).
    pub level: Option<u8>,
    /// `ListItem.marker` (`"-"` or `"*"`).
    pub marker: Option<String>,
    /// `FencedCode.info` (may be empty).
    pub info: Option<String>,
    /// `FencedCode.content` raw byte interval `[content_start,
    /// content_end)`; the only zero-length interval in the vocabulary.
    pub content: Option<(usize, usize)>,
    /// `Link.destination` / `ReferenceLink.destination` (resolved) /
    /// `ReferenceDefinition.destination`.
    pub destination: Option<String>,
    /// `ReferenceLink.label` / `ReferenceDefinition.label` (normalized).
    pub label: Option<String>,
    pub children: Vec<Node>,
}

impl Node {
    /// A node with no fields set and no children.
    pub fn new(kind: NodeKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            start,
            end,
            level: None,
            marker: None,
            info: None,
            content: None,
            destination: None,
            label: None,
            children: Vec::new(),
        }
    }
}

/// The complete normalized result: exactly one [`NodeKind::Document`]
/// root whose span is `[0, len)` of the complete document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedDocument {
    pub root: Node,
}

impl NormalizedDocument {
    pub fn new(root: Node) -> Self {
        debug_assert_eq!(root.kind, NodeKind::Document);
        Self { root }
    }

    pub fn len_bytes(&self) -> usize {
        self.root.end
    }
}

// ---------------------------------------------------------------------------
// Deterministic normalized checksum
// ---------------------------------------------------------------------------

/// Deterministic checksum of the fully materialized normalized result.
///
/// Derivation (documented, stable): the document is serialized into a
/// canonical byte encoding — per node: kind tag (u8), span (2 x u64
/// little-endian), field presence flags (u8), then each present field
/// (level u8, marker/info/label/destination as u64 length + UTF-8 bytes,
/// content as 2 x u64) — depth-first, children after fields, then
/// SHA-256 over the encoding; the checksum is the first 8 bytes read as
/// a big-endian u64.
///
/// Properties: stable across runs; independent of allocation addresses,
/// `HashMap` iteration, and Rust hash seeds (no `DefaultHasher`); derived
/// only from normalized semantic content. NOT a correctness proof — the
/// oracle compares structure; this fills `Completed::result_checksum`.
pub fn normalized_checksum(doc: &NormalizedDocument) -> u64 {
    let mut enc = Vec::new();
    encode_node(&mut enc, &doc.root);
    let digest = Sha256::digest(&enc);
    let mut be = [0u8; 8];
    be.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(be)
}

fn push_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    push_u64(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn encode_node(out: &mut Vec<u8>, node: &Node) {
    out.push(node.kind.tag());
    push_u64(out, node.start as u64);
    push_u64(out, node.end as u64);
    let mut flags = 0u8;
    if node.level.is_some() {
        flags |= 1;
    }
    if node.marker.is_some() {
        flags |= 1 << 1;
    }
    if node.info.is_some() {
        flags |= 1 << 2;
    }
    if node.content.is_some() {
        flags |= 1 << 3;
    }
    if node.destination.is_some() {
        flags |= 1 << 4;
    }
    if node.label.is_some() {
        flags |= 1 << 5;
    }
    out.push(flags);
    if let Some(level) = node.level {
        out.push(level);
    }
    if let Some(m) = &node.marker {
        push_str(out, m);
    }
    if let Some(i) = &node.info {
        push_str(out, i);
    }
    if let Some((a, b)) = node.content {
        push_u64(out, a as u64);
        push_u64(out, b as u64);
    }
    if let Some(d) = &node.destination {
        push_str(out, d);
    }
    if let Some(l) = &node.label {
        push_str(out, l);
    }
    push_u64(out, node.children.len() as u64);
    for child in &node.children {
        encode_node(out, child);
    }
}

// ---------------------------------------------------------------------------
// QUERY — NODE_PATH_AT (NORMALIZED-RESULT-v1 §4, frozen contract)
// ---------------------------------------------------------------------------

/// One reported node of a NODE_PATH_AT answer: kind plus `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathNode {
    pub kind: NodeKind,
    pub start: usize,
    pub end: usize,
}

/// `NODE_PATH_AT(offset)`: the ordered path root -> deepest node where
/// every node `n` satisfies `n.start <= offset < n.end`.
///
/// Containment is half-open; at whitespace gaps the path is just
/// `[Document]`; `offset == len(source)` is the single end-inclusive
/// special case and also answers `[Document]`. Defined over the
/// normalized tree only; this traversal performs no parsing and needs no
/// index (H0's R4 implementation; a mechanism may answer differently per
/// the R0 §7.3 fairness rule).
///
/// Panics if `offset > len` (contract violation by the caller).
pub fn node_path_at(doc: &NormalizedDocument, offset: usize) -> Vec<PathNode> {
    let root = &doc.root;
    assert!(offset <= root.end, "NODE_PATH_AT offset out of range");
    let mut path = vec![PathNode {
        kind: root.kind,
        start: root.start,
        end: root.end,
    }];
    // EOF special case: containment fails for every node; the frozen
    // answer is the single-node path.
    if offset >= root.end {
        return path;
    }
    let mut current = root;
    'descend: loop {
        for child in &current.children {
            if child.start <= offset && offset < child.end {
                path.push(PathNode {
                    kind: child.kind,
                    start: child.start,
                    end: child.end,
                });
                current = child;
                continue 'descend;
            }
        }
        return path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(spans: &[(NodeKind, usize, usize)]) -> NormalizedDocument {
        // builds a linear chain Document -> k1 -> k2 ... for path tests
        let mut root = Node::new(NodeKind::Document, 0, 100);
        let mut cur = &mut root;
        for &(kind, s, e) in spans {
            let n = Node::new(kind, s, e);
            cur.children.push(n);
            cur = cur.children.last_mut().unwrap();
        }
        NormalizedDocument::new(root)
    }

    #[test]
    fn checksum_is_deterministic_and_content_sensitive() {
        let mut a = Node::new(NodeKind::Paragraph, 0, 5);
        a.children.push(Node::new(NodeKind::Text, 0, 5));
        let d1 = NormalizedDocument::new(Node::new(NodeKind::Document, 0, 6));
        let mut d2root = Node::new(NodeKind::Document, 0, 6);
        d2root.children.push(a.clone());
        let d2 = NormalizedDocument::new(d2root);
        assert_eq!(normalized_checksum(&d1), normalized_checksum(&d1));
        assert_ne!(normalized_checksum(&d1), normalized_checksum(&d2));
        // span change -> different checksum
        let mut b = a.clone();
        b.end = 6;
        let mut d3root = Node::new(NodeKind::Document, 0, 6);
        d3root.children.push(b);
        assert_ne!(
            normalized_checksum(&d2),
            normalized_checksum(&NormalizedDocument::new(d3root))
        );
    }

    #[test]
    fn checksum_ignores_nothing_semantic() {
        // destination change must flip the checksum (semantic fact)
        let mut l1 = Node::new(NodeKind::Link, 0, 5);
        l1.destination = Some("/a".into());
        let mut l2 = Node::new(NodeKind::Link, 0, 5);
        l2.destination = Some("/b".into());
        let mk = |n: Node| {
            let mut r = Node::new(NodeKind::Document, 0, 5);
            r.children.push(n);
            NormalizedDocument::new(r)
        };
        assert_ne!(normalized_checksum(&mk(l1)), normalized_checksum(&mk(l2)));
    }

    #[test]
    fn path_at_walks_to_deepest_container() {
        let d = doc(&[(NodeKind::Paragraph, 0, 10), (NodeKind::Text, 2, 8)]);
        assert_eq!(node_path_at(&d, 4).len(), 3);
        assert_eq!(node_path_at(&d, 4)[2].kind, NodeKind::Text);
        // inside paragraph but outside text run
        assert_eq!(node_path_at(&d, 1).len(), 2);
        // gap: document only
        assert_eq!(node_path_at(&d, 50).len(), 1);
        assert_eq!(node_path_at(&d, 50)[0].kind, NodeKind::Document);
    }

    #[test]
    fn path_at_end_exclusive_and_eof() {
        let d = doc(&[(NodeKind::Paragraph, 0, 10)]);
        // offset == node end is NOT contained
        assert_eq!(node_path_at(&d, 10).len(), 1);
        // EOF special case: single-node path
        assert_eq!(node_path_at(&d, 100).len(), 1);
        assert_eq!(node_path_at(&d, 100)[0].kind, NodeKind::Document);
    }
}
