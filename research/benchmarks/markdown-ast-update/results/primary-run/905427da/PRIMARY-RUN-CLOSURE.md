# MARKIT-31 PRIMARY-RUN-CLOSURE

Non-interpretive closure report for the frozen primary performance campaign
execution. This report records collection integrity and identities only.
It contains no horse rankings, no speedups, no p50/p95 comparisons, no
winner, no crossover, and no DQ answers.

## Authority and identities

```text
campaign_id            = MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1
authority merge SHA    = c20d8efb0a864b6b7f458ca170f11532fee78e0d  (PR #43 merge)
CampaignSpecId         = ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId                  = 905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
primary executable     = target/release/mdbench-campaign  (profile release-primary-v1)
executable SHA256      = a3ef4e63a8548604fff991c5b981e3dfc856238fd06ead3e9192a5a3b6acd5bb
machine_id             = primary-e5-xeon2666v3-fedora44
```

CampaignSpecId / RunId / executable SHA256 counts across all 8 receipts:
each exactly 1 distinct value. The RunId is new (the #43 merge changed the
executable/runner commit), as anticipated by the campaign contract.

## Launch gate (Phase A)

```text
verify-r7.sh                       = EXIT_CODE 0
VERIFY_OK                          = payloads=449 full_read_records=37 receipt_artifacts=6
DETERMINISM_OK                     = artifacts=7 byte-identical
CAMPAIGN_MANIFEST_OK               = campaign_id=MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1
CAMPAIGN_RECEIPT_OK                = spec_id=ad45c7cb...66f2 artifacts=14
SCHEDULE_DETERMINISM_PASS          = bytes=597147
CAMPAIGN_PREFLIGHT_PASS            = scopes All / Timing / Attribution; host_binding=enforced; blockers=[]
NON_RESEARCH_SMOKE_PASS            = timing_rows=20 attribution_rows=10 failure_propagated=true
PRIMARY_TIMING                     = NOT_STARTED by the gate (no real campaign clock above)
```

The MemTotal delta (4096 bytes) appeared only as
`identity_role = diagnostic_only` and did not block, per the frozen
corrective contract.

### Launch-gate attempt 1 (environment gap, documented)

Gate attempt 1 (log `logs/01-launch-gate.log`) failed inside
`cargo test --workspace`: two preflight unit tests panicked because the
materialized workload bytes were absent from this freshly created worktree
(`workloads/sources/*/files/` is gitignored by design, storage policy
`MATERIALIZE_ONLY`). No measurement had started; no raw file existed; no
source, workload, protocol, or manifest file was modified. The documented
replay path was executed (`workloads/README.md`: materialize locally on
demand, byte-verified; never re-pin, never refresh, never write the lock):

```text
logs/00b-corpus-materialization.log
  - local copy of gitignored files/ trees + _cache/ from this machine's
    existing verified materialization
  - python3 tools/acquire.py verify --full
      VERIFY PASS: 20 sources, 3970 materialized files byte-verified
      (66391919 bytes), 0 not materialized locally
```

Gate attempt 2 (`logs/01-launch-gate-attempt2.log`) then passed fully, as
recorded above. Classification: environment preparation gap before any
measurement; not a campaign, workload, or machine-identity failure.

## Formal lanes (Phases B–D)

Every lane executed the frozen binary directly, guarded by the SHA256 check
in `logs/run-lane.sh.txt` before each execution. Every lane returned
`FINAL_RAW_FILE_PASS` and its lane-complete marker with `EXIT_CODE=0`.

```text
attribution clean_state   = ATTRIBUTION_COMPLETE observations=110   logs/10
attribution edit_write    = ATTRIBUTION_COMPLETE observations=1810  logs/11

timing clean_state s0     = SESSION_COMPLETE rows=4400  warmup=1100  measured=3300  logs/20
timing clean_state s1     = SESSION_COMPLETE rows=4400  warmup=1100  measured=3300  logs/21
timing clean_state s2     = SESSION_COMPLETE rows=4400  warmup=1100  measured=3300  logs/22

timing edit_write s0      = SESSION_COMPLETE rows=72400  warmup=18100  measured=54300  logs/30
timing edit_write s1      = SESSION_COMPLETE rows=72400  warmup=18100  measured=54300  logs/31
timing edit_write s2      = SESSION_COMPLETE rows=72400  warmup=18100  measured=54300  logs/32
```

## Closure audit (Phase E) — computed from files

```text
successful raw JSONL   = 8
success receipts       = 8
invalid markers        = 0

attribution rows       = 110 + 1810             = 1920
clean_state timing     = 3 x 4400               = 13200
edit_write timing      = 3 x 72400              = 217200
timing total           = 13200 + 217200         = 230400
warmup total           = 57600
measured total         = 172800
grand raw observations = 230400 + 1920          = 232320
```

Each of the 8 raw files was independently re-hashed (sha256) and matched the
sha256 recorded in its receipt. Full inventory with paths, SHA256, byte
sizes, and row counts: `raw-inventory.txt`. Receipt copies and the receipt
index: `receipts/`, `receipt-index.txt`.

Raw data storage: `results/raw` is README-only by repository policy
("DO NOT COMMIT BENCHMARK OUTPUT", R1 task §17); the 350 MB raw run stays on
the primary machine at
`results/raw/ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2/905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d/`
and is not duplicated into Git. This directory is the evidence pointer to it.

## Integrity invariants held

```text
PRIMARY_EXECUTABLE_UNCHANGED  = YES  (start SHA == end SHA, a3ef4e63...d5bb)
no source / workload / protocol / manifest / schedule / Cargo mutation
no invalid marker created, no raw file deleted or rewritten
performance interpretation    = NOT_STARTED
```

## Failures

```text
execution failures  = none
correctness failures = none
(environment note: gate attempt 1 workload-materialization gap, resolved and
byte-verified before any measurement; see above and logs/00b, logs/01*)
```

## Verdict

```text
PRIMARY_PERFORMANCE_RUN_PASS
```
