//! Step 2R same-authority repair tests (issue #104; parent #95).
//!
//! Three gates, all against the FROZEN V behavior, never against a
//! rewritten expectation:
//!
//! 1. **Seam differential** — a verbatim replica of the frozen V matcher
//!    (master `352e214`,
//!    `mechanisms/horse-a/src/certificate.rs`,
//!    `persist_interior_certificates`; blob-identified in the PR receipt)
//!    runs beside R over a battery of seam inputs; persisted Owner state,
//!    result class, error text and `certificate_write` charges must be
//!    element-for-element identical. The R branch changes no code outside
//!    the certificate seam (plus the additive diagnostic sink seam), so
//!    seam-level V/R equivalence plus the untouched shared code is what
//!    makes this a V-vs-R gate and not merely a self-test.
//! 2. **Work reduction** — V's inspection count is exactly `N_b × B`
//!    (the lazy filter is exhausted for every examined boundary); R's
//!    charged tally is asserted at exact values on a hand-traced case and
//!    under the `B + N_b` bound (and strictly below V) on a dense case.
//! 3. **End-to-end same-authority** — through the public update API:
//!    R-produced READY states must equal the from-scratch clean
//!    full-build authority as FULL `ReadyDocument` state (Owner sequence
//!    including persisted certificates, RefTable, ids), across next-edit
//!    chains, certificate-adjacent edits, local convergence, retained
//!    suffixes, and CJK/emoji content.

use crate::certificate::{persist_interior_certificates, RestartCertificate, RestartSupport};
use crate::state::{Owner, OwnerPayload};
use crate::structural::{
    ForbiddenKind, HorseAStructuralSink, NoopHorseAStructuralSink, Observed,
    RecordingHorseAStructuralSink,
};
use markit_mdbench_shared_grammar::RootBlankEvent;

// ---------------------------------------------------------------------------
// The frozen V matcher, replicated verbatim for differential comparison.
//
// Source of truth: master 352e214,
// research/benchmarks/markdown-ast-update/mechanisms/horse-a/src/
// certificate.rs, `persist_interior_certificates`. The loops below mirror
// its lazy filter one predicate evaluation at a time; `inspections` counts
// exactly those evaluations and nothing else. Support checks, installation,
// error text and charge sites are copied byte-for-byte in intent; any
// divergence from 352e214 is a bug in THIS test, not in R.
// ---------------------------------------------------------------------------

