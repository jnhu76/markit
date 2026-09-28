# H4 fidelity contract — RESTART_CONVERGENCE

Status: **FROZEN GATE-A FIDELITY CONTRACT (#76)**

```text
HORSE = H4 — RESTART_CONVERGENCE (wagner-graham / swift-inspired
        benchmark model; explicitly a multi-donor composite)

LOCAL_MECHANISM_AUTHORITY =
  research/benchmarks/markdown-ast-update/mechanisms/restart-convergence/
  (crate markit-mdbench-restart-convergence)
  protocol/R5-HORSE-CORRECTNESS-PARITY.md §9 (freeze + required fidelity note)

DONOR_SOURCES =
  Wagner & Graham — TOPLAS 20(5) 1998, DOI 10.1145/293677.293678;
    dissertation CSD-97-946 Ch. 6 (paper-only donor; R2 record + PDF
    identity re-check 2026-09-28)
  swift-syntax 604.0.0 = 050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf
    (Sources/SwiftParser/IncrementalParseTransition.swift, TopLevel.swift,
    Parser.swift LookaheadRanges)
  tree-sitter v0.27.0 = 6070dbfefd326bd735e5683eb128cc1b57dad0c0
    (state-agreement gating observations)
  @lezer/markdown 1.6.3 = 9942d7ce41d734d743cbdb48177dddc1975fdc5c
    (Markdown block-boundary / NotLast convergence guards)
```

## What is borrowed from each donor (kept distinct — the four donors do
NOT share one convergence authority)

| Concept | Borrowed from | Local realization |
|---|---|---|
| restart before damage, forward reparse | Wagner & Graham (shape) | restart at the damaged block's checkpoint (or backed-up boundary) |
| checkpoint table + consult-at-position with live state | swift-syntax (mechanics) | 1:1 `Vec<Checkpoint{position, ContextKey, gen}>`; SpliceHook consult; binary search |
| state-agreement convergence gate | tree-sitter (content) | ContextKey equality at the mapped checkpoint |
| block-boundary / blank-line / continuation margins | @lezer/markdown (guards) | blank-line paragraph margin; restart-boundary continuation margin |
| suffix-to-EOF one-shot take (no interleaving) | W&G (suffix-as-input), NOT Swift | single convergence splices the whole suffix |
| speculative validation + rollback | NOT borrowed (declared) | check-then-take conservative predicate; nothing speculative to roll back |
| byte-local predicate | NOT borrowed (declared) | structural state equality, never byte equality |
| definition-generation restart | benchmark-model-defined | required by the #22 semantic core; no donor has it |

## D1–D12 summary (full detail in `../reviews/gate-a-r4-h4-2026-09-28.md`)

