//! W_COUNTERS — direct representation-operation counters (Issue #50 §7).
//!
//! These count **actual visits/events** at the code sites that perform
//! them. They deliberately do not substitute a final `len()`, a
//! `capacity`, or a metadata counter for the operation: `pairs_vec_pushes`
//! counts `Vec::push` calls, `pairs_vec_reallocations` counts pushes that
//! actually had to grow the buffer, `checkpoint_records_read` counts
//! records actually read by the assembly loop, and so on.
//!
//! Only compiled under the `counters` feature. The counters live in a
//! thread-local `RefCell`; the lane is not timed, so the borrow cost is
//! irrelevant, and no counting code exists at all in the timing binaries.
//!
//! # What the counters do NOT claim
//!
//! Per Issue #50 §6, W_COUNTERS explains **work growth**, not U_PLAIN
//! time. Two specific honesty notes are recorded with the results:
//!
//! - `arc_handles_released` / `final_payload_destructions` require
//!   retiring the consumed old state element-by-element instead of
//!   letting the vectors bulk-drop. That is *more* code than A0 runs, so
//!   it is a counter-lane-only construction; it never appears in a
//!   timing lane.
//! - `retained_block_nodes_*`-style derived quantities are not counted
//!   here at all. The frozen `WorkSink` already reports the frozen
//!   `nodes_reused` slot; under `NoopWorkSink` the compiler may eliminate
//!   the computation that feeds it (Issue #50 §6), so the counters lane
//!   reports the *traversal* (`prefix_slots_visited`, `suffix_slots_visited`)
//!   that the frozen sink's number is derived from, rather than
//!   re-deriving the frozen number.

#[cfg(feature = "counters")]
use std::cell::RefCell;

/// Every diagnostic operation counter, in frozen report order.
///
/// Field names are the CSV column names. `*_final_capacity` fields are
/// gauges (last value wins); every other field is a cumulative event
/// count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagCounters {
    // ---- P1 prepare: restart selection / margin / damage scan ----------
    /// Old checkpoints examined by the `.rposition` restart predicate.
    pub restart_predicate_evaluations: u64,
    /// Old top-level blocks examined by the damage scan.
    pub damage_records_visited: u64,
    /// Margin back-up loop iterations (checkpoints walked back).
    pub restart_margin_steps: u64,
    /// Old blocks actually intersecting the edit span.
    pub damaged_entries: u64,

    // ---- P2 forward parse + convergence -------------------------------
    /// Convergence-hook consults (one per live block start offered).
    pub convergence_hook_calls: u64,
    /// Comparisons performed by the checkpoint `binary_search_by`.
    pub convergence_binary_search_comparisons: u64,
    /// Consults that reached the `ContextKey` equality test.
    pub convergence_key_comparisons: u64,
    /// Consults that reached the generation test.
    pub convergence_generation_checks: u64,
    /// Consults that reached the damaged-end test.
    pub convergence_damage_checks: u64,
    /// Consults that reached the paragraph-margin blank check.
    pub convergence_blank_line_checks: u64,
    /// Accepted suffix takes (0 or 1 per update).
    pub accepted_suffix_takes: u64,
    /// Forward-parser blocks emitted for the reparsed region.
    pub forward_blocks_emitted: u64,

    // ---- P3 prefix assembly -------------------------------------------
    pub prefix_slots_visited: u64,

    // ---- P4 definition collection / table / compare -------------------
    /// `collect_defs_skel` recursive node visits (every block visited,
    /// not only definition blocks).
    pub definition_nodes_visited: u64,
    /// `ReferenceDefinition` skeletons encountered.
    pub definition_blocks_found: u64,
    /// Entries inserted into the assembled reference table.
    pub definition_table_entries: u64,
    /// Entry comparisons in the assembled-vs-retained table check.
    pub definition_table_comparisons: u64,
    /// Assembled-table vs retained-table comparison performed at all.
    pub definition_table_compare_performed: u64,

    // ---- P5 fresh materialization + suffix assembly -------------------
    /// Fresh (non-spliced) blocks materialized in the reparsed region.
    pub fresh_blocks_materialized: u64,
    /// Suffix slots re-attached from the retained state.
    pub suffix_slots_visited: u64,
    /// Inline nodes constructed by fresh materialization.
    pub inline_nodes_materialized: u64,

    // ---- P6 pairs -> slots / checkpoints ------------------------------
    /// Slots constructed into the new state vector.
    pub slots_created: u64,
    /// Checkpoint records constructed into the new state vector.
    pub checkpoint_records_created: u64,
    /// `ContextKey` clones performed while assembling checkpoints.
    pub checkpoint_key_clones: u64,
    /// `BlockSlot::line_position()` calls (base-offset arithmetic).
    pub line_position_calls: u64,

    // ---- retirement (P7) ----------------------------------------------
    /// `Arc<RetainedBlock>` handles cloned (prefix + suffix + fresh).
    pub arc_handles_cloned: u64,
    /// `Arc<RetainedBlock>` handles released when the old state retires.
    pub arc_handles_released: u64,
    /// Retained payloads actually destroyed (strong count reached zero).
    pub final_payload_destructions: u64,
    /// Old `BlockSlot` elements retired.
    pub old_block_slots_retired: u64,
    /// Old `Checkpoint` records retired.
    pub old_checkpoint_records_retired: u64,

    // ---- vector growth events -----------------------------------------
    pub pairs_vec_pushes: u64,
    pub pairs_vec_reallocations: u64,
    pub pairs_vec_final_len: u64,
    pub pairs_vec_final_capacity: u64,
    pub slots_vec_pushes: u64,
    pub slots_vec_reallocations: u64,
    pub slots_vec_final_capacity: u64,
    pub checkpoints_vec_pushes: u64,
    pub checkpoints_vec_reallocations: u64,
    pub checkpoints_vec_final_capacity: u64,

    // ---- definition-table vector growth -------------------------------
    pub defs_vec_pushes: u64,
    pub defs_vec_reallocations: u64,
    pub defs_vec_final_capacity: u64,
}

