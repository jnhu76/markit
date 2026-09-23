//! MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — H4 RESTART_CONVERGENCE
//! identity audit. Audit authority: PR #44 merge `0bf678cc`. No
//! production code is changed. Expected values are HAND-DERIVED from the
//! frozen mechanism rules (R5 freeze §9 + R5-CORRECTIVE-1 §6/§11.4/§8 +
//! MEASUREMENT-CORRECTIVE-1 §14/§16) and from the shared grammar.
//!
//! Audit questions answered here:
//!
//! - H4-ID-1 (§18): restart convergence is REAL — the retained prefix
//!   and the converged suffix enter the new state as the SAME
//!   `Arc<RetainedBlock>` identity (asserted with `Arc::ptr_eq`), with
//!   exact gauges `restart_distance = es - r` and
//!   `convergence_distance = take_pos - r`, both Known on every update.
//! - H4-ID-2 (§19): premature convergence is refused by the frozen
//!   predicate, one clause at a time: (e) the paragraph margin refuses
//!   an interruptor-adjacent splice (the heading case), the scanner's
//!   fence rule refuses everything inside a fence body (no hook fires),
//!   and (a)-(d) leave the refused candidate to reparse naturally.
//! - H4-ID-3 (§20): reference-definition damage restarts at ZERO at the
//!   next generation (not a fallback: `fallback_to_full_count` stays
//!   NotApplicable), and the sound assembled-table detection catches the
//!   fence-closer case where NO damaged byte and NO inserted byte is
//!   definition-like — with the discarded forward pass counted
//!   cumulatively (MEASUREMENT-CORRECTIVE-1 §14).
//! - H4-ID-4 (§21): restart-point selection — the last checkpoint at or
//!   before the damage; the restart-boundary continuation margin backs
//!   the restart up when the edit reaches into the boundary line (its
//!   OLD-source reads are reported); es = 0 gives restart_distance 0; an
//!   EOF edit measures the full remaining region.
//! - H4-ID-5 (§22/§39/§41/§24): BREAK→RESTORE, fresh-state determinism,
//!   UTF-8 byte-coordinate distances, completed-state purity.
//!
//! Counting rule (R5-CORRECTIVE-1 §8): `retained_block_nodes` = the
//! block skeleton units + every retained inline syntax node; a plain
//! paragraph = 2 (1 skeleton + 1 Text); a fence/definition = 1.
//! Spans exclude the terminating LF (Skel end = LF position).

use std::sync::Arc;

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::validate_root;
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_restart_convergence::{H4State, RestartConvergenceMechanism};

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn at(haystack: &[u8], needle: &str) -> usize {
    let n = needle.as_bytes();
    haystack
        .windows(n.len())
        .position(|w| w == n)
        .unwrap_or_else(|| panic!("needle {needle:?} not present"))
}

fn post_of(old: &[u8], edit: &CanonicalEdit) -> Vec<u8> {
    edit.apply(&source_of(old, 99), SourceId(100))
        .expect("post")
        .as_bytes()
        .to_vec()
}

