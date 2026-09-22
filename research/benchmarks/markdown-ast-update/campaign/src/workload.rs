//! Frozen-workload consumption for the primary campaign (task §3, §6,
//! §23).
//!
//! #35 owns selection; this loader only CONSUMES the frozen artifacts:
//!
//! ```text
//! workloads/selections/selected-files-v1.json (+ repair overlay)
//!   -> hash-verified source bytes (load_selected_files, fail-closed)
//! workloads/payloads/full-read-manifest-v1.jsonl
//!   -> the 22 G0_STRICT_FULL_READ cases (Surface A / CLEAN_STATE)
//! workloads/payloads/edit-write-manifest-v1.jsonl
//!   -> the 362 G0_PRIMARY cases (Surface B / EDIT_WRITE)
//! ```
//!
//! Nothing here reselects, regenerates, or repairs workload members. Any
//! drift between on-disk bytes and the frozen digests is a hard error.
//!
//! ## FULL_READ CaseId (task §23)
//!
//! The existing `CaseKeyV1` machinery fully represents a clean-parse
//! case (`OperationKind::FullParse` carries no edit fields), so the
//! campaign reuses it — no second hashing algorithm is invented:
//!
//! ```text
//! CaseKeyV1 {
//!     payload_id:      "full-read:<source_key>",
//!     payload_shape:   Mixed,
//!     payload_size_bytes: <file bytes>,
//!     old_source_sha256:  <frozen source sha>,
//!     operation:       FullParse,
//!     generator_id:    CORRECTIVE-C-WORKLOAD-FREEZE-v1,
//! }
//! ```
//!
//! The id depends on the frozen source identity + the FULL_PARSE
//! operation only — never on horse, session, or schedule order.
//!
//! ## EDIT_WRITE CaseId
//!
//! Derived by the SAME frozen function the #35 dry-run used
//! (`workload_freeze::dryrun::case_id_of`), so campaign rows and dry-run
//! rows share identities by construction.

use std::collections::BTreeMap;

use markit_mdbench_common::{
    CanonicalEdit, CaseId, CaseKeyV1, OperationKind, PayloadShape, Source, SourceId,
};
use markit_mdbench_semantics::payload::{validate_payload, PayloadRecord};
use markit_mdbench_workload_freeze::dryrun::{case_id_of, read_jsonl};
use markit_mdbench_workload_freeze::fullread::FullReadRecord;
use markit_mdbench_workload_freeze::{SelectedFile, CASE_GENERATOR_ID, MEMBERSHIP_G0_PRIMARY};

/// One CLEAN_STATE case: 22 G0-strict files (Surface A).
#[derive(Debug, Clone)]
pub struct CleanStateCase {
    pub case_id: CaseId,
    pub case_id_hex: String,
    pub payload_id: String,
    pub source_key: String,
    pub source_id: String,
    pub file_bytes: u64,
    pub source_sha256: String,
    /// The exact materialized (hash-verified) source bytes.
    pub source_text: String,
}

/// One EDIT_WRITE case: one frozen G0_PRIMARY payload (Surface B).
#[derive(Debug, Clone)]
pub struct EditWriteCase {
    pub case_id: CaseId,
    pub case_id_hex: String,
    pub payload_id: String,
    pub trace_id: String,
    pub source_id: String,
    pub source_path: String,
    pub edit_family: String,
    pub expected_transition: String,
    /// Exact frozen pre-edit source (step 0: the file bytes; step >= 1:
    /// base bytes with the trace's earlier edits applied).
    pub pre_source_text: String,
    /// Exact frozen post-edit source (pre + this payload's edit).
    pub post_source_text: String,
    /// The payload's canonical edit.
    pub edit: CanonicalEdit,
    pub pre_len_bytes: u64,
    /// Logical edited bytes (PA denominator; canonical edit length).
    pub logical_edited_bytes: u64,
}

/// The consumed frozen workload: both primary surfaces, materialized.
#[derive(Debug, Clone)]
pub struct CampaignWorkload {
    pub clean_state: Vec<CleanStateCase>,
    pub edit_write: Vec<EditWriteCase>,
}

/// Expected frozen cardinalities (task §3/§6).
pub const EXPECTED_CLEAN_STATE_CASES: usize = 22;
pub const EXPECTED_EDIT_WRITE_CASES: usize = 362;

fn hex_to_32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("digest {hex:?} is not 64 hex chars"));
    }
    (0..32)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|e| format!("digest {hex:?} byte {i}: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "digest length".to_string())
}

