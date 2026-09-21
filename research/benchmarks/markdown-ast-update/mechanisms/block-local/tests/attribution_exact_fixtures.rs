//! H1 exact counter-accounting fixtures (MEASUREMENT-CORRECTIVE-1 §15/§21).
//!
//! Frozen authority: attribution counters report ACTUAL CUMULATIVE work
//! that occurred, including work later discarded by a fallback; one scan
//! of a record is ONE charge. Expected values are derived explicitly in
//! the comments and helpers below — "counter != 0" is not the gate.
//!
//! Fixtures:
//!
//! 1. `h1_metadata_scan_is_charged_exactly_once` — H1-A: the damage scan
//!    over the retained tiling is charged once even when the update ends
//!    in the F1 total fallback (the fallback used to re-charge the same
//!    scan).
//! 2. `h1_fallback_keeps_discarded_region_work` — H1-B: when the region
//!    reparse runs and the F6 fence guard then forces the total fallback,
//!    the discarded region's block units remain in `blocks_reparsed` and
//!    `nodes_rebuilt`, on TOP of the full parse's counts.
//! 3. `h1_fallback_repeated_inspection_effort` — §19: the discarded
//!    region re-reads source the delivered full parse also reads, so the
//!    cumulative `source_bytes_inspected_total` strictly exceeds the
//!    unique POST coverage, with every byte of the region covered at
//!    least twice.
//! 4. `h1_length_shrinking_old_post_inspections` — §18: H1's update path
//!    reads no OLD source bytes; every inspection event is POST, and the
//!    OLD derived slot is the measured zero.

use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, SourceVersion,
    WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::{Node, NodeKind};
use markit_mdbench_oracle::ReferenceOracle;
use markit_mdbench_runner::orchestrate::{build_initial_state, run_update_attributed};
use markit_mdbench_shared_grammar as sg;

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

/// R5 §11.6 counting walk over the H0 reference tree: blocks = block-kind
/// nodes; nodes = every normalized node; Document excluded.
fn count_blocks_and_nodes(root: &Node) -> (u64, u64) {
    const INLINE: [NodeKind; 5] = [
        NodeKind::Text,
        NodeKind::Emphasis,
        NodeKind::CodeSpan,
        NodeKind::Link,
        NodeKind::ReferenceLink,
    ];
    fn walk(n: &Node, blocks: &mut u64, nodes: &mut u64) {
        if n.kind != NodeKind::Document {
            *nodes += 1;
            if !INLINE.contains(&n.kind) {
                *blocks += 1;
            }
        }
        for c in &n.children {
            walk(c, blocks, nodes);
        }
    }
    let mut blocks = 0;
    let mut nodes = 0;
    walk(root, &mut blocks, &mut nodes);
    (blocks, nodes)
}

