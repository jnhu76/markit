//! The frozen same-target full builder (spec §13; task #7 pipeline):
//!
//! ```text
//! shared full block parse over the exact source (observed I1 seam)
//! → obtain complete root-level block structure + online provenance
//! → collect complete ordered document definition facts
//! → construct the FINAL complete RefTable
//! → eagerly materialize ALL semantic payload under that FINAL RefTable
//! → convert retained coordinates to Owner-relative form
//! → construct canonical physical-first-line Owner coverage
//! → attach permitted parser-derived restart certificates
//! → construct the retained OwnerSeq (construction-only balanced build)
//! → construct ReadyDocument
//! → return READY
//! ```
//!
//! The builder either returns a complete Horse-A ReadyDocument or fails:
//! no fallback to an H0-style state, no partial READY, no best-effort
//! Owner state (task #49). No incremental update, no I3 operators, no I5
//! counters live here. The READY-invariant validators are test/debug
//! gates (see `validate`); the frozen mechanism requires no mandatory
//! whole-state audit pass on the production path.

use markit_mdbench_common::{Source, WorkSink};
use markit_mdbench_shared_grammar::{
    materialize_one_with_sink, parse_region_observed, ObserverControl, RegionObserver,
    RootBlankEvent, TopLevelEvent,
};

use crate::coverage::CoveragePlan;
use crate::payload::shift_spans;
use crate::state::{AstPayload, InterpretationId, Owner, OwnerPayload, OwnerSeq, ReadyDocument};
use crate::validate;

