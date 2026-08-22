//! Monotonic document revision identity and the stale-result rejection seam.
//!
//! A revision number is meaningful **only inside one document**: two
//! documents can both be at revision 7 and share nothing. Derived work
//! therefore never carries a bare revision across a boundary — it carries a
//! [`DocumentVersion`], the (identity, revision) pair that names exactly one
//! coherent state (`docs/product/plugin-compatibility-contract.md` §6).

use crate::id::DocumentId;

/// Revision of a [`Document`](crate::Document).
///
/// Advances by exactly one per successful mutation (single edit,
/// transaction, or whole-document replacement). Revisions are never reused
/// and never expressed as bare `usize` in APIs: the type **is** the
/// compatibility rule that keeps deferred/background work safe
/// (`docs/product/performance-invariants.md` INV-10).
///
/// A revision alone is **not** a version: it is only the second half of
/// [`DocumentVersion`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentRevision(u64);

impl DocumentRevision {
    /// Revision of a freshly constructed document.
    pub const INITIAL: Self = Self(0);

    /// Numeric form (serialization seam).
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// The revision produced by the next successful mutation. Only the
    /// document mutates; this is not part of the public mutation contract.
    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// A coherent version of one document: its stable identity plus the
/// revision. `(DocumentId, DocumentRevision)` names exactly one document
/// state, so this is the unit of validity for every derived result.
///
/// Deliberately **not** ordered: comparing versions of different documents
/// is meaningless, and even equal numeric revisions from different
/// documents are unrelated states. Only [`DocumentRevision`] ordering —
/// within one document — is meaningful, via
/// [`DocumentVersion::revision`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DocumentVersion {
    document_id: DocumentId,
    revision: DocumentRevision,
}

impl DocumentVersion {
    /// The version of `document_id` at `revision`.
    pub fn new(document_id: DocumentId, revision: DocumentRevision) -> Self {
        Self {
            document_id,
            revision,
        }
    }

    /// The document this version belongs to.
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    /// The revision within that document.
    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }
}

/// A derived result tagged with the document version it was computed from.
///
/// This is the seam that makes stale work rejectable by construction:
/// deferred or background jobs capture the version of the state they read,
/// and their results are only committed against that exact version — same
/// document, same revision. A matching revision number from a *different*
/// document is rejected just like an outdated revision from the same one.
///
/// The P0-01 compatibility rule is exact-version equality. Finer-grained
/// dependency-based compatibility (unchanged-region reuse) arrives when
/// real derived state exists; it will extend this seam, not bypass it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Revisioned<T> {
    base: DocumentVersion,
    value: T,
}

impl<T> Revisioned<T> {
    /// Tags `value` as derived from `base`.
    pub fn new(base: DocumentVersion, value: T) -> Self {
        Self { base, value }
    }

    /// The document version this result was computed from.
    pub fn base_version(&self) -> DocumentVersion {
        self.base
    }

    /// Whether this result is still valid for `current`.
    pub fn is_current(&self, current: DocumentVersion) -> bool {
        self.base == current
    }

    /// Commits this result against the document's `current` version.
    ///
    /// Returns the value only if it was derived from exactly `current`
    /// (same document, same revision); otherwise the result is handed back
    /// as [`StaleResult`] so the caller can drop it, log it, or rebase it
    /// explicitly. A stale result never silently overwrites newer state —
    /// and neither does a fresh-looking revision from the wrong document.
    pub fn commit(self, current: DocumentVersion) -> Result<T, StaleResult<T>> {
        if self.base == current {
            Ok(self.value)
        } else {
            Err(StaleResult {
                base_version: self.base,
                current_version: current,
                value: self.value,
            })
        }
    }
}

/// Rejection of a derived result computed from a different version than
/// the current one — an older revision of the same document, or any
/// revision of a different document (INV-10). The rejected value is
/// returned so callers can inspect or explicitly rebase it — nothing
/// commits it implicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaleResult<T> {
    /// Version the result was computed from.
    pub base_version: DocumentVersion,
    /// Version the document has reached.
    pub current_version: DocumentVersion,
    /// The rejected value.
    pub value: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(n: u64) -> DocumentRevision {
        let mut r = DocumentRevision::INITIAL;
        for _ in 0..n {
            r = r.next();
        }
        r
    }

    fn version(id_num: u64, rev: u64) -> DocumentVersion {
        DocumentVersion::new(DocumentId::from_u64(id_num), revision(rev))
    }

    #[test]
    fn revision_is_monotonic() {
        let mut r = DocumentRevision::INITIAL;
        assert_eq!(r.as_u64(), 0);
        for expected in 1..=5 {
            r = r.next();
            assert_eq!(r.as_u64(), expected);
        }
    }

    #[test]
    fn version_exposes_both_halves() {
        let id = DocumentId::new();
        let v = DocumentVersion::new(id, DocumentRevision::INITIAL.next());
        assert_eq!(v.document_id(), id);
        assert_eq!(v.revision().as_u64(), 1);
    }

    #[test]
    fn commit_accepts_exact_version() {
        // same document + same revision => accept
        let base = version(7, 3);
        let tagged = Revisioned::new(base, 42);
        assert!(tagged.is_current(base));
        assert_eq!(tagged.commit(base), Ok(42));
    }

    #[test]
    fn commit_rejects_newer_revision_of_same_document() {
        let base = version(7, 3);
        let tagged = Revisioned::new(base, 42);
        let newer = version(7, 5);
        assert!(!tagged.is_current(newer));

        let stale = tagged.commit(newer).unwrap_err();
        assert_eq!(stale.base_version, base);
        assert_eq!(stale.current_version, newer);
        assert_eq!(stale.value, 42);
    }

    #[test]
    fn commit_rejects_different_document_with_same_revision() {
        // The regression the revision-only seam allowed through: equal
        // numeric revisions from different documents are unrelated states.
        let base = version(7, 0);
        let other_document = version(8, 0);
        let tagged = Revisioned::new(base, 42);
        assert!(!tagged.is_current(other_document));
        assert!(tagged.commit(other_document).is_err());
    }

    #[test]
    fn newer_revision_results_commit_against_newer_state() {
        // A result derived after the second edit is valid for that
        // revision of the same document, and nothing else.
        let r1 = revision(1);
        let r2 = revision(2);
        let doc = DocumentId::from_u64(1);
        assert_eq!(
            Revisioned::new(DocumentVersion::new(doc, r2), "x")
                .commit(DocumentVersion::new(doc, r2)),
            Ok("x")
        );
        assert!(Revisioned::new(DocumentVersion::new(doc, r2), "x")
            .commit(DocumentVersion::new(doc, r1))
            .is_err());
    }
}
