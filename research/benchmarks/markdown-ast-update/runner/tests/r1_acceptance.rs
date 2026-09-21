//! R1 acceptance gate tests (task §18 / handoff "R1 TESTS / ACCEPTANCE").
//!
//! These tests prove the harness substrate contract only. They contain no
//! benchmark measurement and no performance claim.

use std::cell::Cell;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::CaseId;
use markit_mdbench_common::CaseKeyV1;
use markit_mdbench_common::Completed;
use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::ExecutionStatus;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::Observed;
use markit_mdbench_common::OperationKind;
use markit_mdbench_common::PayloadShape;
use markit_mdbench_common::Seed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkCounters;
use markit_mdbench_common::WorkSink;
use markit_mdbench_instrumentation::CaseMemoryProbe;
use markit_mdbench_instrumentation::Clock;
use markit_mdbench_instrumentation::LaneMeasurement;
use markit_mdbench_instrumentation::ManualClock;
use markit_mdbench_instrumentation::ManualMemoryReporter;
use markit_mdbench_instrumentation::MemoryRecord;
use markit_mdbench_instrumentation::MemoryReporter;
use markit_mdbench_instrumentation::NoMemoryReporter;
use markit_mdbench_null_r1::fixture::{
    smoke_fixture, smoke_payload_id, smoke_payload_shape, smoke_payload_size_bytes,
    R1_SMOKE_ONLY_GENERATOR_ID, SMOKE_EDIT_OPERATION,
};
use markit_mdbench_common::ResultChecksum;
use markit_mdbench_common::SourceVersion;
use markit_mdbench_null_r1::{null_checksum, NullMechanism, NullState, NULL_R1_MECHANISM_ID};
use markit_mdbench_oracle::CorrectnessHook;
use markit_mdbench_oracle::ScalarChecksumHook;
use markit_mdbench_runner::assemble_row;
use markit_mdbench_runner::build_initial_state;
use markit_mdbench_runner::current_build_identity;
use markit_mdbench_runner::edit_meta;
use markit_mdbench_runner::run_full_parse_timed;
use markit_mdbench_runner::run_update_attributed;
use markit_mdbench_runner::run_update_memory;
use markit_mdbench_runner::run_update_timed;
use markit_mdbench_runner::CaseFacts;
use markit_mdbench_runner::PayloadMetaV1;
use markit_mdbench_runner::ResultRowV1;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn payload_meta() -> PayloadMetaV1 {
    PayloadMetaV1 {
        payload_id: smoke_payload_id().0,
        shape: smoke_payload_shape(),
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
    .expect("test case key must validate")
}

fn facts_for(case_id: CaseId, operation: OperationKind, edit: Option<&CanonicalEdit>) -> CaseFacts {
    CaseFacts {
        case_id,
        seed: Seed(1),
        mechanism_id: MechanismId(NULL_R1_MECHANISM_ID.to_string()),
        operation,
        payload: payload_meta(),
        edit: edit_meta(operation, edit).expect("test facts must satisfy the operation contract"),
    }
}

/// The null mechanism's expected checksum (MEASUREMENT-CORRECTIVE-1): a
/// POST-TIMER export of the sealed `NullState` — no longer a function of
/// the consumed pending scalars.
fn expected_null_checksum(post: &Source, revision: u64) -> u64 {
    null_checksum(&NullState {
        source_len_bytes: post.len_bytes() as u64,
        revision,
    })
}

/// Test-only hook that advances the clock while verifying — used to prove
/// oracle execution happens outside every timer.
struct ClockAdvancingHook {
    clock: Rc<ManualClock>,
    expected: u64,
    ran: Cell<bool>,
}

impl<S: ResultChecksum> CorrectnessHook<S> for ClockAdvancingHook {
    fn verify(&self, completed: &Completed<S>) -> CorrectnessStatus {
        self.ran.set(true);
        self.clock.advance_nanos(1000);
        if completed.state.result_checksum() == self.expected {
            CorrectnessStatus::Pass
        } else {
            CorrectnessStatus::WrongResult
        }
    }
}

/// Local state newtype for the test mechanisms (a foreign trait cannot
/// be implemented on a primitive). The export checksum is the state
/// value itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeState(u64);

impl ResultChecksum for ProbeState {
    fn result_checksum(&self) -> u64 {
        self.0
    }
}

/// Test-only mechanism that simulates phase work by advancing a shared
/// `ManualClock` and records the clock value it observes in each phase.
struct ClockProbe {
    clock: Rc<ManualClock>,
    advance_full_parse: u64,
    advance_prepare: u64,
    advance_native: u64,
    full_parse_read: Cell<u64>,
    prepare_read: Cell<u64>,
    update_read: Cell<u64>,
    complete_read: Cell<u64>,
}

impl ClockProbe {
    fn new(
        clock: Rc<ManualClock>,
        advance_full_parse: u64,
        advance_prepare: u64,
        advance_native: u64,
    ) -> Self {
        Self {
            clock,
            advance_full_parse,
            advance_prepare,
            advance_native,
            full_parse_read: Cell::new(0),
            prepare_read: Cell::new(0),
            update_read: Cell::new(0),
            complete_read: Cell::new(0),
        }
    }
}

impl Mechanism for ClockProbe {
    type State = ProbeState;
    type Prepared = ProbeState;
    type Pending = ProbeState;

    fn id(&self) -> MechanismId {
        MechanismId("__r1_clock_probe_test_only__".to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        self.clock.advance_nanos(self.advance_full_parse);
        self.full_parse_read.set(self.clock.now_nanos());
        Ok(ProbeState(source.len_bytes() as u64))
    }

    fn prepare_update<W: WorkSink>(
        &self,
        _old: &Source,
        _post: &Source,
        _edit: &CanonicalEdit,
        old_state: &Self::State,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        self.clock.advance_nanos(self.advance_prepare);
        self.prepare_read.set(self.clock.now_nanos());
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
        self.clock.advance_nanos(self.advance_native);
        self.update_read.set(self.clock.now_nanos());
        Ok(ProbeState(old_state.0 + 1))
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        self.complete_read.set(self.clock.now_nanos());
        Ok(Completed { state: pending })
    }
}

/// Test-only mechanism that panics inside `update` (real crash path).
struct PanicsInUpdateMechanism;

impl Mechanism for PanicsInUpdateMechanism {
    type State = ProbeState;
    type Prepared = ProbeState;
    type Pending = ProbeState;

    fn id(&self) -> MechanismId {
        MechanismId("__r1_panic_probe_test_only__".to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        Ok(ProbeState(source.len_bytes() as u64))
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
        _old_state: Self::State,
        _prepared: Self::Prepared,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        panic!("injected crash for orchestration testing");
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        Ok(Completed { state: pending })
    }
}

fn workspace_root() -> PathBuf {
    // The runner crate lives directly under the benchmark workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn smoke_case_key_and_edit() -> (CaseKeyV1, CanonicalEdit, Source, Source) {
    let (old, edit, post) = smoke_fixture();
    let key = key_for(SMOKE_EDIT_OPERATION, &old, Some(&edit));
    (key, edit, old, post)
}

// ---------------------------------------------------------------------------
// end-to-end + frozen timer boundaries
// ---------------------------------------------------------------------------

#[test]
fn null_mechanism_end_to_end_pass_through_real_runner() {
    let (key, edit, old, post) = smoke_case_key_and_edit();
    let clock = ManualClock::new();
    let mechanism = NullMechanism::new();
    let old_state = build_initial_state(&mechanism, &old).expect("initial state");

    let report = run_update_timed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &clock,
        &ScalarChecksumHook::new(expected_null_checksum(&post, 1)),
    );

    assert_eq!(report.execution_status, ExecutionStatus::Pass);
    assert_eq!(report.correctness_status, CorrectnessStatus::Pass);
    assert_eq!(report.failure, None);
    assert!(report.result_checksum.is_some());
    match &report.measurement {
        LaneMeasurement::Timing(t) => {
            assert!(t.prepare_ns.is_known());
            assert!(t.native_ns.is_known());
            assert!(t.total_ns.is_known());
        }
        other => panic!("T-LANE run produced {other:?}"),
    }

    let row = assemble_row(
        &facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit)),
        &report,
        &current_build_identity(),
        "manifest/environment.toml#r1-test",
        "r1-test",
    );
    assert_eq!(row.execution_status, ExecutionStatus::Pass);
    assert_eq!(row.correctness_status, CorrectnessStatus::Pass);
    assert!(row.result_checksum.is_some());
}

