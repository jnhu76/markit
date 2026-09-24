# Markit research status

This page is the campaign status map for Markit parser research.

## Research synthesis and algorithm-design handoff

Read the synthesis first; it is the cross-stage entry point.

- **[Markit-31 research synthesis](markit-31-research-synthesis.md)** — canonical
  handoff: what is proven / not proven, how the evidence changed the
  understanding (Stage 0→8), the final H0–H4 Weakness Map, the evidence-to-requirement
  traceability matrix (R1–R6), the methodological lessons, and the next research
  question. **Start here.**

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
                                (PRIMARY_TIMING = NOT_STARTED; no timing evidence)
Campaign-2 evidence             PR #47 merge  334eea6201fc0258e35a7c5b21feb722641ddcbd
H4 large-N root cause (#50)     PR #51 merge  6cec47e9bb756affb0a4477bcb4962c3c78d8901
```

These documents do not close #22/#31/#33/#48, authorize production implementation,
or lift the product architecture HOLD. The candidate's concrete data structures
and benefits remain subject to the scoped research validation described in the
design; `V1_DATA_STRUCTURE = UNDECIDED`.

## Campaign authority and substrate

```text
ACTIVE UMBRELLA:
    #22 MARKIT-MARKDOWN-BENCHMARK-1
    Controlled Rust comparison of H0-H4 Markdown update mechanisms.

ACTIVE EXECUTION ISSUE:
    #31 project-driven performance evaluation for H0-H4
    Real-project corpus -> deterministic edit traces -> timing/work lanes
    -> attribution -> Weakness Map.

ACTIVE INTERPRETATION ISSUE:
    #33 paper-style regime map study
    #48 Campaign-2 mechanism synthesis / A-J interpretation
    (their stage work is recorded, but the issues themselves stay open —
    nothing here closes them; only the maintainer does)

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

## Authorized research path

`DONE` below means *the stage's evidence is recorded on merged master*. It does
not mean the umbrella or execution issues are closed: #22, #31, #33 and #48 all
stay open until the maintainer closes them.

```text
prior art / mechanism extraction
        ↓
controlled Rust substrate
        ↓
correctness-complete H0-H4 models              DONE  (#40 correctness, PR #30 parity)
        ↓
frozen #31 primary performance campaign        PARTIAL  (collection closure only;
        ↓                                                 PRIMARY_TIMING = NOT_STARTED)
real-project performance measurement           DONE  (#31, PR #47 Campaign-2)
        ↓
mechanism attribution + controlled scaling      DONE  (#48 synthesis, Campaign-2 axes)
        ↓
H4 large-N causal decomposition                 DONE  (#50, PR #51)
        ↓
Weakness Map + evidence-backed V1 requirements  DONE  (markit-31-research-synthesis.md)
        ↓
replication / optimization sensitivity          NOT DONE as a stage — no replication
        ↓                                       campaign exists; Campaign-2's
        ↓                                       controlled axes and #50's single-factor
        ↓                                       ablations are diagnosis, not replication
Markit-specific algorithm (new horse)           NEXT  — not started
        ↓
architecture / production implementation        blocked until earned
```

The last step is still unauthorized. A new candidate mechanism must first be
compared against H0–H4 under the same semantic core, oracle, payload/edit
contract, workload and measurement protocol. It must not be defined as
"H4 but optimized" before its mechanism identity is explicitly designed.

## Active workspace

`research/benchmarks/markdown-ast-update/` is the only active Markdown parser
research workspace.

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
```

Synthetic corpora are controlled explanatory tools. Real-project measurements
are the primary realism surface for #31.
