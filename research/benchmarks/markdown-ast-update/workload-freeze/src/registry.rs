//! TRANSITION-REGISTRY-v1 — the one frozen transition registry
//! (CORRECTIVE-C §8 of the task contract).
//!
//! `expected_transition` must not remain a free-form label: every
//! executable payload references exactly one registry entry, and the
//! payload's own fields must AGREE with that entry (see
//! [`payload_agrees_with_registry`]). A payload that disagrees is
//! `INVALID_PAYLOAD` (G6), regardless of its hashes.
//!
//! Design rules (from the corrective contract):
//!
//! - the registry binds the MINIMUM document-independent predicate sets
//!   (`expected_pre` / `expected_post`). A payload may add
//!   document-specific predicates (exact counts computed at generation
//!   time and re-proven by the oracle); it may not omit or contradict a
//!   registry predicate;
//! - the registry reuses PREDICATE-v1 unchanged — no new predicate
//!   vocabulary, no new transition framework;
//! - E1-E6 are the authoritative headline edit families
//!   (`baseline_edit_family`); syntax-specific transitions carry their
//!   own family label and record the mapping (or the honest absence of
//!   one) explicitly;
//! - G1-table entries are SEMANTIC_ONLY / NOT_HORSE_QUALIFIED: they are
//!   real semantic payloads, never G0-strict or H0-H4 evidence.

use serde::{Deserialize, Serialize};

use markit_mdbench_semantics::transition::CmpOp;
use markit_mdbench_semantics::PredicateV1;

use crate::TRANSITION_REGISTRY_SCHEMA;

/// Registry version.
pub const TRANSITION_REGISTRY_VERSION: &str = "TRANSITION-REGISTRY-v1";

/// Coverage status of one registry entry (closed vocabulary, task §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryCoverageStatus {
    /// Executable payloads exist and are proven under a frozen lane.
    ActiveEditCovered,
    /// The entry documents parse/realism evidence only; no executable
    /// payload is generated under it.
    ParseCoverageOnly,
    /// The owning lane is deferred; no executable payload.
    LaneDeferred,
}

/// Qualification class of an entry's payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryQualification {
    /// G0 primary campaign: pre/post strict-scope-clean, H0-H4-qualified
    /// lane; eligible for the strict correctness dry-run.
    G0Strict,
    /// G1 table pilot scope: semantically proven, horses NOT qualified;
    /// never dispatched into H0-H4 comparison.
    G1SemanticOnly,
}

/// Restore policy of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePolicy {
    /// No RESTORE leg is generated (local content edit without research
    /// value in restoration).
    None,
    /// A paired RESTORE transition exists (its own registry entry).
    PairedTransition,
    /// This entry IS a RESTORE leg; it starts from the frozen broken
    /// state and restores exactly (S2 == S0 verified).
    ExactRestoreLeg,
}

/// One frozen transition registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionEntry {
    pub transition_id: String,
    pub grammar_id: String,
    pub lane_id: String,
    /// The construct the transition is about (snake_case SyntaxKind name).
    pub syntax_target: String,
    /// The authoritative headline edit family this transition maps to
    /// (E1..E6), when one genuinely applies; `None` = syntax-specific
    /// transition without a forced mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_edit_family: Option<String>,
    /// The exact `edit_family` string every payload of this entry carries
    /// (identity field agreement).
    pub edit_family_label: String,
    pub operation_variant: String,
    /// Minimum document-independent pre predicates.
    pub expected_pre: Vec<PredicateV1>,
    /// Minimum document-independent post predicates.
    pub expected_post: Vec<PredicateV1>,
    pub restore_policy: RestorePolicy,
    /// The paired RESTORE entry, when `restore_policy` is
    /// `PairedTransition`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_transition_id: Option<String>,
    pub qualification: RegistryQualification,
    pub coverage_status: RegistryCoverageStatus,
    /// Reviewer-facing statement of what this transition proves.
    pub description: String,
    /// Claim-boundary note carried into every payload's `notes`.
    pub boundary_note: String,
}

