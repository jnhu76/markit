//! H4 exact counter-accounting fixtures (MEASUREMENT-CORRECTIVE-1 §16/§21).
//!
//! Frozen authority: attribution counters report ACTUAL CUMULATIVE work
//! that occurred, including work later discarded by a restart. These
//! fixtures pin the exact arithmetic, with every expected value derived
//! in the comments/helpers below — not merely "counter != 0".
//!
//! Fixtures:
//!
//! 1. `h4_prefix_fresh_region_reused_suffix` — H4-A: retained PREFIX
//!    reuse is counted into `nodes_reused` exactly like the converged
//!    suffix (the old suffix-only accumulator silently omitted it).
//! 2. `h4_discarded_forward_pass_then_restart_at_zero` — H4-B: when the
//!    sound definition-environment check discards the forward pass, the
//!    discarded pass's blocks/nodes/metadata/inspections remain in the
//!    cumulative counters, on TOP of the delivered restart's counters.
//! 3. `h4_length_growing_old_post_inspections` — OLD/POST source-version
//!    provenance on a length-growing edit: the prepare phase inspects
//!    OLD bytes, the reparse inspects POST bytes, and the collector
//!    keeps the two coordinate spaces apart.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{Node, NodeKind};
use markit_mdbench_oracle::ReferenceOracle;
use markit_mdbench_restart_convergence::{H4State, RestartConvergenceMechanism};
use markit_mdbench_runner::orchestrate::{build_initial_state, run_update_attributed};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

/// Exact walk used by every derivation below (R5 §11.6 counting rule):
/// a block parse unit is one structural node; inline-kind normalized
/// nodes add on top. Document and blank bytes contribute nothing.
fn count_blocks_and_nodes(root: &Node) -> (u64, u64) {
    const INLINE: [NodeKind; 5] = [
        NodeKind::Text,
        NodeKind::Emphasis,
        NodeKind::CodeSpan,
        NodeKind::Link,
        NodeKind::ReferenceLink,
    ];
    fn walk(n: &Node, blocks: &mut u64, nodes: &mut u64) {
        // The Document root itself is not a parse product.
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

fn run_attributed(
    old: &[u8],
    post: &[u8],
    edit: CanonicalEdit,
    old_state: H4State,
) -> (
    WorkCounters,
    Vec<(markit_mdbench_common::SourceVersion, u64, u64)>,
) {
    let mut counters = WorkCounters::all_unknown();
    let hook = ReferenceOracle::new(parse_document(post).clone());
    let mechanism = RestartConvergenceMechanism::new();
    let report = run_update_attributed(
        &mechanism,
        &source_of(old, 1),
        &source_of(post, 2),
        &edit,
        old_state,
        &mut counters,
        &hook,
    );
    assert_eq!(
        report.execution_status,
        markit_mdbench_common::ExecutionStatus::Pass
    );
    assert_eq!(
        report.correctness_status,
        markit_mdbench_common::CorrectnessStatus::Pass
    );
    // The event stream below belongs to the sink inside the runner; to
    // read raw events the fixture re-runs the phases by hand when needed
    // (see `raw_inspections`).
    (counters, Vec::new())
}

fn raw_inspections(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H4State,
) -> Vec<(markit_mdbench_common::SourceVersion, u64, u64)> {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = RestartConvergenceMechanism::new();
    let old_source = source_of(old, 1);
    let post_source = source_of(post, 2);
    let prepared = mech
        .prepare_update(&old_source, &post_source, edit, &old_state, &mut cx)
        .expect("prepare_update");
    mech.update(
        &old_source,
        &post_source,
        edit,
        old_state,
        prepared,
        &mut cx,
    )
    .expect("update");
    sink.inspections().to_vec()
}

/// Fixture 1 — H4-A retained-prefix reuse.
///
/// Document (top-level blocks, blank-separated):
///
/// ```text
/// alpha paragraph one
///
/// beta paragraph two
///
/// gamma paragraph three
/// ```
///
/// Same-length REPLACE of `beta` -> `BETA`. The restart selects the
/// checkpoint of block 2 (`beta`); the retained prefix is block 1
/// (`alpha`); the fresh region is the edited `BETA` block; the converged
/// suffix is blocks 3-4 (`gamma`). Derivation:
///
/// ```text
/// h0(post)           = 3 paragraphs, 3 text nodes  => (3 blocks, 3 nodes)
/// fresh region       = the BETA paragraph          => 1 block + 1 text
/// nodes_reused       = prefix(alpha: 1 para + 1 text = 2)
///                      + suffix(gamma: 2)          = 4   [EXACT]
/// blocks_reparsed    = 1 (fresh region only)
/// nodes_rebuilt      = 2 (BETA paragraph + its text)
/// ```
#[test]
fn h4_prefix_fresh_region_reused_suffix() {
    let old = b"alpha paragraph one\n\nbeta paragraph two\n\ngamma paragraph three\n";
    let start = old.windows(4).position(|w| w == b"beta").unwrap();
    let edit = CanonicalEdit::new(start, start + 4, "BETA".to_string()).unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();

    let mechanism = RestartConvergenceMechanism::new();
    let old_state = build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    let (counters, _) = run_attributed(old, post_bytes, edit, old_state);

    // Derived expectation (see the fixture comment):
    let (_h0_blocks, _h0_nodes) = count_blocks_and_nodes(&parse_document(post_bytes).root);
    assert_eq!(_h0_blocks, 3, "derivation check: 3 paragraphs");
    assert_eq!(_h0_nodes, 6, "derivation check: 3 paragraphs + 3 text runs");
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "H4-A: prefix(2) + suffix(2) — prefix reuse must NOT be omitted"
    );
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H4 has no fallback concept"
    );
    // Gauges are measured on every update.
    assert!(matches!(counters.restart_distance, Observed::Known(_)));
    assert!(matches!(counters.convergence_distance, Observed::Known(_)));
}

