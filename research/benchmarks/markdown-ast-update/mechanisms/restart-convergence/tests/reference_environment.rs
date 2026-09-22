//! CORRECTNESS_REGRESSION_FIXTURE — H4 reference-environment closure
//! (#22 REAL WORKLOAD CORRECTNESS CLOSURE).
//!
//! Level-1 mechanism regression: the SMALLEST readable documents that
//! reproduce the defect class the frozen real workload exposed
//! (`G0-FENCE-CLOSER-REMOVE` / `G0-FENCE-CLOSER-RESTORE` /
//! `G0-REFDEF-RESTORE` / `G0-REFDEF-REMOVE`). These are NOT workload
//! cases — they are explanatory fixtures for the invariant (task
//! §16/§17); the frozen G0 workload is untouched by them, and the frozen
//! regression authority stays the 362 pinned cases replayed in the
//! closure harness.
//!
//! THE INVARIANT UNDER TEST
//!
//! A retained subtree's materialized reference resolution is valid only
//! while the document-global definition environment is unchanged.
//! Unchanged definition BYTES are not sufficient: deleting a fence closer
//! turns the rest of the document into fence content, so definitions
//! inside it leave the table even though not one of their bytes changed.
//! A reused payload resolved against the old table is then stale.
//!
//! Every fixture asserts `H4 update == H0 clean parse(post)` — the frozen
//! correctness authority — and that the fixture really is the shape it
//! claims (the two clean parses differ in the reference direction, and
//! the edit's own bytes are definition-free, so the source-local probe
//! alone cannot see the change).
//!
//! H4's frozen response to definition-changing damage is a RESTART AT
//! ZERO at the next generation (not a fallback — H4 has no degraded mode),
//! so these fixtures assert the restart gauges as well as the result: the
//! retained prefix/suffix is dropped deliberately, and the safe-edit
//! witness below proves the ordinary path still converges and reuses.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_restart_convergence::{H4State, RestartConvergenceMechanism};

/// Opening prose, longer than the frozen `MIN_GAP` (128 bytes), so the
/// left fragment survives and the retained paragraph is really reused.
const PAD: &str = "Opening prose paragraph. It exists so that the document is longer than \
the frozen TreeFragment minGap, which is what makes the retained block \
below genuinely reusable instead of dropped by the fragment table.\n";

/// Trailing prose, so the reference-bearing paragraph is NOT the block
/// adjacent to the edit (the safe windows exclude that one block).
const TAIL: &str = "Trailing prose paragraph. It sits between the reference-bearing \
paragraph and the edit, so the reuse windows do not exclude the block \
under test, and it is long enough to keep every fragment alive.\n";

fn source_of(bytes: &[u8]) -> Source {
    Source::new(
        SourceId(0),
        String::from_utf8(bytes.to_vec()).expect("UTF-8"),
    )
}

fn reference(bytes: &[u8]) -> NormalizedDocument {
    parse_document(bytes)
}

/// Whether the normalized result resolves any reference link.
fn resolves_a_reference(document: &NormalizedDocument) -> bool {
    fn walk(node: &Node) -> bool {
        node.kind == NodeKind::ReferenceLink || node.children.iter().any(walk)
    }
    walk(&document.root)
}

fn h4_full_state(bytes: &[u8]) -> H4State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mechanism = RestartConvergenceMechanism::new();
    let pending = mechanism
        .full_parse(&source_of(bytes), &mut cx)
        .expect("full_parse");
    mechanism.complete(pending).expect("complete").state
}

/// Run one H2 update; returns the completed state's projection and the
/// frozen work counters (correctness/parity facts only — no timing).
fn h4_update(
    pre: &[u8],
    post: &[u8],
    start: usize,
    end: usize,
    inserted: &str,
) -> (NormalizedDocument, WorkCounters) {
    let edit = CanonicalEdit::new(start, end, inserted.to_string()).expect("edit in range");
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mechanism = RestartConvergenceMechanism::new();
    let state = h4_full_state(pre);
    let prepared = mechanism
        .prepare_update(&source_of(pre), &source_of(post), &edit, &state, &mut cx)
        .expect("prepare_update");
    let pending = mechanism
        .update(
            &source_of(pre),
            &source_of(post),
            &edit,
            state,
            prepared,
            &mut cx,
        )
        .expect("update");
    let completed = mechanism.complete(pending).expect("complete");
    (completed.state.normalize_v1(), counters)
}

