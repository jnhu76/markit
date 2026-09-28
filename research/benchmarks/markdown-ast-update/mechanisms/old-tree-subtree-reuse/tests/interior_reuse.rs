//! H3 damaged-ancestor interior reuse + forward-discovery witnesses —
//! #80 corrective (Gate-A R3 P1-1 / P1-2 repair evidence).
//!
//! These witnesses pin the donor-faithful mechanism shapes the
//! 2026-09-28 Gate-A review found dead or hidden (see
//! `docs/research/h0-h4-donor-fidelity/H3-FIDELITY.md` D3/D5/D11 and
//! review `gate-a-r3-h3-2026-09-28.md`):
//!
//! - DAMAGED-LIST WITNESS — an edit inside an early item of a list
//!   leaves the unaffected later items REUSED (shared `Arc` identity)
//!   INSIDE the damaged container: the changed-flag List ancestor is
//!   rejected as a candidate and the cursor DESCENDS to its unmarked
//!   item children (tree-sitter `ts_parser__reuse_node`: flag
//!   rejection → descend if composite, parser.c:808-811). The donor
//!   C harness recorded 60 interior reuse events inside a damaged
//!   blockquote; the pre-corrective local H3 had 0 operative interior
//!   descent (verified four ways in the Gate-A review).
//! - DAMAGED-QUOTE WITNESS (the decisive F4 shape) — one blockquote
//!   containing several paragraphs; the edit genuinely damages the
//!   QUOTE ANCESTOR (its changed flag is set by the patch); the
//!   ancestor candidate is rejected; descent occurs; the eligible
//!   interior descendants after the damaged paragraph are reused;
//!   result == clean H0.
//! - FORWARD-DISCOVERY WITNESS — candidate discovery is a persistent
//!   forward-only pre-order cursor (reusable_node.h shape), not a
//!   per-consult stateless top-level table rebuild + scan-from-zero.
//!   Asserted STRUCTURALLY via the crate's discovery accounting (no
//!   timing; #80 §21).
//!
//! Every witness also asserts `H3 update result == H0 clean parse`.

