//! Focused I4 incremental-update suites (HORSE-A-IMPL-1 #62, slice I4; I4
//! task contract §25/§33): the frozen pipeline geometry, the mechanism-
//! identity pins, the semantic-preservation branch, and the structural-
//! retention probes.
//!
//! The decision-bearing I4 surfaces (`stage`/`commit`, the `UpdateRecord`
//! geometry, the candidate walk) are `pub(crate)` — the I3 precedent — so
//! the identity suites are in-crate `#[cfg(test)]` modules rather than
//! `tests/` binaries. The public-API correctness battery lives in
//! `tests/i4_incremental_oracle.rs`.
//!
//! Every expected value is hand-derived from the byte layout of the
//! literal source plus the frozen contracts
//! (`docs/research/horse-a-v1-algorithm.md`,
//! `docs/research/horse-a-logical-data-model/`, PR #63's observed seam) —
//! never read back from the implementation.

mod pipeline;
mod semantics;
mod structure;
mod validation;
mod walk;

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_oracle::normalized::NormalizedDocument;
use markit_mdbench_oracle::{validate_normalized, NormalizeV1};
use markit_mdbench_shared_grammar::parse_full;

use crate::full_build::full_build;
use crate::state::ReadyDocument;
use crate::update::{commit, stage, StagedUpdate};

/// One update fixture: the READY state built for a literal source plus
/// that source (the update's `old_source` association).
pub(crate) struct Fixture {
    pub(crate) old: ReadyDocument,
    pub(crate) old_source: Source,
}

impl Fixture {
    /// Full-build the independent initial READY state (spec §18 excludes
    /// this whole-document construction from primary update work).
    pub(crate) fn new(text: &str) -> Self {
        let old_source = Source::new(SourceId(1), text);
        let old = full_build(&old_source, &mut NoopWorkSink).expect("initial full build");
        Self { old, old_source }
    }

    /// The post source the canonical edit contract materializes (runner
    /// side, strictly outside every mechanism timer).
    pub(crate) fn post(&self, edit: &CanonicalEdit, id: u64) -> Source {
        edit.apply(&self.old_source, SourceId(id))
            .expect("the edit applies to the old source")
    }

    /// Stage one update, handing back the staging record and the post
    /// source (the old READY state stays untouched and borrowable).
    pub(crate) fn staged(&self, edit: &CanonicalEdit) -> (StagedUpdate, Source) {
        let post = self.post(edit, 2);
        let staged = stage(&self.old, &self.old_source, &post, edit, &mut NoopWorkSink)
            .expect("the update stages");
        (staged, post)
    }

    /// Run the complete I4 update (stage + commit) and hand back the READY
    /// result plus the post source.
    pub(crate) fn run(self, edit: &CanonicalEdit) -> (ReadyDocument, Source) {
        let post = self.post(edit, 2);
        let staged = stage(&self.old, &self.old_source, &post, edit, &mut NoopWorkSink)
            .expect("the update stages");
        let next = commit(self.old, staged);
        (next, post)
    }
}

/// An edit from literal geometry.
pub(crate) fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

/// The H0 clean-parse reference result (the correctness-lane authority,
/// replicated exactly as `markit-mdbench-full-rebuild::parse_document`
/// defines it).
pub(crate) fn h0(text: &str) -> NormalizedDocument {
    let bytes = text.as_bytes();
    let mut noop = NoopWorkSink;
    let document = parse_full(bytes, &mut noop);
    if !bytes.is_empty() {
        validate_normalized(&document, Some(bytes))
            .expect("H0 result violates NORMALIZED-RESULT-v1");
    }
    document
}

/// The frozen correctness oracle (spec §19; I4 task contract §26): the
/// returned state's normalized export equals clean H0 parsing of the new
/// source — full structural equality, never a hash — and the state passes
/// every READY invariant.
pub(crate) fn assert_ready_equals_h0(doc: &ReadyDocument, source: &Source) {
    crate::validate::validate_ready(doc).expect("READY invariants after the update");
    let exported = doc.normalize_v1();
    if !source.as_str().is_empty() {
        validate_normalized(&exported, Some(source.as_bytes()))
            .expect("the updated export violates NORMALIZED-RESULT-V1");
    }
    assert_eq!(
        exported,
        h0(source.as_str()),
        "normalize(Horse-A update) != normalize(H0 clean parse)"
    );
}
