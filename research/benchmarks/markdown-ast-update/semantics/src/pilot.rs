//! SEMANTIC_PILOT_SET — the CORRECTIVE-A contract-validation pilot.
//!
//! Authority: `workloads/pilots/README.md` + CORRECTIVE-A §N. This is a
//! **pilot**, not a workload: it is explicitly not the representative set,
//! the extremal set, the syntax-coverage set or the full-document set, and
//! no representativeness claim may be derived from it.
//!
//! The pilot exists to prove, mechanically, that the substrate works:
//!
//! ```text
//! profiler      declared expectations vs observed facts, per lane
//! transition    expected_pre / expected_post under the lane oracle
//! eligibility   pre and post checked independently
//! lifecycle     BREAK -> RESTORE SHA chain, position dedup, payload validation
//! ```

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::canonical::{canonical_json, sha256_hex};
use crate::facts::{
    FactReason, LaneScopeGrade, RecognitionStatus, Span, StructuralFacts, SyntaxFact, SyntaxKind,
    PROFILER_VERSION,
};
use crate::lanes::{lane_spec, LANE_REGISTRY_VERSION};
use crate::payload::{
    build_payload, resolve_positions, validate_break_restore, validate_payload, validate_trace,
    Anchor, BreakRestoreReport, BreakRestoreRequest, PayloadPosition, PayloadValidation,
    PositionResolutionOutcome, RequestedPosition, SourceKind, TraceForm, TraceRequest,
    PAYLOAD_LIFECYCLE_VERSION,
};
use crate::profile::{lane_profile_with, LaneProfile};
use crate::transition::{
    validate_transition, EditSpec, PredicateV1, TransitionReport, TRANSITION_ORACLE_VERSION,
};

/// Frozen pilot-set identifier.
pub const PILOT_SET_ID: &str = "SEMANTIC_PILOT_SET";
/// Frozen artifact schemas.
pub const PILOT_MANIFEST_SCHEMA: &str = "semantic-pilot-manifest-v1";
pub const PILOT_RESULTS_SCHEMA: &str = "semantic-pilot-results-v1";
/// Frozen fixture schema.
pub const PILOT_FIXTURE_SCHEMA: &str = "semantic-pilot-fixture-v1";