fn h4_state_of(bytes: &[u8]) -> H4State {
    let mech = RestartConvergenceMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

/// ONE sink across BOTH phases — exactly the runner's A-LANE shape:
/// H4's prepare phase reads the OLD source (the restart-boundary
/// continuation margin), and a per-phase sink would drop those events at
/// `finalize_derived`. Also returns the prepared value for the audit's
/// restart-selection observations.
fn h4_update(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H4State,
) -> (
    H4State,
    markit_mdbench_restart_convergence::H4Prepared,
    WorkCounters,
) {
    let mech = RestartConvergenceMechanism::new();
    let old = source_of(old, 2);
    let post = source_of(post, 3);
    let mut counters = WorkCounters::all_unknown();
    let (state, prepared) = {
        let mut sink = CounterSink::new(&mut counters);
        let prepared = {
            let mut cx = MechanismContext::new(&mut sink);
            mech.prepare_update(&old, &post, edit, &old_state, &mut cx)
                .expect("prepare")
        };
        let prepared_clone = prepared.clone();
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .update(&old, &post, edit, old_state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        sink.finalize_derived();
        (done.state, prepared_clone)
    };
    (state, prepared, counters)
}

fn h4_run(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H4State,
) -> (H4State, WorkCounters) {
    let (state, _, counters) = h4_update(old, post, edit, old_state);
    (state, counters)
}

fn assert_matches_h0(state: &H4State, post: &[u8], context: &str) {
    let want = normalized_checksum(&parse_document(post));
    assert_eq!(
        state.result_checksum(),
        want,
        "{context}: result must equal H0"
    );
    let doc = state.normalize_v1();
    validate_root(&doc.root, Some(post))
        .unwrap_or_else(|e| panic!("{context}: retained state violates NORMALIZED-RESULT-v1: {e}"));
}

/// H4-ID-1: the canonical local edit ("two" -> "TWO" in p2 of
/// `para one / para two / para three`, all spans LF-exclusive:
/// p1 [0,8), p2 [10,18), p3 [20,30), es=15, ee=18, delta=0).
///
/// Hand derivation: restart selection picks the checkpoint at 10
/// (last one <= 15); the boundary margin reads OLD [8,10) (2 LFs ->
/// boundary intact, no backup). Damage: p2 only, damaged_end = 18.
/// Forward parse from 10: consults at 10 (<= ee_new, refused), 19 (no
/// checkpoint at q=19), 20 (checkpoint (a) at q=20, (b) key, (c) gen,
/// (d) 20 >= 18, (e) blank margin [18,19) -> TAKE to EOF), 30 (one
/// convergence per update). p2' fresh; p1 retained prefix; p3 retained
/// suffix. reused = 2 + 2 = 4; rebuilt = 2; blocks = 1.
/// Gauges: restart_distance = 15 - 10 = 5; convergence_distance =
/// 20 - 10 = 10. metadata = prepare (3 cps + 3 entries + 1 backed) +
/// update (3 consults + 1 slot + 3 checkpoints + 1 rebased) = 15 (the
/// splice jumps straight to EOF, so the taken range offers no further
/// consult — there is no consult at 30).
/// Structural sharing is proven with Arc::ptr_eq, not just counts.
#[test]
fn h4_convergence_reuses_prefix_and_suffix_with_exact_gauges() {
    let old_b = b"para one\n\npara two\n\npara three\n";
    let es = at(old_b, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(old_b, &edit);
    let old_state = h4_state_of(old_b);
    let old_arcs: Vec<_> = old_state
        .blocks()
        .iter()
        .map(|s| Arc::clone(&s.block))
        .collect();
    let (state, prepared, counters) = h4_update(old_b, &post_b, &edit, old_state);

    assert_matches_h0(&state, &post_b, "exact convergence");
    // Restart selection is observable on the prepared value.
    assert_eq!(prepared.restart_slot, 1, "checkpoint at 10 <= es 15");
    assert_eq!(prepared.restart_position, 10);
    assert_eq!(prepared.damaged_entries, 1, "p2 only");
    assert_eq!(prepared.damaged_end, 18);
    assert_eq!(prepared.scanned_checkpoints, 3);
    // Exact attribution.
    assert_eq!(counters.blocks_reparsed, Observed::Known(1), "p2' fresh");
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "prefix p1 (2) + converged suffix p3 (2)"
    );
    assert_eq!(
        counters.metadata_records_touched,
        Observed::Known(15),
        "consults at 10, 19, 20 only: the splice jumps over the taken range"
    );
    // Gauges: measured on EVERY update, Known, never N/A.
    assert_eq!(counters.restart_distance, Observed::Known(5));
    assert_eq!(counters.convergence_distance, Observed::Known(10));
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "H4 has no degraded mode: restart-at-zero is a restart, not a fallback"
    );
    // Source coverage: OLD = the boundary margin [8,10) only; POST =
    // [9,20) (probe [15,18), lines (10,19) and (19,20), the (e)-margin
    // read (18,19), p2' segment (10,18), p2' line offset (9,10)) plus
    // the taken suffix's carried byte [30,31) — the taken range itself
    // is never scanned.
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(2));
    assert_eq!(counters.unique_post_source_bytes, Observed::Known(12));
    // Structural sharing: prefix and suffix are the SAME Arcs.
    let blocks = state.blocks();
    assert_eq!(blocks.len(), 3);
    assert!(Arc::ptr_eq(&blocks[0].block, &old_arcs[0]), "prefix shared");
    assert!(Arc::ptr_eq(&blocks[2].block, &old_arcs[2]), "suffix shared");
    assert!(!Arc::ptr_eq(&blocks[1].block, &old_arcs[1]), "p2' fresh");
    assert_eq!(blocks[2].abs_start(), 20, "delta 0: suffix unmoved");
    // Retargeted suffix checkpoint keeps its provenance.
    assert_eq!(state.checkpoints()[2].position, 20);
    assert_eq!(state.checkpoints()[2].gen, 0);
    assert_eq!(state.generation(), 0);
}