#[test]
fn update_timing_boundaries_are_exact_and_complete_runs_inside_t_native() {
    let (_key, edit, old, post) = smoke_case_key_and_edit();
    let clock = Rc::new(ManualClock::new());
    // Build-state step happens outside the case (full parse advances 0,
    // one extra complete read at value 0); prepare work adds 11, native
    // work adds 23.
    let probe = Rc::new(ClockProbe::new(clock.clone(), 0, 11, 23));
    let old_state = build_initial_state(&*probe, &old).expect("initial state");

    let report = run_update_timed(
        &*probe,
        &old,
        &post,
        &edit,
        old_state,
        clock.as_ref(),
        &ScalarChecksumHook::new(old_state.0 + 1),
    );

    assert_eq!(report.execution_status, ExecutionStatus::Pass);
    match &report.measurement {
        LaneMeasurement::Timing(t) => {
            assert_eq!(t.prepare_ns, Observed::Known(11));
            assert_eq!(t.native_ns, Observed::Known(23));
            // T_total is the arithmetic sum, nothing else.
            assert_eq!(t.total_ns, Observed::Known(34));
        }
        other => panic!("T-LANE run produced {other:?}"),
    }
    assert!(report.result_checksum.is_some());

    // Exact clock-read sequence: the runner performed reads #2, #4, #5
    // and #8 (prepare start/stop, native start/stop); the mechanism
    // performed #3 (prepare), #6 (update) and #7 (complete).
    // `complete` observed a value strictly between the native START and
    // STOP reads -> complete() executed inside T_native, and nothing
    // (oracle, serialization, logging) read the clock afterwards except
    // what the test does itself.
    assert_eq!(
        probe.prepare_read.get(),
        11,
        "prepare ran inside its window"
    );
    assert_eq!(probe.update_read.get(), 34);
    assert_eq!(
        probe.complete_read.get(),
        34,
        "complete() observed the pre-STOP value: it ran inside T_native"
    );

    // Oracle runs (and takes fake time) strictly outside the timers.
    let oracle_clock = Rc::new(ManualClock::new());
    let oracle_probe = ClockProbe::new(oracle_clock.clone(), 0, 11, 23);
    let state = build_initial_state(&oracle_probe, &old).expect("initial state");
    let hook = ClockAdvancingHook {
        clock: oracle_clock.clone(),
        expected: ProbeState(state.0 + 1).result_checksum(),
        ran: Cell::new(false),
    };
    let report = run_update_timed(
        &oracle_probe,
        &old,
        &post,
        &edit,
        state,
        oracle_clock.as_ref(),
        &hook,
    );
    assert!(hook.ran.get(), "oracle hook must execute");
    match &report.measurement {
        LaneMeasurement::Timing(t) => {
            // The +1000ns of oracle time is NOT in the parser timing.
            assert_eq!(t.prepare_ns, Observed::Known(11));
            assert_eq!(t.native_ns, Observed::Known(23));
            assert_eq!(t.total_ns, Observed::Known(34));
        }
        other => panic!("T-LANE run produced {other:?}"),
    }

    // Serialization/row assembly performs no clock reads at all.
    let reads_before = oracle_clock.recorded_reads().len();
    let key = key_for(SMOKE_EDIT_OPERATION, &old, Some(&edit));
    let _row = assemble_row(
        &facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit)),
        &report,
        &current_build_identity(),
        "manifest/environment.toml#r1-test",
        "r1-test",
    );
    oracle_clock.advance_nanos(2000); // fake serialization cost
    assert_eq!(
        oracle_clock.recorded_reads().len(),
        reads_before,
        "assembly/serialization must not read the timer clock"
    );
}

