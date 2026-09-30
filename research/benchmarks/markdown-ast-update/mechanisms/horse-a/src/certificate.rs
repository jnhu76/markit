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
/// # Step 2R matching traversal (`CERTIFICATE_MATCH_REPAIR_ID`, contract
/// amendment v3)
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
/// **Eligibility gate (amendment v3, extended after review E).** Legal
/// production parsing always satisfies both orderings, so the fast path
/// always applies there. The gate itself is an allocation-free O(B + N_b)
/// scan that re-verifies BOTH orderings the traversal relies on — the
/// barriers strictly increasing by `cut` AND the cuts strictly increasing
/// (the exhaustion break "nothing later can match" is unsound for
/// decreasing cuts) — before entering the fast traversal. Any slice pair
/// violating either ordering — a duplicate or decreasing event, or
/// non-monotone cuts, all unreachable from legal parser generation — is
/// routed to [`v1_defensive_fallback`], which is the frozen V matcher
/// copied verbatim and is defined on arbitrary input. Defensive input
/// therefore keeps V's exact seam semantics on EVERY shape: same
/// whole-slice duplicate search, same examined-
/// boundary scope, same precedence (lowest examined duplicate boundary
/// errors first), same error text, same support checks, silent skips,
/// `certificate_write` accounting, EOF/local-convergence behavior and
/// forbidden-sentinel behavior. No sorting, deduplication, retained
/// state, heap allocation, or new error class is introduced, and the
/// traversal stays total (no new panic mode) on any input.
///
/// A barrier at the cursor is *stopped at* by at most one boundary and
/// advanced past at most once — except a support-rejected exact match,
/// which stays at the cursor and is compared once more at the next
/// examined boundary (the `R` term of the fast-match bound below). The
/// match predicate stays exact `cut` equality.
/// Duplicate detection preserves V's semantics and scope on BOTH routes:
/// only examined interior boundaries detect duplicates, the check runs
/// BEFORE any support check, the lowest examined boundary errors first,
/// and the error class/text is V's. On the fast path the duplicate check
/// is the adjacent-element peek below — provably inert under the gate
/// (strict monotonicity of both sequences makes a second same-cut event
/// impossible), kept as a defended site; the real duplicate defense for
/// violative input is the fallback's whole-slice V scan. Duplicates at
/// non-examined cuts stay silently ignored, exactly as in V.
///
/// # Work accounting (amendment v3, proved against this code)
///
/// - `ORDER_CHECK_WORK = max(B - 1, 0) + boundary_count` cut comparisons
///   for the eligibility gate (barriers scan + cuts scan); allocation-
///   free; NOT charged to the diagnostic counter (its semantics are
///   fixed below).
/// - `FAST_MATCH` (what `certificate_barrier_inspections` counts: seek
///   iterations + duplicate peeks) `<= B + N_b + R`, where `R` is the
///   number of exact-cut matches whose support is not persistable (each
///   such barrier stays at the cursor and is re-inspected once at the
///   next examined boundary; each contributes one seek inspection and
///   one peek beyond the `B + N_b` core). `R = 0` on every measured
///   evidence path. Derivation: every barrier is advanced past at most
///   once (`A = B - U - I`, `U` never-reached, `I` installed), every
///   examined boundary breaks at most once (`T <= N_b`), every match
///   peeks at most once (`P <= M = I + R`), so
///   `A + T + P <= B + N_b + R - U`.
/// - Total legal-path matching + eligibility work is therefore
///   `<= 2B + 2*boundary_count + R - 1` (empty shape `B = 0`,
///   `boundary_count = 0`: no work), i.e. linear in `B + N_b`, against
///   V's exact `N_b × B`.
/// - The defensive fallback route is exactly V's `N_b × B` predicate
///   evaluations (the frozen algorithm's own cost) and exists only for
///   inputs outside legal production generation.
/// - Diagnostic-counter scope: `certificate_barrier_inspections` counts
///   FAST-MATCH seek + peek inspections only. Order-check comparisons
///   are not included, and the V fallback (V has no such diagnostic)
///   charges nothing. Total work is accounted analytically above.
///
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
    // Amendment v3 eligibility gate (extended after review E): strict
    // monotonicity of BOTH sequences the traversal relies on, checked
    // allocation-free in one forward pass each. Legal production parsing
    // is always eligible (R-C2 collection invariants + CoveragePlan
    // runtime cuts checks at both callers); anything else takes the
    // frozen-V fallback below, which is defined on arbitrary input.
    let strictly_increasing = barriers.windows(2).all(|w| w[0].cut < w[1].cut)
        && cuts[..=boundary_count].windows(2).all(|w| w[0] < w[1]);
    if !strictly_increasing {
        return v1_defensive_fallback(owners, cuts, barriers, boundary_count, sink);
    }
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
        // the eligibility gate guarantees strict monotonicity, so a
        // second same-cut event cannot exist and this adjacent peek is
        // provably inert. It is retained as a defended site (one charged
        // inspection per match), never as the duplicate defense — that
        // role belongs to the fallback's whole-slice V scan.
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
        // shape can abort a legal state. A support-rejected match stays at
        // the cursor and is re-inspected (once) at the next examined
        // boundary — the `R` term of the fast-match bound above.
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

/// The frozen V matcher (master `352e214`,
/// `mechanisms/horse-a/src/certificate.rs`, `persist_interior_certificates`
/// loop), copied verbatim as the amendment-v3 defensive fallback. It runs
/// ONLY for barrier slices that are not strictly increasing by `cut` —
/// shapes unreachable from legal parser generation — so that non-monotone
/// defensive input keeps V's exact seam behavior instead of the fast
/// traversal's monotone assumptions. Verbatim means verbatim: the lazy
/// per-boundary full-slice filter, the whole-remainder duplicate search
/// before any support check, the identical error text, support checks,
/// silent skips, charge sites, and the early `Err` that skips the
/// forbidden-sentinel tail exactly as V's does. No inspections are
/// charged here (V has no such diagnostic; see the counter-scope note on
/// [`persist_interior_certificates`]).
fn v1_defensive_fallback(
    owners: &mut [Owner],
    cuts: &[usize],
    barriers: &[RootBlankEvent],
    boundary_count: usize,
    sink: &mut dyn HorseAStructuralSink,
) -> Result<(), String> {
    for i in 0..boundary_count {
        let boundary = cuts[i + 1];
        let mut candidates = barriers.iter().filter(|ev| ev.cut == boundary);
        let Some(ev) = candidates.next() else {
            continue;
        };
        if candidates.next().is_some() {
            return Err(format!(
                "multiple root blank barriers certify the same boundary {boundary}"
            ));
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
        owners[i].outgoing_restart = Some(RestartCertificate {
            support: RestartSupport {
                preceding_lf: rel_preceding,
                blank_line: rel_blank_start..rel_blank_end,
            },
        });
        sink.certificate_write();
    }
    // V's tail, verbatim: the defended-site sentinel charges on successful
    // completion exactly as in V, and the duplicate `Err` above skips it
    // exactly as in V.
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