/// `observed` must be a measured value (not Unknown / NotApplicable).
fn measured(value: Observed<u64>) -> u64 {
    match value {
        Observed::Known(v) => v,
        other => panic!("expected a measured counter, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Fixture A — fence closer REMOVE / RESTORE
// ---------------------------------------------------------------------------

/// `[a]: /a` is a definition ONLY while the fence above it is closed.
const FENCE_OPENER: &str = "```\nx\n";
/// The definition, and the blank line before it, are fence content once
/// the closer is gone.
const FENCE_BODY: &str = "\n[a]: /a\n";

fn fence_document(closed: bool) -> String {
    let full = format!("{PAD}\nSee [the target][a] here.\n\n{FENCE_OPENER}```\n{FENCE_BODY}");
    if closed {
        return full;
    }
    // Built by byte surgery so the pair differs by EXACTLY the three
    // closer bytes the edit removes.
    let start = closer_offset(&full);
    format!("{}{}", &full[..start], &full[start + 3..])
}

/// The closer's byte offset in the closed document.
fn closer_offset(closed: &str) -> usize {
    closed.find("x\n```").expect("closer present") + 2
}

#[test]
fn fence_closer_removal_refreshes_a_reused_payload() {
    let pre = fence_document(true);
    let post = fence_document(false);
    let start = closer_offset(&pre);
    // The fixture really is a reference-resolution flip...
    assert!(resolves_a_reference(&reference(pre.as_bytes())));
    assert!(!resolves_a_reference(&reference(post.as_bytes())));
    // ... and the edit's own three bytes carry no definition marker, so
    // the source-local probe alone cannot see the change.
    assert_eq!(&pre.as_bytes()[start..start + 3], b"```");

    let (result, counters) = h4_update(pre.as_bytes(), post.as_bytes(), start, start + 3, "");
    assert_eq!(
        result,
        reference(post.as_bytes()),
        "H4 update must equal the H0 clean parse of the post source"
    );
    // The definition environment changed, so the retained prefix/suffix
    // semantics are stale: H4's frozen response is the restart at zero.
    assert_eq!(measured(counters.nodes_reused), 0);
    assert_eq!(measured(counters.restart_distance), start as u64);
    assert_eq!(
        measured(counters.convergence_distance),
        post.len() as u64,
        "a restart at zero parses to EOF"
    );
    assert!(
        measured(counters.nodes_rebuilt) > 0,
        "the document is rebuilt from the restart, not reused"
    );
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H4 has no fallback concept: the restart is the frozen response"
    );
}

#[test]
fn fence_closer_restoration_refreshes_a_reused_payload() {
    let broken = fence_document(false);
    let restored = fence_document(true);
    let start = closer_offset(&restored);
    assert!(!resolves_a_reference(&reference(broken.as_bytes())));
    assert!(resolves_a_reference(&reference(restored.as_bytes())));
    // RESTORE direction: the retained payload holds NO reference link
    // (the pre state does not resolve), so a `has_ref` test alone cannot
    // see the change — only the definition-environment comparison can.
    let (result, counters) = h4_update(broken.as_bytes(), restored.as_bytes(), start, start, "```");
    assert_eq!(result, reference(restored.as_bytes()));
    assert_eq!(
        measured(counters.convergence_distance),
        restored.len() as u64,
        "the restored definition changes the environment: restart at zero"
    );
}

// ---------------------------------------------------------------------------
// Fixture B — reference definition RESTORE / REMOVE
// ---------------------------------------------------------------------------

const REF_LINE: &str = "[a]: /a\n";

fn refdef_document(with_definition: bool) -> String {
    let definition = if with_definition { REF_LINE } else { "" };
    format!("{PAD}\nSee [the target][a] here.\n\n{TAIL}\n{definition}")
}

#[test]
fn reference_definition_restore_refreshes_a_reused_payload() {
    let without = refdef_document(false);
    let with = refdef_document(true);
    assert!(!resolves_a_reference(&reference(without.as_bytes())));
    assert!(resolves_a_reference(&reference(with.as_bytes())));
    let (result, _) = h4_update(
        without.as_bytes(),
        with.as_bytes(),
        without.len(),
        without.len(),
        REF_LINE,
    );
    assert_eq!(result, reference(with.as_bytes()));
}

#[test]
fn reference_definition_removal_refreshes_a_reused_payload() {
    let with = refdef_document(true);
    let without = refdef_document(false);
    let (result, _) = h4_update(
        with.as_bytes(),
        without.as_bytes(),
        without.len(),
        with.len(),
        "",
    );
    assert_eq!(result, reference(without.as_bytes()));
}

// ---------------------------------------------------------------------------
// Mechanism fidelity — ordinary edits still exercise restart/convergence
// ---------------------------------------------------------------------------

#[test]
fn safe_local_edits_still_converge_and_reuse() {
    // A local text edit inside the opening prose changes no definition,
    // so the forward pass converges on the mapped old checkpoint and the
    // stable suffix keeps its retained blocks (nodes_reused > 0): the fix
    // must not turn H4 into a reparser.
    let pre = fence_document(true);
    let needle = "Opening prose paragraph.";
    let at = pre.find(needle).expect("prose present");
    let mut post = pre.clone();
    post.replace_range(at..at + needle.len(), "Opening PROSE paragraph.");
    let (result, counters) = h4_update(
        pre.as_bytes(),
        post.as_bytes(),
        at,
        at + needle.len(),
        "Opening PROSE paragraph.",
    );
    assert_eq!(result, reference(post.as_bytes()));
    assert!(
        measured(counters.nodes_reused) > 0,
        "a safe edit must still reuse the retained suffix"
    );
    assert!(
        measured(counters.convergence_distance) < post.len() as u64,
        "a safe edit must still converge before EOF"
    );
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H4 has no fallback concept"
    );
}
