//! A6 — the complete real-anchor applicability matrix over the frozen
//! CORRECTIVE-B selection (task §30-§36) plus A7 payload assembly.
//!
//! For every (physical selected file × registry transition) cell the
//! matrix resolves to an explicit status — no silent missing entries.
//! Resolution per requested position (EARLY ≈ 10%, MIDDLE ≈ 50%,
//! LATE ≈ 90%) walks the real profiler-recognized anchors in the frozen
//! deterministic order
//!
//! ```text
//! 1. minimum |relative − target| distance
//! 2. lower byte offset
//! 3. stable occurrence index
//! ```
//!
//! until one anchor produces a registry-valid, oracle-proven transition;
//! every rejection on the way is recorded (task §32). Positions that
//! resolve to the same physical anchor are ONE payload (task §33).
//! RESTORE legs start from the frozen broken state S1, never from S0,
//! and exact restoration is verified by the frozen SHA chain (task §36).

use markit_mdbench_semantics::payload::{
    build_payload, validate_break_restore, validate_payload, BreakRestoreRequest, PayloadPosition,
    PayloadRecord, RequestedPosition, SourceKind, TraceForm,
};
use markit_mdbench_semantics::transition::{
    validate_transition, ChangeClass, PredicateV1, TransitionReport, TransitionRequest,
};
use markit_mdbench_semantics::{
    lane_profile_with, lane_spec, sha256_hex, Anchor, LaneParse, LaneProfile, SyntaxFact,
    SyntaxKind, G0_GRAMMAR_ID,
};

use crate::editors::{self, ConstructedEdit, Derivation};
use crate::registry::{
    payload_agrees_with_registry, registry_entry, transition_registry_v1, RegistryQualification,
    RestorePolicy, TransitionEntry,
};
use crate::{SelectedFile, MEMBERSHIP_G0_PRIMARY, MEMBERSHIP_G1_TABLE_SEMANTIC};

/// Closed applicability status vocabulary (task §30 subset; every status
/// used by this generator is declared here).
pub const STATUS_APPLICABLE: &str = "APPLICABLE";
pub const STATUS_NO_TARGET_SYNTAX: &str = "NO_TARGET_SYNTAX";
pub const STATUS_NO_VALID_REAL_ANCHOR: &str = "NO_VALID_REAL_ANCHOR";
pub const STATUS_GRAMMAR_INELIGIBLE: &str = "GRAMMAR_INELIGIBLE";
pub const STATUS_POST_GRAMMAR_INELIGIBLE: &str = "POST_GRAMMAR_INELIGIBLE";
pub const STATUS_TRANSITION_NOT_PRODUCED: &str = "NO_PROVABLE_TRANSITION";

/// `SyntaxKind` from its frozen snake_case name (registry tables store
/// names; this is the inverse of `SyntaxKind::name`).
pub fn kind_from_name(name: &str) -> Option<SyntaxKind> {
    use SyntaxKind::*;
    let kind = match name {
        "document" => Document,
        "paragraph" => Paragraph,
        "heading_atx" => HeadingAtx,
        "heading_setext" => HeadingSetext,
        "block_quote" => BlockQuote,
        "list" => List,
        "list_item" => ListItem,
        "code_block_fenced" => CodeBlockFenced,
        "code_block_indented" => CodeBlockIndented,
        "html_block" => HtmlBlock,
        "thematic_break" => ThematicBreak,
        "reference_definition" => ReferenceDefinition,
        "table" => Table,
        "table_header_row" => TableHeaderRow,
        "table_row" => TableRow,
        "table_cell" => TableCell,
        "text" => Text,
        "emphasis" => Emphasis,
        "strong" => Strong,
        "code_span" => CodeSpan,
        "link_inline" => LinkInline,
        "link_reference" => LinkReference,
        "link_autolink" => LinkAutolink,
        "image" => Image,
        "raw_html_inline" => RawHtmlInline,
        "hard_break" => HardBreak,
        "soft_break" => SoftBreak,
        "inline_math" => InlineMath,
        "display_math" => DisplayMath,
        "strikethrough" => Strikethrough,
        "task_list_item" => TaskListItem,
        "front_matter" => FrontMatter,
        "directive" => Directive,
        _ => return None,
    };
    debug_assert_eq!(kind.name(), name);
    Some(kind)
}

/// One recorded anchor rejection (task §32: record every rejection).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RejectionRecord {
    pub anchor_identity: String,
    pub reason: String,
}

/// Per-position resolution outcome.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PositionOutcome {
    pub requested: String,
    pub resolved_anchor: Option<String>,
    /// Anchors tried and rejected before the resolution (frozen order).
    pub fallback_rejections: Vec<RejectionRecord>,
    pub deduplicated: bool,
}

/// One resolved leg (BREAK side) with everything needed for payloads.
#[derive(Debug, Clone)]
pub struct Resolution {
    pub anchor: Anchor,
    pub context: String,
    pub requested_positions: Vec<RequestedPosition>,
    pub constructed: ConstructedEdit,
    pub derivation: Derivation,
    pub pre_predicates: Vec<PredicateV1>,
    pub post_predicates: Vec<PredicateV1>,
    pub restore: Option<RestoreLeg>,
}

#[derive(Debug, Clone)]
pub struct RestoreLeg {
    pub constructed: ConstructedEdit,
    pub pre_predicates: Vec<PredicateV1>,
    pub post_predicates: Vec<PredicateV1>,
    pub exact: bool,
}

