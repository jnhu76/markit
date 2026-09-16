//! Payload metadata (logical identity only — R1 does not build corpora).
//!
//! R0 §9 freezes the shape set and first-round sizes for R3+. R1 only
//! carries the identifiers through case identity and result rows.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Logical identifier of a payload (e.g. `"R1_SMOKE_ONLY/fixture-1"`).
/// Payload identity enters CaseId via this string, never via content
/// hashing of the payload alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PayloadId(pub String);

impl fmt::Display for PayloadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// R0 §9 synthetic shape set. Serialized with the canonical spelling used
/// in CaseKey encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayloadShape {
    Plain,
    ManyBlocks,
    HugeBlock,
    DeepContainer,
    InlineDense,
    FenceHeavy,
    ReferenceFanout,
    Mixed,
}

impl PayloadShape {
    /// Canonical spelling used in the versioned CaseKey byte encoding.
    pub fn canonical_name(self) -> &'static str {
        match self {
            PayloadShape::Plain => "plain",
            PayloadShape::ManyBlocks => "many_blocks",
            PayloadShape::HugeBlock => "huge_block",
            PayloadShape::DeepContainer => "deep_container",
            PayloadShape::InlineDense => "inline_dense",
            PayloadShape::FenceHeavy => "fence_heavy",
            PayloadShape::ReferenceFanout => "reference_fanout",
            PayloadShape::Mixed => "mixed",
        }
    }
}

/// Payload size in bytes (R0 first-round sizes are 64 KiB / 1 MiB /
/// 16 MiB; R1 smoke fixtures are far smaller and carry their real size).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct PayloadSize {
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_names_are_stable() {
        assert_eq!(PayloadShape::ManyBlocks.canonical_name(), "many_blocks");
        assert_eq!(
            serde_json::to_value(PayloadShape::ReferenceFanout).unwrap(),
            serde_json::json!("reference_fanout")
        );
    }
}
