# Measurement results

Owns machine-readable output from #31 measurement campaigns.

Expected shape:

```text
results/
  README.md
  raw/
    <campaign-id>/*.jsonl
  summaries/
    <campaign-id>/*.csv
  manifests/
    <campaign-id>.toml
```

Rules:

- `raw/` is immutable evidence emitted by the runner; never hand-edit rows.
- `summaries/` is derived from raw observations by checked-in analysis code.
- every campaign manifest pins runner commit, project/trace manifests, machine,
  toolchain, build profile, allocator, seeds, sessions, and lane.
- timing and attribution lanes remain distinguishable and join by stable case /
  trace identity.
- failed/unsupported/timeout/OOM/crash cases remain explicit rows; never drop
  them silently.

Human causal interpretation belongs in `analysis/`, not in raw result files.
