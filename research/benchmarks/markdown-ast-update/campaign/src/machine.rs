//! Machine capture, matching, and CPU affinity (task §24-§26, §47).
//!
//! The machine manifest records the actual primary benchmark machine in
//! three tiers (wording corrected by MARKIT-31-MACHINE-BINDING-
//! CORRECTIVE-1; the recorded values are unchanged):
//!
//! - HARD host-binding fields — every mismatch blocks the preflight,
//!   with no fuzzy matching: architecture, distribution/kernel, CPU
//!   vendor/model/microcode, cores, SMT, NUMA topology, the selected
//!   CPU with its core/sibling/NUMA relationship, governor + turbo
//!   policy, rustc/cargo/LLVM/target, allocator policy, release
//!   profile, RUSTFLAGS, Cargo.lock digest, affinity applicability.
//! - RECORDED host observations — captured once, still reported, never
//!   byte-exact matched: `total_ram_bytes` is
//!   `/proc/meminfo:MemTotal` at capture time. Linux defines MemTotal
//!   as usable RAM (installed capacity minus firmware/kernel
//!   reservations), not immutable installed capacity, so it legitimately
//!   moves between boots; the frozen value stays exactly as recorded
//!   and a current-vs-frozen delta is a visible, auditable,
//!   non-blocking preflight diagnostic.
//! - TRANSIENT diagnostics — per-session values (current frequency,
//!   load average, temperature, uptime) that never enter machine
//!   identity.
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

/// What the LIVE host currently reports, captured once for machine
/// binding (task §47). Pure data: [`compare_host_binding`] turns it
/// into blockers, so tests can synthesize observations instead of
/// depending on a real `/proc`+`/sys`.
pub struct HostObservations {
    pub architecture: String,
    pub kernel: String,
    pub os_distribution: String,
    pub cpu_vendor: String,
    pub cpu_model: String,
    pub microcode: String,
    /// Count of online logical CPUs (`None` = unreadable; the capture
    /// blockers say why).
    pub logical_cpus: Option<u32>,
    /// Recomputation of the frozen CPU selection rule plus the topology
    /// it implies (`None` = the rule could not be recomputed).
    pub topology: Option<HostTopology>,
    pub frequency_governor: String,
    pub turbo_boost_policy: String,
    /// `/proc/meminfo:MemTotal` at observation time — DIAGNOSTIC ONLY,
    /// never a hard binding field (see [`memory_binding_diagnostic`]).
    pub mem_total_bytes: u64,
    pub rustc: String,
    pub cargo: String,
    pub llvm: String,
    pub target_triple: String,
    pub rustflags: String,
    /// Result of the non-intrusive affinity probe for the frozen CPU
    /// (applied and restored).
    pub affinity_probe: Result<(), String>,
    /// SHA256 of the workspace `Cargo.lock` (`Err` = unreadable).
    pub cargo_lock_sha256: Result<String, String>,
}

/// Topology implied by a successful recomputation of the frozen CPU
/// selection rule.
pub struct HostTopology {
    pub physical_cores: u32,
    pub smt_enabled: bool,
    /// NUMA facts (`None` = unreadable; the capture blockers say why).
    pub numa: Option<HostNuma>,
    pub selected_cpu: u32,
    pub selected_core_id: u32,
    pub selected_thread_siblings: String,
}

/// NUMA topology as observed.
pub struct HostNuma {
    pub node_count: u32,
    pub cpu_map: String,
    /// Node containing the frozen `selected_cpu` (`None` = no node
    /// lists it, itself a mismatch against the frozen manifest).
    pub selected_node: Option<u32>,
}