/// One hand-authored pilot fixture (declared authority, like the R3
/// grammar fixtures: the expectation is true by declaration under the lane
/// contract, and is compared against both the profiler and the oracle).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotFixture {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub role: String,
    /// Lane ids this fixture is exercised under (`G0` / `G1`).
    pub lanes: Vec<String>,
    pub source: String,
    pub source_sha256: String,
    /// Declared post source for the transition case (`S1`).
    #[serde(default)]
    pub post_source: Option<String>,
    #[serde(default)]
    pub post_source_sha256: Option<String>,
    /// Declared profiles expectations, one section per lane.
    #[serde(default)]
    pub expect: Vec<LaneExpectation>,
    #[serde(default)]
    pub transition: Option<FixtureTransition>,
    #[serde(default)]
    pub position: Option<FixturePosition>,
    /// Declared chained trace (semantics only; not a long-session lane).
    #[serde(default)]
    pub trace: Option<FixtureTrace>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaneExpectation {
    /// Lane id: `G0` or `G1`.
    pub lane: String,
    #[serde(default)]
    pub strict_scope_clean: Option<bool>,
    #[serde(default)]
    pub block_count: Option<u64>,
    #[serde(default)]
    pub max_container_depth: Option<u32>,
    #[serde(default)]
    pub table_count: Option<u64>,
    #[serde(default)]
    pub code_occupancy_zero: Option<bool>,
    /// Facts that must exist exactly (kind, status, span, host context).
    #[serde(default)]
    pub facts: Vec<ExpectedFact>,
    /// Kinds that must have no recognized fact at all.
    #[serde(default)]
    pub absent_kinds: Vec<SyntaxKind>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedFact {
    pub kind: SyntaxKind,
    pub status: RecognitionStatus,
    /// Declared byte span; omitted when only the kind/status matters.
    #[serde(default)]
    pub span: Option<[usize; 2]>,
    #[serde(default)]
    pub host_context: Option<bool>,
    #[serde(default)]
    pub grade: Option<LaneScopeGrade>,
    #[serde(default)]
    pub reason: Option<FactReason>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixtureTransition {
    pub syntax_target: SyntaxKind,
    pub expected_transition: String,
    pub edit_family: String,
    pub operation_variant: String,
    pub break_edit: EditSpec,
    #[serde(default)]
    pub restore_edit: Option<EditSpec>,
    #[serde(default)]
    pub restore_exact: Option<bool>,
    /// `[[transition.pre]]` / `[[transition.post]]` predicate tables.
    #[serde(default)]
    pub pre: Vec<PredicateV1>,
    #[serde(default)]
    pub post: Vec<PredicateV1>,
    /// Declared expectation of the post-source eligibility verdict
    /// (checked independently of the transition verdict).
    #[serde(default)]
    pub post_strict_comparison_eligible: Option<bool>,
    /// Predicates for the RESTORE leg, when declared.
    #[serde(default)]
    pub restore_pre: Vec<PredicateV1>,
    #[serde(default)]
    pub restore_post: Vec<PredicateV1>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixturePosition {
    pub lane: String,
    pub anchor_kind: SyntaxKind,
    /// Expected number of surviving (deduplicated) resolutions.
    pub expected_resolutions: usize,
    /// Whether the three requests are expected to collapse onto one anchor.
    pub expected_deduplicated: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixtureTrace {
    pub trace_id: String,
    pub form: TraceForm,
    pub steps: Vec<FixtureTraceStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixtureTraceStep {
    pub edit: EditSpec,
    pub expected_transition: String,
}

/// The pilot manifest (declared inventory, regenerated deterministically).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotManifest {
    pub schema: String,
    pub pilot_set_id: String,
    pub profiler_version: String,
    pub lane_registry_version: String,
    pub transition_oracle_version: String,
    pub payload_lifecycle_version: String,
    pub notice: String,
    pub fixtures: Vec<PilotFixture>,
    #[serde(default)]
    pub real_source: Option<RealSourceSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RealSourceSpec {
    pub source_id: String,
    /// Path relative to the benchmark root of the *materialized* acquisition
    /// bytes (gitignored; rebuilt by `workloads/tools/acquire.py`).
    pub path: String,
    pub source_sha256: String,
    #[serde(default)]
    pub acquisition_commit_sha: Option<String>,
    pub lanes: Vec<String>,
    pub why: String,
}

/// Observed outcome of one check inside the pilot.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckOutcome {
    pub check: String,
    pub passed: bool,
    /// Whether this outcome gates the pilot verdict. A check whose evidence
    /// cannot be evaluated at all (the real-source bytes are not materialized)
    /// still reports what it observed, but abstains from gating: absence of
    /// PR #34's ignored snapshot bytes is not a semantic failure. A check whose
    /// evidence IS present always gates, so a wrong source still fails closed.
    #[serde(default = "gating_default")]
    pub gating: bool,
    pub detail: String,
}

fn gating_default() -> bool {
    true
}

/// Compact per-lane summary used for real sources (the full fact list is
/// kept for fixtures only, so the committed artifact stays reviewable).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaneSummary {
    pub lane_id: String,
    pub grammar_id: String,
    pub strict_scope_clean: bool,
    pub scope_blocker_count: u64,
    pub scope_blocker_kinds: Vec<String>,
    pub structural: StructuralFacts,
    pub facts_by_kind_status: Vec<KindStatusCount>,
    pub recognized_strict_count: u64,
    pub unexpected_oracle_tags: Vec<String>,
    pub consistency_warnings: Vec<String>,
    pub profile_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KindStatusCount {
    pub kind: SyntaxKind,
    pub status: RecognitionStatus,
    pub host_context: bool,
    pub count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixtureResult {
    pub id: String,
    pub name: String,
    pub role: String,
    pub source_sha256: String,
    pub lanes: Vec<String>,
    pub checks: Vec<CheckOutcome>,
    pub profiles: Vec<LaneSummary>,
    pub facts: Vec<SyntaxFact>,
    #[serde(default)]
    pub transitions: Vec<TransitionReport>,
    #[serde(default)]
    pub break_restore: Option<BreakRestoreReport>,
    #[serde(default)]
    pub payload_validation: Option<PayloadValidation>,
    #[serde(default)]
    pub position: Option<PositionResolutionOutcome>,
    #[serde(default)]
    pub trace_report: Option<crate::payload::TraceReport>,
    pub pass: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RealSourceResult {
    pub spec: RealSourceSpec,
    pub available: bool,
    /// `verified` when the bytes were present and every gating check ran;
    /// `abstained_absent_bytes` when the PR #34 acquisition material was not
    /// materialized locally. `pass` alone must never be read as "the real
    /// source was checked", so the state is named in the artifact.
    pub status: String,
    #[serde(default)]
    pub observed_sha256: Option<String>,
    pub checks: Vec<CheckOutcome>,
    #[serde(default)]
    pub profiles: Vec<LaneSummary>,
    pub pass: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotResults {
    pub schema: String,
    pub pilot_set_id: String,
    pub profiler_version: String,
    pub lane_registry_version: String,
    pub transition_oracle_version: String,
    pub payload_lifecycle_version: String,
    pub fixtures: Vec<FixtureResult>,
    #[serde(default)]
    pub real_source: Option<RealSourceResult>,
    /// Work this pilot explicitly does NOT do (kept in the artifact so the
    /// boundary travels with the evidence).
    pub deferred: Vec<String>,
    pub pass: bool,
}

/// The real source was present and every gating check ran.
pub const REAL_SOURCE_VERIFIED: &str = "verified";
/// The real source's bytes are PR #34 acquisition material and were not
/// materialized locally: nothing about it was evaluated.
pub const REAL_SOURCE_ABSTAINED: &str = "abstained_absent_bytes";

/// Load one fixture file.
pub fn load_fixture(path: &Path) -> Result<PilotFixture, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let fixture: PilotFixture =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    if fixture.schema != PILOT_FIXTURE_SCHEMA {
        return Err(format!(
            "{}: schema must be {PILOT_FIXTURE_SCHEMA}, found {}",
            path.display(),
            fixture.schema
        ));
    }
    Ok(fixture)
}

/// Load every fixture under `<root>/workloads/pilots/fixtures`, sorted by
/// file name so the manifest is deterministic.
pub fn load_fixtures(bench_root: &Path) -> Result<Vec<PilotFixture>, String> {
    let dir = bench_root.join("workloads/pilots/fixtures");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|error| format!("{}: {error}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    paths.iter().map(|path| load_fixture(path)).collect()
}

/// Run one fixture end to end.
pub fn run_fixture(fixture: &PilotFixture) -> FixtureResult {
    let mut checks: Vec<CheckOutcome> = Vec::new();
    let mut profiles: Vec<LaneSummary> = Vec::new();
    let mut facts: Vec<SyntaxFact> = Vec::new();
    let mut transitions: Vec<TransitionReport> = Vec::new();
    let mut break_restore: Option<BreakRestoreReport> = None;
    let mut payload_validation: Option<PayloadValidation> = None;
    let mut position_outcome: Option<PositionResolutionOutcome> = None;
    let mut trace_report = None;

    let observed_sha = sha256_hex(fixture.source.as_bytes());
    checks.push(CheckOutcome {
        check: "fixture_source_sha256".to_string(),
        passed: observed_sha == fixture.source_sha256,
        gating: true,
        detail: format!("declared={} observed={observed_sha}", fixture.source_sha256),
    });

    let mut lane_profiles: Vec<(String, LaneProfile)> = Vec::new();
    for lane_id in &fixture.lanes {
        let Some(lane) = lane_for_id(lane_id) else {
            checks.push(CheckOutcome {
                check: format!("lane_{lane_id}_known"),
                passed: false,
                gating: true,
                detail: "unknown lane id in fixture".to_string(),
            });
            continue;
        };
        let profile = lane_profile_with(&fixture.source, &lane);
        checks.extend(check_lane_expectations(
            fixture,
            &profile,
            fixture
                .expect
                .iter()
                .find(|expectation| expectation.lane == *lane_id),
        ));
        profiles.push(summarize(&profile));
        facts.extend(profile.syntax_facts.clone());
        lane_profiles.push((lane_id.clone(), profile));
    }

    // Transition + BREAK/RESTORE + payload lifecycle.
    if let Some(transition) = &fixture.transition {
        let Some(post_source) = fixture.post_source.as_deref() else {
            checks.push(CheckOutcome {
                check: "transition_post_source_declared".to_string(),
                passed: false,
                gating: true,
                detail: "a transition fixture must declare post_source".to_string(),
            });
            return finish(
                fixture,
                checks,
                profiles,
                facts,
                transitions,
                break_restore,
                payload_validation,
                position_outcome,
                trace_report,
            );
        };
        if let Some(declared) = &fixture.post_source_sha256 {
            checks.push(CheckOutcome {
                check: "fixture_post_source_sha256".to_string(),
                passed: sha256_hex(post_source.as_bytes()) == *declared,
                gating: true,
                detail: format!(
                    "declared={declared} observed={}",
                    sha256_hex(post_source.as_bytes())
                ),
            });
        }

        for (lane_id, _) in &lane_profiles {
            let lane = lane_for_id(lane_id).expect("checked above");
            let report = validate_transition(&crate::transition::TransitionRequest {
                grammar_id: lane.grammar_id.clone(),
                pre_source: fixture.source.clone(),
                post_source: Some(post_source.to_string()),
                edit: Some(transition.break_edit.clone()),
                expected_pre: transition.pre.clone(),
                expected_post: transition.post.clone(),
            });
            if let Some(expected_eligible) = transition.post_strict_comparison_eligible {
                let observed = report.strict_comparison_eligible;
                checks.push(CheckOutcome {
                    check: format!("post_strict_comparison_eligibility_{lane_id}"),
                    passed: observed == expected_eligible,
                    gating: true,
                    detail: format!(
                        "strict_comparison_eligible={observed} expected={expected_eligible} \
                         post_scope_blockers={:?}",
                        report
                            .post
                            .as_ref()
                            .map(|post| post
                                .eligibility
                                .scope_blockers
                                .iter()
                                .map(|blocker| blocker.syntax_kind.name())
                                .collect::<Vec<_>>())
                            .unwrap_or_default()
                    ),
                });
            }
            checks.push(CheckOutcome {
                check: format!("transition_{lane_id}"),
                passed: report.is_valid(),
                gating: true,
                detail: format!(
                    "verdict={:?} failures={:?} pre_clean={} post_clean={} change_class={:?}",
                    report.verdict,
                    report.failure_codes,
                    report.pre.eligibility.strict_scope_clean,
                    report
                        .post
                        .as_ref()
                        .map(|post| post.eligibility.strict_scope_clean)
                        .unwrap_or(false),
                    report.change_class
                ),
            });
            transitions.push(report);
        }

        if let Some(restore_edit) = &transition.restore_edit {
            let lane_id = fixture
                .lanes
                .first()
                .cloned()
                .unwrap_or_else(|| "G1".into());
            let lane = lane_for_id(&lane_id).expect("checked above");
            let chain = validate_break_restore(&BreakRestoreRequest {
                grammar_id: lane.grammar_id.clone(),
                base_source: fixture.source.clone(),
                break_edit: transition.break_edit.clone(),
                break_expected_pre: transition.pre.clone(),
                break_expected_post: transition.post.clone(),
                restore_edit: restore_edit.clone(),
                restore_expected_pre: transition.restore_pre.clone(),
                restore_expected_post: transition.restore_post.clone(),
                expected_restore_exact: transition.restore_exact.unwrap_or(true),
            });
            checks.push(CheckOutcome {
                check: "break_restore_chain".to_string(),
                passed: chain.valid,
                gating: true,
                detail: format!(
                    "S0={} S1={} S2={} restore_exact={} failures={:?}",
                    &chain.base_source_sha256[..12],
                    &chain.s1_source_sha256[..12],
                    if chain.s2_source_sha256.len() >= 12 {
                        &chain.s2_source_sha256[..12]
                    } else {
                        "-"
                    },
                    chain.restore_exact,
                    chain.failure_codes
                ),
            });
            break_restore = Some(chain);
        }

        // Payload lifecycle over the declared BREAK edit.
        let lane_id = fixture
            .lanes
            .first()
            .cloned()
            .unwrap_or_else(|| "G1".into());
        let lane = lane_for_id(&lane_id).expect("checked above");
        let record = build_payload(
            &fixture.id,
            &format!("workloads/pilots/fixtures/{}", fixture.id),
            SourceKind::SyntheticFixture,
            None,
            &lane.grammar_id,
            &fixture.source,
            &fixture.source,
            transition.syntax_target,
            &transition.edit_family,
            &transition.operation_variant,
            &transition.expected_transition,
            transition.pre.clone(),
            transition.post.clone(),
            &format!("{}-break", fixture.id),
            TraceForm::SingleReset,
            0,
            None,
            transition.break_edit.clone(),
            vec!["semantic_pilot".to_string()],
            None,
        );
        let validation = validate_payload(&record, &fixture.source, &fixture.source);
        checks.push(CheckOutcome {
            check: "payload_lifecycle".to_string(),
            passed: validation.valid,
            gating: true,
            detail: format!(
                "payload_id={} failures={:?} post_sha={}",
                record.payload_id,
                validation.failure_codes,
                &record.post_source_sha256[..12]
            ),
        });
        payload_validation = Some(validation);
    }

    // Position dedup semantics.
    if let Some(position) = &fixture.position {
        let anchors: Vec<Anchor> = facts
            .iter()
            .filter(|fact| {
                fact.host_context
                    && fact.recognition_status == RecognitionStatus::Recognized
                    && fact.syntax_kind == position.anchor_kind
            })
            .map(|fact| Anchor {
                syntax_kind: fact.syntax_kind,
                span: Span::new(fact.source_start, fact.source_end),
                occurrence: fact.occurrence,
            })
            .collect();
        let outcome = resolve_positions(&anchors, fixture.source.len(), &RequestedPosition::all());
        checks.push(CheckOutcome {
            check: "position_resolution".to_string(),
            passed: outcome.resolutions.len() == position.expected_resolutions,
            gating: true,
            detail: format!(
                "anchors={} resolutions={} expected={} dedup_collapsed={} uncovered={:?}",
                anchors.len(),
                outcome.resolutions.len(),
                position.expected_resolutions,
                outcome
                    .resolutions
                    .iter()
                    .any(|resolution| resolution.deduplicated),
                outcome.uncovered_requests
            ),
        });
        if position.expected_deduplicated {
            let collapsed = outcome
                .resolutions
                .iter()
                .any(|resolution| resolution.deduplicated);
            checks.push(CheckOutcome {
                check: "position_dedup_collapse".to_string(),
                passed: collapsed,
                gating: true,
                detail: "EARLY/MIDDLE/LATE must collapse onto one physical anchor".to_string(),
            });
        }
        position_outcome = Some(outcome);

        // A payload that carries the deduplicated position record.
        if let Some(resolution) = position_outcome
            .as_ref()
            .and_then(|outcome| outcome.resolutions.first())
        {
            if let Some(transition) = &fixture.transition {
                let lane_id = fixture
                    .lanes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "G1".into());
                let lane = lane_for_id(&lane_id).expect("checked above");
                let record = build_payload(
                    &fixture.id,
                    &format!("workloads/pilots/fixtures/{}", fixture.id),
                    SourceKind::SyntheticFixture,
                    None,
                    &lane.grammar_id,
                    &fixture.source,
                    &fixture.source,
                    transition.syntax_target,
                    &transition.edit_family,
                    &transition.operation_variant,
                    &transition.expected_transition,
                    transition.pre.clone(),
                    transition.post.clone(),
                    &format!("{}-position", fixture.id),
                    TraceForm::SingleReset,
                    0,
                    Some(PayloadPosition::from_resolution(resolution)),
                    transition.break_edit.clone(),
                    vec!["semantic_pilot".to_string(), "position_probe".to_string()],
                    None,
                );
                let validation = validate_payload(&record, &fixture.source, &fixture.source);
                checks.push(CheckOutcome {
                    check: "position_payload_lifecycle".to_string(),
                    passed: validation.valid
                        && resolution.requested_positions.len()
                            == usize::from(position.expected_deduplicated) * 3
                                + usize::from(!position.expected_deduplicated),
                    gating: true,
                    detail: format!(
                        "requested_positions={:?} valid={} failures={:?}",
                        resolution.requested_positions, validation.valid, validation.failure_codes
                    ),
                });
            }
        }
    }

    // Chained trace semantics (coordinates belong to each step's pre-source).
    if let Some(trace) = &fixture.trace {
        let lane_id = fixture
            .lanes
            .first()
            .cloned()
            .unwrap_or_else(|| "G1".into());
        let lane = lane_for_id(&lane_id).expect("checked above");
        let mut source = fixture.source.clone();
        let mut steps = Vec::new();
        for (index, step) in trace.steps.iter().enumerate() {
            let post = step
                .edit
                .apply(&source)
                .expect("fixture trace edit is valid");
            steps.push(crate::payload::TraceStep {
                step: index as u32,
                edit: step.edit.clone(),
                expected_transition: step.expected_transition.clone(),
                post_source_sha256: sha256_hex(post.as_bytes()),
            });
            source = post;
        }
        let report = validate_trace(&TraceRequest {
            trace_id: trace.trace_id.clone(),
            trace_form: trace.form,
            grammar_id: lane.grammar_id.clone(),
            base_source: fixture.source.clone(),
            steps,
        });
        checks.push(CheckOutcome {
            check: "trace_replay".to_string(),
            passed: report.valid,
            gating: true,
            detail: format!(
                "steps={} final={} identity={} failures={:?}",
                report.steps.len(),
                &report.final_source_sha256[..12],
                report.trace_identity_key,
                report.failure_codes
            ),
        });
        trace_report = Some(report);
    }

    finish(
        fixture,
        checks,
        profiles,
        facts,
        transitions,
        break_restore,
        payload_validation,
        position_outcome,
        trace_report,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish(
    fixture: &PilotFixture,
    checks: Vec<CheckOutcome>,
    profiles: Vec<LaneSummary>,
    facts: Vec<SyntaxFact>,
    transitions: Vec<TransitionReport>,
    break_restore: Option<BreakRestoreReport>,
    payload_validation: Option<PayloadValidation>,
    position: Option<PositionResolutionOutcome>,
    trace_report: Option<crate::payload::TraceReport>,
) -> FixtureResult {
    let pass = checks.iter().all(|check| !check.gating || check.passed);
    FixtureResult {
        id: fixture.id.clone(),
        name: fixture.name.clone(),
        role: fixture.role.clone(),
        source_sha256: fixture.source_sha256.clone(),
        lanes: fixture.lanes.clone(),
        checks,
        profiles,
        facts,
        transitions,
        break_restore,
        payload_validation,
        position,
        trace_report,
        pass,
    }
}

fn lane_for_id(lane_id: &str) -> Option<crate::lanes::LaneSpec> {
    match lane_id {
        "G0" => Some(crate::lanes::g0_lane()),
        "G1" => Some(crate::lanes::g1_lane()),
        "G2" => lane_spec(crate::lanes::G2_GRAMMAR_ID),
        _ => None,
    }
}

fn check_lane_expectations(
    fixture: &PilotFixture,
    profile: &LaneProfile,
    expectation: Option<&LaneExpectation>,
) -> Vec<CheckOutcome> {
    let lane_id = &profile.lane_id;
    let mut checks = Vec::new();
    let Some(expectation) = expectation else {
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_expectation_declared"),
            passed: false,
            gating: true,
            detail: "fixture declares the lane but no expectations for it".to_string(),
        });
        return checks;
    };

    for expected in &expectation.facts {
        let declared_span = expected.span.map(|span| Span::new(span[0], span[1]));
        let found = profile.syntax_facts.iter().find(|fact| {
            fact.syntax_kind == expected.kind
                && fact.recognition_status == expected.status
                && declared_span
                    .map(|span| fact.span() == span)
                    .unwrap_or(true)
                && expected
                    .host_context
                    .map(|host| host == fact.host_context)
                    .unwrap_or(true)
        });
        let detail = match found {
            Some(fact) => format!(
                "observed status={:?} grade={:?} host_context={} reason={:?}",
                fact.recognition_status, fact.lane_scope_grade, fact.host_context, fact.reason
            ),
            None => format!(
                "no fact kind={} status={:?} span={:?} in {} facts",
                expected.kind.name(),
                expected.status,
                declared_span.map(|span| (span.start, span.end)),
                profile.syntax_facts.len()
            ),
        };
        let grade_ok = expected
            .grade
            .map(|grade| found.is_some_and(|fact| fact.lane_scope_grade == grade))
            .unwrap_or(true);
        let reason_ok = expected
            .reason
            .map(|reason| found.is_some_and(|fact| fact.reason == reason))
            .unwrap_or(true);
        checks.push(CheckOutcome {
            check: format!(
                "lane_{lane_id}_fact_{}_{:?}",
                expected.kind.name(),
                expected.status
            ) + &declared_span
                .map(|span| format!("_{}-{}", span.start, span.end))
                .unwrap_or_default(),
            passed: found.is_some() && grade_ok && reason_ok,
            gating: true,
            detail,
        });
    }

    for kind in &expectation.absent_kinds {
        let recognized = profile
            .syntax_facts
            .iter()
            .filter(|fact| {
                fact.syntax_kind == *kind
                    && fact.recognition_status == RecognitionStatus::Recognized
            })
            .count();
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_absent_{}", kind.name()),
            passed: recognized == 0,
            gating: true,
            detail: format!("recognized facts of this kind: {recognized}"),
        });
    }

    if let Some(expected_clean) = expectation.strict_scope_clean {
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_strict_scope_clean"),
            passed: profile.eligibility.strict_scope_clean == expected_clean,
            gating: true,
            detail: format!(
                "strict_scope_clean={} blockers={:?}",
                profile.eligibility.strict_scope_clean,
                profile
                    .eligibility
                    .scope_blockers
                    .iter()
                    .map(|blocker| blocker.syntax_kind.name())
                    .collect::<Vec<_>>()
            ),
        });
    }
    if let Some(expected_blocks) = expectation.block_count {
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_block_count"),
            passed: profile.structural.block_count == expected_blocks,
            gating: true,
            detail: format!(
                "block_count={} expected={expected_blocks}",
                profile.structural.block_count
            ),
        });
    }
    if let Some(expected_depth) = expectation.max_container_depth {
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_max_container_depth"),
            passed: profile.structural.max_container_depth == expected_depth,
            gating: true,
            detail: format!(
                "max_container_depth={} expected={expected_depth}",
                profile.structural.max_container_depth
            ),
        });
    }
    if let Some(expected_tables) = expectation.table_count {
        let observed = profile
            .structural
            .table
            .as_ref()
            .map(|table| table.table_count)
            .unwrap_or(0);
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_table_count"),
            passed: observed == expected_tables,
            gating: true,
            detail: format!("table_count={observed} expected={expected_tables}"),
        });
    }
    if let Some(expected_zero) = expectation.code_occupancy_zero {
        let observed = profile.structural.code_occupancy == 0.0;
        checks.push(CheckOutcome {
            check: format!("lane_{lane_id}_code_occupancy"),
            passed: observed == expected_zero,
            gating: true,
            detail: format!(
                "code_occupancy={} fenced_bytes={}",
                profile.structural.code_occupancy, profile.structural.fenced_code_content_bytes
            ),
        });
    }

    let _ = fixture;
    checks
}

