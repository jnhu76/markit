//! The complete Horse-A incremental update (spec §7–§13; #59 §3/§5/§8/§12;
//! I4 task contract §2/§7–§22/§29/§32).
//!
//! The pipeline is deliberately visible, in the frozen order:
//!
//! ```text
//! validate edit/source association            (§7  of the I4 task)
//! → weighted damage locate, RIGHT affinity    (§8)
//! → nearest eligible certified predecessor    (§9)
//! → conservative left guard                   (§10)
//! → forward parse, shared observed parser     (§11)
//! → monotone candidate walk                   (§12–§14)
//!       first valid convergence strictly after the restart cut, or real EOF
//! → complete old/new replacement intervals    (§16)
//! → complete ordered replacement facts        (§17)
//! → facts equal / preservation proven:
//!       reuse old RefTable, eager-materialize fresh Owners, splice locally
//!   facts differ OR preservation unknown:
//!       same-target full build                  (§18–§21)
//! ```
//!
//! Binding rules that are not negotiable in this file: the restart cut is
//! never a convergence candidate; position eligibility precedes persistent
//! certificate inspection; the walk is monotone; the first valid
//! convergence wins; real EOF is a legal replacement end; and there is no
//! budget/cost selector anywhere — the only semantic full-build branch is
//! `facts differ OR preservation unknown`.
//!
//! Staging (`stage`) is fallible and leaves the old READY state untouched
//! and coherent. `commit` is the pre-I5 commit boundary: consuming
//! ownership moves, one structural splice, exactly one RefTable move, and
//! the retirement of the detached middle. I5 owns the formal
//! `PreparedCommit` frontier, the retirement discipline and the structural
//! counters; nothing here counts anything.

use std::ops::Range;

use markit_mdbench_common::{CanonicalEdit, Source, SourceId, WorkSink};
use markit_mdbench_shared_grammar::{
    parse_region_observed, ObserverControl, RefTable, RegionObserver, RegionOutcome,
    RootBlankEvent, TopLevelEvent,
};

use crate::candidate::{AcceptedConvergence, CandidateWalk, ConvergencePolicy, WalkOutcome};
use crate::facts::OrderedFacts;
use crate::fresh::{build_replacement_owners, SealedRegion};
use crate::full_build::{full_build, BuildError};
use crate::sequence::Located;
use crate::state::{InterpretationId, OwnerSeq, ReadyDocument};

/// Why an update refused to return a state. Every variant is a
/// pre-frontier refusal: the old READY state is untouched and still
/// coherent, and the caller may keep using it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// The old state, the canonical edit and the post source do not describe
    /// one consistent transition (identity, edit geometry, UTF-8 boundaries,
    /// or length arithmetic).
    InvalidAssociation { detail: String },
    /// Parser/structural evidence contradicts the frozen contracts. An
    /// implementation/invariant failure — never an algorithmic fallback.
    InconsistentObservation { detail: String },
    /// The same-target full builder refused to produce a state.
    FullBuild(BuildError),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::InvalidAssociation { detail } => {
                write!(f, "invalid update association: {detail}")
            }
            UpdateError::InconsistentObservation { detail } => {
                write!(f, "inconsistent parser observation: {detail}")
            }
            UpdateError::FullBuild(e) => write!(f, "same-target full build failed: {e}"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// Which route completed the update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdatePath {
    /// Facts-preserved local path (spec §8, §10 10a): the old RefTable is
    /// retained, fresh replacement Owners are eager-materialized under it,
    /// and the retained prefix/suffix are spliced structurally.
    Local,
    /// The frozen conservative branch (spec §8, §10 10b).
    SameTargetFullBuild,
}

/// The nearest eligible certified restart predecessor (spec §7.1), or the
/// distinguished BOF authority when none exists. BOF is an authority, not a
/// synthetic certificate object: `certified_rank` is `None` for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestartSelection {
    /// The absolute byte cut the forward parse starts from.
    pub(crate) cut: usize,
    /// Source-order rank of the certified Owner whose outgoing cut is `cut`.
    pub(crate) certified_rank: Option<usize>,
}

/// The complete old/new replacement intervals (spec §16). Byte space and
/// Owner-rank space stay explicit: the parser/restart logic reasons in
/// bytes, the local structural splice needs the rank range, and the
/// conversion between them happens exactly once, here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplacementIntervals {
    /// Old replacement interval `[r, q_old)` in old-source bytes.
    pub(crate) old: Range<usize>,
    /// New replacement interval `[r, q_new)` in new-source bytes. The base
    /// is the SAME `r`: bytes before the restart are byte-identical.
    pub(crate) new: Range<usize>,
    /// Old replacement interval in Owner ranks `[lo, hi)`.
    pub(crate) old_ranks: Range<usize>,
}

