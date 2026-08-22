//! Forward block parsing over a [`DocumentSnapshot`].
//!
//! One parser, two entry paths: [`MarkdownState::build`](crate::markdown::MarkdownState::build)
//! parses from line 0 to end of document; the incremental updater starts
//! the same parser at a safe restart boundary and stops it at convergence
//! (contract §9). Both therefore produce byte-identical records for the
//! same suffix — which is exactly what the differential oracle asserts.

use std::borrow::Cow;
use std::ops::Range;

use crate::markdown::block::{
    BlockDetail, BlockFingerprint, BlockKind, BlockRecord, FenceInfo, ListItem,
};
use crate::markdown::identity::InternalBlockId;
use crate::markdown::lex;
use crate::markdown::state::BlockParseState;
use crate::position::{ByteOffset, LineNumber, SourceRange};
use crate::snapshot::DocumentSnapshot;

/// A parsed block before identity is assigned.
///
/// Identical to [`BlockRecord`] minus the id: the full build mints ids in
/// order; the incremental updater pairs island blocks with dead old
/// blocks first and mints only the leftovers (contract §10).
pub(crate) struct ParsedBlock {
    kind: BlockKind,
    source_range: SourceRange,
    line_span: Range<LineNumber>,
    state_after: BlockParseState,
    fingerprint: BlockFingerprint,
    detail: BlockDetail,
}

impl ParsedBlock {
    /// The parsed block's kind.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn kind(&self) -> BlockKind {
        self.kind
    }

    /// The parsed block's detail.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn detail(&self) -> &BlockDetail {
        &self.detail
    }

    /// The parsed block's source range.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn source_range(&self) -> SourceRange {
        self.source_range
    }

    /// The parsed block's line span.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn line_span(&self) -> Range<LineNumber> {
        self.line_span.clone()
    }

    /// The parsed block's fingerprint.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn fingerprint(&self) -> BlockFingerprint {
        self.fingerprint
    }

    /// Turns this block into a record with the given identity.
    pub(crate) fn into_record(self, id: InternalBlockId) -> BlockRecord {
        BlockRecord {
            id,
            kind: self.kind,
            source_range: self.source_range,
            line_span: self.line_span,
            // The parser only ever starts a block at a Ground boundary
            // (fence consumption happens inside the fence block itself).
            state_before: BlockParseState::Ground,
            state_after: self.state_after,
            fingerprint: self.fingerprint,
            detail: self.detail,
        }
    }
}

/// Line-by-line forward parser of the flat L1 block stream.
pub(crate) struct BlockParser<'a> {
    snap: &'a DocumentSnapshot<'a>,
    /// Next line to consume (0-based); `== line_count` means end of
    /// document.
    line: usize,
    /// Structural work counters for this parse.
    lines_scanned: u64,
    bytes_scanned: u64,
}

impl<'a> BlockParser<'a> {
    /// Parser positioned at the start of `from_line` (may equal the line
    /// count, i.e. end of document).
    pub(crate) fn new(snap: &'a DocumentSnapshot<'a>, from_line: usize) -> Self {
        assert!(
            from_line <= snap.line_count(),
            "restart line {from_line} beyond line count {}",
            snap.line_count()
        );
        Self {
            snap,
            line: from_line,
            lines_scanned: 0,
            bytes_scanned: 0,
        }
    }

    /// Whether the whole document has been consumed.
    pub(crate) fn at_eof(&self) -> bool {
        self.line >= self.snap.line_count()
    }

    /// The line the next block would start on.
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn current_line(&self) -> usize {
        self.line
    }

    /// The byte offset the next block would start at (document length at
    /// end of document). This is the parser-state position used for the
    /// convergence check (contract §9.2).
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn current_offset(&self) -> usize {
        self.line_start(self.line).as_usize()
    }

    /// Lines consumed so far.
    pub(crate) fn lines_scanned(&self) -> u64 {
        self.lines_scanned
    }

    /// Block bytes fingerprinted so far (the parser's byte cost; it never
    /// reads a byte it does not emit into a record).
    pub(crate) fn bytes_scanned(&self) -> u64 {
        self.bytes_scanned
    }

