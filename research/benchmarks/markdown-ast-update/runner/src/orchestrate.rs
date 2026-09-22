//! Case orchestration with frozen timer boundaries and lane separation.
//!
//! Timer placement rules enforced here (R0 §7, as corrected by
//! MEASUREMENT-CORRECTIVE-1):
//!
//! - the runner is the only clock caller; each phase reads the clock
//!   exactly twice (start/stop);
//! - `prepare_update` runs only inside `T_prepare`;
//! - `update` + `complete` + `black_box(completed)` run only inside
//!   `T_native` — `complete()` is the explicit native-sealing AUTHORITY
//!   boundary (see `markit_mdbench_common::Mechanism` for its precise,
//!   non-overclaimed meaning);
//! - the timer STOPS once a valid eager native state exists and is
//!   `black_box`ed. Everything after that is experiment
//!   verification/export: the `NormalizeV1` projection, its validation,
//!   the deterministic result checksum, and the oracle hook all run
//!   strictly after every timer has stopped, via the state's frozen
//!   [`markit_mdbench_common::ResultChecksum`] export and the hook. No
//!   parsing may be deferred past the stop, and no mechanism-required
//!   state/index construction may move outside timing
//!   (MEASUREMENT-CORRECTIVE-1 §7/§8/§11);
//! - `T_total` is always the arithmetic sum from [`TimingRecord`], never
//!   an enclosing wall-clock measurement;
//! - panics inside a mechanism phase are caught and recorded as
//!   `ExecutionStatus::Crash`. In-process catching covers UNWIND panics
//!   only; OOM / stack overflow / abort / fatal signals / hangs kill the
//!   process, so R1-CORRECTIVE-1 adds a process-level supervisor
//!   (`crate::supervisor`) whose worker boundary classifies those deaths
//!   and keeps the failure row alive.
//!
//! Lane rules (R1-CORRECTIVE-1):
//!
//! - T-LANE passes [`NoopWorkSink`] to EVERY phase — zero attribution
//!   cost, zero timing cost;
//! - A-LANE passes one [`CounterSink`] to prepare AND update, so
//!   mechanism-owned preparation work cannot appear in `T_prepare` and
//!   then vanish from attribution; the derived `unique_source_*` slots
//!   come from the common union collector only;
//! - M-LANE wraps exactly one begin_case/end_case window around the
//!   mechanism phases.

use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::Completed;
use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::CounterSink;
use markit_mdbench_common::ExecutionStatus;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::NoopWorkSink;
use markit_mdbench_common::ResultChecksum;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkCounters;
use markit_mdbench_instrumentation::Clock;
use markit_mdbench_instrumentation::LaneMeasurement;
use markit_mdbench_instrumentation::MemoryReporter;
use markit_mdbench_instrumentation::PhaseGuard;
use markit_mdbench_instrumentation::TimingError;
use markit_mdbench_instrumentation::TimingRecord;
use markit_mdbench_oracle::CorrectnessHook;

/// Result of one case run in one lane. Facts only — no ranking, no
/// score, no speedup conclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub execution_status: ExecutionStatus,
    pub correctness_status: CorrectnessStatus,
    pub measurement: LaneMeasurement,
    /// Deterministic checksum of the completed result, when the run
    /// completed. Verified outside every timer.
    pub result_checksum: Option<u64>,
    /// Mechanism/instrumentation failure, when any. `None` on pass.
    pub failure: Option<FailureStatus>,
}

fn catch_phase<T>(phase: impl FnOnce() -> Result<T, FailureStatus>) -> Result<T, FailureStatus> {
    match catch_unwind(AssertUnwindSafe(phase)) {
        Ok(result) => result,
        Err(_) => Err(FailureStatus::Crash),
    }
}

// ---------------------------------------------------------------------------
// Correctness-only (dry-run) lane — CORRECTIVE-C A8
// ---------------------------------------------------------------------------

