//! H2 mid-container / live-level reuse witnesses — #79 corrective
//! (Gate-A R2 P1-1 / P1-2 repair evidence).
//!
//! These witnesses pin the donor-faithful mechanism shapes that the
//! 2026-09-28 Gate-A review found missing (see
//! `docs/research/h0-h4-donor-fidelity/H2-FIDELITY.md` D3/D5/D11 and
//! review `gate-a-r2-h2-2026-09-28.md`):
//!
//! - MID-CONTAINER WITNESS — an edit inside an early item of a long list
//!   must leave the unaffected later items REUSED (shared `Arc`
//!   identity) inside the damaged container. Donor @lezer/markdown
//!   1.6.3 shares 25/30 ListItem identities on the frozen 30-item
//!   shape; the pre-corrective local H2 shared 0. The exact count is
//!   substrate-dependent (single-edit contract, no NotLast trailing
//!   pop); the witness asserts the performance-relevant MECHANISM:
//!   nested candidates are reachable, nested eligible runs are taken,
//!   and reuse continues inside the damaged container.
//! - NESTED-QUOTE WITNESS — the same shape one level down inside a
//!   blockquote.
//! - INTERRUPTOR-BOUNDARY WITNESS — an undamaged tight list directly
//!   after a damaged paragraph (no blank line) is reused: the donor
//!   reuses it (verified upstream `f_extra.ts`), the pre-corrective
//!   blank-line margin forfeited it.
//! - DISCOVERY-SHAPE WITNESS — candidate discovery is a persistent
//!   forward-only cursor (amortized-forward), not a per-consult
//!   full-table rebuild + scan-from-zero. Asserted STRUCTURALLY via the
//!   crate's discovery accounting (no timing; #79 §13).
//!
//! Every witness also asserts `H2 update result == H0 clean parse`.

use std::sync::Arc;

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, ResultChecksum, Source, WorkCounters,
};
use markit_mdbench_fragment_reuse::{FNode, FragmentReuseMechanism, H2State};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{normalized_checksum, NodeKind};
use markit_mdbench_oracle::{validate_root, NormalizeV1};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn h2_state_of(bytes: &[u8]) -> H2State {
    let mech = FragmentReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

fn h2_update(old: &[u8], post: &[u8], edit: &CanonicalEdit, old_state: H2State) -> H2State {
    let mech = FragmentReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let done = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let old_source = source_of(old, 10);
        let post_source = source_of(post, 11);
        let prepared = mech
            .prepare_update(&old_source, &post_source, edit, &old_state, &mut cx)
            .expect("prepare_update");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        mech.complete(pending).expect("complete")
    };
    let _ = done;
    done.state
}

/// All old-tree `Arc<FNode>` pointers of one kind, in document order —
/// the identity-sharing probe (`Arc::ptr_eq` witness technique).
fn node_ids_of_kind(state: &H2State, kind: NodeKind) -> Vec<usize> {
    fn walk(n: &Arc<FNode>, kind: NodeKind, out: &mut Vec<usize>) {
        if n.kind == kind {
            out.push(Arc::as_ptr(n) as usize);
        }
        for (_, c) in &n.children {
            walk(c, kind, out);
        }
    }
    let mut out = Vec::new();
    for slot in &state.tree().slots {
        walk(&slot.node, kind, &mut out);
    }
    out
}

fn assert_matches_h0(state: &H2State, post: &[u8], context: &str) {
    let clean = parse_document(post);
    assert_eq!(
        state.normalize_v1(),
        clean,
        "{context}: structural mismatch vs H0"
    );
    assert_eq!(
        state.result_checksum(),
        normalized_checksum(&clean),
        "{context}: checksum mismatch vs H0"
    );
    validate_root(&state.normalize_v1().root, Some(post))
        .unwrap_or_else(|e| panic!("{context}: violates NORMALIZED-RESULT-v1: {e}"));
}

