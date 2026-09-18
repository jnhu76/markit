//! markit-mdbench-restart-convergence — H4 RESTART_CONVERGENCE
//! (#22, stage R5).
//!
//! Mechanism identity (frozen in
//! `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §9; checkpoint/restart
//! model): retain the old top-level blocks (shared `Arc<Skel>` identity)
//! plus a CHECKPOINT RECORD at every top-level block start — the
//! line-aligned position, the entry [`ContextKey`], and the definition
//! generation the record was validated under. On an edit, select the
//! restart checkpoint at/before the damage; parse the post source
//! forward from the restart (the prefix before it is retained untouched);
//! at each live block start beyond the damage compare the live parser
//! state against the MAPPED old checkpoint (`q = p − delta`); when the
//! frozen convergence predicate holds, reuse the old stable suffix to EOF
//! (structural sharing: the suffix keeps its `Arc<Skel>`s, retargeted by
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

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::Completed;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::NotApplicableSlot;
use markit_mdbench_common::Observed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::{normalized_checksum, NormalizedDocument};
use markit_mdbench_shared_grammar as sg;
use sg::parser::{parse_region_with_hook, ContextKey, Skel, SpliceHook};

/// Mechanism identifier.
pub const H4_MECHANISM_ID: &str = "restart-convergence-h4";

// ---------------------------------------------------------------------------
// Retained native state (checkpoints + shared blocks with base offsets)
// ---------------------------------------------------------------------------

/// One retained top-level block: a shared skeleton plus its base offset.
/// The block's absolute span in the CURRENT document is
/// `skel.start() + base_shift` — suffix reuse retargets the shared
/// `Arc<Skel>` by adjusting `base_shift` alone (structural sharing).
/// `line_offset` is the distance from the block's line start to its span
/// start (always 0 for top-level blocks in this grammar; kept as the
/// general line-alignment bookkeeping).
#[derive(Debug, Clone)]
pub struct BlockSlot {
    pub base_shift: isize,
    pub line_offset: usize,
    pub block: Arc<Skel>,
}

impl BlockSlot {
    /// Absolute span start in the current document's coordinates.
    pub fn abs_start(&self) -> usize {
        (self.block.start() as isize + self.base_shift) as usize
    }

    /// Absolute span end in the current document's coordinates.
    pub fn abs_end(&self) -> usize {
        (self.block.end() as isize + self.base_shift) as usize
    }

    /// The block's LINE start in the current coordinates — the checkpoint
    /// position.
    pub fn line_position(&self) -> usize {
        self.abs_start() - self.line_offset
    }
}

/// A retained checkpoint record: proof that a top-level block started at
/// `position` (line-aligned) under entry parser state `key`, validated
/// against the definition table of generation `gen`.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub position: usize,
    pub key: ContextKey,
    pub gen: u64,
}

/// H4 retained state: top-level blocks with base offsets, the parallel
/// checkpoint table (1:1), the definition facts, and the current
/// definition generation.
#[derive(Debug, Clone, Default)]
pub struct H4State {
    blocks: Vec<BlockSlot>,
    checkpoints: Vec<Checkpoint>,
    gen: u64,
    defs: Vec<(String, String)>,
    src_len: usize,
}

impl H4State {
    pub fn blocks(&self) -> &[BlockSlot] {
        &self.blocks
    }

    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    pub fn generation(&self) -> u64 {
        self.gen
    }

    /// Retained definition facts in document order (first-wins already
    /// applied).
    pub fn defs(&self) -> &[(String, String)] {
        &self.defs
    }

    pub fn src_len(&self) -> usize {
        self.src_len
    }
}

/// Pending work handed to `complete()` — already fully materialized
/// (eager completion boundary, R5 freeze §4).
pub struct H4Pending {
    state: H4State,
    result: NormalizedDocument,
}

impl H4Pending {
    pub fn result(&self) -> &NormalizedDocument {
        &self.result
    }
}

/// `prepare_update` product: edit coordinates, the selected restart
/// checkpoint, and the damage facts the convergence predicate needs.
#[derive(Debug, Clone)]
pub struct H4Prepared {
    pub edit_start: usize,
    pub edit_end_new: usize,
    pub delta: isize,
    /// Line-aligned position of the restart checkpoint (0 when no
    /// checkpoint precedes the damage).
    pub restart_position: usize,
    /// Index of the restart checkpoint = the number of retained prefix
    /// blocks.
    pub restart_slot: usize,
    /// Max end (old coordinates) over damaged old entries; 0 when the
    /// edit intersects no entry span.
    pub damaged_end: usize,
    pub damaged_entries: u64,
    pub damaged_has_def: bool,
    pub scanned_checkpoints: u64,
}

// ---------------------------------------------------------------------------
// Mechanism
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct RestartConvergenceMechanism;

