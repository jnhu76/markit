//! H1 BLOCK_LOCAL_REPARSE gate suite (stage R5, task contract §9).
//!
//! Gates proven here, per the frozen H1 gate list:
//!
//! - `H1_GRAMMAR_PASS` — the 43 golden fixtures: H1's clean parse equals
//!   the shared-grammar result and the retained tiling tiles the document
//!   exactly;
//! - `H1_DIFFERENTIAL_PASS` — over the frozen corpus grid, structural
//!   recipes, hand-built guard probes, and edit chains:
//!   `H1 update result == H0 clean authoritative parse` (structural tree
//!   equality + checksum), including every fallback class;
//! - `H1_STRUCTURAL_PASS` — post-update states satisfy the
//!   NORMALIZED-RESULT-v1 invariants;
//! - `H1_COUNTER_PASS` — the frozen applicability matrix for H1
//!   (fallback/metadata Known, restart/convergence NotApplicable, honest
//!   measured zeros);
//! - `H1_FALLBACK_PASS` — the guard classes fire exactly where the H1
//!   model cannot soundly localize, and never depend on case identity;
//! - `H1_IDENTITY_PASS` — identity witnesses W1 (safe local edit:
//!   no fallback, sub-full inspection) and W2 (definition presence:
//!   exactly one total fallback);
//! - `H1_EAGER_COMPLETION_PASS` — the pending already holds the complete
//!   eager native STATE; complete() seals only (the normalized projection
//!   and checksum are post-complete pure exports).

use gen::{PayloadShape, ALL_SHAPES, SIZE_16M, SIZE_1M, SIZE_64K};
use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, SourceVersion,
    WorkCounters, ResultChecksum,
};
use markit_mdbench_corpusgen as gen;
use markit_mdbench_corpusgen::mutations::{
    generic_edit, structural_edit_for, AnchorClass, EditSize, GenericOp,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::fixture::{load_fixtures, repo_fixture_dir};
use markit_mdbench_oracle::normalized::{node_path_at, normalized_checksum, NormalizedDocument};
use markit_mdbench_oracle::{validate_root, NormalizeV1};

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

/// H1 clean parse through the mechanism (own counter stream).
fn h1_full_parse(bytes: &[u8]) -> H1State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = BlockLocalMechanism::new();
    let source = source_of(bytes, 1);
    let pending = mech.full_parse(&source, &mut cx).expect("full_parse");
    mech.complete(pending).expect("complete").state
}

/// The frozen differential gate for one H1 update: the sealed state's
/// post-complete normalized projection must equal the H0 clean
/// authoritative parse of the post source, structurally and by checksum.
/// Returns (new state, update counters).
fn assert_update_structural(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H1State,
    context: &str,
) -> (H1State, WorkCounters) {
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
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
        // MEASUREMENT-CORRECTIVE-1: the projection is derived from the
        // SEALED state post-`complete()` (pure export), not carried in
        // the pending. Seal first, then assert structure + checksum.
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state.normalize_v1(),
            clean,
            "structural mismatch ({context})"
        );
        assert_eq!(
            done.state.result_checksum(),
            normalized_checksum(&clean),
            "checksum mismatch ({context})"
        );
        assert_eq!(done.state.source_len_bytes(), post.len(), "{context}");
        new_state = done.state;
    }
    (new_state, counters)
}

fn assert_tiling_integrity(state: &H1State, src: &[u8]) {
    let entries = state.entries();
    assert!(
        !entries.is_empty() || src.is_empty(),
        "empty tiling on non-empty source"
    );
    if entries.is_empty() {
        assert_eq!(src.len(), 0);
        return;
    }
    assert_eq!(entries.first().unwrap().span().0, 0, "tiling starts at 0");
    assert_eq!(
        entries.last().unwrap().span().1,
        src.len(),
        "tiling ends at len"
    );
    for w in entries.windows(2) {
        assert_eq!(
            w[0].span().1,
            w[1].span().0,
            "tiling must be contiguous: {:?} then {:?}",
            w[0].span(),
            w[1].span()
        );
    }
    for e in entries {
        let (s, en) = e.span();
        assert!(s < en, "entries are non-degenerate: {s}..{en}");
    }
}

fn validate_tree(doc: &NormalizedDocument, src: &[u8], context: &str) {
    if let Err(e) = validate_root(&doc.root, Some(src)) {
        panic!("normalized tree violates NORMALIZED-RESULT-v1 ({context}): {e}");
    }
}

