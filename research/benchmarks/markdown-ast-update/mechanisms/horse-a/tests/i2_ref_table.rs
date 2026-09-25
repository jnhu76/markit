//! I2 T2e — the document-global RefTable and reference-sensitive
//! semantics (spec §2.4; data-model §10; tasks #6/#27/#28).
//!
//! Frozen rules under test:
//!
//! ```text
//! ReadyDocument.refs = the document-global source-order projection of
//!   all retained ReferenceDefinition facts (duplicates included);
//! lookup is the shared semantic core's first-wins resolution — no
//!   winner repair logic exists here;
//! the FINAL RefTable exists before any reference-sensitive semantic
//!   materialization: a reference use BEFORE its definition still
//!   resolves in the Owner containing the use.
//! ```

mod common;

use common::{build, h0};
use markit_mdbench_horse_a::{NormalizeV1, OwnerPayload};
use markit_mdbench_oracle::normalized::NodeKind;

fn find_first_node(
    node: &markit_mdbench_oracle::normalized::Node,
    kind: NodeKind,
) -> Option<&markit_mdbench_oracle::normalized::Node> {
    if node.kind == kind {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_first_node(c, kind))
}

#[test]
fn a_single_definition_projects_into_the_ref_table() {
    let doc = build(b"[l]: /u\n\npara [x][l]\n");
    assert_eq!(doc.refs.entries(), &[("l".to_string(), "/u".to_string())]);
    markit_mdbench_horse_a::validate_ref_table_projection(&doc).expect("projection");
    // The reference link in the second Owner resolved under the table.
    let owners = doc.owners.owners_in_order();
    let OwnerPayload::Syntax(p) = &owners[1].payload else {
        panic!("expected Syntax");
    };
    let link = find_first_node(&p.root, NodeKind::ReferenceLink).expect("resolved reference link");
    assert_eq!(link.label.as_deref(), Some("l"));
    assert_eq!(link.destination.as_deref(), Some("/u"));
    assert_eq!(doc.normalize_v1(), h0(b"[l]: /u\n\npara [x][l]\n"));
}

#[test]
fn duplicate_definitions_are_all_retained_and_first_wins_lookup_holds() {
    // "[l]: /u\n\n[l]: /v\n\nx [l]\n": both facts are source-order
    // entries; the shared core's first-wins lookup resolves /u. The
    // duplicate ReferenceDefinition node keeps its own destination — it
    // is source semantic state, never a second mutable index.
    let doc = build(b"[l]: /u\n\n[l]: /v\n\nx [t][l]\n");
    assert_eq!(
        doc.refs.entries(),
        &[
            ("l".to_string(), "/u".to_string()),
            ("l".to_string(), "/v".to_string())
        ]
    );
    let owners = doc.owners.owners_in_order();
    let OwnerPayload::Syntax(d0) = &owners[0].payload else {
        panic!("expected Syntax");
    };
    let OwnerPayload::Syntax(d1) = &owners[1].payload else {
        panic!("expected Syntax");
    };
    assert_eq!(d0.root.destination.as_deref(), Some("/u"));
    assert_eq!(d1.root.destination.as_deref(), Some("/v"));
    let OwnerPayload::Syntax(p) = &owners[2].payload else {
        panic!("expected Syntax");
    };
    let link = find_first_node(&p.root, NodeKind::ReferenceLink).expect("resolved");
    assert_eq!(
        link.destination.as_deref(),
        Some("/u"),
        "first definition wins"
    );
    markit_mdbench_horse_a::validate_ref_table_projection(&doc).expect("projection");
    assert_eq!(doc.normalize_v1(), h0(b"[l]: /u\n\n[l]: /v\n\nx [t][l]\n"));
}

#[test]
fn a_shadowing_later_definition_keeps_first_wins_semantics() {
    let doc = build(b"[a]: /1\n\nx [v][a]\n\n[a]: /2\n");
    assert_eq!(
        doc.refs.entries(),
        &[
            ("a".to_string(), "/1".to_string()),
            ("a".to_string(), "/2".to_string())
        ]
    );
    let owners = doc.owners.owners_in_order();
    let OwnerPayload::Syntax(p) = &owners[1].payload else {
        panic!("expected Syntax");
    };
    let link = find_first_node(&p.root, NodeKind::ReferenceLink).expect("resolved");
    assert_eq!(link.destination.as_deref(), Some("/1"));
    markit_mdbench_horse_a::validate_ref_table_projection(&doc).expect("projection");
    assert_eq!(doc.normalize_v1(), h0(b"[a]: /1\n\nx [v][a]\n\n[a]: /2\n"));
}

