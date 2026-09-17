//! H2 FRAGMENT_REUSE gate suite (stage R5, task contract §10).
//!
//! Gates proven here, per the frozen H2 gate list:
//!
//! - `H2_GRAMMAR_PASS` — the 43 golden fixtures: H2's clean parse equals
//!   the authoritative result and the retained tree covers the document;
//! - `H2_DIFFERENTIAL_PASS` — corpus grid, structural recipes, edit
//!   chains, and hand-built fragment-lifecycle probes:
//!   `H2 update result == H0 clean authoritative parse`;
//! - `H2_STRUCTURAL_PASS` — post-update states satisfy the
//!   NORMALIZED-RESULT-v1 invariants (shared validation gate);
//! - `H2_COUNTER_PASS` — the frozen applicability matrix for H2
//!   (fallback/restart/convergence NotApplicable; honest measured zeros);
//! - `H2_REUSE_PASS` — reuse happens exactly under the frozen rule
//!   (alignment + vouching + window) and degrades naturally elsewhere;
//! - `H2_IDENTITY_PASS` — witnesses W1 (safe local edit: a fragment
//!   survives, nodes_reused > 0, sub-full inspection) and W2 (a
//!   context-changing edit refuses reuse where vouching differs);
//! - `H2_EAGER_COMPLETION_PASS` — the pending already holds the complete
//!   state and result; complete() seals only.

use gen::{PayloadShape, ALL_SHAPES, SIZE_16M, SIZE_1M, SIZE_64K};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, WorkCounters,
};
use markit_mdbench_corpusgen as gen;
use markit_mdbench_corpusgen::mutations::{
    generic_edit, structural_edit_for, AnchorClass, EditSize, GenericOp,
};
use markit_mdbench_fragment_reuse as h2;
use markit_mdbench_fragment_reuse::{FragmentReuseMechanism, H2State};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::fixture::{load_fixtures, repo_fixture_dir};
use markit_mdbench_oracle::normalized::{node_path_at, normalized_checksum, NodeKind};
use markit_mdbench_oracle::validate_root;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn canon(edit: &gen::mutations::PlannedEdit) -> CanonicalEdit {
    CanonicalEdit::new(
        edit.start,
        edit.end,
        String::from_utf8(edit.inserted.clone()).expect("insertion is UTF-8"),
    )
    .expect("edit in range")
}

fn h2_full_parse(bytes: &[u8]) -> H2State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = FragmentReuseMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 1), &mut cx)
        .expect("full_parse");
    mech.complete(pending).expect("complete").state
}

/// The frozen differential gate for one H2 update: the pending's already
/// materialized result must equal the H0 clean authoritative parse of the
/// post source, structurally and by checksum.
fn assert_update_structural(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H2State,
    context: &str,
) -> (H2State, WorkCounters) {
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old, 10);
        let post_source = source_of(post, 11);
        let prepared = mech
            .prepare_update(&old_source, &post_source, edit, &old_state, &mut cx)
            .expect("prepare_update");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        let clean = parse_document(post);
        assert_eq!(pending.result(), &clean, "structural mismatch ({context})");
        assert_eq!(
            normalized_checksum(pending.result()),
            normalized_checksum(&clean),
            "checksum mismatch ({context})"
        );
        let done = mech.complete(pending).expect("complete");
        assert_eq!(done.result_checksum, normalized_checksum(&clean));
        new_state = done.state;
    }
    (new_state, counters)
}

fn assert_tree_integrity(state: &H2State, src: &[u8], context: &str) {
    // The retained tree must cover the document: slots (with their gaps)
    // tile [0, len); every node's children lie inside its own span.
    let tree = state.tree();
    let mut cursor = 0usize;
    for slot in tree.slots.iter() {
        let start = cursor + slot.gap;
        assert!(start >= cursor);
        cursor = start + slot.node.size;
    }
    // Slots cover every block; a trailing blank gap needs no slot (the
    // Document span carries the full length).
    assert!(
        cursor <= src.len(),
        "{context}: slots must not exceed the document"
    );
    fn walk(node: &h2::FNode, base: usize) {
        assert!(node.size > 0, "nodes are non-degenerate");
        let mut child_cursor = 0usize;
        for (rel, child) in &node.children {
            assert!(*rel >= child_cursor, "children ordered");
            walk(child, base + rel);
            child_cursor = rel + child.size;
        }
        assert!(child_cursor <= node.size, "children inside the parent span");
    }
    for slot in tree.slots.iter() {
        walk(&slot.node, 0);
    }
}

