//! markit-mdbench-block-local — H1 BLOCK_LOCAL_REPARSE (#22, stage R5).
//!
//! Mechanism identity (frozen in
//! `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §6; mizchi-markdown-inspired
//! at pinned commit ffe7dc00): retain the old TOP-LEVEL document tiling
//! (blocks + blank runs), map the edit to the damaged top-level region by
//! strict overlap (insertion-gap mapping at exact boundaries), clean-reparse
//! ONLY that region over the shared grammar substrate, splice prefix
//! entries through, and reconstruct the suffix with delta-shifted spans.
//! Conservative, semantically defined fallbacks (F1–F6, all
//! source/edit-state derived) replace the update with a total full parse
//! whenever the H1 model cannot soundly localize.
//!
//! This crate owns ONLY H1 mechanism state and policy: the tiling, the
//! damage scan, the region construction, the soundness guards, the suffix
//! shift, and the counters. Grammar semantics (block scanner, inline
//! scanner, materialization) live in `markit-mdbench-shared-grammar`; the
//! result vocabulary and validation in `markit-mdbench-oracle`. H1 never
//! calls H0; test crates may use H0 as the correctness oracle.

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::Completed;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::Observed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::{normalized_checksum, Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_shared_grammar as sg;
use sg::parser::{classify_top_level_line, LineClass, RegionParse, Skel};

/// Mechanism identifier.
pub const H1_MECHANISM_ID: &str = "block-local-reparse-h1";

// ---------------------------------------------------------------------------
// Retained native state
// ---------------------------------------------------------------------------

/// Parse-time soundness facts of one retained block entry, derived from
/// the [`Skel`] + the parsed source when the tiling is built (R5 freeze
/// §6: "derived from the retained Skel + source, not separately
/// indexed"). They answer exactly one question: can this block CONTINUE
/// onto the following line, and under which lexical condition?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFacts {
    /// Paragraph: continues onto any following non-blank,
    /// non-interrupting line (B7 continuation).
    Para,
    /// Blockquote: continues onto a line carrying `>` after <= 3 spaces.
    Quote,
    /// List: `indent` is the marker indent relative to the list's parent
    /// content column; `strip` is the LAST item's content indent. A
    /// following line continues the list as a sibling marker at exactly
    /// `indent`, or as item content at `>= strip` (§6/§7).
    List { indent: usize, strip: usize },
    /// Line-terminated blocks (heading, closed fence, definition): never
    /// continue.
    Terminated,
}

/// One entry of the retained top-level tiling. Blank runs are first-class
/// entries (the mizchi BlankLines analogue): the tiling covers the whole
/// document with no gaps and no overlaps. A block entry carries its
/// ALREADY MATERIALIZED semantic subtree (`sem`, R5-CORRECTIVE-1 §5) in
/// the entry's own coordinates: prefix pass-through never re-reads or
/// re-scans it, and completed-state projection is a pure traversal.
// The variant size difference is the native representation, not a
// layout problem to optimize: boxing `sem` would change nothing about
// the mechanism and only obscure the ownership witness.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum TopEntry {
    Block {
        skel: Skel,
        sem: Node,
        facts: BlockFacts,
    },
    Blank {
        start: usize,
        end: usize,
    },
}

impl TopEntry {
    pub fn span(&self) -> (usize, usize) {
        match self {
            TopEntry::Block { skel, .. } => (skel.start(), skel.end()),
            TopEntry::Blank { start, end } => (*start, *end),
        }
    }
}

/// H1 retained state (MECHANISM_INTRINSIC_STATE, R5 freeze §6): the whole
/// old document tiled in order, the retained definitions array (mizchi's
/// `Document.definitions`, which doubles as the F1 fallback trigger), and
/// the old source length. No block index, no hashes, no dependency map.
#[derive(Debug, Clone)]
pub struct H1State {
    entries: Vec<TopEntry>,
    defs: Vec<(String, String)>,
    src_len: usize,
}

