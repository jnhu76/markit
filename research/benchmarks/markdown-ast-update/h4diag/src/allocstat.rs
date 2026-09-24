//! A_ALLOCATOR — counting-allocator accounting (Issue #50 §8).
//!
//! Only compiled under the `allocator` feature, and only linked into
//! `mdbench-h4diag-alloc`, so **no timing lane can ever contain the
//! allocator wrapper**. Allocator instrumentation is never mixed with
//! U_PLAIN (Issue #50 §8).
//!
//! # Byte semantics (Issue #50 §7)
//!
//! Everything here is **requested payload bytes** as seen by
//! `GlobalAlloc` — `Layout::size()` at the call site. That is not
//! allocator-internal overhead and not RSS. `realloc` reports the old and
//! new requested sizes separately (both are known exactly from the two
//! `Layout`s); no temporary double-buffering performed *inside* the
//! allocator is observable here, and none is claimed.
//!
//! `live` is the running sum of requested bytes currently held by
//! successfully returned allocations. It is maintained from process
//! start, so the baseline `B0` read when the window opens is a real
//! baseline and not a fabricated zero. `peak_live` is exact for the
//! single-threaded pinned worker this lane uses (a relaxed
//! load/compare/store on one thread cannot lose an update).
//!
//! # Category attribution
//!
//! Allocations are attributed to the phase whose boundary was most
//! recently announced (`set_category`) — one relaxed atomic store per
//! boundary, no per-object tagging, no change to the representation.
//! This is phase-scoped attribution: an allocation made deep inside a
//! called library while category X is active is attributed to X. It is
//! reported as such and never presented as per-object provenance.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicU8, Ordering};

/// Number of attribution categories (kept in sync with [`CATEGORY_NAMES`]).
pub const CATEGORY_COUNT: usize = 10;

/// Category names in index order; the CSV header uses these.
pub const CATEGORY_NAMES: [&str; CATEGORY_COUNT] = [
    "untagged",
    "forward_parse",
    "prefix_assembly",
    "defs_table",
    "fresh_materialization",
    "suffix_assembly",
    "pairs_vec",
    "slots_checkpoints",
    "retirement",
    "seal_other",
];

pub const CAT_UNTAGGED: u8 = 0;
pub const CAT_FORWARD_PARSE: u8 = 1;
pub const CAT_PREFIX_ASSEMBLY: u8 = 2;
pub const CAT_DEFS_TABLE: u8 = 3;
pub const CAT_FRESH_MATERIALIZATION: u8 = 4;
pub const CAT_SUFFIX_ASSEMBLY: u8 = 5;
pub const CAT_PAIRS_VEC: u8 = 6;
pub const CAT_SLOTS_CHECKPOINTS: u8 = 7;
pub const CAT_RETIREMENT: u8 = 8;
pub const CAT_SEAL_OTHER: u8 = 9;

/// Map a category name used by the `alloc_cat!` macro to its index.
#[macro_export]
macro_rules! concat_idents_cat {
    (UNTAGGED) => {
        $crate::allocstat::CAT_UNTAGGED
    };
    (FORWARD_PARSE) => {
        $crate::allocstat::CAT_FORWARD_PARSE
    };
    (PREFIX_ASSEMBLY) => {
        $crate::allocstat::CAT_PREFIX_ASSEMBLY
    };
    (DEFS_TABLE) => {
        $crate::allocstat::CAT_DEFS_TABLE
    };
    (FRESH_MATERIALIZATION) => {
        $crate::allocstat::CAT_FRESH_MATERIALIZATION
    };
    (SUFFIX_ASSEMBLY) => {
        $crate::allocstat::CAT_SUFFIX_ASSEMBLY
    };
    (PAIRS_VEC) => {
        $crate::allocstat::CAT_PAIRS_VEC
    };
    (SLOTS_CHECKPOINTS) => {
        $crate::allocstat::CAT_SLOTS_CHECKPOINTS
    };
    (RETIREMENT) => {
        $crate::allocstat::CAT_RETIREMENT
    };
    (SEAL_OTHER) => {
        $crate::allocstat::CAT_SEAL_OTHER
    };
}

static ARMED: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicI64 = AtomicI64::new(0);
static CATEGORY: AtomicU8 = AtomicU8::new(CAT_UNTAGGED);

static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOC_OLD_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOC_NEW_BYTES: AtomicU64 = AtomicU64::new(0);
static FREED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_START: AtomicI64 = AtomicI64::new(0);
static PEAK_LIVE: AtomicI64 = AtomicI64::new(0);