/// H4-ID-2a (§19): premature convergence refused by predicate (e) — the
/// paragraph margin. Old = `para one / ## head / (blank) / para two`
/// (p1 [0,8), head [9,16), p3 [18,25); the heading INTERRUPTS the
/// paragraph, so the checkpoint at 9 has NO blank line before it).
/// Edit "one"->"ONE" (es=5, ee=8, delta=0): the live parse offers a
/// block start at 9 with q=9 matching the checkpoint and a matching
/// top-level key — but the line before it is p1's own text, so (e)
/// reads [0,8) and refuses. The heading reparses fresh; the later
/// consult at 18 (blank-separated) converges and takes p3.
/// Without the (e) refusal the take at 9 would deliver convergence at
/// 9 with a spliced open paragraph — the exact failure the frozen
/// predicate exists to prevent (adversarial: found by the predicate
/// design, D5).
#[test]
fn h4_predicate_e_refuses_interruptor_adjacent_splice() {
    let old_b = b"para one\n## head\n\npara two\n";
    let es = at(old_b, "one");
    let edit = CanonicalEdit::new(es, es + 3, "ONE").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "predicate (e) refusal");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "p1' AND head' fresh"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(4),
        "2 + 2 (head has 1 Text)"
    );
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(2),
        "only the blank-separated suffix p3 is taken"
    );
    assert_eq!(counters.restart_distance, Observed::Known(5));
    assert_eq!(
        counters.convergence_distance,
        Observed::Known(18),
        "the take happened at 18, NOT at the refused candidate 9"
    );
    assert_eq!(counters.metadata_records_touched, Observed::Known(15));
    // POST coverage = [0,18) + the carried byte [25,26) = 19: the
    // (e)-margin read at consult(9) covers [0,8) (p1's own line), and
    // the taken tail [18,26) is never line-scanned — only its last
    // byte is reported by the splice's carried check.
    assert_eq!(counters.unique_post_source_bytes, Observed::Known(19));
    assert_eq!(
        counters.unique_old_source_bytes,
        Observed::Known(0),
        "restart at slot 0: no boundary margin read"
    );
}

