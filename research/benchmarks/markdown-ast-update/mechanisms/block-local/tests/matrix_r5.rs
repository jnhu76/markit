//! Frozen 370-slot CASE-MATRIX-v1 differential for H1 BLOCK_LOCAL_REPARSE (stage R5).
//!
//! Instantiates the frozen case matrix (`cases/CASE-MATRIX-v1.md`) and
//! runs every UNIQUE case through block_local against the H0 authoritative
//! parse: `H1 result == H0` on every update, on the FULL_PARSE
//! baseline, and on the frozen QUERY batch. The case-slot expansion and
//! the 7 declared slot overlaps follow CASE-MATRIX-v1 section 6 exactly;
//! the per-recipe Block-D counts are checked against the frozen table.
//! `#[ignore]`d in the debug profile; `verify-r5.sh` runs it with
//! `--release`. No timing, no benchmark data — correctness only.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use gen::{PayloadShape, ALL_SHAPES, SIZE_16M, SIZE_1M, SIZE_64K};
use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Source, WorkCounters,
};
use markit_mdbench_corpusgen as gen;
use markit_mdbench_corpusgen::mutations::{
    apply, generic_edit, query_anchors, structural_edit_for, validate, AnchorClass, EditSize,
    GenericOp, PlannedEdit, Recipe,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{node_path_at, normalized_checksum, NodeKind};
use markit_mdbench_oracle::validate_root;

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn full_parse_state(bytes: &[u8]) -> H1State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = BlockLocalMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 1), &mut cx)
        .expect("full_parse");
    mech.complete(pending).expect("complete").state
}

fn completed_doc(
    bytes: &[u8],
    context: &str,
) -> markit_mdbench_oracle::normalized::NormalizedDocument {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = BlockLocalMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 2), &mut cx)
        .unwrap_or_else(|e| panic!("{context}: full_parse failed: {e:?}"));
    let doc = pending.result().clone();
    let done = mech.complete(pending).expect("complete");
    assert_eq!(done.result_checksum, normalized_checksum(&doc), "{context}");
    doc
}

static CASE_NO: AtomicUsize = AtomicUsize::new(0);

/// Per-case progress line (stderr; visible with `--nocapture`): the
/// matrix is ONE `#[test]`, and without this the gate log is silent for
/// hours at a time. Operational observability only — no measurement
/// content, no case semantics.
fn matrix_progress(context: &str) {
    let n = CASE_NO.fetch_add(1, Ordering::Relaxed) + 1;
    eprintln!("[matrix case {n:03}] {context}");
}

fn run_full_parse_case(bytes: &[u8], context: &str) {
    matrix_progress(context);
    let doc = completed_doc(bytes, context);
    let clean = parse_document(bytes);
    assert_eq!(&doc, &clean, "{context}: structural mismatch vs H0");
    assert_eq!(
        normalized_checksum(&doc),
        normalized_checksum(&clean),
        "{context}: checksum mismatch vs H0"
    );
    validate_root(&doc.root, Some(bytes))
        .unwrap_or_else(|e| panic!("{context}: NORMALIZED-RESULT-v1 violation: {e}"));
}

fn run_query_case(bytes: &[u8], context: &str) {
    matrix_progress(context);
    let doc = completed_doc(bytes, context);
    let clean = parse_document(bytes);
    let [early, middle, late] = query_anchors(bytes);
    for (offset, label) in [(early, "EARLY"), (middle, "MIDDLE"), (late, "LATE")] {
        let path = node_path_at(&doc, offset);
        assert!(!path.is_empty(), "{context} {label}: empty path");
        assert_eq!(path[0].kind, NodeKind::Document, "{context} {label}");
        for pn in &path {
            assert!(
                (pn.start <= offset && offset < pn.end) || offset == bytes.len(),
                "{context} {label}: containment"
            );
        }
        assert_eq!(
            path,
            node_path_at(&clean, offset),
            "{context} {label}: must equal the H0 answer"
        );
    }
}

