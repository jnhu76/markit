//! markit-mdbench-common — stable substrate contracts for
//! MARKIT-MARKDOWN-BENCHMARK-1 (issue #22).
//!
//! This crate owns only shared, non-research substrate types:
//! source representation, canonical edits, case identity, work-counter
//! schema, failure/correctness taxonomies, and the [`Mechanism`] phase
//! boundary. R0 authority:
//! `protocol/R0-METHODOLOGY.md`.
//!
//! R1 boundary (deliberate):
//!
//! - no Markdown node / AST / CST / tree / fragment / restart
//!   representation is defined here;
//! - no timer or clock type is exposed to mechanisms (timers belong to
//!   `runner`/`instrumentation`);
//! - no performance claim, tuning, or horse algorithm lives here.

pub mod case;
pub mod edit;
pub mod mechanism;
pub mod observed;
pub mod payload;
pub mod source;
pub mod status;
pub mod work;

pub use case::{CaseId, CaseKeyV1, Seed, CASE_ID_ALGORITHM_ID, CASE_KEY_VERSION_V1};
pub use edit::{CanonicalEdit, EditError, OperationKind};
pub use mechanism::{Completed, Mechanism, MechanismContext, MechanismId, ResultChecksum};
pub use observed::Observed;
pub use payload::{PayloadId, PayloadShape, PayloadSize};
pub use source::{to_lower_hex, Source, SourceId};
pub use status::{CorrectnessStatus, ExecutionStatus, FailureStatus};
pub use work::{
    CounterSink, NoopWorkSink, NotApplicableSlot, SourceVersion, WorkCounters, WorkSink,
};
