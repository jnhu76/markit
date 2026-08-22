//! Markdown L1 layer: the block index and its derived inline IR.
//!
//! GPUI-independent Markdown semantics for Markit, implementing
//! `docs/product/markdown-l1-semantic-contract.md` (the normative L1
//! dialect definition) over the P0-01 document seams:
//!
//! - [`MarkdownState`] owns the block index for one document version;
//!   it is built once from a [`DocumentSnapshot`](crate::DocumentSnapshot)
//!   and then updated **incrementally** from the canonical per-edit
//!   regions of [`EditResult`](crate::EditResult) — never from covering
//!   ranges and never by rescanning untouched regions.
//! - [`BlockRecord`] entries are a flat, tiling stream over the document
//!   (every byte belongs to exactly one block), held in a plain `Vec`
//!   with binary-search lookup — the representation ADR-004 prescribes.
//!   No interval trees, no arenas, no ropes here.
//! - Identity ([`InternalBlockId`]) is internal product identity, not
//!   plugin identity; ids survive edits per the contract's §10 rules.
//!
//! Parser internals (the line classifier, the incremental updater) are
//! crate-private and replaceable; the public surface is the state, the
//! record types, and the query views.

mod block;
mod identity;
mod lex;
mod parser;
mod state;

use std::ops::Range;

use crate::markdown::parser::BlockParser;
use crate::position::{ByteOffset, LineNumber, SourceRange};
use crate::revision::DocumentVersion;
use crate::snapshot::DocumentSnapshot;

pub use block::{
    BlockDetail, BlockFingerprint, BlockKind, BlockRecord, FenceInfo, ListItem, ListSignature,
};
pub use identity::InternalBlockId;
pub use state::{BlockParseState, FenceChar};

/// Structural work counters for one Markdown state transition (build or
/// incremental update). Like [`EditWork`](crate::EditWork), these are
/// structural, not wall-clock, so CI asserts algorithmic behavior without
/// timing flakes; unlike [`EditWork`] they count Markdown-block work.
///
/// `blocks_reparsed == 1` for a local edit is the **expected** case, not
/// a law (issue #12 R6): the invariant is "smallest semantically valid
/// region", and an edit that honestly requires more (fence propagation,
/// structural reinterpretation) reports it here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownWork {
    /// Number of disjoint invalidation islands processed (per-edit
    /// regions that merged count once; islands subsumed by another
    /// island's reparse do not count).
    pub dirty_regions: u64,
    /// Lowest line the update restarted parsing from.
    pub restart_line: u64,
    /// Lines whose text was parsed.
    pub lines_scanned: u64,
    /// Block bytes fingerprinted (the parse's byte cost).
    pub bytes_scanned: u64,
    /// Old records inspected while planning (rewind, dead set,
    /// convergence checks). Shifting survivor ranges is bookkeeping and
    /// is not counted as examining.
    pub blocks_examined: u64,
    /// Island blocks whose id was paired with a dead old block (kept id).
    pub blocks_reused: u64,
    /// Blocks emitted by island reparsing (all of them, reused ids
    /// included).
    pub blocks_reparsed: u64,
    /// Island blocks that received a freshly minted id.
    pub blocks_created: u64,
    /// Dead old blocks whose id retired (no partner).
    pub blocks_removed: u64,
    /// Blocks whose inline IR was recomputed (island blocks with inline
    /// runs; kept survivors are never reparsed).
    pub inline_blocks_reparsed: u64,
    /// Highest line at which an island converged (document end when a
    /// reparse ran to end of document — the honest unclosed-fence case).
    pub convergence_line: u64,
}

/// The Markdown block index for exactly one document version.
///
/// Version binding is strict: the state carries the
/// [`DocumentVersion`] it was built from, and incremental updates must
/// present an [`EditResult`](crate::EditResult) whose base revision is
/// the state's revision plus a [`DocumentSnapshot`](crate::DocumentSnapshot)
/// at the result's new revision. Anything else fails closed
/// ([`MarkdownStateError`]) and leaves the state untouched.
#[derive(Clone, Debug)]
pub struct MarkdownState {
    version: DocumentVersion,
    /// Next id to mint (monotonic; minted ids are never reused).
    #[allow(dead_code)] // read by the incremental updater (this PR)
    next_id: u64,
    /// The tiling block stream, ordered by position.
    blocks: Vec<BlockRecord>,
    last_work: MarkdownWork,
    cumulative_work: MarkdownWork,
}