fn validate_tree(
    doc: &markit_mdbench_oracle::normalized::NormalizedDocument,
    src: &[u8],
    context: &str,
) {
    if let Err(e) = validate_root(&doc.root, Some(src)) {
        panic!("normalized tree violates NORMALIZED-RESULT-v1 ({context}): {e}");
    }
}

// ---------------------------------------------------------------------------
// H2_GRAMMAR_PASS — fixtures
// ---------------------------------------------------------------------------

#[test]
fn h2_grammar_pass_all_43_fixtures() {
    let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load and validate");
    assert_eq!(fixtures.len(), 43, "the frozen fixture count is 43");
    let mut failed = Vec::new();
    for f in &fixtures {
        let src = f.source.as_bytes();
        let state = h2_full_parse(src);
        assert_tree_integrity(&state, src, &f.id);
        let mut counters = WorkCounters::all_unknown();
        let result = {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = FragmentReuseMechanism::new();
            let pending = mech
                .full_parse(&source_of(src, 2), &mut cx)
                .expect("full_parse");
            let r = pending.result().clone();
            mech.complete(pending).expect("complete");
            r
        };
        let clean = parse_document(src);
        if result != clean {
            failed.push(format!("{}: structural mismatch", f.id));
        }
        validate_tree(&result, src, &f.id);
    }
    assert!(
        failed.is_empty(),
        "fixtures FAILED ({}/43 passed):\n  {}",
        43 - failed.len(),
        failed.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// H2_DIFFERENTIAL_PASS — corpus grid + recipes + chains
// ---------------------------------------------------------------------------

fn generic_grid_case(
    shape: PayloadShape,
    size: usize,
    class: AnchorClass,
    es: EditSize,
    op: GenericOp,
) {
    let old = gen::generate(shape, size).expect("generate");
    let old_state = h2_full_parse(&old);
    assert_tree_integrity(&old_state, &old, "grid old state");

    let edit = generic_edit(&old, class, es, op);
    let post = gen::mutations::apply(&old, &edit);
    gen::mutations::validate(&old, &edit, &post).expect("frozen mutation invariants");

    let context = format!(
        "{:?}-{} {:?} {:?} {:?} @{}",
        shape,
        gen::size_label(size),
        class,
        es,
        op,
        edit.start
    );
    let (new_state, _) = assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
    let clean = parse_document(&post);
    validate_tree(&clean, &post, &context);
    assert_tree_integrity(&new_state, &post, &context);
}

#[test]
fn h2_update_matches_clean_parse_generic_grid_64k() {
    for shape in [
        PayloadShape::Plain,
        PayloadShape::FenceHeavy,
        PayloadShape::Mixed,
    ] {
        for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
            for es in [EditSize::Tiny, EditSize::Small, EditSize::Medium] {
                for op in [
                    GenericOp::Insert,
                    GenericOp::Delete,
                    GenericOp::ReplaceEq,
                    GenericOp::ReplaceGrow,
                    GenericOp::ReplaceShrink,
                ] {
                    generic_grid_case(shape, SIZE_64K, class, es, op);
                }
            }
        }
    }
}

#[test]
fn h2_update_matches_clean_parse_every_shape_insert_tiny_all_anchors() {
    for shape in ALL_SHAPES {
        for size in [SIZE_64K, SIZE_1M] {
            for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
                generic_grid_case(shape, size, class, EditSize::Tiny, GenericOp::Insert);
            }
        }
    }
}

#[test]
fn h2_update_matches_clean_parse_scaling_slice() {
    for shape in [PayloadShape::ManyBlocks, PayloadShape::HugeBlock] {
        for size in [SIZE_64K, SIZE_16M] {
            generic_grid_case(
                shape,
                size,
                AnchorClass::Early,
                EditSize::Tiny,
                GenericOp::Insert,
            );
            generic_grid_case(
                shape,
                size,
                AnchorClass::Middle,
                EditSize::Tiny,
                GenericOp::Delete,
            );
            generic_grid_case(
                shape,
                size,
                AnchorClass::Middle,
                EditSize::Medium,
                GenericOp::ReplaceEq,
            );
        }
    }
}

