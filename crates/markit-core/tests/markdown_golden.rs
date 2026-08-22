//! Semantic golden fixtures for the Markdown L1 dialect
//! (`docs/product/markdown-l1-semantic-contract.md` §12).
//!
//! Each fixture pins the block stream (kinds, exact source bytes, line
//! spans) and, where inline structure matters, the flattened inline IR
//! of the first run. Every fixture also re-proves the tiling invariant
//! through public data only. Deviation cases cite their D-number from
//! the contract; CommonMark-aligned cases adopt the 0.31.2 behavior.

use markit_core::markdown::{BlockDetail, MarkdownState, MarkdownStateError};
use markit_core::{
    BlockKind, BlockRecord, ByteOffset, Document, EditTransaction, InlineNode, LineNumber,
    SourceRange, TextEdit,
};

struct ExpectedBlock {
    kind: BlockKind,
    text: &'static str,
    /// Flattened pre-order `(node kind, source text)` of the first
    /// inline run; empty means "no inline expectations".
    inlines: &'static [(&'static str, &'static str)],
}

const NO_INLINE: &[(&str, &str)] = &[];

fn block(kind: BlockKind, text: &'static str) -> ExpectedBlock {
    ExpectedBlock {
        kind,
        text,
        inlines: NO_INLINE,
    }
}

fn inline_block(
    kind: BlockKind,
    text: &'static str,
    inlines: &'static [(&'static str, &'static str)],
) -> ExpectedBlock {
    ExpectedBlock {
        kind,
        text,
        inlines,
    }
}

fn assert_tiling(blocks: &[BlockRecord], len: usize) {
    let mut expected_start = 0;
    for (i, b) in blocks.iter().enumerate() {
        assert_eq!(
            b.source_range.start.as_usize(),
            expected_start,
            "fixture block {i}"
        );
        assert!(
            b.source_range.end.as_usize() > expected_start
                || (i + 1 == blocks.len() && expected_start == len),
            "fixture block {i} is empty mid-document"
        );
        expected_start = b.source_range.end.as_usize();
    }
    assert_eq!(expected_start, len, "stream must end at EOF");
}

fn flatten(nodes: &[InlineNode], out: &mut Vec<InlineNode>) {
    for node in nodes {
        out.push(node.clone());
        match node {
            InlineNode::Emphasis { children, .. }
            | InlineNode::Strong { children, .. }
            | InlineNode::Link { children, .. } => flatten(children, out),
            _ => {}
        }
    }
}

