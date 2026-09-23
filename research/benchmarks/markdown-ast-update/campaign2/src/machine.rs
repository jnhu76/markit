//! Machine identity and environment telemetry (task §10-§11, §32-§33).
//!
//! The capture itself is the FROZEN Campaign-1 machinery
//! (`markit_mdbench_campaign::machine`), reused unchanged so Campaign-2
//! cannot quietly redefine what "the same machine" means. MemTotal is a
//! DIAGNOSTIC observation only (per the #43 corrective), never a gate.

pub use markit_mdbench_campaign::machine::{
    apply_affinity, capture_machine, compare_host_binding, observe_host_for_binding,
    parse_cpu_list, probe_apply_affinity, select_primary_cpu, transient_diagnostics,
    HostObservations, HostTopology,
};
pub use markit_mdbench_campaign::manifest::MachineManifest;

/// Environment telemetry row (task §32), recorded per session OUTSIDE
/// microtimers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentTelemetryV1 {
    pub schema: String,
    pub session_ordinal: Option<u32>,
    pub sub_campaign_tag: String,
    pub point: String,
    pub utc_timestamp: String,
    pub governor: String,
    pub turbo_state: String,
    pub affinity: String,
    /// Per-CPU current frequency in kHz where the kernel exposes it.
    pub scaling_cur_freq_khz: Vec<(u32, String)>,
    /// Temperature where the kernel exposes it (millidegrees C).
    pub temperature_millicelsius: Vec<(String, String)>,
    /// Context switches / migrations of the measuring process.
    pub voluntary_ctxt_switches: String,
    pub nonvoluntary_ctxt_switches: String,
    /// `/proc/loadavg` at the sample point.
    pub loadavg: String,
    /// RSS sample (diagnostic only).
    pub rss_bytes: u64,
}

/// Read one environment telemetry sample.
pub fn environment_telemetry(
    sub_campaign_tag: &str,
    session_ordinal: Option<u32>,
    point: &str,
) -> EnvironmentTelemetryV1 {
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
    let governor = read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .trim()
        .to_string();
    let turbo = format!(
        "no_turbo={}",
        read("/sys/devices/system/cpu/intel_pstate/no_turbo").trim()
    );
    let affinity = read("/proc/self/status")
        .lines()
        .find(|line| line.starts_with("Cpus_allowed_list:"))
        .map(|line| line.trim().to_string())
        .unwrap_or_default();
    let status = read("/proc/self/status");
    let status_field = |key: &str| -> String {
        status
            .lines()
            .find(|line| line.starts_with(key))
            .map(|line| line.trim().to_string())
            .unwrap_or_default()
    };
    let mut frequencies = Vec::new();
    for cpu in 0..256u32 {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq");
        match std::fs::read_to_string(&path) {
            Ok(value) => frequencies.push((cpu, value.trim().to_string())),
            Err(_) => continue,
        }
    }
    let mut temperatures = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("thermal_zone") {
                continue;
            }
            if let Ok(value) = std::fs::read_to_string(entry.path().join("temp")) {
                temperatures.push((name, value.trim().to_string()));
            }
        }
    }
    EnvironmentTelemetryV1 {
        schema: "campaign2-environment-telemetry-v1".to_string(),
        session_ordinal,
        sub_campaign_tag: sub_campaign_tag.to_string(),
        point: point.to_string(),
        utc_timestamp: utc_timestamp(),
        governor,
        turbo_state: turbo,
        affinity,
        scaling_cur_freq_khz: frequencies,
        temperature_millicelsius: temperatures,
        voluntary_ctxt_switches: status_field("voluntary_ctxt_switches:"),
        nonvoluntary_ctxt_switches: status_field("nonvoluntary_ctxt_switches:"),
        loadavg: read("/proc/loadavg").trim().to_string(),
        rss_bytes: crate::memory::sample(point, 0)
            .map(|s| s.rss_bytes)
            .unwrap_or(0),
    }
}

/// UTC timestamp from the system clock (recorded facts only; never fed
/// into any measure).
pub fn utc_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{now}")
}