#[test]
fn h2_update_matches_clean_parse_multibyte_sensitive_ops() {
    for shape in [
        PayloadShape::Plain,
        PayloadShape::ManyBlocks,
        PayloadShape::HugeBlock,
        PayloadShape::DeepContainer,
        PayloadShape::InlineDense,
        PayloadShape::FenceHeavy,
        PayloadShape::ReferenceFanout,
    ] {
        for op in [GenericOp::Delete, GenericOp::ReplaceEq, GenericOp::Insert] {
            generic_grid_case(shape, SIZE_64K, AnchorClass::Early, EditSize::Tiny, op);
        }
    }
}

#[test]
fn h2_update_matches_clean_parse_structural_recipes_64k() {
    for shape in ALL_SHAPES {
        let old = gen::generate(shape, SIZE_64K).expect("generate");
        for recipe in gen::mutations::ALL_RECIPES {
            let anchors: &[AnchorClass] = if recipe == gen::mutations::Recipe::FsFenceOpen {
                &[AnchorClass::Early, AnchorClass::Middle]
            } else {
                &[recipe.selection_anchor()]
            };
            for &class in anchors {
                let Ok(edit) = structural_edit_for(&old, recipe, class, shape, SIZE_64K) else {
                    continue; // declared NOT_APPLICABLE for this shape
                };
                let post = gen::mutations::apply(&old, &edit);
                gen::mutations::validate(&old, &edit, &post).expect("frozen mutation invariants");
                let old_state = h2_full_parse(&old);
                let context = format!("{shape:?}-64k {} {class:?}", recipe.id());
                let (new_state, _) =
                    assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
                assert_tree_integrity(&new_state, &post, &context);
            }
        }
    }
}

#[test]
fn h2_update_matches_clean_parse_structural_recipes_1m() {
    for (shape, recipes) in [
        (
            PayloadShape::ReferenceFanout,
            &[
                gen::mutations::Recipe::SdDefReplace,
                gen::mutations::Recipe::SdDefDelete,
                gen::mutations::Recipe::IdsLinkDelim,
            ] as &[gen::mutations::Recipe],
        ),
        (
            PayloadShape::FenceHeavy,
            &[
                gen::mutations::Recipe::FsFenceClose,
                gen::mutations::Recipe::LocUtf8Swap,
            ] as &[gen::mutations::Recipe],
        ),
        (
            PayloadShape::Mixed,
            &[
                gen::mutations::Recipe::SdDefReplace,
                gen::mutations::Recipe::FsFenceClose,
                gen::mutations::Recipe::CsItemIndent,
            ] as &[gen::mutations::Recipe],
        ),
        (
            PayloadShape::DeepContainer,
            &[
                gen::mutations::Recipe::CsBqNestLine,
                gen::mutations::Recipe::BbParaSplit,
            ] as &[gen::mutations::Recipe],
        ),
    ] {
        let old = gen::generate(shape, SIZE_1M).expect("generate");
        for &recipe in recipes {
            let edit = structural_edit_for(&old, recipe, recipe.selection_anchor(), shape, SIZE_1M)
                .unwrap_or_else(|na| panic!("{} on {shape:?}-1m: {na:?}", recipe.id()));
            let post = gen::mutations::apply(&old, &edit);
            let old_state = h2_full_parse(&old);
            let context = format!("{shape:?}-1m {}", recipe.id());
            assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
        }
    }
}

#[test]
fn h2_chained_edits_stay_equal_to_clean_parses() {
    let mut current = gen::generate(PayloadShape::Plain, SIZE_64K).expect("generate");
    let mut state = h2_full_parse(&current);
    let ops = [
        (AnchorClass::Early, EditSize::Tiny, GenericOp::Insert),
        (AnchorClass::Middle, EditSize::Small, GenericOp::ReplaceGrow),
        (AnchorClass::Late, EditSize::Tiny, GenericOp::Delete),
        (AnchorClass::Middle, EditSize::Tiny, GenericOp::Insert),
        (AnchorClass::Late, EditSize::Small, GenericOp::ReplaceShrink),
    ];
    for (i, (class, es, op)) in ops.into_iter().enumerate() {
        let edit = generic_edit(&current, class, es, op);
        let post = gen::mutations::apply(&current, &edit);
        let context = format!("chain step {i}");
        let (next, _) = assert_update_structural(&current, &post, &canon(&edit), state, &context);
        assert_tree_integrity(&next, &post, &context);
        state = next;
        current = post;
    }
}

// ---------------------------------------------------------------------------
// H2_REUSE_PASS — fragment lifecycle probes
// ---------------------------------------------------------------------------

