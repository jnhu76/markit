//! Markit document authority: the concrete product `Document` that owns
//! Markdown source, source revisions, edit commits, undo/redo history,
//! pinned snapshots, and the saved baseline / dirty relation
//! (`docs/product/architecture.md` §2.1). Storage representation is
//! private and must not escape this crate.

mod document;
mod identity;
mod plugin;

pub use document::{Document, DocumentError, SourceEdit, SourceOffset, SourceRange, SourceSnapshot};
pub use identity::{DocumentId, SourceRevision};
pub use plugin::{DocumentStore, DocumentStoreCapability, document_plugin};
