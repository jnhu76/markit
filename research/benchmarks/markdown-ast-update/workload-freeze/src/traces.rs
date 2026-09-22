//! A7 — the frozen trace manifest (task §41-§42).
//!
//! The primary real performance workload is SINGLE_RESET: every primary
//! case starts from one frozen pre-state. BREAK/RESTORE pairs are two
//! single-reset legs sharing one trace identity whose chain correctness
//! (per-step coordinates belong to that step's pre-source, RESTORE starts
//! from S1, exact SHA restoration) is machine-proven at generation time.
//! `local_burst` / `document_session` remain lifecycle-supported but are
//! DEFERRED as performance campaigns; no chained campaign is created here.

use serde::{Deserialize, Serialize};

use markit_mdbench_semantics::payload::PayloadRecord;

use crate::TRACE_MANIFEST_SCHEMA;

/// One step of a frozen trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStepRecord {
    pub step: u32,
    pub payload_id: String,
    pub pre_source_sha256: String,
    pub post_source_sha256: String,
    pub expected_transition: String,
    /// Exact canonical edit of this step (coordinates belong to THIS
    /// step's pre-source).
    pub edit_start: u64,
    pub edit_end: u64,
    pub inserted_text: String,
}

/// One frozen trace record (canonical JSONL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceRecord {
    pub schema: String,
    pub generator_version: String,
    pub trace_id: String,
    pub trace_form: String,
    pub grammar_id: String,
    pub source_key: String,
    pub base_source_sha256: String,
    /// `single_step` (one frozen pre-state) or `break_restore_pair`
    /// (S0 -> S1 -> S2 with exact restoration, chain-proven).
    pub chain_kind: String,
    pub steps: Vec<TraceStepRecord>,
    /// Digest over the declared step sequence (identity of the content,
    /// not just the label).
    pub declared_steps_sha256: String,
    pub memberships: Vec<String>,
}

/// Digest over a canonical rendering of the steps.
fn steps_digest(steps: &[TraceStepRecord]) -> String {
    let canonical = markit_mdbench_semantics::canonical_json(&steps);
    markit_mdbench_semantics::sha256_hex(canonical.as_bytes())
}

/// Build the trace manifest from the frozen payloads (grouped by trace
/// identity, ordered by step).
pub fn build_traces(payloads: &[PayloadRecord]) -> Vec<TraceRecord> {
    let mut grouped: std::collections::BTreeMap<(&str, &str, &str), Vec<&PayloadRecord>> =
        std::collections::BTreeMap::new();
    for payload in payloads {
        grouped
            .entry((
                payload.trace_id.as_str(),
                payload.source_path.as_str(),
                payload.grammar_id.as_str(),
            ))
            .or_default()
            .push(payload);
    }

    let mut records = Vec::new();
    for ((trace_id, _source_key, grammar_id), mut group) in grouped {
        group.sort_by_key(|payload| payload.step);
        let base = group
            .iter()
            .find(|payload| payload.step == 0)
            .expect("every trace has its step-0 leg");
        let chain_kind = if group.len() == 1 {
            "single_step"
        } else {
            "break_restore_pair"
        };
        let steps: Vec<TraceStepRecord> = group
            .iter()
            .map(|payload| TraceStepRecord {
                step: payload.step,
                payload_id: payload.payload_id.clone(),
                pre_source_sha256: payload.pre_source_sha256.clone(),
                post_source_sha256: payload.post_source_sha256.clone(),
                expected_transition: payload.expected_transition.clone(),
                edit_start: payload.edit.edit_start,
                edit_end: payload.edit.edit_end,
                inserted_text: payload.edit.inserted_text.clone(),
            })
            .collect();
        records.push(TraceRecord {
            schema: TRACE_MANIFEST_SCHEMA.to_string(),
            generator_version: crate::CORRECTIVE_C_VERSION.to_string(),
            trace_id: trace_id.to_string(),
            trace_form: base.trace_form.name().to_string(),
            grammar_id: grammar_id.to_string(),
            source_key: base.source_path.clone(),
            base_source_sha256: base.base_source_sha256.clone(),
            chain_kind: chain_kind.to_string(),
            declared_steps_sha256: steps_digest(&steps),
            steps,
            memberships: base.memberships.clone(),
        });
    }
    records
}
