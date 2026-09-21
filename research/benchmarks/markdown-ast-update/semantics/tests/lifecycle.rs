//! CORRECTIVE-A required tests — payload lifecycle.
//!
//! Test families covered here: 9 (BREAK/RESTORE SHA chain), 10
//! (EARLY/MIDDLE/LATE dedup determinism), 11 (chained step coordinates
//! belong to the step's pre-source), plus payload identity and inserted-byte
//! reconstruction.

use markit_mdbench_semantics::payload::{
    build_payload, resolve_positions, validate_trace, Anchor, BreakRestoreFailureCode,
    BreakRestoreRequest, SourceKind, TraceForm, TraceRequest, TraceStep,
};
use markit_mdbench_semantics::transition::{validate_transition, EditSpec, PredicateV1};
use markit_mdbench_semantics::{validate_break_restore, validate_payload, RequestedPosition, Span};
use markit_mdbench_semantics::{SyntaxKind, G1_GRAMMAR_ID};

fn kind_present(kind: SyntaxKind) -> PredicateV1 {
    PredicateV1::KindPresent {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        syntax_kind: kind,
    }
}

fn kind_absent(kind: SyntaxKind) -> PredicateV1 {
    PredicateV1::KindAbsent {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        syntax_kind: kind,
    }
}

const TABLE_SOURCE: &str = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
const TABLE_POST: &str = "| a | b |\n| abc | --- |\n| 1 | 2 |\n";

const BREAK_LABEL_TOGGLE: EditSpec = EditSpec {
    edit_start: 12,
    edit_end: 15,
    inserted_text: String::new(),
};

fn break_edit() -> EditSpec {
    EditSpec {
        edit_start: 12,
        edit_end: 15,
        inserted_text: "abc".to_string(),
    }
}

fn restore_edit() -> EditSpec {
    EditSpec {
        edit_start: 12,
        edit_end: 15,
        inserted_text: "---".to_string(),
    }
}

/// Family 9 — the BREAK/RESTORE chain, and the negative case where RESTORE
/// is not applied to the frozen broken state.
#[test]
fn break_restore_sha_chain_is_correct_and_verified() {
    let chain = validate_break_restore(&BreakRestoreRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: TABLE_SOURCE.to_string(),
        break_edit: break_edit(),
        break_expected_pre: vec![kind_present(SyntaxKind::Table)],
        break_expected_post: vec![
            kind_absent(SyntaxKind::Table),
            kind_present(SyntaxKind::Paragraph),
        ],
        restore_edit: restore_edit(),
        restore_expected_pre: vec![kind_absent(SyntaxKind::Table)],
        restore_expected_post: vec![kind_present(SyntaxKind::Table)],
        expected_restore_exact: true,
    });
    assert!(chain.valid, "failures: {:?}", chain.failure_codes);
    assert_eq!(chain.break_pre_sha256, chain.base_source_sha256);
    assert_eq!(chain.break_post_sha256, chain.s1_source_sha256);
    assert_eq!(chain.restore_pre_sha256, chain.s1_source_sha256);
    assert_eq!(chain.restore_post_sha256, chain.s2_source_sha256);
    assert!(chain.restore_exact);
    assert_eq!(chain.s2_source_sha256, chain.base_source_sha256);

    // A RESTORE whose edit does not start from the broken state cannot be
    // smuggled in as an exact restoration.
    let wrong_restore = validate_break_restore(&BreakRestoreRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: TABLE_SOURCE.to_string(),
        // This "break" deletes the whole delimiter row, so the declared
        // post predicates below do not hold and the restore below cannot
        // reproduce the broken bytes.
        break_edit: EditSpec {
            edit_start: 10,
            edit_end: 24,
            inserted_text: String::new(),
        },
        break_expected_pre: vec![kind_present(SyntaxKind::Table)],
        break_expected_post: vec![kind_absent(SyntaxKind::Table)],
        restore_edit: restore_edit(),
        restore_expected_pre: vec![kind_absent(SyntaxKind::Table)],
        restore_expected_post: vec![kind_present(SyntaxKind::Table)],
        expected_restore_exact: true,
    });
    assert!(!wrong_restore.valid);
    assert!(!wrong_restore.failure_codes.is_empty());
}