/// Capture the current host's binding-relevant state (task §47).
/// Capture failures are returned as fail-closed blockers exactly like
/// field mismatches — an unreadable fact is never silently skipped.
pub fn observe_host_for_binding(
    frozen: &MachineManifest,
    benchmark_root: &std::path::Path,
) -> (HostObservations, Vec<String>) {
    let mut capture_blockers = Vec::new();

    let architecture = run_capture("uname", &["-m"]).unwrap_or_default();
    let kernel = run_capture("uname", &["-r"]).unwrap_or_default();
    let os_distribution = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_default();
    let (cpu_vendor, cpu_model, microcode) = proc_cpuinfo_fields();

    let online = online_cpus();
    if online.is_err() {
        capture_blockers.push("cannot read online CPUs".to_string());
    }
    let logical_cpus = online.as_ref().ok().map(|cpus| cpus.len() as u32);

    let topology = match select_primary_cpu() {
        Ok((selected, core_id, siblings)) => {
            let cpus = online.unwrap_or_default();
            let mut sibling_groups: BTreeSet<String> = BTreeSet::new();
            for cpu in &cpus {
                sibling_groups
                    .insert(cpu_topology_file(*cpu, "thread_siblings_list").unwrap_or_default());
            }
            let physical = sibling_groups.len() as u32;
            let smt = cpus.len() as u32 > physical;
            let numa = match numa_nodes() {
                Ok(nodes) => Some(HostNuma {
                    node_count: nodes.len() as u32,
                    cpu_map: nodes
                        .iter()
                        .map(|(id, cpus)| format!("node{id}:{}", join_cpus(cpus)))
                        .collect::<Vec<_>>()
                        .join("; "),
                    selected_node: nodes
                        .iter()
                        .find(|(_, node_cpus)| node_cpus.contains(&frozen.selected_cpu))
                        .map(|(id, _)| *id),
                }),
                Err(error) => {
                    capture_blockers.push(format!("NUMA topology unreadable: {error}"));
                    None
                }
            };
            Some(HostTopology {
                physical_cores: physical,
                smt_enabled: smt,
                numa,
                selected_cpu: selected,
                selected_core_id: core_id,
                selected_thread_siblings: siblings,
            })
        }
        Err(_) => {
            capture_blockers.push("cannot recompute the frozen CPU selection rule".to_string());
            None
        }
    };

    let governor = governor_of(frozen.selected_cpu);
    let rustc = run_capture("rustc", &["--version"]).unwrap_or_default();
    let cargo = run_capture("cargo", &["--version"]).unwrap_or_default();
    let verbose = run_capture("rustc", &["--version", "--verbose"]).unwrap_or_default();
    let target_triple = verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(|s| s.to_string()))
        .unwrap_or_default();
    let llvm = verbose
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: ").map(|s| s.to_string()))
        .unwrap_or_default();

    (
        HostObservations {
            architecture,
            kernel,
            os_distribution,
            cpu_vendor,
            cpu_model,
            microcode,
            logical_cpus,
            topology,
            frequency_governor: governor,
            turbo_boost_policy: turbo_policy(),
            mem_total_bytes: total_ram_bytes(),
            rustc,
            cargo,
            llvm,
            target_triple,
            rustflags: std::env::var("RUSTFLAGS").unwrap_or_default(),
            affinity_probe: probe_apply_affinity(frozen.selected_cpu),
            cargo_lock_sha256: crate::sha256_file(&benchmark_root.join("Cargo.lock")),
        },
        capture_blockers,
    )
}

