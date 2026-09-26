//! I2 T2f — the primary correctness gate:
//!
//! ```text
//! normalize(Horse-A full build(source)) == normalize(H0 clean parse(source))
//! ```
//!
//! Full normalized structural equality (`PartialEq` over the frozen
//! NORMALIZED-RESULT-v1 vocabulary) — never a hash, checksum, block
//! count, or text output. The corpus below is the smallest deterministic
//! set covering every retained-state rule, reusing the I1 inert-observer
//! case shapes plus the frozen coverage/certificate edge sources.

mod common;

use common::assert_export_equals_h0;

/// The representative I2 corpus: every retained-state rule appears in at
/// least one case (empty, trivia-only, TAB/CR ordinary text, leading /
/// interstitial / trailing trivia, containers, fences, definitions and
/// references, duplicates, Unicode, unclosed fence, EOF shapes).
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
    b"para *b* [l](/u) [d]\n\n[d]: /d\n",
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
    b"para [l]\n\n[l]: /u\n",
    b"> [q]: /qu\n\n[t]: /top\n",
    b"```\n[x]: /y\n```\n",
    b"x\n\n```\nc\n```\n",
    b"\n\npara\n\n# h\n\n",
    // Adversarial thematic/list-marker-like lines (independent I2 review
    // §25): sources whose list-marker bytes could be mistaken for blank
    // evidence. Correctness regression coverage only — generated data is
    // never research evidence.
    b"* * *",
    b"- - -",
    b"-",
    b"- ",
    b"* * *\ntext",
    b"a\n\n* * *",
    // EOF-without-LF shapes: the final line has no terminator, so it is
    // ordinary content (or an empty-item marker line), never a trailing
    // blank barrier.
    b"para",
    b"a\n\nb",
    b"# h",
    b"- x",
    b"> q",
];

#[test]
fn full_build_matches_h0_on_the_representative_corpus() {
    for src in CORPUS {
        assert_export_equals_h0(src);
    }
}

#[test]
fn full_build_matches_h0_on_unicode_and_cjk() {
    let unicode = "# 標題\n\n段落 🎉 *強調* [リンク](/u)\n\n- 項目\n  - 子\n".as_bytes();
    assert_export_equals_h0(unicode);
}

/// The independent I2 review's exact P1 repro family: sources whose
/// list-marker-line events carry cuts that coincide with interior Owner
/// boundaries, where the (broad, pre-I1-corrective) event's "blank
/// line" starts at the left Owner's very base — support that cannot
/// satisfy the frozen persistence condition. The frozen rule
/// (data-model §8.3) is DO NOT PERSIST that certificate and keep
/// building READY; before the repair this aborted the whole build.
///
/// Until the I1 physical-root-blank corrective (PR #65) merges, this
/// branch still observes the old broad event set, so these cases pass
/// through the skip path; after it merges they pass through the
/// no-event path. I2 deliberately contains NO second lexical check to
/// distinguish the two — persistence-level skipping is seam-agnostic.
#[test]
fn a_non_persistable_optional_certificate_skips_instead_of_aborting_ready() {
    for src in [
        b"* * *\ntext\n".as_slice(),
        b"- - -\ntext\n".as_slice(),
        b"-\n# h\n".as_slice(),
        b"- \n- \n".as_slice(),
        b"a\n\n* * *\n:::".as_slice(),
    ] {
        let doc = assert_export_equals_h0(src);
        markit_mdbench_horse_a::validate_ready(&doc)
            .unwrap_or_else(|e| panic!("READY invariants for {src:?}: {e}"));
    }
}

#[test]
fn full_build_matches_h0_on_a_many_owner_document() {
    // Smoke test at scale for the construction-only balanced build: many
    // Owners, certificates at every interior boundary, H0 equality.
    let mut src = Vec::new();
    for i in 0..2000 {
        if i > 0 {
            src.push(b'\n');
            src.push(b'\n');
        }
        src.extend_from_slice(format!("para {i} body").as_bytes());
        src.push(b'\n');
    }
    let doc = assert_export_equals_h0(&src);
    assert_eq!(doc.owners.records(), 2000);
    assert!(doc.owners.has_safe());
    markit_mdbench_horse_a::validate_ready(&doc).expect("READY invariants at scale");
}

#[test]
fn full_build_matches_h0_when_every_owner_base_is_shifted() {
    // A prefix insertion shifts every later absolute base; the export
    // must still equal H0 (T5-style relative-coordinate equivalence,
    // exercised through the full builder).
    assert_export_equals_h0(b"xxxx\n\n# h\n\npara *b* [l](/u)\n\n- l\n\n[d]: /d\n");
}
