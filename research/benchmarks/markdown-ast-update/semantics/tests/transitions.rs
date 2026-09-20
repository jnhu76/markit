//! CORRECTIVE-A required tests — transition oracle.
//!
//! Test families covered here: 5 (G0 transition), 6 (table pilot
//! BREAK/RESTORE), 7 (content-only control), 8 (independent pre/post
//! eligibility), plus the negative cases that make the oracle non-vacuous.

use std::path::{Path, PathBuf};

use markit_mdbench_semantics::facts::SyntaxKind;
use markit_mdbench_semantics::lanes::{G0_GRAMMAR_ID, G1_GRAMMAR_ID};
use markit_mdbench_semantics::pilot::load_fixture;
use markit_mdbench_semantics::transition::{
    validate_transition, ChangeClass, FailureCode, PredicateV1, TransitionRequest,
    TransitionVerdict,
};
use markit_mdbench_semantics::validate_break_restore;
use markit_mdbench_semantics::{payload::BreakRestoreRequest, EditSpec};

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

fn fixture(id: &str) -> markit_mdbench_semantics::pilot::PilotFixture {
    load_fixture(&bench_root().join(format!("workloads/pilots/fixtures/{id}")))
        .unwrap_or_else(|error| panic!("{error}"))
}

fn kind_present(grammar_id: &str, kind: SyntaxKind) -> PredicateV1 {
    PredicateV1::KindPresent {
        grammar_id: grammar_id.to_string(),
        syntax_kind: kind,
    }
}

fn kind_absent(grammar_id: &str, kind: SyntaxKind) -> PredicateV1 {
    PredicateV1::KindAbsent {
        grammar_id: grammar_id.to_string(),
        syntax_kind: kind,
    }
}

/// Family 5 — G0 core transition, with the declared predicates as the gate.
#[test]
fn g0_heading_to_paragraph_transition_holds() {
    let fixture = fixture("p01-g0-atx-heading-to-paragraph.toml");
    let transition = fixture.transition.as_ref().expect("transition declared");
    let request = TransitionRequest {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        pre_source: fixture.source.clone(),
        post_source: fixture.post_source.clone(),
        edit: Some(transition.break_edit.clone()),
        expected_pre: transition.pre.clone(),
        expected_post: transition.post.clone(),
    };
    let report = validate_transition(&request);
    assert_eq!(report.verdict, TransitionVerdict::TransitionValid);
    assert!(report.failure_codes.is_empty());
    assert_eq!(report.change_class, Some(ChangeClass::StructureChange));
    assert!(report.strict_comparison_eligible);
    // Both sides are checked independently.
    assert!(report.pre.predicates.iter().all(|result| result.satisfied));
    assert!(report
        .post
        .as_ref()
        .expect("post side")
        .predicates
        .iter()
        .all(|result| result.satisfied));

    // The declared post source is exactly what the canonical edit produces.
    let derived = transition
        .break_edit
        .apply(&fixture.source)
        .expect("edit applies");
    assert_eq!(derived, fixture.post_source.clone().expect("declared post"));

    // A correct-hash payload with a false claim is rejected.
    let lying = TransitionRequest {
        expected_post: vec![kind_present(G0_GRAMMAR_ID, SyntaxKind::HeadingAtx)],
        ..request.clone()
    };
    let lying_report = validate_transition(&lying);
    assert_eq!(lying_report.verdict, TransitionVerdict::InvalidPayload);
    assert!(lying_report
        .failure_codes
        .contains(&FailureCode::PostPredicateFailed));
}

/// Family 5 (continued) — a post source that does not match the declared
/// edit is a hash failure, never a "passing" transition.
#[test]
fn transition_rejects_post_source_that_does_not_match_the_edit() {
    let request = TransitionRequest {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        pre_source: "# T\n".to_string(),
        post_source: Some("T\n".to_string()),
        edit: Some(EditSpec {
            edit_start: 0,
            edit_end: 2,
            inserted_text: "x".to_string(),
        }),
        expected_pre: vec![kind_present(G0_GRAMMAR_ID, SyntaxKind::HeadingAtx)],
        expected_post: vec![kind_present(G0_GRAMMAR_ID, SyntaxKind::Paragraph)],
    };
    let report = validate_transition(&request);
    assert_eq!(report.verdict, TransitionVerdict::InvalidPayload);
    assert!(report.failure_codes.contains(&FailureCode::PostHashMismatch));
}

