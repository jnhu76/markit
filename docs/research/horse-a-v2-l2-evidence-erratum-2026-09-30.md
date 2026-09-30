# Horse-A v2 L2 — append-only evidence erratum (2026-09-30)

> Scope: issue [#100](https://github.com/jnhu76/markit/issues/100) (CLOSED) —
> the completed L2 record
> ([docs/research/horse-a-v2-l2-context-profile-100.md](horse-a-v2-l2-context-profile-100.md)
> and `research/benchmarks/markdown-ast-update/results/horse-a-v2-l2-100/`).
> Parent authorities: [#95](https://github.com/jnhu76/markit/issues/95) ·
> [#96](https://github.com/jnhu76/markit/issues/96) ·
> [#97](https://github.com/jnhu76/markit/issues/97);
> [#101](https://github.com/jnhu76/markit/issues/101) is conceptual Γ/Σ
> framing only.
> Status: **APPEND-ONLY ERRATUM — evidence boundary / interpretation
> correction. The historical L2 report and evidence are preserved
> byte-for-byte and must still be read first.**

```text
HORSE_A_V2_L2_HISTORICAL_REPORT = PRESERVED
HORSE_A_V2_L2_RAW_EVIDENCE      = PRESERVED
ERRATUM_TYPE                    = EVIDENCE_BOUNDARY / INTERPRETATION
L2_HOTSPOT_DIRECTION_REVERSED   = NO
L2_MECHANISM_DECISION_REVISED   = REPAIR_SUBSTRATE_THEN_REASSESS
```

## 0. How to consume the L2 record from now on

Read the original report and this erratum **together**:

```text
original L2 report (immutable historical record)
        +
this append-only erratum (interpretation boundary)
```

The original report's observations — what was measured, what the sampled
stacks contain, which contexts dominate the retained differential — remain
the record of what happened. What this erratum corrects is **what those
quantities mean** and **which inferences they license**. Nothing here
reopens #100, rewrites sealed evidence, re-collects profiles, implements
repair R, designs a semantic challenger, or executes L3/L4/L5.

## 1. Authority receipt at erratum time

```text
LIVE_MASTER        = fb359deb721c2020b0880739424ae4ea4dfc27c1
                     (= the PR #102 merge commit; no intervening commits,
                     no authority drift)
HISTORICAL_L2_MERGE= fb359deb721c2020b0880739424ae4ea4dfc27c1 (PR #102)
PR_102_HEAD        = 5722388504e9331795e7992c079c9e2321a0b9fd
PR_99_MERGE        = 9030b6137f77fc0ff0ed4d00a7285a8f10669f23
ISSUE_98           = CLOSED (L0/L1)
ISSUE_100          = CLOSED (L2) — not reopened, no comment mutated
ISSUE_95/96/97/101 = OPEN (current research authorities)
ERRATUM_BRANCH     = research/100-l2-evidence-erratum (documentation /
                     derived-evidence correction only; no Horse-A
                     mechanism code, no benchmark treatment collection)
```

## 2. Historical immutability (mechanical proof)

Read-only for this task, with identity recorded before any edit:

```text
L2 evidence tree   = 0d6f0d28400d3a801a2c132fc372d717b9afbf9e
                     (results/horse-a-v2-l2-100 @ fb359de, 80 tracked
                     files, 80/80 SHA-256 verified unchanged)
L1 evidence tree   = 51a142687a5e28139b08bda9c488d5e192b6ef13
                     (results/horse-a-v2-diag-98 @ fb359de, 8 tracked
                     files, 8/8 SHA-256 verified unchanged)
L2 report blob     = decda302cb074f97f9e56c1f0628d5dd08ce27f9
                     sha256 cec491ec4f371ccf1891d5d1a99f0ce7d2a91bcbad00f1e98d9c522d1455e437
L1 report blob     = bd56bce9b2aa6ef8d2082532475103347ce04023
                     sha256 8220316f03a86ce520908710834b511a26eeed62db6bbf3e02f22f8df46e6ced
```

After the erratum work the same tree/blob identities are re-verified
(§10); the erratum lives entirely in new files / new history.

---

## E1. Warmup inclusion / denominator defect (primary correction)

### ORIGINAL OBSERVATION

The L2 report published per-context `samples/op` values (e.g. certificate
103.79 / owner_materialize 28.49 on E6-1/P1/rep1) and differential excess
shares derived from them (§E, §F), with the repetition model disclosed as
"E6-1/E6-5 = 100 ops (warmup 20), E6-6 = 1500 ops (warmup 30)".

### POST-HOC DEFECT

The published quantity described as `samples/op` was **not** a strict
"formal warmed-only samples ÷ formal operations" quantity. Verified from
source and retained evidence:

1. **The profiling harness runs warmup through the same marker functions
   the formal region uses** (`horse-a-v2-l2/src/bin/
   mdbench-horse-a-v2-l2.rs:101-104`): warmup calls
   `l2_p1_build` / `l2_p0_parse` directly from `main`; the formal region
   calls them inside the `#[inline(never)] profile_region` wrapper.
2. **`perf record` covers the whole process**
   (`collect_profile.sh:26` launches the binary under `perf record`), so
   warmup executes inside the sampling window; the retained `mid/` dumps
   begin with `ld.so` startup samples.
3. **`perf_fold.py` retains samples by call-path containment of the
   marker functions only** (`REGION_MARKERS = ["l2_p1_build",
   "l2_p0_parse"]`, retention predicate at line 121) — there is no
   formal-phase time marker and no `profile_region` requirement. The
   harness's design-intent comment ("warmup … excluded from attribution
   by the marker-frame filter in the analysis stage") is therefore **not
   what the implemented filter does**.
4. **The `profile_region` frame is absent from every retained stack** —
   0 occurrences in all 12 `mid/` files and all 12 `folded/` files — so
   warmup and formal executions are **not separable by call path** in the
   retained evidence.
5. **`classify.py` divides retained samples by `region_ops` (formal
   only)** (`samples_per_op = samples / args.reps`, line 131). Operation
   counts confirmed from the run receipts: E6-1 = 20 warmup + 100 formal,
   E6-5 = 20 + 100, E6-6 = 30 + 1500 (identical within each P1/P0 pair).

Independent confirmation that warmup samples are *in fact* inside the
retained sets: the timestamped in-region window per profile exceeds
`region_wall_ns` (the run receipt's formal-region duration) by almost
exactly the warmup phase (e.g. E6-1/P1/rep1: 0.6111 s window − 0.4995 s
formal ≈ 0.1116 s ≈ 20 ops at the observed warmup rate). This is not a
hypothetical risk; the mixed-phase numerator is present in the retained
data. It must be stated plainly: **the published `samples/op` numerator
may include both warmup and formal executions while the denominator
counts only formal operations.** This is a normalization defect, not
"extra noise".

For contrast, the #98 T-LANE decompose probe excluded warmup correctly
(`decompose.rs:91` gates each measurement on `round >= WARMUP_ROUNDS`);
the L2 P-LANE replaced that per-operation gating with a marker-region
concept the analysis stage never enforced.

### CORRECTED INTERPRETATION

Three quantities must be distinguished (computed in
`results/horse-a-v2-l2-100-erratum/phase_window.json`, sensitivity-only):

```text
A. HISTORICAL_REPORTED     retained samples / region_ops — mixed-phase
                           numerator, formal-only denominator (as published)
B. POOLED_NORMALIZED       retained samples / (region_ops + warmup_ops)
                           — a valid pooled average per profiled op
C. TRUE_WARMED_ONLY        formal-phase samples only / region_ops
```

- **A is preserved as published** but must be read as a pooled/mixed
  attribution over all profiled operations normalized to the formal
  count. On E6-1/E6-5 that inflates per-op magnitudes by ≈ ×1.20
  (warmup ops are 20/120 of profiled ops; measured warmup vs formal
  per-op rates agree within ±2.4% on rep1, max deviation E6-5/P0
  −2.3%); on E6-6 by ≈ ×1.02.
- **B is valid when explicitly labeled** as a pooled per-profiled-op
  average (every retained sample *is* one of those operations).
- **C is NOT RECOVERED strictly.** No phase boundary was recorded at
  collection time; the retained evidence cannot deterministically
  isolate formal from warmup execution. A constrained *estimate* of C
  exists (timestamp window anchored by the run receipts'
  `region_wall_ns`; boundary shifts of ±2 ms move the split ≤ ±0.4%),
  stored under the erratum namespace as **sensitivity-only** evidence.
  C must not be derived from A by a simple arithmetic factor — warmup
  and formal distributions need not be identical (and rep2 of E6-1/P1
  runs its warmup ≈ 7.9% hotter per op; E6-6/P0 rep2 ≈ 49% hotter on
  small counts), even though rep1 warmup/formal rates agree within
  ±2.4%).

No profile was re-collected for this erratum.

### WHAT REMAINS VALID

- All *relative* structure of the retained attribution: bucket ordering,
  the P1/P0 ratio per cell (3.93/3.78/1.32 → 3.91/3.76/1.32 under the
  warmed-only window estimate), the cancellation rows, the below-floor
  rows.
- The differential **shares** are mathematically invariant under the
  pooled normalization (both arms rescale by the same factor), and move
  by at most 0.41 percentage points in absolute value under the
  warmed-only window estimate on E6-1/E6-5 (both directions observed;
  §E2).

### WHAT IS NO LONGER CLAIMED

- That the published `samples/op` values are warmed-only per-operation
  quantities.
- That the absolute magnitudes (e.g. "certificate ≈ 104 samples/op")
  describe formal-operation cost without a ≈ ×1.2 (E6-1/E6-5) mixed-phase
  inflation. Warmed-only window estimates: certificate ≈ 86.2–87.3,
  totals ≈ 130.6–132.6 (P1) / 33.1–34.8 (P0) on E6-1/E6-5.

---

## E2. The "≈ 88–90%" certificate share is bounded, not absolute

### ORIGINAL OBSERVATION

Historical L2 (rep1): E6-1 certificate excess 103.79 of total excess
118.52 = **87.6%**; E6-5 103.72 of 115.58 = **89.7%** (arithmetic
verified against `differential_rep1.json`; rep2 87.7% / 87.9%).

### POST-HOC DEFECT

Nothing in the arithmetic is wrong; the defect is the reading. These are
**historical differential shares under the original mixed-phase
normalization**. They are NOT:

```text
88–90% of P1 absolute execution        (certificate is 65–66% of P1)
88–90% of normal-update wall time
88–90% guaranteed recoverable latency
a universal Horse-A constant
a warmed-only estimate proven by the original L2 pipeline
```

### CORRECTED INTERPRETATION

```text
CERTIFICATE_LOOKUP_DOMINANCE  = ROBUST QUALITATIVE LOCALIZATION
EXACT_WARMED_ONLY_SHARE       = NOT ESTABLISHED BY THE ORIGINAL L2 PIPELINE
```

Supporting sensitivity (window warmed-only estimate, both reps):
E6-1 87.9% / 88.1%, E6-5 90.0% / 87.8% — the qualitative dominance
survives; the exact warmed-only share was never established by the
original pipeline and remains an estimate range. This is one
sensitivity check under the erratum namespace; it is not a replacement
official campaign.

### WHAT REMAINS VALID

In the retained L2 evidence, the E6-1/E6-5 P1−P0 differential remains
strongly concentrated in the certificate-barrier lookup context — under
every normalization computable from retained evidence (A, B, C-window).

### WHAT IS NO LONGER CLAIMED

Any precise warmed-only percentage, and any conversion of the share into
recoverable wall-time or speedup of a replacement mechanism.

---

## E3. E6-4 was misclassified inside the semantic-opportunity story

### ORIGINAL OBSERVATION

L1 H-D wording: "the minimum repair candidate (#95 Candidate B —
effective binding/value certification) … would convert E6-1/E6-4-class
updates into local splices" — grouping E6-4 with E6-1 under one broad
effective-binding/value certification opportunity.

### POST-HOC DEFECT

The retained effect ledger contradicts the grouping. From
`results/horse-a-v2-diag-98/effects.json` (and the frozen cell role
text): E6-4's **effective winner destination actually changes**
(winner label `rd`: `/efghijklmnopqrstuvwxyzab` →
`/stuvwxyzabcdefghijklmnop`), **10 consumer ReferenceLinks are
re-pointed**, 11 outputs must change. E6-4 is therefore NOT an
equality-only case, and the historical broad H-D wording overreached.

### CORRECTED INTERPRETATION

```text
E6_1_CLASS != E6_4_CLASS
```

- **E6-1 (losing duplicate changes):** complete ordered facts change
  while the consumer-visible effective environment is unchanged
  (`F_old != F_new` but `π(F_old) == π(F_new)`; effects.json:
  `effective_winner_changed = false`, 1 must-change output). This is a
  candidate **equality / early-cutoff** opportunity — while complete
  producer state must still be updated correctly (E4).
- **E6-4 (low-fanout value change):** the effective semantic destination
  changes and ~10 existing consumers change with it. Its opportunity —
  if still worth pursuing after repair R — is **dependency discovery +
  selective (sparse) propagation**, avoiding a whole-document rebuild
  (2049 owners / 4125 nodes were rebuilt for 11 must-change outputs),
  not early cutoff of unchanged consumers.

The sealed L1 report is not rewritten; this section is the correction of
record.

### WHAT REMAINS VALID

- L1's H-D verdict itself (the frozen ordered-facts certification fires
  `FactsDiffer` on E6-1's semantically irrelevant fact change) — the
  refusal-reason finding stands.
- That both classes are evidence of the certification/rebuild boundary
  being too coarse; they simply need different mechanisms.

### WHAT IS NO LONGER CLAIMED

That a single equality-style certification converts *both* E6-1-class
and E6-4-class updates into local splices.

---

## E4. Query equality does not permit stale producer state

### ORIGINAL OBSERVATION (gap in the record)

The L1/L2 opportunity discussion treated "effective environment
unchanged" as the certification target without stating what a future
equality/early-cutoff mechanism must still maintain.

### CORRECTED INTERPRETATION (semantic-state boundary)

The retained semantic state is **more than the effective winner map**:
`ReadyDocument.refs: RefTable` holds the **complete ordered definition
fact sequence** ("source order preserved, duplicates included" —
`shared-grammar/src/inline.rs`, `mechanisms/horse-a/src/facts.rs`), and
the frozen certification compares complete ordered sequences. Example:

```text
winner = /a, loser = /b;  edit loser: /b -> /c
effective lookup before = /a ; after = /a   (unchanged)
complete retained facts: [/a, /b] -> [/a, /c]  (must change)
```

Therefore a future equality/early-cutoff mechanism may potentially avoid
**propagating change to unchanged consumers**, but it may NOT leave
complete producer state stale. The required future invariant is recorded
as a **research obligation, not a proved theorem**:

```text
Inv(S, source)  and  update(S, edit) = S'
    must imply:
observable(S') = from_scratch(edited_source)
    and
Inv(S', edited_source)
```

This is an interpretation correction only. It authorizes no semantic
mechanism implementation and does not decide `facts_equal` vs
`effective_equal` (future research).

### WHAT REMAINS VALID / NOT CLAIMED

Valid: the L1 finding that E6-1's differing fact is semantically
irrelevant to every consumer's effective binding. Not claimed: that
skipping consumer propagation may also skip updating retained facts,
owners, or any state a later edit or export observes.

---

## E5. Effect-ledger scope: diagnostic classification, not a minimum-work oracle

### ORIGINAL OBSERVATION

L1 §G published required-effect counts (1 / 11 / 321 / 4482 …) as the
"eager obligation floor".

### CORRECTED INTERPRETATION

Audited against `horse-a-v2-diag/src/effects.rs`: the ledger is an
**offline diagnostic classification** under the implemented comparisons,
not a universal minimum-work oracle. In particular:

- The node diff is a **(kind + value) signature multiset difference** —
  structural parent/child correspondence is ignored; a node moved
  between parents with an unchanged signature counts as no change.
- `value_changed_nodes` is inferred from **balanced per-kind deltas**
  (`add == rem` heuristic), not from a node-matching proof; an unbalanced
  mixture of unrelated adds/removes would be classified differently.
- Span-only accounting is the implemented **class-level** signature/span
  comparison (signature classes with equal multiplicity but differing
  sorted span sets), not a complete optimal node matching.
- `effective_winner_changed` does **not** encode None↔Some existence
  transitions — those live in the separate
  `reference_existence_changed` field (E6-6 demonstrates the split:
  `effective_winner_changed = false`, `reference_existence_changed =
  true`).
- Consequently the counts are **not automatically**: the minimum number
  of writes, a minimum-computation lower bound, the exact dependency
  locations, or the exact reusable-node cardinality.

Historical counts (1, 11, 321, 4482, …) must not be described as a
universal minimum amount of runtime work. Where a case has a clear
direct interpretation it may remain, case-specifically — e.g. E6-1's
"1 must-change output" (the loser definition node's destination field)
is a direct reading on that cell. E6-6 clarification: its large count
(4482 = 4481 removed + 1 added) is **dominated by removals** — it does
not imply the new state constructs that many new outputs.

### WHAT REMAINS VALID

The bimodality finding (effective-change cells need 321–4482 output
changes; ineffective/low-fanout cells need 1–11) as a *classification*
of the frozen cells.

---

## E6. Comparator interpretation boundary: P1−P0 is not removable work

### ORIGINAL OBSERVATION

L2 contrasts `P1_FULL_BUILD_ONLY` (`horse_a::full_build(post)` alone)
against `P0_H0_FULL_PARSE_ONLY` (H0 `full_parse` + `complete` alone) and
attributes the P1−P0 differential across calling contexts.

### CORRECTED INTERPRETATION

The contrast is **legal as an L2 construction-path comparison** (same
source, same host, same sampling contract, one binary family) and it
validly answers *where the construction-path differential executes*. But:

```text
P1 - P0  !=  pure removable Horse-A work
```

because P0 does not carry Horse-A's retained-state / next-edit
obligations. `full_build` produces a complete READY state — complete
ordered facts (RefTable), eagerly materialized payloads under the final
environment, owner-relative coordinates, coverage, restart certificates,
and the retained OwnerSeq — i.e. work a *next Horse-A edit* needs. P0
produces a NormalizedDocument with no such obligations (H0's next edit
re-parses anyway). The differential therefore mixes (a) legal
READY-obligation construction with (b) implementation inefficiency (the
repeated certificate-barrier lookup).

L2 accordingly supports: "the differential executes mostly in the
certificate-barrier lookup context". L2 does not support: "every
P1−P0 cycle should disappear in Horse-A". This clarification does **not**
invalidate the reason to test a same-authority repair R (#95 Step 2R:
same authority, same certificate/support semantics, zero new retained
state budget) — it sharpens what R must be compared against.

---

## 3. What this erratum does NOT retract

Unless contradicted by retained evidence (nothing found), these stand:

```text
L0 same-READY comparator legality                    = intact
L1 conclusion: E6-1..5 are not primarily dominated by
  large discarded incremental work before full build   = stands
L2 qualitative localization: certificate-barrier lookup
  is the dominant dense-boundary P1−P0 differential
  context                                              = stands (E2 robustness)
certificate authority itself                          = not shown unnecessary
Owner / AVL / coverage                                = not promoted to primary targets
L3 / L4 / L5                                          = not required now
NEXT                                                  = SUBSTRATE_REPAIR R
```

The erratum changes the **precision and interpretation boundary** of the
L2 evidence, not the existence of the discovered repeated-matching
problem or the direction of the hotspot.

## 4. No new mechanism conclusions

This erratum asserts none of the following (all remain future research
questions under #95):

```text
linear merge is the correct repair     O(B+R) is proven
HashMap is required                    certificate density should decrease
reverse dependency index is justified  facts_equal should become effective_equal
E6-4 is solved by early cutoff
```

## 5. V / R governance alignment (#95)

```text
V = frozen historical Horse-A v1
R = future independent semantics-preserving minimum-repair identity
```

The historical L1/L2 evidence is evidence about **V**. This erratum does
NOT "repair the V1 baseline" — historical V1 remains exactly as measured;
the erratum corrects what may be inferred from that evidence. The
escalation consequence is unchanged in direction, refined in reading:
proceed to #95 **Step 2R** (substrate repair R under its own identity)
and only then reassess whether a semantic challenger remains worth
opening.

## 6. Correction matrix

| # | Historical statement / implication | Correction | Status after correction |
|---|---|---|---|
| E1 | L2 `samples/op` represent formal warmed-only operations | mixed-phase numerator (warmup provably inside the retained sets) ÷ formal-only denominator; per-op magnitudes inflated ≈ ×1.20 on E6-1/E6-5 (≈ ×1.02 on E6-6) | CORRECTED |
| E2 | certificate share ≈ 88–90% (of the excess) | robust qualitative dominance; shares invariant under pooled normalization and stable (max \|Δ\| = 0.41 pp on E6-1/E6-5, both directions) under the warmed-only window estimate; exact warmed-only share not established by the original pipeline | BOUNDED |
| E3 | E6-4 belongs to the equality-only certification opportunity | effective answer changes (winner re-points, 10 consumers, 11 outputs); it is sparse propagation, not early cutoff — `E6_1_CLASS != E6_4_CLASS` | CORRECTED |
| E4 | (implicit) equality on effective queries suffices | complete retained facts (ordered, duplicates) must still be updated; producer state may not remain stale — stated as a research obligation | ADDED-BOUNDARY |
| E5 | effect counts represent minimum required runtime work | they are diagnostic differences under the implemented multiset/signature/winner-map ledger | BOUNDED |
| E6 | P1−P0 equals removable Horse-A work | comparator differential mixes legal READY obligations with inefficiency; valid for path localization only | BOUNDED |
| — | certificate lookup hotspot means certificate authority should change | only the current matching implementation is implicated (#95: REQUIRED_RESPONSIBILITY_WITH_BAD_IMPLEMENTATION) | REJECTED (already bounded in the original §I; retained here) |

## 7. AUTHORITATIVE AFTER ERRATUM

```text
UNCHANGED:
- direct READY comparator legality (L0)
- L1: E6-1..5 not dominated by pre-fallback path-selection work
- certificate-barrier-lookup qualitative dominance of the E6-1/E6-5
  P1−P0 differential (robust under every retained-evidence normalization)
- relative structure of the L2 attribution (bucket order, cancellations,
  below-floor rows, P1/P0 ratios)
- substrate repair R is justified as the next step (#95 Step 2R)

DOWNGRADED:
- exact L2 samples/op interpretation (mixed-phase pooled normalization;
  absolute per-op magnitudes ≈ ×1.2 on E6-1/E6-5)
- exact 88–90% warmed-only share (estimate range only; never established
  by the original pipeline)

CORRECTED:
- E6-4 semantic classification (equality-only → sparse propagation)
- the harness/filter design-intent claim that warmup was excluded from
  attribution

NOT ESTABLISHED:
- exact recoverable wall-time percentage of any repair
- TRUE_WARMED_ONLY quantities (strictly; constrained estimates only)
- the final repair algorithm (incl. any merge/index structure)
- semantic challenger value after R
```

## 8. Derived-evidence namespace

New, append-only:
`research/benchmarks/markdown-ast-update/results/horse-a-v2-l2-100-erratum/`
(`erratum.json`, `warmup_audit.json`, `phase_window.json`,
`sensitivity.md`, `SHA256SUMS`, `README.md`, plus the two replay scripts).
Every derived artifact records its source historical artifacts, source
SHA-256s, calculation, denominator, assumption, and sensitivity-only
status; the derivation reproduces all 12 committed attribution buckets
integer-exact before any phase claim (replay gate 12/12). No historical
attribution JSON was copied or modified.

## 9. GitHub history

Issue #100 remains CLOSED; no existing comment on #100, #98, or the PRs
was edited. The erratum may later be posted to #100 as a **new
append-only comment** after this PR is reviewed — not before.

## 10. Independent audit record (performed before opening the PR)

Read-only audit against repository source and retained evidence —
findings and classifications:

```text
warmup path claim vs harness source            VERIFIED (lines 101-104 vs 123-129)
fold/filter behavior vs perf_fold.py           VERIFIED (REGION_MARKERS predicate;
                                                profile_region 0/24 retained files)
denominator behavior vs classify.py            VERIFIED (line 131, /region_ops)
operation counts vs run receipts               VERIFIED (20/100, 20/100, 30/1500 ×2 reps)
historical percentages vs differential_rep1.json VERIFIED (87.57%, 89.74%)
E6-4 effect semantics vs effects.json + cells.rs VERIFIED (winner change, 10
                                                consumers, 11 outputs)
RefTable/complete-state claim vs inline.rs/facts.rs VERIFIED (ordered, duplicates)
effect-ledger semantics vs effects.rs          VERIFIED (multiset/class-level
                                                heuristics; separate existence field)
P1/P0 obligation difference vs full_build.rs + decompose.rs VERIFIED
phase-window reconstruction                     replay gate 12/12; boundary
                                                sensitivity quantified (±2 ms → ≤ ±0.4%)
P0 = 0   P1 = 0   P2 = 3 (draft precision-wording nits — ±2% vs ±2.4%
                warmup/formal rate bound, ≤0.4 pp vs +0.41 pp share
                shift, P1 totals range 130.6–132.6 — found by this
                audit and corrected in place before the PR)   P3 = 0
ERRATUM_AUTHORITY = PASS
```

Historical immutability re-verified after all erratum work:
`ORIGINAL_L2_REPORT_CHANGED = NO`, `ORIGINAL_L2_RESULTS_CHANGED = NO`,
`SEALED_L1_RESULTS_CHANGED = NO` (same git tree/blob identities as §2;
see the PR's proof block).

## 11. Expected repository changes

```text
NEW:      docs/research/horse-a-v2-l2-evidence-erratum-2026-09-30.md
NEW:      research/benchmarks/markdown-ast-update/results/
          horse-a-v2-l2-100-erratum/**
MODIFIED: docs/research/README.md   (consume original + erratum together)
```

No other file is touched; the historical L2 report and both sealed
evidence directories are byte-identical to `fb359de`.
