//! Focused I5 commit-frontier and accounting suites (HORSE-A-IMPL-1 #62,
//! slice I5). Like the I3/I4 precedent, the decision-bearing I5 surfaces
//! (`UpdateStaging`/`PreparedCommit`, the structural sinks) are
//! `pub(crate)`, so the identity suites are in-crate `#[cfg(test)]`
//! modules; the public-API correctness battery stays in `tests/`.
//!
//! Every expected value is hand-derived from the frozen authorities
//! (`docs/research/horse-a-v1-algorithm.md`, #59 §9–§11/§20, #60 §9) and
//! the literal byte layout of the fixtures — never read back from the
//! implementation.

mod adversarial;
mod alloc_probe;
mod conformance;
mod counters;
mod frontier;
mod frozen60;
mod primitives;
mod retirement;