/// Compare observed host state against the frozen machine manifest
/// (task §47). Every HARD host-binding field mismatch is a blocker;
/// there is no fuzzy matching.
///
/// `total_ram_bytes` is deliberately NOT compared here
/// (MARKIT-31-MACHINE-BINDING-CORRECTIVE-1): MemTotal is usable RAM,
/// not installed capacity, so the frozen value is a recorded
/// observation reported through [`memory_binding_diagnostic`], never a
/// byte-exact identity gate.
pub fn compare_host_binding(frozen: &MachineManifest, observed: &HostObservations) -> Vec<String> {
    let mut blockers = Vec::new();
    macro_rules! push {
        ($field:expr, $expected:expr, $actual:expr) => {
            if $actual != $expected {
                blockers.push(format!(
                    "machine field {}: frozen {:?} != current {:?}",
                    $field, $expected, $actual
                ));
            }
        };
    }

    push!("architecture", &frozen.architecture, &observed.architecture);
    push!("kernel", &frozen.kernel, &observed.kernel);
    push!(
        "os_distribution",
        &frozen.os_distribution,
        &observed.os_distribution
    );

    push!("cpu_vendor", &frozen.cpu_vendor, &observed.cpu_vendor);
    push!("cpu_model", &frozen.cpu_model, &observed.cpu_model);
    push!("microcode", &frozen.microcode, &observed.microcode);

    match observed.logical_cpus {
        Some(count) if count != frozen.logical_cpus => blockers.push(format!(
            "machine field logical_cpus: frozen {} != current {count}",
            frozen.logical_cpus
        )),
        // `None` already produced a capture blocker.
        _ => {}
    }
    if let Some(topology) = &observed.topology {
        if topology.selected_cpu != frozen.selected_cpu {
            blockers.push(format!(
                "selected cpu: frozen cpu{} != recomputed cpu{}",
                frozen.selected_cpu, topology.selected_cpu
            ));
        }
        if topology.selected_core_id != frozen.selected_core_id
            || topology.selected_thread_siblings != frozen.selected_thread_siblings
        {
            blockers.push(format!(
                "selected cpu topology changed: frozen core {} siblings {:?} != current core {} siblings {:?}",
                frozen.selected_core_id,
                frozen.selected_thread_siblings,
                topology.selected_core_id,
                topology.selected_thread_siblings
            ));
        }
        // SMT / core-count / NUMA topology are part of the frozen machine
        // identity: compare every recorded field, not just the selected
        // CPU.
        if topology.physical_cores != frozen.physical_cores {
            blockers.push(format!(
                "machine field physical_cores: frozen {} != current {}",
                frozen.physical_cores, topology.physical_cores
            ));
        }
        if topology.smt_enabled != frozen.smt_enabled {
            blockers.push(format!(
                "machine field smt_enabled: frozen {} != current {}",
                frozen.smt_enabled, topology.smt_enabled
            ));
        }
        if let Some(numa) = &topology.numa {
            if numa.node_count != frozen.numa_nodes {
                blockers.push(format!(
                    "machine field numa_nodes: frozen {} != current {}",
                    frozen.numa_nodes, numa.node_count
                ));
            }
            if numa.cpu_map != frozen.numa_cpu_map {
                blockers.push(format!(
                    "machine field numa_cpu_map: frozen {:?} != current {:?}",
                    frozen.numa_cpu_map, numa.cpu_map
                ));
            }
            if numa.selected_node != Some(frozen.selected_numa_node) {
                blockers.push(format!(
                    "machine field selected_numa_node: frozen {} != current {:?}",
                    frozen.selected_numa_node, numa.selected_node
                ));
            }
        }
        // `topology.numa == None` already produced a capture blocker.
    }
    // `observed.topology == None` already produced a capture blocker.

    push!(
        "frequency_governor",
        &frozen.frequency_governor,
        &observed.frequency_governor
    );
    push!(
        "turbo_boost_policy",
        &frozen.turbo_boost_policy,
        &observed.turbo_boost_policy
    );

    push!("rustc", &frozen.rustc, &observed.rustc);
    push!("cargo", &frozen.cargo, &observed.cargo);
    push!(
        "target_triple",
        &frozen.target_triple,
        &observed.target_triple
    );
    push!("llvm", &frozen.llvm, &observed.llvm);
    push!(
        "allocator_policy",
        &frozen.allocator_policy,
        &"rust-system-default".to_string()
    );

    // Affinity must be applicable on this host (task §47); the probe
    // applied and restored it non-intrusively at capture time.
    if let Err(error) = &observed.affinity_probe {
        blockers.push(format!(
            "CPU affinity to frozen cpu{} cannot be applied: {error}",
            frozen.selected_cpu
        ));
    }

    // Build identity cross-checks (Cargo.lock digest + RUSTFLAGS).
    match &observed.cargo_lock_sha256 {
        Ok(digest) => push!("cargo_lock_sha256", &frozen.cargo_lock_sha256, digest),
        Err(error) => blockers.push(format!("Cargo.lock unreadable: {error}")),
    }
    push!("rustflags", &frozen.rustflags, &observed.rustflags);

    blockers
}

/// Memory binding diagnostic (MARKIT-31-MACHINE-BINDING-CORRECTIVE-1).
///
/// Linux `/proc/meminfo:MemTotal` reports usable RAM — installed
/// capacity minus firmware/kernel reservations — which legitimately
/// moves between boots. The frozen `total_ram_bytes` therefore stays
/// exactly as captured, and any current-vs-frozen delta is REPORTED,
/// never matched: this value is a recorded host observation, not a
/// hard machine identity field. The diagnostic is visible, auditable,
/// and non-blocking by construction.
pub fn memory_binding_diagnostic(
    frozen_total_ram_bytes: u64,
    current_mem_total_bytes: u64,
) -> serde_json::Value {
    let delta_bytes = current_mem_total_bytes as i128 - frozen_total_ram_bytes as i128;
    serde_json::json!({
        "frozen_mem_total_bytes": frozen_total_ram_bytes,
        "current_mem_total_bytes": current_mem_total_bytes,
        "delta_bytes": delta_bytes,
        "identity_role": "diagnostic_only",
        "definition": "/proc/meminfo:MemTotal = usable RAM (installed capacity minus firmware/kernel reservations), not installed physical capacity; recorded observation, never byte-exact matched",
    })
}