/// FULL_READ CaseId via the frozen CaseKeyV1 machinery (task §23).
pub fn clean_state_case_id(record: &FullReadRecord) -> Result<CaseId, String> {
    let key = CaseKeyV1 {
        payload_id: format!("full-read:{}", record.source_key),
        // Real files claim no synthetic corpus shape; `mixed` is the
        // documented neutral shape for real-workload adapters (same
        // convention as the #35 dry-run).
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: record.file_bytes,
        old_source_sha256: hex_to_32(&record.source_sha256)?,
        operation: OperationKind::FullParse,
        edit_start_byte: None,
        edit_end_byte: None,
        inserted_text_sha256: None,
        generator_id: Some(CASE_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .map_err(|e| format!("full-read case key for {}: {e:?}", record.source_key))?;
    Ok(CaseId::from_key(&key))
}

/// Load and materialize both primary surfaces from the frozen artifacts.
///
/// Fails closed on: source byte drift, missing sources, non-G0-strict
/// FULL_READ membership counts, G1 payload leakage into the G0 surface,
/// payload re-validation failure, edit/post-source reconstruction
/// mismatch, and CaseId derivation failure.
pub fn load_campaign_workload(
    benchmark_root: &std::path::Path,
) -> Result<CampaignWorkload, String> {
    // Hash-verified source materialization (shared with #35 tooling).
    let files: Vec<SelectedFile> =
        markit_mdbench_workload_freeze::load_selected_files(benchmark_root)?;
    let sources: BTreeMap<String, &str> = files
        .iter()
        .map(|f| (f.key.clone(), f.text.as_str()))
        .collect();

    // ---- Surface A: CLEAN_STATE (22 G0-strict FULL_READ files) --------
    let full_read: Vec<FullReadRecord> =
        read_jsonl(&benchmark_root.join("workloads/payloads/full-read-manifest-v1.jsonl"))?;
    let mut clean_state = Vec::new();
    for record in &full_read {
        let strict = record
            .lanes
            .iter()
            .any(|lane| lane.case_class == "G0_STRICT_FULL_READ");
        if !strict {
            continue;
        }
        let source_text = sources
            .get(record.source_key.as_str())
            .ok_or_else(|| format!("FULL_READ source {} not materialized", record.source_key))?;
        if crate::sha256_hex(source_text.as_bytes()) != record.source_sha256 {
            return Err(format!(
                "FULL_READ source {} bytes do not match frozen sha256",
                record.source_key
            ));
        }
        if source_text.len() as u64 != record.file_bytes {
            return Err(format!(
                "FULL_READ source {} byte count {} != frozen {}",
                record.source_key,
                source_text.len(),
                record.file_bytes
            ));
        }
        clean_state.push(CleanStateCase {
            case_id: clean_state_case_id(record)?,
            case_id_hex: String::new(),
            payload_id: format!("full-read:{}", record.source_key),
            source_key: record.source_key.clone(),
            source_id: record.source_id.clone(),
            file_bytes: record.file_bytes,
            source_sha256: record.source_sha256.clone(),
            source_text: source_text.to_string(),
        });
    }
    let clean_state = clean_state
        .into_iter()
        .map(|mut case| {
            case.case_id_hex = case.case_id.hex();
            case
        })
        .collect::<Vec<_>>();
    if clean_state.len() != EXPECTED_CLEAN_STATE_CASES {
        return Err(format!(
            "G0-strict FULL_READ cases {} != frozen {EXPECTED_CLEAN_STATE_CASES}",
            clean_state.len()
        ));
    }

    // ---- Surface B: EDIT_WRITE (362 G0_PRIMARY payloads) --------------
    let payloads: Vec<PayloadRecord> =
        read_jsonl(&benchmark_root.join("workloads/payloads/edit-write-manifest-v1.jsonl"))?;
    // Reconstruct each trace's pre sources from the frozen base + the
    // trace's earlier steps (the same lifecycle reconstruction the #35
    // dry-run performs; SINGLE_RESET semantics, task §4).
    let mut by_trace: BTreeMap<&str, Vec<&PayloadRecord>> = BTreeMap::new();
    for payload in &payloads {
        by_trace
            .entry(payload.trace_id.as_str())
            .or_default()
            .push(payload);
    }
    let mut edit_write = Vec::new();
    for (_trace_id, mut group) in by_trace {
        group.sort_by_key(|payload| payload.step);
        let base_payload = group[0];
        let base_source = sources
            .get(base_payload.source_path.as_str())
            .ok_or_else(|| {
                format!(
                    "payload source {} not materialized",
                    base_payload.source_path
                )
            })?;
        if base_payload.grammar_id != crate::G0_GRAMMAR_ID
            || !base_payload
                .memberships
                .iter()
                .any(|m| m == MEMBERSHIP_G0_PRIMARY)
        {
            // G1 table payloads are never dispatched into H0-H4.
            continue;
        }
        for payload in &group {
            let pre_source_text = if payload.step == 0 {
                (*base_source).to_string()
            } else {
                let step0 = group
                    .iter()
                    .find(|candidate| candidate.step == 0)
                    .expect("trace has step 0");
                step0.edit.apply(base_source).map_err(|e| {
                    format!("trace reconstruction for {}: {e:?}", payload.payload_id)
                })?
            };
            let post_source_text = payload
                .edit
                .apply(&pre_source_text)
                .map_err(|e| format!("post source for {}: {e:?}", payload.payload_id))?;
            // Re-validate the frozen payload from manifest bytes
            // (lifecycle + transition oracle; fail closed like the
            // #35 dry-run).
            let validation = validate_payload(payload, &pre_source_text, base_source);
            if !validation.valid {
                return Err(format!(
                    "frozen payload {} failed re-validation: {:?}",
                    payload.payload_id, validation.failure_codes
                ));
            }
            let edit = payload
                .edit
                .to_canonical()
                .map_err(|e| format!("canonical edit for {}: {e:?}", payload.payload_id))?;
            let logical_edited_bytes =
                (edit.end_byte() - edit.start_byte()).max(edit.inserted_text().len() as u64);
            // CaseId: build the SAME CaseKeyV1 the #35 dry-run builds and
            // ASSERT equality with `case_id_of`'s output — identical ids
            // by construction, never two derivations that could drift.
            let dry_run_hex = case_id_of(payload, pre_source_text.len() as u64)?;
            let key = CaseKeyV1 {
                payload_id: payload.payload_id.clone(),
                payload_shape: PayloadShape::Mixed,
                payload_size_bytes: pre_source_text.len() as u64,
                old_source_sha256: hex_to_32(&payload.pre_source_sha256)?,
                operation: payload.edit.operation_kind(),
                edit_start_byte: payload
                    .edit
                    .operation_kind()
                    .has_edit()
                    .then_some(payload.edit.edit_start),
                edit_end_byte: payload
                    .edit
                    .operation_kind()
                    .has_edit()
                    .then_some(payload.edit.edit_end),
                inserted_text_sha256: payload
                    .edit
                    .operation_kind()
                    .has_inserted_text()
                    .then_some(hex_to_32(&payload.inserted_sha256)?),
                generator_id: Some(CASE_GENERATOR_ID.to_string()),
                generator_seed: None,
            }
            .validated()
            .map_err(|e| format!("case key for {}: {e:?}", payload.payload_id))?;
            let case_id = CaseId::from_key(&key);
            if case_id.hex() != dry_run_hex {
                return Err(format!(
                    "EDIT_WRITE CaseId derivation drift for {}: campaign {:?} != dry-run {dry_run_hex:?}",
                    payload.payload_id,
                    case_id.hex()
                ));
            }
            edit_write.push(EditWriteCase {
                case_id,
                case_id_hex: dry_run_hex,
                payload_id: payload.payload_id.clone(),
                trace_id: payload.trace_id.clone(),
                source_id: payload.source_id.clone(),
                source_path: payload.source_path.clone(),
                edit_family: payload.edit_family.clone(),
                expected_transition: payload.expected_transition.clone(),
                pre_source_text,
                post_source_text,
                edit,
                pre_len_bytes: 0, // filled after the move below
                logical_edited_bytes,
            });
        }
    }
    let edit_write = edit_write
        .into_iter()
        .map(|mut case| {
            case.pre_len_bytes = case.pre_source_text.len() as u64;
            case
        })
        .collect::<Vec<_>>();
    if edit_write.len() != EXPECTED_EDIT_WRITE_CASES {
        return Err(format!(
            "G0_PRIMARY EDIT_WRITE cases {} != frozen {EXPECTED_EDIT_WRITE_CASES}",
            edit_write.len()
        ));
    }
    // No duplicate case ids (identity collision would break ObservationId
    // uniqueness).
    let unique: std::collections::BTreeSet<&str> =
        edit_write.iter().map(|c| c.case_id_hex.as_str()).collect();
    if unique.len() != edit_write.len() {
        return Err("duplicate EDIT_WRITE case ids in frozen workload".to_string());
    }
    let unique_clean: std::collections::BTreeSet<&str> =
        clean_state.iter().map(|c| c.case_id_hex.as_str()).collect();
    if unique_clean.len() != clean_state.len() {
        return Err("duplicate CLEAN_STATE case ids in frozen workload".to_string());
    }

    Ok(CampaignWorkload {
        clean_state,
        edit_write,
    })
}

/// Build the schedule-generation inputs from the consumed workload.
pub fn schedule_inputs(workload: &CampaignWorkload) -> Vec<crate::schedule::SurfaceCases> {
    vec![
        crate::schedule::SurfaceCases {
            surface: crate::Surface::CleanState,
            cases: workload
                .clean_state
                .iter()
                .map(|c| {
                    (
                        c.case_id_hex.clone(),
                        c.payload_id.clone(),
                        c.source_key.clone(),
                        None,
                    )
                })
                .collect(),
        },
        crate::schedule::SurfaceCases {
            surface: crate::Surface::EditWrite,
            cases: workload
                .edit_write
                .iter()
                .map(|c| {
                    (
                        c.case_id_hex.clone(),
                        c.payload_id.clone(),
                        c.source_path.clone(),
                        Some(c.trace_id.clone()),
                    )
                })
                .collect(),
        },
    ]
}

/// Construct the runner `Source` handles for one EDIT_WRITE case.
pub fn edit_case_sources(case: &EditWriteCase) -> (Source, Source) {
    (
        Source::new(SourceId(0), case.pre_source_text.clone()),
        Source::new(SourceId(1), case.post_source_text.clone()),
    )
}
