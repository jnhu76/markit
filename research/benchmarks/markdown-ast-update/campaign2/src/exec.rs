//! Campaign-2 execution: the four surfaces, all dispatched through the
//! FROZEN `markit_mdbench_runner` phase functions (task §13-§17, §27).
//!
//! Nothing here re-implements a horse. The only new timed function is
//! [`run_update_chain_step`], which Surface C needs because the frozen
//! `run_update_timed` consumes the old state and does not hand the NEW
//! state back; the chained step uses the IDENTICAL timer boundary
//! (`T_prepare` around `prepare_update`, `T_native` around
//! `update` + `complete` + `black_box`, oracle strictly after both
//! timers). `chain_step_matches_frozen_runner` in `tests/` proves the
//! two paths agree on execution status, correctness status and result
//! checksum for the same inputs.

use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

use markit_mdbench_common::{
    CanonicalEdit, CaseId, Completed, CorrectnessStatus, CounterSink, ExecutionStatus,
    FailureStatus, Mechanism, MechanismContext, NoopWorkSink, OperationKind, ResultChecksum,
    Source, SourceId, WorkCounters,
};
use markit_mdbench_instrumentation::{Clock, LaneMeasurement, PhaseGuard, TimingRecord};
use markit_mdbench_oracle::{validate_normalized, CorrectnessHook, NormalizeV1, ReferenceOracle};
use markit_mdbench_runner::orchestrate::{build_initial_state, run_full_parse_timed, run_update_timed};
use markit_mdbench_runner::{assemble_row, failure_row, CaseFacts, PayloadMetaV1, ResultRowV1};

use crate::envelope::{ExecutionIdentity2, StateReprV1};

/// Dispatch a horse label to its concrete mechanism type.
///
/// The body is monomorphized once per horse, exactly like the frozen
/// Campaign-1 executor's explicit match (there is no `dyn Mechanism`).
#[macro_export]
macro_rules! with_horse {
    ($horse_id:expr, |$m:ident| $body:expr) => {
        match $horse_id {
            "H0" => {
                let $m = markit_mdbench_full_rebuild::FullRebuildMechanism::new();
                $body
            }
            "H1" => {
                let $m = markit_mdbench_block_local::BlockLocalMechanism::new();
                $body
            }
            "H2" => {
                let $m = markit_mdbench_fragment_reuse::FragmentReuseMechanism::new();
                $body
            }
            "H3" => {
                let $m = markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new();
                $body
            }
            "H4" => {
                let $m = markit_mdbench_restart_convergence::RestartConvergenceMechanism::new();
                $body
            }
            other => Err(format!("unknown horse {other:?}")),
        }
    };
}

/// Descriptive state-representation export for one sealed state.
///
/// Every field is a pure read of already-sealed state (task §29): no
/// parsing, no index work, no mechanism work. Fields a horse does not
/// expose stay `UNAVAILABLE` — never a fabricated zero.
pub trait StateReprExport {
    fn state_repr(&self) -> StateReprV1;
}

impl StateReprExport for markit_mdbench_full_rebuild::H0State {
    fn state_repr(&self) -> StateReprV1 {
        StateReprV1 {
            retained_blocks: self.node_count().to_string(),
            checkpoints: "UNAVAILABLE".to_string(),
            fragment_metadata_entries: "UNAVAILABLE".to_string(),
            old_tree_index_entries: "UNAVAILABLE".to_string(),
            retained_source_bytes: self.source_len_bytes.to_string(),
            provenance: "H0State::node_count + source_len_bytes (post-timer pure read)"
                .to_string(),
        }
    }
}

impl StateReprExport for markit_mdbench_block_local::H1State {
    fn state_repr(&self) -> StateReprV1 {
        StateReprV1 {
            retained_blocks: self.entries().len().to_string(),
            checkpoints: "UNAVAILABLE".to_string(),
            fragment_metadata_entries: self.definitions().len().to_string(),
            old_tree_index_entries: "UNAVAILABLE".to_string(),
            retained_source_bytes: self.source_len_bytes().to_string(),
            provenance:
                "H1State::entries/definitions/source_len_bytes (post-timer pure read)"
                    .to_string(),
        }
    }
}

impl StateReprExport for markit_mdbench_fragment_reuse::H2State {
    fn state_repr(&self) -> StateReprV1 {
        StateReprV1 {
            retained_blocks: self.tree().node_count().to_string(),
            checkpoints: "UNAVAILABLE".to_string(),
            fragment_metadata_entries: self.fragment_count().to_string(),
            old_tree_index_entries: "UNAVAILABLE".to_string(),
            retained_source_bytes: "UNAVAILABLE".to_string(),
            provenance: "H2State::tree().node_count + fragment_count (post-timer pure read)"
                .to_string(),
        }
    }
}

