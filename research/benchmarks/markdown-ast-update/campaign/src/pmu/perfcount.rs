//! Region-scoped PMU counting for the PMU explanatory driver.
//!
//! Audited descendant of `campaign2/mdbench-replay/perfcount.rs` (task §6:
//! reuse the existing region-scoped `perf_event_open` mechanism, do not
//! copy blindly). Differences required by this study, all frozen before
//! any horse observation:
//!
//! - counters are opened as ONE GROUP (`group_fd` chaining) so the
//!   kernel schedules them atomically and never multiplexes a 2-event
//!   hardware group across the 3 watchdog-free generic PMCs;
//! - `read_format` carries `PERF_FORMAT_TOTAL_TIME_ENABLED |
//!   PERF_FORMAT_TOTAL_TIME_RUNNING`, so every event records
//!   `time_enabled`/`time_running` and the multiplexing qualification
//!   (`running_fraction == 1.0`) is checkable per event, never assumed;
//! - enable/disable/reset are issued on the GROUP LEADER only;
//! - per-event raw counts are read individually after disable (no group
//!   read format, no scaling of multiplexed values).
//!
//! User-space-only policy (host `perf_event_paranoid = 2`): every
//! counter opens with `exclude_kernel = 1` and `exclude_hv = 1`. A
//! refused counter is reported, never silently zeroed.

use super::events::EventDef;

/// `PERF_ATTR_SIZE_VER0`: the kernel reads only this many leading bytes.
const PERF_ATTR_SIZE_VER0: u32 = 64;

/// Flag-word bits (linux/perf_event.h bitfield order).
const FLAG_DISABLED: u64 = 1 << 0;
const FLAG_EXCLUDE_KERNEL: u64 = 1 << 5;
const FLAG_EXCLUDE_HV: u64 = 1 << 6;

/// `read_format` bits: `PERF_FORMAT_TOTAL_TIME_ENABLED |
/// PERF_FORMAT_TOTAL_TIME_RUNNING` — the fields this study exists to
/// validate.
const READ_FORMAT_TIMES: u64 = 1 | 2;

/// `_IO('$', n)` ioctl numbers.
const IOC_ENABLE: libc::c_ulong = 0x2400;
const IOC_DISABLE: libc::c_ulong = 0x2401;
const IOC_RESET: libc::c_ulong = 0x2403;

/// `PERF_IOC_FLAG_GROUP`: with this bit in the ioctl argument, the
/// RESET/ENABLE/DISABLE applies to EVERY member of the leader's group.
/// Without it the kernel touches the leader event ONLY — members stay
/// dead (enabled=0, running=0, count=0). Proven on the formal host
/// 2026-09-29 with a minimal C probe: arg 0 leaves the member at
/// 0/0/0 while arg 1 counts it with running == enabled.
const PERF_IOC_FLAG_GROUP: libc::c_ulong = 1;

/// The leading bytes of `struct perf_event_attr` the kernel reads.
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

/// What one event read returns under `READ_FORMAT_TIMES`.
///
/// `#[repr(C)]` is load-bearing: this is the direct `read()` buffer for
/// the kernel's `(value, time_enabled, time_running)` layout, so field
/// order must never be compiler-chosen.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventReading {
    pub value: u64,
    pub time_enabled: u64,
    pub time_running: u64,
}

impl EventReading {
    /// `running_fraction` denominator guard: never divide by a disabled
    /// counter silently.
    pub fn running_fraction(&self) -> Option<f64> {
        if self.time_enabled == 0 {
            None
        } else {
            Some(self.time_running as f64 / self.time_enabled as f64)
        }
    }
}

/// One open counter.
struct Counter {
    fd: i32,
}

impl Drop for Counter {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { libc::close(self.fd) };
        }
    }
}

/// An open event group: leader first, members chained via `group_fd`;
/// or, for the software diagnostics group, STANDALONE counters (see
/// [`super::events::EventGroup::standalone`] for the frozen empirical
/// reason).
pub struct EventCounterGroup {
    defs: &'static [EventDef],
    counters: Vec<Counter>,
    standalone: bool,
    /// Pre-enable snapshot (values ≈ 0 after RESET; kept so the region is
    /// a strict delta, mirroring the audited donor's baseline discipline).
    before: Vec<EventReading>,
}