/// The frozen Gate-A mid-container shape: a 30-item tight list, edit
/// inside item 3's text. Donor: 25/30 ListItem identities shared.
/// Local witness: the later unaffected items must be identity-shared
/// inside the damaged container (run reuse at the live item level).
#[test]
fn h2_mid_container_item_run_reuse_witness() {
    let items: Vec<String> = (1..=30)
        .map(|i| format!("- item number {i:02}\n"))
        .collect();
    let old: String = format!("intro paragraph\n\n{}", items.concat());
    let old_b = old.as_bytes();
    let at3 = old.find("item number 03").expect("item 3 present");
    let edit = CanonicalEdit::new(at3 + 6, at3 + 6, "XX").expect("edit");
    let post: String = format!("{}XX{}", &old[..at3 + 6], &old[at3 + 6..]);
    let post_b = post.as_bytes();

    let old_state = h2_state_of(old_b);
    let old_item_ids = node_ids_of_kind(&old_state, NodeKind::ListItem);
    assert_eq!(old_item_ids.len(), 30, "the frozen shape has 30 items");

    let new_state = h2_update(old_b, post_b, &edit, old_state);
    assert_matches_h0(&new_state, post_b, "mid-container witness");

    let new_item_ids = node_ids_of_kind(&new_state, NodeKind::ListItem);
    assert_eq!(new_item_ids.len(), 30, "still one 30-item list");
    let shared = new_item_ids
        .iter()
        .filter(|id| old_item_ids.contains(id))
        .count();
    // Donor shares 25/30 (items 4..29; the trailing NotLast pop is a
    // donor guard the local model deliberately declares away). The
    // mechanism claim: the unaffected LATER items are reused inside the
    // damaged container. Items 4..30 = 27 eligible; assert a decisive
    // majority (>= 20) so the witness is about the shape, not an exact
    // donor-count equality the substrate does not promise.
    assert!(
        shared >= 20,
        "mid-container item reuse missing: only {shared}/30 ListItem identities shared (donor: 25/30)"
    );
    // The damaged item 3 itself must NOT be shared (its bytes changed).
    let old3 = old_item_ids[2];
    assert!(
        !new_item_ids.contains(&old3),
        "the damaged item must be reparsed, not reused"
    );
}

/// One level deeper: the damaged container is a list nested inside a
/// blockquote (`> - …`), the reusable run is inside the quote.
#[test]
fn h2_nested_quote_interior_reuse_witness() {
    let items: Vec<String> = (1..=12)
        .map(|i| format!("> - quoted item {i:02}\n"))
        .collect();
    let old: String = format!("before\n\n{}\nafter tail\n", items.concat());
    let old_b = old.as_bytes();
    let at5 = old.find("quoted item 05").expect("item 5 present");
    let edit = CanonicalEdit::new(at5 + 7, at5 + 7, "YY").expect("edit");
    let post: String = format!("{}YY{}", &old[..at5 + 7], &old[at5 + 7..]);
    let post_b = post.as_bytes();

    let old_state = h2_state_of(old_b);
    let old_item_ids = node_ids_of_kind(&old_state, NodeKind::ListItem);
    assert_eq!(old_item_ids.len(), 12);

    let new_state = h2_update(old_b, post_b, &edit, old_state);
    assert_matches_h0(&new_state, post_b, "nested-quote witness");

    let new_item_ids = node_ids_of_kind(&new_state, NodeKind::ListItem);
    let shared = new_item_ids
        .iter()
        .filter(|id| old_item_ids.contains(id))
        .count();
    assert!(
        shared >= 5,
        "nested-quote interior reuse missing: only {shared}/12 ListItem identities shared"
    );
}

