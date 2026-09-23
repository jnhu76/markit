//! markit-mdbench-h4diag — Issue #50 (H4-LARGE-N-CAUSE-1) diagnostic crate.
//!
//! # What this crate is
//!
//! A **diagnostic-only** instrumented copy of the frozen H4
//! RESTART_CONVERGENCE mechanism, plus the Issue #50 frozen workload
//! cells and the diagnostic lane drivers. It exists to answer:
//!
//! > When parser work is held approximately fixed, which concrete
//! > operations cause H4 resident-update latency to grow from 128 KiB to
//! > 16 MiB?
//!
//! # What this crate is NOT
//!
//! - It is **not** a new horse and **not** a V1 design. Nothing here is
//!   a product or architecture claim.
//! - The frozen H4 crate (`markit-mdbench-restart-convergence`) is
//!   **untouched**: the U_PLAIN lane runs the original mechanism through
//!   the original frozen runner timer boundary, so the authoritative
//!   latency curve contains no diagnostic code at all.
//! - [`alg::H4Diag`] with [`alg::Variant::A0`] is a *copy*, and it is
//!   accepted as equivalent only because it is **verified** equivalent
//!   (`mdbench-h4diag verify-equivalence`: identical normalized-result
//!   checksums and identical frozen work counters on every cell, both
//!   `NoopWorkSink` and `CounterSink`).
//!
//! # Lane separation (Issue #50 §3, §5-§8)
//!
//! ```text
//! U_PLAIN      original H4, original frozen runner timers, NoopWorkSink,
//!              no phase timers, no counters, no allocator wrapper
//! U_PHASE      H4Diag + `phases` feature: mutually exclusive phase timers
//! W_COUNTERS   H4Diag + `counters` feature: direct operation counts
//! A_ALLOCATOR  H4Diag + `allocator` feature: counting global allocator
//! ```
//!
//! The features are **compile-time** on purpose: a timing binary must not
//! even contain the counting code it would otherwise be tempted to
//! interpret. `mdbench-h4diag` (default features) is the ablation and
//! A0-copy binary; `mdbench-h4diag-phases`, `mdbench-h4diag-counters` and
//! `mdbench-h4diag-alloc` are the instrumented ones.
//!
//! # Timer placement note (deliberate, documented deviation)
//!
//! `markit_mdbench_common::MechanismContext` deliberately exposes no
//! clock: in the frozen harness, timer placement belongs to
//! `runner`/`instrumentation`. The U_PHASE lane needs *inner* phase
//! boundaries that the frozen runner cannot express, so
//! [`alg::H4Diag`] reads its own diagnostic clock (behind the `phases`
//! feature). This is a **diagnostic lane only**: the frozen runner's
//! outer `T_prepare`/`T_native` timers are unchanged and are still what
//! `U_PHASE` reports; the inner phase times are reported as a separate,
//! explicitly-inclusive/exclusive decomposition, and their perturbation
//! is measured against U_PLAIN before any share is transferred.

pub mod alg;
pub mod allocstat;
pub mod cells;
pub mod cli;
pub mod counters;
pub mod deferred;
pub mod phases;
pub mod schedule;
pub mod stats;

#[cfg(feature = "allocator")]
pub mod alloc_wrapper;

pub use alg::{H4Diag, H4DiagState, Variant};
pub use cells::{all_cells, cell_by_label, Cell, CELL_LABELS};
