//! # markit-core
//!
//! Framework-independent editor core for Markit.
//!
//! This crate owns editor policy and state — document, edit model,
//! selection, revision/change semantics — and deliberately depends on
//! nothing else: no GPUI, no platform crates, no plugin runtime
//! (ADR-002, ADR-008, `docs/product/architecture.md` §2).
//!
//! ## Execution laws this core encodes
//!
//! The realtime execution model (`docs/product/realtime-execution-model.md`)
//! requires:
//!
//! ```text
//! explicit changed-range propagation
//! smallest semantically valid invalidation
//! revision-safe cancellation / stale-result rejection
//! coherent publication
//! ```
//!
//! Concretely, in this crate:
//!
//! - every successful mutation produces an [`EditResult`] whose changed
//!   ranges were computed **at mutation time** — downstream layers consume
//!   that range and never re-derive it by rescanning the document;
//! - every mutation advances a monotonic [`DocumentRevision`] by exactly
//!   one, and derived results carry their base revision so stale results
//!   are rejectable by construction ([`Revisioned`]);
//! - the [`LineIndex`](crate::line_index::LineIndex) is updated
//!   incrementally per edit (ADR-003); a normal local edit scans only the
//!   bytes it inserts, never the document.
//!
//! ## Coordinate semantics
//!
//! The canonical source coordinate is the byte-based [`ByteOffset`] /
//! [`SourceRange`]. Unicode scalars, grapheme boundaries, display
//! positions, and platform UTF-16 coordinates are **not** core
//! coordinates: UTF-16 in particular belongs to the platform edge
//! (GPUI input handling), and grapheme/display layers get their own
//! explicit vocabulary when they exist (AGENTS.md §8).
//!
//! ## Storage privacy
//!
//! [`Document`] keeps its text representation private. Reading goes
//! through range/line queries returning borrowed views. Nothing in the
//! public API exposes `String`, `Vec`, indexes, or pointers as durable
//! contract, so the buffer can later change (rope, piece table) without
//! breaking consumers.
//!
//! ## Extension-boundary status
//!
//! Nothing in this crate is the plugin ABI. These types are internal core
//! semantics. Future plugins consume versioned snapshots/queries and
//! submit commands/results through an adapter described by
//! `docs/product/plugin-compatibility-contract.md`; the shapes here
//! (stable [`DocumentId`], [`DocumentRevision`], coherent
//! [`DocumentSnapshot`], [`EditTransaction`] commands) are the seams that
//! adapter will build on.

#![forbid(unsafe_code)]

pub mod change;
pub mod document;
pub mod id;
pub mod line_index;
pub mod position;
pub mod revision;

pub use change::{AppliedEdit, ChangeKind, EditError, EditResult, EditWork, TextEdit};
pub use document::Document;
pub use id::DocumentId;
pub use line_index::{LineIndex, LineIndexCounters};
pub use position::{ByteOffset, LineNumber, SourceRange};
pub use revision::{DocumentRevision, Revisioned, StaleResult};
