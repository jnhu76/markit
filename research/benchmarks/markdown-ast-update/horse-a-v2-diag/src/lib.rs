#![recursion_limit = "512"]

//! markit-mdbench-horse-a-v2-diag — Issue #98 L0+L1 diagnostic driver
//! (E6 + tiny same-READY work/effect decomposition).
//!
//! # What this crate is
//!
//! DIAGNOSTIC TOOLING ONLY (the h4diag precedent applied to #98):
//!
//! - `cells` — the frozen #98 diagnostic cell manifest: six E6 semantic
//!   challenges, three tiny boundary cells, three local/container
//!   sentinels, all with stable IDs and deterministic byte-exact
//!   construction from the frozen campaign2 controlled-construction
//!   lineage (`para_region` padding + the C-F reference-fanout document
//!   geometry).
//! - `validate_control` — the L0.5 comparator validation: every cell
//!   must pass `HORSE_A_V1_DIRECT_READY_REBUILD` vs
//!   `HORSE_A_V1_NORMAL` parity/semantic/next-edit/lifecycle gates
//!   BEFORE any treatment data is collected.
//! - `tlane` — the T-LANE: minimally instrumented end-to-end comparator
//!   timing (A = Horse-A v1 normal, B = direct READY rebuild,
//!   C = H0 reference, D = H2 reference for E6) with a recorded
//!   warmup/repetition policy.
//! - `alane` — the A-LANE: one un-timed attributed pass per cell/arm
//!   reusing the frozen counter surfaces (`WorkCounters` via
//!   `CounterSink` for every arm; `HORSE-A-STRUCTURAL-COUNTERS-v1`
//!   via the recording structural sink for the Horse-A arms).
//! - `effects` — the offline required-effect ledger, derived from the
//!   frozen H0 normalized oracle (definition facts, effective
//!   first-wins winner map, node-signature output diff). This is
//!   DIAGNOSTIC knowledge only — never a free online mechanism.
//!
//! # What this crate is NOT
//!
//! - It is not Horse-A v2 and contains no candidate mechanism.
//! - It does not modify the frozen Horse-A v1 pipeline. The only #98
//!   change inside a frozen crate is `horse_a::direct_ready` — a
//!   clearly-separated diagnostic control that no frozen pipeline path
//!   calls — plus two `pub(crate)` visibility widenings that change no
//!   behavior.
//! - Its T-LANE numbers are a small diagnostic panel, not a new
//!   campaign result.

pub mod alane;
pub mod decompose;
pub mod cells;
pub mod effects;
pub mod tlane;
pub mod validate_control;