/// One applicability-matrix row (JSONL artifact record).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MatrixRow {
    pub source_key: String,
    pub transition_id: String,
    pub grammar_id: String,
    pub lane_id: String,
    pub syntax_target: String,
    pub status: String,
    pub candidate_anchor_count: u64,
    pub rejections: Vec<RejectionRecord>,
    pub position_outcomes: Vec<PositionOutcome>,
    pub resolved_anchor_count: u64,
    pub payload_ids: Vec<String>,
    pub contexts_observed: Vec<String>,
    /// Oracle evidence of the first resolution (transition truth).
    pub evidence: Option<EvidenceRecord>,
}

/// Machine-proven transition evidence attached to a matrix row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceRecord {
    pub anchor_identity: String,
    pub strict_comparison_eligible: bool,
    pub change_class: Option<String>,
    pub pre_topology_sha256: String,
    pub post_topology_sha256: String,
    pub oracle_verdict: String,
    pub failure_codes: Vec<String>,
}

/// Observed host-context syntax of one file under one lane, for the
/// final coverage accounting (recognized facts and non-recognized
/// candidate facts, separately).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObservedSyntax {
    pub source_key: String,
    pub grammar_id: String,
    /// Recognized host-context facts per kind name.
    pub recognized: std::collections::BTreeMap<String, u64>,
    /// Non-recognized host-context candidate facts per `kind:status`.
    pub candidates: std::collections::BTreeMap<String, u64>,
}

/// Everything A6/A7 produce for the frozen workload.
#[derive(Debug, Clone)]
pub struct FrozenWorkload {
    pub rows: Vec<MatrixRow>,
    pub payloads: Vec<PayloadRecord>,
    pub break_restore_reports: Vec<markit_mdbench_semantics::payload::BreakRestoreReport>,
    pub observed: Vec<ObservedSyntax>,
}

/// Prebuilt per-(file, grammar) lane profiles shared by the workload
/// build, the FULL_READ manifest and the coverage accounting.
pub type ProfileMap = std::collections::BTreeMap<(String, String), LaneProfile>;

/// Profile every selected file under both comparison lanes. G2 is never
/// profiled: it has no oracle and no semantics (deferred lane).
pub fn build_profile_map(files: &[SelectedFile]) -> ProfileMap {
    let mut map = ProfileMap::new();
    for file in files {
        for grammar_id in [crate::G0_GRAMMAR_ID, crate::G1_GRAMMAR_ID] {
            let lane = lane_spec(grammar_id).expect("registered lane");
            let profile = lane_profile_with(&file.text, &lane);
            map.insert((file.key.clone(), grammar_id.to_string()), profile);
        }
    }
    map
}

/// Per-lane parse + profile for one file (computed lazily, once).
struct FileLaneContext {
    profile: LaneProfile,
    parse: LaneParse,
}

impl FileLaneContext {
    fn new(source: &str, grammar_id: &str) -> Self {
        let lane = lane_spec(grammar_id).expect("registered lane");
        let profile = lane_profile_with(source, &lane);
        let parse = editors::parse_lane(source, lane.lane_id.as_str());
        Self { profile, parse }
    }

    /// Context over a prebuilt profile (shared profile map); the lane
    /// parse is still local to this context.
    fn from_profile(source: &str, grammar_id: &str, profile: LaneProfile) -> Self {
        let lane = lane_spec(grammar_id).expect("registered lane");
        let parse = editors::parse_lane(source, lane.lane_id.as_str());
        Self { profile, parse }
    }

    fn facts_for(&self, kind: SyntaxKind) -> Vec<Anchor> {
        self.profile
            .syntax_facts
            .iter()
            .filter(|fact| {
                fact.syntax_kind == kind
                    && fact.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized
                    && fact.host_context
                    && fact.is_strict_coverage()
            })
            .map(|fact| Anchor {
                syntax_kind: fact.syntax_kind,
                span: fact.span(),
                occurrence: fact.occurrence,
            })
            .collect()
    }
}

fn anchor_identity(anchor: &Anchor) -> String {
    anchor.identity()
}

/// Emphasis/code-span context refinement (task §17): inside link text vs
/// ordinary/container text, recorded without exploding the matrix.
fn inline_context(anchor: &Anchor, profile: &LaneProfile) -> String {
    let inside_link = profile.syntax_facts.iter().any(|other| {
        matches!(other.syntax_kind, SyntaxKind::LinkInline | SyntaxKind::LinkReference)
            && other.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized
            && other.span().contains(&anchor.span)
    });
    let inside_container = profile.syntax_facts.iter().any(|other| {
        matches!(other.syntax_kind, SyntaxKind::List | SyntaxKind::BlockQuote)
            && other.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized
            && other.span().contains(&anchor.span)
    });
    if inside_link {
        "inside_link_text".to_string()
    } else if inside_container {
        "inside_container_text".to_string()
    } else {
        "plain_paragraph".to_string()
    }
}

/// Fence context (task §16): top-level vs inside-container fence.
fn fence_context(anchor: &Anchor, profile: &LaneProfile) -> String {
    let inside_container = profile.syntax_facts.iter().any(|other| {
        matches!(other.syntax_kind, SyntaxKind::List | SyntaxKind::BlockQuote)
            && other.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized
            && other.span().contains(&anchor.span)
    });
    if inside_container {
        "container_fence".to_string()
    } else {
        "top_level_fence".to_string()
    }
}

