# Report / Weakness Map

Owns reviewed human-facing outputs from the benchmark campaign.

Expected contents:

```text
report/
  README.md
  weakness-map.md
  project-results.md
  figures/
  paper/              optional manuscript material
```

This directory is the presentation layer, not raw evidence authority.
Every numerical claim must trace back to `results/` and, for causal claims, to
`analysis/`.

The final per-mechanism profile should eventually report:

```text
MECHANISM
PRIOR_ART_ANCHOR
BEST REGIME
WORST REGIME
SCALING SIGNATURE
WORK AMPLIFICATION
MEMORY / ALLOCATION COST
FAILURE / FALLBACK REGIME
KEY STRENGTH
KEY WEAKNESS
EVIDENCE
OPTIMIZATION_SENSITIVITY
CONFIDENCE
```

Do not design or implement the Markit production algorithm in this directory.
The Weakness Map must be reviewed first.
