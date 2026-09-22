//! CORRECTIVE-A required tests — profiler fact classes.
//!
//! Test families covered here: 2 (host-language rule), 3 (span evidence),
//! 4 (ambiguous/unknown stay explicit), plus source-fact and determinism
//! checks.
//!
//! The fixtures loaded here are the same declared pilot fixtures the pilot
//! driver runs: expectations are hand-authored authority, not outputs.

use std::path::{Path, PathBuf};

use markit_mdbench_semantics::facts::{FactReason, LaneScopeGrade, NewlineForm, RecognitionStatus};
use markit_mdbench_semantics::pilot::load_fixture;
use markit_mdbench_semantics::{profile_g0, profile_g1, source_facts, SyntaxKind};

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

fn fixture_source(id: &str) -> String {
    let path = bench_root().join(format!("workloads/pilots/fixtures/{id}"));
    load_fixture(&path)
        .unwrap_or_else(|error| panic!("{error}"))
        .source
}

fn has_fact(
    profile: &markit_mdbench_semantics::LaneProfile,
    kind: SyntaxKind,
    status: RecognitionStatus,
    span: (usize, usize),
    host_context: bool,
) -> bool {
    profile.syntax_facts.iter().any(|fact| {
        fact.syntax_kind == kind
            && fact.recognition_status == status
            && (fact.source_start, fact.source_end) == span
            && fact.host_context == host_context
    })
}

/// Family 2 — Markdown-looking bytes inside fences/code are not host syntax.
#[test]
fn host_context_rule_keeps_literal_content_out_of_host_syntax() {
    let source = fixture_source("p05-g1-host-context.toml");
    let profile = profile_g1(&source);

    // The G1 oracle recognizes the fence and the code span themselves: they
    // ARE host syntax.
    assert!(
        profile
            .syntax_facts
            .iter()
            .any(|fact| fact.syntax_kind == SyntaxKind::CodeBlockFenced
                && fact.recognition_status == RecognitionStatus::Recognized
                && fact.host_context),
        "the fenced block is a host construct"
    );
    assert!(has_fact(
        &profile,
        SyntaxKind::CodeSpan,
        RecognitionStatus::Recognized,
        (62, 72),
        true
    ));

    // A table inside the fence body is reported, but never as host syntax
    // and never as recognized.
    assert!(has_fact(
        &profile,
        SyntaxKind::Table,
        RecognitionStatus::NotRecognized,
        (22, 45),
        false
    ));
    assert!(!profile.syntax_facts.iter().any(|fact| {
        fact.syntax_kind == SyntaxKind::Table
            && fact.recognition_status == RecognitionStatus::Recognized
    }));
    assert!(!profile
        .syntax_facts
        .iter()
        .any(|fact| { fact.syntax_kind == SyntaxKind::Table && fact.host_context }));

    // `$not math$` inside the fence and `$inline$` inside the code span are
    // candidates in non-host context, never host math evidence.
    assert!(has_fact(
        &profile,
        SyntaxKind::InlineMath,
        RecognitionStatus::NotRecognized,
        (46, 56),
        false
    ));
    assert!(has_fact(
        &profile,
        SyntaxKind::InlineMath,
        RecognitionStatus::NotRecognized,
        (63, 71),
        false
    ));
    for fact in &profile.syntax_facts {
        if matches!(
            fact.syntax_kind,
            SyntaxKind::Table | SyntaxKind::InlineMath | SyntaxKind::DisplayMath
        ) {
            assert!(
                !fact.host_context,
                "literal content must not be host syntax"
            );
            assert_eq!(fact.reason, FactReason::NonHostContext);
            assert_ne!(fact.recognition_status, RecognitionStatus::Recognized);
            assert!(!fact.is_strict_coverage());
            assert!(!fact.is_scope_blocker());
        }
    }

    // Nothing outside the lane's scope appears at host context, so the
    // source stays strictly clean even though it contains a table shape.
    assert!(profile.eligibility.strict_scope_clean);
    assert!(profile.eligibility.scope_blockers.is_empty());
    assert_eq!(profile.structural.table, None);
}

/// Family 2 (G0) — the same rule through the repository reference parse.
#[test]
fn host_context_rule_holds_for_g0_code_spans() {
    let profile = profile_g0("a `| x | y |` b\n");
    assert!(!profile.syntax_facts.iter().any(|fact| {
        fact.syntax_kind == SyntaxKind::Table
            && fact.host_context
            && fact.recognition_status == RecognitionStatus::Recognized
    }));
    // The code span itself is recognized host syntax; its content is not
    // scanned for host constructs.
    assert!(profile
        .syntax_facts
        .iter()
        .any(|fact| fact.syntax_kind == SyntaxKind::CodeSpan && fact.host_context));
}

