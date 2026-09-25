//! Shared I2 test helpers. Every expected value in the I2 suites is
//! hand-derived from the byte layout of the literal source plus the
//! frozen contracts (`docs/research/horse-a-v1-algorithm.md`,
//! `docs/research/horse-a-logical-data-model/`, PR #63's observed seam) —
//! never read back from the implementation under test.

// Each test binary links this module; individual binaries use different
// subsets of the helpers.
#![allow(dead_code)]

use markit_mdbench_common::{NoopWorkSink, Source, SourceId};
use markit_mdbench_horse_a::{full_build, ReadyDocument};
use markit_mdbench_oracle::{validate_normalized, NormalizeV1};
use markit_mdbench_shared_grammar::parse_full;

/// The H0 clean-parse reference result (the correctness-lane authority,
/// replicated exactly as `markit-mdbench-full-rebuild::parse_document`
/// defines it: shared `parse_full` + the NORMALIZED-RESULT-v1 gate).
///
/// The conformance gate is skipped for the EMPTY source only: the
/// validator's zero-length rule applies to ordinary non-Document nodes,
/// while the empty document's `Document [0,0)` is fixed by the
/// `Document == [0, len)` rule (frozen clarification, logical-data-model
/// §3.3). No campaign source is empty, so H0's own gate never meets this.
pub fn h0(src: &[u8]) -> markit_mdbench_oracle::normalized::NormalizedDocument {
    let mut noop = NoopWorkSink;
    let document = parse_full(src, &mut noop);
    if !src.is_empty() {
        validate_normalized(&document, Some(src)).expect("H0 result violates NORMALIZED-RESULT-v1");
    }
    document
}

/// Full-build a READY document from literal bytes.
pub fn build(src: &[u8]) -> ReadyDocument {
    let text = String::from_utf8(src.to_vec()).expect("test sources are UTF-8");
    let source = Source::new(SourceId(1), text);
    full_build(&source, &mut NoopWorkSink).expect("I2 full build must succeed on test sources")
}

/// Full-build a READY document carrying an explicit source identity.
pub fn build_with_id(id: SourceId, src: &[u8]) -> ReadyDocument {
    let text = String::from_utf8(src.to_vec()).expect("test sources are UTF-8");
    let source = Source::new(id, text);
    full_build(&source, &mut NoopWorkSink).expect("I2 full build must succeed on test sources")
}

/// The H0 equality gate: full normalized structural equality (never a
/// hash), with the exported result also passing the shared conformance
/// gate.
pub fn assert_export_equals_h0(src: &[u8]) -> ReadyDocument {
    let doc = build(src);
    let exported = doc.normalize_v1();
    if !src.is_empty() {
        validate_normalized(&exported, Some(src))
            .expect("Horse-A export violates NORMALIZED-RESULT-v1");
    }
    assert_eq!(
        exported,
        h0(src),
        "normalize(Horse-A full build) != normalize(H0 clean parse) for {src:?}"
    );
    doc
}
