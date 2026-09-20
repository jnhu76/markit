//! CORRECTIVE-B selection-contract tests (§49): deterministic bins,
//! collapsed quantiles, representative tie-breaks and caps, mechanical
//! cap relaxation, extremal rules, syntax set-cover, full-document
//! scoring, cross-set dedup, and the performance-blind dependency guard.

use markit_mdbench_profile_select::redundancy::{signature, RedundancyArtifact};
use markit_mdbench_profile_select::selection::{
    feature_value, key, select, syntax_cell, MATH_CANDIDATE_FEATURE, PROJECT_CAP,
};
use markit_mdbench_profile_select::stats::{bin_of, derive_bins, percentile_rank};
use markit_mdbench_profile_select::{CandidateIdentity, CandidateRow};

fn identity(source_id: &str, path: &str, domain: &str, sha256: &str, bytes: u64) -> CandidateIdentity {
    CandidateIdentity {
        source_id: source_id.to_string(),
        snapshot_path: path.to_string(),
        upstream_path: path.to_string(),
        sha256: sha256.to_string(),
        bytes,
        git_blob_sha1: String::new(),
        domain: domain.to_string(),
    }
}

fn row(source_id: &str, path: &str, domain: &str, markdown: &str) -> CandidateRow {
    let digest = markit_mdbench_profile_select::sha256_hex(markdown.as_bytes());
    let identity = identity(source_id, path, domain, &digest, markdown.len() as u64);
    markit_mdbench_profile_select::rows::profile_candidate(&identity, markdown)
}

fn empty_redundancy() -> RedundancyArtifact {
    RedundancyArtifact {
        schema: "redundancy-v1".to_string(),
        method: "test".to_string(),
        threshold: 0.75,
        shingle_width: 5,
        permutations: 64,
        exact_groups: Vec::new(),
        near_groups: Vec::new(),
        notes: Vec::new(),
    }
}

fn g0_strict_doc(heading: &str, extra: &str) -> String {
    format!("# {heading}\n\nParagraph with *emphasis* and `code`.\n\n- item one\n- item two\n\n```rust\nfn main() {{}}\n```\n\n[link]: https://example.com/\n\nSee [link].\n{extra}")
}

