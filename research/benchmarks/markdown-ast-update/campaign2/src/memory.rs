//! Process-memory lane (task §30).
//!
//! DESCRIPTIVE_PROCESS_MEMORY only. RSS is a process-level observation,
//! not an object size, and is never treated as an exact retained-state
//! measurement.
//!
//! Methodology (recorded exactly):
//!
//! - measurements are taken in the SAME process that owns the state, from
//!   `/proc/self/status` (`VmHWM` peak, `VmRSS` current) and
//!   `/proc/self/statm`;
//! - sampling points are named and fixed by the caller (before build,
//!   after build, after each lifecycle checkpoint, at exit);
//! - no allocator interposition, no `mallinfo`, no instrumentation inside
//!   any timed region: the probe is a file read performed strictly
//!   between timed operations;
//! - the process is otherwise idle, so between-point deltas describe
//!   process memory growth, not object layout.

/// One DESCRIPTIVE_PROCESS_MEMORY sample.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySampleV1 {
    pub point: String,
    /// `VmRSS` in bytes (current resident set size).
    pub rss_bytes: u64,
    /// `VmHWM` in bytes (peak resident set size since process start).
    pub peak_rss_bytes: u64,
    /// `VmSize` in bytes (virtual address space).
    pub vsize_bytes: u64,
    /// Bytes of the process's data segment (`statm` field 6 * page size).
    pub data_bytes: u64,
    /// Monotone probe counter, so ordering is visible in the raw file.
    pub probe_ordinal: u64,
}

/// Read one sample from `/proc/self/status` + `/proc/self/statm`.
pub fn sample(point: &str, probe_ordinal: u64) -> Result<MemorySampleV1, String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("read /proc/self/status: {e}"))?;
    let read_kb = |key: &str| -> Result<u64, String> {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix(key) {
                let value = rest
                    .trim()
                    .trim_end_matches(" kB")
                    .trim()
                    .parse::<u64>()
                    .map_err(|e| format!("parse {key}: {e}"))?;
                return Ok(value * 1024);
            }
        }
        Err(format!("{key} missing from /proc/self/status"))
    };
    let statm = std::fs::read_to_string("/proc/self/statm")
        .map_err(|e| format!("read /proc/self/statm: {e}"))?;
    let fields: Vec<u64> = statm
        .split_whitespace()
        .map(|f| f.parse::<u64>().unwrap_or(0))
        .collect();
    let page = page_size();
    let data_bytes = fields.get(5).copied().unwrap_or(0) * page;
    Ok(MemorySampleV1 {
        point: point.to_string(),
        rss_bytes: read_kb("VmRSS:")?,
        peak_rss_bytes: read_kb("VmHWM:")?,
        vsize_bytes: read_kb("VmSize:")?,
        data_bytes,
        probe_ordinal,
    })
}

/// Host page size in bytes.
pub fn page_size() -> u64 {
    // SAFETY: `sysconf` is a pure query with no memory effects.
    let value = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if value <= 0 {
        4096
    } else {
        value as u64
    }
}

/// A probe that stamps a monotonically increasing ordinal on every
/// sample, so a raw file's ordering is self-describing.
#[derive(Debug, Default)]
pub struct MemoryProbe {
    pub samples: Vec<MemorySampleV1>,
    next: u64,
}

impl MemoryProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn probe(&mut self, point: &str) -> Result<&MemorySampleV1, String> {
        let sample = sample(point, self.next)?;
        self.next += 1;
        self.samples.push(sample);
        Ok(self.samples.last().expect("just pushed"))
    }

    /// Named checkpoints of a construction / resident-update / lifecycle
    /// run (task §30).
    pub fn probe_construction(&mut self, case_id: &str, horse: &str) -> Result<(), String> {
        self.probe(&format!("before_source_load|{case_id}|{horse}"))?;
        self.probe(&format!("after_source_load|{case_id}|{horse}"))?;
        self.probe(&format!("after_build|{case_id}|{horse}"))?;
        Ok(())
    }

    pub fn probe_lifecycle_checkpoint(
        &mut self,
        trace_id: &str,
        horse: &str,
        step: u32,
    ) -> Result<(), String> {
        self.probe(&format!("lifecycle|{trace_id}|{horse}|step{step}"))?;
        Ok(())
    }
}
