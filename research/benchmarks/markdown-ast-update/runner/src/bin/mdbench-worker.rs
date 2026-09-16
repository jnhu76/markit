//! R1 failure-isolation worker (R1-CORRECTIVE-1, MAJOR-3).
//!
//! ```text
//! mdbench-worker <job.json> <row_out.jsonl>
//! ```
//!
//! The worker runs ONE case (or one controlled death fixture) inside its
//! own process so the parent supervisor can classify terminations that
//! `catch_unwind` cannot see (abort / fatal signal / abnormal exit /
//! hang). All real work goes through the same runner functions and the
//! same timer code as in-process runs — the worker introduces no
//! measurement path of its own.
//!
//! Output rows are labeled `R1_SMOKE_ONLY/NON_RESEARCH_RESULT`: this is
//! harness-validation plumbing, never benchmark data.

use std::env;
use std::fs;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::ExitCode;

use serde::Deserialize;

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::CaseId;
use markit_mdbench_common::CaseKeyV1;
use markit_mdbench_common::Completed;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::OperationKind;
use markit_mdbench_common::PayloadShape;
use markit_mdbench_common::Seed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkSink;
use markit_mdbench_instrumentation::InstantClock;
use markit_mdbench_null_r1::fixture::{
    smoke_fixture, smoke_payload_id, smoke_payload_size_bytes, R1_SMOKE_ONLY_GENERATOR_ID,
    SMOKE_EDIT_OPERATION,
};
use markit_mdbench_null_r1::{null_checksum, NullMechanism, NullPending};
use markit_mdbench_oracle::ScalarChecksumHook;
use markit_mdbench_runner::{
    assemble_row, build_initial_state, current_build_identity, edit_meta, run_full_parse_timed,
    run_update_timed, write_row, CaseFacts, PayloadMetaV1,
};

const ENVIRONMENT_REF: &str = "manifest/environment.toml#r1-worker";
const PROVENANCE_REF: &str = "R1_SMOKE_ONLY/NON_RESEARCH_RESULT";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WorkerMode {
    /// Real flow: null mechanism UPDATE through `run_update_timed`.
    NullSmokeUpdate,
    /// Real flow: null mechanism FULL_PARSE through `run_full_parse_timed`.
    NullSmokeFullParse,
    /// Mechanism reports `Unsupported` -> failure row, exit 0.
    Unsupported,
    /// Mechanism `update` unwinds -> runner catch -> Crash row, exit 0.
    Panic,
    /// Initial-state construction unwinds -> Crash row, exit 0.
    PanicState,
    /// Hard `abort()`: no row can survive; the supervisor must classify.
    Abort,
    /// Hard nonzero exit: no row; supervisor classification required.
    ExitNonzero,
    /// Hang forever: the supervisor budget must classify a timeout.
    Hang,
}

#[derive(Debug, Deserialize)]
struct WorkerJob {
    mode: WorkerMode,
    exit_code: Option<i32>,
}

/// Test fixture mechanism: `update` unwinds.
struct PanicsInUpdate;
/// Test fixture mechanism: the initial-state `full_parse` unwinds.
struct PanicsInFullParse;

macro_rules! impl_trivial_mechanism {
    ($name:ident, $panic_phase:ident) => {
        impl Mechanism for $name {
            type State = u64;
            type Prepared = u64;
            type Pending = u64;

            fn id(&self) -> MechanismId {
                MechanismId("__r1_worker_death_fixture_test_only__".to_string())
            }

            fn full_parse<W: WorkSink>(
                &self,
                source: &Source,
                _cx: &mut MechanismContext<'_, W>,
            ) -> Result<Self::Pending, FailureStatus> {
                if stringify!($panic_phase) == "full_parse" {
                    panic!("injected initial-state crash for supervisor testing");
                }
                Ok(source.len_bytes() as u64)
            }

            fn prepare_update<W: WorkSink>(
                &self,
                _old: &Source,
                _post: &Source,
                _edit: &CanonicalEdit,
                old_state: &Self::State,
                _cx: &mut MechanismContext<'_, W>,
            ) -> Result<Self::Prepared, FailureStatus> {
                Ok(*old_state)
            }

            fn update<W: WorkSink>(
                &self,
                _old: &Source,
                _post: &Source,
                _edit: &CanonicalEdit,
                old_state: Self::State,
                _prepared: Self::Prepared,
                _cx: &mut MechanismContext<'_, W>,
            ) -> Result<Self::Pending, FailureStatus> {
                if stringify!($panic_phase) == "update" {
                    panic!("injected crash for supervisor testing");
                }
                Ok(old_state + 1)
            }

            fn complete(
                &self,
                pending: Self::Pending,
            ) -> Result<Completed<Self::State>, FailureStatus> {
                Ok(Completed {
                    state: pending,
                    result_checksum: pending,
                })
            }
        }
    };
}

