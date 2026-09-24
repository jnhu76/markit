//! Adrop — ownership-correct deferred retirement (Issue #50 §10).
//!
//! The `Adrop` variant moves the **complete consumed old state** into an
//! explicit post-timer owner instead of letting it drop inside the timed
//! region. Nothing is cloned to achieve the retention: the state is
//! *moved*, so the produced state, the algorithm, and every reference
//! relation are identical to A0 — only the moment of destruction moves.
//!
//! Temporary objects (the convergence cursor, the splice hook box, the
//! pairs vector) keep their original lifetimes. The deferred set is
//! therefore exactly "the consumed old `H4DiagState`", which is what the
//! retirement inventory names.
//!
//! The parked state is destroyed deterministically by
//! [`drain_counted`], called by the harness *after* the frozen outer
//! timer has stopped, and before the next fresh state is built. The
//! process never accumulates more than one parked state.
//!
//! # Retirement inventory (Issue #50 §10)
//!
//! Four distinct things are kept apart, and none of them is summarised as
//! "old state":
//!
//! 1. old `Arc` handles for unchanged blocks — releasing one normally
//!    only decrements a count, because the new state holds the shared
//!    payload;
//! 2. the last reference to a replaced block, and its payload;
//! 3. the old `blocks` / `checkpoints` vectors and their owned keys;
//! 4. temporaries (cursor / hook box / pairs) — which this variant does
//!    **not** defer, and therefore does not list as deferred.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use crate::alg::H4DiagState;

thread_local! {
    static PARKED: RefCell<Option<H4DiagState>> = const { RefCell::new(None) };
    static INVENTORY: Cell<RetirementInventory> = const { Cell::new(RetirementInventory::EMPTY) };
}

/// Counts describing exactly what was deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetirementInventory {
    /// Old `BlockSlot` elements moved into the deferred owner.
    pub old_block_slots: u64,
    /// Old `Checkpoint` records moved into the deferred owner.
    pub old_checkpoint_records: u64,
    /// Old block handles whose payload the NEW state still holds
    /// (retained prefix + converged suffix): releasing them decrements a
    /// strong count only.
    pub handles_kept_shared: u64,
    /// Old block handles that were the last reference (replaced block):
    /// draining them destroys the payload.
    pub handles_last_reference: u64,
}

impl RetirementInventory {
    pub const EMPTY: Self = Self {
        old_block_slots: 0,
        old_checkpoint_records: 0,
        handles_kept_shared: 0,
        handles_last_reference: 0,
    };

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "old_block_slots": self.old_block_slots,
            "old_checkpoint_records": self.old_checkpoint_records,
            "handles_kept_shared": self.handles_kept_shared,
            "handles_last_reference": self.handles_last_reference,
        })
    }
}

/// Record the deferred-retirement inventory for the current observation.
pub fn set_inventory(inventory: RetirementInventory) {
    INVENTORY.with(|i| i.set(inventory));
}

/// Read the recorded inventory.
pub fn inventory() -> RetirementInventory {
    INVENTORY.with(|i| i.get())
}

/// Park the consumed old state for post-timer destruction.
///
/// Panics if a state is already parked: the harness must drain before
/// building the next sample, and a silently stacked second state would
/// make the retention unbounded (Issue #50 §10 forbids leaking
/// indefinitely).
pub fn park(state: H4DiagState) {
    PARKED.with(|p| {
        let mut p = p.borrow_mut();
        assert!(
            p.is_none(),
            "Adrop: a state is already parked — the harness must drain after each measured update"
        );
        *p = Some(state);
    });
}

/// Whether a state is currently parked.
pub fn is_parked() -> bool {
    PARKED.with(|p| p.borrow().is_some())
}

/// Destroy the parked state (if any) and report what was destroyed.
///
/// This is the timed `T_drain` body. The A0 control calls the same
/// function with nothing parked, so the empty-drain control measures the
/// identical code path with no retirement work.
pub fn drain_counted() -> DrainReport {
    PARKED.with(|p| {
        let mut p = p.borrow_mut();
        match p.take() {
            Some(state) => {
                let mut report = DrainReport {
                    drained_states: 1,
                    ..DrainReport::default()
                };
                let H4DiagState {
                    blocks,
                    checkpoints,
                    defs,
                    ..
                } = state;
                report.block_slots_destroyed = blocks.len() as u64;
                report.checkpoint_records_destroyed = checkpoints.len() as u64;
                for slot in blocks {
                    // A strong count of 1 here means this handle is the last
                    // reference: dropping it frees the retained payload.
                    if Arc::strong_count(&slot.block) == 1 {
                        report.payloads_destroyed += 1;
                    } else {
                        report.shared_handles_released += 1;
                    }
                    drop(slot.block);
                }
                report.def_entries_destroyed = defs.len() as u64;
                drop(defs);
                drop(checkpoints);
                report
            }
            None => DrainReport::default(),
        }
    })
}

/// What one drain destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrainReport {
    pub drained_states: u64,
    pub block_slots_destroyed: u64,
    pub checkpoint_records_destroyed: u64,
    pub shared_handles_released: u64,
    pub payloads_destroyed: u64,
    pub def_entries_destroyed: u64,
}

impl DrainReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "drained_states": self.drained_states,
            "block_slots_destroyed": self.block_slots_destroyed,
            "checkpoint_records_destroyed": self.checkpoint_records_destroyed,
            "shared_handles_released": self.shared_handles_released,
            "payloads_destroyed": self.payloads_destroyed,
            "def_entries_destroyed": self.def_entries_destroyed,
        })
    }
}
