//! Campaign session execution + the raw observation envelope (task §8,
//! §16-§18, §21, §42-§46, §49).
//!
//! The campaign layer is ORCHESTRATION ONLY: every dispatch goes through
//! the existing runner functions with their frozen timer boundaries.
//! Nothing here re-implements timing, correctness, or the result row.
//!
//! Fresh-state rule (task §8, SINGLE_RESET): every measured EDIT_WRITE
//! iteration builds a NEW clean pre-edit horse state outside every timer;
//! no state is retained from a previous iteration and none is cloned
//! from a cached build. The cost of constructing that state belongs to
//! the CLEAN_STATE surface, never to the update timing.
//!
//! Concurrency (task §43): strictly one case at a time, one horse at a
//! time, one iteration at a time, single algorithm worker — the
//! comparison stays algorithmic, not scheduler throughput.
//!
//! File I/O boundary (task §44): sources are materialized and verified
//! before the measured phase; no disk read, checkout, decompression,
//! JSON parse, manifest lookup, or network ever happens inside
//! mechanism timing, and the campaign performs no network access at all.
//!
//! Failure policy (task §36-§38): any sample whose execution or
//! correctness did not pass — warmup or measured alike — is RETAINED as
//! raw evidence, marks the campaign `PRIMARY_CAMPAIGN_INVALID` for
//! headline comparison, and stops the session cleanly. Nothing is
//! deleted, replaced, retried, imputed, or zeroed.

use std::io;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use markit_mdbench_common::{
    CorrectnessStatus, ExecutionStatus, MechanismId, Observed, Seed, WorkCounters,
};
use markit_mdbench_instrumentation::{Clock, InstantClock, LaneMeasurement, ManualClock};
use markit_mdbench_oracle::{validate_normalized, NormalizeV1, ReferenceOracle};
use markit_mdbench_runner::orchestrate::{
    build_initial_state, run_full_parse_attributed, run_full_parse_timed, run_update_attributed,
    run_update_timed,
};
use markit_mdbench_runner::{assemble_row, failure_row, CaseFacts, ResultRowV1};

use crate::workload::{CleanStateCase, EditWriteCase};
use crate::{SampleKind, Surface, ENVELOPE_SCHEMA_ID};

/// Provenance tags.
pub const PROVENANCE_TIMING: &str = "PRIMARY-PERFORMANCE-CAMPAIGN-v1/TIMING";
pub const PROVENANCE_ATTRIBUTION: &str = "PRIMARY-PERFORMANCE-CAMPAIGN-v1/ATTRIBUTION";
pub const PROVENANCE_NON_RESEARCH_SMOKE: &str = "CAMPAIGN_SMOKE/NON_RESEARCH_RESULT";

/// The raw campaign observation envelope (task §21): a narrow scheduling
/// identity wrapped around ONE existing schema-v2 result row. The core
/// row is NOT overloaded with campaign scheduling facts. Banned
/// analysis fields (winner / rank / speedup / score / interpretation)
/// are structurally absent and must never be added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CampaignObservationV1 {
    pub schema: String,
    pub campaign_spec_id: String,
    pub run_id: String,
    pub session_id: String,
    pub surface: String,
    pub sample_kind: String,
    /// `None` for attribution rows (they belong to no timing session).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_ordinal: Option<u32>,
    pub case_order_ordinal: u32,
    pub horse_order_ordinal: u32,
    pub iteration_ordinal: u32,
    /// Derived from the immutable identity fields (task §22); unique
    /// across the whole campaign (duplicate-guarded by verification).
    pub observation_id: String,
    /// The existing raw result row (schema v2; Rust authority
    /// `runner/src/result.rs`).
    pub result_row_v2: ResultRowV1,
}

/// Which clock the session runs on. Real is for the FUTURE primary
/// timing task only; this freeze executes only the fake-clock smoke
/// (task §49-§50).
pub enum CampaignClock {
    Real(InstantClock),
    /// Deterministic NON-RESEARCH clock for plumbing smoke only.
    Fake(ManualClock),
}

