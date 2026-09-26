//! I4 pipeline geometry tests (task contract §7–§16/§25/§33): the frozen
//! phase order and its binding rules, pinned on literal sources whose
//! every byte position, coverage cut, restart, convergence and resulting
//! replacement interval is hand-derived.
//!
//! ```text
//! validate association
//! → weighted damage locate (RIGHT affinity)
//! → nearest eligible certified restart predecessor
//! → conservative left guard
//! → forward parse (shared observed parser)
//! → monotone candidate walk: first valid convergence, else real EOF
//! → complete old/new replacement intervals
//! → ordered replacement facts → environment decision
//! → local structural replacement OR same-target full build
//! ```

use markit_mdbench_common::{NoopWorkSink, SourceId};

use crate::candidate::AcceptedConvergence;
use crate::update::{ReplacementIntervals, RestartSelection, UpdatePath};

use super::{assert_ready_equals_h0, edit, Fixture};

/// `S4` — the definition + three-paragraph fixture, the flagship geometry
/// source for the local path:
///
/// ```text
/// byte  0  [   1  a   2  ]   3  :   4  sp  5  /   6  x   7 LF
///       8 LF
///       9  p  10  1  11 LF
///      12 LF
///      13  p  14  2  15 LF
///      16 LF
///      17  p  18  3  19 LF                        L = 20
///
/// TopLevelStart = 0, 9, 13, 17     cuts = 0, 9, 13, 17, 20
/// barriers: line_start 8 -> cut 9, preceding_lf 7
///           line_start 12 -> cut 13, preceding_lf 11
///           line_start 16 -> cut 17, preceding_lf 15
/// ```
pub(crate) const S4: &str = "[a]: /x\n\np1\n\np2\n\np3\n";

/// `SA` — three paragraphs (no definitions):
///
/// ```text
/// 0 a 1 l 2 p 3 h 4 a 5 LF  6 LF  7 b 8 e 9 t 10 a 11 LF 12 LF
/// 13 g 14 a 15 m 16 m 17 a 18 LF                          L = 19
/// cuts = 0, 7, 13, 19        barriers: cut 7 (lf 5), cut 13 (lf 11)
/// ```
pub(crate) const SA: &str = "alpha\n\nbeta\n\ngamma\n";

#[test]
fn a_simple_paragraph_edit_converges_on_the_local_path() {
    let fixture = Fixture::new(SA);
    // Insert "X" inside "alpha": [3,3) -> "X" (delta +1).
    let (staged, post) = fixture.staged(&edit(3, 3, "X"));

    let record = &staged.record;
    assert_eq!(
        record.damage_base, 0,
        "RIGHT affinity locates the first Owner"
    );
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 0,
            certified_rank: None
        },
        "no certified interior boundary before the damage: BOF restarts"
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 7,
            new_cut: 8,
            certified_rank: 0
        }),
        "the first Owner's certified boundary maps exactly onto the new barrier at 8"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..7,
            new: 0..8,
            old_ranks: 0..1
        }
    );
    assert!(record.facts_equal, "no definitions on either side");
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
    assert_eq!(post.as_str(), "alpXha\n\nbeta\n\ngamma\n");
}