/// Column order of the `work-counters.csv` report, frozen so that the
/// header and every row are generated from one source of truth.
pub const COUNTER_COLUMNS: &[&str] = &[
    "restart_predicate_evaluations",
    "damage_records_visited",
    "restart_margin_steps",
    "damaged_entries",
    "convergence_hook_calls",
    "convergence_binary_search_comparisons",
    "convergence_key_comparisons",
    "convergence_generation_checks",
    "convergence_damage_checks",
    "convergence_blank_line_checks",
    "accepted_suffix_takes",
    "forward_blocks_emitted",
    "prefix_slots_visited",
    "definition_nodes_visited",
    "definition_blocks_found",
    "definition_table_entries",
    "definition_table_comparisons",
    "definition_table_compare_performed",
    "fresh_blocks_materialized",
    "suffix_slots_visited",
    "inline_nodes_materialized",
    "slots_created",
    "checkpoint_records_created",
    "checkpoint_key_clones",
    "line_position_calls",
    "arc_handles_cloned",
    "arc_handles_released",
    "final_payload_destructions",
    "old_block_slots_retired",
    "old_checkpoint_records_retired",
    "pairs_vec_pushes",
    "pairs_vec_reallocations",
    "pairs_vec_final_len",
    "pairs_vec_final_capacity",
    "slots_vec_pushes",
    "slots_vec_reallocations",
    "slots_vec_final_capacity",
    "checkpoints_vec_pushes",
    "checkpoints_vec_reallocations",
    "checkpoints_vec_final_capacity",
    "defs_vec_pushes",
    "defs_vec_reallocations",
    "defs_vec_final_capacity",
];

impl DiagCounters {
    /// One row in [`COUNTER_COLUMNS`] order.
    pub fn row(&self) -> Vec<u64> {
        COUNTER_COLUMNS.iter().map(|c| self.value(c)).collect()
    }

    /// Value of one named counter column.
    pub fn value(&self, column: &str) -> u64 {
        match column {
            "restart_predicate_evaluations" => self.restart_predicate_evaluations,
            "damage_records_visited" => self.damage_records_visited,
            "restart_margin_steps" => self.restart_margin_steps,
            "damaged_entries" => self.damaged_entries,
            "convergence_hook_calls" => self.convergence_hook_calls,
            "convergence_binary_search_comparisons" => {
                self.convergence_binary_search_comparisons
            }
            "convergence_key_comparisons" => self.convergence_key_comparisons,
            "convergence_generation_checks" => self.convergence_generation_checks,
            "convergence_damage_checks" => self.convergence_damage_checks,
            "convergence_blank_line_checks" => self.convergence_blank_line_checks,
            "accepted_suffix_takes" => self.accepted_suffix_takes,
            "forward_blocks_emitted" => self.forward_blocks_emitted,
            "prefix_slots_visited" => self.prefix_slots_visited,
            "definition_nodes_visited" => self.definition_nodes_visited,
            "definition_blocks_found" => self.definition_blocks_found,
            "definition_table_entries" => self.definition_table_entries,
            "definition_table_comparisons" => self.definition_table_comparisons,
            "definition_table_compare_performed" => self.definition_table_compare_performed,
            "fresh_blocks_materialized" => self.fresh_blocks_materialized,
            "suffix_slots_visited" => self.suffix_slots_visited,
            "inline_nodes_materialized" => self.inline_nodes_materialized,
            "slots_created" => self.slots_created,
            "checkpoint_records_created" => self.checkpoint_records_created,
            "checkpoint_key_clones" => self.checkpoint_key_clones,
            "line_position_calls" => self.line_position_calls,
            "arc_handles_cloned" => self.arc_handles_cloned,
            "arc_handles_released" => self.arc_handles_released,
            "final_payload_destructions" => self.final_payload_destructions,
            "old_block_slots_retired" => self.old_block_slots_retired,
            "old_checkpoint_records_retired" => self.old_checkpoint_records_retired,
            "pairs_vec_pushes" => self.pairs_vec_pushes,
            "pairs_vec_reallocations" => self.pairs_vec_reallocations,
            "pairs_vec_final_len" => self.pairs_vec_final_len,
            "pairs_vec_final_capacity" => self.pairs_vec_final_capacity,
            "slots_vec_pushes" => self.slots_vec_pushes,
            "slots_vec_reallocations" => self.slots_vec_reallocations,
            "slots_vec_final_capacity" => self.slots_vec_final_capacity,
            "checkpoints_vec_pushes" => self.checkpoints_vec_pushes,
            "checkpoints_vec_reallocations" => self.checkpoints_vec_reallocations,
            "checkpoints_vec_final_capacity" => self.checkpoints_vec_final_capacity,
            "defs_vec_pushes" => self.defs_vec_pushes,
            "defs_vec_reallocations" => self.defs_vec_reallocations,
            "defs_vec_final_capacity" => self.defs_vec_final_capacity,
            other => panic!("unknown diagnostic counter column {other:?}"),
        }
    }

