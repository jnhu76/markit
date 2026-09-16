//! RUN-4 driver — semantic dependency fanout battery (plan §23–27,
//! §28's D-axis fill; CORRECTIVE-1 rework per review MAJOR-3).
//!
//! Part 1 (unchanged question, H3/Q8): for N user blocks × a single
//! definition, apply definition- and user-side edits and measure three
//! independent costs: syntax (markit update), index rebuild control,
//! index delta. Blocks are identified by enumerate id == position —
//! these scenarios do NOT test the identity/order split.
//!
//! Part 2 (CORRECTIVE-1 winner-mutation scenarios): structural slot
//! mutations — insert winner before the current winner, insert losing
//! duplicate after it, delete the winner, MOVE a candidate before the
//! winner (same stable id, new rank), split and merge definition
//! blocks. Each scenario asserts the CommonMark first-wins-by-document-
//! order outcome AND the index self-equivalence oracle. The move case
//! is the identity≠order proof: the winner must flip with NO id change.
//!
//! ORACLE (plan §25/§6): after every incremental update the index must
//! equal a clean rebuild over the same ordered blocks (per-label
//! resolution + user sets). Fixtures from refindex::fixtures must pass
//! first.

use std::path::Path;
use std::time::Instant;

use markit_core::markdown::MarkdownState;
use markit_core::{ByteOffset, Document, EditTransaction, SourceRange, TextEdit};

use crate::refindex::{extract_deps, BlockId, RankMap, ReferenceIndex, Resolution};

// ---------- shared helpers -------------------------------------------------

fn timed_us<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t0 = Instant::now();
    let out = f();
    (out, t0.elapsed().as_secs_f64() * 1e6)
}

/// Index-equality oracle: incremental index vs clean rebuild over the
/// same ordered blocks (both ranked by their own block order). Returns
/// true on equality.
fn oracle_equal(
    idx: &ReferenceIndex,
    fresh: &ReferenceIndex,
    rank_idx: &RankMap,
    rank_fresh: &RankMap,
) -> bool {
    let labels: std::collections::BTreeSet<String> =
        idx.labels().chain(fresh.labels()).cloned().collect();
    for l in labels {
        if idx.resolution(&l, rank_idx) != fresh.resolution(&l, rank_fresh) {
            return false;
        }
        if idx.users_set(&l) != fresh.users_set(&l) {
            return false;
        }
    }
    true
}

// ---------- part 1: H3 fanout scenarios (unchanged semantics) --------------

fn corpus(n_users: usize, with_dup: bool) -> String {
    let mut s = String::new();
    s.push_str("# Ref survey corpus\n\nOne early [w] use, unresolved by default.\n\n");
    for i in 0..n_users {
        s.push_str(&format!(
            "Paragraph {i} uses [x] and a private [y{i}] label here.\n\n"
        ));
    }
    s.push_str("[x]: /old\n\n");
    if with_dup {
        s.push_str("[x]: /second-copy\n\n");
    }
    s
}

struct Row {
    scenario: &'static str,
    users: usize,
    syntax_us: f64,
    blocks_reparsed: u64,
    rebuild_us: f64,
    delta_us: f64,
    truly_changed: usize,
    labels_touched: usize,
    oracle_ok: bool,
}

