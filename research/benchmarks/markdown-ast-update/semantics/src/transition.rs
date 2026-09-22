//! TRANSITION-ORACLE-v1 — pre/post syntax-transition validation.
//!
//! Authority: `grammar/TRANSITION-ORACLE-v1.md` (owner) + `#35`
//! Corrective-1 §5 + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §4.
//!
//! Hash replayability is necessary but insufficient. For every active
//! syntax payload this module proves, under the declared grammar lane:
//!
//! ```text
//! pre source  -> reference parse -> expected_pre  must hold
//! apply edit  -> post source     -> expected_post must hold
//! ```
//!
//! plus independent grammar eligibility for the pre source and the post
//! source. A case whose edit exposes host syntax outside the lane becomes
//! post-ineligible: that is recorded, never silently benchmarked, and it is
//! reported separately from the transition verdict.

use serde::{Deserialize, Serialize};

use markit_mdbench_common::{CanonicalEdit, EditError, OperationKind, Source, SourceId};

use crate::canonical::sha256_hex;
use crate::facts::{EligibilityFacts, RecognitionStatus, SyntaxKind};
use crate::lanes::{lane_spec, LaneSpec};
use crate::parse::{LaneNode, LaneParse};
use crate::profile::{lane_profile_with, LaneProfile};

/// Frozen oracle version string.
pub const TRANSITION_ORACLE_VERSION: &str = "TRANSITION-ORACLE-v1";
/// Frozen predicate vocabulary version string.
pub const PREDICATE_VERSION: &str = "PREDICATE-v1";

/// The closed predicate vocabulary (CORRECTIVE-A keeps it minimal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "predicate", rename_all = "snake_case")]
pub enum PredicateV1 {
    /// A recognized node of this kind exists in the lane parse.
    KindPresent {
        grammar_id: String,
        syntax_kind: SyntaxKind,
    },
    /// No recognized node of this kind exists in the lane parse.
    KindAbsent {
        grammar_id: String,
        syntax_kind: SyntaxKind,
    },
    /// Recognized-node count for this kind satisfies `op count`.
    KindCount {
        grammar_id: String,
        syntax_kind: SyntaxKind,
        op: CmpOp,
        count: u64,
    },
    /// Cross-source predicate: the lane topology (kind sequence, nesting
    /// and kind details, ignoring offsets) is identical for pre and post.
    /// Evaluated only on the post side.
    TopologyEqual,
    /// Cross-source predicate: at least one text/code span differs between
    /// pre and post. Evaluated only on the post side.
    TextContentDiffers,
}

impl PredicateV1 {
    /// The `grammar_id` a predicate is bound to, when it names one.
    pub fn bound_grammar_id(&self) -> Option<&str> {
        match self {
            PredicateV1::KindPresent { grammar_id, .. }
            | PredicateV1::KindAbsent { grammar_id, .. }
            | PredicateV1::KindCount { grammar_id, .. } => Some(grammar_id),
            PredicateV1::TopologyEqual | PredicateV1::TextContentDiffers => None,
        }
    }

    pub fn is_kind_predicate(&self) -> bool {
        self.bound_grammar_id().is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    pub fn eval(self, left: u64, right: u64) -> bool {
        match self {
            CmpOp::Eq => left == right,
            CmpOp::Ne => left != right,
            CmpOp::Lt => left < right,
            CmpOp::Le => left <= right,
            CmpOp::Gt => left > right,
            CmpOp::Ge => left >= right,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        }
    }
}

/// One canonical edit as it appears in a transition request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EditSpec {
    pub edit_start: u64,
    pub edit_end: u64,
    /// The inserted bytes are carried verbatim: a digest alone cannot
    /// reconstruct them (`#35` §8, amendment §8).
    pub inserted_text: String,
}

impl EditSpec {
    pub fn to_canonical(&self) -> Result<CanonicalEdit, EditError> {
        CanonicalEdit::new(
            self.edit_start as usize,
            self.edit_end as usize,
            self.inserted_text.clone(),
        )
    }

    pub fn operation_kind(&self) -> OperationKind {
        match self.to_canonical() {
            Ok(edit) => OperationKind::classify(&edit),
            Err(_) => OperationKind::StructuralEdit,
        }
    }