impl MarkdownState {
    /// Full parse of `snapshot` at its version. The one place a whole
    /// document is scanned (load / reload / replace).
    pub fn build(snapshot: &DocumentSnapshot<'_>) -> Self {
        let mut parser = BlockParser::new(snapshot, 0);
        let mut next_id = 0u64;
        let mut blocks = Vec::new();
        while let Some(parsed) = parser.next_block() {
            blocks.push(parsed.into_record(InternalBlockId::mint(&mut next_id)));
        }
        let work = MarkdownWork {
            dirty_regions: 1,
            restart_line: 0,
            lines_scanned: parser.lines_scanned(),
            bytes_scanned: parser.bytes_scanned(),
            blocks_examined: blocks.len() as u64,
            blocks_reparsed: 0,
            blocks_created: blocks.len() as u64,
            inline_blocks_reparsed: 0,
            convergence_line: snapshot.line_count() as u64,
            ..MarkdownWork::default()
        };
        Self {
            version: snapshot.version(),
            next_id,
            blocks,
            last_work: work,
            cumulative_work: work,
        }
    }

    /// The document version this state describes. Downstream consumers
    /// gate on this before trusting any record.
    pub fn version(&self) -> DocumentVersion {
        self.version
    }

    /// Number of blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// The whole tiling stream, ordered by position.
    pub fn blocks(&self) -> &[BlockRecord] {
        &self.blocks
    }

    /// Record lookup by internal id. O(blocks) — a query convenience for
    /// the future view model, not a hot path (a position query exists
    /// for that).
    pub fn block_by_id(&self, id: InternalBlockId) -> Option<&BlockRecord> {
        self.blocks.iter().find(|block| block.id == id)
    }

    /// The block containing `offset` (the final block also owns the
    /// document-end offset). O(log blocks).
    pub fn block_at_offset(&self, offset: ByteOffset) -> Option<&BlockRecord> {
        if self.blocks.is_empty() {
            return None;
        }
        let idx = self
            .blocks
            .partition_point(|b| b.source_range.start <= offset);
        // idx is the first block starting after `offset`; the candidate
        // is idx-1. Tiling makes idx-1 the unique container.
        let idx = idx.saturating_sub(1);
        let block = &self.blocks[idx];
        let inside = offset < block.source_range.end
            || (idx + 1 == self.blocks.len() && offset == block.source_range.end);
        inside.then_some(block)
    }

    /// Blocks overlapping the byte `range`, in order. O(log blocks).
    pub fn blocks_in_range(&self, range: SourceRange) -> &[BlockRecord] {
        let first = self
            .blocks
            .partition_point(|b| b.source_range.end <= range.start);
        let last = self
            .blocks
            .partition_point(|b| b.source_range.start < range.end);
        &self.blocks[first..last.max(first)]
    }

    /// Blocks overlapping the line span, in order. O(log blocks).
    pub fn blocks_in_lines(&self, lines: Range<LineNumber>) -> &[BlockRecord] {
        let first = self
            .blocks
            .partition_point(|b| b.line_span.end <= lines.start);
        let last = self
            .blocks
            .partition_point(|b| b.line_span.start < lines.end);
        &self.blocks[first..last.max(first)]
    }

    /// Work counters of the last transition (build or update).
    pub fn last_work(&self) -> MarkdownWork {
        self.last_work
    }

    /// Work counters accumulated since the state was built.
    pub fn cumulative_work(&self) -> MarkdownWork {
        self.cumulative_work
    }

