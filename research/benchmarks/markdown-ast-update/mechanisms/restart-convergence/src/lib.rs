//! markit-mdbench-restart-convergence — H4 RESTART_CONVERGENCE
//! (#22, stage R5).
//!
//! Mechanism identity (frozen in
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
use markit_mdbench_oracle::normalized::{normalized_checksum, Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_shared_grammar as sg;
use sg::parser::{parse_region_with_hook, ContextKey, Skel, SpliceHook};

/// Mechanism identifier.
pub const H4_MECHANISM_ID: &str = "restart-convergence-h4";

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
pub struct RetainedBlock {
    pub skel: Skel,
    pub sem: Node,
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
pub struct BlockSlot {
    pub base_shift: isize,
    pub line_offset: usize,
    pub block: Arc<RetainedBlock>,
}

impl BlockSlot {
    /// Absolute span start in the current document's coordinates.
    pub fn abs_start(&self) -> usize {
        (self.block.skel.start() as isize + self.base_shift) as usize
    }

    /// Absolute span end in the current document's coordinates.
    pub fn abs_end(&self) -> usize {
        (self.block.skel.end() as isize + self.base_shift) as usize
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

/// Completed-state normalization (R5-CORRECTIVE-1, MAJOR-3): the pure
/// projection over the retained semantic subtrees — no source, no sink,
/// no parser. Retained suffix reuse shares complete syntax through the
/// `Arc<RetainedBlock>` identity; the projection only applies each
/// slot's `base_shift`.
impl NormalizeV1 for H4State {
    fn normalize_v1(&self) -> NormalizedDocument {
        project(&self.blocks, self.src_len)
    }
}

/// Pending work handed to `complete()` — the complete new blocks,
/// checkpoints, and definition facts (eager completion boundary, R5
/// freeze §4).
///
/// MEASUREMENT-CORRECTIVE-1 §9: the normalized projection (`project`, a
/// pure retargeting of retained semantic subtrees) is experiment/oracle
/// EXPORT, not H4 mechanism state — it is derived at the runner's
/// post-timer export boundary via `NormalizeV1`, never inside the timed
/// update.
pub struct H4Pending {
    state: H4State,
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
        // Definition facts come from the block skeletons (document
        // order); the table must exist before the inline pass
        // materializes the retained semantic subtrees.
        let mut defs = Vec::new();
        for sk in &rp.blocks {
            collect_defs_skel(sk, &mut defs);
        }
        let table = ref_table(&defs);
        let mut built = Built::default();
        let slots = fresh_slots(&rp.blocks, src, &table, &mut built, cx.sink);
        let checkpoints = registers(&slots, 0);
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        cx.sink.add_nodes_reused(0);
        cx.sink
            .add_metadata_records_touched(checkpoints.len() as u64);
        declare_gauges_not_applicable(cx);
        H4Pending {
            state: H4State {
                blocks: slots,
                checkpoints,
                gen: 0,
                defs,
                src_len: src.len(),
            },
        }
    }
}

/// RESTART AT ZERO — the frozen H4 response to definition-changing
/// damage: parse everything fresh through the same forward machinery (not
/// a fallback — H4 has no degraded mode), re-register every checkpoint at
/// the next generation, and retain nothing from the old state.
///
/// Used by BOTH sound detection paths: the pre-parse source-local fast
/// path (`damaged_has_def` / `]: ` in the edited span) and the assembled
/// definition-environment comparison after the forward pass.
fn restart_at_zero<W: WorkSink>(
    post: &[u8],
    old_gen: u64,
    es: usize,
    cx: &mut MechanismContext<'_, W>,
) -> H4Pending {
    let rp = sg::parse_region(post, 0, post.len(), cx.sink);
    let mut defs = Vec::new();
    for sk in &rp.blocks {
        collect_defs_skel(sk, &mut defs);
    }
    let table = ref_table(&defs);
    let mut built = Built::default();
    let slots = fresh_slots(&rp.blocks, post, &table, &mut built, cx.sink);
    let gen = old_gen + 1;
    let checkpoints = registers(&slots, gen);
    cx.sink.add_blocks_reparsed(built.fnodes);
    cx.sink.add_nodes_rebuilt(built.nodes);
    cx.sink.add_nodes_reused(0);
    cx.sink
        .add_metadata_records_touched(checkpoints.len() as u64);
    cx.sink.set_restart_distance(Observed::Known(es as u64));
    cx.sink
        .set_convergence_distance(Observed::Known(post.len() as u64));
    H4Pending {
        state: H4State {
            blocks: slots,
            checkpoints,
            gen,
            defs,
            src_len: post.len(),
        },
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
                // Both margin reads below are prepare-phase mechanism
                // work on the old source: report the exact scanned
                // ranges (R5-CORRECTIVE-2, source-inspection closure).
                let (sep_lo, sep_hi) = (prev.abs_end(), boundary);
                cx.sink.record_source_inspection(
                    markit_mdbench_common::SourceVersion::Old,
                    sep_lo as u64,
                    sep_hi as u64,
                );
                let sep_lfs = old[sep_lo..sep_hi].iter().filter(|&&b| b == b'\n').count();
                if sep_lfs >= 2 {
                    break; // blank line terminates every continuation
                }
                let k = &old_state.blocks[s];
                let k_start = k.abs_start();
                let k_line_end = sg::parser::memchr_lf_reported_in(
                    markit_mdbench_common::SourceVersion::Old,
                    old,
                    k_start,
                    cx.sink,
                );
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
                damaged_has_def |= skel_has_def(&slot.block.skel);
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
            cx.sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Post,
                es as u64,
                ee_new as u64,
            );
            post[es..ee_new].windows(3).any(|w| w == b"]: ")
        };

        if definition_changing {
            return Ok(restart_at_zero(post, old_state.gen, es, cx));
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
            cx.sink
                .record_source_inspection(markit_mdbench_common::SourceVersion::Post, *a, *b);
        }

        // Assemble: retained prefix + fresh region blocks + (at the splice)
        // the retained suffix with retargeted base offsets. Retained
        // blocks carry their old checkpoint records (key + generation
        // provenance); fresh blocks register at the current generation.
        // MAJOR (R5-CORRECTIVE-1 §6): retained blocks share the whole
        // `Arc<RetainedBlock>` — skeleton AND materialized semantic
        // subtree — so neither the prefix nor the converged suffix
        // re-reads or re-scans a single byte of inline content.
        let gen = old_state.gen;
        let mut pairs: Vec<(BlockSlot, Option<Checkpoint>)> = Vec::new();
        // Retained-prefix reuse is REAL reuse and is counted (H4-A,
        // MEASUREMENT-CORRECTIVE-1 §16): the prefix moves in by shared
        // `Arc<RetainedBlock>` identity — zero parser source reads, zero
        // reconstruction — exactly like the converged suffix. The old
        // suffix-only accumulator silently omitted this reuse.
        let mut prefix_reused = 0u64;
        for (s, cp) in old_state.blocks[..prepared.restart_slot]
            .iter()
            .zip(&old_state.checkpoints[..prepared.restart_slot])
        {
            prefix_reused += retained_block_nodes(&s.block);
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
            // The discarded forward pass really did its work, and
            // cumulative attribution reports ACTUAL work including work
            // later discarded by a restart (MEASUREMENT-CORRECTIVE-1
            // §14/§16 H4-B). Reported here:
            // - metadata/consultation work (checkpoints consulted, live
            //   slots registered);
            // - the region's skeleton parse: every non-spliced block
            //   unit was actually reparsed and its skeleton actually
            //   constructed (skeleton-only — the discarded attempt never
            //   reached inline materialization);
            // - source inspection and the consult-time margin reads were
            //   already reported through the sink as the pass ran.
            // The delivered result is the restart's, and ITS counters
            // accumulate on top of this.
            let discarded_fnodes: u64 = rp
                .blocks
                .iter()
                .filter(|sk| !matches!(sk, Skel::Spliced { .. }))
                .map(skel_count)
                .sum();
            cx.sink
                .add_metadata_records_touched(consultations + slot_count as u64);
            cx.sink.add_blocks_reparsed(discarded_fnodes);
            cx.sink.add_nodes_rebuilt(discarded_fnodes);
            return Ok(restart_at_zero(post, old_state.gen, es, cx));
        }

        let mut built = Built::default();
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
                        reused += retained_block_nodes(&s.block);
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
                    built.fnodes += skel_count(other);
                    let start = other.start();
                    let sem =
                        sg::inline::materialize_one_with_sink(post, other.clone(), &table, cx.sink);
                    built.nodes +=
                        skel_count(other) + inline_forest_count(std::slice::from_ref(&sem));
                    pairs.push((
                        BlockSlot {
                            base_shift: 0,
                            line_offset: start
                                - sg::parser::line_start_of_reported(post, start, cx.sink),
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
                    gen,
                },
            });
            slots.push(slot);
        }

        // Counters: both gauges are measured on EVERY update (Known(0) is
        // a measured zero — e.g. an edit at the document start restarts at
        // position 0 with no distance).
        let convergence_pos = take.map_or(post.len(), |(_, p)| p);
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        cx.sink.add_nodes_reused(prefix_reused + reused);
        cx.sink.add_metadata_records_touched(
            consultations + slot_count as u64 + checkpoints.len() as u64 + rebased,
        );
        cx.sink
            .set_restart_distance(Observed::Known((es - r) as u64));
        cx.sink
            .set_convergence_distance(Observed::Known((convergence_pos - r) as u64));

        Ok(H4Pending {
            state: H4State {
                blocks: slots,
                checkpoints,
                gen,
                defs,
                src_len: post.len(),
            },
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        // Native-sealing ONLY (eager completion boundary, R5 freeze §4):
        // the projection + checksum are the runner's post-timer export
        // (MEASUREMENT-CORRECTIVE-1).
        Ok(Completed {
            state: pending.state,
        })
    }
}

/// Post-timer experiment export (MEASUREMENT-CORRECTIVE-1): the H4
/// checksum is the checksum of the pure `NormalizeV1` projection over
/// the retained blocks.
impl markit_mdbench_common::ResultChecksum for H4State {
    fn result_checksum(&self) -> u64 {
        normalized_checksum(&self.normalize_v1())
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
        // The read happens on every consult that reaches (e) — buffer the
        // inspected range (the backward scan [prev_ls-1, pos-1) covers
        // the blank check's span) BEFORE the check so failing checks are
        // reported too (R5-CORRECTIVE-2).
        let prev_ls = line_start_of(self.post, pos - 1);
        self.blank_checks
            .push((prev_ls.saturating_sub(1) as u64, (pos - 1) as u64));
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

/// Construction counters (R5-CORRECTIVE-1 §8): `fnodes` counts block
/// structure (skeleton) nodes of FRESH material — the `blocks_reparsed`
/// unit; `nodes` counts every constructed native node INCLUDING the
/// freshly materialized inline syntax — the `nodes_rebuilt` unit.
#[derive(Debug, Default)]
struct Built {
    fnodes: u64,
    nodes: u64,
}

/// Fresh slots from parsed blocks (absolute spans in `src`, base offset
/// 0). Each block's semantic subtree is materialized here with the
/// instrumented inline pass (R5-CORRECTIVE-1 §6/MAJOR-2); every scanned
/// segment is reported to `sink`.
fn fresh_slots<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Vec<BlockSlot> {
    blocks
        .iter()
        .map(|sk| {
            let k = skel_count(sk);
            built.fnodes += k;
            let start = sk.start();
            let sem = sg::inline::materialize_one_with_sink(src, sk.clone(), table, sink);
            built.nodes += k + inline_forest_count(std::slice::from_ref(&sem));
            BlockSlot {
                base_shift: 0,
                // The line-offset derivation is a mechanism source read:
                // reported (R5-CORRECTIVE-2).
                line_offset: start - sg::parser::line_start_of_reported(src, start, sink),
                block: Arc::new(RetainedBlock {
                    skel: sk.clone(),
                    sem,
                }),
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
            key: s.block.skel.ctx().clone(),
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

/// Native node count of one retained block under the corrective counting
/// rule (R5-CORRECTIVE-1 §8): every structural block/container node of
/// the skeleton PLUS every retained inline syntax node of the
/// materialized subtree (recursive).
fn retained_block_nodes(b: &RetainedBlock) -> u64 {
    skel_count(&b.skel) + inline_forest_count(std::slice::from_ref(&b.sem))
}

/// Recursive count of INLINE syntax nodes in a normalized forest. The
/// inline scanner produces only inline-kind nodes below the block node,
/// so this counts every retained inline syntax node exactly once (the
/// materialized subtree's block-kind nodes are the same structural units
/// the skeleton encodes and are counted once via `skel_count`).
fn inline_forest_count(nodes: &[Node]) -> u64 {
    nodes.iter().map(inline_node_count).sum()
}

fn inline_node_count(n: &Node) -> u64 {
    let here = matches!(
        n.kind,
        NodeKind::Text
            | NodeKind::Emphasis
            | NodeKind::CodeSpan
            | NodeKind::Link
            | NodeKind::ReferenceLink
    ) as u64;
    here + inline_forest_count(&n.children)
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
