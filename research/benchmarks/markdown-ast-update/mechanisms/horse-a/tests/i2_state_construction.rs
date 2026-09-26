//! I2 T2a — state construction through the public seam:
//!
//! ```text
//! full_build(&Source, &mut WorkSink) -> Result<ReadyDocument, BuildError>
//! ```
//!
//! Every Owner count, coverage length, payload kind, and span below is
//! hand-derived from the byte layout of the literal source and the frozen
//! coverage rule (spec §3.2: c0 = 0, c_i = physical first-line start,
//! c_k = source_len).

mod common;

use common::{build, build_with_id, h0};
use markit_mdbench_common::SourceId;
use markit_mdbench_horse_a::{InterpretationId, NormalizeV1, OwnerPayload, ReadyDocument};
use markit_mdbench_oracle::normalized::NodeKind;

#[test]
fn empty_document_yields_an_empty_ownerseq() {
    let doc = build(b"");
    assert_eq!(doc.source_len, 0);
    assert_eq!(doc.owners.records(), 0);
    assert_eq!(doc.owners.total_bytes(), 0);
    assert!(doc.owners.is_empty());
    assert!(doc.refs.is_empty());
    // No fake syntax Owner for the empty file (frozen case, task #12).
    // The normalized empty document is still valid and equal to H0.
    assert_eq!(doc.normalize_v1(), h0(b""));
}

#[test]
fn one_paragraph_is_one_syntax_owner() {
    // "hello\n": bytes 0..6; no blank line, so no interior boundary and
    // no certificate.
    let doc = build(b"hello\n");
    assert_eq!(doc.source_len, 6);
    assert_eq!(doc.owners.records(), 1);
    assert_eq!(doc.owners.total_bytes(), 6);
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners[0].coverage_len, 6);
    assert!(owners[0].outgoing_restart.is_none());
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::Paragraph);
            // relative == absolute at base 0; the semantic span ends at
            // the paragraph's last content byte, while the Owner's
            // coverage [0,6) additionally owns the terminating LF.
            assert_eq!((p.root.start, p.root.end), (0, 5));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_export_ready(&doc, b"hello\n");
}

#[test]
fn a_heading_is_one_syntax_owner_with_its_level() {
    // "# t\n": heading span [0,3), level 1, content [2,3).
    let doc = build(b"# t\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::Heading);
            assert_eq!(p.root.level, Some(1));
            assert_eq!((p.root.start, p.root.end), (0, 3));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_export_ready(&doc, b"# t\n");
}

#[test]
fn a_reference_definition_is_one_owner_and_fills_the_ref_table() {
    // "[l]: /u\n": span [0,7); the document RefTable records the fact.
    let doc = build(b"[l]: /u\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::ReferenceDefinition);
            assert_eq!(p.root.label.as_deref(), Some("l"));
            assert_eq!(p.root.destination.as_deref(), Some("/u"));
            assert_eq!((p.root.start, p.root.end), (0, 7));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_eq!(doc.refs.entries(), &[("l".to_string(), "/u".to_string())]);
    assert_export_ready(&doc, b"[l]: /u\n");
}

#[test]
fn a_fence_is_one_owner_with_info_and_content_interval() {
    // "```rust\nx\n```\n": opener [0,4), body "x\n" [8..10), closer run at
    // [10,13); span [0,13), content (8,10) — relative == absolute at
    // base 0.
    let doc = build(b"```rust\nx\n```\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::FencedCode);
            assert_eq!(p.root.info.as_deref(), Some("rust"));
            assert_eq!(p.root.content, Some((8, 10)));
            assert_eq!((p.root.start, p.root.end), (0, 13));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_export_ready(&doc, b"```rust\nx\n```\n");
}

#[test]
fn a_quote_is_one_owner_for_the_whole_container() {
    // "> q\n> r\n": one root BlockQuote; both paragraphs are descendants.
    let doc = build(b"> q\n> r\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::BlockQuote);
            assert_eq!((p.root.start, p.root.end), (0, 7));
            assert_eq!(p.root.children.len(), 1);
            assert_eq!(p.root.children[0].kind, NodeKind::Paragraph);
            assert_eq!(p.root.children[0].children.len(), 2, "two text segments");
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_export_ready(&doc, b"> q\n> r\n");
}

#[test]
fn a_list_is_one_owner_for_all_items() {
    // "- a\n- b\n": one root List with two ListItems (W-A1: never split).
    let doc = build(b"- a\n- b\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::List);
            assert_eq!((p.root.start, p.root.end), (0, 7));
            assert_eq!(p.root.children.len(), 2);
            assert_eq!(p.root.children[0].kind, NodeKind::ListItem);
            assert_eq!(p.root.children[0].marker.as_deref(), Some("-"));
            assert_eq!(p.root.children[1].kind, NodeKind::ListItem);
            assert_eq!(p.root.children[1].marker.as_deref(), Some("-"));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    assert_export_ready(&doc, b"- a\n- b\n");
}

#[test]
fn multiple_top_level_blocks_become_multiple_owners_in_order() {
    // "# h\n\npara\n": bytes: "# h\n" 0..4, "\n" 4, "para\n" 5..10.
    // p0 = 0, p1 = 5; coverage [0,5) and [5,10). The interior boundary
    // c1 = 5 is certified by the blank barrier (cut 5, preceding LF 3).
    let doc = build(b"# h\n\npara\n");
    assert_eq!(doc.owners.records(), 2);
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners[0].coverage_len, 5);
    assert_eq!(owners[1].coverage_len, 5);
    match &owners[0].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::Heading);
            assert_eq!((p.root.start, p.root.end), (0, 3));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    match &owners[1].payload {
        OwnerPayload::Syntax(p) => {
            assert_eq!(p.root.kind, NodeKind::Paragraph);
            // absolute [5,9) -> relative (0,4) at base 5
            assert_eq!((p.root.start, p.root.end), (0, 4));
        }
        other => panic!("expected a Syntax payload, got {other:?}"),
    }
    // The boundary certificate belongs to the LEFT owner.
    let cert = owners[0]
        .outgoing_restart
        .as_ref()
        .expect("the blank before byte 5 certifies the interior boundary");
    assert_eq!(cert.support.preceding_lf, Some(3));
    assert_eq!(cert.support.blank_line, 4..5);
    assert!(owners[1].outgoing_restart.is_none());
    assert_export_ready(&doc, b"# h\n\npara\n");
}

#[test]
fn source_association_uses_the_substrate_identity() {
    // Task #24: reuse SourceId; carry source_len and the interpretation
    // identity. No parallel HorseASourceId exists anywhere.
    let doc: ReadyDocument = build_with_id(SourceId(42), b"hello\n");
    assert_eq!(doc.source_id, SourceId(42));
    assert_eq!(doc.source_len, 6);
    assert_eq!(doc.interpretation, InterpretationId::HORSE_A_V1);
}

fn assert_export_ready(doc: &ReadyDocument, src: &[u8]) {
    let exported = doc.normalize_v1();
    markit_mdbench_oracle::validate_normalized(&exported, Some(src))
        .expect("exported document violates NORMALIZED-RESULT-v1");
    assert_eq!(exported, h0(src), "export != H0 for {src:?}");
}
