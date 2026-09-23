//! Raw evidence layout and inventory (task §49-§52).
//!
//! ```text
//! results/campaign-2/
//!   manifests/     frozen identity / spec / schedule / machine artifacts
//!   construction/  Surface A raw JSONL + receipts
//!   resident-update/ Surface B raw JSONL + receipts
//!   lifecycle/     Surface C raw JSONL + receipts
//!   controlled/{N,B,D-fence,F-reference,K-container}/
//!   profiling/{perf-stat,perf-record,ebpf,allocator}/
//!   memory/
//!   receipts/
//!   logs/
//! ```

use std::path::{Path, PathBuf};

/// Root of the Campaign-2 raw layout.
pub fn campaign_root(benchmark_root: &Path) -> PathBuf {
    benchmark_root.join("results/campaign-2")
}

/// Every sub-directory of the frozen Campaign-2 layout.
pub const SUBDIRS: [&str; 14] = [
    "manifests",
    "construction",
    "resident-update",
    "lifecycle",
    "controlled/N",
    "controlled/B",
    "controlled/D-fence",
    "controlled/F-reference",
    "controlled/K-container",
    "profiling/perf-stat",
    "profiling/perf-record",
    "profiling/ebpf",
    "profiling/allocator",
    "memory",
];

/// Additional directories written alongside the raw lanes.
pub const AUX_SUBDIRS: [&str; 2] = ["receipts", "logs"];

/// Create the whole layout (idempotent).
pub fn ensure_layout(benchmark_root: &Path) -> Result<PathBuf, String> {
    let root = campaign_root(benchmark_root);
    for sub in SUBDIRS.iter().chain(AUX_SUBDIRS.iter()) {
        let path = root.join(sub);
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("create {}: {e}", path.display()))?;
    }
    Ok(root)
}

/// Directory of one controlled axis.
pub fn controlled_dir(benchmark_root: &Path, axis_tag: &str) -> PathBuf {
    campaign_root(benchmark_root)
        .join("controlled")
        .join(axis_tag)
}

/// One file in the raw inventory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryEntryV1 {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    /// Row count for JSONL raw files; `None` for non-JSONL artifacts.
    pub rows: Option<u64>,
    pub lane: String,
    pub sub_campaign_spec_id: String,
    pub run_id: String,
    pub complete_marker: bool,
    pub exit_status: i32,
}

/// One inventory file: every raw artifact of one collection phase.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawInventoryV1 {
    pub schema: String,
    pub study_id: String,
    pub campaign_spec_id: String,
    pub authority_sha: String,
    pub generated_from: String,
    pub entries: Vec<InventoryEntryV1>,
}

impl RawInventoryV1 {
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|e| e.bytes).sum()
    }

    pub fn total_rows(&self) -> u64 {
        self.entries.iter().filter_map(|e| e.rows).sum()
    }
}

/// Count the rows of a JSONL file (one JSON object per non-empty line).
pub fn count_jsonl_rows(path: &Path) -> Result<u64, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut rows = 0u64;
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        serde_json::from_slice::<serde_json::Value>(line)
            .map_err(|e| format!("{}: malformed JSONL row {rows}: {e}", path.display()))?;
        rows += 1;
    }
    Ok(rows)
}

/// Recursively collect a deterministic inventory of every regular file
/// under `root`, sorted by path.
pub fn inventory(
    root: &Path,
    study_id: &str,
    campaign_spec_id: &str,
    authority_sha: &str,
    generated_from: &str,
    lane_of: &dyn Fn(&Path) -> (String, String, String),
) -> Result<RawInventoryV1, String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_files(root, &mut paths)?;
    paths.sort();
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|e| format!("strip prefix: {e}"))?
            .to_string_lossy()
            .to_string();
        let bytes = std::fs::metadata(&path)
            .map_err(|e| format!("stat {}: {e}", path.display()))?
            .len();
        let sha256 = crate::sha256_file(&path)?;
        let rows = if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            Some(count_jsonl_rows(&path)?)
        } else {
            None
        };
        let (lane, sub_campaign_spec_id, run_id) = lane_of(&path);
        entries.push(InventoryEntryV1 {
            path: relative,
            bytes,
            sha256,
            rows,
            lane,
            sub_campaign_spec_id,
            run_id,
            complete_marker: true,
            exit_status: 0,
        });
    }
    Ok(RawInventoryV1 {
        schema: "campaign2-raw-inventory-v1".to_string(),
        study_id: study_id.to_string(),
        campaign_spec_id: campaign_spec_id.to_string(),
        authority_sha: authority_sha.to_string(),
        generated_from: generated_from.to_string(),
        entries,
    })
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("read_dir {}: {e}", dir.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry in {}: {e}", dir.display()))?;
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| format!("metadata {}: {e}", path.display()))?;
        if meta.is_dir() {
            collect_files(&path, out)?;
        } else if meta.is_file() {
            out.push(path);
        }
    }
    Ok(())
}