    /// Apply this edit to `pre_source`, returning the post source.
    pub fn apply(&self, pre_source: &str) -> Result<String, EditError> {
        let edit = self.to_canonical()?;
        let source = Source::new(SourceId(0), pre_source.to_string());
        let post = edit.apply(&source, SourceId(1))?;
        Ok(post.as_str().to_string())
    }
}

/// A transition-validation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TransitionRequest {
    pub grammar_id: String,
    pub pre_source: String,
    /// Absent for a pre-only eligibility/predicate check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_source: Option<String>,
    /// When present, the post source must be exactly what applying this
    /// edit to the pre source produces (checked byte-for-byte).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<EditSpec>,
    pub expected_pre: Vec<PredicateV1>,
    pub expected_post: Vec<PredicateV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransitionVerdict {
    TransitionValid,
    InvalidPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    UnknownGrammarLane,
    PredicateGrammarMismatch,
    MissingPostSource,
    EditStartAfterEnd,
    EditOutOfRange,
    EditNotCharBoundary,
    PostHashMismatch,
    PrePredicateFailed,
    PostPredicateFailed,
    PreLaneInvalid,
    PostLaneInvalid,
    /// The edit reproduces the pre bytes exactly: there is no transition.
    NoSourceChange,
    /// The declared assertion set holds identically on both sides and
    /// contains no satisfied cross-source predicate, so it describes
    /// nothing the edit did.
    TransitionNotExercised,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PredicateResult {
    pub predicate: PredicateV1,
    pub satisfied: bool,
    /// What was actually observed (predicate-specific rendering).
    pub observed: String,
}

/// Per-source oracle report: eligibility facts plus predicate outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SourceOracleCheck {
    pub grammar_id: String,
    pub source_sha256: String,
    pub file_bytes: u64,
    pub eligibility: EligibilityFacts,
    pub predicates: Vec<PredicateResult>,
    pub topology_sha256: String,
    pub text_fingerprint_sha256: String,
}

/// The change classification vocabulary (CORRECTIVE-A §G). Only
/// `ContentChange` and `StructureChange` are derivable in this corrective;
/// the other two are reserved for the later attribution contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeClass {
    ContentChange,
    StructureChange,
    /// RESERVED — not derivable in CORRECTIVE-A.
    DependencyResolutionChange,
    /// RESERVED — not derivable in CORRECTIVE-A.
    PositionShiftOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TransitionReport {
    pub oracle_version: String,
    pub predicate_version: String,
    pub grammar_id: String,
    pub verdict: TransitionVerdict,
    pub failure_codes: Vec<FailureCode>,
    pub pre: SourceOracleCheck,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<SourceOracleCheck>,
    /// True only when both sources are lane-valid AND strictly inside the
    /// lane's construct scope.
    pub strict_comparison_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_class: Option<ChangeClass>,
    /// What `change_class` is and is not.
    pub change_class_note: String,
}

impl TransitionReport {
    pub fn is_valid(&self) -> bool {
        self.verdict == TransitionVerdict::TransitionValid
    }
}

