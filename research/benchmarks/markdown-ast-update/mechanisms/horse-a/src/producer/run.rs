//! The one-invocation treatment lifecycle (task §7; readiness record
//! §10/§11) — ONE cell × ONE repetition × ONE raw observation per
//! process, with the recording window EXACTLY the frozen primary native
//! update boundary:
//!
//! ```text
//! PROCESS START
//!   capture static provenance                    (outside)
//!   construct the exact pre source + edit + post (outside)
//!   verify every frozen byte identity            (outside)
//!   Horse-A full build, NO-OP structural sink    (outside)
//!   C1 / READY sanity                            (outside)
//!   START RECORDING WINDOW
//!     update_with_structural (stage → prepare → commit)
//!   END RECORDING WINDOW
//!   C2 normalized oracle, NO-OP sink             (outside)
//!   C3 restore probe on the SAME returned state  (outside)
//!   adjudicate
//!   write exactly one immutable raw row
//! PROCESS EXIT
//! ```
//!
//! The recording lane and the no-op lane execute the identical Horse-A
//! algorithm and differ only in sink identity (#60 §10; readiness
//! record §7). No initial full build, no oracle normalization/export, no
//! C3 restore work, no document destruction and no output serialization
//! belongs inside the structural counters; the retirement required
//! before READY stays inside.
//!
//! This module collects no performance data of any kind (#60 §16): no
//! wall-clock, no cycles, no PMU, no allocation timing.

use std::path::Path;

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, ResultChecksum, SourceId};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_shared_grammar::parse_full;

use crate::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, Observed, RecordingHorseAStructuralSink,
};
use crate::update::update_with_structural;
use crate::validate::validate_ready;
use crate::{full_build, NormalizeV1, ReadyDocument};

use super::adjudicate::{adjudicate, Adjudication};
use super::contract::{self, ConstructedCell, FrozenCell};
use super::provenance::{self, Provenance};
use super::schema::{AdjudicationBlock, CorrectnessBlock, HostBlock, RawRowV1, ResultChecksums};
use super::writer::{check_output_path, write_row};

/// The terminal classification of one invocation. Exit codes (frozen CLI
/// contract): 0 = PASS (continue schedule); 3 = valid structural FAIL
/// (continue schedule — the remaining frozen observations still run,
/// task §38); 4 = INVALID (stop, task §40); 5 = correctness / cell
/// identity failure (stop, task §6/§39); 2 = usage errors (no treatment
/// performed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// Row written; adjudication PASS.
    Pass,
    /// Row written; valid structural FAIL — evidence against the frozen
    /// claim, NOT an instrumentation invalidity.
    StructuralFail,
    /// Row written; producer/instrumentation INVALID — stop collection.
    InvalidRow,
    /// Row written; a correctness gate failed — stop collection
    /// (CORRECTNESS_FAILURE).
    CorrectnessFailure,
    /// Row written; frozen byte-identity failure — stop collection
    /// (CELL_IDENTITY_FAILURE).
    IdentityFailure,
    /// No treatment performed: the destination already exists
    /// (duplicate-run refusal, fail closed).
    DuplicateOutput,
    /// No row writable at all: provenance capture or lifecycle failure
    /// before any observation existed. Stop and report.
    ProducerFailure(String),
}

impl RunOutcome {
    /// The process exit code (frozen CLI contract).
    pub fn exit_code(&self) -> i32 {
        match self {
            RunOutcome::Pass => 0,
            RunOutcome::StructuralFail => 3,
            RunOutcome::InvalidRow | RunOutcome::ProducerFailure(_) => 4,
            RunOutcome::CorrectnessFailure | RunOutcome::IdentityFailure => 5,
            RunOutcome::DuplicateOutput => 2,
        }
    }

    /// One-line status for the orchestrator's log.
    pub fn status_line(&self) -> String {
        match self {
            RunOutcome::Pass => "PASS".to_string(),
            RunOutcome::StructuralFail => "STRUCTURAL_FAIL".to_string(),
            RunOutcome::InvalidRow => "INVALID".to_string(),
            RunOutcome::CorrectnessFailure => "CORRECTNESS_FAILURE".to_string(),
            RunOutcome::IdentityFailure => "CELL_IDENTITY_FAILURE".to_string(),
            RunOutcome::DuplicateOutput => "DUPLICATE_OUTPUT_REFUSED".to_string(),
            RunOutcome::ProducerFailure(detail) => format!("PRODUCER_FAILURE: {detail}"),
        }
    }
}