/// Result of one correctness-only case run: execution + correctness facts,
/// and deliberately NOTHING else.
///
/// This is the harness's explicit correctness-only/dry-run mode (issue #35
/// CORRECTIVE-C §A8 / G7): the same mechanism phases and the same oracle
/// authority as the measured lanes, with no clock call and no
/// `LaneMeasurement` of any kind — not timing, not work counters, not
/// memory. Dry-run evidence therefore cannot be mistaken for (or converted
/// into) performance evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectnessReport {
    pub execution_status: ExecutionStatus,
    pub correctness_status: CorrectnessStatus,
    /// Deterministic checksum of the completed result, when the run
    /// completed. Verified outside every timer — there are no timers here.
    pub result_checksum: Option<u64>,
    /// Mechanism failure, when any. `None` on pass.
    pub failure: Option<FailureStatus>,
}

fn finish_correctness<S: ResultChecksum>(
    outcome: Result<Completed<S>, FailureStatus>,
    hook: &dyn CorrectnessHook<S>,
) -> CorrectnessReport {
    match outcome {
        Ok(done) => {
            // Post-timer experiment export: the checksum is derived from
            // the sealed state, never inside a mechanism phase.
            let checksum = done.state.result_checksum();
            CorrectnessReport {
                execution_status: ExecutionStatus::Pass,
                correctness_status: hook.verify(&done),
                result_checksum: Some(checksum),
                failure: None,
            }
        }
        Err(failure) => CorrectnessReport {
            execution_status: failure.into(),
            correctness_status: CorrectnessStatus::NotChecked,
            result_checksum: None,
            failure: Some(failure),
        },
    }
}

/// UPDATE case, correctness-only: run the mechanism's update path once and
/// verify it against the hook. No clock is constructed or consulted; no
/// measurement value exists.
pub fn run_update_correctness<M>(
    mechanism: &M,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    hook: &dyn CorrectnessHook<M::State>,
) -> CorrectnessReport
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let (old, post, edit) = (
        black_box(old_source),
        black_box(post_source),
        black_box(edit),
    );
    let outcome = {
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        catch_phase(|| mechanism.prepare_update(old, post, edit, &old_state, &mut cx)).and_then(
            |prep| {
                catch_phase(|| {
                    let pending = mechanism.update(old, post, edit, old_state, prep, &mut cx)?;
                    let done = mechanism.complete(pending)?;
                    black_box(&done);
                    Ok(done)
                })
            },
        )
    };
    finish_correctness(outcome, hook)
}

/// FULL_PARSE case, correctness-only: clean parse + complete + verify. No
/// clock, no measurement value.
pub fn run_full_parse_correctness<M>(
    mechanism: &M,
    source: &Source,
    hook: &dyn CorrectnessHook<M::State>,
) -> CorrectnessReport
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let source = black_box(source);
    let outcome = {
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        catch_phase(|| {
            let pending = mechanism.full_parse::<NoopWorkSink>(source, &mut cx)?;
            let done = mechanism.complete(pending)?;
            black_box(&done);
            Ok(done)
        })
    };
    finish_correctness(outcome, hook)
}

/// Successful/failed execution + materialized timing record -> report.
/// The oracle hook and the result-checksum export (called here for
/// completed runs) run strictly AFTER all timers stopped
/// (MEASUREMENT-CORRECTIVE-1).
fn finish_timed<S: ResultChecksum>(
    outcome: Result<Completed<S>, FailureStatus>,
    record: TimingRecord,
    hook: &dyn CorrectnessHook<S>,
) -> RunReport {
    let measurement = LaneMeasurement::Timing(record);
    match outcome {
        Ok(done) => {
            let checksum = done.state.result_checksum();
            RunReport {
                execution_status: ExecutionStatus::Pass,
                correctness_status: hook.verify(&done),
                measurement,
                result_checksum: Some(checksum),
                failure: None,
            }
        }
        Err(failure) => RunReport {
            execution_status: failure.into(),
            correctness_status: CorrectnessStatus::NotChecked,
            measurement,
            result_checksum: None,
            failure: Some(failure),
        },
    }
}