fn v1_replica_persist_interior_certificates(
    owners: &mut [Owner],
    cuts: &[usize],
    barriers: &[RootBlankEvent],
    last_boundary_is_interior: bool,
    sink: &mut dyn HorseAStructuralSink,
    inspections: &mut u64,
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
    for i in 0..boundary_count {
        let boundary = cuts[i + 1];
        // `barriers.iter().filter(|ev| ev.cut == boundary)` — first next():
        let mut first: Option<usize> = None;
        for (idx, ev) in barriers.iter().enumerate() {
            *inspections += 1;
            if ev.cut == boundary {
                first = Some(idx);
                break;
            }
        }
        let Some(k) = first else {
            continue;
        };
        // second next(): the whole remainder, before any support check.
        let mut duplicate = false;
        for ev in &barriers[k + 1..] {
            *inspections += 1;
            if ev.cut == boundary {
                duplicate = true;
                break;
            }
        }
        if duplicate {
            return Err(format!(
                "multiple root blank barriers certify the same boundary {boundary}"
            ));
        }
        let ev = barriers[k];
        let base = cuts[i];
        let Some(rel_blank_start) = ev.line_start.checked_sub(base).filter(|rel| *rel >= 1) else {
            continue;
        };
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
    sink.forbidden(ForbiddenKind::UnaffectedCertificateWrites, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

fn ev(line_start: usize, line_lf: usize, preceding: Option<usize>) -> RootBlankEvent {
    RootBlankEvent {
        line_start,
        line_lf,
        cut: line_lf + 1,
        preceding_lf: preceding,
    }
}

fn blank_owners(cuts: &[usize]) -> Vec<Owner> {
    cuts[..cuts.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, &base)| Owner {
            coverage_len: cuts[i + 1] - base,
            payload: OwnerPayload::TriviaOnly,
            outgoing_restart: None,
        })
        .collect()
}

struct Differential {
    /// (result, owners, certificate_writes)
    r: (Result<(), String>, Vec<Owner>, Observed),
    v: (Result<(), String>, Vec<Owner>, Observed),
    r_inspections: u64,
    v_inspections: u64,
}

fn run_differential(cuts: &[usize], barriers: &[RootBlankEvent], last: bool) -> Differential {
    let mut owners_r = blank_owners(cuts);
    let mut owners_v = blank_owners(cuts);

    let mut sink_r = RecordingHorseAStructuralSink::new();
    let r = persist_interior_certificates(&mut owners_r, cuts, barriers, last, &mut sink_r);
    let writes_r = sink_r.counters().certificate_writes;
    let r_inspections = sink_r.certificate_barrier_inspections();

    let mut sink_v = RecordingHorseAStructuralSink::new();
    let mut v_inspections = 0u64;
    let v = v1_replica_persist_interior_certificates(
        &mut owners_v,
        cuts,
        barriers,
        last,
        &mut sink_v,
        &mut v_inspections,
    );
    let writes_v = sink_v.counters().certificate_writes;

    Differential {
        r: (r, owners_r, writes_r),
        v: (v, owners_v, writes_v),
        r_inspections,
        v_inspections,
    }
}

#[track_caller]
fn assert_v_r_parity(cuts: &[usize], barriers: &[RootBlankEvent], last: bool) -> Differential {
    let d = run_differential(cuts, barriers, last);
    assert_eq!(
        d.r.0, d.v.0,
        "R and V disagree on the result class/message for cuts {cuts:?} last={last}"
    );
    assert_eq!(
        d.r.1, d.v.1,
        "R and V disagree on the persisted Owner state for cuts {cuts:?} last={last}"
    );
    assert_eq!(
        d.r.2, d.v.2,
        "R and V disagree on certificate_write charges for cuts {cuts:?} last={last}"
    );
    d
}

// ---------------------------------------------------------------------------
// Gate 1 + 2: the seam battery
// ---------------------------------------------------------------------------

#[test]
fn the_seam_battery_is_v_equivalent() {
    // 1. no barriers at all — nothing persists anywhere.
    let d = assert_v_r_parity(&[0, 10, 20, 30], &[], false);
    assert!(d.r.0.is_ok());
    assert!(d.r.1.iter().all(|o| o.outgoing_restart.is_none()));

    // 2. one valid interior certificate.
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(4, 9, Some(3))], false);
    assert_eq!(
        d.r.1[0].outgoing_restart,
        Some(RestartCertificate {
            support: RestartSupport {
                preceding_lf: Some(3),
                blank_line: 4..10
            }
        })
    );
    assert!(d.r.1[1].outgoing_restart.is_none());

    // 3. multiple interior boundaries, all certified.
    assert_v_r_parity(&[0, 10, 20, 30], &[ev(4, 9, Some(3)), ev(14, 19, Some(13))], false);

    // 4. several blank events inside one gap — only the boundary-cut event
    //    persists (mid-gap events are silently skipped, R-C4 class 1).
    let d = assert_v_r_parity(
        &[0, 10, 20],
        &[ev(2, 3, Some(1)), ev(4, 5, Some(3)), ev(6, 9, Some(5))],
        false,
    );
    assert_eq!(
        d.r.1[0]
            .outgoing_restart
            .as_ref()
            .unwrap()
            .support
            .blank_line,
        6..10
    );

    // 5. leading-trivia events (before the first examined boundary).
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(0, 1, None), ev(2, 3, Some(1))], false);
    assert!(d.r.1.iter().all(|o| o.outgoing_restart.is_none()));

    // 6. the EOF cut is never an examined boundary (R-C6 full build).
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(6, 9, Some(5)), ev(16, 19, Some(15))], false);
    assert!(d.r.1[0].outgoing_restart.is_some());
    assert!(d.r.1[1].outgoing_restart.is_none());

    // 7. a BOF-shaped event (line_start 0, preceding None) at an examined
    //    boundary is not strictly inside the left Owner: silently skipped
    //    (R-C4 class 2), never persisted, never an error.
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(0, 9, None)], false);
    assert!(d.r.0.is_ok());
    assert!(d.r.1.iter().all(|o| o.outgoing_restart.is_none()));

    // 8. support that cannot be represented in the left Owner: blank at
    //    the Owner base (rel start 0) and blank before the base
    //    (underflow) — both skipped, no error (R-C4 classes 2 and 3).
    assert_v_r_parity(&[5, 15, 25], &[ev(5, 14, Some(4))], false);
    assert_v_r_parity(&[5, 15, 25], &[ev(3, 14, Some(2))], false);

    // 9. two events with the same cut at an examined boundary — the same
    //    hard error as V (R-C3), duplicate check before support checks.
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(4, 9, Some(3)), ev(6, 9, Some(5))], false);
    assert_eq!(
        d.r.0,
        Err("multiple root blank barriers certify the same boundary 10".to_string())
    );

    // 10. duplicate at a NON-first examined boundary — errors at the lowest
    //     examined boundary carrying duplicates, exact message (B-P3-4).
    let d = assert_v_r_parity(
        &[0, 10, 20, 30],
        &[
            ev(4, 9, Some(3)),
            ev(14, 19, Some(13)),
            ev(16, 19, Some(15)),
        ],
        false,
    );
    assert_eq!(
        d.r.0,
        Err("multiple root blank barriers certify the same boundary 20".to_string())
    );

    // 11. duplicates at an UNEXAMINED cut (the EOF cut here) stay silently
    //     ignored — no global duplicate validation may appear (A-P1-1).
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(14, 19, Some(13)), ev(16, 19, Some(15))], false);
    assert!(d.r.0.is_ok());
    assert!(d.r.1.iter().all(|o| o.outgoing_restart.is_none()));

    // 12. local convergence: with `last_boundary_is_interior` the LAST
    //     fresh boundary is a real interior boundary and IS certified
    //     (R-C6 local branch).
    let d = assert_v_r_parity(&[0, 10, 20], &[ev(6, 9, Some(5)), ev(16, 19, Some(15))], true);
    assert!(d.r.1[0].outgoing_restart.is_some());
    assert!(d.r.1[1].outgoing_restart.is_some());

    // 13. single-Owner / no-interior-boundary / empty shapes.
    assert_v_r_parity(&[0, 10], &[ev(4, 9, Some(3))], false);
    let d = assert_v_r_parity(&[0, 10], &[ev(4, 9, Some(3))], true);
    assert!(d.r.1[0].outgoing_restart.is_some());
    assert_v_r_parity(&[0], &[], false);
}