// ---------------------------------------------------------------------------
// H1_GRAMMAR_PASS — fixtures
// ---------------------------------------------------------------------------

#[test]
fn h1_grammar_pass_all_43_fixtures() {
    let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load and validate");
    assert_eq!(fixtures.len(), 43, "the frozen fixture count is 43");
    let mut failed = Vec::new();
    for f in &fixtures {
        let src = f.source.as_bytes();
        let state = h1_full_parse(src);
        assert_tiling_integrity(&state, src);
        let clean = parse_document(src);
        // The tiling's blocks project to the same normalized document.
        let mut counters = WorkCounters::all_unknown();
        let result = {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = BlockLocalMechanism::new();
            let pending = mech
                .full_parse(&source_of(src, 2), &mut cx)
                .expect("full_parse");
            let done = mech.complete(pending).expect("complete");
            done.state.normalize_v1()
        };
        if result != clean {
            failed.push(format!("{}: structural mismatch", f.id));
        } else if normalized_checksum(&result) != normalized_checksum(&clean) {
            failed.push(format!("{}: checksum mismatch", f.id));
        }
    }
    assert!(
        failed.is_empty(),
        "fixtures FAILED ({}/43 passed):\n  {}",
        43 - failed.len(),
        failed.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// H1_DIFFERENTIAL_PASS — corpus grid + recipes + chains
// ---------------------------------------------------------------------------

fn generic_grid_case(
    shape: PayloadShape,
    size: usize,
    class: AnchorClass,
    es: EditSize,
    op: GenericOp,
) {
    let old = gen::generate(shape, size).expect("generate");
    let old_state = h1_full_parse(&old);
    assert_tiling_integrity(&old_state, &old);

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
    validate_tree(&parse_document(&post), &post, &context);
    assert_tiling_integrity(&new_state, &post);
}

#[test]
fn h1_update_matches_clean_parse_generic_grid_64k() {
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
fn h1_update_matches_clean_parse_every_shape_insert_tiny_all_anchors() {
    for shape in ALL_SHAPES {
        for size in [SIZE_64K, SIZE_1M] {
            for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
                generic_grid_case(shape, size, class, EditSize::Tiny, GenericOp::Insert);
            }
        }
    }
}

#[test]
fn h1_update_matches_clean_parse_scaling_slice() {
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
fn h1_update_matches_clean_parse_multibyte_sensitive_ops() {
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
fn h1_update_matches_clean_parse_structural_recipes_64k() {
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
                let old_state = h1_full_parse(&old);
                let context = format!("{shape:?}-64k {} {class:?}", recipe.id());
                let (new_state, _) =
                    assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
                assert_tiling_integrity(&new_state, &post);
            }
        }
    }
}

#[test]
fn h1_update_matches_clean_parse_structural_recipes_1m() {
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
            let old_state = h1_full_parse(&old);
            let context = format!("{shape:?}-1m {}", recipe.id());
            assert_update_structural(&old, &post, &canon(&edit), old_state, &context);
        }
    }
}

#[test]
fn h1_chained_edits_stay_equal_to_clean_parses() {
    let mut current = gen::generate(PayloadShape::Plain, SIZE_64K).expect("generate");
    let mut state = h1_full_parse(&current);
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
        assert_tiling_integrity(&next, &post);
        state = next;
        current = post;
    }
}

// ---------------------------------------------------------------------------
// H1_FALLBACK_PASS — hand-built guard probes
// ---------------------------------------------------------------------------

/// One hand-built probe: old source, byte-range edit, and the EXPECTED
/// fallback count (the guard class analysis is frozen in the R5 record).
/// Every probe must satisfy the differential gate regardless of whether
/// it falls back.
fn guard_probe(
    name: &str,
    old: &str,
    start: usize,
    end: usize,
    inserted: &str,
    expect_fallback: usize,
) {
    let old_b = old.as_bytes();
    let edit = CanonicalEdit::new(start, end, inserted).expect("edit");
    let mut post = String::new();
    post.push_str(&old[..start]);
    post.push_str(inserted);
    post.push_str(&old[end..]);
    let post_b = post.as_bytes();
    let old_state = h1_full_parse(old_b);
    assert_tiling_integrity(&old_state, old_b);

    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
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
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state.normalize_v1(),
            clean,
            "{name}: structural mismatch"
        );
        assert_eq!(
            done.state.result_checksum(),
            normalized_checksum(&clean),
            "{name}"
        );
        assert_tiling_integrity(&done.state, post_b);
    }
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(expect_fallback as u64),
        "{name}: fallback count"
    );
}