#[test]
fn full_parse_has_not_applicable_prepare_and_native_only_timing() {
    let (old, _edit, _post) = smoke_fixture();
    let clock = Rc::new(ManualClock::new());
    let probe = ClockProbe::new(clock.clone(), 23, 0, 0);

    let report = run_full_parse_timed(
        &probe,
        &old,
        clock.as_ref(),
        &ScalarChecksumHook::new(old.len_bytes() as u64),
    );

    assert_eq!(report.execution_status, ExecutionStatus::Pass);
    match &report.measurement {
        LaneMeasurement::Timing(t) => {
            assert_eq!(t.prepare_ns, Observed::NotApplicable);
            assert_eq!(t.native_ns, Observed::Known(23));
            assert_eq!(
                t.total_ns,
                Observed::Known(23),
                "T_total == T_native for FULL_PARSE"
            );
        }
        other => panic!("T-LANE run produced {other:?}"),
    }
    // Probe's full-parse read (23) sits strictly between the native
    // START read (0) and STOP read (23-window) -> inside T_native.
    assert_eq!(probe.full_parse_read.get(), 23);
    assert_eq!(probe.complete_read.get(), 23);
}

// ---------------------------------------------------------------------------
// lane separation
// ---------------------------------------------------------------------------

#[test]
fn lanes_are_structurally_exclusive_in_reports_and_rows() {
    let (key, edit, old, post) = smoke_case_key_and_edit();
    let mechanism = NullMechanism::new();
    let old_state = build_initial_state(&mechanism, &old).expect("initial state");
    let hook = ScalarChecksumHook::new(expected_null_checksum(&post, 1));

    let timed = run_update_timed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &ManualClock::new(),
        &hook,
    );
    let mut counters = WorkCounters::all_unknown();
    let attributed = run_update_attributed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &mut counters,
        &hook,
    );
    let memory = run_update_memory(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &NoMemoryReporter,
        &hook,
    );

    assert!(matches!(timed.measurement, LaneMeasurement::Timing(_)));
    assert!(matches!(
        attributed.measurement,
        LaneMeasurement::Attribution(_)
    ));
    assert!(matches!(memory.measurement, LaneMeasurement::Memory(_)));

    let build = current_build_identity();
    let facts = facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit));
    let timed_row = assemble_row(&facts, &timed, &build, "env", "r1-test");
    let attributed_row = assemble_row(&facts, &attributed, &build, "env", "r1-test");
    let memory_row = assemble_row(&facts, &memory, &build, "env", "r1-test");

    let timed_json = serde_json::to_value(&timed_row).unwrap();
    let attributed_json = serde_json::to_value(&attributed_row).unwrap();
    let memory_json = serde_json::to_value(&memory_row).unwrap();

    assert_eq!(timed_json["measurement"]["lane"], "timing");
    assert!(timed_json["measurement"]["metrics"]
        .get("prepare_ns")
        .is_some());
    assert!(timed_json.to_string().find("blocks_reparsed").is_none());
    assert!(timed_json.to_string().find("allocated_bytes").is_none());

    assert_eq!(attributed_json["measurement"]["lane"], "attribution");
    assert!(attributed_json["measurement"]["metrics"]
        .get("blocks_reparsed")
        .is_some());
    assert!(attributed_json.to_string().find("prepare_ns").is_none());

    assert_eq!(memory_json["measurement"]["lane"], "memory");
    assert!(memory_json["measurement"]["metrics"]
        .get("allocated_bytes")
        .is_some());
    assert!(memory_json.to_string().find("prepare_ns").is_none());

    // A-LANE facts of the null mechanism, exactly as documented:
    // Known(0) stays 0, restart/convergence are NotApplicable, the rest
    // is Unknown — three distinct states, all visible in the row.
    let metrics = &attributed_json["measurement"]["metrics"];
    assert_eq!(metrics["blocks_reparsed"], 0);
    // ATTRIBUTION-SCHEMA-v2 derived slots: the null mechanism records no
    // inspection events, so the finalized derivation is the measured
    // zero on every derived slot (per version, combined, and effort).
    assert_eq!(metrics["unique_old_source_bytes"], 0);
    assert_eq!(metrics["unique_post_source_bytes"], 0);
    assert_eq!(metrics["unique_source_bytes"], 0);
    assert_eq!(metrics["source_bytes_inspected_total"], 0);
    assert_eq!(metrics["restart_distance"], "NOT_APPLICABLE");
    assert_eq!(metrics["nodes_rebuilt"], "UNKNOWN");
}

