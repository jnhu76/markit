# IMPLEMENTATION-MAP

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.
All paths relative to `research/benchmarks/markdown-ast-update/`.

Function-level map of where each claimed mechanism behavior actually
executes. Evidence class `CODE_INSPECTION_SUPPORT` throughout (each
anchor was read in full during the audit); behavior claims get
`TEST_SUPPORT` pointers to the audit suites.

## Shared substrate (NOT horse-authored)

| Concern | Location | Notes |
|---|---|---|
| Block scanner | `shared-grammar/src/parser.rs` `BlockScanner::run` (L325–L358) | Line-driven; calls the horse splice hook at every line start BEFORE prefix consumption, never while a fence is open (L331–L340) |
| Splice protocol | `parser.rs` `splice_to` (L373–L400) | Emits `Skel::Spliced`, reports the carried tail byte (L385–L389), advances frame bookkeeping |
| Splice hook type | `parser.rs` L205–L212 | `FnMut(pos, &ContextKey) -> Option<new_pos>`; "the scanner never decides reuse — the hook does" |
| Inline scanner | `shared-grammar/src/inline.rs` `scan_region_with_sink` (L263–L275) | Reports each scanned segment to the sink before scanning |
| Reported scans | `parser.rs` `line_start_of_reported_in` (L1024–L1033), `memchr_lf_reported_in` (L1044–L1057) | R5-CORRECTIVE-2 closure: backward/forward scans report exactly the inspected bytes |
| Counters | `common/src/work.rs` `WorkCounters` (L106+), `CounterSink` (L274), `finalize_derived` (L313) | `finalize_derived` recomputes unique_old/post/combined + total from THIS sink's event log (per-version union) |
| Timing boundary | `common/src/mechanism.rs` L113–L167; `runner/src/orchestrate.rs` `run_update_attributed` (L347–L390) | A-LANE = ONE `CounterSink` across prepare+update, `finalize_derived` once (L383); mechanism never sees a clock |
| Lanes | `instrumentation/src/lanes.rs` L23–L37, L234–L249 | T/M/A lanes structurally distinct payloads |

## H0 FULL_REBUILD — `mechanisms/full-rebuild/src/lib.rs`

| Behavior | Anchor |
|---|---|
| Drops retained state; full parse per update | `update` L249–L268 (old state consumed and discarded; `parse_document`-equivalent region parse via shared grammar) |
| Attribution profile: `nodes_reused = 0` measured, fallback/metadata/gauges N/A on update | `report_attribution` L153–L185 |
| Native node counting (R5 §11.6) | `count_blocks_nodes` L186–L222 (Document root excluded) |

`TEST_SUPPORT`: `mechanisms/full-rebuild/tests/audit_identity.rs`
(history independence, complete-post coverage, attribution profile,
completed-state purity, UTF-8/boundary edits).

## H1 BLOCK_LOCAL_REPARSE — `mechanisms/block-local/src/lib.rs`

| Behavior | Anchor |
|---|---|
| Top-entry tiling (`TopEntry::Block{skel,sem,facts}` / `Blank`) | state structs, L~60–L170 |
| Damage scan (strict overlap, one linear pass) | `prepare_update` L261+ (damage scan block; comment L306–L307) |
| Insertion-gap mapping (no-overlap path) | `prepare_update` (comment "insertion-gap mapping") |
| Fallback to full (F-path) | `total_fallback` L223–L260: `record_fallback_to_full` once (L229), discarded region counted as skeleton-only, `add_nodes_reused(0)` |
| F1–F6 guards (fence boundary, terminator scan, continuation pairs) | `guards_fire` L585–L664, `continuation_pair` L665–L694, `would_continue` L695–L749 |
| Suffix reconstruction by delta shift | `shift_entry_owned` L750–L771, `shift_node_owned` L772–L810 |
| Counting rule | `entry_native_nodes` L912+ |

`TEST_SUPPORT`: `mechanisms/block-local/tests/audit_identity.rs`
(10 tests: exact local-edit counters, probe+fallback exactly-once,
F1 def fallback, F4(a) beyond-edge, F6 bounded/unbounded, insertion-gap
mapping, BREAK/RESTORE + Property C, interleaved fresh-state isolation,
CJK, purity).

## H2 FRAGMENT_REUSE — `mechanisms/fragment-reuse/src/lib.rs`