/// One hand-built probe: old source, byte-range edit, expected reuse
/// verdict. Every probe must satisfy the differential gate regardless of
/// how much was reused.
fn reuse_probe(name: &str, old: &str, start: usize, end: usize, inserted: &str, min_reused: u64) {
    let old_b = old.as_bytes();
    let edit = CanonicalEdit::new(start, end, inserted).expect("edit");
    let post: String = format!("{}{}{}", &old[..start], inserted, &old[end..]);
    let post_b = post.as_bytes();
    let old_state = h2_full_parse(old_b);
    assert_tree_integrity(&old_state, old_b, name);

    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old_b, 40);
        let post_source = source_of(post_b, 41);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        let clean = parse_document(post_b);
        assert_eq!(pending.result(), &clean, "{name}: structural mismatch");
        let done = mech.complete(pending).expect("complete");
        assert_eq!(done.result_checksum, normalized_checksum(&clean), "{name}");
        assert_tree_integrity(&done.state, post_b, name);
    }
    match counters.nodes_reused {
        Observed::Known(n) => assert!(
            n >= min_reused,
            "{name}: nodes_reused {n} < expected minimum {min_reused}"
        ),
        other => panic!("{name}: nodes_reused must be Known, got {other:?}"),
    }
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "{name}: H2 has no fallback concept"
    );
}

#[test]
fn h2_reuse_pass_fragment_probes() {
    // A safe local edit deep in a many-block document: both fragment
    // pieces survive minGap and the blocks beyond the edit are taken
    // whole (the 128-byte minGap needs enough lead distance; the doc is
    // built with long blocks).
    let long =
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor.\n\n";
    let mut doc = String::new();
    for _ in 0..12 {
        doc.push_str(long);
    }
    // Edit inside block 2 (~170 bytes in): left piece survives minGap,
    // right piece survives, and the blocks beyond the edit are taken
    // whole (11 of the 12 blocks share identity).
    let e1 = long.len() * 2 + 10;
    reuse_probe("local edit deep in doc", &doc, e1, e1 + 2, "XY", 8);

    // Nested reuse: an edit inside a quote's THIRD paragraph must take
    // the quote's earlier sibling paragraph at the nested level (the
    // paragraph adjacent to the open edge stays excluded).
    let q1 = "> alpha beta gamma delta epsilon zeta eta theta iota kappa\n";
    let q2 = "> second line with some more words to fill the fragment size out\n";
    let q3 = "> third line with even more filler words to get safely past minGap\n";
    let tail = "tail para with additional words so the right fragment survives too\n\n";
    let nested_doc = format!("{q1}\n{q2}\n{q3}\n{tail}{tail}");
    let npos = q1.len() + 1 + q2.len() + 1 + q3.len() + 2; // inside q3's text
    reuse_probe("nested quote edit", &nested_doc, npos, npos + 6, "TWEAK", 1);

    // Unclosed fence inserted early: everything after is inside the fence
    // in the clean parse; no vouched take exists (W2 shape).
    reuse_probe(
        "unclosed fence swallows",
        "para one\n\npara two\n",
        0,
        4,
        "```\n",
        0,
    );
    // Container-state change before unchanged blocks: the vouching
    // refuses the mismatched candidates (W2 shape, mid-document).
    reuse_probe(
        "quote state inserted",
        "para one\n\npara two\n\npara three\n",
        8,
        10,
        "> ",
        0,
    );
}

// ---------------------------------------------------------------------------
// H2_IDENTITY_PASS — witnesses W1 / W2
// ---------------------------------------------------------------------------

