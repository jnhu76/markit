//! H4 RESTART_CONVERGENCE gate suite (stage R5, task contract §11).
//!
//! Gates proven here, per the frozen H4 gate list:
//!
//! - `H4_GRAMMAR_PASS` — the 43 golden fixtures: H4's clean parse equals
//!   the authoritative result and the retained state satisfies the
//!   checkpoint/block invariants;
//! - `H4_DIFFERENTIAL_PASS` — corpus grid, structural recipes, edit
//!   chains, and hand-built convergence probes:
//!   `H4 update result == H0 clean authoritative parse`;
//! - `H4_STRUCTURAL_PASS` — post-update states satisfy the
//!   NORMALIZED-RESULT-v1 invariants (shared validation gate);
//! - `H4_COUNTER_PASS` — the frozen applicability matrix for H4
//!   (restart/convergence distances Known on EVERY update, including
//!   measured zeros; fallback NotApplicable — restart-at-zero is a
//!   restart, not a fallback);
//! - `H4_CONVERGENCE_PASS` — the predicate witnesses: convergence before
//!   EOF with suffix reuse, refusal when the mapped checkpoint is absent,
//!   refusal when the paragraph margin (blank line) is missing, and the
//!   unclosed-fence case that converges only at EOF;
//! - `H4_IDENTITY_PASS` — witnesses W1 (restart beyond 0 + convergence
//!   before EOF + nodes_reused > 0 + sub-full inspection), W2 (persistent
//!   context damage ⇒ exactly zero reuse, result == H0), W3 (definition-
//!   changing damage ⇒ restart at zero, generation bump, no reuse);
//! - `H4_EAGER_COMPLETION_PASS` — the pending already holds the complete
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
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::fixture::{load_fixtures, repo_fixture_dir};
use markit_mdbench_oracle::normalized::{node_path_at, normalized_checksum, NodeKind};
use markit_mdbench_oracle::{validate_root, NormalizeV1};
use std::sync::Arc;

use markit_mdbench_restart_convergence::{H4State, RestartConvergenceMechanism};

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

fn h4_full_parse(bytes: &[u8]) -> H4State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = RestartConvergenceMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 1), &mut cx)
        .expect("full_parse");
    // Control operation: no restart happened, so the gauges have no
    // referent here (they are measured on updates only).
    assert_eq!(counters.restart_distance, Observed::NotApplicable);
    assert_eq!(counters.convergence_distance, Observed::NotApplicable);
    assert_eq!(counters.fallback_to_full_count, Observed::NotApplicable);
    mech.complete(pending).expect("complete").state
}

/// The frozen differential gate for one H4 update: the pending's already
/// materialized result must equal the H0 clean authoritative parse of the
/// post source, structurally and by checksum.
fn assert_update_structural(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H4State,
    context: &str,
) -> (H4State, WorkCounters) {
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
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
        // Gauges are measured on every update (Known(0) is a measured
        // zero) — never Unknown, never NotApplicable.
        assert!(
            matches!(counters.restart_distance, Observed::Known(_)),
            "{context}: restart_distance must be Known, got {:?}",
            counters.restart_distance
        );
        assert!(
            matches!(counters.convergence_distance, Observed::Known(_)),
            "{context}: convergence_distance must be Known, got {:?}",
            counters.convergence_distance
        );
        assert_eq!(
            counters.fallback_to_full_count,
            Observed::NotApplicable,
            "{context}: H4 has no fallback concept"
        );
        let done = mech.complete(pending).expect("complete");
        assert_eq!(done.result_checksum, normalized_checksum(&clean));
        new_state = done.state;
    }
    (new_state, counters)
}