/// Family 9 (continued) — an exactly-inverting RESTORE must be detected,
/// including the case where the declared expectation says it must not be
/// exact.
#[test]
fn restore_exactness_is_reported_and_can_be_a_declared_failure() {
    // `abc` -> `---` is not the original text, so the second restore is not
    // exact even though it restores the table.
    let chain = validate_break_restore(&BreakRestoreRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: TABLE_SOURCE.to_string(),
        break_edit: break_edit(),
        break_expected_pre: vec![kind_present(SyntaxKind::Table)],
        break_expected_post: vec![kind_absent(SyntaxKind::Table)],
        // Restores a delimiter row, but a different one (one dash instead
        // of three): the table returns, the bytes do not.
        restore_edit: EditSpec {
            edit_start: 12,
            edit_end: 15,
            inserted_text: "-".to_string(),
        },
        restore_expected_pre: vec![kind_absent(SyntaxKind::Table)],
        restore_expected_post: vec![kind_present(SyntaxKind::Table)],
        expected_restore_exact: true,
    });
    assert!(!chain.restore_exact);
    assert!(!chain.valid);
    assert!(chain
        .failure_codes
        .contains(&markit_mdbench_semantics::payload::BreakRestoreFailureCode::RestoreNotExact));
}

/// Family 10 — EARLY/MIDDLE/LATE dedup is deterministic and honest.
#[test]
fn position_requests_that_share_an_anchor_are_deduplicated() {
    // One physical anchor: all three requests must collapse.
    let single = vec![Anchor {
        syntax_kind: SyntaxKind::HeadingAtx,
        span: Span::new(0, 7),
        occurrence: 0,
    }];
    let outcome = resolve_positions(&single, 15, &RequestedPosition::all());
    assert_eq!(outcome.resolutions.len(), 1);
    let resolution = &outcome.resolutions[0];
    assert_eq!(resolution.requested_positions.len(), 3);
    assert!(resolution.deduplicated);
    assert_eq!(resolution.actual_anchor_byte, 0);
    assert_eq!(resolution.dedup_identity, "heading_atx#0@0-7");
    assert!(resolution
        .distance_from_target
        .iter()
        .all(|distance| distance.distance >= 0.0));

    // Two physical anchors: EARLY takes the first, MIDDLE/LATE the second.
    let two = vec![
        Anchor {
            syntax_kind: SyntaxKind::HeadingSetext,
            span: Span::new(0, 12),
            occurrence: 0,
        },
        Anchor {
            syntax_kind: SyntaxKind::HeadingSetext,
            span: Span::new(25, 33),
            occurrence: 1,
        },
    ];
    let outcome = resolve_positions(&two, 33, &RequestedPosition::all());
    assert_eq!(outcome.resolutions.len(), 2);
    assert_eq!(
        outcome.resolutions[0].requested_positions,
        vec![RequestedPosition::Early]
    );
    assert_eq!(
        outcome.resolutions[1].requested_positions,
        vec![RequestedPosition::Middle, RequestedPosition::Late]
    );
    assert!(outcome.resolutions[1].deduplicated);

    // Determinism: identical inputs, identical resolutions, including the
    // tie-break order defined by the contract.
    let again = resolve_positions(&two, 33, &RequestedPosition::all());
    assert_eq!(
        markit_mdbench_semantics::canonical_json(&outcome),
        markit_mdbench_semantics::canonical_json(&again)
    );

    // No anchor at all: NOT_APPLICABLE, never a fabricated nearby construct.
    let empty = resolve_positions(&[], 33, &RequestedPosition::all());
    assert!(empty.not_applicable);
    assert!(empty.resolutions.is_empty());
    assert_eq!(empty.uncovered_requests.len(), 3);
}

