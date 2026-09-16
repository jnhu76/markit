//! Supervisor/worker failure-isolation tests (R1-CORRECTIVE-1, MAJOR-3).
//!
//! Each test drives the REAL worker binary through the REAL supervisor
//! entry point and proves that every termination class yields a surviving
//! row: normal PASS, mechanism-reported failure, unwind panic, abnormal
//! exit, hard abort (fatal signal), and timeout/hang.
//!
//! OOM and stack-overflow are NOT injected here (unsafe/flaky in a test
//! environment). Their classification path is proven through the
//! controlled fixtures: `abort` produces a real fatal signal (SIGABRT —
//! the same signal a stack overflow raises), and any fatal signal —
//! including the OOM killer's SIGKILL — flows through the identical
//! `FatalSignal -> Crash` classifier. See `runner/src/supervisor.rs` for
//! the recorded platform boundary.

use std::process::Command;
use std::time::Duration;

use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::ExecutionStatus;
use markit_mdbench_common::Observed;
use markit_mdbench_runner::run_supervised;
use markit_mdbench_runner::synthesized_failure_row;
use markit_mdbench_runner::CaseFacts;
use markit_mdbench_runner::MeasurementV1;
use markit_mdbench_runner::ResultRowV1;
use markit_mdbench_runner::WorkerTermination;

/// Drive one worker mode under the supervisor; return the termination
/// class and the worker's row, if it managed to report one.
fn supervised(
    mode: &str,
    exit_code: Option<i32>,
    budget: Duration,
) -> (WorkerTermination, Option<ResultRowV1>) {
    let tmp = std::env::temp_dir();
    let job_path = tmp.join(format!("mdbench-it-{mode}-job.json"));
    let out_path = tmp.join(format!("mdbench-it-{mode}-out.jsonl"));
    let _ = std::fs::remove_file(&out_path);

    let job = match exit_code {
        Some(code) => format!(r#"{{"mode":"{mode}","exit_code":{code}}}"#),
        None => format!(r#"{{"mode":"{mode}"}}"#),
    };
    std::fs::write(&job_path, job).expect("write job");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mdbench-worker"));
    cmd.arg(&job_path).arg(&out_path);
    let termination = run_supervised(cmd, budget).expect("supervised run");

    let row = std::fs::read_to_string(&out_path)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_string))
        .map(|line| serde_json::from_str::<ResultRowV1>(&line).expect("worker row must parse"));

    let _ = std::fs::remove_file(&job_path);
    let _ = std::fs::remove_file(&out_path);
    (termination, row)
}

/// The row the test must finally record: the worker's own row when it
/// reported one, the parent-synthesized row otherwise.
fn settled_row(
    facts: &markit_mdbench_runner::CaseFacts,
    termination: WorkerTermination,
    reported: Option<ResultRowV1>,
) -> ResultRowV1 {
    match (termination, reported) {
        (WorkerTermination::Completed, Some(row)) => row,
        (termination, _) => synthesized_failure_row(
            facts,
            termination,
            &markit_mdbench_runner::current_build_identity(),
            "manifest/environment.toml#r1-worker-test",
            "R1_SMOKE_ONLY/NON_RESEARCH_RESULT",
        ),
    }
}

