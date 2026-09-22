//! markit-mdbench-semantics — CORRECTIVE-A semantic substrate (#35).
//!
//! Authority chain:
//!
//! ```text
//! issue #35 (Stage A workload construction)
//!   -> Corrective-1 (grammar lanes / profiler / transition / lifecycle)
//!   -> Workload Construction Algorithm v1
//!   -> protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md (PR #36)
//!   -> this crate
//! ```
//!
//! What this crate is:
//!
//! - [`lanes`] — GRAMMAR-LANES-v1: G0 BENCH-GRAMMAR-v1, G1 CommonMark
//!   0.31.2 + GFM tables, G2 reserved Markit extension identity, each with
//!   independent semantic and H0-H4 qualification states;
//! - [`profile`] — REAL-MARKDOWN-PROFILER-v1: SOURCE_FACT / SYNTAX_FACT /
//!   STRUCTURAL_FACT / ELIGIBILITY_FACT with span evidence, recognition
//!   statuses and the host-context rule;
//! - [`transition`] — TRANSITION-ORACLE-v1: `expected_pre` /
//!   `expected_post` validation over the lane reference parse, plus
//!   independent pre/post eligibility;
//! - [`payload`] — PAYLOAD-LIFECYCLE-v1: identities, inserted-byte
//!   reconstruction, BREAK/RESTORE chains, chained traces, and the
//!   EARLY/MIDDLE/LATE position-dedup rule.
//!
//! What this crate is **not**:
//!
//! - it never measures anything: no clock, no H0-H4 dependency, no
//!   latency/allocation/reuse fact is available to it;
//! - it does not implement semantic interference or any AST-diff metric
//!   (CORRECTIVE-A §G/§H: vocabulary only);
//! - it does not select a workload, freeze a corpus, or materialize
//!   CaseIds.
//!
//! Both G0 and G1 are driven by *reference* implementations: G0 by the
//! frozen repository parse every horse already shares, G1 by the pinned
//! independent oracle recorded in the lane registry. This crate contains no
//! second Markdown parser.

pub mod artifacts;
pub mod canonical;
pub mod facts;
pub mod g0;
pub mod g1;
pub mod lanes;
pub mod parse;
pub mod payload;
pub mod pilot;
pub mod probes;
pub mod profile;
pub mod transition;

pub use canonical::{canonical_json, canonical_json_line, sha256_hex};
pub use facts::{
    EligibilityFacts, FactReason, LaneScopeGrade, NewlineForm, RecognitionStatus, SourceFacts,
    Span, StructuralFacts, SyntaxFact, SyntaxKind, PROFILER_VERSION, PROFILE_SCHEMA,
};
pub use lanes::{
    lane_registry_v1, lane_spec, ConstructStatus, HorseId, HorseStatus, LaneRegistry, LaneSpec,
    SemanticStatus, G0_GRAMMAR_ID, G1_GRAMMAR_ID, G2_GRAMMAR_ID, LANE_REGISTRY_VERSION,
};
pub use parse::{fenced_content_interval, LaneNode, LaneParse};
pub use payload::{
    build_payload, resolve_positions, trace_identity_key, validate_break_restore, validate_payload,
    validate_trace, Anchor, BreakRestoreReport, BreakRestoreRequest, PayloadPosition,
    PayloadRecord, PayloadValidation, PositionResolutionOutcome, RequestedPosition, SourceKind,
    TraceForm, TraceReport, TraceRequest, PAYLOAD_ID_NAMESPACE, PAYLOAD_LIFECYCLE_VERSION,
    PAYLOAD_SCHEMA,
};
pub use probes::{probe_candidates, RawCandidate, PROBE_PRIORITY};
pub use profile::{
    lane_profile, lane_profile_with, profile, profile_g0, profile_g1, source_facts, LaneProfile,
    Profile,
};
pub use transition::{
    validate_transition, ChangeClass, EditSpec, PredicateV1, TransitionReport, TransitionRequest,
    TransitionVerdict, PREDICATE_VERSION, TRANSITION_ORACLE_VERSION,
};
