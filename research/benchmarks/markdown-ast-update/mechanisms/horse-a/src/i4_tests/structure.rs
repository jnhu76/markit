//! I4 structural-retention and invariant tests (task contract §19/§23/§24/
//! §28/§33): the local path must transfer retained prefix/suffix ownership
//! structurally — no record-by-record reinsertion, no suffix payload
//! rewrite merely because the absolute base shifted, no suffix certificate
//! regeneration — while every AVL/coverage/certificate invariant holds
//! afterwards.
//!
//! "Retained" is proven by allocation identity: the I3 operators only move
//! `Box`es between slots, so a moved node keeps its heap address. The
//! probes below compare raw addresses (never stable production IDs) — the
//! same instrument the I3 suite uses.

use markit_mdbench_common::NoopWorkSink;

use crate::state::{Owner, ReadyDocument};
use crate::update::{ReplacementIntervals, UpdatePath};

use super::{assert_ready_equals_h0, edit, Fixture};

/// The `S4` fixture (see `pipeline::S4` for its byte layout).
const S4: &str = "[a]: /x\n\np1\n\np2\n\np3\n";

/// In-order `(node address, coverage_len)` pairs — retained-node identity.
fn nodes(doc: &ReadyDocument) -> Vec<(usize, usize)> {
    fn walk(node: &crate::state::AvlNode, out: &mut Vec<(usize, usize)>) {
        if let Some(left) = &node.left {
            walk(left, out);
        }
        out.push((
            node as *const crate::state::AvlNode as usize,
            node.owner.coverage_len,
        ));
        if let Some(right) = &node.right {
            walk(right, out);
        }
    }
    let mut out = Vec::new();
    if let Some(root) = &doc.owners.root {
        walk(root, &mut out);
    }
    out
}

/// In-order document-absolute coverage bases (the byte-weight prefix sums).
fn bases(doc: &ReadyDocument) -> Vec<usize> {
    let mut out = Vec::new();
    doc.owners.for_each_in_order(|base, _| out.push(base));
    out
}

/// In-order Owner values (logical state, not identity).
fn owner_values(doc: &ReadyDocument) -> Vec<Owner> {
    doc.owners.owners_in_order().into_iter().cloned().collect()
}

#[test]
fn a_local_replacement_retains_the_prefix_and_suffix_nodes_structurally() {
    let fixture = Fixture::new(S4);
    let before = nodes(&fixture.old);
    assert_eq!(
        before.iter().map(|(_, len)| *len).collect::<Vec<_>>(),
        vec![9, 4, 4, 3],
        "def / p1 / p2 / p3 coverage lengths"
    );

    // Insert "X" inside "p2": the replacement covers the guard Owner (p1)
    // and p2; the definition Owner (prefix) and p3 (suffix) are retained.
    let (staged, _post) = fixture.staged(&edit(14, 14, "X"));
    assert_eq!(
        staged.record.intervals,
        ReplacementIntervals {
            old: 9..17,
            new: 9..18,
            old_ranks: 1..3
        }
    );
    let next = crate::update::commit(fixture.old, staged);
    let after = nodes(&next);

    assert_eq!(
        after.iter().map(|(_, len)| *len).collect::<Vec<_>>(),
        vec![9, 4, 5, 3],
        "def / fresh p1 / fresh pX2 / retained p3"
    );
    // Retained prefix node: same allocation.
    assert_eq!(after[0].0, before[0].0, "the prefix Owner node is retained");
    // Retained suffix node: same allocation.
    assert_eq!(after[3].0, before[3].0, "the suffix Owner node is retained");
    // The retired middle's nodes are gone; the fresh nodes are new
    // allocations (they are built before the retired subtree is dropped, so
    // no address can be recycled).
    assert_ne!(after[1].0, before[1].0, "p1's replacement is a fresh node");
    assert_ne!(after[2].0, before[2].0, "p2's replacement is a fresh node");
    assert!(
        !after.iter().any(|(addr, _)| *addr == before[2].0),
        "the retired p2 node must not survive"
    );
}