#[test]
fn an_edit_at_bof_restarts_at_the_distinguished_bof_authority() {
    let fixture = Fixture::new(S4);
    // A leading blank line: [0,0) -> "\n" (delta +1). The definition line
    // keeps its meaning (a blank line may precede a definition), so the
    // replacement facts stay equal.
    let (staged, post) = fixture.staged(&edit(0, 0, "\n"));
    assert_eq!(post.as_str(), "\n[a]: /x\n\np1\n\np2\n\np3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 0);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 0,
            certified_rank: None
        }
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 9,
            new_cut: 10,
            certified_rank: 0
        })
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..9,
            new: 0..10,
            old_ranks: 0..1
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn an_edit_at_eof_terminates_at_real_eof() {
    let fixture = Fixture::new("a\n\nb\n");
    // Append "Z" at L = 5: [5,5) -> "Z" (delta +1). The append has no
    // convergence: the damaged Owner's outgoing boundary IS the EOF
    // boundary, which is never certified.
    let (staged, post) = fixture.staged(&edit(5, 5, "Z"));
    assert_eq!(post.as_str(), "a\n\nb\nZ");

    let record = &staged.record;
    assert_eq!(
        record.damage_base, 5,
        "a == L_old is the explicit EOF position"
    );
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 3,
            certified_rank: Some(0)
        },
        "the nearest certified boundary strictly before t = L_old"
    );
    assert_eq!(
        record.convergence, None,
        "real EOF is a legal replacement end, not a fallback"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 3..5,
            new: 3..6,
            old_ranks: 1..2
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_paragraph_deletion_merges_blocks_and_converges_at_a_later_boundary() {
    let fixture = Fixture::new(SA);
    // Delete the blank line between "beta" and "gamma": [12,13) -> "".
    // "gamma" now continues the "beta" paragraph.
    let (staged, post) = fixture.staged(&edit(12, 13, ""));
    assert_eq!(post.as_str(), "alpha\n\nbeta\ngamma\n");

    let record = &staged.record;
    assert_eq!(
        record.damage_base, 7,
        "RIGHT affinity: the Owner containing 12"
    );
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 0,
            certified_rank: None
        }
    );
    // Candidate 7 (the guard Owner's own cut) maps to 6 and has no barrier
    // there; candidate 19 (EOF) is not certified. Real EOF ends the
    // replacement.
    assert_eq!(record.convergence, None);
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..19,
            new: 0..18,
            old_ranks: 0..3
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_length_preserving_replacement_converges_and_shifts_nothing() {
    let source = "alpha\n\nbeta\n\ngamma\n\ndelta\n";
    let fixture = Fixture::new(source);
    // Replace "gamma" (bytes 13..18) with "GAMMA": same length, delta 0.
    let (staged, post) = fixture.staged(&edit(13, 18, "GAMMA"));
    assert_eq!(post.as_str(), "alpha\n\nbeta\n\nGAMMA\n\ndelta\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 13);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 7,
            certified_rank: Some(0)
        },
        "the guard Owner is 'beta' (coverage 7..13)"
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 20,
            new_cut: 20,
            certified_rank: 2
        }),
        "the damaged Owner's own certified boundary maps to itself"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 7..20,
            new: 7..20,
            old_ranks: 1..3
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn an_insertion_at_an_owner_boundary_locates_with_right_affinity() {
    let fixture = Fixture::new(SA);
    // Insert "X" exactly at the physical line start of "gamma" (byte 13).
    // RIGHT affinity locates the Owner that STARTS there, so t = 13 and
    // the nearest certified predecessor is 7 — a LEFT view would have
    // produced t = 7 and restarted at BOF.
    let (staged, post) = fixture.staged(&edit(13, 13, "X"));
    assert_eq!(post.as_str(), "alpha\n\nbeta\n\nXgamma\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 13);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 7,
            certified_rank: Some(0)
        }
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 7..19,
            new: 7..20,
            old_ranks: 1..3
        }
    );
    assert_eq!(record.convergence, None);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn the_restart_is_the_nearest_eligible_certified_predecessor() {
    let fixture = Fixture::new(S4);
    // Insert "X" inside "p3" (byte 18): t = 17, and the nearest eligible
    // certified boundary is 13 (rank 2), not 9.
    let (staged, post) = fixture.staged(&edit(18, 18, "X"));
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\np2\n\npX3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 17);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 13,
            certified_rank: Some(1)
        }
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 13..20,
            new: 13..21,
            old_ranks: 2..4
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn the_conservative_left_guard_owner_precedes_the_damaged_owner() {
    let fixture = Fixture::new(S4);
    // Insert "X" inside "p2" (byte 14): t = 13 (the damaged Owner's
    // coverage start) and the replacement begins at 9 — one complete
    // guard block ("p1") earlier. Restarting at t itself would leave the
    // replacement without the guard block the frozen rule requires.
    let (staged, post) = fixture.staged(&edit(14, 14, "X"));
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\npX2\n\np3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 13);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 9,
            certified_rank: Some(0)
        },
        "the boundary immediately preceding the damaged Owner"
    );
    assert!(
        record.intervals.old.start < record.damage_base,
        "the conservative left guard begins strictly left of the damage"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 9..17,
            new: 9..18,
            old_ranks: 1..3
        },
        "the guard Owner (rank 1, coverage 9..13) plus the damaged Owner (rank 2)"
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 17,
            new_cut: 18,
            certified_rank: 2
        }),
        "the guard Owner's own cut (rank 1) was offered and rejected first"
    );
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn the_guarded_restart_finds_a_convergence_the_bare_damage_start_cannot() {
    let source = "a\n\nb\n\nc\n\nd\n";
    let fixture = Fixture::new(source);
    assert_eq!(source.len(), 11, "fixture geometry");
    // Delete "c" (byte 6): the damaged Owner becomes an empty line, so a
    // restart at t = 6 would leave the replacement prefix blockless and
    // coverage would not be splittable there. The guard Owner "b" makes
    // the region coverable, and the walk converges at the damaged
    // Owner's own boundary.
    let (staged, post) = fixture.staged(&edit(6, 7, ""));
    assert_eq!(post.as_str(), "a\n\nb\n\n\n\nd\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 6);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 3,
            certified_rank: Some(0)
        },
        "the guard block 'b' (coverage 3..6) is re-parsed with the damage"
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 9,
            new_cut: 8,
            certified_rank: 2
        })
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 3..9,
            new: 3..8,
            old_ranks: 1..3
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn an_insertion_inside_the_certificate_support_invalidates_that_candidate() {
    let fixture = Fixture::new(S4);
    // Insert "\n" at byte 12 — the LF that establishes the blank line
    // certifying the boundary at 13 (§6 support bytes {11,12}; this is the
    // E24-shaped support-touch case). The candidate at 13 is otherwise
    // fully eligible (edit crossed, exact mapping, covered region) and is
    // still rejected, so the walk converges one boundary later.
    let (staged, post) = fixture.staged(&edit(12, 12, "\n"));
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\n\np2\n\np3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 9);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 0,
            certified_rank: None
        }
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 17,
            new_cut: 18,
            certified_rank: 2
        }),
        "the touched candidate at 13 (mapped 14) is skipped"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..17,
            new: 0..18,
            old_ranks: 0..3
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn multiple_valid_convergence_points_accept_the_first_one() {
    let fixture = Fixture::new("a\n\nb\n\nc\n");
    // Insert "\n" at the physical line start of "b" (byte 3): the new
    // source has barriers at 3, 4 and 7. Candidate 3 maps to 4 and
    // candidate 6 maps to 7 — BOTH are fully valid convergences, and the
    // frozen rule takes the first. There is no cost selector that could
    // prefer the later one.
    let (staged, post) = fixture.staged(&edit(3, 3, "\n"));
    assert_eq!(post.as_str(), "a\n\n\nb\n\nc\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 3);
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: 3,
            new_cut: 4,
            certified_rank: 0
        }),
        "the earliest valid convergence wins; candidate 6 -> 7 is never reached"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..3,
            new: 0..4,
            old_ranks: 0..1
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn without_a_convergence_the_replacement_ends_at_real_eof() {
    let fixture = Fixture::new(S4);
    // Delete the blank line between "p2" and "p3" (byte 16): the last two
    // blocks merge, no barrier survives after the damage, and the
    // replacement runs to real EOF. Real EOF is a legal end — not a
    // failure, a timeout, or a budget fallback.
    let (staged, post) = fixture.staged(&edit(16, 17, ""));
    assert_eq!(post.as_str(), "[a]: /x\n\np1\n\np2\np3\n");

    let record = &staged.record;
    assert_eq!(record.damage_base, 13);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 9,
            certified_rank: Some(0)
        }
    );
    assert_eq!(
        record.convergence, None,
        "real EOF terminates the replacement"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 9..20,
            new: 9..19,
            old_ranks: 1..4
        },
        "the guard Owner plus every block after it, through real L_old"
    );
    assert_eq!(record.intervals.new.end, post.len_bytes(), "L_new");
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn a_blockless_replacement_prefix_is_not_a_legal_coverage_cut() {
    let fixture = Fixture::new("a\n\nb\n");
    // Delete "a" (byte 0): the new source starts with two blank lines
    // before "b". Candidate 3 maps onto the barrier at 2, but the
    // replacement prefix [0,2) would contain no top-level block, so those
    // bytes belong to the first retained block instead — the cut is
    // rejected and the replacement ends at real EOF.
    let (staged, post) = fixture.staged(&edit(0, 1, ""));
    assert_eq!(post.as_str(), "\n\nb\n");
    assert_eq!(post.len_bytes(), 4);

    let record = &staged.record;
    assert_eq!(record.damage_base, 0);
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: 0,
            certified_rank: None
        }
    );
    assert_eq!(
        record.convergence, None,
        "the blockless prefix makes the barrier an illegal coverage cut"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: 0..5,
            new: 0..4,
            old_ranks: 0..2
        }
    );
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}