#[test]
fn h1_fallback_pass_guard_probes() {
    // W2 class — F1: any retained definition forces the TOTAL fallback.
    guard_probe(
        "F1 old definition",
        "[a]: /x\n\npara\n\nmore\n",
        14,
        18,
        "text",
        1,
    );
    // F1: a definition created INSIDE the region also forces it.
    guard_probe(
        "F1 new definition",
        "para one\n\npara two\n",
        10,
        14,
        "[b]: /y\n",
        1,
    );

    // F2 class — paragraph continuation across the region's left edge
    // (replace a heading with paragraph text; the clean parse merges).
    guard_probe("F2 para continuation", "para\n# h\nzz\n", 5, 8, "xy", 1);
    // F2 carry — deleting the terminator byte merges the paragraphs.
    guard_probe("F2 terminator deletion", "para\n\npara2\n", 4, 5, "", 1);
    // F2 carry — deleting the blank line between paragraphs merges them.
    guard_probe("F2 blank deletion", "para one\n\npara two\n", 9, 10, "", 1);

    // F3 class — quote continuation: a '>' line after a quote joins it.
    guard_probe("F3 quote continuation", "> a\n# h\nzz\n", 4, 8, "> b\n", 1);
    // F3 class — list continuation: a sibling marker line joins the list.
    guard_probe("F3 list sibling", "- a\n# h\nzz\n", 4, 8, "- b\n", 1);
    // F3 carry — deleting the blank between two lists merges them (the
    // blank line was the only separator; the sibling marker continues).
    guard_probe("F5 carry list merge", "- a\n\n- b\n\n- c\n", 4, 5, "", 1);

    // F4 class — an insert whose final line has no terminator merges with
    // the suffix's first line ("x" + "# h" is ONE line in the clean parse).
    guard_probe("F4 unterminated insert", "para\n\n# h\n", 6, 6, "x", 1);

    // F6 class — a fence opened inside the region and still open at the
    // region cut would swallow the suffix (a clean parse swallows the
    // following blocks into the fence).
    guard_probe(
        "F6 fence opens",
        "para one\n\nzz\n\npara two\n",
        10,
        12,
        "```",
        1,
    );
    // A fence edit whose closer stays inside the region localizes soundly.
    guard_probe(
        "fence localized",
        "```a\nx\n```\n\npara\n",
        0,
        4,
        "```b\n",
        0,
    );
    // F6/carry — deleting the fence opener: the body line becomes a
    // paragraph and the closer line opens a NEW fence that swallows the
    // tail; not localizable.
    guard_probe(
        "F6 fence opener deleted",
        "```a\nx\n```\n\npara\n",
        0,
        5,
        "",
        1,
    );

    // NO-fallback probes — sound localizations:
    // A mid-block text edit in a multi-block document localizes.
    guard_probe(
        "local para edit",
        "para one\n\npara two\n\npara three\n",
        24,
        26,
        "XX",
        0,
    );
    // A heading replacement localizes (interrupted boundary is preserved).
    guard_probe("heading edit", "# hello world\nzz\n", 7, 12, "welt", 0);
    // Deleting an interrupting heading at the DOCUMENT START: empty-region
    // suffix shift (no prefix block can merge).
    guard_probe(
        "interruptor deleted at doc start",
        "# h\ntext\n",
        0,
        4,
        "",
        0,
    );
    // Deleting one of two blank lines: still separated, no merge.
    guard_probe("blank thinned", "para one\n\n\npara two\n", 9, 10, "", 0);
    // Insert of a complete block at a blank/block boundary localizes.
    guard_probe(
        "block insert at boundary",
        "para\n\n# h\n",
        6,
        6,
        "# Y\n",
        0,
    );
    // EOF insert of paragraph text merges with the tail para -> fallback
    // (the para continuation class, at the document end).
    guard_probe("EOF para insert", "para\n", 5, 5, "text", 1);
    // Deleting an interrupting heading that separates two paragraphs
    // merges them (F4 carry class) -> fallback.
    guard_probe(
        "F4 carry interruptor deleted",
        "para\n# h\ntext\n",
        5,
        9,
        "",
        1,
    );
}