/// The cuts-length precondition error is identical in R and V (it precedes
/// the matcher in both), and the forbidden sentinel is not charged on the
/// early error — exactly as in V.
#[test]
fn the_cuts_precondition_error_is_v_identical() {
    // 2 Owners but only 2 cuts (must be 3): the precondition error.
    let mut owners = blank_owners(&[0, 10, 20]);
    let mut sink = RecordingHorseAStructuralSink::new();
    let err =
        persist_interior_certificates(&mut owners, &[0, 10], &[], false, &mut sink).unwrap_err();
    assert_eq!(err, "2 Owners but 2 coverage cuts");
    assert_eq!(
        sink.counters().forbidden_unaffected_certificate_writes,
        Observed::Unknown,
        "the early error precedes the sentinel charge, as in V"
    );
}

/// Dense mixed geometry: events at every other examined boundary with
/// mid-gap noise everywhere — full state parity plus the work bound.
#[test]
fn the_dense_geometry_shows_strict_reduction_with_state_parity() {
    let mut cuts = vec![0usize];
    let mut barriers = Vec::new();
    for i in 0..32 {
        let boundary = cuts[i] + 10;
        barriers.push(ev(cuts[i] + 2, cuts[i] + 3, Some(cuts[i] + 1)));
        barriers.push(ev(cuts[i] + 5, cuts[i] + 6, Some(cuts[i] + 4)));
        if i % 2 == 0 {
            // line_lf = boundary - 1 -> cut = boundary: matches the boundary.
            barriers.push(ev(boundary - 1, boundary - 1, Some(boundary - 2)));
        }
        cuts.push(boundary);
    }
    let n_b = 31usize; // 32 owners, last boundary (EOF) unexamined
    let b = barriers.len();
    let d = assert_v_r_parity(&cuts, &barriers, false);

    // V: exactly N_b × B — every examined boundary exhausts the filter.
    assert_eq!(d.v_inspections as usize, n_b * b);
    // R, hand-traced: 31 examined boundaries × (2 noise advances + 1
    // stop/match iteration) = 93 seek iterations, + 1 duplicate peek per
    // matched boundary (16 boundary events at even i) = 109; the two noise
    // events behind the last examined boundary are never entered.
    assert_eq!(d.r_inspections as usize, 93 + 16);
    assert!(
        d.r_inspections as usize <= b + n_b,
        "R charged {} inspections; bound B + N_b = {b} + {n_b}",
        d.r_inspections
    );
    assert!(
        d.r_inspections < d.v_inspections,
        "dense geometry must show strict reduction: R {} vs V {}",
        d.r_inspections,
        d.v_inspections
    );
}