/// Family 3 — recognized facts carry exact, bounded byte spans.
#[test]
fn recognized_facts_carry_byte_spans_on_char_boundaries() {
    let source = fixture_source("p07-cjk-utf8.toml");
    let profile = profile_g1(&source);

    let heading = profile
        .syntax_facts
        .iter()
        .find(|fact| {
            fact.syntax_kind == SyntaxKind::HeadingAtx
                && fact.recognition_status == RecognitionStatus::Recognized
        })
        .expect("heading fact");
    // "# 中文标题\n" — the oracle span includes the terminator.
    assert_eq!((heading.source_start, heading.source_end), (0, 15));
    assert!(source.is_char_boundary(heading.source_start));
    assert!(source.is_char_boundary(heading.source_end));
    assert_eq!(
        &source[heading.source_start..heading.source_end],
        "# 中文标题\n"
    );

    let emoji_cell = profile
        .syntax_facts
        .iter()
        .filter(|fact| fact.syntax_kind == SyntaxKind::TableCell)
        .map(|fact| &source[fact.source_start..fact.source_end])
        .find(|text| text.contains('🐉'))
        .expect("emoji cell span");
    // The emoji is a 4-byte sequence: a character-based profiler would cut
    // this span short.
    assert!(emoji_cell.contains("🐉"));

    // Spans are half-open and within bounds everywhere.
    for fact in &profile.syntax_facts {
        assert!(fact.source_start <= fact.source_end);
        assert!(fact.source_end <= source.len());
        assert!(source.is_char_boundary(fact.source_start));
        assert!(source.is_char_boundary(fact.source_end));
    }
}

/// Family 3 (continued) — the table's structural facts come from the
/// recognized construct, not from the shape of the text.
#[test]
fn table_facts_come_from_the_recognized_construct() {
    let source = fixture_source("p02-g1-table-delimiter-break.toml");
    let profile = profile_g1(&source);
    let table = profile.structural.table.as_ref().expect("table facts");
    assert_eq!(table.table_count, 1);
    assert_eq!(table.max_table_columns, 2);
    assert_eq!(table.max_table_rows_including_header, 2);
    assert_eq!(profile.structural.block_count, 1);
    assert!(profile.eligibility.strict_scope_clean);
    // G0 sees the same bytes as a paragraph: no table is claimed there.
    let g0 = profile_g0(&source);
    assert_eq!(g0.structural.table, None);
    assert!(!g0.eligibility.strict_scope_clean);
    assert!(g0
        .eligibility
        .scope_blockers
        .iter()
        .any(|blocker| blocker.syntax_kind == SyntaxKind::Table));
}

/// Family 4 — ambiguous and unknown candidates stay explicit.
#[test]
fn ambiguous_and_unknown_candidates_are_never_resolved() {
    let source = fixture_source("p08-g1-math-ambiguous.toml");
    let profile = profile_g1(&source);

    let inline = profile
        .syntax_facts
        .iter()
        .find(|fact| fact.syntax_kind == SyntaxKind::InlineMath)
        .expect("inline math candidate");
    assert_eq!(inline.recognition_status, RecognitionStatus::Ambiguous);
    assert_eq!(inline.lane_scope_grade, LaneScopeGrade::LaneDeferred);
    assert_eq!(inline.reason, FactReason::AmbiguousAcrossDeclaredLanes);

    let unterminated = profile
        .syntax_facts
        .iter()
        .find(|fact| {
            fact.syntax_kind == SyntaxKind::DisplayMath
                && fact.recognition_status == RecognitionStatus::Unknown
        })
        .expect("unterminated display math candidate");
    assert_eq!(unterminated.reason, FactReason::LaneSemanticsDeferred);
    assert_eq!(unterminated.lane_scope_grade, LaneScopeGrade::LaneDeferred);

    // No math is ever recognized or counted, and the source is not clean.
    assert!(!profile.syntax_facts.iter().any(|fact| {
        matches!(
            fact.syntax_kind,
            SyntaxKind::InlineMath | SyntaxKind::DisplayMath
        ) && fact.recognition_status == RecognitionStatus::Recognized
    }));
    assert!(!profile.eligibility.strict_scope_clean);
    assert!(profile
        .eligibility
        .scope_blockers
        .iter()
        .any(|blocker| blocker.syntax_kind == SyntaxKind::DisplayMath));

    // Under G0 the same candidates are plain out-of-lane evidence.
    let g0 = profile_g0(&source);
    let g0_math = g0
        .syntax_facts
        .iter()
        .find(|fact| fact.syntax_kind == SyntaxKind::InlineMath)
        .expect("G0 inline math candidate");
    assert_eq!(g0_math.recognition_status, RecognitionStatus::NotRecognized);
    assert_eq!(g0_math.reason, FactReason::OutsideLaneConstructSet);
}