#[test]
fn two_consecutive_updates_on_the_same_returned_state() {
    // state0 -> state1 -> state2, each step checked against clean H0
    // parsing: the returned state must be immediately usable as the next
    // update's input (continuation readiness, task contract §27).
    let fixture = Fixture::new("a\n\nb\n\nc\n");
    let first = edit(3, 3, "\n");
    let (state1, source1) = fixture.run(&first);
    assert_eq!(source1.as_str(), "a\n\n\nb\n\nc\n");
    assert_ready_equals_h0(&state1, &source1);

    // Second edit: insert "Y" at byte 7, the physical start of "c".
    let second = edit(7, 7, "Y");
    let post2 = second
        .apply(&source1, SourceId(3))
        .expect("the second edit applies");
    assert_eq!(post2.as_str(), "a\n\n\nb\n\nYc\n");

    let staged = crate::update::stage(&state1, &source1, &post2, &second, &mut NoopWorkSink)
        .expect("the second update stages on the returned state");
    assert_eq!(
        staged.record.restart,
        RestartSelection {
            cut: 4,
            certified_rank: Some(0)
        },
        "state1's own certificate at cut 4 (Owner rank 0) is the nearest eligible predecessor"
    );
    assert_eq!(
        staged.record.intervals,
        ReplacementIntervals {
            old: 4..9,
            new: 4..10,
            old_ranks: 1..3
        },
        "the guard Owner 'b' plus the damaged Owner 'c' run to real EOF"
    );
    assert_eq!(staged.record.convergence, None);
    assert_eq!(staged.record.path, UpdatePath::Local);

    let state2 = crate::update::commit(state1, staged);
    assert_ready_equals_h0(&state2, &post2);
}