fn assert_state_integrity(state: &H4State, src: &[u8], context: &str) {
    let blocks = state.blocks();
    let cps = state.checkpoints();
    assert_eq!(
        blocks.len(),
        cps.len(),
        "{context}: checkpoints are 1:1 with block slots"
    );
    let mut prev_end = 0usize;
    for (slot, cp) in blocks.iter().zip(cps) {
        let (s, e) = (slot.abs_start(), slot.abs_end());
        assert!(s >= prev_end, "{context}: blocks ordered");
        assert!(e > s, "{context}: blocks non-degenerate");
        assert_eq!(
            cp.position,
            slot.line_position(),
            "{context}: checkpoint position matches its slot's line start"
        );
        prev_end = e;
    }
    // Slots cover every block; trailing blank bytes need no slot.
    assert!(
        prev_end <= src.len(),
        "{context}: blocks must not exceed the document"
    );
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
// H4_GRAMMAR_PASS — fixtures
// ---------------------------------------------------------------------------

#[test]
fn h4_grammar_pass_all_43_fixtures() {
    let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load and validate");
    assert_eq!(fixtures.len(), 43, "the frozen fixture count is 43");
    let mut failed = Vec::new();
    for f in &fixtures {
        let src = f.source.as_bytes();
        let state = h4_full_parse(src);
        assert_state_integrity(&state, src, &f.id);
        assert_eq!(
            state.generation(),
            0,
            "{}: a clean parse is generation 0",
            f.id
        );
        let mut counters = WorkCounters::all_unknown();
        let result = {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = RestartConvergenceMechanism::new();
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
// H4_DIFFERENTIAL_PASS — corpus grid + recipes + chains
// ---------------------------------------------------------------------------

fn generic_grid_case(
    shape: PayloadShape,
    size: usize,
    class: AnchorClass,
    es: EditSize,
    op: GenericOp,
) {
    let old = gen::generate(shape, size).expect("generate");
    let old_state = h4_full_parse(&old);
    assert_state_integrity(&old_state, &old, "grid old state");

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
    assert_state_integrity(&new_state, &post, &context);
}

#[test]
fn h4_update_matches_clean_parse_generic_grid_64k() {
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
fn h4_update_matches_clean_parse_every_shape_insert_tiny_all_anchors() {
    for shape in ALL_SHAPES {
        for size in [SIZE_64K, SIZE_1M] {
            for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
                generic_grid_case(shape, size, class, EditSize::Tiny, GenericOp::Insert);
            }
        }
    }
}

#[test]
fn h4_update_matches_clean_parse_scaling_slice() {
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
fn h4_update_matches_clean_parse_multibyte_sensitive_ops() {
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
fn h4_update_matches_clean_parse_structural_recipes_64k() {
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
                let old_state = h4_full_parse(&old);
                let context = format!("{shape:?}-64k {} {class:?}", recipe.id());
                let (new_state, _) =
                    assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
                assert_state_integrity(&new_state, &post, &context);
            }
        }
    }
}

#[test]
fn h4_update_matches_clean_parse_structural_recipes_1m() {
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
            let old_state = h4_full_parse(&old);
            let context = format!("{shape:?}-1m {}", recipe.id());
            assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
        }
    }
}

#[test]
fn h4_chained_edits_stay_equal_to_clean_parses() {
    let mut current = gen::generate(PayloadShape::Plain, SIZE_64K).expect("generate");
    let mut state = h4_full_parse(&current);
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
        assert_state_integrity(&next, &post, &context);
        state = next;
        current = post;
    }
}

// ---------------------------------------------------------------------------
// H4_CONVERGENCE_PASS — predicate witnesses
// ---------------------------------------------------------------------------

/// One hand-built convergence probe: old source, byte-range edit, then
/// (a) the differential gate regardless of convergence, (b) the expected
/// gauges, (c) the expected reuse.
fn convergence_probe(
    name: &str,
    old: &str,
    (start, end, inserted): (usize, usize, &str),
    expect_restart: Option<u64>,
    expect_convergence: Option<u64>,
    min_reused: u64,
) -> H4State {
    let old_b = old.as_bytes();
    let edit = CanonicalEdit::new(start, end, inserted).expect("edit");
    let post: String = format!("{}{}{}", &old[..start], inserted, &old[end..]);
    let post_b = post.as_bytes();
    let old_state = h4_full_parse(old_b);
    assert_state_integrity(&old_state, old_b, name);

    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
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
        new_state = done.state;
        cx.sink.finalize_derived();
    }
    assert_state_integrity(&new_state, post_b, name);
    if let Some(expected) = expect_restart {
        assert_eq!(
            counters.restart_distance,
            Observed::Known(expected),
            "{name}: restart distance"
        );
    }
    if let Some(expected) = expect_convergence {
        assert_eq!(
            counters.convergence_distance,
            Observed::Known(expected),
            "{name}: convergence distance"
        );
    }
    match counters.nodes_reused {
        Observed::Known(n) => assert!(
            n >= min_reused,
            "{name}: nodes_reused {n} < expected minimum {min_reused}"
        ),
        other => panic!("{name}: nodes_reused must be Known, got {other:?}"),
    }
    new_state
}