/// Table context labels (task §21), computed mechanically from the
/// table's own bytes. Never manufactures a table: labels describe what
/// the real table contains.
fn table_contexts(source: &str, table_span: markit_mdbench_semantics::Span) -> Vec<String> {
    let mut contexts = vec!["ordinary_table".to_string()];
    let text = &source[table_span.start..table_span.end];
    let line_count = text.lines().count();
    let first_line = text.lines().next().unwrap_or_default();
    let header_pipes = first_line.matches('|').count().saturating_sub(1);
    if header_pipes >= 4 {
        contexts.push("wide_table".to_string());
    }
    if line_count >= 5 {
        contexts.push("long_table".to_string());
    }
    if text.lines().nth(1).unwrap_or_default().contains(':') {
        contexts.push("alignment_markers".to_string());
    }
    if text.contains("\\|") {
        contexts.push("escaped_pipe".to_string());
    }
    if text.contains('*') || text.contains('`') || text.contains('[') {
        contexts.push("inline_syntax_in_cell".to_string());
    }
    contexts.sort();
    contexts.dedup();
    contexts
}

/// The heavy, cacheable part of one anchor attempt.
struct CachedLeg {
    context: String,
    constructed: ConstructedEdit,
    derivation: Derivation,
    pre_predicates: Vec<PredicateV1>,
    post_predicates: Vec<PredicateV1>,
    restore: Option<RestoreLeg>,
}

/// Reconstruct the reified `SyntaxFact` for a cached anchor (constructors
/// read only span + occurrence).
fn fact_of(entry: &TransitionEntry, anchor: &Anchor) -> SyntaxFact {
    SyntaxFact {
        grammar_id: entry.grammar_id.clone(),
        syntax_kind: anchor.syntax_kind,
        source_start: anchor.span.start,
        source_end: anchor.span.end,
        recognition_status: markit_mdbench_semantics::RecognitionStatus::Recognized,
        lane_scope_grade: markit_mdbench_semantics::LaneScopeGrade::StrictLaneCoverage,
        host_context: true,
        reason: markit_mdbench_semantics::FactReason::RecognizedUnderLaneOracle,
        occurrence: anchor.occurrence,
        detail: None,
    }
}

fn failure_codes_of(report: &TransitionReport) -> Vec<String> {
    report
        .failure_codes
        .iter()
        .map(|code| format!("{code:?}"))
        .collect()
}