/// Family 5/8 — a non-UTF-8-safe edit range is rejected, not clamped.
#[test]
fn transition_rejects_non_char_boundary_edits() {
    let request = TransitionRequest {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        pre_source: "中\n".to_string(),
        post_source: Some("x\n".to_string()),
        edit: Some(EditSpec {
            edit_start: 1,
            edit_end: 3,
            inserted_text: "x".to_string(),
        }),
        expected_pre: vec![],
        expected_post: vec![],
    };
    let report = validate_transition(&request);
    assert!(report
        .failure_codes
        .contains(&FailureCode::EditNotCharBoundary));
}

/// A predicate bound to a different lane is a cross-lane claim, not a typo
/// to ignore.
#[test]
fn transition_rejects_predicates_bound_to_another_lane() {
    let request = TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: "| a |\n| --- |\n".to_string(),
        post_source: None,
        edit: None,
        expected_pre: vec![kind_present(G0_GRAMMAR_ID, SyntaxKind::Table)],
        expected_post: vec![],
    };
    let report = validate_transition(&request);
    assert!(report
        .failure_codes
        .contains(&FailureCode::PredicateGrammarMismatch));
}

/// Family 6 — the table pilot: valid table, delimiter BREAK, RESTORE.
#[test]
fn table_delimiter_break_and_restore_are_proven_by_the_oracle() {
    let fixture = fixture("p02-g1-table-delimiter-break.toml");
    let transition = fixture.transition.as_ref().expect("transition declared");
    let post = fixture.post_source.clone().expect("post source declared");

    let break_report = validate_transition(&TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: fixture.source.clone(),
        post_source: Some(post.clone()),
        edit: Some(transition.break_edit.clone()),
        expected_pre: transition.pre.clone(),
        expected_post: transition.post.clone(),
    });
    assert_eq!(break_report.verdict, TransitionVerdict::TransitionValid);
    // The node-kind transition is real: Table on one side, none on the
    // other. This is what makes it coverage rather than byte replay.
    let pre_topology = break_report.pre.topology_sha256.clone();
    let post_topology = break_report
        .post
        .as_ref()
        .expect("post side")
        .topology_sha256
        .clone();
    assert_ne!(pre_topology, post_topology);
    assert_eq!(break_report.change_class, Some(ChangeClass::StructureChange));

    // A byte change that leaves both sides as tables is NOT a Table ->
    // non-Table case: the declared predicates must reject it, so a
    // cell-level edit can never be counted as a delimiter transition.
    let still_a_table = validate_transition(&TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: fixture.source.clone(),
        post_source: Some("| a | b |\n| --- | --- |\n| 9 | 2 |\n".to_string()),
        edit: Some(EditSpec {
            edit_start: 26,
            edit_end: 27,
            inserted_text: "9".to_string(),
        }),
        expected_pre: transition.pre.clone(),
        expected_post: transition.post.clone(),
    });
    assert_eq!(
        still_a_table.verdict,
        TransitionVerdict::InvalidPayload,
        "both sides are still tables, so the declared Table -> non-Table claim fails"
    );
    assert!(still_a_table
        .failure_codes
        .contains(&FailureCode::PostPredicateFailed));

    let chain = validate_break_restore(&BreakRestoreRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: fixture.source.clone(),
        break_edit: transition.break_edit.clone(),
        break_expected_pre: transition.pre.clone(),
        break_expected_post: transition.post.clone(),
        restore_edit: transition.restore_edit.clone().expect("restore declared"),
        restore_expected_pre: transition.restore_pre.clone(),
        restore_expected_post: transition.restore_post.clone(),
        expected_restore_exact: true,
    });
    assert!(chain.valid, "failures: {:?}", chain.failure_codes);
    assert!(chain.restore_exact);
    assert_eq!(chain.base_source_sha256, chain.s2_source_sha256);
    assert_ne!(chain.base_source_sha256, chain.s1_source_sha256);
    assert_eq!(chain.break_post_sha256, chain.restore_pre_sha256);
}

