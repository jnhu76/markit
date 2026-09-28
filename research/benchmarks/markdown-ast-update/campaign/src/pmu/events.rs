//! Frozen PMU event table for MARKIT-76-SIX-HORSE-PMU-EXPLANATION-v1.
//!
//! Authority chain: the capability-discovery record
//! (`results/diagnostics/pmu-v1/capability-discovery-v1.txt`) established
//! that this host runs `perf_event_paranoid = 2` (kernel counting denied:
//! every counter opens with `exclude_kernel = 1`), the NMI watchdog stays
//! enabled (costing one generic PMC: 3 of 4 usable), and a 6-event
//! combined group multiplexed ~42.99% — so this study freezes SMALL
//! groups only.
//!
//! Encodings (frozen BEFORE any horse observation; never substituted
//! silently):
//!
//! - `cycles`, `instructions`, `branches`, `branch-misses`,
//!   `cache-references`, `cache-misses` are kernel-generic
//!   `PERF_TYPE_HARDWARE` configs (the kernel's frozen enum order).
//! - `L1-dcache-*`, `LLC-*`, `dTLB-*`, `iTLB-*` are kernel-generic
//!   `PERF_TYPE_HW_CACHE` configs `(id) | (op << 8) | (result << 16)` —
//!   the same encoding `perf stat` resolves those aliases to on this
//!   host. They are absent from `perf list` output but accepted by the
//!   kernel (capability record), and the KERNEL performs the Haswell-EP
//!   translation, so no userspace raw-code guessing exists here.
//! - `page-faults`, `context-switches`, `cpu-migrations`, `task-clock`
//!   are `PERF_TYPE_SOFTWARE` configs (kernel enum order).
//!
//! Group D1 is software events (no PMC occupancy); every hardware group
//! holds at most two counters so a group fits the 3 watchdog-free
//! generic PMCs without multiplexing.

/// `PERF_TYPE_HARDWARE`.
pub const PERF_TYPE_HARDWARE: u32 = 0;
/// `PERF_TYPE_SOFTWARE`.
pub const PERF_TYPE_SOFTWARE: u32 = 1;
/// `PERF_TYPE_HW_CACHE`.
pub const PERF_TYPE_HW_CACHE: u32 = 3;

/// Kernel `perf_hw_id` configs (frozen enum order).
pub const HW_CPU_CYCLES: u64 = 0;
pub const HW_INSTRUCTIONS: u64 = 1;
pub const HW_CACHE_REFERENCES: u64 = 2;
pub const HW_CACHE_MISSES: u64 = 3;
pub const HW_BRANCH_INSTRUCTIONS: u64 = 4;
pub const HW_BRANCH_MISSES: u64 = 5;

/// Kernel `perf_sw_ids` configs (frozen enum order).
pub const SW_TASK_CLOCK: u64 = 1;
pub const SW_PAGE_FAULTS: u64 = 2;
pub const SW_CONTEXT_SWITCHES: u64 = 3;
pub const SW_CPU_MIGRATIONS: u64 = 4;

/// `PERF_TYPE_HW_CACHE` ids/ops/results (kernel enum order).
pub const CACHE_L1D: u64 = 0;
pub const CACHE_LL: u64 = 2;
pub const CACHE_DTLB: u64 = 3;
pub const CACHE_ITLB: u64 = 4;
pub const CACHE_OP_READ: u64 = 0;
pub const CACHE_RESULT_ACCESS: u64 = 0;
pub const CACHE_RESULT_MISS: u64 = 1 << 16;

/// One frozen counter encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDef {
    /// The alias this study reports the counter under.
    pub alias: &'static str,
    /// `perf_event_attr.type`.
    pub kind: u32,
    /// `perf_event_attr.config`.
    pub config: u64,
}

/// One frozen event group. Hardware groups are SMALL (≤ 2 events) by the
/// multiplexing rule; `D1` is the software diagnostics group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventGroup {
    pub id: &'static str,
    pub kind_label: &'static str,
    pub events: &'static [EventDef],
    /// True for the software diagnostics group: its events open as
    /// STANDALONE single-event counters, not one group. Empirical
    /// kernel fact (formal-host probes, 2026-09-29): software events
    /// chained into a group via `group_fd` receive NO time accounting
    /// and non-leader software members DROP their counts entirely,
    /// while standalone software counters carry full
    /// `time_enabled`/`time_running` and correct counts. Hardware
    /// groups behave correctly as groups and stay grouped.
    pub standalone: bool,
}

const fn hw(alias: &'static str, config: u64) -> EventDef {
    EventDef {
        alias,
        kind: PERF_TYPE_HARDWARE,
        config,
    }
}

const fn cache(alias: &'static str, id: u64, result: u64) -> EventDef {
    EventDef {
        alias,
        kind: PERF_TYPE_HW_CACHE,
        config: id | CACHE_OP_READ | result,
    }
}

const fn sw(alias: &'static str, config: u64) -> EventDef {
    EventDef {
        alias,
        kind: PERF_TYPE_SOFTWARE,
        config,
    }
}

