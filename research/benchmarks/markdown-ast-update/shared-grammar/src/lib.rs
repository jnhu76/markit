//! markit-mdbench-shared-grammar — the shared non-research
//! BENCH-GRAMMAR-v1 primitives (R5 parity substrate).
//!
//! Authority: `grammar/BENCH-GRAMMAR-v1.md` (frozen R3 semantics) +
//! `protocol/R0-METHODOLOGY.md` §4.2 (shared substrate) + the R5
//! mechanism-decision freeze (`protocol/R5-HORSE-CORRECTNESS-PARITY.md`
//! §1). The code is the R4 H0 parser extracted line-for-line into a
//! resumable scanner, plus exactly the shared grammar state later horses
//! need:
//!
//! - [`parser`] — the block state machine (container stack, fence,
//!   paragraph), block [`parser::Skel`] skeletons with their entry
//!   [`parser::ContextKey`], whole-document and byte-range region
//!   parsing, and a horse-supplied splice hook (the scanner never
//!   decides reuse — it only executes a take the horse requested);
//! - [`inline`] — the inline scanner (code spans, links, reference
//!   resolution, emphasis), label normalization, the flat first-wins
//!   reference table, and materialization of skeletons into the frozen
//!   NORMALIZED-RESULT-v1 vocabulary.
//!
//! This crate does NOT own damage detection, fragment lookup, old-tree
//! lookup, checkpoint selection, convergence, reuse policy, incremental
//! indexes, or retained mechanism state. Horses own all of that. The
//! oracle comparison lives in the oracle crate; nothing here compares
//! results.

pub mod inline;
pub mod parser;

pub use inline::{materialize, norm_label, RefTable};
pub use parser::{
    classify_top_level_line, fence_opener_at, heading_at, parse_full, parse_marker, parse_region,
    parse_region_with_hook, ContextKey, FrameKey, LineClass, RegionParse, Skel, SpliceHook,
};
