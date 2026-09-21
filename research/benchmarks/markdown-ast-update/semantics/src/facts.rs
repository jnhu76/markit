//! REAL-MARKDOWN-PROFILER-v1 fact vocabulary.
//!
//! Authority: `workloads/profiles/PROFILER-CONTRACT-v1.md` (owner) +
//! `grammar/GRAMMAR-LANES-v1.md`. The types here are the executable form
//! of that contract; every frozen definition that a downstream reader
//! could otherwise re-interpret is carried as data (`block_kinds`,
//! `container_depth_convention`, `reference_definition_counting`, …) or
//! as a `const` in this module.
//!
//! Three fact classes stay separate (CORRECTIVE-A §C):
//!
//! ```text
//! SOURCE_FACT      parser-independent bytes/lines/hashes
//! SYNTAX_FACT      grammar-scoped construct occurrence with a span and a
//!                  recognition status; never a bare regex hit
//! STRUCTURAL_FACT  grammar-scoped counting rules applied to a lane parse
//! ELIGIBILITY_FACT #35 §3's third class: whether pre/post source is valid
//!                  for the claimed comparison lane
//! ```
//!
//! A `SYNTAX_FACT` is emitted in exactly two situations: the lane oracle
//! recognized the construct, or a *declared lane probe* found candidate
//! evidence. Absence without candidate evidence is never emitted, so
//! "detector did not recognize it" can never be read back as "construct
//! absent".

use serde::{Deserialize, Serialize};

/// Frozen profiler identity. Bumping this string is a contract event.
pub const PROFILER_VERSION: &str = "REAL-MARKDOWN-PROFILER-v1";
/// Frozen profile record schema tag.
pub const PROFILE_SCHEMA: &str = "real-profile-v1";
/// Frozen convention string carried by every structural record.
pub const CONTAINER_DEPTH_CONVENTION: &str =
    "container_depth(node) = number of ancestors whose kind is in container_kinds; \
     a top-level block has depth 0; List and ListItem each add 1";
/// Frozen zero-denominator rule string carried by every structural record.
pub const ZERO_DENOMINATOR_RULE: &str =
    "any ratio whose denominator is zero is reported as 0.0 (never NaN/inf); \
     the numerator/denominator byte counts are reported alongside";
/// Frozen rule string for `largest_block_bytes`.
pub const LARGEST_BLOCK_RULE: &str =
    "max over nodes whose kind is in block_kinds of (span.end - span.start) in UTF-8 bytes; \
     0 for a document with no block node; ties broken by the lowest start offset";

/// The closed syntax vocabulary shared by every lane. A lane declares
/// which kinds it can recognize (`lanes::LaneSpecV1::recognized_kinds`);
/// kinds outside that set can still appear as *candidate* facts with a
/// non-`Recognized` status.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxKind {
    /// Synthetic root of a lane parse; never emitted as a syntax fact.
    Document,
    // Block-level
    Paragraph,
    HeadingAtx,
    HeadingSetext,
    BlockQuote,
    List,
    ListItem,
    CodeBlockFenced,
    CodeBlockIndented,
    HtmlBlock,
    ThematicBreak,
    ReferenceDefinition,
    // Table extension (G1 pilot)
    Table,
    TableHeaderRow,
    TableRow,
    TableCell,
    // Inline
    Text,
    Emphasis,
    Strong,
    CodeSpan,
    LinkInline,
    LinkReference,
    LinkAutolink,
    Image,
    RawHtmlInline,
    HardBreak,
    SoftBreak,
    // Lanes / extensions not enabled in the current configurations
    InlineMath,
    DisplayMath,
    Strikethrough,
    TaskListItem,
    FrontMatter,
    Directive,
}