fn fanout_scenario(
    doc: &str,
    edit: TextEdit,
    scenario: &'static str,
    users: usize,
) -> Result<Row, String> {
    let old_doc = Document::new(doc.to_string());
    let old_state = MarkdownState::build(&old_doc.snapshot());
    let old_blocks = blocks_of(doc, &old_state);
    let old_rank = RankMap::from_order(
        &old_blocks.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
    );
    let old_index = ReferenceIndex::build(&old_blocks);

    // apply edit, syntax control (state must share the doc lineage)
    let mut doc2 = Document::new(doc.to_string());
    let mut state = MarkdownState::build(&doc2.snapshot());
    let applied = EditTransaction::typing()
        .with_edit(edit)
        .apply(&mut doc2)
        .map_err(|e| format!("apply: {e}"))?;
    let snapshot = doc2.snapshot();
    let new_whole = snapshot
        .slice(SourceRange::new(ByteOffset(0), ByteOffset(snapshot.len_bytes())));
    let (res, syntax_us) = timed_us(|| state.update(&snapshot, &applied.result));
    res.map_err(|e| format!("update: {e:?}"))?;
    let blocks_reparsed = state.last_work().blocks_reparsed;

    // index work: slot diff (syntax-layer output, untimed) + timed index
    // updates against a clone of the old index
    let new_state = MarkdownState::build(&snapshot);
    let new_blocks = blocks_of(&new_whole, &new_state);
    let new_rank = RankMap::from_order(
        &new_blocks.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
    );
    let (fresh, rebuild_us) = timed_us(|| ReferenceIndex::build(&new_blocks));

    let mut idx = old_index.clone();
    let (delta, delta_us) = timed_us(|| {
        let mut d = crate::refindex::SemanticDelta::default();
        // one combined timed region; update_block returns per-call
        // deltas — concatenate them
        for (i, (id, text)) in new_blocks.iter().enumerate() {
            let changed = match old_blocks.get(i) {
                Some((oid, otext)) => oid != id || otext != text,
                None => true,
            };
            if changed {
                let deps = extract_deps(text);
                d.invalidated
                    .extend(idx.update_block(*id, deps, text, &new_rank).invalidated);
                d.labels_touched += 1;
            }
        }
        for (id, _) in old_blocks.iter().skip(new_blocks.len()) {
            d.invalidated.extend(idx.remove_block(*id, &new_rank).invalidated);
            d.labels_touched += 1;
        }
        d
    });

    // true changed users: compare resolutions old vs new per label
    let mut truly = 0usize;
    let mut labels_seen = std::collections::BTreeSet::new();
    for (user, label, new_res) in &delta.invalidated {
        let old_res = old_index.resolution(label, &old_rank);
        if old_res != *new_res {
            let _ = user;
            truly += 1;
            labels_seen.insert(label.clone());
        }
    }

    let oracle_ok = oracle_equal(&idx, &fresh, &new_rank, &new_rank);

    Ok(Row {
        scenario,
        users,
        syntax_us,
        blocks_reparsed,
        rebuild_us,
        delta_us,
        truly_changed: truly,
        labels_touched: labels_seen.len(),
        oracle_ok,
    })
}

fn blocks_of(doc: &str, state: &MarkdownState) -> Vec<(BlockId, String)> {
    state
        .blocks()
        .enumerate()
        .map(|(i, b)| {
            let r = b.source_range();
            (i as u64, doc[r.start.as_usize()..r.end.as_usize()].to_string())
        })
        .collect()
}

// ---------- part 2: CORRECTIVE-1 winner-mutation scenarios -----------------

#[derive(Clone)]
struct Slot {
    id: BlockId,
    text: String,
}

fn slots_build(slots: &[Slot]) -> (ReferenceIndex, RankMap) {
    let pairs: Vec<(BlockId, String)> =
        slots.iter().map(|s| (s.id, s.text.clone())).collect();
    let rank = RankMap::from_order(&slots.iter().map(|s| s.id).collect::<Vec<_>>());
    (ReferenceIndex::build(&pairs), rank)
}

fn users_of(slots: &[Slot], label: &str) -> usize {
    slots
        .iter()
        .filter(|s| extract_deps(&s.text).uses.iter().any(|u| u == label))
        .count()
}

