//! I1 (#62) — the Horse-A parser observation seam, exercised through the
//! crate's PUBLIC API only (no scanner internals).
//!
//! The seam under test:
//!
//! ```text
//! parse_region(src, base, end, sink)              existing unobserved entry
//! parse_region_observed(src, base, end, sink, o)  observed entry, no SpliceHook
//!     RegionObserver::on_top_level_start(TopLevelEvent)
//!     RegionObserver::on_root_blank_barrier(RootBlankEvent) -> ObserverControl
//! RegionOutcome::{RanToEnd, StoppedAtCertifiedCut}
//! WorkSink::record_source_inspection              existing attribution seam
//! ```
//!
//! Every expected offset below is hand-derived from the byte layout of the
//! literal source plus the frozen observation contract — never read back
//! from the implementation under test.

use markit_mdbench_common::work::InspectionEvent;
use markit_mdbench_common::{CounterSink, NoopWorkSink, SourceVersion, WorkCounters};
use markit_mdbench_oracle::normalized::NormalizedDocument;
use markit_mdbench_shared_grammar::{
    finish_document_with_sink, parse_region, parse_region_observed, ObservedRegionParse,
    ObserverControl, RegionObserver, RegionOutcome, RegionParse, RootBlankEvent, TopLevelEvent,
};

/// The frozen no-op observer: ignores every observation, always Continue.
struct NoopObserver;

impl RegionObserver for NoopObserver {
    fn on_top_level_start(&mut self, _ev: TopLevelEvent) {}

    fn on_root_blank_barrier(&mut self, _ev: RootBlankEvent) -> ObserverControl {
        ObserverControl::Continue
    }
}

/// One complete run of a source: block pass + inline pass, with the
/// mechanism-visible outputs (result, attribution event stream, counters).
#[derive(Debug)]
struct ParseRun {
    region: RegionParse,
    outcome: RegionOutcome,
    normalized: NormalizedDocument,
    events: Vec<InspectionEvent>,
    counters: WorkCounters,
}

/// The existing unobserved parse path.
fn unobserved(src: &[u8], base: usize, end: usize) -> ParseRun {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let region = parse_region(src, base, end, &mut sink);
    let normalized = finish_document_with_sink(src, region.blocks.clone(), &region.defs, &mut sink);
    let events = sink.inspections().to_vec();
    sink.finalize_derived();
    drop(sink);
    ParseRun {
        region,
        outcome: RegionOutcome::RanToEnd,
        normalized,
        events,
        counters,
    }
}

/// The observed parse path with the no-op observer installed.
fn observed_noop(src: &[u8], base: usize, end: usize) -> ParseRun {
    let mut observer = NoopObserver;
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let ObservedRegionParse { region, outcome } =
        parse_region_observed(src, base, end, &mut sink, &mut observer);
    let normalized = finish_document_with_sink(src, region.blocks.clone(), &region.defs, &mut sink);
    let events = sink.inspections().to_vec();
    sink.finalize_derived();
    drop(sink);
    ParseRun {
        region,
        outcome,
        normalized,
        events,
        counters,
    }
}

/// The primary I1 contract: with a no-op observer installed, the result,
/// the normalized document, the source-inspection event stream and the
/// work counters are all exactly those of the unobserved parse.
fn assert_noop_observer_is_inert(src: &[u8], base: usize, end: usize) {
    let plain = unobserved(src, base, end);
    let observed = observed_noop(src, base, end);
    assert_eq!(
        plain.region, observed.region,
        "no-op observer changed RegionParse for {src:?}"
    );
    assert_eq!(
        plain.normalized, observed.normalized,
        "no-op observer changed the normalized document for {src:?}"
    );
    assert_eq!(
        plain.events, observed.events,
        "no-op observer changed the source-inspection event stream for {src:?}"
    );
    assert_eq!(
        plain.counters, observed.counters,
        "no-op observer changed the work counters for {src:?}"
    );
    assert_eq!(
        observed.outcome,
        RegionOutcome::RanToEnd,
        "the no-op observer never stops, so the parse ran to the region end"
    );
}

/// Records every observation, always continues (a stop observer is added
/// by the early-stop tests).
#[derive(Default)]
struct Recorder {
    starts: Vec<usize>,
    barriers: Vec<RootBlankEvent>,
}

