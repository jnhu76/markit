//! The transplanted BENCH-GRAMMAR-v1 substrate (donor
//! `markit-mdbench-shared-grammar`): the block state machine (container
//! stack, fence, paragraph), block [`parser::Skel`] skeletons with their
//! entry [`parser::ContextKey`], whole-document and byte-range region
//! parsing, and a provider-supplied splice hook (the scanner never
//! decides reuse — it only executes a take the mechanism requested);
//! plus the inline scanner (code spans, links, reference resolution,
//! emphasis), label normalization, the flat first-wins reference table,
//! and materialization of skeletons into the normalized vocabulary.
//!
//! This module does NOT own damage detection, checkpoint selection,
//! convergence, reuse policy, retained mechanism state, or comparison —
//! the restart mechanism owns those. Nothing here compares results.

pub mod inline;
pub mod parser;