    /// JSON object for the raw observation stream.
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for column in COUNTER_COLUMNS {
            map.insert(
                (*column).to_string(),
                serde_json::Value::from(self.value(column)),
            );
        }
        serde_json::Value::Object(map)
    }
}

#[cfg(feature = "counters")]
thread_local! {
    static ACTIVE: RefCell<DiagCounters> = RefCell::new(DiagCounters::default());
}

/// Reset the thread-local counters to zero (counter lane only).
#[cfg(feature = "counters")]
pub fn reset() {
    ACTIVE.with(|c| *c.borrow_mut() = DiagCounters::default());
}

/// Snapshot the thread-local counters (counter lane only).
#[cfg(feature = "counters")]
pub fn snapshot() -> DiagCounters {
    ACTIVE.with(|c| c.borrow().clone())
}

/// Increment one cumulative counter by `n`.
#[cfg(feature = "counters")]
#[inline]
pub fn bump(slot: Slot, n: u64) {
    ACTIVE.with(|c| {
        let mut c = c.borrow_mut();
        let target = slot.get_mut(&mut c);
        *target = target.saturating_add(n);
    });
}

/// Set one gauge counter (last value wins).
#[cfg(feature = "counters")]
#[inline]
pub fn set(slot: Slot, n: u64) {
    ACTIVE.with(|c| {
        let mut c = c.borrow_mut();
        let target = slot.get_mut(&mut c);
        *target = n;
    });
}

/// The counter slots addressable from the algorithm.
#[derive(Debug, Clone, Copy)]
pub enum Slot {
    RestartPredicateEvaluations,
    DamageRecordsVisited,
    RestartMarginSteps,
    DamagedEntries,
    ConvergenceHookCalls,
    ConvergenceBinarySearchComparisons,
    ConvergenceKeyComparisons,
    ConvergenceGenerationChecks,
    ConvergenceDamageChecks,
    ConvergenceBlankLineChecks,
    AcceptedSuffixTakes,
    ForwardBlocksEmitted,
    PrefixSlotsVisited,
    DefinitionNodesVisited,
    DefinitionBlocksFound,
    DefinitionTableEntries,
    DefinitionTableComparisons,
    DefinitionTableComparePerformed,
    FreshBlocksMaterialized,
    SuffixSlotsVisited,
    InlineNodesMaterialized,
    SlotsCreated,
    CheckpointRecordsCreated,
    CheckpointKeyClones,
    LinePositionCalls,
    ArcHandlesCloned,
    ArcHandlesReleased,
    FinalPayloadDestructions,
    OldBlockSlotsRetired,
    OldCheckpointRecordsRetired,
    PairsVecPushes,
    PairsVecReallocations,
    PairsVecFinalLen,
    PairsVecFinalCapacity,
    SlotsVecPushes,
    SlotsVecReallocations,
    SlotsVecFinalCapacity,
    CheckpointsVecPushes,
    CheckpointsVecReallocations,
    CheckpointsVecFinalCapacity,
    DefsVecPushes,
    DefsVecReallocations,
    DefsVecFinalCapacity,
}

