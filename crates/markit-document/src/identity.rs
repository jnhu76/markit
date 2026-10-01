//! Document/revision identity (architecture §3): a source revision
//! identifies one committed source state and never ABA-reuses identities;
//! every revision-bound identity carries the document it belongs to,
//! because two documents may legitimately hold the same revision number.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identity of one open document/buffer instance. Unique per process;
/// allocation is a monotonic counter, so identities are never reused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct DocumentId(u64);

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

impl DocumentId {
    pub(crate) fn allocate() -> Self {
        Self(NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "doc#{}", self.0)
    }
}

/// One committed source state of one document. Revisions start at
/// [`SourceRevision::INITIAL`] (the opened/created content) and every
/// successful commit — ordinary edit, undo, or redo — produces the next
/// value. The counter never moves backward and never reuses a value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SourceRevision(u64);

impl SourceRevision {
    /// The revision of the content a document was created with; no commit
    /// has happened yet.
    pub const INITIAL: Self = Self(0);

    pub fn get(self) -> u64 {
        self.0
    }

    /// The next revision identity. Monotonic, so a value is never issued
    /// twice (no ABA reuse).
    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}
