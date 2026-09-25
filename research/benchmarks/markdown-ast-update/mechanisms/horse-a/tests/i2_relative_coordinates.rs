//! I2 T2c — Owner-relative coordinates (spec §4; tasks #15/#16/#40/#41).
//!
//! ```text
//! absolute = owner_base + relative   (recursively, every span/content interval)
//! ```
//!
//! The audit goes beyond export equality: retained payloads are
//! inspected directly, and a deliberately perturbed state with an
//! absolute coordinate is proven to fail the validator.

mod common;

use common::{build, h0};
use markit_mdbench_horse_a::{NormalizeV1, OwnerPayload};
use markit_mdbench_oracle::normalized::NodeKind;
use markit_mdbench_oracle::validate_normalized;

#[test]
fn a_top_level_span_is_owner_relative() {
    // "hello\nworld\n": one paragraph [0,11); one Owner [0,12).
    let doc = build(b"hello\nworld\n");
    let owners = doc.owners.owners_in_order();
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => assert_eq!((p.root.start, p.root.end), (0, 11)),
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert_eq!(doc.normalize_v1(), h0(b"hello\nworld\n"));
}

#[test]
fn nested_block_spans_are_owner_relative() {
    // "> q\n> > deep\n": one root Quote Owner [0,13); nested quote and
    // paragraph spans stay relative to the same base (0 here).
    // Byte layout: outer quote [0,12); inner quote [6,12); para [8,12);
    // first para [2,3).
    let doc = build(b"> q\n> > deep\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::BlockQuote);
            assert_eq!((p.root.start, p.root.end), (0, 12));
            assert_eq!((p.root.children[0].start, p.root.children[0].end), (2, 3));
            let inner = &p.root.children[1];
            assert_eq!(inner.kind, NodeKind::BlockQuote);
            assert_eq!((inner.start, inner.end), (6, 12));
            assert_eq!((inner.children[0].start, inner.children[0].end), (8, 12));
        }
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert_eq!(doc.normalize_v1(), h0(b"> q\n> > deep\n"));
}

#[test]
fn inline_spans_are_owner_relative_at_a_nonzero_base() {
    // "x\n\npara *em* `c` [l](/u)\n": x(0) LF(1) LF(2), para block [3,24).
    // Owner 1 base = 3. Text runs run up to each construct start (the
    // space before a construct belongs to the run). Absolute spans:
    // Text [3,8), Emphasis [8,12), Text [12,13), CodeSpan [13,16),
    // Text [16,17), Link [17,24) — relative minus 3.
    let doc = build(b"x\n\npara *em* `c` [l](/u)\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    match &owners[1].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!((p.root.start, p.root.end), (0, 21));
            let inl = &p.root.children;
            assert_eq!(inl[0].kind, NodeKind::Text);
            assert_eq!((inl[0].start, inl[0].end), (0, 5));
            assert_eq!(inl[1].kind, NodeKind::Emphasis);
            assert_eq!((inl[1].start, inl[1].end), (5, 9));
            assert_eq!((inl[1].children[0].start, inl[1].children[0].end), (6, 8));
            assert_eq!(inl[2].kind, NodeKind::Text);
            assert_eq!((inl[2].start, inl[2].end), (9, 10));
            assert_eq!(inl[3].kind, NodeKind::CodeSpan);
            assert_eq!((inl[3].start, inl[3].end), (10, 13));
            assert_eq!(inl[4].kind, NodeKind::Text);
            assert_eq!((inl[4].start, inl[4].end), (13, 14));
            assert_eq!(inl[5].kind, NodeKind::Link);
            assert_eq!((inl[5].start, inl[5].end), (14, 21));
            assert_eq!(inl[5].destination.as_deref(), Some("/u"));
            assert_eq!((inl[5].children[0].start, inl[5].children[0].end), (15, 16));
        }
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert_eq!(doc.normalize_v1(), h0(b"x\n\npara *em* `c` [l](/u)\n"));
}

#[test]
fn fenced_code_content_is_owner_relative() {
    // The mandatory FencedCode.content edge case (task #16): the Owner's
    // coverage begins BEFORE the semantic fence content.
    // "x\n\n```\nc\n```\n": fence opener at 3, body "c\n" [7,9), closer
    // run ends at 12. Owner 1 base = 3; fence span [3,12) -> (0,9);
    // content (7,9) -> (4,6). Leaving `content` absolute while rebasing
    // only the outer span would produce (7,9) here.
    let doc = build(b"x\n\n```\nc\n```\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    match &owners[1].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::FencedCode);
            assert_eq!((p.root.start, p.root.end), (0, 9));
            assert_eq!(p.root.content, Some((4, 6)));
        }
        other => panic!("expected Syntax, got {other:?}"),
    }
    // Export restores the absolute content interval exactly as H0 has it.
    let exported = doc.normalize_v1();
    validate_normalized(&exported, Some(b"x\n\n```\nc\n```\n".as_slice()))
        .expect("export violates NORMALIZED-RESULT-v1");
    assert_eq!(exported, h0(b"x\n\n```\nc\n```\n"));
    assert_eq!(
        exported.root.children[1].content,
        Some((7, 9)),
        "exported FencedCode.content must be the H0-absolute interval"
    );
}

