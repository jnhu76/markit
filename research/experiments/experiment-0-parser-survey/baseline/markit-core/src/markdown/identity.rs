//! Internal block identity for the Markdown block index.
//!
//! [`InternalBlockId`] is **internal product identity**: the future view
//! model uses it to keep stable references to blocks across edits. It is
//! deliberately **not** extension/plugin identity
//! (`docs/product/markdown-l1-semantic-contract.md` §10,
//! `docs/product/plugin-compatibility-contract.md` §4): plugins will
//! consume versioned snapshots through an adapter, never these ids, so
//! this type carries no compatibility promises.
//!
//! Ids are minted from a monotonic counter owned by
//! [`MarkdownState`](crate::markdown::MarkdownState); minted ids are never
//! reused, and ids of removed blocks retire permanently.

/// Identity of one block inside one document's block index.
///
/// Unique within the index's lifetime; ordering reflects creation order
/// and carries no positional meaning (blocks keep their ids while text
/// around them changes).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct InternalBlockId(u64);

impl InternalBlockId {
    /// Mints the next id from `counter`, advancing it.
    pub(crate) fn mint(counter: &mut u64) -> Self {
        let id = *counter;
        *counter += 1;
        Self(id)
    }

    /// Numeric form (diagnostics and tests).
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
impl InternalBlockId {
    /// Test-only constructor for id probes.
    pub(crate) fn from_u64_for_test(value: u64) -> Self {
        Self(value)
    }
}