impl Clock for CampaignClock {
    fn now_nanos(&self) -> u64 {
        match self {
            CampaignClock::Real(clock) => clock.now_nanos(),
            CampaignClock::Fake(clock) => clock.now_nanos(),
        }
    }
}

/// Identity context stamped into every observation of one execution.
#[derive(Debug, Clone)]
pub struct ExecutionIdentity {
    pub campaign_spec_id: String,
    pub run_id: String,
    pub machine_environment_ref: String,
    pub provenance: &'static str,
    pub non_research: bool,
}

/// Outcome of one session (or smoke) execution.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionOutcome {
    Completed {
        observations: u64,
        warmup_rows: u64,
        measured_rows: u64,
    },
    /// A sample failed execution/correctness verification: the row is
    /// retained, the session stopped cleanly, and the campaign is
    /// PRIMARY_CAMPAIGN_INVALID for headline comparison (task §36).
    Invalid {
        reason: String,
        observations: u64,
        warmup_rows: u64,
        measured_rows: u64,
    },
}

impl SessionOutcome {
    pub fn observations(&self) -> u64 {
        match self {
            SessionOutcome::Completed { observations, .. }
            | SessionOutcome::Invalid { observations, .. } => *observations,
        }
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, SessionOutcome::Invalid { .. })
    }
}

/// One scheduled cell to execute: the schedule row's identity plus the
/// materialized case.
pub enum ScheduledCase<'a> {
    CleanState {
        order_ordinal: u32,
        horse_order: Vec<String>,
        case: &'a CleanStateCase,
    },
    EditWrite {
        order_ordinal: u32,
        horse_order: Vec<String>,
        case: &'a EditWriteCase,
    },
}

impl ScheduledCase<'_> {
    pub fn order_ordinal(&self) -> u32 {
        match self {
            ScheduledCase::CleanState { order_ordinal, .. }
            | ScheduledCase::EditWrite { order_ordinal, .. } => *order_ordinal,
        }
    }

    pub fn horse_order(&self) -> &[String] {
        match self {
            ScheduledCase::CleanState { horse_order, .. }
            | ScheduledCase::EditWrite { horse_order, .. } => horse_order,
        }
    }

    pub fn case_id_hex(&self) -> &str {
        match self {
            ScheduledCase::CleanState { case, .. } => &case.case_id_hex,
            ScheduledCase::EditWrite { case, .. } => &case.case_id_hex,
        }
    }
}

/// Where observations are appended (create/append-only; task §39).
pub trait ObservationSink {
    fn append(&mut self, observation: &CampaignObservationV1) -> io::Result<()>;
}

impl<W: io::Write> ObservationSink for W {
    fn append(&mut self, observation: &CampaignObservationV1) -> io::Result<()> {
        let line = serde_json::to_string(observation)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_all(line.as_bytes())?;
        self.write_all(b"\n")
    }
}

/// Session executor over scheduled cases. `warmup`/`measured` come from
/// the frozen manifest for real sessions; the NON_RESEARCH smoke passes
/// reduced counts (task §49: tiny fixed subset).
pub struct SessionExecutor<'a> {
    pub identity: &'a ExecutionIdentity,
    pub surface: Surface,
    /// `None` for attribution.
    pub session_ordinal: Option<u32>,
    pub session_id: String,
    pub session_seed: u64,
    pub build_identity: markit_mdbench_runner::BuildIdentityV1,
    pub warmup: u32,
    pub measured: u32,
    pub observations: u64,
    pub warmup_rows: u64,
    pub measured_rows: u64,
    /// Failure-injection flag (NON_RESEARCH smoke + tests only): the
    /// correctness hook always reports `wrong_result`, proving that a
    /// wrong sample is retained, marks the campaign invalid, and stops
    /// the session cleanly (task §49 failure propagation). Never set on
    /// a research execution path.
    pub poison_correctness: bool,
}