/// Compare the CURRENT host against the frozen machine manifest (task
/// §47). Every hard host-binding mismatch is a blocker; there is no
/// fuzzy matching. MemTotal is not matched (see
/// [`memory_binding_diagnostic`]).
pub fn match_current_host(
    frozen: &MachineManifest,
    benchmark_root: &std::path::Path,
) -> Result<(), Vec<String>> {
    let (observed, mut blockers) = observe_host_for_binding(frozen, benchmark_root);
    blockers.extend(compare_host_binding(frozen, &observed));
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

    // -----------------------------------------------------------------
    // MARKIT-31-MACHINE-BINDING-CORRECTIVE-1 tests (A/B/C).
    //
    // The comparison is tested through the PURE [`compare_host_binding`]
    // so these hold on any dev machine, not only on the primary host.
    // -----------------------------------------------------------------

    fn frozen_primary_manifest() -> MachineManifest {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        MachineManifest::load(&root).expect("frozen machine manifest loads")
    }

    /// Observations of a host that agrees with the frozen manifest on
    /// EVERY hard binding field (affinity applies; Cargo.lock reads).
    fn mirror_observations(frozen: &MachineManifest, mem_total_bytes: u64) -> HostObservations {
        HostObservations {
            architecture: frozen.architecture.clone(),
            kernel: frozen.kernel.clone(),
            os_distribution: frozen.os_distribution.clone(),
            cpu_vendor: frozen.cpu_vendor.clone(),
            cpu_model: frozen.cpu_model.clone(),
            microcode: frozen.microcode.clone(),
            logical_cpus: Some(frozen.logical_cpus),
            topology: Some(HostTopology {
                physical_cores: frozen.physical_cores,
                smt_enabled: frozen.smt_enabled,
                numa: Some(HostNuma {
                    node_count: frozen.numa_nodes,
                    cpu_map: frozen.numa_cpu_map.clone(),
                    selected_node: Some(frozen.selected_numa_node),
                }),
                selected_cpu: frozen.selected_cpu,
                selected_core_id: frozen.selected_core_id,
                selected_thread_siblings: frozen.selected_thread_siblings.clone(),
            }),
            frequency_governor: frozen.frequency_governor.clone(),
            turbo_boost_policy: frozen.turbo_boost_policy.clone(),
            mem_total_bytes,
            rustc: frozen.rustc.clone(),
            cargo: frozen.cargo.clone(),
            llvm: frozen.llvm.clone(),
            target_triple: frozen.target_triple.clone(),
            rustflags: frozen.rustflags.clone(),
            affinity_probe: Ok(()),
            cargo_lock_sha256: Ok(frozen.cargo_lock_sha256.clone()),
        }
    }

    fn blockers_for(
        frozen: &MachineManifest,
        mutate: impl FnOnce(&mut HostObservations),
    ) -> Vec<String> {
        let mut observed = mirror_observations(frozen, frozen.total_ram_bytes);
        mutate(&mut observed);
        compare_host_binding(frozen, &observed)
    }

    /// Test A: the same frozen host with MemTotal moved by the observed
    /// +4096 bytes must NOT block solely because of the memory total.
    /// There is no tolerance involved — MemTotal is not compared at all,
    /// so an arbitrarily large delta is equally non-blocking.
    #[test]
    fn mem_total_drift_alone_never_blocks() {
        let frozen = frozen_primary_manifest();

        // The exact observed primary-host drift: +4096 bytes.
        let blockers = blockers_for(&frozen, |observed| {
            observed.mem_total_bytes = frozen.total_ram_bytes + 4096;
        });
        assert!(
            blockers.is_empty(),
            "MemTotal drift must not block: {blockers:?}"
        );

        // No hidden tolerance: a 1 GiB delta behaves identically because
        // there is NO MemTotal comparison, not a widened one.
        let blockers = blockers_for(&frozen, |observed| {
            observed.mem_total_bytes = frozen.total_ram_bytes - (1 << 30);
        });
        assert!(
            blockers.is_empty(),
            "a large MemTotal delta must be as non-blocking as a small one: {blockers:?}"
        );

        // Baseline: a mirrored host with unchanged MemTotal blocks on
        // nothing at all.
        assert!(compare_host_binding(
            &frozen,
            &mirror_observations(&frozen, frozen.total_ram_bytes)
        )
        .is_empty());
    }

    /// Test B: real hard-identity mismatches still block — cpu_model,
    /// kernel, selected_cpu, governor, and the other hard binding
    /// fields. Unrelated matching is not weakened by the corrective.
    #[test]
    fn hard_identity_mismatches_still_block() {
        let frozen = frozen_primary_manifest();

        // cpu_model
        let blockers = blockers_for(&frozen, |observed| {
            observed.cpu_model = "Intel(R) Xeon(R) CPU E5-9999 v9 @ 9.90GHz".to_string();
        });
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("machine field cpu_model:")),
            "cpu_model mismatch must block; got {blockers:?}"
        );

        // kernel
        let blockers = blockers_for(&frozen, |observed| {
            observed.kernel = "7.2.6-200.fc44.x86_64".to_string();
        });
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("machine field kernel:")),
            "kernel mismatch must block; got {blockers:?}"
        );

        // selected_cpu (the frozen CPU selection rule recomputed to a
        // different CPU)
        let blockers = blockers_for(&frozen, |observed| {
            observed.topology.as_mut().unwrap().selected_cpu += 1;
        });
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("selected cpu: frozen cpu")),
            "selected_cpu mismatch must block; got {blockers:?}"
        );

        // governor
        let blockers = blockers_for(&frozen, |observed| {
            observed.frequency_governor = "performance".to_string();
        });
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("machine field frequency_governor:")),
            "governor mismatch must block; got {blockers:?}"
        );

        // microcode, rustc, target triple, Cargo.lock digest, RUSTFLAGS,
        // affinity applicability: all remain hard.
        for (name, mutate) in [
            (
                "machine field microcode:",
                &(|o: &mut HostObservations| {
                    o.microcode = "0x50".to_string();
                }) as &dyn Fn(&mut HostObservations),
            ),
            (
                "machine field rustc:",
                &(|o: &mut HostObservations| {
                    o.rustc = "rustc 1.98.0 (000000000 2026-01-01)".to_string();
                }),
            ),
            (
                "machine field target_triple:",
                &(|o: &mut HostObservations| {
                    o.target_triple = "aarch64-unknown-linux-gnu".to_string();
                }),
            ),
            (
                "machine field cargo_lock_sha256:",
                &(|o: &mut HostObservations| {
                    o.cargo_lock_sha256 = Ok("ab".repeat(32));
                }),
            ),
            (
                "machine field rustflags:",
                &(|o: &mut HostObservations| {
                    o.rustflags = "-C target-cpu=native".to_string();
                }),
            ),
        ] {
            let blockers = blockers_for(&frozen, |observed| {
                mutate(observed);
            });
            assert!(
                blockers.iter().any(|b| b.starts_with(name)),
                "{name} mismatch must block; got {blockers:?}"
            );
        }

        let blockers = blockers_for(&frozen, |observed| {
            observed.affinity_probe = Err("sched_setaffinity failed".to_string());
        });
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("CPU affinity to frozen cpu")),
            "affinity inapplicability must block; got {blockers:?}"
        );

        // A failed topology recomputation blocks fail-closed through the
        // capture path (observe), never silently passes the comparison.
        let observed = HostObservations {
            topology: None,
            ..mirror_observations(&frozen, frozen.total_ram_bytes)
        };
        assert_eq!(
            compare_host_binding(&frozen, &observed),
            Vec::<String>::new(),
            "comparison adds nothing; the capture blocker owns the failure"
        );
    }

    /// Test C: MemTotal remains present in capture/reporting — the
    /// frozen manifest still carries the recorded value unchanged, live
    /// capture still reads it, and the diagnostic reports the full
    /// frozen/current/delta shape.
    #[test]
    fn mem_total_remains_captured_and_reported() {
        let frozen = frozen_primary_manifest();

        // The recorded value stays exactly as captured (not replaced by
        // today's MemTotal).
        assert_eq!(
            frozen.total_ram_bytes, 67_252_445_184,
            "frozen recorded MemTotal must stay as captured"
        );

        // Live capture still reads a nonzero MemTotal where /proc exists.
        #[cfg(target_os = "linux")]
        assert!(total_ram_bytes() > 0, "MemTotal capture must keep working");

        // The diagnostic is visible, auditable, non-blocking by shape.
        let diagnostic = memory_binding_diagnostic(frozen.total_ram_bytes, frozen.total_ram_bytes);
        assert_eq!(diagnostic["frozen_mem_total_bytes"], frozen.total_ram_bytes);
        assert_eq!(
            diagnostic["current_mem_total_bytes"],
            frozen.total_ram_bytes
        );
        assert_eq!(diagnostic["delta_bytes"], 0);
        assert_eq!(diagnostic["identity_role"], "diagnostic_only");

        // The observed primary-host drift reports as +4096, non-blocking.
        let diagnostic =
            memory_binding_diagnostic(frozen.total_ram_bytes, frozen.total_ram_bytes + 4096);
        assert_eq!(diagnostic["delta_bytes"], 4096);
        assert_eq!(diagnostic["identity_role"], "diagnostic_only");
    }
}