/// The H0-clean-parse equality gate on one Horse-A state (full
/// normalized structural equality — never a hash; checksums are
/// provenance). Returns the state's result checksum.
fn gate_ready_equals_h0(doc: &ReadyDocument, bytes: &[u8]) -> Result<u64, String> {
    validate_ready(doc).map_err(|e| format!("READY invariants: {e}"))?;
    let exported = doc.normalize_v1();
    validate_normalized(&exported, Some(bytes))
        .map_err(|e| format!("export violates NORMALIZED-RESULT-v1: {e}"))?;
    let clean = parse_full(bytes, &mut NoopWorkSink);
    if exported != clean {
        return Err("normalize(Horse-A) != normalize(H0 clean parse)".to_string());
    }
    Ok(doc.result_checksum())
}

/// Assemble the immutable row from the observation's parts.
#[allow(clippy::too_many_arguments)]
fn assemble_row(
    cell: &FrozenCell,
    repetition: u64,
    provenance: &Provenance,
    constructed: Option<&ConstructedCell>,
    counters: &HorseAStructuralCountersV1,
    correctness: CorrectnessBlock,
    checksums: ResultChecksums,
    adjudication: &Adjudication,
    execution_command: &str,
) -> RawRowV1 {
    let counters_map = RawRowV1::counters_map(counters);
    let route = match counters.full_build_selected {
        Observed::Known(0) => "local",
        Observed::Known(_) => "same_target_full_build",
        Observed::Unknown => "unknown",
    }
    .to_string();
    let reason = match counters.full_build_reason {
        Observed::Known(0) => "none",
        Observed::Known(1) => "facts_differ",
        Observed::Known(2) => "preservation_unknown",
        _ => "unknown",
    }
    .to_string();
    let (observed_pre, observed_post) = match constructed {
        Some(c) => (c.pre.sha256_hex(), c.post.sha256_hex()),
        None => (
            contract::sha256_hex(contract::frozen_pre_source(cell).as_bytes()),
            String::new(),
        ),
    };
    RawRowV1 {
        schema: contract::RAW_SCHEMA.to_string(),
        row_identity: RawRowV1::compute_row_identity(
            contract::STUDY_ID,
            cell.cell_id,
            repetition,
            &provenance.executable_sha256,
        ),
        study_id: contract::STUDY_ID.to_string(),
        protocol: contract::PROTOCOL.to_string(),
        frozen_contract_revision: contract::FROZEN_CONTRACT_REVISION.to_string(),
        study_mechanism_baseline: contract::STUDY_MECHANISM_BASELINE.to_string(),
        authorization_baseline: contract::AUTHORIZATION_BASELINE.to_string(),
        repository_commit: provenance.repository_commit.clone(),
        repository_tree: provenance.repository_tree.clone(),
        mechanism: contract::MECHANISM.to_string(),
        mechanism_design_head: contract::MECHANISM_DESIGN_HEAD.to_string(),
        mechanism_merge: contract::MECHANISM_MERGE.to_string(),
        cell_id: cell.cell_id.to_string(),
        case_id_hex: cell.case_id_hex.to_string(),
        n_bytes: cell.n_bytes,
        m: cell.m,
        target: cell.target,
        edit_start: cell.edit_start,
        edit_end: cell.edit_start,
        inserted_text_sha256: contract::INSERTED_TEXT_SHA256.to_string(),
        pre_sha256: observed_pre,
        post_sha256: observed_post,
        repetition,
        executable_sha256: provenance.executable_sha256.clone(),
        rustc: provenance.rustc.clone(),
        cargo: provenance.cargo.clone(),
        toolchain_channel: provenance.toolchain_channel.clone(),
        build_profile: provenance.build_profile.clone(),
        features: provenance.features.clone(),
        host: HostBlock {
            os: provenance.host.os.clone(),
            kernel: provenance.host.kernel.clone(),
            arch: provenance.host.arch.clone(),
            cpu_model: provenance.host.cpu_model.clone(),
        },
        execution_command: execution_command.to_string(),
        counter_schema: contract::COUNTER_SCHEMA.to_string(),
        counters: counters_map,
        route,
        full_build_reason: reason,
        correctness,
        result_checksums: checksums,
        adjudication: AdjudicationBlock {
            status: adjudication.status().to_string(),
            reasons: adjudication.reasons().to_vec(),
        },
    }
}