/// Compact lane summary (used for fixtures and real sources alike).
pub fn summarize(profile: &LaneProfile) -> LaneSummary {
    let mut counts: Vec<KindStatusCount> = Vec::new();
    for fact in &profile.syntax_facts {
        match counts.iter_mut().find(|count| {
            count.kind == fact.syntax_kind
                && count.status == fact.recognition_status
                && count.host_context == fact.host_context
        }) {
            Some(count) => count.count += 1,
            None => counts.push(KindStatusCount {
                kind: fact.syntax_kind,
                status: fact.recognition_status,
                host_context: fact.host_context,
                count: 1,
            }),
        }
    }
    let mut scope_blocker_kinds: Vec<String> = profile
        .eligibility
        .scope_blockers
        .iter()
        .map(|blocker| blocker.syntax_kind.name().to_string())
        .collect();
    scope_blocker_kinds.sort();
    scope_blocker_kinds.dedup();

    LaneSummary {
        lane_id: profile.lane_id.clone(),
        grammar_id: profile.grammar_id.clone(),
        strict_scope_clean: profile.eligibility.strict_scope_clean,
        scope_blocker_count: profile.eligibility.scope_blockers.len() as u64,
        scope_blocker_kinds,
        structural: profile.structural.clone(),
        facts_by_kind_status: counts,
        recognized_strict_count: profile
            .syntax_facts
            .iter()
            .filter(|fact| fact.is_strict_coverage())
            .count() as u64,
        unexpected_oracle_tags: profile.unexpected_oracle_tags.clone(),
        consistency_warnings: profile.consistency_warnings.clone(),
        profile_sha256: sha256_hex(canonical_json(profile).as_bytes()),
    }
}