/// H4-ID-2b (§19): fence-forward-state change. Deleting the closer of
/// ` ```go / body / ``` ` turns the unchanged tail into fence BODY: the
/// scanner never fires the hook inside a fence body, so no consult can
/// even offer the tail — structural refusal upstream of the predicate.
/// Everything after the fence opener reparses (the EOF fence).
/// RESTORE (re-insert the closer, §39 metamorphic): the broken state's
/// only checkpoint is the fence at 0, so nothing can converge (q maps
/// to 12/13, no checkpoint) and the restored document is parsed fresh —
/// BREAK→RESTORE is sound in both directions. A fresh-state repeat of
/// the restore produces IDENTICAL counters (Property C).
#[test]
fn h4_fence_state_change_swallows_the_tail_then_restores() {
    let old_b = b"```go\nbody\n```\n\ntail para\n";
    let first = at(old_b, "```");
    let closer = old_b[first + 3..]
        .windows(3)
        .position(|w| w == b"```")
        .map(|p| p + first + 3)
        .expect("closer present");
    // BREAK: fence [0,14), tail [16,25); delete the closer [11,14).
    let break_edit = CanonicalEdit::new(closer, closer + 3, "").expect("edit");
    let broken_b = post_of(old_b, &break_edit);
    assert_eq!(broken_b.len(), 23);
    let (broken_state, _, broken_counters) =
        h4_update(old_b, &broken_b, &break_edit, h4_state_of(old_b));
    assert_matches_h0(&broken_state, &broken_b, "after BREAK");
    assert_eq!(
        broken_counters.blocks_reparsed,
        Observed::Known(1),
        "the EOF fence"
    );
    assert_eq!(broken_counters.nodes_rebuilt, Observed::Known(1));
    assert_eq!(
        broken_counters.nodes_reused,
        Observed::Known(0),
        "the tail is fence body now: no consult is ever offered"
    );
    assert_eq!(broken_counters.restart_distance, Observed::Known(11));
    assert_eq!(broken_counters.convergence_distance, Observed::Known(23));
    assert_eq!(broken_counters.metadata_records_touched, Observed::Known(6));
    assert_eq!(broken_state.checkpoints().len(), 1, "only the fence at 0");

    // RESTORE: re-insert the closer at [11,11).
    let restore_edit = CanonicalEdit::new(closer, closer, "```").expect("edit");
    let restored_b = post_of(&broken_b, &restore_edit);
    assert_eq!(
        restored_b,
        old_b.to_vec(),
        "restore reconstructs the original"
    );
    let (restored, restore_counters) = h4_run(&broken_b, &restored_b, &restore_edit, broken_state);
    assert_matches_h0(&restored, &restored_b, "after RESTORE");
    assert_eq!(restore_counters.blocks_reparsed, Observed::Known(2));
    assert_eq!(restore_counters.nodes_rebuilt, Observed::Known(3));
    assert_eq!(
        restore_counters.nodes_reused,
        Observed::Known(0),
        "the broken state's checkpoint table cannot vouch the tail (q=12/13)"
    );
    assert_eq!(restore_counters.restart_distance, Observed::Known(11));
    assert_eq!(restore_counters.convergence_distance, Observed::Known(26));

    // Property C: the same restore from a FRESH state is identical.
    let fresh = h4_state_of(&broken_b);
    let (again, again_counters) = h4_run(&broken_b, &restored_b, &restore_edit, fresh);
    assert_matches_h0(&again, &restored_b, "fresh-state repeat");
    assert_eq!(
        restore_counters, again_counters,
        "all counters deterministic"
    );
}

/// H4-ID-2c (§19): container join converges at the NEXT checkpoint.
/// Old = `- a / - b / (blank) / tail` (list [0,7), tail [9,13)).
/// Deleting `- b\n` ([4,8), delta -4) joins nothing INSIDE the old
/// list — the damaged list reparses fresh (one item), and the consult
/// at the tail's new position 5 maps back to the old checkpoint 9:
/// (a) exact, (b) top-level key, (c) gen, (d) 9 >= damaged_end 7,
/// (e) blank margin [4,4). Take at 5.
#[test]
fn h4_container_join_converges_at_next_checkpoint() {
    let old_b = b"- a\n- b\n\ntail\n";
    let edit = CanonicalEdit::new(4, 8, "").expect("edit");
    let post_b = post_of(old_b, &edit);
    assert_eq!(post_b, b"- a\n\ntail\n".to_vec());
    let (state, _, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "container join");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(3),
        "fresh list skeleton units: List + Item + its Para"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(4), "3 + Text a");
    assert_eq!(counters.nodes_reused, Observed::Known(2), "the tail");
    assert_eq!(counters.restart_distance, Observed::Known(4));
    assert_eq!(counters.convergence_distance, Observed::Known(5));
    assert_eq!(counters.metadata_records_touched, Observed::Known(11));
    let blocks = state.blocks();
    assert_eq!(blocks[1].abs_start(), 5, "suffix retargeted by delta -4");
}

/// H4-ID-4 (§21): the RESTART-BOUNDARY continuation margin. Old =
/// `para one / ## head / (blank) / z` (checkpoints 0, 9, 18). The edit
/// "head"->"hat" (es=12, ee=16) selects the checkpoint at 9 — but the
/// separation [8,9) carries ONE LF (< 2), and the edit (12) reaches
/// INTO the boundary line [9,16): the margin backs the restart up to
/// slot 0 so the merge happens inside the reparsed region. OLD reads:
/// the separation (8,9) and the boundary-line scan (9,17) — 9 bytes.
/// Forward from 0: consults at 0 (<= ee_new 5), 9 (q=10, no
/// checkpoint), 16 (q=17, none), 17 (q=18 exact, blank -> TAKE).
#[test]
fn h4_restart_boundary_backs_up_when_the_edit_reaches_the_boundary_line() {
    let old_b = b"para one\n## head\n\nz\n";
    let es = at(old_b, "head");
    let edit = CanonicalEdit::new(es, es + 4, "hat").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, prepared, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "restart-boundary backup");
    assert_eq!(prepared.restart_slot, 0, "backed up from slot 1 to 0");
    assert_eq!(prepared.restart_position, 0);
    assert_eq!(prepared.damaged_end, 16);
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "p1' + head' fresh"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(4));
    assert_eq!(counters.nodes_reused, Observed::Known(2), "only z");
    // OLD coverage: separation (8,9) + boundary line (9,17) = [8,17).
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(9));
    assert_eq!(
        counters.restart_distance,
        Observed::Known(12),
        "es 12 - r 0"
    );
    assert_eq!(counters.convergence_distance, Observed::Known(17));
    assert_eq!(
        counters.metadata_records_touched,
        Observed::Known(16),
        "prepare 7 (3 cps + 3 entries + 1 backed) + update 9 (4 consults \
         at 0/9/16/17 + 1 slot + 3 checkpoints + 1 rebased)"
    );
}