/// Family 7 — a content-only edit is not a structural transition.
#[test]
fn content_only_edit_is_a_content_change_not_a_structure_change() {
    let fixture = fixture("p03-g1-content-only-control.toml");
    let transition = fixture.transition.as_ref().expect("transition declared");
    let report = validate_transition(&TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: fixture.source.clone(),
        post_source: fixture.post_source.clone(),
        edit: Some(transition.break_edit.clone()),
        expected_pre: transition.pre.clone(),
        expected_post: transition.post.clone(),
    });
    assert_eq!(report.verdict, TransitionVerdict::TransitionValid);
    assert_eq!(report.change_class, Some(ChangeClass::ContentChange));
    let pre = &report.pre;
    let post = report.post.as_ref().expect("post side");
    assert_eq!(pre.topology_sha256, post.topology_sha256, "topology must hold");
    assert_ne!(
        pre.text_fingerprint_sha256, post.text_fingerprint_sha256,
        "content must differ"
    );
}

/// Family 8 — pre and post eligibility are computed independently, and a
/// post-ineligible case is recorded rather than silently benchmarked.
#[test]
fn pre_and_post_eligibility_are_independent() {
    let fixture = fixture("p04-g0-post-ineligibility.toml");
    let transition = fixture.transition.as_ref().expect("transition declared");
    let report = validate_transition(&TransitionRequest {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        pre_source: fixture.source.clone(),
        post_source: fixture.post_source.clone(),
        edit: Some(transition.break_edit.clone()),
        expected_pre: transition.pre.clone(),
        expected_post: transition.post.clone(),
    });

    assert_eq!(report.verdict, TransitionVerdict::TransitionValid);
    assert!(
        report.pre.eligibility.strict_scope_clean,
        "the pre source is clean G0"
    );
    let post = report.post.as_ref().expect("post side");
    assert!(
        post.eligibility.lane_valid,
        "G0 is total: the post source is still a valid G0 document"
    );
    assert!(
        !post.eligibility.strict_scope_clean,
        "the edit exposes host syntax outside the G0 construct set"
    );
    assert!(post
        .eligibility
        .scope_blockers
        .iter()
        .any(|blocker| blocker.syntax_kind == SyntaxKind::Table));
    assert!(
        !report.strict_comparison_eligible,
        "a post-ineligible payload must not be strict comparison evidence"
    );
}

/// The G1 counterpart: the same edit under G1 IS strict coverage, because
/// G1 owns the table construct. Lanes are not interchangeable.
#[test]
fn the_same_edit_is_strictly_eligible_under_the_lane_that_owns_it() {
    let source = "Just text.\n";
    let post = "Just text.\n| a | b |\n| --- | --- |\n";
    let g0 = validate_transition(&TransitionRequest {
        grammar_id: G0_GRAMMAR_ID.to_string(),
        pre_source: source.to_string(),
        post_source: Some(post.to_string()),
        edit: Some(EditSpec {
            edit_start: 10,
            edit_end: 10,
            inserted_text: "\n| a | b |\n| --- | --- |".to_string(),
        }),
        expected_pre: vec![kind_present(G0_GRAMMAR_ID, SyntaxKind::Paragraph)],
        // Under G0 the table bytes are ordinary paragraph text, so the edit
        // is a *content* change: no G0 kind appears or disappears. The
        // assertion set has to say that, or it describes nothing the edit
        // did and the oracle rejects it as unexercised.
        expected_post: vec![
            kind_present(G0_GRAMMAR_ID, SyntaxKind::Paragraph),
            PredicateV1::TextContentDiffers,
        ],
    });
    assert_eq!(g0.verdict, TransitionVerdict::TransitionValid);
    assert!(!g0.strict_comparison_eligible);

    let g1 = validate_transition(&TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: source.to_string(),
        post_source: Some(post.to_string()),
        edit: Some(EditSpec {
            edit_start: 10,
            edit_end: 10,
            inserted_text: "\n| a | b |\n| --- | --- |".to_string(),
        }),
        expected_pre: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::Paragraph)],
        expected_post: vec![
            kind_present(G1_GRAMMAR_ID, SyntaxKind::Table),
            kind_absent(G1_GRAMMAR_ID, SyntaxKind::HeadingAtx),
        ],
    });
    assert_eq!(g1.verdict, TransitionVerdict::TransitionValid);
    assert!(g1.strict_comparison_eligible);
    assert_eq!(g1.change_class, Some(ChangeClass::StructureChange));
}

