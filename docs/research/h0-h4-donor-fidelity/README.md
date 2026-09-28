# H0–H4 donor fidelity (Gate A of #76)

Status: **GATE A — DONOR FIDELITY = FAIL / BLOCKED (H2, H3 MATERIAL_DEVIATION)**

Frozen evidence for issue
[#76](https://github.com/jnhu76/markit/issues/76) **Gate A** — the
prior-art / donor-fidelity audit of the H0–H4 benchmark horses before any
Horse-A comparison may be interpreted at donor level.

```text
BASELINE
  MARKIT_MASTER = ad115bd12301a926aa1c3c19603f2a72fc1d183b
  TREE          = e0aeedef72858ea6b00d8324b6d85a2e93da7532

VERDICTS (independent donor-first reviewers R1–R4)
  H0 = FAITHFUL_MECHANISM_MODEL            (P0=0 P1=0)
  H1 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION (P0=0 P1=0)
  H2 = MATERIAL_DEVIATION                  (P1=2 — Gate-A blocker)
  H3 = MATERIAL_DEVIATION                  (P1=2 — Gate-A blocker)
  H4 = FAITHFUL_MECHANISM_MODEL            (P0=0 P1=0)

GATE_A_VERDICT = FAIL / BLOCKED
GATE_B_AUTHORIZED = NO
PERFORMANCE_COLLECTION_AUTHORIZED = NO
PERFORMANCE_COLLECTION_RUN = NO
```

Corrective issues (required before Gate A can be re-evaluated; mechanism
repair needs a separate PR + fresh fidelity re-review):

- H2: see the H2 row of FIDELITY-MATRIX.md — restore live-level
  (mid-container) candidate discovery, or re-freeze the claim boundary as
  a conservative strict subset and declare the per-consult discovery scan.
- H3: see the H3 row of FIDELITY-MATRIX.md — restore functional interior
  descent + incremental cursor (or counter attribution), or freeze
  top-level-entry granularity as a declared simplification with its
  under-reuse quantified.

## Contents

| File | Content |
|---|---|
| `DONOR-SOURCES.md` | immutable donor manifest (pins, SHAs, paths, roles, retrieval, provenance augmentation notes) |
| `H0-FIDELITY.md` … `H4-FIDELITY.md` | per-horse frozen fidelity contracts (schema of #76 §4; D1–D12 classifications, deviations, claim boundaries, verdicts) |
| `FIDELITY-MATRIX.md` | cross-horse matrix, Gate-A decision, corrective decisions, evidence inventory |
| `CHALLENGE-CASES.md` | per-horse F1–F5 challenge-case execution record (mechanism behavior only — no timing) |
| `challenge-evidence/` | frozen donor-side models, upstream probe scripts + observed outputs, local probe crates (as-run records) |

Reviewer records (verbatim): `../reviews/gate-a-r{1..4}-*-2026-09-28.md`.

## What this directory is NOT

- It is not a performance result. No timing, counters of economics, or
  comparative performance claims live here.
- It does not repair H2/H3. The deviations are recorded, not fixed; the
  corrective decisions belong to their own issues/PRs and a fresh
  fidelity re-review.
- It does not authorize Gate B, the six-horse performance comparison, or
  any Horse-A optimization.