#[test]
fn h2_identity_w1_safe_local_edit() {
    // A safe local edit inside one block of a definition-free document:
    // at least one mapped fragment survives, at least one old block
    // subtree is taken, source inspection < full source, result == H0.
    let old = "alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n\nepsilon five\n\nzeta six\n\neta seven\n";
    // The doc is ~86 bytes < minGap 128 — a single edit would drop BOTH
    // pieces; so W1 uses a document padded past minGap on both sides.
    let pad = "filler line one to push the fragment length well beyond the frozen minGap boundary\n\nfiller line two to keep both surviving fragment pieces safely above it\n\n";
    let old_full = format!("{pad}{old}{pad}");
    let old_b = old_full.as_bytes();
    // Edit inside "gamma three" region (well past minGap from both ends).
    let gpos = pad.len() + old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 5, "UMMA").expect("edit");
    let post: String = format!(
        "{}{}{}",
        &old_full[..gpos + 2],
        "UMMA",
        &old_full[gpos + 5..]
    );
    let post_b = post.as_bytes();

    let old_state = h2_full_parse(old_b);
    assert!(
        old_state.fragment_count() >= 1,
        "W1: a whole-doc fragment exists"
    );
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old_b, 50);
        let post_source = source_of(post_b, 51);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        assert_eq!(pending.result(), &parse_document(post_b), "W1 result == H0");
        mech.complete(pending).expect("complete");
        cx.sink.finalize_derived();
    }
    match counters.nodes_reused {
        Observed::Known(n) => assert!(n > 0, "W1: at least one old block subtree is taken"),
        other => panic!("W1: nodes_reused must be Known, got {other:?}"),
    }
    match counters.unique_source_bytes_inspected {
        Observed::Known(n) => assert!(
            n < post_b.len() as u64,
            "W1: inspected {n} must be < post len {}",
            post_b.len()
        ),
        other => panic!("W1: inspection must be Known, got {other:?}"),
    }
}

#[test]
fn h2_identity_w2_context_change_refuses_reuse() {
    // A fence-state change before unchanged blocks: the inserted unclosed
    // fence swallows every following line in the clean parse, so NO block
    // start beyond the edit exists in the live parse — the cursor is
    // never able to vouch a take again, and the whole-document reuse is
    // exactly zero while the result stays correct.
    let old = "alpha one\n\nfiller line one to push the document length beyond the frozen minGap\n\nfiller line two to keep the right fragment piece alive past minGap\n\nbeta two\n\ngamma three\n";
    let edit = CanonicalEdit::new(0, 0, "```\n").expect("edit");
    let post: String = format!("```\n{old}");
    let post_b = post.as_bytes();

    let old_state = h2_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old.as_bytes(), 60);
        let post_source = source_of(post_b, 61);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        assert_eq!(pending.result(), &parse_document(post_b), "W2 result == H0");
        mech.complete(pending).expect("complete");
    }
    // The fence persists to EOF: no vouched take exists after the change.
    match counters.nodes_reused {
        Observed::Known(n) => assert!(n == 0, "W2: vouching must refuse (got {n} reused)"),
        other => panic!("W2: nodes_reused must be Known, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// H2_COUNTER_PASS + H2_EAGER_COMPLETION_PASS
// ---------------------------------------------------------------------------

#[test]
fn h2_counters_and_eager_completion() {
    let pad = "filler line one to push the fragment length well beyond the frozen minGap boundary\n\nfiller line two to keep both surviving fragment pieces safely above it\n\n";
    let old = format!("{pad}alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 4, "UM").expect("edit");
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UM", &old[gpos + 4..]);
    let post_b = post.as_bytes();

    let mech = FragmentReuseMechanism::new();
    let old_state = h2_full_parse(old.as_bytes());
    let old_source = source_of(old.as_bytes(), 70);
    let post_source = source_of(post_b, 71);

    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        cx.sink.finalize_derived();
        pending
    };

    // EAGER (behavioral): counters + the complete inspection union are
    // observed strictly BEFORE complete().
    let clean = parse_document(post_b);
    assert_eq!(pending.result(), &clean, "pending already holds the result");
    assert!(matches!(counters.blocks_reparsed, Observed::Known(_)));
    assert!(matches!(counters.nodes_rebuilt, Observed::Known(_)));
    assert!(matches!(counters.nodes_reused, Observed::Known(n) if n > 0));
    assert!(matches!(
        counters.metadata_records_touched,
        Observed::Known(_)
    ));
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H2 has no fallback concept"
    );
    assert_eq!(counters.restart_distance, Observed::NotApplicable);
    assert_eq!(counters.convergence_distance, Observed::NotApplicable);

    // EAGER (structural): complete() receives no source and no sink.
    let done = mech.complete(pending).expect("complete");
    assert_eq!(done.result_checksum, normalized_checksum(&clean));
}

#[test]
fn h2_definition_changing_edits_reuse_only_reference_free_subtrees() {
    // Definition-changing edit: the reference clause refuses has_ref
    // candidates while the flag is set; reference-free blocks still
    // reuse; the result stays H0-equal (fresh inline content resolves
    // against the rebuilt document table).
    let pad = "filler line one to push the fragment length well beyond the frozen minGap boundary\n\nfiller line two to keep both surviving fragment pieces safely above it\n\n";
    let def_part = "[a]: /x\n\n";
    let ref_part = "see [text][a] here with quite a lot of additional filler bytes\n\n";
    let old = format!("{pad}{def_part}{ref_part}tail block one\n\ntail block two\n");
    // Replace the definition's destination only (definition-changing).
    let dpos = pad.len() + 5;
    let edit = CanonicalEdit::new(dpos, dpos + 2, "/yy").expect("edit");
    let post: String = format!("{}{}{}", &old[..dpos], "/yy", &old[dpos + 2..]);
    let post_b = post.as_bytes();

    let old_state = h2_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old.as_bytes(), 80);
        let post_source = source_of(post_b, 81);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        assert_eq!(pending.result(), &parse_document(post_b));
        mech.complete(pending).expect("complete");
    }
    // The tail blocks (reference-free) may still be taken.
    assert!(matches!(counters.nodes_reused, Observed::Known(n) if n > 0));
}