/// Validate one transition end to end.
pub fn validate_transition(request: &TransitionRequest) -> TransitionReport {
    let mut failure_codes: Vec<FailureCode> = Vec::new();

    let Some(lane) = lane_spec(&request.grammar_id) else {
        let pre = SourceOracleCheck {
            grammar_id: request.grammar_id.clone(),
            source_sha256: sha256_hex(request.pre_source.as_bytes()),
            file_bytes: request.pre_source.len() as u64,
            eligibility: EligibilityFacts {
                grammar_id: request.grammar_id.clone(),
                lane_valid: false,
                strict_scope_clean: false,
                scope_blockers: Vec::new(),
                strict_scope_clean_rule: "unknown lane".to_string(),
            },
            predicates: Vec::new(),
            topology_sha256: sha256_hex(b""),
            text_fingerprint_sha256: sha256_hex(b""),
        };
        return TransitionReport {
            oracle_version: TRANSITION_ORACLE_VERSION.to_string(),
            predicate_version: PREDICATE_VERSION.to_string(),
            grammar_id: request.grammar_id.clone(),
            verdict: TransitionVerdict::InvalidPayload,
            failure_codes: vec![FailureCode::UnknownGrammarLane],
            pre,
            post: None,
            strict_comparison_eligible: false,
            change_class: None,
            change_class_note: change_class_note(),
        };
    };

    // A predicate bound to another lane would be a silent cross-lane claim.
    for predicate in request
        .expected_pre
        .iter()
        .chain(request.expected_post.iter())
    {
        if let Some(grammar_id) = predicate.bound_grammar_id() {
            if grammar_id != lane.grammar_id {
                failure_codes.push(FailureCode::PredicateGrammarMismatch);
            }
        }
    }

    // ---- pre source -----------------------------------------------------
    let pre_parse = parse_lane(&request.pre_source, &lane);
    let pre_profile = lane_profile_with(&request.pre_source, &lane);
    let pre_predicates = evaluate_predicates(
        &request.expected_pre,
        &pre_profile,
        &pre_parse,
        &request.pre_source,
        None,
        &mut failure_codes,
        true,
    );
    if !pre_profile.eligibility.lane_valid {
        failure_codes.push(FailureCode::PreLaneInvalid);
    }
    let pre = SourceOracleCheck {
        grammar_id: lane.grammar_id.clone(),
        source_sha256: sha256_hex(request.pre_source.as_bytes()),
        file_bytes: request.pre_source.len() as u64,
        eligibility: pre_profile.eligibility.clone(),
        predicates: pre_predicates,
        topology_sha256: sha256_hex(topology(&pre_parse).as_bytes()),
        text_fingerprint_sha256: sha256_hex(
            text_fingerprint(&pre_parse, &request.pre_source).as_bytes(),
        ),
    };

    // ---- post source ----------------------------------------------------
    let Some(post_source) = request.post_source.as_deref() else {
        if request.edit.is_some() {
            failure_codes.push(FailureCode::MissingPostSource);
        }
        let verdict = verdict_of(&failure_codes);
        return TransitionReport {
            oracle_version: TRANSITION_ORACLE_VERSION.to_string(),
            predicate_version: PREDICATE_VERSION.to_string(),
            grammar_id: lane.grammar_id.clone(),
            verdict,
            failure_codes,
            pre,
            post: None,
            strict_comparison_eligible: false,
            change_class: None,
            change_class_note: change_class_note(),
        };
    };

    // The declared edit must reproduce the post source byte-exactly.
    if let Some(spec) = &request.edit {
        match spec.apply(&request.pre_source) {
            Ok(derived) => {
                if sha256_hex(derived.as_bytes()) != sha256_hex(post_source.as_bytes()) {
                    failure_codes.push(FailureCode::PostHashMismatch);
                }
            }
            Err(error) => failure_codes.push(match error {
                EditError::StartAfterEnd => FailureCode::EditStartAfterEnd,
                EditError::OutOfRange => FailureCode::EditOutOfRange,
                EditError::NotCharBoundary => FailureCode::EditNotCharBoundary,
            }),
        }
    }

    let post_parse = parse_lane(post_source, &lane);
    let post_profile = lane_profile_with(post_source, &lane);
    let post_predicates = evaluate_predicates(
        &request.expected_post,
        &post_profile,
        &post_parse,
        post_source,
        Some((&pre_parse, &request.pre_source)),
        &mut failure_codes,
        false,
    );
    if !post_profile.eligibility.lane_valid {
        failure_codes.push(FailureCode::PostLaneInvalid);
    }

    let post = SourceOracleCheck {
        grammar_id: lane.grammar_id.clone(),
        source_sha256: sha256_hex(post_source.as_bytes()),
        file_bytes: post_source.len() as u64,
        eligibility: post_profile.eligibility.clone(),
        predicates: post_predicates,
        topology_sha256: sha256_hex(topology(&post_parse).as_bytes()),
        text_fingerprint_sha256: sha256_hex(text_fingerprint(&post_parse, post_source).as_bytes()),
    };

    let strict_comparison_eligible = pre.eligibility.lane_valid
        && post.eligibility.lane_valid
        && pre.eligibility.strict_scope_clean
        && post.eligibility.strict_scope_clean;

    // An edit that reproduces the pre bytes exactly is not a transition, no
    // matter what it declares.
    if post_source == request.pre_source {
        failure_codes.push(FailureCode::NoSourceChange);
    }

    // The declared assertion set must be exercised by the edit. A set that
    // holds identically on both sides and contains no satisfied cross-source
    // predicate describes nothing the edit did, so the case would pass on
    // hashes and a label alone — exactly the hole the transition oracle
    // exists to close. Non-emptiness (`MissingTransitionAssertion`) is
    // necessary but not sufficient.
    if !assertions_are_exercised(
        request,
        &pre_profile,
        &pre_parse,
        &post_profile,
        &post_parse,
    ) {
        failure_codes.push(FailureCode::TransitionNotExercised);
    }

    let change_class = classify_change(&pre, &post);
    let verdict = verdict_of(&failure_codes);

    TransitionReport {
        oracle_version: TRANSITION_ORACLE_VERSION.to_string(),
        predicate_version: PREDICATE_VERSION.to_string(),
        grammar_id: lane.grammar_id.clone(),
        verdict,
        failure_codes,
        pre,
        post: Some(post),
        strict_comparison_eligible,
        change_class,
        change_class_note: change_class_note(),
    }
}

