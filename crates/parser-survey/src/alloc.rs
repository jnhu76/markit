//! Counting global allocator: allocation count + bytes around measured
//! regions (issue #19 `alloc_count` / `allocated_bytes` metrics).
//!
//! Single-threaded harness: the counters are cumulative process-wide and
//! only differences across a measured closure are reported.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocation count + requested bytes inside a measured region.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllocDelta {
    pub count: u64,
    pub bytes: u64,
}

/// Runs `f`, returning its output, wall-clock microseconds, and the
/// allocation delta of the call.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, f64, AllocDelta) {
    let c0 = ALLOC_COUNT.load(Ordering::Relaxed);
    let b0 = ALLOC_BYTES.load(Ordering::Relaxed);
    let t0 = Instant::now();
    let out = f();
    let us = t0.elapsed().as_nanos() as f64 / 1000.0;
    let c1 = ALLOC_COUNT.load(Ordering::Relaxed);
    let b1 = ALLOC_BYTES.load(Ordering::Relaxed);
    (
        out,
        us,
        AllocDelta {
            count: c1 - c0,
            bytes: b1 - b0,
        },
    )
}