impl RestartConvergenceMechanism {
    pub fn new() -> Self {
        Self
    }

    /// Clean parse into H4 state + result (the restart model's control
    /// operation: generation 0, checkpoints at every top-level block).
    fn parse_into_pending<W: WorkSink>(
        &self,
        src: &[u8],
        cx: &mut MechanismContext<'_, W>,
    ) -> H4Pending {
        let rp = sg::parse_region(src, 0, src.len(), cx.sink);
        let mut built = 0u64;
        let slots = fresh_slots(&rp.blocks, src, &mut built);
        let checkpoints = registers(&slots, 0);
        cx.sink.add_blocks_reparsed(built);
        cx.sink.add_nodes_rebuilt(built);
        cx.sink.add_nodes_reused(0);
        cx.sink
            .add_metadata_records_touched(checkpoints.len() as u64);
        let defs = collect_defs(&slots);
        let table = ref_table(&defs);
        let result = project(&slots, src, &table);
        declare_gauges_not_applicable(cx);
        H4Pending {
            state: H4State {
                blocks: slots,
                checkpoints,
                gen: 0,
                defs,
                src_len: src.len(),
            },
            result,
        }
    }
}

impl Mechanism for RestartConvergenceMechanism {
    type State = H4State;
    type Prepared = H4Prepared;
    type Pending = H4Pending;

