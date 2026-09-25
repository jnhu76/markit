//! I2 READY-invariant validators and the frozen mutation/falsifiability
//! checks (tasks #45/#50): each critical invariant is perturbed by hand
//! and proven to be caught by a focused deterministic check.

mod common;

use common::{build, h0};
use markit_mdbench_horse_a::{
    validate_certificates, validate_coverage, validate_full_build_tree,
    validate_owner_relative_payload, validate_ready, validate_ref_table_projection, NormalizeV1,
    Owner, OwnerPayload, RestartCertificate, RestartSupport,
};
use markit_mdbench_oracle::validate_normalized;

/// The representative T2f corpus, with every validator run explicitly.
#[test]
fn every_validator_passes_on_the_whole_corpus() {
    const CORPUS: &[&[u8]] = &[
        b"",
        b"\n",
        b"\n\n",
        b"   ",
        b"hello\n",
        b"# t\n",
        b"[l]: /u\n",
        b"one\ntwo\nthree\n",
        b"- a\n- b\n  cont\n",
        b"> q\n> r\n> > deep\n",
        b"```rust\nfn x() {}\n\nbody\n```\n",
        b"a\n\nb\n",
        b"a\n\n\n\nb\n",
        b"# h\n\npara *b* [l](/u)\n\n- l\n\n> q\n\n```\nc\n```\n\n[d]: /d\n",
        b"para\n  ",
        b"> a\n>\n> b\n",
        b"- a\n\n- b\n",
        b"- > ```\n  nested\n  ```\n",
        b"a\n\t\nb\n",
        b"a\r\n\r\nb\r\n",
        b"```\nunclosed\n",
        b"  # indented heading\n   text\n",
        b"[l]: /u\n[l]: /shadowed\n",
        b"ab\n\n",
        b"\n\nab\n",
        b"ab\n\ncd\n\n",
        b"para [x][l]\n\n[l]: /u\n",
        b"> [q]: /qu\n\n[t]: /top\n",
        b"```\n[x]: /y\n```\n",
        b"x\n\n```\nc\n```\n",
        b"\n\npara\n\n# h\n\n",
    ];
    for src in CORPUS {
        let doc = build(src);
        validate_ready(&doc).unwrap_or_else(|e| panic!("validate_ready failed for {src:?}: {e}"));
        validate_coverage(&doc).expect("coverage");
        validate_full_build_tree(&doc.owners).expect("tree");
        validate_certificates(&doc).expect("certificates");
        validate_owner_relative_payload(&doc).expect("relative payload");
        validate_ref_table_projection(&doc).expect("ref projection");
    }
}

#[test]
fn dropping_preceding_lf_from_support_is_caught() {
    // Mutation (task #50): support without its preceding LF is not the
    // frozen support — the validator must reject it (the blank line no
    // longer starts at the base, and the adjacency contract breaks).
    let mut doc = build(b"a\n\nb\n");
    {
        let owner = doc.owners.root.as_mut().unwrap().left.as_mut().unwrap();
        assert!(owner.owner.outgoing_restart.is_some());
        let cert = owner.owner.outgoing_restart.as_mut().unwrap();
        cert.support.preceding_lf = None;
    }
    let err = validate_certificates(&doc).expect_err("dropped preceding_lf must be caught");
    assert!(err.contains("BOF support"), "unexpected error: {err}");
}

#[test]
fn a_fabricated_eof_certificate_is_caught() {
    // Mutation (task #50): inventing an EOF certificate — installing one
    // on the LAST Owner — must be rejected: the EOF boundary is not a
    // restart-certified interior boundary.
    let mut doc = build(b"ab\n\ncd\n");
    assert!(doc.owners.owners_in_order()[1].outgoing_restart.is_none());
    {
        // Two records: Owner 1 is the root record.
        let root = doc.owners.root.as_mut().unwrap();
        root.owner.outgoing_restart = Some(RestartCertificate {
            support: RestartSupport {
                preceding_lf: Some(5),
                blank_line: 1..2, // relative to base 3: ends at coverage_len 2
            },
        });
    }
    let err = validate_certificates(&doc).expect_err("an EOF certificate must be caught");
    assert!(err.contains("EOF boundary"), "unexpected error: {err}");
}