/// An unknown lane cannot silently pass a transition check.
#[test]
fn unknown_lane_is_rejected() {
    let report = validate_transition(&TransitionRequest {
        grammar_id: "COMMONMARK-GFM".to_string(),
        pre_source: "x\n".to_string(),
        post_source: Some("y\n".to_string()),
        edit: None,
        expected_pre: vec![],
        expected_post: vec![],
    });
    assert_eq!(report.verdict, TransitionVerdict::InvalidPayload);
    assert!(report
        .failure_codes
        .contains(&FailureCode::UnknownGrammarLane));
}

/// G1 exposes reference definitions as out-of-band recognized facts rather
/// than as nodes. A kind predicate that counted nodes only would answer
/// `kind_absent` for a kind the same profile reports as `recognized` —
/// reading non-recognition back as absence, the one inference the profiler
/// contract forbids.
#[test]
fn kind_predicates_see_out_of_band_recognized_facts() {
    let source = "[a]: http://example.com\n\n[a]\n";
    let profile = markit_mdbench_semantics::profile_g1(source);
    assert!(
        profile
            .syntax_facts
            .iter()
            .any(|fact| fact.syntax_kind == SyntaxKind::ReferenceDefinition
                && fact.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized),
        "the fixture must produce a recognized out-of-band reference definition"
    );

    let request = TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: source.to_string(),
        post_source: None,
        edit: None,
        expected_pre: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::ReferenceDefinition)],
        expected_post: Vec::new(),
    };
    let report = validate_transition(&request);
    let result = &report.pre.predicates[0];
    assert!(
        result.satisfied,
        "kind_present must see the recognized fact, observed: {}",
        result.observed
    );

    // And the mirror image: kind_absent must not claim absence here.
    let absent_request = TransitionRequest {
        expected_pre: vec![kind_absent(G1_GRAMMAR_ID, SyntaxKind::ReferenceDefinition)],
        ..request
    };
    let absent_report = validate_transition(&absent_request);
    assert!(
        !absent_report.pre.predicates[0].satisfied,
        "kind_absent must not report absence for a recognized construct, observed: {}",
        absent_report.pre.predicates[0].observed
    );
    assert!(absent_report
        .failure_codes
        .contains(&FailureCode::PrePredicateFailed));
}

/// A payload whose edit changes nothing is not a transition, however many
/// assertions it declares.
#[test]
fn a_no_op_edit_is_rejected() {
    let source = "| a | b |\n| --- | --- |\n";
    let request = TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: source.to_string(),
        post_source: Some(source.to_string()),
        edit: Some(EditSpec {
            edit_start: 5,
            edit_end: 5,
            inserted_text: String::new(),
        }),
        expected_pre: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::Table)],
        expected_post: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::Table)],
    };
    let report = validate_transition(&request);
    assert_eq!(report.verdict, TransitionVerdict::InvalidPayload);
    assert!(report
        .failure_codes
        .contains(&FailureCode::NoSourceChange));
    assert!(report
        .failure_codes
        .contains(&FailureCode::TransitionNotExercised));
}

/// An assertion set that holds identically before and after, with no
/// satisfied cross-source predicate, describes nothing the edit did: the
/// case would otherwise pass on hashes and a label alone.
#[test]
fn a_vacuous_assertion_set_is_rejected() {
    let request = TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: "alpha\n".to_string(),
        post_source: Some("alphax\n".to_string()),
        edit: Some(EditSpec {
            edit_start: 5,
            edit_end: 5,
            inserted_text: "x".to_string(),
        }),
        expected_pre: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::Paragraph)],
        expected_post: vec![kind_present(G1_GRAMMAR_ID, SyntaxKind::Paragraph)],
    };
    let report = validate_transition(&request);
    assert!(
        report
            .failure_codes
            .contains(&FailureCode::TransitionNotExercised),
        "a paragraph-present assertion holds on both sides: {:?}",
        report.failure_codes
    );
    assert!(!report.failure_codes.contains(&FailureCode::NoSourceChange));
    assert_eq!(report.verdict, TransitionVerdict::InvalidPayload);
}
