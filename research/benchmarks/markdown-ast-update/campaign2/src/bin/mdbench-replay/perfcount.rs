//! Region-scoped hardware counters via `perf_event_open` (task §38).
//!
//! Counters are opened for the CALLING process (`pid = 0`, `cpu = -1`),
//! with `exclude_kernel = 1` (this host runs at
//! `perf_event_paranoid = 2`, where kernel counting is denied to an
//! unprivileged user), `disabled = 1`, and are enabled/disabled by
//! `ioctl` immediately around the measured region. The counts therefore
//! describe the region, not the process, and no subtraction is used.
//!
//! If the kernel refuses a counter, the value is reported as `null` —
//! never silently zero — and the replay still produces its in-process
//! timing.

/// The hardware events this replay requests (task §35).
pub const EVENTS: [&str; 6] = [
    "cycles",
    "instructions",
    "branches",
    "branch_misses",
    "cache_references",
    "cache_misses",
];

/// Accumulated counters over the measured region.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct Counts {
    pub task_clock_ns: u64,
    pub cycles: Option<u64>,
    pub instructions: Option<u64>,
    pub branches: Option<u64>,
    pub branch_misses: Option<u64>,
    pub cache_references: Option<u64>,
    pub cache_misses: Option<u64>,
}

impl Counts {
    pub fn add(&mut self, other: &Counts) {
        self.task_clock_ns = self.task_clock_ns.saturating_add(other.task_clock_ns);
        self.cycles = add_opt(self.cycles, other.cycles);
        self.instructions = add_opt(self.instructions, other.instructions);
        self.branches = add_opt(self.branches, other.branches);
        self.branch_misses = add_opt(self.branch_misses, other.branch_misses);
        self.cache_references = add_opt(self.cache_references, other.cache_references);
        self.cache_misses = add_opt(self.cache_misses, other.cache_misses);
    }
}

fn add_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// `PERF_ATTR_SIZE_VER0`: the kernel reads only this many leading bytes,
/// which already contain `type`, `size`, `config`, the flag word holding
/// `disabled` / `exclude_kernel` / `exclude_hv`, and `config1`.
const PERF_ATTR_SIZE_VER0: u32 = 64;

/// Flag-word bits (linux/perf_event.h bitfield order).
const FLAG_DISABLED: u64 = 1 << 0;
const FLAG_EXCLUDE_KERNEL: u64 = 1 << 5;
const FLAG_EXCLUDE_HV: u64 = 1 << 6;

/// `_IO('$', n)` ioctl numbers (linux/perf_event.h; '$' = 0x24).
const IOC_ENABLE: libc::c_ulong = 0x2400;
const IOC_DISABLE: libc::c_ulong = 0x2401;
const IOC_RESET: libc::c_ulong = 0x2403;

/// The leading 72 bytes of `struct perf_event_attr` (the kernel reads the
/// first `PERF_ATTR_SIZE_VER0` bytes of it).
#[repr(C, align(8))]
#[derive(Default, Clone, Copy)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    wakeup_events: u32,
    bp_type: u32,
    config1: u64,
    config2: u64,
}

/// One open counter.
struct Counter {
    fd: i32,
    slot: Option<usize>,
}

/// The open counter set for one process.
pub struct PerfCounters {
    counters: Vec<Counter>,
    slots: usize,
    errors: Vec<String>,
}

impl PerfCounters {
    /// Open the counter set. A refused counter is reported, not fatal.
    pub fn open() -> Result<Self, String> {
        let mut counters = Vec::new();
        let mut slots = 0usize;
        let mut errors = Vec::new();
        for (index, event) in EVENTS.iter().enumerate() {
            match open_one(hardware_config(index)) {
                Ok(fd) => {
                    counters.push(Counter { fd, slot: Some(slots) });
                    slots += 1;
                }
                Err(error) => {
                    errors.push(format!("{event}: {error}"));
                    counters.push(Counter { fd: -1, slot: None });
                }
            }
        }
        for error in &errors {
            eprintln!("PERF_COUNTER_UNAVAILABLE {error}");
        }
        Ok(Self { counters, slots, errors })
    }

