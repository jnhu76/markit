# Markit

**Markit** is a local-first Markdown editor/workspace. Markdown source is the
single document truth.

## Current phase

```text
Markdown parser algorithm research — issue #22
```

The current active campaign is a **standardized Markdown AST/CST update
benchmark** across existing parsers and update designs. It is research, not
product implementation:

```text
existing Markdown parser / AST-CST update algorithms
        ↓ standardized benchmark (#22)
        ↓ Weakness Map
        ↓ Markit-specific algorithm (future)
        ↓ formal/correctness work (future)
        ↓ architecture (future)
        ↓ production implementation (future, BLOCKED now)
```

## Authority map

```text
Product requirements        docs/product/   (docs/PRD.md is the top authority)
Active research             issue #22 — MARKIT-MARKDOWN-BENCHMARK-1
                            research/benchmarks/markdown-ast-update/
Historical experiments      research/experiments/   (evidence only)
Markdown architecture       NOT YET FROZEN — docs/product/architecture.md is HOLD
Production parser           NOT YET DEFINED
```

## Repository layout

```text
docs/                                   product truth + research records
research/
  experiments/experiment-0-parser-survey/   archived #19 survey (evidence only)
  benchmarks/markdown-ast-update/           active #22 benchmark area (skeleton)
```

There is no root Cargo workspace and no production implementation in this
repository right now. The pre-reset implementation and all superseded
material are preserved in Git history:

```text
pre-reset repository:  d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
Experiment 0 code:     present before the MARKIT-EXPERIMENT-FIRST-REPO-RESET-1
                       branch (now archived under research/experiments/)
```

## Working rules

Read `AGENTS.md` before changing anything, and `docs/PRD.md` for what the
product must do. The current rule:

> **Benchmark first. No production parser, no Markit algorithm claims, and no
> architecture before #22 produces its measurement surface and Weakness Map.**