#[test]
fn case_id_is_identical_across_lanes_and_mechanisms() {
    let (key, edit, old, post) = smoke_case_key_and_edit();
    let mechanism = NullMechanism::new();
    let old_state = build_initial_state(&mechanism, &old).expect("initial state");
    let hook = ScalarChecksumHook::new(expected_null_checksum(&post, 1));

    let timed = run_update_timed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &ManualClock::new(),
        &hook,
    );
    let mut counters = WorkCounters::all_unknown();
    let attributed = run_update_attributed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &mut counters,
        &hook,
    );

    // Different mechanism instance, same logical case -> same CaseId.
    let probe_clock = Rc::new(ManualClock::new());
    let probe = ClockProbe::new(probe_clock.clone(), 0, 0, 0);
    let _ = run_update_timed(
        &probe,
        &old,
        &post,
        &edit,
        ProbeState(0),
        probe_clock.as_ref(),
        &ScalarChecksumHook::new(1),
    );

    let build = current_build_identity();
    let facts = facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit));
    let a = assemble_row(&facts, &timed, &build, "env", "r1-test").case_id;
    let b = assemble_row(&facts, &attributed, &build, "env", "r1-test").case_id;
    let expected = key_hex(&key);
    assert_eq!(a, expected);
    assert_eq!(b, expected);
}

fn key_hex(key: &CaseKeyV1) -> String {
    CaseId::from_key(key).hex()
}

// ---------------------------------------------------------------------------
// failure preservation
// ---------------------------------------------------------------------------

#[test]
fn injected_failures_are_preserved_and_never_dropped() {
    let (key, edit, old, post) = smoke_case_key_and_edit();
    let build = current_build_identity();
    let facts = facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit));

    let unsupported =
        NullMechanism::with_failure_mode(markit_mdbench_null_r1::FailureMode::Unsupported);
    let stale_state = markit_mdbench_null_r1::NullState {
        source_len_bytes: old.len_bytes() as u64,
        revision: 0,
    };
    let report_unsupported = run_update_timed(
        &unsupported,
        &old,
        &post,
        &edit,
        stale_state,
        &ManualClock::new(),
        &ScalarChecksumHook::new(0),
    );
    assert_eq!(
        report_unsupported.execution_status,
        ExecutionStatus::Unsupported
    );
    assert_eq!(
        report_unsupported.correctness_status,
        CorrectnessStatus::NotChecked
    );
    assert_eq!(report_unsupported.failure, Some(FailureStatus::Unsupported));
    assert_eq!(report_unsupported.result_checksum, None);
    match &report_unsupported.measurement {
        LaneMeasurement::Timing(t) => {
            assert_eq!(t.prepare_ns, Observed::Unknown);
            assert_eq!(t.native_ns, Observed::Unknown);
            assert_eq!(t.total_ns, Observed::Unknown);
        }
        other => panic!("failed run produced {other:?}"),
    }

    // Real panic inside a mechanism phase -> CRASH, still a row.
    let report_crash = run_update_timed(
        &PanicsInUpdateMechanism,
        &old,
        &post,
        &edit,
        ProbeState(0),
        &ManualClock::new(),
        &ScalarChecksumHook::new(0),
    );
    assert_eq!(report_crash.execution_status, ExecutionStatus::Crash);
    assert_eq!(
        report_crash.correctness_status,
        CorrectnessStatus::NotChecked
    );

    // Failure rows must survive assembly: run one case set of three
    // (pass / unsupported / crash) and emit all three rows.
    let passing = NullMechanism::new();
    let state = build_initial_state(&passing, &old).expect("initial state");
    let report_pass = run_update_timed(
        &passing,
        &old,
        &post,
        &edit,
        state,
        &ManualClock::new(),
        &ScalarChecksumHook::new(expected_null_checksum(&post, 1)),
    );
    let rows = [
        assemble_row(&facts, &report_pass, &build, "env", "r1-test"),
        assemble_row(&facts, &report_unsupported, &build, "env", "r1-test"),
        assemble_row(&facts, &report_crash, &build, "env", "r1-test"),
    ];
    assert_eq!(rows.len(), 3, "failure rows are not silently dropped");
    assert_eq!(rows[0].execution_status, ExecutionStatus::Pass);
    assert_eq!(rows[1].execution_status, ExecutionStatus::Unsupported);
    assert_eq!(rows[2].execution_status, ExecutionStatus::Crash);
    for row in &rows {
        // Every row must round-trip with its failure taxonomy intact.
        let json = serde_json::to_string(row).unwrap();
        let back: ResultRowV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, row);
    }
}