/// H4-ID-3a (§20): definition damage RESTARTS AT ZERO at the next
/// generation — with the gauge semantics of a restart, not a fallback.
/// Old = `para one / (blank) / [a]: /url / (blank) / para three`
/// (p1 [0,8), def [10,19), p3 [21,31); len 32). Editing /url->/new
/// (es=15, ee=19) marks the def damaged (damaged_has_def), which
/// short-circuits the `]: ` probe (no probe event is reported).
/// restart_at_zero: full fresh parse (blocks 3, nodes 2+1+2 = 5),
/// reused 0, metadata +3 new checkpoints, gen 0 -> 1,
/// restart_distance = Known(es) = 15, convergence_distance =
/// Known(post.len()) = 32.
#[test]
fn h4_definition_damage_restarts_at_zero_with_generation_bump() {
    let old_b = b"para one\n\n[a]: /url\n\npara three\n";
    let es = at(old_b, "/url");
    let edit = CanonicalEdit::new(es, es + 4, "/new").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "definition restart-at-zero");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(3),
        "the full fresh parse"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(5));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "document-global reference invalidation: nothing is vouched"
    );
    assert_eq!(counters.restart_distance, Observed::Known(15));
    assert_eq!(counters.convergence_distance, Observed::Known(32));
    assert_eq!(counters.metadata_records_touched, Observed::Known(10));
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::NotApplicable,
        "a restart is not a fallback"
    );
    assert_eq!(state.generation(), 1, "the definition generation bumped");
    assert_eq!(state.defs(), &[("a".to_string(), "/new".to_string())]);
    assert_eq!(
        counters.unique_old_source_bytes,
        Observed::Known(2),
        "only the boundary margin [8,10) read the OLD source"
    );
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(32),
        "the restart reparses every POST byte"
    );
}

/// H4-ID-3b (§20): the SOUND definition-environment detection. Old =
/// ` ```go / body / ``` / (blank) / [a]: /url ` (fence [0,14), def
/// [16,25); len 26; defs = [(a, /url)]). Deleting the closer
/// ([11,14), delta -3) touches NO definition byte and inserts NO
/// `]: ` — the fast probes are both negative — yet the def leaves the
/// document table (it becomes fence content). The assembled-table
/// comparison ([] != [(a, /url)]) catches it: the discarded forward
/// pass is counted (skeleton-only, MEASUREMENT-CORRECTIVE-1 §14), then
/// restart_at_zero delivers the result at gen 1. Cumulative totals:
/// blocks = 1 (discarded) + 1 (restart), nodes = 2, metadata =
/// 4 (prepare) + 1 (discarded consult) + 1 (restart checkpoints).
#[test]
fn h4_assembled_table_detects_the_fence_swallowed_definition() {
    let old_b = b"```go\nbody\n```\n\n[a]: /url\n";
    let first = at(old_b, "```");
    let closer = old_b[first + 3..]
        .windows(3)
        .position(|w| w == b"```")
        .map(|p| p + first + 3)
        .expect("closer present");
    let edit = CanonicalEdit::new(closer, closer + 3, "").expect("edit");
    let post_b = post_of(old_b, &edit);
    assert_eq!(post_b.len(), 23);
    let (state, _, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "fence-swallowed definition");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "1 discarded fence skeleton + 1 restart fence"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    assert_eq!(counters.restart_distance, Observed::Known(11));
    assert_eq!(counters.convergence_distance, Observed::Known(23));
    assert_eq!(counters.metadata_records_touched, Observed::Known(6));
    assert_eq!(state.generation(), 1);
    assert_eq!(state.defs(), &[], "the definition left the environment");
}