impl StateReprExport for markit_mdbench_old_tree_subtree_reuse::H3State {
    fn state_repr(&self) -> StateReprV1 {
        StateReprV1 {
            retained_blocks: self.tree().node_count().to_string(),
            checkpoints: "UNAVAILABLE".to_string(),
            fragment_metadata_entries: "UNAVAILABLE".to_string(),
            old_tree_index_entries: self.tree().node_count().to_string(),
            retained_source_bytes: "UNAVAILABLE".to_string(),
            provenance: "H3State::tree().node_count (post-timer pure read)".to_string(),
        }
    }
}

impl StateReprExport for markit_mdbench_restart_convergence::H4State {
    fn state_repr(&self) -> StateReprV1 {
        StateReprV1 {
            retained_blocks: self.blocks().len().to_string(),
            checkpoints: self.checkpoints().len().to_string(),
            fragment_metadata_entries: self.defs().len().to_string(),
            old_tree_index_entries: "UNAVAILABLE".to_string(),
            retained_source_bytes: self.src_len().to_string(),
            provenance: "H4State::blocks/checkpoints/defs/src_len (post-timer pure read)"
                .to_string(),
        }
    }
}

/// One materialized case for any surface.
#[derive(Debug, Clone)]
pub struct CaseSpec {
    pub case_id: CaseId,
    pub case_id_hex: String,
    pub payload_id: String,
    pub pre_source: String,
    pub post_source: String,
    pub edit: Option<CanonicalEdit>,
    pub pre_len_bytes: u64,
    pub operation: OperationKind,
}

impl CaseSpec {
    /// Construction of a full-parse case (`post_source` is ignored).
    pub fn full_parse(case_id: CaseId, case_id_hex: String, payload_id: String, source: String) -> Self {
        let len = source.len() as u64;
        Self {
            case_id,
            case_id_hex,
            payload_id,
            pre_source: source.clone(),
            post_source: source,
            edit: None,
            pre_len_bytes: len,
            operation: OperationKind::FullParse,
        }
    }

    /// An update case.
    pub fn update(
        case_id: CaseId,
        case_id_hex: String,
        payload_id: String,
        pre_source: String,
        post_source: String,
        edit: CanonicalEdit,
    ) -> Self {
        let len = pre_source.len() as u64;
        Self {
            case_id,
            case_id_hex,
            payload_id,
            pre_source,
            post_source,
            operation: OperationKind::classify(&edit),
            edit: Some(edit),
            pre_len_bytes: len,
        }
    }

    pub fn sources(&self) -> (Source, Source) {
        (
            Source::new(SourceId(0), self.pre_source.clone()),
            Source::new(SourceId(1), self.post_source.clone()),
        )
    }

    /// Case facts for the frozen result row.
    pub fn facts(&self, mechanism_id: &str, seed: u64) -> CaseFacts {
        CaseFacts {
            case_id: self.case_id,
            seed: markit_mdbench_common::Seed(seed),
            mechanism_id: markit_mdbench_common::MechanismId(mechanism_id.to_string()),
            operation: self.operation,
            payload: PayloadMetaV1 {
                payload_id: self.payload_id.clone(),
                shape: markit_mdbench_common::PayloadShape::Mixed,
                size_bytes: self.pre_len_bytes,
            },
            edit: markit_mdbench_runner::edit_meta(self.operation, self.edit.as_ref())
                .expect("canonical edit satisfies the operation contract"),
        }
    }
}

/// The correctness authority for a case: the H0 clean full parse of the
/// POST-edit source, normalized and validated. Computed ONCE per case,
/// strictly outside every timer (task §12).
pub fn reference_for(post_source: &str) -> Result<markit_mdbench_oracle::NormalizedDocument, String> {
    let doc = markit_mdbench_full_rebuild::parse_document(post_source.as_bytes());
    validate_normalized(&doc, None).map_err(|e| format!("reference gate: {e:?}"))?;
    Ok(doc)
}

fn catch_phase<T>(phase: impl FnOnce() -> Result<T, FailureStatus>) -> Result<T, FailureStatus> {
    match catch_unwind(AssertUnwindSafe(phase)) {
        Ok(result) => result,
        Err(_) => Err(FailureStatus::Crash),
    }
}