impl_trivial_mechanism!(PanicsInUpdate, update);
impl_trivial_mechanism!(PanicsInFullParse, full_parse);

fn fixture_payload() -> PayloadMetaV1 {
    PayloadMetaV1 {
        payload_id: smoke_payload_id().0,
        shape: PayloadShape::Mixed,
        size_bytes: smoke_payload_size_bytes(),
    }
}

fn key_for(operation: OperationKind, old: &Source, edit: Option<&CanonicalEdit>) -> CaseKeyV1 {
    use sha2::{Digest, Sha256};
    let (start, end, inserted_digest) = match edit {
        None => (None, None, None),
        Some(e) => {
            let mut hasher = Sha256::new();
            hasher.update(e.inserted_text().as_bytes());
            (
                Some(e.start_byte()),
                Some(e.end_byte()),
                Some(hasher.finalize().into()),
            )
        }
    };
    CaseKeyV1 {
        payload_id: smoke_payload_id().0,
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: smoke_payload_size_bytes(),
        old_source_sha256: old.sha256(),
        operation,
        edit_start_byte: start,
        edit_end_byte: end,
        inserted_text_sha256: inserted_digest,
        generator_id: Some(R1_SMOKE_ONLY_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .expect("worker case key must validate")
}

fn facts_for(operation: OperationKind, old: &Source, edit: Option<&CanonicalEdit>) -> CaseFacts {
    CaseFacts {
        case_id: CaseId::from_key(&key_for(operation, old, edit)),
        seed: Seed(0x5EED_0000_0000_0001),
        mechanism_id: MechanismId("__r1_null__".to_string()),
        operation,
        payload: fixture_payload(),
        edit: edit_meta(operation, edit).expect("worker edit meta must satisfy the op contract"),
    }
}

/// Run the requested mode; returns the row to emit. Death-fixture modes
/// never return.
fn run_case(job: &WorkerJob) -> Result<markit_mdbench_runner::ResultRowV1, FailureStatus> {
    let (old, edit, post) = smoke_fixture();
    let clock = InstantClock::new();
    let build = current_build_identity();

    match job.mode {
        WorkerMode::NullSmokeUpdate => {
            let mechanism = NullMechanism::new();
            let old_state = build_initial_state(&mechanism, &old)?;
            let expected = null_checksum(&NullPending {
                old_len_bytes: old.len_bytes() as u64,
                post_len_bytes: post.len_bytes() as u64,
                edit_start_byte: edit.start_byte(),
                edit_end_byte: edit.end_byte(),
                inserted_len_bytes: edit.inserted_text_len_bytes(),
                revision: 1,
            });
            let report = run_update_timed(
                &mechanism,
                &old,
                &post,
                &edit,
                old_state,
                &clock,
                &ScalarChecksumHook::new(expected),
            );
            Ok(assemble_row(
                &facts_for(SMOKE_EDIT_OPERATION, &old, Some(&edit)),
                &report,
                &build,
                ENVIRONMENT_REF,
                PROVENANCE_REF,
            ))
        }
        WorkerMode::NullSmokeFullParse => {
            let mechanism = NullMechanism::new();
            let expected = null_checksum(&NullPending {
                old_len_bytes: old.len_bytes() as u64,
                post_len_bytes: old.len_bytes() as u64,
                edit_start_byte: 0,
                edit_end_byte: 0,
                inserted_len_bytes: 0,
                revision: 0,
            });
            let report =
                run_full_parse_timed(&mechanism, &old, &clock, &ScalarChecksumHook::new(expected));
            Ok(assemble_row(
                &facts_for(OperationKind::FullParse, &old, None),
                &report,
                &build,
                ENVIRONMENT_REF,
                PROVENANCE_REF,
            ))
        }
        WorkerMode::Unsupported => {
            let mechanism =
                NullMechanism::with_failure_mode(markit_mdbench_null_r1::FailureMode::Unsupported);
            let stale_state = markit_mdbench_null_r1::NullState {
                source_len_bytes: old.len_bytes() as u64,
                revision: 0,
            };
            let report = run_update_timed(
                &mechanism,
                &old,
                &post,
                &edit,
                stale_state,
                &clock,
                &ScalarChecksumHook::new(0),
            );
            Ok(assemble_row(
                &facts_for(SMOKE_EDIT_OPERATION, &old, Some(&edit)),
                &report,
                &build,
                ENVIRONMENT_REF,
                PROVENANCE_REF,
            ))
        }
        WorkerMode::Panic => {
            let report = run_update_timed(
                &PanicsInUpdate,
                &old,
                &post,
                &edit,
                0,
                &clock,
                &ScalarChecksumHook::new(0),
            );
            Ok(assemble_row(
                &facts_for(SMOKE_EDIT_OPERATION, &old, Some(&edit)),
                &report,
                &build,
                ENVIRONMENT_REF,
                PROVENANCE_REF,
            ))
        }
        WorkerMode::PanicState => {
            // build_initial_state unwinds; the runner's catch turns it
            // into Err(Crash) — the case must still produce a row.
            let failure = build_initial_state(&PanicsInFullParse, &old).unwrap_err();
            Err(failure)
        }
        WorkerMode::Abort => {
            std::process::abort();
        }
        WorkerMode::ExitNonzero => {
            std::process::exit(job.exit_code.unwrap_or(70));
        }
        WorkerMode::Hang => loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        },
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let (job_path, out_path) = match (args.get(1), args.get(2)) {
        (Some(job), Some(out)) => (job.clone(), out.clone()),
        _ => {
            eprintln!("usage: mdbench-worker <job.json> <row_out.jsonl>");
            return ExitCode::from(2);
        }
    };
    let job: WorkerJob = match fs::read_to_string(&job_path)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
    {
        Ok(job) => job,
        Err(err) => {
            eprintln!("mdbench-worker: bad job: {err}");
            return ExitCode::from(2);
        }
    };

    // Whole-case guard: even a panic outside the runner's catches must
    // not take the row down without a classification.
    let outcome = catch_unwind(AssertUnwindSafe(|| run_case(&job)));

    let operation = facts_operation(&job);
    let edit = facts_edit(&job);
    let facts = facts_for(operation, &smoke_fixture().0, edit);
    let row = match outcome {
        Ok(Ok(row)) => row,
        Ok(Err(failure)) => markit_mdbench_runner::failure_row(
            &facts,
            failure,
            &current_build_identity(),
            ENVIRONMENT_REF,
            PROVENANCE_REF,
        ),
        Err(_) => markit_mdbench_runner::failure_row(
            &facts,
            FailureStatus::Crash,
            &current_build_identity(),
            ENVIRONMENT_REF,
            PROVENANCE_REF,
        ),
    };

    let write = fs::File::create(&out_path).and_then(|mut file| {
        write_row(&mut file, &row)?;
        file.flush()
    });
    if let Err(err) = write {
        eprintln!("mdbench-worker: could not write row: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Operation the row facts describe (death fixtures describe the edit
/// case; PanicState describes the update case whose initial state died).
fn facts_operation(job: &WorkerJob) -> OperationKind {
    match job.mode {
        WorkerMode::NullSmokeFullParse => OperationKind::FullParse,
        _ => SMOKE_EDIT_OPERATION,
    }
}

fn facts_edit(job: &WorkerJob) -> Option<&'static CanonicalEdit> {
    // The smoke fixture is a process-wide constant; leak one instance so
    // the facts can borrow it for the process lifetime.
    use std::sync::OnceLock;
    static FIXTURE: OnceLock<(Source, CanonicalEdit, Source)> = OnceLock::new();
    let fixture = FIXTURE.get_or_init(smoke_fixture);
    match job.mode {
        WorkerMode::NullSmokeFullParse => None,
        _ => Some(&fixture.1),
    }
}
