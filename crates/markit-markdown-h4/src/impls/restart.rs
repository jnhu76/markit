//! The H4 RESTART_CONVERGENCE update mechanism (product transplant).
//!
//! Mechanism identity (donor freeze
//! `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §9; checkpoint/restart
//! model): retain the old top-level blocks (shared `Arc<RetainedBlock>`
//! identity — the block's skeleton AND its materialized semantic
//! subtree, R5-CORRECTIVE-1 §11.4) plus a CHECKPOINT RECORD at every
//! top-level block start — the line-aligned position, the entry
//! [`ContextKey`], and the definition generation the record was
//! validated under. On an edit, select the restart checkpoint at/before
//! the damage; parse the post source forward from the restart (the
//! prefix before it is retained untouched); at each live block start
//! beyond the damage compare the live parser state against the MAPPED
//! old checkpoint (`q = p − delta`); when the frozen convergence
//! predicate holds, reuse the old stable suffix to EOF (structural
//! sharing: the suffix keeps its `Arc<RetainedBlock>`s, retargeted by
//! per-block base offsets).
//!
//! Convergence predicate (all parts source/state-derived, frozen):
//! (a) an old checkpoint sits exactly at the mapped position `q`;
//! (b) its entry `ContextKey` equals the live parser state (frame and
//!     fence agreement);
//! (c) its generation equals the current definition generation;
//! (d) `q` is at/beyond the end of every damaged old entry;
//! (e) the live side is blank-line separated: the line immediately
//!     before `p` is blank. The `ContextKey` deliberately excludes
//!     paragraph state (R5 freeze §2), so (e) is the paragraph margin:
//!     a blank line terminates every continuation (§3, D5, §6), which
//!     guarantees no paragraph is open at the splice in either parse.
//!     (An old checkpoint whose preceding gap carries no blank line can
//!     only be a non-paragraph interruptor; refusing those cases costs
//!     reuse, never correctness.)
//!
//! Definition-changing damage (a damaged subtree contains a
//! ReferenceDefinition, or the edited region creates one — any `]: `
//! occurrence in the edited span's post bytes) restarts at ZERO and
//! parses to EOF: reference resolution is document-global, so no suffix
//! can be vouched. This is a RESTART, not a fallback — H4 has no
//! degraded mode and `fallback_to_full_count` is NotApplicable to it.
//! The generation counter increments; every re-registered checkpoint
//! carries the new generation.
//!
//! This crate owns ONLY H4 mechanism state and policy. Grammar semantics
//! live in `markit-mdbench-shared-grammar` (the convergence consult is
//! horse-supplied through the splice hook — the scanner never decides
//! reuse); the result vocabulary and validation in `markit-mdbench-oracle`.
//! H4 never calls H0; test crates may use H0 as the correctness oracle.

use std::sync::Arc;

use super::normalize::{Node, NodeKind, NormalizedDocument};

use crate::grammar::{self as sg};
use sg::parser::{ContextKey, Skel, SpliceHook};
// ---------------------------------------------------------------------------
// Retained native state (checkpoints + shared blocks with base offsets)
// ---------------------------------------------------------------------------

/// One retained top-level block: the shared identity unit. It carries
/// the block skeleton AND its already-materialized semantic subtree
/// (`sem`, R5-CORRECTIVE-1 §6), so retained reuse (stable prefix,
/// converged suffix) shares COMPLETE syntax — the projection never
/// rescans inline content. `sem`'s spans are absolute in the coordinate
/// system the block was parsed in; the current-document position derives
/// purely at projection time via the slot's `base_shift`.
#[derive(Debug, Clone)]
pub(crate) struct RetainedBlock {
    pub(crate) skel: Skel,
    pub(crate) sem: Node,
}

/// One retained top-level slot: a shared block plus its base offset.
/// The block's absolute span in the CURRENT document is
/// `skel.start() + base_shift` — suffix reuse retargets the shared
/// `Arc<RetainedBlock>` by adjusting `base_shift` alone (structural
/// sharing; the normalized projection applies the shift purely).
/// `line_offset` is the distance from the block's line start to its span
/// start (always 0 for top-level blocks in this grammar; kept as the
/// general line-alignment bookkeeping).
#[derive(Debug, Clone)]
pub(crate) struct BlockSlot {
    pub base_shift: isize,
    pub line_offset: usize,
    pub block: Arc<RetainedBlock>,
}