fn run_update_case(old: &[u8], edit: &PlannedEdit, context: &str) {
    matrix_progress(context);
    let post = apply(old, edit);
    validate(old, edit, &post).expect("frozen mutation invariants");
    let canonical = CanonicalEdit::new(
        edit.start,
        edit.end,
        String::from_utf8(edit.inserted.clone()).expect("insertion is UTF-8"),
    )
    .expect("edit in range");
    let old_state = full_parse_state(old);
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
        let old_source = source_of(old, 10);
        let post_source = source_of(&post, 11);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &canonical, &old_state, &mut cx)
            .unwrap_or_else(|e| panic!("{context}: prepare_update failed: {e:?}"));
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &canonical,
                old_state,
                prepared,
                &mut cx,
            )
            .unwrap_or_else(|e| panic!("{context}: update failed: {e:?}"));
        let clean = parse_document(&post);
        assert_eq!(
            pending.result(),
            &clean,
            "{context}: structural mismatch vs H0"
        );
        assert_eq!(
            normalized_checksum(pending.result()),
            normalized_checksum(&clean),
            "{context}: checksum mismatch vs H0"
        );
        validate_root(&pending.result().root, Some(&post))
            .unwrap_or_else(|e| panic!("{context}: NORMALIZED-RESULT-v1 violation: {e}"));
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.result_checksum,
            normalized_checksum(&clean),
            "{context}"
        );
        new_state = done.state;
    }
    // The new retained state must be a usable input for a follow-up
    // update: re-parse coherence is checked by reusing it on a trivial
    // self-edit (no-op replace with the same bytes is NOT valid — instead
    // verify the state round-trips through another full differential on
    // a tiny tail insert).
    let tail = CanonicalEdit::new(post.len(), post.len(), "+".to_string()).expect("edit");
    let post2 = {
        let mut v = post.clone();
        v.extend_from_slice(b"+");
        v
    };
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
        let old_source = source_of(&post, 12);
        let post_source = source_of(&post2, 13);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &tail, &new_state, &mut cx)
            .unwrap_or_else(|e| panic!("{context}: chained prepare failed: {e:?}"));
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &tail,
                new_state,
                prepared,
                &mut cx,
            )
            .unwrap_or_else(|e| panic!("{context}: chained update failed: {e:?}"));
        let clean = parse_document(&post2);
        assert_eq!(
            pending.result(),
            &clean,
            "{context}: chained mismatch vs H0"
        );
        mech.complete(pending).expect("complete");
    }
}

/// CASE-MATRIX-v1 section 6: the core INSERT-TINY-MIDDLE slot coincides
/// with a generic-grid slot (1m corpora) or a scaling-slice slot (64k /
/// 16m corpora) for these shapes — one case, counted once.
fn core_slot_is_shared(shape: PayloadShape, size: usize) -> bool {
    (size == SIZE_1M
        && matches!(
            shape,
            PayloadShape::Mixed | PayloadShape::Plain | PayloadShape::FenceHeavy
        ))
        || (matches!(shape, PayloadShape::ManyBlocks | PayloadShape::HugeBlock)
            && (size == SIZE_64K || size == SIZE_16M))
}

