# HORSE-A-V2-L2-100-PROFILE-MANIFEST-v1

Issue #100 — Horse-A v2 L2 paired context-profile campaign.
Frozen before collection (the sampling contract and repetition model below
were fixed after the call-chain quality pilot and before the campaign;
repeats use the identical contract).

## Frozen sampling contract (identical for all six profiles, both arms)

```text
EVENT              = cycles:u (the only L2 localization event)
PERIOD             = fixed -c 100000 (no adaptive frequency)
CALL_CHAIN         = --call-graph fp (frame-pointer lane; see PROVENANCE for
                     why the DWARF-unwind attempt was rejected by the pilot)
SYMBOLIZATION      = perf script --inline (debug=2 profiling build expands
                     inlined frames; bare DWARF method names)
AFFINITY           = taskset -c 2 (same pinned core, every profile)
BINARY             = target/release-l2-profile-v1/mdbench-horse-a-v2-l2
                     sha256 ee9b680085128993507fecbeb9fbbb141d0358490a455af56d051de63592de09
                     (ONE binary family, both arms, all cells)
PERF               = perf version 7.2.5-200.fc44.x86_64
HOST TWEAK         = kernel.perf_event_mlock_kb 516 -> 8192 (ring capacity,
                     P-LANE infrastructure only)
REPETITION MODEL   = warmup ops OUTSIDE the marker region, then N region ops;
                     N is fixed per CELL and identical for the P1/P0 pair
CELL-REP BINDING   = E6-1: N=100 (warmup 20) · E6-5: N=100 (warmup 20) ·
                     E6-6: N=1500 (warmup 30)
OPERATION          = the frozen #98 decompose probe, verbatim:
                     P1 = horse_a::full_build(post) alone
                     P0 = H0 full_parse(post) + complete alone
                     build -> black_box -> drop per iteration; the drop stays
                     inside the marker region (a sampler cannot exclude it by
                     time) and is attributed to a separate teardown bucket
REPEATS            = rep1 + rep2, two independent executions of the whole
                     six-entry manifest under the same contract
SAMPLE ADEQUACY    = region samples >= 1000 per profile (gate passed by all
                     twelve collections; smallest = 3979)
```

## The six frozen manifest profiles (+ identical-contract repeats)

| Profile | Cell | Arm | Ops | Region samples rep1/rep2 | Resolved rep1/rep2 |
|---|---|---|---:|---|---|
| 1 | E6-1-LOSE-DUP-CHANGE | P1_FULL_BUILD_ONLY | 100 | 15899 / 16104 | 99.2% / 99.1% |
| 2 | E6-1-LOSE-DUP-CHANGE | P0_H0_FULL_PARSE_ONLY | 100 | 4047 / 4041 | 96.9% / 96.4% |
| 3 | E6-5-HIGH-FANOUT-VALUE | P1_FULL_BUILD_ONLY | 100 | 15713 / 15899 | 99.1% / 99.0% |
| 4 | E6-5-HIGH-FANOUT-VALUE | P0_H0_FULL_PARSE_ONLY | 100 | 4155 / 3979 | 95.6% / 96.8% |
| 5 | E6-6-FENCE-HIDE-DEF | P1_FULL_BUILD_ONLY | 1500 | 14555 / 14566 | 100.0% / 100.0% |
| 6 | E6-6-FENCE-HIDE-DEF | P0_H0_FULL_PARSE_ONLY | 1500 | 10996 / 11865 | 100.0% / 100.0% |

Cell identity is inherited byte-for-byte from the sealed #98 manifest
(`HORSE-A-V2-DIAG-98-CELL-MANIFEST-v1`, `horse-a-v2-diag::cells::frozen_cells`).
No cell, edit, or arm was redefined for L2.

## Artifacts per profile

```text
raw/perf_<CELL>_<ARM>_<REP>.data     raw perf record output (git-ignored,
                                     retained on the execution host)
mid/perf_<CELL>_<ARM>_<REP>.txt      perf script --inline dump (git-ignored)
folded/perf_<CELL>_<ARM>_<REP>.folded  ROOT->LEAF folded stacks (committed)
receipts/run_*.json                  harness receipt (cell id, arm, ops, wall)
receipts/quality_*.json              call-chain quality gate metrics
receipts/attribution_*.json          machine-readable context attribution
receipts/differential_rep{1,2}.{json,md}  paired P1-P0 attribution
receipts/sha256_*.txt                checksums of raw/mid/folded + binary
```
