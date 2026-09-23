//! Diagnostic copy of the frozen H4 RESTART_CONVERGENCE mechanism.
//!
//! # Why a copy exists at all
//!
//! Issue #50 requires H4 to be decomposed into phases, counted, ablated
//! and allocator-accounted, while the original A0 H4 algorithm must remain
//! available and unchanged. Both cannot be true of one function body, so
//! this module carries a **diagnostic copy** and the frozen crate
//! `markit-mdbench-restart-convergence` is left untouched.
//!
//! The copy is not *assumed* equivalent: `mdbench-h4diag
//! verify-equivalence` runs every #50 cell through both mechanisms in both
//! the `NoopWorkSink` and `CounterSink` lanes and requires identical
//! normalized-result checksums and identical frozen work counters. The
//! U_PLAIN lane does not use this copy at all — it runs the original
//! mechanism.
//!
//! # Instrumentation placement
//!
//! Every diagnostic statement is behind `#[cfg(feature = ...)]` (through
//! the [`crate::cnt!`], [`crate::cnt_set!`], [`crate::tracked_push!`] and
//! [`crate::phase!`] macros), so the default build contains none of it.
//!
//! # Deliberate, documented deviations from the frozen source text
//!
//! 1. `prepare_update`'s restart selection is written as an explicit
//!    reverse loop with the same predicate as `.rposition` (so the
//!    predicate evaluations are countable). Same result, same work.
//! 2. `update` retires the consumed old state through an explicit
//!    `retire_old_state` call placed after the assembly loops and before
//!    the pending value is constructed. In the frozen source the drop
//!    happens at function scope exit. Both destroy exactly the same
//!    objects in exactly the same order; nothing between the two points
//!    allocates. The move exists so P7 can be timed and so `Adrop` has an
//!    ownership-transfer point.
//! 3. The assembled-vs-retained definition-table comparison is written as
//!    an explicit element loop (so entry comparisons are countable); it is
//!    semantically `table.entries() != old_state.defs.as_slice()`.

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
use markit_mdbench_common::SourceVersion;
use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::{normalized_checksum, Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_shared_grammar as sg;
use sg::parser::{parse_region, parse_region_with_hook, ContextKey, Skel, SpliceHook};

use crate::deferred;
use crate::{cnt, cnt_set, phase, tracked_push};

/// Mechanism identifier of the diagnostic copy (never the frozen id).
pub const H4DIAG_MECHANISM_ID: &str = "restart-convergence-h4-diag-copy";

/// Which diagnostic variant the copy runs.
///
/// `A0` is the faithful copy. The others change exactly one thing each
/// (Issue #50 §8) and must produce the same normalized result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Faithful copy of the frozen H4 algorithm. The control.
    A0,
    /// `Adefs` — skip the global definition traversal / table build /
    /// comparison only when already-maintained state proves the assembled
    /// definition environment is empty (Issue #50 §9).
    ADefs,
    /// `Adrop` — move the complete consumed old state to an explicit
    /// post-timer owner instead of destroying it inside the timed region.
    ADrop,
    /// `Acapacity` — give the `pairs` vector an exact capacity computed
    /// from lengths the algorithm already has, instead of growing from
    /// `Vec::new()`.
    ACapacity,
}

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::A0,
        Variant::ADefs,
        Variant::ADrop,
        Variant::ACapacity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Variant::A0 => "A0",
            Variant::ADefs => "Adefs",
            Variant::ADrop => "Adrop",
            Variant::ACapacity => "Acapacity",
        }
    }
}

// ---------------------------------------------------------------------------
// Retained state (identical shape to the frozen H4State)
// ---------------------------------------------------------------------------

/// One retained top-level block: skeleton + materialized semantic subtree.
#[derive(Debug, Clone)]
pub struct DiagRetainedBlock {
    pub skel: Skel,
    pub sem: Node,
}

/// One retained top-level slot: shared block plus base offset.
#[derive(Debug, Clone)]
pub struct DiagBlockSlot {
    pub base_shift: isize,
    pub line_offset: usize,
    pub block: Arc<DiagRetainedBlock>,
}

