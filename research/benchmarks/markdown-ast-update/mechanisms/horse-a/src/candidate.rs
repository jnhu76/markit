//! The monotone old-side candidate walk (spec §7.3, §12–§14; #59 §8; I4
//! task contract §12–§15).
//!
//! The walk offers the old retained suffix's *certified* boundaries to the
//! frozen convergence predicate, in source order, as the forward parse
//! seals new-side root barriers. Binding rules implemented here, literally:
//!
//! ```text
//! the restart cut is NOT a convergence candidate
//! the first candidate position is strictly after the restart cut
//! candidate walking is monotone: one cursor, no root seek, no re-offering
//! the first valid convergence wins; nothing is searched beyond it
//! exhaustion means real EOF is the replacement end (not a failure)
//! ```
//!
//! The cursor is the I3 monotone substrate: one positioning descent at
//! `begin`, then sequential `next()` advances. The walk holds no candidate
//! vector, re-seeks nothing, and owns no grammar knowledge.

use crate::cursor::{CursorItem, OwnerCursor};
use crate::state::OwnerSeq;

/// The frozen convergence-policy inputs (spec §7.3). Every value comes from
/// the validated edit association and the selected restart; none of them is
/// a work estimate, so nothing here can become a cost selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConvergencePolicy {
    /// The restart cut `r`: the forward parse starts here, and this boundary
    /// is never itself offered as a candidate (spec §7.3).
    pub(crate) restart_cut: usize,
    /// The edit's old interval `[edit_start, edit_end)`: the deleted range
    /// that must be fully crossed before any candidate can converge.
    pub(crate) edit_start: usize,
    pub(crate) edit_end: usize,
    /// `inserted_len - (edit_end - edit_start)`, in a width that cannot
    /// wrap. Candidates map to the new source by `q_old + delta`; this is
    /// the canonical edit association, never a re-diffed edit.
    pub(crate) delta: i128,
}

/// An accepted convergence (spec §7.3): the old certified cut, its exact
/// new-side image, and the source-order rank of the certified Owner whose
/// outgoing boundary is the old cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AcceptedConvergence {
    pub(crate) old_cut: usize,
    pub(crate) new_cut: usize,
    pub(crate) certified_rank: usize,
}

/// The walk's answer for one sealed new-side root barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkOutcome {
    /// No unoffered certified boundary remains: no earlier convergence is
    /// possible, so the parse simply continues to real EOF.
    Exhausted,
    /// Every candidate the parser has reached so far was rejected; the next
    /// candidate is still ahead of this barrier. Parsing continues.
    Continue,
    /// The first valid convergence, accepted where it stands. The caller
    /// stops the forward parse here; the walk keeps the same answer for any
    /// later call rather than silently advancing.
    Converged(AcceptedConvergence),
}

/// The monotone candidate walk over one old retained sequence.
pub(crate) struct CandidateWalk<'s> {
    cursor: OwnerCursor<'s>,
    /// The next certified boundary not yet offered.
    next: Option<CursorItem<'s>>,
    policy: ConvergencePolicy,
    /// The accepted convergence, recorded so the answer is stable.
    accepted: Option<AcceptedConvergence>,
    /// Strict-monotonicity guard: the rank of the last offered candidate.
    /// The walk only ever moves forward; a regression here would mean a
    /// re-seek.
    last_offered_rank: Option<usize>,
}

impl<'s> CandidateWalk<'s> {
    /// Position the walk at the first eligible certified boundary strictly
    /// after `restart_cut`. `first_rank` is the caller's honest starting
    /// rank (the first replaced Owner); the restart-cut guard below holds
    /// independently of it, so a wrong rank cannot offer the restart cut.
    pub(crate) fn begin(
        owners: &'s OwnerSeq,
        first_rank: usize,
        policy: ConvergencePolicy,
    ) -> Self {
        let cursor = owners.cursor_at_rank(first_rank);
        let mut walk = Self {
            cursor,
            next: None,
            policy,
            accepted: None,
            last_offered_rank: None,
        };
        walk.seek_next_certified();
        walk
    }