/// H4-ID-4 (§21): restart-point positions. (a) es = 0: restart at the
/// checkpoint 0 with restart_distance Known(0) (a measured zero); the
/// convergence consult takes the {p2, p3} suffix at its shifted
/// position 16 (base_shift +6, Arc-shared). (b) an edit AT EOF that
/// continues p3 ("para three\ntail" joins into one paragraph): no old
/// entry is damaged; the region [20,35) reparses; no consult reaches a
/// take (the only candidates are <= ee_new or at EOF), so the prefix
/// {p1, p2} is the reuse and convergence_distance measures to EOF.
/// (c) an edit INSIDE a fence body: the fence is one atomic block —
/// the whole fence reparses and convergence happens right after it.
#[test]
fn h4_restart_point_positions() {
    // (a) es = 0.
    let old_b = b"para one\n\npara two\n\npara three\n";
    let head_edit = CanonicalEdit::new(0, 0, "HEAD\n\n").expect("edit");
    let head_post = post_of(old_b, &head_edit);
    let old_state = h4_state_of(old_b);
    let old_arcs: Vec<_> = old_state
        .blocks()
        .iter()
        .map(|s| Arc::clone(&s.block))
        .collect();
    let (state_a, _, counters_a) = h4_update(old_b, &head_post, &head_edit, old_state);
    assert_matches_h0(&state_a, &head_post, "es=0 insert");
    assert_eq!(
        counters_a.restart_distance,
        Observed::Known(0),
        "measured zero"
    );
    assert_eq!(counters_a.convergence_distance, Observed::Known(16));
    assert_eq!(
        counters_a.nodes_reused,
        Observed::Known(4),
        "p2 + p3 suffix"
    );
    assert_eq!(counters_a.blocks_reparsed, Observed::Known(2));
    assert_eq!(counters_a.nodes_rebuilt, Observed::Known(4));
    assert_eq!(
        counters_a.metadata_records_touched,
        Observed::Known(18),
        "checkpoints.len() = 4 assembled slots (HEAD' + p1' + p2 + p3)"
    );
    assert_eq!(counters_a.unique_old_source_bytes, Observed::Known(0));
    assert_eq!(
        counters_a.unique_post_source_bytes,
        Observed::Known(17),
        "[0,16) + the carried byte [36,37): the taken suffix is unscanned"
    );
    let blocks_a = state_a.blocks();
    assert_eq!(blocks_a.len(), 4, "HEAD' + p1' fresh, p2 + p3 shared");
    assert!(Arc::ptr_eq(&blocks_a[2].block, &old_arcs[1]), "p2 shared");
    assert!(Arc::ptr_eq(&blocks_a[3].block, &old_arcs[2]), "p3 shared");
    assert_eq!(blocks_a[2].abs_start(), 16, "base_shift +6");

    // (b) EOF continuation edit.
    let eof_edit = CanonicalEdit::new(old_b.len(), old_b.len(), "tail").expect("edit");
    let eof_post = post_of(old_b, &eof_edit);
    assert_eq!(
        eof_post,
        b"para one\n\npara two\n\npara three\ntail".to_vec()
    );
    let (_, _, counters_b) = h4_update(old_b, &eof_post, &eof_edit, h4_state_of(old_b));
    assert_eq!(
        counters_b.restart_distance,
        Observed::Known(11),
        "es 31 - r 20"
    );
    assert_eq!(
        counters_b.convergence_distance,
        Observed::Known(15),
        "to EOF"
    );
    assert_eq!(
        counters_b.nodes_reused,
        Observed::Known(4),
        "prefix p1 + p2"
    );
    assert_eq!(counters_b.blocks_reparsed, Observed::Known(1), "p3' joined");
    assert_eq!(
        counters_b.nodes_rebuilt,
        Observed::Known(2),
        "1 skeleton + ONE merged Text run: the inline scanner joins the \\
         continuation lines' literal content into a single [20,35) run"
    );
    assert_eq!(counters_b.metadata_records_touched, Observed::Known(12));
    assert_eq!(counters_b.unique_old_source_bytes, Observed::Known(2));
    assert_eq!(
        counters_b.unique_post_source_bytes,
        Observed::Known(16),
        "[19,35): the region from the restart, its margins, p3' segments"
    );

    // (c) edit inside a fence body: old = p1 / fence / tail.
    let old_c = b"para one\n\n```go\nbody\n```\n\ntail\n";
    let arms_edit =
        CanonicalEdit::new(at(old_c, "body"), at(old_c, "body") + 4, "arms").expect("edit");
    let arms_post = post_of(old_c, &arms_edit);
    let (state_c, prepared_c, counters_c) =
        h4_update(old_c, &arms_post, &arms_edit, h4_state_of(old_c));
    assert_matches_h0(&state_c, &arms_post, "edit inside a fence body");
    assert_eq!(
        prepared_c.restart_position, 10,
        "restart AT the fence start"
    );
    assert_eq!(counters_c.restart_distance, Observed::Known(6));
    assert_eq!(counters_c.convergence_distance, Observed::Known(16));
    assert_eq!(
        counters_c.nodes_reused,
        Observed::Known(4),
        "prefix p1 + tail"
    );
    assert_eq!(
        counters_c.blocks_reparsed,
        Observed::Known(1),
        "the whole fence"
    );
    assert_eq!(counters_c.nodes_rebuilt, Observed::Known(1));
    assert_eq!(counters_c.metadata_records_touched, Observed::Known(15));
    assert_eq!(counters_c.unique_old_source_bytes, Observed::Known(2));
    assert_eq!(
        counters_c.unique_post_source_bytes,
        Observed::Known(18),
        "[9,26) (line offset, fence lines incl. body, margins, probe) + \\
         the taken tail's carried byte [30,31) — [26,31) is never scanned"
    );
}