#[test]
#[ignore = "full 370-slot matrix; verify-r5.sh runs it with --release"]
fn r5_case_matrix_370_slots_differential() {
    let mut recipe_counts: BTreeMap<&'static str, usize> = gen::mutations::ALL_RECIPES
        .iter()
        .map(|r| (r.id(), 0usize))
        .collect();
    let mut full_parse_cases = 0usize;
    let mut query_cases = 0usize;
    let mut update_cases = 0usize;

    for &size in &[SIZE_64K, SIZE_1M, SIZE_16M] {
        for shape in ALL_SHAPES {
            let src = gen::generate(shape, size)
                .unwrap_or_else(|e| panic!("generate {shape:?}-{}: {e}", gen::size_label(size)));
            let base = format!("{shape:?}-{}", gen::size_label(size));

            // Block A1 — FULL_PARSE baseline (24 cases).
            run_full_parse_case(&src, &format!("{base} FULL_PARSE"));
            full_parse_cases += 1;

            // Block A2 — QUERY ordered batch (24 cases).
            run_query_case(&src, &format!("{base} QUERY"));
            query_cases += 1;

            // Block B — generic edit grid: three payloads @ 1m (135 cases).
            if size == SIZE_1M
                && matches!(
                    shape,
                    PayloadShape::Mixed | PayloadShape::Plain | PayloadShape::FenceHeavy
                )
            {
                for op in [
                    GenericOp::Insert,
                    GenericOp::Delete,
                    GenericOp::ReplaceEq,
                    GenericOp::ReplaceGrow,
                    GenericOp::ReplaceShrink,
                ] {
                    for es in [EditSize::Tiny, EditSize::Small, EditSize::Medium] {
                        for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
                            let edit = generic_edit(&src, class, es, op);
                            let context = format!("{base} grid {op:?}-{es:?}-{class:?}");
                            run_update_case(&src, &edit, &context);
                            update_cases += 1;
                        }
                    }
                }
            }

            // Block C — scaling slice (12 cases).
            if matches!(shape, PayloadShape::ManyBlocks | PayloadShape::HugeBlock)
                && (size == SIZE_64K || size == SIZE_16M)
            {
                for (class, es, op) in [
                    (AnchorClass::Early, EditSize::Tiny, GenericOp::Insert),
                    (AnchorClass::Middle, EditSize::Tiny, GenericOp::Insert),
                    (AnchorClass::Middle, EditSize::Medium, GenericOp::ReplaceEq),
                ] {
                    let edit = generic_edit(&src, class, es, op);
                    let context = format!("{base} scaling {op:?}-{es:?}-{class:?}");
                    run_update_case(&src, &edit, &context);
                    update_cases += 1;
                }
            }

            // Block A3 — core INSERT-TINY-MIDDLE (24 slots, 7 shared).
            if !core_slot_is_shared(shape, size) {
                let edit =
                    generic_edit(&src, AnchorClass::Middle, EditSize::Tiny, GenericOp::Insert);
                run_update_case(&src, &edit, &format!("{base} core INSERT-TINY-MIDDLE"));
                update_cases += 1;
            }

            // Block D — structural targeted matrix (158 slots).
            for recipe in gen::mutations::ALL_RECIPES {
                let anchors: &[AnchorClass] = if recipe == Recipe::FsFenceOpen {
                    &[AnchorClass::Early, AnchorClass::Middle]
                } else {
                    &[recipe.selection_anchor()]
                };
                for &class in anchors {
                    let Ok(edit) = structural_edit_for(&src, recipe, class, shape, size) else {
                        continue; // NOT_APPLICABLE slot (declared by the matrix)
                    };
                    *recipe_counts.get_mut(recipe.id()).expect("known recipe") += 1;
                    let context = format!("{base} {} {class:?}", recipe.id());
                    run_update_case(&src, &edit, &context);
                    update_cases += 1;
                }
            }
        }
    }

    // Frozen totals (CASE-MATRIX-v1 section 6): 370 unique cases.
    assert_eq!(full_parse_cases, 24, "Block A1");
    assert_eq!(query_cases, 24, "Block A2");
    assert_eq!(
        update_cases, 322,
        "updates: 135 grid + 12 scaling + 17 core-unique + 158 structural"
    );
    assert_eq!(
        full_parse_cases + query_cases + update_cases,
        370,
        "frozen unique case total"
    );
    const FROZEN: [(&str, usize); 13] = [
        ("M-LOC-TEXT", 12),
        ("M-LOC-UTF8-SWAP", 21),
        ("M-BB-PARA-SPLIT", 15),
        ("M-BB-PARA-MERGE", 17),
        ("M-CS-ITEM-INDENT", 6),
        ("M-CS-BQ-NEST-LINE", 6),
        ("M-FS-FENCE-OPEN", 42),
        ("M-FS-FENCE-CLOSE", 6),
        ("M-IDS-EMPH-INSERT", 6),
        ("M-IDS-CODE-DELIM", 6),
        ("M-IDS-LINK-DELIM", 9),
        ("M-SD-DEF-REPLACE", 6),
        ("M-SD-DEF-DELETE", 6),
    ];
    for (id, count) in FROZEN {
        assert_eq!(
            recipe_counts.get(id).copied().unwrap_or(0),
            count,
            "Block-D slot count for {id}"
        );
    }
}