impl H1State {
    pub fn entries(&self) -> &[TopEntry] {
        &self.entries
    }

    pub fn definitions(&self) -> &[(String, String)] {
        &self.defs
    }

    pub fn source_len_bytes(&self) -> usize {
        self.src_len
    }
}

/// Completed-state normalization (R5-CORRECTIVE-1, MAJOR-3): a PURE
/// traversal over the retained semantic subtrees. No source, no sink, no
/// parser, no repair — the retained representation itself carries the
/// syntax facts.
impl NormalizeV1 for H1State {
    fn normalize_v1(&self) -> NormalizedDocument {
        document_from_entries(&self.entries, self.src_len)
    }
}

/// Assemble the normalized document from the tiling's retained semantic
/// subtrees (pure; clones the per-block subtrees).
fn document_from_entries(entries: &[TopEntry], src_len: usize) -> NormalizedDocument {
    let mut root = Node::new(NodeKind::Document, 0, src_len);
    root.children = entries
        .iter()
        .filter_map(|e| match e {
            TopEntry::Block { sem, .. } => Some(sem.clone()),
            TopEntry::Blank { .. } => None,
        })
        .collect();
    NormalizedDocument::new(root)
}

/// Pending work handed to `complete()`. Per the eager-completion boundary
/// (R5 freeze §4) it ALREADY holds the complete new tiling, definition
/// array, and the fully materialized normalized result; `complete()`
/// seals and checksums only.
pub struct H1Pending {
    state: H1State,
    result: NormalizedDocument,
}

impl H1Pending {
    /// The already-materialized normalized result (eager-completion
    /// inspection hook; no parse work happens here).
    pub fn result(&self) -> &NormalizedDocument {
        &self.result
    }
}

/// H1's `prepare_update` product: pure edit coordinates (R1 phase
/// boundary — the damage scan itself is state maintenance, runs in
/// `update`).
#[derive(Debug, Clone)]
pub struct H1Prepared {
    edit_start: usize,
    edit_end: usize,
    delta: isize,
}

// ---------------------------------------------------------------------------
// Mechanism
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct BlockLocalMechanism;

impl BlockLocalMechanism {
    pub fn new() -> Self {
        Self
    }

    /// Clean parse of a complete document into H1 state + result — the
    /// one pipeline shared by `full_parse` and the total fallback.
    fn parse_into_pending<W: WorkSink>(
        &self,
        src: &[u8],
        cx: &mut MechanismContext<'_, W>,
    ) -> H1Pending {
        let rp = sg::parse_region(src, 0, src.len(), cx.sink);
        let defs = rp.defs.entries().to_vec();
        let table = ref_table(&defs);
        let entries = tile_from_blocks(&rp.blocks, src, &table, cx.sink);
        let blocks_total = count_blocks(&rp.blocks);
        let nodes_total: u64 = entries.iter().map(entry_native_nodes).sum();
        cx.sink.add_blocks_reparsed(blocks_total);
        cx.sink.add_nodes_rebuilt(nodes_total);
        cx.sink.add_metadata_records_touched(entries.len() as u64);
        let result = document_from_entries(&entries, src.len());
        H1Pending {
            state: H1State {
                entries,
                defs,
                src_len: src.len(),
            },
            result,
        }
    }

    /// The total fallback (F1–F6): re-parse the complete post source with
    /// the ordinary block + inline pipeline and rebuild the tiling.
    /// Reported counters are the full scan + full reconstruction counts
    /// (they happened); nodes_reused is the measured zero.
    fn total_fallback<W: WorkSink>(
        &self,
        post: &[u8],
        scanned_old_entries: u64,
        cx: &mut MechanismContext<'_, W>,
    ) -> H1Pending {
        cx.sink.record_fallback_to_full();
        let mut pending = self.parse_into_pending(post, cx);
        cx.sink.add_nodes_reused(0);
        cx.sink.add_metadata_records_touched(scanned_old_entries);
        pending.state.src_len = post.len();
        pending
    }
}

