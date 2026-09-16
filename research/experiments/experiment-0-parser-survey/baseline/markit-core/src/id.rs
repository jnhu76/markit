//! Stable document identity.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Durable identity of a [`Document`](crate::Document).
///
/// Not a pointer, not a `Vec` index, not a GPUI entity id: the value is a
/// process-wide monotonic number that stays meaningful across future
/// snapshot/command boundaries
/// (`docs/product/plugin-compatibility-contract.md` §6).
///
/// Allocation is cheap and lock-free. `from_u64` exists only to round-trip
/// an id across a serialization boundary; it does not allocate and must
/// not be used to mint fresh identities.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(u64);

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

impl DocumentId {
    /// Allocates a fresh, previously unused id.
    pub fn new() -> Self {
        Self(NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Reconstructs an id from its numeric form (deserialization seam).
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Numeric form of the id, for serialization boundaries only.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DocumentId({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_increasing() {
        let a = DocumentId::new();
        let b = DocumentId::new();
        assert_ne!(a, b);
        assert!(b.as_u64() > a.as_u64());
    }

    #[test]
    fn ids_round_trip_through_u64() {
        let id = DocumentId::new();
        assert_eq!(DocumentId::from_u64(id.as_u64()), id);
    }

    #[test]
    fn debug_does_not_leak_addresses() {
        let id = DocumentId::new();
        let rendered = format!("{id:?}");
        assert!(rendered.starts_with("DocumentId("));
        assert!(!rendered.contains("0x"));
    }
}