/// The I4 decision record: the geometry the conformance tests observe and
/// that the commit needs. It is NOT the I5 structural counter schema — it
/// holds no visit/link-write/rotation/aggregate counters, and nothing here
/// is a work estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateRecord {
    /// `t`: the coverage base of the Owner the RIGHT-affinity locate hit
    /// (i.e. `L_old` for an edit at EOF).
    pub(crate) damage_base: usize,
    pub(crate) restart: RestartSelection,
    /// `Some` when the first valid convergence was accepted; `None` when the
    /// forward parse ran to real EOF.
    pub(crate) convergence: Option<AcceptedConvergence>,
    pub(crate) intervals: ReplacementIntervals,
    /// Whether the complete ordered replacement facts were equal — and
    /// therefore whether preservation under the retained environment is
    /// proven.
    pub(crate) facts_equal: bool,
    pub(crate) path: UpdatePath,
}

/// Which commit plan staging produced. Both are complete before the commit
/// boundary: the local plan carries the eager fresh Owners, the full plan
/// carries an already-READY state.
#[derive(Debug)]
pub(crate) enum CommitPlan {
    Local {
        fresh: OwnerSeq,
        post_id: SourceId,
        post_len: usize,
    },
    Full(ReadyDocument),
}

/// The staged update: everything decided and prepared while the old READY
/// state was still untouched (the pre-I5 shape of the frozen staging state;
/// I5 owns the formal `PreparedCommit` frontier and its no-fail proof).
#[derive(Debug)]
pub(crate) struct StagedUpdate {
    pub(crate) record: UpdateRecord,
    pub(crate) plan: CommitPlan,
}

/// The validated association of `(old state, edit, old source, post source)`
/// (I4 task contract §7).
struct Association {
    edit_start: usize,
    edit_end: usize,
    /// `inserted_len - (edit_end - edit_start)`, in a width that cannot
    /// wrap: the canonical edit association's position delta.
    delta: i128,
}

/// Phase 1 — establish that the old READY state, the canonical edit and the
/// supplied post source are mutually consistent (spec §10 step 1).
///
/// The association authority is the shared canonical-edit contract: this
/// validates identity, the byte range, UTF-8 char boundaries and the length
/// arithmetic. It never re-derives a different edit by diffing old and new
/// source, and it never repairs a malformed edit description. Byte
/// coordinates are the only coordinates here (frozen spec §8; no
/// byte/scalar/grapheme conflation).
fn validate_association(
    old: &ReadyDocument,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
) -> Result<Association, UpdateError> {
    let invalid = |detail: String| UpdateError::InvalidAssociation { detail };

    if old.source_id != old_source.id() {
        return Err(invalid(format!(
            "the old state describes {} but the supplied old source is {}",
            old.source_id,
            old_source.id()
        )));
    }
    if old.source_len != old_source.len_bytes() {
        return Err(invalid(format!(
            "the old state covers {} bytes but the old source has {}",
            old.source_len,
            old_source.len_bytes()
        )));
    }
    if old.interpretation != InterpretationId::HORSE_A_V1 {
        return Err(invalid(format!(
            "the old state was produced under interpretation {:?}, not Horse-A v1",
            old.interpretation
        )));
    }
    // O(1) aggregate consistency: the retained sequence must still cover the
    // source it claims.
    if old.owners.total_bytes() != old.source_len {
        return Err(invalid(format!(
            "the retained sequence covers {} bytes but the state claims {}",
            old.owners.total_bytes(),
            old.source_len
        )));
    }

    // Byte range + UTF-8 char boundaries against the old source.
    edit.validate_against(old_source)
        .map_err(|e| invalid(e.to_string()))?;

    let edit_start = edit.start_byte() as usize;
    let edit_end = edit.end_byte() as usize;
    let inserted = edit.inserted_text_len_bytes() as usize;
    let removed = edit_end - edit_start;
    let expected = old
        .source_len
        .checked_add(inserted)
        .and_then(|len| len.checked_sub(removed))
        .ok_or_else(|| {
            invalid("the edit's length arithmetic underflows the old source length".to_string())
        })?;
    if expected != post_source.len_bytes() {
        return Err(invalid(format!(
            "the edit describes a {expected}-byte post source but {} bytes were supplied",
            post_source.len_bytes()
        )));
    }

    // Debug-only belt: the post bytes really are this edit applied to the
    // old bytes. This audits HOST input, never retained state, and is
    // compiled out of release exactly like the READY validators; the O(1)
    // association set above is what the mechanism itself relies on.
    #[cfg(debug_assertions)]
    {
        let derived = edit
            .apply(old_source, post_source.id())
            .map_err(|e| invalid(format!("the edit does not produce the post source: {e}")))?;
        debug_assert!(
            derived.as_bytes() == post_source.as_bytes(),
            "the supplied post source is not this edit applied to the old source"
        );
    }

    Ok(Association {
        edit_start,
        edit_end,
        delta: inserted as i128 - removed as i128,
    })
}

