//! markit-mdbench-horse-a — the frozen Horse-A v1 mechanism.
//!
//! **Slice I3** of the `HORSE-A-IMPL-1` umbrella (issue #62): the
//! weighted-AVL navigation/mutation substrate on top of the merged I2
//! retained READY state (PR #64) and I1 parser observation seam
//! (PR #63). Authority: `docs/research/horse-a-v1-algorithm.md` (frozen;
//! source authorities #55/PR #54, #59, #60).
//!
//! What I3 establishes (spec §12; #59 §7.2–§7.6, §8; all `pub(crate)` —
//! mechanism substrate, not a reusable AVL library):
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
//! What I3 deliberately does NOT implement (later slices):
//!
//! - I4: the incremental update / restart / convergence / semantic-
//!   preservation branch — no `update()`, no edit-damage selection, no
//!   RIGHT-affinity navigation; the I3 operators are its substrate and
//!   call none of the policy;
//! - I5: `PreparedCommit`, retirement, the structural counter schema
//!   (the operators keep relink/rotation/recompute events explicit and
//!   countable, but no counters exist);
//! - any #60 treatment registration or collection lane, and no
//!   performance measurement of any kind.

pub mod certificate;
pub mod coverage;
mod cursor;
pub mod export;
pub mod full_build;
pub mod payload;
pub mod sequence;
pub mod state;
pub mod validate;

#[cfg(test)]
mod i3_tests;

pub use certificate::{RestartCertificate, RestartSupport};
pub use full_build::{full_build, BuildError};
pub use state::{
    Aggregate, AstPayload, AvlNode, InterpretationId, Owner, OwnerPayload, OwnerSeq, ReadyDocument,
};
pub use validate::{
    validate_certificates, validate_coverage, validate_full_build_tree,
    validate_owner_relative_payload, validate_ready, validate_ref_table_projection,
};

// The export seam is the oracle's frozen projection contract; re-exported
// so the state's capability is visible from this crate's API.
pub use markit_mdbench_common::ResultChecksum;
pub use markit_mdbench_oracle::NormalizeV1;