impl BlockSlot {
    /// Absolute span start in the current document's coordinates.
    pub(crate) fn abs_start(&self) -> usize {
        (self.block.skel.start() as isize + self.base_shift) as usize
    }

    /// Absolute span end in the current document's coordinates.
    pub(crate) fn abs_end(&self) -> usize {
        (self.block.skel.end() as isize + self.base_shift) as usize
    }

    /// The block's LINE start in the current coordinates — the checkpoint
    /// position.
    pub(crate) fn line_position(&self) -> usize {
        self.abs_start() - self.line_offset
    }
}

/// A retained checkpoint record: proof that a top-level block started at
/// `position` (line-aligned) under entry parser state `key`, validated
/// against the definition table of generation `generation`.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub position: usize,
    pub key: ContextKey,
    pub generation: u64,
}

/// H4 retained state: top-level blocks with base offsets, the parallel
/// checkpoint table (1:1), the definition facts, and the current
/// definition generation.
#[derive(Debug, Clone, Default)]
pub struct H4State {
    blocks: Vec<BlockSlot>,
    checkpoints: Vec<Checkpoint>,
    generation: u64,
    defs: Vec<(String, String)>,
    src_len: usize,
}

/// The pure projection over the retained semantic subtrees — no source,
/// no parser. Retained suffix reuse shares complete syntax through the
/// `Arc<RetainedBlock>` identity; the projection only applies each
/// slot's `base_shift`.
pub(crate) fn project_state(state: &H4State) -> NormalizedDocument {
    project(&state.blocks, state.src_len)
}

/// The restart-selection and damage facts the convergence predicate
/// needs (donor `H4Prepared`).
#[derive(Debug, Clone)]
struct H4Prepared {
    edit_start: usize,
    edit_end_new: usize,
    delta: isize,
    /// Line-aligned position of the restart checkpoint (0 when no
    /// checkpoint precedes the damage).
    restart_position: usize,
    /// Index of the restart checkpoint = the number of retained prefix
    /// blocks.
    restart_slot: usize,
    /// Max end (old coordinates) over damaged old entries; 0 when the
    /// edit intersects no entry span.
    damaged_end: usize,
    damaged_has_def: bool,
}

// ---------------------------------------------------------------------------
// Product face: one H4 document's retained state
// ---------------------------------------------------------------------------

/// One H4-backed document: the retained mechanism state plus its pure
/// projection. Provider-private representation; consumers see only the
/// product Markdown contract's read views.
#[derive(Debug, Clone)]
pub(crate) struct H4Document {
    state: H4State,
}

impl H4Document {
    /// Clean parse (the restart model's control operation: generation 0,
    /// checkpoints at every top-level block).
    pub(crate) fn parse(src: &[u8]) -> Self {
        let rp = sg::parser::parse_region(src, 0, src.len());
        // Definition facts come from the block skeletons (document
        // order); the table must exist before the inline pass
        // materializes the retained semantic subtrees.
        let mut defs = Vec::new();
        for sk in &rp.blocks {
            collect_defs_skel(sk, &mut defs);
        }
        let table = ref_table(&defs);
        let slots = fresh_slots(&rp.blocks, src, &table);
        let checkpoints = registers(&slots, 0);
        Self {
            state: H4State {
                blocks: slots,
                checkpoints,
                generation: 0,
                defs,
                src_len: src.len(),
            },
        }
    }

    /// One committed byte replacement: the old source's bytes
    /// `[edit_start, edit_end_old)` were replaced by `inserted_len` new
    /// bytes, and `post` is the complete post source. `old` is the old
    /// source (read by restart-boundary margin checks). Correctness never
    /// depends on the edit being small.
    pub(crate) fn update(
        &mut self,
        old: &[u8],
        post: &[u8],
        edit_start: usize,
        edit_end_old: usize,
        inserted_len: usize,
    ) {
        let old_state = std::mem::take(&mut self.state);
        self.state = update_state(
            &old_state,
            old,
            post,
            edit_start,
            edit_end_old,
            inserted_len,
        );
    }

    /// The pure projection over the retained semantic subtrees — no
    /// source, no parser.
    pub(crate) fn project(&self) -> NormalizedDocument {
        project_state(&self.state)
    }
}

