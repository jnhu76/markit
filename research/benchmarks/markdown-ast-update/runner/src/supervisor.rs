//! Process-level failure isolation (R1-CORRECTIVE-1, MAJOR-3).
//!
//! `catch_unwind` covers unwind panics only. OOM, stack overflow,
//! `abort()`, segfaults, and hangs kill the process — in one process
//! those deaths would erase the case. The R1 authority boundary is:
//!
//! ```text
//! parent supervisor
//!   -> worker execution boundary (the whole case: initial-state build,
//!      mechanism phases, row emission)
//!   -> parent classifies worker termination / timeout
//!   -> the failure row SURVIVES (worker-reported, or parent-synthesized)
//! ```
//!
//! Timing authority is untouched: `T_prepare` / `T_native` are produced
//! INSIDE the worker by the same runner timer code, so supervisor
//! overhead (spawn, wait, classification) is structurally outside every
//! timed interval.
//!
//! Classification authority and platform boundary (recorded honestly):
//!
//! - exit code 0 + a row file -> the row is authoritative (including
//!   mechanism-reported failures);
//! - budget overrun -> `Timeout` (the supervisor kills the worker);
//! - fatal signal (SIGABRT from `abort()`/stack overflow, SIGSEGV,
//!   SIGKILL from the OOM killer) -> `Crash`. R1 does NOT distinguish
//!   individual fatal signals: the taxonomy values `oom` and
//!   `stack_overflow` remain worker-self-reportable through
//!   `FailureStatus` rows only. A dedicated OOM/stack-overflow observer
//!   is later measurement-infrastructure work;
//! - nonzero exit without a row -> `Crash` (abnormal exit).
//!
//! Proven in tests with controlled worker fixtures (`abort`,
//! `exit_nonzero`, `hang` modes of `mdbench-worker`); reliable OOM /
//! stack-overflow injection is deliberately NOT attempted in the test
//! environment.

use std::io;
use std::process::Command;
use std::process::ExitStatus;
use std::time::Duration;
use std::time::Instant;

use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_instrumentation::LaneMeasurement;
use markit_mdbench_instrumentation::TimingRecord;

/// How the worker process terminated, as observed by the supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerTermination {
    /// Exit code 0: the worker reported its row(s) itself.
    Completed,
    /// Nonzero exit code without a fatal signal.
    FailedExit(i32),
    /// Killed by a fatal signal (Unix signal number; `-1` when the
    /// platform reports abnormal termination without a number).
    FatalSignal(i32),
    /// Exceeded the per-case budget and was killed by the supervisor.
    TimedOut,
}

impl WorkerTermination {
    /// The failure class recorded when the worker could not report a
    /// row. `Completed` needs no classification (its row speaks).
    pub fn failure_status(self) -> Option<FailureStatus> {
        match self {
            WorkerTermination::Completed => None,
            WorkerTermination::FailedExit(_) | WorkerTermination::FatalSignal(_) => {
                Some(FailureStatus::Crash)
            }
            WorkerTermination::TimedOut => Some(FailureStatus::Timeout),
        }
    }
}

/// Run `cmd` to completion under `budget`. On overrun the worker is
/// killed and [`WorkerTermination::TimedOut`] is returned — never the
/// kill's own exit status, which would misreport a hang as a crash.
pub fn run_supervised(mut cmd: Command, budget: Duration) -> io::Result<WorkerTermination> {
    let mut child = cmd.spawn()?;
    let started = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(classify_exit(status)),
            None => {
                if started.elapsed() > budget {
                    let _ = child.kill();
                    child.wait()?;
                    return Ok(WorkerTermination::TimedOut);
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}

#[cfg(unix)]
fn classify_exit(status: ExitStatus) -> WorkerTermination {
    use std::os::unix::process::ExitStatusExt;
    if let Some(signal) = status.signal() {
        WorkerTermination::FatalSignal(signal)
    } else {
        match status.code() {
            Some(0) => WorkerTermination::Completed,
            Some(code) => WorkerTermination::FailedExit(code),
            None => WorkerTermination::FatalSignal(-1),
        }
    }
}

#[cfg(not(unix))]
fn classify_exit(status: ExitStatus) -> WorkerTermination {
    match status.code() {
        Some(0) => WorkerTermination::Completed,
        Some(code) => WorkerTermination::FailedExit(code),
        None => WorkerTermination::FatalSignal(-1),
    }
}

/// Failure row for a known [`FailureStatus`] (worker-side self-report of
/// a failed run, or the parent after classification).
pub fn failure_row(
    facts: &super::result::CaseFacts,
    failure: FailureStatus,
    build_identity: &super::build_identity::BuildIdentityV1,
    environment_ref: &str,
    provenance_ref: &str,
) -> super::result::ResultRowV1 {
    let report = super::orchestrate::RunReport {
        execution_status: failure.into(),
        correctness_status: CorrectnessStatus::NotChecked,
        measurement: LaneMeasurement::Timing(TimingRecord::unknown()),
        result_checksum: None,
        failure: Some(failure),
    };
    super::result::assemble_row(
        facts,
        &report,
        build_identity,
        environment_ref,
        provenance_ref,
    )
}

/// Parent-side failure row for a case whose worker died without
/// reporting one. The case stays visible with honest `Unknown` metrics;
/// nothing about it is dropped or invented.
pub fn synthesized_failure_row(
    facts: &super::result::CaseFacts,
    termination: WorkerTermination,
    build_identity: &super::build_identity::BuildIdentityV1,
    environment_ref: &str,
    provenance_ref: &str,
) -> super::result::ResultRowV1 {
    let failure = termination
        .failure_status()
        .expect("synthesis needs a non-completed termination");
    failure_row(
        facts,
        failure,
        build_identity,
        environment_ref,
        provenance_ref,
    )
}
