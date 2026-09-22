//! Machine capture, matching, and CPU affinity (task §24-§26, §47).
//!
//! The machine manifest records STABLE identity fields of the actual
//! primary benchmark machine. Transient values (current frequency, load
//! average, temperature, uptime) are preflight diagnostics and never
//! enter machine identity.
//!
//! Affinity (task §25): primary horse execution is single-worker and
//! pinned to ONE frozen logical CPU. The selection process is
//! deterministic and benchmark-free: inspect topology, take the
//! lowest-numbered logical CPU that is not CPU 0 (CPU 0 commonly
//! services interrupts), and record its physical core id and SMT
//! sibling relationship. The campaign worker fails closed if affinity
//! cannot be established; no session may silently run on arbitrary CPUs.
//!
//! Host policy (task §26): governor / turbo / SMT are INSPECTED and
//! recorded, never changed by this tool. If the frozen primary host
//! cannot provide a stable/reproducible policy the machine is
//! `MACHINE_PROFILE_NOT_READY`; no timing is invented anyway.

use std::collections::BTreeSet;

use crate::manifest::MachineManifest;

/// The frozen CPU selection rule (task §25: deterministic,
/// non-benchmarked).
pub const CPU_SELECTION_RULE: &str =
    "lowest logical CPU != 0 by /sys enumeration; record core_id and thread_siblings";

fn read_trimmed(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

fn run_capture(command: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(command)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Enumerate online logical CPUs from /sys.
fn online_cpus() -> Result<Vec<u32>, String> {
    let online = read_trimmed(std::path::Path::new("/sys/devices/system/cpu/online"))
        .ok_or_else(|| "cannot read /sys/devices/system/cpu/online".to_string())?;
    parse_cpu_list(&online)
}

/// Parse "0-3,8,10-11" style CPU lists.
pub fn parse_cpu_list(text: &str) -> Result<Vec<u32>, String> {
    let mut cpus = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: u32 = lo.parse().map_err(|_| format!("cpu list token {part:?}"))?;
            let hi: u32 = hi.parse().map_err(|_| format!("cpu list token {part:?}"))?;
            for cpu in lo..=hi {
                cpus.push(cpu);
            }
        } else {
            let cpu: u32 = part
                .parse()
                .map_err(|_| format!("cpu list token {part:?}"))?;
            cpus.push(cpu);
        }
    }
    cpus.sort_unstable();
    Ok(cpus)
}

fn cpu_topology_file(cpu: u32, file: &str) -> Option<String> {
    read_trimmed(std::path::Path::new(&format!(
        "/sys/devices/system/cpu/cpu{cpu}/topology/{file}"
    )))
}

fn numa_nodes() -> Result<Vec<(u32, Vec<u32>)>, String> {
    let mut nodes = Vec::new();
    let base = std::path::Path::new("/sys/devices/system/node");
    let entries = std::fs::read_dir(base).map_err(|e| format!("read {base:?}: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(node) = name.strip_prefix("node") else {
            continue;
        };
        let Ok(node_id) = node.parse::<u32>() else {
            continue;
        };
        let cpus = read_trimmed(&base.join(name.to_string()).join("cpulist"))
            .ok_or_else(|| format!("node {node_id} cpulist"))?;
        nodes.push((node_id, parse_cpu_list(&cpus)?));
    }
    nodes.sort_by_key(|(id, _)| *id);
    Ok(nodes)
}

fn proc_cpuinfo_fields() -> (String, String, String) {
    let text = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut vendor = String::new();
    let mut model = String::new();
    let mut microcode = String::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "vendor_id" if vendor.is_empty() => vendor = value.to_string(),
                "model name" if model.is_empty() => model = value.to_string(),
                "microcode" if microcode.is_empty() => microcode = value.to_string(),
                _ => {}
            }
        }
    }
    (vendor, model, microcode)
}

fn total_ram_bytes() -> u64 {
    let text = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kib: u64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
            return kib * 1024;
        }
    }
    0
}