    fn id(&self) -> MechanismId {
        MechanismId(H4_MECHANISM_ID.to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        Ok(self.parse_into_pending(source.as_bytes(), cx))
    }

    fn prepare_update<W: WorkSink>(
        &self,
        old_source: &Source,
        _post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        let old = old_source.as_bytes();
        let es = edit.start_byte() as usize;
        let ee = edit.end_byte() as usize;
        let delta = edit.inserted_text_len_bytes() as isize - (ee - es) as isize;
        let ee_new = es + edit.inserted_text_len_bytes() as usize;

        // RESTART SELECTION: the last checkpoint at/before the damage.
        // Its existence proves an old block boundary there, and the bytes
        // before the edit are unchanged — the prefix parse carries over
        // untouched (restart-boundary soundness, R5 freeze §9 notes).
        let scanned_checkpoints = old_state.checkpoints.len() as u64;
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
        let mut backed = 0u64;
        if let Some(s) = restart_slot {
            let mut s = s;
            while s > 0 {
                backed += 1;
                let prev = &old_state.blocks[s - 1];
                let boundary = old_state.checkpoints[s].position;
                let sep_lfs = old[prev.abs_end()..boundary]
                    .iter()
                    .filter(|&&b| b == b'\n')
                    .count();
                if sep_lfs >= 2 {
                    break; // blank line terminates every continuation
                }
                let k = &old_state.blocks[s];
                let k_start = k.abs_start();
                let k_line_end = old[k_start..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(old.len(), |p| k_start + p);
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
        let mut scanned_entries = 0u64;
        let mut damaged_entries = 0u64;
        let mut damaged_end = 0usize;
        let mut damaged_has_def = false;
        for slot in &old_state.blocks {
            scanned_entries += 1;
            let (s, e) = (slot.abs_start(), slot.abs_end());
            if s < ee && e > es {
                damaged_entries += 1;
                damaged_end = damaged_end.max(e);
                damaged_has_def |= skel_has_def(&slot.block);
            }
        }
        cx.sink
            .add_metadata_records_touched(scanned_checkpoints + scanned_entries + backed);
        Ok(H4Prepared {
            edit_start: es,
            edit_end_new: ee_new,
            delta,
            restart_position,
            restart_slot,
            damaged_end,
            damaged_entries,
            damaged_has_def,
            scanned_checkpoints,
        })
    }

    fn update<W: WorkSink>(
        &self,
        _old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let post = post_source.as_bytes();
        let es = prepared.edit_start;
        let ee_new = prepared.edit_end_new;
        let delta = prepared.delta;
        debug_assert_eq!(ee_new, es + edit.inserted_text_len_bytes() as usize);
        cx.sink
            .set_slot_not_applicable(NotApplicableSlot::FallbackToFullCount);

        // The reparsed region may itself create a definition: any `]: `
        // occurrence inside the edited span's post bytes (source-derived,
        // pre-computable).
        let definition_changing = prepared.damaged_has_def || {
            cx.sink.record_source_inspection(es as u64, ee_new as u64);
            post[es..ee_new].windows(3).any(|w| w == b"]: ")
        };

        if definition_changing {
            // RESTART AT ZERO: reference resolution is document-global.
            // Parse everything fresh (NOT a fallback — a planned restart
            // through the same forward machinery), re-register every
            // checkpoint at the next generation.
            let rp = sg::parse_region(post, 0, post.len(), cx.sink);
            let mut built = 0u64;
            let slots = fresh_slots(&rp.blocks, post, &mut built);
            let gen = old_state.gen + 1;
            let checkpoints = registers(&slots, gen);
            cx.sink.add_blocks_reparsed(built);
            cx.sink.add_nodes_rebuilt(built);
            cx.sink.add_nodes_reused(0);
            cx.sink
                .add_metadata_records_touched(checkpoints.len() as u64);
            cx.sink.set_restart_distance(Observed::Known(es as u64));
            cx.sink
                .set_convergence_distance(Observed::Known(post.len() as u64));
            let defs = collect_defs(&slots);
            let table = ref_table(&defs);
            let result = project(&slots, post, &table);
            return Ok(H4Pending {
                state: H4State {
                    blocks: slots,
                    checkpoints,
                    gen,
                    defs,
                    src_len: post.len(),
                },
                result,
            });
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
            gen: old_state.gen,
            take: None,
            consultations: 0,
            blank_checks: Vec::new(),
        };
        let mut hook: Box<SpliceHook<'_>> = Box::new(|pos, key| cursor.consult(pos, key));
        let (rp, slot_count) = parse_region_with_hook(post, r, post.len(), cx.sink, &mut hook);
        let (take, consultations, blank_checks) = {
            drop(hook); // end the cursor borrow before reading the take record
            (cursor.take, cursor.consultations, cursor.blank_checks)
        };
        for (a, b) in &blank_checks {
            cx.sink.record_source_inspection(*a, *b);
        }

        // Assemble: retained prefix + fresh region blocks + (at the splice)
        // the retained suffix with retargeted base offsets. Retained
        // blocks carry their old checkpoint records (key + generation
        // provenance); fresh blocks register at the current generation.
        let gen = old_state.gen;
        let mut pairs: Vec<(BlockSlot, Option<Checkpoint>)> = Vec::new();
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
                    gen: cp.gen,
                }),
            ));
        }
        let mut built = 0u64;
        let mut reused = 0u64;
        let mut rebased = 0u64;
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
                        reused += skel_count(&s.block);
                        rebased += 1;
                        pairs.push((
                            BlockSlot {
                                base_shift: s.base_shift + delta,
                                line_offset: s.line_offset,
                                block: Arc::clone(&s.block),
                            },
                            Some(Checkpoint {
                                position: 0,
                                key: cp.key.clone(),
                                gen: cp.gen,
                            }),
                        ));
                    }
                }
                other => {
                    built += skel_count(other);
                    let start = other.start();
                    pairs.push((
                        BlockSlot {
                            base_shift: 0,
                            line_offset: start - line_start_of(post, start),
                            block: Arc::new(other.clone()),
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
                    key: slot.block.ctx().clone(),
                    gen,
                },
            });
            slots.push(slot);
        }

        // Counters: both gauges are measured on EVERY update (Known(0) is
        // a measured zero — e.g. an edit at the document start restarts at
        // position 0 with no distance).
        let convergence_pos = take.map_or(post.len(), |(_, p)| p);
        cx.sink.add_blocks_reparsed(built);
        cx.sink.add_nodes_rebuilt(built);
        cx.sink.add_nodes_reused(reused);
        cx.sink.add_metadata_records_touched(
            consultations + slot_count as u64 + checkpoints.len() as u64 + rebased,
        );
        cx.sink
            .set_restart_distance(Observed::Known((es - r) as u64));
        cx.sink
            .set_convergence_distance(Observed::Known((convergence_pos - r) as u64));

        let defs = collect_defs(&slots);
        let table = ref_table(&defs);
        let result = project(&slots, post, &table);
        Ok(H4Pending {
            state: H4State {
                blocks: slots,
                checkpoints,
                gen,
                defs,
                src_len: post.len(),
            },
            result,
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        // Sealing only (eager completion boundary, R5 freeze §4).
        let checksum = normalized_checksum(&pending.result);
        Ok(Completed {
            state: pending.state,
            result_checksum: checksum,
        })
    }
}

fn declare_gauges_not_applicable<W: WorkSink>(cx: &mut MechanismContext<'_, W>) {
    cx.sink
        .set_slot_not_applicable(NotApplicableSlot::FallbackToFullCount);
    cx.sink.set_restart_distance(Observed::NotApplicable);
    cx.sink.set_convergence_distance(Observed::NotApplicable);
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
    gen: u64,
    /// The single convergence take: (old suffix start index, take start
    /// in post coordinates).
    take: Option<(usize, usize)>,
    consultations: u64,
    blank_checks: Vec<(u64, u64)>,
}