// ---------------------------------------------------------------------------
// H1_IDENTITY_PASS — witnesses W1 / W2
// ---------------------------------------------------------------------------

#[test]
fn h1_identity_w1_safe_local_edit() {
    // A safe local edit inside one block of a definition-free document:
    // no fallback, sub-full source inspection, affected block reparsed,
    // result == H0.
    let old = "alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n";
    let edit = CanonicalEdit::new(28, 30, "TH").expect("edit"); // inside "gamma"
    let post_b: String = format!("{}{}{}", &old[..28], "TH", &old[30..]);
    let post = post_b.as_bytes();

    let old_state = h1_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
        let old_source = source_of(old.as_bytes(), 50);
        let post_source = source_of(post, 51);
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
        // MEASUREMENT-CORRECTIVE-1: complete() seals the state; the
        // normalized projection is a post-complete pure export of it.
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state.normalize_v1(),
            parse_document(post),
            "W1 result == H0"
        );
        cx.sink.finalize_derived();
    }
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(0),
        "W1: no fallback"
    );
    match counters.unique_source_bytes {
        Observed::Known(n) => assert!(
            n < post.len() as u64,
            "W1: inspected {n} must be < post len {}",
            post.len()
        ),
        other => panic!("W1: inspection must be Known, got {other:?}"),
    }
    assert!(
        matches!(counters.blocks_reparsed, Observed::Known(n) if n > 0),
        "W1: the affected block was reparsed"
    );
    // Corrective counting rule (R5-CORRECTIVE-1 §8): alpha and beta pass
    // through structurally — each retained block entry is one structural
    // block node (the skeleton) plus its retained inline syntax (the
    // Text node), so the two moved entries are 2 x 2 = 4 native nodes.
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "W1: alpha and beta pass through structurally"
    );
}

#[test]
fn h1_identity_w2_definition_presence() {
    // A definition-bearing document with an edit ANYWHERE: exactly one
    // total fallback, result == H0.
    for (name, old, range, inserted) in [
        (
            "edit far from the definition",
            "[a]: /x\n\npara one\n\npara two\n",
            20usize..24usize,
            "TWO",
        ),
        (
            "edit before the definition",
            "para one\n\n[a]: /x\n\npara two\n",
            0usize..4usize,
            "ONE",
        ),
        (
            "definition-adjacent edit",
            "[a]: /x\n\npara\n",
            9usize..13usize,
            "TEXT",
        ),
    ] {
        let edit = CanonicalEdit::new(range.start, range.end, inserted).expect("edit");
        let post: String = format!("{}{}{}", &old[..range.start], inserted, &old[range.end..]);
        let post_b = post.as_bytes();
        let old_state = h1_full_parse(old.as_bytes());
        let mut counters = WorkCounters::all_unknown();
        {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = BlockLocalMechanism::new();
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
            // MEASUREMENT-CORRECTIVE-1: complete() seals the state; the
            // normalized projection is a post-complete pure export of it.
            let done = mech.complete(pending).expect("complete");
            assert_eq!(
                done.state.normalize_v1(),
                parse_document(post_b),
                "W2 {name}"
            );
        }
        assert_eq!(
            counters.fallback_to_full_count,
            Observed::Known(1),
            "W2 {name}: exactly one total fallback"
        );
    }
}

// ---------------------------------------------------------------------------
// H1_COUNTER_PASS + H1_EAGER_COMPLETION_PASS
// ---------------------------------------------------------------------------

