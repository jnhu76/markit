//! I5 candidate-walk, certificate-persistence and fact-comparison counter
//! semantics (#59 §20; #60 §9.3; task contract §16/§17/§34): the restart
//! cut is never a candidate, each predicate evaluation charges exactly one
//! candidate check + one cursor visit + one certificate read, each
//! movement to an offered candidate is one cursor advance, transient
//! parser barriers are not certificate writes, and the facts comparison
//! charges what it actually compares.

use crate::candidate::{CandidateWalk, ConvergencePolicy, WalkOutcome};
use crate::i5_tests::primitives::{owner, three_node_sequence};
use crate::state::OwnerSeq;
use crate::structural::{Observed, RecordingHorseAStructuralSink};

fn policy(restart_cut: usize, edit: (usize, usize), delta: i128) -> ConvergencePolicy {
    ConvergencePolicy {
        restart_cut,
        edit_start: edit.0,
        edit_end: edit.1,
        delta,
    }
}

/// Three 20-byte Owners, every boundary certified.
fn certified() -> OwnerSeq {
    three_node_sequence([true, true, true])
}

#[test]
fn the_restart_cut_generates_zero_candidate_checks_and_the_first_offer_one() {
    let owners = certified();
    // Restart at 20: the certified boundary at 20 is never offered. The
    // first offered candidate is 40.
    let mut sink = RecordingHorseAStructuralSink::new();
    let mut walk = CandidateWalk::begin(&owners, 0, policy(20, (0, 0), 0), &mut sink);

    // A barrier at the restart cut: the walk seeks past it (one advance
    // to the next offered candidate) but evaluates nothing.
    let at_restart_cut = walk.on_sealed_barrier(20, true);
    // The first offered candidate: exactly one check / visit / read.
    let at_next_candidate = walk.on_sealed_barrier(40, true);
    drop(walk);

    assert_eq!(at_restart_cut, WalkOutcome::Continue);
    assert!(
        matches!(at_next_candidate, WalkOutcome::Converged(accepted) if accepted.old_cut == 40),
        "expected convergence at 40"
    );
    let c = sink.counters();
    // The restart cut generated no candidate check: one check for the
    // first real offer only.
    assert_eq!(c.candidate_checks, Observed::known(1));
    assert_eq!(c.cursor_advances, Observed::known(1), "only 40 was sought");
    assert_eq!(c.certificate_reads, Observed::known(1));
}

#[test]
fn two_offered_candidates_charge_exactly_two_checks_and_two_advances() {
    let owners = certified();
    let mut sink = RecordingHorseAStructuralSink::new();
    // Candidate 1 (20) is rejected: the deleted range is not behind it yet.
    let mut walk = CandidateWalk::begin(&owners, 0, policy(0, (0, 25), 0), &mut sink);
    let first = walk.on_sealed_barrier(20, true);
    // Candidate 2 (40) converges.
    let second = walk.on_sealed_barrier(40, true);
    drop(walk);

    assert_eq!(first, WalkOutcome::Continue);
    assert!(
        matches!(second, WalkOutcome::Converged(accepted) if accepted.old_cut == 40),
        "expected the second candidate to converge"
    );
    let c = sink.counters();
    assert_eq!(c.candidate_checks, Observed::known(2));
    assert_eq!(
        c.cursor_advances,
        Observed::known(2),
        "one per offered candidate"
    );
    assert_eq!(c.certificate_reads, Observed::known(2));
    assert_eq!(
        c.certificate_writes,
        Observed::Unknown,
        "the walk never writes"
    );
}

// ------------------------------------------------ certificate writes

#[test]
fn a_matched_barrier_is_one_persistent_write_and_a_transient_event_is_none() {
    // 2 fresh Owners, coverage cuts 0/10/20, interior boundary 10 only
    // (last boundary is the EOF and is never certified).
    let mut fresh = vec![owner(10, None), owner(10, None)];
    let cuts = vec![0, 10, 20];
    let mut sink = RecordingHorseAStructuralSink::new();

    // A transient barrier at a NON-boundary cut installs nothing.
    let transient = markit_mdbench_shared_grammar::RootBlankEvent {
        line_start: 3,
        line_lf: 4,
        cut: 4,
        preceding_lf: Some(2),
    };
    crate::certificate::persist_interior_certificates(
        &mut fresh,
        &cuts,
        std::slice::from_ref(&transient),
        false,
        &mut sink,
    )
    .expect("transient evidence never aborts a legal state");
    {
        let c = sink.counters();
        assert_eq!(c.certificate_writes, Observed::Unknown, "no installation");
    }

    // The barrier that certifies the interior boundary at 10 installs one
    // persistent certificate — exactly one write at the persistence seam.
    let certifying = markit_mdbench_shared_grammar::RootBlankEvent {
        line_start: 4,
        line_lf: 5,
        cut: 10,
        preceding_lf: Some(3),
    };
    crate::certificate::persist_interior_certificates(
        &mut fresh,
        &cuts,
        std::slice::from_ref(&certifying),
        false,
        &mut sink,
    )
    .expect("the boundary certifies");
    let c = sink.counters();
    assert_eq!(c.certificate_writes, Observed::known(1));
    assert!(fresh[0].outgoing_restart.is_some());
    assert!(
        fresh[1].outgoing_restart.is_none(),
        "EOF is never certified"
    );
}

// ------------------------------------------------ facts comparison