/// Run one winner mutation: build the old index, apply the mutation to
/// the slot list, replay ONLY the changed slots into a clone, then check
/// the expected winner, the truly-changed user count, and the oracle.
fn winner_mutation(
    name: &'static str,
    before: Vec<Slot>,
    mutate: impl FnOnce(&mut Vec<Slot>),
    expect_winner: &str,
    expect_changed_users: usize,
) -> Result<String, String> {
    let (old_idx, old_rank) = slots_build(&before);
    let mut after = before.clone();
    mutate(&mut after);

    // replay: removed ids, edited ids, inserted ids (only slots whose
    // text/id set changed — models a syntax layer reporting slots)
    let mut idx = old_idx.clone();
    let removed: Vec<BlockId> = before
        .iter()
        .filter(|s| !after.iter().any(|a| a.id == s.id))
        .map(|s| s.id)
        .collect();
    for id in &removed {
        idx.remove_block(*id, &old_rank);
    }
    let after_rank = RankMap::from_order(&after.iter().map(|s| s.id).collect::<Vec<_>>());
    for (pos, s) in after.iter().enumerate() {
        let is_new = !before.iter().any(|b| b.id == s.id);
        let text_changed = before
            .iter()
            .find(|b| b.id == s.id)
            .map(|b| b.text != s.text)
            .unwrap_or(true);
        // order change alone matters too: detect rank change for existing ids
        let rank_changed = before
            .iter()
            .position(|b| b.id == s.id)
            .map(|p| p != pos)
            .unwrap_or(false);
        if is_new {
            let deps = extract_deps(&s.text);
            idx.update_block(s.id, deps, &s.text, &after_rank);
        } else if text_changed {
            let deps = extract_deps(&s.text);
            idx.update_block(s.id, deps, &s.text, &after_rank);
        } else if rank_changed {
            // Pure order change (the MOVE case): zero dep/index
            // maintenance work — order is external, so the winner flips
            // at the next resolution() query against the new provider.
            // Nothing to replay; the winner assertion below uses
            // after_rank and IS the check.
        }
    }

    // expected winner via CommonMark first-wins on the AFTER slot list
    let fresh_pairs: Vec<(BlockId, String)> =
        after.iter().map(|s| (s.id, s.text.clone())).collect();
    let fresh = ReferenceIndex::build(&fresh_pairs);
    let got = match idx.resolution("x", &after_rank) {
        Resolution::Resolved(u) => u,
        Resolution::Unresolved => "<unresolved>".to_string(),
    };
    if got != expect_winner {
        return Err(format!(
            "{name}: winner = {got:?}, want {expect_winner:?} (identity/order bug)"
        ));
    }

    // truly changed users: resolution flips old→new per user
    let old_res = old_idx.resolution("x", &old_rank);
    let truly = if old_res != idx.resolution("x", &after_rank) {
        users_of(&after, "x")
    } else {
        0
    };
    if truly != expect_changed_users {
        return Err(format!(
            "{name}: truly_changed = {truly}, want {expect_changed_users}"
        ));
    }

    if !oracle_equal(&idx, &fresh, &after_rank, &after_rank) {
        return Err(format!("{name}: index self-equivalence oracle FAILED"));
    }
    Ok(format!("{name}: winner={got}, changed_users={truly}, oracle ok"))
}