#[test]
fn h1_counters_and_eager_completion() {
    let old = "alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n";
    let edit = CanonicalEdit::new(28, 30, "TH").expect("edit");
    let post: String = format!("{}{}{}", &old[..28], "TH", &old[30..]);
    let post_b = post.as_bytes();

    let mech = BlockLocalMechanism::new();
    let old_state = h1_full_parse(old.as_bytes());
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

    // EAGER (behavioral proof): the full counter set and the complete
    // inspection union are observed strictly BEFORE complete().
    let clean = parse_document(post_b);
    // MEASUREMENT-CORRECTIVE-1: the pending carries the eager native STATE; the normalized projection is a post-complete pure export of that state.
    assert!(matches!(counters.blocks_reparsed, Observed::Known(_)));
    assert!(matches!(counters.nodes_rebuilt, Observed::Known(_)));
    assert!(matches!(counters.nodes_reused, Observed::Known(n) if n > 0));
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    assert!(matches!(
        counters.metadata_records_touched,
        Observed::Known(_)
    ));
    assert_eq!(
        counters.restart_distance,
        Observed::NotApplicable,
        "H1 has no restart concept"
    );
    assert_eq!(
        counters.convergence_distance,
        Observed::NotApplicable,
        "H1 has no convergence concept"
    );

    // EAGER (structural proof): complete() receives no source and no sink
    // — it cannot parse. It seals the state; the checksum is the
    // post-timer export.
    let done = mech.complete(pending).expect("complete");
    assert_eq!(done.state.result_checksum(), normalized_checksum(&clean));

    // COMPLETED-STATE LAW (R5-CORRECTIVE-1, MAJOR-3/§10): the sealed
    // state projects to the normalized result PURELY — no Source, no
    // WorkSink, no parser call is even expressible here.
    let doc = done.state.normalize_v1();
    assert_eq!(doc, clean, "completed-state projection == H0");
    assert_eq!(
        normalized_checksum(&doc),
        done.state.result_checksum(),
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
    // The completed state is a usable input for the NEXT update.
    let tail = CanonicalEdit::new(post_b.len(), post_b.len(), "+".to_string()).expect("edit");
    let post2: Vec<u8> = post_b.iter().copied().chain(b"+".iter().copied()).collect();
    let mut counters2 = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters2);
        let mut cx = MechanismContext::new(&mut sink);
        let prepared = mech
            .prepare_update(
                &source_of(post_b, 72),
                &source_of(&post2, 73),
                &tail,
                &done.state,
                &mut cx,
            )
            .expect("follow-up prepare");
        let pending2 = mech
            .update(
                &source_of(post_b, 72),
                &source_of(&post2, 73),
                &tail,
                done.state,
                prepared,
                &mut cx,
            )
            .expect("follow-up update");
        let clean2 = parse_document(&post2);
        let done2 = mech.complete(pending2).expect("follow-up complete");
        assert_eq!(done2.state.normalize_v1(), clean2, "follow-up projection");
    }

    // Counter arithmetic is exact on this hand-built case: alpha and beta
    // pass through (2 retained blocks x 2 native nodes: skeleton + Text),
    // gamma is reparsed (2), delta is reconstructed over its retained
    // syntax (2).
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(4));
    assert_eq!(counters.nodes_reused, Observed::Known(4));
}