#[test]
fn h4_convergence_pass_predicate_witnesses() {
    let pad = "filler line one to push the document length well past any boundary checks\n\nfiller line two keeps the tail far from the edit region\n\n";
    // W1 shape: edit inside the THIRD of four blank-separated paragraphs.
    // Restart at that paragraph's checkpoint (distance 2 into the block),
    // convergence at the checkpoint of the LAST paragraph — before EOF —
    // and the whole tail after it reused.
    let old = format!("{pad}alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UMMA", &old[gpos + 5..]);
    // The gauge measures the LIVE parse distance: the convergence point
    // in POST coordinates minus the restart position (the edit shifted
    // the delta paragraph by +1 here).
    let dpos_post = post.find("delta four").unwrap();
    let state = convergence_probe(
        "converge before EOF",
        &old,
        (gpos + 2, gpos + 5, "UMMA"),
        Some(2),                         // restart at the gamma checkpoint, 2 bytes in
        Some((dpos_post - gpos) as u64), // convergence at the delta checkpoint (post coords)
        1,                               // the delta subtree is reused
    );
    assert_eq!(state.generation(), 0, "no definition change");

    // An edit exactly AT a block boundary restarts with distance 0 — a
    // measured zero, still Known.
    convergence_probe(
        "restart distance zero",
        &old,
        (gpos, gpos, "ZZ "),
        Some(0),
        None,
        1,
    );

    // Deleting one LF of the blank line between the last two paragraphs
    // merges them: the live paragraph is open at the mapped checkpoint's
    // position, so convergence must be refused (predicate (e)/(a)) and
    // the suffix reparsed — the differential proves the merge.
    let merge_old = "para one\n\npara two\n\npara three\n";
    let thin = merge_old.rfind("\n\n").unwrap() + 1;
    convergence_probe(
        "blank thinned before suffix",
        merge_old,
        (thin, thin + 1, ""),
        Some(9), // restart at the "para two" checkpoint: es 19 - restart 10
        None,
        0,
    );

    // THE (e)-marginal case: deleting a separator LF from a blank gap
    // BEFORE a paragraph checkpoint. The mapped checkpoint exists and
    // every other predicate holds, but the live paragraph is open at the
    // splice — taking the suffix would split one paragraph in two. The
    // blank-line margin must refuse it (the differential proves the
    // paragraphs joined).
    let join_old = "aaaa\n\nbbbb\ncccc\n";
    let jpos = join_old.find("\n\n").unwrap() + 1; // the SECOND separator LF
    convergence_probe(
        "paragraph join across thinned gap",
        join_old,
        (jpos, jpos + 1, ""),
        Some(5), // restart at the document start (position 0): es 5 - 0
        None,
        0,
    );

    // An unclosed fence inserted at the document start swallows the whole
    // clean parse: no block start ever fires, convergence happens only at
    // EOF, and nothing is reused (W2 shape).
    convergence_probe(
        "unclosed fence swallows",
        "alpha one\n\nbeta two\n",
        (0, 0, "```\n"),
        Some(0),
        Some("```\nalpha one\n\nbeta two\n".len() as u64),
        0,
    );
}

// ---------------------------------------------------------------------------
// H4_IDENTITY_PASS — witnesses W1 / W2 / W3
// ---------------------------------------------------------------------------