/// Per-category requested bytes allocated inside the window.
static CAT_ALLOC_BYTES: [AtomicU64; CATEGORY_COUNT] = [const { AtomicU64::new(0) }; CATEGORY_COUNT];
/// Per-category alloc+realloc call count inside the window.
static CAT_ALLOC_CALLS: [AtomicU64; CATEGORY_COUNT] = [const { AtomicU64::new(0) }; CATEGORY_COUNT];

/// Announce the attribution category for subsequent allocations.
#[inline]
pub fn set_category(category: u8) {
    CATEGORY.store(category, Ordering::Relaxed);
}

/// Current attribution category.
#[inline]
pub fn category() -> u8 {
    CATEGORY.load(Ordering::Relaxed)
}

/// Open the accounting window (Issue #50 §7: the live baseline `B0` is
/// established from the already-built fresh state, before the update).
pub fn arm() {
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    REALLOC_OLD_BYTES.store(0, Ordering::Relaxed);
    REALLOC_NEW_BYTES.store(0, Ordering::Relaxed);
    FREED_BYTES.store(0, Ordering::Relaxed);
    for i in 0..CATEGORY_COUNT {
        CAT_ALLOC_BYTES[i].store(0, Ordering::Relaxed);
        CAT_ALLOC_CALLS[i].store(0, Ordering::Relaxed);
    }
    let live = LIVE.load(Ordering::Relaxed);
    LIVE_START.store(live, Ordering::Relaxed);
    PEAK_LIVE.store(live, Ordering::Relaxed);
    CATEGORY.store(CAT_UNTAGGED, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
}

/// Close the accounting window and return the snapshot.
pub fn disarm() -> AllocWindow {
    ARMED.store(false, Ordering::Relaxed);
    let live_end = LIVE.load(Ordering::Relaxed);
    let mut cat_alloc_calls = [0u64; CATEGORY_COUNT];
    let mut cat_alloc_bytes = [0u64; CATEGORY_COUNT];
    for i in 0..CATEGORY_COUNT {
        cat_alloc_calls[i] = CAT_ALLOC_CALLS[i].load(Ordering::Relaxed);
        cat_alloc_bytes[i] = CAT_ALLOC_BYTES[i].load(Ordering::Relaxed);
    }
    AllocWindow {
        alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
        realloc_calls: REALLOC_CALLS.load(Ordering::Relaxed),
        dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
        alloc_requested_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        realloc_old_requested_bytes: REALLOC_OLD_BYTES.load(Ordering::Relaxed),
        realloc_new_requested_bytes: REALLOC_NEW_BYTES.load(Ordering::Relaxed),
        freed_requested_bytes: FREED_BYTES.load(Ordering::Relaxed),
        live_start_requested_bytes: LIVE_START.load(Ordering::Relaxed),
        live_end_requested_bytes: live_end,
        peak_live_requested_bytes: PEAK_LIVE.load(Ordering::Relaxed),
        cat_alloc_calls,
        cat_alloc_bytes,
    }
}

/// Live requested bytes right now (maintained from process start).
#[inline]
pub fn live_requested_bytes() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

/// Whether the accounting window is open.
#[inline]
pub fn armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

#[inline]
fn note_peak(live: i64) {
    // Single-threaded worker: load/compare/store is exact here. Documented
    // in the module header; not claimed for multi-threaded scopes.
    if live > PEAK_LIVE.load(Ordering::Relaxed) {
        PEAK_LIVE.store(live, Ordering::Relaxed);
    }
}

/// Allocator hook: a successful `alloc` of `size` requested bytes.
#[inline]
pub fn on_alloc(size: usize) {
    let live = LIVE.fetch_add(size as i64, Ordering::Relaxed) + size as i64;
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    ALLOC_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    let cat = CATEGORY.load(Ordering::Relaxed) as usize;
    CAT_ALLOC_CALLS[cat].fetch_add(1, Ordering::Relaxed);
    CAT_ALLOC_BYTES[cat].fetch_add(size as u64, Ordering::Relaxed);
    note_peak(live);
}

/// Allocator hook: a successful `realloc` from `old` to `new` requested
/// bytes.
#[inline]
pub fn on_realloc(old: usize, new: usize) {
    let live = LIVE.fetch_add(new as i64 - old as i64, Ordering::Relaxed)
        + (new as i64 - old as i64);
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    REALLOC_OLD_BYTES.fetch_add(old as u64, Ordering::Relaxed);
    REALLOC_NEW_BYTES.fetch_add(new as u64, Ordering::Relaxed);
    let cat = CATEGORY.load(Ordering::Relaxed) as usize;
    CAT_ALLOC_CALLS[cat].fetch_add(1, Ordering::Relaxed);
    CAT_ALLOC_BYTES[cat].fetch_add(new as u64, Ordering::Relaxed);
    note_peak(live);
}

/// Allocator hook: a successful `dealloc` of `size` requested bytes.
#[inline]
pub fn on_dealloc(size: usize) {
    LIVE.fetch_sub(size as i64, Ordering::Relaxed);
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
    FREED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
}

/// One accounting window's facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocWindow {
    pub alloc_calls: u64,
    pub realloc_calls: u64,
    pub dealloc_calls: u64,
    pub alloc_requested_bytes: u64,
    pub realloc_old_requested_bytes: u64,
    pub realloc_new_requested_bytes: u64,
    pub freed_requested_bytes: u64,
    pub live_start_requested_bytes: i64,
    pub live_end_requested_bytes: i64,
    pub peak_live_requested_bytes: i64,
    pub cat_alloc_calls: [u64; CATEGORY_COUNT],
    pub cat_alloc_bytes: [u64; CATEGORY_COUNT],
}