/// The whole update: restart selection, damage scan, definition fast
/// path, forward pass with the convergence consult, assembly, and the
/// sound definition-environment re-check (donor prepare/update, merged;
/// the split was a measurement boundary).
fn update_state(
    old_state: &H4State,
    old: &[u8],
    post: &[u8],
    es: usize,
    ee: usize,
    inserted_len: usize,
) -> H4State {
    let prepared = prepare(old_state, old, post, es, ee, inserted_len);
    finish_update(old_state, post, es, inserted_len, prepared)
}

/// RESTART AT ZERO — the frozen H4 response to definition-changing
/// damage: parse everything fresh through the same forward machinery (not
/// a fallback — H4 has no degraded mode), re-register every checkpoint at
/// the next generation, and retain nothing from the old state.
///
/// Used by BOTH sound detection paths: the pre-parse source-local fast
/// path (`damaged_has_def` / `]: ` in the edited span) and the assembled
/// definition-environment comparison after the forward pass.
fn restart_at_zero(post: &[u8], old_gen: u64) -> H4State {
    let rp = sg::parser::parse_region(post, 0, post.len());
    let mut defs = Vec::new();
    for sk in &rp.blocks {
        collect_defs_skel(sk, &mut defs);
    }
    let table = ref_table(&defs);
    let slots = fresh_slots(&rp.blocks, post, &table);
    let generation = old_gen + 1;
    let checkpoints = registers(&slots, generation);
    H4State {
        blocks: slots,
        checkpoints,
        generation,
        defs,
        src_len: post.len(),
    }
}

/// Restart selection, continuation-margin backoff, and the damage scan
/// (donor `prepare_update`).
fn prepare(
    old_state: &H4State,
    old: &[u8],
    _post: &[u8],
    es: usize,
    ee: usize,
    inserted_len: usize,
) -> H4Prepared {
    {
        let delta = inserted_len as isize - (ee - es) as isize;
        let ee_new = es + inserted_len;

        // RESTART SELECTION: the last checkpoint at/before the damage.
        // Its existence proves an old block boundary there, and the bytes
        // before the edit are unchanged — the prefix parse carries over
        // untouched (restart-boundary soundness, R5 freeze §9 notes).
        let mut restart_slot = old_state
            .checkpoints
            .iter()
            .rposition(|cp| cp.position <= es);

        // RESTART-BOUNDARY CONTINUATION MARGIN (the H1-F2 / H3-margin
        // analogue at the restart): the retained prefix's LAST block can
        // continue into the reparsed region when no blank line separates
        // it from the restart — a paragraph by continuation text, a quote
        // by a line regaining its `> ` prefix, a list item by gained
        // indentation. An UNCHANGED boundary line cannot continue any of
        // them (the old parse proves it: a line carrying the prefix would
        // have continued the block, so no boundary would exist), and an
        // interrupting dispatch is determined by the unchanged line
        // prefix. When the edit reaches INTO the boundary line's bytes,
        // the boundary may have vanished: back the restart up one
        // checkpoint, so the merge happens inside the reparsed region.
        if let Some(s) = restart_slot {
            let mut s = s;
            while s > 0 {
                let prev = &old_state.blocks[s - 1];
                let boundary = old_state.checkpoints[s].position;
                let (sep_lo, sep_hi) = (prev.abs_end().min(old.len()), boundary.min(old.len()));
                let sep_lfs = old[sep_lo..sep_hi].iter().filter(|&&b| b == b'\n').count();
                if sep_lfs >= 2 {
                    break; // blank line terminates every continuation
                }
                let k = &old_state.blocks[s];
                let k_start = k.abs_start();
                let k_line_end = sg::parser::memchr_lf(old, k_start.min(old.len()));
                if es >= k_line_end {
                    break; // the boundary line is intact
                }
                s -= 1;
            }
            restart_slot = Some(s);
        }
        let (restart_position, restart_slot) = restart_slot.map_or((0usize, 0usize), |i| {
            (
                if i == 0 {
                    0
                } else {
                    old_state.checkpoints[i].position
                },
                i,
            )
        });

        // DAMAGE SCAN: which old entries does the edit intersect, and does
        // any of them carry a ReferenceDefinition (definition-changing)?
        let mut damaged_end = 0usize;
        let mut damaged_has_def = false;
        for slot in &old_state.blocks {
            let (s, e) = (slot.abs_start(), slot.abs_end());
            if s < ee && e > es {
                damaged_end = damaged_end.max(e);
                damaged_has_def |= skel_has_def(&slot.block.skel);
            }
        }
        H4Prepared {
            edit_start: es,
            edit_end_new: ee_new,
            delta,
            restart_position,
            restart_slot,
            damaged_end,
            damaged_has_def,
        }
    }
}