    /// Asserts the tiling invariant (test seam; O(blocks)).
    #[allow(dead_code)] // in-crate tests only
    pub(crate) fn assert_tiling(&self, snapshot: &DocumentSnapshot<'_>) {
        assert_eq!(self.version, snapshot.version(), "state version drift");
        let mut expected_start = 0usize;
        for (i, block) in self.blocks.iter().enumerate() {
            assert_eq!(
                block.source_range.start.as_usize(),
                expected_start,
                "block {i} breaks tiling"
            );
            assert_eq!(
                block.line_span.start.0,
                snapshot.line_of(block.source_range.start).0,
                "block {i} line span start disagrees with its bytes"
            );
            assert!(
                block.source_range.end.as_usize() > expected_start
                    || (i + 1 == self.blocks.len() && snapshot.len_bytes() == expected_start),
                "block {i} is empty"
            );
            expected_start = block.source_range.end.as_usize();
        }
        assert_eq!(
            expected_start,
            snapshot.len_bytes(),
            "stream must end at EOF"
        );
    }
}

/// Why an incremental update was rejected. The state is never partially
/// updated: on any of these the caller keeps the old state and must
/// rebuild (or rebase explicitly).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownStateError {
    /// The snapshot belongs to a different document than the state.
    DocumentMismatch,
    /// The state is not at the edit result's base revision (skipped
    /// revisions are not supported: update once per edit, in order).
    StaleBase,
    /// The snapshot is not at the edit result's new revision.
    SnapshotNotAtNewRevision,
}

impl std::fmt::Display for MarkdownStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentMismatch => write!(f, "snapshot belongs to a different document"),
            Self::StaleBase => {
                write!(f, "state revision is not the edit result's base revision")
            }
            Self::SnapshotNotAtNewRevision => {
                write!(f, "snapshot is not at the edit result's new revision")
            }
        }
    }
}

impl std::error::Error for MarkdownStateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    const DOC: &str = "# 标题\n\n段落 🙂\n\n> 引用\n\n- 甲\n- 乙\n\n1. 一\n\n```\ncode\n```\n";

    #[test]
    fn build_fully_scans_once_and_tiles() {
        let doc = Document::new(DOC);
        let snap = doc.snapshot();
        let state = MarkdownState::build(&snap);
        assert_eq!(state.version(), doc.version());
        state.assert_tiling(&snap);
        let work = state.last_work();
        assert_eq!(work.dirty_regions, 1);
        assert_eq!(work.restart_line, 0);
        assert_eq!(work.lines_scanned, snap.line_count() as u64);
        assert_eq!(work.bytes_scanned, DOC.len() as u64);
        assert_eq!(work.blocks_created, state.block_count() as u64);
        assert_eq!(work.blocks_examined, state.block_count() as u64);
    }

    #[test]
    fn queries_answer_by_position_and_id() {
        let doc = Document::new(DOC);
        let snap = doc.snapshot();
        let state = MarkdownState::build(&snap);

        // Every offset maps to exactly one block.
        for offset in 0..=snap.len_bytes() {
            let block = state
                .block_at_offset(ByteOffset(offset))
                .expect("tiling covers every offset");
            assert!(
                block.source_range.start.as_usize() <= offset
                    && (offset < block.source_range.end.as_usize()
                        || block.source_range.end.as_usize() == snap.len_bytes()),
                "offset {offset} outside its block"
            );
        }

        // Byte-range and line-range slices are ordered and overlapping.
        let heading_bytes = state.blocks_in_range(SourceRange::new(ByteOffset(0), ByteOffset(4)));
        assert_eq!(heading_bytes.len(), 1);
        assert_eq!(heading_bytes[0].kind, BlockKind::Heading);

        let fence_lines = state.blocks_in_lines(LineNumber(11)..LineNumber(14));
        assert!(fence_lines.iter().any(|b| b.kind == BlockKind::FencedCode));

        let first = &state.blocks()[0];
        assert_eq!(state.block_by_id(first.id).map(|b| b.id), Some(first.id));
        assert!(state
            .block_by_id(InternalBlockId::from_u64_for_test(9999))
            .is_none());
    }

    #[test]
    fn empty_and_edge_documents() {
        for text in ["", "\n", "# only heading", "no final newline"] {
            let doc = Document::new(text);
            let snap = doc.snapshot();
            let state = MarkdownState::build(&snap);
            state.assert_tiling(&snap);
            assert!(state.block_count() >= 1);
        }
    }
}
