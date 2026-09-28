//! The frozen #60 structural-collection producer (slice I6; the #60
//! authorization record's P2-1, executed under
//! `docs/research/horse-a-structural-collection-readiness-1.md`).
//!
//! This module is producer/instrumentation surface only: it adds NO
//! mechanism, NO algorithm change, NO counter-charge-semantics change
//! and NO performance measurement. The Horse-A algorithm files are
//! untouched; the recording lane and the no-op lane remain the identical
//! algorithm differing only in sink identity (#60 §10).
//!
//! What lives here:
//!
//! - [`contract`] — the frozen StudyId, contract revision, baselines,
//!   mechanism identity, the three primary cells (complete identity +
//!   exact geometry + the synchronized #60 §9.5 threshold table) and the
//!   in-process byte-exact cell construction/verification;
//! - [`schema`] — HORSE-A-STRUCTURAL-RAW-v1 (readiness record §9 + the
//!   pre-treatment P2-2/P2-4 extensions) and the compile-time-exhaustive
//!   counter-field enumeration;
//! - [`provenance`] — fail-closed static provenance capture (readiness
//!   record §8);
//! - [`adjudicate`] — the complete #60 §14 Known sweep as a pure
//!   function over (frozen cell, raw row), plus the mechanical
//!   determinism / cell / overall verdict derivation (#60 §15);
//! - [`writer`] — the append-only immutable raw-row writer (duplicate
//!   identity refused, never truncate/replace);
//! - [`run`] — the one-invocation treatment lifecycle (task §7) with
//!   the recording window exactly the frozen primary native update
//!   boundary, and the non-treatment `--validate-only` /
//!   `--print-provenance` surfaces.
//!
//! STRUCTURAL_COLLECTION_AUTHORIZED = YES (recorded on #60, 2026-09-28);
//! this module exists to execute that authorization under the frozen
//! protocol, and it may not be retuned after the first decision-bearing
//! raw row.

pub mod adjudicate;
pub mod contract;
pub mod provenance;
pub mod run;
pub mod schema;
pub mod writer;

#[cfg(test)]
mod tests;