impl Mechanism for BlockLocalMechanism {
    type State = H1State;
    type Prepared = H1Prepared;
    type Pending = H1Pending;

    fn id(&self) -> MechanismId {
        MechanismId(H1_MECHANISM_ID.to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        declare_gauges_not_applicable(cx);
        let pending = self.parse_into_pending(source.as_bytes(), cx);
        // A clean parse reuses nothing: the precise measured zero.
        cx.sink.add_nodes_reused(0);
        Ok(pending)
    }

    fn prepare_update<W: WorkSink>(
        &self,
        _old_source: &Source,
        _post_source: &Source,
        edit: &CanonicalEdit,
        _old_state: &Self::State,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        let edit_start = edit.start_byte() as usize;
        let edit_end = edit.end_byte() as usize;
        Ok(H1Prepared {
            edit_start,
            edit_end,
            delta: edit.inserted_text_len_bytes() as isize - (edit_end - edit_start) as isize,
        })
    }

    fn update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let old = old_source.as_bytes();
        let post = post_source.as_bytes();
        let es = prepared.edit_start;
        let ee = prepared.edit_end;
        let delta = prepared.delta;
        debug_assert_eq!(post.len() as i64, old.len() as i64 + delta as i64);
        debug_assert_eq!(es, edit.start_byte() as usize);
        debug_assert_eq!(ee, edit.end_byte() as usize);
        declare_gauges_not_applicable(cx);
        // MAJOR-1 (R5-CORRECTIVE-1): the old state is consumed. All
        // fallback decisions below are made while only borrowing
        // `entries`; the vector is moved out of only after the last
        // possible fallback return.
        let H1State {
            entries,
            defs: old_defs,
            src_len: _,
        } = old_state;

        // DAMAGE SCAN (mizchi-faithful): one linear pass; strict overlap
        // `entry.start < edit_end && entry.end > edit_start`. The scan
        // inspects every tiling entry (the linear scan IS the mechanism).
        let mut first: Option<usize> = None;
        let mut last = 0usize;
        for (i, e) in entries.iter().enumerate() {
            let (s, en) = e.span();
            if s < ee && en > es {
                if first.is_none() {
                    first = Some(i);
                }
                last = i;
            }
        }
        let scanned = entries.len() as u64;
        cx.sink.add_metadata_records_touched(scanned);

        // Region boundaries in OLD coordinates + the prefix/suffix split.
        // Overlap path (mizchi): [end of the entry before the first
        // affected entry, start of the first unaffected entry after the
        // last one). No-overlap path (insertion-gap mapping; the mizchi
        // `(i, i)` outcome): the region is the boundary gap itself; the
        // inserted bytes join it through the delta-shifted right edge.
        let (rs, re_old, prefix_len, suffix_from) = match first {
            Some(f) => (
                entries[..f].last().map(|e| e.span().1).unwrap_or(0),
                entries
                    .get(last + 1)
                    .map(|e| e.span().0)
                    .unwrap_or(old.len()),
                f,
                last + 1,
            ),
            None => {
                let after = entries
                    .iter()
                    .position(|e| e.span().0 >= ee)
                    .unwrap_or(entries.len());
                (
                    entries[..after].last().map(|e| e.span().1).unwrap_or(0),
                    entries.get(after).map(|e| e.span().0).unwrap_or(old.len()),
                    after,
                    after,
                )
            }
        };
        debug_assert!(rs <= es && ee <= re_old);
        // Right edge shifted by delta and clamped (mizchi reparse_end_new).
        let re_new = ((re_old as isize + delta).max(rs as isize) as usize).min(post.len());

        // Region reparse over the shared grammar: a fresh clean parse of
        // [rs, re_new) producing absolute document-coordinate spans.
        let rp = sg::parse_region(post, rs, re_new, cx.sink);

        // F1 — the mizchi definition-presence TOTAL fallback (R2-H01
        // class), reproduced verbatim as a semantic rule: any retained
        // definition, or any definition created by the region parse,
        // forces a total parse. (On the normal path below the old
        // definitions are empty, so prefix/suffix contain no Def entries
        // and the new table is exactly the region's table.)
        if !old_defs.is_empty() || !rp.defs.is_empty() {
            return Ok(self.total_fallback(post, scanned, cx));
        }

        // Soundness guards F2–F6 (left/right edge continuation + fence
        // propagation). Any fire -> total fallback: correct, counted,
        // never a silent guess.
        if guards_fire(
            &entries,
            prefix_len,
            suffix_from,
            &rp,
            rs,
            re_new,
            post,
            delta,
            cx,
        ) {
            return Ok(self.total_fallback(post, scanned, cx));
        }

        // MAJOR-1 (R5-CORRECTIVE-1): CONSUME the old tiling. Prefix
        // entries MOVE into the new state unchanged — ownership
        // pass-through, not clone-based pseudo reuse — and are counted as
        // `nodes_reused`. Suffix entries are RECONSTRUCTED with
        // delta-shifted spans over their retained syntax: parser-work
        // avoidance but representation rebuild -> `nodes_rebuilt`, never
        // `nodes_reused`. Every fallback decision above is complete
        // before the vector is consumed.
        let mut moved_prefix: Vec<TopEntry> = Vec::with_capacity(prefix_len);
        let mut suffix_entries: Vec<TopEntry> = Vec::new();
        let mut reused_nodes = 0u64;
        let mut suffix_nodes = 0u64;
        for (i, e) in entries.into_iter().enumerate() {
            if i < prefix_len {
                reused_nodes += entry_native_nodes(&e);
                moved_prefix.push(e);
            } else if i >= suffix_from {
                suffix_nodes += entry_native_nodes(&e);
                suffix_entries.push(shift_entry_owned(e, delta));
            }
        }
        cx.sink.add_nodes_reused(reused_nodes);

        // Assemble the new tiling: moved prefix + region fill + shifted
        // suffix. The F1 gate guarantees no definitions exist, so the
        // region's fresh blocks materialize against an empty table.
        let region_table = ref_table(&[]);
        let mut new_entries = moved_prefix;
        let mut cursor = rs;
        let mut region_nodes = 0u64;
        for sk in &rp.blocks {
            let (s, e) = (sk.start(), sk.end());
            debug_assert!(s >= cursor && e >= s);
            if s > cursor {
                new_entries.push(TopEntry::Blank {
                    start: cursor,
                    end: s,
                });
            }
            let sem =
                sg::inline::materialize_one_with_sink(post, sk.clone(), &region_table, cx.sink);
            let entry = TopEntry::Block {
                skel: sk.clone(),
                sem,
                facts: block_facts(sk, post),
            };
            region_nodes += entry_native_nodes(&entry);
            new_entries.push(entry);
            cursor = e;
        }
        if cursor < re_new {
            new_entries.push(TopEntry::Blank {
                start: cursor,
                end: re_new,
            });
        }
        let suffix_count = suffix_entries.len() as u64;
        new_entries.extend(suffix_entries);
        let region_blocks = count_blocks(&rp.blocks);
        let region_inserted = (new_entries.len() - prefix_len) as u64 - suffix_count;

        cx.sink.add_blocks_reparsed(region_blocks);
        cx.sink.add_nodes_rebuilt(region_nodes + suffix_nodes);
        cx.sink
            .add_metadata_records_touched(region_inserted + suffix_count);
        // No fallback happened: the measured zero (Known(0) != Unknown).
        cx.sink.add_fallback_to_full(0);

        let result = document_from_entries(&new_entries, post.len());
        Ok(H1Pending {
            state: H1State {
                entries: new_entries,
                defs: rp.defs.entries().to_vec(),
                src_len: post.len(),
            },
            result,
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        // Sealing only: the pending already holds the complete state and
        // the materialized result (eager completion boundary, R5 §4).
        let checksum = normalized_checksum(&pending.result);
        Ok(Completed {
            state: pending.state,
            result_checksum: checksum,
        })
    }
}

// ---------------------------------------------------------------------------
// Tiling construction
// ---------------------------------------------------------------------------

/// Tile [0, len) with block + blank-run entries from completed top-level
/// blocks (absolute spans, in order). Each block entry carries its
/// materialized semantic subtree, built here with the instrumented
/// inline pass (R5-CORRECTIVE-1 §5/MAJOR-2).
fn tile_from_blocks<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    sink: &mut W,
) -> Vec<TopEntry> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for skel in blocks {
        let (s, e) = (skel.start(), skel.end());
        debug_assert!(s >= cursor, "block spans must be ordered");
        if s > cursor {
            out.push(TopEntry::Blank {
                start: cursor,
                end: s,
            });
        }
        debug_assert!(e > s, "block entries are non-degenerate");
        let sem = sg::inline::materialize_one_with_sink(src, skel.clone(), table, sink);
        out.push(TopEntry::Block {
            skel: skel.clone(),
            sem,
            facts: block_facts(skel, src),
        });
        cursor = e;
    }
    if cursor < src.len() {
        out.push(TopEntry::Blank {
            start: cursor,
            end: src.len(),
        });
    }
    if !out.is_empty() {
        debug_assert_eq!(out.first().map(|e| e.span().0), Some(0));
        debug_assert_eq!(out.last().map(|e| e.span().1), Some(src.len()));
    }
    out
}