/// Run ONE frozen observation (one cell, one repetition) and write
/// exactly one immutable raw row. See the module docs for the frozen
/// lifecycle and the recording-window boundary.
pub fn run_observation(
    cell: &'static FrozenCell,
    repetition: u64,
    execution_command: &str,
    output: &Path,
) -> RunOutcome {
    // Fail closed before any treatment work when the destination is not
    // a fresh, legal raw path.
    if let Err(e) = check_output_path(output) {
        return match e {
            super::writer::WriteError::OutputExists(_) => RunOutcome::DuplicateOutput,
            other => RunOutcome::ProducerFailure(other.to_string()),
        };
    }

    // ---- static provenance (PROCESS START) ----
    let provenance = match provenance::capture() {
        Ok(p) => p,
        Err(e) => return RunOutcome::ProducerFailure(e.to_string()),
    };

    let mut counters = HorseAStructuralCountersV1::new();
    let not_run = CorrectnessBlock {
        c1: "NOT_RUN".to_string(),
        c2: "NOT_RUN".to_string(),
        c3: "NOT_RUN".to_string(),
        normalized_structural_equality: false,
    };
    let zero_checksums = ResultChecksums {
        c1: 0,
        c2: 0,
        c3: 0,
    };

    // ---- exact frozen identities (verified before any treatment) ----
    let constructed = match contract::construct_and_verify(cell) {
        Ok(c) => c,
        Err(detail) => {
            let adjudication =
                Adjudication::Invalid(vec![format!("cell_identity_failure:{detail}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                None,
                &counters,
                not_run,
                zero_checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::IdentityFailure;
        }
    };

    // ---- C1: the pre-edit Horse-A full build, NO-OP structural sink ----
    let old = match full_build(
        &constructed.pre,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    ) {
        Ok(doc) => doc,
        Err(e) => {
            let adjudication = Adjudication::Invalid(vec![format!("c1_full_build_failed:{e}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                CorrectnessBlock {
                    c1: "FAIL".to_string(),
                    c2: "NOT_RUN".to_string(),
                    c3: "NOT_RUN".to_string(),
                    normalized_structural_equality: false,
                },
                zero_checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::CorrectnessFailure;
        }
    };
    let c1_checksum = match gate_ready_equals_h0(&old, constructed.pre.as_bytes()) {
        Ok(cs) => cs,
        Err(detail) => {
            let adjudication = Adjudication::Fail(vec![format!("correctness:c1:{detail}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                CorrectnessBlock {
                    c1: "FAIL".to_string(),
                    c2: "NOT_RUN".to_string(),
                    c3: "NOT_RUN".to_string(),
                    normalized_structural_equality: false,
                },
                zero_checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::CorrectnessFailure;
        }
    };

    // ---- RECORDING WINDOW: exactly the frozen primary native update
    // (stage → prepare → commit, retirement included). Nothing else runs
    // with the recording sink. ----
    let mut recording = RecordingHorseAStructuralSink::new();
    let next = match update_with_structural(
        old,
        &constructed.pre,
        &constructed.post,
        &constructed.edit,
        &mut NoopWorkSink,
        &mut recording,
    ) {
        Ok(doc) => doc,
        Err(e) => {
            // A pre-frontier refusal means no primary observation exists;
            // the partial record is archived as INVALID instrumentation
            // state, never as structural evidence.
            counters = recording.into_counters();
            let adjudication = Adjudication::Invalid(vec![format!("primary_update_refused:{e}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                CorrectnessBlock {
                    c1: "PASS".to_string(),
                    c2: "NOT_RUN".to_string(),
                    c3: "NOT_RUN".to_string(),
                    normalized_structural_equality: false,
                },
                ResultChecksums {
                    c1: c1_checksum,
                    c2: 0,
                    c3: 0,
                },
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::InvalidRow;
        }
    };
    counters = recording.into_counters();

    // ---- C2: normalized oracle on the primary update (NO-OP sink) ----
    let mut correctness = CorrectnessBlock {
        c1: "PASS".to_string(),
        c2: "PASS".to_string(),
        c3: "PASS".to_string(),
        normalized_structural_equality: true,
    };
    let mut checksums = ResultChecksums {
        c1: c1_checksum,
        c2: 0,
        c3: 0,
    };
    let c2_checksum = gate_ready_equals_h0(&next, constructed.post.as_bytes());
    match c2_checksum {
        Ok(cs) => checksums.c2 = cs,
        Err(detail) => {
            correctness.c2 = "FAIL".to_string();
            correctness.normalized_structural_equality = false;
            let adjudication = Adjudication::Fail(vec![format!("correctness:c2:{detail}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                correctness,
                checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::CorrectnessFailure;
        }
    }

    // ---- C3: the restore probe on the SAME returned state (NO-OP
    // sink; outside the primary structural verdict) ----
    let restore_start = constructed.edit.start_byte() as usize;
    let restore = match CanonicalEdit::new(
        restore_start,
        restore_start + contract::INSERTED_TEXT.len(),
        "",
    ) {
        Ok(e) => e,
        Err(e) => return RunOutcome::ProducerFailure(format!("restore edit geometry: {e}")),
    };
    let restored_source = match restore.apply(&constructed.post, SourceId(3)) {
        Ok(s) => s,
        Err(e) => return RunOutcome::ProducerFailure(format!("restore edit applies: {e}")),
    };
    if restored_source.as_bytes() != constructed.pre.as_bytes() {
        return RunOutcome::ProducerFailure(
            "restored source is not byte-identical to the frozen pre source".to_string(),
        );
    }
    let restored = match crate::update::update(
        next,
        &constructed.post,
        &restored_source,
        &restore,
        &mut NoopWorkSink,
    ) {
        Ok(doc) => doc,
        Err(e) => {
            correctness.c3 = "FAIL".to_string();
            correctness.normalized_structural_equality = false;
            let adjudication = Adjudication::Fail(vec![format!("correctness:c3_update:{e}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                correctness,
                checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::CorrectnessFailure;
        }
    };
    match gate_ready_equals_h0(&restored, restored_source.as_bytes()) {
        Ok(cs) => checksums.c3 = cs,
        Err(detail) => {
            correctness.c3 = "FAIL".to_string();
            correctness.normalized_structural_equality = false;
            let adjudication = Adjudication::Fail(vec![format!("correctness:c3:{detail}")]);
            let row = assemble_row(
                cell,
                repetition,
                &provenance,
                Some(&constructed),
                &counters,
                correctness,
                checksums,
                &adjudication,
                execution_command,
            );
            if let Err(e) = write_row(output, &row) {
                return RunOutcome::ProducerFailure(e.to_string());
            }
            return RunOutcome::CorrectnessFailure;
        }
    }

    // ---- adjudicate + write exactly one immutable row ----
    let pre_row = assemble_row(
        cell,
        repetition,
        &provenance,
        Some(&constructed),
        &counters,
        correctness.clone(),
        checksums.clone(),
        &Adjudication::Pass,
        execution_command,
    );
    // Adjudicate over the row as it will be read back (the embedded
    // adjudication block is filled in afterwards).
    let adjudication = adjudicate(cell, &pre_row);
    let row = assemble_row(
        cell,
        repetition,
        &provenance,
        Some(&constructed),
        &counters,
        correctness,
        checksums,
        &adjudication,
        execution_command,
    );
    if let Err(e) = write_row(output, &row) {
        return RunOutcome::ProducerFailure(e.to_string());
    }
    match adjudication {
        Adjudication::Pass => RunOutcome::Pass,
        Adjudication::Fail(_) => RunOutcome::StructuralFail,
        Adjudication::Invalid(_) => RunOutcome::InvalidRow,
    }
}

/// `--validate-only` (NON-TREATMENT): verify StudyId/contract
/// self-consistency, every cell's constructed byte identity (pre, post,
/// inserted), the threshold-table transcription, the schema round trip,
/// the toolchain/build metadata and output-path legality. Never executes
/// a recording-mode primary update; never full-builds.
pub fn validate_only(
    cell: Option<&'static FrozenCell>,
    output: Option<&Path>,
) -> Result<String, String> {
    if contract::STUDY_ID.len() != 64 {
        return Err("study_id is not a 64-hex sha256".to_string());
    }
    if contract::FROZEN_CONTRACT_REVISION.is_empty() {
        return Err("frozen_contract_revision is empty".to_string());
    }
    let cells: Vec<&'static FrozenCell> = match cell {
        Some(c) => vec![c],
        None => contract::FROZEN_CELLS.iter().collect(),
    };
    let mut report = String::new();
    for cell in &cells {
        if !cell.threshold_table_is_consistent() {
            return Err(format!(
                "{}: threshold-table transcription inconsistent",
                cell.cell_id
            ));
        }
        // Full byte-identity verification (constructs + hashes; no
        // update, no full build).
        contract::construct_and_verify(cell)
            .map_err(|detail| format!("{}: {detail}", cell.cell_id))?;
        report.push_str(&format!(
            "{}: identity + threshold table VERIFIED\n",
            cell.cell_id
        ));
    }
    // Provenance/toolchain/build metadata capture.
    let provenance = provenance::capture().map_err(|e| e.to_string())?;
    report.push_str(&format!(
        "provenance: commit {} tree {} profile {} features {}\n",
        provenance.repository_commit,
        provenance.repository_tree,
        provenance.build_profile,
        provenance.features
    ));
    report.push_str(&format!(
        "toolchain: channel {} / {} / {}\n",
        provenance.toolchain_channel, provenance.rustc, provenance.cargo
    ));
    // Schema round trip of a synthetic (non-treatment) row.
    let sample = RawRowV1 {
        schema: contract::RAW_SCHEMA.to_string(),
        row_identity: "0".repeat(64),
        study_id: contract::STUDY_ID.to_string(),
        protocol: contract::PROTOCOL.to_string(),
        frozen_contract_revision: contract::FROZEN_CONTRACT_REVISION.to_string(),
        study_mechanism_baseline: contract::STUDY_MECHANISM_BASELINE.to_string(),
        authorization_baseline: contract::AUTHORIZATION_BASELINE.to_string(),
        repository_commit: provenance.repository_commit.clone(),
        repository_tree: provenance.repository_tree.clone(),
        mechanism: contract::MECHANISM.to_string(),
        mechanism_design_head: contract::MECHANISM_DESIGN_HEAD.to_string(),
        mechanism_merge: contract::MECHANISM_MERGE.to_string(),
        cell_id: "synthetic".to_string(),
        case_id_hex: "0".repeat(64),
        n_bytes: 0,
        m: 0,
        target: 0,
        edit_start: 0,
        edit_end: 0,
        inserted_text_sha256: contract::INSERTED_TEXT_SHA256.to_string(),
        pre_sha256: "0".repeat(64),
        post_sha256: "0".repeat(64),
        repetition: 0,
        executable_sha256: provenance.executable_sha256.clone(),
        rustc: provenance.rustc.clone(),
        cargo: provenance.cargo.clone(),
        toolchain_channel: provenance.toolchain_channel.clone(),
        build_profile: provenance.build_profile.clone(),
        features: provenance.features.clone(),
        host: HostBlock {
            os: provenance.host.os.clone(),
            kernel: provenance.host.kernel.clone(),
            arch: provenance.host.arch.clone(),
            cpu_model: provenance.host.cpu_model.clone(),
        },
        execution_command: "validate-only-schema-round-trip".to_string(),
        counter_schema: contract::COUNTER_SCHEMA.to_string(),
        counters: RawRowV1::counters_map(&HorseAStructuralCountersV1::new()),
        route: "local".to_string(),
        full_build_reason: "none".to_string(),
        correctness: CorrectnessBlock {
            c1: "NOT_RUN".to_string(),
            c2: "NOT_RUN".to_string(),
            c3: "NOT_RUN".to_string(),
            normalized_structural_equality: false,
        },
        result_checksums: ResultChecksums {
            c1: 0,
            c2: 0,
            c3: 0,
        },
        adjudication: AdjudicationBlock {
            status: "INVALID".to_string(),
            reasons: vec!["synthetic validate-only row".to_string()],
        },
    };
    let json = serde_json::to_string(&sample).map_err(|e| format!("schema serialize: {e}"))?;
    serde_json::from_str::<RawRowV1>(&json).map_err(|e| format!("schema round trip: {e}"))?;
    report.push_str("schema: HORSE-A-STRUCTURAL-RAW-v1 round trip OK\n");
    if let Some(path) = output {
        check_output_path(path).map_err(|e| e.to_string())?;
        report.push_str(&format!("output path legal: {}\n", path.display()));
    }
    report.push_str("VALIDATE_ONLY: PASS (no recording-mode update executed)\n");
    Ok(report)
}

/// `--print-provenance` (NON-TREATMENT): the captured static provenance
/// as JSON.
pub fn print_provenance_json() -> Result<String, String> {
    let p = provenance::capture().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&p).map_err(|e| format!("serialize provenance: {e}"))
}