#[cfg(feature = "counters")]
impl Slot {
    fn get_mut(self, c: &mut DiagCounters) -> &mut u64 {
        match self {
            Slot::RestartPredicateEvaluations => &mut c.restart_predicate_evaluations,
            Slot::DamageRecordsVisited => &mut c.damage_records_visited,
            Slot::RestartMarginSteps => &mut c.restart_margin_steps,
            Slot::DamagedEntries => &mut c.damaged_entries,
            Slot::ConvergenceHookCalls => &mut c.convergence_hook_calls,
            Slot::ConvergenceBinarySearchComparisons => {
                &mut c.convergence_binary_search_comparisons
            }
            Slot::ConvergenceKeyComparisons => &mut c.convergence_key_comparisons,
            Slot::ConvergenceGenerationChecks => &mut c.convergence_generation_checks,
            Slot::ConvergenceDamageChecks => &mut c.convergence_damage_checks,
            Slot::ConvergenceBlankLineChecks => &mut c.convergence_blank_line_checks,
            Slot::AcceptedSuffixTakes => &mut c.accepted_suffix_takes,
            Slot::ForwardBlocksEmitted => &mut c.forward_blocks_emitted,
            Slot::PrefixSlotsVisited => &mut c.prefix_slots_visited,
            Slot::DefinitionNodesVisited => &mut c.definition_nodes_visited,
            Slot::DefinitionBlocksFound => &mut c.definition_blocks_found,
            Slot::DefinitionTableEntries => &mut c.definition_table_entries,
            Slot::DefinitionTableComparisons => &mut c.definition_table_comparisons,
            Slot::DefinitionTableComparePerformed => &mut c.definition_table_compare_performed,
            Slot::FreshBlocksMaterialized => &mut c.fresh_blocks_materialized,
            Slot::SuffixSlotsVisited => &mut c.suffix_slots_visited,
            Slot::InlineNodesMaterialized => &mut c.inline_nodes_materialized,
            Slot::SlotsCreated => &mut c.slots_created,
            Slot::CheckpointRecordsCreated => &mut c.checkpoint_records_created,
            Slot::CheckpointKeyClones => &mut c.checkpoint_key_clones,
            Slot::LinePositionCalls => &mut c.line_position_calls,
            Slot::ArcHandlesCloned => &mut c.arc_handles_cloned,
            Slot::ArcHandlesReleased => &mut c.arc_handles_released,
            Slot::FinalPayloadDestructions => &mut c.final_payload_destructions,
            Slot::OldBlockSlotsRetired => &mut c.old_block_slots_retired,
            Slot::OldCheckpointRecordsRetired => &mut c.old_checkpoint_records_retired,
            Slot::PairsVecPushes => &mut c.pairs_vec_pushes,
            Slot::PairsVecReallocations => &mut c.pairs_vec_reallocations,
            Slot::PairsVecFinalLen => &mut c.pairs_vec_final_len,
            Slot::PairsVecFinalCapacity => &mut c.pairs_vec_final_capacity,
            Slot::SlotsVecPushes => &mut c.slots_vec_pushes,
            Slot::SlotsVecReallocations => &mut c.slots_vec_reallocations,
            Slot::SlotsVecFinalCapacity => &mut c.slots_vec_final_capacity,
            Slot::CheckpointsVecPushes => &mut c.checkpoints_vec_pushes,
            Slot::CheckpointsVecReallocations => &mut c.checkpoints_vec_reallocations,
            Slot::CheckpointsVecFinalCapacity => &mut c.checkpoints_vec_final_capacity,
            Slot::DefsVecPushes => &mut c.defs_vec_pushes,
            Slot::DefsVecReallocations => &mut c.defs_vec_reallocations,
            Slot::DefsVecFinalCapacity => &mut c.defs_vec_final_capacity,
        }
    }
}

/// Increment a diagnostic counter (compiles to nothing without the
/// `counters` feature).
#[macro_export]
macro_rules! cnt {
    ($slot:ident, $n:expr) => {{
        #[cfg(feature = "counters")]
        {
            $crate::counters::bump($crate::counters::Slot::$slot, $n as u64);
        }
    }};
}

/// Set a diagnostic gauge counter (compiles to nothing without the
/// `counters` feature).
#[macro_export]
macro_rules! cnt_set {
    ($slot:ident, $n:expr) => {{
        #[cfg(feature = "counters")]
        {
            $crate::counters::set($crate::counters::Slot::$slot, $n as u64);
        }
    }};
}

/// One `push` on a tracked vector: counts the push and, when the push had
/// to grow the buffer (`len == capacity` before the push), the
/// reallocation event.
#[macro_export]
macro_rules! tracked_push {
    ($vec:expr, $value:expr, $push:ident, $realloc:ident) => {{
        #[cfg(feature = "counters")]
        {
            if $vec.len() == $vec.capacity() {
                $crate::counters::bump($crate::counters::Slot::$realloc, 1);
            }
            $crate::counters::bump($crate::counters::Slot::$push, 1);
        }
        $vec.push($value);
    }};
}