#[test]
fn the_facts_comparison_charges_only_what_it_actually_compares() {
    use crate::facts::OrderedFacts;

    let facts = |entries: Vec<(String, String)>| {
        let mut defs = markit_mdbench_shared_grammar::RefTable::new();
        defs.extend_from(&entries);
        OrderedFacts::of_fresh_region(&defs, &mut RecordingHorseAStructuralSink::new())
    };

    // Empty vs empty: a real decision with zero compared entries — the
    // frozen witness shape (`facts_compared = Known(0)`).
    let a = facts(vec![]);
    let b = facts(vec![]);
    let mut sink = RecordingHorseAStructuralSink::new();
    assert!(a.eq_with_recording(&b, &mut sink));
    let c = sink.counters();
    assert_eq!(c.facts_compared, Observed::known(0));
    assert_eq!(c.fact_compare_bytes, Observed::known(0));

    // One entry pair: label + destination bytes are charged once.
    let a = facts(vec![("x".to_string(), "/y".to_string())]);
    let b = facts(vec![("x".to_string(), "/z".to_string())]);
    let mut sink = RecordingHorseAStructuralSink::new();
    assert!(!a.eq_with_recording(&b, &mut sink));
    let c = sink.counters();
    assert_eq!(c.facts_compared, Observed::known(1));
    assert_eq!(
        c.fact_compare_bytes,
        Observed::known(3),
        "label + destination"
    );

    // Differing lengths decide without any entry comparison — the
    // comparison site still executed and measures its zero pairs.
    let a = facts(vec![("x".to_string(), "/y".to_string())]);
    let b = facts(vec![]);
    let mut sink = RecordingHorseAStructuralSink::new();
    assert!(!a.eq_with_recording(&b, &mut sink));
    let c = sink.counters();
    assert_eq!(c.facts_compared, Observed::known(0));
}

#[test]
fn the_old_fact_extraction_charges_exactly_the_replacement_owners() {
    let seq = OwnerSeq::bulk_build(
        vec![
            owner(10, None),
            owner(10, None),
            owner(10, None),
            owner(10, None),
        ],
        &mut RecordingHorseAStructuralSink::new(),
    );
    let mut sink = RecordingHorseAStructuralSink::new();
    // Extract Defs(O) for ranks 1..3 — exactly the two middle records.
    let _ = crate::facts::OrderedFacts::of_old_replacement(&seq, 1..3, &mut sink);
    let c = sink.counters();
    assert_eq!(c.old_fact_owner_visits, Observed::known(2));
    // TriviaOnly payloads carry no definition facts.
    assert_eq!(c.old_facts_extracted, Observed::known(0));
    // Cursor work: the fact-range positioning descent plus the successor
    // walk pushes — bounded, never a document-wide traversal.
    let visits = c.fact_range_node_visits.value().expect("charged");
    assert!(
        visits <= 4,
        "no linear enumeration of the retained sequence"
    );
    // Defended site: no unaffected payload was inspected.
    assert_eq!(
        c.forbidden_unaffected_payload_inspections,
        Observed::known(0)
    );
}

#[test]
fn the_fresh_region_facts_charge_their_own_table_entries() {
    use crate::facts::OrderedFacts;
    let mut defs = markit_mdbench_shared_grammar::RefTable::new();
    defs.define("a".to_string(), "/x".to_string());
    defs.define("b".to_string(), "/y".to_string());
    let mut sink = RecordingHorseAStructuralSink::new();
    let facts = OrderedFacts::of_fresh_region(&defs, &mut sink);
    let c = sink.counters();
    assert_eq!(c.new_facts_extracted, Observed::known(2));
    assert_eq!(c.reftable_entries_visited, Observed::known(2));
    assert_eq!(facts.entries().len(), 2);
}

// ------------------------------------------------ cursor attribution

#[test]
fn the_cursor_positions_and_advances_charge_their_callers_counter() {
    let seq = three_node_sequence([false; 3]);
    // Cursor initialization for the candidate walk: the positioning
    // descent is cursor work.
    let mut sink = RecordingHorseAStructuralSink::new();
    let mut cursor = seq.cursor_at_rank(0, crate::structural::StructuralOp::Cursor, &mut sink);
    let positioning = sink.counters().cursor_node_visits.value().expect("charged");
    while cursor
        .next(crate::structural::StructuralOp::Cursor, &mut sink)
        .is_some()
    {}
    let total = sink.counters().cursor_node_visits.value().expect("charged");
    assert!(
        total > positioning,
        "successor-walk stack entries charge visits too"
    );
    assert!(
        total <= 2 * 3 + 2,
        "monotone forward, no revisits, no root seeks"
    );

    // The fact-range lookup attributes its own positioning to
    // fact_range_node_visits and never to cursor_node_visits.
    let mut sink = RecordingHorseAStructuralSink::new();
    let mut cursor = seq.cursor_at_rank(1, crate::structural::StructuralOp::FactRange, &mut sink);
    while cursor
        .next(crate::structural::StructuralOp::FactRange, &mut sink)
        .is_some()
    {}
    let c = sink.counters();
    assert!(c.fact_range_node_visits.is_known());
    assert_eq!(c.cursor_node_visits, Observed::Unknown);
}

#[test]
#[should_panic(expected = "out of range")]
fn locate_preconditions_stay_precondition_violations() {
    let seq = three_node_sequence([false; 3]);
    let mut sink = RecordingHorseAStructuralSink::new();
    let _ = seq.locate_by_byte(61, &mut sink);
}