/// A chained update step: the frozen report PLUS the new sealed state.
pub struct ChainStepReport<State> {
    pub report: markit_mdbench_runner::RunReport,
    pub new_state: Option<State>,
    pub state_repr: Option<StateReprV1>,
}

/// UPDATE case on an EXISTING state, T-LANE — Surface C's chained step.
///
/// Timer boundary is byte-for-byte the frozen
/// `markit_mdbench_runner::orchestrate::run_update_timed` boundary:
///
/// ```text
/// T_prepare START -> prepare_update          -> T_prepare STOP
/// T_native  START -> update + complete + black_box -> T_native STOP
/// T_total = T_prepare + T_native (arithmetic)
/// AFTER both timers: result checksum, oracle hook
/// ```
///
/// The only difference is that the sealed state is handed back instead of
/// being dropped, which is what a chained lifecycle run requires.
pub fn run_update_chain_step<M, C>(
    mechanism: &M,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    old_state: M::State,
    clock: &C,
    hook: &dyn CorrectnessHook<M::State>,
) -> ChainStepReport<M::State>
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
        Err(failure) => {
            return ChainStepReport {
                report: markit_mdbench_runner::RunReport {
                    execution_status: failure.into(),
                    correctness_status: CorrectnessStatus::NotChecked,
                    measurement: LaneMeasurement::Timing(TimingRecord::unknown()),
                    result_checksum: None,
                    failure: Some(failure),
                },
                new_state: None,
                state_repr: None,
            }
        }
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

    let measurement = match TimingRecord::update(prepare_ns, native_ns) {
        Ok(record) => LaneMeasurement::Timing(record),
        Err(_) => {
            let checksum = outcome
                .as_ref()
                .ok()
                .map(|done: &Completed<M::State>| done.state.result_checksum());
            return ChainStepReport {
                report: markit_mdbench_runner::RunReport {
                    execution_status: ExecutionStatus::InstrumentationUnavailable,
                    correctness_status: CorrectnessStatus::NotChecked,
                    measurement: LaneMeasurement::Timing(TimingRecord::unknown()),
                    result_checksum: checksum,
                    failure: Some(FailureStatus::InstrumentationUnavailable),
                },
                new_state: None,
                state_repr: None,
            };
        }
    };

    match outcome {
        Ok(done) => {
            let checksum = done.state.result_checksum();
            let correctness = hook.verify(&done);
            ChainStepReport {
                report: markit_mdbench_runner::RunReport {
                    execution_status: ExecutionStatus::Pass,
                    correctness_status: correctness,
                    measurement,
                    result_checksum: Some(checksum),
                    failure: None,
                },
                new_state: Some(done.state),
                state_repr: None,
            }
        }
        Err(failure) => ChainStepReport {
            report: markit_mdbench_runner::RunReport {
                execution_status: failure.into(),
                correctness_status: CorrectnessStatus::NotChecked,
                measurement,
                result_checksum: None,
                failure: Some(failure),
            },
            new_state: None,
            state_repr: None,
        },
    }
}

/// Uniform verification of one dispatch (task §36 semantics: warmups are
/// not disposable correctness).
pub fn verify_report(report: &markit_mdbench_runner::RunReport) -> Result<(), String> {
    if report.execution_status != ExecutionStatus::Pass {
        return Err(format!("execution_status != pass: {:?}", report.execution_status));
    }
    if report.correctness_status != CorrectnessStatus::Pass {
        return Err(format!(
            "correctness_status != pass: {:?}",
            report.correctness_status
        ));
    }
    match &report.measurement {
        LaneMeasurement::Timing(timing) => {
            let unknown = |v: &markit_mdbench_common::Observed<u64>| {
                *v == markit_mdbench_common::Observed::Unknown
            };
            if unknown(&timing.prepare_ns) || unknown(&timing.native_ns) || unknown(&timing.total_ns)
            {
                return Err("qualified timing metric UNKNOWN".to_string());
            }
        }
        LaneMeasurement::Attribution(counters) => {
            if counters.all_unknown_slots() {
                return Err("attribution counters are all UNKNOWN on a completed run".to_string());
            }
        }
        LaneMeasurement::Memory(_) => {}
    }
    Ok(())
}

