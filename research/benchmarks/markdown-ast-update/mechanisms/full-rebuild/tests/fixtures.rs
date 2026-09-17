//! R4 golden grammar fixture gate (task contract §11).
//!
//! The 43 hand-authored fixtures are independent semantic authority:
//! their expected trees are TRUE BY DECLARATION under
//! BENCH-GRAMMAR-v1 + NORMALIZED-RESULT-v1. H0 parses each source and
//! the COMPLETE normalized tree is compared structurally — never by
//! checksum, never against a tree the same parser produced.

use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::fixture::{load_fixtures, repo_fixture_dir};

#[test]
fn all_43_golden_fixtures_pass() {
    let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load and validate");
    assert_eq!(fixtures.len(), 43, "the frozen fixture count is 43");

    let mut failures = Vec::new();
    for f in &fixtures {
        let actual = parse_document(f.source.as_bytes());
        if actual != f.expected {
            failures.push(format!("{} ({})", f.id, f.name));
        }
    }
    assert!(
        failures.is_empty(),
        "fixtures FAILED ({}/43 passed):\n  {}",
        43 - failures.len(),
        failures.join("\n  ")
    );
}

#[test]
fn fixture_comparison_is_not_tautological() {
    // guard against the circular-oracle failure mode (task contract §24):
    // a deliberately WRONG tree must not compare equal to H0 output.
    let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load");
    let f001 = fixtures.iter().find(|f| f.id == "F001").expect("F001");
    let mut wrong = f001.expected.clone();
    if let Some(p) = wrong.root.children.first_mut() {
        p.end += 1; // off-by-one span corruption
    }
    let actual = parse_document(f001.source.as_bytes());
    assert_ne!(actual, wrong);
    assert_eq!(actual, f001.expected);
}
