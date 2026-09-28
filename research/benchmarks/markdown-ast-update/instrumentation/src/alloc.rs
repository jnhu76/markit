//! Real allocation/retained-memory backend for the M-LANE (#76 Gate B).
//!
//! The R1-CORRECTIVE-1 memory lane shipped its lifecycle and an honest
//! placeholder only. This module adds the real backend:
//!
//! - [`CountingAllocator`] — a `GlobalAlloc` wrapper over the system
//!   allocator that maintains process-global allocation/deallocation
//!   counters and a live-bytes gauge. It is installed as the process
//!   allocator ONLY in a dedicated M-LANE binary ("separate instrumented
//!   runs"): the timing and attribution lanes always run under the plain
//!   system allocator, so measurement overhead can never enter a timed
//!   region (R0/R1 lane rules; MEASUREMENT-CORRECTIVE-1).
//! - [`AllocReporter`] — a `MemoryReporter` whose per-case windows are
//!   counter snapshots around exactly one case run, satisfying the frozen
//!   per-case window lifecycle (`begin_case`/`end_case`).
//!
//! Counting semantics (frozen before any formal memory row):
//!
//! - every `alloc` is ONE allocation event charged its requested size;
//! - every `dealloc` is ONE deallocation event charged its released
//!   size (the live-bytes gauge decrements);
//! - `realloc` is ONE deallocation of the old block plus ONE allocation
//!   of the new block (the conservative default-path semantics);
//! - `alloc_zeroed` is ONE allocation event;
//! - shared allocations (e.g. `Arc` clones) are counted ONCE at creation
//!   and once at the final destroy: a clone increments a reference
//!   count, not the allocator;
//! - `allocated_bytes`/`allocation_count` are window DELTAS of the
//!   cumulative counters;
//! - `peak_bytes` is the maximum LIVE (allocated-minus-freed) bytes
//!   observed inside the window; the gauge resets at `begin_case`;
//! - `retained_bytes` is `live_at_close - live_at_open` SATURATING at
//!   zero: the schema slot is unsigned, so a window that ends holding
//!   FEWER live bytes than it opened with (the update shrank the
//!   retained state) reports the measured zero, never a wrapped value.
//!   Growth is always exact. The absolute live counts of both window
//!   edges are available for a signed re-derivation in analysis.
//!
//! Threading: the campaign worker is strictly single-threaded during
//! measurement (one case × one horse × one iteration at a time), and the
//! counters are atomics regardless. Windows must not overlap; the
//! open-window stack detects and refuses a second `begin_case` while one
//! is open.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

use markit_mdbench_common::Observed;

use crate::lanes::{CaseMemoryProbe, MemoryRecord, MemoryReporter};

static ALLOC_EVENTS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOC_EVENTS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Maximum live bytes since the last `begin_window`; reset per window.
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Saturating-counter flag: an overflow reports an `Unknown` byte total
/// instead of lying with a wrapped number (event counts cannot wrap in
/// practice; the byte totals can in principle).
static BYTE_OVERFLOW: AtomicBool = AtomicBool::new(false);

fn saturating_add(counter: &AtomicU64, n: u64) {
    loop {
        let cur = counter.load(Ordering::Relaxed);
        match cur.checked_add(n) {
            Some(next) => {
                if counter
                    .compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
            }
            None => {
                BYTE_OVERFLOW.store(true, Ordering::Relaxed);
                return;
            }
        }
    }
}

fn live_add(n: usize) {
    let live = LIVE_BYTES.fetch_add(n, Ordering::Relaxed) + n;
    // Running max: a benign race under the strictly serial M-LANE.
    PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
}

fn live_sub(n: usize) {
    loop {
        let cur = LIVE_BYTES.load(Ordering::Relaxed);
        let next = cur.saturating_sub(n);
        if LIVE_BYTES
            .compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }
    }
}

/// The M-LANE process allocator: the system allocator plus counter
/// maintenance. Install with
/// `#[global_allocator] static A: CountingAllocator = CountingAllocator;`
/// in the dedicated memory-lane binary only.
pub struct CountingAllocator;

impl CountingAllocator {
    fn note_alloc(&self, size: usize) {
        saturating_add(&ALLOC_EVENTS, 1);
        saturating_add(&ALLOC_BYTES, size as u64);
        live_add(size);
    }

    fn note_dealloc(&self, size: usize) {
        saturating_add(&DEALLOC_EVENTS, 1);
        saturating_add(&DEALLOC_BYTES, size as u64);
        live_sub(size);
    }
}

// SAFETY: every operation delegates to the system allocator unchanged;
// the added atomics maintain counters only and never affect allocation
// results or pointer identity.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            self.note_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.note_dealloc(layout.size());
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Frozen semantics: one dealloc of the old block + one alloc of
        // the new block.
        self.note_dealloc(layout.size());
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            self.note_alloc(new_size);
        }
        new_ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            self.note_alloc(layout.size());
        }
        ptr
    }
}

/// One open M-LANE window: the counter snapshot taken at `begin_case`.
struct WindowSnapshot {
    alloc_events: u64,
    alloc_bytes: u64,
    live_at_open: usize,
    case_index: u64,
}

/// At most one window open at a time (the M-LANE is strictly serial).
static WINDOW_OPEN: Mutex<bool> = Mutex::new(false);
/// LIFO stack of open windows (one element in practice).
static OPEN_WINDOWS: Mutex<Vec<WindowSnapshot>> = Mutex::new(Vec::new());

