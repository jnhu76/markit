//! GRAMMAR-LANES-v1 — the frozen grammar-lane registry (CORRECTIVE-A §A).
//!
//! Authority: `grammar/GRAMMAR-LANES-v1.md` (owner document) + `#35`
//! Corrective-1 §2/§10 + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`.
//!
//! Three lanes exist and are **not interchangeable**:
//!
//! ```text
//! G0  BENCH-GRAMMAR-v1            frozen R3 benchmark input language
//! G1  COMMONMARK-0.31.2+GFM-TABLES real-Markdown comparison lane
//! G2  MARKIT-EXT-MATH-v1          reserved Markit extension lane (deferred)
//! ```
//!
//! Every lane reports two independent qualifications
//! (CORRECTIVE-A §B):
//!
//! ```text
//! semantic_status          is the grammar/version/config frozen, is the
//!                          expected meaning defined, is there a reference
//!                          oracle, can pre/post transitions be validated?
//! horse_qualification      does each H0-H4 implement this lane, does
//!                          incremental == clean reference, eager?
//! ```
//!
//! A frozen semantic contract never implies horse qualification, and the
//! registry serialization is the machine-checkable statement of that.

use serde::{Deserialize, Serialize};

use crate::facts::SyntaxKind;

/// Frozen registry version.
pub const LANE_REGISTRY_VERSION: &str = "GRAMMAR-LANES-v1";

/// G0 — the frozen R3 benchmark input language.
pub const G0_GRAMMAR_ID: &str = "BENCH-GRAMMAR-v1";
/// G1 — real-Markdown comparison lane: CommonMark 0.31.2 core + the GFM
/// tables extension, and nothing else.
pub const G1_GRAMMAR_ID: &str = "COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1";
/// G2 — Markit extension lane; identity reserved, semantics deferred.
pub const G2_GRAMMAR_ID: &str = "MARKIT-EXT-MATH-v1";

/// G1 base specification identity (primary source: the CommonMark spec).
pub const G1_BASE_SPEC_VERSION: &str = "0.31.2";
pub const G1_BASE_SPEC_DATE: &str = "2024-01-28";
pub const G1_BASE_SPEC_URL: &str = "https://spec.commonmark.org/0.31.2/";
/// GFM specification identity that owns the tables extension text.
pub const G1_EXTENSION_SPEC_VERSION: &str = "0.29-gfm";
pub const G1_EXTENSION_SPEC_DATE: &str = "2019-04-06";
pub const G1_EXTENSION_SPEC_URL: &str = "https://github.github.com/gfm/";
/// Frozen executable G1 oracle implementation and its exact pin.
pub const G1_ORACLE_CRATE: &str = "pulldown-cmark";
pub const G1_ORACLE_VERSION: &str = "0.13.4";
pub const G1_ORACLE_OPTIONS: &str = "Options::ENABLE_TABLES only (all other options off)";