impl RegionObserver for Recorder {
    fn on_top_level_start(&mut self, ev: TopLevelEvent) {
        self.starts.push(ev.physical_line_start);
    }

    fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl {
        self.barriers.push(ev);
        ObserverControl::Continue
    }
}

/// Run the observed path with a recording observer, discarding attribution.
fn record_observed(src: &[u8], base: usize, end: usize) -> (ObservedRegionParse, Recorder) {
    let mut observer = Recorder::default();
    let mut sink = NoopWorkSink;
    let observed = parse_region_observed(src, base, end, &mut sink, &mut observer);
    (observed, observer)
}

/// The barrier-carrying root blank lines of a source, in issuance order.
fn barriers(src: &[u8]) -> Vec<RootBlankEvent> {
    let (_, rec) = record_observed(src, 0, src.len());
    rec.barriers
}

/// The observed root-level top-level block starts, in issuance order.
fn starts(src: &[u8]) -> Vec<usize> {
    let (_, rec) = record_observed(src, 0, src.len());
    rec.starts
}

/// Semantic span start of every top-level block of the unobserved parse.
fn semantic_starts(src: &[u8]) -> Vec<usize> {
    unobserved(src, 0, src.len())
        .region
        .blocks
        .iter()
        .map(|b| b.start())
        .collect()
}

#[test]
fn a_paragraph_reports_one_start_on_its_first_physical_line_only() {
    // "one\ntwo\n": the second line continues the open paragraph.
    assert_eq!(starts(b"one\ntwo\n"), vec![0]);
}

#[test]
fn a_heading_reports_one_start_at_its_physical_line_start() {
    assert_eq!(starts(b"# t\n"), vec![0]);
}

#[test]
fn leading_spaces_are_part_of_the_physical_start_not_the_semantic_span() {
    // "  # t\n": the heading's semantic span starts at byte 2 (the '#').
    assert_eq!(semantic_starts(b"  # t\n"), vec![2]);
    assert_eq!(starts(b"  # t\n"), vec![0]);

    // "   text\n": the paragraph's semantic span starts at byte 3.
    assert_eq!(semantic_starts(b"   text\n"), vec![3]);
    assert_eq!(starts(b"   text\n"), vec![0]);
}

#[test]
fn a_root_quote_reports_exactly_one_start_for_all_of_its_descendants() {
    // "> q\n> > deep\n": the root quote opens at byte 0; the nested quote
    // and both paragraphs are descendants inside that one Owner.
    assert_eq!(starts(b"> q\n> > deep\n"), vec![0]);
    // A fence and a paragraph inside the root quote are descendants too.
    assert_eq!(starts(b"> ```\n> x\n> ```\n"), vec![0]);
    // So is a reference definition inside the quote.
    assert_eq!(starts(b"> [l]: /u\n"), vec![0]);
}

#[test]
fn a_root_list_reports_one_start_for_the_whole_container_not_per_item() {
    // "- a\n- b\n": the second item is a sibling inside the same root list.
    assert_eq!(starts(b"- a\n- b\n"), vec![0]);
}

#[test]
fn a_root_fence_reports_its_opening_line_and_no_body_or_closer_line() {
    assert_eq!(starts(b"```\nx\n```\n"), vec![0]);
}

#[test]
fn a_root_reference_definition_reports_its_physical_line() {
    assert_eq!(starts(b"[l]: /u\n"), vec![0]);
}

#[test]
fn a_root_block_after_a_closed_container_reports_its_own_line() {
    // "> q\n# h\n": the quote closes on line 2, which then starts a root
    // heading at byte 4.
    assert_eq!(starts(b"> q\n# h\n"), vec![0, 4]);
    // "- a\n\n- b\n": the blank closes the first list, so line 3's list is
    // a NEW root top-level block at byte 5.
    assert_eq!(starts(b"- a\n\n- b\n"), vec![0, 5]);
}

#[test]
fn a_root_blank_line_is_not_a_top_level_start() {
    // "a\n\nb\n": starts at 0 and 3; the blank line at 2 starts nothing
    // but does certify (cut 3).
    assert_eq!(starts(b"a\n\nb\n"), vec![0, 3]);
    assert_eq!(barriers(b"a\n\nb\n").len(), 1);
}

