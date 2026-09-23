# ADVERSARIAL-TEST-MATRIX

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

The 40 audit tests (5 suites, all green at the audit SHA), what each
pins, and what a WRONG implementation of the same family would do.
Every expected value is hand-derived from the frozen rules + shared
grammar, NOT read off the mechanism.

## H0 — `mechanisms/full-rebuild/tests/audit_identity.rs` (5)

| Test | Pins | Would catch |
|---|---|---|
| `h0_history_independence` | two edit histories -> identical result AND counters | any hidden cross-update state |
| `h0_complete_post_coverage` | unique_post == post.len() exactly; union = one interval; old bytes 0; blocks/nodes hand-counted | partial parse masquerading as full; reuse lying |
| `h0_attribution_profile` | nodes_reused Known(0) measured; fallback/metadata/gauges N/A | mislabeled N/A vs measured-zero |
| `h0_completed_state_purity` | normalize/checksum pure; node_count tree-walk agrees; second update consumes completed state | post-timer work hiding in complete() |
| `h0_utf8_and_boundaries` | CJK byte-3 insert, byte 0, EOF, empty doc, non-boundary rejection | coordinate-space conflation |

## H1 — `mechanisms/block-local/tests/audit_identity.rs` (10)

| Test | Pins | Would catch |
|---|---|---|
| `h1_safe_local_edit_exact_counters_and_subfull_coverage` | blocks=1, rebuilt=4, reused=2, fallback=0, old=0, post union [9,22) | hidden full parse; suffix rebuild pretending to be local |
| `h1_probe_plus_fallback_counts_fallback_once_and_keeps_divergence` | fallback Known(1) exactly once; blocks=2 merged paras; total > unique (cumulative honesty) | double-counted or dropped fallback work |
| `h1_f1_def_fallback` | fallback=1; blocks=4 incl. the DISCARDED region's skeleton | discarded work silently dropped |
| `h1_f4a_beyond_edge_terminator` | fallback=1; blocks=3 = 1 discarded + 2 | missing terminator guard |
| `h1_f6_fence_boundary_both_sides` | bounded side: no fallback (blocks=1); unbounded side: fallback=1 | asymmetric fence guard absence |
| `h1_insertion_gap_mapping` | blocks=1, reused=2, rebuilt=6, metadata=12 | gap mapping arithmetic |
| `h1_break_restore_round_trip` | BREAK then RESTORE == original; Property C determinism | history-dependent results |
| `h1_fresh_state_isolation_under_interleaving` | two docs interleaved, no cross-talk | shared state across documents |
| `h1_cjk_byte_coordinates` | byte-exact CJK edit | UTF-16/char confusion |
| `h1_completed_state_pure_query` | pure normalize/checksum | hidden work in complete() |

## H2 — `mechanisms/fragment-reuse/tests/audit_identity.rs` (7)

| Test | Pins | Would catch |
|---|---|---|
| `h2_exact_reuse_of_the_stable_suffix_run` | run {big, after} reused=4; interior [20,150) UNscanned; margin reads confined to the run's tail | byte-diff rescan masquerading as fragment take |
| `h2_fence_context_change_refuses_all_reuse` | reused=0, blocks=2 under changed forward state | context-blind reuse |
| `h2_rematerialization_splits_structure_reuse_from_semantic_rebuild` | env change: reused=4 (ref-insensitive members only); rebuilt includes lit's payload derived from the H0 tree; blocks=4 | payload blindly reused (wrong tree) or whole run dropped (no repair) |
| `h2_mingap_boundary_128_both_sides` | es=128 -> left fragment exists (reused=2); es=127 -> reused=0 | adaptive/tuned threshold |
| `h2_break_restore_and_determinism` | fence-closer BREAK/RESTORE + full `WorkCounters` equality | history dependence |
| `h2_cjk_byte_coordinates` | CJK fragment take | coordinate confusion |
| `h2_completed_state_pure_query` | purity | — |

## H3 — `mechanisms/old-tree-subtree-reuse/tests/audit_identity.rs` (8)

