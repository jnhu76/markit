//! The normalized export path (spec §4 query/export; data-model §6):
//! whole-document export synthesizes one Document root and sequentially
//! projects every payload with a running byte-weight base —
//! `absolute = owner_base + relative` — never one root seek per Owner,
//! never materialization or repair during export.

use markit_mdbench_common::ResultChecksum;
use markit_mdbench_oracle::normalized::{normalized_checksum, Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;

use crate::payload::shift_spans;
use crate::state::{OwnerPayload, ReadyDocument};

impl NormalizeV1 for ReadyDocument {
    fn normalize_v1(&self) -> NormalizedDocument {
        let mut root = Node::new(NodeKind::Document, 0, self.source_len);
        // Sequential Owner traversal with a running base (O(M) over the
        // output; extra traversal stack O(H + payload depth)).
        self.owners.for_each_in_order(|base, owner| {
            if let OwnerPayload::Syntax(payload) = &owner.payload {
                let mut absolute = payload.root.clone();
                shift_spans(&mut absolute, base as isize);
                root.children.push(absolute);
            }
            // A TriviaOnly Owner contributes no normalized trivia node
            // (data-model §3.1: the TriviaOnly record outputs no
            // normalized node).
        });
        NormalizedDocument::new(root)
    }
}

/// The frozen post-timer export checksum shape (identical to the other
/// horses): the deterministic checksum of the state's normalized result.
impl ResultChecksum for ReadyDocument {
    fn result_checksum(&self) -> u64 {
        normalized_checksum(&self.normalize_v1())
    }
}
