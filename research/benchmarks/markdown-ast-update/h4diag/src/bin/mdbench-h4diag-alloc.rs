//! `mdbench-h4diag-alloc` — built with `--features allocator`
//! (A_ALLOCATOR lane).
//!
//! The counting global allocator is installed HERE, in the binary, so no
//! other diagnostic or timing binary can contain it. It delegates to
//! `System` and adds requested-byte accounting only.

use std::alloc::System;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct CountingAllocator;

unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            markit_mdbench_h4diag::allocstat::on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            markit_mdbench_h4diag::allocstat::on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        markit_mdbench_h4diag::allocstat::on_dealloc(layout.size());
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        let out = System.realloc(ptr, layout, new_size);
        if !out.is_null() {
            markit_mdbench_h4diag::allocstat::on_realloc(layout.size(), new_size);
        }
        out
    }
}

fn main() -> std::process::ExitCode {
    markit_mdbench_h4diag::cli::main()
}
