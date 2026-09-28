//! Shared run-finalization support for the formal execution binaries
//! (#76 M-COLLECTOR-1; relocated from the `mdbench-campaign` binary so
//! the dedicated memory binary reuses — never duplicates — the
//! receipt/finalization code).
//!
//! One `CampaignSpecId` carries exactly ONE `RunId` **per execution
//! binary**: the RunId binds the SHA256 of the executable that produces
//! the rows, and the frozen campaign runs TWO executables —
//! `mdbench-campaign` (timing + attribution, plain system allocator)
//! and `mdbench-memory` (formal M-LANE, `CountingAllocator` installed).
//! The lane-aware single-run-identity check below preserves the
//! anti-split rule for each binary: a sibling run directory blocks only
//! when it already holds THAT binary's lanes, so a rebuilt executable
//! still starts a new run or is archived deliberately — it never
//! silently continues an existing campaign run.

use std::path::{Path, PathBuf};

use crate::finalize::{RawFileExpectation, RawFileSummary};

/// The lane directories one execution binary owns: the campaign binary
/// owns timing + attribution (one executable, one RunId); the memory
/// binary owns the memory lane.
pub const CAMPAIGN_BINARY_LANES: [&str; 2] = ["timing", "attribution"];
pub const MEMORY_BINARY_LANES: [&str; 1] = ["memory"];

/// One spec must never be split across two executables of the same
/// lanes: if the spec's raw root already holds rows for any of `lanes`
/// under a DIFFERENT RunId, refuse. Sibling run directories holding
/// only OTHER binaries' lanes (e.g. the memory run next to a timing
/// run) are the frozen multi-binary architecture, not a split.
pub fn ensure_single_run_identity(
    spec_root: &Path,
    run_id: &str,
    lanes: &[&str],
) -> Result<(), String> {
    if !spec_root.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(spec_root).map_err(|e| format!("read {}: {e}", spec_root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", spec_root.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name == run_id {
            continue;
        }
        let sibling_holds_our_lanes = lanes.iter().any(|lane| {
            entry
                .path()
                .join(lane)
                .read_dir()
                .map(|mut dir| dir.next().is_some())
                .unwrap_or(false)
        });
        if sibling_holds_our_lanes {
            return Err(format!(
                "{} already holds raw data for run {name}, but this binary/commit/machine binds run \
                 {run_id}: a rebuilt executable is a NEW RunId, and one lane is never split \
                 across two binaries. Archive the existing run directory (a deliberate act) before \
                 starting a new campaign run.",
                spec_root.display()
            ));
        }
    }
    Ok(())
}

/// Read a finalized raw file back and check it against the frozen
/// contract. On success the verified facts enter the run receipt; on
/// failure the file is retained as evidence and marked invalid — it
/// NEVER gets a success receipt.
pub fn finalize_raw_file(
    raw_path: &Path,
    expectation: &RawFileExpectation,
    run_executable_sha256: &str,
) -> Result<bool, String> {
    match crate::finalize::verify_raw_file(raw_path, expectation) {
        Ok(summary) => {
            write_run_receipt(raw_path, &summary, run_executable_sha256)?;
            println!(
                "FINAL_RAW_FILE_PASS file={} rows={} sha256={}",
                raw_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default(),
                summary.rows,
                summary.sha256
            );
            Ok(true)
        }
        Err(blockers) => {
            let marker = write_invalid_marker(raw_path, &blockers)?;
            eprintln!(
                "FINAL_RAW_FILE_INVALID file={} detail={}",
                raw_path.display(),
                marker.display()
            );
            for blocker in &blockers {
                eprintln!("RAW_BLOCKER {blocker}");
            }
            Ok(false)
        }
    }
}

/// Write `<raw file>.invalid.json`: the failure evidence marker for a raw
/// file that did not pass finalization. Written once, never overwritten.
pub fn write_invalid_marker(raw_path: &Path, blockers: &[String]) -> Result<PathBuf, String> {
    let marker_path = raw_path.with_extension("jsonl.invalid.json");
    if marker_path.exists() {
        return Ok(marker_path);
    }
    let marker = serde_json::json!({
        "schema": "run-file-invalid-v1",
        "verdict": "PRIMARY_CAMPAIGN_INVALID",
        "file": raw_path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "producer_pid": std::process::id(),
        "blockers": blockers,
    });
    std::fs::write(&marker_path, format!("{marker}\n"))
        .map_err(|e| format!("write {}: {e}", marker_path.display()))?;
    Ok(marker_path)
}

