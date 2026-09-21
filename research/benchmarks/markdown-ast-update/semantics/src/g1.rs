//! G1 lane adapter — COMMONMARK-0.31.2+GFM-TABLES through the pinned
//! independent oracle.
//!
//! Configuration is frozen by `lanes::G1_ORACLE_OPTIONS`: exactly
//! `Options::ENABLE_TABLES`, every other option off. The adapter never
//! enables an extension "because the oracle supports it": enabling one
//! would change the lane identity and requires a registry change.
//!
//! The event stream is folded into a [`LaneParse`] in the shared
//! [`SyntaxKind`] vocabulary. Where a construct needs a sub-distinction
//! the spec does not expose as an event (ATX vs Setext headings, fenced
//! content intervals), the rule is a documented byte rule applied inside
//! an oracle-recognized span — never a second recognition decision.

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag};

use crate::facts::{
    FactReason, LaneScopeGrade, RecognitionStatus, Span, SyntaxFact, SyntaxKind, TableFacts,
};
use crate::lanes::{G1_GRAMMAR_ID, G1_ORACLE_VERSION};
use crate::parse::{fence_char_at, fenced_content_interval, LaneExtras, LaneNode, LaneParse};

/// Frozen G1 leaf-block counting rule (`block_kinds`).
pub const G1_BLOCK_KINDS: &[SyntaxKind] = &[
    SyntaxKind::Paragraph,
    SyntaxKind::HeadingAtx,
    SyntaxKind::HeadingSetext,
    SyntaxKind::CodeBlockFenced,
    SyntaxKind::CodeBlockIndented,
    SyntaxKind::HtmlBlock,
    SyntaxKind::ThematicBreak,
    SyntaxKind::Table,
];

/// Frozen G1 container kinds (`container_kinds`).
pub const G1_CONTAINER_KINDS: &[SyntaxKind] = &[
    SyntaxKind::BlockQuote,
    SyntaxKind::List,
    SyntaxKind::ListItem,
];

/// The frozen G1 oracle configuration: `ENABLE_TABLES` and nothing else.
pub fn oracle_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options
}

/// Oracle identity string recorded with every G1 fact.
pub fn oracle_id() -> String {
    format!("G1-ORACLE-pulldown-cmark-{G1_ORACLE_VERSION}")
}

/// Parse `source` under G1.
pub fn parse_g1(source: &str) -> LaneParse {
    let parser = Parser::new_ext(source, oracle_options());

    // Reference definitions are out of band in the oracle's event model.
    let definitions: Vec<(String, Span)> = parser
        .reference_definitions()
        .iter()
        .map(|(label, def)| (label.to_string(), Span::new(def.span.start, def.span.end)))
        .collect();

    let mut builder = Builder::new(source, definitions);
    for (event, range) in parser.into_offset_iter() {
        builder.event(event, Span::new(range.start, range.end));
    }
    builder.finish()
}

struct Builder<'a> {
    source: &'a str,
    definitions: Vec<(String, Span)>,
    stack: Vec<LaneNode>,
    root_children: Vec<LaneNode>,
    non_host_spans: Vec<Span>,
    literal_spans: Vec<Span>,
    fenced_code_content_bytes: u64,
    unexpected_tags: Vec<String>,
}

impl<'a> Builder<'a> {
    fn new(source: &'a str, definitions: Vec<(String, Span)>) -> Self {
        Self {
            source,
            definitions,
            stack: Vec::new(),
            root_children: Vec::new(),
            non_host_spans: Vec::new(),
            literal_spans: Vec::new(),
            fenced_code_content_bytes: 0,
            unexpected_tags: Vec::new(),
        }
    }