/// Why a full build refused to return a state. A full-build failure is
/// an error: there is no weaker target to fall back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Parser observation evidence contradicts the block structure.
    InconsistentObservation { detail: String },
    /// A blockless non-empty document is not of the frozen root-blank
    /// grammar class, so no legal retained state exists for it.
    InvalidTriviaDocument { detail: String },
    /// A READY invariant failed at construction time.
    InvariantViolation(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::InconsistentObservation { detail } => {
                write!(f, "inconsistent parser observation: {detail}")
            }
            BuildError::InvalidTriviaDocument { detail } => {
                write!(f, "invalid trivia-only document: {detail}")
            }
            BuildError::InvariantViolation(detail) => {
                write!(f, "READY invariant violation: {detail}")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// Online provenance buffering (task #9): the scalar events the I1 seam
/// emits during the full parse, buffered for assembly. This is
/// construction-local state — dropped when the builder returns — never
/// persistent READY state and never a global certificate index.
#[derive(Default)]
struct FullBuildObserver {
    starts: Vec<usize>,
    barriers: Vec<RootBlankEvent>,
}

impl RegionObserver for FullBuildObserver {
    fn on_top_level_start(&mut self, ev: TopLevelEvent) {
        self.starts.push(ev.physical_line_start);
    }

    fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl {
        self.barriers.push(ev);
        // The full build never stops early: it must see the whole source.
        ObserverControl::Continue
    }
}

/// Build the complete same-target Horse-A READY state for `source`
/// (spec §13). The returned state is READY: complete, self-contained,
/// and next-edit-capable even though the incremental update algorithm
/// itself belongs to later slices.
pub fn full_build<W: WorkSink>(source: &Source, sink: &mut W) -> Result<ReadyDocument, BuildError> {
    let src = source.as_bytes();
    let source_len = src.len();

    // Shared full block parse over the exact source; TopLevelStart and
    // RootBlankBarrier provenance is collected online from the real I1
    // seam — never reconstructed later by rescanning the source, reading
    // semantic span starts, or guessing blank-line locations.
    let mut observer = FullBuildObserver::default();
    let observed = parse_region_observed(src, 0, source_len, sink, &mut observer);
    match observed.outcome {
        markit_mdbench_shared_grammar::RegionOutcome::RanToEnd => {}
        // Unreachable with an always-continue observer; a stop here would
        // mean the state is not the parse of the whole source.
        markit_mdbench_shared_grammar::RegionOutcome::StoppedAtCertifiedCut { cut } => {
            return Err(BuildError::InconsistentObservation {
                detail: format!("full build stopped early at certified cut {cut}"),
            });
        }
    }
    let region = observed.region;
    let markit_mdbench_shared_grammar::RegionParse {
        blocks,
        defs,
        fence_open_at_end: _,
    } = region;

    // Complete root-level block structure maps 1:1 onto the observed
    // TopLevelStart provenance (task #29 mapping invariant; the only
    // exceptions are the frozen empty and TriviaOnly special cases).
    if blocks.len() != observer.starts.len() {
        return Err(BuildError::InconsistentObservation {
            detail: format!(
                "{} root-level blocks but {} TopLevelStart observations",
                blocks.len(),
                observer.starts.len()
            ),
        });
    }

    // Canonical physical-first-line coverage (spec §3.2).
    let plan = CoveragePlan::build(&observer.starts, source_len)?;

    // Stage the Owners in source order.
    let mut staged: Vec<Owner> = Vec::with_capacity(blocks.len());
    if blocks.is_empty() {
        if source_len > 0 {
            // Frozen TriviaOnly case (spec §3.2): the document consists
            // only of the root-blank grammar class (SPACES*/LF bytes).
            // TAB/CR are ordinary text under BENCH-GRAMMAR-v1 and would
            // have formed a real paragraph Owner, so a blockless
            // non-blank-class document is an invariant failure, not a
            // TriviaOnly document.
            if !is_root_blank_class(src) {
                return Err(BuildError::InvalidTriviaDocument {
                    detail: format!(
                        "no root blocks but source contains bytes outside the SPACES*/LF blank class (len {source_len})"
                    ),
                });
            }
            staged.push(Owner {
                coverage_len: source_len,
                payload: OwnerPayload::TriviaOnly,
                outgoing_restart: None,
            });
        }
        // source_len == 0 stays the frozen empty case: an empty OwnerSeq,
        // never a fake syntax Owner (task #12).
    } else {
        // The FINAL complete document RefTable exists NOW, before any
        // reference-sensitive semantic materialization (spec §13.1; task
        // #7 critical rule). Eager materialization happens under it —
        // never with partial refs and never patched afterwards.
        for (skel, _p_i) in blocks.into_iter().zip(observer.starts.iter()) {
            let owner_index = staged.len();
            // Owner base = byte-weight prefix sum = the coverage cut c_i
            // (c_0 = 0; c_i = p_i for i > 0).
            let base = plan.cuts[owner_index];
            // Eager materialization of the complete block+inline semantic
            // subtree under the final RefTable (shared semantics reused,
            // not forked).
            let mut node = materialize_one_with_sink(src, skel, &defs, sink);
            if !(base <= node.start && node.end <= plan.cuts[owner_index + 1]) {
                return Err(BuildError::InconsistentObservation {
                    detail: format!(
                        "block semantic span [{}, {}) escapes its Owner coverage [{}, {})",
                        node.start,
                        node.end,
                        base,
                        plan.cuts[owner_index + 1]
                    ),
                });
            }
            // All retained coordinates become Owner-relative, recursively
            // (spec §4; FencedCode.content included).
            shift_spans(&mut node, -(base as isize));
            staged.push(Owner {
                coverage_len: plan.coverage_len(owner_index),
                payload: OwnerPayload::Syntax(AstPayload { root: node }),
                outgoing_restart: None,
            });
        }
        attach_interior_certificates(&mut staged, &plan, &observer.barriers)?;
    }

    // Retained sequence: construction-only balanced build (spec §12.5);
    // I3 owns the mutation/navigation operators.
    let owners = OwnerSeq::bulk_build(staged);

    let document = ReadyDocument {
        source_id: source.id(),
        source_len,
        interpretation: InterpretationId::HORSE_A_V1,
        owners,
        refs: defs,
    };

    // READY must mean actually ready (task #4/#7): construction itself
    // establishes every invariant (the incremental checks above — span
    // containment, coverage plan, certificate persistence — plus the
    // balanced build's own aggregate recomputation). The frozen
    // full-builder mechanism (spec §13) does NOT require a redundant
    // post-construction whole-state O(M) audit walk on the production
    // path, so the validators are test/debug gates only: tests call
    // validate_ready explicitly, and debug builds keep it as a belt.
    // No parser work, semantic repair, deferred index construction, or
    // required state-building step remains past this point either way.
    #[cfg(debug_assertions)]
    validate::validate_ready(&document).map_err(BuildError::InvariantViolation)?;

    Ok(document)
}

/// The frozen root-blank grammar class: every byte is a SPACE or the LF
/// terminating a `SPACES* LF` line (BENCH-GRAMMAR-v1 §1 — TAB/CR are
/// ordinary text, not whitespace). Shared with the incremental local path's
/// whole-document replacement cases (`fresh`), so the full and local
/// builders classify the class identically.
pub(crate) fn is_root_blank_class(src: &[u8]) -> bool {
    src.iter().all(|&b| b == b' ' || b == b'\n')
}

/// Install the persistent outgoing certificates (spec §6; data-model
/// §8.3; task #19): one certificate per INTERIOR Owner boundary that a real
/// pre-EOF `RootBlankEvent` certifies, with the frozen persistence
/// condition. The rule itself is shared with the incremental local path
/// (`certificate::persist_interior_certificates`, spec §24) so the fresh
/// and full boundaries can never diverge.
fn attach_interior_certificates(
    owners: &mut [Owner],
    plan: &CoveragePlan,
    barriers: &[RootBlankEvent],
) -> Result<(), BuildError> {
    // The full builder's last cut is the document EOF: never certified.
    crate::certificate::persist_interior_certificates(owners, &plan.cuts, barriers, false)
        .map_err(|detail| BuildError::InconsistentObservation { detail })
}
