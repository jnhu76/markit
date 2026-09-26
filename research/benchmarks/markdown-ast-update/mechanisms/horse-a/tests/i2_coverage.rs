//! I2 T2b — canonical physical-first-line coverage (spec §3.2). The
//! frozen rules under test:
//!
//! ```text
//! c0 = 0; c_i = p_i (TopLevelStart physical line start); c_k = source_len
//! leading trivia -> first Owner; interstitial/trailing bytes -> left Owner
//! empty source -> empty OwnerSeq
//! frozen root-blank class -> exactly one TriviaOnly Owner
//! TAB/CR are ordinary text under BENCH-GRAMMAR-v1 (real paragraph Owners)
//! coverage union = [0, source_len), gaps = 0, overlap = 0
//! ```
//!
//! Every offset below is hand-derived from the byte layout of the
//! literal source.

mod common;

use common::{build, h0};
use markit_mdbench_horse_a::{NormalizeV1, OwnerPayload};

#[test]
fn leading_trivia_belongs_to_the_first_owner_not_a_trivia_record() {
    // "\n\npara\n": blank lines [0,1) and [1,2); p0 = 2; len 7.
    // One Syntax Owner [0,7) — NOT one TriviaOnly + one Syntax Owner.
    let doc = build(b"\n\npara\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].coverage_len, 7);
    assert!(matches!(owners[0].payload, OwnerPayload::Syntax(_)));
    // the paragraph's semantic span stays absolute-equal in relative
    // coordinates (Owner base 0): the leading trivia is owned coverage,
    // so the relative span start is legally > 0 (data-model §3.3).
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => assert_eq!((p.root.start, p.root.end), (2, 6)),
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert_eq!(doc.normalize_v1(), h0(b"\n\npara\n"));
}

#[test]
fn interstitial_blank_bytes_belong_to_the_left_owner() {
    // "a\n\nb\n": a(0) LF(1) LF(2) b(3) LF(4). p1 = 3; the blank byte 2
    // is inside Owner 0's coverage [0,3).
    let doc = build(b"a\n\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].coverage_len, 3);
    assert_eq!(owners[1].coverage_len, 2);
}

#[test]
fn trailing_blank_bytes_belong_to_the_last_owner() {
    // "para\n\n": one Owner covering everything up to the source end —
    // the trailing blank never becomes a new fake Owner.
    let doc = build(b"para\n\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].coverage_len, 6);

    // "ab\n\ncd\n\n": trailing blank byte 7 is inside Owner 1's [4,8).
    let doc = build(b"ab\n\ncd\n\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].coverage_len, 4);
    assert_eq!(owners[1].coverage_len, 4);
}

#[test]
fn multiple_blank_lines_all_belong_left() {
    // "a\n\n\n\nb\n": blanks [2,3), [3,4), [4,5); p1 = 5; len 7.
    let doc = build(b"a\n\n\n\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].coverage_len, 5);
    assert_eq!(owners[1].coverage_len, 2);
}

#[test]
fn empty_document_has_no_owner() {
    let doc = build(b"");
    assert_eq!(doc.owners.records(), 0);
    assert_eq!(doc.owners.total_bytes(), 0);
}

#[test]
fn blank_only_document_is_exactly_one_triviaonly_owner() {
    // "\n\n": the frozen root-blank class, len 2 — one TriviaOnly Owner
    // [0,2), no payload.
    let doc = build(b"\n\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].coverage_len, 2);
    assert!(matches!(owners[0].payload, OwnerPayload::TriviaOnly));
    // no definition facts, no normalized trivia node
    assert!(doc.refs.is_empty());
    assert_eq!(doc.normalize_v1(), h0(b"\n\n"));

    // Spaces-only without a final LF is the same blank class to the
    // shared grammar (B1), so it is the same single TriviaOnly Owner.
    let doc = build(b"   ");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].coverage_len, 3);
    assert!(matches!(owners[0].payload, OwnerPayload::TriviaOnly));
    assert_eq!(doc.normalize_v1(), h0(b"   "));

    // Mixed spaces/LF lines: " \n \n" — both lines are SPACES* LF.
    let doc = build(b" \n \n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].coverage_len, 4);
    assert!(matches!(owners[0].payload, OwnerPayload::TriviaOnly));
    assert_eq!(doc.normalize_v1(), h0(b" \n \n"));
}

#[test]
fn tab_only_content_is_ordinary_paragraph_text_not_trivia() {
    // D1: a tab is NOT whitespace. "\t\n" is a real paragraph Owner.
    let doc = build(b"\t\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert!(matches!(owners[0].payload, OwnerPayload::Syntax(_)));
    assert_eq!(doc.normalize_v1(), h0(b"\t\n"));

    // A tab line between paragraphs continues the paragraph (I1: the
    // tab line is content), so the whole document is one Owner.
    let doc = build(b"a\n\t\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert!(matches!(owners[0].payload, OwnerPayload::Syntax(_)));
    assert_eq!(doc.normalize_v1(), h0(b"a\n\t\nb\n"));
}

#[test]
fn carriage_returns_are_ordinary_text_not_trivia() {
    // CR is not whitespace under BENCH-GRAMMAR-v1: every line here is
    // paragraph text, so the document is one Syntax Owner — never a
    // TriviaOnly record (task #13).
    let doc = build(b"a\r\n\r\nb\r\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert!(matches!(owners[0].payload, OwnerPayload::Syntax(_)));
    assert_eq!(doc.normalize_v1(), h0(b"a\r\n\r\nb\r\n"));
}

#[test]
fn coverage_comes_from_physical_line_starts_not_semantic_span_starts() {
    // "  # indented heading\n   text\n": the heading's SEMANTIC span
    // starts at byte 2, but its PHYSICAL line starts at 0. Deriving
    // coverage from span starts would leave bytes 0..2 unowned (sum
    // != source_len) — the frozen authority is the physical line start.
    // Byte layout: 2 spaces + "# indented heading" (18) + LF = 21; then
    // "   text\n" = 8. Total 29.
    let doc = build(b"  # indented heading\n   text\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].coverage_len, 21);
    assert_eq!(owners[1].coverage_len, 8);
    assert_eq!(doc.owners.total_bytes(), 29);
    assert_eq!(doc.source_len, 29);
}
