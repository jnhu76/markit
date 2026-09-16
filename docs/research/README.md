# Markit research status

This page is the campaign status map for Markit parser research.

```text
ACTIVE:
    #22 MARKIT-MARKDOWN-BENCHMARK-1
    Standardized Markdown AST/CST update benchmark:
    operations, payloads, work amplification, performance attribution.
    Area: research/benchmarks/markdown-ast-update/

ARCHIVED:
    Experiment 0 — MARKIT-INCREMENTAL-MARKDOWN-PARSER-SURVEY-1 (#19, PR #20)
    Historical evidence only. Physical archive:
    research/experiments/experiment-0-parser-survey/

SUPERSEDED:
    #21 MARKIT-MARKDOWN-ARCHITECTURE-1 (CLOSED)
    Treated Experiment 0 as architecture input; that sequencing is replaced
    by benchmark-first research. Do not resume it.
```

## The only research path currently authorized

```text
existing Markdown parser / AST-CST update algorithms
        ↓ #22 standardized benchmark          <- ACTIVE
        ↓ Weakness Map
        ↓ Markit-specific algorithm (future issue)
        ↓ formal/correctness work (as applicable)
        ↓ architecture synthesis
        ↓ production implementation (BLOCKED)
```

Experiment 0 produced useful reconnaissance (locality, structural
propagation, absolute-offset metadata cost, full/incremental crossover,
syntax-vs-semantic invalidation separation), but it compared mechanisms on a
non-normalized footing. Its `HYBRID` direction verdict and FROZEN / NOT-FROZEN
lists are hypotheses, not decisions: `green-tree prototype != production
representation`, `ReferenceIndex != production semantic index`, `HYBRID !=
current architecture`, `P0-02 != current parser candidate`.

## Rules

- Research documents may propose mechanisms; they do not become architecture
  authority automatically.
- The lifecycle is `question -> experiment -> evidence -> verdict -> reviewed
  architectural decision`.
- Old experiments remain evidence within their original setup; old
  recommendations are not inherited into the new Markit architecture by
  default.
- New benchmark evidence belongs under
  `research/benchmarks/markdown-ast-update/results/` (raw local, curated
  committed) once #22 defines the protocol.