/// The forward pass, assembly, definition-environment re-check, and (on
/// failure) restart (donor `update`).
fn finish_update(
    old_state: &H4State,
    post: &[u8],
    _es: usize,
    _inserted_len: usize,
    prepared: H4Prepared,
) -> H4State {
    {
        let es = prepared.edit_start;
        let ee_new = prepared.edit_end_new;
        let delta = prepared.delta;

        // The reparsed region may itself create a definition: any `]: `
        // occurrence inside the edited span's post bytes (source-derived,
        // pre-computable).
        let definition_changing =
            prepared.damaged_has_def || post[es..ee_new].windows(3).any(|w| w == b"]: ");

        if definition_changing {
            return restart_at_zero(post, old_state.generation);
        }

        // FORWARD PASS from the restart checkpoint. The prefix before it
        // is retained untouched; the region [r, post.len()) parses fresh
        // until the convergence consult takes the old stable suffix.
        let r = prepared.restart_position;
        let mut cursor = Cursor {
            checkpoints: &old_state.checkpoints,
            post,
            ee_new,
            delta,
            damaged_end: prepared.damaged_end,
            generation: old_state.generation,
            take: None,
        };
        // (The third hook argument — `starts_block` — is the #79
        // scanner-protocol addition; H4's restart margins gate takes
        // independently, so it is not consumed here.)
        let mut hook: Box<SpliceHook<'_>> =
            Box::new(|pos, key, _starts_block| cursor.consult(pos, key));
        let (rp, _slot_count) = sg::parser::parse_region_with_hook(post, r, post.len(), &mut hook);
        drop(hook); // end the cursor borrow before reading the take record
        let take = cursor.take;

        // Assemble: retained prefix + fresh region blocks + (at the splice)
        // the retained suffix with retargeted base offsets. Retained
        // blocks carry their old checkpoint records (key + generation
        // provenance); fresh blocks register at the current generation.
        // MAJOR (R5-CORRECTIVE-1 §6): retained blocks share the whole
        // `Arc<RetainedBlock>` — skeleton AND materialized semantic
        // subtree — so neither the prefix nor the converged suffix
        // re-reads or re-scans a single byte of inline content.
        let generation = old_state.generation;
        let mut pairs: Vec<(BlockSlot, Option<Checkpoint>)> = Vec::new();
        // The retained prefix moves in by shared `Arc<RetainedBlock>`
        // identity — zero parser source reads, zero reconstruction —
        // exactly like the converged suffix.
        for (s, cp) in old_state.blocks[..prepared.restart_slot]
            .iter()
            .zip(&old_state.checkpoints[..prepared.restart_slot])
        {
            pairs.push((
                BlockSlot {
                    base_shift: s.base_shift,
                    line_offset: s.line_offset,
                    block: Arc::clone(&s.block),
                },
                Some(Checkpoint {
                    position: 0,
                    key: cp.key.clone(),
                    generation: cp.generation,
                }),
            ));
        }
        // Document-order definition table for the assembled state:
        // retained prefix facts, then (at the splice position) the
        // retained suffix facts, with the fresh region's facts in their
        // document positions. Computed from skeletons BEFORE the fresh
        // inline pass materializes, so the table is complete.
        let mut defs: Vec<(String, String)> = Vec::new();
        for s in &old_state.blocks[..prepared.restart_slot] {
            collect_defs_skel(&s.block.skel, &mut defs);
        }
        let mut suffix_defs_done = false;
        for sk in &rp.blocks {
            if matches!(sk, Skel::Spliced { .. }) {
                let (old_idx, _) = take.expect("splice placeholder without a recorded take");
                for s in &old_state.blocks[old_idx..] {
                    collect_defs_skel(&s.block.skel, &mut defs);
                }
                suffix_defs_done = true;
            } else {
                collect_defs_skel(sk, &mut defs);
            }
        }
        debug_assert!(suffix_defs_done || take.is_none());
        let table = ref_table(&defs);

        // SOUND DEFINITION-ENVIRONMENT CHECK (the detection the frozen
        // model always required). The pre-parse probe above is a
        // source-local FAST PATH, not a proof: an edit whose own bytes
        // look definition-free can still change whether OTHER bytes are
        // definitions at all — deleting a fence closer turns the rest of
        // the document into fence content, so the reference definitions
        // inside it leave the table without a single definition byte
        // changing. The assembled table (retained prefix facts + the
        // region's fresh facts + the retained suffix's facts, in
        // document order) is exactly the environment the retained
        // semantic subtrees would be reinterpreted under, and it is
        // complete BEFORE any materialization happens. If it differs
        // from the retained one, the retained prefix/suffix semantics are
        // stale: H4's frozen response to definition-changing damage
        // applies — RESTART AT ZERO at the next generation.
        if table.entries() != old_state.defs.as_slice() {
            // The forward pass really did its work and is discarded: an
            // edit whose own bytes look definition-free can still change
            // whether OTHER bytes are definitions at all (e.g. deleting a
            // fence closer turns the rest of the document into fence
            // content), so the assembled table — the environment the
            // retained semantic subtrees would be reinterpreted under —
            // is re-checked before any retained materialization. H4's
            // frozen response applies: RESTART AT ZERO at the next
            // generation.
            return restart_at_zero(post, old_state.generation);
        }

        for sk in &rp.blocks {
            match sk {
                Skel::Spliced { slot, .. } => {
                    debug_assert_eq!(*slot, 0, "at most one convergence take per update");
                    let Some((old_idx, _)) = take else {
                        unreachable!("splice placeholder without a recorded take")
                    };
                    for (s, cp) in old_state.blocks[old_idx..]
                        .iter()
                        .zip(&old_state.checkpoints[old_idx..])
                    {
                        pairs.push((
                            BlockSlot {
                                base_shift: s.base_shift + delta,
                                line_offset: s.line_offset,
                                block: Arc::clone(&s.block),
                            },
                            Some(Checkpoint {
                                position: 0,
                                key: cp.key.clone(),
                                generation: cp.generation,
                            }),
                        ));
                    }
                }
                other => {
                    let start = other.start();
                    let sem = sg::inline::materialize_one(post, other.clone(), &table);
                    pairs.push((
                        BlockSlot {
                            base_shift: 0,
                            line_offset: start - sg::parser::line_start_of(post, start),
                            block: Arc::new(RetainedBlock {
                                skel: other.clone(),
                                sem,
                            }),
                        },
                        None,
                    ));
                }
            }
        }
        let mut slots = Vec::with_capacity(pairs.len());
        let mut checkpoints = Vec::with_capacity(pairs.len());
        for (slot, cp) in pairs {
            let position = slot.line_position();
            checkpoints.push(match cp {
                Some(mut c) => {
                    c.position = position;
                    c
                }
                None => Checkpoint {
                    position,
                    key: slot.block.skel.ctx().clone(),
                    generation,
                },
            });
            slots.push(slot);
        }

        H4State {
            blocks: slots,
            checkpoints,
            generation,
            defs,
            src_len: post.len(),
        }
    }
}

