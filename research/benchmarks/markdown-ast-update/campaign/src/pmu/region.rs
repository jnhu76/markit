//! The PMU measurement region — the ONLY place counters are enabled.
//!
//! Region contract (task §9/§11 of the PMU campaign order): the counted
//! window must contain exactly the mechanism work the primary campaign's
//! `T_total = T_prepare + T_native` contains, and nothing else. This is
//! a line-for-line mirror of the runner's frozen phase composition
//! (`runner/src/orchestrate.rs::run_update_timed` / the campaign
//! executor's cell loop) with the two `PhaseGuard` clocks replaced by
//! the counter group's begin/end:
//!
//! ```text
//! CLEAN_STATE, inside counters:  full_parse -> complete -> black_box
//! EDIT_WRITE,  inside counters:  prepare_update ->
//!                                 update -> complete -> black_box
//! EDIT_WRITE, outside counters:  fresh pre-state construction
//!                                (SINGLE_RESET: build_initial_state)
//! BOTH,        outside counters: oracle hook verify, result checksum
//! ```
//!
//! The work sink is the T-LANE `NoopWorkSink` (zero attribution cost,
//! zero timing cost) — PMU observes the same phase shapes the primary
//! timing lane timed. Panics inside a phase are caught exactly like the
//! runner catches them (`Crash`), and the counters are ALWAYS disabled
//! before the correctness work runs.

use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::{
    Completed, CorrectnessStatus, ExecutionStatus, FailureStatus, Mechanism, NoopWorkSink,
    ResultChecksum, Source,
};
use markit_mdbench_oracle::CorrectnessHook;

use super::perfcount::EventCounterGroup;

/// What one region execution produced (counters + correctness facts).
pub struct RegionOutcome<S> {
    pub counts: Result<Vec<super::perfcount::EventReading>, String>,
    pub execution_status: ExecutionStatus,
    pub correctness_status: CorrectnessStatus,
    pub result_checksum: Option<u64>,
    pub failure: Option<FailureStatus>,
    pub completed: Option<Completed<S>>,
}

/// Consolidated counter-window bookkeeping: a failed begin poisons the
/// counts (the region still ran and its correctness is still verified);
/// otherwise the end read decides. The disable in `end` runs even after
/// a failed begin — a failed begin never enabled the counters.
fn window_counts(
    begin_error: Option<String>,
    group: &mut EventCounterGroup,
) -> Result<Vec<super::perfcount::EventReading>, String> {
    match (begin_error, group.end()) {
        (Some(error), _) => Err(format!("counter begin failed: {error}")),
        (None, Ok(readings)) => Ok(readings),
        (None, Err(error)) => Err(format!("counter end failed: {error}")),
    }
}

fn catch_phase<T>(phase: impl FnOnce() -> Result<T, FailureStatus>) -> Result<T, FailureStatus> {
    match catch_unwind(AssertUnwindSafe(phase)) {
        Ok(result) => result,
        Err(_) => Err(FailureStatus::Crash),
    }
}

/// CLEAN_STATE region: `full_parse -> complete -> black_box` inside the
/// counter window; correctness strictly outside it.
pub fn run_clean_state_region<M>(
    mechanism: &M,
    source: &Source,
    group: &mut EventCounterGroup,
    hook: &dyn CorrectnessHook<M::State>,
) -> RegionOutcome<M::State>
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let source = black_box(source);
    let mut sink = NoopWorkSink;
    let mut cx = markit_mdbench_common::MechanismContext::new(&mut sink);
    let begin_error = group.begin().err();
    let outcome = {
        catch_phase(|| {
            let pending = mechanism.full_parse::<NoopWorkSink>(source, &mut cx)?;
            let done = mechanism.complete(pending)?;
            black_box(&done);
            Ok(done)
        })
    };
    let counts = window_counts(begin_error, group);
    finish(outcome, counts, hook)
}

/// EDIT_WRITE region: SINGLE_RESET pre-state OUTSIDE the window, then
/// `prepare_update -> update -> complete -> black_box` inside it,
/// correctness strictly outside.
pub fn run_edit_write_region<M>(
    mechanism: &M,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    group: &mut EventCounterGroup,
    hook: &dyn CorrectnessHook<M::State>,
) -> RegionOutcome<M::State>
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let (pre, post, edit) = (black_box(pre), black_box(post), black_box(edit));
    let mut sink = NoopWorkSink;
    let mut cx = markit_mdbench_common::MechanismContext::new(&mut sink);
    let begin_error = group.begin().err();
    let outcome = {
        let prepared =
            catch_phase(|| mechanism.prepare_update(pre, post, edit, &old_state, &mut cx));
        prepared.and_then(|prep| {
            catch_phase(|| {
                let pending = mechanism.update(pre, post, edit, old_state, prep, &mut cx)?;
                let done = mechanism.complete(pending)?;
                black_box(&done);
                Ok(done)
            })
        })
    };
    let counts = window_counts(begin_error, group);
    finish(outcome, counts, hook)
}

fn finish<S: ResultChecksum>(
    outcome: Result<Completed<S>, FailureStatus>,
    counts: Result<Vec<super::perfcount::EventReading>, String>,
    hook: &dyn CorrectnessHook<S>,
) -> RegionOutcome<S> {
    // Correctness is verified strictly AFTER the counters stopped
    // (task §9: oracle outside the PMU region).
    match outcome {
        Ok(done) => {
            let checksum = done.state.result_checksum();
            RegionOutcome {
                counts,
                execution_status: ExecutionStatus::Pass,
                correctness_status: hook.verify(&done),
                result_checksum: Some(checksum),
                failure: None,
                completed: Some(done),
            }
        }
        Err(failure) => RegionOutcome {
            counts,
            execution_status: failure.into(),
            correctness_status: CorrectnessStatus::NotChecked,
            result_checksum: None,
            failure: Some(failure),
            completed: None,
        },
    }
}