    /// True when every requested counter is live.
    pub fn available(&self) -> bool {
        self.errors.is_empty()
    }

    /// Reset every live counter, take the pre-region snapshot WHILE STILL
    /// DISABLED, then enable. Reading after `ENABLE` (as a naive
    /// implementation does) samples the region itself into the baseline,
    /// which is exactly the kind of ordering error §38 exists to prevent.
    pub fn begin(&self) -> Vec<u64> {
        let mut before = vec![0u64; self.slots];
        for counter in &self.counters {
            let Some(slot) = counter.slot else { continue };
            unsafe { libc::ioctl(counter.fd, IOC_RESET, 0) };
            before[slot] = read_value(counter.fd).unwrap_or(0);
        }
        for counter in &self.counters {
            if counter.slot.is_some() {
                unsafe { libc::ioctl(counter.fd, IOC_ENABLE, 0) };
            }
        }
        before
    }

    /// Disable every live counter and return the region delta.
    pub fn end(&self, before: Vec<u64>) -> Counts {
        for counter in &self.counters {
            if counter.slot.is_some() {
                unsafe { libc::ioctl(counter.fd, IOC_DISABLE, 0) };
            }
        }
        let mut after = vec![0u64; self.slots];
        for counter in &self.counters {
            let Some(slot) = counter.slot else { continue };
            after[slot] = read_value(counter.fd).unwrap_or(0);
        }
        let delta = |index: usize| -> Option<u64> {
            let slot = self.counters.get(index)?.slot?;
            Some(after[slot].saturating_sub(before[slot]))
        };
        Counts {
            task_clock_ns: 0,
            cycles: delta(0),
            instructions: delta(1),
            branches: delta(2),
            branch_misses: delta(3),
            cache_references: delta(4),
            cache_misses: delta(5),
        }
    }
}

impl Drop for PerfCounters {
    fn drop(&mut self) {
        for counter in &self.counters {
            if counter.fd >= 0 {
                unsafe { libc::close(counter.fd) };
            }
        }
    }
}

/// `PERF_TYPE_HARDWARE` config codes, in the kernel's frozen enum order.
fn hardware_config(index: usize) -> u64 {
    const CONFIGS: [u64; 6] = [
        0, // PERF_COUNT_HW_CPU_CYCLES
        1, // PERF_COUNT_HW_INSTRUCTIONS
        4, // PERF_COUNT_HW_BRANCH_INSTRUCTIONS
        5, // PERF_COUNT_HW_BRANCH_MISSES
        2, // PERF_COUNT_HW_CACHE_REFERENCES
        3, // PERF_COUNT_HW_CACHE_MISSES
    ];
    CONFIGS[index]
}

fn open_one(config: u64) -> Result<i32, String> {
    let attr = PerfEventAttr {
        type_: 0, // PERF_TYPE_HARDWARE
        size: PERF_ATTR_SIZE_VER0,
        config,
        flags: FLAG_DISABLED | FLAG_EXCLUDE_KERNEL | FLAG_EXCLUDE_HV,
        ..Default::default()
    };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_perf_event_open,
            &attr as *const PerfEventAttr,
            0i32,  // pid = 0 (this process)
            -1i32, // cpu = -1 (any)
            -1i32, // group_fd
            0u64,  // flags
        )
    };
    if fd < 0 {
        Err(format!("{}", std::io::Error::last_os_error()))
    } else {
        Ok(fd as i32)
    }
}

fn read_value(fd: i32) -> Option<u64> {
    let mut value: u64 = 0;
    let read = unsafe {
        libc::read(
            fd,
            &mut value as *mut u64 as *mut libc::c_void,
            std::mem::size_of::<u64>(),
        )
    };
    if read == std::mem::size_of::<u64>() as isize {
        Some(value)
    } else {
        None
    }
}