/// Phase 2 — weighted damage locate with the frozen RIGHT affinity (spec §10
/// step 2; data-model §3.3). One `O(H)` weighted descent over the retained
/// sequence, never an Owner enumeration.
///
/// The result is `t`, the coverage base of the Owner the position lands in
/// (the RIGHT Owner when the position is a boundary), or `L_old` for
/// `a == L_old` including the empty document. The deletion's upper endpoint
/// needs no separate LEFT view: the candidate predicate tests the raw
/// canonical edit interval, and every candidate must lie at or after
/// `edit_end`.
fn locate_damage(owners: &OwnerSeq, edit_start: usize) -> usize {
    match owners.locate_by_byte(edit_start) {
        Located::Owner(located) => located.base,
        Located::Eof => owners.total_bytes(),
    }
}

/// Phase 3 — the nearest eligible certified predecessor strictly before `t`
/// (spec §7.1), or the distinguished BOF authority.
///
/// Position eligibility precedes certificate inspection inside the I3
/// substrate. The returned certificate is only a *candidate* boundary:
/// whether the edit invalidated its support is decided by the convergence
/// predicate, and whether it can be reused at all is decided by this round's
/// own forward parse.
fn select_restart(owners: &OwnerSeq, damage_base: usize) -> RestartSelection {
    match owners.safe_predecessor(damage_base) {
        Some(boundary) => RestartSelection {
            cut: boundary.boundary,
            certified_rank: Some(boundary.rank),
        },
        None => RestartSelection {
            cut: 0,
            certified_rank: None,
        },
    }
}

/// Phase 4 — the conservative left guard (spec §7.2; data-model §3.4).
///
/// The replacement begins at the restart cut, so `[r, t)` contains at least
/// one complete, unedited top-level block — the guard Owner — which can
/// absorb the replacement's trailing blank lines and keeps the fresh region
/// coverable. The first replaced Owner is therefore the one that starts at
/// the restart cut. This extra parse is an intentional Horse-A cost (W-A1),
/// never a tuning knob.
fn first_replaced_rank(restart: RestartSelection) -> usize {
    restart.certified_rank.map_or(0, |rank| rank + 1)
}

/// The forward parse's observation state (spec §5, §7.3; #59 §3.3/§8).
///
/// The observer owns the monotone candidate walk and this round's region
/// provenance. It stops the parse at the first sealed barrier the walk
/// accepts — the frozen live-root local stop: the cut is already sealed,
/// nothing before it is pending, no artificial `finish()` is involved, and
/// real EOF stays a separate completion path.
struct ForwardObserver<'s> {
    walk: CandidateWalk<'s>,
    /// `TopLevelStart` physical line starts inside the region (coverage
    /// provenance).
    starts: Vec<usize>,
    /// Every pre-EOF root blank barrier the parser sealed inside the region
    /// (certificate evidence) — recorded whether or not it converges.
    barriers: Vec<RootBlankEvent>,
    accepted: Option<AcceptedConvergence>,
}

impl RegionObserver for ForwardObserver<'_> {
    fn on_top_level_start(&mut self, ev: TopLevelEvent) {
        self.starts.push(ev.physical_line_start);
    }

    fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl {
        self.barriers.push(ev);
        // Coverage closure (spec §3.4 item 4): the replacement prefix must
        // already contain a top-level block, otherwise the bytes in front of
        // this barrier belong to the first retained block and the barrier is
        // not a legal coverage cut. `starts` holds exactly the blocks sealed
        // before this barrier.
        let replacement_has_block = !self.starts.is_empty();
        match self.walk.on_sealed_barrier(ev.cut, replacement_has_block) {
            WalkOutcome::Converged(accepted) => {
                self.accepted = Some(accepted);
                ObserverControl::Stop
            }
            // No convergence here: keep parsing toward the next barrier, and
            // otherwise toward real EOF.
            WalkOutcome::Continue | WalkOutcome::Exhausted => ObserverControl::Continue,
        }
    }
}