#[test]
fn a_witness_shaped_geometry_keeps_the_guard_and_damage_replacement_at_two_owners() {
    // The frozen #60 witness SHAPE (spec §17.2), reproduced only as a
    // correctness/geometry conformance check — no counters, no timing, no
    // treatment collection: fixed 128-byte units of 126 content bytes +
    // LF + LF, `zzzzzzzz` inserted at `target*128 + 63`.
    const UNITS: usize = 16;
    let content = "z".repeat(126);
    let mut source = String::new();
    for _ in 0..UNITS {
        source.push_str(&content);
        source.push_str("\n\n");
    }
    let target = UNITS / 2;
    let edit_start = target * 128 + 63;

    let fixture = Fixture::new(&source);
    let (staged, post) = fixture.staged(&edit(edit_start, edit_start, "zzzzzzzz"));

    let record = &staged.record;
    assert_eq!(
        record.damage_base,
        target * 128,
        "RIGHT affinity locates unit `target`"
    );
    assert_eq!(
        record.restart,
        RestartSelection {
            cut: (target - 1) * 128,
            certified_rank: Some(target - 2)
        },
        "the nearest certified cut strictly before t = target*128"
    );
    assert_eq!(
        edit_start - record.restart.cut,
        191,
        "the frozen `edit_start - restart` distance"
    );
    assert_eq!(
        record.convergence,
        Some(AcceptedConvergence {
            old_cut: (target + 1) * 128,
            new_cut: (target + 1) * 128 + 8,
            certified_rank: target
        }),
        "the frozen witness rejects the first candidate (damage not crossed) and \
         accepts the second"
    );
    assert_eq!(
        record.intervals,
        ReplacementIntervals {
            old: (target - 1) * 128..(target + 1) * 128,
            new: (target - 1) * 128..(target + 1) * 128 + 8,
            old_ranks: target - 1..target + 1
        },
        "Δ_old = Δ_new = 2: the guard Owner plus the damaged Owner"
    );
    assert!(record.facts_equal);
    assert_eq!(record.path, UpdatePath::Local);

    let next = crate::update::commit(fixture.old, staged);
    assert_ready_equals_h0(&next, &post);
}
