//! markit-mdbench-horse-a — the frozen Horse-A v1 mechanism.
//!
//! **Slice I2** of the `HORSE-A-IMPL-1` umbrella (issue #62): the
//! retained READY state, canonical Owner coverage, persistent
//! RestartCertificate support, document-global RefTable ownership, and
//! the same-target full builder. Authority:
//! `docs/research/horse-a-v1-algorithm.md` (frozen; source authorities
//! #55/PR #54, #59, #60), on top of the merged I1 parser observation
//! seam (PR #63).
//!
//! What I2 establishes:
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
//! What I2 deliberately does NOT implement (later slices):
//!
//! - I3: the mutable weighted-AVL operators (`locate_by_byte`,
//!   `safe_predecessor`, `split`, `join*`, `replace_range`, cursor) —
//!   only the construction-only balanced build exists here;
//! - I4: the incremental update / restart / convergence / semantic-
//!   preservation branch — no `update()`, no edit-damage selection, no
//!   RIGHT-affinity navigation;
//! - I5: `PreparedCommit`, retirement, the structural counter schema;
//! - any #60 treatment registration or collection lane.

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
