# H2 fidelity contract — FRAGMENT_REUSE

Status: **FROZEN GATE-A FIDELITY CONTRACT (#76) — VERDICT: MATERIAL_DEVIATION**

```text
HORSE = H2 — FRAGMENT_REUSE (lezer-inspired)

LOCAL_MECHANISM_AUTHORITY =
  research/benchmarks/markdown-ast-update/mechanisms/fragment-reuse/
  (crate markit-mdbench-fragment-reuse)
  protocol/R5-HORSE-CORRECTNESS-PARITY.md §7 (+§2, §12 amendments)

DONOR_SOURCES =
  @lezer/common  1.5.2  de5f96276a2954c249de1475e8b03f79c20d9ce4
  @lezer/lr      1.4.8  f81d6a25c3482aa7fc12434e9adea9d75c56ad08
  @lezer/markdown 1.6.3 9942d7ce41d734d743cbdb48177dddc1975fdc5c
  (github.com/lezer-parser/{common,lr,markdown}; see DONOR-SOURCES.md
  for the org-rename provenance note)
```

## D1–D12 summary (full detail in `../reviews/gate-a-r2-h2-2026-09-28.md`)

| Dim | Classification | Essence |
|---|---|---|
| D1 retained representation | MATCH (family) | FTree of top-level slots + `Fragment{new_start,new_end,to_old,open_start,open_end}` — same shape as donor Tree+offset+open flags (+ benchmark-required facts, see D9) |
| D2 edit/damage propagation | DECLARED_SIMPLIFICATION + 1 undeclared asymmetry | single-edit split/drop-left/delta/open-flags identical to donor; UNDECLARED: right piece dropped when old tail < 128 — donor NEVER drops the after-last-change piece (≤127B, conservative) |
| D3 candidate discovery | **MATERIAL_DEVIATION** | donor: forward-only per-LINE cursor discovering candidates at the LIVE composite level, taking mid-container sibling runs (verified upstream: 25/30 ListItems identity-shared after edit inside item 3); local: every consult searches from the TOP-LEVEL slot list; candidates must line-align to top-level entries; nested descent effectively unreachable → 0 items shared on the same case |
| D4 eligibility/context compatibility | faithful core + declared margins + missing donor guards | ContextKey structural equality strictly stronger than donor rolling hash (declared); declared conservative margins (blank-line take margin, left-edge continuation margin, open-edge one-block exclusion); missing donor guards (NotLast, trailing-partial-line, openEnd−1) → bounded over-reuse |
| D5 reuse/reparse granularity | **MATERIAL_DEVIATION** | leaf-block granularity matches; container-interior granularity does not — donor takes item-level runs inside damaged containers, local cannot; frozen §7 claims "not a strict subset" — the implementation does not deliver the operative Lezer shape |
| D6 candidate rejection | MATCH | natural degradation per line; fragments retained; retried every line |
| D7 fallback/progress | MATCH | empty table → plain full parse; no fallback concept |
| D8 coordinate/edit mapping | MATCH (family) | to_old delta mapping; old tree immutable; absolutes derived at read time |
| D9 semantic/global dependency | NON_MATERIAL_ADAPTATION | ADDS reference invalidation + rematerialization the donor never does — benchmark-necessitated, declared, honest accounting |
| D10 retained-state maintenance | MATCH (family) | fragment table rebuilt per edit; new state registered as one whole-document fragment (addTree analogue) |
| D11 indexing vs enumeration | **MATERIAL_DEVIATION** | donor: amortized-linear forward cursor; local: EVERY consult rebuilds the absolute top-level entries vector (O(T) Arc clones) + linear position scan; worst case O(B²) in refusal-heavy regimes — undeclared ("no query index" does not cover it) |
| D12 lifetime/retirement | MATCH | fresh wrapper spine with Arc-shared members |

```text
EXPERIMENT_SPECIFIC_ADAPTATION =
  BENCH-GRAMMAR-v1; common Source/Edit contract (single canonical edit —
  donor change lists not modeled, substrate contract); normalized result
  contract; reference-environment clause + rematerialization (declared).

INTENTIONALLY_OMITTED =
  generated LR tables / lezer-generator; JS runtime object model;
  parseMixed nested-language plumbing; CodeMirror viewport scheduling;
  the exact rolling-hash function (replaced by strictly-stronger
  structural ContextKey).

PERFORMANCE_RELEVANT_DEVIATIONS =
  P1-class (see corrective issue):
    1. mid-container candidate discovery/granularity (D3/D5) — reparse
       work proportional to damaged-container size where donor reuses it;
       direction AGAINST H2 on container axes.
    2. per-consult O(top-level-blocks) discovery scan (D11) — asymptotic
       discovery-cost class change; worst case O(B²) in refusal-heavy
       regimes; undeclared.
  P2-class (declared, conservative, bounded or run-bounded):
    3. blank-line take margin (forfeits interruptor-boundary runs)
    4. left-edge continuation margin (can drop entire left fragment)
    5. open-edge one-whole-block exclusion vs donor ~one line
    6. missing NotLast/trailing-partial-line/openEnd−1 guards → bounded
       over-reuse (FOR H2)
  P3-class: symmetric minGap application (right-piece drop, ≤127B);
  single-edit contract; moveTo 1-char step-back not modeled.

CHALLENGE_CASES =
  H2-F1 aligned fragment + compatible context — MATCH on core
  H2-F2 fragment overlaps edit — table lifecycle MATCH; left-edge
    behavior MATERIAL conservative divergence (whole left fragment
    dropped vs donor reuse)
  H2-F3 unchanged bytes + incompatible context — MATCH
  H2-F4 edit shifts retained fragment — MATCH (delta/offset faithful)
  H2-F5 multiple candidate fragments — DECLARED_SIMPLIFICATION
    (single-edit contract); D3/D11 deviations govern discovery cost
  Evidence: upstream markdown-consumer lane executed end-to-end at the
  pinned npm versions (scripts + outputs in challenge-evidence/r2-h2-lezer/);
  local lane via h2_gate (17/17) + scratch probe (Arc identity, counters,
  ==clean). NO timing.

UPSTREAM_QUALITATIVE_CROSS_CHECK =
  PARTIAL — markdown consumer COMPLETE (runtime); LR lane COMPLETE at
  pinned-source reading level (running it needs a lezer-generator-built
  parser — not attempted).

CLAIM_BOUNDARY =
  DEFENSIBLE: "a lezer-inspired, soundness-conservative fragment-table-
  gated whole-top-level-block reuse model with structural ContextKey
  vouching, under the #22 semantic core."
  NOT DEFENSIBLE AS WRITTEN: frozen R5 §7 FIDELITY_BOUNDARY's "taking
  runs of sibling blocks at nested levels reproduces Lezer markdown's
  block-run reuse … not a strict subset" — the implementation cannot take
  mid-container sibling runs (empirically 25/30 items upstream vs 0
  locally), and minGap=128 is applied symmetrically while @lezer/common
  applies it only before/between changes. Results attributed to
  "Lezer-style fragment reuse" on container-damage, tight-boundary, and
  reference-dense workloads carry an ANTI-H2 bias relative to the donor
  mechanism and must be labeled as such.

FIDELITY_VERDICT =
  MATERIAL_DEVIATION
  (correctness is not in question — 17/17 h2_gate tests pass and every
  probe equals the H0 clean parse; the deviation is mechanism-level)
```

Gate-A consequence: H2 is a Gate-A BLOCKER. Per #76 §13 the horse may not
make donor-level performance claims until either (a) the cursor gains
live-level candidate discovery and the claim boundary is re-frozen, or
(b) the claim boundary is rewritten to "conservative strict-subset-with-
stronger-vouching" and container-axis results are bounded accordingly.
Corrective issue: see FIDELITY-MATRIX.md and the #76 status record.
Repair requires a separate PR + fresh fidelity re-review (not this PR).
