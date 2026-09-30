//! Persistent restart-certificate support (spec §6; data-model §8).
//!
//! Logical support of a certificate at an interior Owner boundary `q` is
//! the LF that establishes the blank line's start plus the complete blank
//! physical line including its LF. The I1 `RootBlankEvent` carries
//! document-absolute construction-time offsets; persistence converts
//! them to Owner-relative coordinates before READY (task #18). The
//! support never crosses the left Owner's coverage start.

/// The frozen logical support form (spec §6): the LF terminating the
/// physical line before the blank barrier (`None` only at true BOF) and
/// the complete blank physical line `[start, end)` including its LF.
/// All coordinates are relative to the certificate's Owner base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartSupport {
    pub preceding_lf: Option<usize>,
    pub blank_line: std::ops::Range<usize>,
}

impl RestartSupport {
    /// The support as one contiguous half-open range. The I1 seam only
    /// issues `preceding_lf == blank_line.start - 1` (or `None` at BOF),
    /// so the two pieces are always adjacent; construction and validation
    /// enforce this, which makes the contiguous form exact.
    pub fn support_span(&self) -> std::ops::Range<usize> {
        match self.preceding_lf {
            Some(p) => {
                debug_assert_eq!(
                    p + 1,
                    self.blank_line.start,
                    "support pieces must be adjacent"
                );
                p..self.blank_line.end
            }
            None => {
                debug_assert_eq!(
                    self.blank_line.start, 0,
                    "no preceding LF is legal only at BOF"
                );
                self.blank_line.clone()
            }
        }
    }

    /// The frozen support-touch predicate (spec §6; task #20): an edit
    /// `[start, end)` touches the support iff it intersects the support,
    /// or — for a zero-length insertion — iff `start` lies in the
    /// support. E24 (frozen example, source `"ab\n\n"`, support bytes
    /// `{2,3}`): inserting at byte 2 touches the support, so the
    /// adjacent certificate is not reusable.
    ///
    /// `start`/`end` are in the same coordinate space as the stored
    /// support, i.e. Owner-relative for a persisted certificate.
    pub fn touches(&self, start: usize, end: usize) -> bool {
        touches_span(self.support_span(), start, end)
    }

    /// The same frozen predicate with the certificate's Owner base applied:
    /// `start`/`end` are document-absolute edit coordinates and the stored
    /// support is Owner-relative. The update's convergence predicate uses
    /// this form — an edit's byte interval is always document-absolute,
    /// while a persisted certificate never is (spec §4/§6).
    pub fn touches_absolute(&self, owner_base: usize, start: usize, end: usize) -> bool {
        let span = self.support_span();
        touches_span(owner_base + span.start..owner_base + span.end, start, end)
    }
}

/// The single support-touch formula both coordinate spaces share: an edit
/// intersects the support span, or — zero-length — lands inside it.
fn touches_span(span: std::ops::Range<usize>, start: usize, end: usize) -> bool {
    if start < end {
        start < span.end && end > span.start
    } else {
        span.contains(&start)
    }
}

use crate::state::Owner;
use crate::structural::{ForbiddenKind, HorseAStructuralSink};
use markit_mdbench_shared_grammar::RootBlankEvent;

/// Step 2R experimental mechanism identity (#95 Step 2R; issue #104): the
/// same-authority minimum certificate-matching repair over frozen Horse-A
/// v1. V itself is frozen and untouched at master `352e214`; a binary built
/// from a tree carrying this constant is the R mechanism, never V. The
/// identity must never enter `ReadyDocument` or any persisted state.
pub const CERTIFICATE_MATCH_REPAIR_ID: &str = "HORSE-A-V2-STEP2R-CERT-MATCH-R1";