/// Try to construct + derive + validate one leg at one anchor.
fn try_leg(
    entry: &TransitionEntry,
    file: &SelectedFile,
    lane: &FileLaneContext,
    anchor: &Anchor,
) -> Result<CachedLeg, String> {
    let fact = fact_of(entry, anchor);
    let constructed: ConstructedEdit = match entry.transition_id.as_str() {
        "G0-LOCAL-TEXT-REPLACE-EQ" => editors::construct_local_text_replace_eq(&file.text, &fact),
        "G0-PARAGRAPH-SPLIT" => editors::construct_paragraph_split(&file.text, &fact),
        "G0-PARAGRAPH-MERGE" => editors::construct_paragraph_merge(&file.text, &fact),
        "G0-ATX-TO-PARAGRAPH" => editors::construct_atx_marker_remove(&file.text, &fact),
        "G0-LIST-ITEM-INDENT" => editors::construct_list_item_indent(&file.text, &fact),
        "G0-BQ-NEST-LINE" => editors::construct_bq_nest(&file.text, &fact),
        "G0-FENCE-CLOSER-REMOVE" => editors::construct_fence_closer_remove(&file.text, &fact),
        "G0-EMPH-DELIM-BREAK" => editors::construct_emph_delim_break(&file.text, &fact),
        "G0-CODESPAN-DELIM-BREAK" => editors::construct_codespan_delim_break(&file.text, &fact),
        "G0-REFDEF-REMOVE" => editors::construct_refdef_remove(&file.text, &fact),
        "G0-LINK-DEST-BREAK" => editors::construct_link_dest_break(&file.text, &fact),
        "G1-TABLE-DELIM-BREAK" => editors::construct_table_delim_break(&file.text, &fact),
        "G1-TABLE-HEADER-PIPE-REMOVE" => {
            editors::construct_table_header_pipe_remove(&file.text, &fact)
        }
        "G1-TABLE-CELL-EDIT" => editors::construct_table_cell_edit(&file.text, &fact),
        "G1-TABLE-ROW-DELETE" => editors::construct_table_row_delete(&file.text, &fact),
        other => return Err(format!("UNREGISTERED_CONSTRUCTOR:{other}")),
    }?;

    let mut context = constructed.context.clone();
    match entry.transition_id.as_str() {
        "G0-EMPH-DELIM-BREAK" | "G0-CODESPAN-DELIM-BREAK" => {
            context = inline_context(anchor, &lane.profile);
        }
        "G0-FENCE-CLOSER-REMOVE" => {
            context = fence_context(anchor, &lane.profile);
        }
        "G1-TABLE-DELIM-BREAK" | "G1-TABLE-HEADER-PIPE-REMOVE" => {
            context = table_contexts(&file.text, anchor.span).join("+");
        }
        _ => {}
    }

    let post_source = constructed
        .edit
        .apply(&file.text)
        .map_err(|error| format!("edit invalid: {error:?}"))?;
    let post_lane = FileLaneContext::new(&post_source, &entry.grammar_id);
    let derivation = editors::derive_predicates(
        &entry.grammar_id,
        &lane.parse,
        &file.text,
        &lane.profile.syntax_facts,
        &post_lane.parse,
        &post_source,
        &post_lane.profile.syntax_facts,
        entry.expected_pre.clone(),
        entry.expected_post.clone(),
    );
    if !editors::derivation_is_provable(&derivation) {
        return Err("NO_PROVABLE_TRANSITION".to_string());
    }

    // Container-depth truth for the E3 contexts (task §15). Measured
    // LOCALLY around the anchor: a document-global maximum would mask a
    // real 1 -> 2 nest whenever the file nests deeper elsewhere. The
    // window is the anchor span, shifted through the edit.
    if matches!(
        entry.transition_id.as_str(),
        "G0-LIST-ITEM-INDENT" | "G0-BQ-NEST-LINE"
    ) {
        let inserted = constructed.edit.inserted_text.len();
        let pre_depth = window_container_depth(
            &lane.parse,
            anchor.span.start,
            anchor.span.end,
            &lane.profile.structural.container_kinds,
        );
        let post_depth = window_container_depth(
            &post_lane.parse,
            anchor.span.start,
            anchor.span.end + inserted,
            &post_lane.profile.structural.container_kinds,
        );
        if post_depth <= pre_depth {
            return Err("DEPTH_NOT_INCREASED".to_string());
        }
    }

    // Reference-dependency truth: the definition must have a resolved use
    // (task §18: changing definition bytes alone is insufficient).
    if entry.transition_id == "G0-REFDEF-REMOVE" {
        let uses_before = editors::recognized_count(
            &lane.parse,
            &lane.profile.syntax_facts,
            SyntaxKind::LinkReference,
        );
        let uses_after = editors::recognized_count(
            &post_lane.parse,
            &post_lane.profile.syntax_facts,
            SyntaxKind::LinkReference,
        );
        if uses_after >= uses_before {
            return Err("DEFINITION_HAS_NO_RESOLVED_USE".to_string());
        }
    }

    let check = validate_transition(&TransitionRequest {
        grammar_id: entry.grammar_id.clone(),
        pre_source: file.text.clone(),
        post_source: Some(post_source.clone()),
        edit: Some(constructed.edit.clone()),
        expected_pre: derivation.pre_predicates.clone(),
        expected_post: derivation.post_predicates.clone(),
    });
    if !check.is_valid() {
        return Err(format!(
            "ORACLE_INVALID:{}",
            failure_codes_of(&check).join(",")
        ));
    }
    let needs_strict = entry.qualification == RegistryQualification::G0Strict;
    if needs_strict && !check.strict_comparison_eligible {
        // Distinguish pre-side from post-side strict ineligibility
        // (task §39.9/10: pre eligible / post ineligible must be visible).
        return Err(if lane.profile.eligibility.strict_scope_clean {
            "POST_GRAMMAR_INELIGIBLE".to_string()
        } else {
            "PRE_GRAMMAR_INELIGIBLE".to_string()
        });
    }

    let restore =
        build_restore_leg(entry, file, anchor, &constructed, needs_strict)?;

    Ok(CachedLeg {
        context,
        constructed,
        pre_predicates: derivation.pre_predicates.clone(),
        post_predicates: derivation.post_predicates.clone(),
        derivation,
        restore,
    })
}

/// Build the RESTORE leg (from the frozen broken state S1) for entries
/// with `PairedTransition`. Entries with `ExactRestoreLeg` policy ARE
/// restore legs and never appear as break-side cells.
fn build_restore_leg(
    entry: &TransitionEntry,
    file: &SelectedFile,
    anchor: &Anchor,
    constructed: &ConstructedEdit,
    needs_strict: bool,
) -> Result<Option<RestoreLeg>, String> {
    let _ = anchor;
    if entry.restore_policy != RestorePolicy::PairedTransition {
        return Ok(None);
    }
    let s1 = constructed
        .edit
        .apply(&file.text)
        .map_err(|error| format!("break edit invalid: {error:?}"))?;
    let full_registry = transition_registry_v1();
    let restore_entry = entry
        .restore_transition_id
        .as_deref()
        .and_then(|id| registry_entry(&full_registry, id))
        .expect("paired restore entry exists")
        .clone();

    // The exact inverse edit at the same offsets in S1. One formula
    // covers pure insert (removed == ""), pure delete (inserted == "")
    // and replace: the inverse removes exactly the inserted bytes and
    // re-inserts exactly the removed bytes.
    let removed = editors::slice(
        &file.text,
        constructed.edit.edit_start,
        constructed.edit.edit_end,
    );
    let restore_edit = editors::construct_exact_reinsert(
        &s1,
        &removed,
        constructed.edit.edit_start,
        constructed.edit.edit_start + constructed.edit.inserted_text.len() as u64,
        &constructed.context,
    );
    let post_s2 = restore_edit
        .edit
        .apply(&s1)
        .map_err(|error| format!("restore edit invalid: {error:?}"))?;
    let s1_lane = FileLaneContext::new(&s1, &entry.grammar_id);
    let post_lane = FileLaneContext::new(&post_s2, &entry.grammar_id);
    let derivation = editors::derive_predicates(
        &entry.grammar_id,
        &s1_lane.parse,
        &s1,
        &s1_lane.profile.syntax_facts,
        &post_lane.parse,
        &post_s2,
        &post_lane.profile.syntax_facts,
        restore_entry.expected_pre.clone(),
        restore_entry.expected_post.clone(),
    );
    if !editors::derivation_is_provable(&derivation) {
        return Err("RESTORE_NO_PROVABLE_TRANSITION".to_string());
    }
    let check = validate_transition(&TransitionRequest {
        grammar_id: entry.grammar_id.clone(),
        pre_source: s1.clone(),
        post_source: Some(post_s2.clone()),
        edit: Some(restore_edit.edit.clone()),
        expected_pre: derivation.pre_predicates.clone(),
        expected_post: derivation.post_predicates.clone(),
    });
    if !check.is_valid() {
        return Err(format!(
            "RESTORE_ORACLE_INVALID:{}",
            failure_codes_of(&check).join(",")
        ));
    }
    if needs_strict && !check.strict_comparison_eligible {
        return Err("RESTORE_POST_GRAMMAR_INELIGIBLE".to_string());
    }
    let exact = post_s2 == file.text;
    if !exact {
        return Err("RESTORE_NOT_EXACT".to_string());
    }
    Ok(Some(RestoreLeg {
        constructed: restore_edit,
        pre_predicates: derivation.pre_predicates,
        post_predicates: derivation.post_predicates,
        exact,
    }))
}

