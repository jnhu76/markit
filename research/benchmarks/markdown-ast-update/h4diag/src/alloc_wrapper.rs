//! Counting global allocator (Issue #50 §8).
//!
//! Only compiled under the `allocator` feature, and only linked into
//! `mdbench-h4diag-alloc`. It delegates every operation to
//! [`std::alloc::System`] — the same underlying allocator every other
//! lane uses — and adds accounting only. The counters themselves never
//! allocate and never recurse: they are plain atomics.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::allocstat;

/// The `System` allocator plus requested-byte accounting.
pub struct CountingAllocator;

static INSTALLED: AtomicBool = AtomicBool::new(true);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() && INSTALLED.load(Ordering::Relaxed) {
            allocstat::on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() && INSTALLED.load(Ordering::Relaxed) {
            allocstat::on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if INSTALLED.load(Ordering::Relaxed) {
            allocstat::on_dealloc(layout.size());
        }
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let out = System.realloc(ptr, layout, new_size);
        if !out.is_null() && INSTALLED.load(Ordering::Relaxed) {
            allocstat::on_realloc(layout.size(), new_size);
        }
        out
    }
}
