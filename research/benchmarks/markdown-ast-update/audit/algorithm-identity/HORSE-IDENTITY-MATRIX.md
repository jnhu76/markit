# HORSE-IDENTITY-MATRIX

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

Question: does each implementation execute the algorithm its frozen
model claims — not merely produce correct trees?

Verdict scale: PASS / FAIL / AMBIGUOUS. "PASS" requires mechanism-level
code inspection PLUS exact-counter pinning (hand-derived) PLUS at least
one adversarial case that a wrong algorithm of the same family would
fail.

## H0 FULL_REBUILD

| Frozen claim | Code reality | Verdict |
|---|---|---|
| Every update = clean full parse; no retained structure | `update` (full-rebuild lib.rs L249–L268) parses the whole post source and discards the old state; no cross-update state exists in `H0State` | PASS `CODE_INSPECTION_SUPPORT` |
| `nodes_reused = 0` is a MEASURED zero, metadata/fallback/gauges N/A | `report_attribution` L153–L185 | PASS `TEST_SUPPORT` (`h0_attribution_profile_*`) |
| History independence (two edit histories to the same text agree in result AND counters) | No state to diverge; pinned by `h0_history_independence` | PASS `TEST_SUPPORT` |

Hidden-reuse check: H0 cannot reuse — the state type carries no
retained nodes; the counters cannot lie about reuse because the value
is a constant set by `report_attribution`.
`MECHANICALLY_PROVEN_BY_INVARIANT` (type-level).

**H0_ALGORITHM_IDENTITY = PASS**

## H1 BLOCK_LOCAL_REPARSE

| Frozen claim | Code reality | Verdict |
|---|---|---|
| Top-level tiling; local damage -> region reparse + prefix pass-through + delta-shifted suffix reconstruction | damage scan + insertion-gap mapping in `prepare_update` (L261+); `shift_entry_owned`/`shift_node_owned` (L750–L810) rebuild suffix entries with shifted spans — the suffix is RECONSTRUCTED (owned nodes), not shared | PASS `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT` (exact counters: blocks=1, rebuilt=4, reused=2 on the canonical local edit) |
| Fallback exists, counted exactly once, discarded work still counted | `total_fallback` L223–L260: `record_fallback_to_full()` once; discarded region blocks counted into blocks_reparsed/nodes_rebuilt as skeleton-only | PASS `TEST_SUPPORT` (`h1_probe_plus_fallback...`: fallback Known(1); `h1_f1_def...`: blocks=4 incl. discarded 1) |
| Guards F1–F6 fire on the frozen conditions | `guards_fire` L585–L664 (fence boundary), `continuation_pair`/`would_continue` L665–L749 | PASS `TEST_SUPPORT` (F1 def fallback, F4(a) beyond-edge, F6 bounded/unbounded asymmetry) |
| No hidden whole-document parse on the safe path | safe-path counters are sub-full by exact derivation (post union [9,22) on DOC3), fallback Unknown->0 | PASS `TEST_SUPPORT` + `PARSE-RANGE-TRACE.md` |

Adversarial discriminator: the F6 bounded/unbounded pair — a mechanism
that always reparses locally (no fence guard) or always falls back would
produce different fallback/blocks values; both are pinned exactly.

**H1_ALGORITHM_IDENTITY = PASS**

## H2 FRAGMENT_REUSE