#[test]
fn h4_identity_w1_restart_convergence_and_subfull_inspection() {
    let pad = "filler line one to push the document length well past any boundary checks\n\nfiller line two keeps the tail far from the edit region\n\n";
    let old = format!("{pad}alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 5, "UMMA").expect("edit");
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UMMA", &old[gpos + 5..]);
    let post_b = post.as_bytes();

    let old_state = h4_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    let prepared_restart;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
        let old_source = source_of(old.as_bytes(), 50);
        let post_source = source_of(post_b, 51);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        // W1: the restart lands on the damaged block's own checkpoint.
        prepared_restart = prepared.restart_position;
        assert_eq!(prepared_restart, gpos, "W1: restart at the damaged block");
        assert_eq!(prepared.damaged_entries, 1, "W1: exactly one damaged entry");
        assert!(!prepared.damaged_has_def, "W1: no definitions in play");
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
    assert_eq!(
        counters.restart_distance,
        Observed::Known(2),
        "W1: restart distance is the offset into the damaged block"
    );
    match counters.nodes_reused {
        Observed::Known(n) => assert!(n > 0, "W1: the stable suffix was reused"),
        other => panic!("W1: nodes_reused must be Known, got {other:?}"),
    }
    match counters.nodes_rebuilt {
        Observed::Known(n) => assert!(n > 0, "W1: the damaged region was rebuilt"),
        other => panic!("W1: nodes_rebuilt must be Known, got {other:?}"),
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
fn h4_identity_w2_persistent_context_damage_zero_reuse() {
    // The unclosed fence flips every later line's live context: no
    // convergence candidate can agree, and the exact measured zero is
    // reported.
    let old = "alpha one\n\nfiller line one to keep the tail block far away from the edit region\n\nbeta two\n";
    let edit = CanonicalEdit::new(0, 0, "```\n").expect("edit");
    let post: String = format!("```\n{old}");
    let post_b = post.as_bytes();
    let old_state = h4_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
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
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "W2: the fence damage refuses every candidate"
    );
    assert_eq!(
        counters.restart_distance,
        Observed::Known(0),
        "W2: damage at the document start"
    );
}

#[test]
fn h4_identity_w3_definition_change_restarts_at_zero() {
    // (a) Deleting a ReferenceDefinition that a LATER-use reference link
    // resolves against: the damage is far from the document start, yet
    // the restart must be at ZERO (resolution is document-global), with
    // a generation bump and zero reuse. The differential proves the
    // earlier reference link degraded to literal text.
    let old = "see [x] here\n\nalpha one\n\nbeta two\n\n[x]: https://example.com\n";
    let dpos = old.find("[x]:").unwrap();
    let edit = CanonicalEdit::new(dpos, old.len(), "").expect("edit");
    let post: String = old[..dpos].to_string();
    let post_b = post.as_bytes();
    let old_state = h4_full_parse(old.as_bytes());
    assert_eq!(old_state.generation(), 0);
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
        let old_source = source_of(old.as_bytes(), 70);
        let post_source = source_of(post_b, 71);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare");
        assert!(
            prepared.damaged_has_def,
            "W3a: the damaged entry carries the definition"
        );
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
        assert_eq!(
            pending.result(),
            &parse_document(post_b),
            "W3a result == H0"
        );
        let done = mech.complete(pending).expect("complete");
        new_state = done.state;
        cx.sink.finalize_derived();
    }
    assert_eq!(
        counters.restart_distance,
        Observed::Known(dpos as u64),
        "W3a: restart AT ZERO despite the late damage — distance = edit start"
    );
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "W3a: nothing is vouched across a definition change"
    );
    assert_eq!(
        counters.convergence_distance,
        Observed::Known(post_b.len() as u64),
        "W3a: the restart parse runs to EOF"
    );
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "W3a: a restart is not a fallback"
    );
    assert_eq!(new_state.generation(), 1, "W3a: generation bumped");

    // (b) CREATING a definition inside the edited region (`]: ` in the
    // post bytes) is definition-changing too, even with no damaged Def.
    let old_b = "alpha one\n\nbeta two\n\ngamma three\n";
    let ipos = old_b.find("beta").unwrap();
    let edit_b = CanonicalEdit::new(ipos, ipos, "[n]: dest\n").expect("edit");
    let post_b2: String = format!("{}[n]: dest\n{}", &old_b[..ipos], &old_b[ipos..]);
    let old_state_b = h4_full_parse(old_b.as_bytes());
    let mut counters_b = WorkCounters::all_unknown();
    let new_state_b;
    {
        let mut sink = CounterSink::new(&mut counters_b);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
        let old_source = source_of(old_b.as_bytes(), 72);
        let post_source = source_of(post_b2.as_bytes(), 73);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit_b, &old_state_b, &mut cx)
            .expect("prepare");
        assert!(
            !prepared.damaged_has_def,
            "W3b: no damaged definition — the region creation triggers"
        );
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit_b,
                old_state_b,
                prepared,
                &mut cx,
            )
            .expect("update");
        assert_eq!(
            pending.result(),
            &parse_document(post_b2.as_bytes()),
            "W3b result == H0"
        );
        let done = mech.complete(pending).expect("complete");
        new_state_b = done.state;
    }
    assert_eq!(
        counters_b.restart_distance,
        Observed::Known(ipos as u64),
        "W3b: region-created definition forces the zero restart"
    );
    assert_eq!(counters_b.nodes_reused, Observed::Known(0), "W3b");
    assert_eq!(new_state_b.generation(), 1, "W3b: generation bumped");
    assert_eq!(
        new_state_b.defs().len(),
        1,
        "W3b: the new definition is kept"
    );
}