/// Soundness facts for one block, derived from the Skel + the source it
/// was parsed from.
fn block_facts(skel: &Skel, src: &[u8]) -> BlockFacts {
    match skel {
        Skel::Para { .. } => BlockFacts::Para,
        Skel::Quote { .. } => BlockFacts::Quote,
        Skel::List { start, items, .. } => {
            // Top-level list: marker indent = spaces before the marker on
            // the list's first line; the continuation-relevant item is the
            // LAST one, whose strip = indent + its marker delta (§7).
            let ls = line_start_of(src, *start);
            let indent = start - ls;
            let strip = items
                .last()
                .map(|it| last_item_strip(it, src, indent))
                .unwrap_or(indent + 1);
            BlockFacts::List { indent, strip }
        }
        Skel::Heading { .. } | Skel::Fence { .. } | Skel::Def { .. } => BlockFacts::Terminated,
        // Top-level entries are never Items (items live inside List
        // frames) and H1 never splices.
        Skel::Item { .. } | Skel::Spliced { .. } => unreachable!("not a top-level entry"),
    }
}

fn last_item_strip(item: &Skel, src: &[u8], indent: usize) -> usize {
    if let Skel::Item { start, .. } = item {
        let ls = line_start_of(src, *start);
        let lf = sg::parser::memchr_lf(src, ls);
        if let Some((_, delta)) = sg::parser::parse_marker(src, *start, lf) {
            return indent + delta;
        }
    }
    indent + 1
}

