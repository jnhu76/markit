# PHASE-MAP.md — H4-LARGE-N-CAUSE-1 (Issue #50 §5)

**Status: FROZEN before any formal diagnostic row was collected.**

Frozen at base `334eea6201fc0258e35a7c5b21feb722641ddcbd`, diagnostic copy
`h4diag/src/alg.rs`. This map was written against the actual source control
flow of the frozen H4 `update()`/`prepare_update()`, not against the
suggested mapping in the task text; where the two differ, the difference is
recorded in §4 below.

Scope of this map: the **normal path** of the #50 workload (local text edit,
no definitions, no fences, no containers, no restart). The two
`restart_at_zero` early-return paths are *not* decomposed here; when they
occur the observation is marked `RESTART_AT_ZERO` and its phase row is not
used as a normal-path sample. They do not occur in any #50 cell (preflight:
`definition_changing` is false for all eight cells).

---

## 1. Disjoint phases

Seven phases, each opened and closed exactly once per update. No phase is
entered while another is open, so `sum(disjoint phases)` is a legitimate
arithmetic quantity. `U_PHASE` (the frozen runner's outer
`T_prepare + T_native`) is the enclosing window; the phases decompose it.

| Phase | Source region | Inclusive of | Excludes |
|---|---|---|---|
| `P1_prepare_damage_restart` | entire `prepare_update` body: restart `.rposition` predicate loop, restart-boundary margin back-up loop, full damage scan | the two `record_source_inspection` calls the margin loop makes and the `memchr_lf_reported_in` margin read | — |
| `P2_forward_parse_and_convergence` | `parse_region_with_hook(post, r, post.len(), …)` plus the post-parse `blank_checks` reporting loop | **every convergence consult** (the hook runs inside the scanner), the scanner's own source reads, the take/consultation extraction | the `definition_changing` probe (outside all phases, see §3) |
| `P3_prefix_pair_assembly` | the `for (s, cp) in old_state.blocks[..restart_slot]` loop | `Arc::clone`, `ContextKey::clone`, `pairs.push` | — |
| `P4_definition_collect_table_compare` | `collect_defs_skel` over retained prefix + fresh/suffix, `ref_table`, assembled-vs-retained comparison | the `Adefs` guard's fresh-region `skel_has_def` check when the guard is active | — |
| `P5_fresh_materialization_and_suffix_assembly` | the `for sk in &rp.blocks` loop: `materialize_one_with_sink` for fresh blocks, `Arc::clone` + key clone + push for the converged suffix | fresh inline materialization, suffix retargeting | — |
| `P6_pairs_to_slots_checkpoints` | the `for (slot, cp) in pairs` loop plus the two `Vec::with_capacity` calls that create `slots` and `checkpoints` | `line_position()`, checkpoint position rewrite, key clone for fresh checkpoints | — |
| `P7_seal_and_retirement` | explicit `retire_old_state(old_state, …)` followed by the `H4DiagPending` construction | destruction of the consumed old state (its two vectors, every `Arc` handle, every `ContextKey`, the definition vector) | `complete()` (a move-only no-op) and everything the runner does after the timer |

### 1.1 Inclusive sub-measure (never summed)

| Sub-measure | Meaning |
|---|---|
| `hook_inclusive_sub_measure` | the convergence-hook body only (`Cursor::consult`), which runs **inside** the scanner and therefore inside P2. It is recorded separately so hook cost is visible, and it is **never** added to the disjoint sum. It is a sub-interval of P2, not a sibling of it. |

## 2. Closure arithmetic

Per **individual observation** (never across p50s):

```text
U_PHASE_i          = T_prepare_i + T_native_i          (frozen runner, outer)
disjoint_sum_i     = P1 + P2 + P3 + P4 + P5 + P6 + P7  (this map)
phase_residual_i   = U_PHASE_i - disjoint_sum_i
```

`phase_residual` is reported raw and may be negative. It contains, by
construction:

- the runner's own work inside `T_prepare`/`T_native` outside any phase
  (`black_box`, `catch_unwind`, the `H4DiagPending` move, the
  `Observed`/sink gauge writes at the end of `update`);
- the `definition_changing` probe (`post[es..ee_new].windows(3)`) and its
  `record_source_inspection`;
- the phase instrumentation's own overhead: 14 clock reads per update
  (2 per scope × 7 scopes), reported as `phase_clock_reads`.

`sum of per-phase p50s` is never used as a whole-update p50.

## 3. Code that is deliberately OUTSIDE every phase

| Region | Why it is unphased |
|---|---|
| the `definition_changing` pre-parse probe | it is a source-local predicate that runs before the phase structure begins; it is O(1) (8 bytes) and its cost is folded into `phase_residual` rather than mis-attributed to P1 or P2 |
| `cx.sink.*` reporting at the end of `update` | sink calls under `NoopWorkSink` are no-ops; charging them to a phase would misreport them as work |
| `complete()` | move-only; the runner calls it inside `T_native`, and it is inside `U_PHASE` but outside `P1..P7`, so it lands in `phase_residual` |

## 4. Deviations from the task text's suggested mapping

The task text suggests `P7 return/seal_and_internal_retirement` and warns:
*"If retirement cannot be isolated honestly, do not pretend P7 equals
retirement. Record residual separately."*

In the frozen source the old state's destructor runs at function scope exit,
which no probe can bracket. This map therefore makes the retirement
**explicit and bracketed**: `retire_old_state` is called after the assembly
loops and before the pending value is constructed (module header of
`h4diag/src/alg.rs`, deviation 2). Between those two points the frozen source
performs only moves, so the same objects are destroyed in the same order.

Consequences recorded honestly:

- `P7` **is** retirement + sealing for the normal path, and this is
  *established by construction* (one explicit `drop` of one owned value),
  not inferred from a residual.
- The implicit drops that remain outside P7 are exactly: the `pairs` vector
  buffer (already consumed by P6), the `Cursor`/hook box (dropped at the end
  of P2), and the `RegionParse`'s own definition table. None of them is the
  consumed old state.
- `Adrop` (§10 of the issue) moves the same value to a post-timer owner,
  which is what makes the P7 quantity falsifiable by ablation.

The task text also suggests splitting `prepare` into `restart/margin lookup`
and `damage scan`; both are inside P1 here. They are separated by
**counters**, not by a second timer, because a second timer would double the
probe count in the cheapest phase (at 128 KiB the whole update is ~90 µs and
P1 is a small fraction of it). `restart_predicate_evaluations`,
`restart_margin_steps` and `damage_records_visited` carry the split.

## 5. Perturbation rule

`U_PHASE / U_PLAIN` is reported per N and per session. If it exceeds
`max(5%, A/A noise)` or changes the scaling trend, the lane is marked
`PHASE_PERTURBED`; it stays descriptive, and no phase share is transferred
to U_PLAIN. The two are measured by **different executables**
(`mdbench-h4diag-phases` vs the original H4 through
`mdbench-h4diag run-plain`), and both executables' SHA256 are recorded.