impl AllocWindow {
    /// `peak_live - live_start`, the headline growth quantity of §7.
    pub fn peak_growth(&self) -> i64 {
        self.peak_live_requested_bytes - self.live_start_requested_bytes
    }

    /// Net live change across the window.
    pub fn net_live_change(&self) -> i64 {
        self.live_end_requested_bytes - self.live_start_requested_bytes
    }

    /// Total requested bytes handed out inside the window (alloc +
    /// realloc-new).
    pub fn total_requested_bytes(&self) -> u64 {
        self.alloc_requested_bytes + self.realloc_new_requested_bytes
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut cat = serde_json::Map::new();
        for i in 0..CATEGORY_COUNT {
            cat.insert(
                CATEGORY_NAMES[i].to_string(),
                serde_json::json!({
                    "calls": self.cat_alloc_calls[i],
                    "requested_bytes": self.cat_alloc_bytes[i],
                }),
            );
        }
        serde_json::json!({
            "alloc_calls": self.alloc_calls,
            "realloc_calls": self.realloc_calls,
            "dealloc_calls": self.dealloc_calls,
            "alloc_requested_bytes": self.alloc_requested_bytes,
            "realloc_old_requested_bytes": self.realloc_old_requested_bytes,
            "realloc_new_requested_bytes": self.realloc_new_requested_bytes,
            "freed_requested_bytes": self.freed_requested_bytes,
            "live_start_requested_bytes": self.live_start_requested_bytes,
            "live_end_requested_bytes": self.live_end_requested_bytes,
            "peak_live_requested_bytes": self.peak_live_requested_bytes,
            "peak_growth_requested_bytes": self.peak_growth(),
            "net_live_change_requested_bytes": self.net_live_change(),
            "by_category": serde_json::Value::Object(cat),
        })
    }
}

/// CSV columns of the allocator report (window quantities).
pub const ALLOC_COLUMNS: &[&str] = &[
    "alloc_calls",
    "realloc_calls",
    "dealloc_calls",
    "alloc_requested_bytes",
    "realloc_old_requested_bytes",
    "realloc_new_requested_bytes",
    "freed_requested_bytes",
    "total_requested_bytes",
    "live_start_requested_bytes",
    "live_end_requested_bytes",
    "peak_live_requested_bytes",
    "peak_growth_requested_bytes",
    "net_live_change_requested_bytes",
];

impl AllocWindow {
    /// One row in [`ALLOC_COLUMNS`] order.
    pub fn row(&self) -> Vec<String> {
        vec![
            self.alloc_calls.to_string(),
            self.realloc_calls.to_string(),
            self.dealloc_calls.to_string(),
            self.alloc_requested_bytes.to_string(),
            self.realloc_old_requested_bytes.to_string(),
            self.realloc_new_requested_bytes.to_string(),
            self.freed_requested_bytes.to_string(),
            self.total_requested_bytes().to_string(),
            self.live_start_requested_bytes.to_string(),
            self.live_end_requested_bytes.to_string(),
            self.peak_live_requested_bytes.to_string(),
            self.peak_growth().to_string(),
            self.net_live_change().to_string(),
        ]
    }
}