fn verdict_of(failure_codes: &[FailureCode]) -> TransitionVerdict {
    if failure_codes.is_empty() {
        TransitionVerdict::TransitionValid
    } else {
        TransitionVerdict::InvalidPayload
    }
}

/// Whether the declared assertion set distinguishes the pre state from the
/// post state.
///
/// Two independent ways to be exercised:
///
/// ```text
/// a source-local (kind) predicate whose truth value differs between sides
/// a cross-source predicate (topology_equal, text_content_differs) that is
///   satisfied on the post side
/// ```
///
/// A cross-source predicate is undefined on a pre source, so it cannot
/// "differ"; it counts by being satisfied, which is what makes a
/// content-only control (`p03`) a legitimate case rather than a vacuous one.
fn assertions_are_exercised(
    request: &TransitionRequest,
    pre_profile: &LaneProfile,
    pre_parse: &LaneParse,
    post_profile: &LaneProfile,
    post_parse: &LaneParse,
) -> bool {
    let union: Vec<PredicateV1> = request
        .expected_pre
        .iter()
        .chain(request.expected_post.iter())
        .cloned()
        .collect();
    if union.is_empty() {
        return false;
    }
    let mut scratch = Vec::new();
    let pre_results = evaluate_predicates(
        &union,
        pre_profile,
        pre_parse,
        &request.pre_source,
        None,
        &mut scratch,
        true,
    );
    let post_results = evaluate_predicates(
        &union,
        post_profile,
        post_parse,
        request.post_source.as_deref().unwrap_or_default(),
        Some((pre_parse, &request.pre_source)),
        &mut scratch,
        false,
    );
    let flipped = pre_results
        .iter()
        .zip(post_results.iter())
        .any(|(pre, post)| pre.predicate.is_kind_predicate() && pre.satisfied != post.satisfied);
    let cross_source_holds = request
        .expected_post
        .iter()
        .zip(post_results.iter())
        .any(|(predicate, result)| !predicate.is_kind_predicate() && result.satisfied);
    flipped || cross_source_holds
}

fn change_class_note() -> String {
    "Observed effect under the frozen lane rules and the PREDICATE-v1 vocabulary. This is NOT \
     the theoretical minimal work an incremental parser must perform, and it must not be \
     confused with semantic interference (a later attribution contract, CORRECTIVE-A §H) or \
     with any horse work counter."
        .to_string()
}

fn parse_lane(source: &str, lane: &LaneSpec) -> LaneParse {
    match lane.lane_id.as_str() {
        "G1" => crate::g1::parse_g1(source),
        _ => crate::g0::parse_g0(source),
    }
}