#[test]
fn a_retained_suffix_owner_survives_a_base_shift_unchanged() {
    let fixture = Fixture::new(S4);
    let before_values = owner_values(&fixture.old);
    let before_bases = bases(&fixture.old);
    assert_eq!(before_bases, vec![0, 9, 13, 17]);

    let (staged, _post) = fixture.staged(&edit(14, 14, "X"));
    let next = crate::update::commit(fixture.old, staged);
    let after_values = owner_values(&next);
    let after_bases = bases(&next);

    assert_eq!(
        after_bases,
        vec![0, 9, 13, 18],
        "the untouched suffix's absolute base shifts by delta = +1"
    );
    assert_eq!(
        after_values[3], before_values[3],
        "the retained suffix Owner — payload, relative spans, coverage length and \
         certificate state — is preserved state-for-state; only its derived base moved"
    );
    assert_eq!(
        after_values[0], before_values[0],
        "the retained prefix Owner (with its certificate) is unchanged too"
    );
}

#[test]
fn the_ref_table_is_moved_not_rebuilt_on_the_local_path() {
    let fixture = Fixture::new(S4);
    assert!(
        !fixture.old.refs.is_empty(),
        "fixture must carry a definition"
    );
    let before = fixture.old.refs.entries().as_ptr();

    let (staged, _post) = fixture.staged(&edit(14, 14, "X"));
    let next = crate::update::commit(fixture.old, staged);

    assert_eq!(
        next.refs.entries().as_ptr(),
        before,
        "the local path moves the old RefTable exactly once (same buffer, no rebuild)"
    );
    assert_eq!(next.refs.entries(), &[("a".to_string(), "/x".to_string())]);
}

