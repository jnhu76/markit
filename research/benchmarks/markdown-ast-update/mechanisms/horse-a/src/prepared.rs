//! The formal commit frontier (#59 §6.1, §9, §11; algorithm spec §10
//! steps 11–16, §11): [`UpdateStaging`] is the pre-frontier staging state
//! and [`PreparedCommit`] is the frontier-crossing state. The distinction
//! is the frozen ownership and failure boundary, not a rename:
//!
//! ```text
//! stage()  — fallible; the old READY state stays owned by the caller,
//!            coherent and borrowable; every fallible phase completes here
//! prepare() — the last pre-frontier preparation: the old READY document
//!            is consumed BY OWNERSHIP into the prepared state and the
//!            fixed scalar scale diagnostics are frozen
//! commit() — CROSS THE FRONTIER: infallible, no Result, no return to
//!            staging; only structural ownership moves, split/join/
//!            relink, rotations, aggregate recomputation, fixed scalar
//!            counter updates, the single RefTable move, state
//!            installation and retirement
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

/// Which route the prepared commit will cross the frontier with. Both
/// variants are complete before formation: the local plan carries the
/// eager fresh Owners, the full plan carries an already-READY state.
#[derive(Debug)]
pub(crate) enum CommitPlan {
    /// The frozen facts-preserved route (spec §8 10a): the old RefTable is
    /// REUSED (intent marker — never cloned), the fresh replacement Owners
    /// were materialized under it during staging.
    Local {
        fresh: crate::state::OwnerSeq,
        post_id: markit_mdbench_common::SourceId,
        post_len: usize,
    },
    /// The frozen conservative route (spec §8 10b): a complete same-target
    /// READY candidate that owns its own new RefTable.
    Full(ReadyDocument),
}

/// The pre-frontier staging state: every fallible phase has completed and
/// both candidate plans are fully materialized, while the old READY state
/// is still owned by the caller and remains coherent and borrowable.
/// Staging holds no borrow of the old document and no duplicate of its
/// RefTable.
#[derive(Debug)]
pub(crate) struct UpdateStaging {
    pub(crate) record: UpdateRecord,
    pub(crate) plan: CommitPlan,
}

impl UpdateStaging {
    /// Form the [`PreparedCommit`]: consume the old READY document by
    /// ownership and freeze the remaining fixed scalar preparation.
    ///
    /// This is the last pre-frontier work. It performs only O(1)
    /// root-aggregate reads on the old sequence and the prepared candidate
    /// (charged to the structural ledger) and computes the scale
    /// diagnostics; it is infallible and allocates nothing.
    pub(crate) fn prepare(
        self,
        old: ReadyDocument,
        sink: &mut dyn HorseAStructuralSink,
    ) -> PreparedCommit {
        // Old-state scale/height diagnostics: O(1) root aggregate reads
        // (charged where they occur). The candidate's record count is read
        // from the prepared plan; its height is only known once the new
        // root is installed past the frontier, so `H_new` is recorded at
        // the end of `commit`.
        let m_old = old.owners.records();
        let h_old = old.owners.height();
        if old.owners.root.is_some() {
            sink.aggregate_reads(2);
        }
        sink.record_scale_old(m_old as u64, h_old as u64);

        let m_new = match &self.plan {
            CommitPlan::Local {
                fresh,
                post_len: _,
                post_id: _,
            } => {
                let m_new = m_old - old_replaced_records(&self.record) + fresh.records();
                if fresh.root.is_some() {
                    sink.aggregate_reads(1);
                }
                m_new
            }
            CommitPlan::Full(fresh) => {
                let m_new = fresh.owners.records();
                if fresh.owners.root.is_some() {
                    sink.aggregate_reads(1);
                }
                m_new
            }
        };
        sink.record_scale_new(m_new as u64);

        PreparedCommit {
            old,
            record: self.record,
            plan: self.plan,
        }
    }
}

/// The old replacement interval's Owner-record count (the records this
/// commit detaches on the local route).
fn old_replaced_records(record: &UpdateRecord) -> usize {
    record.intervals.old_ranks.end - record.intervals.old_ranks.start
}

/// The frontier-crossing state (#59 §6.1): owns the old READY document by
/// ownership, the frozen geometry record, and the complete route plan.
/// Every ordinary recoverable/fallible operation required for commit has
/// completed; only non-fallible structural ownership operations remain.
#[derive(Debug)]
pub(crate) struct PreparedCommit {
    old: ReadyDocument,
    record: UpdateRecord,
    plan: CommitPlan,
}

impl PreparedCommit {
    /// CROSS THE COMMIT FRONTIER — consume the old representation and
    /// produce the next READY state.
    ///
    /// Infallible by construction: no `Result`, no fallible branch, no
    /// return to staging. The only failure mode is a process-level panic
    /// on a violated internal invariant, which is an implementation bug,
    /// not a mechanism branch.
    pub(crate) fn commit(self, sink: &mut dyn HorseAStructuralSink) -> ReadyDocument {
        let PreparedCommit { old, record, plan } = self;
        match plan {
            CommitPlan::Local {
                fresh,
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
                let detached = owners.replace_range(ranks.start, ranks.end, fresh, sink);
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
                // suffix payload is never traversed. The retirement cannot
                // touch P/S structurally: it receives exactly the detached
                // tree and nothing else.
                drop(detached);
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
                drop(old);

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