impl SyntaxKind {
    /// Canonical snake_case name (stable; used in artifacts and fixtures).
    pub fn name(self) -> &'static str {
        use SyntaxKind::*;
        match self {
            Document => "document",
            Paragraph => "paragraph",
            HeadingAtx => "heading_atx",
            HeadingSetext => "heading_setext",
            BlockQuote => "block_quote",
            List => "list",
            ListItem => "list_item",
            CodeBlockFenced => "code_block_fenced",
            CodeBlockIndented => "code_block_indented",
            HtmlBlock => "html_block",
            ThematicBreak => "thematic_break",
            ReferenceDefinition => "reference_definition",
            Table => "table",
            TableHeaderRow => "table_header_row",
            TableRow => "table_row",
            TableCell => "table_cell",
            Text => "text",
            Emphasis => "emphasis",
            Strong => "strong",
            CodeSpan => "code_span",
            LinkInline => "link_inline",
            LinkReference => "link_reference",
            LinkAutolink => "link_autolink",
            Image => "image",
            RawHtmlInline => "raw_html_inline",
            HardBreak => "hard_break",
            SoftBreak => "soft_break",
            InlineMath => "inline_math",
            DisplayMath => "display_math",
            Strikethrough => "strikethrough",
            TaskListItem => "task_list_item",
            FrontMatter => "front_matter",
            Directive => "directive",
        }
    }
}

/// Recognition status (CORRECTIVE-A §C2). `NotRecognized` requires
/// candidate evidence; it never means "absent, we did not look".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionStatus {
    /// The lane's reference oracle produced a node of this kind.
    Recognized,
    /// The lane's rules interpret this candidate as ordinary text, or the
    /// candidate fails a frozen lane rule.
    NotRecognized,
    /// Two declared lanes disagree about the candidate (e.g. `$…$` under
    /// G1 text semantics vs the deferred G2 math lane) and this
    /// corrective does not silently pick one.
    Ambiguous,
    /// The construct belongs to a lane whose semantics are not frozen yet.
    Unknown,
}

/// Evidence grade of a fact. Only `StrictLaneCoverage` may be counted as
/// strict lane coverage or as strict H0-H4 comparison evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LaneScopeGrade {
    /// Semantically qualified lane scope with a frozen oracle.
    StrictLaneCoverage,
    /// The lane declares the construct (spec text frozen) but this
    /// corrective does not qualify oracle/coverage evidence for it.
    ContractDeclaredNotQualified,
    /// The candidate belongs to a construct set this lane does not enable.
    OutOfLaneCandidate,
    /// The owning lane has no frozen semantics yet (G2 math).
    LaneDeferred,
}

/// Machine-readable reason attached to every non-recognized fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactReason {
    RecognizedUnderLaneOracle,
    /// Candidate present; the lane's rules make it ordinary text.
    InterpretsAsTextUnderLane,
    /// Candidate looks like the construct but fails a frozen lane rule.
    CandidateRejectedByLaneRule,
    /// The construct is outside this lane's construct set entirely.
    OutsideLaneConstructSet,
    /// Candidate bytes are inside a code/fence/raw region: not host syntax.
    NonHostContext,
    /// The owning lane is deferred (no frozen semantics/oracle).
    LaneSemanticsDeferred,
    /// Two declared lanes disagree; recorded instead of resolved.
    AmbiguousAcrossDeclaredLanes,
}

