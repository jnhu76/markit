//! `HORSE_A_V1_DIRECT_READY_REBUILD` — the #98 L0 diagnostic control.
//!
//! **This module is diagnostic-only (#98) and is NOT part of the frozen
//! Horse-A v1 mechanism identity.** Nothing in `update.rs`, `stage`,
//! `prepared`, `full_build`, or `retirement` was modified to add it; the
//! normal update path (`update` / `update_with_structural`) executes the
//! identical frozen pipeline whether or not this module exists.
//!
//! The control's conceptual intervention is exactly one (#98 §L0.3):
//!
//! ```text
//! skip incremental staging
//! -> invoke the existing legal Horse-A full-build route
//! -> construct an equivalent Horse-A READY post-state
//! ```
//!
//! It therefore reuses, unchanged:
//!
//! - the frozen association-validation boundary (`validate_association`,
//!   stage Phase 1 — same identity/range/UTF-8/length checks, same
//!   O(1) release shape, same caller-precondition trust model);
//! - the frozen same-target full builder (`full_build`: shared observed
//!   parser, final-first RefTable, eager materialization, Owner-relative
//!   coordinates, coverage plan, interior certificates, balanced build);
//! - the frozen `CommitPlan::Full` retirement path (whole-document
//!   retirement, charged inside the same completion boundary);
//! - the frozen ownership/lifetime model (old state consumed by value;
//!   pre-frontier refusal on validation failure via the same
//!   `UpdateError::InvalidAssociation`).
//!
//! It introduces no new parser, representation, semantic table, reduced
//! READY state, lazy repair, dependency index, allocator, or ownership
//! scheme. The full-build route it calls is byte-for-byte the branch the
//! frozen normal path already takes when `facts differ`.
//!
//! Counter discipline: the control charges only what it actually does.
//! It does NOT charge `record_full_build` (that field records the normal
//! path's semantic decision, which the control never makes); its route
//! identity lives in the #98 driver's row labels, not in this record.

use markit_mdbench_common::{CanonicalEdit, Source, WorkSink};

use crate::full_build::full_build;
use crate::state::ReadyDocument;
use crate::structural::{ForbiddenKind, HorseAStructuralSink};
use crate::update::{validate_association, UpdateError};

/// Execute the direct equivalent-READY rebuild control for one edit:
/// validate the association exactly as the normal path does, build the
/// complete same-target READY state for the post source through the
/// frozen full builder, retire the complete old document inside the same
/// boundary, and return the fresh READY state.
///
/// The timing/completion boundary is the whole call: validation + full
/// build + old-state retirement, mirroring the normal path's
/// stage+commit full route (retirement is charged inside `commit`
/// there; here it is charged inside this call).
pub fn direct_ready_rebuild<W: WorkSink>(
    old: ReadyDocument,
    old_source: &Source,
    post_source: &Source,
    edit: &CanonicalEdit,
    sink: &mut W,
    structural: &mut dyn HorseAStructuralSink,
) -> Result<ReadyDocument, UpdateError> {
    // The SAME validation boundary as stage Phase 1: association,
    // range, UTF-8 boundaries, length arithmetic. A refusal here is the
    // same pre-frontier `InvalidAssociation` the normal path returns.
    let _association = validate_association(&old, old_source, post_source, edit, structural)?;

    // Scale diagnostics, mirroring stage Phase 11's old-state reads.
    let m_old = old.owners.records();
    let h_old = old.owners.height();
    if old.owners.root.is_some() {
        structural.aggregate_reads(2);
    }
    structural.record_scale_old(m_old as u64, h_old as u64);

    // The existing legal full-build route (frozen `full_build`, the
    // same builder the facts-differ branch invokes).
    let fresh = full_build(post_source, sink, structural).map_err(UpdateError::FullBuild)?;

    // Stage Phase 11's full-branch scale diagnostics.
    let m_new = fresh.owners.records();
    if fresh.owners.root.is_some() {
        structural.aggregate_reads(1);
    }
    structural.record_scale_new(m_new as u64);

    // The frozen `CommitPlan::Full` frontier sequence, verbatim in
    // substance: whole-old-document removal + retirement + H_new read,
    // all inside this completion boundary.
    structural.owners_removed(old.owners.records() as u64);
    crate::retirement::retire_document(old, structural);

    let h_new = fresh.owners.height();
    if fresh.owners.root.is_some() {
        structural.aggregate_reads(1);
    }
    structural.record_height_new(h_new as u64);
    structural.forbidden(ForbiddenKind::AttributionTreeWalk, 0);

    Ok(fresh)
}