/// INTERRUPTOR-BOUNDARY WITNESS: an undamaged tight list DIRECTLY after
/// a damaged paragraph (single LF separation — the list-marker lines
/// interrupt the paragraph). Donor reuses the list (upstream
/// `f_extra.ts`, UPSTREAM-OBSERVED-OUTPUT.txt); the pre-corrective
/// blank-line take margin forfeited exactly this shape. (The tail is
/// padded past minGap so the after-change fragment survives the frozen
/// @lezer/common drop rule — the shape under test is the boundary, not
/// the minGap asymmetry.)
#[test]
fn h2_interruptor_boundary_take_witness() {
    let old = "paragraph one that gets edited here\n- alpha\n- beta\n- gamma\n- delta\n\n\
               tail padding line one to keep the after-change fragment alive past minGap\n\n\
               tail padding line two with more text to be safely above the boundary\n";
    let old_b = old.as_bytes();
    let at = old.find("edited").expect("anchor");
    let edit = CanonicalEdit::new(at, at + 6, "EDITED").expect("edit");
    let post: String = format!("{}EDITED{}", &old[..at], &old[at + 6..]);
    let post_b = post.as_bytes();

    let old_state = h2_state_of(old_b);
    let old_item_ids = node_ids_of_kind(&old_state, NodeKind::ListItem);
    assert_eq!(old_item_ids.len(), 4);

    let new_state = h2_update(old_b, post_b, &edit, old_state);
    assert_matches_h0(&new_state, post_b, "interruptor-boundary witness");

    let new_item_ids = node_ids_of_kind(&new_state, NodeKind::ListItem);
    let shared = new_item_ids
        .iter()
        .filter(|id| old_item_ids.contains(id))
        .count();
    assert!(
        shared >= 3,
        "interruptor-boundary take missing: only {shared}/4 ListItem identities shared (donor reuses the undamaged tight list)"
    );
}

/// DISCOVERY-SHAPE WITNESS (#79 §13, structural — no timing): over one
/// update, discovery is amortized-forward. The crate exposes the
/// discovery accounting through `debug_discovery_summary` (recorded
/// during `update`); the witness asserts the forward-cursor property:
/// the per-consult discovery visit count never depends on the
/// top-level table size (no consult rescans from zero), and the total
/// is bounded by forward traversal, not consultations × entries.
#[test]
fn h2_discovery_shape_forward_cursor_witness() {
    // Many top-level blocks + a container-heavy damaged region: a
    // refusal-heavy regime for the old per-consult full-table scan.
    let mut blocks = Vec::new();
    for i in 0..200 {
        blocks.push(format!("block {i:03} plain text\n\n"));
    }
    let old: String = format!(
        "{}{}",
        blocks.concat(),
        "- one\n- two\n- three\n- four\n- five\n"
    );
    let old_b = old.as_bytes();
    // An edit at the very start: every consult in the long suffix is a
    // fragment-covered discovery decision.
    let edit = CanonicalEdit::new(3, 3, "!").expect("edit");
    let post: String = format!("{}!{}", &old[..3], &old[3..]);
    let post_b = post.as_bytes();

    let old_state = h2_state_of(old_b);
    let entry_count = old_state.tree().node_count();
    let mech = FragmentReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let summary = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let old_source = source_of(old_b, 20);
        let post_source = source_of(post_b, 21);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .expect("prepare_update");
        let pending = mech
            .update(
                &old_source,
                &post_source,
                &edit,
                old_state,
                prepared,
                &mut cx,
            )
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        assert_matches_h0(&done.state, post_b, "discovery-shape witness");
        mech.discovery_summary()
    };
    // FORWARD-TRAVERSAL BOUND (#79 §13): every old-tree entry is
    // visited O(1) times overall (skip/descend/take enumeration are all
    // forward-only), plus O(path-depth) fresh positions per consult.
    // The pre-#79 shape charged O(top-level entries) per consult
    // (consultations x T ~ O(B^2) here); the bound below excludes it
    // decisively.
    assert!(
        summary.total_visits <= 2 * entry_count + 8 * summary.consultations,
        "total discovery visits ({}) must be bounded by forward traversal (entries {}, consults {})",
        summary.total_visits,
        entry_count,
        summary.consultations
    );
    assert_eq!(
        summary.table_rebuilds, 0,
        "no per-consult top-level table rebuild"
    );
    let _ = summary.max_consult_visits; // informational: a taking consult may enumerate its run
}