/// The forward parse's sealed result.
struct ForwardParse {
    region: SealedRegion,
    /// The region's own definition facts — the complete `Defs(N)`.
    defs: RefTable,
    accepted: Option<AcceptedConvergence>,
    /// `L_new`, kept explicitly because the region end may be an interior
    /// convergence cut instead.
    new_len: usize,
}

/// Phases 5–7 — forward parse from the restart with the SHARED observed
/// parser, driving the monotone candidate walk, ending at the first valid
/// convergence or at real EOF (spec §7.3; I4 task §11–§15).
///
/// The parse always runs to the real end of the new source: real EOF is
/// never truncated away, no budget stops it, and no `finish()` is used to
/// manufacture convergence (the stop happens at an already-sealed cut).
fn forward_parse<W: WorkSink>(
    old: &ReadyDocument,
    post_source: &Source,
    association: &Association,
    restart: RestartSelection,
    first_rank: usize,
    sink: &mut W,
) -> ForwardParse {
    let src = post_source.as_bytes();
    let new_len = src.len();
    let policy = ConvergencePolicy {
        restart_cut: restart.cut,
        edit_start: association.edit_start,
        edit_end: association.edit_end,
        delta: association.delta,
    };
    let mut observer = ForwardObserver {
        walk: CandidateWalk::begin(&old.owners, first_rank, policy),
        starts: Vec::new(),
        barriers: Vec::new(),
        accepted: None,
    };

    let observed = parse_region_observed(src, restart.cut, new_len, sink, &mut observer);
    let ForwardObserver {
        starts,
        barriers,
        accepted,
        ..
    } = observer;

    let end = match observed.outcome {
        RegionOutcome::StoppedAtCertifiedCut { cut } => {
            debug_assert_eq!(
                accepted.map(|c| c.new_cut),
                Some(cut),
                "the parse may only stop at the convergence the walk accepted"
            );
            cut
        }
        RegionOutcome::RanToEnd => {
            debug_assert!(
                accepted.is_none(),
                "a run to real EOF accepted no convergence"
            );
            new_len
        }
    };

    let region = SealedRegion {
        base: restart.cut,
        end,
        blocks: observed.region.blocks,
        starts,
        barriers,
    };
    ForwardParse {
        region,
        defs: observed.region.defs,
        accepted,
        new_len,
    }
}

/// Phase 8 — the complete old/new replacement intervals (spec §16).
///
/// On a convergence the old side ends at the accepted certified cut and the
/// new side at its exact image; at real EOF the old side ends at `L_old` and
/// the new side at `L_new`, leaving an empty retained suffix. Both sides
/// share the restart base, because bytes before the restart are
/// byte-identical.
fn complete_intervals(
    old: &ReadyDocument,
    restart: RestartSelection,
    first_rank: usize,
    forward: &ForwardParse,
) -> ReplacementIntervals {
    let (old_end, old_end_rank) = match forward.accepted {
        Some(accepted) => (accepted.old_cut, accepted.certified_rank + 1),
        None => (old.source_len, old.owners.records()),
    };
    let intervals = ReplacementIntervals {
        old: restart.cut..old_end,
        new: restart.cut..forward.region.end,
        old_ranks: first_rank..old_end_rank,
    };
    debug_assert!(
        intervals.old.end <= old.source_len && intervals.new.end <= forward.new_len,
        "the replacement intervals must stay inside their sources"
    );
    debug_assert!(
        intervals.old_ranks.end <= old.owners.records(),
        "the rank interval must stay inside the retained sequence"
    );
    intervals
}