/// Install the persistent outgoing certificates of one Owner sequence from
/// real parser evidence (§6; data-model §8.3): a certificate exists only at
/// an INTERIOR boundary, `cuts[i + 1]` for `i < owners.len() - 1`, and only
/// where a real `RootBlankEvent` observed during the actual parse carries
/// exactly that cut.
///
/// `last_boundary_is_interior` distinguishes the two callers. The full
/// builder's last cut is always the document EOF, which is never certified.
/// The incremental local path's last boundary is the document EOF when the
/// forward parse ran to real EOF, but it is a REAL INTERIOR boundary when
/// the parse converged and a retained suffix follows — the convergence
/// barrier itself, which the frozen certificate-write derivation certifies
/// on the last fresh Owner (§24).
///
/// Explicitly NOT persisted:
/// - the document EOF boundary (real EOF is a separate legal completion path
///   and needs no certificate; `finish()` manufactures nothing);
/// - mid-gap blank events (several blank lines in one trivia gap produce
///   several transient events but exactly one interior boundary);
/// - leading-trivia blanks (their cuts precede the first block, which is not
///   an interior boundary; BOF stays a distinguished virtual restart
///   authority);
/// - a candidate whose support CANNOT satisfy the frozen persistence
///   condition (support must belong to the left Owner, must not cross its
///   coverage start, and must end at this boundary). Such a candidate is
///   real parser evidence that is merely not persistable HERE — the frozen
///   rule is DO NOT PERSIST that certificate, never reject the whole state.
///   The optional restart point is simply absent and READY stays valid. A
///   genuinely inconsistent state (two events claiming the same cut) is a
///   different class and stays a hard error.
///
/// `cuts` is the Owner cut sequence of the sequence being assembled:
/// `cuts[i]..cuts[i + 1]` is Owner `i`'s coverage, so `cuts.len()` must be
/// `owners.len() + 1`. Shared by the full builder and the local replacement
/// path (spec §24: the fresh boundaries are certified from this round's own
/// parser evidence). Callers always pass exactly the FRESH Owners being
/// assembled — retained suffix certificates are never rewritten here, and
/// the site charges that sentinel zero on every execution.
///
/// # Step 2R matching traversal (`CERTIFICATE_MATCH_REPAIR_ID`)
///
/// The matcher is a single forward merge traversal over two sequences the
/// callers construct strictly increasing, replacing V's per-boundary full
/// filter scan (V = master `352e214`, where every examined boundary
/// exhausted `barriers.iter().filter(..)` — exactly `N_b × B` cut
/// inspections per call):
///
/// - `cuts[1..=boundary_count]` strictly increasing — runtime-enforced by
///   `CoveragePlan::build_with_base` at both callers;
/// - `barriers` strictly increasing by `cut` — single-scan collection
///   (one `parse_region_observed` per call, one forward per-line pass, at
///   most one event per dispatched line, append-only observers).
///
/// A barrier is *examined* by the cursor at most once; events skipped
/// before a boundary have `cut < boundary <` every later boundary and can
/// never match again. The match predicate stays exact `cut` equality.
/// Duplicate detection is preserved with V's semantics and scope: only
/// examined interior boundaries detect duplicates (one adjacent-element
/// peek — under the stated monotonicity a second same-cut event, if any,
/// is exactly the next element), the check runs BEFORE any support check,
/// the lowest examined boundary errors first, and the error class/text is
/// V's. Duplicates at non-examined cuts stay silently ignored, exactly as
/// in V.
///
/// The monotone precondition is a documented caller invariant (proven at
/// both callers), not re-checked here: this traversal is total and adds no
/// new panic mode on violative input (which would have undefined matching
/// behavior, is unreachable from both callers, and must not be relied on).
/// The pre-existing unchecked `ev.cut - base` keeps V's exact arithmetic.
pub(crate) fn persist_interior_certificates(
    owners: &mut [Owner],
    cuts: &[usize],
    barriers: &[RootBlankEvent],
    last_boundary_is_interior: bool,
    sink: &mut dyn HorseAStructuralSink,
) -> Result<(), String> {
    if owners.len() + 1 != cuts.len() {
        return Err(format!(
            "{} Owners but {} coverage cuts",
            owners.len(),
            cuts.len()
        ));
    }
    let boundary_count = if last_boundary_is_interior {
        owners.len()
    } else {
        owners.len().saturating_sub(1)
    };
    let mut cursor = 0usize;
    for i in 0..boundary_count {
        let boundary = cuts[i + 1];
        // Advance past every barrier that ends before this boundary.
        // One diagnostic inspection per barrier cut compared here; the
        // batch is charged once per examined boundary, never per
        // comparison (see `certificate_barrier_inspections`).
        let mut inspected = 0u64;
        while cursor < barriers.len() {
            inspected += 1;
            if barriers[cursor].cut >= boundary {
                break;
            }
            cursor += 1;
        }
        sink.certificate_barrier_inspections(inspected);
        let Some(ev) = barriers.get(cursor) else {
            // Every barrier is behind this boundary; later boundaries are
            // larger still, so nothing further can match (monotonicity).
            break;
        };
        if ev.cut > boundary {
            // First not-yet-consumed barrier ends past this boundary: no
            // candidate here, and the barrier stays pending for later,
            // larger boundaries.
            continue;
        }
        // ev.cut == boundary: the exact-equality match. V searched the
        // whole remainder for a second match BEFORE any support check;
        // under the monotone collection the second match, if any, is
        // exactly the adjacent element.
        if let Some(next) = barriers.get(cursor + 1) {
            sink.certificate_barrier_inspections(1);
            if next.cut == boundary {
                return Err(format!(
                    "multiple root blank barriers certify the same boundary {boundary}"
                ));
            }
        }
        let base = cuts[i];
        // The blank line must lie strictly inside the left Owner's coverage:
        // a blank starting AT the Owner's base would pull its preceding LF
        // (or a BOF claim) across the coverage start.
        let Some(rel_blank_start) = ev.line_start.checked_sub(base).filter(|rel| *rel >= 1) else {
            continue;
        };
        // `None` (BOF) is legal support and stays None; an LF before the
        // left Owner's base cannot be represented Owner-relatively, so that
        // candidate is non-persistable too. Unreachable for well-formed
        // events (the seam always reports `preceding_lf == line_start - 1`,
        // and `line_start >= base + 1` above puts that LF at or after
        // `base`) — skipped rather than errored so no transient evidence
        // shape can abort a legal state.
        let rel_preceding = match ev.preceding_lf {
            None => None,
            Some(lf) => match lf.checked_sub(base) {
                Some(rel) => Some(rel),
                None => continue,
            },
        };
        let rel_blank_end = ev.cut - base;
        // The persistence seam itself: installation of one persistent
        // outgoing certificate. A transient RootBlankEvent observed by the
        // parser is not a write; this assignment is.
        owners[i].outgoing_restart = Some(RestartCertificate {
            support: RestartSupport {
                preceding_lf: rel_preceding,
                blank_line: rel_blank_start..rel_blank_end,
            },
        });
        sink.certificate_write();
        // The matched barrier is consumed: it certified this boundary and
        // every later boundary is strictly larger (monotonicity).
        cursor += 1;
    }
    // Defended-site sentinel assertion: only the fresh Owners passed in
    // were touched; the retained suffix's certificates are structurally
    // retained and never rewritten. A regression that rewrote them would
    // charge this site.
    sink.forbidden(ForbiddenKind::UnaffectedCertificateWrites, 0);
    Ok(())
}

/// A persistent outgoing RestartCertificate at one Owner's right
/// coverage cut. It exists only where a real I1 `RootBlankEvent` was
/// observed by the actual parser (never from `finish()`, never from a
/// reconstructed AST state, never at the EOF boundary). A transient
/// event is not persistent state; installation into the retained Owner
/// boundary creates the certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartCertificate {
    pub support: RestartSupport,
}