// ---------------------------------------------------------------------------
// Convergence cursor (the horse-supplied splice consult)
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    checkpoints: &'a [Checkpoint],
    post: &'a [u8],
    /// Edited span end, POST coordinates.
    ee_new: usize,
    delta: isize,
    /// Max damaged old entry end (old coordinates); predicate (d).
    damaged_end: usize,
    /// Current definition generation; predicate (c).
    generation: u64,
    /// The single convergence take: (old suffix start index, take start
    /// in post coordinates).
    take: Option<(usize, usize)>,
}

impl Cursor<'_> {
    /// The convergence consult at a live block-start line `pos` with live
    /// state `key`. Returns the take end (the document end — the old
    /// stable suffix runs to EOF) or `None` (keep parsing).
    fn consult(&mut self, pos: usize, key: &ContextKey) -> Option<usize> {
        if self.take.is_some() {
            return None; // one convergence per update
        }
        // Only block starts strictly beyond the damage are candidates.
        if pos <= self.ee_new || pos >= self.post.len() {
            return None;
        }
        // (a) an old checkpoint exactly at the mapped position.
        let q = map_back(pos, self.delta);
        let Ok(idx) = self.checkpoints.binary_search_by(|cp| cp.position.cmp(&q)) else {
            return None;
        };
        let cp = &self.checkpoints[idx];
        // (b) state agreement (frames + fence).
        if cp.key != *key {
            return None;
        }
        // (c) generation agreement (definition provenance).
        if cp.generation != self.generation {
            return None;
        }
        // (d) beyond every damaged old entry.
        if q < self.damaged_end {
            return None;
        }
        // (e) the paragraph margin: the line immediately before the splice
        // is blank, so no paragraph is open in the live parse (a blank
        // line terminates every continuation), matching the checkpoint's
        // proven block boundary. Source-derived: post[prev_line] blank.
        let prev_ls = sg::parser::line_start_of(self.post, pos - 1);
        if !sg::parser::all_spaces(self.post, prev_ls, pos - 1) {
            return None;
        }
        self.take = Some((idx, pos));
        Some(self.post.len())
    }
}