fn evaluate_predicates(
    predicates: &[PredicateV1],
    profile: &LaneProfile,
    parse: &LaneParse,
    source: &str,
    previous: Option<(&LaneParse, &str)>,
    failure_codes: &mut Vec<FailureCode>,
    is_pre: bool,
) -> Vec<PredicateResult> {
    let mut results = Vec::new();
    for predicate in predicates {
        let (satisfied, observed) = match predicate {
            PredicateV1::KindPresent { syntax_kind, .. } => {
                let count = recognized_count(parse, profile, *syntax_kind);
                (count > 0, format!("recognized_count={count}"))
            }
            PredicateV1::KindAbsent { syntax_kind, .. } => {
                let count = recognized_count(parse, profile, *syntax_kind);
                (count == 0, format!("recognized_count={count}"))
            }
            PredicateV1::KindCount {
                syntax_kind,
                op,
                count,
                ..
            } => {
                let observed_count = recognized_count(parse, profile, *syntax_kind);
                (
                    op.eval(observed_count, *count),
                    format!(
                        "recognized_count={observed_count} expected {} {count}",
                        op.symbol()
                    ),
                )
            }
            PredicateV1::TopologyEqual => match previous {
                Some((previous_parse, _)) => {
                    let equal = topology(previous_parse) == topology(parse);
                    (equal, format!("topology_equal={equal}"))
                }
                None => (false, "not evaluable on a pre source".to_string()),
            },
            PredicateV1::TextContentDiffers => match previous {
                Some((previous_parse, previous_source)) => {
                    let differs = text_fingerprint(previous_parse, previous_source)
                        != text_fingerprint(parse, source);
                    (differs, format!("text_content_differs={differs}"))
                }
                None => (false, "not evaluable on a pre source".to_string()),
            },
        };
        if !satisfied {
            failure_codes.push(if is_pre {
                FailureCode::PrePredicateFailed
            } else {
                FailureCode::PostPredicateFailed
            });
        }
        results.push(PredicateResult {
            predicate: predicate.clone(),
            satisfied,
            observed,
        });
    }
    results
}

/// Recognized occurrences of `kind` under this lane: parse nodes plus the
/// lane's out-of-band recognized facts.
///
/// G1 exposes reference definitions outside the node tree, so a count that
/// walked nodes only would answer `kind_absent` for a kind the same profile
/// reports as `recognized` — the one inference
/// `workloads/profiles/PROFILER-CONTRACT-v1.md` §2.1 forbids. A fact that a
/// node already covers is not counted twice.
fn recognized_count(parse: &LaneParse, profile: &LaneProfile, kind: SyntaxKind) -> u64 {
    let nodes: Vec<&LaneNode> = parse
        .nodes()
        .into_iter()
        .filter(|n| n.kind == kind)
        .collect();
    let extra = profile
        .syntax_facts
        .iter()
        .filter(|fact| {
            fact.syntax_kind == kind
                && fact.recognition_status == RecognitionStatus::Recognized
                && !nodes.iter().any(|node| node.span.intersects(&fact.span()))
        })
        .count() as u64;
    nodes.len() as u64 + extra
}

/// Position-independent topology: kinds, nesting and kind details only.
pub fn topology(parse: &LaneParse) -> String {
    let mut out = String::new();
    for child in &parse.root.children {
        write_topology(child, &mut out);
    }
    out
}

fn write_topology(node: &LaneNode, out: &mut String) {
    out.push('(');
    out.push_str(node.kind.name());
    if let Some(detail) = &node.detail {
        out.push('{');
        out.push_str(detail);
        out.push('}');
    }
    for child in &node.children {
        write_topology(child, out);
    }
    out.push(')');
}

/// Ordered source slice of every text/code span: the frozen content
/// fingerprint used by the content-only control.
pub fn text_fingerprint(parse: &LaneParse, source: &str) -> String {
    let mut out = String::new();
    for node in parse.nodes() {
        if matches!(
            node.kind,
            SyntaxKind::Text | SyntaxKind::CodeSpan | SyntaxKind::RawHtmlInline
        ) {
            out.push_str(&source[node.span.start..node.span.end]);
            out.push('\u{1f}');
        }
    }
    out
}

fn classify_change(pre: &SourceOracleCheck, post: &SourceOracleCheck) -> Option<ChangeClass> {
    if pre.topology_sha256 != post.topology_sha256 || kind_predicates_differ(pre, post) {
        return Some(ChangeClass::StructureChange);
    }
    if pre.text_fingerprint_sha256 != post.text_fingerprint_sha256 {
        return Some(ChangeClass::ContentChange);
    }
    None
}

fn kind_predicates_differ(pre: &SourceOracleCheck, post: &SourceOracleCheck) -> bool {
    // A declared kind predicate whose truth value flipped between the two
    // sides is structural evidence even when the topology rendering of
    // both sides happens to be identical.
    for pre_result in &pre.predicates {
        if !pre_result.predicate.is_kind_predicate() {
            continue;
        }
        if let Some(post_result) = post
            .predicates
            .iter()
            .find(|result| result.predicate == pre_result.predicate)
        {
            if pre_result.satisfied != post_result.satisfied {
                return true;
            }
        }
    }
    false
}