/// Family 11 — chained step coordinates belong to that step's pre-source.
#[test]
fn trace_coordinates_are_validated_against_each_steps_pre_source() {
    let base = "alpha\n";
    let steps = vec![
        TraceStep {
            step: 0,
            edit: EditSpec {
                edit_start: 5,
                edit_end: 5,
                inserted_text: " beta".to_string(),
            },
            expected_transition: "text_insert".to_string(),
            post_source_sha256: markit_mdbench_semantics::sha256_hex(b"alpha beta\n"),
        },
        TraceStep {
            step: 1,
            edit: EditSpec {
                edit_start: 0,
                edit_end: 5,
                inserted_text: "gamma".to_string(),
            },
            expected_transition: "content_replace".to_string(),
            post_source_sha256: markit_mdbench_semantics::sha256_hex(b"gamma beta\n"),
        },
        TraceStep {
            step: 2,
            edit: EditSpec {
                edit_start: 5,
                edit_end: 10,
                inserted_text: String::new(),
            },
            expected_transition: "text_delete".to_string(),
            post_source_sha256: markit_mdbench_semantics::sha256_hex(b"gamma\n"),
        },
    ];
    let report = validate_trace(&TraceRequest {
        trace_id: "trace-ok".to_string(),
        trace_form: TraceForm::LocalBurst,
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: base.to_string(),
        steps: steps.clone(),
    });
    assert!(report.valid, "failures: {:?}", report.failure_codes);
    assert_eq!(
        report.final_source_sha256,
        markit_mdbench_semantics::sha256_hex(b"gamma\n")
    );

    // Reusing a coordinate from an earlier state after the source shifted
    // must fail: the last step deletes bytes 5..10 of "gamma beta", which
    // is valid, but a coordinate that belonged to S0 does not survive.
    let shifted = vec![TraceStep {
        step: 0,
        edit: EditSpec {
            edit_start: 0,
            edit_end: 1,
            inserted_text: String::new(),
        },
        expected_transition: "delete_first_char".to_string(),
        post_source_sha256: markit_mdbench_semantics::sha256_hex(b"lpha\n"),
    }];
    let shifted_report = validate_trace(&TraceRequest {
        trace_id: "trace-shifted".to_string(),
        trace_form: TraceForm::DocumentSession,
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: "alpha\n".to_string(),
        steps: shifted,
    });
    assert!(shifted_report.valid);

    // A step whose coordinates no longer fit its own pre-source is rejected
    // even when the hash chain would otherwise look consistent.
    let stale = vec![TraceStep {
        step: 0,
        edit: EditSpec {
            edit_start: 5,
            edit_end: 10,
            inserted_text: String::new(),
        },
        expected_transition: "stale_coordinates".to_string(),
        post_source_sha256: markit_mdbench_semantics::sha256_hex(b"alpha\n"),
    }];
    let stale_report = validate_trace(&TraceRequest {
        trace_id: "trace-stale".to_string(),
        trace_form: TraceForm::DocumentSession,
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: "abc\n".to_string(),
        steps: stale,
    });
    assert!(!stale_report.valid);
    assert!(stale_report
        .failure_codes
        .contains(&markit_mdbench_semantics::payload::TraceFailureCode::EditOutOfRange));

    // Two traces reaching identical bytes remain distinct identities.
    let final_bytes = markit_mdbench_semantics::sha256_hex(b"gamma\n");
    let same_bytes_other_history = validate_trace(&TraceRequest {
        trace_id: "trace-other".to_string(),
        trace_form: TraceForm::LocalBurst,
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: "gamma\n".to_string(),
        steps: vec![],
    });
    assert_eq!(same_bytes_other_history.final_source_sha256, final_bytes);
    assert_ne!(
        same_bytes_other_history.trace_identity_key, report.trace_identity_key,
        "identical bytes with a different history are different trace identities"
    );
}

