//! PAYLOAD-LIFECYCLE-v1 — semantic payload facts, BREAK/RESTORE chains,
//! chained traces, and position dedup.
//!
//! Authority: `workloads/payloads/PAYLOAD-LIFECYCLE-v1.md` (owner) + `#35`
//! Workload Construction Algorithm v1 §11-§16 + `protocol/
//! R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §8.
//!
//! Rules enforced here, not merely documented:
//!
//! - inserted bytes are carried verbatim **and** digested: a digest alone
//!   cannot reconstruct them;
//! - a payload identity is deterministic from its own frozen fields;
//! - RESTORE starts from the frozen broken state, never from the clean
//!   source, and an exactly-inverting RESTORE is verified by hash;
//! - a chained step's coordinates belong to that step's pre-source;
//! - EARLY/MIDDLE/LATE requests that resolve to the same real anchor are
//!   one payload, not three coverage cases.

use serde::{Deserialize, Serialize};

use crate::canonical::{canonical_json, sha256_hex};
use crate::facts::{Span, SyntaxKind};
use crate::transition::{
    validate_transition, EditSpec, PredicateV1, TransitionReport,
};

/// Frozen payload schema tag.
pub const PAYLOAD_SCHEMA: &str = "real-payload-v1";
/// Frozen lifecycle contract version.
pub const PAYLOAD_LIFECYCLE_VERSION: &str = "PAYLOAD-LIFECYCLE-v1";
/// Frozen payload-id namespace prefix. Deliberately distinct from the R1
/// `CaseKeyV1`/`CaseId` namespace: real payloads must not be squeezed into
/// the synthetic identity space (`#35` §7).
pub const PAYLOAD_ID_NAMESPACE: &str = "rp1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestedPosition {
    Early,
    Middle,
    Late,
}

impl RequestedPosition {
    /// Frozen target percentiles (`#35` §5.1).
    pub fn target_fraction(self) -> f64 {
        match self {
            RequestedPosition::Early => 0.10,
            RequestedPosition::Middle => 0.50,
            RequestedPosition::Late => 0.90,
        }
    }

