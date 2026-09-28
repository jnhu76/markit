//! The formal commit frontier (#59 §6.1, §9, §11; algorithm spec §10
//! steps 11–16, §11): [`UpdateStaging`] is the pre-frontier staging state
//! and [`PreparedCommit`] is the frontier-crossing state. The distinction
//! is the frozen ownership and failure boundary, not a rename:
//!
//! ```text
//! stage()  — fallible; the old READY state stays owned by the caller,
//!            coherent and borrowable; every fallible phase completes
//!            here, INCLUDING the bounded commit workspace's logical
//!            bound derivation and its fallible try_reserve_exact
//!            reservation — no operation that can return an ordinary
//!            pre-frontier error may require irrevocably consuming the
//!            old READY state first (#59 §9.1 resource table)
//! prepare() — consume the old READY document BY OWNERSHIP into the
//!             prepared state: a pure ownership move of already-complete
//!             parts (record, plan, reserved workspace). It is
//!             infallible BY TYPE — there is no fallible work left to
//!             do, so there is no branch that could return to the
//!             caller with the old state consumed
//! commit() — CROSS THE FRONTIER: infallible, no Result, no return to
//!            staging, no allocation; only structural ownership moves,
//!            split/join/relink on the pre-staged stacks, rotations,
//!            aggregate recomputation, fixed scalar counter updates, the
//!            single RefTable move, state installation and retirement
//! ```
//!
//! After `PreparedCommit` has been formed, the normal path performs no
//! parsing, no source inspection, no semantic lookup, no fact
//! recollection, no fallback selection, no validation that could return
//! to staging, no Vec/String growth, and no ordinary recoverable heap
//! allocation (#59 §9.1 resource table). Process-level failure (panic on
//! an invariant bug, OOM, stack overflow) is not an algorithmic fallback
//! and is never converted into any normal branch.
//!
//! `PreparedCommit` holds the old `ReadyDocument` itself by ownership (for
//! retirement and the single RefTable move, #59 §6.1); the plan carries
//! only the semantic route. The local plan records the reuse of the old
//! RefTable as intent — the table is never cloned into staging.

use crate::state::{InterpretationId, OwnerSeq, ReadyDocument};
use crate::structural::{ForbiddenKind, HorseAStructuralSink};
use crate::update::UpdateRecord;
use crate::workspace::CommitWorkspace;

/// Which route the prepared commit will cross the frontier with. Both
/// variants are complete before formation: the local plan carries the
/// eager fresh Owners, the full plan carries an already-READY state.
#[derive(Debug)]
pub(crate) enum CommitPlan {
    /// The frozen facts-preserved route (spec §8 10a): the old RefTable is
    /// REUSED (intent marker — never cloned), the fresh replacement Owners
    /// were materialized under it during staging. `fresh_height` is
    /// transient construction metadata carried out of the bulk-build seam
    /// — the staging workspace sizing consumed it; it is never re-read
    /// from the persistent root (which would be an unaccounted aggregate
    /// read).
    Local {
        fresh: crate::state::OwnerSeq,
        fresh_height: u32,
        post_id: markit_mdbench_common::SourceId,
        post_len: usize,
    },
    /// The frozen conservative route (spec §8 10b): a complete same-target
    /// READY candidate that owns its own new RefTable.
    Full(ReadyDocument),
}

/// The pre-frontier staging state: every fallible phase has completed,
/// both candidate plans are fully materialized, and the bounded explicit
/// commit workspace is already reserved — while the old READY state is
/// still owned by the caller and remains coherent and borrowable.
/// Staging holds no borrow of the old document and no duplicate of its
/// RefTable.
#[derive(Debug)]
pub(crate) struct UpdateStaging {
    pub(crate) record: UpdateRecord,
    pub(crate) plan: CommitPlan,
    /// The pre-staged bounded explicit commit workspace (#59 §9.1):
    /// logical limits derived and capacity fallibly reserved during
    /// staging, so formation below is a pure move.
    pub(crate) workspace: CommitWorkspace,
}

impl UpdateStaging {
    /// Form the [`PreparedCommit`]: consume the old READY document by
    /// ownership.
    ///
    /// Infallible BY TYPE and by construction: every fallible pre-frontier
    /// operation — all staging phases, the workspace logical-bound
    /// derivation, the scale diagnostics, and the workspace's fallible
    /// `try_reserve_exact` reservation — already completed inside
    /// `stage()`, while the caller still owned the old READY document.
    /// This function performs no work beyond moving already-complete
    /// parts; there is no branch that can fail, and therefore no path
    /// that could consume the old state and then return an error.
    pub(crate) fn prepare(self, old: ReadyDocument) -> PreparedCommit {
        PreparedCommit {
            old,
            record: self.record,
            plan: self.plan,
            workspace: self.workspace,
        }
    }
}

