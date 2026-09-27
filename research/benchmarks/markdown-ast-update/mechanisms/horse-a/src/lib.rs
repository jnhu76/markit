//! markit-mdbench-horse-a — the frozen Horse-A v1 mechanism.
//!
//! **Slice I5** of the `HORSE-A-IMPL-1` umbrella (issue #62): the formal
//! `PreparedCommit` frontier, the no-normal-failure post-frontier resource
//! discipline, ownership-proportional retirement, and the
//! `HORSE-A-STRUCTURAL-COUNTERS-v1` attribution record — on top of the
//! merged I4 complete incremental update (PR #69), the I3 weighted-AVL
//! substrate (PR #68), the I2 retained READY state (PR #64) and the I1
//! parser observation seam (PR #63). Authority:
//! `docs/research/horse-a-v1-algorithm.md` (frozen; source authorities
//! #55/PR #54, #59, #60).
//!
//! What I4 established before it — one update is exactly the frozen
//! pipeline, with every phase a distinguishable function
//! ([`update::stage`] owns it):
//!
//! - **edit/source association** — the shared canonical-edit contract
//!   validates identity, range, UTF-8 boundaries and length arithmetic;
//!   no repair, no re-derived edit, byte coordinates only. The post
//!   source being exactly `edit.apply(old_source)` is a caller/host
//!   precondition: production validation stays O(1) and full-content
//!   comparison is deliberately outside the treatment (see `update`);
//! - **weighted damage locate** — one `O(H)` descent with the frozen RIGHT
//!   affinity, so an edit at a boundary belongs to the text after it;
//! - **restart selection** — the nearest eligible certified predecessor
//!   strictly before the damage, else the distinguished BOF authority;
//! - **conservative left guard** — the certified predecessor's own Owner is
//!   inside the replacement, so `[r, t)` is re-parsed too (W-A1 preserved);
//! - **observed forward parse** — the shared parser runs from the restart
//!   cut and reports each top-level start and root-blank barrier;
//! - **monotone candidate walk** — strictly after the restart cut,
//!   position-eligible before certificate inspection, first valid
//!   convergence wins, real EOF is a legal end (module `candidate`);
//! - **complete replacement intervals** — old and new intervals plus the
//!   single explicit byte-space→Owner-rank-space conversion;
//! - **complete ordered replacement facts** — definition facts of the old
//!   replacement interval vs the freshly parsed region (module `facts`);
//! - **the semantic decision** — equal facts retain the old RefTable and
//!   eager-materialize fresh Owners under it, then splice structurally;
//!   facts differ or preservation is unknown means a same-target full
//!   build. There is no third branch and no budget/cost selector.
//!
//! The returned state is a normal READY state, immediately usable for the
//! next update.
//!
//! What I5 establishes on top of it (#59 §6.1, §9–§11, §14; #60 §9):
//!
//! - [`prepared::UpdateStaging`] — the pre-frontier staging state: every
//!   fallible phase complete, both candidate plans materialized, the old
//!   READY state still owned by the caller;
//! - [`prepared::PreparedCommit`] — the frontier-crossing state: the old
//!   READY document consumed by ownership, the frozen geometry record
//!   set, every ordinary recoverable/fallible operation complete. Crossing
//!   ([`prepared::PreparedCommit::commit`]) is infallible: structural
//!   ownership moves, split/join/relink, rotations, aggregate
//!   recomputation, fixed scalar counter updates, the single RefTable
//!   move, state installation and retirement — nothing else. There is no
//!   post-frontier algorithmic fallback, and a process-level panic is an
//!   implementation bug, not a branch;
//! - [`structural::HorseAStructuralCountersV1`] — the explicitly
//!   versioned structural attribution record, charged at the point of
//!   work through a [`structural::HorseAStructuralSink`]; the no-op lane
//!   ([`update`]) and the recording lane ([`update_with_structural`])
//!   execute the identical algorithm and differ only in sink identity.
//!   Fields distinguish `Unknown` from `Known(0)`/`Known(n)`; forbidden
//!   sentinels are charged `Known(0)` at their defended sites.
//!
//! What I3 established before that (spec §12; #59 §7.2–§7.6, §8; all
//! `pub(crate)` — mechanism substrate, not a reusable AVL library):
//!
//! - [`sequence::locate_by_byte`] — weighted `O(H)` byte locate with the
//!   explicit EOF position at `x == L`;
//! - [`sequence::safe_predecessor`] — the aggregate-pruned nearest
//!   certified predecessor strictly below an exclusive byte bound; no
//!   linear scan toward BOF, position eligibility before certificate
//!   inspection;
//! - `remove_max` / `join_with_pivot` / `join` / `split` — relink-only
//!   structural operators: the frozen deterministic join policy
//!   (`remove_max(left)` exactly once), rank-based one-spine split,
//!   shared `recompute` seam, explicit rotation primitives composed into
//!   doubles; a moved `Box` never relocates its allocation, so retained
//!   node identity survives every operation;
//! - [`sequence::replace_range`] — the frozen split/split/join/join
//!   formula over Owner record ranks, returning the removed range B to
//!   the caller (retirement is I5's);
//! - the sequential monotone cursor (module `cursor`) — one `O(H)`
//!   positioning descent, then source-order advance with no per-Owner
//!   root seek, no parent pointers, no persisted cursor state.
//!
//! What I2 established before it:
//!
//! - [`ReadyDocument`] — the frozen retained state: source association
//!   (substrate `SourceId` + length + [`InterpretationId`]), one record
//!   per root-level top-level block Owner in a byte-weighted
//!   [`OwnerSeq`], and THE document-global
//!   [`RefTable`](markit_mdbench_shared_grammar::RefTable);
//! - [`Owner`] / [`OwnerPayload`] / [`AstPayload`] — the complete eager
//!   semantic subtree of one top-level block in the shared
//!   NORMALIZED-RESULT-v1 vocabulary, owning its values, borrowing
//!   neither the source nor the RefTable, with every span and content
//!   interval Owner-relative (`FencedCode.content` included);
//! - canonical physical-first-line coverage: `c0 = 0`,
//!   `c_i = TopLevelStart.physical_line_start`, `c_k = source_len` —
//!   leading trivia into the first Owner, interstitial/trailing bytes
//!   left, empty source an empty sequence, the frozen root-blank class a
//!   single TriviaOnly Owner;
//! - [`RestartCertificate`] / [`RestartSupport`] — persistent support
//!   built only from real I1 `RootBlankEvent` evidence at interior
//!   Owner boundaries, Owner-relative, with the frozen support-touch
//!   predicate (E24 included);
//! - [`full_build`] — the same-target full builder with the frozen
//!   pipeline order: FINAL RefTable exists before any reference-sensitive
//!   materialization; no deferred semantic repair after READY.
//!
//! What no slice has implemented yet (later slices):
//!
//! - any #60 treatment registration or collection lane, and no
//!   performance measurement of any kind. The frozen #60 bounds are
//!   adjudication instruments for a later explicitly authorized
//!   collection; nothing here encodes them into the mechanism path.
//!   STRUCTURAL_COLLECTION_RUN=NO, PERFORMANCE_COLLECTION_RUN=NO.
//!
//! What I5 deliberately does NOT do (frozen weaknesses preserved, not
//! repaired): W-A1 stays root-only restart plus atomic top-level Owner plus
//! the conservative left guard; W-A2 stays "facts differ or preservation
//! unknown → same-target full build"; W-A3 stays one Owner per AVL node. No
//! nested restart checkpoints, winner index, consumer postings, stable
//! cross-edit IDs, persistent locator map, global subtree reuse index,
//! packed/chunked sequence, COW snapshot, or budget selector exists here.

