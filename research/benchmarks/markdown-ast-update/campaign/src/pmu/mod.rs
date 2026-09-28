//! `mdbench-pmu` — the minimum mechanism-neutral PMU diagnostic driver
//! for MARKIT-76-SIX-HORSE-PMU-EXPLANATION-v1 (#76 PMU campaign order).
//!
//! EXPLANATORY_DIAGNOSTIC_ONLY: this is not a new horse, not a
//! replacement campaign runner, not a primary timing runner, and not a
//! mechanism implementation. The six mechanism crates are consumed
//! unchanged from the primary mechanism authority
//! (`df1955c9faf06a36bb3fc7d6453d720dfcdc8f0f`).
//!
//! Module map:
//!
//! - [`events`]: the frozen event table (small groups; kernel-generic
//!   encodings; `exclude_kernel` policy);
//! - [`perfcount`]: audited, group-scoped region counters with
//!   `time_enabled`/`time_running` capture;
//! - [`panel`]: frozen-panel authority verification (full-hash match,
//!   no reselection);
//! - [`schedule`]: deterministic PMU schedule from the frozen seed,
//!   written and hashed before collection;
//! - [`region`]: the T_total-mirror counter window;
//! - [`schema`]: the append-only observation row.

pub mod events;
pub mod panel;
pub mod perfcount;
pub mod region;
pub mod schedule;
pub mod schema;

/// Raw SHA256 digest bytes (lib's hex helper returns a String; the seed
/// derivation needs the first 8 bytes).
pub(crate) fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    digest.into()
}