// ---------------------------------------------------------------------------
// result schema
// ---------------------------------------------------------------------------

fn schema_path() -> PathBuf {
    workspace_root()
        .join("protocol")
        .join("result-schema-v2.json")
}

fn sample_rows() -> Vec<ResultRowV1> {
    let (key, edit, old, post) = smoke_case_key_and_edit();
    let mechanism = NullMechanism::new();
    let old_state = build_initial_state(&mechanism, &old).expect("initial state");
    let hook = ScalarChecksumHook::new(expected_null_checksum(&post, 1));
    let build = current_build_identity();
    let env = "manifest/environment.toml#r1-test";
    let provenance = "R1_SMOKE_ONLY/NON_RESEARCH_RESULT";
    let facts = facts_for(CaseId::from_key(&key), SMOKE_EDIT_OPERATION, Some(&edit));

    let timed = run_update_timed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &ManualClock::new(),
        &hook,
    );
    let mut counters = WorkCounters::all_unknown();
    let attributed = run_update_attributed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &mut counters,
        &hook,
    );
    let memory = run_update_memory(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &NoMemoryReporter,
        &hook,
    );

    let key_full = key_for(OperationKind::FullParse, &old, None);
    let expected_full = null_checksum(&NullState {
        source_len_bytes: old.len_bytes() as u64,
        revision: 0,
    });
    let report_full = run_full_parse_timed(
        &mechanism,
        &old,
        &ManualClock::new(),
        &ScalarChecksumHook::new(expected_full),
    );

    let failing =
        NullMechanism::with_failure_mode(markit_mdbench_null_r1::FailureMode::Unsupported);
    let failing_state = markit_mdbench_null_r1::NullState {
        source_len_bytes: old.len_bytes() as u64,
        revision: 0,
    };
    let failed = run_update_timed(
        &failing,
        &old,
        &post,
        &edit,
        failing_state,
        &ManualClock::new(),
        &ScalarChecksumHook::new(0),
    );

    vec![
        assemble_row(&facts, &timed, &build, env, provenance),
        assemble_row(&facts, &attributed, &build, env, provenance),
        assemble_row(&facts, &memory, &build, env, provenance),
        assemble_row(
            &facts_for(CaseId::from_key(&key_full), OperationKind::FullParse, None),
            &report_full,
            &build,
            env,
            provenance,
        ),
        assemble_row(&facts, &failed, &build, env, provenance),
    ]
}

#[test]
fn rows_round_trip_through_jsonl() {
    for row in sample_rows() {
        let json = serde_json::to_string(&row).expect("serialize");
        let back: ResultRowV1 = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, row, "round trip must be lossless for {json}");
    }
}

#[test]
fn rows_validate_against_generated_schema_and_schema_has_not_drifted() {
    let schema_text = fs::read_to_string(schema_path()).unwrap_or_else(|err| {
        panic!(
            "protocol/result-schema-v2.json missing ({err}); regenerate with \
             `cargo run -p markit-mdbench-runner --bin mdbench-gen-schema -- protocol/result-schema-v2.json`"
        )
    });
    let checked_in: serde_json::Value = serde_json::from_str(&schema_text).expect("schema parses");
    let fresh = serde_json::to_value(schemars::schema_for!(ResultRowV1)).expect("schema generates");
    assert_eq!(
        checked_in, fresh,
        "checked-in result-schema-v2.json drifted from the Rust ResultRowV1 model; regenerate it"
    );

    let validator = jsonschema::validator_for(&checked_in).expect("schema compiles");
    for row in sample_rows() {
        let instance = serde_json::to_value(&row).expect("row serializes");
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        assert!(
            errors.is_empty(),
            "row not schema-valid: {errors:?}\n{instance}"
        );
    }
}

