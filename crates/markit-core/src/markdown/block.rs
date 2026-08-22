//! Block vocabulary: kinds, per-kind detail, and the block record.
//!
//! Everything here is derived, source-referenced state for the flat L1
//! block stream (`docs/product/markdown-l1-semantic-contract.md` §3/§8).
//! Records are value types: the parser fills them, the incremental
//! updater splices them, consumers read them. There is no text stored —
//! text belongs to the [`Document`](crate::Document); records reference it
//! by [`SourceRange`].

use std::ops::Range;

use crate::markdown::identity::InternalBlockId;
use crate::markdown::state::{BlockParseState, FenceChar};
use crate::position::{ByteOffset, LineNumber, SourceRange};

/// The seven L1 block kinds (contract §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// A maximal run of blank lines.
    Blank,
    /// A run of plain (non-opener) non-blank lines.
    Paragraph,
    /// An ATX heading: exactly one line.
    Heading,
    /// A maximal run of `>` lines.
    BlockQuote,
    /// A maximal run of same-signature `-`/`+`/`*` marker lines with
    /// their continuation text.
    UnorderedList,
    /// A maximal run of same-signature ordered marker lines with their
    /// continuation text.
    OrderedList,
    /// A fenced code block (backtick or tilde), closed or running to end
    /// of document.
    FencedCode,
}

impl BlockKind {
    /// Short stable name (counters, diagnostics, fixtures).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Blank => "blank",
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::BlockQuote => "blockquote",
            Self::UnorderedList => "ul",
            Self::OrderedList => "ol",
            Self::FencedCode => "fence",
        }
    }
}

/// FNV-1a-64 over a block's source bytes (contract §8).
///
/// Diagnostics and identity-pairing evidence only: **soundness of
/// incremental updates never depends on fingerprint equality** —
/// convergence is parser state + positional alignment (contract §9.3).
/// Fingerprints are asserted equal in the differential oracle as a
/// strengthening check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockFingerprint(u64);

impl BlockFingerprint {
    /// Fingerprint of `bytes` (a block's exact source bytes).
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        // FNV-1a 64-bit: no dependencies, incremental-friendly.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(hash)
    }

    /// Numeric form (diagnostics, oracle comparisons).
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// What makes list marker lines belong to one list (contract §6.5):
/// the bullet character, or the ordered delimiter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListSignature {
    /// Unordered: `-`, `+`, or `*`.
    Bullet {
        /// The bullet character.
        marker: char,
    },
    /// Ordered: digits plus `.` or `)`.
    Ordered {
        /// The delimiter character.
        delimiter: char,
    },
}

/// One list item: its marker and its contiguous content run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    /// The marker (`-`, `1.`, …), spaces after it excluded.
    pub marker_range: SourceRange,
    /// The item's text: from the first content byte on the marker line
    /// to the end of its last continuation line, interior newlines
    /// included. One independent inline run (contract §7.6).
    pub content: SourceRange,
    /// The item's literal number (ordered lists only).
    pub number: Option<u32>,
}

/// Fence opener facts retained for closing and presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FenceInfo {
    /// Backtick or tilde fence.
    pub fence_char: FenceChar,
    /// Length of the opening run (minimum closing length).
    pub fence_len: usize,
    /// Trimmed info-string range within the document (None when empty).
    pub info: Option<SourceRange>,
}

/// Kind-specific refinement of a [`BlockRecord`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockDetail {
    /// Blank run: no content.
    Blank,
    /// Paragraph: the inline run is the whole source range.
    Paragraph,
    /// ATX heading.
    Heading {
        /// Heading level (1–6).
        level: u8,
        /// Content between opener and closing sequence (empty possible).
        content: SourceRange,
    },
    /// Blockquote: one content segment per quote line (segments are
    /// independent inline runs; marker bytes between them belong to no
    /// node).
    BlockQuote {
        /// Per-line content segments, in order.
        content: Vec<SourceRange>,
    },
    /// List of items sharing one signature.
    List {
        /// Bullet character or ordered delimiter.
        signature: ListSignature,
        /// The list's items, in order.
        items: Vec<ListItem>,
    },
    /// Fenced code block.
    FencedCode {
        /// Opener facts.
        fence: FenceInfo,
        /// Whether a closing fence was found (`false` = runs to end of
        /// document — honest propagation, contract §6.7).
        closed: bool,
    },
}

/// One block in the flat, tiling L1 stream (contract §3.1/§8).
///
/// Records tile the document: consecutive records are adjacent
/// (`blocks[i].source_range.end == blocks[i+1].source_range.start`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRecord {
    /// Internal product identity (contract §10). Not plugin identity.
    pub id: InternalBlockId,
    /// The block kind.
    pub kind: BlockKind,
    /// Full source bytes of the block, line terminators included.
    pub source_range: SourceRange,
    /// Half-open `[first_line, last_line + 1)`.
    pub line_span: Range<LineNumber>,
    /// Parser state entering this block (Ground for every complete-parse
    /// record; kept explicit so convergence checks are honest).
    pub state_before: BlockParseState,
    /// Parser state after this block (`InFence` only for an unclosed
    /// fence at end of document).
    pub state_after: BlockParseState,
    /// FNV-1a-64 of the block's source bytes (diagnostics, pairing
    /// evidence; never load-bearing for correctness).
    pub fingerprint: BlockFingerprint,
    /// Kind-specific detail.
    pub detail: BlockDetail,
}

impl BlockRecord {
    /// The same record moved by `(byte_delta, line_delta)` — used when an
    /// edit elsewhere in the document shifts untouched blocks without
    /// reparsing them (their text, and therefore their fingerprint, is
    /// unchanged by construction).
    #[allow(dead_code)] // incremental updater (this PR)
    pub(crate) fn shifted(&self, byte_delta: i64, line_delta: i64) -> Self {
        let mut shifted = self.clone();
        shifted.source_range = shift_range(&self.source_range, byte_delta);
        shifted.line_span = LineNumber((self.line_span.start.0 as i64 + line_delta).max(0) as usize)
            ..LineNumber((self.line_span.end.0 as i64 + line_delta).max(0) as usize);
        shifted.detail = match &self.detail {
            BlockDetail::Heading { level, content } => BlockDetail::Heading {
                level: *level,
                content: shift_range(content, byte_delta),
            },
            BlockDetail::BlockQuote { content } => BlockDetail::BlockQuote {
                content: content.iter().map(|r| shift_range(r, byte_delta)).collect(),
            },
            BlockDetail::List { signature, items } => BlockDetail::List {
                signature: *signature,
                items: items
                    .iter()
                    .map(|item| ListItem {
                        marker_range: shift_range(&item.marker_range, byte_delta),
                        content: shift_range(&item.content, byte_delta),
                        number: item.number,
                    })
                    .collect(),
            },
            BlockDetail::FencedCode { fence, closed } => BlockDetail::FencedCode {
                fence: FenceInfo {
                    fence_char: fence.fence_char,
                    fence_len: fence.fence_len,
                    info: fence.info.map(|r| shift_range(&r, byte_delta)),
                },
                closed: *closed,
            },
            other => other.clone(),
        };
        shifted
    }
}

#[allow(dead_code)] // incremental updater (this PR)
fn shift_range(range: &SourceRange, delta: i64) -> SourceRange {
    let shift = |offset: ByteOffset| ByteOffset((offset.as_usize() as i64 + delta).max(0) as usize);
    SourceRange::new(shift(range.start), shift(range.end))
}