// ---------------------------------------------------------------------------
// Soundness guards (F2–F6)
// ---------------------------------------------------------------------------

/// The region may only be trusted when neither edge of the splice can
/// continue into its neighbor. Every check is derived from the edit and
/// the source/state bytes ONLY (never CaseId, corpus identity, label, or
/// timing). Guards fire a TOTAL fallback (correct, counted), never a
/// silent guess.
///
/// Separation between an edge block and the next block is measured as LF
/// bytes strictly between their spans in POST coordinates. One LF is a
/// bare line terminator (the block line ended; the next line decides);
/// two or more LFs contain a blank line, which terminates every
/// continuation (§3, D5, §6): no merge is possible.
#[allow(clippy::too_many_arguments)]
fn guards_fire<W: WorkSink>(
    entries: &[TopEntry],
    prefix_len: usize,
    suffix_from: usize,
    rp: &RegionParse,
    rs: usize,
    re_new: usize,
    post: &[u8],
    delta: isize,
    cx: &mut MechanismContext<'_, W>,
) -> bool {
    // F6 — forward fence state (R2-H03 class): the region parse ended
    // inside an open fence whose extent cannot be bounded within the
    // region. A region extending to EOF IS the bounded case (unclosed
    // fences run to EOF, §8).
    if rp.fence_open_at_end && re_new < post.len() {
        return true;
    }

    // F4 (a) — right-edge line termination: the region's last NON-EMPTY
    // line must end at or before the region's right edge. A line whose
    // terminator lies beyond the region was merged with suffix bytes in
    // the clean parse; the region parser (which EOF-closes at the cut)
    // cannot see that. A terminator exactly AT the right edge is
    // unchanged suffix bytes: the line is complete within the region. An
    // empty tail line (the cut sits on a line start) carries no content
    // and cannot merge — the continuation pairs below own that case.
    if rs < re_new {
        let cut = post[rs..re_new]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(rs, |p| rs + p + 1);
        if cut < re_new && sg::parser::memchr_lf(post, cut) > re_new {
            return true;
        }
    }

    // Continuation pairs. LEFT: prefix-last block vs the region's first
    // block (F2 paragraph merge / F3 container continuation). RIGHT: the
    // region's last block — or, when the region produced no blocks, the
    // carried prefix-last block — vs the suffix's first block (F4 / F5;
    // the carried form makes blank-run deletion merges sound instead of
    // silently wrong, R2-H02 class).
    let prefix_last = last_block_entry(&entries[..prefix_len]);

    if let (Some(a), Some(first_block)) = (prefix_last.as_ref(), rp.blocks.first()) {
        let a_end = a.span().1;
        if continuation_pair(a_end, entry_facts(a), first_block.start(), post, cx) {
            return true;
        }
    }

    let (right_end, right_facts) = match rp.blocks.last() {
        Some(sk) => (sk.end(), block_facts(sk, post)),
        None => match prefix_last.as_ref() {
            Some(e) => (e.span().1, entry_facts(e).clone()),
            None => return false,
        },
    };
    let suffix_first_block = entries[suffix_from..].iter().find_map(|e| match e {
        TopEntry::Block { .. } => Some(e.span().0),
        TopEntry::Blank { .. } => None,
    });
    if let Some(old_start) = suffix_first_block {
        let b_start = (old_start as isize + delta) as usize;
        if continuation_pair(right_end, &right_facts, b_start, post, cx) {
            return true;
        }
    }
    false
}