/// Write `<raw file>.receipt.json` binding the VERIFIED finalized raw
/// file: SHA256, row counts, first/last observation id, campaign/run/
/// session identity (task §26/§39).
pub fn write_run_receipt(
    raw_path: &Path,
    summary: &RawFileSummary,
    run_executable_sha256: &str,
) -> Result<(), String> {
    // Process identity: the frozen session semantics require a FRESH
    // worker process per session (task §11); recording pid + kernel
    // start-time makes that auditable after the fact.
    let producer_pid = std::process::id();
    let producer_start_ticks = std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|text| {
            // Field 22 (1-indexed) is starttime; the comm field may
            // contain spaces, so split after the last ')'.
            let after_comm = text.rsplit_once(')').map(|(_, rest)| rest.to_string())?;
            after_comm
                .split_whitespace()
                .nth(19)
                .map(|value| value.to_string())
        });
    // The receipt is written by the binary that produced the rows; if
    // that binary changed mid-run the RunId would name a different
    // executable, which is a hard error rather than a recorded footnote.
    let executable_sha256 = crate::identity::current_executable_sha256()?;
    if executable_sha256 != run_executable_sha256 {
        return Err(format!(
            "the receipt-writing executable {executable_sha256} is not the executable bound into the \
             RunId ({run_executable_sha256})"
        ));
    }
    let receipt = serde_json::json!({
        "schema": "run-file-receipt-v1",
        "verdict": summary.verdict,
        "file": raw_path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "sha256": summary.sha256,
        "row_count": summary.rows,
        "warmup_rows": summary.warmup_rows,
        "measured_rows": summary.measured_rows,
        "attribution_rows": summary.attribution_rows,
        "expected_rows": summary.expected_rows,
        "unique_observation_ids": summary.unique_observation_ids,
        "case_count": summary.case_count,
        "lane": summary.lane,
        "surface": summary.surface,
        "session_ordinal": summary.session_ordinal,
        "campaign_spec_id": summary.campaign_spec_id,
        "run_id": summary.run_id,
        "session_id": summary.session_id,
        "first_observation_id": summary.first_observation_id,
        "last_observation_id": summary.last_observation_id,
        "producer_pid": producer_pid,
        "producer_start_ticks": producer_start_ticks,
        "executable_sha256": executable_sha256,
    });
    let receipt_path = raw_path.with_extension("jsonl.receipt.json");
    if receipt_path.exists() {
        return Err(format!(
            "{} already exists: run receipts are written once",
            receipt_path.display()
        ));
    }
    std::fs::write(&receipt_path, format!("{receipt}\n"))
        .map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One lane carries one RunId per binary: a sibling holding THIS
    /// binary's lanes blocks; a sibling holding only the OTHER binary's
    /// lanes (the frozen two-binary architecture) does not.
    #[test]
    fn one_spec_carries_one_run_identity_per_binary() {
        let dir = std::env::temp_dir().join(format!("mdbench-runid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // A spec with no raw data yet has nothing to contradict.
        assert!(ensure_single_run_identity(&dir, "run-a", &CAMPAIGN_BINARY_LANES).is_ok());
        std::fs::create_dir_all(dir.join("run-a").join("timing")).unwrap();
        assert!(ensure_single_run_identity(&dir, "run-a", &CAMPAIGN_BINARY_LANES).is_ok());
        std::fs::write(dir.join("run-a").join("timing").join("s0.jsonl"), "row\n").unwrap();
        // A different campaign binary (rebuilt) cannot continue the T/A
        // lanes.
        let error = ensure_single_run_identity(&dir, "run-b", &CAMPAIGN_BINARY_LANES)
            .expect_err("run-b must be refused");
        assert!(error.contains("never split"), "got {error}");
        // A sibling holding only OTHER lanes is the frozen multi-binary
        // architecture, never a split: the memory RunId may live beside
        // the timing RunId (its own check sees no memory rows elsewhere).
        std::fs::create_dir_all(dir.join("run-m").join("memory")).unwrap();
        std::fs::write(dir.join("run-m").join("memory").join("s.jsonl"), "row\n").unwrap();
        assert!(ensure_single_run_identity(&dir, "run-m", &MEMORY_BINARY_LANES).is_ok());
        // But a REBUILT campaign binary still cannot continue the old
        // binary's timing rows, and a rebuilt memory binary cannot
        // continue run-m's memory rows.
        assert!(
            ensure_single_run_identity(&dir, "run-t2", &CAMPAIGN_BINARY_LANES)
                .expect_err("run-t2 conflicts with run-a's timing rows")
                .contains("never split")
        );
        assert!(
            ensure_single_run_identity(&dir, "run-m2", &MEMORY_BINARY_LANES)
                .expect_err("run-m2 conflicts with run-m's memory rows")
                .contains("never split")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C3 lane isolation, structurally: only the dedicated instrumented
    /// binaries install the CountingAllocator as the process allocator;
    /// the timing/attribution binary never does.
    #[test]
    fn only_dedicated_memory_binaries_install_the_counting_allocator() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let campaign_bin =
            std::fs::read_to_string(Path::new(manifest_dir).join("src/bin/mdbench-campaign.rs"))
                .expect("mdbench-campaign source readable");
        let memory_bin =
            std::fs::read_to_string(Path::new(manifest_dir).join("src/bin/mdbench-memory.rs"))
                .expect("mdbench-memory source readable");
        let smoke_bin = std::fs::read_to_string(
            Path::new(manifest_dir).join("src/bin/mdbench-memory-smoke.rs"),
        )
        .expect("mdbench-memory-smoke source readable");
        assert!(
            !campaign_bin.contains("#[global_allocator]"),
            "the timing/attribution binary must never install a custom process allocator"
        );
        assert!(
            memory_bin.contains("#[global_allocator]") && memory_bin.contains("CountingAllocator"),
            "the formal M-LANE binary must install the CountingAllocator"
        );
        assert!(
            smoke_bin.contains("#[global_allocator]") && smoke_bin.contains("CountingAllocator"),
            "the NON_RESEARCH memory-smoke binary stays a dedicated instrumented binary"
        );
    }
}
