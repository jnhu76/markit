//! markit-mdbench-runner — case orchestration, timer placement, lane
//! selection, result-row emission.
//!
//! The runner owns the ONLY clock reference and enforces the frozen R0
//! timer contract structurally:
//!
//! ```text
//! T_prepare START  -> mechanism.prepare_update(...)  -> T_prepare STOP
//! T_native  START  -> mechanism.update(...) + mechanism.complete(...)
//!                     + black_box(completed state)   -> T_native STOP
//! T_total = T_prepare + T_native (arithmetic, never a third interval)
//! OUTSIDE ALL TIMERS: oracle, checksum validation, serialization, logging
//! ```
//!
//! Measurement lanes are structurally separate (T/M/A); one run produces
//! exactly one lane's payload and never a composite "headline" number.

pub mod build_identity;
pub mod case_order;
pub mod jsonl;
pub mod orchestrate;
pub mod result;
pub mod supervisor;

pub use build_identity::{current_build_identity, BuildIdentityV1};
pub use case_order::{order_cases, SplitMix64V1, SHUFFLE_ALGORITHM_ID};
pub use jsonl::write_row;
pub use orchestrate::{
    build_initial_state, run_full_parse_attributed, run_full_parse_memory, run_full_parse_timed,
    run_update_attributed, run_update_memory, run_update_timed, RunReport,
};
pub use result::{
    assemble_row, edit_meta, CaseFacts, EditMetaError, EditMetaV1, MeasurementV1, PayloadMetaV1,
    ResultRowV1, TimingMetricsV1, PROTOCOL_VERSION, RESULT_SCHEMA_VERSION_V2,
};
pub use supervisor::{failure_row, run_supervised, synthesized_failure_row, WorkerTermination};
