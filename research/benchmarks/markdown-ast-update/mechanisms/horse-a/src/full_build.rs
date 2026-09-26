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

use crate::certificate::RestartCertificate;
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
/// ordinary text, not whitespace).
fn is_root_blank_class(src: &[u8]) -> bool {
    src.iter().all(|&b| b == b' ' || b == b'\n')
}

/// Install the persistent outgoing certificates (spec §6; data-model
/// §8.3; task #19). A certificate exists only at an INTERIOR Owner
/// boundary: the boundary between two coverage records. The certifying
/// event is the real `RootBlankEvent` whose cut equals that boundary —
/// the last root blank line before the next Owner's first physical line.
///
/// Explicitly NOT persisted:
/// - the EOF boundary `c_k = source_len` (real EOF is a separate legal
///   completion path and needs no certificate; `finish()` manufactures
///   nothing — task #23);
/// - mid-gap blank events (several blank lines in one trivia gap produce
///   several transient events but exactly one interior boundary — the
///   frozen one-certificate-per-Owner-boundary mapping, task #19);
/// - leading-trivia blanks (their cuts precede the first Owner's block,
///   which is not an interior boundary; BOF stays a distinguished virtual
///   restart authority, data-model §8.3);
/// - a candidate whose support CANNOT satisfy the frozen persistence
///   condition (data-model §8.3): support must belong to the left Owner,
///   must not cross the left Owner's coverage start, and must correspond
///   to this interior boundary. Such a candidate is real parser evidence
///   that is merely not persistable HERE — the frozen rule is DO NOT
///   PERSIST that certificate, never reject the whole ReadyDocument. The
///   optional restart point is simply absent and READY stays valid; the
///   build continues (independent I2 review P1 repair). A genuinely
///   inconsistent builder state (two events claiming the same cut) is a
///   different class and stays a hard error.
fn attach_interior_certificates(
    owners: &mut [Owner],
    plan: &CoveragePlan,
    barriers: &[RootBlankEvent],
) -> Result<(), BuildError> {
    // Interior boundaries are c_1 .. c_(k-1); c_k = source_len is the EOF
    // boundary and is never certified.
    for i in 0..owners.len().saturating_sub(1) {
        let boundary = plan.cuts[i + 1];
        let mut candidates = barriers.iter().filter(|ev| ev.cut == boundary);
        let Some(ev) = candidates.next() else {
            continue;
        };
        if candidates.next().is_some() {
            // Two events with the same cut is impossible for real line
            // provenance (each physical line owns exactly one LF), so
            // this is inconsistent builder evidence, not a transient
            // candidate: a hard error (independent I2 review P1 repair
            // keeps this one).
            return Err(BuildError::InconsistentObservation {
                detail: format!(
                    "multiple root blank barriers certify the same boundary {boundary}"
                ),
            });
        }
        let base = plan.cuts[i];
        // Frozen persistence condition (data-model §8.3): the blank line
        // must lie strictly inside the left Owner's coverage — a blank
        // starting AT the Owner's base would pull its preceding LF (or a
        // BOF claim) across the coverage start. A candidate failing this
        // is skipped below, not an error: the certificate is optional.
        let Some(rel_blank_start) = ev.line_start.checked_sub(base).filter(|rel| *rel >= 1) else {
            continue;
        };
        // `None` (BOF) is legal support and stays None; an LF before the
        // left Owner's base cannot be represented Owner-relatively, so
        // that candidate is non-persistable too. Unreachable for
        // well-formed events (the seam always reports preceding_lf ==
        // line_start - 1, and line_start >= base + 1 above puts that LF
        // at or after base) — skipped rather than errored so no transient
        // evidence shape can abort READY.
        let rel_preceding = match ev.preceding_lf {
            None => None,
            Some(lf) => match lf.checked_sub(base) {
                Some(rel) => Some(rel),
                None => continue,
            },
        };
        let rel_blank_end = ev.cut - base;
        owners[i].outgoing_restart = Some(RestartCertificate {
            support: crate::certificate::RestartSupport {
                preceding_lf: rel_preceding,
                blank_line: rel_blank_start..rel_blank_end,
            },
        });
    }
    Ok(())
}