/// Fixture 2 — H4-B discarded forward pass, then restart-at-zero.
///
/// Document:
///
/// ```text
/// intro paragraph
///
/// ```
/// fenced code
/// ```
///
/// [refdef]: /destination
///
/// [use][refdef]
/// ```
///
/// The edit DELETES THE SECOND FENCE CLOSER. Everything after the opener
/// becomes fence content, so `refdef` leaves the definition table and
/// the reference `[use][refdef]` becomes literal text — WITHOUT a single
/// definition byte changing and without the edit span carrying `]: `.
///
/// Derivation (H4 flow):
///
/// ```text
/// pre-parse probe: damaged subtree has no definition, edited span has
///   no `]: `            => the fast path does NOT fire; the FORWARD
///                          PASS runs (this is the point of the fixture)
/// forward pass from the restart checkpoint: the live parse folds the
///   whole tail into ONE fence block; no old checkpoint line is ever
///   reached, so no convergence take happens
/// assembled table: prefix facts (none) + fresh facts (none) = []
/// retained table     = [("refdef", "/destination")]
/// tables DIFFER      => DISCARD the forward pass, RESTART AT ZERO
///
/// discarded pass: 1 skeleton unit (the big fence)  => blocks_reparsed +1,
///                                                   nodes_rebuilt +1
/// restart: h0(post) = intro paragraph + one fenced code block
///   (2 blocks; the fence's content interval is not a node) => +2 blocks,
///   +2 nodes
/// nodes_reused       = 0 (restart reuses nothing)
/// ```
#[test]
fn h4_discarded_forward_pass_then_restart_at_zero() {
    let old: &[u8] =
        b"intro paragraph\n\n```fence\nfenced code\n```\n\n[refdef]: /destination\n\n[use][refdef]\n";
    // Delete the second fence closer ("```\n" right after "fenced code\n").
    let closer = old.windows(5).position(|w| w == b"```\n\n").unwrap();
    let edit = CanonicalEdit::new(closer, closer + 4, String::new()).unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();

    let mechanism = RestartConvergenceMechanism::new();
    let old_state = build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    // The old state really retained a definition, and the edit sits far
    // from it — the fast path cannot have fired.
    assert_eq!(old_state.defs().len(), 1, "derivation check: one refdef");
    assert!(edit.end_byte() as usize <= closer + 4);

    let (counters, _) = run_attributed(old, post_bytes, edit, old_state);

    // Delivered result: intro paragraph + one folded fence to EOF.
    let (h0_blocks, h0_nodes) = count_blocks_and_nodes(&parse_document(post_bytes).root);
    assert_eq!(h0_blocks, 2, "derivation check: paragraph + folded fence");
    assert_eq!(
        h0_nodes, 3,
        "derivation check: paragraph + its text run + the fence"
    );

    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(3),
        "H4-B: discarded forward pass (1 fence skeleton) + restart (2 blocks)"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(4),
        "H4-B: discarded skeleton (1) + restart rebuild (paragraph, text, fence)"
    );
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "restart-at-zero reuses nothing"
    );
    // A restart happened: the distance gauge is the edit offset, the
    // convergence gauge is the full post length.
    assert_eq!(
        counters.restart_distance,
        Observed::Known(closer as u64),
        "restart distance = restart position (0) .. edit start"
    );
    assert_eq!(
        counters.convergence_distance,
        Observed::Known(post_bytes.len() as u64),
        "restart-at-zero converges only at EOF"
    );
}

