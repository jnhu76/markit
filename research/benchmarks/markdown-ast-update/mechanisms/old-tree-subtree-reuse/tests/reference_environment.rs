//! CORRECTNESS_REGRESSION_FIXTURE — H3 reference-environment closure
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
//! Every fixture asserts `H3 update == H0 clean parse(post)` — the frozen
//! correctness authority — and that the fixture really is the shape it
//! claims (the two clean parses differ in the reference direction, the
//! edit's own bytes are definition-free where the defect is the
//! non-local one, and the documents are long enough that the frozen
//! `MIN_GAP` fragment survives so the retained payload is really taken).

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, Source, WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_old_tree_subtree_reuse::{H3State, OldTreeSubtreeReuseMechanism};
use markit_mdbench_oracle::normalized::{Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;

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

fn h3_full_state(bytes: &[u8]) -> H3State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mechanism = OldTreeSubtreeReuseMechanism::new();
    let pending = mechanism
        .full_parse(&source_of(bytes), &mut cx)
        .expect("full_parse");
    mechanism.complete(pending).expect("complete").state
}

/// Run one H2 update; returns the completed state's projection and the
/// frozen work counters (correctness/parity facts only — no timing).
fn h3_update(
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
    let mechanism = OldTreeSubtreeReuseMechanism::new();
    let state = h3_full_state(pre);
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

    let (result, counters) = h3_update(pre.as_bytes(), post.as_bytes(), start, start + 3, "");
    assert_eq!(
        result,
        reference(post.as_bytes()),
        "H3 update must equal the H0 clean parse of the post source"
    );
    assert!(
        measured(counters.nodes_rebuilt) > 0,
        "the stale payload is re-materialized, not silently accepted"
    );
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H3 has no fallback concept"
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
    let (result, _) = h3_update(broken.as_bytes(), restored.as_bytes(), start, start, "```");
    assert_eq!(result, reference(restored.as_bytes()));
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
    let (result, _) = h3_update(
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
    let (result, _) = h3_update(
        with.as_bytes(),
        without.as_bytes(),
        without.len(),
        with.len(),
        "",
    );
    assert_eq!(result, reference(without.as_bytes()));
}

// ---------------------------------------------------------------------------
// Mechanism fidelity — ordinary edits still exercise fragment reuse
// ---------------------------------------------------------------------------

#[test]
fn safe_local_edits_still_reuse_fragments() {
    // Appending prose changes no definition, so the retained payloads
    // stay valid and are REUSED (nodes_reused > 0): the fix must not turn
    // H3 into a reparser.
    let pre = fence_document(true);
    let post = format!("{pre}{TAIL}\n");
    let (result, counters) = h3_update(
        pre.as_bytes(),
        post.as_bytes(),
        pre.len(),
        pre.len(),
        &format!("{TAIL}\n"),
    );
    assert_eq!(result, reference(post.as_bytes()));
    assert!(
        measured(counters.nodes_reused) > 0,
        "a safe edit must still take retained blocks"
    );
}
