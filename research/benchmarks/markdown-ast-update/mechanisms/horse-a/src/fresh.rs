//! Fresh replacement Owner construction for the local path (spec §13.1
//! pipeline mirror, scoped to the sealed replacement region; I4 task
//! contract §19–§20/§23–§24).
//!
//! The fresh Owners are built exactly the way the same-target full builder
//! builds them — canonical physical-first-line coverage over the region,
//! eager materialization under the environment the facts decision chose,
//! Owner-relative coordinates, and certificates taken only from this
//! round's real pre-EOF parser barriers — so local completion and full
//! construction agree on every logical Owner (spec §13.2). What differs is
//! only the region: the local path builds Δ_new Owners, never the whole
//! document.

use markit_mdbench_common::WorkSink;
use markit_mdbench_shared_grammar::{materialize_one_with_sink, RefTable, RootBlankEvent, Skel};

use crate::certificate::persist_interior_certificates;
use crate::coverage::CoveragePlan;
use crate::full_build::is_root_blank_class;
use crate::payload::shift_spans;
use crate::state::{AstPayload, Owner, OwnerPayload, OwnerSeq};
use crate::update::UpdateError;

/// Everything the forward parse sealed for one replacement region.
pub(crate) struct SealedRegion {
    /// The restart cut `r`: the region base, and therefore the first fresh
    /// Owner's coverage start.
    pub(crate) base: usize,
    /// The region end: the convergence cut `q_new`, or real `L_new` when the
    /// forward parse ran to EOF.
    pub(crate) end: usize,
    /// Completed root-level blocks sealed before `end`, in source order.
    pub(crate) blocks: Vec<Skel>,
    /// Observed `TopLevelStart` physical line starts (region provenance).
    pub(crate) starts: Vec<usize>,
    /// Observed pre-EOF root blank barriers (certificate evidence).
    pub(crate) barriers: Vec<RootBlankEvent>,
}

/// Build the fresh replacement Owners of one sealed region: coverage from
/// the region's own physical-first-line provenance, complete eager payload
/// under `refs`, this round's parser-derived certificates, and an O(Δ_new)
/// balanced build.
pub(crate) fn build_replacement_owners<W: WorkSink>(
    src: &[u8],
    region: SealedRegion,
    refs: &RefTable,
    sink: &mut W,
) -> Result<OwnerSeq, UpdateError> {
    let SealedRegion {
        base,
        end,
        blocks,
        starts,
        barriers,
    } = region;

    if blocks.len() != starts.len() {
        return Err(UpdateError::InconsistentObservation {
            detail: format!(
                "{} replacement blocks but {} TopLevelStart observations",
                blocks.len(),
                starts.len()
            ),
        });
    }

    if blocks.is_empty() {
        // A legal replacement region always contains a top-level block —
        // the guard block on the `r > 0` path, and (for `r = 0`) whatever
        // the blockless-prefix clause of the convergence predicate kept the
        // parse from cutting in front of. The one region that may legally
        // have no block is the WHOLE new document at real EOF: then the
        // frozen empty / TriviaOnly special cases apply.
        if base != 0 || end != src.len() {
            return Err(UpdateError::InconsistentObservation {
                detail: format!(
                    "replacement region [{base}, {end}) contains no top-level block but is \
                     not the whole document (length {})",
                    src.len()
                ),
            });
        }
        if end == 0 {
            // The frozen empty document: an empty OwnerSeq.
            return Ok(OwnerSeq::default());
        }
        if !is_root_blank_class(&src[..end]) {
            return Err(UpdateError::InconsistentObservation {
                detail: format!(
                    "no root blocks but the source contains bytes outside the SPACES*/LF blank \
                     class (len {end})"
                ),
            });
        }
        return Ok(OwnerSeq::bulk_build(vec![Owner {
            coverage_len: end,
            payload: OwnerPayload::TriviaOnly,
            outgoing_restart: None,
        }]));
    }

    // Canonical region coverage: `base`, then the 2nd..k-th blocks' physical
    // first-line starts, then the region end. The first fresh block's own
    // physical start is not a cut — the first fresh Owner already begins at
    // the restart cut and swallows the region's leading trivia, exactly as
    // the document's first Owner swallows the document's (spec §3.2).
    let plan = CoveragePlan::build_with_base(base, &starts, end).map_err(|e| {
        UpdateError::InconsistentObservation {
            detail: e.to_string(),
        }
    })?;

    let mut staged: Vec<Owner> = Vec::with_capacity(blocks.len());
    for (i, skel) in blocks.into_iter().enumerate() {
        let owner_base = plan.cuts[i];
        // Eager materialization under the chosen environment — the retained
        // RefTable on the facts-preserved path (spec §9: no reference-
        // sensitive payload is built before that environment is known).
        let mut node = materialize_one_with_sink(src, skel, refs, sink);
        if !(owner_base <= node.start && node.end <= plan.cuts[i + 1]) {
            return Err(UpdateError::InconsistentObservation {
                detail: format!(
                    "replacement block semantic span [{}, {}) escapes its Owner coverage \
                     [{owner_base}, {})",
                    node.start,
                    node.end,
                    plan.cuts[i + 1]
                ),
            });
        }
        // Every retained coordinate becomes Owner-relative (spec §4), so a
        // later edit that shifts this Owner's absolute base does not have to
        // rewrite its payload.
        shift_spans(&mut node, -(owner_base as isize));
        staged.push(Owner {
            coverage_len: plan.coverage_len(i),
            payload: OwnerPayload::Syntax(AstPayload { root: node }),
            outgoing_restart: None,
        });
    }

    // Fresh certificates come from this round's own barriers — including the
    // convergence barrier, which seals the boundary to the retained suffix
    // (spec §24; the I4 task contract's certificate-write derivation). That
    // last boundary is a real interior boundary of the NEW document whenever
    // the parse converged; it is the EOF boundary only when the replacement
    // ran to real EOF.
    let last_boundary_is_interior = end < src.len();
    persist_interior_certificates(
        &mut staged,
        &plan.cuts,
        &barriers,
        last_boundary_is_interior,
    )
    .map_err(|detail| UpdateError::InconsistentObservation { detail })?;

    Ok(OwnerSeq::bulk_build(staged))
}