/// Hand-traced exact work accounting on a small case (the receipt's
/// measured-R anchor).
#[test]
fn the_work_accounting_is_exact_on_a_hand_traced_case() {
    // 3 Owners, last=false: N_b = 2 examined boundaries (cuts 10 and 20;
    // cut 30 is the EOF and is never examined); B = 5 barriers; boundary
    // events at 10 and 20; mid-gap noise at cuts 4, 14, 24.
    let cuts = [0usize, 10, 20, 30];
    let barriers = [
        ev(2, 3, Some(1)),    // cut 4  (noise)
        ev(4, 9, Some(3)),    // cut 10 (matches boundary 10)
        ev(11, 13, Some(10)), // cut 14 (noise)
        ev(14, 19, Some(13)), // cut 20 (matches boundary 20)
        ev(21, 23, Some(20)), // cut 24 (noise, behind the last match)
    ];
    let d = run_differential(&cuts, &barriers, false);
    assert!(d.r.0.is_ok());
    assert_eq!(d.r.1, d.v.1);

    assert_eq!(d.v_inspections, 2 * 5, "V is exactly N_b × B here");
    // R, hand-traced: boundary 10 = 2 seek + 1 peek; boundary 20 = 2 seek
    // + 1 peek; the EOF cut is never examined. Total 6.
    assert_eq!(d.r_inspections, 6);
    assert!(d.r_inspections < d.v_inspections);
}

// ---------------------------------------------------------------------------
// Gate 3: end-to-end same-authority through the public update API.
//
// The strongest available from-scratch authority for a READY state is the
// clean same-target full build of the post source: byte-identical Owner
// sequence (payloads AND persisted certificates), RefTable, lengths, ids.
// These cases hold the update result to exactly that, so any R-caused
// drift in certificate presence, support coordinates or Owner association
// would fail here even where a normalized export would not.
// ---------------------------------------------------------------------------