/// Maximum container depth among container nodes intersecting the byte
/// window `[start, end)`, using the lane's frozen container-kind list and
/// the structural_facts depth convention (a top-level container = 1).
fn window_container_depth(
    parse: &markit_mdbench_semantics::LaneParse,
    start: usize,
    end: usize,
    container_kinds: &[SyntaxKind],
) -> u32 {
    fn walk(node: &markit_mdbench_semantics::LaneNode, depth: u32, best: &mut u32, kinds: &[SyntaxKind], start: usize, end: usize) {
        if node.span.start < end && start < node.span.end && kinds.contains(&node.kind) {
            *best = (*best).max(depth);
        }
        for child in &node.children {
            walk(child, depth + 1, best, kinds, start, end);
        }
    }
    let mut best = 0u32;
    for child in &parse.root.children {
        walk(child, 1, &mut best, container_kinds, start, end);
    }
    best
}

/// Ordered candidate anchors for one requested position (frozen rule:
/// distance, then byte offset, then occurrence).
fn ordered_anchors<'a>(
    anchors: &'a [Anchor],
    source_len: usize,
    requested: RequestedPosition,
) -> Vec<&'a Anchor> {
    let target = requested.target_fraction();
    let mut ordered: Vec<&Anchor> = anchors.iter().collect();
    ordered.sort_by(|a, b| {
        let da = ((a.span.start as f64 / source_len as f64) - target).abs();
        let db = ((b.span.start as f64 / source_len as f64) - target).abs();
        da.partial_cmp(&db)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.span.start.cmp(&b.span.start))
            .then_with(|| a.occurrence.cmp(&b.occurrence))
    });
    ordered
}

/// Assemble the frozen payloads for one resolution (BREAK + optional
/// RESTORE legs) and validate the BREAK/RESTORE chain.
fn assemble_payloads(
    entry: &TransitionEntry,
    file: &SelectedFile,
    commit_sha: Option<String>,
    resolution: &Resolution,
    membership_label: &str,
) -> (Vec<PayloadRecord>, Option<markit_mdbench_semantics::payload::BreakRestoreReport>) {
    let source = &file.text;
    let target_kind = kind_from_name(&entry.syntax_target).expect("registry kind");
    let trace_id = format!(
        "{}|{}|{}",
        entry.transition_id, file.key, resolution.anchor.identity()
    );
    let mut memberships = file.memberships.clone();
    memberships.push("EDIT_WRITE".to_string());
    memberships.push(membership_label.to_string());

    let mut notes = format!("{} context={}", entry.boundary_note, resolution.context);
    if entry.qualification == RegistryQualification::G1SemanticOnly {
        notes.push_str(" SEMANTIC_ONLY; NOT_HORSE_QUALIFIED; never dispatched into H0-H4.");
    }
    notes.push_str(&format!(
        " requested={}",
        resolution
            .requested_positions
            .iter()
            .map(|p| match p {
                RequestedPosition::Early => "early",
                RequestedPosition::Middle => "middle",
                RequestedPosition::Late => "late",
            })
            .collect::<Vec<_>>()
            .join("+")
    ));
    let source_len = source.len() as f64;
    let relative = resolution.anchor.span.start as f64 / source_len;
    let position = PayloadPosition {
        requested_positions: resolution.requested_positions.clone(),
        actual_anchor_byte: resolution.anchor.span.start as u64,
        actual_relative_position: relative,
        distance_from_target: resolution
            .requested_positions
            .iter()
            .map(|requested| markit_mdbench_semantics::payload::TargetDistance {
                requested: *requested,
                target_fraction: requested.target_fraction(),
                distance: (relative - requested.target_fraction()).abs(),
            })
            .collect(),
        dedup_identity: resolution.anchor.identity(),
        deduplicated: resolution.requested_positions.len() > 1,
    };

    let break_payload = build_payload(
        &file.source_id,
        &file.key,
        SourceKind::RealAcquisition,
        commit_sha.clone(),
        &entry.grammar_id,
        source,
        source,
        target_kind,
        &entry.edit_family_label,
        &entry.operation_variant,
        &entry.transition_id,
        resolution.pre_predicates.clone(),
        resolution.post_predicates.clone(),
        &trace_id,
        TraceForm::SingleReset,
        0,
        Some(position),
        resolution.constructed.edit.clone(),
        memberships.clone(),
        Some(notes.clone()),
    );

    let mut payloads = vec![break_payload];
    let mut report = None;

    if let Some(restore) = &resolution.restore {
        let s1 = resolution.constructed.edit.apply(source).expect("break applies");
        let full_registry = transition_registry_v1();
        let restore_entry = entry
            .restore_transition_id
            .as_deref()
            .and_then(|id| registry_entry(&full_registry, id))
            .expect("paired restore entry exists");
        let restore_payload = build_payload(
            &file.source_id,
            &file.key,
            SourceKind::RealAcquisition,
            commit_sha.clone(),
            &entry.grammar_id,
            source,
            &s1,
            target_kind,
            &restore_entry.edit_family_label,
            &restore_entry.operation_variant,
            &restore_entry.transition_id,
            restore.pre_predicates.clone(),
            restore.post_predicates.clone(),
            &trace_id,
            TraceForm::SingleReset,
            1,
            None,
            restore.constructed.edit.clone(),
            memberships.clone(),
            Some(notes.clone()),
        );
        payloads.push(restore_payload);

        let request = BreakRestoreRequest {
            grammar_id: entry.grammar_id.clone(),
            base_source: source.clone(),
            break_edit: resolution.constructed.edit.clone(),
            break_expected_pre: resolution.pre_predicates.clone(),
            break_expected_post: resolution.post_predicates.clone(),
            restore_edit: restore.constructed.edit.clone(),
            restore_expected_pre: restore.pre_predicates.clone(),
            restore_expected_post: restore.post_predicates.clone(),
            expected_restore_exact: true,
        };
        let validated = validate_break_restore(&request);
        if !validated.valid {
            panic!(
                "BREAK/RESTORE chain failed for {trace_id}: {:?}",
                validated.failure_codes
            );
        }
        report = Some(validated);
    }

    (payloads, report)
}

