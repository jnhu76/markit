# Report / Weakness Map

This directory is reserved for the **later reviewed synthesis stage** of the
benchmark campaign. It is not a scratch area for tentative #31 observations.

Expected eventual contents:

```text
report/
  README.md
  weakness-map.md
  project-results.md
  figures/
  paper/              optional manuscript material
```

The report layer is presentation, not raw evidence authority. Every numerical
claim must trace back to `results/` and, for causal claims, to reviewed
`analysis/` artifacts.

During #31, candidate tables/figures and attribution work belong in
`analysis/`; machine-readable observations and summaries belong in `results/`.
Moving a conclusion into `report/` does not strengthen its evidence grade.

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
The final Weakness Map must pass its later human review gate first.
