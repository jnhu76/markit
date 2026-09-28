//! #62 final closure chain — C1/C2/C3 over the EXACT frozen #60 primary
//! cells, as one unbroken retained-state chain through the public
//! incremental API (CORRECTNESS_AND_CONFORMANCE_ONLY — **NOT**
//! collection).
//!
//! Complements `frozen60` (which already adjudicates the primary update's
//! normalized equality, READY, exact geometry and frozen bounds through
//! the staged commit path) with the two closure obligations it does not
//! encode:
//!
//! - **C1** — the pre-edit Horse-A full build itself:
//!   `normalize(Horse-A full-build(pre)) == normalize(H0 clean parse(pre))`
//!   (full normalized structural equality, never a hash) plus the READY
//!   validator on that pre state;
//! - **C2** — the primary incremental update consumes **the C1 state**
//!   (no rebuild in between) through the public `update` lane, and the
//!   returned state is READY and equals `normalize(H0 clean parse(post))`;
//! - **C3** — READY continuation on **the exact state C2 returned**: the
//!   canonical restore edit `[edit_start, edit_start + 8) -> ""` with the
//!   restored source byte-identical to the original frozen pre source is
//!   applied as a second real incremental update, and the result is READY
//!   and equals `normalize(H0 clean parse(pre))`. No rebuild, no clone,
//!   no serialization round-trip, no fresh full build between edits.
//!
//! The construction and the primary edit are the frozen #60 authorities
//! reused verbatim from `frozen60` (the frozen corpus generator + the
//! `target * 128 + 63` geometry); every byte identity is PROVEN against
//! the frozen anchor hashes, never assumed. This module reads no
//! structural counters, writes no treatment JSON, stores no timing and
//! compares no latency. STRUCTURAL_COLLECTION_RUN = NO;
//! PERFORMANCE_COLLECTION_RUN = NO.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_shared_grammar::parse_full;

use crate::full_build::full_build;
use crate::i5_tests::frozen60::{frozen_edit, frozen_pre_source};
use crate::structural::NoopHorseAStructuralSink;
use crate::update::update;
use crate::validate::validate_ready;
use crate::NormalizeV1;

/// The frozen 8-byte inserted run (#60 §9.1).
const INSERTED: &str = "zzzzzzzz";
/// sha256("zzzzzzzz") — the frozen `edit_sha256` of every cell.
const INSERTED_SHA256: &str = "c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429";

/// One frozen primary cell's closure identity (#60 §9.1): the byte
/// identities the chain must reproduce exactly.
struct Cell {
    label: &'static str,
    /// N in bytes (a multiple of 128).
    n: usize,
    /// Frozen `pre_sha256` (byte identity gate).
    pre_sha256: &'static str,
    /// Frozen `post_sha256` (byte identity gate).
    post_sha256: &'static str,
}

const CELLS: &[Cell] = &[
    Cell {
        label: "128 KiB",
        n: 131_072,
        pre_sha256: "58b39c389bfe5d5cbcfdbbd74150d276c7996e1b25a84a467fa4deb3cec29a30",
        post_sha256: "1ca6bb1f138507758230ba868c6a55bb9bc824b94fe0b9c75ff3d39c8fb777ea",
    },
    Cell {
        label: "1 MiB",
        n: 1_048_576,
        pre_sha256: "681c2c032bd1585deef29cf2ce9f9c2a3b8c4bf163ca54c8001d45309462b294",
        post_sha256: "e3e0d2ddf4717cc9bbd0e60848ada7b542500d143134b39609cd2537da8c174c",
    },
    Cell {
        label: "16 MiB",
        n: 16_777_216,
        pre_sha256: "0fdca64e71df4386d4407afa1dd5e72aa0faaf60d6854a1fab730d580be6c78d",
        post_sha256: "c8014b5ecae8ca489cb083e6a1a1f8170b3fd2d8d115a85ffa4c58ab4bb29eb0",
    },
];

/// The H0 clean-parse reference, computed independently of Horse-A.
fn h0(bytes: &[u8]) -> markit_mdbench_oracle::normalized::NormalizedDocument {
    parse_full(bytes, &mut NoopWorkSink)
}