/// Surface A — one CONSTRUCTION dispatch (T-LANE).
pub fn construction_row<M, C>(
    mechanism: &M,
    source: &Source,
    clock: &C,
    hook: &dyn CorrectnessHook<M::State>,
    facts: &CaseFacts,
    identity: &ExecutionIdentity2,
    build: &markit_mdbench_runner::BuildIdentityV1,
) -> (ResultRowV1, Option<StateReprV1>)
where
    M: Mechanism,
    M::State: ResultChecksum + StateReprExport,
    C: Clock,
{
    let report = run_full_parse_timed(mechanism, source, clock, hook);
    let row = assemble_row(
        facts,
        &report,
        build,
        &identity.machine_environment_ref,
        identity.provenance,
    );
    (row, None)
}

/// Surface B — one RESIDENT_UPDATE dispatch (T-LANE), SINGLE_RESET.
///
/// The fresh pre-edit state is built OUTSIDE every timer (frozen
/// `build_initial_state`); construction cost never enters `T_total`.
pub fn resident_update_row<M, C>(
    mechanism: &M,
    case: &CaseSpec,
    clock: &C,
    hook: &dyn CorrectnessHook<M::State>,
    facts: &CaseFacts,
    identity: &ExecutionIdentity2,
    build: &markit_mdbench_runner::BuildIdentityV1,
) -> Result<ResultRowV1, String>
where
    M: Mechanism,
    M::State: ResultChecksum + StateReprExport,
    C: Clock,
{
    let (pre, post) = case.sources();
    let edit = case
        .edit
        .as_ref()
        .ok_or_else(|| format!("{}: RESIDENT_UPDATE case carries no edit", case.case_id_hex))?;
    let old_state = match build_initial_state(mechanism, &pre) {
        Ok(state) => state,
        Err(failure) => {
            return Ok(failure_row(
                facts,
                failure,
                build,
                &identity.machine_environment_ref,
                identity.provenance,
            ))
        }
    };
    let report = run_update_timed(mechanism, &pre, &post, edit, old_state, clock, hook);
    Ok(assemble_row(
        facts,
        &report,
        build,
        &identity.machine_environment_ref,
        identity.provenance,
    ))
}

/// Surface C — build once, then chain `steps` edits on the SAME state.
///
/// Returns one row per step, plus the state-representation export of the
/// final sealed state. A step that fails returns its row and stops the
/// chain (the caller records the failure and stops the sub-campaign).
pub struct LifecycleStepOutcome {
    pub step: u32,
    pub row: ResultRowV1,
}

pub struct LifecycleRunOutcome {
    pub steps: Vec<LifecycleStepOutcome>,
    pub final_state_repr: Option<StateReprV1>,
}

pub fn lifecycle_run<M, C>(
    mechanism: &M,
    initial_source: &Source,
    step_sources: &[(Source, Source, CanonicalEdit)],
    step_facts: &[CaseFacts],
    clock: &C,
    hooks: &[ReferenceOracle],
    identity: &ExecutionIdentity2,
    build: &markit_mdbench_runner::BuildIdentityV1,
) -> Result<LifecycleRunOutcome, String>
where
    M: Mechanism,
    M::State: ResultChecksum + StateReprExport + NormalizeV1,
    C: Clock,
{
    if step_sources.len() != step_facts.len() || step_sources.len() != hooks.len() {
        return Err("lifecycle step vectors disagree in length".to_string());
    }
    // Build the state ONCE (task §15) — outside every timer.
    let initial_state = match build_initial_state(mechanism, initial_source) {
        Ok(state) => state,
        Err(failure) => {
            let row = failure_row(
                &step_facts[0],
                failure,
                build,
                &identity.machine_environment_ref,
                identity.provenance,
            );
            return Ok(LifecycleRunOutcome {
                steps: vec![LifecycleStepOutcome { step: 0, row }],
                final_state_repr: None,
            });
        }
    };
    let mut state = Some(initial_state);
    let mut rows = Vec::with_capacity(step_sources.len());
    let mut final_repr = None;
    for (index, ((pre, post, edit), facts)) in step_sources.iter().zip(step_facts).enumerate() {
        let Some(current_state) = state.take() else {
            break;
        };
        let outcome =
            run_update_chain_step(mechanism, pre, post, edit, current_state, clock, &hooks[index]);
        let row = assemble_row(
            facts,
            &outcome.report,
            build,
            &identity.machine_environment_ref,
            identity.provenance,
        );
        let failed = outcome.new_state.is_none();
        if let Some(new_state) = outcome.new_state {
            final_repr = Some(new_state.state_repr());
            state = Some(new_state);
        }
        rows.push(LifecycleStepOutcome {
            step: index as u32,
            row,
        });
        if failed {
            // The chain cannot continue without a sealed state.
            break;
        }
    }
    Ok(LifecycleRunOutcome {
        steps: rows,
        final_state_repr: final_repr,
    })
}