    pub fn all() -> [RequestedPosition; 3] {
        [
            RequestedPosition::Early,
            RequestedPosition::Middle,
            RequestedPosition::Late,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceForm {
    SingleReset,
    LocalBurst,
    DocumentSession,
}

impl TraceForm {
    pub fn name(self) -> &'static str {
        match self {
            TraceForm::SingleReset => "single_reset",
            TraceForm::LocalBurst => "local_burst",
            TraceForm::DocumentSession => "document_session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    RealAcquisition,
    SyntheticFixture,
}

/// A real syntax occurrence usable as an edit anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Anchor {
    pub syntax_kind: SyntaxKind,
    pub span: Span,
    /// Stable occurrence index from the profiler fact list.
    pub occurrence: u32,
}

impl Anchor {
    /// Frozen anchor identity: kind + occurrence + span. Used for the
    /// §11.1 dedup rule and as the payload's positional identity.
    pub fn identity(&self) -> String {
        format!(
            "{}#{}@{}-{}",
            self.syntax_kind.name(),
            self.occurrence,
            self.span.start,
            self.span.end
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TargetDistance {
    pub requested: RequestedPosition,
    pub target_fraction: f64,
    pub distance: f64,
}

/// One resolved position request group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PositionResolution {
    pub anchor: Anchor,
    /// Requested positions that resolved to this anchor (one entry when
    /// no dedup happened).
    pub requested_positions: Vec<RequestedPosition>,
    pub actual_anchor_byte: u64,
    pub actual_relative_position: f64,
    pub distance_from_target: Vec<TargetDistance>,
    pub dedup_identity: String,
    /// True when two or more requested positions collapsed onto this one
    /// real anchor: the payload then covers one physical position, not
    /// three.
    pub deduplicated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PositionResolutionOutcome {
    pub resolutions: Vec<PositionResolution>,
    /// Requests that produced no anchor at all: reported, never filled
    /// with a fabricated nearby construct.
    pub uncovered_requests: Vec<RequestedPosition>,
    /// True when no candidate anchor existed for any request.
    pub not_applicable: bool,
}

/// Frozen nearest-anchor rule: minimize `(|relative − target|, start byte,
/// occurrence)` per request, then group identical anchors.
pub fn resolve_positions(
    anchors: &[Anchor],
    source_len: usize,
    requests: &[RequestedPosition],
) -> PositionResolutionOutcome {
    let mut resolutions: Vec<PositionResolution> = Vec::new();
    let mut uncovered = Vec::new();

    for requested in requests {
        let target = requested.target_fraction();
        let best = anchors
            .iter()
            .map(|anchor| {
                let relative = relative_position(anchor.span.start, source_len);
                let distance = (relative - target).abs();
                (distance, anchor)
            })
            .min_by(|left, right| {
                left.0
                    .partial_cmp(&right.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.1.span.start.cmp(&right.1.span.start))
                    .then_with(|| left.1.occurrence.cmp(&right.1.occurrence))
            });

        let Some((distance, anchor)) = best else {
            uncovered.push(*requested);
            continue;
        };

        let relative = relative_position(anchor.span.start, source_len);
        match resolutions
            .iter_mut()
            .find(|resolution| resolution.anchor.identity() == anchor.identity())
        {
            Some(existing) => {
                existing.requested_positions.push(*requested);
                existing.distance_from_target.push(TargetDistance {
                    requested: *requested,
                    target_fraction: target,
                    distance,
                });
                existing.deduplicated = existing.requested_positions.len() > 1;
            }
            None => resolutions.push(PositionResolution {
                anchor: anchor.clone(),
                requested_positions: vec![*requested],
                actual_anchor_byte: anchor.span.start as u64,
                actual_relative_position: relative,
                distance_from_target: vec![TargetDistance {
                    requested: *requested,
                    target_fraction: target,
                    distance,
                }],
                dedup_identity: anchor.identity(),
                deduplicated: false,
            }),
        }
    }

    PositionResolutionOutcome {
        not_applicable: resolutions.is_empty(),
        resolutions,
        uncovered_requests: uncovered,
    }
}

fn relative_position(offset: usize, source_len: usize) -> f64 {
    if source_len == 0 {
        0.0
    } else {
        offset as f64 / source_len as f64
    }
}

/// Payload position record (present for anchored edit payloads).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PayloadPosition {
    pub requested_positions: Vec<RequestedPosition>,
    pub actual_anchor_byte: u64,
    pub actual_relative_position: f64,
    pub distance_from_target: Vec<TargetDistance>,
    pub dedup_identity: String,
    pub deduplicated: bool,
}

impl PayloadPosition {
    pub fn from_resolution(resolution: &PositionResolution) -> Self {
        Self {
            requested_positions: resolution.requested_positions.clone(),
            actual_anchor_byte: resolution.actual_anchor_byte,
            actual_relative_position: resolution.actual_relative_position,
            distance_from_target: resolution.distance_from_target.clone(),
            dedup_identity: resolution.dedup_identity.clone(),
            deduplicated: resolution.deduplicated,
        }
    }
}

/// A frozen real-workload payload record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PayloadRecord {
    pub schema: String,
    pub lifecycle_version: String,
    pub payload_id: String,
    pub source_id: String,
    pub source_path: String,
    pub source_kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_commit_sha: Option<String>,
    pub grammar_id: String,
    pub base_source_sha256: String,
    pub pre_source_sha256: String,
    pub post_source_sha256: String,
    pub syntax_target: SyntaxKind,
    pub edit_family: String,
    pub operation_variant: String,
    pub expected_transition: String,
    pub expected_pre: Vec<PredicateV1>,
    pub expected_post: Vec<PredicateV1>,
    pub trace_id: String,
    pub trace_form: TraceForm,
    pub step: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<PayloadPosition>,
    pub edit: EditSpec,
    pub inserted_sha256: String,
    pub memberships: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl PayloadRecord {
    /// Deterministic identity: same frozen fields → same id. The id is
    /// derived from the *identity* fields only (never from observed
    /// performance, and never from a timestamp).
    pub fn identity(&self) -> String {
        let material = serde_json::json!({
            "namespace": PAYLOAD_ID_NAMESPACE,
            "source_id": self.source_id,
            "source_path": self.source_path,
            "grammar_id": self.grammar_id,
            "pre_source_sha256": self.pre_source_sha256,
            "syntax_target": self.syntax_target.name(),
            "edit_family": self.edit_family,
            "operation_variant": self.operation_variant,
            "edit_start": self.edit.edit_start,
            "edit_end": self.edit.edit_end,
            "inserted_sha256": self.inserted_sha256,
            // The asserted transition is part of the case identity: two
            // records that differ only in what they claim to prove are not
            // the same payload, and deduplicating them would hide the
            // disagreement behind one id.
            "expected_transition": self.expected_transition,
            "expected_pre": self.expected_pre,
            "expected_post": self.expected_post,
            "trace_id": self.trace_id,
            "trace_form": self.trace_form.name(),
            "step": self.step,
        });
        let digest = sha256_hex(canonical_json(&material).as_bytes());
        format!("{PAYLOAD_ID_NAMESPACE}{}", &digest[..32])
    }

    pub fn transition_request(&self) -> crate::transition::TransitionRequest {
        crate::transition::TransitionRequest {
            grammar_id: self.grammar_id.clone(),
            pre_source: String::new(), // filled by the validator
            post_source: None,
            edit: Some(self.edit.clone()),
            expected_pre: self.expected_pre.clone(),
            expected_post: self.expected_post.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayloadFailureCode {
    PayloadIdMismatch,
    InsertedDigestMismatch,
    PreSourceHashMismatch,
    BaseSourceHashMismatch,
    EmptyTraceId,
    MissingTransitionAssertion,
    PositionOutOfRange,
    EditInvalid,
    /// The declared syntax transition did not hold (see `transition`).
    TransitionInvalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PayloadValidation {
    pub payload_id: String,
    pub valid: bool,
    pub failure_codes: Vec<PayloadFailureCode>,
    pub transition: TransitionReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconstructed_post_sha256: Option<String>,
}

/// Validate a payload against its pre-source bytes: identity determinism,
/// inserted-byte digest, edit validity, hash reproduction and the declared
/// syntax transition (via TRANSITION-ORACLE-v1).
pub fn validate_payload(record: &PayloadRecord, pre_source: &str, base_source: &str) -> PayloadValidation {
    let mut failure_codes = Vec::new();

    if record.payload_id != record.identity() {
        failure_codes.push(PayloadFailureCode::PayloadIdMismatch);
    }
    if record.inserted_sha256 != sha256_hex(record.edit.inserted_text.as_bytes()) {
        failure_codes.push(PayloadFailureCode::InsertedDigestMismatch);
    }
    let pre_sha = sha256_hex(pre_source.as_bytes());
    if record.pre_source_sha256 != pre_sha {
        failure_codes.push(PayloadFailureCode::PreSourceHashMismatch);
    }
    if record.base_source_sha256 != sha256_hex(base_source.as_bytes()) {
        failure_codes.push(PayloadFailureCode::BaseSourceHashMismatch);
    }
    if record.trace_id.is_empty() {
        failure_codes.push(PayloadFailureCode::EmptyTraceId);
    }
    // R6 §4: an active syntax payload must declare what state transition it
    // tests. Hashes alone never prove that the case tests the claimed change.
    if record.expected_pre.is_empty() || record.expected_post.is_empty() {
        failure_codes.push(PayloadFailureCode::MissingTransitionAssertion);
    }
    if let Some(position) = &record.position {
        if position.actual_anchor_byte as usize > pre_source.len() {
            failure_codes.push(PayloadFailureCode::PositionOutOfRange);
        }
    }

    let request = crate::transition::TransitionRequest {
        grammar_id: record.grammar_id.clone(),
        pre_source: pre_source.to_string(),
        post_source: record.edit.apply(pre_source).ok(),
        edit: Some(record.edit.clone()),
        expected_pre: record.expected_pre.clone(),
        expected_post: record.expected_post.clone(),
    };
    let transition = validate_transition(&request);
    if !transition.is_valid() {
        failure_codes.push(PayloadFailureCode::TransitionInvalid);
    }
    if transition.post.is_none() {
        failure_codes.push(PayloadFailureCode::EditInvalid);
    }

    let reconstructed_post_sha256 = transition
        .post
        .as_ref()
        .map(|post| post.source_sha256.clone())
        .filter(|sha| sha == &record.post_source_sha256);
    if reconstructed_post_sha256.is_none() {
        failure_codes.push(PayloadFailureCode::EditInvalid);
    }

    let valid = failure_codes.is_empty();
    PayloadValidation {
        payload_id: record.payload_id.clone(),
        valid,
        failure_codes,
        transition,
        reconstructed_post_sha256,
    }
}

/// Build a payload record with a deterministic id and digests.
#[allow(clippy::too_many_arguments)]
pub fn build_payload(
    source_id: &str,
    source_path: &str,
    source_kind: SourceKind,
    acquisition_commit_sha: Option<String>,
    grammar_id: &str,
    base_source: &str,
    pre_source: &str,
    syntax_target: SyntaxKind,
    edit_family: &str,
    operation_variant: &str,
    expected_transition: &str,
    expected_pre: Vec<PredicateV1>,
    expected_post: Vec<PredicateV1>,
    trace_id: &str,
    trace_form: TraceForm,
    step: u32,
    position: Option<PayloadPosition>,
    edit: EditSpec,
    memberships: Vec<String>,
    notes: Option<String>,
) -> PayloadRecord {
    let post_source = edit.apply(pre_source).unwrap_or_default();
    let mut record = PayloadRecord {
        schema: PAYLOAD_SCHEMA.to_string(),
        lifecycle_version: PAYLOAD_LIFECYCLE_VERSION.to_string(),
        payload_id: String::new(),
        source_id: source_id.to_string(),
        source_path: source_path.to_string(),
        source_kind,
        acquisition_commit_sha,
        grammar_id: grammar_id.to_string(),
        base_source_sha256: sha256_hex(base_source.as_bytes()),
        pre_source_sha256: sha256_hex(pre_source.as_bytes()),
        post_source_sha256: sha256_hex(post_source.as_bytes()),
        syntax_target,
        edit_family: edit_family.to_string(),
        operation_variant: operation_variant.to_string(),
        expected_transition: expected_transition.to_string(),
        expected_pre,
        expected_post,
        trace_id: trace_id.to_string(),
        trace_form,
        step,
        position,
        inserted_sha256: sha256_hex(edit.inserted_text.as_bytes()),
        edit,
        memberships,
        notes,
    };
    record.payload_id = record.identity();
    record
}

// ---------------------------------------------------------------------------
// BREAK / RESTORE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BreakRestoreRequest {
    pub grammar_id: String,
    /// S0
    pub base_source: String,
    /// S0 --BREAK--> S1
    pub break_edit: EditSpec,
    pub break_expected_pre: Vec<PredicateV1>,
    pub break_expected_post: Vec<PredicateV1>,
    /// S1 --RESTORE--> S2 (validated to start from the broken state)
    pub restore_edit: EditSpec,
    pub restore_expected_pre: Vec<PredicateV1>,
    pub restore_expected_post: Vec<PredicateV1>,
    /// When true, S2 must equal S0 byte-for-byte.
    pub expected_restore_exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BreakRestoreFailureCode {
    BreakEditInvalid,
    RestoreEditInvalid,
    /// The RESTORE pre-source is not the frozen broken state.
    RestorePreIsNotBrokenState,
    /// The BREAK edit left the source unchanged: there is no broken state
    /// for the RESTORE leg to return from.
    BreakNoEffect,
    RestoreNotExact,
    BreakTransitionInvalid,
    RestoreTransitionInvalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BreakRestoreReport {
    pub grammar_id: String,
    pub base_source_sha256: String,
    pub s1_source_sha256: String,
    pub s2_source_sha256: String,
    pub break_pre_sha256: String,
    pub break_post_sha256: String,
    pub restore_pre_sha256: String,
    pub restore_post_sha256: String,
    pub restore_exact: bool,
    pub expected_restore_exact: bool,
    pub valid: bool,
    pub failure_codes: Vec<BreakRestoreFailureCode>,
    pub break_transition: TransitionReport,
    pub restore_transition: TransitionReport,
}

/// Validate a BREAK/RESTORE pair with the frozen SHA chain.
pub fn validate_break_restore(request: &BreakRestoreRequest) -> BreakRestoreReport {
    let mut failure_codes = Vec::new();

    let s1 = match request.break_edit.apply(&request.base_source) {
        Ok(source) => Some(source),
        Err(_) => {
            failure_codes.push(BreakRestoreFailureCode::BreakEditInvalid);
            None
        }
    };

    let break_transition = validate_transition(&crate::transition::TransitionRequest {
        grammar_id: request.grammar_id.clone(),
        pre_source: request.base_source.clone(),
        post_source: s1.clone(),
        edit: Some(request.break_edit.clone()),
        expected_pre: request.break_expected_pre.clone(),
        expected_post: request.break_expected_post.clone(),
    });
    if !break_transition.is_valid() {
        failure_codes.push(BreakRestoreFailureCode::BreakTransitionInvalid);
    }

    let Some(s1) = s1 else {
        return BreakRestoreReport {
            grammar_id: request.grammar_id.clone(),
            base_source_sha256: sha256_hex(request.base_source.as_bytes()),
            s1_source_sha256: String::new(),
            s2_source_sha256: String::new(),
            break_pre_sha256: sha256_hex(request.base_source.as_bytes()),
            break_post_sha256: String::new(),
            restore_pre_sha256: String::new(),
            restore_post_sha256: String::new(),
            restore_exact: false,
            expected_restore_exact: request.expected_restore_exact,
            valid: false,
            failure_codes,
            break_transition,
            restore_transition: validate_transition(&crate::transition::TransitionRequest {
                grammar_id: request.grammar_id.clone(),
                pre_source: String::new(),
                post_source: None,
                edit: None,
                expected_pre: request.restore_expected_pre.clone(),
                expected_post: request.restore_expected_post.clone(),
            }),
        };
    };

    // RESTORE must start from the frozen broken state: applying it to the
    // clean base source instead must not silently pass.
    let restore_from_broken = request.restore_edit.apply(&s1);
    let s2 = match &restore_from_broken {
        Ok(source) => Some(source.clone()),
        Err(_) => {
            failure_codes.push(BreakRestoreFailureCode::RestoreEditInvalid);
            None
        }
    };

    if s1 == request.base_source {
        failure_codes.push(BreakRestoreFailureCode::BreakNoEffect);
    }

    if let Ok(restore_from_clean) = request.restore_edit.apply(&request.base_source) {
        if restore_from_clean == s1 {
            // Applying the RESTORE edit to the clean source reproduces the
            // broken state: the pair is not a real BREAK/RESTORE.
            failure_codes.push(BreakRestoreFailureCode::RestorePreIsNotBrokenState);
        }
    }

    let restore_transition = validate_transition(&crate::transition::TransitionRequest {
        grammar_id: request.grammar_id.clone(),
        pre_source: s1.clone(),
        post_source: s2.clone(),
        edit: Some(request.restore_edit.clone()),
        expected_pre: request.restore_expected_pre.clone(),
        expected_post: request.restore_expected_post.clone(),
    });
    if !restore_transition.is_valid() {
        failure_codes.push(BreakRestoreFailureCode::RestoreTransitionInvalid);
    }

    let restore_exact = s2
        .as_deref()
        .map(|s2| s2 == request.base_source)
        .unwrap_or(false);
    if request.expected_restore_exact && !restore_exact {
        failure_codes.push(BreakRestoreFailureCode::RestoreNotExact);
    }

    BreakRestoreReport {
        grammar_id: request.grammar_id.clone(),
        base_source_sha256: sha256_hex(request.base_source.as_bytes()),
        s1_source_sha256: sha256_hex(s1.as_bytes()),
        s2_source_sha256: s2
            .as_deref()
            .map(|s2| sha256_hex(s2.as_bytes()))
            .unwrap_or_default(),
        break_pre_sha256: sha256_hex(request.base_source.as_bytes()),
        break_post_sha256: sha256_hex(s1.as_bytes()),
        restore_pre_sha256: sha256_hex(s1.as_bytes()),
        restore_post_sha256: s2
            .as_deref()
            .map(|s2| sha256_hex(s2.as_bytes()))
            .unwrap_or_default(),
        restore_exact,
        expected_restore_exact: request.expected_restore_exact,
        valid: failure_codes.is_empty(),
        failure_codes,
        break_transition,
        restore_transition,
    }
}

// ---------------------------------------------------------------------------
// Chained traces
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TraceStep {
    /// Zero-based index inside the trace.
    pub step: u32,
    pub edit: EditSpec,
    pub expected_transition: String,
    /// The step's post-source digest; the next step must continue from it.
    pub post_source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TraceRequest {
    pub trace_id: String,
    pub trace_form: TraceForm,
    pub grammar_id: String,
    pub base_source: String,
    pub steps: Vec<TraceStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceFailureCode {
    EmptyTraceId,
    UnknownGrammarLane,
    StepIndexMismatch,
    EditStartAfterEnd,
    EditOutOfRange,
    EditNotCharBoundary,
    StepPostHashMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TraceStepReport {
    pub step: u32,
    /// The step's pre-source digest: coordinates belong to THIS source.
    pub pre_source_sha256: String,
    pub post_source_sha256: String,
    pub expected_transition: String,
    pub applied: bool,
    pub failure_codes: Vec<TraceFailureCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TraceReport {
    pub trace_id: String,
    pub trace_form: TraceForm,
    pub grammar_id: String,
    pub base_source_sha256: String,
    pub final_source_sha256: String,
    pub steps: Vec<TraceStepReport>,
    pub valid: bool,
    pub failure_codes: Vec<TraceFailureCode>,
    /// Two traces reaching identical bytes stay distinct identities.
    pub trace_identity_key: String,
    /// Digest of the declared step sequence. The identity key is a label
    /// (`form:trace_id`); this digest is the content. Two traces can share a
    /// label while declaring different steps, and the report makes that
    /// difference visible instead of hiding it behind the label.
    pub declared_steps_sha256: String,
}

/// Trace identity key: `trace_id` + form.
pub fn trace_identity_key(trace_id: &str, form: TraceForm) -> String {
    format!("{}:{}", form.name(), trace_id)
}

/// Replay a trace from its base source, validating every step against its
/// own pre-source (bounds, char boundaries, and the hash chain).
pub fn validate_trace(request: &TraceRequest) -> TraceReport {
    let mut failure_codes: Vec<TraceFailureCode> = Vec::new();
    if request.trace_id.is_empty() {
        failure_codes.push(TraceFailureCode::EmptyTraceId);
    }
    // A trace under an unregistered lane is not replayable evidence, exactly
    // as an unknown lane invalidates a transition request.
    if crate::lanes::lane_spec(&request.grammar_id).is_none() {
        failure_codes.push(TraceFailureCode::UnknownGrammarLane);
    }
    let declared_steps_sha256 =
        sha256_hex(canonical_json(&request.steps).as_bytes());

    let mut source = request.base_source.clone();
    let mut step_reports = Vec::new();

    for (expected_step, step) in request.steps.iter().enumerate() {
        let mut step_failures = Vec::new();
        if step.step as usize != expected_step {
            step_failures.push(TraceFailureCode::StepIndexMismatch);
        }
        let pre_sha = sha256_hex(source.as_bytes());
        let applied = match step.edit.apply(&source) {
            Ok(post) => {
                let post_sha = sha256_hex(post.as_bytes());
                if post_sha != step.post_source_sha256 {
                    step_failures.push(TraceFailureCode::StepPostHashMismatch);
                }
                source = post;
                true
            }
            Err(error) => {
                step_failures.push(match error {
                    markit_mdbench_common::EditError::StartAfterEnd => {
                        TraceFailureCode::EditStartAfterEnd
                    }
                    markit_mdbench_common::EditError::OutOfRange => {
                        TraceFailureCode::EditOutOfRange
                    }
                    markit_mdbench_common::EditError::NotCharBoundary => {
                        TraceFailureCode::EditNotCharBoundary
                    }
                });
                false
            }
        };
        failure_codes.extend(step_failures.iter().copied());
        step_reports.push(TraceStepReport {
            step: step.step,
            pre_source_sha256: pre_sha,
            post_source_sha256: sha256_hex(source.as_bytes()),
            expected_transition: step.expected_transition.clone(),
            applied,
            failure_codes: step_failures,
        });
    }

    TraceReport {
        trace_id: request.trace_id.clone(),
        trace_form: request.trace_form,
        grammar_id: request.grammar_id.clone(),
        base_source_sha256: sha256_hex(request.base_source.as_bytes()),
        final_source_sha256: sha256_hex(source.as_bytes()),
        steps: step_reports,
        valid: failure_codes.is_empty(),
        failure_codes,
        trace_identity_key: trace_identity_key(&request.trace_id, request.trace_form),
        declared_steps_sha256,
    }
}
