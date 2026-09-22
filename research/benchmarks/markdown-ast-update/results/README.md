# Measurement results

Owns machine-readable output from #31 measurement campaigns.

Expected shape:

```text
results/
  README.md
  raw/
    <CampaignSpecId>/<RunId>/timing|attribution/*.jsonl
  summaries/
    <CampaignSpecId>/*.csv
  manifests/
    primary-performance-campaign-v1.toml   # frozen campaign contract (#31)
    primary-machine-v1.toml                # frozen primary benchmark machine
    primary-schedule-v1.jsonl              # frozen case/horse schedule
    primary-campaign-receipt-v1.json       # SHA256 bindings of the freeze
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

The `primary-*` manifests are the frozen #31 primary performance campaign
execution contract (`protocol/R7-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-v1.md`):
the receipt SHA256-binds them plus the workload/schemas/toolchain artifacts;
raw timing output (when the RUN task executes) lands only under
`results/raw/<CampaignSpecId>/<RunId>/` as append/create-only JSONL
campaign-observation envelopes. No primary timing exists yet.

Human causal interpretation belongs in `analysis/`, not in raw result files.