impl EventCounterGroup {
    /// Open the counters for THIS process (`pid = 0`, `cpu = -1`),
    /// disabled at open. Grouped mode: the first event is the leader and
    /// the rest chain to it via `group_fd` (atomic scheduling, no
    /// multiplexing of a small hardware group). Standalone mode: every
    /// event opens with its own `group_fd = -1` (software events only).
    /// Any refusal fails the WHOLE set — this study never measures a
    /// partial group.
    pub fn open(defs: &'static [EventDef], standalone: bool) -> Result<Self, String> {
        let mut counters = Vec::with_capacity(defs.len());
        let mut leader_fd: i32 = -1;
        for (index, def) in defs.iter().enumerate() {
            let attr = PerfEventAttr {
                type_: def.kind,
                size: PERF_ATTR_SIZE_VER0,
                config: def.config,
                read_format: READ_FORMAT_TIMES,
                flags: FLAG_DISABLED | FLAG_EXCLUDE_KERNEL | FLAG_EXCLUDE_HV,
                ..Default::default()
            };
            let group_fd = if standalone { -1i32 } else { leader_fd };
            let fd = unsafe {
                libc::syscall(
                    libc::SYS_perf_event_open,
                    &attr as *const PerfEventAttr,
                    0i32,     // this process
                    -1i32,    // any cpu (affinity pins the worker)
                    group_fd, // group chaining: -1 for the leader / standalone
                    0u64,     // flags
                )
            };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                return Err(format!(
                    "perf_event_open({} #{index}) refused: {error}",
                    def.alias
                ));
            }
            let fd = fd as i32;
            if index == 0 {
                leader_fd = fd;
            }
            counters.push(Counter { fd });
        }
        Ok(Self {
            defs,
            counters,
            standalone,
            before: Vec::new(),
        })
    }

    fn scope_ioctl(&self, request: libc::c_ulong, what: &str) -> Result<(), String> {
        let failed = if self.standalone {
            self.counters
                .iter()
                .any(|counter| unsafe { libc::ioctl(counter.fd, request, 0) != 0 })
        } else {
            // Grouped mode: the leader's ioctl with PERF_IOC_FLAG_GROUP
            // controls the whole group.
            unsafe { libc::ioctl(self.counters[0].fd, request, PERF_IOC_FLAG_GROUP) != 0 }
        };
        if failed {
            return Err(format!(
                "ioctl {what} failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    fn read_one(fd: i32) -> Result<EventReading, String> {
        let mut reading = EventReading::default();
        let bytes = unsafe {
            libc::read(
                fd,
                &mut reading as *mut EventReading as *mut libc::c_void,
                std::mem::size_of::<EventReading>(),
            )
        };
        if bytes != std::mem::size_of::<EventReading>() as isize {
            return Err(format!(
                "counter read returned {bytes} bytes (expected {})",
                std::mem::size_of::<EventReading>()
            ));
        }
        Ok(reading)
    }

    /// Reset the group and snapshot the (disabled) baseline, then enable
    /// every counter atomically via the leader.
    pub fn begin(&mut self) -> Result<(), String> {
        self.scope_ioctl(IOC_RESET, "PERF_EVENT_IOC_RESET")?;
        let mut before = Vec::with_capacity(self.counters.len());
        for counter in &self.counters {
            before.push(Self::read_one(counter.fd)?);
        }
        self.before = before;
        self.scope_ioctl(IOC_ENABLE, "PERF_EVENT_IOC_ENABLE")?;
        Ok(())
    }

    /// Disable the group and return the per-event region deltas.
    pub fn end(&mut self) -> Result<Vec<EventReading>, String> {
        self.scope_ioctl(IOC_DISABLE, "PERF_EVENT_IOC_DISABLE")?;
        let mut after = Vec::with_capacity(self.counters.len());
        for counter in &self.counters {
            after.push(Self::read_one(counter.fd)?);
        }
        let before = std::mem::take(&mut self.before);
        Ok(after
            .into_iter()
            .zip(before)
            .map(|(a, b)| EventReading {
                value: a.value.saturating_sub(b.value),
                time_enabled: a.time_enabled.saturating_sub(b.time_enabled),
                time_running: a.time_running.saturating_sub(b.time_running),
            })
            .collect())
    }

    /// The frozen event definitions of this group.
    pub fn defs(&self) -> &'static [EventDef] {
        self.defs
    }
}