/// Run the pilot: fixtures plus the optional real-source case.
pub fn run_pilot(
    bench_root: &Path,
    fixtures: &[PilotFixture],
    real_source: Option<&RealSourceSpec>,
) -> PilotResults {
    let fixtures_results: Vec<FixtureResult> = fixtures.iter().map(run_fixture).collect();
    let real_result = real_source.map(|spec| run_real_source(bench_root, spec));

    let pass = fixtures_results.iter().all(|result| result.pass)
        && real_result
            .as_ref()
            .map(|result| result.pass)
            .unwrap_or(true);

    PilotResults {
        schema: PILOT_RESULTS_SCHEMA.to_string(),
        pilot_set_id: PILOT_SET_ID.to_string(),
        profiler_version: PROFILER_VERSION.to_string(),
        lane_registry_version: LANE_REGISTRY_VERSION.to_string(),
        transition_oracle_version: TRANSITION_ORACLE_VERSION.to_string(),
        payload_lifecycle_version: PAYLOAD_LIFECYCLE_VERSION.to_string(),
        fixtures: fixtures_results,
        real_source: real_result,
        deferred: deferred_work(),
        pass,
    }
}

fn run_real_source(bench_root: &Path, spec: &RealSourceSpec) -> RealSourceResult {
    let path = bench_root.join(&spec.path);
    let mut checks = Vec::new();
    let mut profiles = Vec::new();
    let observed = fs::read(&path).ok().map(|bytes| sha256_hex(&bytes));

    let available = observed.is_some();
    // The real-source bytes are PR #34 acquisition material: the pinned
    // snapshot is rebuilt locally and never tracked, so their absence is an
    // abstention, not a semantic failure. A source that IS present always
    // gates, so bytes that contradict the declared identity fail closed.
    checks.push(CheckOutcome {
        check: "real_source_bytes_available".to_string(),
        passed: available,
        gating: available,
        detail: format!(
            "{} ({})",
            spec.path,
            if available {
                "materialized"
            } else {
                "absent: run `python3 workloads/tools/acquire.py materialize`"
            }
        ),
    });
    checks.push(CheckOutcome {
        check: "real_source_sha256".to_string(),
        passed: observed.as_deref() == Some(spec.source_sha256.as_str()),
        gating: available,
        detail: format!(
            "declared={} observed={}",
            spec.source_sha256,
            observed
                .clone()
                .unwrap_or_else(|| "not evaluated: bytes absent".to_string())
        ),
    });

    if let Some(bytes) = fs::read(&path).ok() {
        if let Ok(text) = String::from_utf8(bytes) {
            for lane_id in &spec.lanes {
                if let Some(lane) = lane_for_id(lane_id) {
                    let profile = lane_profile_with(&text, &lane);
                    profiles.push(summarize(&profile));
                }
            }
        } else {
            checks.push(CheckOutcome {
                check: "real_source_utf8".to_string(),
                passed: false,
                gating: true,
                detail: "source is not valid UTF-8".to_string(),
            });
        }
    }

    let pass = checks.iter().all(|check| !check.gating || check.passed);
    RealSourceResult {
        spec: spec.clone(),
        available,
        status: if available {
            REAL_SOURCE_VERIFIED.to_string()
        } else {
            REAL_SOURCE_ABSTAINED.to_string()
        },
        observed_sha256: observed,
        checks,
        profiles,
        pass,
    }
}