fn begin_window(case_index: u64) -> Result<WindowSnapshot, String> {
    let mut open = WINDOW_OPEN.lock().expect("window mutex poisoned");
    if *open {
        return Err("M-LANE windows must not overlap: begin_case inside an open window".into());
    }
    *open = true;
    drop(open);
    // Reset the peak gauge to the current live level so the window peak
    // describes only this window.
    PEAK_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
    BYTE_OVERFLOW.store(false, Ordering::Relaxed);
    Ok(WindowSnapshot {
        alloc_events: ALLOC_EVENTS.load(Ordering::Relaxed),
        alloc_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        live_at_open: LIVE_BYTES.load(Ordering::Relaxed),
        case_index,
    })
}

fn end_window(snapshot: WindowSnapshot) -> MemoryRecord {
    let delta = |total: u64, begin: u64| total.saturating_sub(begin);
    let retained =
        (LIVE_BYTES.load(Ordering::Relaxed) as i64 - snapshot.live_at_open as i64).max(0) as u64;
    let record = MemoryRecord {
        allocated_bytes: if BYTE_OVERFLOW.load(Ordering::Relaxed) {
            Observed::Unknown
        } else {
            Observed::Known(delta(
                ALLOC_BYTES.load(Ordering::Relaxed),
                snapshot.alloc_bytes,
            ))
        },
        allocation_count: Observed::Known(delta(
            ALLOC_EVENTS.load(Ordering::Relaxed),
            snapshot.alloc_events,
        )),
        peak_bytes: Observed::Known(PEAK_BYTES.load(Ordering::Relaxed) as u64),
        retained_bytes: Observed::Known(retained),
    };
    let mut open = WINDOW_OPEN.lock().expect("window mutex poisoned");
    *open = false;
    drop(open);
    record
}

/// M-LANE reporter over the [`CountingAllocator`] process counters.
///
/// Exactly one window per case, opened before the mechanism runs and
/// closed strictly after it finished — the frozen [`MemoryReporter`]
/// lifecycle. Every opened probe MUST be closed, including on failure
/// paths, so the lane is preserved.
pub struct AllocReporter;

impl AllocReporter {
    pub fn new() -> Self {
        AllocReporter
    }
}

impl Default for AllocReporter {
    fn default() -> Self {
        AllocReporter
    }
}

fn next_case_index() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed) + 1
}

impl MemoryReporter for AllocReporter {
    fn begin_case(&self) -> CaseMemoryProbe {
        let index = next_case_index();
        let snapshot = begin_window(index).expect("M-LANE window overlap");
        OPEN_WINDOWS
            .lock()
            .expect("windows mutex poisoned")
            .push(snapshot);
        CaseMemoryProbe::new(index)
    }

    fn end_case(&self, probe: CaseMemoryProbe) -> MemoryRecord {
        let snapshot = OPEN_WINDOWS.lock().expect("windows mutex poisoned").pop();
        match snapshot {
            Some(s) => {
                debug_assert_eq!(probe.case_index(), s.case_index);
                end_window(s)
            }
            // Closing a window that was never opened is a caller bug; the
            // lane stays honest with an unavailable record.
            None => MemoryRecord::unavailable(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Window arithmetic against the process-global counters. The test
    /// binary does NOT install a CountingAllocator, so no other code
    /// touches these atomics and the synthetic traffic is exact. One
    /// combined `#[test]` keeps the global state single-threaded.
    #[test]
    fn window_delta_peak_and_retained_semantics() {
        // -- growth window: +3 allocs (300 bytes), one dealloc (50) --
        // Synthetic live level BEFORE the window opens: the snapshot
        // taken at begin is live_at_open = 600.
        LIVE_BYTES.store(600, Ordering::Relaxed);
        PEAK_BYTES.store(600, Ordering::Relaxed);
        let begin = begin_window(1).expect("window opens");
        saturating_add(&ALLOC_EVENTS, 3);
        saturating_add(&ALLOC_BYTES, 300);
        saturating_add(&DEALLOC_EVENTS, 1);
        saturating_add(&DEALLOC_BYTES, 50);
        live_sub(50);
        live_add(300); // peak probe: live touches 850
        let record = end_window(begin);
        assert_eq!(record.allocation_count, Observed::Known(3));
        assert_eq!(record.allocated_bytes, Observed::Known(300));
        assert_eq!(record.peak_bytes, Observed::Known(850));
        // Retained: live_at_close (850) - live_at_open (600) = 250.
        assert_eq!(record.retained_bytes, Observed::Known(250));

        // -- shrink window saturates at the measured zero --
        let begin = begin_window(2).expect("window opens");
        LIVE_BYTES.store(850, Ordering::Relaxed);
        PEAK_BYTES.store(850, Ordering::Relaxed);
        live_sub(150); // update shrank the retained state
        let record = end_window(begin);
        assert_eq!(record.retained_bytes, Observed::Known(0));
        assert_eq!(record.allocation_count, Observed::Known(0));

        // -- byte overflow is honest Unknown --
        let begin = begin_window(3).expect("window opens");
        saturating_add(&ALLOC_BYTES, u64::MAX);
        saturating_add(&ALLOC_BYTES, 1);
        let record = end_window(begin);
        assert_eq!(record.allocated_bytes, Observed::Unknown);

        // -- overlapping windows are refused --
        let _first = begin_window(4).expect("window opens");
        assert!(begin_window(5).is_err());
        let record = end_window(_first);
        assert_eq!(record.allocation_count, Observed::Known(0));

        // restore a clean level for any later window
        LIVE_BYTES.store(0, Ordering::Relaxed);
        PEAK_BYTES.store(0, Ordering::Relaxed);
    }
}
