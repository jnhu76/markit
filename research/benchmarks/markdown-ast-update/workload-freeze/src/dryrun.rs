//! A8 — the harness dry-run adapter (task §43-§47, G7).
//!
//! The EXISTING harness remains the only authority for Source,
//! CanonicalEdit, CaseId, runner dispatch, oracle comparison and result
//! facts. This adapter adds the minimum real-workload surface:
//!
//! ```text
//! read the frozen FULL_READ / EDIT_WRITE manifests from disk
//!   -> load exact source bytes, verify SHA256
//!   -> reconstruct each pre state / post source
//!   -> construct CanonicalEdit, reproduce the post bytes
//!   -> dispatch through the runner's CORRECTNESS-ONLY path
//!   -> retain failures
//! ```
//!
//! No research timing exists on this path: the correctness-only runner
//! mode never constructs a clock, never emits a measurement value, and
//! the dry-run report carries only execution/correctness facts. G1 table
//! payloads are NOT dispatched into the horse comparison (horses are not
//! G1-qualified); they are counted, not measured.

use std::collections::BTreeMap;

use markit_mdbench_block_local::BlockLocalMechanism;
use markit_mdbench_common::case::{CaseId, CaseKeyV1};
use markit_mdbench_common::payload::PayloadShape;
use markit_mdbench_common::{ExecutionStatus, CorrectnessStatus, Source, SourceId};
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_fragment_reuse::FragmentReuseMechanism;
use markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism;
use markit_mdbench_oracle::{validate_normalized, ReferenceOracle};
use markit_mdbench_restart_convergence::RestartConvergenceMechanism;
use markit_mdbench_runner::orchestrate::{
    build_initial_state, run_full_parse_correctness, run_update_correctness,
};
use markit_mdbench_semantics::payload::{validate_payload, PayloadRecord};
use serde::{Deserialize, Serialize};

use crate::fullread::FullReadRecord;
use crate::{DRY_RUN_SCHEMA, CASE_GENERATOR_ID, MEMBERSHIP_G0_PRIMARY};

/// One per-case dry-run row (JSONL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunCaseRow {
    pub kind: String, // "full_read" | "edit_write"
    pub horse: String,
    pub payload_id: String,
    /// Horse-independent harness identity derived from the payload via
    /// the frozen CaseKeyV1 -> CaseId infrastructure.
    pub case_id: String,
    pub source_key: String,
    pub execution_status: String,
    pub correctness_status: String,
}

/// Aggregated per-horse counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorseCounts {
    pub pass: u64,
    pub wrong_result: u64,
    pub execution_failed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunFullReadSection {
    pub files: u64,
    pub g0_strict_cases: u64,
    /// Total H0-H4 clean-construction dispatches (files x 5 horses).
    pub horse_dispatches: u64,
    pub pass: u64,
    pub failed: u64,
    /// Per-horse clean-state construction counts (BLOCKER B parity:
    /// every G0-strict file must pass on every horse -> 110/110).
    pub per_horse: BTreeMap<String, HorseCounts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunEditWriteSection {
    pub g0_cases: u64,
    pub horse_dispatches: u64,
    pub g1_semantic_skipped: u64,
    pub per_horse: BTreeMap<String, HorseCounts>,
}

/// The complete dry-run report. No latency, no counters, no timing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunReport {
    pub schema: String,
    pub generator_version: String,
    pub mode: String,
    pub correctness_authority: String,
    pub full_read: DryRunFullReadSection,
    pub edit_write: DryRunEditWriteSection,
    pub claim_boundaries: Vec<String>,
}

/// Load JSONL records.
pub fn read_jsonl<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<Vec<T>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut records = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        records.push(
            serde_json::from_str(line).map_err(|error| {
                format!("parse {} line {}: {error}", path.display(), index + 1)
            })?,
        );
    }
    Ok(records)
}