#[test]
fn invalid_lane_combinations_are_rejected() {
    let row = &sample_rows()[0];
    let mut timing_with_attribution_keys = serde_json::to_value(row).unwrap();
    timing_with_attribution_keys["measurement"]["metrics"]["blocks_reparsed"] =
        serde_json::json!(0);
    assert!(
        serde_json::from_value::<ResultRowV1>(timing_with_attribution_keys).is_err(),
        "timing metrics must reject attribution keys"
    );

    let mut attribution_with_timing_keys = serde_json::to_value(row).unwrap();
    attribution_with_timing_keys["measurement"]["metrics"] = serde_json::json!({
        "prepare_ns": 11, "native_ns": 23, "total_ns": 34
    });
    attribution_with_timing_keys["measurement"]["lane"] = serde_json::json!("attribution");
    assert!(
        serde_json::from_value::<ResultRowV1>(attribution_with_timing_keys).is_err(),
        "attribution metrics must reject timing keys"
    );

    let mut unknown_lane = serde_json::to_value(row).unwrap();
    unknown_lane["measurement"]["lane"] = serde_json::json!("composite");
    assert!(
        serde_json::from_value::<ResultRowV1>(unknown_lane).is_err(),
        "unknown lanes must be rejected; no composite headline lane may appear"
    );
}

// ---------------------------------------------------------------------------
// MAJOR-1: attribution covers prepare, events union via the common collector
// ---------------------------------------------------------------------------

/// Test mechanism that reports source-inspection events from BOTH the
/// prepare and the update phase, with overlapping/duplicate ranges.
struct InspectionProbe {
    fail_update: bool,
}

impl Mechanism for InspectionProbe {
    type State = ProbeState;
    type Prepared = ProbeState;
    type Pending = ProbeState;

    fn id(&self) -> MechanismId {
        MechanismId("__r1_inspection_probe_test_only__".to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        Ok(ProbeState(source.len_bytes() as u64))
    }

    fn prepare_update<W: WorkSink>(
        &self,
        _old: &Source,
        _post: &Source,
        _edit: &CanonicalEdit,
        old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        // Prepare-phase work MUST be attributable (R1-CORRECTIVE-1).
        // Prepare consults the OLD source: `Old` version events
        // (MEASUREMENT-CORRECTIVE-1 §18).
        cx.sink
            .record_source_inspection(SourceVersion::Old, 0, 10);
        cx.sink
            .record_source_inspection(SourceVersion::Old, 100, 200);
        Ok(*old_state)
    }

    fn update<W: WorkSink>(
        &self,
        _old: &Source,
        _post: &Source,
        _edit: &CanonicalEdit,
        old_state: Self::State,
        _prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        // Update reads the POST source: `Post` version events. The two
        // union to [0, 20); duplicates never double-count unique
        // coverage.
        cx.sink
            .record_source_inspection(SourceVersion::Post, 5, 20);
        cx.sink
            .record_source_inspection(SourceVersion::Post, 0, 10);
        if self.fail_update {
            Err(FailureStatus::Unsupported)
        } else {
            Ok(ProbeState(old_state.0 + 1))
        }
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        Ok(Completed { state: pending })
    }
}

#[test]
fn attribution_covers_prepare_and_unions_overlapping_inspections() {
    let (_key, edit, old, post) = smoke_case_key_and_edit();

    // A-LANE: both phases' events reach the counters; the union is common
    // arithmetic, so overlaps/duplicates never double-count.
    let mut counters = WorkCounters::all_unknown();
    let report = run_update_attributed(
        &InspectionProbe { fail_update: false },
        &old,
        &post,
        &edit,
        ProbeState(0),
        &mut counters,
        &ScalarChecksumHook::new(1),
    );
    assert_eq!(report.execution_status, ExecutionStatus::Pass);
    // Per-version derivation (ATTRIBUTION-SCHEMA-v2): the OLD and POST
    // unions are kept APART, the combined primary quantity is their SUM,
    // and the cumulative total counts every event (repeats included).
    assert_eq!(counters.unique_old_source_intervals, Observed::Known(2));
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(110));
    assert_eq!(counters.unique_post_source_intervals, Observed::Known(1));
    assert_eq!(counters.unique_post_source_bytes, Observed::Known(20));
    assert_eq!(
        counters.unique_source_intervals,
        Observed::Known(3),
        "combined = old union + post union, never a cross-version union"
    );
    assert_eq!(counters.unique_source_bytes, Observed::Known(130));
    assert_eq!(
        counters.source_bytes_inspected_total,
        Observed::Known(135),
        "cumulative effort: all four events count, overlaps/duplicates included"
    );

    // The same mechanism runs in the T-LANE with the no-op sink: the
    // inspection events cost nothing and the run still passes.
    let timed = run_update_timed(
        &InspectionProbe { fail_update: false },
        &old,
        &post,
        &edit,
        ProbeState(0),
        &ManualClock::new(),
        &ScalarChecksumHook::new(1),
    );
    assert_eq!(timed.execution_status, ExecutionStatus::Pass);
    assert!(matches!(timed.measurement, LaneMeasurement::Timing(_)));

    // A run that did not complete never finalizes: the derived slots stay
    // Unknown (a crash must not masquerade as a measured zero).
    let mut counters = WorkCounters::all_unknown();
    let failed = run_update_attributed(
        &InspectionProbe { fail_update: true },
        &old,
        &post,
        &edit,
        ProbeState(0),
        &mut counters,
        &ScalarChecksumHook::new(0),
    );
    assert_eq!(failed.execution_status, ExecutionStatus::Unsupported);
    assert_eq!(
        counters.unique_source_bytes,
        Observed::Unknown,
        "underived slots must stay Unknown on failed runs"
    );
    assert_eq!(counters.source_bytes_inspected_total, Observed::Unknown);
}