#[test]
fn consecutive_root_blocks_each_report_one_start() {
    // byte layout: "# h\n" 0..3, "\n" 4, "para\n" 5..9, "\n" 10,
    // "> q\n" 11..14, "\n" 15, "```\n" 16..19, "c\n" 20..21,
    // "```\n" 22..25, "\n" 26, "[l]: /d\n" 27..34  (len 35)
    let src = b"# h\n\npara\n\n> q\n\n```\nc\n```\n\n[l]: /d\n";
    assert_eq!(starts(src), vec![0, 5, 11, 16, 27]);
    // Every blank line between them is a root blank barrier whose cut is
    // exactly the next block's physical start.
    let cuts: Vec<usize> = barriers(src).iter().map(|b| b.cut).collect();
    assert_eq!(cuts, vec![5, 11, 16, 27]);
    let preceding: Vec<Option<usize>> = barriers(src).iter().map(|b| b.preceding_lf).collect();
    assert_eq!(preceding, vec![Some(3), Some(9), Some(14), Some(25)]);
}

/// Requests Stop at the barrier whose cut equals `stop_cut`.
struct StopAtCut {
    stop_cut: usize,
    stops_requested: usize,
}

impl RegionObserver for StopAtCut {
    fn on_top_level_start(&mut self, _ev: TopLevelEvent) {}

    fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl {
        if ev.cut == self.stop_cut {
            self.stops_requested += 1;
            ObserverControl::Stop
        } else {
            ObserverControl::Continue
        }
    }
}

struct StopRun {
    observed: ObservedRegionParse,
    events: Vec<InspectionEvent>,
    stops_requested: usize,
}

fn run_stopping_at(src: &[u8], stop_cut: usize) -> StopRun {
    let mut observer = StopAtCut {
        stop_cut,
        stops_requested: 0,
    };
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let observed = parse_region_observed(src, 0, src.len(), &mut sink, &mut observer);
    StopRun {
        observed,
        events: sink.inspections().to_vec(),
        stops_requested: observer.stops_requested,
    }
}

#[test]
fn a_certified_stop_returns_the_cut_and_never_dispatches_the_next_line() {
    let src = b"A\n\nB\n";
    let run = run_stopping_at(src, 3);
    assert_eq!(
        run.observed.outcome,
        RegionOutcome::StoppedAtCertifiedCut { cut: 3 }
    );
    assert_eq!(run.stops_requested, 1);
    // The result is already the parse of the sealed prefix [0, cut). This
    // is also what proves the barrier is issued only AFTER B1's flush and
    // closure: a barrier issued from the pre-B1 state would let a stop cut
    // the paragraph off from its own block list.
    assert_eq!(
        run.observed.region,
        parse_region(src, 0, 3, &mut NoopWorkSink)
    );
    // Line "B" (bytes 3..5) was never dispatched: the per-line inspection
    // stream stops exactly at the cut, and no block starts at or after it.
    assert_eq!(
        run.events,
        vec![(SourceVersion::Post, 0, 2), (SourceVersion::Post, 2, 3)]
    );
    assert!(run.observed.region.blocks.iter().all(|b| b.start() < 3));
}

#[test]
fn an_unterminated_tail_after_the_cut_is_never_eof_closed_into_the_result() {
    let src = b"A\n\n```\nbody\n";
    let run = run_stopping_at(src, 3);
    assert_eq!(
        run.observed.outcome,
        RegionOutcome::StoppedAtCertifiedCut { cut: 3 }
    );
    assert_eq!(
        run.observed.region,
        parse_region(src, 0, 3, &mut NoopWorkSink)
    );
    assert_eq!(
        run.events,
        vec![(SourceVersion::Post, 0, 2), (SourceVersion::Post, 2, 3)]
    );
    assert!(!run.observed.region.fence_open_at_end);
    // The stop really suppressed the tail: the full parse is strictly
    // larger, so an implementation that ran to EOF would fail here.
    assert_ne!(
        run.observed.region.blocks,
        unobserved(src, 0, src.len()).region.blocks
    );
}

#[test]
fn a_certified_stop_at_the_source_end_is_still_reported_as_a_stop() {
    // "A\n\n": the only barrier's cut is the source end. The observer's
    // accepted stop is reported — the region was sealed by the certified
    // cut, not by EOF force-closure.
    let src = b"A\n\n";
    let run = run_stopping_at(src, 3);
    assert_eq!(
        run.observed.outcome,
        RegionOutcome::StoppedAtCertifiedCut { cut: 3 }
    );
    assert_eq!(
        run.observed.region,
        parse_region(src, 0, 3, &mut NoopWorkSink)
    );
}