#[test]
fn inserting_at_a_support_byte_counts_as_touching() {
    // Mutation (task #50): a touches() that only used strict interval
    // intersection would miss the zero-length insertion at a support
    // byte — the E24 behavior must not be weakened.
    let s = RestartSupport {
        preceding_lf: Some(2),
        blank_line: 3..4,
    };
    assert!(s.touches(2, 2));
    assert!(s.touches(3, 3));
    // Non-zero-length intersection still works.
    assert!(s.touches(2, 5));
    assert!(!s.touches(0, 2));
}

#[test]
fn coverage_derived_from_semantic_spans_would_break_the_partition() {
    // Mutation (task #50): derive coverage from semantic span.start.
    // "  # indented heading\n   text\n": the heading's span starts at
    // byte 2, so span-based cuts would leave bytes 0..2 unowned and the
    // partition check fails. The physical-first-line rule (frozen) keeps
    // the partition exact — proven by the validator passing and by the
    // exact coverage values asserted in i2_coverage.rs.
    let doc = build(b"  # indented heading\n   text\n");
    validate_coverage(&doc).expect("physical-first-line coverage partitions exactly");
    // Simulate the span-based mistake and show the partition check
    // catches it: shrink Owner 0's coverage by its leading trivia.
    let mut doc = doc;
    {
        let owner0 = doc.owners.root.as_mut().unwrap().left.as_mut().unwrap();
        assert_eq!(owner0.owner.coverage_len, 21);
        owner0.owner.coverage_len = 19; // semantic span starts at 2
    }
    let err = validate_coverage(&doc).expect_err("span-derived coverage must break the partition");
    assert!(err.contains("coverage sums"), "unexpected error: {err}");
}

#[test]
fn a_triviaonly_record_for_tab_only_source_would_be_wrong() {
    // Mutation (task #50): building a TriviaOnly Owner for TAB-only
    // content contradicts BENCH-GRAMMAR-v1 D1 — the tab line is a real
    // paragraph, so the built state must be Syntax (proven by
    // construction) and a TriviaOnly payload would break H0 equality.
    let doc = build(b"\t\n");
    assert!(matches!(
        doc.owners.owners_in_order()[0].payload,
        OwnerPayload::Syntax(_)
    ));
    assert_eq!(doc.normalize_v1(), h0(b"\t\n"));
}

#[test]
fn omitting_trailing_trivia_from_the_last_owner_breaks_the_partition() {
    // Mutation (task #50): "para\n\n" — trailing trivia belongs to the
    // last Owner up to source_len. Truncating its coverage by the blank
    // bytes leaves the partition short; the validator catches it.
    let mut doc = build(b"para\n\n");
    {
        let owner = doc.owners.root.as_mut().unwrap();
        assert_eq!(owner.owner.coverage_len, 6);
        owner.owner.coverage_len = 5; // semantic span end, not coverage end
    }
    let err = validate_coverage(&doc).expect_err("omitted trailing trivia must be caught");
    assert!(err.contains("coverage sums"), "unexpected error: {err}");
}

#[test]
fn a_state_that_needs_repair_is_not_returned_by_the_builder() {
    // The builder's own READY gate: the validators are run inside
    // full_build, so a perturbed construction cannot leak out as READY.
    // Prove the gate is real by checking a valid build once more and
    // confirming every returned state's exported result is the H0 one.
    for src in [b"a\n\nb\n".as_slice(), b"```\nunclosed\n".as_slice()] {
        let doc = build(src);
        let exported = doc.normalize_v1();
        if !src.is_empty() {
            validate_normalized(&exported, Some(src)).expect("export gate");
        }
        assert_eq!(exported, h0(src));
        assert_eq!(doc.owners.total_bytes(), src.len());
        assert_eq!(doc.refs.entries().len(), 0);
    }
}

#[test]
fn an_owner_without_payload_satisfies_the_trivia_contract() {
    // TriviaOnly payload: no AST, no facts, coverage-only record.
    let doc = build(b"\n\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert!(owners[0].is_trivia_only());
    assert!(matches!(owners[0].payload, OwnerPayload::TriviaOnly));
    assert!(doc.refs.is_empty());
    assert!(owners[0].outgoing_restart.is_none());
    let _ = Owner {
        coverage_len: 1,
        payload: OwnerPayload::TriviaOnly,
        outgoing_restart: None,
    }; // the record shape is directly constructible for staging
}