/// A correctness hook that always fails — used ONLY to prove failure
/// propagation (task §49). Never a research path.
pub struct AlwaysWrongHook;

impl<S> markit_mdbench_oracle::CorrectnessHook<S> for AlwaysWrongHook {
    fn verify(&self, _completed: &markit_mdbench_common::Completed<S>) -> CorrectnessStatus {
        CorrectnessStatus::WrongResult
    }
}

impl<'a> SessionExecutor<'a> {
    fn observation_id_for(
        &self,
        case_id: &str,
        horse_id: &str,
        sample_kind: &str,
        iteration_ordinal: u32,
    ) -> String {
        crate::identity::observation_id(
            &self.identity.run_id,
            &self.session_id,
            self.surface.as_str(),
            case_id,
            horse_id,
            sample_kind,
            iteration_ordinal,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        sink: &mut dyn ObservationSink,
        case: &ScheduledCase<'_>,
        horse_order_ordinal: u32,
        horse_id: &str,
        sample_kind: SampleKind,
        iteration_ordinal: u32,
        row: ResultRowV1,
    ) -> Result<(), String> {
        let observation_id = self.observation_id_for(
            case.case_id_hex(),
            horse_id,
            sample_kind.as_str(),
            iteration_ordinal,
        );
        let observation = CampaignObservationV1 {
            schema: ENVELOPE_SCHEMA_ID.to_string(),
            campaign_spec_id: self.identity.campaign_spec_id.clone(),
            run_id: self.identity.run_id.clone(),
            session_id: self.session_id.clone(),
            surface: self.surface.as_str().to_string(),
            sample_kind: sample_kind.as_str().to_string(),
            session_ordinal: self.session_ordinal,
            case_order_ordinal: case.order_ordinal(),
            horse_order_ordinal,
            iteration_ordinal,
            observation_id,
            result_row_v2: row,
        };
        sink.append(&observation)
            .map_err(|e| format!("append observation: {e}"))?;
        match sample_kind {
            SampleKind::Warmup => self.warmup_rows += 1,
            SampleKind::Measured => self.measured_rows += 1,
            SampleKind::Attribution => {}
        }
        self.observations += 1;
        Ok(())
    }

    /// Verify one report (task §16-§17, §36-§37): warmups are not
    /// disposable correctness.
    fn verify_report(report: &markit_mdbench_runner::RunReport) -> Result<(), String> {
        if report.execution_status != ExecutionStatus::Pass {
            return Err(format!(
                "execution_status != pass: {:?}",
                report.execution_status
            ));
        }
        if report.correctness_status != CorrectnessStatus::Pass {
            return Err(format!(
                "correctness_status != pass: {:?}",
                report.correctness_status
            ));
        }
        if let LaneMeasurement::Timing(timing) = &report.measurement {
            let unknown = |value: &Observed<u64>| *value == Observed::Unknown;
            if unknown(&timing.prepare_ns)
                || unknown(&timing.native_ns)
                || unknown(&timing.total_ns)
            {
                return Err("qualified timing metric UNKNOWN (task §36)".to_string());
            }
        }
        Ok(())
    }

    fn fail(&mut self, reason: String) -> SessionOutcome {
        SessionOutcome::Invalid {
            reason,
            observations: self.observations,
            warmup_rows: self.warmup_rows,
            measured_rows: self.measured_rows,
        }
    }

    /// Execute every scheduled case of the session, in schedule order,
    /// one case × one horse × one iteration at a time. Horses dispatch
    /// through their concrete mechanism types (the same match pattern
    /// as the #35 dry-run adapter; there is no dyn Mechanism).
    pub fn run(
        mut self,
        cases: &[ScheduledCase<'_>],
        clock: &CampaignClock,
        sink: &mut dyn ObservationSink,
    ) -> Result<SessionOutcome, String> {
        for case in cases {
            for (horse_order_ordinal, horse_id) in case.horse_order().iter().enumerate() {
                let horse_order_ordinal = horse_order_ordinal as u32;
                let outcome = match horse_id.as_str() {
                    "H0" => self.run_cell(
                        markit_mdbench_full_rebuild::H0_MECHANISM_ID,
                        &markit_mdbench_full_rebuild::FullRebuildMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        clock,
                        sink,
                    )?,
                    "H1" => self.run_cell(
                        markit_mdbench_block_local::H1_MECHANISM_ID,
                        &markit_mdbench_block_local::BlockLocalMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        clock,
                        sink,
                    )?,
                    "H2" => self.run_cell(
                        markit_mdbench_fragment_reuse::H2_MECHANISM_ID,
                        &markit_mdbench_fragment_reuse::FragmentReuseMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        clock,
                        sink,
                    )?,
                    "H3" => self.run_cell(
                        markit_mdbench_old_tree_subtree_reuse::H3_MECHANISM_ID,
                        &markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        clock,
                        sink,
                    )?,
                    "H4" => self.run_cell(
                        markit_mdbench_restart_convergence::H4_MECHANISM_ID,
                        &markit_mdbench_restart_convergence::RestartConvergenceMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        clock,
                        sink,
                    )?,
                    other => return Err(format!("unknown horse {other:?} in schedule")),
                };
                if outcome.is_invalid() {
                    return Ok(outcome);
                }
            }
        }
        Ok(SessionOutcome::Completed {
            observations: self.observations,
            warmup_rows: self.warmup_rows,
            measured_rows: self.measured_rows,
        })
    }

    /// One `case × horse` cell: `warmup` warmups then `measured`
    /// measured iterations, contiguous, with IDENTICAL setup semantics
    /// (task §16) — `sample_kind` is the only difference.
    #[allow(clippy::too_many_arguments)]
    fn run_cell<M>(
        &mut self,
        mechanism_id: &str,
        mechanism: &M,
        case: &ScheduledCase<'_>,
        horse_order_ordinal: u32,
        horse_id: &str,
        clock: &CampaignClock,
        sink: &mut dyn ObservationSink,
    ) -> Result<SessionOutcome, String>
    where
        M: markit_mdbench_common::Mechanism,
        M::State: NormalizeV1 + markit_mdbench_common::ResultChecksum,
    {
        match case {
            ScheduledCase::CleanState {
                case: clean_case, ..
            } => {
                let source = markit_mdbench_common::Source::new(
                    markit_mdbench_common::SourceId(0),
                    clean_case.source_text.clone(),
                );
                // Correctness authority: H0 clean parse of the same
                // source, computed ONCE per case outside every timer
                // (parse_document keeps the NORMALIZED-RESULT-v1
                // conformance gate on the reference path).
                let reference = markit_mdbench_full_rebuild::parse_document(source.as_bytes());
                validate_normalized(&reference, None)
                    .map_err(|e| format!("clean-state reference gate: {e:?}"))?;
                let reference_hook = ReferenceOracle::new(reference);
                let poison = AlwaysWrongHook;
                let hook: &dyn markit_mdbench_oracle::CorrectnessHook<M::State> =
                    if self.poison_correctness {
                        &poison
                    } else {
                        &reference_hook
                    };
                let facts = clean_state_facts(clean_case, mechanism_id, Seed(self.session_seed));
                for iteration in 0..(self.warmup + self.measured) {
                    let (sample_kind, iteration_ordinal) = if iteration < self.warmup {
                        (SampleKind::Warmup, iteration)
                    } else {
                        (SampleKind::Measured, iteration - self.warmup)
                    };
                    let report = run_full_parse_timed(mechanism, &source, clock, hook);
                    let row = assemble_row(
                        &facts,
                        &report,
                        &self.build_identity,
                        &self.identity.machine_environment_ref,
                        self.identity.provenance,
                    );
                    if let Err(reason) = Self::verify_report(&report) {
                        let reason = format!(
                            "clean_state case {} horse {horse_id} {} iter {iteration_ordinal}: {reason}",
                            clean_case.case_id_hex,
                            sample_kind.as_str()
                        );
                        self.emit(
                            sink,
                            case,
                            horse_order_ordinal,
                            horse_id,
                            sample_kind,
                            iteration_ordinal,
                            row,
                        )?;
                        return Ok(self.fail(reason));
                    }
                    self.emit(
                        sink,
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sample_kind,
                        iteration_ordinal,
                        row,
                    )?;
                }
            }
            ScheduledCase::EditWrite {
                case: edit_case, ..
            } => {
                let (pre, post) = crate::workload::edit_case_sources(edit_case);
                // Correctness authority: H0 clean full parse of the POST
                // source, once per case, outside every timer.
                let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
                validate_normalized(&reference, None)
                    .map_err(|e| format!("edit reference gate: {e:?}"))?;
                let reference_hook = ReferenceOracle::new(reference);
                let poison = AlwaysWrongHook;
                let hook: &dyn markit_mdbench_oracle::CorrectnessHook<M::State> =
                    if self.poison_correctness {
                        &poison
                    } else {
                        &reference_hook
                    };
                let facts = edit_write_facts(edit_case, mechanism_id, Seed(self.session_seed));
                for iteration in 0..(self.warmup + self.measured) {
                    let (sample_kind, iteration_ordinal) = if iteration < self.warmup {
                        (SampleKind::Warmup, iteration)
                    } else {
                        (SampleKind::Measured, iteration - self.warmup)
                    };
                    // FRESH-STATE RULE (task §8): a NEW clean pre-edit
                    // state per iteration, outside every timer, never
                    // retained from a previous iteration, never cloned
                    // from a cached build.
                    let old_state = match build_initial_state(mechanism, &pre) {
                        Ok(state) => state,
                        Err(failure) => {
                            let row = failure_row(
                                &facts,
                                failure,
                                &self.build_identity,
                                &self.identity.machine_environment_ref,
                                self.identity.provenance,
                            );
                            let reason = format!(
                                "edit_write case {} horse {horse_id}: fresh pre-state construction failed: {failure:?}",
                                edit_case.case_id_hex
                            );
                            self.emit(
                                sink,
                                case,
                                horse_order_ordinal,
                                horse_id,
                                sample_kind,
                                iteration_ordinal,
                                row,
                            )?;
                            return Ok(self.fail(reason));
                        }
                    };
                    let report = run_update_timed(
                        mechanism,
                        &pre,
                        &post,
                        &edit_case.edit,
                        old_state,
                        clock,
                        hook,
                    );
                    let row = assemble_row(
                        &facts,
                        &report,
                        &self.build_identity,
                        &self.identity.machine_environment_ref,
                        self.identity.provenance,
                    );
                    if let Err(reason) = Self::verify_report(&report) {
                        let reason = format!(
                            "edit_write case {} horse {horse_id} {} iter {iteration_ordinal}: {reason}",
                            edit_case.case_id_hex,
                            sample_kind.as_str()
                        );
                        self.emit(
                            sink,
                            case,
                            horse_order_ordinal,
                            horse_id,
                            sample_kind,
                            iteration_ordinal,
                            row,
                        )?;
                        return Ok(self.fail(reason));
                    }
                    self.emit(
                        sink,
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sample_kind,
                        iteration_ordinal,
                        row,
                    )?;
                }
            }
        }
        Ok(SessionOutcome::Completed {
            observations: self.observations,
            warmup_rows: self.warmup_rows,
            measured_rows: self.measured_rows,
        })
    }

    /// Attribution lane (task §18, §45-§46): one untimed dispatch per
    /// logical case × horse with the CounterSink; no timing values exist
    /// on this path. Deterministic-order callers iterate the surface's
    /// cases in (case_id, horse) order.
    pub fn run_attribution(
        mut self,
        cases: &[ScheduledCase<'_>],
        sink: &mut dyn ObservationSink,
    ) -> Result<SessionOutcome, String> {
        for case in cases {
            for (horse_order_ordinal, horse_id) in case.horse_order().iter().enumerate() {
                let horse_order_ordinal = horse_order_ordinal as u32;
                let outcome = match horse_id.as_str() {
                    "H0" => self.run_attribution_cell(
                        markit_mdbench_full_rebuild::H0_MECHANISM_ID,
                        &markit_mdbench_full_rebuild::FullRebuildMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sink,
                    )?,
                    "H1" => self.run_attribution_cell(
                        markit_mdbench_block_local::H1_MECHANISM_ID,
                        &markit_mdbench_block_local::BlockLocalMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sink,
                    )?,
                    "H2" => self.run_attribution_cell(
                        markit_mdbench_fragment_reuse::H2_MECHANISM_ID,
                        &markit_mdbench_fragment_reuse::FragmentReuseMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sink,
                    )?,
                    "H3" => self.run_attribution_cell(
                        markit_mdbench_old_tree_subtree_reuse::H3_MECHANISM_ID,
                        &markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sink,
                    )?,
                    "H4" => self.run_attribution_cell(
                        markit_mdbench_restart_convergence::H4_MECHANISM_ID,
                        &markit_mdbench_restart_convergence::RestartConvergenceMechanism::new(),
                        case,
                        horse_order_ordinal,
                        horse_id,
                        sink,
                    )?,
                    other => {
                        return Err(format!("unknown horse {other:?} in attribution schedule"))
                    }
                };
                if outcome.is_invalid() {
                    return Ok(outcome);
                }
            }
        }
        Ok(SessionOutcome::Completed {
            observations: self.observations,
            warmup_rows: self.warmup_rows,
            measured_rows: self.measured_rows,
        })
    }

    fn run_attribution_cell<M>(
        &mut self,
        mechanism_id: &str,
        mechanism: &M,
        case: &ScheduledCase<'_>,
        horse_order_ordinal: u32,
        horse_id: &str,
        sink: &mut dyn ObservationSink,
    ) -> Result<SessionOutcome, String>
    where
        M: markit_mdbench_common::Mechanism,
        M::State: NormalizeV1 + markit_mdbench_common::ResultChecksum,
    {
        match case {
            ScheduledCase::CleanState {
                case: clean_case, ..
            } => {
                let source = markit_mdbench_common::Source::new(
                    markit_mdbench_common::SourceId(0),
                    clean_case.source_text.clone(),
                );
                let reference = markit_mdbench_full_rebuild::parse_document(source.as_bytes());
                validate_normalized(&reference, None)
                    .map_err(|e| format!("attribution reference gate: {e:?}"))?;
                let hook = ReferenceOracle::new(reference);
                let facts = clean_state_facts(clean_case, mechanism_id, Seed(self.session_seed));
                let mut counters = WorkCounters::all_unknown();
                let report = run_full_parse_attributed(mechanism, &source, &mut counters, &hook);
                let row = assemble_row(
                    &facts,
                    &report,
                    &self.build_identity,
                    &self.identity.machine_environment_ref,
                    self.identity.provenance,
                );
                if let Err(reason) = Self::verify_report(&report) {
                    let reason = format!(
                        "attribution clean_state case {} horse {horse_id}: {reason}",
                        clean_case.case_id_hex
                    );
                    self.emit(
                        sink,
                        case,
                        horse_order_ordinal,
                        horse_id,
                        SampleKind::Attribution,
                        0,
                        row,
                    )?;
                    return Ok(self.fail(reason));
                }
                self.emit(
                    sink,
                    case,
                    horse_order_ordinal,
                    horse_id,
                    SampleKind::Attribution,
                    0,
                    row,
                )?;
            }
            ScheduledCase::EditWrite {
                case: edit_case, ..
            } => {
                let (pre, post) = crate::workload::edit_case_sources(edit_case);
                let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
                validate_normalized(&reference, None)
                    .map_err(|e| format!("attribution reference gate: {e:?}"))?;
                let hook = ReferenceOracle::new(reference);
                let facts = edit_write_facts(edit_case, mechanism_id, Seed(self.session_seed));
                // Fresh pre-edit state, outside any measurement.
                let old_state = build_initial_state(mechanism, &pre)
                    .map_err(|failure| format!("attribution pre-state: {failure:?}"))?;
                let mut counters = WorkCounters::all_unknown();
                let report = run_update_attributed(
                    mechanism,
                    &pre,
                    &post,
                    &edit_case.edit,
                    old_state,
                    &mut counters,
                    &hook,
                );
                let row = assemble_row(
                    &facts,
                    &report,
                    &self.build_identity,
                    &self.identity.machine_environment_ref,
                    self.identity.provenance,
                );
                if let Err(reason) = Self::verify_report(&report) {
                    let reason = format!(
                        "attribution edit_write case {} horse {horse_id}: {reason}",
                        edit_case.case_id_hex
                    );
                    self.emit(
                        sink,
                        case,
                        horse_order_ordinal,
                        horse_id,
                        SampleKind::Attribution,
                        0,
                        row,
                    )?;
                    return Ok(self.fail(reason));
                }
                self.emit(
                    sink,
                    case,
                    horse_order_ordinal,
                    horse_id,
                    SampleKind::Attribution,
                    0,
                    row,
                )?;
            }
        }
        Ok(SessionOutcome::Completed {
            observations: self.observations,
            warmup_rows: self.warmup_rows,
            measured_rows: self.measured_rows,
        })
    }
}

/// Case facts for a CLEAN_STATE dispatch (full-read payload meta, no
/// edit — the SAME operation contract as CaseKeyV1).
fn clean_state_facts(case: &CleanStateCase, mechanism_id: &str, seed: Seed) -> CaseFacts {
    CaseFacts {
        case_id: case.case_id,
        seed,
        mechanism_id: MechanismId(mechanism_id.to_string()),
        operation: markit_mdbench_common::OperationKind::FullParse,
        payload: markit_mdbench_runner::PayloadMetaV1 {
            payload_id: case.payload_id.clone(),
            shape: markit_mdbench_common::PayloadShape::Mixed,
            size_bytes: case.file_bytes,
        },
        edit: markit_mdbench_runner::edit_meta(
            markit_mdbench_common::OperationKind::FullParse,
            None,
        )
        .expect("FULL_PARSE carries no edit"),
    }
}

/// Case facts for an EDIT_WRITE dispatch.
fn edit_write_facts(case: &EditWriteCase, mechanism_id: &str, seed: Seed) -> CaseFacts {
    let operation = markit_mdbench_common::OperationKind::classify(&case.edit);
    CaseFacts {
        case_id: case.case_id,
        seed,
        mechanism_id: MechanismId(mechanism_id.to_string()),
        operation,
        payload: markit_mdbench_runner::PayloadMetaV1 {
            payload_id: case.payload_id.clone(),
            shape: markit_mdbench_common::PayloadShape::Mixed,
            size_bytes: case.pre_len_bytes,
        },
        edit: markit_mdbench_runner::edit_meta(operation, Some(&case.edit))
            .expect("canonical edit satisfies the operation contract"),
    }
}

/// Mechanism id for a horse label (frozen roster).
pub fn horse_id_to_mechanism(horse_id: &str) -> Result<&'static str, String> {
    crate::HORSE_ROSTER
        .iter()
        .find(|entry| entry.id == horse_id)
        .map(|entry| entry.mechanism_id)
        .ok_or_else(|| format!("unknown horse {horse_id:?}"))
}

/// Derive the session id of an attribution lane (no timing session).
pub fn attribution_session_id(spec_id: &str, surface: &str) -> String {
    let material = format!("{spec_id}\n{surface}\nattribution");
    crate::sha256_hex(material.as_bytes())
}