| Test | Pins | Would catch |
|---|---|---|
| `h3_exact_reuse_and_change_flags` | reused=4, rebuilt=2, blocks=1; old=[8,15)=7B; post=31/31 (margin closure, F-4); metadata=13; prepared scanned=3/patched=1/flags=1; new-state flags=0 | damage-map invisibility; flag leakage into new state; undeclared margin reads |
| `h3_prefix_insertion_relocates_the_suffix` | gap edit: p2 taken at NEW pos 16 (reused=2); p1 margin-refused conservatively (blocks=2, rebuilt=4); spans non-overlapping increasing | stale-position reuse (wrong tree); over-permissive gap handling |
| `h3_fence_closer_removal_refuses_all_reuse` | same bytes, changed forward state: reused=0, blocks=1, flags=1 (fence only) | byte-diff reuser taking the tail |
| `h3_definition_environment_repair_uses_the_own_table` | def edit: reused=2, rebuilt=3 (lit payload rebuilt, big kept), blocks=1 | oracle input (anti-oracle); payload reuse under changed table |
| `h3_multi_entry_deletion_clamps_and_reparses` | clamp arithmetic: reused=0, blocks=1, rebuilt=2; result == H0 | clamp crash / stale ranges |
| `h3_break_restore_and_determinism` | BREAK/RESTORE + full counter equality | — |
| `h3_cjk_byte_coordinates` | CJK patch path | — |
| `h3_completed_state_pure_query` | purity | — |

## H4 — `mechanisms/restart-convergence/tests/audit_identity.rs` (10)

| Test | Pins | Would catch |
|---|---|---|
| `h4_convergence_reuses_prefix_and_suffix_with_exact_gauges` | restart=5, convergence=10; reused=4 via `Arc::ptr_eq`; metadata=15; post=[9,20)+[30,31)=12 | fake sharing (clone counted as reuse); gauge drift |
| `h4_predicate_e_refuses_interruptor_adjacent_splice` | (e) refusal: head' fresh, take pushed to 18 (convergence=18); WRONG-tree case for a margin-less predicate | premature convergence with open paragraph |
| `h4_fence_state_change_swallows_the_tail_then_restores` | BREAK: reused=0 (no consult inside fence); RESTORE: reused=0 (no vouching checkpoint); Property C | fence-body reuse; restore asymmetry |
| `h4_container_join_converges_at_next_checkpoint` | list join: fresh list (blocks=3), take at tail (convergence=5), base_shift −4 | container mishandling |
| `h4_restart_boundary_backs_up_when_the_edit_reaches_the_boundary_line` | restart backed 1 -> 0; OLD coverage [8,17)=9B; restart=12 | boundary-line continuation bug (wrong tree) |
| `h4_definition_damage_restarts_at_zero_with_generation_bump` | gen 0->1; reused=0; restart=15; convergence=32; fallback N/A; probe short-circuit (no probe event) | fallback mislabeling; gen not bumped |
| `h4_assembled_table_detects_the_fence_swallowed_definition` | NO def byte edited, NO `]: ` inserted -> assembled-table detection; discarded pass counted (blocks=2 total) | unsound env detection (stale payload reuse = wrong tree) |
| `h4_restart_point_positions` | es=0 (restart=0 measured; suffix shared, base_shift +6); EOF continuation (joined para = 1 merged Text run); inside-fence edit (whole fence fresh, take right after) | restart selection drift; mid-fence reuse |
| `h4_cjk_byte_coordinate_distances` | gauges in BYTES: 15/20 around 3-byte chars | char-count gauges |
| `h4_completed_state_pure_query` | purity + repeat determinism | — |

## Metamorphic / cross-cutting coverage (§39)

BREAK→RESTORE round trips: H1, H2, H3, H4 (H0 trivially). Property C
(fresh-state repeat == incremental repeat, full counter equality): H1,
H2, H3, H4. Interleaved fresh-state isolation: H1 explicit; all suites
construct states via `full_parse` per case. Completed-state purity: all
five. CJK: all five. H0-oracle equality (`assert_matches_h0`):
every update case in every suite (checksum + NORMALIZED-RESULT-v1
span validation against the post source).