fn case_facts() -> CaseFacts {
    use markit_mdbench_common::{CaseId, CaseKeyV1, PayloadShape, Seed};
    use markit_mdbench_null_r1::fixture::{
        smoke_fixture, smoke_payload_id, R1_SMOKE_ONLY_GENERATOR_ID,
    };
    let (old, edit, _post) = smoke_fixture();
    let key = CaseKeyV1 {
        payload_id: smoke_payload_id().0,
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: old.len_bytes() as u64,
        old_source_sha256: old.sha256(),
        operation: markit_mdbench_common::OperationKind::Insert,
        edit_start_byte: Some(edit.start_byte()),
        edit_end_byte: Some(edit.end_byte()),
        inserted_text_sha256: Some({
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(edit.inserted_text().as_bytes());
            hasher.finalize().into()
        }),
        generator_id: Some(R1_SMOKE_ONLY_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .expect("facts key");
    CaseFacts {
        case_id: CaseId::from_key(&key),
        seed: Seed(1),
        mechanism_id: markit_mdbench_common::MechanismId("__r1_null__".to_string()),
        operation: markit_mdbench_common::OperationKind::Insert,
        payload: markit_mdbench_runner::PayloadMetaV1 {
            payload_id: smoke_payload_id().0,
            shape: PayloadShape::Mixed,
            size_bytes: old.len_bytes() as u64,
        },
        edit: markit_mdbench_runner::edit_meta(
            markit_mdbench_common::OperationKind::Insert,
            Some(&edit),
        )
        .expect("facts edit meta"),
    }
}

fn assert_unknown_timing(row: &ResultRowV1) {
    match &row.measurement {
        MeasurementV1::Timing(t) => {
            assert_eq!(t.prepare_ns, Observed::Unknown);
            assert_eq!(t.native_ns, Observed::Unknown);
            assert_eq!(t.total_ns, Observed::Unknown);
        }
        other => {
            panic!("supervised failure row must keep the T-LANE with Unknown values, got {other:?}")
        }
    }
}

#[test]
fn worker_normal_pass_survives_with_a_pass_row() {
    let (termination, reported) = supervised("null_smoke_update", None, Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::Completed);
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Pass);
    assert_eq!(row.correctness_status, CorrectnessStatus::Pass);
    assert!(row.result_checksum.is_some());
    match &row.measurement {
        MeasurementV1::Timing(t) => {
            assert!(
                matches!(t.prepare_ns, Observed::Known(_)),
                "real worker run times its prepare phase"
            );
        }
        other => panic!("T-LANE row expected, got {other:?}"),
    }
}

#[test]
fn worker_full_parse_reports_pass_with_not_applicable_prepare() {
    let (termination, reported) =
        supervised("null_smoke_full_parse", None, Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::Completed);
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Pass);
    match &row.measurement {
        MeasurementV1::Timing(t) => {
            assert_eq!(t.prepare_ns, Observed::NotApplicable);
        }
        other => panic!("T-LANE row expected, got {other:?}"),
    }
}

#[test]
fn mechanism_reported_failure_survives_the_process_boundary() {
    let (termination, reported) = supervised("unsupported", None, Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::Completed);
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Unsupported);
    assert_eq!(row.correctness_status, CorrectnessStatus::NotChecked);
    assert_eq!(row.result_checksum, None);
}

#[test]
fn worker_panic_is_recorded_as_crash_and_the_row_survives() {
    let (termination, reported) = supervised("panic", None, Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::Completed);
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Crash);
    assert_eq!(row.correctness_status, CorrectnessStatus::NotChecked);
    assert_eq!(row.result_checksum, None);
}

#[test]
fn panic_in_initial_state_construction_never_erases_the_case() {
    let (termination, reported) = supervised("panic_state", None, Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::Completed);
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Crash);
    assert_eq!(row.correctness_status, CorrectnessStatus::NotChecked);
}

#[test]
fn abnormal_child_exit_is_classified_as_crash() {
    let (termination, reported) = supervised("exit_nonzero", Some(70), Duration::from_secs(60));
    assert_eq!(termination, WorkerTermination::FailedExit(70));
    assert!(reported.is_none(), "a hard exit cannot report a row");
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Crash);
    assert_eq!(row.correctness_status, CorrectnessStatus::NotChecked);
    assert_unknown_timing(&row);
}

#[test]
fn fatal_signal_is_classified_as_crash() {
    // `abort()` raises SIGABRT on Unix — the same signal a stack overflow
    // raises. Windows aborts through a nonzero termination code, which
    // the classifier maps to the same Crash class.
    let (termination, reported) = supervised("abort", None, Duration::from_secs(60));
    assert!(matches!(
        termination,
        WorkerTermination::FatalSignal(_) | WorkerTermination::FailedExit(_)
    ));
    assert!(reported.is_none(), "a hard abort cannot report a row");
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Crash);
    assert_unknown_timing(&row);
}

#[test]
fn hang_is_classified_as_timeout_within_the_budget() {
    let (termination, reported) = supervised("hang", None, Duration::from_millis(400));
    assert_eq!(termination, WorkerTermination::TimedOut);
    assert!(reported.is_none());
    let row = settled_row(&case_facts(), termination, reported);
    assert_eq!(row.execution_status, ExecutionStatus::Timeout);
    assert_eq!(row.correctness_status, CorrectnessStatus::NotChecked);
    assert_unknown_timing(&row);
}