#[test]
fn retained_payload_of_every_owner_passes_the_relative_audit() {
    // The recursive no-absolute-coordinate audit (task #40) over a
    // document whose Owners all have non-zero bases except the first.
    let doc = build(b"# h\n\npara *b* [l](/u)\n\n- l\n\n> q\n\n```\nc\n```\n\n[d]: /d\n");
    markit_mdbench_horse_a::validate_owner_relative_payload(&doc)
        .expect("a retained coordinate escaped its Owner-relative range");
    markit_mdbench_horse_a::validate_ready(&doc)
        .expect("the READY invariants must hold on the built state");
}

#[test]
fn an_absolute_coordinate_leak_is_caught_by_the_audit() {
    // Mutation checks (task #50) on a source where the fence Owner's
    // base is large and its coverage small, so the ABSOLUTE coordinates
    // exceed the Owner-relative range and the audit must catch them.
    //
    // Byte layout: 'a'x48 [0,48), LF(48), LF(49), fence opener [50,54),
    // body "c\n" [54,56), closer run [56,59), LF(59), LF(60), "z\n"
    // [61,63). p1 = 50, p2 = 61: Owner 1 = [50,61), coverage 11.
    // Retained: fence span (0,9), content (4,6).
    let src = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\n```\nc\n```\n\nz\n";
    let mut doc = build(src);
    // Three records: the balanced build makes Owner 1 the root record.
    assert_eq!(doc.owners.records(), 3);
    {
        let payload = &mut doc.owners.root.as_mut().unwrap().owner.payload;
        let OwnerPayload::Syntax(p) = payload else {
            panic!("expected a Syntax payload on the fence Owner");
        };
        assert_eq!(p.root.kind, NodeKind::FencedCode);
        assert_eq!((p.root.start, p.root.end), (0, 9));
        assert_eq!(p.root.content, Some((4, 6)));
        // Perturbation 1: leave Fence.content absolute (task #50).
        p.root.content = Some((54, 56));
    }
    let err = markit_mdbench_horse_a::validate_owner_relative_payload(&doc)
        .expect_err("an absolute FencedCode.content must fail the relative audit");
    assert!(
        err.contains("escapes its fence span") || err.contains("absolute coordinate leaked"),
        "unexpected audit error: {err}"
    );
    {
        // Perturbation 2: leave the outer fence span absolute.
        let payload = &mut doc.owners.root.as_mut().unwrap().owner.payload;
        let OwnerPayload::Syntax(p) = payload else {
            panic!("expected a Syntax payload on the fence Owner");
        };
        p.root.content = Some((4, 6));
        p.root.end = 59;
    }
    let err = markit_mdbench_horse_a::validate_owner_relative_payload(&doc)
        .expect_err("an absolute span end must fail the relative audit");
    assert!(
        err.contains("absolute coordinate leaked"),
        "unexpected audit error: {err}"
    );
}

#[test]
fn prefix_shift_leaves_the_relative_payload_unchanged() {
    // Representation test (task #41) — NOT an incremental-retention
    // claim: two independent full builds of `source A` and `prefix +
    // source A` must retain an equivalent relative payload for the
    // otherwise-identical suffix block, proving the payload is not
    // absolute-position encoded.
    let a = b"aaa\n\nbbb\n";
    let shifted = b"xxxx\n\naaa\n\nbbb\n";

    let doc_a = build(a);
    let doc_shifted = build(shifted);

    let owners_a = doc_a.owners.owners_in_order();
    let owners_shifted = doc_shifted.owners.owners_in_order();
    assert_eq!(owners_a.len(), 2);
    // The prefix adds a third block ("xxxx") in the shifted source.
    assert_eq!(owners_shifted.len(), 3);

    // The suffix block "bbb": relative spans identical modulo Owner
    // identity/base. Byte layouts: A: "bbb" [5,8), Owner base 5 ->
    // (0,3). Shifted: "bbb" [11,14), Owner base 11 -> (0,3).
    assert_eq!(owners_a[1].payload, owners_shifted[2].payload);
    // The "aaa" block is likewise relative-identical (its Owner base
    // differs, the payload does not).
    assert_eq!(owners_a[0].payload, owners_shifted[1].payload);

    // ...while the absolute export tracks each source exactly.
    assert_eq!(doc_a.normalize_v1(), h0(a));
    assert_eq!(doc_shifted.normalize_v1(), h0(shifted));
}
