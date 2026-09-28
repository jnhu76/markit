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

---

## Corrective re-evaluation — 2026-09-28 (#79)

The initial Gate-A verdict above (`MATERIAL_DEVIATION`, reviewer R2) is
preserved as history. The H2 corrective cycle was executed separately and
re-reviewed by a fresh-context reviewer. Path (a) was taken: the mechanism
was repaired — no claim-boundary downgrade.

```text
CORRECTIVE_ISSUE   = #79 (H2 fragment-reuse donor-fidelity corrective)
CORRECTIVE_PR      = PR #82 (branch fix/79-h2-donor-fidelity, head
                     367060aff624c35fbbad96bc5be72f35702ec487)
MERGE_COMMIT       = a3a551f (PR #82; LIVE_HEAD == REVIEWED_HEAD verified)
FRESH_REVIEW       = RH2 (fresh-context, not an author); verbatim record:
                     ../reviews/gate-a-rh2-h2-recheck-2026-09-28.md
NEW_VERDICT        = FAITHFUL_MECHANISM_MODEL
                     (P0=0, P1=0; P1_1_MID_CONTAINER_DISCOVERY = PASS,
                      P1_2_DISCOVERY_COST_SHAPE = PASS; D3/D5/D11 raised
                      from MATERIAL_DEVIATION; H2_GATE_A_READY = YES)
```

What was repaired (maps to R2's P1-1 / P1-2):

- **P1-1 (D3/D5 mid-container discovery/granularity)**: the shared-scanner
  consult point moved to the donor-faithful post-closure/pre-dispatch
  position (`run` phase 2 — after prefix-driven frame closure, before the
  new-frame push/dispatch; the `markdown.ts advance()` ordering of
  readLine/finishContext before reuseFragment), with a `starts_block` hook
  argument (false only where a live open paragraph would absorb the line —
  the sound, donor-faithful replacement of the R5 blank-line consult
  margin; interruptor-line takes restored). `fragment-reuse` gained a
  persistent forward-only monotone descent path (per-level monotone child
  indices; `seek` = the moveTo childAfter/parent analogue with line
  alignment). Frozen mid-container witness now shares 27/30 ListItem
  identities inside the damaged List with the List identity itself
  unshared (donor runtime re-reproduced at the pinned npm versions:
  25/30, byte-identical to the frozen record; the 2-item delta is exactly
  the declared absent NotLast trailing pop). The frozen §7 "taking runs
  of sibling blocks at nested levels … not a strict subset" claim is now
  delivered.
- **P1-2 (D11 discovery-cost shape)**: the per-consult entries rebuild and
  linear scan are gone (borrowed slices, zero per-consult allocation);
  discovery is amortized-forward with exact structural accounting
  (`metadata_records_touched = consultations + total_visits + slot_count`,
  `DiscoverySummary` exported; `table_rebuilds = 0` in every reviewer
  regime). Refusal-heavy regime: 202 consultations → 202 total visits on
  a 501-entry tree (pre-corrective shape ≈ 20,402 visits) — the O(B²)
  shape is excluded by an order of magnitude.
- Supporting repairs: take ends round to the next line start (donor steps
  past the last taken line's LF) while the Spliced placeholder keeps the
  raw span end for frame closure; lazy consult-lane line reads; new
  witness suite `mechanisms/fragment-reuse/tests/mid_container_reuse.rs`
  (4 witnesses, incl. the unspoofable identity witness and the
  forward-discovery-shape bound `total_visits ≤ 2·entries + 8·consults`).

Corrective evidence: all suites green (fragment-reuse 34, shared-grammar
45, neighbors 29+36+32+241; `matrix_r5` correctly left ignored), 7
independent reviewer probes == H0 clean parse; no timing collected.

Residual findings recorded by RH2 (non-blocking, declared):

- P2×4: left-edge continuation margin (against-H2); open-edge one-whole-
  block exclusion vs donor ~one line (against-H2); missing
  NotLast/trailing-partial-line/openEnd−1 guards → bounded over-reuse
  (27/30 vs 25/30, for-H2, declared); exact line-alignment + line-granular
  consult refuses gap-positioned candidates + the `fragment.from ≤ pos−1`
  step-back not modeled (conservative, ≤1 line).
- P3×5: symmetric minGap right-piece drop not yet noted as a donor
  asymmetry in R5 §7 DAMAGE text (completed in the R5 §7 addendum of this
  corrective cycle); single-edit contract (substrate); `max_consult_visits`
  omits failed-seek visits (informational only; total_visits attribution
  exact); h2_gate "nested quote edit" probe's comment claim now pinned by
  mid_container_reuse.rs instead; shared-scanner protocol change re-baselined
  H3/H4 attribution expectations (handled in #82/#83 with documented
  rationale).