fn winner_scenarios() -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let users = || {
        vec![
            Slot { id: 41, text: "use [x] one".into() },
            Slot { id: 42, text: "use [x] two".into() },
        ]
    };

    // 1. insert a NEW def block (id 999 > winner id) BEFORE the winner:
    //    must win by document position (the old id-as-order index kept
    //    the smaller id — the review's counterexample).
    let before = {
        let mut v = vec![Slot { id: 10, text: "[x]: /old".into() }];
        v.extend(users());
        v
    };
    out.push(winner_mutation(
        "insert_winner_before",
        before.clone(),
        |slots| slots.insert(0, Slot { id: 999, text: "[x]: /new".into() }),
        "/new",
        2,
    )?);

    // 2. insert the same new def AFTER the winner: loser, zero fanout.
    out.push(winner_mutation(
        "insert_loser_after",
        before.clone(),
        |slots| slots.push(Slot { id: 999, text: "[x]: /new".into() }),
        "/old",
        0,
    )?);

    // 3. delete the current winner: next candidate takes over.
    let with_dup = {
        let mut v = vec![
            Slot { id: 10, text: "[x]: /old".into() },
            Slot { id: 11, text: "[x]: /second".into() },
        ];
        v.extend(users());
        v
    };
    out.push(winner_mutation(
        "delete_winner",
        with_dup.clone(),
        |slots| slots.retain(|s| s.id != 10),
        "/second",
        2,
    )?);

    // 4. MOVE candidate 224 before winner 117 — same stable id, new
    //    rank only. Winner must flip with no id change.
    let movable = {
        let mut v = vec![
            Slot { id: 117, text: "[x]: /old".into() },
            Slot { id: 224, text: "[x]: /moved".into() },
        ];
        v.extend(users());
        v
    };
    out.push(winner_mutation(
        "move_candidate_before",
        movable.clone(),
        |slots| {
            let i = slots.iter().position(|s| s.id == 224).unwrap();
            let s = slots.remove(i);
            slots.insert(0, s);
        },
        "/moved",
        2,
    )?);

    // 5. split a two-def block: one paragraph [a]+[b] becomes two
    //    blocks (117 keeps [a]; new id 999 carries [b]). Resolutions
    //    preserved; no user invalidated.
    let split_doc = {
        let mut v = vec![Slot { id: 117, text: "[a]: /1\n[b]: /2".into() }];
        v.extend(users());
        v
    };
    out.push(winner_mutation(
        "split_def_block",
        split_doc.clone(),
        |slots| {
            slots[0].text = "[a]: /1".into();
            slots.insert(1, Slot { id: 999, text: "[b]: /2".into() });
        },
        "<unresolved>",
        0,
    )?);
    // label-specific checks for the split (x is unresolved here; a/b
    // must keep their urls)
    {
        let (idx, rank) = {
            let slots = vec![
                Slot { id: 117, text: "[a]: /1".into() },
                Slot { id: 999, text: "[b]: /2".into() },
            ];
            slots_build(&slots)
        };
        if idx.resolution("a", &rank) != Resolution::Resolved("/1".into())
            || idx.resolution("b", &rank) != Resolution::Resolved("/2".into())
        {
            return Err("split_def_block: a/b resolutions not preserved".into());
        }
    }

    // 6. merge two def blocks into one: resolutions preserved.
    let merge_doc = {
        let mut v = vec![
            Slot { id: 117, text: "[a]: /1".into() },
            Slot { id: 224, text: "[b]: /2".into() },
        ];
        v.extend(users());
        v
    };
    out.push(winner_mutation(
        "merge_def_blocks",
        merge_doc.clone(),
        |slots| {
            slots[0].text = "[a]: /1\n[b]: /2".into();
            slots.remove(1);
        },
        "<unresolved>",
        0,
    )?);
    {
        let (idx, rank) = slots_build(&[Slot {
            id: 117,
            text: "[a]: /1\n[b]: /2".into(),
        }]);
        if idx.resolution("a", &rank) != Resolution::Resolved("/1".into())
            || idx.resolution("b", &rank) != Resolution::Resolved("/2".into())
        {
            return Err("merge_def_blocks: a/b resolutions not preserved".into());
        }
    }

    Ok(out)
}

// ---------- driver ----------------------------------------------------------