#[test]
fn bins_split_zero_and_tertiles_deterministically() {
    let values: Vec<f64> = vec![0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let bins = derive_bins("test_feature", true, &values);
    assert!(bins.bins.contains(&"ZERO".to_string()));
    assert!(bins.bins.contains(&"LOW".to_string()));
    assert!(bins.bins.contains(&"MEDIUM".to_string()));
    assert!(bins.bins.contains(&"HIGH".to_string()));
    assert_eq!(bins.zero_meaningful, true);
    // Deterministic across calls.
    assert_eq!(derive_bins("test_feature", true, &values), bins);
    // ZERO assignment.
    assert_eq!(bin_of(&bins, 0.0), "ZERO");
}

#[test]
fn collapsed_quantiles_do_not_jitter() {
    // All positive values equal: only ZERO + LOW are attainable.
    let values: Vec<f64> = vec![0.0, 5.0, 5.0, 5.0];
    let bins = derive_bins("collapsed", true, &values);
    assert_eq!(bins.bins, vec!["ZERO".to_string(), "LOW".to_string()]);
    assert_eq!(bin_of(&bins, 5.0), "LOW");
    // No positives at all: only ZERO.
    let only_zero = derive_bins("only_zero", true, &[0.0, 0.0]);
    assert_eq!(only_zero.bins, vec!["ZERO".to_string()]);
    // Non-zero-meaningful feature: no ZERO bin even when zeros exist.
    let plain = derive_bins("file_bytes", false, &[0.0, 1.0, 9.0]);
    assert!(!plain.bins.contains(&"ZERO".to_string()));
}

#[test]
fn profile_source_hash_binds_to_identity() {
    let markdown = "# Title\n\nText.\n";
    let row = row("src", "files/a.md", "D", markdown);
    assert_eq!(row.source_sha256, row.identity.sha256);
    assert!(row.hash_match);
    // A row constructed against a wrong identity hash is flagged at
    // construction, never accepted.
    let wrong_identity = identity("src", "files/a.md", "D", &"0".repeat(64), markdown.len() as u64);
    let tampered = markit_mdbench_profile_select::rows::profile_candidate(&wrong_identity, markdown);
    assert!(!tampered.hash_match);
    assert!(!tampered.g0_strict_eligible());
}

#[test]
fn representative_selection_respects_project_cap_and_covers_cells() {
    let mut rows = Vec::new();
    for index in 0..6 {
        rows.push(row(
            "project-a",
            &format!("files/a{index}.md"),
            "DOM_ONE",
            &g0_strict_doc(&format!("A{index}"), &format!("\n\npara {index} filler\n")),
        ));
    }
    for index in 0..6 {
        rows.push(row(
            "project-b",
            &format!("files/b{index}.md"),
            "DOM_TWO",
            &g0_strict_doc(&format!("B{index}"), &format!("\n\nother {index} filler\n")),
        ));
    }
    for index in 0..4 {
        rows.push(row(
            "project-c",
            &format!("files/c{index}.md"),
            "DOM_THREE",
            &g0_strict_doc(&format!("C{index}"), &format!("\n\nthird {index} filler\n")),
        ));
    }
    assert!(rows.iter().all(|row| row.g0_strict_eligible()));
    let outcome = select(&rows, &empty_redundancy()).expect("selection");
    for (project, count) in [
        ("project-a", 0),
        ("project-b", 0),
        ("project-c", 0),
    ] {
        let used = outcome
            .representative
            .iter()
            .filter(|key| key.starts_with(&format!("{project}/")))
            .count();
        assert!(
            used as u64 <= PROJECT_CAP,
            "project {project} consumed {used} representative slots"
        );
        let _ = count;
    }
    // Every attainable mandatory cell is covered (stop rule).
    assert!(
        outcome.uncovered_cells.is_empty(),
        "uncovered cells: {:?}",
        outcome.uncovered_cells
    );
    // All selected representative members are G0-strict.
    for member_key in &outcome.representative {
        let member = &outcome.members[member_key];
        assert!(member.g0_strict_eligible, "{member_key} must be G0-strict");
    }
}

#[test]
fn representative_tie_break_is_lexical_and_deterministic() {
    let mut rows = Vec::new();
    // Two projects with byte-identical eligibility profile: identical
    // feature vectors, so every ordering tie falls through to lexical.
    let doc = g0_strict_doc("Same", "");
    rows.push(row("alpha", "files/x.md", "DOM", &doc));
    rows.push(row("beta", "files/y.md", "DOM", &doc));
    let first = select(&rows, &empty_redundancy()).expect("selection");
    let second = select(&rows, &empty_redundancy()).expect("selection");
    assert_eq!(first.representative, second.representative);
    assert_eq!(first.trace.len(), second.trace.len());
    // Lexically smallest source wins the first slot.
    assert_eq!(first.representative[0], "alpha/files/x.md");
}

#[test]
fn domain_cap_relaxation_is_mechanical_and_logged() {
    // One domain only: the 35% cap cannot hold once more than a couple of
    // files are needed, so any further selection is a logged relaxation.
    let mut rows = Vec::new();
    for index in 0..8 {
        let doc = format!(
            "# H{index}\n\n{}\n",
            "para ".repeat(index * 40)
        );
        rows.push(row("only-project", &format!("files/p{index}.md"), "SINGLE_DOM", &doc));
    }
    let outcome = select(&rows, &empty_redundancy()).expect("selection");
    // Either the cap held for the whole (small) set, or every violation
    // is logged with the mechanical reason.
    for record in &outcome.trace {
        if record.set == "representative" {
            if let Some(note) = &record.cap_relaxation {
                assert!(note.contains("cap relaxation"), "{note}");
            }
        }
    }
    for note in &outcome.cap_relaxations {
        assert!(note.contains("no cap-compliant candidate"), "{note}");
    }
}

#[test]
fn extremal_maximum_and_tail_replicate_rules() {
    let mut rows = Vec::new();
    rows.push(row("big", "files/big.md", "D1", &format!("# T\n\n{}", "x".repeat(50_000))));
    // The replicate must be a distinct project INSIDE the top 1% (min 2):
    // rank 2 is the cross-project file; the same-project runner-up at
    // rank 3 must not claim the replicate role.
    rows.push(row("other", "files/rep.md", "D2", &format!("# T\n\n{}", "x".repeat(45_000))));
    rows.push(row("big", "files/second.md", "D1", &format!("# T\n\n{}", "x".repeat(40_000))));
    let outcome = select(&rows, &empty_redundancy()).expect("selection");
    let roles: Vec<(String, Vec<String>)> = outcome
        .members
        .iter()
        .map(|(key, member)| (key.clone(), member.extreme_roles.clone()))
        .collect();
    let big = roles.iter().find(|(key, _)| key == "big/files/big.md").unwrap();
    assert!(big.1.contains(&"file_bytes:OBSERVED_MAXIMUM".to_string()));
    let rep = roles.iter().find(|(key, _)| key == "other/files/rep.md").unwrap();
    assert!(rep.1.contains(&"file_bytes:TAIL_REPLICATE".to_string()));
    // Same-project runner-up is NOT the replicate.
    let second = roles.iter().find(|(key, _)| key == "big/files/second.md").unwrap();
    assert!(!second.1.iter().any(|role| role.contains("TAIL_REPLICATE")));

    // Single-project universe: replicate is NONE_AVAILABLE.
    let mut single = Vec::new();
    single.push(row("solo", "files/a.md", "D", &format!("# T\n\n{}", "y".repeat(10_000))));
    single.push(row("solo", "files/b.md", "D", &format!("# T\n\n{}", "y".repeat(9_000))));
    let outcome = select(&single, &empty_redundancy()).expect("selection");
    assert!(outcome
        .trace
        .iter()
        .any(|record| record.selected.starts_with("NONE_AVAILABLE(")));
}

#[test]
fn syntax_cells_carry_evidence_grades_not_mixed_counts() {
    let table_doc = "| a | b |\n|---|---|\n| 1 | 2 |\n\nText with [link](https://x).\n";
    let row_table = row("t", "files/t.md", "D", table_doc);
    assert_eq!(syntax_cell(&row_table, "syntax:table"), Some("strict"));
    assert_eq!(syntax_cell(&row_table, "table:ordinary"), Some("strict"));

    let math_doc = "Inline $x+y$ and display\n\n$$\nz\n$$\n";
    let row_math = row("m", "files/m.md", "D", math_doc);
    assert_eq!(
        syntax_cell(&row_math, "syntax:math_inline_candidate"),
        Some("ambiguous_or_unknown")
    );
    assert_eq!(
        syntax_cell(&row_math, "syntax:math_display_candidate"),
        Some("ambiguous_or_unknown")
    );

    let strike_doc = "Struck ~~gone~~ text.\n";
    let row_strike = row("s", "files/s.md", "D", strike_doc);
    assert_eq!(syntax_cell(&row_strike, "extra:strikethrough"), Some("candidate"));

    // Core cells require strict G0 evidence.
    let plain = row("p", "files/p.md", "D", &g0_strict_doc("H", ""));
    assert_eq!(syntax_cell(&plain, "core:heading_atx"), Some("strict"));
    assert_eq!(syntax_cell(&plain, "core:emphasis"), Some("strict"));
    // The table doc carries a strict G0 inline link but no emphasis.
    assert!(syntax_cell(&row_table, "core:link_inline").is_some());
    assert!(syntax_cell(&row_table, "core:emphasis").is_none());
    assert_ne!(syntax_cell(&row_table, "syntax:table"), None);
}

#[test]
fn full_document_rank_uses_math_candidate_occurrences_not_occupancy() {
    let small = row("openmlsys", "files/small.md", "D", "# S\n\ntext\n");
    let mathy = row(
        "openmlsys",
        "files/mathy.md",
        "D",
        "# M\n\n$a$ $b$ $c$\n\n$$d$$\n",
    );
    let rows = vec![small.clone(), mathy.clone()];
    // The pseudo-feature is occurrence-based (G2 deferred).
    assert!(feature_value(&mathy, MATH_CANDIDATE_FEATURE) > feature_value(&small, MATH_CANDIDATE_FEATURE));
    let outcome = select(&rows, &empty_redundancy()).expect("selection");
    let full = outcome
        .trace
        .iter()
        .find(|record| record.set == "full_document" && record.selected.starts_with("openmlsys/"))
        .expect("openmlsys full-document record");
    assert!(full.why.contains("occurrence"));
    assert!(full.why.contains("G2 deferred"));
}

#[test]
fn cross_set_membership_is_physically_deduplicated() {
    // One file dominating every dimension: it must appear once in
    // members with several roles, never as duplicate bytes.
    let dominant = format!("# T\n\n{}", "z".repeat(80_000));
    let mut rows = vec![row("dom", "files/huge.md", "D1", &dominant)];
    rows.push(row("other", "files/o.md", "D2", "# O\n\nsmall\n"));
    let outcome = select(&rows, &empty_redundancy()).expect("selection");
    let member = outcome.members.get("dom/files/huge.md").expect("member");
    assert!(member.memberships.contains(&"extremal".to_string()));
    let unique: std::collections::BTreeSet<&String> = outcome
        .representative
        .iter()
        .chain(outcome.extremal.iter())
        .chain(outcome.syntax_coverage.iter())
        .chain(outcome.full_document.iter())
        .collect();
    assert_eq!(unique.len(), outcome.members.len());
    // Logical counts may exceed unique physical count; they are reported
    // separately in the coverage artifact (coverage tests).
    assert!(outcome.members.len() <= unique.len());
}

#[test]
fn exact_duplicates_form_one_group_with_lexical_canonical() {
    let doc = g0_strict_doc("Dup", "");
    let mut rows = vec![
        row("beta", "files/b2.md", "D", &doc),
        row("beta", "files/b1.md", "D", &doc),
        row("alpha", "files/a9.md", "D", &doc),
    ];
    rows.push(row("gamma", "files/other.md", "D", &g0_strict_doc("Other", "")));
    let groups = markit_mdbench_profile_select::redundancy::exact_groups(&rows);
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    assert_eq!(group.members.len(), 3);
    assert_eq!(group.canonical_member, "alpha/files/a9.md");
    // Duplicates must not consume multiple representative slots: with
    // identical feature vectors the selector dedups by key anyway; the
    // canonical member is the lexical first.
}

#[test]
fn near_duplicate_signature_is_deterministic_and_inspectable() {
    let text_a = "the quick brown fox jumps over the lazy dog again and again";
    let text_b = "the quick brown fox jumps over the lazy dog again and again";
    let text_c = "completely different content about rust compilers and parsers";
    let sig_a = signature(text_a);
    let sig_a2 = signature(text_b);
    let sig_c = signature(text_c);
    assert_eq!(sig_a, sig_a2);
    let equal = sig_a.iter().zip(sig_c.iter()).filter(|(a, b)| a == b).count();
    assert!(equal < 60, "unrelated texts must not look near-identical");
}

#[test]
fn candidate_universe_loader_rejects_undeclared_and_missing_sources() {
    let dir = std::env::temp_dir().join(format!("cb-test-universe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join("manifests"));
    let _ = std::fs::create_dir_all(dir.join("analysis"));
    let _ = std::fs::create_dir_all(dir.join("sources/src-one"));
    std::fs::write(
        dir.join("manifests/candidate-universe-v1.json"),
        r#"{"campaign_id":"MARKIT-REAL-WORKLOAD-ACQUISITION-1","candidates":[{"source_id":"src-one"}]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("analysis/domain-strata-v1.json"),
        r#"{"schema":"domain-strata-v1","sources":[{"source_id":"src-one","stratum":"OTHER_DECLARED","rationale":"t","counts_as_role_coverage":true}]}"#,
    )
    .unwrap();
    // Completeness: a declared source without SOURCE.json fails.
    let err = markit_mdbench_profile_select::universe::load_identities(&dir);
    assert!(err.is_err());
    // With SOURCE.json present, loading succeeds and is lexical.
    std::fs::write(
        dir.join("sources/src-one/SOURCE.json"),
        r#"{"source_id":"src-one","files":[{"snapshot_path":"files/b.md","sha256":"x","bytes":2,"git_blob_sha1":"g"},{"snapshot_path":"files/a.md","sha256":"y","bytes":1,"git_blob_sha1":"g"}]}"#,
    )
    .unwrap();
    let (identities, per_source) =
        markit_mdbench_profile_select::universe::load_identities(&dir).expect("load");
    assert_eq!(identities.len(), 2);
    assert_eq!(identities[0].snapshot_path, "files/a.md");
    assert_eq!(per_source.get("src-one").copied(), Some(2));
    // An undeclared extra SOURCE.json fails closed.
    let _ = std::fs::create_dir_all(dir.join("sources/src-extra"));
    std::fs::write(dir.join("sources/src-extra/SOURCE.json"), r#"{"source_id":"src-extra","files":[]}"#).unwrap();
    assert!(markit_mdbench_profile_select::universe::load_identities(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_horse_or_performance_dependency_is_reachable() {
    // The crate's dependency list must not name mechanisms,
    // instrumentation or runner (comments may discuss the ban).
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest");
    let dependencies: String = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .split('[')
        .next()
        .unwrap_or("")
        .to_string();
    for forbidden in [
        "mechanisms",
        "markit-mdbench-instrumentation",
        "markit-mdbench-runner",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "profile-select [dependencies] must not contain {forbidden}"
        );
    }
    // And the lib re-exports nothing horse-flavoured: doc prose may
    // state the ban, code must not name the horse APIs.
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs");
    let lib_code: String = lib
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<&str>>()
        .join("\n");
    for forbidden in ["HorseId", "HorseStatus", "mechanism::", "instrumentation::"] {
        assert!(
            !lib_code.contains(forbidden),
            "lib.rs code must not reference {forbidden}"
        );
    }
}

#[test]
fn selection_is_stable_across_repeated_runs() {
    let mut rows = Vec::new();
    for index in 0..5 {
        rows.push(row(
            "proj",
            &format!("files/f{index}.md"),
            "DOM",
            &g0_strict_doc(&format!("H{index}"), &"filler ".repeat(index * 200)),
        ));
    }
    rows.push(row("t", "files/table.md", "DOM2", "| a | b |\n|---|---|\n| 1 | 2 |\n"));
    let a = select(&rows, &empty_redundancy()).expect("sel");
    let b = select(&rows, &empty_redundancy()).expect("sel");
    assert_eq!(a.representative, b.representative);
    assert_eq!(a.extremal, b.extremal);
    assert_eq!(a.syntax_coverage, b.syntax_coverage);
    assert_eq!(a.full_document, b.full_document);
    assert_eq!(
        markit_mdbench_profile_select::canonical_json(&a.trace),
        markit_mdbench_profile_select::canonical_json(&b.trace)
    );
}

#[test]
fn percentile_rank_is_tie_stable() {
    let sorted = vec![1.0, 2.0, 2.0, 3.0];
    assert_eq!(percentile_rank(&sorted, 1.0), 0.125);
    assert_eq!(percentile_rank(&sorted, 2.0), 0.5);
    assert_eq!(percentile_rank(&sorted, 3.0), 0.875);
    assert_eq!(percentile_rank(&sorted, 99.0), 1.0);
}

#[test]
fn spot_check_span_evidence_rejects_corrupt_spans() {
    use markit_mdbench_profile_select::rows::span_evidence_ok;
    use markit_mdbench_semantics::facts::{Span, SyntaxFact};
    let source = "# Heading\n\nText here\n";
    let mut fact = SyntaxFact {
        grammar_id: "g".to_string(),
        syntax_kind: markit_mdbench_semantics::facts::SyntaxKind::HeadingAtx,
        source_start: 0,
        source_end: 9,
        recognition_status: markit_mdbench_semantics::facts::RecognitionStatus::Recognized,
        lane_scope_grade: markit_mdbench_semantics::facts::LaneScopeGrade::StrictLaneCoverage,
        host_context: true,
        reason: markit_mdbench_semantics::facts::FactReason::RecognizedUnderLaneOracle,
        occurrence: 0,
        detail: None,
    };
    assert!(span_evidence_ok(source, &fact).is_ok());
    fact.source_start = 1; // mid-UTF-8 / not '#' anymore
    assert!(span_evidence_ok(source, &fact).is_err());
    let _ = Span::new(0, 1);
}

#[test]
fn syntax_inventory_grades_mirror_frozen_construct_scopes() {
    // The four extension targets (strikethrough, task list items, front
    // matter, directives) are out_of_lane under BOTH G0 and G1
    // (grammar/grammar-lanes-v1.json construct_scopes): their inventory
    // grade must never read as strict or declared lane coverage, and their
    // occurrences must land in the candidate column.
    let plain = row("p", "plain.md", "OTHER_DECLARED", &g0_strict_doc("t", ""));
    let extensions = row(
        "e",
        "ext.md",
        "OTHER_DECLARED",
        "---\ntitle: x\n---\n\n# H\n\nText with ~~strike~~ and :::note\n\n- [ ] task\n",
    );
    let rows = vec![plain, extensions];
    let inventory = markit_mdbench_profile_select::artifacts::syntax_inventory(&rows);
    let grade_of = |target: &str| -> &str {
        inventory
            .iter()
            .find(|entry| entry.target == target)
            .map(|entry| entry.evidence_grade.as_str())
            .expect("inventory target present")
    };
    assert_eq!(grade_of("paragraph"), "strict_lane_coverage");
    assert_eq!(grade_of("table"), "strict_lane_coverage");
    assert_eq!(grade_of("image"), "contract_declared_not_qualified");
    assert_eq!(grade_of("inline_math"), "lane_deferred");
    for target in ["strikethrough", "task_list_item", "front_matter", "directive"] {
        assert_eq!(
            grade_of(target),
            "out_of_lane_candidate",
            "{target} is out_of_lane under both G0 and G1"
        );
        let entry = inventory
            .iter()
            .find(|e| e.target == target)
            .expect("inventory target present");
        assert_eq!(
            entry.recognized_occurrences, 0,
            "{target} has no qualified recognition anywhere"
        );
    }
    // The extension-bearing document produced candidate evidence for the
    // out-of-lane probes under G0 (non-recognition is never silence).
    let front_matter = inventory
        .iter()
        .find(|e| e.target == "front_matter")
        .expect("inventory target present");
    assert!(front_matter.candidate_occurrences > 0);
}