| Dim | Classification | Essence |
|---|---|---|
| D1 retained representation | MATCH (composite) | block slots + 1:1 checkpoint table + defs + gen (Swift-checkpoint-table analogue + W&G retained structure; no fragment table — declared non-goal; LookaheadRanges deliberately not adopted) |
| D2 edit/damage propagation | DECLARED_SIMPLIFICATION | per-update LINEAR scan of all old block slots + linear reverse checkpoint scan (donors: lazy/edit-path/fragments/version-marks); counted in counters; grows with block count (undeclared as donor-divergent in R5 §9 — P2-3) |
| D3 candidate discovery | MATCH | line-start consult with live ContextKey; binary search over checkpoint positions; no arbitrary old-tree search |
| D4 eligibility/continuation/state compatibility | MATCH to declared benchmark-model-defined authority | (a) exact mapped checkpoint, (b) ContextKey equality, (c) generation [vacuous by construction — P2-2], (d) q ≥ damaged_end, (e) blank-line margin; fence-state structural. Caveat: (b)'s decisive role UNWITNESSED (P2-1) — no false convergence ever occurred, but carried by the full guard stack |
| D5 reuse granularity | MATCH | whole top-level block subtree incl. inline semantics, Arc identity; prefix retained; suffix in ONE convergence to EOF (W&G shape; Swift interleaving consciously not adopted) |
| D6 candidate rejection | DECLARED_SIMPLIFICATION | consult returns None → parse continues (continuous degradation); W&G rollback absent — declared model-defined |
| D7 fallback/progress | MATCH | no fallback mode; EOF path verified; restart-at-zero reuses the same forward machinery, honestly counted |
| D8 coordinate/edit mapping | MATCH | single-edit delta mapping; strict beyond-inserted-region check; per-block base_shift; single-edit = R0 canonical contract |
| D9 semantic/global dependency | NON_MATERIAL_ADAPTATION | definition-changing damage → restart at zero + gen bump + re-registration (two detection paths incl. assembled-table fence-swallow case); benchmark-required |
| D10 retained-state maintenance | MATCH | checkpoint table re-derived per update; defs re-collected per update (real O(state) cost — the #50 Adefs ablation exists because of it) |
| D11 indexing vs enumeration | NON_MATERIAL_ADAPTATION | binary search vs donors' monotone cursor walks; per-line consult density (Lezer-like); counted |
| D12 lifetime/retirement | MATCH | old state destroyed at update end; Arc sharing; discarded-forward-pass counted |

```text
EXPERIMENT_SPECIFIC_ADAPTATION =
  BENCH-GRAMMAR-v1; common Source/Edit contract (single canonical edit);
  normalized result contract; definition-generation restart machinery
  (no donor has it; required by the #22 semantic core).

INTENTIONALLY_OMITTED =
  W&G speculative validation + delayed right_breakdown rollback (no LR
  states exist for Markdown — its safety theorem does not transfer);
  Swift byte-local predicate and interleaved per-item reuse; Swift
  LookaheadRanges; Lezer rolling hash (replaced by strictly-stronger
  structural equality); tree-sitter skip-based old-tree cursor.

PERFORMANCE_RELEVANT DEVIATIONS (all exposed by frozen counters):
  1. damage detection = full linear scan over retained entries + reverse
     checkpoint scan per update (grows with block count, independent of
     edit size)
  2. per-update definition re-collection + table compare (O(state))
  3. one-take-to-EOF suffix: forfeits aligned identical blocks inside the
     reparsed region
  4. per-line consult density + O(log C) search per consult
  5. restart-margin separator scans per back-step (bounded)

CHALLENGE_CASES =
  H4-F1 valid nearby restart + convergence — PASS (probes F1a-c + W1)
  H4-F2 damaged restart support → earlier restart — PASS (committed test
    h4_restart_boundary_backs_up_when_the_edit_reaches_the_boundary_line)
  H4-F3 candidate before damage crossed → rejected — PASS
  H4-F4 no interior convergence → correct EOF/progress path — PASS
    (incl. restart-at-zero + assembled-table fence-swallow reproduction)
  H4-F5 state/context incompatibility → no false convergence — PASS on
    behavior (6 constructions, all == H0, convergence deferred); the
    state-agreement clause's own decisive role is UNWITNESSED (P2-1)

UPSTREAM_QUALITATIVE_CROSS_CHECK =
  COMPLETE for tree-sitter / swift-syntax / lezer-markdown (pinned
  source read line-level; donor model frozen before horse inspection —
  challenge-evidence/r4-h4/donor-model-frozen.md);
  PARTIAL for W&G (no runnable donor — paper-level cross-check; no
  executable comparison fabricated).

CLAIM_BOUNDARY =
  "H4 is our controlled restart/convergence model for the explicitly
  defined composite: W&G restart-before-damage + one-shot suffix-to-EOF
  + Swift checkpoint-consult mechanics + tree-sitter state-agreement
  content + Lezer Markdown boundary guards + a model-defined definition-
  generation restart. The convergence authority is parser-STATE agreement
  at block boundaries — NOT W&G batch-parser equivalence and NOT Swift's
  byte-local predicate; neither anchor is literally reproduced." This is
  what frozen R5 §9 and the crate header state. A later H4/Horse-A
  comparison is not a strawman provided it names THIS composite rather
  than "W&G/Swift performance".

FIDELITY_VERDICT =
  FAITHFUL_MECHANISM_MODEL
```

Open findings (non-blocking, tracked): P2-1 convergence clause (b)
decisive role unwitnessed (add a (b) witness or soften R5 §9 wording);
P2-2 clause (c) vacuous + R5 §9 semantic-dependency narrative
misdescribes the implemented code path (fast-path/assembled-table
restart); P2-3 damage-detection linear scan divergence undeclared;
P3 wording items.
