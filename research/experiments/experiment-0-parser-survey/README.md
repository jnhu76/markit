# Experiment 0 — Incremental Markdown Parser Survey (ARCHIVE)

```text
Experiment:
    MARKIT-INCREMENTAL-MARKDOWN-PARSER-SURVEY-1

Issue:
    #19

PR:
    #20

Merge:
    55f6326f360f77d2caf46bb60571c8f15a88de53

Status:
    ARCHIVED EXPERIMENT 0

Authority:
    historical evidence only

Successor:
    #22 MARKIT-MARKDOWN-BENCHMARK-1
```

This directory is the physical archive of Markit's first parser research
experiment (Experiment 0). It is retained for reproducibility and evidence.
It is **not** active research, **not** a product component, and **not** the
current Markdown parser direction.

The active Markdown parser research authority is
issue #22 (`research/benchmarks/markdown-ast-update/`). The architecture
remains on HOLD (`docs/product/architecture.md`).

## Explicit non-identity statements

Nothing in this archive may be promoted by implication. In particular:

```text
green-tree prototype  != production representation
ReferenceIndex        != production semantic index
HYBRID                != current architecture
P0-02                 != current parser candidate
markit-core snapshot  != future product core
```

The `HYBRID` direction verdict, the FROZEN / NOT-FROZEN lists, and every
measured number here were produced **before** any normalized cross-parser
benchmark existed. They are hypotheses and evidence summaries, not decisions.
Issue #21 tried to promote them into architecture synthesis and was closed as
SUPERSEDED; do not resume that sequencing.

## Layout (moved by MARKIT-EXPERIMENT-FIRST-REPO-RESET-1)

```text
old path (removed)              this archive
------------------------------  ------------------------------------------------
crates/parser-survey/**         harness/          (survey harness; own workspace)
crates/markit-core/**           baseline/markit-core/  (measured subject snapshot)
results/summary/parser-survey-* results/summary/  (curated evidence, committed)
results/raw/parser-survey/      results/raw/      (local raw output, gitignored)
docs/research/parser-survey-1.md     docs/parser-survey-1.md
docs/research/markdown-parser/       docs/markdown-parser/
docs/research/code-baseline.md       docs/code-baseline.md
```

`baseline/markit-core` is kept only because the harness measures its public
surface (`MarkdownState::build` / `update`, `MarkdownWork`, `BlockView`).
It is the Experiment 0 measurement subject — a pre-reset snapshot — and has
no product or future-architecture role.

## Independent reproduction

From the repository root (no root Cargo workspace exists; this archive is
its own workspace):

```sh
cargo test --manifest-path \
  research/experiments/experiment-0-parser-survey/Cargo.toml --release
```

Run modes (see `harness/README.md`):

```sh
cargo run --release --manifest-path \
  research/experiments/experiment-0-parser-survey/harness/Cargo.toml -- \
  [--sizes 1k,10k,100k,1m] [--iters N] [--warm N] [--out DIR] \
  [--md4c | --ts | --lezer | --green | --green-history | --refs | --commonmark [PATH]]
```

The Lezer baseline additionally requires the pinned Node toolchain —
`@lezer/markdown 1.7.2`, `@lezer/common 1.5.2`, Node v24.15.0 — restored
exactly from the committed lockfile:

```sh
cd research/experiments/experiment-0-parser-survey/harness/scripts
npm ci

cargo run --release --manifest-path \
  research/experiments/experiment-0-parser-survey/harness/Cargo.toml -- \
  --lezer --sizes 1k --iters 1 --out DIR
```

Relocation notes (path fixes only, no semantic change):

- the harness locates `scripts/lezer-worker.js` and the default
  CommonMark spec file via `CARGO_MANIFEST_DIR` instead of
  repository-root-relative paths;
- `baseline/markit-core` is resolved through this archive workspace.

Toolchain, git SHA, corpus version, and benchmark mode for each recorded run
are in the `results/summary/*.md` evidence files.