/// Stage one update: run every fallible phase while the old READY state
/// remains untouched and coherent.
///
/// The ordering is frozen: the complete replacement block parse happens
/// first, then the complete ordered facts on both sides, then the comparison,
/// then the environment choice — and only then is any reference-sensitive
/// payload materialized (spec §9: no payload may be built under an
/// environment the facts decision has not yet fixed).
pub(crate) fn stage<W: WorkSink>(
    old: &ReadyDocument,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    sink: &mut W,
) -> Result<StagedUpdate, UpdateError> {
    // Phase 1 — validation.
    let association = validate_association(old, old_source, post_source, edit)?;

    // Phase 2 — weighted damage locate (RIGHT affinity).
    let damage_base = locate_damage(&old.owners, association.edit_start);
    debug_assert!(damage_base <= old.source_len);

    // Phase 3 — nearest eligible certified restart predecessor.
    let restart = select_restart(&old.owners, damage_base);

    // Phase 4 — conservative left guard: the first replaced Owner starts at
    // the restart cut.
    let first_rank = first_replaced_rank(restart);

    // Phases 5–7 — forward parse + monotone candidate walk.
    let forward = forward_parse(old, post_source, &association, restart, first_rank, sink);

    // Phase 8 — complete old/new replacement intervals (byte and rank spaces
    // made explicit).
    let intervals = complete_intervals(old, restart, first_rank, &forward);

    // Phase 9 — complete ordered replacement facts on both sides, before any
    // semantic materialization.
    let old_facts = OrderedFacts::of_old_replacement(&old.owners, intervals.old_ranks.clone());
    let new_facts = OrderedFacts::of_fresh_region(&forward.defs);
    let facts_equal = old_facts == new_facts;
    debug_assert_eq!(
        facts_equal,
        old_facts.entries() == new_facts.entries(),
        "preservation is decided by the complete ordered fact sequences, \
         never by a derived flag"
    );

    // Phase 10 — the semantic-preservation decision. Exactly two outcomes:
    // proven (facts equal) and not proven (differ, or unknown — and in this
    // realization an extraction that cannot be completed is an invariant
    // error, never a silent "unknown" that pretends to be equal).
    let (path, plan) = if facts_equal {
        // 10a — retain the old RefTable and eager-materialize the fresh
        // replacement Owners under it. No document-wide definition
        // recollection, no per-Owner rewrite of retained payload, no patch
        // of the table.
        let fresh =
            build_replacement_owners(post_source.as_bytes(), forward.region, &old.refs, sink)?;
        (
            UpdatePath::Local,
            CommitPlan::Local {
                fresh,
                post_id: post_source.id(),
                post_len: forward.new_len,
            },
        )
    } else {
        // 10b — the frozen conservative branch: build the complete
        // same-target READY state. Not a weaker state, not a repair of
        // selected Owners, and never selected by anything except this
        // semantic decision.
        let full = full_build(post_source, sink).map_err(UpdateError::FullBuild)?;
        (UpdatePath::SameTargetFullBuild, CommitPlan::Full(full))
    };

    Ok(StagedUpdate {
        record: UpdateRecord {
            damage_base,
            restart,
            convergence: forward.accepted,
            intervals,
            facts_equal,
            path,
        },
        plan,
    })
}

/// The frozen update entry point: stage, then cross the commit boundary.
///
/// A staging error returns `Err` while the old state is still coherent and
/// untouched; the frozen disposition table places association/resource
/// failures here, never in an algorithmic full-build fallback.
pub fn update<W: WorkSink>(
    old: ReadyDocument,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    sink: &mut W,
) -> Result<ReadyDocument, UpdateError> {
    let staged = stage(&old, old_source, post_source, edit, sink)?;
    Ok(commit(old, staged))
}

/// The commit boundary (pre-I5 shape): consume the old representation and
/// produce the next READY state.
///
/// Local plan: two rank splits, the fresh middle spliced in, the old
/// RefTable moved exactly once, and exactly the detached middle retired —
/// retained prefix/suffix ownership is transferred structurally, never
/// reinserted record-by-record. Full plan: install the already-complete
/// same-target state and retire the whole old representation.
pub(crate) fn commit(old: ReadyDocument, staged: StagedUpdate) -> ReadyDocument {
    let StagedUpdate { record, plan } = staged;
    match plan {
        CommitPlan::Local {
            fresh,
            post_id,
            post_len,
        } => {
            debug_assert_eq!(record.path, UpdatePath::Local);
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
            // The one structural splice: called with the explicit rank
            // interval, so the retained prefix and suffix are transferred by
            // ownership and the detached middle comes back intact rather
            // than being dropped inside the operator.
            let detached = owners.replace_range(ranks.start, ranks.end, fresh);
            let next = ReadyDocument {
                source_id: post_id,
                source_len: post_len,
                interpretation: InterpretationId::HORSE_A_V1,
                owners,
                refs,
            };
            // Retire the detached middle only — the retained prefix/suffix
            // payload is never traversed (I5 owns the accounting for this
            // drop walk; this slice owns doing it before READY).
            drop(detached);

            #[cfg(debug_assertions)]
            if let Err(detail) = crate::validate::validate_ready(&next) {
                panic!("a local replacement produced an invalid READY state: {detail}");
            }
            next
        }
        CommitPlan::Full(fresh) => {
            debug_assert_eq!(record.path, UpdatePath::SameTargetFullBuild);
            debug_assert!(!record.facts_equal);
            // The full branch explicitly replaces the complete document
            // state, so the whole old representation retires here.
            drop(old);
            fresh
        }
    }
}
