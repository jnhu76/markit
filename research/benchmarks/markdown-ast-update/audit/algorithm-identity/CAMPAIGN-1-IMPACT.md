# CAMPAIGN-1-IMPACT

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

Question: what does this audit do to the measurements and claims
produced by Campaign-1 (MARKIT-31 primary performance campaign, PR #44
merge)?

## Verdict

```text
CAMPAIGN_1_DATA_INVALIDATED = NO
CAMPAIGN_1_COUNTERS_TRUSTED = YES (with documented semantics)
FULL_EVIDENCE_CAMPAIGN_2_STARTED = NO
```

Campaign-1's attribution-lane data was produced by the SAME code paths
this audit verified at the SAME SHA. The audit found no mechanism
defect and no counter that measures something other than its (frozen,
documented) unit. Campaign-1 results therefore stand, interpreted
under the semantics pinned in `COUNTER-SEMANTICS-MATRIX.md`.

## Per-claim impact

| Campaign-1 dependency | Audit status | Impact |
|---|---|---|
| Correctness of H0–H4 (parity gates green) | Re-verified: R5 gate rerun at the audit SHA; plus 40 new adversarial tests | STRENGTHENED |
| `nodes_reused` as the reuse metric | Identity sharing proven structurally (`Arc::ptr_eq`) and via type invariants | STRENGTHENED |
| `blocks_reparsed` as a work proxy | F-1: unit is skeleton NODES (containers recurse; discarded fallback regions included; fences = 1) | INTERPRET with the pinned unit |
| `unique_*_source_bytes` as coverage | Derived by per-version union in one sink; the single-sink contract is what makes them meaningful | CONFIRMED |
| PA (R7 §10 formula) | Formula inputs confirmed; H3 numerator is margin-dominated (F-4) | INTERPRET per-horse |
| Cumulative-vs-unique divergence on fallback paths | Confirmed and pinned (H1 probe test) | CONFIRMED |
| H4 gauges in restart/convergence distances | Byte-coordinate semantics pinned (CJK test); EOF = no-take case (F-6) | CONFIRMED with reading rule |
| Any timing claim | Not re-measured here (forbidden); unaffected by audit findings (T-lane untouched, mechanisms clock-free) | UNCHANGED |

## What Campaign-1 does NOT yet contain (and this audit adds)

1. The counter SEMANTICS (this document + the matrix) — Campaign-1
   recorded values; the audit records what the values MEAN.
2. Adversarial identity evidence (wrong-algorithm discriminators) —
   the 40 tests.
3. The H3 margin-closure profile (F-4) — relevant when comparing H3's
   coverage-based metrics against H1/H2/H4.
4. The H1 full_parse slot gap (F-2) — a consumer reading Campaign-1
   `full_parse` rows must treat H1's fallback slot as N/A-by-phase.

## Consequences for FULL_EVIDENCE_CAMPAIGN-2

The audit's hard gate (both PASS verdicts required before Campaign-2)
is MET: no defect, no counter untrusted. Campaign-2 may proceed per
its own authority. Recommended (non-blocking) inputs from this audit:

- pin the counter-unit glossary (F-1) into the Campaign-2 protocol
  before the first run;
- treat H3 PA comparisons as vouch-cost-inclusive (F-4) or add the
  split slot first;
- mirror the gitignored workload corpus into fresh worktrees (F-7) or
  preflight will misreport.

Nothing in this audit requires re-running Campaign-1: no production
code changed, and the audit SHA is Campaign-1's own merge.