/// Half-open UTF-8 byte span, relative to the complete document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }

    /// True when the two half-open intervals share at least one byte.
    pub fn intersects(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// True when `self` fully contains `other`.
    pub fn contains(&self, other: &Span) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

/// One syntax occurrence fact. `host_context == false` facts are recorded
/// for transparency (candidate bytes inside code/fence/raw regions) and
/// must never be counted as host syntax or as coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SyntaxFact {
    pub grammar_id: String,
    pub syntax_kind: SyntaxKind,
    pub source_start: usize,
    pub source_end: usize,
    pub recognition_status: RecognitionStatus,
    pub lane_scope_grade: LaneScopeGrade,
    pub host_context: bool,
    pub reason: FactReason,
    /// Stable source-order occurrence index among facts with the same
    /// `grammar_id` + `syntax_kind` + `recognition_status` in this
    /// profile. Serves as the frozen anchor identity for §11.1 tie-breaks.
    pub occurrence: u32,
    /// Kind-specific frozen detail (e.g. fence character for
    /// `code_block_fenced`, delimiter row text for `table`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl SyntaxFact {
    pub fn span(&self) -> Span {
        Span::new(self.source_start, self.source_end)
    }

    /// Coverage-countable fact: recognized, in host context, and inside
    /// the semantically qualified lane scope.
    pub fn is_strict_coverage(&self) -> bool {
        self.recognition_status == RecognitionStatus::Recognized
            && self.host_context
            && self.lane_scope_grade == LaneScopeGrade::StrictLaneCoverage
    }

    /// Blocker for `strict_scope_clean`: a host-context candidate that
    /// the claimed lane cannot interpret as one of its own constructs.
    pub fn is_scope_blocker(&self) -> bool {
        self.host_context
            && matches!(
                self.lane_scope_grade,
                LaneScopeGrade::OutOfLaneCandidate | LaneScopeGrade::LaneDeferred
            )
    }
}

/// Syntax-specific table facts (#35 §4.2). Present only for lanes that
/// recognize the table extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TableFacts {
    pub table_count: u64,
    /// Cells in the widest header row.
    pub max_table_columns: u64,
    /// Largest row count including the header row.
    pub max_table_rows_including_header: u64,
}

/// STRUCTURAL_FACT record for one grammar lane (CORRECTIVE-A §C3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StructuralFacts {
    pub grammar_id: String,
    /// Echoed counting rule: the exact kinds counted as blocks.
    pub block_kinds: Vec<SyntaxKind>,
    /// Echoed counting rule: the exact kinds that contribute depth.
    pub container_kinds: Vec<SyntaxKind>,
    pub block_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub largest_block_span: Option<Span>,
    pub largest_block_bytes: u64,
    pub max_container_depth: u32,
    pub container_depth_convention: String,
    pub largest_block_rule: String,
    pub zero_denominator_rule: String,
    pub fence_density_per_kib: f64,
    pub code_occupancy: f64,
    pub fenced_code_content_bytes: u64,
    pub reference_definition_count: u64,
    pub reference_definition_counting: String,
    pub reference_use_count: u64,
    pub reference_density_per_kib: f64,
    /// `None` when the metric is not applicable to this lane ("math
    /// occupancy where applicable").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math_occupancy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math_occupancy_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<TableFacts>,
}

/// ELIGIBILITY_FACT record for one grammar lane (#35 §3's third class).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EligibilityFacts {
    pub grammar_id: String,
    /// The source has a defined interpretation under this lane.
    pub lane_valid: bool,
    /// No host-context candidate outside the lane's construct set.
    pub strict_scope_clean: bool,
    /// Host-context candidates that block strict scope cleanliness.
    pub scope_blockers: Vec<SyntaxFact>,
    /// Frozen statement of what `strict_scope_clean` means for this lane.
    pub strict_scope_clean_rule: String,
}

/// SOURCE_FACT record (parser-independent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SourceFacts {
    pub source_sha256: String,
    pub file_bytes: u64,
    pub line_count: u64,
    /// LF-terminated lines; the counts make the newline form explicit
    /// without rewriting any byte.
    pub lf_count: u64,
    pub cr_count: u64,
    pub crlf_count: u64,
    pub final_line_terminated: bool,
    pub newline_form: NewlineForm,
    pub ascii_bytes: u64,
    pub non_ascii_bytes: u64,
    /// UTF-8 bytes that decode to code points in the CJK blocks the
    /// profiler contract enumerates; the byte share is over `file_bytes`.
    pub cjk_bytes: u64,
    pub cjk_byte_share: f64,
    pub utf8_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NewlineForm {
    None,
    Lf,
    Crlf,
    Mixed,
    Cr,
}
