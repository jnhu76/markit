//! Per-raw-file receipts (task §50) and finalization (task §48).
//!
//! A raw file is not final because the writer stopped: it is final when
//! its CONTENT has been re-read and checked against the frozen contract
//! that produced it. This module performs that check and writes the
//! receipt; a file that fails is retained unchanged as failure evidence
//! and never gets a success receipt.

use std::path::Path;

use crate::envelope::Campaign2ObservationV1;

/// Receipt schema tag.
pub const RECEIPT_SCHEMA: &str = "campaign2-raw-receipt-v1";

/// One raw file's receipt (task §50): every field the task lists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawReceiptV1 {
    pub schema: String,
    pub study_id: String,
    pub campaign_spec_id: String,
    pub sub_campaign_spec_id: String,
    pub run_id: String,
    pub authority_sha: String,
    pub executable_sha256: String,
    pub machine_id: String,
    pub lane: String,
    pub surface: String,
    pub raw_path: String,
    pub row_count: u64,
    pub byte_size: u64,
    pub sha256: String,
    pub complete_marker: String,
    pub exit_status: i32,
    /// Distinct identity values seen in the file (must each be exactly 1).
    pub distinct_identity: DistinctIdentityV1,
}

/// Identity cardinality of one raw file — the structural guarantee that a
/// file never mixes identities.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistinctIdentityV1 {
    pub study_ids: u64,
    pub campaign_spec_ids: u64,
    pub sub_campaign_spec_ids: u64,
    pub run_ids: u64,
    pub schemas: u64,
    pub surfaces: u64,
    pub observation_ids: u64,
    pub duplicate_observation_ids: u64,
}

/// Parse a raw JSONL file into observations, failing on malformed rows.
pub fn read_observations(path: &Path) -> Result<Vec<Campaign2ObservationV1>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let row: Campaign2ObservationV1 = serde_json::from_slice(line)
            .map_err(|e| format!("{}: row {index}: {e}", path.display()))?;
        out.push(row);
    }
    Ok(out)
}

/// Compute the identity cardinality of a parsed raw file.
pub fn distinct_identity(rows: &[Campaign2ObservationV1]) -> DistinctIdentityV1 {
    use std::collections::BTreeSet;
    let set = |f: &dyn Fn(&Campaign2ObservationV1) -> String| -> u64 {
        rows.iter().map(f).collect::<BTreeSet<String>>().len() as u64
    };
    let ids: Vec<String> = rows.iter().map(|r| r.observation_id.clone()).collect();
    let unique = ids.iter().collect::<BTreeSet<&String>>().len() as u64;
    DistinctIdentityV1 {
        study_ids: set(&|r| r.study_id.clone()),
        campaign_spec_ids: set(&|r| r.campaign_spec_id.clone()),
        sub_campaign_spec_ids: set(&|r| r.sub_campaign_spec_id.clone()),
        run_ids: set(&|r| r.run_id.clone()),
        schemas: set(&|r| r.schema.clone()),
        surfaces: set(&|r| r.surface.clone()),
        observation_ids: unique,
        duplicate_observation_ids: ids.len() as u64 - unique,
    }
}

/// Build the receipt for a raw file that has already been verified.
#[allow(clippy::too_many_arguments)]
pub fn build_receipt(
    raw_path: &Path,
    relative_path: &str,
    rows: &[Campaign2ObservationV1],
    study_id: &str,
    campaign_spec_id: &str,
    sub_campaign_spec_id: &str,
    run_id: &str,
    authority_sha: &str,
    executable_sha256: &str,
    machine_id: &str,
    lane: &str,
    surface: &str,
    complete_marker: &str,
) -> Result<RawReceiptV1, String> {
    let meta = std::fs::metadata(raw_path).map_err(|e| format!("stat {}: {e}", raw_path.display()))?;
    Ok(RawReceiptV1 {
        schema: RECEIPT_SCHEMA.to_string(),
        study_id: study_id.to_string(),
        campaign_spec_id: campaign_spec_id.to_string(),
        sub_campaign_spec_id: sub_campaign_spec_id.to_string(),
        run_id: run_id.to_string(),
        authority_sha: authority_sha.to_string(),
        executable_sha256: executable_sha256.to_string(),
        machine_id: machine_id.to_string(),
        lane: lane.to_string(),
        surface: surface.to_string(),
        raw_path: relative_path.to_string(),
        row_count: rows.len() as u64,
        byte_size: meta.len(),
        sha256: crate::sha256_file(raw_path)?,
        complete_marker: complete_marker.to_string(),
        exit_status: 0,
        distinct_identity: distinct_identity(rows),
    })
}

/// Write a receipt next to its raw file (`<raw>.receipt.json`).
pub fn write_receipt(raw_path: &Path, receipt: &RawReceiptV1) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(format!("{}.receipt.json", raw_path.display()));
    let bytes = serde_json::to_vec_pretty(receipt).map_err(|e| format!("serialize receipt: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Re-verify a raw file against its receipt (byte hash + row count).
pub fn verify_receipt(raw_path: &Path, receipt: &RawReceiptV1) -> Result<(), Vec<String>> {
    let mut blockers = Vec::new();
    let meta = match std::fs::metadata(raw_path) {
        Ok(meta) => meta,
        Err(e) => {
            blockers.push(format!("stat {}: {e}", raw_path.display()));
            return Err(blockers);
        }
    };
    if meta.len() != receipt.byte_size {
        blockers.push(format!(
            "{}: byte size {} != receipt {}",
            raw_path.display(),
            meta.len(),
            receipt.byte_size
        ));
    }
    match crate::sha256_file(raw_path) {
        Ok(sha) if sha == receipt.sha256 => {}
        Ok(sha) => blockers.push(format!(
            "{}: sha256 {sha} != receipt {}",
            raw_path.display(),
            receipt.sha256
        )),
        Err(e) => blockers.push(e),
    }
    match read_observations(raw_path) {
        Ok(rows) => {
            if rows.len() as u64 != receipt.row_count {
                blockers.push(format!(
                    "{}: row count {} != receipt {}",
                    raw_path.display(),
                    rows.len(),
                    receipt.row_count
                ));
            }
            let distinct = distinct_identity(&rows);
            if distinct.run_ids != 1 || distinct.campaign_spec_ids != 1 || distinct.study_ids != 1 {
                blockers.push(format!(
                    "{}: identity cardinality study/campaign/run = {}/{}/{} (must be 1/1/1)",
                    raw_path.display(),
                    distinct.study_ids,
                    distinct.campaign_spec_ids,
                    distinct.run_ids
                ));
            }
            if distinct.duplicate_observation_ids != 0 {
                blockers.push(format!(
                    "{}: {} duplicate observation ids",
                    raw_path.display(),
                    distinct.duplicate_observation_ids
                ));
            }
        }
        Err(e) => blockers.push(e),
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// Print the frozen completion marker for a finalized raw file.
pub fn print_final_pass(path: &Path, receipt: &RawReceiptV1) {
    println!(
        "FINAL_RAW_FILE_PASS file={} rows={} sha256={}",
        path.display(),
        receipt.row_count,
        receipt.sha256
    );
}
