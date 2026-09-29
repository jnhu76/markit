# Horse-A v1 baseline figures

Generated PNG figures for the Horse-A v1 baseline experimental record
([`docs/research/horse-a-v1-baseline-experimental-record.md`](../../horse-a-v1-baseline-experimental-record.md),
issue #91). All eight figures are rendered deterministically with ECharts
(SSR SVG rasterized via resvg) from **sealed derived authority artifacts** —
never from hand-copied prose numbers (baseline record §18 provenance rule).

| File | Figure | Primary source artifact(s) |
|---|---|---|
| `01-six-mechanism-edit-write-speedup.png` | Six-mechanism EDIT_WRITE speedup vs H0 (§4) | R7 `stage-b-facts-v1.json` |
| `02-horse-a-size-regime-q1-q4.png` | Horse-A Q1→Q4 size scaling (§6) | R8 `regime-map.csv` + RQ8 `rq8-sensitivity-facts-v1.json` cross-check |
| `03-horse-a-edit-family-regimes.png` | Selected edit-family / regime speedups (§7) | R7 `stage-b-facts-v1.json` + R8 `regime-map.csv` |
| `04-edit-latency-allocation-pareto.png` | Latency × allocation Pareto view (§11) | R8 `r8-values.json` + R7 `stage-b-facts-v1.json` |
| `05-h4-vs-horse-a-pmu-profile.png` | H4 vs Horse-A PMU efficiency profile (§12) | PMU `pmu-cells-v1.json` |
| `06-rq8-thinlto-vs-ltooff.png` | ThinLTO vs LTO-off sensitivity (§5) | RQ8 `rq8-sensitivity-facts-v1.json` |
| `07-pre-registered-weaknesses-vs-observed.png` | Pre-registered weaknesses vs observed evidence (§13) | baseline §13 (qualitative) + sealed extraction for all numbers |
| `08-weakness-to-next-mechanism-priority.png` | Weakness → next-mechanism priority matrix (§16) | baseline §16 (qualitative) + sealed extraction for all numbers |

## Provenance

- Every numeric value plotted here is extracted by
  [`scripts/research/horse-a-v1-baseline-figures/lib/extract.mjs`](../../../scripts/research/horse-a-v1-baseline-figures/lib/extract.mjs)
  from committed sealed-input copies under
  `scripts/research/horse-a-v1-baseline-figures/inputs/`. Their SHA-256s are
  verified at generation time against that directory's `SHA256SUMS`, whose
  entries are exactly the verified R0–RQ8 capsule's own `SHA256SUMS` entries
  (capsule `156ec1f3…`, baseline record §3).
- `manifest.json` (GENERATED) records, per figure: source files + hashes,
  extraction/transformation description, full-precision plotted values, and
  the PNG SHA-256.
- Display rounding is uniform and matches the frozen baseline tables:
  speedups 4 dp where the baseline freezes 4 dp (§4/§5) and 3 dp where the
  sealed source is 3 dp (§6/§7); allocation KB = bytes/1000 at 1 dp (§11
  convention); PMU instructions exact integers, MPKI 1 dp (§12 published
  table). Full precision is retained in `manifest.json`.
- Figures 7–8 are qualitative restatements of the frozen baseline record
  §13/§16 (no new aggregation, no new conclusions). The text comes from
  `inputs/qualitative-verdicts.json`, which contains no numeric values; every
  number shown in those figures is injected from the same sealed extraction
  used by Figures 1–4.

## Regenerate

```bash
cd scripts/research/horse-a-v1-baseline-figures
npm install        # first time only
npm run generate   # one command regenerates all 8 PNGs + manifest
```

The pipeline fails closed on sealed-input drift or on any frozen-value guard
mismatch. Reruns are byte-identical for every generated artifact (PNGs,
`manifest.json`, and the derived projection
`scripts/research/horse-a-v1-baseline-figures/derived/figures-data.json`);
`git diff` after a rerun is the practical determinism check.

## Latency representation note (Figure 4)

Figure 4 expresses latency as the EDIT_WRITE project-macro speedup vs H0 —
the same frozen §4 numbers as Figure 1 (relative latency = 1/speedup). The
x-axis is the sealed §11 median allocated KB per edit (1000 B convention) and
bubble area encodes the median allocation count. On this view Horse-A is
non-dominated (lowest allocation, first-tier speedup), together with H4 on
the non-dominated frontier.