/// Gate one state: READY invariants, NORMALIZED-RESULT-v1 validity of the
/// export, then FULL normalized structural equality against the H0 clean
/// parse of the same bytes (never a hash, never selected fields).
fn assert_ready_and_equals_h0(cell: &str, gate: &str, doc: &crate::ReadyDocument, bytes: &[u8]) {
    validate_ready(doc).unwrap_or_else(|err| panic!("{cell}: {gate}: READY invariants: {err}"));
    let exported = doc.normalize_v1();
    validate_normalized(&exported, Some(bytes)).unwrap_or_else(|err| {
        panic!("{cell}: {gate}: export violates NORMALIZED-RESULT-v1: {err}")
    });
    assert_eq!(
        exported,
        h0(bytes),
        "{cell}: {gate}: normalize(Horse-A) != normalize(H0 clean parse)"
    );
}

/// Run the complete unbroken C1 → C2 → C3 chain over one frozen cell.
fn run_chain(cell: &Cell) {
    let cell_id = cell.label;

    // ---- frozen byte identities (proven, never assumed) ----
    let pre = Source::new(SourceId(1), frozen_pre_source(cell.n));
    assert_eq!(
        pre.sha256_hex(),
        cell.pre_sha256,
        "{cell_id}: pre source is not byte-identical to the frozen anchor"
    );
    assert_eq!(
        Source::new(SourceId(0), INSERTED).sha256_hex(),
        INSERTED_SHA256,
        "{cell_id}: the inserted text is not the frozen 8-byte run"
    );

    // ---- C1: the pre-edit Horse-A full build ----
    let a_pre = full_build(&pre, &mut NoopWorkSink, &mut NoopHorseAStructuralSink)
        .unwrap_or_else(|err| panic!("{cell_id}: C1: full build of the frozen pre source: {err}"));
    assert_ready_and_equals_h0(cell_id, "C1 (pre full build)", &a_pre, pre.as_bytes());

    // ---- C2: the primary incremental update FROM THE C1 STATE ----
    // `update` consumes `a_pre` by value: there is no rebuild, clone or
    // reconstruction between C1 and C2.
    let edit = frozen_edit(cell.n);
    let post = edit
        .apply(&pre, SourceId(2))
        .unwrap_or_else(|err| panic!("{cell_id}: C2: the frozen edit applies: {err}"));
    assert_eq!(
        post.sha256_hex(),
        cell.post_sha256,
        "{cell_id}: C2: post source is not byte-identical to the frozen anchor"
    );
    let a_post = update(a_pre, &pre, &post, &edit, &mut NoopWorkSink)
        .unwrap_or_else(|err| panic!("{cell_id}: C2: incremental update: {err}"));
    assert_ready_and_equals_h0(cell_id, "C2 (primary update)", &a_post, post.as_bytes());

    // ---- C3: the restore edit ON THE EXACT STATE C2 RETURNED ----
    // Delete exactly the inserted eight bytes: [edit_start, edit_start+8)
    // -> "", new source = the original frozen pre bytes (fresh identity,
    // proven byte-identical). `update` consumes `a_post` by value: no
    // rebuild, no clone, no reload, no second full build between edits.
    let restore_start = edit.start_byte() as usize;
    let restore = CanonicalEdit::new(restore_start, restore_start + INSERTED.len(), "")
        .expect("C3: restore edit geometry");
    let restored = restore
        .apply(&post, SourceId(3))
        .unwrap_or_else(|err| panic!("{cell_id}: C3: the restore edit applies: {err}"));
    assert_eq!(
        restored.sha256_hex(),
        cell.pre_sha256,
        "{cell_id}: C3: restored source is not byte-identical to the frozen pre source"
    );
    assert_eq!(
        restored.as_bytes(),
        pre.as_bytes(),
        "{cell_id}: C3: restored bytes differ from the original frozen pre bytes"
    );
    let a_restored = update(a_post, &post, &restored, &restore, &mut NoopWorkSink)
        .unwrap_or_else(|err| panic!("{cell_id}: C3: restore incremental update: {err}"));
    assert_ready_and_equals_h0(
        cell_id,
        "C3 (restore update)",
        &a_restored,
        restored.as_bytes(),
    );
}

/// The 128 KiB frozen cell closure chain.
#[test]
fn c123_chain_128k() {
    run_chain(&CELLS[0]);
}

/// The 1 MiB frozen cell closure chain.
#[test]
fn c123_chain_1m() {
    run_chain(&CELLS[1]);
}

/// The 16 MiB frozen cell closure chain.
#[test]
fn c123_chain_16m() {
    run_chain(&CELLS[2]);
}
