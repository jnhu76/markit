//! Shared immutable source representation.
//!
//! R0 §6: source authority is UTF-8 bytes. The substrate uses one shared
//! immutable representation ([`Arc<str>`]) so every mechanism receives the
//! same bytes without private copies. Digest computation is O(n) and is a
//! runner-side concern; it never runs inside a mechanism timer.

use std::fmt;
use std::sync::Arc;

use sha2::{Digest, Sha256};

/// Logical identity of one materialized source inside a run.
///
/// Assigned by the runner. Not part of [`crate::case::CaseId`]; case
/// identity is derived from content digests, not runtime object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(pub u64);

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source-{}", self.0)
    }
}

/// Immutable shared UTF-8 source text.
///
/// Cheap to clone; contents never change after construction, which is what
/// lets the runner hand the same `&Source` to every mechanism lane.
#[derive(Debug, Clone)]
pub struct Source {
    id: SourceId,
    text: Arc<str>,
}

impl Source {
    pub fn new(id: SourceId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: Arc::from(text.into()),
        }
    }

    pub fn from_arc(id: SourceId, text: Arc<str>) -> Self {
        Self { id, text }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    pub fn sha256(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.text.as_bytes());
        hasher.finalize().into()
    }

    pub fn sha256_hex(&self) -> String {
        to_lower_hex(&self.sha256())
    }
}

/// Lowercase hex encoding without an extra dependency.
pub fn to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_utf8_and_digested_from_bytes() {
        let s = Source::new(SourceId(1), String::from("中文 🐉 a+b"));
        assert_eq!(s.as_str(), "中文 🐉 a+b");
        // len is byte length, not char count (CJK 3 bytes each, emoji 4).
        assert_eq!(s.len_bytes(), 3 + 3 + 1 + 4 + 1 + 3);
        let hex = s.sha256_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn digest_is_stable() {
        let a = Source::new(SourceId(7), "stable content\n");
        let b = Source::new(SourceId(9), "stable content\n");
        assert_eq!(a.sha256(), b.sha256());
        let c = Source::new(SourceId(7), "stable content ");
        assert_ne!(a.sha256(), c.sha256());
    }
}