impl DiagBlockSlot {
    pub fn abs_start(&self) -> usize {
        (self.block.skel.start() as isize + self.base_shift) as usize
    }

    pub fn abs_end(&self) -> usize {
        (self.block.skel.end() as isize + self.base_shift) as usize
    }

    pub fn line_position(&self) -> usize {
        self.abs_start() - self.line_offset
    }
}

/// One retained checkpoint record.
#[derive(Debug, Clone)]
pub struct DiagCheckpoint {
    pub position: usize,
    pub key: ContextKey,
    pub gen: u64,
}

/// Retained diagnostic state: same fields, same invariants as `H4State`.
#[derive(Debug, Clone, Default)]
pub struct H4DiagState {
    pub(crate) blocks: Vec<DiagBlockSlot>,
    pub(crate) checkpoints: Vec<DiagCheckpoint>,
    pub(crate) gen: u64,
    pub(crate) defs: Vec<(String, String)>,
    pub(crate) src_len: usize,
}

impl H4DiagState {
    pub fn blocks(&self) -> &[DiagBlockSlot] {
        &self.blocks
    }

    pub fn checkpoints(&self) -> &[DiagCheckpoint] {
        &self.checkpoints
    }

    pub fn generation(&self) -> u64 {
        self.gen
    }

    pub fn defs(&self) -> &[(String, String)] {
        &self.defs
    }

    pub fn src_len(&self) -> usize {
        self.src_len
    }

    pub fn blocks_len(&self) -> usize {
        self.blocks.len()
    }

    pub fn checkpoints_len(&self) -> usize {
        self.checkpoints.len()
    }
}

impl NormalizeV1 for H4DiagState {
    fn normalize_v1(&self) -> NormalizedDocument {
        project(&self.blocks, self.src_len)
    }
}

impl markit_mdbench_common::ResultChecksum for H4DiagState {
    fn result_checksum(&self) -> u64 {
        normalized_checksum(&self.normalize_v1())
    }
}

/// Pending work handed to `complete()`.
pub struct H4DiagPending {
    state: H4DiagState,
}

/// `prepare_update` product: same facts as the frozen `H4Prepared`.
#[derive(Debug, Clone)]
pub struct H4DiagPrepared {
    pub edit_start: usize,
    pub edit_end_new: usize,
    pub delta: isize,
    pub restart_position: usize,
    pub restart_slot: usize,
    pub damaged_end: usize,
    pub damaged_entries: u64,
    pub damaged_has_def: bool,
    pub scanned_checkpoints: u64,
}

// ---------------------------------------------------------------------------
// Mechanism
// ---------------------------------------------------------------------------

/// The diagnostic H4 copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct H4Diag {
    variant: Variant,
}

impl Default for Variant {
    fn default() -> Self {
        Variant::A0
    }
}