#[test]
fn h2_definition_moved_reference_reuses_when_unchanged() {
    // When nothing definition-bearing is damaged, a reference-bearing
    // subtree may be reused: its resolved destination stays valid because
    // every definition lives in unchanged bytes.
    let pad = "filler line one to push the fragment length well beyond the frozen minGap boundary\n\nfiller line two to keep both surviving fragment pieces safely above it\n\n";
    let def_part = "[a]: /x\n\n";
    let old =
        format!("{pad}alpha one\n\n{def_part}beta [text][a] two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 4, "UM").expect("edit");
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UM", &old[gpos + 4..]);
    let post_b = post.as_bytes();

    let old_state = h2_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old.as_bytes(), 85);
        let post_source = source_of(post_b, 86);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        assert_eq!(pending.result(), &parse_document(post_b));
        mech.complete(pending).expect("complete");
    }
    // The reference-bearing "beta" block sits BEFORE the edit in the left
    // fragment and MUST have been reused (no definition changed).
    assert!(matches!(counters.nodes_reused, Observed::Known(n) if n > 0));
}

// ---------------------------------------------------------------------------
// QUERY + determinism
// ---------------------------------------------------------------------------

#[test]
fn h2_query_batch_matches_the_frozen_contract() {
    for shape in ALL_SHAPES {
        let bytes = gen::generate(shape, SIZE_64K).expect("generate");
        let state = h2_full_parse(&bytes);
        assert_tree_integrity(&state, &bytes, "query state");
        let [early, middle, late] = gen::mutations::query_anchors(&bytes);
        for (offset, label) in [(early, "EARLY"), (middle, "MIDDLE"), (late, "LATE")] {
            // §5 of the R5 freeze: the answer comes from the projected
            // normalized tree of the COMPLETED state.
            let doc = {
                let mut counters = WorkCounters::all_unknown();
                let mut sink = CounterSink::new(&mut counters);
                let mut cx = MechanismContext::new(&mut sink);
                let mech = FragmentReuseMechanism::new();
                let pending = mech
                    .full_parse(&source_of(&bytes, 90), &mut cx)
                    .expect("full_parse");
                let d = pending.result().clone();
                mech.complete(pending).expect("complete");
                d
            };
            let path = node_path_at(&doc, offset);
            assert!(!path.is_empty(), "{shape:?} {label}");
            assert_eq!(path[0].kind, NodeKind::Document);
            assert_eq!(
                path,
                node_path_at(&parse_document(&bytes), offset),
                "{shape:?} {label}: must equal the H0 answer"
            );
        }
    }
}

#[test]
fn h2_updates_are_deterministic() {
    let old = gen::generate(PayloadShape::Mixed, SIZE_64K).expect("generate");
    let edit = generic_edit(
        &old,
        AnchorClass::Middle,
        EditSize::Small,
        GenericOp::ReplaceGrow,
    );
    let post = gen::mutations::apply(&old, &edit);
    let canonical = canon(&edit);
    let run = || {
        let state = h2_full_parse(&old);
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let prepared = mech
            .prepare_update(
                &source_of(&old, 1),
                &source_of(&post, 2),
                &canonical,
                &state,
                &mut cx,
            )
            .expect("prepare");
        let pending = mech
            .update(
                &source_of(&old, 1),
                &source_of(&post, 2),
                &canonical,
                state,
                prepared,
                &mut cx,
            )
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        (done.result_checksum, counters.nodes_reused)
    };
    assert_eq!(run(), run(), "two identical runs must agree");
}
