# Markit research status

This page is the campaign status map for Markit parser research.

## Campaign-2 synthesis and algorithm-design handoff

Issue [#48](https://github.com/jnhu76/markit/issues/48) preserves the complete
independent analysis and follow-up guidance requested by the maintainer:

- [Campaign-2 mechanism synthesis](campaign-2-mechanism-synthesis.md): A–J
  evidence verdict, regime/causal maps, lifecycle, weaknesses, and limitations.
- [From experiment to AST implementation](campaign-2-to-ast-guidance.md):
  the engineering direction and staged rationale.
- [Markit AST/update algorithm design v0](markit-ast-update-design-v0.md):
  **research design candidate**, including representation, coordinates,
  restart/convergence, semantic dependencies, full-rebuild selection,
  invariants, and the smallest discriminating experiment.

The reviewed evidence is pinned to PR #47 at `819928966e05ca00bb82c58b887014e54f3d0cd5`,
with mechanism authority `3762b7a42e1c284a4c2c2e0ebac8496e70c63431`.
These documents do not assert PR #47 is merged, close #22/#31/#33, authorize
production implementation, or lift the product architecture HOLD. The
candidate's concrete data structures and benefits remain subject to the
scoped research validation described in the design.

## Campaign authority and substrate

```text
ACTIVE UMBRELLA:
    #22 MARKIT-MARKDOWN-BENCHMARK-1
    Controlled Rust comparison of H0-H4 Markdown update mechanisms.

ACTIVE EXECUTION ISSUE:
    #31 project-driven performance evaluation for H0-H4
    Real-project corpus -> deterministic edit traces -> timing/work lanes
    -> attribution -> Weakness Map.

COMPLETED SUBSTRATE:
    R0 methodology                         PASS
    R1 controlled harness                  PASS
    R2 prior-art extraction                PASS
    R3 grammar/corpus/mutation freeze      PASS
    R4 H0 reference                        PASS
    R5 H1-H4 correctness/parity            PASS (PR #30 merged)

ARCHIVED:
    Experiment 0 — #19 / PR #20
    research/experiments/experiment-0-parser-survey/

SUPERSEDED:
    #21 MARKIT-MARKDOWN-ARCHITECTURE-1
```

## Authorized research path

```text
prior art / mechanism extraction
        ↓
controlled Rust substrate
        ↓
correctness-complete H0-H4 models          DONE
        ↓
real-project performance measurement       #31 NOW
        ↓
mechanism attribution + controlled scaling
        ↓
replication / optimization sensitivity
        ↓
Weakness Map
        ↓
Markit-specific algorithm                  future issue
        ↓
architecture / production implementation   blocked until earned
```

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
