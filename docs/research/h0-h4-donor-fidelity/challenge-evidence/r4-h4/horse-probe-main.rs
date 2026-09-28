//! GATE-A R4 challenge-case probe for H4 (markit-mdbench-restart-convergence).
//! Qualitative mechanism observables ONLY — no timing.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_restart_convergence::{H4Prepared, H4State, RestartConvergenceMechanism};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(SourceId(id), String::from_utf8(bytes.to_vec()).expect("UTF-8"))
}

fn full_parse(bytes: &[u8]) -> H4State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = RestartConvergenceMechanism::new();
    let pending = mech.full_parse(&source_of(bytes, 1), &mut cx).expect("full_parse");
    mech.complete(pending).expect("complete").state
}

/// Run one update; print every mechanism observable; return the counters.
fn probe(name: &str, old: &str, es: usize, ee: usize, ins: &str) -> (WorkCounters, H4Prepared, H4State) {
    let old_b = old.as_bytes();
    let post: String = format!("{}{}{}", &old[..es], ins, &old[ee..]);
    let post_b = post.as_bytes();
    let old_state = full_parse(old_b);
    let edit = CanonicalEdit::new(es, ee, ins.to_string()).expect("edit");
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = RestartConvergenceMechanism::new();
    let old_source = source_of(old_b, 40);
    let post_source = source_of(post_b, 41);
    let prepared = mech
        .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
        .expect("prepare");
    let pending = mech
        .update(&old_source, &post_source, &edit, old_state, prepared.clone(), &mut cx)
        .expect("update");
    let done = mech.complete(pending).expect("complete");
    let clean = parse_document(post_b);
    let eq = done.state.normalize_v1() == clean;
    let ck_eq = done.state.result_checksum() == normalized_checksum(&clean);
    cx.sink.finalize_derived();
    println!("== {name} ==");
    println!("  old = {:?}", old);
    println!("  post = {:?}", post);
    println!("  edit = [{}, {}) + {:?}", es, ee, ins);
    println!("  prepared: restart_position={} restart_slot={} damaged_end={} damaged_entries={} damaged_has_def={}",
        prepared.restart_position, prepared.restart_slot, prepared.damaged_end, prepared.damaged_entries, prepared.damaged_has_def);
    println!("  counters: restart_distance={:?} convergence_distance={:?} nodes_reused={:?} nodes_rebuilt={:?} blocks_reparsed={:?} fallback={:?}",
        counters.restart_distance, counters.convergence_distance, counters.nodes_reused, counters.nodes_rebuilt, counters.blocks_reparsed, counters.fallback_to_full_count);
    println!("  result==H0: structural={} checksum={}", eq, ck_eq);
    println!("  new gen={} blocks={}", done.state.generation(), done.state.blocks().len());
    if !eq || !ck_eq {
        println!("  !!! CORRECTNESS DIVERGENCE");
    }
    (counters, prepared, done.state)
}