/// Timing arithmetic overflow poisons the run even if the work
/// completed: the number cannot be represented honestly, so the row
/// records `InstrumentationUnavailable` with `Unknown` timing.
fn finish_timing_overflow<S: ResultChecksum>(
    outcome: Result<Completed<S>, FailureStatus>,
) -> RunReport {
    let checksum = outcome
        .as_ref()
        .ok()
        .map(|done| done.state.result_checksum());
    RunReport {
        execution_status: ExecutionStatus::InstrumentationUnavailable,
        correctness_status: CorrectnessStatus::NotChecked,
        measurement: LaneMeasurement::Timing(TimingRecord::unknown()),
        result_checksum: checksum,
        failure: Some(FailureStatus::InstrumentationUnavailable),
    }
}

fn finish_untimed<S: ResultChecksum>(
    outcome: Result<Completed<S>, FailureStatus>,
    measurement: LaneMeasurement,
    hook: &dyn CorrectnessHook<S>,
) -> RunReport {
    match outcome {
        Ok(done) => {
            let checksum = done.state.result_checksum();
            RunReport {
                execution_status: ExecutionStatus::Pass,
                correctness_status: hook.verify(&done),
                measurement,
                result_checksum: Some(checksum),
                failure: None,
            }
        }
        Err(failure) => RunReport {
            execution_status: failure.into(),
            correctness_status: CorrectnessStatus::NotChecked,
            measurement,
            result_checksum: None,
            failure: Some(failure),
        },
    }
}

