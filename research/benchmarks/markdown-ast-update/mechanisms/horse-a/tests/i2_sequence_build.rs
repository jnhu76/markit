//! I2 construction-only OwnerSeq build invariants (tasks #31–#34; spec
//! §12.5). Only full-build construction is tested here — the I3 slice
//! owns the mutation/navigation operators and their property tests.
//!
//! Middle-split heights of the O(M) builder follow `h(n) = 1 +
//! max(h(n/2), h(n - n/2 - 1))`, `h(0)=0`, `h(1)=1`:
//!
//! ```text
//! n : 1  2  3  4  5  6  7  8
//! h : 1  2  2  3  3  3  3  4
//! ```

mod common;

use common::build;
use markit_mdbench_horse_a::{validate_full_build_tree, OwnerPayload};

/// k paragraph Owners separated by single blank lines:
/// "a\n\nb\n\nc\n..." — each paragraph is 2 bytes ("x\n").
fn source_with_k_owners(k: usize) -> Vec<u8> {
    let letters = b"abcdefgh";
    let mut src = Vec::new();
    for (i, &letter) in letters.iter().take(k).enumerate() {
        if i > 0 {
            src.push(b'\n');
            src.push(b'\n');
        }
        src.push(letter);
        src.push(b'\n');
    }
    src
}

#[test]
fn balanced_build_metadata_is_coherent_for_one_to_eight_owners() {
    let heights = [1usize, 2, 2, 3, 3, 3, 3, 4];
    for k in 1..=8usize {
        let src = source_with_k_owners(k);
        let doc = build(&src);
        let seq = &doc.owners;
        assert_eq!(seq.records(), k, "records for k={k}");
        assert_eq!(seq.total_bytes(), src.len(), "bytes for k={k}");
        assert_eq!(
            seq.root.as_ref().unwrap().height,
            heights[k - 1] as u32,
            "height for k={k}"
        );
        validate_full_build_tree(seq)
            .unwrap_or_else(|e| panic!("tree invariants broken for k={k}: {e}"));

        // In-order traversal preserves source order and the running
        // byte-weight base: consecutive coverages tile [0, source_len).
        let mut expected_base = 0usize;
        let mut seen = 0usize;
        seq.for_each_in_order(|base, owner| {
            assert_eq!(base, expected_base, "in-order base for record {seen}");
            assert!(owner.coverage_len > 0);
            assert!(matches!(owner.payload, OwnerPayload::Syntax(_)));
            expected_base += owner.coverage_len;
            seen += 1;
        });
        assert_eq!(seen, k);
        assert_eq!(expected_base, src.len());
    }
}

#[test]
fn subtree_has_safe_summarizes_persistent_certificates() {
    // "a\n\nb\n\nc\n\nd\n": four paragraph Owners, each separated by a
    // real root blank — the interior boundaries 3, 6, 9 are certified;
    // the last Owner's outgoing boundary is EOF and carries nothing.
    // Four records: root = [Owner0, Owner1] | pivot Owner2 | [Owner3].
    let doc = build(b"a\n\nb\n\nc\n\nd\n");
    let root = doc.owners.root.as_ref().unwrap();
    assert_eq!(root.agg.subtree_records, 4);
    assert_eq!(root.agg.subtree_bytes, 11); // 4 paragraphs + 3 blank LFs
                                            // Root aggregate: true — certified boundaries exist.
    assert!(root.agg.subtree_has_safe);
    // Left subtree (Owners 0,1): contains certified boundaries.
    let left = root.left.as_ref().unwrap();
    assert!(left.agg.subtree_has_safe);
    // Right subtree (Owner 3): the EOF boundary carries nothing —
    // subtree_has_safe stays false there (task #34).
    let right = root.right.as_ref().unwrap();
    assert!(!right.agg.subtree_has_safe);
    validate_full_build_tree(&doc.owners).expect("tree invariants");
}

#[test]
fn a_certificate_free_document_reports_has_safe_false() {
    // "a\nb\n": one paragraph, one Owner, no blank line, no certificate.
    let doc = build(b"a\nb\n");
    assert_eq!(doc.owners.records(), 1);
    assert!(!doc.owners.has_safe());
    validate_full_build_tree(&doc.owners).expect("tree invariants");
}

#[test]
fn aggregates_are_exact_over_the_root() {
    let doc = build(b"# h\n\npara\n\n> q\n\n```\nc\n```\n\n[l]: /d\n");
    let owners = doc.owners.owners_in_order();
    let sum: usize = owners.iter().map(|o| o.coverage_len).sum();
    assert_eq!(doc.owners.total_bytes(), sum);
    assert_eq!(doc.owners.total_bytes(), doc.source_len);
    assert_eq!(doc.owners.records(), owners.len());
}