impl Cursor<'_> {
    /// The convergence consult at a live block-start line `pos` with live
    /// state `key`. Returns the take end (the document end — the old
    /// stable suffix runs to EOF) or `None` (keep parsing).
    fn consult(&mut self, pos: usize, key: &ContextKey) -> Option<usize> {
        self.consultations += 1;
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
        if cp.gen != self.gen {
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
        let prev_ls = line_start_of(self.post, pos - 1);
        if !sg::parser::all_spaces(self.post, prev_ls, pos - 1) {
            return None;
        }
        self.blank_checks.push((prev_ls as u64, (pos - 1) as u64));
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

/// Fresh slots from parsed blocks (absolute spans in `src`, base offset 0).
fn fresh_slots(blocks: &[Skel], src: &[u8], built: &mut u64) -> Vec<BlockSlot> {
    blocks
        .iter()
        .map(|sk| {
            *built += skel_count(sk);
            let start = sk.start();
            BlockSlot {
                base_shift: 0,
                line_offset: start - line_start_of(src, start),
                block: Arc::new(sk.clone()),
            }
        })
        .collect()
}

/// Register a checkpoint for every slot at generation `gen`.
fn registers(slots: &[BlockSlot], gen: u64) -> Vec<Checkpoint> {
    slots
        .iter()
        .map(|s| Checkpoint {
            position: s.line_position(),
            key: s.block.ctx().clone(),
            gen,
        })
        .collect()
}

/// Recursive node count of one retained subtree.
fn skel_count(sk: &Skel) -> u64 {
    let children: &[Skel] = match sk {
        Skel::Quote { children, .. }
        | Skel::List {
            items: children, ..
        } => children,
        Skel::Item { children, .. } => children,
        _ => &[],
    };
    1 + children.iter().map(skel_count).sum::<u64>()
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

/// Document-order first-wins definition table over the assembled slots.
fn collect_defs(slots: &[BlockSlot]) -> Vec<(String, String)> {
    let mut defs = Vec::new();
    for slot in slots {
        collect_defs_skel(&slot.block, &mut defs);
    }
    defs
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

/// Materialize the completed document: clone each slot's skeleton,
/// shifting spans whose base offset is nonzero, then run the shared
/// finish pass (inline scan against the CURRENT table).
fn project(slots: &[BlockSlot], src: &[u8], table: &sg::RefTable) -> NormalizedDocument {
    let blocks = slots
        .iter()
        .map(|s| {
            if s.base_shift == 0 {
                (*s.block).clone()
            } else {
                shift_skel(&s.block, s.base_shift)
            }
        })
        .collect();
    sg::inline::finish_document(src, blocks, table)
}

/// Deep clone of a skeleton with every absolute span shifted by `base`.
fn shift_skel(sk: &Skel, base: isize) -> Skel {
    let sh = |p: usize| (p as isize + base) as usize;
    match sk {
        Skel::Para {
            start,
            end,
            segments,
            ctx,
        } => Skel::Para {
            start: sh(*start),
            end: sh(*end),
            segments: segments.iter().map(|&(a, b)| (sh(a), sh(b))).collect(),
            ctx: ctx.clone(),
        },
        Skel::Heading {
            start,
            end,
            level,
            content,
            ctx,
        } => Skel::Heading {
            start: sh(*start),
            end: sh(*end),
            level: *level,
            content: (sh(content.0), sh(content.1)),
            ctx: ctx.clone(),
        },
        Skel::Quote {
            start,
            end,
            children,
            ctx,
        } => Skel::Quote {
            start: sh(*start),
            end: sh(*end),
            children: children.iter().map(|c| shift_skel(c, base)).collect(),
            ctx: ctx.clone(),
        },
        Skel::List {
            start,
            end,
            items,
            ctx,
        } => Skel::List {
            start: sh(*start),
            end: sh(*end),
            items: items.iter().map(|c| shift_skel(c, base)).collect(),
            ctx: ctx.clone(),
        },
        Skel::Item {
            start,
            end,
            marker,
            children,
            ctx,
        } => Skel::Item {
            start: sh(*start),
            end: sh(*end),
            marker: *marker,
            children: children.iter().map(|c| shift_skel(c, base)).collect(),
            ctx: ctx.clone(),
        },
        Skel::Fence {
            start,
            end,
            info,
            content,
            ctx,
        } => Skel::Fence {
            start: sh(*start),
            end: sh(*end),
            info: info.clone(),
            content: (sh(content.0), sh(content.1)),
            ctx: ctx.clone(),
        },
        Skel::Def {
            start,
            end,
            label,
            destination,
            ctx,
        } => Skel::Def {
            start: sh(*start),
            end: sh(*end),
            label: label.clone(),
            destination: destination.clone(),
            ctx: ctx.clone(),
        },
        Skel::Spliced { .. } => {
            unreachable!("splice placeholders are expanded before projection")
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