#[test]
fn a_reference_use_before_its_definition_resolves_under_the_final_table() {
    // Task #28's order discriminator: the use Owner is materialized only
    // after the complete document RefTable exists, so the forward
    // reference resolves exactly like H0. Materializing each Owner as
    // soon as it was parsed would leave this link unresolved.
    let doc = build(b"para [x][l]\n\n[l]: /u\n");
    assert_eq!(doc.refs.entries(), &[("l".to_string(), "/u".to_string())]);
    let owners = doc.owners.owners_in_order();
    let OwnerPayload::Syntax(p) = &owners[0].payload else {
        panic!("expected Syntax");
    };
    let link = find_first_node(&p.root, NodeKind::ReferenceLink)
        .expect("forward reference must resolve under the FINAL RefTable");
    assert_eq!(link.destination.as_deref(), Some("/u"));
    assert_eq!(doc.normalize_v1(), h0(b"para [x][l]\n\n[l]: /u\n"));
}

#[test]
fn an_unresolved_reference_decomposes_to_literal_text() {
    let doc = build(b"x [z]\n");
    assert!(doc.refs.is_empty());
    let owners = doc.owners.owners_in_order();
    let OwnerPayload::Syntax(p) = &owners[0].payload else {
        panic!("expected Syntax");
    };
    assert!(find_first_node(&p.root, NodeKind::ReferenceLink).is_none());
    assert_eq!(doc.normalize_v1(), h0(b"x [z]\n"));
}

#[test]
fn definitions_across_owners_project_in_source_order() {
    // "> [q]: /qu\n\n[t]: /top\n": the nested definition inside the
    // quote Owner is a real fact (the grammar pushes it into the
    // container), and it projects BEFORE the later top-level one.
    let doc = build(b"> [q]: /qu\n\n[t]: /top\n");
    assert_eq!(
        doc.refs.entries(),
        &[
            ("q".to_string(), "/qu".to_string()),
            ("t".to_string(), "/top".to_string())
        ]
    );
    markit_mdbench_horse_a::validate_ref_table_projection(&doc).expect("projection");
    assert_eq!(doc.normalize_v1(), h0(b"> [q]: /qu\n\n[t]: /top\n"));
}

#[test]
fn a_definition_inside_a_fence_is_fence_body_not_a_fact() {
    let doc = build(b"```\n[x]: /y\n```\n");
    assert!(
        doc.refs.is_empty(),
        "fence body must not produce a definition fact"
    );
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    let OwnerPayload::Syntax(p) = &owners[0].payload else {
        panic!("expected Syntax");
    };
    assert_eq!(p.root.kind, NodeKind::FencedCode);
    assert!(find_first_node(&p.root, NodeKind::ReferenceDefinition).is_none());
    assert_eq!(doc.normalize_v1(), h0(b"```\n[x]: /y\n```\n"));
}

#[test]
fn a_defact_table_matches_the_shared_block_pass_table_exactly() {
    // RefTable equality against the existing semantic result: the
    // retained table equals the shared full block parse's definition
    // table for the same source (source-order facts, duplicates kept).
    for src in [
        &b"[l]: /u\n"[..],
        &b"[l]: /u\n\n[l]: /v\n\nx [t][l]\n"[..],
        &b"[a]: /1\n\nx [v][a]\n\n[a]: /2\n"[..],
        &b"> [q]: /qu\n\n[t]: /top\n"[..],
        &b"```\n[x]: /y\n```\n"[..],
        &b"no definitions at all\n"[..],
    ] {
        let doc = build(src);
        let mut noop = markit_mdbench_common::NoopWorkSink;
        let region = markit_mdbench_shared_grammar::parse_region(src, 0, src.len(), &mut noop);
        assert_eq!(
            doc.refs.entries(),
            region.defs.entries(),
            "RefTable != shared block-pass table for {src:?}"
        );
    }
}