#[test]
fn h1_fallback_counters_are_full_reconstruction_plus_discarded_region() {
    // On any F-class fallback the reported counts are the full scan +
    // full reconstruction (they happened) PLUS the discarded region
    // reparse's actual work, which also happened and is never discarded
    // from attribution (MEASUREMENT-CORRECTIVE-1 §15 H1-B). nodes_reused
    // is the measured zero.
    let old = "[a]: /x\n\npara one\n\npara two\n";
    let edit = CanonicalEdit::new(20, 24, "TWO").expect("edit");
    let post: String = format!("{}{}{}", &old[..20], "TWO", &old[24..]);
    let post_b = post.as_bytes();
    let old_state = h1_full_parse(old.as_bytes());
    let clean = parse_document(post_b);

    // The discarded region's actual block count, derived independently
    // from the frozen region rule (R5 freeze §6) over the PUBLIC
    // retained tiling + the shared block scanner.
    let entries: Vec<(usize, usize)> = old_state.entries().iter().map(|e| e.span()).collect();
    let es = edit.start_byte() as usize;
    let ee = edit.end_byte() as usize;
    let delta =
        edit.inserted_text_len_bytes() as isize - edit.removed_len_bytes() as isize;
    let mut first = None;
    let mut last = 0usize;
    for (i, &(s, en)) in entries.iter().enumerate() {
        if s < ee && en > es {
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    let f = first.expect("the edit overlaps a retained entry");
    let rs = entries[..f].last().map(|&(_, en)| en).unwrap_or(0);
    let re_old = entries
        .get(last + 1)
        .map(|&(s, _)| s)
        .unwrap_or(old.len());
    let re_new = ((re_old as isize + delta).max(rs as isize) as usize).min(post_b.len());
    let discarded = {
        let mut noop = markit_mdbench_common::NoopWorkSink;
        markit_mdbench_block_local::count_blocks(
            &markit_mdbench_shared_grammar::parser::parse_region(
                post_b,
                rs,
                re_new,
                &mut noop,
            )
            .blocks,
        )
    };

    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
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
        let done = mech.complete(pending).expect("complete");
        assert_eq!(done.state.normalize_v1(), clean);
        cx.sink.finalize_derived();
    }
    assert_eq!(counters.fallback_to_full_count, Observed::Known(1));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    let clean_blocks = {
        fn walk(n: &markit_mdbench_oracle::normalized::Node, acc: &mut u64) {
            if !matches!(
                n.kind,
                markit_mdbench_oracle::normalized::NodeKind::Text
                    | markit_mdbench_oracle::normalized::NodeKind::Emphasis
                    | markit_mdbench_oracle::normalized::NodeKind::CodeSpan
                    | markit_mdbench_oracle::normalized::NodeKind::Link
                    | markit_mdbench_oracle::normalized::NodeKind::ReferenceLink
            ) {
                *acc += 1;
            }
            for c in &n.children {
                walk(c, acc);
            }
        }
        let mut acc = 0u64;
        walk(&clean.root, &mut acc);
        acc - 1 // Document root is not a parse product
    };
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(discarded + clean_blocks),
        "H1-B: discarded region blocks + the full document block count"
    );
    // Corrective counting rule (§8): a full reconstruction counts EVERY
    // native node it built — structural blocks and inline syntax alike.
    let clean_nodes = {
        fn walk(n: &markit_mdbench_oracle::normalized::Node, acc: &mut u64) {
            *acc += 1;
            for c in &n.children {
                walk(c, acc);
            }
        }
        let mut acc = 0u64;
        walk(&clean.root, &mut acc);
        acc - 1 // Document root is not a parse product
    };
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(discarded + clean_nodes),
        "H1-B: discarded skeleton units + the full native node count \
         (blocks + inline)"
    );
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_b.len() as u64),
        "fallback inspects the whole post source"
    );
}

// ---------------------------------------------------------------------------
// QUERY — the frozen NODE_PATH_AT batch from completed H1 state
// ---------------------------------------------------------------------------

