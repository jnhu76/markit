# MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — Evidence Pack

Audit authority: PR #44 merge `0bf678cc1505098e6afe26cb8ec54cda24115831`
(branch `research/31-algorithm-identity-audit-1`, worktree
`~/Source/markit-algorithm-audit`). Every code statement in this pack is
anchored to that SHA by `path` + function/type name + line range.

## What this audit asked

1. Are H0–H4 actually implementing the mechanism **models** they claim
   (full rebuild / block-local reparse / fragment reuse / old-tree
   subtree reuse / restart convergence)?
2. Do the **work counters** actually measure the events they name?

Correctness evidence alone is NOT sufficient: a mechanism can deliver
the correct tree while executing the wrong algorithm. This audit
therefore inspects code paths, mechanically pins counter values with
hand-derived exact assertions, and runs adversarial cases designed to
make a wrong algorithm produce a wrong observable.

## Evidence classes

Every load-bearing statement carries one of:

- `CODE_INSPECTION_SUPPORT` — read directly from the source at the
  audit SHA (path + function + lines given).
- `TEST_SUPPORT` — executable assertion in the audit test suites
  (`mechanisms/*/tests/audit_identity.rs`, all added by this audit).
- `MECHANICALLY_PROVEN_BY_INVARIANT` — derived from a structural
  property (type system, trait boundary, dependency graph, or an
  exhaustive construction), not merely observed.

## Documents

| File | Question it answers |
|---|---|
| `ALGORITHM-AUTHORITY-MAP.md` | What frozen document governs what code |
| `IMPLEMENTATION-MAP.md` | Where each mechanism actually lives, function by function |
| `HORSE-IDENTITY-MATRIX.md` | Per-horse verdict: does the code implement the claimed model |
| `CALL-PATH-MAP.md` | Who may call the parser/inline scanner, and through which seam |
| `H4-CONVERGENCE-PREDICATE.md` | Clause-by-clause audit of the convergence predicate |
| `COUNTER-SEMANTICS-MATRIX.md` | Counter-by-counter: what is actually measured per horse |
| `ADVERSARIAL-TEST-MATRIX.md` | The adversarial cases and what each would catch |
| `PARSE-RANGE-TRACE.md` | Hidden-full-rebuild check: how scan ranges were traced |
| `FINDINGS.md` | Findings, taxonomy, severity |
| `CAMPAIGN-1-IMPACT.md` | What Campaign-1 claims survive this audit |
| `GPT6-ALGORITHM-AUDIT-HANDOFF.md` | Evidence-first handoff for the independent reviewer |

## Audit result (headline)

- `H0..H4_ALGORITHM_IDENTITY = PASS` for all five horses — each
  implementation realizes its frozen mechanism model, with the audit
  observations recorded in `FINDINGS.md` (all observations, none rises
  to a mechanism defect).
- `ORACLE_CONTAMINATION = NONE_FOUND`
- `HIDDEN_FULL_REBUILD = NONE_FOUND`
- `FRESH_STATE_ISOLATION = PASS`
- `COMPLETE_BOUNDARY = PASS` (complete() is native sealing only)
- Counter verdicts: see `COUNTER-SEMANTICS-MATRIX.md` (one P3 naming
  observation: `blocks_reparsed` counts block STRUCTURE units, not
  byte-rescan work; one P3 applicability observation: H1 leaves
  `fallback_to_full_count` Unknown on `full_parse` where H0/H2/H3/H4
  declare NotApplicable).

## Method

- Static: full read of every mechanism crate, the shared grammar seam,
  the counter substrate, and the runner lane wiring at the audit SHA.
- Dynamic: five new test-only suites (40 tests) asserting HAND-DERIVED
  exact counter values, structural identity (`Arc::ptr_eq`), damage-map
  observability, adversarial refusal paths, and H0-oracle equality.
- The audit modified NO production code. Allowed artifacts only: audit
  documentation (this pack), regression tests, test-only
  instrumentation. `git diff 0bf678cc --stat` shows tests + docs.

## Reproduction

```sh
cd ~/Source/markit-algorithm-audit/research/benchmarks/markdown-ast-update
cargo test -p markit-mdbench-full-rebuild           --test audit_identity
cargo test -p markit-mdbench-block-local            --test audit_identity
cargo test -p markit-mdbench-fragment-reuse         --test audit_identity
cargo test -p markit-mdbench-old-tree-subtree-reuse --test audit_identity
cargo test -p markit-mdbench-restart-convergence    --test audit_identity
./scripts/verify-r5.sh        # frozen R5 correctness gate (release)
```

Note: the frozen R5 matrix cases need the gitignored materialized
workload sources under `workloads/sources/*/files/` (see
`workloads/.gitignore` line 10); a fresh worktree must mirror them from
a materialized checkout or the campaign preflight tests fail on missing
files. This is environmental, not a code property.

## Hard rule honored

`FULL_EVIDENCE_CAMPAIGN_2_STARTED = NO`. No performance measurement of
any kind was performed by this audit (no timings, no horse ranking, no
profiling; `scripts/verify-r5.sh` is explicitly correctness-only).
