//! Monotonic document revision identity and the stale-result rejection seam.

/// Revision of a [`Document`](crate::Document).
///
/// Advances by exactly one per successful mutation (single edit,
/// transaction, or whole-document replacement). Revisions are never reused
/// and never expressed as bare `usize` in APIs: the type **is** the
/// compatibility rule that keeps deferred/background work safe
/// (`docs/product/performance-invariants.md` INV-10).
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

/// A derived result tagged with the document revision it was computed from.
///
/// This is the seam that makes stale work rejectable by construction:
/// deferred or background jobs capture the revision of the state they
/// read, and their results are only committed against that exact revision.
///
/// The P0-01 compatibility rule is exact-revision equality. Finer-grained
/// dependency-based compatibility (unchanged-region reuse) arrives when
/// real derived state exists; it will extend this seam, not bypass it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Revisioned<T> {
    base_revision: DocumentRevision,
    value: T,
}

impl<T> Revisioned<T> {
    /// Tags `value` as derived from `base_revision`.
    pub fn new(base_revision: DocumentRevision, value: T) -> Self {
        Self {
            base_revision,
            value,
        }
    }

    /// The revision this result was computed from.
    pub fn base_revision(&self) -> DocumentRevision {
        self.base_revision
    }

    /// Whether this result is still valid for `current`.
    pub fn is_current(&self, current: DocumentRevision) -> bool {
        self.base_revision == current
    }

    /// Commits this result against the document's `current` revision.
    ///
    /// Returns the value only if it was derived from exactly `current`;
    /// otherwise the result is handed back as [`StaleResult`] so the
    /// caller can drop it, log it, or rebase it explicitly. A stale
    /// result never silently overwrites newer state.
    pub fn commit(self, current: DocumentRevision) -> Result<T, StaleResult<T>> {
        if self.base_revision == current {
            Ok(self.value)
        } else {
            Err(StaleResult {
                base_revision: self.base_revision,
                current_revision: current,
                value: self.value,
            })
        }
    }
}

/// Rejection of a derived result computed from an older revision
/// (INV-10). The rejected value is returned so callers can inspect or
/// explicitly rebase it — nothing commits it implicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaleResult<T> {
    /// Revision the result was computed from.
    pub base_revision: DocumentRevision,
    /// Revision the document has reached.
    pub current_revision: DocumentRevision,
    /// The rejected value.
    pub value: T,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn commit_accepts_exact_revision_only() {
        let base = DocumentRevision::INITIAL;
        let tagged = Revisioned::new(base, 42);
        assert!(tagged.is_current(base));
        assert!(!tagged.is_current(base.next()));
        assert_eq!(tagged.commit(base), Ok(42));

        let newer = base.next().next();
        let stale = tagged.commit(newer).unwrap_err();
        assert_eq!(stale.base_revision, base);
        assert_eq!(stale.current_revision, newer);
        assert_eq!(stale.value, 42);
    }

    #[test]
    fn newer_revision_results_commit_against_newer_state() {
        let r1 = DocumentRevision::INITIAL.next();
        let r2 = r1.next();
        // A result derived after the second edit is valid for r2, not r1.
        assert_eq!(Revisioned::new(r2, "x").commit(r2), Ok("x"));
        assert!(Revisioned::new(r2, "x").commit(r1).is_err());
    }
}