/// The frozen registry (single source of truth for the JSON artifact and
/// for payload agreement validation).
pub fn transition_registry_v1() -> Vec<TransitionEntry> {
    use markit_mdbench_semantics::{SyntaxKind, G0_GRAMMAR_ID, G1_GRAMMAR_ID};
    let g0 = |kind: SyntaxKind| kind_predicate(G0_GRAMMAR_ID, kind);
    let g1 = |kind: SyntaxKind| kind_predicate(G1_GRAMMAR_ID, kind);

    let g0_boundary = "G0 BENCH-GRAMMAR-v1 core != full CommonMark/GFM; deviations D1-D13 apply.";
    let g1_boundary = "G1 table SEMANTIC coverage only: horses are NOT qualified for G1; this \
        payload is NOT H0-H4-ready and must never enter strict performance comparison.";

    vec![
        // ------------------------------------------------------------------
        // G0 primary campaign — E1..E6 + required syntax coverage
        // ------------------------------------------------------------------
        TransitionEntry {
            transition_id: "G0-LOCAL-TEXT-REPLACE-EQ".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::Text.name().into(),
            baseline_edit_family: Some("E1_LOCAL_TEXT".into()),
            edit_family_label: "E1_LOCAL_TEXT".into(),
            operation_variant: "replace_eq".into(),
            expected_pre: vec![g0(SyntaxKind::Text)],
            expected_post: vec![PredicateV1::TopologyEqual, PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Deterministic UTF-8-safe same-length replacement inside one real Text \
                run: content changes, the block topology is unchanged."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-PARAGRAPH-SPLIT".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::Paragraph.name().into(),
            baseline_edit_family: Some("E2_PARAGRAPH_SPLIT_MERGE".into()),
            edit_family_label: "E2_PARAGRAPH_SPLIT_MERGE".into(),
            operation_variant: "split".into(),
            expected_pre: vec![g0(SyntaxKind::Paragraph)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "One real paragraph becomes two at an interior space (blank-line \
                boundary created from existing paragraph text only)."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-PARAGRAPH-MERGE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::Paragraph.name().into(),
            baseline_edit_family: Some("E2_PARAGRAPH_SPLIT_MERGE".into()),
            edit_family_label: "E2_PARAGRAPH_SPLIT_MERGE".into(),
            operation_variant: "merge".into(),
            expected_pre: vec![
                g0(SyntaxKind::Paragraph),
                PredicateV1::KindCount {
                    grammar_id: G0_GRAMMAR_ID.into(),
                    syntax_kind: SyntaxKind::Paragraph,
                    op: CmpOp::Ge,
                    count: 2,
                },
            ],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Two adjacent real paragraphs separated by one blank line merge \
                (soft-break join); the inverse boundary edit of the split."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-ATX-TO-PARAGRAPH".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::HeadingAtx.name().into(),
            baseline_edit_family: None,
            edit_family_label: "ATX_HEADING_TOGGLE".into(),
            operation_variant: "marker_remove".into(),
            expected_pre: vec![g0(SyntaxKind::HeadingAtx)],
            expected_post: vec![g0(SyntaxKind::Paragraph), PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-PARAGRAPH-TO-ATX".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Real ATX heading becomes a paragraph by removing the actual '#' \
                marker (#35 requires active ATX-heading coverage; syntax-specific transition, \
                no E-family mapping is forced)."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-PARAGRAPH-TO-ATX".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::HeadingAtx.name().into(),
            baseline_edit_family: None,
            edit_family_label: "ATX_HEADING_TOGGLE".into(),
            operation_variant: "marker_insert".into(),
            expected_pre: vec![g0(SyntaxKind::Paragraph)],
            expected_post: vec![g0(SyntaxKind::HeadingAtx), PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg of the ATX toggle: re-insert the removed marker into the \
                frozen broken state; exact restoration (S2 == S0) is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-LIST-ITEM-INDENT".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::ListItem.name().into(),
            baseline_edit_family: Some("E3_CONTAINER_DEPTH".into()),
            edit_family_label: "E3_CONTAINER_DEPTH".into(),
            operation_variant: "indent_deepen".into(),
            expected_pre: vec![g0(SyntaxKind::ListItem)],
            // Pure container-prefix bytes change here (indent spaces, '>'):
            // they are trivia, so the text fingerprint does not move and the
            // proof is the derived container kind-count change, not text.
            expected_post: vec![],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-LIST-ITEM-DEDENT".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "LIST context container-depth change: indenting an item marker line \
                re-anchors the item under its previous sibling (nested list)."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-LIST-ITEM-DEDENT".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::ListItem.name().into(),
            baseline_edit_family: Some("E3_CONTAINER_DEPTH".into()),
            edit_family_label: "E3_CONTAINER_DEPTH".into(),
            operation_variant: "indent_restore".into(),
            expected_pre: vec![g0(SyntaxKind::ListItem)],
            // Pure container-prefix bytes change here (indent spaces, '>'):
            // they are trivia, so the text fingerprint does not move and the
            // proof is the derived container kind-count change, not text.
            expected_post: vec![],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg of the list indent: removes the inserted indent from the \
                frozen broken state; exact restoration is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-BQ-NEST-LINE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::BlockQuote.name().into(),
            baseline_edit_family: Some("E3_CONTAINER_DEPTH".into()),
            edit_family_label: "E3_CONTAINER_DEPTH".into(),
            operation_variant: "nest_deepen".into(),
            expected_pre: vec![g0(SyntaxKind::BlockQuote)],
            // Pure container-prefix bytes change here (indent spaces, '>'):
            // they are trivia, so the text fingerprint does not move and the
            // proof is the derived container kind-count change, not text.
            expected_post: vec![],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-BQ-UNNEST-LINE".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "BLOCKQUOTE context container-depth change: one real quote line \
                becomes nested one level deeper ('> a' -> '>> a')."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-BQ-UNNEST-LINE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::BlockQuote.name().into(),
            baseline_edit_family: Some("E3_CONTAINER_DEPTH".into()),
            edit_family_label: "E3_CONTAINER_DEPTH".into(),
            operation_variant: "nest_restore".into(),
            expected_pre: vec![g0(SyntaxKind::BlockQuote)],
            // Pure container-prefix bytes change here (indent spaces, '>'):
            // they are trivia, so the text fingerprint does not move and the
            // proof is the derived container kind-count change, not text.
            expected_post: vec![],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg of the quote nest: removes the inserted '>' from the \
                frozen broken state; exact restoration is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-FENCE-CLOSER-REMOVE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::CodeBlockFenced.name().into(),
            baseline_edit_family: Some("E4_FENCE_OPEN_CLOSE".into()),
            edit_family_label: "E4_FENCE_OPEN_CLOSE".into(),
            operation_variant: "close_break".into(),
            expected_pre: vec![g0(SyntaxKind::CodeBlockFenced)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-FENCE-CLOSER-RESTORE".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Real closed fence becomes an unclosed fence that runs to EOF: the \
                valid/closed interpretation is broken by removing the actual closer line."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-FENCE-CLOSER-RESTORE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::CodeBlockFenced.name().into(),
            baseline_edit_family: Some("E4_FENCE_OPEN_CLOSE".into()),
            edit_family_label: "E4_FENCE_OPEN_CLOSE".into(),
            operation_variant: "close_restore".into(),
            expected_pre: vec![g0(SyntaxKind::CodeBlockFenced)],
            expected_post: vec![g0(SyntaxKind::CodeBlockFenced)],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg: re-insert the removed closer into the frozen broken \
                state; exact restoration is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-EMPH-DELIM-BREAK".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::Emphasis.name().into(),
            baseline_edit_family: Some("E5_INLINE_DELIMITER".into()),
            edit_family_label: "E5_INLINE_DELIMITER".into(),
            operation_variant: "delimiter_break".into(),
            expected_pre: vec![g0(SyntaxKind::Emphasis)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-EMPH-DELIM-RESTORE".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Real matched '*' emphasis becomes unmatched text by deleting one \
                delimiter byte (matched -> unmatched)."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-EMPH-DELIM-RESTORE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::Emphasis.name().into(),
            baseline_edit_family: Some("E5_INLINE_DELIMITER".into()),
            edit_family_label: "E5_INLINE_DELIMITER".into(),
            operation_variant: "delimiter_restore".into(),
            expected_pre: vec![g0(SyntaxKind::Text)],
            expected_post: vec![g0(SyntaxKind::Emphasis)],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg: re-insert the deleted delimiter into the frozen broken \
                state (unmatched -> restored); exact restoration is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-CODESPAN-DELIM-BREAK".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::CodeSpan.name().into(),
            baseline_edit_family: Some("E5_INLINE_DELIMITER".into()),
            edit_family_label: "E5_INLINE_DELIMITER".into(),
            operation_variant: "delimiter_break".into(),
            expected_pre: vec![g0(SyntaxKind::CodeSpan)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-CODESPAN-DELIM-RESTORE".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Real code span loses one opening backtick: the run-length rule can \
                no longer close it and the span becomes literal text."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-CODESPAN-DELIM-RESTORE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::CodeSpan.name().into(),
            baseline_edit_family: Some("E5_INLINE_DELIMITER".into()),
            edit_family_label: "E5_INLINE_DELIMITER".into(),
            operation_variant: "delimiter_restore".into(),
            expected_pre: vec![g0(SyntaxKind::Text)],
            expected_post: vec![g0(SyntaxKind::CodeSpan)],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg: re-insert the backtick into the frozen broken state; \
                exact restoration is verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-REFDEF-REMOVE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::ReferenceDefinition.name().into(),
            baseline_edit_family: Some("E6_REFERENCE_DEFINITION".into()),
            edit_family_label: "E6_REFERENCE_DEFINITION".into(),
            operation_variant: "definition_remove".into(),
            expected_pre: vec![g0(SyntaxKind::ReferenceDefinition)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G0-REFDEF-RESTORE".into()),
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "A real RESOLVED reference relation is destroyed: removing a used \
                definition turns its reference uses into literal text (resolved -> \
                unresolved, proven by the reference-use count drop). Also the active-edit \
                proof for reference-link basics."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-REFDEF-RESTORE".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::ReferenceDefinition.name().into(),
            baseline_edit_family: Some("E6_REFERENCE_DEFINITION".into()),
            edit_family_label: "E6_REFERENCE_DEFINITION".into(),
            operation_variant: "definition_restore".into(),
            expected_pre: vec![g0(SyntaxKind::Paragraph)],
            expected_post: vec![g0(SyntaxKind::ReferenceDefinition)],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg (unresolved -> resolved): re-insert the definition line \
                into the frozen broken state; exact restoration and the reference-use count \
                recovery are verified."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G0-LINK-DEST-BREAK".into(),
            grammar_id: G0_GRAMMAR_ID.into(),
            lane_id: "G0".into(),
            syntax_target: SyntaxKind::LinkInline.name().into(),
            baseline_edit_family: Some("E5_INLINE_DELIMITER".into()),
            edit_family_label: "E5_INLINE_DELIMITER".into(),
            operation_variant: "destination_break".into(),
            expected_pre: vec![g0(SyntaxKind::LinkInline)],
            expected_post: vec![],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G0Strict,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Inline-link active coverage: a space inserted into a real inline \
                link's destination makes the construct fail (G0 §9.2); the link decomposes \
                into literal text. A pure destination byte replacement would leave the link \
                a link and be invisible to every PREDICATE-v1 predicate, so the break form \
                is the registry's destination edit."
                .into(),
            boundary_note: g0_boundary.into(),
        },
        // ------------------------------------------------------------------
        // G1 table — SEMANTIC_ONLY, NOT_HORSE_QUALIFIED
        // ------------------------------------------------------------------
        TransitionEntry {
            transition_id: "G1-TABLE-DELIM-BREAK".into(),
            grammar_id: G1_GRAMMAR_ID.into(),
            lane_id: "G1".into(),
            syntax_target: SyntaxKind::Table.name().into(),
            baseline_edit_family: None,
            edit_family_label: "TABLE_SEMANTIC".into(),
            operation_variant: "delimiter_break".into(),
            expected_pre: vec![g1(SyntaxKind::Table)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::PairedTransition,
            restore_transition_id: Some("G1-TABLE-DELIM-RESTORE".into()),
            qualification: RegistryQualification::G1SemanticOnly,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Required machine-proven case: a real valid GFM table becomes a \
                non-Table interpretation when its delimiter row is edited."
                .into(),
            boundary_note: g1_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G1-TABLE-DELIM-RESTORE".into(),
            grammar_id: G1_GRAMMAR_ID.into(),
            lane_id: "G1".into(),
            syntax_target: SyntaxKind::Table.name().into(),
            baseline_edit_family: None,
            edit_family_label: "TABLE_SEMANTIC".into(),
            operation_variant: "delimiter_restore".into(),
            expected_pre: vec![g1(SyntaxKind::Paragraph)],
            expected_post: vec![g1(SyntaxKind::Table)],
            restore_policy: RestorePolicy::ExactRestoreLeg,
            restore_transition_id: None,
            qualification: RegistryQualification::G1SemanticOnly,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "RESTORE leg of the table delimiter break; exact restoration is \
                verified."
                .into(),
            boundary_note: g1_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G1-TABLE-CELL-EDIT".into(),
            grammar_id: G1_GRAMMAR_ID.into(),
            lane_id: "G1".into(),
            syntax_target: SyntaxKind::TableCell.name().into(),
            baseline_edit_family: None,
            edit_family_label: "TABLE_SEMANTIC".into(),
            operation_variant: "cell_content_edit".into(),
            expected_pre: vec![g1(SyntaxKind::Table)],
            expected_post: vec![PredicateV1::TopologyEqual, PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G1SemanticOnly,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Table -> Table local content edit inside one real cell.".into(),
            boundary_note: g1_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G1-TABLE-ROW-DELETE".into(),
            grammar_id: G1_GRAMMAR_ID.into(),
            lane_id: "G1".into(),
            syntax_target: SyntaxKind::TableRow.name().into(),
            baseline_edit_family: None,
            edit_family_label: "TABLE_SEMANTIC".into(),
            operation_variant: "row_delete".into(),
            expected_pre: vec![g1(SyntaxKind::Table)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G1SemanticOnly,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Table -> Table row edit: one real body row line is removed.".into(),
            boundary_note: g1_boundary.into(),
        },
        TransitionEntry {
            transition_id: "G1-TABLE-HEADER-PIPE-REMOVE".into(),
            grammar_id: G1_GRAMMAR_ID.into(),
            lane_id: "G1".into(),
            syntax_target: SyntaxKind::Table.name().into(),
            baseline_edit_family: None,
            edit_family_label: "TABLE_SEMANTIC".into(),
            operation_variant: "column_boundary_edit".into(),
            expected_pre: vec![g1(SyntaxKind::Table)],
            expected_post: vec![PredicateV1::TextContentDiffers],
            restore_policy: RestorePolicy::None,
            restore_transition_id: None,
            qualification: RegistryQualification::G1SemanticOnly,
            coverage_status: RegistryCoverageStatus::ActiveEditCovered,
            description: "Column-boundary edit: removing a header-row pipe breaks the \
                header/delimiter cell-count match (valid Table -> non-Table where the \
                oracle proves it)."
                .into(),
            boundary_note: g1_boundary.into(),
        },
    ]
}

/// The `R^` restore-leg helper: entries whose `restore_policy` is
/// `ExactRestoreLeg` are identified by this marker in coverage reporting.
pub fn is_restore_leg(entry: &TransitionEntry) -> bool {
    entry.restore_policy == RestorePolicy::ExactRestoreLeg
}

/// Look up a registry entry by transition id.
pub fn registry_entry<'a>(
    registry: &'a [TransitionEntry],
    transition_id: &str,
) -> Option<&'a TransitionEntry> {
    registry
        .iter()
        .find(|entry| entry.transition_id == transition_id)
}

/// Predicate-set containment: every registry predicate must appear in the
/// payload's predicate list (exact `PartialEq` match). Payload-specific
/// additional predicates are allowed — the oracle re-proves all of them.
fn predicate_set_covers(registry: &[PredicateV1], payload: &[PredicateV1]) -> bool {
    registry.iter().all(|rp| payload.iter().any(|pp| pp == rp))
}

/// The registry agreement rule (task §39.8): a payload referencing a
/// transition must satisfy the registry entry, else `INVALID_PAYLOAD`.
// The frozen registry-entry shape fixes this parameter list.
#[allow(clippy::too_many_arguments)]
pub fn payload_agrees_with_registry(
    payload_grammar_id: &str,
    payload_syntax_target: &str,
    payload_expected_transition: &str,
    payload_edit_family: &str,
    payload_operation_variant: &str,
    payload_expected_pre: &[PredicateV1],
    payload_expected_post: &[PredicateV1],
    entry: &TransitionEntry,
) -> Result<(), String> {
    if payload_expected_transition != entry.transition_id {
        return Err(format!(
            "transition id {} does not reference registry entry {}",
            payload_expected_transition, entry.transition_id
        ));
    }
    if payload_grammar_id != entry.grammar_id {
        return Err(format!(
            "grammar {} disagrees with registry {}",
            payload_grammar_id, entry.grammar_id
        ));
    }
    if payload_syntax_target != entry.syntax_target {
        return Err(format!(
            "syntax_target {} disagrees with registry {}",
            payload_syntax_target, entry.syntax_target
        ));
    }
    if payload_edit_family != entry.edit_family_label {
        return Err(format!(
            "edit_family {} disagrees with registry {}",
            payload_edit_family, entry.edit_family_label
        ));
    }
    if payload_operation_variant != entry.operation_variant {
        return Err(format!(
            "operation_variant {} disagrees with registry {}",
            payload_operation_variant, entry.operation_variant
        ));
    }
    if !predicate_set_covers(&entry.expected_pre, payload_expected_pre) {
        return Err("expected_pre does not contain the registry minimum".into());
    }
    if !predicate_set_covers(&entry.expected_post, payload_expected_post) {
        return Err("expected_post does not contain the registry minimum".into());
    }
    Ok(())
}

/// Build the JSON artifact value for the registry.
pub fn registry_artifact() -> serde_json::Value {
    #[derive(Serialize)]
    struct Artifact<'a> {
        schema: &'a str,
        registry_version: &'a str,
        generator_version: &'a str,
        predicate_version: &'static str,
        transition_oracle_version: &'static str,
        agreement_rule: &'static str,
        entries: &'a [TransitionEntry],
    }
    let entries = transition_registry_v1();
    let artifact = Artifact {
        schema: TRANSITION_REGISTRY_SCHEMA,
        registry_version: TRANSITION_REGISTRY_VERSION,
        generator_version: crate::CORRECTIVE_C_VERSION,
        predicate_version: markit_mdbench_semantics::PREDICATE_VERSION,
        transition_oracle_version: markit_mdbench_semantics::TRANSITION_ORACLE_VERSION,
        agreement_rule: "A payload references exactly one entry; its grammar_id, syntax_target, \
            edit_family, operation_variant and transition id must equal the entry's, and its \
            expected_pre/expected_post must CONTAIN the entry's minimum predicate sets (extra \
            document-specific predicates are allowed and are re-proven by the oracle). Any \
            disagreement is INVALID_PAYLOAD.",
        entries: &entries,
    };
    serde_json::to_value(artifact).expect("registry artifact is serializable")
}

/// Convenience constructor used by the frozen registry table.
fn kind_predicate(
    grammar_id: &str,
    syntax_kind: markit_mdbench_semantics::SyntaxKind,
) -> PredicateV1 {
    PredicateV1::KindPresent {
        grammar_id: grammar_id.to_string(),
        syntax_kind,
    }
}