/// One edge pair: can block `a` (facts + end position) continue onto the
/// line of block `b`'s start? Fires when no blank line separates them and
/// `a`'s continuation condition holds for `b`'s first line.
fn continuation_pair<W: WorkSink>(
    a_end: usize,
    a_facts: &BlockFacts,
    b_start: usize,
    post: &[u8],
    cx: &mut MechanismContext<'_, W>,
) -> bool {
    if b_start <= a_end {
        // Spans must be ordered; an overlap means the region mapping is
        // inconsistent — refuse to trust it.
        return true;
    }
    // The guard read these bytes: report the inspection honestly.
    cx.sink
        .record_source_inspection(a_end as u64, b_start as u64);
    let lfs = post[a_end..b_start].iter().filter(|&&b| b == b'\n').count();
    if lfs >= 2 {
        return false; // a blank line separates: no continuation is possible
    }
    would_continue(a_facts, post, b_start)
}

/// Can the block with `facts` continue onto the line containing
/// `b_start` (the next block's first line, POST coordinates)?
fn would_continue(facts: &BlockFacts, post: &[u8], b_start: usize) -> bool {
    let ls = line_start_of(post, b_start);
    let lf = sg::parser::memchr_lf(post, ls);
    match facts {
        BlockFacts::Terminated => false,
        BlockFacts::Para => matches!(
            classify_top_level_line(post, ls, lf),
            LineClass::ParagraphText
        ),
        BlockFacts::Quote => {
            matches!(
                classify_top_level_line(post, ls, lf),
                LineClass::QuoteMarker
            )
        }
        BlockFacts::List { indent, strip } => {
            if sg::parser::all_spaces(post, ls, lf) {
                return false; // a blank line never continues a list
            }
            let s = sg::parser::count_spaces(post, ls, lf);
            // Sibling marker at exactly the list indent (§7), or content
            // at/inside the last item's strip.
            (s == *indent && sg::parser::parse_marker(post, ls + s, lf).is_some()) || s >= *strip
        }
    }
}