/// H1's documented region construction (R5 freeze §6): region = [end of
/// the entry before the first affected entry, start of the first
/// unaffected entry after the last affected entry), right edge shifted by
/// `delta` and clamped. Re-derived here from the PUBLIC retained state so
/// the expected counters come from the frozen rules, not from the
/// mechanism under test.
fn region_range(state: &H1State, edit: &CanonicalEdit, post_len: usize) -> (usize, usize) {
    let es = edit.start_byte() as usize;
    let ee = edit.end_byte() as usize;
    let delta =
        edit.inserted_text_len_bytes() as isize - edit.removed_len_bytes() as isize;
    let old_len = state.source_len_bytes();
    let mut first = None;
    let mut last = 0usize;
    for (i, e) in state.entries().iter().enumerate() {
        let (s, en) = e.span();
        if s < ee && en > es {
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    let (rs, re_old) = match first {
        Some(f) => (
            state.entries()[..f]
                .last()
                .map(|e| e.span().1)
                .unwrap_or(0),
            state
                .entries()
                .get(last + 1)
                .map(|e| e.span().0)
                .unwrap_or(old_len),
        ),
        None => (0, old_len),
    };
    let re_new = ((re_old as isize + delta).max(rs as isize) as usize).min(post_len);
    (rs, re_new)
}

/// The discarded region's block-unit count under the shared BENCH-GRAMMAR
/// block scanner — the same semantics H1's region reparse uses.
fn discarded_region_blocks(state: &H1State, post: &[u8], edit: &CanonicalEdit) -> u64 {
    let (rs, re_new) = region_range(state, edit, post.len());
    if rs >= re_new {
        return 0;
    }
    let mut noop = markit_mdbench_common::NoopWorkSink;
    let rp = sg::parser::parse_region(post, rs, re_new, &mut noop);
    markit_mdbench_block_local::count_blocks(&rp.blocks)
}

/// Minimum per-byte POST coverage over `[from, to)` derived from the RAW
/// event stream (how many distinct reports read each byte).
fn min_post_coverage(events: &[(SourceVersion, u64, u64)], from: u64, to: u64) -> u32 {
    assert!(from < to, "degenerate coverage window");
    let mut cov = vec![0u32; (to - from) as usize];
    for (v, s, e) in events {
        if *v != SourceVersion::Post {
            continue;
        }
        let s = (*s).max(from).min(to);
        let e = (*e).max(from).min(to);
        for b in cov[s as usize - from as usize..e as usize - from as usize].iter_mut() {
            *b += 1;
        }
    }
    cov.iter().copied().min().unwrap_or(0)
}

/// Full raw inspection event stream + finalized counters of one update.
fn raw_update(
    mechanism: &BlockLocalMechanism,
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H1State,
) -> (Vec<(SourceVersion, u64, u64)>, WorkCounters) {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let old_source = source_of(old, 1);
    let post_source = source_of(post, 2);
    let prepared = mechanism
        .prepare_update(&old_source, &post_source, edit, &old_state, &mut cx)
        .expect("prepare_update");
    mechanism
        .update(&old_source, &post_source, edit, old_state, prepared, &mut cx)
        .expect("update");
    sink.finalize_derived();
    (sink.inspections().to_vec(), counters.clone())
}

/// Fixture 1 — H1-A: one scan, one charge.
///
/// Document (old, 28 bytes):
///
/// ```text
/// [x]: /y
///
/// para one
///
/// para two
/// ```
///
/// `[x]: /y` IS a reference definition (§9.4: bracket label, colon,
/// spaces, destination), so the retained definitions array is non-empty
/// and H1's F1 definition-presence TOTAL fallback fires on every update.
/// The tiling (blank runs are first-class entries):
///
/// ```text
/// old:  Def[0,7) Blank[7,9) Para[9,17) Blank[17,19) Para[19,27) Blank[27,28) = 6
/// post: Def[0,7) Blank[7,9) Para[9,24) Blank[24,26) Para[26,34) Blank[34,35) = 6
/// ```
///
/// Derivation:
///
/// ```text
/// damage scan         consults the 6 retained entries -> metadata +6,
///                     EXACTLY ONCE (at the scan site in `update`)
/// region parse [9,24) 1 Para skeleton — F1 fires AFTER the region parse,
///                     so its work is real and counted: blocks +1, nodes +1
/// fallback full parse rebuilds the tiling: 6 new entries -> metadata +6
/// metadata_records_touched = 6 + 6 = 12
///     (the pre-corrective code re-charged the damage scan inside the
///      fallback: 6 + 6 + 6 = 18 — exactly what H1-A forbids)
/// blocks_reparsed = 1 (discarded region) + 3 (H0 blocks: Def + 2 paras) = 4
/// nodes_rebuilt   = 1 (discarded skeleton unit) + 5 (H0 nodes: Def +
///                   Para+Text, Para+Text) = 6
/// nodes_reused    = 0 (fallback reuses nothing)
/// ```
#[test]
fn h1_metadata_scan_is_charged_exactly_once() {
    let old: &[u8] = b"[x]: /y\n\npara one\n\npara two\n";
    let start = old.windows(4).position(|w| w == b"para").unwrap();
    let edit = CanonicalEdit::new(start + 5, start + 5, " insert".to_string()).unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();

    let mechanism = BlockLocalMechanism::new();
    let old_state =
        build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    // The fixture really carries a retained definition (F1's trigger).
    assert_eq!(old_state.definitions().len(), 1);
    // Derivation check: the retained tiling really has 6 entries — the
    // 3 blocks (Def, Para, Para) PLUS the 3 first-class blank runs.
    assert_eq!(old_state.entries().len(), 6);
    // The fallback's full parse rebuilds EXACTLY the tiling a clean
    // initial-state build produces — derived on an independent path.
    let post_state =
        build_initial_state(&mechanism, &source_of(post_bytes, 2)).expect("post state");
    assert_eq!(post_state.entries().len(), 6);
    // The discarded region parse (1 Para skeleton) happened before F1;
    // the region maps against the OLD tiling.
    let discarded = discarded_region_blocks(&old_state, post_bytes, &edit);
    assert_eq!(discarded, 1, "derivation check: region [9,24) is one Para");

    let hook = ReferenceOracle::new(parse_document(post_bytes).clone());
    let mut counters = WorkCounters::all_unknown();
    let report = run_update_attributed(
        &mechanism,
        &source_of(old, 1),
        &source_of(post_bytes, 2),
        &edit,
        old_state,
        &mut counters,
        &hook,
    );
    assert_eq!(report.execution_status, markit_mdbench_common::ExecutionStatus::Pass);
    assert_eq!(
        report.correctness_status,
        markit_mdbench_common::CorrectnessStatus::Pass
    );
    // The update really took the F1 fallback, so the fixture proves what
    // it claims.
    assert_eq!(counters.fallback_to_full_count, Observed::Known(1));
    // EXACT: 6 (damage scan, once) + 6 (new tiling records) — not 18.
    assert_eq!(
        counters.metadata_records_touched,
        Observed::Known(12),
        "H1-A: the damage scan must be charged exactly once across a fallback"
    );
    // The discarded region parse (1 Para skeleton) happened before F1.
    let (h0_blocks, h0_nodes) = count_blocks_and_nodes(&parse_document(post_bytes).root);
    assert_eq!(h0_blocks, 3, "derivation check: Def + Para + Para");
    assert_eq!(h0_nodes, 5, "derivation check: + 2 paragraph Text runs");
    assert_eq!(counters.blocks_reparsed, Observed::Known(discarded + h0_blocks));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(discarded + h0_nodes));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// Fixture 2 — H1-B: discarded region work survives a guard fallback.
///
/// Document (old, 31 bytes), no definitions anywhere (F1 cannot fire):
///
/// ```text
/// para one
///
/// para two
///
/// para three
/// ```
///
/// REPLACE "para two" with ` ```py ` (an unclosed fence opener). The
/// region reparse RUNS over [10,15) — 1 fence skeleton, open at the
/// region end — then F6, the forward fence-state guard, fires: the region
/// ends inside an open fence whose extent cannot be bounded within the
/// region (the post document continues past it). Derivation:
///
/// ```text
/// discarded region parse = 1 fence skeleton (derived independently from
///                          the shared scanner over the documented region)
/// delivered full parse   = H0(post, 28 bytes): the unclosed fence runs
///                          to EOF, so "para three" is FENCE BODY, not a
///                          paragraph — 2 blocks (Paragraph + FencedCode),
///                          3 nodes (Paragraph, its Text, FencedCode; a
///                          fence has no inline children)
/// blocks_reparsed        = 1 + 2 = 3
/// nodes_rebuilt          = 1 + 3 = 4   (the discarded attempt is
///                          skeleton-only: it never reached inline
///                          materialization)
/// metadata_records_touched = 6 (damage scan, once) + 3 (new tiling:
///                          Para[0,8) Blank[8,10) Fence[10,28)) = 9
/// nodes_reused           = 0 (fallback reuses nothing)
/// ```
#[test]
fn h1_fallback_keeps_discarded_region_work() {
    let old: &[u8] = b"para one\n\npara two\n\npara three\n";
    let start = old.windows(8).position(|w| w == b"para two").unwrap();
    let edit = CanonicalEdit::new(start, start + 8, "```py".to_string()).unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();

    let mechanism = BlockLocalMechanism::new();
    let old_state =
        build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");

    let (h0_blocks, h0_nodes) = count_blocks_and_nodes(&parse_document(post_bytes).root);
    assert_eq!(h0_blocks, 2, "derivation check: para + unclosed fence to EOF");
    assert_eq!(h0_nodes, 3, "derivation check: Para + Text + Fence");

    // The discarded region's own count, from the frozen rules.
    let discarded = discarded_region_blocks(&old_state, post_bytes, &edit);
    assert_eq!(discarded, 1, "derivation check: the region is one open fence");

    let (_, counters) = raw_update(&mechanism, old, post_bytes, &edit, old_state);
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(1),
        "the fixture must take the F6 guard-driven total fallback"
    );
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(discarded + h0_blocks),
        "H1-B: discarded region blocks + full-parse blocks"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(discarded + h0_nodes),
        "H1-B: discarded skeleton units + full-parse nodes"
    );
    // 6 retained entries consulted once + 3 new tiling records.
    assert_eq!(counters.metadata_records_touched, Observed::Known(9));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// Fixture 3 — §19: repeated inspection effort on a fallback.
///
/// Same F1-firing document as fixture 1. The raw POST event stream, in
/// order, is fully derived by the frozen reporting rules:
///
/// ```text
/// region parse     [9,24)                              15 B (discarded)
/// full-parse lines [0,8) [8,9) [9,25) [25,26) [26,35)  35 B, tiles [0,35)
/// inline scans     [9,24) [26,34)                      23 B (2 paras)
/// ```
///
/// Derivation:
///
/// ```text
/// unique POST coverage = [0,35) = post.len(): the delivered full parse
///     reports every line WITH its terminator, which tiles the whole
///     document; every other read (region, inline segments) lies inside
/// cumulative effort   = 15 + 35 + 23 = 73 bytes reported
/// repetition          = every byte of the discarded region [9,24) is read
///     by BOTH the region parse and the delivered parse (min coverage 2)
/// source_bytes_inspected_total (73) > unique_post_source_bytes (35)
/// ```
#[test]
fn h1_fallback_repeated_inspection_effort() {
    let old: &[u8] = b"[x]: /y\n\npara one\n\npara two\n";
    let start = old.windows(4).position(|w| w == b"para").unwrap();
    let edit = CanonicalEdit::new(start + 5, start + 5, " insert".to_string()).unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();

    let mechanism = BlockLocalMechanism::new();
    let old_state =
        build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    let (events, counters) = raw_update(&mechanism, old, post_bytes, &edit, old_state);

    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(1),
        "the fixture must take the F1 definition-presence fallback"
    );
    // Cumulative effort IS the raw event sum (never a union, never a
    // deduplicated count).
    let raw_total: u64 = events.iter().map(|&(_, s, e)| e - s).sum();
    assert_eq!(counters.source_bytes_inspected_total, Observed::Known(raw_total));
    assert_eq!(raw_total, 73, "derivation check: 15 (region) + 35 (lines) + 23 (inline)");

    let post_union = match counters.unique_post_source_bytes {
        Observed::Known(v) => v,
        other => panic!("post union must be Known, got {other:?}"),
    };
    assert_eq!(
        post_union,
        post_bytes.len() as u64,
        "the delivered full parse reports every line with its terminator: \
         the unique POST coverage is the whole post document"
    );
    // Repetition, precisely located: the discarded region [rs, re_new) is
    // read by the region parse AND by the delivered full parse.
    let (rs, re_new) = region_range(&build_initial_state(&mechanism, &source_of(old, 1))
        .expect("re-derived old state"), &edit, post_bytes.len());
    assert!(
        min_post_coverage(&events, rs as u64, re_new as u64) >= 2,
        "every byte of the discarded region [{rs}, {re_new}) is inspected \
         at least twice"
    );
    assert!(
        raw_total > post_union,
        "H1-B/§19: cumulative effort strictly exceeds unique coverage — \
         the discarded region's reads are repeated by the delivered parse"
    );
    // Unique coverage never counts repetition, and no OLD-version events
    // exist, so the combined quantity equals the POST union.
    assert_eq!(
        counters.unique_source_bytes, counters.unique_post_source_bytes,
        "no OLD-version events: the combined quantity equals the POST union"
    );
}

