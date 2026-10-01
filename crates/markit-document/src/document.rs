//! The concrete document-content authority (architecture §2.1): source +
//! source revision, edit commit with base-revision validation, undo/redo
//! history, pinned snapshots, and the saved baseline / dirty relation.

use std::fmt;
use std::rc::Rc;

use crate::identity::{DocumentId, SourceRevision};

/// A UTF-8 byte offset into one revision of a document's source. This is
/// the first product coordinate currency: adapters convert from UI-native
/// coordinates (UTF-16 code units, graphemes, …) at the boundary.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SourceOffset(usize);

impl SourceOffset {
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }

    pub fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for SourceOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A half-open byte range `[start, end)` of one source revision.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SourceRange {
    pub start: SourceOffset,
    pub end: SourceOffset,
}

impl SourceRange {
    pub const fn new(start: SourceOffset, end: SourceOffset) -> Self {
        Self { start, end }
    }
}

/// One typed source edit: which document, against which base revision,
/// which byte range of that revision, replaced by what. A stale base
/// revision is rejected explicitly; there is no silent rebase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEdit {
    document: DocumentId,
    base: SourceRevision,
    range: SourceRange,
    replacement: String,
}

impl SourceEdit {
    /// A replacement of `range` in `document` at source revision `base`.
    pub fn replace(
        document: DocumentId,
        base: SourceRevision,
        range: SourceRange,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            document,
            base,
            range,
            replacement: replacement.into(),
        }
    }

    pub fn document(&self) -> DocumentId {
        self.document
    }

    pub fn base(&self) -> SourceRevision {
        self.base
    }

    pub fn range(&self) -> SourceRange {
        self.range
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }
}

/// Why [`Document::commit`] refused an edit. A refusal never mutates the
/// document: source and revision stay exactly as they were.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DocumentError {
    /// The edit addresses a different document than the one it was
    /// submitted to.
    DocumentMismatch { expected: DocumentId, actual: DocumentId },
    /// The edit's base revision is not the document's current revision.
    StaleBase { current: SourceRevision, edit_base: SourceRevision },
    /// The edit's byte range is not fully inside the base revision's
    /// source (`start > end` or `end > source length`).
    RangeInvalid { range: SourceRange, source_len: usize },
    /// A range boundary falls in the middle of a UTF-8 scalar value.
    NotCharBoundary { offset: SourceOffset },
    /// The saved baseline names a revision this document never issued.
    UnknownRevision { revision: SourceRevision },
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocumentMismatch { expected, actual } => write!(
                f,
                "edit addresses {actual} but was submitted to {expected}"
            ),
            Self::StaleBase { current, edit_base } => write!(
                f,
                "edit base {edit_base} is stale; current revision is {current}"
            ),
            Self::RangeInvalid { range, source_len } => write!(
                f,
                "edit range [{}, {}) is not inside the {}-byte source",
                range.start,
                range.end,
                source_len
            ),
            Self::NotCharBoundary { offset } => {
                write!(f, "edit boundary {} splits a UTF-8 scalar value", offset)
            }
            Self::UnknownRevision { revision } => {
                write!(f, "revision {revision} was never issued by this document")
            }
        }
    }
}

impl std::error::Error for DocumentError {}

/// An immutable, pinned capture of one document revision: the retained
/// bytes plus the identity they belong to. A snapshot for revision N keeps
/// reading N after the document advances to N+1; releasing it is dropping
/// the handle. Backed by a shared buffer the document itself never mutates
/// in place (each commit builds a new one), so pinning costs a refcount.
#[derive(Clone, Debug)]
pub struct SourceSnapshot {
    document: DocumentId,
    revision: SourceRevision,
    source: Rc<str>,
}