    /// Parses and returns the next block, or `None` at end of document.
    pub(crate) fn next_block(&mut self) -> Option<ParsedBlock> {
        if self.at_eof() {
            return None;
        }
        let first = self.line;
        let first_text = self.line_text(first);

        let (kind, state_after, detail) = if lex::is_blank_line(&first_text) {
            self.consume_blank_run()
        } else if lex::fence_opener(&first_text).is_some() {
            self.consume_fenced_code(first, &first_text)
        } else if let Some(atx) = lex::atx_heading(&first_text) {
            self.line += 1;
            let level = atx.level;
            let start = self.line_start(first);
            let content = SourceRange::new(
                offset_at(start, atx.content.start),
                offset_at(start, atx.content.end),
            );
            (
                BlockKind::Heading,
                BlockParseState::Ground,
                BlockDetail::Heading { level, content },
            )
        } else if lex::blockquote_line(&first_text).is_some() {
            self.consume_blockquote()
        } else if lex::list_marker(&first_text).is_some() {
            self.consume_list()
        } else {
            self.consume_paragraph()
        };

        let last = self.line - 1;
        let source_range = SourceRange::new(self.line_start(first), self.line_end(last));
        let fingerprint = BlockFingerprint::from_bytes(self.snap.slice(source_range).as_bytes());
        self.lines_scanned += (self.line - first) as u64;
        self.bytes_scanned += source_range.len() as u64;
        Some(ParsedBlock {
            kind,
            source_range,
            line_span: LineNumber(first)..LineNumber(self.line),
            state_after,
            fingerprint,
            detail,
        })
    }

    // -- kind consumers ----------------------------------------------------

    fn consume_blank_run(&mut self) -> (BlockKind, BlockParseState, BlockDetail) {
        while !self.at_eof() && lex::is_blank_line(&self.line_text(self.line)) {
            self.line += 1;
        }
        (
            BlockKind::Blank,
            BlockParseState::Ground,
            BlockDetail::Blank,
        )
    }

    fn consume_fenced_code(
        &mut self,
        opener_line: usize,
        opener_text: &str,
    ) -> (BlockKind, BlockParseState, BlockDetail) {
        let open = lex::fence_opener(opener_text).expect("caller classified a fence opener");
        let start = self.line_start(opener_line);
        let info = open
            .info
            .map(|r| SourceRange::new(offset_at(start, r.start), offset_at(start, r.end)));
        self.line += 1;
        let mut closed = false;
        while !self.at_eof() {
            let text = self.line_text(self.line);
            if lex::is_fence_closer(&text, open.fence_char, open.fence_len) {
                self.line += 1;
                closed = true;
                break;
            }
            self.line += 1;
        }
        // Unclosed: honest propagation to end of document (contract §6.7).
        let state_after = if closed {
            BlockParseState::Ground
        } else {
            BlockParseState::InFence {
                fence_char: open.fence_char,
                fence_len: open.fence_len,
            }
        };
        (
            BlockKind::FencedCode,
            state_after,
            BlockDetail::FencedCode {
                fence: FenceInfo {
                    fence_char: open.fence_char,
                    fence_len: open.fence_len,
                    info,
                },
                closed,
            },
        )
    }

    fn consume_blockquote(&mut self) -> (BlockKind, BlockParseState, BlockDetail) {
        let mut content = Vec::new();
        while !self.at_eof() {
            let line = self.line;
            let text = self.line_text(line);
            match lex::blockquote_line(&text) {
                Some(shape) => {
                    let start = self.line_start(line);
                    content.push(SourceRange::new(
                        offset_at(start, shape.content.start),
                        offset_at(start, shape.content.end),
                    ));
                    self.line += 1;
                }
                None => break,
            }
        }
        (
            BlockKind::BlockQuote,
            BlockParseState::Ground,
            BlockDetail::BlockQuote { content },
        )
    }