#[test]
fn h1_query_batch_matches_the_frozen_contract() {
    use markit_mdbench_oracle::normalized::{node_path_at, NodeKind};
    for shape in ALL_SHAPES {
        let bytes = gen::generate(shape, SIZE_64K).expect("generate");
        let state = h1_full_parse(&bytes);
        assert_tiling_integrity(&state, &bytes);
        let [early, middle, late] = gen::mutations::query_anchors(&bytes);
        for (offset, label) in [(early, "EARLY"), (middle, "MIDDLE"), (late, "LATE")] {
            // §5 of the R5 freeze + MAJOR-3 (R5-CORRECTIVE-1): the answer
            // comes from `done.state.normalize_v1()` — the COMPLETED
            // state's pure projection. No Pending.result() shortcut, no
            // parser work, no horse-only query index.
            let doc = {
                let mut counters = WorkCounters::all_unknown();
                let mut sink = CounterSink::new(&mut counters);
                let mut cx = MechanismContext::new(&mut sink);
                let mech = BlockLocalMechanism::new();
                let pending = mech
                    .full_parse(&source_of(&bytes, 90), &mut cx)
                    .expect("full_parse");
                let done = mech.complete(pending).expect("complete");
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

// ---------------------------------------------------------------------------
// MAJOR-1 ownership witness (R5-CORRECTIVE-1) — prefix reuse is MOVE,
// not clone-based pseudo reuse
// ---------------------------------------------------------------------------

#[test]
fn h1_prefix_reuse_is_ownership_pass_through() {
    // The retained representation of an untouched prefix block must be
    // the SAME object after the update (ownership pass-through), not a
    // clone that merely re-reports nodes_reused. Witness: the heap
    // address of the block's retained inline node buffer is identical
    // before and after — while the update runs, the old tiling is still
    // alive, so a clone can never land on the same address.
    let old = "alpha one\n\nbeta two\n\ngamma three\n\ndelta four\n";
    let edit = CanonicalEdit::new(28, 30, "TH").expect("edit"); // inside gamma
    let post: String = format!("{}{}{}", &old[..28], "TH", &old[30..]);
    let post_b = post.as_bytes();

    let old_state = h1_full_parse(old.as_bytes());
    // beta's retained paragraph subtree ("beta two" starts at byte 11);
    // its children Vec is a non-empty heap buffer.
    let beta_before = old_state
        .entries()
        .iter()
        .find_map(|e| match e {
            markit_mdbench_block_local::TopEntry::Block { sem, .. }
                if sem.kind == markit_mdbench_oracle::normalized::NodeKind::Paragraph
                    && sem.start == 11 =>
            {
                Some(sem.children.as_ptr() as usize)
            }
            _ => None,
        })
        .expect("beta paragraph in the old tiling");

    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
        let old_source = source_of(old.as_bytes(), 110);
        let post_source = source_of(post_b, 111);
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
        let done = mech.complete(pending).expect("complete");
        // beta keeps its identity in the new tiling.
        let beta_after = done
            .state
            .entries()
            .iter()
            .find_map(|e| match e {
                markit_mdbench_block_local::TopEntry::Block { sem, .. }
                    if sem.kind == markit_mdbench_oracle::normalized::NodeKind::Paragraph
                        && sem.start == 11 =>
                {
                    Some(sem.children.as_ptr() as usize)
                }
                _ => None,
            })
            .expect("beta paragraph in the new tiling");
        assert_eq!(
            beta_after, beta_before,
            "prefix reuse must MOVE the retained representation \
             (clone-based pseudo reuse detected: buffer address changed)"
        );
        // And the result is still exactly H0.
        assert_eq!(
            done.state.normalize_v1(),
            parse_document(post_b),
            "ownership pass-through must not change the result"
        );
    }
    assert!(
        matches!(counters.nodes_reused, Observed::Known(n) if n > 0),
        "the moved prefix is counted as nodes_reused"
    );
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn h1_updates_are_deterministic() {
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
        let state = h1_full_parse(&old);
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
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
        (done.state.result_checksum(), counters.fallback_to_full_count)
    };
    assert_eq!(run(), run(), "two identical runs must agree");
}

// ---------------------------------------------------------------------------
// Attribution — guard margin reads are reported (R5-CORRECTIVE-2)
// ---------------------------------------------------------------------------

#[test]
fn h1_guard_margins_report_source_inspection() {
    // Fenced block followed by a paragraph: the right-edge pair (region
    // fence vs suffix paragraph) drives `would_continue` over the
    // suffix's first line — a read OUTSIDE the reparsed region [7, 18),
    // so its exact event pair is attributable to the guard, not to a
    // parse. Layout (old, 23 bytes): "alpha\n\n```\nc\n```\nomega\n" —
    // fence [7, 16), omega [17, 22); edit "c" -> "xy" at [11, 12).
    let old = "alpha\n\n```\nc\n```\nomega\n";
    let post_b = "alpha\n\n```\nxy\n```\nomega\n";
    let edit = CanonicalEdit::new(11, 12, "xy").expect("edit");

    let old_state = h1_full_parse(old.as_bytes());
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = BlockLocalMechanism::new();
        let old_source = source_of(old.as_bytes(), 70);
        let post_source = source_of(post_b.as_bytes(), 71);
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
        // MEASUREMENT-CORRECTIVE-1: complete() seals the state; the
        // normalized projection is a post-complete pure export of it.
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state.normalize_v1(),
            parse_document(post_b.as_bytes()),
            "result == H0"
        );
        cx.sink.finalize_derived();
        let events = sink.inspections();
        // `would_continue`'s suffix line scan is [18, 24) — the region
        // parse never reports that pair. The edge separation pairs
        // (17, 18) coincide with the region's final blank-line report,
        // so they are pinned by multiplicity instead: the guards re-read
        // those bytes (F4 backward scan + F5 pair + line-start scan),
        // the parser reports the line exactly once.
        assert!(
            events.contains(&(SourceVersion::Post, 18, 24)),
            "would_continue's suffix-line scan must be reported, events: {events:?}"
        );
        let edge = events
            .iter()
            .filter(|&&(v, s, e)| v == SourceVersion::Post && (s, e) == (17, 18))
            .count();
        assert!(
            edge >= 2,
            "guard margin re-reads of the edge bytes must be reported, events: {events:?}"
        );
    }
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(0),
        "the guard path must not fall back (the events are from a local update)"
    );
}