/// Attribution lane dispatch for a construction case.
pub fn construction_attributed_row<M>(
    mechanism: &M,
    source: &Source,
    hook: &dyn CorrectnessHook<M::State>,
    facts: &CaseFacts,
    identity: &ExecutionIdentity2,
    build: &markit_mdbench_runner::BuildIdentityV1,
) -> ResultRowV1
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let report = markit_mdbench_runner::orchestrate::run_full_parse_attributed(
        mechanism,
        source,
        &mut WorkCounters::all_unknown(),
        hook,
    );
    assemble_row(
        facts,
        &report,
        build,
        &identity.machine_environment_ref,
        identity.provenance,
    )
}

/// Attribution lane dispatch for an update case (fresh state built
/// outside the collection window).
pub fn update_attributed_row<M>(
    mechanism: &M,
    case: &CaseSpec,
    hook: &dyn CorrectnessHook<M::State>,
    facts: &CaseFacts,
    identity: &ExecutionIdentity2,
    build: &markit_mdbench_runner::BuildIdentityV1,
) -> Result<ResultRowV1, String>
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let (pre, post) = case.sources();
    let edit = case
        .edit
        .as_ref()
        .ok_or_else(|| format!("{}: attribution case carries no edit", case.case_id_hex))?;
    let old_state = match build_initial_state(mechanism, &pre) {
        Ok(state) => state,
        Err(failure) => {
            return Ok(failure_row(
                facts,
                failure,
                build,
                &identity.machine_environment_ref,
                identity.provenance,
            ))
        }
    };
    let mut counters = WorkCounters::all_unknown();
    let report = markit_mdbench_runner::orchestrate::run_update_attributed(
        mechanism,
        &pre,
        &post,
        edit,
        old_state,
        &mut counters,
        hook,
    );
    Ok(assemble_row(
        facts,
        &report,
        build,
        &identity.machine_environment_ref,
        identity.provenance,
    ))
}

/// Attribution lane dispatch for a CHAINED lifecycle step.
///
/// Uses the same `CounterSink` authority as the frozen
/// `run_update_attributed`, with the same chained-state rule as
/// [`run_update_chain_step`].
pub fn lifecycle_step_attributed<M>(
    mechanism: &M,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
    state: M::State,
    hook: &dyn CorrectnessHook<M::State>,
) -> RunAttributionOutcome<M::State>
where
    M: Mechanism,
    M::State: ResultChecksum,
{
    let (old, post, edit) = (
        black_box(pre),
        black_box(post),
        black_box(edit),
    );
    let mut counters = WorkCounters::all_unknown();
    let outcome = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let outcome = catch_phase(|| mechanism.prepare_update(old, post, edit, &state, &mut cx))
            .and_then(|prep| {
                catch_phase(|| {
                    let pending = mechanism.update(old, post, edit, state, prep, &mut cx)?;
                    let done = mechanism.complete(pending)?;
                    black_box(&done);
                    Ok(done)
                })
            });
        if outcome.is_ok() {
            sink.finalize_derived();
        }
        outcome
    };
    match outcome {
        Ok(done) => {
            let checksum = done.state.result_checksum();
            let correctness = hook.verify(&done);
            RunAttributionOutcome {
                execution_status: ExecutionStatus::Pass,
                correctness_status: correctness,
                counters,
                result_checksum: Some(checksum),
                new_state: Some(done.state),
            }
        }
        Err(failure) => RunAttributionOutcome {
            execution_status: failure.into(),
            correctness_status: CorrectnessStatus::NotChecked,
            counters,
            result_checksum: None,
            new_state: None,
        },
    }
}

/// Result of one chained attributed lifecycle step.
pub struct RunAttributionOutcome<State> {
    pub execution_status: ExecutionStatus,
    pub correctness_status: CorrectnessStatus,
    pub counters: WorkCounters,
    pub result_checksum: Option<u64>,
    pub new_state: Option<State>,
}

/// Build the `RunReport` a chained attributed step would have produced,
/// so it can flow through the SAME `assemble_row` path.
pub fn attribution_report(report: &RunAttributionOutcome<impl Sized>) -> markit_mdbench_runner::RunReport {
    markit_mdbench_runner::RunReport {
        execution_status: report.execution_status,
        correctness_status: report.correctness_status,
        measurement: LaneMeasurement::Attribution(report.counters.clone()),
        result_checksum: report.result_checksum,
        failure: None,
    }
}