use crate::full_build::full_build;
use crate::state::ReadyDocument;
use crate::update::update;
use crate::validate::validate_ready;
use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};

fn src(id: u64, text: &str) -> Source {
    Source::new(SourceId(id), text)
}

fn clean(text: &str, id: u64) -> ReadyDocument {
    full_build(&src(id, text), &mut NoopWorkSink, &mut NoopHorseAStructuralSink)
        .expect("clean full build")
}

fn apply_edit(text: &str, start: usize, end: usize, inserted: &str) -> String {
    let mut s = text.to_string();
    s.replace_range(start..end, inserted);
    s
}

fn edit_of(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("canonical edit")
}

/// One update step on the R mechanism, asserted to produce exactly the
/// from-scratch authority. The comparison is the FULL LOGICAL STATE — the
/// in-order Owner sequence (payloads AND persisted certificates),
/// RefTable, lengths, ids, interpretation — with one deliberate
/// exclusion: the retained AVL node shape, which is derived structure
/// (the update path's split/join shape differs from the bulk-build shape
/// for V exactly as for R; the frozen validate_ready gates the shape's
/// invariants instead).
#[track_caller]
fn update_step_must_equal_clean(
    pre: ReadyDocument,
    pre_text: &str,
    post_text: &str,
    start: usize,
    end: usize,
    inserted: &str,
    id: u64,
) -> ReadyDocument {
    let pre_id = pre.source_id;
    let edit = edit_of(start, end, inserted);
    let post_source = src(id, &post_text);
    let updated = update(
        pre,
        &Source::new(pre_id, pre_text),
        &post_source,
        &edit,
        &mut NoopWorkSink,
    )
    .expect("R update succeeds");
    validate_ready(&updated).expect("READY invariants hold");
    let authority = clean(post_text, id);
    let updated_seq: Vec<&Owner> = updated.owners.owners_in_order();
    let authority_seq: Vec<&Owner> = authority.owners.owners_in_order();
    assert!(
        updated_seq == authority_seq,
        "R update Owner sequence != clean from-scratch Owner sequence for {post_text:?}"
    );
    assert_eq!(updated.refs, authority.refs, "RefTable drift");
    assert_eq!(updated.source_len, authority.source_len);
    assert_eq!(updated.source_id, authority.source_id);
    assert_eq!(updated.interpretation, authority.interpretation);
    updated
}

/// E6-style certificate-adjacent chain: edit1 sits next to certificate
/// support (deleting the blank line that carries it), edit2 depends on the
/// persisted restart state further down. Both READY stages must equal the
/// from-scratch authority; the certificate set must move exactly as the
/// clean authority's does.
#[test]
fn the_next_edit_chain_with_certificate_adjacent_edits_matches_the_clean_authority() {
    let s0 = "Alpha para one.\n\nBeta para two.\n\nGamma para three.\n\nDelta para four.\n";

    // edit1: delete the blank line between Alpha and Beta — this touches
    // the support bytes of the Alpha|Beta boundary certificate.
    let s1 = apply_edit(s0, 15, 17, "");
    let ready1 = update_step_must_equal_clean(clean(s0, 1), s0, &s1, 15, 17, "", 2);
    assert_eq!(ready1.owners.records(), 3, "Alpha+Beta merged, two blanks gone");

    // edit2: same-length replace inside Gamma — depends on the retained
    // restart state behind the merge.
    let gamma_at = s1.find("Gamma").unwrap();
    let s2 = apply_edit(&s1, gamma_at, gamma_at + 5, "GAMMA");
    let _ready2 = update_step_must_equal_clean(ready1, &s1, &s2, gamma_at, gamma_at + 5, "GAMMA", 3);
}