// ---------------------------------------------------------------------------
// H4_COUNTER_PASS + H4_EAGER_COMPLETION_PASS
// ---------------------------------------------------------------------------

#[test]
fn h4_counters_and_eager_completion() {
    let pad = "filler line one to push the document length well past any boundary checks\n\nfiller line two keeps the tail far from the edit region\n\n";
    let old = format!("{pad}alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 4, "UM").expect("edit");
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UM", &old[gpos + 4..]);
    let post_b = post.as_bytes();

    let mech = RestartConvergenceMechanism::new();
    let old_state = h4_full_parse(old.as_bytes());
    let old_source = source_of(old.as_bytes(), 80);
    let post_source = source_of(post_b, 81);

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
        "H4 has no fallback concept"
    );
    assert!(
        matches!(counters.restart_distance, Observed::Known(_)),
        "the restart distance is measured on every update"
    );
    assert!(
        matches!(counters.convergence_distance, Observed::Known(_)),
        "the convergence distance is measured on every update"
    );

    // EAGER (structural): complete() receives no source and no sink.
    let done = mech.complete(pending).expect("complete");
    assert_eq!(done.result_checksum, normalized_checksum(&clean));

    // COMPLETED-STATE LAW (R5-CORRECTIVE-1, MAJOR-3/00a710): the sealed
    // state projects to the normalized result PURELY — no Source, no
    // WorkSink, no parser call is even expressible here.
    let doc = done.state.normalize_v1();
    assert_eq!(doc, clean, "completed-state projection == H0");
    assert_eq!(
        normalized_checksum(&doc),
        done.result_checksum,
        "checksum(completed normalize_v1) == completed checksum"
    );
    // QUERY from the completed state equals the H0 answers.
    let [early, middle, late] = gen::mutations::query_anchors(post_b);
    for (offset, label) in [(early, "EARLY"), (middle, "MIDDLE"), (late, "LATE")] {
        assert_eq!(
            node_path_at(&doc, offset),
            node_path_at(&clean, offset),
            "completed-state QUERY {label} must equal the H0 answer"
        );
    }
}

// ---------------------------------------------------------------------------
// QUERY + determinism
// ---------------------------------------------------------------------------

#[test]
fn h4_query_batch_matches_the_frozen_contract() {
    for shape in ALL_SHAPES {
        let bytes = gen::generate(shape, SIZE_64K).expect("generate");
        let state = h4_full_parse(&bytes);
        assert_state_integrity(&state, &bytes, "query state");
        let [early, middle, late] = gen::mutations::query_anchors(&bytes);
        for (offset, label) in [(early, "EARLY"), (middle, "MIDDLE"), (late, "LATE")] {
            // §5 of the R5 freeze: the answer comes from the projected
            // normalized tree of the COMPLETED state.
            let doc = {
                let mut counters = WorkCounters::all_unknown();
                let mut sink = CounterSink::new(&mut counters);
                let mut cx = MechanismContext::new(&mut sink);
                let mech = RestartConvergenceMechanism::new();
                let pending = mech
                    .full_parse(&source_of(&bytes, 90), &mut cx)
                    .expect("full_parse");
                let done = mech.complete(pending).expect("complete");
                // MAJOR-3 (R5-CORRECTIVE-1): the COMPLETED state's pure
                // projection is the query authority.
                done.state.normalize_v1()
            };
            let path = node_path_at(&doc, offset);
            assert!(!path.is_empty(), "{shape:?} {label}");
            assert_eq!(path[0].kind, NodeKind::Document);
            for pn in &path {
                assert!(
                    (pn.start <= offset && offset < pn.end) || offset == bytes.len(),
                    "{shape:?} {label}: containment"
                );
            }
            assert_eq!(
                path,
                node_path_at(&parse_document(&bytes), offset),
                "{shape:?} {label}: must equal the H0 answer"
            );
        }
    }
}