fn last_block_entry(entries: &[TopEntry]) -> Option<&TopEntry> {
    entries
        .iter()
        .rev()
        .find(|e| matches!(e, TopEntry::Block { .. }))
}

fn entry_facts(e: &TopEntry) -> &BlockFacts {
    match e {
        TopEntry::Block { facts, .. } => facts,
        TopEntry::Blank { .. } => unreachable!("blank entries carry no facts"),
    }
}

// ---------------------------------------------------------------------------
// Suffix reconstruction (delta shift)
// ---------------------------------------------------------------------------

/// Reconstruct an OWNED entry with every stored span shifted by `delta`
/// (mizchi shift_block_span): the skeleton AND the retained semantic
/// subtree. Parser work is avoided, but the representation is rebuilt —
/// counted as `nodes_rebuilt`, never `nodes_reused` (R5-CORRECTIVE-1
/// §5).
fn shift_entry_owned(e: TopEntry, delta: isize) -> TopEntry {
    match e {
        TopEntry::Blank { start, end } => TopEntry::Blank {
            start: shift_pos(start, delta),
            end: shift_pos(end, delta),
        },
        TopEntry::Block {
            skel,
            sem,
            facts, // indent/strip facts are shift-invariant
        } => TopEntry::Block {
            skel: shift_skel(&skel, delta),
            sem: shift_node_owned(sem, delta),
            facts,
        },
    }
}

/// Reconstruct an owned normalized subtree with every span shifted by
/// `delta`, preserving all semantic values (pure representation rebuild).
/// Position-bearing fields: the node span, the `FencedCode.content`
/// interval, and (recursively) the children.
fn shift_node_owned(mut n: Node, delta: isize) -> Node {
    n.start = shift_pos(n.start, delta);
    n.end = shift_pos(n.end, delta);
    if let Some((a, b)) = n.content {
        n.content = Some((shift_pos(a, delta), shift_pos(b, delta)));
    }
    n.children = n
        .children
        .into_iter()
        .map(|c| shift_node_owned(c, delta))
        .collect();
    n
}

fn shift_pos(p: usize, delta: isize) -> usize {
    (p as isize + delta) as usize
}