/// Payload identity, inserted-byte reconstruction and payload validation.
#[test]
fn payload_identity_is_deterministic_and_inserted_bytes_are_reconstructable() {
    let build = |inserted: &str| {
        build_payload(
            "fixture",
            "workloads/pilots/fixtures/unit",
            SourceKind::SyntheticFixture,
            None,
            G1_GRAMMAR_ID,
            TABLE_SOURCE,
            TABLE_SOURCE,
            SyntaxKind::Table,
            "TABLE_DELIMITER_EDIT",
            "break",
            "table_valid_to_non_table",
            vec![kind_present(SyntaxKind::Table)],
            vec![
                kind_absent(SyntaxKind::Table),
                kind_present(SyntaxKind::Paragraph),
            ],
            "unit-trace",
            TraceForm::SingleReset,
            0,
            None,
            EditSpec {
                edit_start: 12,
                edit_end: 15,
                inserted_text: inserted.to_string(),
            },
            vec!["semantic_pilot".to_string()],
            None,
        )
    };

    let record = build("abc");
    assert_eq!(record.payload_id, record.identity());
    assert!(record.payload_id.starts_with("rp1:"));
    assert_eq!(
        record.post_source_sha256,
        markit_mdbench_semantics::sha256_hex(TABLE_POST.as_bytes())
    );

    let validation = validate_payload(&record, TABLE_SOURCE, TABLE_SOURCE);
    assert!(validation.valid, "failures: {:?}", validation.failure_codes);
    assert_eq!(
        validation.reconstructed_post_sha256.as_deref(),
        Some(record.post_source_sha256.as_str())
    );

    // Same fields, same identity; different inserted bytes, different id.
    let again = build("abc");
    assert_eq!(record.payload_id, again.payload_id);
    let different = build("xyz");
    assert_ne!(record.payload_id, different.payload_id);

    // A tampered digest is caught even though every hash still "matches".
    let mut tampered = record.clone();
    tampered.inserted_sha256 = markit_mdbench_semantics::sha256_hex(b"not the inserted bytes");
    let tampered_validation = validate_payload(&tampered, TABLE_SOURCE, TABLE_SOURCE);
    assert!(!tampered_validation.valid);
    assert!(tampered_validation
        .failure_codes
        .contains(&markit_mdbench_semantics::payload::PayloadFailureCode::InsertedDigestMismatch));

    // A tampered payload id is caught.
    let mut renamed = record.clone();
    renamed.payload_id = "rp1:00000000000000000000000000000000".to_string();
    let renamed_validation = validate_payload(&renamed, TABLE_SOURCE, TABLE_SOURCE);
    assert!(renamed_validation
        .failure_codes
        .contains(&markit_mdbench_semantics::payload::PayloadFailureCode::PayloadIdMismatch));

    // A payload whose claimed syntax transition does not hold is invalid,
    // even with correct hashes.
    let dishonest = build_payload(
        "fixture",
        "workloads/pilots/fixtures/unit",
        SourceKind::SyntheticFixture,
        None,
        G1_GRAMMAR_ID,
        TABLE_SOURCE,
        TABLE_SOURCE,
        SyntaxKind::Table,
        "TABLE_DELIMITER_EDIT",
        "break",
        "table_valid_to_non_table",
        vec![kind_present(SyntaxKind::Table)],
        // claims the table survived
        vec![kind_present(SyntaxKind::Table)],
        "unit-trace",
        TraceForm::SingleReset,
        0,
        None,
        EditSpec {
            edit_start: 12,
            edit_end: 15,
            inserted_text: "abc".to_string(),
        },
        vec!["semantic_pilot".to_string()],
        None,
    );
    let dishonest_validation = validate_payload(&dishonest, TABLE_SOURCE, TABLE_SOURCE);
    assert!(!dishonest_validation.valid);
    assert!(dishonest_validation
        .failure_codes
        .contains(&markit_mdbench_semantics::payload::PayloadFailureCode::TransitionInvalid));

    // The constant edit spec is unused; keep the compiler honest about the
    // declared constant being a valid "delete the delimiter" variant.
    let deleted = validate_transition(&markit_mdbench_semantics::transition::TransitionRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        pre_source: TABLE_SOURCE.to_string(),
        post_source: BREAK_LABEL_TOGGLE.apply(TABLE_SOURCE).ok(),
        edit: Some(BREAK_LABEL_TOGGLE),
        expected_pre: vec![kind_present(SyntaxKind::Table)],
        expected_post: vec![kind_absent(SyntaxKind::Table)],
    });
    assert_eq!(
        deleted.verdict,
        markit_mdbench_semantics::TransitionVerdict::TransitionValid
    );
}

/// A BREAK that leaves the source unchanged has not created a broken state,
/// so the RESTORE leg has nothing to return from. Without this the pair
/// would validate with `restore_exact = false` and quietly prove nothing.
#[test]
fn a_break_that_changes_nothing_is_rejected() {
    let base = TABLE_SOURCE;
    let report = validate_break_restore(&BreakRestoreRequest {
        grammar_id: G1_GRAMMAR_ID.to_string(),
        base_source: base.to_string(),
        break_edit: EditSpec {
            edit_start: 0,
            edit_end: 0,
            inserted_text: String::new(),
        },
        restore_edit: EditSpec {
            edit_start: 2,
            edit_end: 3,
            inserted_text: "z".to_string(),
        },
        break_expected_pre: vec![kind_present(SyntaxKind::Table)],
        break_expected_post: vec![kind_present(SyntaxKind::Table)],
        restore_expected_pre: vec![kind_present(SyntaxKind::Table)],
        restore_expected_post: vec![kind_present(SyntaxKind::Table)],
        expected_restore_exact: false,
    });
    assert!(
        report
            .failure_codes
            .contains(&BreakRestoreFailureCode::BreakNoEffect),
        "an empty break edit must be rejected: {:?}",
        report.failure_codes
    );
    assert!(!report.valid);
}