/// Fixture 4 — §18: length-shrinking edit, OLD/POST coordinate spaces.
///
/// H1's update path never reads the OLD source (the tiling is consulted
/// as state, not bytes), so EVERY event must be POST-version and the OLD
/// derived slot must be the measured zero (not Unknown, not a fabricated
/// union).
#[test]
fn h1_length_shrinking_old_post_inspections() {
    let old: &[u8] = b"para one\n\npara two\n\npara three\n";
    let start = old.windows(8).position(|w| w == b"para two").unwrap();
    let edit = CanonicalEdit::new(start, start + 8, "two".to_string()).unwrap();
    assert!(
        edit.inserted_text_len_bytes() < edit.removed_len_bytes(),
        "length-shrinking edit"
    );
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();
    assert!(post_bytes.len() < old.len());

    let mechanism = BlockLocalMechanism::new();
    let old_state =
        build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    let (events, counters) = raw_update(&mechanism, old, post_bytes, &edit, old_state);

    assert!(
        events.iter().all(|(v, _, _)| *v == SourceVersion::Post),
        "H1's update reads only the POST source; every event is POST"
    );
    assert_eq!(
        counters.unique_old_source_bytes,
        Observed::Known(0),
        "no OLD events -> measured zero"
    );
    let raw_total: u64 = events.iter().map(|&(_, s, e)| e - s).sum();
    assert_eq!(
        counters.source_bytes_inspected_total,
        Observed::Known(raw_total)
    );
    // This update takes the REGIONAL path (no fallback): one fallback
    // would be the measured zero, and reuse is real.
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    assert!(matches!(counters.nodes_reused, Observed::Known(n) if n > 0));
}