/// Resolve one (file × transition) cell and freeze its payloads.
#[allow(clippy::too_many_lines)]
fn resolve_cell(
    entry: &TransitionEntry,
    file: &SelectedFile,
    commit_sha: Option<String>,
    lane: &FileLaneContext,
    pre_file_ineligible: Option<&str>,
) -> Result<
    (
        MatrixRow,
        Vec<PayloadRecord>,
        Vec<markit_mdbench_semantics::payload::BreakRestoreReport>,
    ),
    String,
> {
    let mut row = MatrixRow {
        source_key: file.key.clone(),
        transition_id: entry.transition_id.clone(),
        grammar_id: entry.grammar_id.clone(),
        lane_id: entry.lane_id.clone(),
        syntax_target: entry.syntax_target.clone(),
        status: STATUS_APPLICABLE.to_string(),
        candidate_anchor_count: 0,
        rejections: Vec::new(),
        position_outcomes: Vec::new(),
        resolved_anchor_count: 0,
        payload_ids: Vec::new(),
        contexts_observed: Vec::new(),
        evidence: None,
    };

    if let Some(reason) = pre_file_ineligible {
        row.status = reason.to_string();
        return Ok((row, Vec::new(), Vec::new()));
    }

    let target_kind = kind_from_name(&entry.syntax_target)
        .unwrap_or_else(|| panic!("registry entry {} has unknown target", entry.transition_id));
    let anchors = lane.facts_for(target_kind);
    row.candidate_anchor_count = anchors.len() as u64;
    if anchors.is_empty() {
        row.status = STATUS_NO_TARGET_SYNTAX.to_string();
        return Ok((row, Vec::new(), Vec::new()));
    }

    let source_len = file.text.len();
    let mut resolutions: Vec<Resolution> = Vec::new();
    // Anchor-identity -> attempted leg (constructor + parse work is the
    // expensive part; positions share the cache).
    let mut leg_cache: std::collections::BTreeMap<String, Result<CachedLeg, String>> =
        std::collections::BTreeMap::new();

    for requested in RequestedPosition::all() {
        let mut fallback_rejections: Vec<RejectionRecord> = Vec::new();
        let mut resolved_identity: Option<String> = None;
        for anchor in ordered_anchors(&anchors, source_len, requested) {
            let identity = anchor_identity(anchor);
            if resolutions
                .iter()
                .any(|existing| existing.anchor.identity() == identity)
            {
                let existing = resolutions
                    .iter_mut()
                    .find(|existing| existing.anchor.identity() == identity)
                    .expect("just checked");
                if !existing.requested_positions.contains(&requested) {
                    existing.requested_positions.push(requested);
                }
                resolved_identity = Some(identity);
                break;
            }
            let attempt = leg_cache
                .entry(identity.clone())
                .or_insert_with(|| try_leg(entry, file, lane, anchor));
            match attempt {
                Ok(leg) => {
                    resolutions.push(Resolution {
                        anchor: anchor.clone(),
                        context: leg.context.clone(),
                        requested_positions: vec![requested],
                        constructed: leg.constructed.clone(),
                        derivation: leg.derivation.clone(),
                        pre_predicates: leg.pre_predicates.clone(),
                        post_predicates: leg.post_predicates.clone(),
                        restore: leg.restore.clone(),
                    });
                    resolved_identity = Some(identity);
                    break;
                }
                Err(reason) => {
                    fallback_rejections.push(RejectionRecord {
                        anchor_identity: identity,
                        reason: reason.clone(),
                    });
                }
            }
        }
        row.position_outcomes.push(PositionOutcome {
            requested: match requested {
                RequestedPosition::Early => "early".to_string(),
                RequestedPosition::Middle => "middle".to_string(),
                RequestedPosition::Late => "late".to_string(),
            },
            resolved_anchor: resolved_identity,
            fallback_rejections,
            deduplicated: false,
        });
    }

    // Merge the per-position rejection records (dedup, keep first-seen
    // order).
    let mut all_rejections: Vec<RejectionRecord> = Vec::new();
    for outcome in &row.position_outcomes {
        for record in &outcome.fallback_rejections {
            if !all_rejections.contains(record) {
                all_rejections.push(record.clone());
            }
        }
    }
    row.rejections = all_rejections;

    // Dedup flags: a resolution carrying more than one requested position
    // covers one physical anchor, not N positions (task §33).
    for resolution in &resolutions {
        let deduplicated = resolution.requested_positions.len() > 1;
        for outcome in &mut row.position_outcomes {
            if outcome.resolved_anchor.as_deref() == Some(resolution.anchor.identity().as_str()) {
                outcome.deduplicated = deduplicated;
            }
        }
    }
    row.resolved_anchor_count = resolutions.len() as u64;

    if resolutions.is_empty() {
        let reasons: Vec<&str> = row
            .rejections
            .iter()
            .map(|record| record.reason.as_str())
            .collect();
        row.status = if reasons.iter().any(|r| r.contains("GRAMMAR_INELIGIBLE")) {
            STATUS_POST_GRAMMAR_INELIGIBLE.to_string()
        } else if reasons.iter().any(|r| {
            r.contains("NO_PROVABLE_TRANSITION")
                || r.contains("DEFINITION_HAS_NO_RESOLVED_USE")
                || r.contains("DEPTH_NOT_INCREASED")
        }) {
            STATUS_TRANSITION_NOT_PRODUCED.to_string()
        } else {
            STATUS_NO_VALID_REAL_ANCHOR.to_string()
        };
        return Ok((row, Vec::new(), Vec::new()));
    }

    // Freeze payloads + chain proofs + evidence.
    let membership_label = if entry.qualification == RegistryQualification::G1SemanticOnly {
        MEMBERSHIP_G1_TABLE_SEMANTIC
    } else {
        MEMBERSHIP_G0_PRIMARY
    };
    let mut cell_payloads = Vec::new();
    let mut chain_reports = Vec::new();
    for resolution in &resolutions {
        let (mut payloads, report) = assemble_payloads(
            entry,
            file,
            commit_sha.clone(),
            resolution,
            membership_label,
        );
        for payload in &payloads {
            let full_registry = transition_registry_v1();
            let agreement_entry = if payload.step == 0 {
                entry
            } else {
                registry_entry(&full_registry, &payload.expected_transition)
                    .expect("restore entry exists")
            };
            payload_agrees_with_registry(
                &payload.grammar_id,
                payload.syntax_target.name(),
                &payload.expected_transition,
                &payload.edit_family,
                &payload.operation_variant,
                &payload.expected_pre,
                &payload.expected_post,
                agreement_entry,
            )
            .map_err(|error| format!("INVALID_PAYLOAD {}: {error}", payload.payload_id))?;
            let pre_source = if payload.step == 0 {
                file.text.clone()
            } else {
                resolution
                    .constructed
                    .edit
                    .apply(&file.text)
                    .expect("break leg applies")
            };
            let validation = validate_payload(payload, &pre_source, &file.text);
            if !validation.valid {
                return Err(format!(
                    "INVALID_PAYLOAD {}: {:?}",
                    payload.payload_id, validation.failure_codes
                ));
            }
        }
        if let Some(report) = report {
            chain_reports.push(report);
        }
        row.payload_ids.extend(payloads.iter().map(|p| p.payload_id.clone()));
        cell_payloads.append(&mut payloads);
        row.contexts_observed.push(resolution.context.clone());
    }
    row.contexts_observed.sort();
    row.contexts_observed.dedup();

    // Oracle evidence of the first resolution (transition truth).
    let first = &resolutions[0];
    let report = validate_transition(&TransitionRequest {
        grammar_id: entry.grammar_id.clone(),
        pre_source: file.text.clone(),
        post_source: first.constructed.edit.apply(&file.text).ok(),
        edit: Some(first.constructed.edit.clone()),
        expected_pre: first.pre_predicates.clone(),
        expected_post: first.post_predicates.clone(),
    });
    row.evidence = Some(EvidenceRecord {
        anchor_identity: first.anchor.identity(),
        strict_comparison_eligible: report.strict_comparison_eligible,
        change_class: report.change_class.map(|class| match class {
            ChangeClass::ContentChange => "content_change".to_string(),
            ChangeClass::StructureChange => "structure_change".to_string(),
            ChangeClass::DependencyResolutionChange => "dependency_resolution_change".to_string(),
            ChangeClass::PositionShiftOnly => "position_shift_only".to_string(),
        }),
        pre_topology_sha256: first.derivation.pre_topology_sha256.clone(),
        post_topology_sha256: first.derivation.post_topology_sha256.clone(),
        oracle_verdict: format!("{:?}", report.verdict),
        failure_codes: failure_codes_of(&report),
    });

    Ok((row, cell_payloads, chain_reports))
}