#[test]
fn a_stop_at_a_later_barrier_parses_through_the_earlier_ones() {
    let src = b"A\n\nB\n\nC\n";
    let run = run_stopping_at(src, 6);
    assert_eq!(
        run.observed.outcome,
        RegionOutcome::StoppedAtCertifiedCut { cut: 6 }
    );
    assert_eq!(run.stops_requested, 1);
    assert_eq!(
        run.observed.region,
        parse_region(src, 0, 6, &mut NoopWorkSink)
    );
    assert_eq!(
        run.events,
        vec![
            (SourceVersion::Post, 0, 2),
            (SourceVersion::Post, 2, 3),
            (SourceVersion::Post, 3, 5),
            (SourceVersion::Post, 5, 6),
        ]
    );
}

#[test]
fn a_stop_request_for_a_cut_that_never_occurs_runs_to_the_region_end() {
    let run = run_stopping_at(b"A\n\nB\n", 99);
    assert_eq!(run.observed.outcome, RegionOutcome::RanToEnd);
    assert_eq!(run.stops_requested, 0);
}

#[test]
fn root_blank_barrier_is_issued_after_the_paragraph_flush_with_exact_offsets() {
    // "a\n\nb\n": the paragraph "a" is flushed by the blank line's B1
    // processing; the blank line is [2,2) with its LF at byte 2, cut 3
    // (the blank line ends there), and the LF that established byte 2 as
    // a physical line start is byte 1.
    assert_eq!(
        barriers(b"a\n\nb\n"),
        vec![RootBlankEvent {
            line_start: 2,
            line_lf: 2,
            cut: 3,
            preceding_lf: Some(1),
        }]
    );
}

#[test]
fn a_blank_at_bof_reports_no_preceding_lf() {
    // "\na\n": the blank line is the first physical line, so no LF
    // precedes it — BOF support.
    assert_eq!(
        barriers(b"\na\n"),
        vec![RootBlankEvent {
            line_start: 0,
            line_lf: 0,
            cut: 1,
            preceding_lf: None,
        }]
    );
}

#[test]
fn consecutive_blanks_each_report_their_own_cut_and_preceding_lf() {
    // "\n\nb\n": two independent root blank lines at [0,0) and [1,1).
    assert_eq!(
        barriers(b"\n\nb\n"),
        vec![
            RootBlankEvent {
                line_start: 0,
                line_lf: 0,
                cut: 1,
                preceding_lf: None,
            },
            RootBlankEvent {
                line_start: 1,
                line_lf: 1,
                cut: 2,
                preceding_lf: Some(0),
            },
        ]
    );
}

#[test]
fn a_trailing_root_blank_reports_the_document_end_as_its_cut() {
    // "ab\n\n": bytes a b LF LF; the blank line is [3,4), its cut is the
    // document end, and support is {2} + [3,4) = {2,3} — the frozen
    // certificate-support example.
    assert_eq!(
        barriers(b"ab\n\n"),
        vec![RootBlankEvent {
            line_start: 3,
            line_lf: 3,
            cut: 4,
            preceding_lf: Some(2),
        }]
    );
}

#[test]
fn blank_looking_fence_body_never_certifies() {
    // The fence body's blank line reaches no B1 dispatch at all.
    assert_eq!(barriers(b"```\n\nx\n```\n"), vec![]);
}

#[test]
fn a_quote_prefixed_blank_with_a_live_quote_never_certifies() {
    // "> a\n>\n> b\n": the blank line carries the quote prefix, so the
    // real grammar keeps the quote live across it.
    assert_eq!(barriers(b"> a\n>\n> b\n"), vec![]);
}

#[test]
fn a_blank_that_closes_a_list_back_to_root_certifies_at_its_real_post_b1_state() {
    // "- a\n\n- b\n": B1 closes the item and the list back to root, so the
    // blank really is a root blank barrier (the frozen grammar's D5
    // tight-only list rule — not a hardcoded list policy).
    assert_eq!(
        barriers(b"- a\n\n- b\n"),
        vec![RootBlankEvent {
            line_start: 4,
            line_lf: 4,
            cut: 5,
            preceding_lf: Some(3),
        }]
    );
}

