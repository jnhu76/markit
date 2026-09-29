# horse-a-v1-baseline-figures — deterministic ECharts figure pipeline

Generates the eight PNG figures embedded in
[`docs/research/horse-a-v1-baseline/`](../../../docs/research/figures/horse-a-v1-baseline/)
for the Horse-A v1 baseline experimental record (#91). Docs/scripts only —
this package contains no benchmark, mechanism, or editor code and never runs
experiments.

## Usage

```bash
npm install        # first time only (echarts + @resvg/resvg-js, pinned)
npm run generate   # one command regenerates everything
```

Outputs:

- `../../../docs/research/figures/horse-a-v1-baseline/0*.png` — the figures
- `../../../docs/research/figures/horse-a-v1-baseline/manifest.json` —
  per-figure provenance (sources, extraction description, full-precision
  values, PNG SHA-256)
- `derived/figures-data.json` — the deterministic machine-readable projection
  all figures are rendered from

## How it works

```text
inputs/ (sealed derived artifacts, SHA256SUMS-verified at runtime)
    -> lib/extract.mjs      extraction + frozen-value guards (fail closed)
    -> derived/figures-data.json
    -> lib/charts.mjs       ECharts option builders (display only)
    -> lib/render.mjs       ECharts SSR SVG -> resvg -> PNG (1600x900, white)
    -> docs/research/figures/horse-a-v1-baseline/*.png + manifest.json
```

- **Rendering is ECharts end to end**: `echarts` produces the SVG (SSR mode),
  `@resvg/resvg-js` rasterizes it with pinned DejaVu Sans fonts. No other
  plotting library is involved.
- **Fail-closed integrity**: every input is checked against
  `inputs/SHA256SUMS` (whose entries are the verified R0–RQ8 capsule's own
  manifest entries — see `inputs/PROVENANCE.md`), and every plotted value
  must match the frozen baseline-record guards; any drift aborts the run
  before a PNG is written.
- **Determinism**: fixed canvas, fixed fonts, animation off, no timestamps in
  outputs. Two consecutive runs produce byte-identical PNGs, manifest, and
  derived projection; `git diff` after a rerun is the practical check.
- **Roundtrip honesty**: `derived/figures-data.json` is committed, so the
  exact values behind every rendered pixel are reviewable without running
  anything.

## Layout (why this directory)

The repo has no top-level `scripts/` tree yet; this path follows the task's
preferred layout (`scripts/research/<topic>/`) and keeps the pipeline beside
its `inputs/` so the provenance chain (capsule → SHA256SUMS → committed copy
→ extraction → figure) is inspectable in one place. The benchmark research
tree under `research/benchmarks/markdown-ast-update/` was deliberately not
touched: those directories hold the active #22 benchmark substrate and
sealed-campaign material, and figure presentation is a docs concern.

## Files

```text
generate-all.mjs            entry point (npm run generate)
lib/extract.mjs             extraction + SHA-256 verification + frozen guards
lib/charts.mjs              the eight ECharts option builders
lib/render.mjs              ECharts SSR + resvg PNG helper
inputs/                     sealed derived artifacts (PROVENANCE.md, SHA256SUMS)
inputs/qualitative-verdicts.json   §13/§16 text (no numbers — see PROVENANCE.md)
derived/figures-data.json   generated deterministic projection (committed)
```