    /// Offer one already-sealed new-side root barrier to the complete
    /// frozen convergence predicate (spec §7.3). `replacement_has_block`
    /// reports whether the replacement prefix parsed so far contains at
    /// least one top-level block start.
    pub(crate) fn on_sealed_barrier(
        &mut self,
        new_cut: usize,
        replacement_has_block: bool,
    ) -> WalkOutcome {
        // A convergence already accepted is the walk's final answer: the
        // caller stops the parse, and a hypothetical later call must not
        // silently move the replacement end.
        if let Some(accepted) = self.accepted {
            return WalkOutcome::Converged(accepted);
        }
        loop {
            let Some(item) = self.next else {
                return WalkOutcome::Exhausted;
            };
            let Some(mapped) = map_to_new(item.boundary_cut, self.policy.delta) else {
                // This old position has no new-side image at all (a deletion
                // larger than the position itself): it can never map onto a
                // barrier, so it is skipped — not offered, not "rejected".
                self.advance();
                continue;
            };
            if mapped > new_cut {
                // The parser has not reached this candidate's mapped cut
                // yet. Barriers arrive in increasing order, so it stays the
                // next candidate.
                return WalkOutcome::Continue;
            }
            // One full predicate evaluation for one offered candidate.
            if let Some(accepted) = self.evaluate(item, mapped, new_cut, replacement_has_block) {
                self.accepted = Some(accepted);
                return WalkOutcome::Converged(accepted);
            }
            self.advance();
        }
    }

    /// The complete frozen convergence predicate (spec §7.3). Every clause
    /// is a frozen requirement; all of them must hold.
    fn evaluate(
        &mut self,
        item: CursorItem<'s>,
        mapped: usize,
        new_cut: usize,
        replacement_has_block: bool,
    ) -> Option<AcceptedConvergence> {
        debug_assert!(
            item.boundary_cut > self.policy.restart_cut,
            "the restart cut is never offered as a convergence candidate"
        );
        debug_assert!(
            self.last_offered_rank.is_none_or(|rank| item.rank > rank),
            "candidate walking is monotone and offers each boundary at most once"
        );
        self.last_offered_rank = Some(item.rank);

        // (1) edit/damage fully crossed: the whole deleted old range lies at
        //     or before the candidate cut, so the replacement cannot stop in
        //     front of the edit.
        if item.boundary_cut < self.policy.edit_end {
            return None;
        }
        // (2) exact old<->new mapping: the candidate's image is THIS sealed
        //     barrier. `q_new >= a + u` follows from (1) and the mapping, so
        //     the inserted bytes are always consumed by the replacement.
        if mapped != new_cut {
            return None;
        }
        // (3) the candidate's persistent certificate is not invalidated by
        //     the edit: a touched support is conservatively refused (spec
        //     §6; E24). The stored support is Owner-relative, the edit is
        //     document-absolute.
        if let Some(cert) = item.owner.outgoing_restart.as_ref() {
            if cert.support.touches_absolute(
                item.base,
                self.policy.edit_start,
                self.policy.edit_end,
            ) {
                return None;
            }
        }
        // (4) the new live parser is at an equivalent empty-root
        //     continuation: exactly what this sealed barrier IS (its cut is
        //     a pre-EOF empty-root settlement, established by the real
        //     parser, not by `finish()`).
        //
        // (5) coverage is legally splittable at the new-side boundary: the
        //     replacement prefix must already contain a top-level block,
        //     otherwise those leading bytes belong to the first retained
        //     block and this cut is not a legal coverage cut.
        if !replacement_has_block {
            return None;
        }
        // (7) the mapped suffix is unchanged: the canonical edit contract
        //     guarantees `old[b, L_old) == new[a + u, L_new)` byte-for-byte,
        //     and (1) puts the candidate at or after `b`, so no suffix
        //     rescan or hash comparison is needed (or allowed) here.
        Some(AcceptedConvergence {
            old_cut: item.boundary_cut,
            new_cut,
            certified_rank: item.rank,
        })
    }

    /// Move to the next certified boundary strictly after the restart cut.
    /// Non-certified Owners crossed on the way carry no candidate.
    fn advance(&mut self) {
        self.seek_next_certified();
    }

    fn seek_next_certified(&mut self) {
        loop {
            let Some(item) = self.cursor.next() else {
                self.next = None;
                return;
            };
            // The restart boundary is a parse starting authority only; a
            // boundary at or before it is never a candidate (spec §7.3).
            if item.boundary_cut <= self.policy.restart_cut {
                continue;
            }
            if item.owner.outgoing_restart.is_none() {
                continue;
            }
            self.next = Some(item);
            return;
        }
    }
}

/// The canonical edit association's position map: `q_old + delta`, or
/// `None` when the position has no new-side image.
fn map_to_new(old: usize, delta: i128) -> Option<usize> {
    usize::try_from(old as i128 + delta).ok()
}