/// Construct the retained old-state for an update case. Per R0 §7.2 this
/// happens BEFORE the edit case, outside every timer. Panics in either
/// phase are caught here too, so initial-state construction can never
/// take the case down with an unwind.
pub fn build_initial_state<M: Mechanism>(
    mechanism: &M,
    source: &Source,
) -> Result<M::State, FailureStatus> {
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let pending = catch_phase(|| mechanism.full_parse::<NoopWorkSink>(source, &mut cx))?;
    let completed = catch_phase(|| mechanism.complete(pending))?;
    Ok(completed.state)
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

/// UPDATE case, T-LANE: real timer, `NoopWorkSink` in every phase, no
/// memory collector.
pub fn run_update_timed<M, C>(
    mechanism: &M,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    clock: &C,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
    C: Clock,
{
    let (old, post, edit) = (
        black_box(old_source),
        black_box(post_source),
        black_box(edit),
    );
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);

    let prepare_guard = PhaseGuard::start(clock);
    let prepared = catch_phase(|| mechanism.prepare_update(old, post, edit, &old_state, &mut cx));
    let prepare_ns = prepare_guard.stop();

    let prep = match prepared {
        // Failed prepare: the lane is kept, the values stay Unknown.
        Err(failure) => return finish_timed(Err(failure), TimingRecord::unknown(), hook),
        Ok(prep) => prep,
    };

    let native_guard = PhaseGuard::start(clock);
    let outcome = catch_phase(|| {
        let pending = mechanism.update(old, post, edit, old_state, prep, &mut cx)?;
        let done = mechanism.complete(pending)?;
        black_box(&done);
        Ok(done)
    });
    let native_ns = native_guard.stop();

    match TimingRecord::update(prepare_ns, native_ns) {
        Ok(record) => finish_timed(outcome, record, hook),
        Err(TimingError::Overflow) => finish_timing_overflow(outcome),
    }
}

/// UPDATE case, A-LANE: work counters collected from prepare AND update,
/// no headline `TimingRecord`, no timing at all.
///
/// On a completed run the common collector derives the
/// `unique_source_*` slots from the recorded inspection events; a failed
/// run skips the derivation, so the derived slots stay `Unknown`.
pub fn run_update_attributed<M>(
    mechanism: &M,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    counters: &mut WorkCounters,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let (old, post, edit) = (
        black_box(old_source),
        black_box(post_source),
        black_box(edit),
    );
    let outcome = {
        let mut sink = CounterSink::new(counters);
        let mut cx = MechanismContext::new(&mut sink);
        let outcome =
            catch_phase(|| mechanism.prepare_update(old, post, edit, &old_state, &mut cx))
                .and_then(|prep| {
                    catch_phase(|| {
                        let pending =
                            mechanism.update(old, post, edit, old_state, prep, &mut cx)?;
                        let done = mechanism.complete(pending)?;
                        black_box(&done);
                        Ok(done)
                    })
                });
        // The union/derivation is common-layer arithmetic over the event
        // stream — never mechanism-authored — and only finalizes when the
        // work actually completed.
        if outcome.is_ok() {
            sink.finalize_derived();
        }
        outcome
    };
    finish_untimed(
        outcome,
        LaneMeasurement::Attribution(counters.clone()),
        hook,
    )
}

/// UPDATE case, M-LANE: exactly one begin_case/end_case window around the
/// mechanism phases; the window closes even on failure. No headline
/// `TimingRecord`.
pub fn run_update_memory<M, R>(
    mechanism: &M,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    reporter: &R,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
    R: MemoryReporter,
{
    let (old, post, edit) = (
        black_box(old_source),
        black_box(post_source),
        black_box(edit),
    );
    let probe = reporter.begin_case();
    let outcome = catch_phase(|| {
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        mechanism
            .prepare_update(old, post, edit, &old_state, &mut cx)
            .and_then(|prep| {
                let pending = mechanism.update(old, post, edit, old_state, prep, &mut cx)?;
                let done = mechanism.complete(pending)?;
                black_box(&done);
                Ok(done)
            })
    });
    let record = reporter.end_case(probe);
    finish_untimed(outcome, LaneMeasurement::Memory(record), hook)
}

// ---------------------------------------------------------------------------
// FULL_PARSE
// ---------------------------------------------------------------------------

/// FULL_PARSE case, T-LANE. `T_prepare` is `NotApplicable`; `T_total ==
/// T_native` by documented arithmetic identity.
pub fn run_full_parse_timed<M, C>(
    mechanism: &M,
    source: &Source,
    clock: &C,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
    C: Clock,
{
    let source = black_box(source);
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);

    let native_guard = PhaseGuard::start(clock);
    let outcome = catch_phase(|| {
        let pending = mechanism.full_parse::<NoopWorkSink>(source, &mut cx)?;
        let done = mechanism.complete(pending)?;
        black_box(&done);
        Ok(done)
    });
    let native_ns = native_guard.stop();
    // No arithmetic sum exists for FULL_PARSE, so no overflow path.
    finish_timed(outcome, TimingRecord::full_parse(native_ns), hook)
}

/// FULL_PARSE case, A-LANE.
pub fn run_full_parse_attributed<M>(
    mechanism: &M,
    source: &Source,
    counters: &mut WorkCounters,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let source = black_box(source);
    let outcome = {
        let mut sink = CounterSink::new(counters);
        let mut cx = MechanismContext::new(&mut sink);
        let outcome = catch_phase(|| {
            let pending = mechanism.full_parse::<CounterSink>(source, &mut cx)?;
            let done = mechanism.complete(pending)?;
            black_box(&done);
            Ok(done)
        });
        if outcome.is_ok() {
            sink.finalize_derived();
        }
        outcome
    };
    finish_untimed(
        outcome,
        LaneMeasurement::Attribution(counters.clone()),
        hook,
    )
}

/// FULL_PARSE case, M-LANE: one per-case window around the phases.
pub fn run_full_parse_memory<M, R>(
    mechanism: &M,
    source: &Source,
    reporter: &R,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunReport
where
    M: Mechanism,
    M::State: ResultChecksum,
    R: MemoryReporter,
{
    let source = black_box(source);
    let probe = reporter.begin_case();
    let outcome = catch_phase(|| {
        let mut sink = NoopWorkSink;
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mechanism.full_parse::<NoopWorkSink>(source, &mut cx)?;
        let done = mechanism.complete(pending)?;
        black_box(&done);
        Ok(done)
    });
    let record = reporter.end_case(probe);
    finish_untimed(outcome, LaneMeasurement::Memory(record), hook)
}