    fn consume_list(&mut self) -> (BlockKind, BlockParseState, BlockDetail) {
        let first = self.line;
        let first_marker =
            lex::list_marker(&self.line_text(first)).expect("caller classified a list");
        let signature = first_marker.signature;

        // Items are finalized when the next marker line (or termination)
        // is seen; an item's content run spans its marker-line text plus
        // all following continuation lines, contiguous in source.
        let mut items: Vec<ListItem> = Vec::new();
        let mut pending: Option<(usize, usize, Option<u32>)> = None; // (line, content_start_abs, number)

        while !self.at_eof() {
            let line = self.line;
            let text = self.line_text(line);
            if lex::is_blank_line(&text) {
                break; // D7: a blank line terminates the list
            }
            if let Some(marker) = lex::list_marker(&text) {
                if marker.signature == signature {
                    if let Some((start_line, content_start, number)) = pending.take() {
                        items.push(self.list_item(start_line, content_start, line - 1, number));
                    }
                    let line_start = self.line_start(line);
                    pending = Some((
                        line,
                        offset_at(line_start, marker.content_start).as_usize(),
                        marker.number,
                    ));
                    self.line += 1;
                    continue;
                }
                break; // different signature opens a new list
            }
            if lex::fence_opener(&text).is_some()
                || lex::atx_heading(&text).is_some()
                || lex::blockquote_line(&text).is_some()
            {
                break; // openers terminate the flat list (contract §6.5)
            }
            self.line += 1; // item continuation text (D9)
        }
        if let Some((start_line, content_start, number)) = pending.take() {
            items.push(self.list_item(start_line, content_start, self.line - 1, number));
        }

        let kind = match signature {
            crate::markdown::block::ListSignature::Bullet { .. } => BlockKind::UnorderedList,
            crate::markdown::block::ListSignature::Ordered { .. } => BlockKind::OrderedList,
        };
        (
            kind,
            BlockParseState::Ground,
            BlockDetail::List { signature, items },
        )
    }

    fn list_item(
        &self,
        marker_line: usize,
        content_start: usize,
        last_line: usize,
        number: Option<u32>,
    ) -> ListItem {
        let marker_text = lex::list_marker(&self.line_text(marker_line)).expect("marker line");
        let line_start = self.line_start(marker_line);
        ListItem {
            marker_range: SourceRange::new(
                offset_at(line_start, marker_text.marker.start),
                offset_at(line_start, marker_text.marker.end),
            ),
            content: SourceRange::new(ByteOffset(content_start), self.line_end(last_line)),
            number,
        }
    }

    fn consume_paragraph(&mut self) -> (BlockKind, BlockParseState, BlockDetail) {
        while !self.at_eof() {
            let text = self.line_text(self.line);
            if lex::is_blank_line(&text)
                || lex::fence_opener(&text).is_some()
                || lex::atx_heading(&text).is_some()
                || lex::blockquote_line(&text).is_some()
                || interrupts_paragraph(&text)
            {
                break;
            }
            self.line += 1;
        }
        (
            BlockKind::Paragraph,
            BlockParseState::Ground,
            BlockDetail::Paragraph,
        )
    }

    // -- line plumbing ------------------------------------------------------

    fn line_text(&self, line: usize) -> Cow<'a, str> {
        self.snap.line_str(LineNumber(line))
    }

    fn line_start(&self, line: usize) -> ByteOffset {
        if line >= self.snap.line_count() {
            return ByteOffset(self.snap.len_bytes());
        }
        self.snap.line_range(LineNumber(line)).start
    }

    /// End of `line` including its `'\n'` terminator.
    fn line_end(&self, line: usize) -> ByteOffset {
        if line + 1 < self.snap.line_count() {
            self.snap.line_range(LineNumber(line + 1)).start
        } else {
            ByteOffset(self.snap.len_bytes())
        }
    }
}

/// Whether a list marker line interrupts a paragraph (contract §5,
/// CommonMark rules adopted verbatim): the item must be non-empty, and an
/// ordered one must start at 1.
fn interrupts_paragraph(line: &str) -> bool {
    match lex::list_marker(line) {
        Some(marker) => {
            let non_empty = marker.content_start < line.len();
            let starts_at_one = marker.number.is_none_or(|n| n == 1);
            non_empty && starts_at_one
        }
        None => false,
    }
}