/// Build the complete frozen workload: matrix rows, payloads, and
/// BREAK/RESTORE chain reports for every (file × registry entry) cell.
pub fn build_frozen_workload(
    files: &[SelectedFile],
    commit_shas: &std::collections::BTreeMap<String, String>,
    profiles: &ProfileMap,
) -> Result<FrozenWorkload, String> {
    let registry = transition_registry_v1();
    let mut rows = Vec::new();
    let mut payloads = Vec::new();
    let mut reports = Vec::new();
    let mut observed: Vec<ObservedSyntax> = Vec::new();

    for file in files {
        // Per-lane observations for the coverage report (G0 only when the
        // file is G0-strict; ineligible files would only yield blocker
        // facts there).
        let mut g0_context: Option<FileLaneContext> = None;
        let mut g1_context: Option<FileLaneContext> = None;
        for entry in &registry {
            // Restore legs are reached through their BREAK entry; they
            // are not independent cells.
            if entry.restore_policy == RestorePolicy::ExactRestoreLeg {
                continue;
            }
            let pre_file_ineligible =
                if entry.grammar_id == G0_GRAMMAR_ID && !file.g0_strict_eligible {
                    Some(STATUS_GRAMMAR_INELIGIBLE)
                } else {
                    None
                };
            if let Some(status) = pre_file_ineligible {
                rows.push(MatrixRow {
                    source_key: file.key.clone(),
                    transition_id: entry.transition_id.clone(),
                    grammar_id: entry.grammar_id.clone(),
                    lane_id: entry.lane_id.clone(),
                    syntax_target: entry.syntax_target.clone(),
                    status: status.to_string(),
                    candidate_anchor_count: 0,
                    rejections: Vec::new(),
                    position_outcomes: Vec::new(),
                    resolved_anchor_count: 0,
                    payload_ids: Vec::new(),
                    contexts_observed: Vec::new(),
                    evidence: None,
                });
                continue;
            }
            let lane_context = match entry.lane_id.as_str() {
                "G1" => g1_context.get_or_insert_with(|| {
                    let profile = profiles
                        .get(&(file.key.clone(), entry.grammar_id.clone()))
                        .cloned()
                        .expect("prebuilt G1 profile");
                    FileLaneContext::from_profile(&file.text, &entry.grammar_id, profile)
                }),
                _ => g0_context.get_or_insert_with(|| {
                    let profile = profiles
                        .get(&(file.key.clone(), entry.grammar_id.clone()))
                        .cloned()
                        .expect("prebuilt G0 profile");
                    FileLaneContext::from_profile(&file.text, &entry.grammar_id, profile)
                }),
            };
            let commit_sha = commit_shas.get(&file.source_id).cloned();
            let (row, mut cell_payloads, chains) =
                resolve_cell(entry, file, commit_sha, lane_context, None)?;
            rows.push(row);
            payloads.append(&mut cell_payloads);
            reports.extend(chains);
        }
        // Collect host-context observations from the lanes that were
        // profiled for this file.
        for (grammar_id, context) in [
            (G0_GRAMMAR_ID, g0_context.as_ref()),
            (crate::G1_GRAMMAR_ID, g1_context.as_ref()),
        ] {
            let Some(context) = context else { continue };
            let mut record = ObservedSyntax {
                source_key: file.key.clone(),
                grammar_id: grammar_id.to_string(),
                recognized: std::collections::BTreeMap::new(),
                candidates: std::collections::BTreeMap::new(),
            };
            for fact in &context.profile.syntax_facts {
                if !fact.host_context {
                    continue;
                }
                if fact.recognition_status == markit_mdbench_semantics::RecognitionStatus::Recognized {
                    *record
                        .recognized
                        .entry(fact.syntax_kind.name().to_string())
                        .or_default() += 1;
                } else {
                    let key = format!(
                        "{}:{}",
                        fact.syntax_kind.name(),
                        match fact.recognition_status {
                            markit_mdbench_semantics::RecognitionStatus::NotRecognized => "not_recognized",
                            markit_mdbench_semantics::RecognitionStatus::Ambiguous => "ambiguous",
                            markit_mdbench_semantics::RecognitionStatus::Unknown => "unknown",
                            markit_mdbench_semantics::RecognitionStatus::Recognized => unreachable!(),
                        }
                    );
                    *record.candidates.entry(key).or_default() += 1;
                }
            }
            observed.push(record);
        }
    }

    // Deterministic artifact order.
    rows.sort_by(|a, b| {
        a.source_key
            .cmp(&b.source_key)
            .then_with(|| a.transition_id.cmp(&b.transition_id))
    });
    payloads.sort_by(|a, b| {
        a.source_path
            .cmp(&b.source_path)
            .then_with(|| a.edit.edit_start.cmp(&b.edit.edit_start))
            .then_with(|| a.payload_id.cmp(&b.payload_id))
    });
    reports.sort_by(|a, b| a.s1_source_sha256.cmp(&b.s1_source_sha256));

    Ok(FrozenWorkload {
        rows,
        payloads,
        break_restore_reports: reports,
        observed,
    })
}

/// Digest helper re-exported for artifact writers.
pub fn digest(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}
