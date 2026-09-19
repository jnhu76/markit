//! Inline source-inspection attribution gate (R5-CORRECTIVE-1, MAJOR-2).
//!
//! The H0 reference path must report every inline content segment the
//! shared scanner reads as a `record_source_inspection` EVENT. The union
//! of the derived `unique_source_*` slots cannot distinguish inline work
//! (the block pass already covers the same lines for a full parse), so
//! this gate inspects the raw event stream: for a document with inline
//! content in paragraphs, headings, and container children, every
//! expected content segment must appear verbatim in the event list.
//! Correctness only — no timing, no benchmark data.

use markit_mdbench_common::WorkSink;
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{Node, NodeKind};
use markit_mdbench_shared_grammar as sg;
use sg::parser::Skel;

/// Attribution sink that keeps the RAW inspection events (the common
/// `CounterSink` collapses them into the union; this witness must see
/// the individual events).
#[derive(Debug, Default)]
struct EventSink {
    events: Vec<(u64, u64)>,
}

impl WorkSink for EventSink {
    fn record_source_inspection(&mut self, start_byte: u64, end_byte: u64) {
        if end_byte > start_byte {
            self.events.push((start_byte, end_byte));
        }
    }
}

/// Every inline content segment the materializer would scan for a
/// skeleton forest (mirrors the shared materialization traversal).
fn expected_segments(skels: &[Skel], out: &mut Vec<(usize, usize)>) {
    for sk in skels {
        match sk {
            Skel::Para { segments, .. } => out.extend(segments.iter().copied()),
            Skel::Heading { content, .. } => out.push(*content),
            Skel::Quote { children, .. } => expected_segments(children, out),
            Skel::List { items, .. } => expected_segments(items, out),
            Skel::Item { children, .. } => expected_segments(children, out),
            _ => {}
        }
    }
}

/// Recursively count inline-syntax nodes of one kind in a normalized
/// forest (guards that the document really exercises the inline pass).
fn count_kind(nodes: &[Node], kind: NodeKind) -> usize {
    nodes
        .iter()
        .map(|n| (n.kind == kind) as usize + count_kind(&n.children, kind))
        .sum()
}

const DOC: &str = "alpha *one* and `two` here\n\n> quoted `x` tail\n\n# Head [ref][a]\n\n- item [l](/d) text\n\n[a]: /dest\n";

#[test]
fn inline_inspection_events_are_reported() {
    let src = DOC.as_bytes();

    // The document must genuinely exercise the inline constructs.
    let clean = parse_document(src);
    assert!(count_kind(&clean.root.children, NodeKind::Emphasis) > 0);
    assert!(count_kind(&clean.root.children, NodeKind::CodeSpan) > 0);
    assert!(count_kind(&clean.root.children, NodeKind::Link) > 0);
    assert!(count_kind(&clean.root.children, NodeKind::ReferenceLink) > 0);

    // The expected segments come from a plain block pass (which never
    // runs the inline scanner).
    let mut expected = Vec::new();
    {
        let mut block_only = EventSink::default();
        let rp = sg::parse_region(src, 0, src.len(), &mut block_only);
        expected_segments(&rp.blocks, &mut expected);
    }
    assert!(
        expected.len() >= 4,
        "the witness document must contain several inline segments"
    );

    // The H0 reference path (block pass + materialization) must report
    // EVERY content segment verbatim as one event (one scan_region
    // invocation per segment). Probe B (disabling the inline inspection
    // emission) fails exactly here.
    let mut sink = EventSink::default();
    sg::parse_full(src, &mut sink);

    // EVERY expected content segment must appear verbatim as one event
    // (one scan_region invocation per segment). Probe B (disabling the
    // inline inspection emission) fails exactly here.
    for &(ss, se) in &expected {
        let (ss, se) = (ss as u64, se as u64);
        assert!(
            sink.events.contains(&(ss, se)),
            "inline content segment {ss}..{se} was scanned but NOT reported \
             as a record_source_inspection event (events: {:?})",
            sink.events
        );
    }

    // The block pass events are still there: the union of all events
    // covers the complete document for a clean parse.
    let mut merged: Vec<(u64, u64)> = sink.events.clone();
    merged.sort_unstable();
    let mut union: Vec<(u64, u64)> = Vec::new();
    for &(s, e) in &merged {
        match union.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => union.push((s, e)),
        }
    }
    let covered: u64 = union.iter().map(|&(s, e)| e - s).sum();
    assert_eq!(
        covered,
        src.len() as u64,
        "H0 derived source coverage must remain the full document"
    );
}