#[test]
fn retained_suffix_certificates_are_inherited_not_regenerated() {
    // Five blocks: the convergence at p2's boundary retains p3 (which
    // carries an interior certificate) and p4.
    let source = "[a]: /x\n\np1\n\np2\n\np3\n\np4\n";
    let fixture = Fixture::new(source);
    let before_values = owner_values(&fixture.old);
    let before_nodes = nodes(&fixture.old);
    assert!(
        before_values[3].outgoing_restart.is_some(),
        "p3 carries an interior certificate in the initial state"
    );

    let (staged, post) = fixture.staged(&edit(14, 14, "X"));
    assert_eq!(
        staged.record.intervals,
        ReplacementIntervals {
            old: 9..17,
            new: 9..18,
            old_ranks: 1..3
        }
    );
    let next = crate::update::commit(fixture.old, staged);
    let after_values = owner_values(&next);
    let after_nodes = nodes(&next);

    assert_eq!(
        after_values[3], before_values[3],
        "the retained suffix Owner keeps its persistent certificate object, \
         Owner-relative and unrewritten"
    );
    assert_eq!(
        after_nodes[3].0, before_nodes[3].0,
        "and it was not rebuilt record-by-record"
    );
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn every_ready_invariant_holds_after_a_local_replacement() {
    let fixture = Fixture::new(S4);
    let (staged, post) = fixture.staged(&edit(14, 14, "X"));
    assert_eq!(staged.record.path, UpdatePath::Local);
    let next = crate::update::commit(fixture.old, staged);

    // The I4 task contract §28 invariant set, checked explicitly.
    crate::validate::validate_coverage(&next).expect("coverage partition");
    crate::validate::validate_full_build_tree(&next.owners).expect("AVL/aggregate invariants");
    crate::validate::validate_certificates(&next).expect("persistent certificate invariants");
    crate::validate::validate_owner_relative_payload(&next).expect("owner-relative coordinates");
    crate::validate::validate_ref_table_projection(&next).expect("RefTable projection");

    assert_eq!(next.owners.records(), 4);
    assert_eq!(next.owners.total_bytes(), post.len_bytes());
    assert_eq!(
        bases(&next).len(),
        4,
        "in-order Owner order survives the splice"
    );
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn the_fresh_replacement_owners_carry_parser_derived_certificates() {
    let fixture = Fixture::new(S4);
    let (staged, post) = fixture.staged(&edit(14, 14, "X"));
    let next = crate::update::commit(fixture.old, staged);
    let values = owner_values(&next);

    // Fresh Owner 1 (p1): the barrier at new cut 13, made Owner-relative to
    // its base 9 → preceding LF at 2, blank line [3,4).
    let cert = values[1]
        .outgoing_restart
        .as_ref()
        .expect("fresh interior boundary certificate");
    assert_eq!(cert.support.preceding_lf, Some(2));
    assert_eq!(cert.support.blank_line, 3..4);

    // Fresh Owner 2 (pX2): the CONVERGENCE barrier at new cut 18, relative
    // to its base 13 → preceding LF at 3, blank line [4,5).
    let cert = values[2]
        .outgoing_restart
        .as_ref()
        .expect("the convergence boundary is certified for the new state");
    assert_eq!(cert.support.preceding_lf, Some(3));
    assert_eq!(cert.support.blank_line, 4..5);

    // The EOF boundary is never certified.
    assert!(values[3].outgoing_restart.is_none());
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn no_budget_selector_keeps_a_large_new_replacement_on_the_local_path() {
    // The replacement grows from 3 bytes to 14 bytes and from 1 to 4
    // Owners, and the parse covers four fresh blocks — a size-thresholded
    // implementation would have chosen the full build.
    let fixture = Fixture::new("a\n\nb\n");
    let (staged, post) = fixture.staged(&edit(3, 3, "X\n\np1\n\np2\n\n"));
    assert_eq!(post.as_str(), "a\n\nX\n\np1\n\np2\n\nb\n");

    let record = &staged.record;
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..3,
            new: 0..14,
            old_ranks: 0..1
        }
    );
    assert!(record.intervals.new.len() > 4 * record.intervals.old.len());
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(next.owners.records(), 5);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn no_budget_selector_keeps_a_large_old_replacement_on_the_local_path() {
    // The mirrored direction: the whole 16-byte, 5-Owner document is
    // removed and replaced by 5 bytes of one Owner, still locally.
    let fixture = Fixture::new("a\n\nX\n\np1\n\np2\n\nb\n");
    let (staged, post) = fixture.staged(&edit(3, 14, ""));
    assert_eq!(post.as_str(), "a\n\nb\n");

    let record = &staged.record;
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..16,
            new: 0..5,
            old_ranks: 0..5
        }
    );
    assert_eq!(record.convergence, None, "the removal ends at real EOF");
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn empty_trivia_and_content_documents_round_trip_through_the_local_path() {
    // empty -> content
    let fixture = Fixture::new("");
    let (staged, post) = fixture.staged(&edit(0, 0, "hello\n"));
    assert_eq!(
        staged.record.intervals,
        ReplacementIntervals {
            old: 0..0,
            new: 0..6,
            old_ranks: 0..0
        }
    );
    assert_eq!(staged.record.path, UpdatePath::Local);
    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(next.owners.records(), 1);
    assert_ready_equals_h0(&next, &post);

    // content -> empty
    let fixture = Fixture::new("hello\n");
    let (staged, post) = fixture.staged(&edit(0, 6, ""));
    assert_eq!(staged.record.path, UpdatePath::Local);
    let next = crate::update::commit(fixture.old, staged);
    assert!(next.owners.is_empty(), "the frozen empty document");
    assert_ready_equals_h0(&next, &post);

    // content -> the frozen TriviaOnly document
    let fixture = Fixture::new("hello\n");
    let (staged, post) = fixture.staged(&edit(0, 6, "\n\n"));
    assert_eq!(
        staged.record.intervals,
        ReplacementIntervals {
            old: 0..6,
            new: 0..2,
            old_ranks: 0..1
        }
    );
    assert_eq!(staged.record.path, UpdatePath::Local);
    let next = crate::update::commit(fixture.old, staged);
    assert!(next.owners.records() == 1);
    assert!(next.owners.owners_in_order()[0].is_trivia_only());
    assert_ready_equals_h0(&next, &post);

    // the frozen TriviaOnly document -> content
    let fixture = Fixture::new("\n\n");
    let (staged, post) = fixture.staged(&edit(0, 0, "hi"));
    assert_eq!(staged.record.path, UpdatePath::Local);
    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_full_branch_commit_installs_the_same_target_state() {
    // The conservative branch returns the complete same-target full build:
    // a normal READY state with the same logical target class, immediately
    // capable of the next update.
    let fixture = Fixture::new(S4);
    let (staged, post) = fixture.staged(&edit(13, 13, "[b]: /y\n\n"));
    assert_eq!(staged.record.path, UpdatePath::SameTargetFullBuild);
    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\n[b]: /y\n\np2\n\np3\n");

    // ... and the returned state really is next-edit capable.
    let follow_up = edit(27, 27, "Q");
    let post2 = follow_up
        .apply(&post, markit_mdbench_common::SourceId(3))
        .expect("edit applies");
    assert_eq!(post2.as_str(), "[a]: /x\n\np1\n\n[b]: /y\n\np2\n\npQ3\n");
    let staged2 = crate::update::stage(&next, &post, &post2, &follow_up, &mut NoopWorkSink)
        .expect("the state after a full branch stages the next update");
    assert_eq!(staged2.record.path, UpdatePath::Local);
    let after = crate::update::commit(next, staged2);
    assert_ready_equals_h0(&after, &post2);
}