fn offset_at(line_start: ByteOffset, relative: usize) -> ByteOffset {
    ByteOffset(line_start.as_usize() + relative)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn kinds(text: &str) -> Vec<BlockKind> {
        let doc = Document::new(text);
        let snap = doc.snapshot();
        let mut parser = BlockParser::new(&snap, 0);
        let mut out = Vec::new();
        while let Some(block) = parser.next_block() {
            out.push(block.kind());
        }
        out
    }

    #[test]
    fn classifies_the_l1_constructs() {
        use BlockKind::*;
        assert_eq!(
            kinds("# 标题\n\n段落 *强调* 🙂\n\n> 引用行\n\n- a\n- b\n\n1. one\n2. two\n\n```\ncode\n```\n"),
            vec![Heading, Blank, Paragraph, Blank, BlockQuote, Blank, UnorderedList, Blank, OrderedList, Blank, FencedCode, Blank]
        );
    }

    #[test]
    fn paragraph_interruption_rules() {
        use BlockKind::*;
        // Blank-separated simple case (the trailing `\n` is a final
        // empty line, i.e. a Blank block — tiling covers every byte).
        assert_eq!(kinds("foo\n- bar\n"), vec![Paragraph, UnorderedList, Blank]);
        // Empty item does not interrupt.
        assert_eq!(kinds("foo\n-\n"), vec![Paragraph, Blank]);
        // Ordered interruption must start at 1.
        assert_eq!(kinds("foo\n2. x\n"), vec![Paragraph, Blank]);
        assert_eq!(kinds("foo\n1. x\n"), vec![Paragraph, OrderedList, Blank]);
        // Heading, fence, quote always interrupt.
        assert_eq!(kinds("a\n## h\n"), vec![Paragraph, Heading, Blank]);
        assert_eq!(kinds("a\n```\n"), vec![Paragraph, FencedCode]);
        assert_eq!(kinds("a\n> q\n"), vec![Paragraph, BlockQuote, Blank]);
    }

    #[test]
    fn deviations_stay_paragraphs() {
        use BlockKind::*;
        assert_eq!(kinds("setext\n===\n"), vec![Paragraph, Blank]); // D1
        assert_eq!(kinds("---\n"), vec![Paragraph, Blank]); // D2
        assert_eq!(kinds("    indented code\n"), vec![Paragraph, Blank]); // D3
        assert_eq!(kinds("\t- not a list\n"), vec![Paragraph, Blank]); // D4
        assert_eq!(kinds("    - x\n"), vec![Paragraph, Blank]); // indent 4
        assert_eq!(kinds("####### seven\n"), vec![Paragraph, Blank]); // 7 hashes
    }

    #[test]
    fn quotes_end_without_laziness() {
        use BlockKind::*;
        assert_eq!(kinds("> a\nb\n"), vec![BlockQuote, Paragraph, Blank]); // D5
        assert_eq!(
            kinds("> a\n> b\n\n> c\n"),
            vec![BlockQuote, Blank, BlockQuote, Blank]
        );
    }

    #[test]
    fn lists_split_on_signature_and_blank() {
        use BlockKind::*;
        assert_eq!(
            kinds("- a\n* b\n"),
            vec![UnorderedList, UnorderedList, Blank]
        );
        assert_eq!(kinds("1. a\n2) b\n"), vec![OrderedList, OrderedList, Blank]);
        assert_eq!(
            kinds("- a\n\n- b\n"),
            vec![UnorderedList, Blank, UnorderedList, Blank]
        ); // D7
           // Continuation text and nested-looking markers stay in one item/list (D8/D9).
        assert_eq!(kinds("- a\n  continued\n- b\n"), vec![UnorderedList, Blank]);
        // Openers terminate the list instead of nesting.
        assert_eq!(kinds("- a\n## h\n"), vec![UnorderedList, Heading, Blank]);
        assert_eq!(
            kinds("- a\n```\n```\n"),
            vec![UnorderedList, FencedCode, Blank]
        );
    }

    #[test]
    fn unclosed_fence_runs_to_eof() {
        let doc = Document::new("para\n\n```rust\nlet x = 1;\nstill code");
        let snap = doc.snapshot();
        let mut parser = BlockParser::new(&snap, 0);
        parser.next_block().unwrap(); // paragraph
        parser.next_block().unwrap(); // blank
        let fence = parser.next_block().unwrap();
        let BlockDetail::FencedCode {
            fence: info,
            closed,
        } = fence.detail().clone()
        else {
            panic!("expected fence");
        };
        assert!(!closed);
        assert_eq!(info.fence_len, 3);
        let info_range = info.info.map(|r| (r.start.as_usize(), r.end.as_usize()));
        assert_eq!(info_range, Some((9, 13)), "trimmed info 'rust'");
        match fence.state_after {
            BlockParseState::InFence {
                fence_char,
                fence_len,
            } => {
                assert_eq!(fence_char.as_char(), '`');
                assert_eq!(fence_len, 3);
            }
            ref other => panic!("expected InFence, got {other:?}"),
        }
        assert!(parser.at_eof());
    }

    #[test]
    fn fence_close_needs_same_char_and_length() {
        use BlockKind::*;
        assert_eq!(kinds("```\n~~~\n```\n"), vec![FencedCode, Blank]);
        assert_eq!(kinds("````\n```\n````\n"), vec![FencedCode, Blank]);
        assert_eq!(kinds("```\n`` x\n```\n"), vec![FencedCode, Blank]);
        // A backtick fence with backticks in the info is not an opener (D14).
        assert_eq!(kinds("``` bad ` info\n"), vec![Paragraph, Blank]);
    }

    #[test]
    fn line_spans_and_details() {
        let doc = Document::new("# H1 #\n- 一\n  二\n1) x\n");
        let snap = doc.snapshot();
        let mut parser = BlockParser::new(&snap, 0);

        let heading = parser.next_block().unwrap();
        let BlockDetail::Heading { level, content } = heading.detail().clone() else {
            panic!("heading");
        };
        assert_eq!(level, 1);
        assert_eq!(&snap.slice(content), "H1", "closer stripped");

        let list = parser.next_block().unwrap();
        let BlockDetail::List { signature, items } = list.detail().clone() else {
            panic!("list");
        };
        assert_eq!(
            signature,
            crate::markdown::block::ListSignature::Bullet { marker: '-' }
        );
        assert_eq!(items.len(), 1);
        assert_eq!(&snap.slice(items[0].marker_range), "-");
        assert_eq!(
            &snap.slice(items[0].content),
            "一\n  二\n",
            "continuation joins the item (run keeps the line terminator)"
        );
        assert_eq!(items[0].number, None);

        let ordered = parser.next_block().unwrap();
        let BlockDetail::List { items, .. } = ordered.detail().clone() else {
            panic!("ordered");
        };
        assert_eq!(items[0].number, Some(1));
        parser.next_block().unwrap(); // trailing blank line
        assert!(parser.at_eof());
    }

    #[test]
    fn blockquote_segments_strip_marker() {
        let doc = Document::new("> a\n>   spaced\n>");
        let snap = doc.snapshot();
        let mut parser = BlockParser::new(&snap, 0);
        let quote = parser.next_block().unwrap();
        let BlockDetail::BlockQuote { content } = quote.detail().clone() else {
            panic!("quote");
        };
        let texts: Vec<String> = content
            .iter()
            .map(|r| snap.slice(*r).into_owned())
            .collect();
        assert_eq!(texts, vec!["a", "  spaced", ""]);
        assert!(parser.at_eof());
    }

    #[test]
    fn restart_from_mid_document_matches_full_parse() {
        // The property the incremental updater leans on: parsing from any
        // Ground boundary yields the same suffix records.
        let doc = Document::new("# h\n\ntext\n\n- a\n- b\n\n```\nx\n```\n");
        let snap = doc.snapshot();
        let mut full = BlockParser::new(&snap, 0);
        let mut full_blocks = Vec::new();
        while let Some(b) = full.next_block() {
            full_blocks.push(b);
        }
        // Restarting from every *block boundary* of the full parse
        // reproduces the exact suffix stream (restarting mid-block is
        // not a safe boundary and is allowed to differ — the updater
        // always rewinds to a boundary).
        let mut boundaries: Vec<usize> =
            full_blocks.iter().map(|b| b.line_span().start.0).collect();
        boundaries.push(snap.line_count());
        for split in boundaries {
            let mut restart = BlockParser::new(&snap, split);
            let mut suffix = Vec::new();
            while let Some(b) = restart.next_block() {
                suffix.push(b);
            }
            let expected: Vec<&_> = full_blocks
                .iter()
                .filter(|b| b.line_span().end.0 > split)
                .collect();
            assert_eq!(suffix.len(), expected.len(), "split {split}");
            for (got, want) in suffix.iter().zip(expected) {
                assert_eq!(got.fingerprint(), want.fingerprint(), "split {split}");
                assert_eq!(got.source_range(), want.source_range(), "split {split}");
                assert_eq!(got.line_span(), want.line_span(), "split {split}");
            }
        }
    }
}
