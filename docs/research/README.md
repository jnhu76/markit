# Markit research status

This page is the campaign status map for Markit parser research.

## Current study status — 2026-09-29

The R0–RQ8 paper-style mechanism study is complete. The current human-readable
entry point is:

- **[Horse-A v1 baseline experimental record](horse-a-v1-baseline-experimental-record.md)** —
  final H0–H4 + Horse-A v1 baseline, RQ8 optimization-sensitivity qualification,
  pre-registered W-A1/W-A2/W-A3 predictions vs observed evidence, measured
  Weakness Map, next-mechanism design inputs, and reproducibility/capsule
  authority. **Start here for the completed study.**
- **[Horse-A v2 L0+L1 diagnosis](horse-a-v2-l0l1-e6-tiny-diagnosis.md)** —
  first Horse-A v2 forensic pass (#98): E6 + tiny same-READY work/effect
  decomposition with the validated `HORSE_A_V1_DIRECT_READY_REBUILD` control.
  L0 PASS / L1 COMPLETE; E6 cost is construction, not path selection;
  escalation decision L2 (one paired-profile question, not yet executed).
- **[Horse-A v2 L2 context-profile localization](horse-a-v2-l2-context-profile-100.md)** —
  paired context-profile pass (#100): the P1−P0 full-build construction
  residual localizes ≈ 88–90% to the repeated certificate-barrier lookup
  inside `persist_interior_certificates` on E6-1/E6-5 (coverage build
  itself below floor); Owner materialization shows no material positive
  P1−P0 residual (≈ cancels vs H0 document materialization on the frozen
  cells); AVL is secondary; escalation decision
  L2_SUFFICIENT_FOR_MECHANISM_DECISION → #95 Step 2.
  **Start here for active Horse-A v2 research.**
- **[Baseline figures](figures/horse-a-v1-baseline/README.md)** — the eight
  ECharts-generated baseline figures (deterministic pipeline over sealed
  derived artifacts; per-figure provenance in the generated `manifest.json`).

Current final state:

```text
#33 paper-style study             = CLOSED / COMPLETE
#76 fidelity/performance gate     = CLOSED / COMPLETE
primary T/A/M                     = COMPLETE / SEALED
PMU explanation                   = COMPLETE / SEALED
RQ8 optimization sensitivity      = COMPLETE
Horse-A v1                        = IMMUTABLE EXPERIMENTAL BASELINE
production Markdown design        = NOT STARTED
```

Durable research capsule identity:

```text
markit-r0-rq8-research-record-v1.tar.gz
SHA256 = 156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849
```

The capsule contains the sealed raw evidence, R7 analysis, PMU campaign, R8
synthesis, RQ8 sensitivity evidence, scripts/provenance, issue/PR authority,
and a complete Git bundle. The host path is operational metadata; future work
should cite the capsule filename + SHA256 as the immutable v1 evidence identity.

**Historical-status warning.** The sections below preserve the repository's
research chronology and therefore contain statements that were true at earlier
campaign stages (for example `PRIMARY_TIMING = NOT_STARTED`, RQ8 not yet run,
or issues still open). Those statements are historical context, not current
status. The baseline record above is the current study authority.

## Research synthesis and algorithm-design handoff

For the pre-Horse-A synthesis chronology, read:

- **[Markit-31 research synthesis](markit-31-research-synthesis.md)** — canonical
  pre-Horse-A handoff: what was proven / not proven at that stage, how the
  evidence changed the understanding (Stage 0→8), the H0–H4 Weakness Map, the
  evidence-to-requirement traceability matrix (R1–R6), methodological lessons,
  and the then-next research question.

Supporting documents (issue [#48](https://github.com/jnhu76/markit/issues/48)):

- [Campaign-2 mechanism synthesis](campaign-2-mechanism-synthesis.md) —
  **historical**: the A–J evidence verdict, regime/causal maps, lifecycle,
  weaknesses and limitations *as interpreted at the time of Campaign-2*
  (reviewed head `819928966e05ca00bb82c58b887014e54f3d0cd5`). Its later-evidence
  section records how #50/PR #51 confirmed, refined or superseded individual
  statements; the body is deliberately not rewritten.
- [From experiment to AST implementation](campaign-2-to-ast-guidance.md):
  the engineering direction and staged rationale.
- [Markit AST/update algorithm design v0](markit-ast-update-design-v0.md):
  **research design candidate**. Every major statement is classified as
  A. REQUIRED_BY_CORRECTNESS / B. SUPPORTED_BY_EVIDENCE /
  C. CANDIDATE_DESIGN_CHOICE / D. OPEN_QUESTION (§0.4).

Evidence authorities — merged master commits, not transient PR heads:

```text
frozen workload / protocol      research/benchmarks/markdown-ast-update/protocol/ …
mechanism + counter semantics   PR #46 merge  3762b7a42e1c284a4c2c2e0ebac8496e70c63431
H0-H4 correct/parity + #40 R5   PR #30 merge  12952a561c79fa051bca6bae7409ef536cd43011
    real-workload correctness   protocol/R5-REAL-WORKLOAD-CORRECTNESS-CLOSURE-v1.md
                                (record merged by PR #40, 1af66c09)
primary campaign collection     results/primary-run/905427da/PRIMARY-RUN-CLOSURE.md
                                (PRIMARY_TIMING = NOT_STARTED; historical stage)
Campaign-2 evidence             PR #47 merge  334eea6201fc0258e35a7c5b21feb722641ddcbd
H4 large-N root cause (#50)     PR #51 merge  6cec47e9bb756affb0a4477bcb4962c3c78d8901
```

The documents in this historical section did not themselves close #22/#31/#33/#48,
authorize production implementation, or lift the then-current product architecture
HOLD. Later Horse-A/R0–RQ8 authority is summarized in the baseline record above.

## Campaign authority and substrate — historical chronology

```text
ACTIVE UMBRELLA AT THIS STAGE:
    #22 MARKIT-MARKDOWN-BENCHMARK-1
    Controlled Rust comparison of H0-H4 Markdown update mechanisms.

ACTIVE EXECUTION ISSUE AT THIS STAGE:
    #31 project-driven performance evaluation for H0-H4
    Real-project corpus -> deterministic edit traces -> timing/work lanes
    -> attribution -> Weakness Map.

ACTIVE INTERPRETATION ISSUE AT THIS STAGE:
    #33 paper-style regime map study
    #48 Campaign-2 mechanism synthesis / A-J interpretation

COMPLETED SUBSTRATE:
    R0 methodology                         PASS
    R1 controlled harness                  PASS
    R2 prior-art extraction                PASS
    R3 grammar/corpus/mutation freeze      PASS
    R4 H0 reference                        PASS
    R5 H1-H4 correctness/parity            PASS (PR #30 merged)
    R5 real-workload correctness closure   PASS (#40, PR #40 merged)
    R6 project-performance execution       FROZEN FOR #31 EXECUTION
       protocol/R6-PROJECT-PERFORMANCE-EXECUTION-AMENDMENT.md
       protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md
    R7 primary-performance campaign freeze FROZEN CANDIDATE
       protocol/R7-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-v1.md
       protocol/R7-MACHINE-BINDING-CORRECTIVE-1.md
       collection closure merged (results/primary-run/) — PRIMARY_TIMING = NOT_STARTED

ARCHIVED:
    Experiment 0 — #19 / PR #20
    research/experiments/experiment-0-parser-survey/

SUPERSEDED:
    #21 MARKIT-MARKDOWN-ARCHITECTURE-1
```

**ID collision warning.** The `R0`–`R7` identifiers above are *protocol stage
records* under `research/benchmarks/markdown-ast-update/protocol/`. The `R1`–`R6`
used in the synthesis and in the design candidate are *V1 behavioural
requirements* — a different namespace with unrelated numbering. Cite the
protocol files by path and the requirements as "V1 requirement Rn".

## Historical authorized research path

The following records the path as it stood before the completed Horse-A/RQ8
study. It is retained to preserve the research chronology; current status is at
the top of this file.

```text
prior art / mechanism extraction
        ↓
controlled Rust substrate
        ↓
correctness-complete H0-H4 models              DONE  (#40 correctness, PR #30 parity)
        ↓
frozen #31 primary performance campaign        PARTIAL  (historical)
        ↓
real-project performance measurement           DONE  (#31, PR #47 Campaign-2)
        ↓
mechanism attribution + controlled scaling      DONE  (#48 synthesis, Campaign-2 axes)
        ↓
H4 large-N causal decomposition                 DONE  (#50, PR #51)
        ↓
Weakness Map + evidence-backed V1 requirements  DONE  (markit-31-research-synthesis.md)
        ↓
replication / optimization sensitivity          HISTORICALLY NOT DONE HERE
        ↓
Markit-specific algorithm (new horse)           was NEXT at that stage
        ↓
architecture / production implementation        blocked until earned
```

The later R0–RQ8 Horse-A study completed the replication/optimization-sensitivity
obligation and froze Horse-A v1 as an immutable baseline; see the current
baseline record.

## #76 Gate A — H0–H4 donor fidelity (historical progression)

Gate A of [#76](https://github.com/jnhu76/markit/issues/76) audited whether
the H0–H4 horses were defensible controlled models of their donor mechanisms
before any donor-level interpretation of a Horse-A comparison. Four
independent donor-first reviewers (R1–R4) initially produced:

```text
H0 = FAITHFUL_MECHANISM_MODEL               H2 = MATERIAL_DEVIATION (2×P1)
H1 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION  H3 = MATERIAL_DEVIATION (2×P1)
H4 = FAITHFUL_MECHANISM_MODEL               GATE A = FAIL / BLOCKED
```

Evidence: [h0-h4-donor-fidelity/](h0-h4-donor-fidelity/) (contracts, donor
manifest, challenge-case record) and
[reviews/gate-a-r1-h0-h1-2026-09-28.md](reviews/gate-a-r1-h0-h1-2026-09-28.md)
… [gate-a-r4-h4-2026-09-28.md](reviews/gate-a-r4-h4-2026-09-28.md).

Both blockers were then repaired as mechanism repairs and independently
re-reviewed: H2 (#79, PR #82, merge `a3a551f`) became
**FAITHFUL_MECHANISM_MODEL**; H3 (#80, PR #83, merge `a8327fb`) became
**FAITHFUL_WITH_DECLARED_SIMPLIFICATION**. The fresh synthesis re-evaluation
from merged master `252bd6c` returned P0=0/P1=0 and no MATERIAL_DEVIATION, so
**Gate A later passed**. Gate B subsequently passed, the six-horse primary
campaign completed and was sealed, PMU explanation completed, and #76 is now
CLOSED / COMPLETE. The initial FAIL remains historical evidence and is not
rewritten away.

Corrective re-review records:
[reviews/gate-a-rh2-h2-recheck-2026-09-28.md](reviews/gate-a-rh2-h2-recheck-2026-09-28.md),
[reviews/gate-a-rh3-h3-recheck-2026-09-28.md](reviews/gate-a-rh3-h3-recheck-2026-09-28.md).

## Active workspace

`research/benchmarks/markdown-ast-update/` is the Markdown parser research
workspace used by the completed study.

Its assets are separated by lifecycle:

```text
protocol/prior-art/grammar/corpus/mutations/cases
    frozen research authority and synthetic controls

common/instrumentation/oracle/runner/corpusgen/shared-grammar/mechanisms
    executable benchmark substrate

projects
    pinned real-project manifests, eligibility, workload profiles

traces
    deterministic project edit traces

results
    raw machine-readable measurements and derived summaries

analysis
    attribution, scaling, crossover, replication analysis

report
    reviewed Weakness Map and paper-like presentation artifacts
```

See `research/benchmarks/markdown-ast-update/STRUCTURE.md` for ownership rules.

## Evidence rule

A performance statement must not stop at a timing adjective. Promoted results
must connect:

```text
project/file distribution
-> edit family
-> p50/p95 latency / throughput / CPU / memory
-> work counters
-> mechanism-specific state/work
-> controlled explanation
-> optimization-sensitivity qualification where applicable
```

Synthetic corpora are controlled explanatory tools. Real-project measurements
are the primary realism surface. Final Horse-A v1 claims must additionally obey
the completed RQ8 profile-sensitivity qualification recorded in the baseline
experimental record.
