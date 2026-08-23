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
//! record types, and the query views. Pinned by construction:
//!
//! ```compile_fail
//! use markit_core::markdown::parser::BlockParser;
//! ```
//!
//! ```compile_fail
//! use markit_core::markdown::lex::leading_spaces;
//! ```

mod block;
mod identity;
mod incremental;
mod inline;
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
pub use inline::{InlineIr, InlineNode, InlineRun};
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

impl MarkdownWork {
    pub(crate) fn absorb(&mut self, other: &Self) {
        self.dirty_regions += other.dirty_regions;
        self.restart_line = other.restart_line;
        self.lines_scanned += other.lines_scanned;
        self.bytes_scanned += other.bytes_scanned;
        self.blocks_examined += other.blocks_examined;
        self.blocks_reused += other.blocks_reused;
        self.blocks_reparsed += other.blocks_reparsed;
        self.blocks_created += other.blocks_created;
        self.blocks_removed += other.blocks_removed;
        self.inline_blocks_reparsed += other.inline_blocks_reparsed;
        self.convergence_line = other.convergence_line;
    }
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
        let mut inline_blocks = 0u64;
        while let Some(parsed) = parser.next_block() {
            let mut record = parsed.into_record(InternalBlockId::mint(&mut next_id));
            if inline::attach_inline(&mut record, snapshot) {
                inline_blocks += 1;
            }
            blocks.push(record);
        }
        let work = MarkdownWork {
            dirty_regions: 1,
            restart_line: 0,
            lines_scanned: parser.lines_scanned(),
            bytes_scanned: parser.bytes_scanned(),
            blocks_examined: blocks.len() as u64,
            blocks_reparsed: 0,
            blocks_created: blocks.len() as u64,
            inline_blocks_reparsed: inline_blocks,
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

    /// Incremental update from one mutation (contract §9).
    ///
    /// `result` is the mutation's canonical per-edit regions;
    /// `snapshot` must be the document **after** that mutation. The
    /// whole triple is validated before anything mutates — state@N +
    /// edit N→N+1 + snapshot@N+1 — and any mismatch fails closed,
    /// leaving this state exactly as it was (the caller then rebuilds or
    /// rebases explicitly; nothing pretends to be valid).
    pub fn update(
        &mut self,
        snapshot: &DocumentSnapshot<'_>,
        result: &crate::change::EditResult,
    ) -> Result<(), MarkdownStateError> {
        if result.new_revision != result.base_revision.next() {
            return Err(MarkdownStateError::InconsistentResult);
        }
        if snapshot.id() != self.version.document_id() {
            return Err(MarkdownStateError::DocumentMismatch);
        }
        if snapshot.revision() != result.new_revision {
            return Err(MarkdownStateError::SnapshotNotAtNewRevision);
        }
        if self.version.revision() != result.base_revision {
            return Err(MarkdownStateError::StaleBase);
        }

        let mut work = MarkdownWork::default();
        incremental::apply_edits(
            &mut self.blocks,
            &mut self.next_id,
            snapshot,
            &result.edits,
            &mut work,
        );
        self.version = snapshot.version();
        self.last_work = work;
        self.cumulative_work.absorb(&work);
        Ok(())
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

    /// The inline IR of one block (query view; contract §7).
    pub fn inline_ir(&self, id: InternalBlockId) -> Option<&InlineIr> {
        self.block_by_id(id).map(|block| &block.inline)
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
    /// The result does not describe a single coherent N→N+1 step.
    InconsistentResult,
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
            Self::InconsistentResult => {
                write!(f, "edit result is not a single coherent revision step")
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

#[cfg(test)]
mod incremental_tests {
    use super::*;
    use crate::change::TextEdit;
    use crate::document::Document;
    use crate::position::ByteOffset;
    use crate::transaction::EditTransaction;

    fn setup(text: &str) -> (Document, MarkdownState) {
        let doc = Document::new(text);
        let state = MarkdownState::build(&doc.snapshot());
        (doc, state)
    }

    fn apply(doc: &mut Document, state: &mut MarkdownState, edits: TextEdit) -> MarkdownWork {
        let applied = EditTransaction::typing()
            .with_edit(edits)
            .apply(doc)
            .unwrap();
        let snapshot = doc.snapshot();
        state
            .update(&snapshot, &applied.result)
            .expect("coherent update");
        state.assert_tiling(&snapshot);
        state.last_work()
    }

    /// The oracle: the incremental stream must equal a fresh full parse
    /// in every observable way except identity (contract §12).
    fn assert_matches_rebuild(doc: &Document, state: &MarkdownState) {
        let rebuilt = MarkdownState::build(&doc.snapshot());
        assert_eq!(
            state.version(),
            rebuilt.version(),
            "versions must agree after update"
        );
        assert_eq!(state.block_count(), rebuilt.block_count(), "block count");
        for (incremental, fresh) in state.blocks().iter().zip(rebuilt.blocks()) {
            assert_eq!(incremental.kind, fresh.kind, "kind");
            assert_eq!(incremental.source_range, fresh.source_range, "range");
            assert_eq!(incremental.line_span, fresh.line_span, "line span");
            assert_eq!(incremental.state_before, fresh.state_before);
            assert_eq!(incremental.state_after, fresh.state_after);
            assert_eq!(incremental.fingerprint, fresh.fingerprint, "fingerprint");
            assert_eq!(incremental.detail, fresh.detail, "detail");
            assert_eq!(incremental.inline, fresh.inline, "inline IR");
        }
    }

    #[test]
    fn local_paragraph_edit_is_local() {
        let text = "# h\n\nfirst\n\nsecond\n\nthird\n";
        let (mut doc, mut state) = setup(text);
        let ids_before: Vec<_> = state.blocks().iter().map(|b| b.id).collect();

        // Edit inside the "second" paragraph.
        let second_at = text.find("second").unwrap();
        let work = apply(
            &mut doc,
            &mut state,
            TextEdit::insert(ByteOffset(second_at + 3), "X"),
        );
        assert_matches_rebuild(&doc, &state);

        assert_eq!(work.dirty_regions, 1);
        assert_eq!(work.blocks_reparsed, 1, "one paragraph reparsed");
        assert_eq!(work.blocks_reused, 1, "the paragraph keeps its id");
        assert_eq!(work.blocks_created, 0);
        assert_eq!(work.blocks_removed, 0);
        assert_eq!(work.lines_scanned, 1, "one line of text parsed");
        assert_eq!(work.inline_blocks_reparsed, 1);
        assert!(work.convergence_line < doc.line_count() as u64);

        // Everything except the edited paragraph kept its id.
        for (before, after) in ids_before.iter().zip(state.blocks()) {
            if after.kind == BlockKind::Paragraph
                && after.source_range.contains(ByteOffset(second_at + 3))
            {
                continue;
            }
            assert_eq!(*before, after.id, "untouched blocks keep ids");
        }
    }

    #[test]
    fn distant_edits_stay_two_islands() {
        let text = "aaa\n\nbbb\n\nccc\n";
        let (mut doc, mut state) = setup(text);
        let tx = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(1), "X"))
            .with_edit(TextEdit::insert(ByteOffset(10), "Y"))
            .apply(&mut doc)
            .unwrap();
        let snapshot = doc.snapshot();
        state.update(&snapshot, &tx.result).unwrap();
        state.assert_tiling(&snapshot);
        assert_matches_rebuild(&doc, &state);
        assert_eq!(state.last_work().dirty_regions, 2, "two semantic islands");
        assert_eq!(state.last_work().blocks_reparsed, 2, "two paragraphs");
    }

    #[test]
    fn fence_content_edit_keeps_fence_identity() {
        let text = "before\n\n```\ncode line\n```\n\nafter\n";
        let (mut doc, mut state) = setup(text);
        let fence_id = state
            .blocks()
            .iter()
            .find(|b| b.kind == BlockKind::FencedCode)
            .unwrap()
            .id;
        let at = text.find("code").unwrap();
        let work = apply(&mut doc, &mut state, TextEdit::insert(ByteOffset(at), "X"));
        assert_matches_rebuild(&doc, &state);
        assert_eq!(
            work.blocks_reparsed, 1,
            "fence block reparsed (restart at fence start)"
        );
        let fence_after = state
            .blocks()
            .iter()
            .find(|b| b.kind == BlockKind::FencedCode)
            .unwrap();
        assert_eq!(
            fence_after.id, fence_id,
            "fence keeps its id through content edits"
        );
        assert_eq!(work.blocks_reused, 1);
    }

    #[test]
    fn deleting_closing_fence_propagates_honestly() {
        let text = "```\ncode\n```\nafter\n";
        let (mut doc, mut state) = setup(text);
        let closer = text.find("```\nafter").unwrap();
        let work = apply(
            &mut doc,
            &mut state,
            TextEdit::delete(crate::position::SourceRange::new(
                ByteOffset(closer),
                ByteOffset(closer + 4),
            )),
        );
        assert_matches_rebuild(&doc, &state);
        // The fence now swallows "after" to EOF: paragraphs died inside it.
        assert!(
            work.lines_scanned >= doc.line_count() as u64,
            "rescan to EOF"
        );
        assert_eq!(work.convergence_line, doc.line_count() as u64);
        let kinds: Vec<_> = state.blocks().iter().map(|b| b.kind).collect();
        assert_eq!(kinds, vec![BlockKind::FencedCode]);
        let BlockDetail::FencedCode { closed, .. } = &state.blocks()[0].detail else {
            panic!("fence")
        };
        assert!(!closed, "fence is now unclosed and runs to EOF");
    }

    #[test]
    fn structural_transforms_follow_identity_rules() {
        // Kind change: paragraph -> heading mints a fresh id (rule C).
        let (mut doc, mut state) = setup("plain text\n");
        let para_id = state.blocks()[0].id;
        let work = apply(&mut doc, &mut state, TextEdit::insert(ByteOffset(0), "## "));
        assert_matches_rebuild(&doc, &state);
        assert_eq!(state.blocks()[0].kind, BlockKind::Heading);
        assert_ne!(state.blocks()[0].id, para_id);
        assert_eq!(work.blocks_reused, 0);
        assert_eq!(work.blocks_created, 1);
        assert_eq!(work.blocks_removed, 1);

        // Split: paragraph -> paragraph + heading + paragraph. The
        // leading paragraph pairs with the old one (1:1, rule B);
        // newcomers mint (rule C).
        let (mut doc, mut state) = setup("one two three\n");
        let para_id = state.blocks()[0].id;
        let two = "one two three".find("two").unwrap();
        let work = apply(
            &mut doc,
            &mut state,
            TextEdit::replace(
                crate::position::SourceRange::new(ByteOffset(two - 1), ByteOffset(two)),
                "\n## ",
            ),
        );
        assert_matches_rebuild(&doc, &state);
        // The heading is the only newcomer; the trailing blank survived
        // the island (convergence landed exactly on it).
        assert_eq!(work.blocks_created, 1, "the heading is new");
        assert_eq!(
            state.blocks()[0].id,
            para_id,
            "surviving leading paragraph keeps its id"
        );
    }

    #[test]
    fn append_extends_last_block() {
        let (mut doc, mut state) = setup("# h\npara\n");
        let ids: Vec<_> = state.blocks().iter().map(|b| b.id).collect();
        let len = "# h\npara\n".len();
        let work = apply(
            &mut doc,
            &mut state,
            TextEdit::insert(ByteOffset(len), "more"),
        );
        assert_matches_rebuild(&doc, &state);
        assert_eq!(work.blocks_reparsed, 1, "the paragraph grew");
        for (before, after) in ids.iter().zip(state.blocks()) {
            assert_eq!(*before, after.id, "append preserves every id");
        }
    }

    #[test]
    fn update_rejects_incoherent_inputs_and_keeps_state() {
        let (mut doc, mut state) = setup("a\n");

        let applied = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(1), "x"))
            .apply(&mut doc)
            .unwrap();
        let snap1 = doc.snapshot();

        // Coherent triple: state@0 + result 0->1 + snapshot@1.
        state.update(&snap1, &applied.result).expect("valid update");
        assert_eq!(state.version(), snap1.version());

        // Replay of the same result: state has moved past its base.
        assert_eq!(
            state.update(&snap1, &applied.result),
            Err(MarkdownStateError::StaleBase)
        );

        // Snapshot at a revision the result does not produce.
        let (mut doc2, mut state2) = setup("a\n");
        let first = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(1), "x"))
            .apply(&mut doc2)
            .unwrap();
        let _second = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(2), "y"))
            .apply(&mut doc2)
            .unwrap();
        let snap2 = doc2.snapshot();
        assert_eq!(
            state2.update(&snap2, &first.result),
            Err(MarkdownStateError::SnapshotNotAtNewRevision)
        );

        // Result from a different document entirely.
        let mut other = Document::new("b\n");
        let other_applied = EditTransaction::typing()
            .with_edit(TextEdit::insert(ByteOffset(1), "y"))
            .apply(&mut other)
            .unwrap();
        assert_eq!(
            state2.update(&other.snapshot(), &other_applied.result),
            Err(MarkdownStateError::DocumentMismatch)
        );

        // A result that is not a single coherent N->N+1 step.
        let rev = snap1.revision();
        let incoherent = crate::change::EditResult {
            base_revision: rev,
            new_revision: rev.next().next(),
            kind: crate::change::ChangeKind::Insert,
            covering_old_range: crate::position::SourceRange::new(ByteOffset(1), ByteOffset(1)),
            covering_new_range: crate::position::SourceRange::new(ByteOffset(1), ByteOffset(2)),
            byte_delta: 1,
            line_delta: 0,
            edits: Vec::new(),
            work: crate::change::EditWork::default(),
        };
        assert_eq!(
            state2.update(&snap2, &incoherent),
            Err(MarkdownStateError::InconsistentResult)
        );

        // Every rejection left the state exactly as it was: it still
        // describes revision 0 of its own document, and the earlier
        // `state.update` success already proves the happy path.
        assert_eq!(state2.version().revision().as_u64(), 0);
        assert_eq!(state2.version().document_id(), doc2.id());
        assert_eq!(state2.block_count(), 2, "paragraph + trailing blank");
    }
}
