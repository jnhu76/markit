//! The shared allocation-denial probe (task contract §32; #59 §9.2/§11):
//! a test-only global allocator that services every request for memory
//! safety but records each attempt made while the guard is active. The
//! guarded region (the commit frontier crossing) must attempt ZERO
//! allocations — post-frontier work is ownership moves, bounded explicit
//! stack manipulation, relinks, rotations, aggregate recomputation,
//! retirement and fixed-size scalar writes only.
//!
//! The probe is the global allocator of this test binary and simply
//! delegates to `System` outside the guard.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static DENY_ALLOC: Cell<bool> = const { Cell::new(false) };
    static ALLOC_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
}

struct FrontierAllocProbe;

unsafe impl GlobalAlloc for FrontierAllocProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if DENY_ALLOC.with(Cell::get) {
            ALLOC_ATTEMPTS.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static FRONTIER_ALLOC_PROBE: FrontierAllocProbe = FrontierAllocProbe;

/// Guard RAII: the flag is always restored, even on panic.
pub(crate) struct DenyGuard;

impl DenyGuard {
    pub(crate) fn deny() -> Self {
        DENY_ALLOC.with(|d| d.set(true));
        DenyGuard
    }
}

impl Drop for DenyGuard {
    fn drop(&mut self) {
        DENY_ALLOC.with(|d| d.set(false));
    }
}

/// Reset the attempt counter before entering the guarded region.
pub(crate) fn reset_attempts() {
    ALLOC_ATTEMPTS.with(|c| c.set(0));
}

/// The number of allocation attempts recorded since the last reset.
pub(crate) fn attempts() -> u64 {
    ALLOC_ATTEMPTS.with(Cell::get)
}