/// G0 reference-parse identity: the repository's frozen R5 parity
/// substrate, already gated by the 43 hand-authored R3 fixtures.
pub const G0_ORACLE_ID: &str = "BENCH-GRAMMAR-v1-REFERENCE-PARSE-v1";
pub const G0_ORACLE_IMPLEMENTATION: &str =
    "repo crate markit-mdbench-shared-grammar::parse_full + markit-mdbench-oracle::validate_normalized";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SemanticStatus {
    /// No frozen grammar/version/config; nothing may be claimed.
    Undefined,
    /// Partially written; not reviewable authority yet.
    Draft,
    /// Grammar/version/config, expected meaning, normalized vocabulary,
    /// reference oracle and eligibility rules are frozen.
    Qualified,
    /// Lane identity reserved; semantics explicitly deferred.
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HorseStatus {
    /// The lane does not target this horse.
    NotApplicable,
    /// The horse has no implementation for this lane.
    NotImplemented,
    /// Implemented; correctness/parity evidence not yet established.
    CorrectnessPending,
    /// Correctness + parity + eager completion established under this lane.
    Qualified,
    /// Evidence exists and the horse failed.
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HorseId {
    H0FullRebuild,
    H1BlockLocalReparse,
    H2FragmentReuse,
    H3OldTreeSubtreeReuse,
    H4RestartConvergence,
}

impl HorseId {
    pub fn name(self) -> &'static str {
        match self {
            HorseId::H0FullRebuild => "H0",
            HorseId::H1BlockLocalReparse => "H1",
            HorseId::H2FragmentReuse => "H2",
            HorseId::H3OldTreeSubtreeReuse => "H3",
            HorseId::H4RestartConvergence => "H4",
        }
    }

    pub fn mechanism_id(self) -> &'static str {
        match self {
            HorseId::H0FullRebuild => "h0-full-rebuild",
            HorseId::H1BlockLocalReparse => "h1-block-local-reparse",
            HorseId::H2FragmentReuse => "h2-fragment-reuse",
            HorseId::H3OldTreeSubtreeReuse => "h3-old-tree-subtree-reuse",
            HorseId::H4RestartConvergence => "h4-restart-convergence",
        }
    }

    pub fn all() -> [HorseId; 5] {
        [
            HorseId::H0FullRebuild,
            HorseId::H1BlockLocalReparse,
            HorseId::H2FragmentReuse,
            HorseId::H3OldTreeSubtreeReuse,
            HorseId::H4RestartConvergence,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HorseQualification {
    pub horse: HorseId,
    pub mechanism_id: String,
    pub status: HorseStatus,
    /// Where the qualification evidence can be read; empty for
    /// `NotImplemented` / `NotApplicable`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Status of one construct inside one lane's declared scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConstructStatus {
    /// Semantics frozen AND oracle-backed in this corrective: facts may
    /// carry `StrictLaneCoverage`.
    Frozen,
    /// Spec text frozen, but oracle/coverage evidence is not qualified in
    /// this corrective: facts carry `ContractDeclaredNotQualified`.
    DeclaredNotQualified,
    /// The construct belongs to a different lane: facts carry
    /// `OutOfLaneCandidate`.
    OutOfLane,
    /// The owning lane has no frozen semantics yet: facts carry
    /// `LaneDeferred`.
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConstructScope {
    pub syntax_kind: SyntaxKind,
    pub status: ConstructStatus,
    /// Why this status, in one sentence (reviewer-facing).
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BaseSpec {
    pub name: String,
    pub exact_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub authority_url: String,
    /// Frozen repository authority that owns the semantics in-tree.
    pub repository_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExtensionSpec {
    pub name: String,
    pub exact_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub authority_url: String,
    pub section: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OracleSpec {
    pub oracle_id: String,
    /// `RepositoryReferenceImplementation` or
    /// `PinnedIndependentImplementation`.
    pub kind: String,
    pub implementation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact_crate_version: Option<String>,
    pub configuration: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EligibilityRules {
    /// What makes a source a valid document of this lane.
    pub lane_valid_rule: String,
    /// What makes a source *strictly clean* for this lane.
    pub strict_scope_clean_rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LaneSpec {
    /// Short lane id used across documents: `G0`, `G1`, `G2`.
    pub lane_id: String,
    pub grammar_id: String,
    pub lane_version: String,
    pub title: String,
    pub base_spec: BaseSpec,
    pub extensions_enabled: Vec<ExtensionSpec>,
    pub extensions_disabled: Vec<String>,
    /// Precedence / host rules that matter for the pilot (recorded, not
    /// restated as new semantics).
    pub host_precedence_rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalized_vocabulary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_oracle: Option<OracleSpec>,
    pub semantic_status: SemanticStatus,
    /// The exact scope of the semantic claim made by this corrective.
    pub semantic_scope: String,
    pub construct_scopes: Vec<ConstructScope>,
    pub horse_qualification: Vec<HorseQualification>,
    pub eligibility_rules: EligibilityRules,
    /// Known semantic differences against other lanes, recorded so they
    /// can never be silently treated as equivalent.
    pub known_differences: Vec<String>,
}

impl LaneSpec {
    pub fn construct_status(&self, kind: SyntaxKind) -> ConstructStatus {
        self.construct_scopes
            .iter()
            .find(|scope| scope.syntax_kind == kind)
            .map(|scope| scope.status)
            .unwrap_or(ConstructStatus::OutOfLane)
    }

    pub fn horse_status(&self, horse: HorseId) -> HorseStatus {
        self.horse_qualification
            .iter()
            .find(|qualification| qualification.horse == horse)
            .map(|qualification| qualification.status)
            .unwrap_or(HorseStatus::NotApplicable)
    }

    /// Kinds this lane's oracle can recognize (as its own constructs).
    pub fn recognized_kinds(&self) -> Vec<SyntaxKind> {
        self.construct_scopes
            .iter()
            .filter(|scope| {
                matches!(
                    scope.status,
                    ConstructStatus::Frozen | ConstructStatus::DeclaredNotQualified
                )
            })
            .map(|scope| scope.syntax_kind)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LaneRegistry {
    pub registry_version: String,
    pub authority: Vec<String>,
    /// Statement separating semantic qualification from horse
    /// qualification (CORRECTIVE-A §B), carried in the artifact itself.
    pub qualification_rule: String,
    pub lanes: Vec<LaneSpec>,
}

/// The frozen registry (single source of truth for the JSON artifact and
/// for every code path that needs a lane identity or a construct status).
pub fn lane_registry_v1() -> LaneRegistry {
    LaneRegistry {
        registry_version: LANE_REGISTRY_VERSION.to_string(),
        authority: vec![
            "issue #35 Corrective-1 §2 (grammar lanes must be explicit before selection)".into(),
            "issue #35 Workload Construction Algorithm v1 §3 (syntax inventory + eligibility)".into(),
            "protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md §3 (semantic vs implementation qualification)".into(),
            "grammar/BENCH-GRAMMAR-v1.md (G0 owner)".into(),
            "grammar/GRAMMAR-LANES-v1.md (this registry's owner document)".into(),
        ],
        qualification_rule: "A frozen semantic contract never implies H0-H4 implementation \
            qualification. Both gates must pass before a lane may enter strict horse performance \
            comparison; using a file for profiling/realism before then is allowed and must be \
            recorded with its evidence grade."
            .into(),
        lanes: vec![g0_lane(), g1_lane(), g2_lane()],
    }
}

/// Look up a lane by its `grammar_id`.
pub fn lane_spec(grammar_id: &str) -> Option<LaneSpec> {
    lane_registry_v1()
        .lanes
        .into_iter()
        .find(|lane| lane.grammar_id == grammar_id)
}

pub fn g0_lane() -> LaneSpec {
    use ConstructStatus::*;
    use SyntaxKind::*;
    let frozen = |syntax_kind: SyntaxKind, rationale: &str| ConstructScope {
        syntax_kind,
        status: Frozen,
        rationale: rationale.to_string(),
    };
    let out_of_lane = |syntax_kind: SyntaxKind| ConstructScope {
        syntax_kind,
        status: OutOfLane,
        rationale:
            "outside the BENCH-GRAMMAR-v1 construct set; under G0 the bytes are ordinary text"
                .to_string(),
    };
    LaneSpec {
        lane_id: "G0".into(),
        grammar_id: G0_GRAMMAR_ID.into(),
        lane_version: "v1".into(),
        title: "BENCH-GRAMMAR-v1 — frozen benchmark input language".into(),
        base_spec: BaseSpec {
            name: "BENCH-GRAMMAR-v1".into(),
            exact_version: "R3 freeze".into(),
            date: None,
            authority_url: "grammar/BENCH-GRAMMAR-v1.md".into(),
            repository_reference: "grammar/BENCH-GRAMMAR-v1.md + grammar/NORMALIZED-RESULT-v1.md"
                .into(),
        },
        extensions_enabled: vec![],
        extensions_disabled: vec![
            "all CommonMark/GFM constructs outside BENCH-GRAMMAR-v1 §3-§10".into(),
        ],
        host_precedence_rules: vec![
            "block dispatch order B1-B7 is total and ordered (BENCH-GRAMMAR-v1 §2)".into(),
            "code spans win over links; fence bodies are raw bytes".into(),
            "reference resolution is document-global and position-independent".into(),
        ],
        normalized_vocabulary: Some("NORMALIZED-RESULT-v1 (13 node kinds)".into()),
        reference_oracle: Some(OracleSpec {
            oracle_id: G0_ORACLE_ID.into(),
            kind: "RepositoryReferenceImplementation".into(),
            implementation: G0_ORACLE_IMPLEMENTATION.into(),
            exact_crate_version: None,
            configuration: "parse_full + validate_normalized; no configuration options".into(),
            notes: "Already gated by the 43 hand-authored R3 fixtures and by R4/R5 correctness \
                    evidence."
                .into(),
        }),
        semantic_status: SemanticStatus::Qualified,
        semantic_scope: "Entire BENCH-GRAMMAR-v1 construct set. Deviations D1-D13 from CommonMark \
            are authoritative and are NOT to be 'fixed' by this corrective."
            .into(),
        construct_scopes: vec![
            frozen(Paragraph, "BENCH-GRAMMAR-v1 §3"),
            frozen(Text, "BENCH-GRAMMAR-v1 §4"),
            frozen(HeadingAtx, "BENCH-GRAMMAR-v1 §5"),
            frozen(BlockQuote, "BENCH-GRAMMAR-v1 §6"),
            frozen(List, "BENCH-GRAMMAR-v1 §7"),
            frozen(ListItem, "BENCH-GRAMMAR-v1 §7"),
            frozen(CodeBlockFenced, "BENCH-GRAMMAR-v1 §8"),
            frozen(Emphasis, "BENCH-GRAMMAR-v1 §10.2"),
            frozen(CodeSpan, "BENCH-GRAMMAR-v1 §10.1"),
            frozen(LinkInline, "BENCH-GRAMMAR-v1 §9.2"),
            frozen(LinkReference, "BENCH-GRAMMAR-v1 §9.2/§9.3"),
            frozen(ReferenceDefinition, "BENCH-GRAMMAR-v1 §9.4"),
            out_of_lane(HeadingSetext),
            out_of_lane(CodeBlockIndented),
            out_of_lane(HtmlBlock),
            out_of_lane(ThematicBreak),
            out_of_lane(Table),
            out_of_lane(TableHeaderRow),
            out_of_lane(TableRow),
            out_of_lane(TableCell),
            out_of_lane(Strong),
            out_of_lane(LinkAutolink),
            out_of_lane(Image),
            out_of_lane(RawHtmlInline),
            out_of_lane(HardBreak),
            out_of_lane(SoftBreak),
            out_of_lane(InlineMath),
            out_of_lane(DisplayMath),
            out_of_lane(Strikethrough),
            out_of_lane(TaskListItem),
            out_of_lane(FrontMatter),
            out_of_lane(Directive),
        ],
        horse_qualification: HorseId::all()
            .into_iter()
            .map(|horse| HorseQualification {
                horse,
                mechanism_id: horse.mechanism_id().to_string(),
                status: HorseStatus::Qualified,
                evidence: Some(
                    "protocol/R5-HORSE-CORRECTNESS-PARITY.md + protocol/R5-HORSES-STAGE-RECORD.md"
                        .into(),
                ),
            })
            .collect(),
        eligibility_rules: EligibilityRules {
            lane_valid_rule: "Every byte sequence is a valid BENCH-GRAMMAR-v1 document (the \
                grammar is total); lane_valid is therefore always true for UTF-8 input."
                .into(),
            strict_scope_clean_rule: "No host-context candidate outside the BENCH-GRAMMAR-v1 \
                construct set (tables, HTML, math, setext, indented code, autolinks, strikethrough, \
                task lists, front matter, directives, tilde fences) may be present. Candidates \
                inside fence bodies or code spans never block cleanliness."
                .into(),
        },
        known_differences: vec![
            "D1 tabs are ordinary characters (CommonMark uses tab stops)".into(),
            "D2 no backslash escapes (CommonMark has them)".into(),
            "D3 ATX closing sequences are content (CommonMark strips them)".into(),
            "D4 no lazy continuation (CommonMark has it)".into(),
            "D5 unordered tight-only lists (CommonMark: ordered, lazy, loose)".into(),
            "D6 backtick fences only (CommonMark also allows tilde fences)".into(),
            "D7 ASCII-only label case folding (CommonMark uses Unicode case folding)".into(),
            "D8 no shortcut reference links (CommonMark has them)".into(),
            "D9 single-line reference definitions, no titles (CommonMark has titles)".into(),
            "D10 code spans keep exact bytes (CommonMark strips/stabilizes spaces)".into(),
            "D11 single-char '*' LIFO emphasis (CommonMark uses the delimiter-run algorithm)".into(),
            "D12 no indented code blocks (CommonMark has them)".into(),
            "D13 no setext headings, HTML, autolinks, hard breaks, tables, footnotes, task lists \
             (CommonMark/GFM have them)"
                .into(),
        ],
    }
}

pub fn g1_lane() -> LaneSpec {
    use ConstructStatus::*;
    use SyntaxKind::*;
    // The G1 pilot scope: table semantics are frozen and oracle-backed in
    // this corrective; the rest of the CommonMark base is declared (spec
    // text frozen) but its oracle evidence is not qualified yet, so it can
    // never be counted as strict lane coverage.
    let frozen = |syntax_kind: SyntaxKind, rationale: &str| ConstructScope {
        syntax_kind,
        status: Frozen,
        rationale: rationale.to_string(),
    };
    let declared = |syntax_kind: SyntaxKind| ConstructScope {
        syntax_kind,
        status: DeclaredNotQualified,
        rationale: "CommonMark 0.31.2 base construct: the spec text is the frozen authority, but \
            this corrective does not qualify oracle/coverage evidence for it outside the table \
            pilot."
            .to_string(),
    };
    let disabled_extension = |syntax_kind: SyntaxKind| ConstructScope {
        syntax_kind,
        status: OutOfLane,
        rationale:
            "declared GFM/CommonMark construct that this lane configuration disables; under G1 \
             the bytes are ordinary text"
                .to_string(),
    };
    let deferred_lane = |syntax_kind: SyntaxKind| ConstructScope {
        syntax_kind,
        status: Deferred,
        rationale: "owned by the reserved G2 extension lane, whose semantics are deferred".into(),
    };
    LaneSpec {
        lane_id: "G1".into(),
        grammar_id: G1_GRAMMAR_ID.into(),
        lane_version: "v1".into(),
        title: "REAL-MARKDOWN lane — CommonMark 0.31.2 + GFM tables extension".into(),
        base_spec: BaseSpec {
            name: "CommonMark".into(),
            exact_version: G1_BASE_SPEC_VERSION.into(),
            date: Some(G1_BASE_SPEC_DATE.into()),
            authority_url: G1_BASE_SPEC_URL.into(),
            repository_reference: "grammar/GRAMMAR-LANES-v1.md §G1".into(),
        },
        extensions_enabled: vec![ExtensionSpec {
            name: "GFM tables".into(),
            exact_version: G1_EXTENSION_SPEC_VERSION.into(),
            date: Some(G1_EXTENSION_SPEC_DATE.into()),
            authority_url: G1_EXTENSION_SPEC_URL.into(),
            section: "§4.10 Tables (extension)".into(),
        }],
        extensions_disabled: vec![
            "GFM strikethrough".into(),
            "GFM task list items".into(),
            "GFM autolink literals".into(),
            "GFM disallowed raw HTML (tagfilter)".into(),
            "footnotes".into(),
            "smart punctuation".into(),
            "YAML/+++ metadata blocks".into(),
            "heading attributes".into(),
            "definition lists".into(),
            "superscript / subscript".into(),
            "wikilinks".into(),
            "math (owned by G2)".into(),
            "MyST directives and other project-specific extensions".into(),
        ],
        host_precedence_rules: vec![
            "the base dialect is CommonMark 0.31.2; the GFM 0.29-gfm text contributes ONLY the \
             table construct, so an extension/base conflict cannot silently redefine the base"
                .into(),
            "table recognition needs a header row immediately followed by a delimiter row whose \
             cell count matches (GFM §4.10)"
                .into(),
            "a table cannot interrupt a paragraph (GFM §4.10)".into(),
            "pipes inside code spans do not split cells (GFM §4.10)".into(),
            "syntax bytes inside fences/code spans/raw HTML are literal content, never host \
             constructs"
                .into(),
        ],
        normalized_vocabulary: Some("G1-NORMALIZED-v1 (semantics module `g1`)".into()),
        reference_oracle: Some(OracleSpec {
            oracle_id: format!("G1-ORACLE-{G1_ORACLE_CRATE}-{G1_ORACLE_VERSION}"),
            kind: "PinnedIndependentImplementation".into(),
            implementation: format!("{G1_ORACLE_CRATE} {G1_ORACLE_VERSION} (default features off)"),
            exact_crate_version: Some(G1_ORACLE_VERSION.into()),
            configuration: G1_ORACLE_OPTIONS.into(),
            notes: "Independent, widely used CommonMark implementation used as the frozen \
                    executable oracle for the lane. The spec texts remain the semantic authority; \
                    the pilot fixtures declare the expected interpretation independently and are \
                    compared against both."
                .into(),
        }),
        semantic_status: SemanticStatus::Qualified,
        semantic_scope: "SEMANTIC_QUALIFIED for the table construct (pilot scope): the extension \
            text, the normalized table vocabulary, the oracle configuration and the pre/post \
            transition predicates are frozen. All other base constructs are \
            SEMANTIC_CONTRACT_FROZEN / EVIDENCE_NOT_QUALIFIED and must not be counted as strict \
            lane coverage in this corrective."
            .into(),
        construct_scopes: vec![
            frozen(Table, "GFM §4.10 tables extension; the G1 pilot construct"),
            frozen(
                TableHeaderRow,
                "GFM §4.10: the header row; normalized as `table_header_row`",
            ),
            frozen(TableRow, "GFM §4.10"),
            frozen(TableCell, "GFM §4.10"),
            declared(Paragraph),
            declared(HeadingAtx),
            declared(HeadingSetext),
            declared(BlockQuote),
            declared(List),
            declared(ListItem),
            declared(CodeBlockFenced),
            declared(CodeBlockIndented),
            declared(HtmlBlock),
            declared(ThematicBreak),
            declared(ReferenceDefinition),
            declared(Text),
            declared(Emphasis),
            declared(Strong),
            declared(CodeSpan),
            declared(LinkInline),
            declared(LinkReference),
            declared(LinkAutolink),
            declared(Image),
            declared(RawHtmlInline),
            declared(HardBreak),
            declared(SoftBreak),
            disabled_extension(Strikethrough),
            disabled_extension(TaskListItem),
            disabled_extension(FrontMatter),
            disabled_extension(Directive),
            deferred_lane(InlineMath),
            deferred_lane(DisplayMath),
        ],
        horse_qualification: HorseId::all()
            .into_iter()
            .map(|horse| HorseQualification {
                horse,
                mechanism_id: horse.mechanism_id().to_string(),
                status: HorseStatus::NotImplemented,
                evidence: None,
            })
            .collect(),
        eligibility_rules: EligibilityRules {
            lane_valid_rule: "CommonMark is a total grammar: every UTF-8 byte sequence is a valid \
                G1 document, so lane_valid is always true for UTF-8 input. Validity is therefore \
                NOT the interesting property; scope cleanliness is."
                .into(),
            strict_scope_clean_rule: "No host-context candidate outside the enabled lane scope \
                (math, disabled GFM extensions, front matter, directives) may be present. \
                Recognized CommonMark base constructs do not block cleanliness; they are reported \
                with grade contract_declared_not_qualified."
                .into(),
        },
        known_differences: vec![
            "G0 D2/D8/D9/D10/D11/D12/D13 divergence: escapes, shortcut references, reference \
             titles, code-span space handling, delimiter runs, indented code and setext headings \
             all differ between G0 and G1; the pilot pins the setext case explicitly"
                .into(),
            "G0 has no tables; under G0 table-shaped bytes are paragraph text".into(),
            "G0 treats tabs as ordinary characters; G1 uses tab stops".into(),
        ],
    }
}

pub fn g2_lane() -> LaneSpec {
    let deferred = |syntax_kind: SyntaxKind, rationale: &str| ConstructScope {
        syntax_kind,
        status: ConstructStatus::Deferred,
        rationale: rationale.to_string(),
    };
    LaneSpec {
        lane_id: "G2".into(),
        grammar_id: G2_GRAMMAR_ID.into(),
        lane_version: "v1".into(),
        title: "MARKIT extension lane — inline/display dollar math (identity reserved)".into(),
        base_spec: BaseSpec {
            name: "MARKIT extension semantics (not yet written)".into(),
            exact_version: "reserved-identity-only".into(),
            date: None,
            authority_url: "grammar/GRAMMAR-LANES-v1.md §G2".into(),
            repository_reference: "grammar/GRAMMAR-LANES-v1.md §G2".into(),
        },
        extensions_enabled: vec![],
        extensions_disabled: vec![],
        host_precedence_rules: vec![
            "RESERVED, NOT FROZEN: `$...$` / `$$...$$` recognition rules, escaping, and \
             interaction with code spans/fences are not defined by this corrective"
                .into(),
        ],
        normalized_vocabulary: None,
        reference_oracle: None,
        semantic_status: SemanticStatus::Deferred,
        semantic_scope:
            "Identity reserved only. No semantics, no normalized result vocabulary, no \
            oracle, no eligibility rule. Math-looking bytes are reported as candidates with \
            `unknown`/`ambiguous` status under other lanes and are never counted as coverage."
                .into(),
        construct_scopes: vec![
            deferred(
                SyntaxKind::InlineMath,
                "G2 semantic status = DEFERRED (CORRECTIVE-A §A3)",
            ),
            deferred(
                SyntaxKind::DisplayMath,
                "G2 semantic status = DEFERRED (CORRECTIVE-A §A3)",
            ),
        ],
        horse_qualification: HorseId::all()
            .into_iter()
            .map(|horse| HorseQualification {
                horse,
                mechanism_id: horse.mechanism_id().to_string(),
                status: HorseStatus::NotApplicable,
                evidence: None,
            })
            .collect(),
        eligibility_rules: EligibilityRules {
            lane_valid_rule: "Undefined: no G2 semantics exist yet.".into(),
            strict_scope_clean_rule: "Undefined: no G2 semantics exist yet.".into(),
        },
        known_differences: vec![
            "Under G1, `$x$` is a text sequence; under G2 it would be inline math. Until G2 is \
             frozen, such bytes are reported `ambiguous` rather than silently classified."
                .into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_identities_are_versioned_and_distinct() {
        let registry = lane_registry_v1();
        assert_eq!(registry.lanes.len(), 3);
        let ids: Vec<&str> = registry
            .lanes
            .iter()
            .map(|lane| lane.grammar_id.as_str())
            .collect();
        assert_eq!(ids, vec![G0_GRAMMAR_ID, G1_GRAMMAR_ID, G2_GRAMMAR_ID]);
        assert_ne!(G0_GRAMMAR_ID, G1_GRAMMAR_ID);
        assert!(G1_GRAMMAR_ID.contains(G1_BASE_SPEC_VERSION));
        assert!(G1_GRAMMAR_ID.contains(G1_EXTENSION_SPEC_VERSION));
    }

    #[test]
    fn semantic_qualification_never_implies_horse_qualification() {
        let g1 = lane_spec(G1_GRAMMAR_ID).expect("G1");
        assert_eq!(g1.semantic_status, SemanticStatus::Qualified);
        // ... and yet no horse is qualified for it.
        for horse in HorseId::all() {
            assert_eq!(g1.horse_status(horse), HorseStatus::NotImplemented);
        }
        let g2 = lane_spec(G2_GRAMMAR_ID).expect("G2");
        assert_eq!(g2.semantic_status, SemanticStatus::Deferred);
        assert!(g2.reference_oracle.is_none());
    }

    #[test]
    fn g0_keeps_its_frozen_construct_set() {
        let g0 = lane_spec(G0_GRAMMAR_ID).expect("G0");
        assert_eq!(
            g0.construct_status(SyntaxKind::HeadingAtx),
            ConstructStatus::Frozen
        );
        assert_eq!(
            g0.construct_status(SyntaxKind::Table),
            ConstructStatus::OutOfLane
        );
        for horse in HorseId::all() {
            assert_eq!(g0.horse_status(horse), HorseStatus::Qualified);
        }
    }
}