#[test]
fn spaces_only_tail_without_a_consumed_lf_never_certifies() {
    // "a\n\n   ": the interior blank at [2,3) certifies; the final
    // spaces-only tail has no LF, so it is not an interior blank line.
    assert_eq!(
        barriers(b"a\n\n   "),
        vec![RootBlankEvent {
            line_start: 2,
            line_lf: 2,
            cut: 3,
            preceding_lf: Some(1),
        }]
    );
    // A paragraph tail of spaces, and a spaces-only document: no barrier.
    assert_eq!(barriers(b"para\n  "), vec![]);
    assert_eq!(barriers(b"   "), vec![]);
}

#[test]
fn a_tab_only_line_is_paragraph_content_not_a_blank() {
    // D1: a tab is NOT whitespace, so "\t" continues the paragraph.
    assert_eq!(barriers(b"a\n\t\nb\n"), vec![]);
}

#[test]
fn a_region_whose_base_is_itself_a_blank_does_not_certify_it() {
    // "a\n\n\nb\n" parsed from byte 3: the region base is the second blank
    // line, but this scan never observed the LF that establishes byte 3
    // as a physical line start, so it cannot issue support for it.
    let (_, rec) = record_observed(b"a\n\n\nb\n", 3, 6);
    assert_eq!(rec.barriers, vec![]);
    // The same blank line, reached as an interior line of a full scan,
    // does certify — the suppression is scan-local provenance, not a
    // property of the bytes.
    let full = barriers(b"a\n\n\nb\n");
    assert_eq!(full.len(), 2);
    assert_eq!(full[1].cut, 4);
    assert_eq!(full[1].preceding_lf, Some(2));
}

#[test]
fn observer_that_never_stops_runs_to_the_region_end() {
    let (observed, _) = record_observed(b"a\n\nb\n", 0, 5);
    assert_eq!(observed.outcome, RegionOutcome::RanToEnd);
    // and the parse itself is the ordinary one
    assert_eq!(observed.region, unobserved(b"a\n\nb\n", 0, 5).region);
}

#[test]
fn noop_observer_is_inert_on_a_plain_paragraph() {
    assert_noop_observer_is_inert(b"hello\n", 0, 6);
}

#[test]
fn noop_observer_is_inert_across_representative_case_shapes() {
    let cases: &[&[u8]] = &[
        b"",
        b"\n",
        b"hello\n",
        b"# t\n",
        b"[l]: /u\n",
        b"one\ntwo\nthree\n",
        b"- a\n- b\n  cont\n",
        b"> q\n> r\n> > deep\n",
        b"```rust\nfn x() {}\n\nbody\n```\n",
        b"a\n\nb\n",
        b"# h\n\npara *b* [l](/u)\n\n- l\n\n> q\n\n```\nc\n```\n\n[d]: /d\n",
        b"para *b* [l](/u) [d]\n\n[d]: /d\n",
        b"para\n  ",
        b"> a\n>\n> b\n",
        b"- a\n\n- b\n",
        b"- > ```\n  nested\n  ```\n",
        b"a\n\t\nb\n",
        b"a\r\n\r\nb\r\n",
        b"```\nunclosed\n",
        b"  # indented heading\n   text\n",
        b"[l]: /u\n[l]: /shadowed\n",
    ];
    for src in cases {
        assert_noop_observer_is_inert(src, 0, src.len());
    }
    // Unicode/CJK/emoji: byte offsets stay the one coordinate system.
    let unicode = "# 標題\n\n段落 🎉 *強調* [リンク](/u)\n\n- 項目\n  - 子\n".as_bytes();
    assert_noop_observer_is_inert(unicode, 0, unicode.len());
}

#[test]
fn noop_observer_is_inert_on_interior_regions() {
    // "# h\n\ntext one\n\ntext two\n": line starts are 0, 4, 5, 14, 15.
    let src = b"# h\n\ntext one\n\ntext two\n";
    assert_noop_observer_is_inert(src, 0, src.len());
    assert_noop_observer_is_inert(src, 4, src.len());
    assert_noop_observer_is_inert(src, 5, src.len());
    assert_noop_observer_is_inert(src, 15, src.len());
    // a region that ends before the document does
    assert_noop_observer_is_inert(src, 5, 15);
    // an empty region
    assert_noop_observer_is_inert(src, 15, 15);
}
