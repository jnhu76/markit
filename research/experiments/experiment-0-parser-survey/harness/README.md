# parser-survey (ARCHIVED Experiment 0 harness)

Research harness for issue #19 (MARKIT-INCREMENTAL-MARKDOWN-PARSER-SURVEY-1).

**ARCHIVED — historical evidence only.** Not product code; not part of any
product/default workspace; this directory and its `markit-core` measurement
subject snapshot live only inside the Experiment 0 archive. The active
Markdown research authority is issue #22
(`research/benchmarks/markdown-ast-update/`).

## What it measures

For every (corpus document, mutation) scenario:

- `full_us` — median wall-clock of a clean full parse of the post-edit
  document (the M0 control);
- `inc_us` — median wall-clock of the incremental `MarkdownState::update`;
- the implementation's own structural `MarkdownWork` counters (bytes/lines
  scanned, blocks reparsed, survivor ranges rewritten, Vec records moved,
  convergence point);
- allocation count/bytes for both paths (counting global allocator);
- projection invalidation (`proj_*`): which blocks a position-keyed
  downstream consumer must re-project, computed by prefix (same absolute
  ranges) / suffix (same distance-to-EOF, same source bytes) stable-region
  merge. Pure offset shifts are excluded here and reported by the
  `survivor_blocks_shifted` / `block_records_moved` counters instead —
  together these form the hidden-O(N) gate (PARSE-INV-08);
- the correctness oracle: incremental state == clean full rebuild on the
  observable public surface (kind, ranges, detail, inline IR).

## Usage

From the archive root (or with `--manifest-path` from anywhere):

```sh
cargo run --release --manifest-path \
  research/experiments/experiment-0-parser-survey/harness/Cargo.toml -- \
  [--sizes 1k,10k,100k,1m] [--iters N] [--warm N] [--out DIR]
```

Outputs `cases.csv` (one row per scenario), `summary.md`, `meta.txt`
(toolchain, git SHA, args) under `--out`.