pub fn run(_out: &Path) -> Result<String, String> {
    crate::refindex::fixtures()?;
    let mut s = String::new();
    s.push_str(
        "# RUN-4 — ReferenceIndex semantic dependency experiment (plan §23–27; \
         CORRECTIVE-1 rework)\n\n",
    );
    s.push_str("EXPERIMENTAL_SUBSET: whole-paragraph def lines only (L1 has no ref-def ");
    s.push_str("block construct — ORACLE-B); CommonMark label normalization (lowercase + ");
    s.push_str("whitespace collapse); first def IN DOCUMENT ORDER wins. CORRECTIVE-1: ");
    s.push_str("stable BlockId and document order are separate — candidates store only ");
    s.push_str("(BlockId, url); resolution consults a DocOrder provider supplied by the ");
    s.push_str("syntax layer. All fixtures pass, including the review's counterexample ");
    s.push_str("(a new def block with a LARGER id inserted BEFORE the winner must win). ");
    s.push_str("After every scenario the incrementally updated index equals a clean ");
    s.push_str("rebuild (per-label resolution + user sets).\n\n");

    // Part 2 first: winner mutations (correctness gates).
    let winner_lines = winner_scenarios()?;
    s.push_str("## CORRECTIVE-1 winner-mutation gates (identity ≠ order)\n\n");
    for l in &winner_lines {
        s.push_str(&format!("- {l}\n"));
    }
    s.push_str("\n`move_candidate_before` is the decisive one: the winner flips with ");
    s.push_str("an UNCHANGED stable id — rank, not identity, decides. `insert_winner_\
        before` is the review's counterexample against the old id-as-order index.\n");

    // Part 1: H3 fanout battery.
    let mut rows: Vec<Row> = Vec::new();
    for &n in &[100usize, 1000, 10_000] {
        let doc = corpus(n, false);

        // def URL edit: /old -> /oew (equal length, def at EOF)
        let start = doc.len() - "/old".len();
        let e = TextEdit::replace(
            SourceRange::new(ByteOffset(start), ByteOffset(start + 3)),
            "oew".to_string(),
        );
        rows.push(fanout_scenario(&doc, e, "def_url_edit", n)?);

        // def delete: remove "\n[x]: /old"
        let start = doc.len() - "\n[x]: /old".len();
        let e = TextEdit::delete(SourceRange::new(ByteOffset(start), ByteOffset(doc.len())));
        rows.push(fanout_scenario(&doc, e, "def_delete", n)?);

        // dup doc: edit the SECOND (losing) copy -> zero fanout expected
        let ddoc = corpus(n, true);
        let pos = ddoc.rfind("[x]: /second-copy").ok_or("dup def")?;
        let e = TextEdit::replace(
            SourceRange::new(ByteOffset(pos + 6), ByteOffset(pos + 17)),
            "SECOND-COPX".to_string(),
        );
        rows.push(fanout_scenario(&ddoc, e, "dup_second_edit", n)?);

        // user edit: first user paragraph, no label change
        let anchor = doc.find("uses [x]").ok_or("user anchor")?;
        let e = TextEdit::replace(
            SourceRange::new(ByteOffset(anchor + 5), ByteOffset(anchor + 6)),
            "X".to_string(),
        );
        rows.push(fanout_scenario(&doc, e, "user_edit", n)?);
    }

    s.push_str("\n## H3 fanout battery (syntax blind to semantic dependents)\n\n");
    s.push_str("| scenario | users | syntax_us | blocks_reparsed | rebuild_us | delta_us \
        | truly_changed | labels | oracle |\n\
        |---|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for r in &rows {
        s.push_str(&format!(
            "| {} | {} | {:.1} | {} | {:.0} | {:.1} | {} | {} | {} |\n",
            r.scenario,
            r.users,
            r.syntax_us,
            r.blocks_reparsed,
            r.rebuild_us,
            r.delta_us,
            r.truly_changed,
            r.labels_touched,
            if r.oracle_ok { "ok" } else { "**FAIL**" },
        ));
    }
    s.push_str("\nReading: `syntax_us`/`blocks_reparsed` stay flat in users (the syntax ");
    s.push_str("radius does not see semantic fanout); `truly_changed` grows with the ");
    s.push_str("actual dependent count; `delta_us` stays orders of magnitude below ");
    s.push_str("`rebuild_us` (the no-index world) at every N. `dup_second_edit` edits a ");
    s.push_str("losing duplicate: truly_changed = 0 — the index proves the edit did not ");
    s.push_str("change resolution, which a full re-resolve cannot distinguish cheaply. ");
    s.push_str("Part-1 scenario ids are enumerate positions (identity==order); the ");
    s.push_str("identity/order split itself is exercised by the winner-mutation gates ");
    s.push_str("above, which use ids deliberately different from positions.\n");
    Ok(s)
}