fn run_fixture(name: &str, text: &str, expected: &[ExpectedBlock]) {
    let doc = Document::new(text);
    let snapshot = doc.snapshot();
    let state = MarkdownState::build(&snapshot);
    let blocks = state.blocks();

    assert_eq!(
        blocks.len(),
        expected.len(),
        "{name}: block count\n  got kinds: {:?}",
        blocks.iter().map(|b| b.kind.name()).collect::<Vec<_>>()
    );
    for (i, (got, want)) in blocks.iter().zip(expected).enumerate() {
        assert_eq!(got.kind, want.kind, "{name}: block {i} kind");
        assert_eq!(
            snapshot.slice(got.source_range).as_ref(),
            want.text,
            "{name}: block {i} bytes"
        );
        if !want.inlines.is_empty() {
            assert!(
                !got.inline.runs.is_empty(),
                "{name}: block {i} should have inline runs"
            );
            let mut flat = Vec::new();
            flatten(&got.inline.runs[0].nodes, &mut flat);
            let text_of = |node: &InlineNode| -> String {
                let range = node_range(node);
                text[range.start.as_usize()..range.end.as_usize()].to_string()
            };
            let shape: Vec<(&str, String)> =
                flat.iter().map(|n| (n.kind_name(), text_of(n))).collect();
            let want_shape: Vec<(&str, String)> = want
                .inlines
                .iter()
                .map(|(k, t)| (*k, t.to_string()))
                .collect();
            assert_eq!(shape, want_shape, "{name}: block {i} inline IR");
        }
    }
    assert_tiling(blocks, text.len());

    // Determinism: a second build is bit-identical.
    let again = MarkdownState::build(&snapshot);
    assert_eq!(
        again.blocks(),
        blocks,
        "{name}: build must be deterministic"
    );
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

#[test]
fn headings() {
    run_fixture(
        "simple",
        "# Simple\n",
        &[
            block(BlockKind::Heading, "# Simple\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "six",
        "###### Six\n",
        &[
            block(BlockKind::Heading, "###### Six\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "seven-is-paragraph",
        "####### Seven\n",
        &[
            block(BlockKind::Paragraph, "####### Seven\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "closing-sequence",
        "## Head ##\n",
        &[
            block(BlockKind::Heading, "## Head ##\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "empty-with-closer",
        "# #\n",
        &[
            block(BlockKind::Heading, "# #\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "escaped-closer-is-text",
        "# foo \\#\n",
        &[
            inline_block(BlockKind::Heading, "# foo \\#\n", &[("text", "foo \\#")]),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "indent-three",
        "   # Three\n",
        &[
            block(BlockKind::Heading, "   # Three\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    // D3/D4-family: indent four and leading tab never open.
    run_fixture(
        "indent-four",
        "    # Four\n",
        &[
            block(BlockKind::Paragraph, "    # Four\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "leading-tab",
        "\t# Tab\n",
        &[
            block(BlockKind::Paragraph, "\t# Tab\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "eof-no-newline",
        "# No final newline",
        &[block(BlockKind::Heading, "# No final newline")],
    );
    run_fixture(
        "cjk-emoji",
        "# 中文 标题 🙂\n",
        &[
            inline_block(
                BlockKind::Heading,
                "# 中文 标题 🙂\n",
                &[("text", "中文 标题 🙂")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn paragraphs_and_blanks() {
    run_fixture(
        "multi-line",
        "one\ntwo\nthree\n",
        &[
            block(BlockKind::Paragraph, "one\ntwo\nthree\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "blank-runs-collapse",
        "a\n\n\n\n\nb\n",
        &[
            block(BlockKind::Paragraph, "a\n"),
            block(BlockKind::Blank, "\n\n\n\n"),
            block(BlockKind::Paragraph, "b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "whitespace-only-lines-are-blank",
        "a\n \t \nb\n",
        &[
            block(BlockKind::Paragraph, "a\n"),
            block(BlockKind::Blank, " \t \n"),
            block(BlockKind::Paragraph, "b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture("empty-document", "", &[block(BlockKind::Blank, "")]);
    run_fixture("newline-only", "\n", &[block(BlockKind::Blank, "\n")]);
    run_fixture(
        "no-final-newline",
        "last line",
        &[block(BlockKind::Paragraph, "last line")],
    );
    run_fixture(
        "cjk-paragraph",
        "中文段落，包含 emoji 🙂 和 é。\n",
        &[
            block(BlockKind::Paragraph, "中文段落，包含 emoji 🙂 和 é。\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    // Setext stays a paragraph (D1); thematic break stays a paragraph (D2).
    run_fixture(
        "setext-is-paragraph-d1",
        "title\n===\n",
        &[
            block(BlockKind::Paragraph, "title\n===\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "thematic-break-is-paragraph-d2",
        "---\n",
        &[
            block(BlockKind::Paragraph, "---\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "indented-code-is-paragraph-d3",
        "    code stays text\n",
        &[
            block(BlockKind::Paragraph, "    code stays text\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "interrupted-by-list",
        "para\n- item\n",
        &[
            block(BlockKind::Paragraph, "para\n"),
            block(BlockKind::UnorderedList, "- item\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "empty-item-does-not-interrupt",
        "para\n-\n",
        &[
            block(BlockKind::Paragraph, "para\n-\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "ordered-must-start-at-one",
        "para\n2. x\n",
        &[
            block(BlockKind::Paragraph, "para\n2. x\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "backslash-escaped-openers-are-text",
        "\\# not a heading\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "\\# not a heading\n",
                &[("text", "\\# not a heading\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn blockquotes() {
    run_fixture(
        "basic",
        "> quoted\n",
        &[
            block(BlockKind::BlockQuote, "> quoted\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "multi-line",
        "> a\n> b\n",
        &[
            block(BlockKind::BlockQuote, "> a\n> b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "one-space-stripped",
        ">   deep\n",
        &[
            inline_block(BlockKind::BlockQuote, ">   deep\n", &[("text", "  deep")]),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "no-space-after-marker",
        ">tight\n",
        &[
            inline_block(BlockKind::BlockQuote, ">tight\n", &[("text", "tight")]),
            block(BlockKind::Blank, ""),
        ],
    );
    // D5: no lazy continuation.
    run_fixture(
        "lazy-continuation-ends-quote-d5",
        "> a\nlazy\n",
        &[
            block(BlockKind::BlockQuote, "> a\n"),
            block(BlockKind::Paragraph, "lazy\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    // D6: openers inside quote lines are literal text.
    run_fixture(
        "no-nesting-d6",
        "> # not a heading\n",
        &[
            inline_block(
                BlockKind::BlockQuote,
                "> # not a heading\n",
                &[("text", "# not a heading")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "empty-quote-line-segment",
        ">\n> a\n",
        &[
            inline_block(BlockKind::BlockQuote, ">\n> a\n", &[("text", "a")]),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "quote-interrupts-paragraph",
        "text\n> quote\n",
        &[
            block(BlockKind::Paragraph, "text\n"),
            block(BlockKind::BlockQuote, "> quote\n"),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn lists() {
    run_fixture(
        "basic-bullets",
        "- a\n- b\n",
        &[
            block(BlockKind::UnorderedList, "- a\n- b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "plus-and-star-signatures",
        "+ a\n* b\n",
        &[
            block(BlockKind::UnorderedList, "+ a\n"),
            block(BlockKind::UnorderedList, "* b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "ordered-basics",
        "1. one\n2. two\n",
        &[
            block(BlockKind::OrderedList, "1. one\n2. two\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "delimiter-change-splits",
        "1. a\n2) b\n",
        &[
            block(BlockKind::OrderedList, "1. a\n"),
            block(BlockKind::OrderedList, "2) b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "blank-terminates-d7",
        "- a\n\n- b\n",
        &[
            block(BlockKind::UnorderedList, "- a\n"),
            block(BlockKind::Blank, "\n"),
            block(BlockKind::UnorderedList, "- b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "item-continuation-d9",
        "- a\n  more text\n- b\n",
        &[
            block(BlockKind::UnorderedList, "- a\n  more text\n- b\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "nested-markers-join-flat-d8",
        "- a\n  - nested\n",
        &[
            block(BlockKind::UnorderedList, "- a\n  - nested\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "empty-items",
        "-\n-\n",
        &[
            block(BlockKind::UnorderedList, "-\n-\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "ten-digit-number-is-paragraph",
        "1234567890. x\n",
        &[
            block(BlockKind::Paragraph, "1234567890. x\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "heading-terminates-list",
        "- a\n# h\n",
        &[
            block(BlockKind::UnorderedList, "- a\n"),
            block(BlockKind::Heading, "# h\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "cjk-items",
        "1. 第一\n2. 第二\n",
        &[
            block(BlockKind::OrderedList, "1. 第一\n2. 第二\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    // Inline IR: each item is an independent run; markers belong to no node.
    run_fixture(
        "item-inline-runs",
        "- *a* x\n- y\n",
        &[
            inline_block(
                BlockKind::UnorderedList,
                "- *a* x\n- y\n",
                &[("em", "*a*"), ("text", "a"), ("text", " x\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn fenced_code() {
    run_fixture(
        "closed",
        "```\ncode\n```\n",
        &[
            block(BlockKind::FencedCode, "```\ncode\n```\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "unclosed-to-eof",
        "```\nnever closed\n",
        &[block(BlockKind::FencedCode, "```\nnever closed\n")],
    );
    run_fixture(
        "tilde-and-info",
        "~~~rust title\nfn main() {}\n~~~\n",
        &[
            block(BlockKind::FencedCode, "~~~rust title\nfn main() {}\n~~~\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "longer-closer-closes",
        "````\n```\n````\n",
        &[
            block(BlockKind::FencedCode, "````\n```\n````\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "shorter-closer-does-not-close",
        "````\n```\n````x\n",
        &[block(BlockKind::FencedCode, "````\n```\n````x\n")],
    );
    run_fixture(
        "wrong-char-closer-does-not-close",
        "```\ncode\n~~~\nstill code\n```\n",
        &[
            block(BlockKind::FencedCode, "```\ncode\n~~~\nstill code\n```\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "backtick-info-with-backtick-is-paragraph-d14",
        "``` bad ` info\n",
        &[
            block(BlockKind::Paragraph, "``` bad ` info\n"),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "content-is-opaque",
        "```\n# not a heading\n* not em *\n```\n",
        &[
            block(
                BlockKind::FencedCode,
                "```\n# not a heading\n* not em *\n```\n",
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "cjk-content-eof-no-newline",
        "~~~\n中文代码 🙂",
        &[block(BlockKind::FencedCode, "~~~\n中文代码 🙂")],
    );
    run_fixture(
        "indented-fence",
        "   ```\n x\n   ```\n",
        &[
            block(BlockKind::FencedCode, "   ```\n x\n   ```\n"),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn inline_constructs() {
    run_fixture(
        "emphasis",
        "*emphasis* plain\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "*emphasis* plain\n",
                &[
                    ("em", "*emphasis*"),
                    ("text", "emphasis"),
                    ("text", " plain\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "strong-and-nesting",
        "**a *b* c**\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "**a *b* c**\n",
                &[
                    ("strong", "**a *b* c**"),
                    ("text", "a "),
                    ("em", "*b*"),
                    ("text", "b"),
                    ("text", " c"),
                    ("text", "\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "intraword-underscore-is-text",
        "snake_case_name\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "snake_case_name\n",
                &[("text", "snake_case_name\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "emphasis-across-lines",
        "*foo\nbar*\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "*foo\nbar*\n",
                &[("em", "*foo\nbar*"), ("text", "foo\nbar"), ("text", "\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "code-span-beats-emphasis",
        "`` *not em* ``\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "`` *not em* ``\n",
                &[("code", "`` *not em* ``"), ("text", "\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "unmatched-delimiters-stay-text",
        "*open _also\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "*open _also\n",
                &[("text", "*open _also\n")],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "link-with-title",
        "[text](https://example.com \"title\")\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "[text](https://example.com \"title\")\n",
                &[
                    ("link", "[text](https://example.com \"title\")"),
                    ("text", "text"),
                    ("text", "\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "image-degrades-to-bang-plus-link-d12",
        "![alt](u)\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "![alt](u)\n",
                &[
                    ("text", "!"),
                    ("link", "[alt](u)"),
                    ("text", "alt"),
                    ("text", "\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "reference-links-stay-text-d12",
        "[a][b]\n",
        &[
            inline_block(BlockKind::Paragraph, "[a][b]\n", &[("text", "[a][b]\n")]),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "cjk-emphasis",
        "中文*强调*测试\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "中文*强调*测试\n",
                &[
                    ("text", "中文"),
                    ("em", "*强调*"),
                    ("text", "强调"),
                    ("text", "测试\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
    run_fixture(
        "triple-run",
        "***x***\n",
        &[
            inline_block(
                BlockKind::Paragraph,
                "***x***\n",
                &[
                    ("em", "***x***"),
                    ("strong", "**x**"),
                    ("text", "x"),
                    ("text", "\n"),
                ],
            ),
            block(BlockKind::Blank, ""),
        ],
    );
}

#[test]
fn block_details_are_queryable() {
    let doc = Document::new("## h\n\n- one\n- two\n\n```rust info\nx\n```\n");
    let snapshot = doc.snapshot();
    let state = MarkdownState::build(&snapshot);

    let heading = &state.blocks()[0];
    let BlockDetail::Heading { level, content } = &heading.detail else {
        panic!("heading");
    };
    assert_eq!(*level, 2);
    assert_eq!(snapshot.slice(*content).as_ref(), "h");

    let list = &state.blocks()[2];
    let BlockDetail::List { items, .. } = &list.detail else {
        panic!("list");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(snapshot.slice(items[1].marker_range).as_ref(), "-");
    assert_eq!(snapshot.slice(items[1].content).as_ref(), "two\n");

    let fence = state
        .blocks()
        .iter()
        .find(|b| b.kind == BlockKind::FencedCode)
        .unwrap();
    let BlockDetail::FencedCode { fence, closed } = &fence.detail else {
        panic!("fence");
    };
    assert!(*closed);
    assert_eq!(fence.fence_len, 3);
    let info = fence.info.map(|r| snapshot.slice(r).into_owned());
    assert_eq!(info.as_deref(), Some("rust info"));

    // Range queries agree with positions.
    let mid = ByteOffset(doc_line_start(&doc, 3));
    let hit = state.block_at_offset(mid).unwrap();
    assert_eq!(hit.kind, BlockKind::UnorderedList);
    assert_eq!(hit.line_span, LineNumber(2)..LineNumber(4));
}

fn doc_line_start(doc: &Document, line: usize) -> usize {
    doc.line_range(LineNumber(line)).start.as_usize()
}

// ---------------------------------------------------------------------------
// Identity battery (contract §10, task §22)
// ---------------------------------------------------------------------------

fn ids(state: &MarkdownState) -> Vec<u64> {
    state.blocks().iter().map(|b| b.id.as_u64()).collect()
}

fn edit_at(doc: &mut Document, state: &mut MarkdownState, at: usize, text: &str) {
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(at), text))
        .apply(doc)
        .expect("edit applies");
    state
        .update(&doc.snapshot(), &applied.result)
        .expect("update");
}

fn delete_range(doc: &mut Document, state: &mut MarkdownState, start: usize, end: usize) {
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::delete(SourceRange::new(
            ByteOffset(start),
            ByteOffset(end),
        )))
        .apply(doc)
        .expect("delete applies");
    state
        .update(&doc.snapshot(), &applied.result)
        .expect("update");
}

#[test]
fn identity_battery() {
    // 1 + 2: untouched blocks and text-edited blocks keep ids.
    {
        let text = "# h\n\nfirst\n\nsecond\n\nthird\n";
        let mut doc = Document::new(text);
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        let at = text.find("second").unwrap();
        edit_at(&mut doc, &mut state, at + 2, "X");
        let after = ids(&state);
        // Heading, blanks, first, third unchanged; "second" paragraph
        // edited but structurally intact.
        assert_eq!(after.len(), before.len());
        assert_eq!(after[0], before[0], "heading untouched");
        assert_eq!(after[2], before[2], "first untouched");
        assert_eq!(after[4], before[4], "edited paragraph keeps its id");
        assert_eq!(after[6], before[6], "third untouched");
    }

    // 3: kind change mints.
    {
        let mut doc = Document::new("plain\n");
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        edit_at(&mut doc, &mut state, 0, "## ");
        let after = ids(&state);
        assert_ne!(after[0], before[0], "paragraph -> heading is a new id");
    }

    // 4: heading -> paragraph mints.
    {
        let mut doc = Document::new("## head\n");
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        delete_range(&mut doc, &mut state, 0, 3);
        let after = ids(&state);
        assert_ne!(after[0], before[0]);
    }

    // 5: split — the first paragraph keeps, the second mints.
    {
        let mut doc = Document::new("one two\n");
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        edit_at(&mut doc, &mut state, 3, "\n\n");
        let after = ids(&state);
        assert_eq!(after[0], before[0], "first half keeps the id");
        assert_eq!(after.len(), 4, "paragraph, blank, paragraph, blank");
        assert!(
            after[2] > *before.iter().max().unwrap(),
            "the new paragraph has a fresh id"
        );
    }

    // 6: merge — first keeps, second retires.
    {
        let mut doc = Document::new("a\n\nb\n");
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        // Delete the blank line between the paragraphs.
        delete_range(&mut doc, &mut state, 2, 3);
        let after = ids(&state);
        assert_eq!(after[0], before[0], "leading paragraph keeps its id");
        assert_eq!(
            after.len() + 2,
            before.len(),
            "blank and second paragraph merged away"
        );
    }

    // 7: fence content edits keep the fence id.
    {
        let text = "```\ncode\n```\n";
        let mut doc = Document::new(text);
        let mut state = MarkdownState::build(&doc.snapshot());
        let fence_before = state.blocks()[0].id;
        edit_at(&mut doc, &mut state, 5, "X");
        assert_eq!(state.blocks()[0].id, fence_before);
        assert_eq!(state.blocks()[0].kind, BlockKind::FencedCode);
    }

    // 8: fence reinterpretation of the closer keeps the fence id; the
    // swallowed paragraph's id retires.
    {
        let text = "```\ncode\n```\nafter\n";
        let mut doc = Document::new(text);
        let mut state = MarkdownState::build(&doc.snapshot());
        let (fence, para) = (state.blocks()[0].id, state.blocks()[1].id);
        delete_range(&mut doc, &mut state, 10, 14);
        assert_eq!(state.blocks().len(), 1);
        assert_eq!(state.blocks()[0].id, fence, "the fence survived and grew");
        assert!(
            !state.blocks().iter().any(|b| b.id == para),
            "the swallowed paragraph's id retired"
        );
    }

    // 9: sparse two-location transaction keeps every untouched id.
    {
        let text = "aaa\n\nbbb\n\nccc\n";
        let mut doc = Document::new(text);
        let mut state = MarkdownState::build(&doc.snapshot());
        let before = ids(&state);
        let applied = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(1), "X"))
            .with_edit(TextEdit::insert(
                ByteOffset(text.find("ccc").unwrap() + 1),
                "Y",
            ))
            .apply(&mut doc)
            .unwrap();
        state.update(&doc.snapshot(), &applied.result).unwrap();
        let after = ids(&state);
        assert_eq!(after.len(), before.len());
        assert_eq!(
            after[2], before[2],
            "middle paragraph untouched between islands"
        );
        assert_eq!(after[0], before[0]);
        assert_eq!(after[4], before[4]);
        assert_eq!(state.last_work().dirty_regions, 2);
    }

    // 10: blank-run edits keep the blank id.
    {
        let mut doc = Document::new("a\n\n\n\nb\n");
        let mut state = MarkdownState::build(&doc.snapshot());
        let blank_before = state.blocks()[1].id;
        delete_range(&mut doc, &mut state, 2, 3);
        let blank_after = &state.blocks()[1];
        assert_eq!(blank_after.kind, BlockKind::Blank);
        assert_eq!(
            blank_after.id, blank_before,
            "shortened blank run keeps its id"
        );
    }
}

// ---------------------------------------------------------------------------
// Version-staleness rejection through the public API
// ---------------------------------------------------------------------------

#[test]
fn update_fails_closed_on_version_mismatches() {
    let mut doc = Document::new("seed\n");
    let mut state = MarkdownState::build(&doc.snapshot());

    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(4), "!"))
        .apply(&mut doc)
        .unwrap();
    let snapshot = doc.snapshot();

    // Replay after the state moved on: stale base.
    state.update(&snapshot, &applied.result).expect("first ok");
    assert_eq!(
        state.update(&snapshot, &applied.result),
        Err(MarkdownStateError::StaleBase)
    );

    // A result from another document, even with matching revision
    // numbers, is rejected by identity.
    let mut other = Document::new("other\n");
    let mut other_state = MarkdownState::build(&other.snapshot());
    let other_applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(5), "?"))
        .apply(&mut other)
        .unwrap();
    assert_eq!(
        other_state.update(&snapshot, &applied.result),
        Err(MarkdownStateError::DocumentMismatch)
    );

    // Snapshot at the wrong revision for the result.
    let third = Document::new("third\n");
    let mut third_state = MarkdownState::build(&third.snapshot());
    assert_eq!(
        third_state.update(&snapshot, &other_applied.result),
        Err(MarkdownStateError::DocumentMismatch)
    );

    // Rejections leave the state untouched and still usable.
    let work_before = other_state.last_work();
    let count_before = other_state.block_count();
    let rev_before = other_state.version();
    let _ = other_state.update(&snapshot, &applied.result);
    assert_eq!(other_state.last_work(), work_before);
    assert_eq!(other_state.block_count(), count_before);
    assert_eq!(other_state.version(), rev_before);
    other_state
        .update(&other.snapshot(), &other_applied.result)
        .expect("coherent update still works after rejections");
}