/// Read the turbo/boost policy honestly: what exists on this host and
/// what its current setting is. Never modified.
fn turbo_policy() -> String {
    for (path, on_means) in [
        ("/sys/devices/system/cpu/intel_pstate/no_turbo", false),
        ("/sys/devices/system/cpu/cpufreq/boost", true),
        ("/sys/devices/system/cpu/amd_pstate/cpb_boost", true),
    ] {
        if let Some(value) = read_trimmed(std::path::Path::new(path)) {
            let enabled = if on_means { value == "1" } else { value == "0" };
            return format!(
                "{path}={value} (turbo/boost {})",
                if enabled { "ENABLED" } else { "DISABLED" }
            );
        }
    }
    "no controllable turbo/boost interface exposed by this host (recorded, not controlled)"
        .to_string()
}

fn governor_of(cpu: u32) -> String {
    read_trimmed(std::path::Path::new(&format!(
        "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"
    )))
    .unwrap_or_else(|| "unavailable".to_string())
}

/// The frozen deterministic CPU selection (task §25). No benchmarking
/// during selection.
pub fn select_primary_cpu() -> Result<(u32, u32, String), String> {
    let cpus = online_cpus()?;
    let selected = cpus
        .iter()
        .copied()
        .find(|cpu| *cpu != 0)
        .ok_or_else(|| "no logical CPU other than 0 is online".to_string())?;
    let core_id = cpu_topology_file(selected, "core_id")
        .and_then(|text| text.parse::<u32>().ok())
        .ok_or_else(|| format!("cpu{selected} core_id unreadable"))?;
    let siblings = cpu_topology_file(selected, "thread_siblings_list")
        .ok_or_else(|| format!("cpu{selected} thread_siblings_list unreadable"))?;
    Ok((selected, core_id, siblings))
}

/// Capture the CURRENT host as a machine manifest (task §24). Used once,
/// on the actual primary benchmark machine, to write
/// `results/manifests/primary-machine-v1.toml`.
pub fn capture_machine(
    machine_id: &str,
    benchmark_root: &std::path::Path,
    control_notes: &str,
) -> Result<MachineManifest, String> {
    let architecture = run_capture("uname", &["-m"]).ok_or("uname -m")?;
    let kernel = run_capture("uname", &["-r"]).ok_or("uname -r")?;
    let os_distribution = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "unknown".to_string());

    let (cpu_vendor, cpu_model, microcode) = proc_cpuinfo_fields();
    let cpus = online_cpus()?;
    let threads_per_core: f64 = {
        // SMT detection: does any cpu share its siblings with another?
        let mut sibling_sets: BTreeSet<String> = BTreeSet::new();
        let mut physical = 0usize;
        for cpu in &cpus {
            if let Some(siblings) = cpu_topology_file(*cpu, "thread_siblings_list") {
                if sibling_sets.insert(siblings) {
                    physical += 1;
                }
            } else {
                physical += 1;
            }
        }
        if physical == 0 {
            return Err("cannot determine physical cores".to_string());
        }
        cpus.len() as f64 / physical as f64
    };

    let nodes = numa_nodes()?;
    let numa_cpu_map = nodes
        .iter()
        .map(|(id, cpus)| format!("node{id}:{}", join_cpus(cpus)))
        .collect::<Vec<_>>()
        .join("; ");

    let (selected, core_id, siblings) = select_primary_cpu()?;
    let selected_node = nodes
        .iter()
        .find(|(_, cpus)| cpus.contains(&selected))
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("cpu{selected} has no NUMA node"))?;

    // Toolchain: the PINNED toolchain from rust-toolchain.toml must be
    // what actually runs (verified by cargo itself at build time); the
    // capture records what the host provides.
    let rustc = run_capture("rustc", &["--version"]).ok_or("rustc --version")?;
    let cargo = run_capture("cargo", &["--version"]).ok_or("cargo --version")?;
    let verbose =
        run_capture("rustc", &["--version", "--verbose"]).ok_or("rustc --version --verbose")?;
    let target_triple = verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(|s| s.to_string()))
        .ok_or_else(|| "rustc -Vv host triple".to_string())?;
    let llvm = verbose
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: ").map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let cargo_lock_sha256 = crate::sha256_file(&benchmark_root.join("Cargo.lock"))?;

    Ok(MachineManifest {
        schema: crate::MACHINE_MANIFEST_SCHEMA.to_string(),
        machine_id: machine_id.to_string(),
        architecture,
        os_distribution,
        kernel,
        cpu_vendor,
        cpu_model,
        microcode,
        physical_cores: (cpus.len() as f64 / threads_per_core).round() as u32,
        logical_cpus: cpus.len() as u32,
        smt_enabled: threads_per_core > 1.0,
        numa_nodes: nodes.len() as u32,
        numa_cpu_map,
        selected_cpu: selected,
        selected_core_id: core_id,
        selected_thread_siblings: siblings,
        selected_numa_node: selected_node,
        frequency_governor: governor_of(selected),
        turbo_boost_policy: turbo_policy(),
        total_ram_bytes: total_ram_bytes(),
        rustc,
        cargo,
        llvm,
        target_triple,
        allocator_policy: "rust-system-default".to_string(),
        release_profile_id: markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID
            .to_string(),
        rustflags: std::env::var("RUSTFLAGS").unwrap_or_default(),
        cargo_lock_sha256,
        control_notes: control_notes.to_string(),
    })
}