/// Fixture 3 — OLD/POST inspection provenance on a LENGTH-GROWING edit
/// (MEASUREMENT-CORRECTIVE-1 §18/§19).
///
/// Same document family as fixture 1; the edit GROWS the middle block by
/// 18 bytes, so OLD and POST offsets after the edit name different bytes.
/// Derivation:
///
/// ```text
/// prepare inspects OLD bytes  : the continuation margin between the
///                               prefix block and the checkpoint line
///                               (>= 1 nonempty event, OLD provenance)
/// update inspects POST bytes  : the probe + the forward reparse
///                               (POST provenance)
/// unique_old_source_bytes     = sum of merged OLD events
/// unique_post_source_bytes    = post.len() (the reparse covers the live
///                               document's full byte set)
/// source_bytes_inspected_total= SUM of every raw event length — events
///                               may overlap/repeat; the total must
///                               equal the raw stream exactly
/// unique spaces are separate  : unique = old-union + post-union; the
///                               collector must NOT union across versions
/// ```
#[test]
fn h4_length_growing_old_post_inspections() {
    let old: &[u8] = b"alpha paragraph one\n\nbeta paragraph two\n\ngamma paragraph three\n";
    let start = old.windows(4).position(|w| w == b"beta").unwrap();
    let edit = CanonicalEdit::new(
        start + 4,
        start + 4,
        " has grown by exactly eighteen bytes".to_string(),
    )
    .unwrap();
    let post = edit
        .apply(&source_of(old, 3), SourceId(3))
        .expect("edit applies");
    let post_bytes = post.as_bytes();
    assert!(post_bytes.len() > old.len(), "length-growing edit");

    let mechanism = RestartConvergenceMechanism::new();
    let old_state = build_initial_state(&mechanism, &source_of(old, 1)).expect("initial state");
    let events = raw_inspections(old, post_bytes, &edit, old_state.clone());

    // Provenance is explicit: BOTH versions appear, and every event is
    // nonempty and in range for its own source.
    let old_events: Vec<&(markit_mdbench_common::SourceVersion, u64, u64)> = events
        .iter()
        .filter(|(v, _, _)| *v == markit_mdbench_common::SourceVersion::Old)
        .collect();
    let post_events: Vec<&(markit_mdbench_common::SourceVersion, u64, u64)> = events
        .iter()
        .filter(|(v, _, _)| *v == markit_mdbench_common::SourceVersion::Post)
        .collect();
    assert!(
        !old_events.is_empty(),
        "prepare must report its OLD-source margin reads"
    );
    assert!(
        !post_events.is_empty(),
        "update must report its POST-source reparse reads"
    );
    for &(_, s, e) in &old_events {
        assert!(e > s && *e <= old.len() as u64, "OLD event in range");
    }
    for &(_, s, e) in &post_events {
        assert!(
            e > s && *e <= post_bytes.len() as u64,
            "POST event in range"
        );
    }

    // Exact derived quantities over the raw event stream.
    let raw_total: u64 = events.iter().map(|&(_, s, e)| e - s).sum();
    let union_in = |mut ranges: Vec<(u64, u64)>| -> u64 {
        ranges.sort_unstable();
        let mut merged: Vec<(u64, u64)> = Vec::new();
        for (s, e) in ranges.drain(..) {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        merged.iter().map(|&(s, e)| e - s).sum()
    };
    let old_union = union_in(
        old_events
            .iter()
            .map(|&&(v, s, e)| {
                let _ = v;
                (s, e)
            })
            .collect(),
    );
    let post_union = union_in(
        post_events
            .iter()
            .map(|&&(v, s, e)| {
                let _ = v;
                (s, e)
            })
            .collect(),
    );

    let (counters, _) = run_attributed(old, post_bytes, edit, old_state);
    assert_eq!(
        counters.source_bytes_inspected_total,
        Observed::Known(raw_total)
    );
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(old_union));
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_union)
    );
    assert_eq!(
        counters.unique_source_bytes,
        Observed::Known(old_union + post_union),
        "combined primary quantity = old-union + post-union (no cross-version union)"
    );
    // Repetition is visible in the effort total, never in the unions:
    // total >= combined, with equality only when nothing was re-read.
    assert!(raw_total >= old_union + post_union);
    // H4 never re-reads the retained prefix: the live reparse starts at
    // the restart checkpoint, so its unique POST coverage is strictly
    // SMALLER than the whole post document (a sub-full coverage witness).
    assert!(
        post_union < post_bytes.len() as u64,
        "retained-prefix bytes must not appear in POST coverage"
    );
}