/// A same-shape chain where edit1 does NOT touch any support: certificates
/// survive into the retained state and the next edit still agrees with the
/// clean authority at every stage.
#[test]
fn the_next_edit_chain_away_from_support_matches_the_clean_authority() {
    let s0 = "Para zero text.\n\nPara one text.\n\nPara two text.\n\nPara three text.\n";
    let s1 = apply_edit(s0, 6, 10, "TEXT");
    let ready1 = update_step_must_equal_clean(clean(s0, 1), s0, &s1, 6, 10, "TEXT", 2);
    let one_at = s1.find("Para one").unwrap();
    let s2 = apply_edit(&s1, one_at + 5, one_at + 8, "ONE");
    let _ready2 = update_step_must_equal_clean(ready1, &s1, &s2, one_at + 5, one_at + 8, "ONE", 3);
}

/// CJK content: full-state authority equality across a chain over
/// multi-byte text (byte offsets, Owner-relative rebasing, certificates).
#[test]
fn the_cjk_chain_matches_the_clean_authority() {
    let s0 = "中文段落一。\n\n中文段落二。\n\nEmoji 🎉 段落三。\n";
    let edit_at = s0.find("二").unwrap();
    let cut_len = "二".len();
    let s1 = apply_edit(s0, edit_at, edit_at + cut_len, "贰");
    let ready1 =
        update_step_must_equal_clean(clean(s0, 1), s0, &s1, edit_at, edit_at + cut_len, "贰", 2);
    let third_at = s1.find("段落三").unwrap();
    let cut3 = "段落三".len();
    let s2 = apply_edit(&s1, third_at, third_at + cut3, "段落叁");
    let _ready2 =
        update_step_must_equal_clean(ready1, &s1, &s2, third_at, third_at + cut3, "段落叁", 3);
}

/// A local update must leave the retained suffix's certificates
/// structurally untouched (same values, same Owner association), and the
/// defended sentinel must charge zero.
#[test]
fn the_retained_suffix_certificates_are_untouched_by_a_local_update() {
    let s0 = "One para.\n\nTwo para.\n\nThree para.\n\nFour para.\n";
    let pre = clean(s0, 1);
    let pre_certs: Vec<_> = pre
        .owners
        .owners_in_order()
        .iter()
        .map(|o| o.outgoing_restart.clone())
        .collect();
    assert_eq!(pre_certs.len(), 4);
    assert!(pre_certs[0].is_some() && pre_certs[1].is_some() && pre_certs[2].is_some());

    // edit inside the SECOND paragraph: ranks 0..=1 replaced (guard),
    // ranks 2..4 retained.
    let two_at = s0.find("Two").unwrap();
    let s1 = apply_edit(s0, two_at, two_at + 3, "TWO");
    let post_source = src(2, &s1);
    let mut structural = RecordingHorseAStructuralSink::new();
    let updated = crate::update::update_with_structural(
        pre.clone(),
        &src(1, s0),
        &post_source,
        &edit_of(two_at, two_at + 3, "TWO"),
        &mut NoopWorkSink,
        &mut structural,
    )
    .expect("R local update succeeds");

    let post_certs: Vec<_> = updated
        .owners
        .owners_in_order()
        .iter()
        .map(|o| o.outgoing_restart.clone())
        .collect();
    assert_eq!(post_certs.len(), pre_certs.len(), "no Owner count drift");
    // The retained tail (everything after the guard) keeps its certificates.
    assert_eq!(post_certs[2], pre_certs[2]);
    assert_eq!(post_certs[3], pre_certs[3]);
    assert_eq!(
        structural
            .counters()
            .forbidden_unaffected_certificate_writes,
        Observed::known(0),
        "the defended sentinel charges zero"
    );
    // And the whole logical state still equals the clean authority
    // (in-order sequence incl. certificates; AVL shape is derived).
    let authority = clean(&s1, 2);
    let updated_seq: Vec<&Owner> = updated.owners.owners_in_order();
    let authority_seq: Vec<&Owner> = authority.owners.owners_in_order();
    assert_eq!(updated_seq, authority_seq);
    assert_eq!(updated.refs, authority.refs);
}