/// Load the declared real-source spec for the pilot, when present.
///
/// The spec pins the source path, its sha256, and the PR #34 acquisition
/// commit. The bytes themselves are acquisition material and may legitimately
/// be absent (see `workloads/pilots/README.md` §4).
pub fn real_source_spec(bench_root: &Path) -> Option<RealSourceSpec> {
    let text = fs::read_to_string(bench_root.join("workloads/pilots/real-source-v1.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn deferred_work() -> Vec<String> {
    vec![
        "final 3,970-file profiling".into(),
        "final file selection (representative/extremal/syntax-coverage/full-document sets)".into(),
        "workload freeze (REAL_WORKLOAD_FREEZE_PASS / CORE_REAL_WORKLOAD_FREEZE_PASS)".into(),
        "full semantic-interference metric".into(),
        "render-ready measurement".into(),
        "whole-project performance".into(),
        "long-session / chained performance".into(),
        "H0-H4 timing of any kind".into(),
    ]
}

impl fmt::Display for CheckOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} — {}", self.check, self.status(), self.detail)
    }
}

impl CheckOutcome {
    /// `PASS` / `FAIL`, or `ABSTAIN` for a non-gating check whose evidence was
    /// absent: the observation is recorded without claiming a verdict.
    pub fn status(&self) -> &'static str {
        match (self.passed, self.gating) {
            (true, _) => "PASS",
            (false, true) => "FAIL",
            (false, false) => "ABSTAIN",
        }
    }
}
