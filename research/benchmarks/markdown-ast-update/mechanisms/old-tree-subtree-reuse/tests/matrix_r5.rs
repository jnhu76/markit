//! Frozen 370-slot CASE-MATRIX-v1 differential for H3 OLD_TREE_SUBTREE_REUSE (stage R5).
//!
//! Instantiates the frozen case matrix (`cases/CASE-MATRIX-v1.md`) and
//! runs every UNIQUE case through old_tree_subtree_reuse against the H0 authoritative
//! parse: `H3 result == H0` on every update, on the FULL_PARSE
//! baseline, and on the frozen QUERY batch. The case-slot expansion and
//! the 7 declared slot overlaps follow CASE-MATRIX-v1 section 6 exactly;
//! the per-recipe Block-D counts are checked against the frozen table.
//! `#[ignore]`d in the debug profile; `verify-r5.sh` runs it with
//! `--release`. No timing, no benchmark data — correctness only.

use std::collections::BTreeMap;

use gen::{PayloadShape, SIZE_16M, SIZE_1M, SIZE_64K};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, ResultChecksum, Source, WorkCounters,
};
use markit_mdbench_corpusgen as gen;
use markit_mdbench_corpusgen::mutations::{
    apply, generic_edit, query_anchors, structural_edit_for, validate, AnchorClass, EditSize,
    GenericOp, PlannedEdit, Recipe,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_old_tree_subtree_reuse::{H3State, OldTreeSubtreeReuseMechanism};
use markit_mdbench_oracle::normalized::{node_path_at, normalized_checksum, NodeKind};
use markit_mdbench_oracle::{validate_root, NormalizeV1};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn full_parse_state(bytes: &[u8]) -> H3State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = OldTreeSubtreeReuseMechanism::new();
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
    let mech = OldTreeSubtreeReuseMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 2), &mut cx)
        .unwrap_or_else(|e| panic!("{context}: full_parse failed: {e:?}"));
    let done = mech.complete(pending).expect("complete");
    // MAJOR-3 (R5-CORRECTIVE-1) + MEASUREMENT-CORRECTIVE-1: the QUERY
    // authority is the COMPLETED state's pure projection — complete()
    // seals the state; the checksum is the post-timer export of it.
    let doc = done.state.normalize_v1();
    assert_eq!(
        normalized_checksum(&doc),
        done.state.result_checksum(),
        "{context}: checksum law"
    );
    doc
}

fn run_full_parse_case(bytes: &[u8], context: &str) {
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
        let mech = OldTreeSubtreeReuseMechanism::new();
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
        let done = mech.complete(pending).expect("complete");
        // MEASUREMENT-CORRECTIVE-1: complete() seals the state; the
        // normalized projection is a post-complete pure export of that
        // state, so the comparisons below cover both the former
        // eager-pending checks and the completed-state projection check.
        let doc = done.state.normalize_v1();
        assert_eq!(doc, clean, "{context}: structural mismatch vs H0");
        assert_eq!(
            done.state.result_checksum(),
            normalized_checksum(&clean),
            "{context}: checksum mismatch vs H0"
        );
        validate_root(&doc.root, Some(&post))
            .unwrap_or_else(|e| panic!("{context}: NORMALIZED-RESULT-v1 violation: {e}"));
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
        let mech = OldTreeSubtreeReuseMechanism::new();
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
        let done2 = mech.complete(pending).expect("complete");
        assert_eq!(
            done2.state.normalize_v1(),
            clean,
            "{context}: chained completed-state projection vs H0"
        );
    }
}

// ---------------------------------------------------------------------------
// Case-per-test driver (MARKIT-R5-GATE-OBSERVABILITY-2 restructure).
//
// The frozen CASE-MATRIX-v1 inventory is embedded as CASE_TABLE — the
// same enumeration order the former single-test driver used (sizes
// outer, ALL_SHAPES inner, then blocks A1/A2/B/C/A3/D per pair). Every
// case is its own #[test]: per-case output, per-case failure context,
// skip via filters, retry via `cargo nextest run --retries`,
// fail-fast via `nextest --fail-fast`, resume by name filter.
// Structural slots that are NOT_APPLICABLE for their corpus (declared
// by the matrix, content-derived) pass trivially; the FROZEN totals
// and per-recipe counts are asserted by r5_case_inventory_frozen_counts.
// No case semantics changed.
// ---------------------------------------------------------------------------