// ---------------------------------------------------------------------------
// MAJOR-2: M-LANE per-case windows
// ---------------------------------------------------------------------------

/// Test reporter that records the begin/run/end sequence.
struct RecordingReporter {
    events: RefCell<Vec<&'static str>>,
}

impl MemoryReporter for RecordingReporter {
    fn begin_case(&self) -> CaseMemoryProbe {
        self.events.borrow_mut().push("begin_case");
        CaseMemoryProbe::new(1)
    }

    fn end_case(&self, _probe: CaseMemoryProbe) -> MemoryRecord {
        self.events.borrow_mut().push("end_case");
        MemoryRecord::unavailable()
    }
}

/// Test mechanism that records its own phase into the shared event log.
struct OrderProbe<'a> {
    events: &'a RefCell<Vec<&'static str>>,
}

impl Mechanism for OrderProbe<'_> {
    type State = ProbeState;
    type Prepared = ProbeState;
    type Pending = ProbeState;

    fn id(&self) -> MechanismId {
        MechanismId("__r1_order_probe_test_only__".to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        Ok(ProbeState(source.len_bytes() as u64))
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
        self.events.borrow_mut().push("mechanism");
        Ok(ProbeState(old_state.0 + 1))
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        Ok(Completed { state: pending })
    }
}

#[test]
fn memory_lane_opens_and_closes_one_window_around_the_mechanism() {
    let (_key, edit, old, post) = smoke_case_key_and_edit();
    let reporter = RecordingReporter {
        events: RefCell::new(Vec::new()),
    };
    let probe = OrderProbe {
        events: &reporter.events,
    };
    let report = run_update_memory(
        &probe,
        &old,
        &post,
        &edit,
        ProbeState(0),
        &reporter,
        &ScalarChecksumHook::new(1),
    );
    assert_eq!(report.execution_status, ExecutionStatus::Pass);
    assert_eq!(
        *reporter.events.borrow(),
        ["begin_case", "mechanism", "end_case"],
        "the M-LANE window must span exactly the mechanism phases"
    );
}

/// Test mechanism that observes allocations through a shared
/// [`ManualMemoryReporter`] while the runner holds the window open.
struct MemoryObservingProbe<'a> {
    reporter: &'a ManualMemoryReporter,
    bytes: u64,
}

impl Mechanism for MemoryObservingProbe<'_> {
    type State = ProbeState;
    type Prepared = ProbeState;
    type Pending = ProbeState;

    fn id(&self) -> MechanismId {
        MechanismId("__r1_memory_probe_test_only__".to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        Ok(ProbeState(source.len_bytes() as u64))
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
        self.reporter.observe_allocation(self.bytes);
        self.reporter.observe_peak(self.bytes);
        self.reporter.observe_retained(self.bytes);
        Ok(ProbeState(old_state.0 + 1))
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        Ok(Completed { state: pending })
    }
}

#[test]
fn memory_lane_values_are_per_case_and_never_leak_across_cases() {
    let (_key, edit, old, post) = smoke_case_key_and_edit();
    let reporter = ManualMemoryReporter::new();

    let first = run_update_memory(
        &MemoryObservingProbe {
            reporter: &reporter,
            bytes: 60,
        },
        &old,
        &post,
        &edit,
        ProbeState(0),
        &reporter,
        &ScalarChecksumHook::new(1),
    );
    let second = run_update_memory(
        &MemoryObservingProbe {
            reporter: &reporter,
            bytes: 7,
        },
        &old,
        &post,
        &edit,
        ProbeState(0),
        &reporter,
        &ScalarChecksumHook::new(1),
    );

    let record = |report: &markit_mdbench_runner::RunReport| match &report.measurement {
        LaneMeasurement::Memory(m) => *m,
        other => panic!("M-LANE run produced {other:?}"),
    };
    let first = record(&first);
    let second = record(&second);

    // Case 2 is a fresh window: nothing from case 1 may leak into it.
    assert_eq!(first.allocated_bytes, Observed::Known(60));
    assert_eq!(first.peak_bytes, Observed::Known(60));
    assert_eq!(first.retained_bytes, Observed::Known(60));
    assert_eq!(second.allocated_bytes, Observed::Known(7));
    assert_eq!(second.allocation_count, Observed::Known(1));
    assert_eq!(second.retained_bytes, Observed::Known(7));
    assert_ne!(first, second);

    // Peak and retained are separate slots in the record schema.
    let json = serde_json::to_value(second).unwrap();
    assert!(json.get("peak_bytes").is_some());
    assert!(json.get("retained_bytes").is_some());
    assert!(json.to_string().find("peak_retained_bytes").is_none());
}

// ---------------------------------------------------------------------------
// MAJOR-4: edit metadata uses the operation contract, not raw hashing
// ---------------------------------------------------------------------------