/// The frontier-crossing state (#59 §6.1): owns the old READY document by
/// ownership, the frozen geometry record, the complete route plan, and
/// the pre-staged bounded explicit commit workspace (#59 §9.1 — after
/// formation, the structural operators allocate nothing).
/// Every ordinary recoverable/fallible operation required for commit has
/// completed; only non-fallible structural ownership operations remain.
#[derive(Debug)]
pub(crate) struct PreparedCommit {
    old: ReadyDocument,
    record: UpdateRecord,
    plan: CommitPlan,
    workspace: CommitWorkspace,
}

impl PreparedCommit {
    /// CROSS THE COMMIT FRONTIER — consume the old representation and
    /// produce the next READY state.
    ///
    /// Infallible by construction: no `Result`, no fallible branch, no
    /// return to staging, and no allocation (the bounded explicit stacks
    /// were reserved before the frontier and a within-limit push never
    /// allocates). The only failure mode is a process-level panic on a
    /// violated internal invariant, which is an implementation bug, not
    /// a mechanism branch.
    pub(crate) fn commit(self, sink: &mut dyn HorseAStructuralSink) -> ReadyDocument {
        let PreparedCommit {
            old,
            record,
            plan,
            mut workspace,
        } = self;
        match plan {
            CommitPlan::Local {
                fresh,
                fresh_height: _,
                post_id,
                post_len,
            } => {
                debug_assert_eq!(record.path, crate::update::UpdatePath::Local);
                debug_assert!(record.facts_equal);
                debug_assert_eq!(record.intervals.old.start, record.restart.cut);
                debug_assert_eq!(record.intervals.new.start, record.restart.cut);
                debug_assert!(record.convergence.is_none_or(|accepted| {
                    accepted.old_cut == record.intervals.old.end
                        && accepted.new_cut == record.intervals.new.end
                }));

                let ranks = record.intervals.old_ranks;
                let ReadyDocument { owners, refs, .. } = old;
                let mut owners: OwnerSeq = owners;
                // The one structural splice: the retained prefix and suffix
                // are transferred by ownership and the detached middle
                // comes back intact rather than being dropped inside the
                // operator.
                let detached =
                    owners.replace_range(ranks.start, ranks.end, fresh, sink, &mut workspace);
                sink.owners_removed((ranks.end - ranks.start) as u64);

                // Defended-site sentinel assertions: the splice transferred
                // the retained prefix and suffix structurally — no
                // sequential enumeration of either side, no coordinate
                // rewrite of retained payload. A regression that enumerates
                // or rewrites would charge these same sites.
                sink.forbidden(ForbiddenKind::PrefixEnumeration, 0);
                sink.forbidden(ForbiddenKind::SuffixEnumeration, 0);
                sink.forbidden(ForbiddenKind::UnaffectedCoordinateWrites, 0);

                let next = ReadyDocument {
                    source_id: post_id,
                    source_len: post_len,
                    interpretation: InterpretationId::HORSE_A_V1,
                    owners,
                    refs,
                };

                // Retire the detached middle only — the retained prefix and
                // suffix payload is never traversed. The retirement is
                // explicit and attributable (module `retirement`, #59 §11):
                // it receives exactly the detached tree and nothing else,
                // so it structurally cannot touch P/S, and each destroyed
                // record/payload node is charged at its own recursion frame.
                crate::retirement::retire_detached(detached, sink);
                sink.forbidden(ForbiddenKind::UnaffectedOldRetirement, 0);

                // H_new is an O(1) root read on the installed state.
                let h_new = next.owners.height();
                if next.owners.root.is_some() {
                    sink.aggregate_reads(1);
                }
                sink.record_height_new(h_new as u64);

                // No retained-tree walk for attribution happens past this
                // point (or anywhere else on this path).
                sink.forbidden(ForbiddenKind::AttributionTreeWalk, 0);
                next
            }
            CommitPlan::Full(fresh) => {
                debug_assert_eq!(record.path, crate::update::UpdatePath::SameTargetFullBuild);
                debug_assert!(!record.facts_equal);
                // The full branch explicitly replaces the complete document
                // state, so the whole old representation retires here. This
                // is the frozen semantic route, never an unaffected-path
                // retirement — the sentinel below is charged on the local
                // route only.
                sink.owners_removed(old.owners.records() as u64);
                crate::retirement::retire_document(old, sink);

                let h_new = fresh.owners.height();
                if fresh.owners.root.is_some() {
                    sink.aggregate_reads(1);
                }
                sink.record_height_new(h_new as u64);

                sink.forbidden(ForbiddenKind::AttributionTreeWalk, 0);
                fresh
            }
        }
    }
}
