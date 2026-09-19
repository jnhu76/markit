# Analysis

Owns reproducible interpretation of `results/` for #31.

This layer answers **why** a mechanism wins or loses. It may join timing and
work lanes, compute project aggregates, fit scaling signatures, identify
crossovers, and run controlled attribution/ablation analyses.

Expected shape:

```text
analysis/
  README.md
  scripts/
  tables/
  figures/
  records/
```

Every analysis artifact must identify its input campaign/result manifest.
Generated tables/figures should be reproducible from checked-in commands or
scripts; do not hand-edit numerical results.

Use the R0 attribution ladder:

```text
timing difference
-> scaling signature
-> work counters
-> allocation / bytes moved / representation maintenance
-> controlled ablation or counterexample
-> optimization-sensitivity check
-> PMU only if residual remains unexplained
```

A timing difference without mechanism evidence remains an observation.