/// A candidate the lane oracle declined on shape grounds is recorded, not
/// dropped: absent evidence is never silently reported as "no construct".
#[test]
fn probe_oracle_disagreement_is_recorded() {
    // The lexical probe splits cells on every `|`, so it accepts this shape
    // (2 header cells, 2 delimiter cells). The lane rule is escape-aware:
    // `\|` is literal cell content, so the header has ONE cell against a
    // 2-cell delimiter row and GFM has no table here. The disagreement is
    // recorded as candidate evidence - it is never silently dropped, and it
    // is never counted as recognized coverage either.
    let source = "| a \\| b |\n| --- | --- |\n";
    let profile = profile_g1(source);
    assert!(
        profile.syntax_facts.iter().any(|fact| {
            fact.syntax_kind == SyntaxKind::Table
                && fact.recognition_status == RecognitionStatus::NotRecognized
                && fact.reason == FactReason::CandidateRejectedByLaneRule
                && fact.host_context
        }),
        "the declined table shape must be recorded as evidence"
    );
    assert!(!profile.syntax_facts.iter().any(|fact| {
        fact.syntax_kind == SyntaxKind::Table
            && fact.recognition_status == RecognitionStatus::Recognized
    }));
    assert_eq!(profile.structural.table, None);

    // Where the lane oracle DOES own the construct, the recognized fact is
    // the only table evidence: the lexical candidate is not duplicated as a
    // second, competing fact beside it.
    let recognized = profile_g1("| a | b |\n| --- | --- |\n");
    assert_eq!(
        recognized
            .syntax_facts
            .iter()
            .filter(|fact| fact.syntax_kind == SyntaxKind::Table)
            .count(),
        1,
        "a recognized construct carries exactly one fact"
    );
    assert_eq!(
        recognized
            .structural
            .table
            .expect("table facts")
            .table_count,
        1
    );

    // A header row followed by a delimiter row with the wrong cell count is
    // not even a probe candidate: the frozen probe rule requires matching
    // counts, and absence of a candidate is not absence of a construct.
    let mismatched = profile_g1("| a | b |\n| --- |\n");
    assert!(mismatched.syntax_facts.iter().all(|fact| {
        !(fact.syntax_kind == SyntaxKind::Table
            && fact.recognition_status == RecognitionStatus::Recognized)
    }));
}

/// SOURCE_FACT basics, including the CJK byte share and newline facts.
#[test]
fn source_facts_are_parser_independent_and_byte_exact() {
    let source = "# 中文\n\nbody\r\nline\n";
    let facts = source_facts(source);
    assert_eq!(facts.file_bytes, source.len() as u64);
    assert_eq!(facts.crlf_count, 1);
    // Every 0x0A byte counts as an LF, including the one inside a CRLF.
    assert_eq!(facts.lf_count, 4);
    assert_eq!(facts.cr_count, 1);
    assert_eq!(facts.newline_form, NewlineForm::Mixed);
    // The last byte is 0x0A, so the final line IS terminated; a source whose
    // last byte is not 0x0A reports false and counts one more line.
    assert!(facts.final_line_terminated);
    assert_eq!(facts.line_count, 4);
    let unterminated = source_facts("body\r\nline");
    assert!(!unterminated.final_line_terminated);
    assert_eq!(unterminated.line_count, 2);
    assert_eq!(unterminated.crlf_count, 1);
    assert!(facts.cjk_bytes > 0);
    assert!(
        (facts.cjk_byte_share - facts.cjk_bytes as f64 / facts.file_bytes as f64).abs() < 1e-12
    );
    assert_eq!(facts.source_sha256.len(), 64);
    assert!(facts.utf8_valid);

    // Zero-denominator rule: no NaN, no infinity.
    let empty = source_facts("");
    assert_eq!(empty.file_bytes, 0);
    assert_eq!(empty.cjk_byte_share, 0.0);
    assert_eq!(empty.line_count, 0);
    assert_eq!(empty.newline_form, NewlineForm::None);
}

/// The same source and lane must profile byte-identically twice.
#[test]
fn profiling_is_byte_deterministic() {
    let source = fixture_source("p07-cjk-utf8.toml");
    let first = markit_mdbench_semantics::canonical_json(&profile_g1(&source));
    let second = markit_mdbench_semantics::canonical_json(&profile_g1(&source));
    assert_eq!(first, second);

    let g0_first = markit_mdbench_semantics::canonical_json(&profile_g0(&source));
    let g0_second = markit_mdbench_semantics::canonical_json(&profile_g0(&source));
    assert_eq!(g0_first, g0_second);
}