impl SourceSnapshot {
    pub fn document_id(&self) -> DocumentId {
        self.document
    }

    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// One open document: the document-content authority over its Markdown
/// source. Storage is a private implementation detail and never escapes.
pub struct Document {
    id: DocumentId,
    source: Rc<str>,
    revision: SourceRevision,
    saved: Option<SourceRevision>,
    history: Vec<HistoryRecord>,
    cursor: usize,
}

/// One committed byte replacement, kept in both directions so linear
/// undo/redo can replay it as a new commit each time.
struct HistoryRecord {
    /// Byte position of the replacement, valid in the pre- and post-state
    /// coordinates of exactly this record's two neighboring revisions.
    pos: usize,
    prev_text: String,
    new_text: String,
}

impl Document {
    /// A new document holding `initial` at [`SourceRevision::INITIAL`].
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            id: DocumentId::allocate(),
            source: Rc::from(initial.into().as_str()),
            revision: SourceRevision::INITIAL,
            saved: None,
            history: Vec::new(),
            cursor: 0,
        }
    }

    pub fn id(&self) -> DocumentId {
        self.id
    }

    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// The current source bytes.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Pin the current revision. The returned snapshot keeps reading this
    /// revision even after further commits, undo, redo, or the document
    /// being closed.
    pub fn snapshot(&self) -> SourceSnapshot {
        SourceSnapshot {
            document: self.id,
            revision: self.revision,
            source: Rc::clone(&self.source),
        }
    }

    /// Commit one typed edit. A valid commit atomically replaces the edit's
    /// byte range and produces the next revision; undo/redo and ordinary
    /// edits are the same commit authority.
    pub fn commit(&mut self, edit: SourceEdit) -> Result<SourceRevision, DocumentError> {
        if edit.document != self.id {
            return Err(DocumentError::DocumentMismatch {
                expected: self.id,
                actual: edit.document,
            });
        }
        if edit.base != self.revision {
            return Err(DocumentError::StaleBase {
                current: self.revision,
                edit_base: edit.base,
            });
        }
        let len = self.source.len();
        if edit.range.start > edit.range.end || edit.range.end.get() > len {
            return Err(DocumentError::RangeInvalid {
                range: edit.range,
                source_len: len,
            });
        }
        let (start, end) = (edit.range.start.get(), edit.range.end.get());
        if !self.source.is_char_boundary(start) || !self.source.is_char_boundary(end) {
            let split = if self.source.is_char_boundary(start) { end } else { start };
            return Err(DocumentError::NotCharBoundary {
                offset: SourceOffset::new(split),
            });
        }

        let prev_text = self.source[start..end].to_owned();
        let mut next = String::with_capacity(len - (end - start) + edit.replacement.len());
        next.push_str(&self.source[..start]);
        next.push_str(&edit.replacement);
        next.push_str(&self.source[end..]);

        self.history.truncate(self.cursor);
        self.history.push(HistoryRecord {
            pos: start,
            prev_text,
            new_text: edit.replacement,
        });
        self.cursor += 1;
        self.source = Rc::from(next.as_str());
        self.revision = self.revision.next();
        Ok(self.revision)
    }

    /// Undo the most recent entry of the linear history as a new commit:
    /// revision advances, content returns to its previous state. `None`
    /// at the beginning of history.
    pub fn undo(&mut self) -> Option<SourceRevision> {
        if self.cursor == 0 {
            return None;
        }
        let record = &self.history[self.cursor - 1];
        self.source = Rc::from(&*self.replaced_at(record, &record.new_text, &record.prev_text));
        self.cursor -= 1;
        self.revision = self.revision.next();
        Some(self.revision)
    }

    /// Redo the most recently undone entry as a new commit. `None` at the
    /// tip of history.
    pub fn redo(&mut self) -> Option<SourceRevision> {
        if self.cursor == self.history.len() {
            return None;
        }
        let record = &self.history[self.cursor];
        self.source = Rc::from(&*self.replaced_at(record, &record.prev_text, &record.new_text));
        self.cursor += 1;
        self.revision = self.revision.next();
        Some(self.revision)
    }

    /// The source produced by replacing the record's current on-disk text
    /// (`from`, whose byte length is valid at `record.pos` in the present
    /// state) with `to`. The linear-history invariant guarantees
    /// `record.pos + from.len()` is a char boundary here.
    fn replaced_at(&self, record: &HistoryRecord, from: &str, to: &str) -> String {
        let mut next = String::with_capacity(self.source.len() - from.len() + to.len());
        next.push_str(&self.source[..record.pos]);
        next.push_str(to);
        next.push_str(&self.source[record.pos + from.len()..]);
        next
    }

    /// Record that revision `revision`'s bytes are the ones persisted by
    /// the filesystem owner. Saving N while the document has advanced to
    /// N+1 leaves N+1 dirty: dirty means `current != saved`, never
    /// "a save happened recently".
    pub fn mark_saved(&mut self, revision: SourceRevision) -> Result<(), DocumentError> {
        if revision > self.revision {
            return Err(DocumentError::UnknownRevision { revision });
        }
        self.saved = Some(revision);
        Ok(())
    }

    /// The most recent revision recorded saved, if any.
    pub fn saved_revision(&self) -> Option<SourceRevision> {
        self.saved
    }

    /// Whether the current revision's bytes differ from the saved
    /// baseline. Content that was never marked saved is dirty.
    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.revision)
    }
}
