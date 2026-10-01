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
    /// Stable diagnostic tag. Renumbering is a representation change and
    /// requires a documented version bump.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The vocabulary keeps its field discipline under construction:
    /// `new` starts every field None, and derived equality compares all
    /// of them (a stray set field fails equality rather than hiding).
    #[test]
    fn nodes_compare_structurally() {
        let mut a = Node::new(NodeKind::Heading, 0, 5);
        a.level = Some(2);
        let mut b = Node::new(NodeKind::Heading, 0, 5);
        b.level = Some(2);
        assert_eq!(a, b);
        b.level = Some(3);
        assert_ne!(a, b);
    }
}