| Frozen claim | Code reality | Verdict |
|---|---|---|
| Fragment table over byte-interval fragments; minGap=128 both cut sides | `MIN_GAP` L59; left cut in `prepare_update` (`old.len() - ee >= MIN_GAP`), right cut in `update` (`es >= MIN_GAP`); boundary pinned from BOTH sides (es=128 -> reused=2; es=127 -> reused=0) | PASS `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT` (`h2_mingap_boundary_*`) |
| Splice-hook take vouched by ContextKey + windows + margin | `Cursor::consult` L642+ (margin buffer -> fragment lookup -> find_run with window + ctx + ref clause) | PASS `CODE_INSPECTION_SUPPORT` |
| Interior of a taken run is NEVER scanned | exact test: interior [20,150) clean; consult-time margin reads confined to the run's final line (R5-CORRECTIVE-2 margin reads) — recorded as an attribution observation, not a violation | PASS `TEST_SUPPORT` |
| Reference-environment clause via rebuilt-table comparison; affected payload rematerialized, structure kept | env comparison L444; `mentions_reference` L1054 (has_ref short-circuit or '[' probe); `rematerialize` L1079 keeps `Arc` descendants; accounting `reused - rematerialized_members + rematerialized_kept` L469 | PASS `TEST_SUPPORT` (`h2_rematerialization_splits...`: reused=4 = the two ref-insensitive members only; rebuilt includes lit's payload derived from the H0 tree) |
| minGap boundary is frozen, not adaptive | constant, no configuration seam | PASS `MECHANICALLY_PROVEN_BY_INVARIANT` (const) |

Adversarial discriminator: the rematerialization split — a mechanism
that either drops the whole run on env change or reuses the payload
blindly would fail the exact reused/rebuilt split.

**H2_ALGORITHM_IDENTITY = PASS**

## H3 OLD_TREE_SUBTREE_REUSE

| Frozen claim | Code reality | Verdict |
|---|---|---|
| Patch-path damage flags on the OLD tree (copy-on-write ancestry), gap edits shift the next entry's gap | `patch_tree` L496–L662; observability: `H3Prepared{scanned_entries, patched_nodes, tree}` public and pinned exactly (scanned=3, patched=1 on the canonical edit) | PASS `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT` |
| Continuation margins: prev reads OLD separation, next reads POST separation; >= 2 LFs keeps the neighbor reusable | L549–L588; exact pin: OLD [8,15) = 7 bytes on the canonical edit (2 LFs -> p1 survives); empty separation -> conservative refusal (prefix-insertion test: p1 margin-marked, blocks=2) | PASS `TEST_SUPPORT` |
| Line-aligned forward cursor; changed -> descend; ctx mismatch -> descend; stale positions refuse naturally | `consult`/`find_run`/`search_level` L782–L917; fence-closer test: reused=0 with exactly ONE flag (the fence) — the tail's refusal comes from the fence ContextKey, not from flags | PASS `TEST_SUPPORT` |
| Relocation: gap edit moves derived positions; suffix taken at NEW coordinates; no stale ranges | prefix-insertion test (H0 equality + non-overlapping increasing spans) | PASS `TEST_SUPPORT` |
| Own-table reference repair (no oracle input) | `mentions_reference` L1114 (subtree '[' probe), `rematerialize` L1140, assembled at `assemble_level` L1364–L1375 | PASS `TEST_SUPPORT` (`h3_definition_environment_repair...`) + static isolation (`MECHANICALLY_PROVEN_BY_INVARIANT`) |
| Flagged nodes are never taken -> assembled state carries no flags | cursor refuses `changed` nodes (L875); pinned: `state.tree().changed_count() == 0` while `prepared.tree.changed_count() == 1` | PASS `TEST_SUPPORT` |

Adversarial discriminator: the fence-closer case (same bytes, changed
forward state) — a pure byte-diff reuser would take the unchanged tail;
H3 refuses it structurally.

**H3_ALGORITHM_IDENTITY = PASS**

## H4 RESTART_CONVERGENCE

See `H4-CONVERGENCE-PREDICATE.md` for the clause-level audit; summary:

| Frozen claim | Code reality | Verdict |
|---|---|---|
| Checkpoint at every top-level block start; restart at/before the damage; forward parse from r | `registers` L843–L852; selection L334–L342; `parse_region_with_hook(post, r, post.len())` L481 | PASS `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT` (restart points 0 / mid / EOF / inside-fence all pinned) |
| Convergence predicate (a)–(e), all clauses live | `Cursor::consult` L739–L782; (e) refusal pinned by the interruptor-adjacent case; (a) pinned by distances | PASS `TEST_SUPPORT` |
| Prefix AND suffix reuse are real reuse (shared `Arc<RetainedBlock>`) | L500–L524 / L597–L620; proven with `Arc::ptr_eq` in two tests + `base_shift` retargeting | PASS `TEST_SUPPORT` + `MECHANICALLY_PROVEN_BY_INVARIANT` (Arc identity) |
| Definition damage -> RESTART at zero at gen+1 (not a fallback; gauges Known) | `restart_at_zero` L268–L301; fallback N/A kept | PASS `TEST_SUPPORT` (`h4_definition_damage...`: gen 0->1, restart=15, convergence=32, fallback N/A) |
| Sound assembled-table detection (fence-closer case) + cumulative discard accounting | L564–L590 | PASS `TEST_SUPPORT` (`h4_assembled_table...`: blocks=2 = discarded 1 + restart 1) |
| Gauges measured on EVERY update; restart-boundary backup reported | L662–L675; L344–L400 (OLD reads (8,9)+(9,17) pinned = 9 bytes) | PASS `TEST_SUPPORT` |

Adversarial discriminator: the predicate-(e) interruptor case — a
convergence mechanism without the paragraph margin would splice with an
open paragraph and deliver a WRONG tree (not just wrong counters); the
refusal is pinned with convergence_distance=18 (not 9).

**H4_ALGORITHM_IDENTITY = PASS**

## Overall

```text
H0_ALGORITHM_IDENTITY = PASS
H1_ALGORITHM_IDENTITY = PASS
H2_ALGORITHM_IDENTITY = PASS
H3_ALGORITHM_IDENTITY = PASS
H4_ALGORITHM_IDENTITY = PASS
```

No mechanism defect was found; therefore per the audit contract the
audit branch records observations only (see `FINDINGS.md`) and does NOT
modify production code.