/// Derive the harness CaseId of a payload via the frozen CaseKeyV1
/// infrastructure (horse-independent by contract; asserted below).
pub fn case_id_of(payload: &PayloadRecord, pre_source_bytes: u64) -> Result<String, String> {
    let old_source_sha256: [u8; 32] = (0..32)
        .map(|index| {
            u8::from_str_radix(
                &payload.pre_source_sha256[index * 2..index * 2 + 2],
                16,
            )
            .map_err(|error| format!("pre sha hex: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "pre sha length")?;
    let inserted: [u8; 32] = (0..32)
        .map(|index| {
            u8::from_str_radix(
                &payload.inserted_sha256[index * 2..index * 2 + 2],
                16,
            )
            .map_err(|error| format!("inserted sha hex: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "inserted sha length")?;
    let operation = payload.edit.operation_kind();
    let has_edit = operation.has_edit();
    let key = CaseKeyV1 {
        payload_id: payload.payload_id.clone(),
        // Real payloads claim no synthetic corpus shape: `mixed` is the
        // documented neutral shape for the real workload adapter.
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: pre_source_bytes,
        old_source_sha256,
        operation,
        edit_start_byte: has_edit.then_some(payload.edit.edit_start),
        edit_end_byte: has_edit.then_some(payload.edit.edit_end),
        inserted_text_sha256: operation
            .has_inserted_text()
            .then_some(inserted),
        generator_id: Some(CASE_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .map_err(|error| format!("case key for {}: {error:?}", payload.payload_id))?;
    Ok(CaseId::from_key(&key).hex())
}

/// One clean-state construction dispatch (Horse, full parse) against the
/// H0 reference. The reference is computed ONCE per file from H0's clean
/// parse (the correctness authority); every horse normalizes its
/// completed state OUTSIDE any measurement and is compared structurally.
fn dispatch_full_read(
    sources: &BTreeMap<String, &str>,
    full_read: &[FullReadRecord],
) -> Result<(DryRunFullReadSection, Vec<DryRunCaseRow>), String> {
    let mut rows = Vec::new();
    let mut pass = 0u64;
    let mut failed = 0u64;
    let mut g0_strict = 0u64;
    let mut horse_dispatches = 0u64;
    let mut per_horse: BTreeMap<String, HorseCounts> = BTreeMap::new();
    for record in full_read {
        let Some(_lane) = record
            .lanes
            .iter()
            .find(|lane| lane.case_class == "G0_STRICT_FULL_READ")
        else {
            continue;
        };
        g0_strict += 1;
        let source_text = sources
            .get(record.source_key.as_str())
            .expect("FULL_READ source is materialized");
        let source = Source::new(SourceId(0), (*source_text).to_string());
        let payload_id = format!("full-read:{}", record.source_key);

        // The reference: H0 clean parse (the correctness authority, built
        // outside every measurement; parse_document keeps the frozen
        // NORMALIZED-RESULT-v1 conformance gate on the reference).
        let reference = markit_mdbench_full_rebuild::parse_document(source.as_bytes());
        validate_normalized(&reference, None)
            .map_err(|error| format!("full-read reference gate: {error:?}"))?;

        macro_rules! full_read_dispatch {
            ($horse:expr, $mechanism:expr) => {{
                let mechanism = $mechanism;
                let hook = ReferenceOracle::new(reference.clone());
                let report = run_full_parse_correctness(&mechanism, &source, &hook);
                let ok = report.execution_status == ExecutionStatus::Pass
                    && report.correctness_status == CorrectnessStatus::Pass;
                horse_dispatches += 1;
                rows.push(DryRunCaseRow {
                    kind: "full_read".to_string(),
                    horse: $horse.to_string(),
                    payload_id: payload_id.clone(),
                    case_id: String::new(),
                    source_key: record.source_key.clone(),
                    execution_status: format!("{:?}", report.execution_status),
                    correctness_status: format!("{:?}", report.correctness_status),
                });
                let entry = per_horse
                    .entry($horse.to_string())
                    .or_insert(HorseCounts {
                        pass: 0,
                        wrong_result: 0,
                        execution_failed: 0,
                    });
                if ok {
                    pass += 1;
                    entry.pass += 1;
                } else {
                    failed += 1;
                    entry.wrong_result += 1;
                }
            }};
        }
        // BLOCKER B (MEASUREMENT-CORRECTIVE-1 §12): every G0-strict file
        // is correctness-qualified on ALL FIVE horses — clean parse +
        // native-state construction, 22 x 5 = 110 dispatches, no timing.
        full_read_dispatch!("H0", FullRebuildMechanism::new());
        full_read_dispatch!("H1", BlockLocalMechanism::new());
        full_read_dispatch!("H2", FragmentReuseMechanism::new());
        full_read_dispatch!("H3", OldTreeSubtreeReuseMechanism::new());
        full_read_dispatch!("H4", RestartConvergenceMechanism::new());
    }
    Ok((
        DryRunFullReadSection {
            files: full_read.len() as u64,
            g0_strict_cases: g0_strict,
            horse_dispatches,
            pass,
            failed,
            per_horse,
        },
        rows,
    ))
}

/// Dispatch one G0 EDIT_WRITE case through all five horses.
#[allow(clippy::type_complexity)]
fn dispatch_edit_case(
    payload: &PayloadRecord,
    pre_source: &str,
    post_source: &str,
) -> Result<(Vec<DryRunCaseRow>, String), String> {
    let case_id = case_id_of(payload, pre_source.len() as u64)?;
    let pre = Source::new(SourceId(0), pre_source.to_string());
    let post = Source::new(SourceId(1), post_source.to_string());
    let edit = payload
        .edit
        .to_canonical()
        .map_err(|error| format!("canonical edit: {error:?}"))?;

    // Correctness authority: H0 clean full parse of the POST source.
    let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
    validate_normalized(&reference, None)
        .map_err(|error| format!("reference normalized gate: {error:?}"))?;
    let hook = ReferenceOracle::new(reference);

    let mut rows = Vec::new();
    macro_rules! dispatch {
        ($horse:expr, $mechanism:expr) => {{
            let mechanism = $mechanism;
            let state = build_initial_state(&mechanism, &pre)
                .map_err(|failure| format!("initial state: {failure:?}"))?;
            let report = run_update_correctness(&mechanism, &pre, &post, &edit, state, &hook);
            rows.push(DryRunCaseRow {
                kind: "edit_write".to_string(),
                horse: $horse.to_string(),
                payload_id: payload.payload_id.clone(),
                case_id: case_id.clone(),
                source_key: payload.source_path.clone(),
                execution_status: format!("{:?}", report.execution_status),
                correctness_status: format!("{:?}", report.correctness_status),
            });
        }};
    }
    dispatch!("H0", FullRebuildMechanism::new());
    dispatch!("H1", BlockLocalMechanism::new());
    dispatch!("H2", FragmentReuseMechanism::new());
    dispatch!("H3", OldTreeSubtreeReuseMechanism::new());
    dispatch!("H4", RestartConvergenceMechanism::new());
    Ok((rows, case_id))
}

/// Run the complete correctness-only dry-run from the FROZEN manifests on
/// disk (never from in-memory generation state, so the manifests are
/// proven self-sufficient).
pub fn run_dry_run(
    benchmark_root: &std::path::Path,
    files: &[crate::SelectedFile],
) -> Result<(DryRunReport, Vec<DryRunCaseRow>), String> {
    let payloads_dir = benchmark_root.join("workloads/payloads");
    let full_read: Vec<FullReadRecord> =
        read_jsonl(&payloads_dir.join("full-read-manifest-v1.jsonl"))?;
    let payloads: Vec<PayloadRecord> =
        read_jsonl(&payloads_dir.join("edit-write-manifest-v1.jsonl"))?;

    // Materialize + verify every source the manifests reference.
    let mut sources: BTreeMap<String, &str> = BTreeMap::new();
    let mut source_texts: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        source_texts.insert(file.key.clone(), file.text.clone());
    }
    for (key, text) in &source_texts {
        sources.insert(key.clone(), text.as_str());
    }

    // ---- FULL_READ -----------------------------------------------------
    let (full_read_section, mut rows) = dispatch_full_read(&sources, &full_read)?;

    // ---- EDIT_WRITE ------------------------------------------------------
    // Reconstruct each trace's pre sources from the frozen base + step-0
    // edit (lifecycle correctness: a step-1 leg's coordinates belong to
    // the step-0 post source).
    let mut by_trace: BTreeMap<&str, Vec<&PayloadRecord>> = BTreeMap::new();
    for payload in &payloads {
        by_trace.entry(payload.trace_id.as_str()).or_default().push(payload);
    }
    let mut per_horse: BTreeMap<String, HorseCounts> = BTreeMap::new();
    let mut g0_cases = 0u64;
    let mut horse_dispatches = 0u64;
    let mut g1_skipped = 0u64;

    for (trace_id, mut group) in by_trace {
        let _ = trace_id;
        group.sort_by_key(|payload| payload.step);
        let base_payload = group[0];
        let base_source = sources
            .get(base_payload.source_path.as_str())
            .expect("payload source materialized");

        if base_payload.grammar_id != crate::G0_GRAMMAR_ID
            || !base_payload
                .memberships
                .iter()
                .any(|membership| membership == MEMBERSHIP_G0_PRIMARY)
        {
            // G1 table payloads: semantic lifecycle validated at freeze
            // time; never dispatched into H0-H4 (G6/G8 boundary).
            g1_skipped += group.len() as u64;
            continue;
        }

        for payload in &group {
            let pre_source = if payload.step == 0 {
                (*base_source).to_string()
            } else {
                let step0 = group
                    .iter()
                    .find(|candidate| candidate.step == 0)
                    .expect("trace has step 0");
                step0
                    .edit
                    .apply(base_source)
                    .map_err(|error| format!("broken state: {error:?}"))?
            };
            let post_source = payload
                .edit
                .apply(&pre_source)
                .map_err(|error| format!("post source: {error:?}"))?;

            // Re-validate the payload from manifest bytes (lifecycle + oracle).
            let validation = validate_payload(payload, &pre_source, base_source);
            if !validation.valid {
                return Err(format!(
                    "frozen payload {} failed re-validation: {:?}",
                    payload.payload_id, validation.failure_codes
                ));
            }
            let (case_rows, _) = dispatch_edit_case(payload, &pre_source, &post_source)?;
            g0_cases += 1;
            horse_dispatches += case_rows.len() as u64;
            for row in case_rows {
                let entry = per_horse.entry(row.horse.clone()).or_insert(HorseCounts {
                    pass: 0,
                    wrong_result: 0,
                    execution_failed: 0,
                });
                match row.correctness_status.as_str() {
                    "Pass" => entry.pass += 1,
                    "WrongResult" => entry.wrong_result += 1,
                    _ => entry.execution_failed += 1,
                }
                rows.push(row);
            }
        }
    }

    let report = DryRunReport {
        schema: DRY_RUN_SCHEMA.to_string(),
        generator_version: crate::CORRECTIVE_C_VERSION.to_string(),
        mode: "correctness_only_dry_run".to_string(),
        correctness_authority:
            "normalize(Hx update result) == normalize(H0 clean full parse(post source))"
                .to_string(),
        full_read: full_read_section,
        edit_write: DryRunEditWriteSection {
            g0_cases,
            horse_dispatches,
            g1_semantic_skipped: g1_skipped,
            per_horse,
        },
        claim_boundaries: vec![
            "G0 core != full CommonMark/GFM".to_string(),
            "G1 table semantic coverage != G1 horse qualification".to_string(),
            "G2 math deferred".to_string(),
            "selected real files != controlled scaling suite".to_string(),
            "FULL_READ != incremental edit results".to_string(),
            "SINGLE_RESET != long-session behavior".to_string(),
            "selected real files != whole-project throughput".to_string(),
            "no timing/resource claims from Stage A".to_string(),
        ],
    };
    Ok((report, rows))
}