/// H4-ID-5a (§41): UTF-8 byte coordinates through the gauges. CJK doc
/// (3-byte chars): p1 [0,15), p2 [17,32), p3 [34,49), checkpoints at
/// 0/17/34. Inserting Ｘ (3 bytes) at p2's end (es=32, delta +3):
/// restart at 17 (boundary margin [15,17) = 2 LFs, intact), damaged_end
/// 0 (the zero-width edit is at p2's span end, outside [s,e)), consult
/// at 37 maps to q=34 exact -> take. restart_distance = 32 - 17 = 15
/// BYTES; convergence_distance = 37 - 17 = 20 BYTES. Gauges are in the
/// persistent coordinate system (UTF-8 bytes), never chars.
#[test]
fn h4_cjk_byte_coordinate_distances() {
    let old_b = "中文第一段\n\n中文第二段\n\n第三段在远处\n".as_bytes();
    assert_eq!(at(old_b, "第二段"), 23);
    let es = 32; // end of p2's content (before its LF)
    assert_eq!(&old_b[29..32], "段".as_bytes());
    let edit = CanonicalEdit::new(es, es, "Ｘ").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, counters) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));

    assert_matches_h0(&state, &post_b, "CJK distances");
    assert_eq!(post_b.len(), 56, "53 + 3 inserted bytes");
    assert_eq!(counters.restart_distance, Observed::Known(15));
    assert_eq!(counters.convergence_distance, Observed::Known(20));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "p1 prefix + p3 suffix"
    );
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(counters.metadata_records_touched, Observed::Known(15));
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(2));
    let blocks = state.blocks();
    assert_eq!(blocks[2].abs_start(), 37, "p3 retargeted to byte 37");
}

/// H4-ID-5b (§24): completed-state purity — the projection and the
/// checksum are pure queries over the retained representation.
#[test]
fn h4_completed_state_pure_query() {
    let old_b = b"para one\n\npara two\n\npara three\n";
    let es = at(old_b, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, _) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));
    assert_eq!(state.normalize_v1(), state.normalize_v1());
    assert_eq!(state.result_checksum(), state.result_checksum());
    // Break/restore determinism (§39) on the canonical doc: the same
    // edit history from different starting states agrees on counters.
    let (s1, _, c1) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));
    let (s2, _, c2) = h4_update(old_b, &post_b, &edit, h4_state_of(old_b));
    assert_eq!(s1.result_checksum(), s2.result_checksum());
    assert_eq!(c1, c2);
}