impl H4Diag {
    pub fn new(variant: Variant) -> Self {
        Self { variant }
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    fn parse_into_pending<W: WorkSink>(
        &self,
        src: &[u8],
        cx: &mut MechanismContext<'_, W>,
    ) -> H4DiagPending {
        let rp = parse_region(src, 0, src.len(), cx.sink);
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
        H4DiagPending {
            state: H4DiagState {
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
/// damage, copied verbatim in behaviour.
fn restart_at_zero<W: WorkSink>(
    post: &[u8],
    old_gen: u64,
    es: usize,
    cx: &mut MechanismContext<'_, W>,
) -> H4DiagPending {
    let rp = parse_region(post, 0, post.len(), cx.sink);
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
    H4DiagPending {
        state: H4DiagState {
            blocks: slots,
            checkpoints,
            gen,
            defs,
            src_len: post.len(),
        },
    }
}

/// Retire the consumed old state.
///
/// `A0` / `ADefs` / `ACapacity`: destroy it here (the frozen source
/// destroys it at function scope exit; see the module header).
///
/// `ADrop`: move it, intact, to the post-timer owner.
///
/// Under the `counters` feature the destruction is performed
/// element-by-element so the actual handle releases and payload
/// destructions are counted. That is strictly more work than A0 runs, and
/// the counter lane is never timed.
fn retire_old_state(
    state: H4DiagState,
    variant: Variant,
    handles_kept_shared: u64,
) {
    if variant == Variant::ADrop {
        deferred::set_inventory(deferred::RetirementInventory {
            old_block_slots: state.blocks.len() as u64,
            old_checkpoint_records: state.checkpoints.len() as u64,
            handles_kept_shared,
            handles_last_reference: (state.blocks.len() as u64).saturating_sub(handles_kept_shared),
        });
        deferred::park(state);
        return;
    }

    #[cfg(feature = "counters")]
    {
        let H4DiagState {
            blocks,
            checkpoints,
            defs,
            ..
        } = state;
        for slot in blocks {
            cnt!(ArcHandlesReleased, 1);
            if Arc::strong_count(&slot.block) == 1 {
                cnt!(FinalPayloadDestructions, 1);
            }
            drop(slot.block);
            cnt!(OldBlockSlotsRetired, 1);
        }
        cnt!(OldCheckpointRecordsRetired, checkpoints.len());
        drop(checkpoints);
        drop(defs);
    }
    #[cfg(not(feature = "counters"))]
    {
        let _ = handles_kept_shared;
        drop(state);
    }
}

impl Mechanism for H4Diag {
    type State = H4DiagState;
    type Prepared = H4DiagPrepared;
    type Pending = H4DiagPending;

    fn id(&self) -> MechanismId {
        MechanismId(H4DIAG_MECHANISM_ID.to_string())
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
        phase!(P1PrepareDamageRestart, {
            let old = old_source.as_bytes();
            let es = edit.start_byte() as usize;
            let ee = edit.end_byte() as usize;
            let delta = edit.inserted_text_len_bytes() as isize - (ee - es) as isize;
            let ee_new = es + edit.inserted_text_len_bytes() as usize;

            // RESTART SELECTION — the frozen `.rposition(|cp| cp.position <= es)`
            // written as an explicit reverse loop so predicate evaluations are
            // countable. Identical predicate, identical result.
            let scanned_checkpoints = old_state.checkpoints.len() as u64;
            let mut restart_slot: Option<usize> = None;
            let mut predicate_evaluations = 0u64;
            for i in (0..old_state.checkpoints.len()).rev() {
                predicate_evaluations += 1;
                if old_state.checkpoints[i].position <= es {
                    restart_slot = Some(i);
                    break;
                }
            }
            cnt!(RestartPredicateEvaluations, predicate_evaluations);
            let _ = predicate_evaluations;

            // RESTART-BOUNDARY CONTINUATION MARGIN (unchanged behaviour).
            let mut backed = 0u64;
            if let Some(s) = restart_slot {
                let mut s = s;
                while s > 0 {
                    backed += 1;
                    let prev = &old_state.blocks[s - 1];
                    let boundary = old_state.checkpoints[s].position;
                    let (sep_lo, sep_hi) = (prev.abs_end(), boundary);
                    cx.sink.record_source_inspection(
                        SourceVersion::Old,
                        sep_lo as u64,
                        sep_hi as u64,
                    );
                    let sep_lfs = old[sep_lo..sep_hi].iter().filter(|&&b| b == b'\n').count();
                    if sep_lfs >= 2 {
                        break;
                    }
                    let k = &old_state.blocks[s];
                    let k_start = k.abs_start();
                    let k_line_end = sg::parser::memchr_lf_reported_in(
                        SourceVersion::Old,
                        old,
                        k_start,
                        cx.sink,
                    );
                    if es >= k_line_end {
                        break;
                    }
                    s -= 1;
                }
                restart_slot = Some(s);
            }
            cnt!(RestartMarginSteps, backed);

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

            // DAMAGE SCAN (unchanged behaviour). `scanned_entries` counts
            // EVERY old block examined, exactly like the frozen source; the
            // frozen `metadata_records_touched` charge uses that count, not
            // the number of intersecting entries.
            let mut scanned_entries = 0u64;
            let mut damaged_entries = 0u64;
            let mut damaged_end = 0usize;
            let mut damaged_has_def = false;
            for slot in &old_state.blocks {
                scanned_entries += 1;
                cnt!(DamageRecordsVisited, 1);
                let (s, e) = (slot.abs_start(), slot.abs_end());
                if s < ee && e > es {
                    damaged_entries += 1;
                    damaged_end = damaged_end.max(e);
                    damaged_has_def |= skel_has_def(&slot.block.skel);
                }
            }
            cnt!(DamagedEntries, damaged_entries);
            cx.sink
                .add_metadata_records_touched(scanned_checkpoints + scanned_entries + backed);
            Ok(H4DiagPrepared {
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

        // The reparsed region may itself create a definition.
        let definition_changing = prepared.damaged_has_def || {
            cx.sink.record_source_inspection(
                SourceVersion::Post,
                es as u64,
                ee_new as u64,
            );
            post[es..ee_new].windows(3).any(|w| w == b"]: ")
        };

        if definition_changing {
            // A restart is not the measured regime for the #50 workload;
            // the caller records `RESTART_AT_ZERO` so the observation is
            // never read as a normal-path sample.
            let pending = restart_at_zero(post, old_state.gen, es, cx);
            drop(old_state);
            return Ok(pending);
        }

        // ---- P2: forward parse from the restart checkpoint -------------
        let r = prepared.restart_position;
        let (rp, slot_count, take, consultations, _blank_checks) =
            phase!(P2ForwardParseAndConvergence, {
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
                let (rp, slot_count) =
                    parse_region_with_hook(post, r, post.len(), cx.sink, &mut hook);
                let (take, consultations, blank_checks) = {
                    drop(hook);
                    (cursor.take, cursor.consultations, cursor.blank_checks)
                };
                for (a, b) in &blank_checks {
                    cx.sink.record_source_inspection(
                        SourceVersion::Post,
                        *a,
                        *b,
                    );
                }
                (rp, slot_count, take, consultations, blank_checks)
            });
        cnt!(ForwardBlocksEmitted, rp.blocks.len());

        let gen = old_state.gen;

        // ---- P3: retained-prefix pair assembly -------------------------
        let mut prefix_reused = 0u64;
        let mut prefix_handles = 0u64;
        let mut pairs: Vec<(DiagBlockSlot, Option<DiagCheckpoint>)> = if self.variant
            == Variant::ACapacity
        {
            let suffix_len = take.map_or(0usize, |(old_idx, _)| {
                old_state.blocks.len() - old_idx
            });
            let fresh_len = rp
                .blocks
                .iter()
                .filter(|sk| !matches!(sk, Skel::Spliced { .. }))
                .count();
            Vec::with_capacity(prepared.restart_slot + fresh_len + suffix_len)
        } else {
            Vec::new()
        };
        phase!(P3PrefixPairAssembly, {
            for (s, cp) in old_state.blocks[..prepared.restart_slot]
                .iter()
                .zip(&old_state.checkpoints[..prepared.restart_slot])
            {
                cnt!(PrefixSlotsVisited, 1);
                prefix_reused += retained_block_nodes(&s.block);
                cnt!(ArcHandlesCloned, 1);
                prefix_handles += 1;
                cnt!(CheckpointKeyClones, 1);
                tracked_push!(
                    pairs,
                    (
                        DiagBlockSlot {
                            base_shift: s.base_shift,
                            line_offset: s.line_offset,
                            block: Arc::clone(&s.block),
                        },
                        Some(DiagCheckpoint {
                            position: 0,
                            key: cp.key.clone(),
                            gen: cp.gen,
                        }),
                    ),
                    PairsVecPushes,
                    PairsVecReallocations
                );
            }
        });

        // ---- P4: definition collection, table build, comparison --------
        //
        // `Adefs` guard (Issue #50 §9): the traversal is skipped only when
        // already-maintained state plus a freshly executed region check
        // prove the assembled environment is empty:
        //   (i)   the retained definition table is empty — every old block
        //         (hence every retained prefix and suffix block) has no
        //         definition;
        //   (ii)  no damaged old entry carries a definition;
        //   (iii) the edited span's post bytes contain no `]: ` marker;
        //   (iv)  no freshly parsed (non-spliced) block carries a
        //         definition — checked here, and timed.
        // Under (i)-(iv) the assembled table is empty, equals the retained
        // empty table, and the frozen response (no restart) is unchanged.
        let mut defs: Vec<(String, String)> = Vec::new();
        let table_matches_retained = phase!(P4DefinitionCollectTableCompare, {
            let skip_defs = self.variant == Variant::ADefs
                && old_state.defs.is_empty()
                && !prepared.damaged_has_def
                && rp.blocks.iter().all(|sk| match sk {
                    Skel::Spliced { .. } => true,
                    other => {
                        cnt!(DefinitionNodesVisited, 1);
                        !skel_has_def(other)
                    }
                });

            if skip_defs {
                // Assembled environment proven empty: no traversal, no table
                // build. The empty table is the correct table.
                true
            } else {
                for s in &old_state.blocks[..prepared.restart_slot] {
                    collect_defs_skel(&s.block.skel, &mut defs);
                }
                for sk in &rp.blocks {
                    if matches!(sk, Skel::Spliced { .. }) {
                        let (old_idx, _) =
                            take.expect("splice placeholder without a recorded take");
                        for s in &old_state.blocks[old_idx..] {
                            collect_defs_skel(&s.block.skel, &mut defs);
                        }
                    } else {
                        collect_defs_skel(sk, &mut defs);
                    }
                }
                let table = ref_table(&defs);
                cnt!(DefinitionTableEntries, defs.len());
                cnt_set!(DefsVecFinalCapacity, defs.capacity());
                cnt!(DefinitionTableComparePerformed, 1);
                !table_differs(table.entries(), old_state.defs.as_slice())
            }
        });
        let table = ref_table(&defs);

        // SOUND DEFINITION-ENVIRONMENT CHECK (unchanged behaviour).
        if !table_matches_retained {
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
            let pending = restart_at_zero(post, old_state.gen, es, cx);
            drop(old_state);
            return Ok(pending);
        }

        // ---- P5: fresh materialization + suffix assembly ---------------
        let mut built = Built::default();
        let mut reused = 0u64;
        let mut suffix_handles = 0u64;
        phase!(P5FreshMaterializationAndSuffixAssembly, {
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
                            cnt!(SuffixSlotsVisited, 1);
                            reused += retained_block_nodes(&s.block);
                            cnt!(ArcHandlesCloned, 1);
                            suffix_handles += 1;
                            cnt!(CheckpointKeyClones, 1);
                            tracked_push!(
                                pairs,
                                (
                                    DiagBlockSlot {
                                        base_shift: s.base_shift + delta,
                                        line_offset: s.line_offset,
                                        block: Arc::clone(&s.block),
                                    },
                                    Some(DiagCheckpoint {
                                        position: 0,
                                        key: cp.key.clone(),
                                        gen: cp.gen,
                                    }),
                                ),
                                PairsVecPushes,
                                PairsVecReallocations
                            );
                        }
                    }
                    other => {
                        cnt!(FreshBlocksMaterialized, 1);
                        built.fnodes += skel_count(other);
                        let start = other.start();
                        let sem = sg::inline::materialize_one_with_sink(
                            post,
                            other.clone(),
                            &table,
                            cx.sink,
                        );
                        let inline_nodes = inline_forest_count(std::slice::from_ref(&sem));
                        cnt!(InlineNodesMaterialized, inline_nodes);
                        built.nodes += skel_count(other) + inline_nodes;
                        cnt!(ArcHandlesCloned, 1);
                        tracked_push!(
                            pairs,
                            (
                                DiagBlockSlot {
                                    base_shift: 0,
                                    line_offset: start
                                        - sg::parser::line_start_of_reported(post, start, cx.sink),
                                    block: Arc::new(DiagRetainedBlock {
                                        skel: other.clone(),
                                        sem,
                                    }),
                                },
                                None,
                            ),
                            PairsVecPushes,
                            PairsVecReallocations
                        );
                    }
                }
            }
        });
        cnt_set!(PairsVecFinalLen, pairs.len());
        cnt_set!(PairsVecFinalCapacity, pairs.capacity());

        // ---- P6: pairs -> slots / checkpoints --------------------------
        let mut slots: Vec<DiagBlockSlot>;
        let mut checkpoints: Vec<DiagCheckpoint>;
        phase!(P6PairsToSlotsCheckpoints, {
            slots = Vec::with_capacity(pairs.len());
            checkpoints = Vec::with_capacity(pairs.len());
            for (slot, cp) in pairs {
                cnt!(LinePositionCalls, 1);
                let position = slot.line_position();
                checkpoints.push(match cp {
                    Some(mut c) => {
                        c.position = position;
                        c
                    }
                    None => {
                        cnt!(CheckpointKeyClones, 1);
                        DiagCheckpoint {
                            position,
                            key: slot.block.skel.ctx().clone(),
                            gen,
                        }
                    }
                });
                slots.push(slot);
            }
        });
        cnt!(SlotsCreated, slots.len());
        cnt!(CheckpointRecordsCreated, checkpoints.len());
        cnt_set!(SlotsVecFinalCapacity, slots.capacity());
        cnt_set!(CheckpointsVecFinalCapacity, checkpoints.capacity());

        // Counters: both gauges are measured on EVERY update.
        let convergence_pos = take.map_or(post.len(), |(_, p)| p);
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        cx.sink.add_nodes_reused(prefix_reused + reused);
        cx.sink.add_metadata_records_touched(
            consultations + slot_count as u64 + checkpoints.len() as u64 + suffix_handles,
        );
        cx.sink
            .set_restart_distance(Observed::Known((es - r) as u64));
        cx.sink
            .set_convergence_distance(Observed::Known((convergence_pos - r) as u64));

        // ---- P7: sealing + explicit retirement of the consumed state ---
        let pending = phase!(P7SealAndRetirement, {
            retire_old_state(old_state, self.variant, prefix_handles + suffix_handles);
            H4DiagPending {
                state: H4DiagState {
                    blocks: slots,
                    checkpoints,
                    gen,
                    defs,
                    src_len: post.len(),
                },
            }
        });
        Ok(pending)
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        Ok(Completed {
            state: pending.state,
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
// Convergence cursor
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    checkpoints: &'a [DiagCheckpoint],
    post: &'a [u8],
    ee_new: usize,
    delta: isize,
    damaged_end: usize,
    gen: u64,
    take: Option<(usize, usize)>,
    consultations: u64,
    blank_checks: Vec<(u64, u64)>,
}

impl Cursor<'_> {
    fn consult(&mut self, pos: usize, key: &ContextKey) -> Option<usize> {
        self.consultations += 1;
        cnt!(ConvergenceHookCalls, 1);
        if self.take.is_some() {
            return None;
        }
        if pos <= self.ee_new || pos >= self.post.len() {
            return None;
        }
        let q = map_back(pos, self.delta);
        let Ok(idx) = self.binary_search(q) else {
            return None;
        };
        let cp = &self.checkpoints[idx];
        cnt!(ConvergenceKeyComparisons, 1);
        if cp.key != *key {
            return None;
        }
        cnt!(ConvergenceGenerationChecks, 1);
        if cp.gen != self.gen {
            return None;
        }
        cnt!(ConvergenceDamageChecks, 1);
        if q < self.damaged_end {
            return None;
        }
        let prev_ls = line_start_of(self.post, pos - 1);
        self.blank_checks
            .push((prev_ls.saturating_sub(1) as u64, (pos - 1) as u64));
        cnt!(ConvergenceBlankLineChecks, 1);
        if !sg::parser::all_spaces(self.post, prev_ls, pos - 1) {
            return None;
        }
        cnt!(AcceptedSuffixTakes, 1);
        self.take = Some((idx, pos));
        Some(self.post.len())
    }

    /// `binary_search_by(|cp| cp.position.cmp(&q))`, with the comparisons
    /// counted. Identical search order and result.
    fn binary_search(&mut self, q: usize) -> Result<usize, usize> {
        let mut size = self.checkpoints.len();
        let mut left = 0usize;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;
            cnt!(ConvergenceBinarySearchComparisons, 1);
            let cmp = self.checkpoints[mid].position.cmp(&q);
            left = if cmp == std::cmp::Ordering::Less {
                mid + 1
            } else {
                left
            };
            right = if cmp == std::cmp::Ordering::Greater {
                mid
            } else {
                right
            };
            if cmp == std::cmp::Ordering::Equal {
                return Ok(mid);
            }
            size = right - left;
        }
        Err(left)
    }
}

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

#[derive(Debug, Default)]
struct Built {
    fnodes: u64,
    nodes: u64,
}

fn fresh_slots<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Vec<DiagBlockSlot> {
    blocks
        .iter()
        .map(|sk| {
            let k = skel_count(sk);
            built.fnodes += k;
            let start = sk.start();
            let sem = sg::inline::materialize_one_with_sink(src, sk.clone(), table, sink);
            built.nodes += k + inline_forest_count(std::slice::from_ref(&sem));
            DiagBlockSlot {
                base_shift: 0,
                line_offset: start - sg::parser::line_start_of_reported(src, start, sink),
                block: Arc::new(DiagRetainedBlock {
                    skel: sk.clone(),
                    sem,
                }),
            }
        })
        .collect()
}

fn registers(slots: &[DiagBlockSlot], gen: u64) -> Vec<DiagCheckpoint> {
    slots
        .iter()
        .map(|s| DiagCheckpoint {
            position: s.line_position(),
            key: s.block.skel.ctx().clone(),
            gen,
        })
        .collect()
}

fn skel_count(sk: &Skel) -> u64 {
    let children: &[Skel] = match sk {
        Skel::Quote { children, .. } | Skel::List { items: children, .. } => children,
        Skel::Item { children, .. } => children,
        _ => &[],
    };
    1 + children.iter().map(skel_count).sum::<u64>()
}

fn skel_has_def(sk: &Skel) -> bool {
    match sk {
        Skel::Def { .. } => true,
        Skel::Quote { children, .. } | Skel::List { items: children, .. } => {
            children.iter().any(skel_has_def)
        }
        Skel::Item { children, .. } => children.iter().any(skel_has_def),
        _ => false,
    }
}

/// `collect_defs_skel` with a visit counter at every recursive node.
fn collect_defs_skel(sk: &Skel, defs: &mut Vec<(String, String)>) {
    cnt!(DefinitionNodesVisited, 1);
    match sk {
        Skel::Def {
            label, destination, ..
        } => {
            cnt!(DefinitionBlocksFound, 1);
            cnt!(DefsVecPushes, 1);
            if defs.len() == defs.capacity() {
                cnt!(DefsVecReallocations, 1);
            }
            defs.push((label.clone(), destination.clone()));
        }
        Skel::Quote { children, .. } | Skel::List { items: children, .. } => {
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

/// `a != b` for definition slices, with element comparisons counted.
/// Lengths are compared after the element loop, matching slice equality's
/// result exactly.
fn table_differs(a: &[(String, String)], b: &[(String, String)]) -> bool {
    let common = a.len().min(b.len());
    for i in 0..common {
        cnt!(DefinitionTableComparisons, 1);
        if a[i] != b[i] {
            return true;
        }
    }
    a.len() != b.len()
}

// ---------------------------------------------------------------------------
// Projection
// ---------------------------------------------------------------------------

fn project(slots: &[DiagBlockSlot], src_len: usize) -> NormalizedDocument {
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

fn retained_block_nodes(b: &DiagRetainedBlock) -> u64 {
    skel_count(&b.skel) + inline_forest_count(std::slice::from_ref(&b.sem))
}

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
