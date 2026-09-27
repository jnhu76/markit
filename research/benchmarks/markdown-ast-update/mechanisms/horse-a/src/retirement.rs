//! I5 explicit attributable retirement (#59 §11, §11.1, §20).
//!
//! Retirement is the post-frontier destruction of retired Owner records
//! and their semantic payload. It is **explicit and attributable**: the
//! drop walk charges the frozen §20 retirement ledger through the
//! structural sink at each recursion frame, so every destroyed record and
//! every destroyed payload node is counted exactly where its destruction
//! is driven — there is no implicit `drop` of retired state and no
//! post-hoc attribution walk.
//!
//! Frozen resource model (§11.1, recursion Option B — no retirement
//! workspace; stack depth is never an operation count):
//!
//! - `retire_node_visits` — one charge per traversed retired AVL record
//!   (exactly the records in the detached range B = Δ_old);
//! - `payload_nodes_retired` — one charge per destroyed normalized
//!   payload node (= P_removed);
//! - `retirement_frames_entered` — one per AVL-record frame and one per
//!   payload-node frame (≤ Δ_old + P_removed);
//! - `max_retirement_depth` — a max-merge gauge deepened per frame kind
//!   in its **own recursion space** (≤ max(H_detached, D_payload)). The
//!   AVL space numbers the levels of the detached subtree (its root
//!   frame is depth 1, so the gauge reaches H_detached); the payload
//!   space numbers normalized-node nesting (the root payload node of an
//!   Owner is depth 1, so the gauge reaches D_payload). The two spaces
//!   never add — the gauge is the max over both, per the frozen bound.
//!
//! Retirement performs NO recomputes, NO link writes, and NO aggregate
//! reads: nothing detached is relinked, nothing dropped is rewritten
//! (§20 retirement row). Retained prefix/suffix payload is never
//! traversed — the walk receives exactly the retired tree and nothing
//! else. Destructor order does not allocate and cannot panic in normal
//! operation: every charge is a fixed-size scalar increment and all
//! remaining destruction is plain drops.

use markit_mdbench_oracle::normalized::Node;

use crate::state::{AvlNode, Owner, OwnerPayload, OwnerSeq, ReadyDocument};
use crate::structural::HorseAStructuralSink;

/// Retire the detached middle B on the local route: the Owner records the
/// structural splice removed, handed over intact by `replace_range`.
/// This is the ONLY state the local-route retirement touches — the
/// retained prefix and suffix are not passed here and cannot be reached.
pub(crate) fn retire_detached(retired: OwnerSeq, sink: &mut dyn HorseAStructuralSink) {
    if let Some(root) = retired.root {
        retire_avl_record(root, 1, sink);
    }
}

/// Retire the complete old READY document on the full route (the frozen
/// conservative route explicitly replaces the whole representation). The
/// document-global RefTable retires as a plain drop — retirement charges
/// cover records and payload nodes only.
pub(crate) fn retire_document(old: ReadyDocument, sink: &mut dyn HorseAStructuralSink) {
    let ReadyDocument {
        owners,
        refs,
        // scalar state fields; plain drops
        ..
    } = old;
    retire_detached(owners, sink);
    drop(refs);
}

/// One recursion frame per retired AVL record (#59 §11.1): charge the
/// frame ledger, destroy the payload subtree, then descend. Destructuring
/// moves the children out of the box, so the record's allocation is
/// released without allocation or panic.
fn retire_avl_record(node: Box<AvlNode>, depth: u64, sink: &mut dyn HorseAStructuralSink) {
    sink.retire_avl_frame(depth);
    let AvlNode {
        left, right, owner, ..
    } = *node;
    retire_owner_payload(owner, 1, sink);
    if let Some(left) = left {
        retire_avl_record(left, depth + 1, sink);
    }
    if let Some(right) = right {
        retire_avl_record(right, depth + 1, sink);
    }
}

/// Destroy one retired Owner's payload in the payload recursion space.
/// A TriviaOnly Owner has no semantic payload nodes; a Syntax Owner's
/// normalized subtree is destroyed node by node. The outgoing restart
/// certificate and the Owner scalars retire as plain drops (no
/// retirement ledger charge).
fn retire_owner_payload(owner: Owner, depth: u64, sink: &mut dyn HorseAStructuralSink) {
    if let OwnerPayload::Syntax(payload) = owner.payload {
        retire_payload_node(payload.root, depth, sink);
    }
}

/// One recursion frame per destroyed normalized payload node (#59 §11.1).
fn retire_payload_node(node: Node, depth: u64, sink: &mut dyn HorseAStructuralSink) {
    sink.retire_payload_frame(depth);
    let Node { children, .. } = node;
    for child in children {
        retire_payload_node(child, depth + 1, sink);
    }
}