/// Map a post-coordinate position back to old coordinates: bytes before
/// the edit are identical, bytes after it shifted by `delta`.
fn map_back(pos: usize, delta: isize) -> usize {
    if delta >= 0 {
        pos - delta as usize
    } else {
        pos + delta.unsigned_abs()
    }
}

// ---------------------------------------------------------------------------
// State construction helpers
// ---------------------------------------------------------------------------

/// Fresh slots from parsed blocks (absolute spans in `src`, base offset
/// 0). Each block's semantic subtree is materialized here, so retained
/// reuse later shares complete syntax.
fn fresh_slots(blocks: &[Skel], src: &[u8], table: &sg::inline::RefTable) -> Vec<BlockSlot> {
    blocks
        .iter()
        .map(|sk| {
            let start = sk.start();
            let sem = sg::inline::materialize_one(src, sk.clone(), table);
            BlockSlot {
                base_shift: 0,
                line_offset: start - sg::parser::line_start_of(src, start),
                block: Arc::new(RetainedBlock {
                    skel: sk.clone(),
                    sem,
                }),
            }
        })
        .collect()
}

/// Register a checkpoint for every slot at generation `generation`.
fn registers(slots: &[BlockSlot], generation: u64) -> Vec<Checkpoint> {
    slots
        .iter()
        .map(|s| Checkpoint {
            position: s.line_position(),
            key: s.block.skel.ctx().clone(),
            generation,
        })
        .collect()
}

/// Whether the subtree contains a ReferenceDefinition.
fn skel_has_def(sk: &Skel) -> bool {
    match sk {
        Skel::Def { .. } => true,
        Skel::Quote { children, .. }
        | Skel::List {
            items: children, ..
        } => children.iter().any(skel_has_def),
        Skel::Item { children, .. } => children.iter().any(skel_has_def),
        _ => false,
    }
}

fn collect_defs_skel(sk: &Skel, defs: &mut Vec<(String, String)>) {
    match sk {
        Skel::Def {
            label, destination, ..
        } => defs.push((label.clone(), destination.clone())),
        Skel::Quote { children, .. }
        | Skel::List {
            items: children, ..
        } => {
            for c in children {
                collect_defs_skel(c, defs);
            }
        }
        Skel::Item { children, .. } => {
            for c in children {
                collect_defs_skel(c, defs);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Projection (retained shared Skels + base offsets -> NORMALIZED-RESULT-v1)
// ---------------------------------------------------------------------------

/// Pure projection of the completed state (R5-CORRECTIVE-1 §6/MAJOR-3):
/// clone each slot's retained semantic subtree, applying the slot's
/// `base_shift` to its spans when nonzero. No source, no sink, no
/// parser, no repair — the retained representation carries every syntax
/// fact.
fn project(slots: &[BlockSlot], src_len: usize) -> NormalizedDocument {
    let mut root = Node::new(NodeKind::Document, 0, src_len);
    root.children = slots
        .iter()
        .map(|s| {
            if s.base_shift == 0 {
                s.block.sem.clone()
            } else {
                shift_node_owned(s.block.sem.clone(), s.base_shift)
            }
        })
        .collect();
    NormalizedDocument::new(root)
}

/// Deep clone of a normalized subtree with every span shifted by `base`,
/// preserving all semantic values (pure representation retargeting).
/// Position-bearing fields: the node span, the `FencedCode.content`
/// interval, and (recursively) the children.
fn shift_node_owned(mut n: Node, base: isize) -> Node {
    let sh = |p: usize| (p as isize + base) as usize;
    n.start = sh(n.start);
    n.end = sh(n.end);
    if let Some((a, b)) = n.content {
        n.content = Some((sh(a), sh(b)));
    }
    n.children = n
        .children
        .into_iter()
        .map(|c| shift_node_owned(c, base))
        .collect();
    n
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ref_table(defs: &[(String, String)]) -> sg::inline::RefTable {
    let mut t = sg::inline::RefTable::new();
    t.extend_from(defs);
    t
}
