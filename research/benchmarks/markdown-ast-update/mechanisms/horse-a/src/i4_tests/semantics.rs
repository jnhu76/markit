//! I4 semantic-preservation branch tests (spec §8–§9; data-model §10; I4
//! task contract §17–§21/§25/§33): the complete ordered replacement facts
//! are compared BEFORE any semantic materialization, and the frozen
//! conservative branch — `facts differ OR preservation unknown → same-target
//! full build` — is pinned for every frozen case class.
//!
//! These tests also pin that the local path compares ONLY the replacement
//! region's facts: a source with definitions retained in the prefix and in
//! the suffix still takes the local path, which a document-global
//! recollection (a forbidden sentinel) could not conclude.

use crate::update::{ReplacementIntervals, RestartSelection, UpdatePath};

use super::{assert_ready_equals_h0, edit, Fixture};

/// The `S4` fixture (see `pipeline::S4` for its byte layout).
const S4: &str = "[a]: /x\n\np1\n\np2\n\np3\n";

#[test]
fn equal_definition_facts_keep_the_local_path() {
    let fixture = Fixture::new(S4);
    // Replace the definition label "a" with "A": the NORMALIZED label is
    // unchanged (`norm_label` ASCII-case-folds), so the ordered facts are
    // exactly equal and the retained RefTable stays the environment.
    let (staged, post) = fixture.staged(&edit(1, 2, "A"));
    assert_eq!(post.as_str(), "[A]: /x\n\np1\n\np2\n\np3\n");

    let record = &staged.record;
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 9,
            new_cut: 9,
            certified_rank: 0
        })
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..9,
            new: 0..9,
            old_ranks: 0..1
        }
    );

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(
        next.refs.entries(),
        &[("a".to_string(), "/x".to_string())],
        "the retained environment keeps the normalized fact"
    );
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_new_definition_forces_the_same_target_full_build() {
    let fixture = Fixture::new(S4);
    // Insert a whole definition block at byte 13: the replacement region
    // converges at the guard Owner's own boundary (13 -> 22) and the facts
    // differ.
    let (staged, post) = fixture.staged(&edit(13, 13, "[b]: /y\n\n"));
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\n[b]: /y\n\np2\n\np3\n");

    let record = &staged.record;
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 9,
            certified_rank: Some(0)
        }
    );
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 13,
            new_cut: 22,
            certified_rank: 1
        }),
        "the guard Owner's own certified boundary is a valid convergence"
    );
    assert!(!record.facts_equal);
    assert_eq!(
        record.path,
        UpdatePath::SameTargetFullBuild,
        "the full branch is the semantic decision, not a failed convergence"
    );

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(
        next.refs.entries(),
        &[
            ("a".to_string(), "/x".to_string()),
            ("b".to_string(), "/y".to_string())
        ]
    );
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_deleted_definition_forces_the_same_target_full_build() {
    let fixture = Fixture::new(S4);
    // Delete the definition block and its trailing blank line ([0,8)).
    let (staged, post) = fixture.staged(&edit(0, 8, ""));
    assert_eq!(post.as_str(), "\np1\n\np2\n\np3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 0);
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 13,
            new_cut: 5,
            certified_rank: 1
        }),
        "the candidate at cut 9 is skipped: the edit touched its certificate support"
    );
    assert!(!record.facts_equal);
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert!(next.refs.is_empty());
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_changed_definition_destination_forces_the_same_target_full_build() {
    let fixture = Fixture::new(S4);
    // Replace the destination "/x" (bytes 5..7) with "/zz".
    let (staged, post) = fixture.staged(&edit(5, 7, "/zz"));
    assert_eq!(post.as_str(), "[a]: /zz\n\np1\n\np2\n\np3\n");

    let record = &staged.record;
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..9,
            new: 0..10,
            old_ranks: 0..1
        }
    );
    assert!(!record.facts_equal);
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(next.refs.entries(), &[("a".to_string(), "/zz".to_string())]);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_shadowed_definition_change_forces_a_conservative_full_build() {
    // "[a]: /x\n\n[a]: /y\n\np\n": both facts are retained and the
    // first-wins lookup already resolves /x, so changing the SHADOWED
    // destination cannot change effective resolution. Horse-A still
    // refuses to prove preservation (W-A2) and rebuilds.
    let fixture = Fixture::new("[a]: /x\n\n[a]: /y\n\np\n");
    let (staged, post) = fixture.staged(&edit(15, 16, "z"));
    assert_eq!(post.as_str(), "[a]: /x\n\n[a]: /z\n\np\n");

    let record = &staged.record;
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 18,
            new_cut: 18,
            certified_rank: 1
        })
    );
    assert!(!record.facts_equal, "the ordered fact sequence changed");
    assert_eq!(
        record.path,
        UpdatePath::SameTargetFullBuild,
        "conservative full build: inequality only means preservation was not proven"
    );

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(
        next.refs.entries(),
        &[
            ("a".to_string(), "/x".to_string()),
            ("a".to_string(), "/z".to_string())
        ]
    );
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn an_unresolved_reference_becoming_resolved_forces_the_full_build() {
    let fixture = Fixture::new("p [x][a]\n\nq\n");
    // Insert the definition the reference use has been waiting for.
    let (staged, post) = fixture.staged(&edit(0, 0, "[a]: /u\n\n"));
    assert_eq!(post.as_str(), "[a]: /u\n\np [x][a]\n\nq\n");

    let record = &staged.record;
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 10,
            new_cut: 19,
            certified_rank: 0
        })
    );
    assert!(!record.facts_equal);
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(next.refs.entries(), &[("a".to_string(), "/u".to_string())]);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_resolved_reference_becoming_unresolved_forces_the_full_build() {
    let fixture = Fixture::new("[a]: /u\n\np [x][a]\n\nq\n");
    // Delete the definition block and its blank line ([0,8)): the
    // reference use is left without a target, and no barrier survives the
    // edit, so the replacement ends at real EOF.
    let (staged, post) = fixture.staged(&edit(0, 8, ""));
    assert_eq!(post.as_str(), "\np [x][a]\n\nq\n");

    let record = &staged.record;
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 19,
            new_cut: 11,
            certified_rank: 1
        }),
        "the definition Owner's own certificate was touched; the reference-use \
         Owner's boundary is the first valid convergence"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..19,
            new: 0..11,
            old_ranks: 0..2
        }
    );
    assert!(!record.facts_equal);
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert!(next.refs.is_empty());
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn removing_a_fence_opener_exposes_a_definition_and_forces_the_full_build() {
    // The definition line is fence BODY while the fence is closed.
    let fixture = Fixture::new("```\n[a]: /x\n```\n\np\n");
    let (staged, post) = fixture.staged(&edit(0, 4, ""));
    assert_eq!(post.as_str(), "[a]: /x\n```\n\np\n");

    let record = &staged.record;
    assert!(
        !record.facts_equal,
        "the same bytes are a definition once the fence no longer hides them"
    );
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(next.refs.entries(), &[("a".to_string(), "/x".to_string())]);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn inserting_a_fence_opener_hides_a_definition_and_forces_the_full_build() {
    let fixture = Fixture::new("[a]: /x\n\np\n");
    let (staged, post) = fixture.staged(&edit(0, 0, "```\n"));
    assert_eq!(post.as_str(), "```\n[a]: /x\n\np\n");

    let record = &staged.record;
    assert!(!record.facts_equal);
    assert_eq!(record.path, UpdatePath::SameTargetFullBuild);

    let next = crate::update::commit(fixture.old, staged);
    assert!(next.refs.is_empty());
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn the_local_path_compares_only_the_replacement_region_facts() {
    // "[a]: /x\n\nq\n\nr\n\n[b]: /y\n\ns\n": the edit is inside "r", so the
    // retained prefix carries definition `a` and the retained suffix
    // carries definition `b`. The replacement region's own facts (empty)
    // are equal on both sides, so the frozen lemma
    // `Defs(O) == Defs(N) ⇒ RefTable_old == RefTable_new` holds and the
    // local path is taken WITHOUT recollecting either retained side.
    let source = "[a]: /x\n\nq\n\nr\n\n[b]: /y\n\ns\n";
    let fixture = Fixture::new(source);
    let (staged, post) = fixture.staged(&edit(12, 12, "Z"));
    assert_eq!(post.as_str(), "[a]: /x\n\nq\n\nZr\n\n[b]: /y\n\ns\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 12);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 9,
            certified_rank: Some(0)
        }
    );
    assert_eq!(
        record.convergence,
        Some(crate::candidate::AcceptedConvergence {
            old_cut: 15,
            new_cut: 16,
            certified_rank: 2
        })
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 9..15,
            new: 9..16,
            old_ranks: 1..3
        },
        "the region is 'q' plus 'r' — no definition is inside it"
    );
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_eq!(
        next.refs.entries(),
        &[
            ("a".to_string(), "/x".to_string()),
            ("b".to_string(), "/y".to_string())
        ],
        "both retained definitions survive in the moved RefTable"
    );
    assert_ready_equals_h0(&next, &post);
}