/// Sizes, indexed by CASE_TABLE rows (CASE-MATRIX-v1 section 4).
const CASE_SIZES: [usize; 3] = [SIZE_64K, SIZE_1M, SIZE_16M];

/// Shapes in ALL_SHAPES order, indexed by CASE_TABLE rows.
const CASE_SHAPES: [PayloadShape; 8] = [
    PayloadShape::Plain,
    PayloadShape::ManyBlocks,
    PayloadShape::HugeBlock,
    PayloadShape::DeepContainer,
    PayloadShape::InlineDense,
    PayloadShape::FenceHeavy,
    PayloadShape::ReferenceFanout,
    PayloadShape::Mixed,
];

#[derive(Debug)]
enum CaseKind {
    FullParse,
    Query,
    Grid {
        op: GenericOp,
        es: EditSize,
        class: AnchorClass,
    },
    Scaling {
        op: GenericOp,
        es: EditSize,
        class: AnchorClass,
    },
    CoreInsertTinyMiddle,
    Structural {
        recipe: Recipe,
        class: AnchorClass,
    },
}

/// (shape idx, size idx, kind) in the frozen enumeration order.
static CASE_TABLE: [(usize, usize, CaseKind); 548] = [
    (0, 0, CaseKind::FullParse),
    (0, 0, CaseKind::Query),
    (0, 0, CaseKind::CoreInsertTinyMiddle),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (0, 1, CaseKind::FullParse),
    (0, 1, CaseKind::Query),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (0, 2, CaseKind::FullParse),
    (0, 2, CaseKind::Query),
    (0, 2, CaseKind::CoreInsertTinyMiddle),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        0,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (1, 0, CaseKind::FullParse),
    (1, 0, CaseKind::Query),
    (
        1,
        0,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        0,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Scaling {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (1, 1, CaseKind::FullParse),
    (1, 1, CaseKind::Query),
    (1, 1, CaseKind::CoreInsertTinyMiddle),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (1, 2, CaseKind::FullParse),
    (1, 2, CaseKind::Query),
    (
        1,
        2,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        2,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Scaling {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        1,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (2, 0, CaseKind::FullParse),
    (2, 0, CaseKind::Query),
    (
        2,
        0,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        0,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Scaling {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (2, 1, CaseKind::FullParse),
    (2, 1, CaseKind::Query),
    (2, 1, CaseKind::CoreInsertTinyMiddle),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (2, 2, CaseKind::FullParse),
    (2, 2, CaseKind::Query),
    (
        2,
        2,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        2,
        CaseKind::Scaling {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Scaling {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        2,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (3, 0, CaseKind::FullParse),
    (3, 0, CaseKind::Query),
    (3, 0, CaseKind::CoreInsertTinyMiddle),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (3, 1, CaseKind::FullParse),
    (3, 1, CaseKind::Query),
    (3, 1, CaseKind::CoreInsertTinyMiddle),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (3, 2, CaseKind::FullParse),
    (3, 2, CaseKind::Query),
    (3, 2, CaseKind::CoreInsertTinyMiddle),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        3,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (4, 0, CaseKind::FullParse),
    (4, 0, CaseKind::Query),
    (4, 0, CaseKind::CoreInsertTinyMiddle),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (4, 1, CaseKind::FullParse),
    (4, 1, CaseKind::Query),
    (4, 1, CaseKind::CoreInsertTinyMiddle),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (4, 2, CaseKind::FullParse),
    (4, 2, CaseKind::Query),
    (4, 2, CaseKind::CoreInsertTinyMiddle),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        4,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (5, 0, CaseKind::FullParse),
    (5, 0, CaseKind::Query),
    (5, 0, CaseKind::CoreInsertTinyMiddle),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (5, 1, CaseKind::FullParse),
    (5, 1, CaseKind::Query),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (5, 2, CaseKind::FullParse),
    (5, 2, CaseKind::Query),
    (5, 2, CaseKind::CoreInsertTinyMiddle),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        5,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (6, 0, CaseKind::FullParse),
    (6, 0, CaseKind::Query),
    (6, 0, CaseKind::CoreInsertTinyMiddle),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (6, 1, CaseKind::FullParse),
    (6, 1, CaseKind::Query),
    (6, 1, CaseKind::CoreInsertTinyMiddle),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (6, 2, CaseKind::FullParse),
    (6, 2, CaseKind::Query),
    (6, 2, CaseKind::CoreInsertTinyMiddle),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        6,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (7, 0, CaseKind::FullParse),
    (7, 0, CaseKind::Query),
    (7, 0, CaseKind::CoreInsertTinyMiddle),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        0,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (7, 1, CaseKind::FullParse),
    (7, 1, CaseKind::Query),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Insert,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::Delete,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceEq,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceGrow,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Tiny,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Small,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Grid {
            op: GenericOp::ReplaceShrink,
            es: EditSize::Medium,
            class: AnchorClass::Late,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        1,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
    (7, 2, CaseKind::FullParse),
    (7, 2, CaseKind::Query),
    (7, 2, CaseKind::CoreInsertTinyMiddle),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocText,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::LocUtf8Swap,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaSplit,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::BbParaMerge,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsItemIndent,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::CsBqNestLine,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceOpen,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::FsFenceClose,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsEmphInsert,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsCodeDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::IdsLinkDelim,
            class: AnchorClass::Middle,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefReplace,
            class: AnchorClass::Early,
        },
    ),
    (
        7,
        2,
        CaseKind::Structural {
            recipe: Recipe::SdDefDelete,
            class: AnchorClass::Early,
        },
    ),
];

fn case_context(shape_idx: usize, size_idx: usize, kind: &CaseKind) -> String {
    let base = format!(
        "{:?}-{}",
        CASE_SHAPES[shape_idx],
        gen::size_label(CASE_SIZES[size_idx])
    );
    match kind {
        CaseKind::FullParse => format!("{base} FULL_PARSE"),
        CaseKind::Query => format!("{base} QUERY"),
        CaseKind::Grid { op, es, class } => format!("{base} grid {op:?}-{es:?}-{class:?}"),
        CaseKind::Scaling { op, es, class } => format!("{base} scaling {op:?}-{es:?}-{class:?}"),
        CaseKind::CoreInsertTinyMiddle => format!("{base} core INSERT-TINY-MIDDLE"),
        CaseKind::Structural { recipe, class } => format!("{base} {} {class:?}", recipe.id()),
    }
}

fn dispatch_case(idx: usize) {
    let (shape_idx, size_idx, kind) = &CASE_TABLE[idx];
    let context = case_context(*shape_idx, *size_idx, kind);
    eprintln!("[matrix case {idx:03}] {context}");
    let src = gen::generate(CASE_SHAPES[*shape_idx], CASE_SIZES[*size_idx])
        .unwrap_or_else(|e| panic!("{context}: generate: {e}"));
    match kind {
        CaseKind::FullParse => run_full_parse_case(&src, &context),
        CaseKind::Query => run_query_case(&src, &context),
        CaseKind::Grid { op, es, class } | CaseKind::Scaling { op, es, class } => {
            let edit = generic_edit(&src, *class, *es, *op);
            run_update_case(&src, &edit, &context);
        }
        CaseKind::CoreInsertTinyMiddle => {
            let edit = generic_edit(&src, AnchorClass::Middle, EditSize::Tiny, GenericOp::Insert);
            run_update_case(&src, &edit, &context);
        }
        CaseKind::Structural { recipe, class } => {
            let Ok(edit) = structural_edit_for(
                &src,
                *recipe,
                *class,
                CASE_SHAPES[*shape_idx],
                CASE_SIZES[*size_idx],
            ) else {
                eprintln!("[matrix case {idx:03}] NOT_APPLICABLE (declared by CASE-MATRIX-v1)");
                return;
            };
            run_update_case(&src, &edit, &context);
        }
    }
}

#[test]
#[ignore = "frozen inventory counts; verify-r5.sh runs it with --release"]
fn r5_case_inventory_frozen_counts() {
    let mut recipe_counts: BTreeMap<&'static str, usize> = gen::mutations::ALL_RECIPES
        .iter()
        .map(|r| (r.id(), 0usize))
        .collect();
    let mut full_parse_cases = 0usize;
    let mut query_cases = 0usize;
    let mut update_cases = 0usize;
    let mut doc_cache: BTreeMap<(usize, usize), Vec<u8>> = BTreeMap::new();
    for (idx, (shape_idx, size_idx, kind)) in CASE_TABLE.iter().enumerate() {
        match kind {
            CaseKind::FullParse => full_parse_cases += 1,
            CaseKind::Query => query_cases += 1,
            CaseKind::Grid { .. } | CaseKind::Scaling { .. } | CaseKind::CoreInsertTinyMiddle => {
                update_cases += 1;
            }
            CaseKind::Structural { recipe, class } => {
                let key = (*shape_idx, *size_idx);
                let src = doc_cache.entry(key).or_insert_with(|| {
                    gen::generate(CASE_SHAPES[*shape_idx], CASE_SIZES[*size_idx])
                        .unwrap_or_else(|e| panic!("case {idx}: generate: {e}"))
                });
                if structural_edit_for(
                    src,
                    *recipe,
                    *class,
                    CASE_SHAPES[*shape_idx],
                    CASE_SIZES[*size_idx],
                )
                .is_ok()
                {
                    *recipe_counts.get_mut(recipe.id()).expect("known recipe") += 1;
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

macro_rules! matrix_cases {
    ($($name:ident => $idx:literal;)*) => {$(
        #[test]
        #[ignore = "matrix case; run with --release (verify-r5.sh, or nextest for parallel/retry/resume)"]
        fn $name() {
            dispatch_case($idx);
        }
    )*};
}

matrix_cases! {
    matrix_case_000 => 0;
    matrix_case_001 => 1;
    matrix_case_002 => 2;
    matrix_case_003 => 3;
    matrix_case_004 => 4;
    matrix_case_005 => 5;
    matrix_case_006 => 6;
    matrix_case_007 => 7;
    matrix_case_008 => 8;
    matrix_case_009 => 9;
    matrix_case_010 => 10;
    matrix_case_011 => 11;
    matrix_case_012 => 12;
    matrix_case_013 => 13;
    matrix_case_014 => 14;
    matrix_case_015 => 15;
    matrix_case_016 => 16;
    matrix_case_017 => 17;
    matrix_case_018 => 18;
    matrix_case_019 => 19;
    matrix_case_020 => 20;
    matrix_case_021 => 21;
    matrix_case_022 => 22;
    matrix_case_023 => 23;
    matrix_case_024 => 24;
    matrix_case_025 => 25;
    matrix_case_026 => 26;
    matrix_case_027 => 27;
    matrix_case_028 => 28;
    matrix_case_029 => 29;
    matrix_case_030 => 30;
    matrix_case_031 => 31;
    matrix_case_032 => 32;
    matrix_case_033 => 33;
    matrix_case_034 => 34;
    matrix_case_035 => 35;
    matrix_case_036 => 36;
    matrix_case_037 => 37;
    matrix_case_038 => 38;
    matrix_case_039 => 39;
    matrix_case_040 => 40;
    matrix_case_041 => 41;
    matrix_case_042 => 42;
    matrix_case_043 => 43;
    matrix_case_044 => 44;
    matrix_case_045 => 45;
    matrix_case_046 => 46;
    matrix_case_047 => 47;
    matrix_case_048 => 48;
    matrix_case_049 => 49;
    matrix_case_050 => 50;
    matrix_case_051 => 51;
    matrix_case_052 => 52;
    matrix_case_053 => 53;
    matrix_case_054 => 54;
    matrix_case_055 => 55;
    matrix_case_056 => 56;
    matrix_case_057 => 57;
    matrix_case_058 => 58;
    matrix_case_059 => 59;
    matrix_case_060 => 60;
    matrix_case_061 => 61;
    matrix_case_062 => 62;
    matrix_case_063 => 63;
    matrix_case_064 => 64;
    matrix_case_065 => 65;
    matrix_case_066 => 66;
    matrix_case_067 => 67;
    matrix_case_068 => 68;
    matrix_case_069 => 69;
    matrix_case_070 => 70;
    matrix_case_071 => 71;
    matrix_case_072 => 72;
    matrix_case_073 => 73;
    matrix_case_074 => 74;
    matrix_case_075 => 75;
    matrix_case_076 => 76;
    matrix_case_077 => 77;
    matrix_case_078 => 78;
    matrix_case_079 => 79;
    matrix_case_080 => 80;
    matrix_case_081 => 81;
    matrix_case_082 => 82;
    matrix_case_083 => 83;
    matrix_case_084 => 84;
    matrix_case_085 => 85;
    matrix_case_086 => 86;
    matrix_case_087 => 87;
    matrix_case_088 => 88;
    matrix_case_089 => 89;
    matrix_case_090 => 90;
    matrix_case_091 => 91;
    matrix_case_092 => 92;
    matrix_case_093 => 93;
    matrix_case_094 => 94;
    matrix_case_095 => 95;
    matrix_case_096 => 96;
    matrix_case_097 => 97;
    matrix_case_098 => 98;
    matrix_case_099 => 99;
    matrix_case_100 => 100;
    matrix_case_101 => 101;
    matrix_case_102 => 102;
    matrix_case_103 => 103;
    matrix_case_104 => 104;
    matrix_case_105 => 105;
    matrix_case_106 => 106;
    matrix_case_107 => 107;
    matrix_case_108 => 108;
    matrix_case_109 => 109;
    matrix_case_110 => 110;
    matrix_case_111 => 111;
    matrix_case_112 => 112;
    matrix_case_113 => 113;
    matrix_case_114 => 114;
    matrix_case_115 => 115;
    matrix_case_116 => 116;
    matrix_case_117 => 117;
    matrix_case_118 => 118;
    matrix_case_119 => 119;
    matrix_case_120 => 120;
    matrix_case_121 => 121;
    matrix_case_122 => 122;
    matrix_case_123 => 123;
    matrix_case_124 => 124;
    matrix_case_125 => 125;
    matrix_case_126 => 126;
    matrix_case_127 => 127;
    matrix_case_128 => 128;
    matrix_case_129 => 129;
    matrix_case_130 => 130;
    matrix_case_131 => 131;
    matrix_case_132 => 132;
    matrix_case_133 => 133;
    matrix_case_134 => 134;
    matrix_case_135 => 135;
    matrix_case_136 => 136;
    matrix_case_137 => 137;
    matrix_case_138 => 138;
    matrix_case_139 => 139;
    matrix_case_140 => 140;
    matrix_case_141 => 141;
    matrix_case_142 => 142;
    matrix_case_143 => 143;
    matrix_case_144 => 144;
    matrix_case_145 => 145;
    matrix_case_146 => 146;
    matrix_case_147 => 147;
    matrix_case_148 => 148;
    matrix_case_149 => 149;
    matrix_case_150 => 150;
    matrix_case_151 => 151;
    matrix_case_152 => 152;
    matrix_case_153 => 153;
    matrix_case_154 => 154;
    matrix_case_155 => 155;
    matrix_case_156 => 156;
    matrix_case_157 => 157;
    matrix_case_158 => 158;
    matrix_case_159 => 159;
    matrix_case_160 => 160;
    matrix_case_161 => 161;
    matrix_case_162 => 162;
    matrix_case_163 => 163;
    matrix_case_164 => 164;
    matrix_case_165 => 165;
    matrix_case_166 => 166;
    matrix_case_167 => 167;
    matrix_case_168 => 168;
    matrix_case_169 => 169;
    matrix_case_170 => 170;
    matrix_case_171 => 171;
    matrix_case_172 => 172;
    matrix_case_173 => 173;
    matrix_case_174 => 174;
    matrix_case_175 => 175;
    matrix_case_176 => 176;
    matrix_case_177 => 177;
    matrix_case_178 => 178;
    matrix_case_179 => 179;
    matrix_case_180 => 180;
    matrix_case_181 => 181;
    matrix_case_182 => 182;
    matrix_case_183 => 183;
    matrix_case_184 => 184;
    matrix_case_185 => 185;
    matrix_case_186 => 186;
    matrix_case_187 => 187;
    matrix_case_188 => 188;
    matrix_case_189 => 189;
    matrix_case_190 => 190;
    matrix_case_191 => 191;
    matrix_case_192 => 192;
    matrix_case_193 => 193;
    matrix_case_194 => 194;
    matrix_case_195 => 195;
    matrix_case_196 => 196;
    matrix_case_197 => 197;
    matrix_case_198 => 198;
    matrix_case_199 => 199;
    matrix_case_200 => 200;
    matrix_case_201 => 201;
    matrix_case_202 => 202;
    matrix_case_203 => 203;
    matrix_case_204 => 204;
    matrix_case_205 => 205;
    matrix_case_206 => 206;
    matrix_case_207 => 207;
    matrix_case_208 => 208;
    matrix_case_209 => 209;
    matrix_case_210 => 210;
    matrix_case_211 => 211;
    matrix_case_212 => 212;
    matrix_case_213 => 213;
    matrix_case_214 => 214;
    matrix_case_215 => 215;
    matrix_case_216 => 216;
    matrix_case_217 => 217;
    matrix_case_218 => 218;
    matrix_case_219 => 219;
    matrix_case_220 => 220;
    matrix_case_221 => 221;
    matrix_case_222 => 222;
    matrix_case_223 => 223;
    matrix_case_224 => 224;
    matrix_case_225 => 225;
    matrix_case_226 => 226;
    matrix_case_227 => 227;
    matrix_case_228 => 228;
    matrix_case_229 => 229;
    matrix_case_230 => 230;
    matrix_case_231 => 231;
    matrix_case_232 => 232;
    matrix_case_233 => 233;
    matrix_case_234 => 234;
    matrix_case_235 => 235;
    matrix_case_236 => 236;
    matrix_case_237 => 237;
    matrix_case_238 => 238;
    matrix_case_239 => 239;
    matrix_case_240 => 240;
    matrix_case_241 => 241;
    matrix_case_242 => 242;
    matrix_case_243 => 243;
    matrix_case_244 => 244;
    matrix_case_245 => 245;
    matrix_case_246 => 246;
    matrix_case_247 => 247;
    matrix_case_248 => 248;
    matrix_case_249 => 249;
    matrix_case_250 => 250;
    matrix_case_251 => 251;
    matrix_case_252 => 252;
    matrix_case_253 => 253;
    matrix_case_254 => 254;
    matrix_case_255 => 255;
    matrix_case_256 => 256;
    matrix_case_257 => 257;
    matrix_case_258 => 258;
    matrix_case_259 => 259;
    matrix_case_260 => 260;
    matrix_case_261 => 261;
    matrix_case_262 => 262;
    matrix_case_263 => 263;
    matrix_case_264 => 264;
    matrix_case_265 => 265;
    matrix_case_266 => 266;
    matrix_case_267 => 267;
    matrix_case_268 => 268;
    matrix_case_269 => 269;
    matrix_case_270 => 270;
    matrix_case_271 => 271;
    matrix_case_272 => 272;
    matrix_case_273 => 273;
    matrix_case_274 => 274;
    matrix_case_275 => 275;
    matrix_case_276 => 276;
    matrix_case_277 => 277;
    matrix_case_278 => 278;
    matrix_case_279 => 279;
    matrix_case_280 => 280;
    matrix_case_281 => 281;
    matrix_case_282 => 282;
    matrix_case_283 => 283;
    matrix_case_284 => 284;
    matrix_case_285 => 285;
    matrix_case_286 => 286;
    matrix_case_287 => 287;
    matrix_case_288 => 288;
    matrix_case_289 => 289;
    matrix_case_290 => 290;
    matrix_case_291 => 291;
    matrix_case_292 => 292;
    matrix_case_293 => 293;
    matrix_case_294 => 294;
    matrix_case_295 => 295;
    matrix_case_296 => 296;
    matrix_case_297 => 297;
    matrix_case_298 => 298;
    matrix_case_299 => 299;
    matrix_case_300 => 300;
    matrix_case_301 => 301;
    matrix_case_302 => 302;
    matrix_case_303 => 303;
    matrix_case_304 => 304;
    matrix_case_305 => 305;
    matrix_case_306 => 306;
    matrix_case_307 => 307;
    matrix_case_308 => 308;
    matrix_case_309 => 309;
    matrix_case_310 => 310;
    matrix_case_311 => 311;
    matrix_case_312 => 312;
    matrix_case_313 => 313;
    matrix_case_314 => 314;
    matrix_case_315 => 315;
    matrix_case_316 => 316;
    matrix_case_317 => 317;
    matrix_case_318 => 318;
    matrix_case_319 => 319;
    matrix_case_320 => 320;
    matrix_case_321 => 321;
    matrix_case_322 => 322;
    matrix_case_323 => 323;
    matrix_case_324 => 324;
    matrix_case_325 => 325;
    matrix_case_326 => 326;
    matrix_case_327 => 327;
    matrix_case_328 => 328;
    matrix_case_329 => 329;
    matrix_case_330 => 330;
    matrix_case_331 => 331;
    matrix_case_332 => 332;
    matrix_case_333 => 333;
    matrix_case_334 => 334;
    matrix_case_335 => 335;
    matrix_case_336 => 336;
    matrix_case_337 => 337;
    matrix_case_338 => 338;
    matrix_case_339 => 339;
    matrix_case_340 => 340;
    matrix_case_341 => 341;
    matrix_case_342 => 342;
    matrix_case_343 => 343;
    matrix_case_344 => 344;
    matrix_case_345 => 345;
    matrix_case_346 => 346;
    matrix_case_347 => 347;
    matrix_case_348 => 348;
    matrix_case_349 => 349;
    matrix_case_350 => 350;
    matrix_case_351 => 351;
    matrix_case_352 => 352;
    matrix_case_353 => 353;
    matrix_case_354 => 354;
    matrix_case_355 => 355;
    matrix_case_356 => 356;
    matrix_case_357 => 357;
    matrix_case_358 => 358;
    matrix_case_359 => 359;
    matrix_case_360 => 360;
    matrix_case_361 => 361;
    matrix_case_362 => 362;
    matrix_case_363 => 363;
    matrix_case_364 => 364;
    matrix_case_365 => 365;
    matrix_case_366 => 366;
    matrix_case_367 => 367;
    matrix_case_368 => 368;
    matrix_case_369 => 369;
    matrix_case_370 => 370;
    matrix_case_371 => 371;
    matrix_case_372 => 372;
    matrix_case_373 => 373;
    matrix_case_374 => 374;
    matrix_case_375 => 375;
    matrix_case_376 => 376;
    matrix_case_377 => 377;
    matrix_case_378 => 378;
    matrix_case_379 => 379;
    matrix_case_380 => 380;
    matrix_case_381 => 381;
    matrix_case_382 => 382;
    matrix_case_383 => 383;
    matrix_case_384 => 384;
    matrix_case_385 => 385;
    matrix_case_386 => 386;
    matrix_case_387 => 387;
    matrix_case_388 => 388;
    matrix_case_389 => 389;
    matrix_case_390 => 390;
    matrix_case_391 => 391;
    matrix_case_392 => 392;
    matrix_case_393 => 393;
    matrix_case_394 => 394;
    matrix_case_395 => 395;
    matrix_case_396 => 396;
    matrix_case_397 => 397;
    matrix_case_398 => 398;
    matrix_case_399 => 399;
    matrix_case_400 => 400;
    matrix_case_401 => 401;
    matrix_case_402 => 402;
    matrix_case_403 => 403;
    matrix_case_404 => 404;
    matrix_case_405 => 405;
    matrix_case_406 => 406;
    matrix_case_407 => 407;
    matrix_case_408 => 408;
    matrix_case_409 => 409;
    matrix_case_410 => 410;
    matrix_case_411 => 411;
    matrix_case_412 => 412;
    matrix_case_413 => 413;
    matrix_case_414 => 414;
    matrix_case_415 => 415;
    matrix_case_416 => 416;
    matrix_case_417 => 417;
    matrix_case_418 => 418;
    matrix_case_419 => 419;
    matrix_case_420 => 420;
    matrix_case_421 => 421;
    matrix_case_422 => 422;
    matrix_case_423 => 423;
    matrix_case_424 => 424;
    matrix_case_425 => 425;
    matrix_case_426 => 426;
    matrix_case_427 => 427;
    matrix_case_428 => 428;
    matrix_case_429 => 429;
    matrix_case_430 => 430;
    matrix_case_431 => 431;
    matrix_case_432 => 432;
    matrix_case_433 => 433;
    matrix_case_434 => 434;
    matrix_case_435 => 435;
    matrix_case_436 => 436;
    matrix_case_437 => 437;
    matrix_case_438 => 438;
    matrix_case_439 => 439;
    matrix_case_440 => 440;
    matrix_case_441 => 441;
    matrix_case_442 => 442;
    matrix_case_443 => 443;
    matrix_case_444 => 444;
    matrix_case_445 => 445;
    matrix_case_446 => 446;
    matrix_case_447 => 447;
    matrix_case_448 => 448;
    matrix_case_449 => 449;
    matrix_case_450 => 450;
    matrix_case_451 => 451;
    matrix_case_452 => 452;
    matrix_case_453 => 453;
    matrix_case_454 => 454;
    matrix_case_455 => 455;
    matrix_case_456 => 456;
    matrix_case_457 => 457;
    matrix_case_458 => 458;
    matrix_case_459 => 459;
    matrix_case_460 => 460;
    matrix_case_461 => 461;
    matrix_case_462 => 462;
    matrix_case_463 => 463;
    matrix_case_464 => 464;
    matrix_case_465 => 465;
    matrix_case_466 => 466;
    matrix_case_467 => 467;
    matrix_case_468 => 468;
    matrix_case_469 => 469;
    matrix_case_470 => 470;
    matrix_case_471 => 471;
    matrix_case_472 => 472;
    matrix_case_473 => 473;
    matrix_case_474 => 474;
    matrix_case_475 => 475;
    matrix_case_476 => 476;
    matrix_case_477 => 477;
    matrix_case_478 => 478;
    matrix_case_479 => 479;
    matrix_case_480 => 480;
    matrix_case_481 => 481;
    matrix_case_482 => 482;
    matrix_case_483 => 483;
    matrix_case_484 => 484;
    matrix_case_485 => 485;
    matrix_case_486 => 486;
    matrix_case_487 => 487;
    matrix_case_488 => 488;
    matrix_case_489 => 489;
    matrix_case_490 => 490;
    matrix_case_491 => 491;
    matrix_case_492 => 492;
    matrix_case_493 => 493;
    matrix_case_494 => 494;
    matrix_case_495 => 495;
    matrix_case_496 => 496;
    matrix_case_497 => 497;
    matrix_case_498 => 498;
    matrix_case_499 => 499;
    matrix_case_500 => 500;
    matrix_case_501 => 501;
    matrix_case_502 => 502;
    matrix_case_503 => 503;
    matrix_case_504 => 504;
    matrix_case_505 => 505;
    matrix_case_506 => 506;
    matrix_case_507 => 507;
    matrix_case_508 => 508;
    matrix_case_509 => 509;
    matrix_case_510 => 510;
    matrix_case_511 => 511;
    matrix_case_512 => 512;
    matrix_case_513 => 513;
    matrix_case_514 => 514;
    matrix_case_515 => 515;
    matrix_case_516 => 516;
    matrix_case_517 => 517;
    matrix_case_518 => 518;
    matrix_case_519 => 519;
    matrix_case_520 => 520;
    matrix_case_521 => 521;
    matrix_case_522 => 522;
    matrix_case_523 => 523;
    matrix_case_524 => 524;
    matrix_case_525 => 525;
    matrix_case_526 => 526;
    matrix_case_527 => 527;
    matrix_case_528 => 528;
    matrix_case_529 => 529;
    matrix_case_530 => 530;
    matrix_case_531 => 531;
    matrix_case_532 => 532;
    matrix_case_533 => 533;
    matrix_case_534 => 534;
    matrix_case_535 => 535;
    matrix_case_536 => 536;
    matrix_case_537 => 537;
    matrix_case_538 => 538;
    matrix_case_539 => 539;
    matrix_case_540 => 540;
    matrix_case_541 => 541;
    matrix_case_542 => 542;
    matrix_case_543 => 543;
    matrix_case_544 => 544;
    matrix_case_545 => 545;
    matrix_case_546 => 546;
    matrix_case_547 => 547;
}