use std::sync::Arc;

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, ResultChecksum, Source, WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_old_tree_subtree_reuse::{H3State, OldTreeSubtreeReuseMechanism, TNode};
use markit_mdbench_oracle::normalized::{normalized_checksum, NodeKind};
use markit_mdbench_oracle::{validate_root, NormalizeV1};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn h3_state_of(bytes: &[u8]) -> H3State {
    let mech = OldTreeSubtreeReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

fn h3_update(old: &[u8], post: &[u8], edit: &CanonicalEdit, old_state: H3State) -> H3State {
    let mech = OldTreeSubtreeReuseMechanism::new();
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
    done.state
}

fn node_ids_of_kind(state: &H3State, kind: NodeKind) -> Vec<usize> {
    fn walk(n: &Arc<TNode>, kind: NodeKind, out: &mut Vec<usize>) {
        if n.kind == kind {
            out.push(Arc::as_ptr(n) as usize);
        }
        for (_, c) in &n.children {
            walk(c, kind, out);
        }
    }
    let mut out = Vec::new();
    for entry in &state.tree().entries {
        walk(&entry.node, kind, &mut out);
    }
    out
}

fn assert_matches_h0(state: &H3State, post: &[u8], context: &str) {
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

/// DAMAGED-LIST WITNESS: 12-item tight list, edit inside item 2's
/// text. The List ancestor carries the changed flag; the cursor must
/// descend past it and reuse the unmarked later items at the item
/// level. (The pre-corrective cursor could not take inside a damaged
/// container at all.)
#[test]
fn h3_damaged_list_interior_item_reuse_witness() {
    let items: Vec<String> = (1..=12)
        .map(|i| format!("- item number {i:02}\n"))
        .collect();
    let old: String = format!("intro paragraph\n\n{}", items.concat());
    let old_b = old.as_bytes();
    let at2 = old.find("item number 02").expect("item 2 present");
    let edit = CanonicalEdit::new(at2 + 6, at2 + 6, "XX").expect("edit");
    let post: String = format!("{}XX{}", &old[..at2 + 6], &old[at2 + 6..]);
    let post_b = post.as_bytes();

    let old_state = h3_state_of(old_b);
    let old_item_ids = node_ids_of_kind(&old_state, NodeKind::ListItem);
    assert_eq!(old_item_ids.len(), 12, "the frozen shape has 12 items");

    let new_state = h3_update(old_b, post_b, &edit, old_state);
    assert_matches_h0(&new_state, post_b, "damaged-list interior witness");

    let new_item_ids = node_ids_of_kind(&new_state, NodeKind::ListItem);
    assert_eq!(new_item_ids.len(), 12, "still one 12-item list");
    let shared = new_item_ids
        .iter()
        .filter(|id| old_item_ids.contains(id))
        .count();
    assert!(
        shared >= 6,
        "damaged-list interior reuse missing: only {shared}/12 ListItem identities shared inside the changed-flag List"
    );
    // The damaged item 2 itself must NOT be shared (its bytes changed).
    let old2 = old_item_ids[1];
    assert!(
        !new_item_ids.contains(&old2),
        "the damaged item must be reparsed, not reused"
    );
}

/// DAMAGED-QUOTE WITNESS (the decisive Gate-A F4 shape): ONE blockquote
/// containing five `>`-separated paragraphs; the edit sits inside the
/// second paragraph and therefore genuinely damages the QUOTE ANCESTOR
/// (the patch marks the whole intersecting top-level entry changed).
/// The ancestor candidate is rejected; the cursor descends; the
/// undamaged interior descendants after the damaged paragraph are
/// reused; result == clean H0.
#[test]
fn h3_damaged_quote_interior_reuse_witness() {
    let paras: Vec<String> = (1..=5)
        .map(|i| format!("> quote paragraph number {i} with filler words\n>\n"))
        .collect();
    let tail = "tail para with additional words so nothing degenerates\n";
    let old: String = format!("{}{}", paras.concat(), tail);
    let old_b = old.as_bytes();
    let at2 = old
        .find("quote paragraph number 2")
        .expect("para 2 present");
    let edit = CanonicalEdit::new(at2 + 8, at2 + 8, "YY").expect("edit");
    let post: String = format!("{}YY{}", &old[..at2 + 8], &old[at2 + 8..]);
    let post_b = post.as_bytes();

    let old_state = h3_state_of(old_b);
    // The old tree is ONE blockquote with five paragraph children (the
    // `>` blank lines keep the quote open; D5 tight-only applies to
    // lists at the same level, not to quote-carried blanks).
    let quote_ids = node_ids_of_kind(&old_state, NodeKind::BlockQuote);
    assert_eq!(quote_ids.len(), 1, "one quote ancestor");
    let old_para_ids: Vec<usize> = {
        fn paras_of(n: &Arc<TNode>, out: &mut Vec<usize>) {
            for (_, c) in &n.children {
                if c.kind == NodeKind::Paragraph {
                    out.push(Arc::as_ptr(c) as usize);
                }
            }
        }
        let mut v = Vec::new();
        for entry in &old_state.tree().entries {
            paras_of(&entry.node, &mut v);
        }
        v
    };
    assert_eq!(old_para_ids.len(), 5, "five interior paragraphs");

    let new_state = h3_update(old_b, post_b, &edit, old_state);
    assert_matches_h0(&new_state, post_b, "damaged-quote interior witness");

    let new_para_ids: Vec<usize> = {
        fn paras_of(n: &Arc<TNode>, out: &mut Vec<usize>) {
            for (_, c) in &n.children {
                if c.kind == NodeKind::Paragraph {
                    out.push(Arc::as_ptr(c) as usize);
                }
            }
        }
        let mut v = Vec::new();
        for entry in &new_state.tree().entries {
            paras_of(&entry.node, &mut v);
        }
        v
    };
    let shared = new_para_ids
        .iter()
        .filter(|id| old_para_ids.contains(id))
        .count();
    assert!(
        shared >= 2,
        "damaged-quote interior reuse missing: only {shared}/5 interior paragraphs shared inside the changed-flag Quote"
    );
    // The damaged paragraph 2 itself must NOT be shared.
    let old2 = old_para_ids[1];
    assert!(
        !new_para_ids.contains(&old2),
        "the damaged interior paragraph must be reparsed, not reused"
    );
}

/// FORWARD-DISCOVERY WITNESS (#80 §21, structural — no timing): over
/// one update, discovery is a persistent forward-only pre-order cursor
/// (reusable_node.h shape): total old-tree entry visits are bounded by
/// forward traversal, and no per-consult top-level table rebuild exists
/// (the pre-#80 stateless-enumeration shape charged O(top-level
/// entries) per consult).
#[test]
fn h3_forward_discovery_shape_witness() {
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
    let edit = CanonicalEdit::new(3, 3, "!").expect("edit");
    let post: String = format!("{}!{}", &old[..3], &old[3..]);
    let post_b = post.as_bytes();

    let old_state = h3_state_of(old_b);
    let entry_count = old_state.tree().node_count();
    let mech = OldTreeSubtreeReuseMechanism::new();
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
        assert_matches_h0(&done.state, post_b, "forward-discovery witness");
        mech.discovery_summary()
    };
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
}