| Behavior | Anchor |
|---|---|
| Fragment table, parent-relative `FNode` + `ContextKey` vouching | `FNode` L105–L125, `Fragment` L178+ |
| Frozen `minGap = 128` | `MIN_GAP` L59; left cut `prepare_update` L301+ (`old.len() - ee >= MIN_GAP`), right cut `update` L337+ |
| Consult-time windows + paragraph margin | `left_window_end` L530–L586 (left-edge paragraph margin can drop the adjacent fragment), `right_window_start` L587+, `Cursor::consult` L642+ |
| Splice-hook integration (scanner never decides) | `update` L337–L470 (hook wiring L409–L414) |
| Reference-environment clause (CORRECTIVE-3: rebuilt-table comparison) | env comparison L444–L447, `rebuilt_table` L1219+ |
| Semantic rematerialization | `mentions_reference` L1054–L1078, `rematerialize` L1079+ (payload re-scan keeps `Arc` descendants); accounting `reused - rematerialized_members + rematerialized_kept` L469 |

`TEST_SUPPORT`: `mechanisms/fragment-reuse/tests/audit_identity.rs`
(7 tests: exact suffix-run reuse with interior-unscanned assertion,
fence context change -> reuse 0, rematerialization split, minGap
boundary 127/128 both sides, BREAK/RESTORE + determinism, CJK,
purity).

## H3 OLD_TREE_SUBTREE_REUSE — `mechanisms/old-tree-subtree-reuse/src/lib.rs`

| Behavior | Anchor |
|---|---|
| Patched-tree damage map (copy-on-write flags, gap edits) | `patch_tree` L496–L662, `patch_node` L689–L720, `mark_changed` L724+ |
| Continuation margins (prev OLD read, next POST read, >= 2 LFs) | `patch_tree` L549–L588 |
| Gap edit: next entry's gap absorbs delta | L589–L602 |
| Forward cursor: consult / find_run / search_level (changed -> descend, ctx mismatch -> descend, stale-position guard, disjoint-from-edit) | `Cursor::consult` L782–L839, `find_run` L850–L859, `search_level` L861–L917 |
| '[' probe + rematerialization (own-table) | `mentions_reference` L1114–L1128, `rematerialize` L1140+, assembled via `assemble_level` L1338–L1392 (env gate L1364–L1375) |
| Assembled-state invariants | new tree carries NO change flags (flagged nodes are never taken) — `TEST_SUPPORT` `h3_exact_reuse_and_change_flags` |

`TEST_SUPPORT`: `mechanisms/old-tree-subtree-reuse/tests/audit_identity.rs`
(8 tests: exact reuse + change flags incl. `prepared.scanned_entries` /
`patched_nodes` observability, prefix-insertion relocation with the
conservative empty-separation refusal, fence-closer removal, def-env
repair, multi-entry deletion clamp, BREAK/RESTORE + determinism, CJK,
purity).

## H4 RESTART_CONVERGENCE — `mechanisms/restart-convergence/src/lib.rs`

| Behavior | Anchor |
|---|---|
| Retained representation: `RetainedBlock{skel, sem}` (ownership pass-through) + `BlockSlot{base_shift, line_offset}` + 1:1 `Checkpoint{position,key,gen}` | L82–L140 |
| Restart selection (last checkpoint <= es) | `prepare_update` L334–L342 |
| Restart-boundary continuation margin (backs up; OLD reads reported) | L344–L400 (OLD record L367–L371; `memchr_lf_reported_in(Old)` L378–L383) |
| Damage scan + damaged_has_def | L402–L418 |
| Definition probe (short-circuits on damaged_has_def) | `update` L452–L459 |
| Restart-at-zero (frozen response; gen bump; gauges = es / post.len()) | `restart_at_zero` L268–L301 |
| Convergence predicate (a)–(e) | `Cursor::consult` L739–L782 — see `H4-CONVERGENCE-PREDICATE.md` |
| Assembled-table sound check + cumulative discard accounting | L564–L590 |
| Prefix reuse counted (H4-A) + suffix rebasing | L500–L524 (prefix), L597–L620 (suffix `base_shift + delta`) |
| Gauges on every update | L662–L675 (`restart_distance = es - r`, `convergence_distance = convergence_pos - r`) |
| Retained counting rule | `retained_block_nodes` L949–L951 |

`TEST_SUPPORT`: `mechanisms/restart-convergence/tests/audit_identity.rs`
(10 tests — see `ADVERSARIAL-TEST-MATRIX.md`).

## Test-side instrumentation seam

The audit suites reproduce the runner's A-LANE exactly: ONE
`CounterSink` across prepare+update, `finalize_derived()` once
(e.g. `mechanisms/old-tree-subtree-reuse/tests/audit_identity.rs`
`h3_update`, L78–L118; `mechanisms/restart-convergence/tests/audit_identity.rs`
`h4_update`). A per-phase sink silently drops the prepare-phase events
at `finalize_derived` — this audit initially made that mistake in the
H3/H1 helpers and corrected it; the production runner was always
correct (`orchestrate.rs` L366–L383).