#[test]
fn edit_meta_matches_operation_contract_for_every_operation() {
    let empty_insert = CanonicalEdit::new(3, 3, "").unwrap();
    let text_insert = CanonicalEdit::new(3, 3, "abc").unwrap();
    let delete = CanonicalEdit::new(3, 7, "").unwrap();
    let sha256 = |text: &str| {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        markit_mdbench_common::to_lower_hex(&digest)
    };

    // (operation, edit, expected range, expected inserted digest)
    let cases = [
        (OperationKind::FullParse, None, None, None),
        (OperationKind::Query, None, None, None),
        (OperationKind::Delete, Some(delete), Some((3, 7)), None),
        (
            OperationKind::Insert,
            Some(text_insert.clone()),
            Some((3, 3)),
            Some(sha256("abc")),
        ),
        // REPLACE_EQ 0->0 carries an EMPTY insertion: the digest of the
        // empty string is present, exactly like CaseKeyV1.
        (
            OperationKind::ReplaceEq,
            Some(empty_insert),
            Some((3, 3)),
            Some(sha256("")),
        ),
        (
            OperationKind::ReplaceGrow,
            Some(text_insert.clone()),
            Some((3, 3)),
            Some(sha256("abc")),
        ),
        (
            OperationKind::ReplaceShrink,
            Some(text_insert.clone()),
            Some((3, 3)),
            Some(sha256("abc")),
        ),
        (
            OperationKind::StructuralEdit,
            Some(text_insert),
            Some((3, 3)),
            Some(sha256("abc")),
        ),
    ];
    for (operation, edit, expected_range, expected_digest) in cases {
        let meta = edit_meta(operation, edit.as_ref()).unwrap_or_else(|err| {
            panic!(
                "{} must accept its canonical edit: {err:?}",
                operation.canonical_name()
            )
        });
        assert_eq!(
            meta.inserted_sha256,
            expected_digest,
            "{} inserted digest facts",
            operation.canonical_name()
        );
        match expected_range {
            None => {
                assert!(
                    meta.start_byte.is_none() && meta.end_byte.is_none(),
                    "{} must carry no range",
                    operation.canonical_name()
                );
            }
            Some((start, end)) => {
                assert_eq!(meta.start_byte, Some(start));
                assert_eq!(meta.end_byte, Some(end));
            }
        }
    }

    // DELETE declares no insertion: the digest is null even though a
    // CanonicalEdit always carries a (possibly empty) string. DELETE
    // result facts agree with DELETE case-key facts.
    let delete = CanonicalEdit::new(3, 7, "").unwrap();
    assert_eq!(
        edit_meta(OperationKind::Delete, Some(&delete))
            .unwrap()
            .inserted_sha256,
        None
    );
}

#[test]
fn edit_meta_rejects_contradictory_combinations() {
    let (_, edit, _) = smoke_fixture();
    let empty = CanonicalEdit::new(0, 0, "").unwrap();

    // Edit-bearing operation without an edit.
    assert_eq!(
        edit_meta(OperationKind::Insert, None),
        Err(markit_mdbench_runner::EditMetaError::MissingEdit)
    );
    assert_eq!(
        edit_meta(OperationKind::StructuralEdit, None),
        Err(markit_mdbench_runner::EditMetaError::MissingEdit)
    );
    // No-edit operation with an edit.
    assert_eq!(
        edit_meta(OperationKind::FullParse, Some(&edit)),
        Err(markit_mdbench_runner::EditMetaError::UnexpectedEdit)
    );
    assert_eq!(
        edit_meta(OperationKind::Query, Some(&edit)),
        Err(markit_mdbench_runner::EditMetaError::UnexpectedEdit)
    );
    // DELETE carrying inserted text would contradict the case-key facts.
    assert_eq!(
        edit_meta(OperationKind::Delete, Some(&edit)),
        Err(markit_mdbench_runner::EditMetaError::UnexpectedInsertedText)
    );
    // An EMPTY insertion is NOT "carrying inserted text": a canonical
    // DELETE is accepted.
    assert!(edit_meta(OperationKind::Delete, Some(&empty)).is_ok());
}

// ---------------------------------------------------------------------------
// substrate guards
// ---------------------------------------------------------------------------

#[test]
fn no_horse_implementations_exist() {
    for dir in [
        "full-rebuild",
        "block-local",
        "fragment-reuse",
        "old-tree-subtree-reuse",
        "restart-convergence",
    ] {
        let horse_dir = workspace_root().join("mechanisms").join(dir);
        let rust_sources = fs::read_dir(&horse_dir)
            .unwrap_or_else(|err| panic!("reserved dir {dir} missing: {err}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .count();
        assert_eq!(
            rust_sources, 0,
            "reserved horse directory {dir} must contain no Rust sources in R1"
        );
    }
}

#[test]
fn schema_has_no_conclusion_fields() {
    // Raw rows preserve facts only: no winner/rank/score/speedup fields
    // may ever appear in the schema text.
    let schema_text = serde_json::to_string(&schemars::schema_for!(ResultRowV1)).unwrap();
    for banned in ["winner", "rank", "score", "speedup", "quality"] {
        assert!(
            !schema_text.contains(banned),
            "result schema must not contain conclusion field {banned}"
        );
    }
}