/// A1 CORE (hardware).
pub static GROUP_A1: [EventDef; 2] = [
    hw("cycles", HW_CPU_CYCLES),
    hw("instructions", HW_INSTRUCTIONS),
];
/// A2 BRANCH (hardware).
pub static GROUP_A2: [EventDef; 2] = [
    hw("branches", HW_BRANCH_INSTRUCTIONS),
    hw("branch-misses", HW_BRANCH_MISSES),
];
/// B1 GENERIC CACHE (hardware).
pub static GROUP_B1: [EventDef; 2] = [
    hw("cache-references", HW_CACHE_REFERENCES),
    hw("cache-misses", HW_CACHE_MISSES),
];
/// B2 L1D (kernel-generic cache events).
pub static GROUP_B2: [EventDef; 2] = [
    cache("L1-dcache-loads", CACHE_L1D, CACHE_RESULT_ACCESS),
    cache("L1-dcache-load-misses", CACHE_L1D, CACHE_RESULT_MISS),
];
/// C1 LLC.
pub static GROUP_C1: [EventDef; 2] = [
    cache("LLC-loads", CACHE_LL, CACHE_RESULT_ACCESS),
    cache("LLC-load-misses", CACHE_LL, CACHE_RESULT_MISS),
];
/// C2 DTLB.
pub static GROUP_C2: [EventDef; 2] = [
    cache("dTLB-loads", CACHE_DTLB, CACHE_RESULT_ACCESS),
    cache("dTLB-load-misses", CACHE_DTLB, CACHE_RESULT_MISS),
];
/// C3 ITLB (included: both aliases schedule cleanly per the capability
/// record's pairwise probes).
pub static GROUP_C3: [EventDef; 2] = [
    cache("iTLB-loads", CACHE_ITLB, CACHE_RESULT_ACCESS),
    cache("iTLB-load-misses", CACHE_ITLB, CACHE_RESULT_MISS),
];
/// D1 OS diagnostics (software; no PMC occupancy).
pub static GROUP_D1: [EventDef; 4] = [
    sw("page-faults", SW_PAGE_FAULTS),
    sw("context-switches", SW_CONTEXT_SWITCHES),
    sw("cpu-migrations", SW_CPU_MIGRATIONS),
    sw("task-clock", SW_TASK_CLOCK),
];

/// The frozen hardware groups, in canonical order.
pub static HARDWARE_GROUPS: [EventGroup; 7] = [
    EventGroup {
        id: "A1",
        standalone: false,
        kind_label: "CORE",
        events: &GROUP_A1,
    },
    EventGroup {
        id: "A2",
        standalone: false,
        kind_label: "BRANCH",
        events: &GROUP_A2,
    },
    EventGroup {
        id: "B1",
        standalone: false,
        kind_label: "GENERIC-CACHE",
        events: &GROUP_B1,
    },
    EventGroup {
        id: "B2",
        standalone: false,
        kind_label: "L1D",
        events: &GROUP_B2,
    },
    EventGroup {
        id: "C1",
        standalone: false,
        kind_label: "LLC",
        events: &GROUP_C1,
    },
    EventGroup {
        id: "C2",
        standalone: false,
        kind_label: "DTLB",
        events: &GROUP_C2,
    },
    EventGroup {
        id: "C3",
        standalone: false,
        kind_label: "ITLB",
        events: &GROUP_C3,
    },
];

/// The frozen software diagnostics group.
pub static SOFTWARE_GROUPS: [EventGroup; 1] = [EventGroup {
    id: "D1",
    standalone: true,
    kind_label: "OS",
    events: &GROUP_D1,
}];

/// Every frozen group (hardware first, then software), canonical order.
pub fn all_groups() -> impl Iterator<Item = &'static EventGroup> {
    HARDWARE_GROUPS.iter().chain(SOFTWARE_GROUPS.iter())
}

/// Look up one frozen group by id.
pub fn group_by_id(id: &str) -> Result<&'static EventGroup, String> {
    all_groups()
        .find(|group| group.id == id)
        .ok_or_else(|| format!("unknown PMU event group {id:?} (not in the frozen table)"))
}

/// Every hardware group must stay small enough for the 3 watchdog-free
/// generic PMCs (the ~42.99% multiplexing lesson).
pub const MAX_HARDWARE_GROUP_EVENTS: usize = 2;

/// Invariant check used by tests and the probe subcommand: hardware
/// groups are small; ids are unique; aliases are unique per group.
pub fn table_invariants() -> Result<(), String> {
    let mut seen_groups = std::collections::BTreeSet::new();
    let mut seen_aliases = std::collections::BTreeSet::new();
    for group in all_groups() {
        if !seen_groups.insert(group.id) {
            return Err(format!("duplicate group id {}", group.id));
        }
        if group.standalone && group.events[0].kind != PERF_TYPE_SOFTWARE {
            return Err(format!("group {} is standalone but not software", group.id));
        }
        if group.events.len() > 2 && group.events[0].kind != PERF_TYPE_SOFTWARE {
            return Err(format!(
                "hardware group {} exceeds {} events",
                group.id, MAX_HARDWARE_GROUP_EVENTS
            ));
        }
        for event in group.events {
            if !seen_aliases.insert((group.id, event.alias)) {
                return Err(format!(
                    "duplicate alias {} in group {}",
                    event.alias, group.id
                ));
            }
        }
    }
    Ok(())
}