fn main() {
    // ------------------------------------------------------------------
    // H4-F1: valid nearby restart + valid nearby convergence.
    // Donor expectation: restart at/before damage (W&G shape); convergence
    // at a block boundary beyond damage with state agreement (TS) +
    // blank-line guard (Lezer); suffix reused, not reparsed (W&G
    // suffix-as-input).
    // ------------------------------------------------------------------
    {
        let old = "para zero\n\nalpha one\n\nbeta two\n\ngamma three\n\ndelta four\n";
        let gpos = old.find("gamma").unwrap();
        probe("F1a insert inside gamma", old, gpos + 2, gpos + 2, "UMMA");
        // shrinking edit
        let old2 = "aaa\n\nbbbbbbbb\n\nccc\n\nddd\n";
        let b = old2.find("bbbbbbbb").unwrap();
        probe("F1b replace_shrink inside bbbb", old2, b + 2, b + 6, "xy");
        // growing edit
        probe("F1c replace_grow inside bbbb", old2, b + 2, b + 4, "XXXXXX");
    }

    // ------------------------------------------------------------------
    // H4-F2: restart support damaged -> earlier restart.
    // Donor expectation: no restart on a boundary whose support is
    // damaged; fall back to an earlier/stronger restart (W&G farther-back
    // split analogue; lezer drops the fragment).
    // ------------------------------------------------------------------
    {
        // No blank line between the closed fence and the "after" paragraph;
        // the edit reaches into the "after" line -> the checkpoint at
        // "after" cannot be trusted as a restart; margin must back up.
        let old = "para one\n\n```\ncode\n```\nafter para\n\nlast block\n";
        let apos = old.find("after").unwrap();
        probe("F2a edit reaches boundary line (fence-adjacent)", old, apos, apos + 1, "X");
        // Control: same edit but blank-separated boundary -> no back-up needed.
        let old2 = "para one\n\n```\ncode\n```\n\nafter para\n\nlast block\n";
        let apos2 = old2.find("after").unwrap();
        probe("F2b control: blank-separated boundary", old2, apos2, apos2 + 1, "X");
        // List-continuation boundary: item line follows list block w/o blank.
        let old3 = "- item one\n- item two\n\nplain para\n\ntail\n";
        let ppos = old3.find("plain").unwrap();
        probe("F2c edit into first line of plain (after list)", old3, ppos, ppos + 1, "X");
    }

    // ------------------------------------------------------------------
    // H4-F3: candidate before required damaged region crossed -> reject.
    // Donor expectation: convergence strictly after damaged_end; a
    // candidate mapping to a checkpoint inside the damage is refused
    // (Swift affect-range / TS has_changes / lezer open-edge analogue).
    // ------------------------------------------------------------------
    {
        // Multi-block deletion: damaged_end covers two blocks; convergence
        // must not happen at either damaged block's mapped checkpoint.
        let old = "aaa\n\nbbb\n\nccc\n\nddd\n\neee\n";
        let bpos = old.find("bbb").unwrap();
        let cend = old.find("ddd").unwrap();
        probe("F3a multi-block deletion (bbb+ccc gone)", old, bpos, cend, "");
        // Candidate inside a still-damaged paragraph (insertion of a line
        // into a multi-line paragraph): old checkpoints of the paragraph
        // itself and later blocks map behind the damage end.
        let old2 = "aaa\nbbb\nccc\n\nddd\n\neee\n";
        probe("F3b insert line inside multi-line para", old2, 4, 4, "X\n");
    }

    // ------------------------------------------------------------------
    // H4-F4: no interior convergence -> correct progress / EOF path.
    // Donor expectation: forward parse runs to EOF, result still == H0
    // (Swift continuous degradation / TS normal lexer path).
    // ------------------------------------------------------------------
    {
        let old = "alpha one\n\nbeta two\n\ngamma three\n";
        probe("F4a unclosed fence swallows all", old, 0, 0, "```\n");
        // Definition change -> restart at zero, gen bump.
        let old2 = "see [x] here\n\nalpha one\n\nbeta two\n\n[x]: https://e.com\n";
        let dpos = old2.find("[x]:").unwrap();
        probe("F4b definition deletion", old2, dpos, old2.len(), "");
        // Fence closer removed mid-document: everything after is swallowed;
        // assembled def table changes -> restart at zero.
        let old3 = "para\n\n```\ncode\n```\nafter\n\n[z]: https://z.com\n\ntail\n";
        let close = old3.find("```").map(|i| i + 4).unwrap(); // the closing fence line
        let closer = old3[close..].find('\n').unwrap() + close;
        probe("F4c fence closer removal (defs swallowed)", old3, close, closer, "");
    }

    // ------------------------------------------------------------------
    // H4-F5: state/context incompatibility -> no false convergence.
    // Donor expectation: same mapped position is NOT enough when live
    // parser state differs (TS parse_state/scanner equality; lezer
    // contextHash). Markdown case: live parse inside a container while
    // the mapped old checkpoint is a root-level block start.
    // ------------------------------------------------------------------
    {
        // Insert a list before "next": the live parse is inside
        // [List, Item] at the mapped old checkpoint of "next" (empty key)
        // -> (b) refuses; convergence only later (at "last").
        let old = "para\n\nnext\n\nlast\n";
        let npos = old.find("next").unwrap();
        let (counters, prepared, state) = probe("F5a live container vs root checkpoint", old, npos, npos, "- item\n");
        let post: String = format!("{}{}{}", &old[..npos], "- item\n", &old[npos..]);
        let last_post = post.find("last").unwrap();
        match counters.convergence_distance {
            Observed::Known(d) => {
                let expected = (last_post - prepared.restart_position) as u64;
                println!("  F5a check: convergence at LAST block (dist {d} == expected {expected}): {}",
                    d == expected);
            }
            other => println!("  F5a convergence_distance {:?}", other),
        }
        let _ = state;
        // Quote-depth change: same text, different quote nesting.
        let old2 = "para\n\n> quoted\n\nnext\n\nlast\n";
        let qpos = old2.find("> quoted").unwrap();
        probe("F5b quote marker doubled (context deepens)", old2, qpos, qpos + 1, "> >");

        // DIRECT (b) witness: insert "> " before bbb. In the post source
        // "ccc" is a lazy continuation of the quoted paragraph, so the
        // live consult at the ccc line start carries key [Quote] while
        // its mapped old checkpoint (root, empty key) differs. (b) must
        // refuse; convergence only at ddd after the blank closes the quote.
        let old3 = "aaa\n\nbbb\n\nccc\n\nddd\n";
        let b3 = old3.find("bbb").unwrap();
        let (c3, prep3, _st3) = probe("F5c quote prefix -> lazy continuation at mapped checkpoint", old3, b3, b3, "> ");
        let post3: String = format!("{}{}{}", &old3[..b3], "> ", &old3[b3..]);
        let ddd3 = post3.find("ddd").unwrap();
        let expected_d3 = (ddd3 - prep3.restart_position) as u64;
        println!("  F5c check: convergence_distance={:?}; expected if take only at DDD: {}; ccc-mapped candidate refused by (b)",
            c3.convergence_distance, expected_d3);

        // DIRECT (b) witness attempt: insert "- z\n\n  cont\n" immediately
        // before the old top-level block "- x". At the mapped line start of
        // old "- x", the live parse still holds [List, Item] frames (the
        // indented continuation kept the item open), while the old
        // checkpoint's entry key is the root (empty). (b) must refuse;
        // without it the suffix would be spliced INSIDE the open item.
        let old4 = "aaa\n\n- x\n\ntail\n";
        let ins4 = "- z\n\n  cont\n";
        let x4 = old4.find("- x").unwrap();
        let post4: String = format!("{}{}{}", &old4[..x4], ins4, &old4[x4..]);
        let new_x = x4 + ins4.len();
        let (c4, p4, _s4) = probe("F5d live [List,Item] frames at mapped root checkpoint", old4, x4, x4, ins4);
        println!("  F5d: mapped candidate pos={} (q={} == old '- x' checkpoint {}); convergence_distance={:?}",
            new_x, new_x - ins4.len(), x4, c4.convergence_distance);
        let _ = (p4, &post4);

        // F5e: turn the single item line into a multi-line item by replacing
        // " y" with "\n  cont" — the item (and list) remain open at the
        // mapped old "tail" checkpoint; (b) must be the refusing clause
        // (pos > ee_new, q is a checkpoint, keys differ).
        let old5 = "para\n\n- x y\n\ntail\n";
        let probe5 = old5.find(" y").unwrap();
        let (c5, p5, _s5) = probe("F5e live item open at mapped tail checkpoint", old5, probe5, probe5 + 2, "\n  cont");
        println!("  F5e: restart={} damaged_end={}; convergence_distance={:?} (post.len() would mean EOF/no take); nodes_reused={:?}",
            p5.restart_position, p5.damaged_end, c5.convergence_distance, c5.nodes_reused);

        // F5f: loose-list continuation. Insert "- i\n\n  " before bbb: the
        // item stays open across the blank (indented continuation), so the
        // live consult at the blank line BEFORE "  bbb" maps to old bbb's
        // checkpoint with frames [List, Item] live. (b) must refuse there
        // (an unguarded take would splice root blocks inside the item).
        let old6 = "aaa\n\nbbb\n\ntail\n";
        let (c6, p6, _s6) = probe("F5f loose list: item open across blank at mapped checkpoint", old6, 5, 5, "- i\n\n  ");
        println!("  F5f: restart={} damaged_entries={}; convergence_distance={:?}; nodes_reused={:?}",
            p6.restart_position, p6.damaged_entries, c6.convergence_distance, c6.nodes_reused);
    }

    // ------------------------------------------------------------------
    // F4c (redo): TRUE fence-closer removal -> [z] def swallowed by the
    // fence -> assembled definition table differs -> restart at zero with
    // discarded forward pass (the assembled-table check path).
    // ------------------------------------------------------------------
    {
        let old = "para\n\n```\ncode\n```\nafter\n\n[z]: https://z.com\n\ntail\n";
        let f1 = old.find("```").unwrap();
        let f2 = old[f1 + 3..].find("```").unwrap() + f1 + 3; // closing fence line start
        probe("F4c2 true fence closer removal", old, f2, f2 + 3, "");
    }

    println!("PROBES DONE");
}