mod candidate;
pub mod certificate;
pub mod coverage;
mod cursor;
pub mod export;
pub mod facts;
mod fresh;
pub mod full_build;
mod payload;
mod prepared;
mod retirement;
pub mod sequence;
pub mod state;
pub mod structural;
pub mod update;
pub mod validate;

#[cfg(test)]
mod i3_tests;

#[cfg(test)]
mod i4_tests;

#[cfg(test)]
mod i5_tests;

pub use certificate::{RestartCertificate, RestartSupport};
pub use full_build::{full_build, BuildError};
pub use state::{
    Aggregate, AstPayload, AvlNode, InterpretationId, Owner, OwnerPayload, OwnerSeq, ReadyDocument,
};
pub use structural::{
    FullBuildReason, HorseAStructuralCountersV1, HorseAStructuralSink, NoopHorseAStructuralSink,
    Observed, RecordingHorseAStructuralSink,
};
pub use update::{update, update_with_structural, UpdateError};
pub use validate::{
    validate_certificates, validate_coverage, validate_full_build_tree,
    validate_owner_relative_payload, validate_ready, validate_ref_table_projection,
};

// The export seam is the oracle's frozen projection contract; re-exported
// so the state's capability is visible from this crate's API.
pub use markit_mdbench_common::ResultChecksum;
pub use markit_mdbench_oracle::NormalizeV1;
