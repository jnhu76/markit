//! Execution / correctness / failure taxonomies (R0 §13, R1 schema §12).
//!
//! Execution failure and correctness are kept separate: a run that never
//! executed is `NOT_CHECKED` for correctness, and a failed execution must
//! still be recorded — failures are never silently dropped.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Mechanism-reported failure. Returned as `Err` from any [`crate::Mechanism`]
/// phase. (Success is `Ok`, so `PASS` is not a variant here.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureStatus {
    Unsupported,
    Timeout,
    Oom,
    StackOverflow,
    Crash,
    InstrumentationUnavailable,
}

/// Outcome of running one case through the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pass,
    Unsupported,
    Timeout,
    Oom,
    StackOverflow,
    Crash,
    InstrumentationUnavailable,
}

impl From<FailureStatus> for ExecutionStatus {
    fn from(f: FailureStatus) -> Self {
        match f {
            FailureStatus::Unsupported => ExecutionStatus::Unsupported,
            FailureStatus::Timeout => ExecutionStatus::Timeout,
            FailureStatus::Oom => ExecutionStatus::Oom,
            FailureStatus::StackOverflow => ExecutionStatus::StackOverflow,
            FailureStatus::Crash => ExecutionStatus::Crash,
            FailureStatus::InstrumentationUnavailable => {
                ExecutionStatus::InstrumentationUnavailable
            }
        }
    }
}

/// Correctness-hook outcome. Kept orthogonal to [`ExecutionStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CorrectnessStatus {
    NotChecked,
    Pass,
    WrongResult,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_maps_into_execution_status() {
        assert_eq!(
            ExecutionStatus::from(FailureStatus::Unsupported),
            ExecutionStatus::Unsupported
        );
        assert_eq!(
            ExecutionStatus::from(FailureStatus::Crash),
            ExecutionStatus::Crash
        );
    }

    #[test]
    fn serde_names_are_snake_case() {
        assert_eq!(
            serde_json::to_value(ExecutionStatus::StackOverflow).unwrap(),
            serde_json::json!("stack_overflow")
        );
        assert_eq!(
            serde_json::to_value(CorrectnessStatus::NotChecked).unwrap(),
            serde_json::json!("not_checked")
        );
        let back: ExecutionStatus =
            serde_json::from_value(serde_json::json!("instrumentation_unavailable")).unwrap();
        assert_eq!(back, ExecutionStatus::InstrumentationUnavailable);
    }
}