fn join_cpus(cpus: &[u32]) -> String {
    cpus.iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Compare the CURRENT host against the frozen machine manifest (task
/// §47). Every mismatch is a blocker; there is no fuzzy matching.
pub fn match_current_host(
    frozen: &MachineManifest,
    benchmark_root: &std::path::Path,
) -> Result<(), Vec<String>> {
    let mut blockers = Vec::new();
    macro_rules! push {
        ($field:expr, $expected:expr, $actual:expr) => {
            if &$actual != $expected {
                blockers.push(format!(
                    "machine field {}: frozen {:?} != current {:?}",
                    $field, $expected, $actual
                ));
            }
        };
    }

    let architecture = run_capture("uname", &["-m"]).unwrap_or_default();
    push!("architecture", &frozen.architecture, architecture);
    let kernel = run_capture("uname", &["-r"]).unwrap_or_default();
    push!("kernel", &frozen.kernel, kernel);
    let os_distribution = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_default();
    push!("os_distribution", &frozen.os_distribution, os_distribution);

    let (vendor, model, microcode) = proc_cpuinfo_fields();
    push!("cpu_vendor", &frozen.cpu_vendor, vendor);
    push!("cpu_model", &frozen.cpu_model, model);
    push!("microcode", &frozen.microcode, microcode);

    if let Ok(cpus) = online_cpus() {
        if cpus.len() as u32 != frozen.logical_cpus {
            blockers.push(format!(
                "machine field logical_cpus: frozen {} != current {}",
                frozen.logical_cpus,
                cpus.len()
            ));
        }
    } else {
        blockers.push("cannot read online CPUs".to_string());
    }
    if let Ok((selected, core_id, siblings)) = select_primary_cpu() {
        if selected != frozen.selected_cpu {
            blockers.push(format!(
                "selected cpu: frozen cpu{} != recomputed cpu{selected}",
                frozen.selected_cpu
            ));
        }
        if core_id != frozen.selected_core_id || siblings != frozen.selected_thread_siblings {
            blockers.push(format!(
                "selected cpu topology changed: frozen core {} siblings {:?} != current core {core_id} siblings {siblings:?}",
                frozen.selected_core_id, frozen.selected_thread_siblings
            ));
        }
    } else {
        blockers.push("cannot recompute the frozen CPU selection rule".to_string());
    }

    let governor = governor_of(frozen.selected_cpu);
    push!("frequency_governor", &frozen.frequency_governor, governor);
    push!(
        "turbo_boost_policy",
        &frozen.turbo_boost_policy,
        turbo_policy()
    );

    if total_ram_bytes() != frozen.total_ram_bytes {
        blockers.push(format!(
            "machine field total_ram_bytes: frozen {} != current {}",
            frozen.total_ram_bytes,
            total_ram_bytes()
        ));
    }

    let rustc = run_capture("rustc", &["--version"]).unwrap_or_default();
    push!("rustc", &frozen.rustc, rustc);
    let verbose = run_capture("rustc", &["--version", "--verbose"]).unwrap_or_default();
    let target_triple = verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(|s| s.to_string()))
        .unwrap_or_default();
    push!("target_triple", &frozen.target_triple, target_triple);

    // Affinity must be applicable on this host (task §47); restored
    // immediately so the check itself is non-intrusive.
    match probe_apply_affinity(frozen.selected_cpu) {
        Ok(()) => {}
        Err(error) => blockers.push(format!(
            "CPU affinity to frozen cpu{} cannot be applied: {error}",
            frozen.selected_cpu
        )),
    }

    // Build identity cross-checks (Cargo.lock digest + RUSTFLAGS).
    match crate::sha256_file(&benchmark_root.join("Cargo.lock")) {
        Ok(digest) => push!("cargo_lock_sha256", &frozen.cargo_lock_sha256, digest),
        Err(error) => blockers.push(format!("Cargo.lock unreadable: {error}")),
    }
    let rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    push!("rustflags", &frozen.rustflags, rustflags);

    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// Apply the frozen single-CPU affinity to the CURRENT process (task
/// §25). Fails closed.
#[cfg(target_os = "linux")]
pub fn apply_affinity(cpu: u32) -> Result<(), String> {
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe { libc::CPU_ZERO(&mut set) };
    unsafe { libc::CPU_SET(cpu as usize, &mut set) };
    let rc = unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) };
    if rc != 0 {
        return Err(format!(
            "sched_setaffinity(cpu {cpu}) failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Non-intrusive affinity probe: apply, read back, restore the original
/// mask.
#[cfg(target_os = "linux")]
pub fn probe_apply_affinity(cpu: u32) -> Result<(), String> {
    let mut original: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe { libc::CPU_ZERO(&mut original) };
    let rc = unsafe {
        libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut original)
    };
    if rc != 0 {
        return Err(format!(
            "sched_getaffinity failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    apply_affinity(cpu)?;
    let restore_rc =
        unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &original) };
    if restore_rc != 0 {
        return Err(format!(
            "restoring original affinity failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn apply_affinity(_cpu: u32) -> Result<(), String> {
    Err("CPU affinity is only implemented for Linux hosts".to_string())
}

#[cfg(not(target_os = "linux"))]
pub fn probe_apply_affinity(_cpu: u32) -> Result<(), String> {
    Err("CPU affinity is only implemented for Linux hosts".to_string())
}

/// Snapshot of transient host diagnostics (task §28). Diagnostics only —
/// never used to include/exclude data adaptively beyond the documented
/// busy-host abort.
pub fn transient_diagnostics() -> serde_json::Value {
    let loadavg = std::fs::read_to_string("/proc/loadavg").ok().map(|text| {
        text.split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    });
    let available = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("MemAvailable:")
                    .map(|rest| rest.trim().trim_end_matches("kB").trim().to_string())
            })
        });
    let temperature = [
        "/sys/class/thermal/thermal_zone0/temp",
        "/sys/class/hwmon/hwmon0/temp1_input",
    ]
    .iter()
    .find_map(|path| read_trimmed(std::path::Path::new(path)));
    serde_json::json!({
        "timestamp_unix": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "load_average_1_5_15": loadavg,
        "mem_available_kib": available,
        "temperature_raw": temperature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_list_parsing() {
        assert_eq!(
            parse_cpu_list("0-3,8,10-11").unwrap(),
            vec![0, 1, 2, 3, 8, 10, 11]
        );
        assert_eq!(parse_cpu_list("2,12").unwrap(), vec![2, 12]);
        assert_eq!(parse_cpu_list("5").unwrap(), vec![5]);
        assert!(parse_cpu_list("x-y").is_err());
    }

    #[test]
    fn selection_rule_prefers_lowest_nonzero_cpu() {
        if let Ok((selected, _, _)) = select_primary_cpu() {
            assert_ne!(selected, 0);
        }
        // On hosts without /sys (non-Linux dev boxes) the selection is
        // an error — fail closed, never a silent default.
    }
}