/// Reconstruct a Skel with every stored span shifted by `delta`
/// (mizchi shift_block_span).
fn shift_skel(s: &Skel, delta: isize) -> Skel {
    match s {
        Skel::Para {
            start,
            end,
            segments,
            ctx,
        } => Skel::Para {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            segments: segments
                .iter()
                .map(|&(a, b)| (shift_pos(a, delta), shift_pos(b, delta)))
                .collect(),
            ctx: ctx.clone(),
        },
        Skel::Heading {
            start,
            end,
            level,
            content,
            ctx,
        } => Skel::Heading {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            level: *level,
            content: (shift_pos(content.0, delta), shift_pos(content.1, delta)),
            ctx: ctx.clone(),
        },
        Skel::Quote {
            start,
            end,
            children,
            ctx,
        } => Skel::Quote {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            children: children.iter().map(|c| shift_skel(c, delta)).collect(),
            ctx: ctx.clone(),
        },
        Skel::List {
            start,
            end,
            items,
            ctx,
        } => Skel::List {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            items: items.iter().map(|c| shift_skel(c, delta)).collect(),
            ctx: ctx.clone(),
        },
        Skel::Item {
            start,
            end,
            marker,
            children,
            ctx,
        } => Skel::Item {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            marker: *marker,
            children: children.iter().map(|c| shift_skel(c, delta)).collect(),
            ctx: ctx.clone(),
        },
        Skel::Fence {
            start,
            end,
            info,
            content,
            ctx,
        } => Skel::Fence {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            info: info.clone(),
            content: (shift_pos(content.0, delta), shift_pos(content.1, delta)),
            ctx: ctx.clone(),
        },
        Skel::Def {
            start,
            end,
            label,
            destination,
            ctx,
        } => Skel::Def {
            start: shift_pos(*start, delta),
            end: shift_pos(*end, delta),
            label: label.clone(),
            destination: destination.clone(),
            ctx: ctx.clone(),
        },
        Skel::Spliced { .. } => unreachable!("H1 never splices"),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursive count of BENCH-GRAMMAR block parse units in a skeleton
/// forest (Paragraph, Heading, BlockQuote, List, ListItem, FencedCode,
/// ReferenceDefinition — the H0 block-kind set).
pub fn count_blocks(blocks: &[Skel]) -> u64 {
    fn one(s: &Skel) -> u64 {
        1 + match s {
            Skel::Quote { children, .. } => count_blocks(children),
            Skel::List { items, .. } => count_blocks(items),
            Skel::Item { children, .. } => count_blocks(children),
            _ => 0,
        }
    }
    blocks.iter().map(one).sum()
}

/// Native node count of one tiling entry under the corrective counting
/// rule (R5-CORRECTIVE-1 §8): one structural block/container node per
/// block unit (counted from the skeleton) PLUS every retained inline
/// syntax node of the materialized subtree (recursive). The materialized
/// subtree's own block-kind nodes are the SAME structural units the
/// skeleton encodes — they are counted once, not twice. Blank entries
/// store no syntax.
fn entry_native_nodes(e: &TopEntry) -> u64 {
    match e {
        TopEntry::Block { skel, sem, .. } => {
            count_blocks(std::slice::from_ref(skel))
                + count_inline_forest(std::slice::from_ref(sem))
        }
        TopEntry::Blank { .. } => 0,
    }
}

/// Recursive count of INLINE syntax nodes in a normalized forest. The
/// inline scanner produces only inline-kind nodes below the block node,
/// so this counts every retained inline syntax node exactly once.
fn count_inline_forest(nodes: &[Node]) -> u64 {
    nodes.iter().map(count_inline_node).sum()
}

fn count_inline_node(n: &Node) -> u64 {
    let here = matches!(
        n.kind,
        NodeKind::Text
            | NodeKind::Emphasis
            | NodeKind::CodeSpan
            | NodeKind::Link
            | NodeKind::ReferenceLink
    ) as u64;
    here + count_inline_forest(&n.children)
}

fn line_start_of(src: &[u8], pos: usize) -> usize {
    src[..pos]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |p| p + 1)
}

fn ref_table(defs: &[(String, String)]) -> sg::RefTable {
    let mut t = sg::RefTable::new();
    t.extend_from(defs);
    t
}

/// H1 owns `fallback_to_full_count` and `metadata_records_touched` (Known
/// slots); the restart/convergence gauges have no H1 referent and are
/// declared NotApplicable in every working phase.
fn declare_gauges_not_applicable<W: WorkSink>(cx: &mut MechanismContext<'_, W>) {
    cx.sink.set_restart_distance(Observed::NotApplicable);
    cx.sink.set_convergence_distance(Observed::NotApplicable);
}