    fn push_leaf(&mut self, node: LaneNode) {
        match self.stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => self.root_children.push(node),
        }
    }

    fn event(&mut self, event: Event<'_>, span: Span) {
        match event {
            Event::Start(tag) => match self.tag_kind(&tag, span) {
                Some(kind) => {
                    let mut node = LaneNode::new(kind, span);
                    node.detail = self.tag_detail(&tag);
                    self.stack.push(node);
                }
                None => {
                    // The oracle emitted a construct this lane's frozen
                    // configuration cannot produce. Record it instead of
                    // silently relabeling it; the content is still walked.
                    self.unexpected_tags.push(format!("start:{tag:?}"));
                }
            },
            Event::End(_) => {
                if let Some(mut node) = self.stack.pop() {
                    node.span.end = node.span.end.max(span.end);
                    match node.kind {
                        SyntaxKind::CodeBlockFenced => {
                            let fence_char = fence_char_at(self.source, node.span);
                            let content =
                                fenced_content_interval(self.source, node.span, fence_char);
                            self.fenced_code_content_bytes += content.len() as u64;
                            self.non_host_spans.push(content);
                            // A candidate is resolved against the whole
                            // block (start of the opener line through the
                            // closing fence), because a probe can anchor on
                            // the opener or on the indentation before it.
                            self.literal_spans.push(Span::new(
                                crate::parse::line_start(self.source, node.span.start),
                                node.span.end,
                            ));
                        }
                        SyntaxKind::CodeBlockIndented | SyntaxKind::HtmlBlock => {
                            self.non_host_spans.push(node.span);
                            // The oracle reports an indented block from its
                            // content; its indentation is part of the
                            // construct and is literal too.
                            self.literal_spans.push(Span::new(
                                crate::parse::line_start(self.source, node.span.start),
                                node.span.end,
                            ));
                        }
                        _ => {}
                    }
                    self.push_leaf(node);
                }
            }
            Event::Text(_) => self.push_leaf(LaneNode::new(SyntaxKind::Text, span)),
            Event::Code(_) => {
                self.non_host_spans.push(span);
                self.literal_spans.push(span);
                self.push_leaf(LaneNode::new(SyntaxKind::CodeSpan, span));
            }
            Event::InlineHtml(_) => {
                self.non_host_spans.push(span);
                self.literal_spans.push(span);
                self.push_leaf(LaneNode::new(SyntaxKind::RawHtmlInline, span));
            }
            Event::Html(_) => {
                // Raw HTML-block content; the enclosing HtmlBlock node owns
                // the construct, so this is content, not a second node.
                self.non_host_spans.push(span);
            }
            Event::SoftBreak => self.push_leaf(LaneNode::new(SyntaxKind::SoftBreak, span)),
            Event::HardBreak => self.push_leaf(LaneNode::new(SyntaxKind::HardBreak, span)),
            Event::Rule => self.push_leaf(LaneNode::new(SyntaxKind::ThematicBreak, span)),
            Event::InlineMath(_) => self.push_leaf(LaneNode::new(SyntaxKind::InlineMath, span)),
            Event::DisplayMath(_) => self.push_leaf(LaneNode::new(SyntaxKind::DisplayMath, span)),
            Event::TaskListMarker(_) => {
                self.push_leaf(LaneNode::new(SyntaxKind::TaskListItem, span))
            }
            Event::FootnoteReference(_) => {
                self.unexpected_tags.push("footnote_reference".to_string())
            }
        }
    }

    fn tag_kind(&self, tag: &Tag<'_>, span: Span) -> Option<SyntaxKind> {
        let kind = match tag {
            Tag::Paragraph => SyntaxKind::Paragraph,
            Tag::Heading { .. } => {
                if self.heading_is_atx(span) {
                    SyntaxKind::HeadingAtx
                } else {
                    SyntaxKind::HeadingSetext
                }
            }
            Tag::BlockQuote(_) => SyntaxKind::BlockQuote,
            Tag::CodeBlock(CodeBlockKind::Fenced(_)) => SyntaxKind::CodeBlockFenced,
            Tag::CodeBlock(CodeBlockKind::Indented) => SyntaxKind::CodeBlockIndented,
            Tag::HtmlBlock => SyntaxKind::HtmlBlock,
            Tag::List(_) => SyntaxKind::List,
            Tag::Item => SyntaxKind::ListItem,
            Tag::Table(_) => SyntaxKind::Table,
            Tag::TableHead => SyntaxKind::TableHeaderRow,
            Tag::TableRow => SyntaxKind::TableRow,
            Tag::TableCell => SyntaxKind::TableCell,
            Tag::Emphasis => SyntaxKind::Emphasis,
            Tag::Strong => SyntaxKind::Strong,
            Tag::Strikethrough => SyntaxKind::Strikethrough,
            Tag::Link { link_type, .. } => match link_type {
                LinkType::Autolink | LinkType::Email => SyntaxKind::LinkAutolink,
                LinkType::Inline => SyntaxKind::LinkInline,
                LinkType::Reference
                | LinkType::ReferenceUnknown
                | LinkType::Collapsed
                | LinkType::CollapsedUnknown
                | LinkType::Shortcut
                | LinkType::ShortcutUnknown => SyntaxKind::LinkReference,
                LinkType::WikiLink { .. } => return None,
            },
            Tag::Image { .. } => SyntaxKind::Image,
            Tag::MetadataBlock(_) => SyntaxKind::FrontMatter,
            Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => return None,
        };
        Some(kind)
    }

    fn tag_detail(&self, tag: &Tag<'_>) -> Option<String> {
        match tag {
            Tag::CodeBlock(CodeBlockKind::Fenced(info)) if !info.is_empty() => {
                Some(format!("info={info}"))
            }
            Tag::Table(alignments) => Some(format!("alignments={alignments:?}")),
            Tag::Heading { level, .. } => Some(format!("level={level:?}")),
            _ => None,
        }
    }

    /// ATX vs Setext inside an oracle-recognized heading span: the spec does
    /// not expose the distinction as an event, so the frozen rule reads the
    /// bytes. A Setext heading's LAST line is its underline (only `=`/`-`
    /// plus spaces); anything else is ATX. Reading the last line rather than
    /// the first is what keeps a heading inside a container correct: `> # h`
    /// starts with `>` (a container prefix, not a `#`), while its last line
    /// `> # h` is not an underline.
    fn heading_is_atx(&self, span: Span) -> bool {
        let text = &self.source[span.start..span.end];
        let last_line = text.rsplit('\n').find(|line| !line.trim().is_empty());
        match last_line {
            Some(line) => {
                let content = line.trim();
                let marker = content.chars().next();
                let is_underline = matches!(marker, Some('=') | Some('-'))
                    && content.chars().all(|ch| Some(ch) == marker || ch == ' ');
                !is_underline
            }
            // No non-blank line: nothing that could be an underline.
            None => true,
        }
    }

    fn finish(mut self) -> LaneParse {
        let root = LaneNode {
            kind: SyntaxKind::Document,
            span: Span::new(0, self.source.len()),
            detail: None,
            children: std::mem::take(&mut self.root_children),
        };

        let reference_use_count = root.children.iter().map(count_reference_links).sum::<u64>();

        let mut table = TableFacts {
            table_count: 0,
            max_table_columns: 0,
            max_table_rows_including_header: 0,
        };
        count_tables(&root.children, &mut table);

        let out_of_band_facts = self
            .definitions
            .iter()
            .map(|(label, span)| SyntaxFact {
                grammar_id: G1_GRAMMAR_ID.to_string(),
                syntax_kind: SyntaxKind::ReferenceDefinition,
                source_start: span.start,
                source_end: span.end,
                recognition_status: RecognitionStatus::Recognized,
                lane_scope_grade: LaneScopeGrade::ContractDeclaredNotQualified,
                host_context: true,
                reason: FactReason::RecognizedUnderLaneOracle,
                occurrence: 0,
                detail: Some(format!("label={label}")),
            })
            .collect::<Vec<_>>();

        LaneParse {
            grammar_id: G1_GRAMMAR_ID.to_string(),
            root,
            non_host_spans: self.non_host_spans.clone(),
            literal_spans: self.literal_spans.clone(),
            extras: LaneExtras {
                fenced_code_content_bytes: self.fenced_code_content_bytes,
                reference_definition_count: self.definitions.len() as u64,
                reference_definition_counting:
                    "UNIQUE_NORMALIZED_LABELS: entries of the oracle's reference-definition map; \
                     duplicate labels collapse (the map is first-wins per the spec)"
                        .to_string(),
                reference_use_count,
                reference_use_counting:
                    "REFERENCE_LINK_EVENTS: links the oracle resolved through a reference form \
                     (reference/collapsed/shortcut)"
                        .to_string(),
                table: if table.table_count > 0 {
                    Some(table)
                } else {
                    None
                },
                math_occupancy_note:
                    "not applicable: no math semantics are enabled in G1; math-looking bytes are \
                     ordinary text and are reported as ambiguous candidates owned by the deferred \
                     G2 lane"
                        .to_string(),
                unexpected_oracle_tags: std::mem::take(&mut self.unexpected_tags),
            },
            out_of_band_facts,
        }
    }
}

fn count_reference_links(node: &LaneNode) -> u64 {
    let own = u64::from(node.kind == SyntaxKind::LinkReference);
    own + node.children.iter().map(count_reference_links).sum::<u64>()
}

fn count_tables(nodes: &[LaneNode], facts: &mut TableFacts) {
    for node in nodes {
        if node.kind == SyntaxKind::Table {
            facts.table_count += 1;
            let mut header_rows = 0u64;
            let mut body_rows = 0u64;
            let mut columns = 0u64;
            for child in &node.children {
                match child.kind {
                    SyntaxKind::TableHeaderRow => {
                        header_rows += 1;
                        columns = columns.max(
                            child
                                .children
                                .iter()
                                .filter(|cell| cell.kind == SyntaxKind::TableCell)
                                .count() as u64,
                        );
                    }
                    SyntaxKind::TableRow => body_rows += 1,
                    _ => {}
                }
            }
            facts.max_table_columns = facts.max_table_columns.max(columns);
            facts.max_table_rows_including_header = facts
                .max_table_rows_including_header
                .max(header_rows + body_rows);
        }
        count_tables(&node.children, facts);
    }
}
