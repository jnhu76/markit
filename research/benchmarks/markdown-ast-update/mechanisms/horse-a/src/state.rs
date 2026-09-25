//! The frozen Horse-A retained state model (spec §2, §2.1–§2.4;
//! logical-data-model §4–§5): one [`ReadyDocument`] per externally
//! current source version, one record per root-level top-level block
//! Owner in a byte-weighted sequence, one document-global
//! [`RefTable`](markit_mdbench_shared_grammar::RefTable).
//!
//! Ownership boundary (spec §5 of the I2 task contract, data-model
//! §5.2): completed payload owns its semantic values and borrows neither
//! the source bytes nor the RefTable. All retained spans and content
//! intervals are Owner-relative; document-absolute positions are derived
//! only at access/export time from byte-weight prefix sums.

use markit_mdbench_common::SourceId;
use markit_mdbench_oracle::normalized::Node;
use markit_mdbench_shared_grammar::RefTable;

use crate::certificate::RestartCertificate;

/// Interpretation identity of a [`ReadyDocument`] (data-model §4): which
/// frozen grammar, which normalized vocabulary, which reference
/// implementation, and which Horse-A data-model/certificate policy
/// revision produced the retained state. Options that would affect
/// interpretation are recorded even when empty.
///
/// The substrate defines no canonical interpretation type, so this is
/// the minimal local identity; it deliberately reuses the substrate's
/// `SourceId` for source association instead of inventing a parallel id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterpretationId {
    pub grammar: &'static str,
    pub result_vocabulary: &'static str,
    pub reference_implementation: &'static str,
    pub options: &'static str,
    pub data_model_policy: &'static str,
}

impl InterpretationId {
    /// The only interpretation a Horse-A v1 full build produces.
    pub const HORSE_A_V1: Self = Self {
        grammar: "BENCH-GRAMMAR-v1",
        result_vocabulary: "NORMALIZED-RESULT-v1",
        reference_implementation: "markit-mdbench-shared-grammar",
        options: "none",
        data_model_policy: "horse-a-v1",
    };
}

/// The frozen ready retained state (spec §2): complete, self-contained,
/// next-edit-capable. READY means actually ready — construction must
/// leave no parsing, semantic repair, deferred index construction, or
/// required state-building step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyDocument {
    /// Substrate identity of the immutable source this state describes.
    pub source_id: SourceId,
    /// Length of the described source in bytes.
    pub source_len: usize,
    /// Which interpretation produced the retained state.
    pub interpretation: InterpretationId,
    /// The retained Owner sequence (one record per root-level top-level
    /// block subtree; construction-only balanced build in I2).
    pub owners: OwnerSeq,
    /// THE document-global RefTable: the source-order projection of all
    /// retained ReferenceDefinition facts (spec §2.4). There is no second
    /// definition truth.
    pub refs: RefTable,
}

/// Byte-weighted ordered Owner sequence (spec §2). I2 materializes it as
/// the frozen one-record-per-node shape with a construction-only O(M)
/// balanced build; the I3 slice owns the mutation/navigation operators.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnerSeq {
    pub root: Option<Box<AvlNode>>,
}

/// One mutable weighted AVL node — exactly one Owner record (spec §12;
/// W-A3 is preserved, not optimized away).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvlNode {
    pub left: Option<Box<AvlNode>>,
    pub right: Option<Box<AvlNode>>,
    /// `h(empty) = 0`, `h(leaf) = 1` (spec §12).
    pub height: u32,
    pub agg: Aggregate,
    pub owner: Owner,
}

/// The persistent per-node sequence aggregates (spec §2, §15.4): height
/// lives beside them on the node; `subtree_has_safe` summarizes whether
/// the subtree contains at least one persistent eligible outgoing
/// RestartCertificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aggregate {
    pub subtree_bytes: usize,
    pub subtree_records: usize,
    pub subtree_has_safe: bool,
}

/// One retained record: one complete root-level top-level block subtree
/// plus its physical-first-line source coverage and the optional
/// outgoing certificate at its right coverage cut (data-model §5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// Length of the covered source range `[base, base + coverage_len)`.
    /// Always > 0; the coverage union over all Owners partitions
    /// `[0, source_len)` with no gap and no overlap.
    pub coverage_len: usize,
    pub payload: OwnerPayload,
    /// Optional outgoing [`RestartCertificate`] at this Owner's right
    /// coverage cut (an interior boundary), built only from real I1
    /// `RootBlankEvent` evidence.
    pub outgoing_restart: Option<RestartCertificate>,
}

impl Owner {
    pub fn is_trivia_only(&self) -> bool {
        matches!(self.payload, OwnerPayload::TriviaOnly)
    }
}

/// The payload of one Owner (spec §2): either the whole document is
/// trivia, or the complete eager semantic subtree of one top-level block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerPayload {
    /// The frozen root-blank special case: the single Owner of a
    /// non-empty all-blank document. No ASTPayload, no definition facts.
    TriviaOnly,
    /// The complete eager semantic subtree for exactly one root-level
    /// top-level Markdown Owner, in the shared NORMALIZED-RESULT-v1
    /// vocabulary, with Owner-relative spans/content intervals and owned
    /// semantic values (spec §2.2).
    Syntax(AstPayload),
}

/// The eager semantic payload of one Syntax Owner. Not an opaque blob:
/// it is the shared normalized `Node` subtree (block topology, nested
/// block nodes, ordered inline nodes, heading levels, list markers, fence
/// info, definition label/destination, resolved reference destinations),
/// with every span and content interval relative to the Owner's coverage
/// base (spec §2.2; data-model §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstPayload {
    pub root: Node,
}