/// A lexical probe can anchor on a literal construct's own delimiter line:
/// the setext probe takes the line *above* the underline, and the table
/// probe starts at the header row's first byte. On a fence that opens with
/// those bytes, the candidate therefore begins at the fence opener and is
/// NOT contained by the fence's content interval.
///
/// The candidate rule resolves against each construct's *full* span, so
/// these bytes are still literal content. Without that, a document that is
/// nothing but a fenced code block would report a host-context scope
/// blocker and be declared ineligible for the lane that owns fences.
#[test]
fn a_candidate_anchored_on_a_fence_opener_is_still_literal_content() {
    // The setext probe's candidate spans the opener line plus the underline.
    let source = "```\n=====\nTitle\n```\n";
    let profile = profile_g0(source);
    let setext: Vec<_> = profile
        .syntax_facts
        .iter()
        .filter(|fact| fact.syntax_kind == SyntaxKind::HeadingSetext)
        .collect();
    assert!(
        !setext.is_empty(),
        "the setext probe must still report the candidate as evidence"
    );
    for fact in &setext {
        assert!(
            !fact.host_context,
            "a candidate inside a fence is literal content, got {:?}",
            (fact.source_start, fact.source_end, fact.reason)
        );
    }
    assert!(
        profile.eligibility.strict_scope_clean,
        "a lone fenced code block is G0-strict; blockers={:?}",
        profile
            .eligibility
            .scope_blockers
            .iter()
            .map(|fact| (fact.syntax_kind.name(), fact.source_start, fact.source_end))
            .collect::<Vec<_>>()
    );

    // The table probe starts at the header row, which here is the opener.
    // Only *candidate* facts are checked: the fence node itself is host
    // syntax and must stay host-context.
    let table_shaped_fence = "```|\n| --- |\nbody\n```\n";
    assert_candidates_are_literal(table_shaped_fence, &profile_g0(table_shaped_fence));

    // An indented code block's table-shaped bytes are literal too.
    let indented = "    | a | b |\n    | - | - |\n";
    assert_candidates_are_literal(indented, &profile_g1(indented));
}

/// Every non-recognized fact (a probe candidate) in this profile must be
/// marked as literal content, never as host syntax.
fn assert_candidates_are_literal(source: &str, profile: &markit_mdbench_semantics::LaneProfile) {
    let candidates: Vec<_> = profile
        .syntax_facts
        .iter()
        .filter(|fact| fact.recognition_status != RecognitionStatus::Recognized)
        .collect();
    assert!(
        !candidates.is_empty(),
        "expected probe candidates for {source:?}"
    );
    for fact in candidates {
        assert!(
            !fact.host_context,
            "literal bytes reported as host syntax in {source:?}: {:?}",
            (fact.syntax_kind.name(), fact.source_start, fact.source_end)
        );
    }
}

/// The empty document is a legitimate input: BENCH-GRAMMAR-v1 is total over
/// UTF-8, so profiling zero bytes must produce a profile, not a panic.
#[test]
fn the_empty_source_profiles_under_both_lanes() {
    let facts = source_facts("");
    assert_eq!(facts.file_bytes, 0);

    let g0 = profile_g0("");
    assert!(g0.eligibility.lane_valid);
    assert!(g0.eligibility.strict_scope_clean);
    assert!(g0.syntax_facts.is_empty());
    assert_eq!(g0.structural.block_count, 0);

    let g1 = profile_g1("");
    assert!(g1.eligibility.lane_valid);
    assert!(g1.eligibility.strict_scope_clean);
}

#[test]
fn fenced_content_interval_handles_container_prefixed_closers() {
    use markit_mdbench_semantics::facts::Span;
    use markit_mdbench_semantics::fenced_content_interval;

    fn content(source: &str) -> &str {
        let span = Span::new(0, source.len());
        let interval = fenced_content_interval(source, span, '`');
        &source[interval.start..interval.end]
    }

    // Oracle spans end at the closer's backtick run (no trailing LF).
    // Top-level closer: content excludes the closing fence line.
    assert_eq!(content("```rust\nfn main() {}\n```"), "fn main() {}\n");
    // Quoted closer (container prefix on the closing fence line): the
    // rule must recognize it, exactly like the oracle state machines.
    assert_eq!(content("> ```\n> a : b\n> ```"), "> a : b\n");
    // List-indented quoted closer.
    assert_eq!(content("- > ```toml\n  > k = 1\n  > ```"), "  > k = 1\n");
    // Closer run may exceed the opener run.
    assert_eq!(content("```\na\n`````"), "a\n");
    // Digits/dots list prefixes count as container prefixes.
    assert_eq!(content("1. ```\n   k\n   ```"), "   k\n");
    // Unclosed fences (span may carry a final LF) keep all body bytes,
    // including body lines that merely contain or end in backticks.
    assert_eq!(content("```\nx = a```b"), "x = a```b");
    assert_eq!(content("```\na\nsee note``` more"), "a\nsee note``` more");
    assert_eq!(content("```\na\n"), "a\n");
}