#[test]
fn h4_determinism_same_inputs_same_witnesses() {
    let old = gen::generate(PayloadShape::Mixed, SIZE_64K).expect("generate");
    let edit = generic_edit(
        &old,
        AnchorClass::Middle,
        EditSize::Small,
        GenericOp::ReplaceGrow,
    );
    let post = gen::mutations::apply(&old, &edit);
    let canon_edit = canon(&edit);

    let mut runs: Vec<(u64, WorkCounters)> = Vec::new();
    for _ in 0..2 {
        let old_state = h4_full_parse(&old);
        let mut counters = WorkCounters::all_unknown();
        {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = RestartConvergenceMechanism::new();
            let old_source = source_of(&old, 100);
            let post_source = source_of(&post, 101);
            let prepared = mech
                .prepare_update(&old_source, &post_source, &canon_edit, &old_state, &mut cx)
                .expect("prepare");
            let pending = mech
                .update(
                    &old_source,
                    &post_source,
                    &canon_edit,
                    old_state,
                    prepared,
                    &mut cx,
                )
                .expect("update");
            let checksum = normalized_checksum(pending.result());
            mech.complete(pending).expect("complete");
            cx.sink.finalize_derived();
            runs.push((checksum, counters));
        }
    }
    assert_eq!(runs[0].0, runs[1].0, "same inputs, same result checksum");
    assert_eq!(runs[0].1, runs[1].1, "same inputs, same counters");
}

#[test]
fn h4_converged_suffix_shares_retained_syntax_identity() {
    // R5-CORRECTIVE-1 (self-review Q5): convergence must reuse the WHOLE
    // retained block — skeleton AND materialized semantic subtree —
    // through Arc identity, never a skeleton retarget followed by an
    // inline rescan. Witness: the converged suffix head slot holds the
    // SAME Arc object the old state held before the update.
    let pad = "filler line one to push the document length well past any boundary checks\n\nfiller line two keeps the tail far from the edit region\n\n";
    let old = format!("{pad}alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n");
    let gpos = old.find("gamma").unwrap();
    let edit = CanonicalEdit::new(gpos + 2, gpos + 5, "UMMA").expect("edit");
    let post: String = format!("{}{}{}", &old[..gpos + 2], "UMMA", &old[gpos + 5..]);
    let post_b = post.as_bytes();

    let old_state = h4_full_parse(old.as_bytes());
    // The converged suffix head is the LAST top-level block ("delta
    // four"): capture its shared object's address BEFORE the update.
    let suffix_ptr_before =
        Arc::as_ptr(&old_state.blocks().last().expect("delta slot").block) as usize;

    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = RestartConvergenceMechanism::new();
        let old_source = source_of(old.as_bytes(), 120);
        let post_source = source_of(post_b, 121);
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
        assert_eq!(pending.result(), &clean, "structural mismatch");
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state.normalize_v1(),
            clean,
            "completed-state projection"
        );
        new_state = done.state;
        cx.sink.finalize_derived();
    }
    let last = new_state.blocks().last().expect("delta slot");
    assert_eq!(
        last.abs_start(),
        post.find("delta four").unwrap(),
        "the converged suffix head is the delta block"
    );
    assert_eq!(
        Arc::as_ptr(&last.block) as usize,
        suffix_ptr_before,
        "converged suffix must be the SAME retained syntax object (Arc identity), \
         not a rebuilt representation rescanned from source"
    );
    assert!(
        matches!(counters.nodes_reused, Observed::Known(n) if n > 0),
        "the shared suffix is counted as nodes_reused"
    );
}
