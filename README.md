# Markit

**Markit** is a local-first Markdown editor/workspace. Markdown source is the
single document truth.

## Current phase

Markit is still in parser-algorithm research. The correctness substrate is now
complete through R5; the active execution work is the project-driven
performance campaign in issue **#31**, under the umbrella methodology of
**#22 MARKIT-MARKDOWN-BENCHMARK-1**.

```text
existing Markdown update mechanisms
        ↓ #22 controlled Rust mechanism benchmark
        ↓ R0-R5 correctness / parity / attribution substrate   PASS
        ↓ #31 project-driven performance evaluation            ACTIVE
        ↓ Weakness Map
        ↓ Markit-specific algorithm                            future
        ↓ formal/correctness work                              future
        ↓ architecture synthesis                               future
        ↓ production implementation                            future
```

R5 merged as PR #30. H0-H4 are now correctness-complete mechanism models; no
formal performance ranking exists until #31 records benchmark measurements.

## Authority map

```text
Product requirements
  docs/PRD.md + docs/product/**

Research status / navigation
  docs/research/README.md
  research/README.md

Benchmark methodology + frozen history
  issue #22
  research/benchmarks/markdown-ast-update/protocol/

Current performance execution
  issue #31
  research/benchmarks/markdown-ast-update/{projects,traces,results,analysis,report}/

Historical experiments
  research/experiments/                  evidence only

Production parser / final architecture
  NOT YET DEFINED
```

## Repository layout

```text
docs/
  product/                    product truth and invariant boundaries
  research/                   research status and cross-campaign records

research/
  README.md                   research navigation
  experiments/                closed / historical experiments
  benchmarks/
    markdown-ast-update/      active controlled benchmark workspace
```

The benchmark workspace has its own Rust Cargo workspace; there is deliberately
no root production Cargo workspace yet.

## Working rule

Read `AGENTS.md` before changing anything. The current rule remains:

> **Measure first. Do not design the production Markit parser or architecture
> from unmeasured mechanism intuition.**

The next evidence milestone is not another correctness gate. It is concrete
project-level latency, throughput, CPU/memory/allocation, work-amplification,
and mechanism-attribution data from #31.
