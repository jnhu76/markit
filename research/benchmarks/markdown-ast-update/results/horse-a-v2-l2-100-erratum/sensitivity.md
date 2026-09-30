# L2 evidence erratum — warmup-inclusion sensitivity analysis

```text
AUTHORITY      = SENSITIVITY ONLY (derived evidence; not a replacement campaign)
SOURCE         = retained L2 evidence (results/horse-a-v2-l2-100/, untouched)
                  campaign/mid/perf_*.txt          (12 perf-script dumps, timestamped)
                  campaign/receipts/run_*.json     (12 run receipts: warmup/region ops,
                                                    region_wall_ns)
                  campaign/receipts/attribution_*.json (12 committed attributions)
TOOLCHAIN      = python3; classifier = frozen horse-a-v2-l2/l2tools/classify.py
                 (imported verbatim); parser regexes/predicates = perf_fold.py
                 byte-identical (see erratum_phase_window.py)
RECOLLECTED    = NO (no perf record, no new campaign, no mechanism code touched)
HISTORICAL     = NOT MODIFIED (SHA256SUMS in this directory proves identity)
REPLAY GATE    = 12/12 profiles: reproduced per-bucket counts == committed
                 attribution buckets, integer-exact (see phase_window.json
                 "replay_gate")
```

## What this answers

The historical L2 pipeline published `samples/op` values whose numerator
retained **every** sample containing the `l2_p1_build` / `l2_p0_parse`
marker frames — a set that provably includes the **warmup** executions
(warmup calls the same functions from `main`; `perf record` launched the
process; the `profile_region` marker frame appears in **zero** of the
24 retained files) — while the denominator was **formal `region_ops`
only** (`classify.py` line 131). The published quantities are therefore
mixed-phase. This note quantifies how much that matters, using only
retained evidence.

## Normalizations compared

```text
A. HISTORICAL_REPORTED  = retained samples / region_ops
                          (what the L2 report published)
B. POOLED_NORMALIZED    = retained samples / (region_ops + warmup_ops)
                          (valid pooled per-profiled-op average; every
                          retained sample IS one of those ops)
C. WINDOW_WARMED_ONLY   = samples with timestamp >= (last in-region
                          timestamp - region_wall_ns) / region_ops
                          (ESTIMATE: the formal region is the final
                          region_wall_ns of the sampled in-region window,
                          anchored by the run receipt)
```

B and C are sensitivity calculations. **C is not claimed as the strict
TRUE_WARMED_ONLY quantity**: no phase boundary was recorded at collection
time, so the boundary is an inference. Its precision is bounded below.

## Boundary quality (why C is tight despite being an estimate)

- The sampled in-region window minus `region_wall_ns` equals the warmup
  phase duration almost exactly, and independently reproduces it from two
  directions (duration ↔ warmup sample count):
  E6-1 P1 rep1: 0.1116 s window residue ≈ 2 692 warmup samples;
  E6-5 P1 rep1: 0.0980 s ≈ 2 657; E6-6 P1 rep1: 0.0109 s ≈ 290.
- Shifting the boundary by ±0.5/±1/±2 ms moves the formal/warmup split
  by at most ±52 of 13 207 samples (±0.4%) on E6-1 P1 rep1; see
  `phase_window.json` → `profiles.*.boundary_sensitivity`.
- Warmup and formal per-op sample rates agree within ±2.4% on
  E6-1/E6-5 rep1 (max deviation E6-5/P0 rep1 −2.3%) and within ±2% on
  E6-6 P1 (rep2 E6-1 warmup runs +7.9%/op; E6-6 P0 rep2 warmup +49%/op
  on 343 small-count samples — the report already scopes E6-6 as
  qualitative).

## Phase split (per profile; totals in samples, rates in samples/op)

| profile | in-region total | warmup samples (per op) | formal per op (C) | historical per op (A) | pooled per op (B) |
|---|---:|---:|---:|---:|---:|
| E6-1 P1 rep1 | 15 899 | 2 692 (134.60) | 132.07 | 158.99 | 132.49 |
| E6-1 P0 rep1 | 4 047 | 669 (33.45) | 33.78 | 40.47 | 33.73 |
| E6-1 P1 rep2 | 16 104 | 2 859 (142.95) | 132.45 | 161.04 | 134.20 |
| E6-1 P0 rep2 | 4 041 | 687 (34.35) | 33.54 | 40.41 | 33.67 |
| E6-5 P1 rep1 | 15 713 | 2 657 (132.85) | 130.56 | 157.13 | 130.94 |
| E6-5 P0 rep1 | 4 155 | 679 (33.95) | 34.76 | 41.55 | 34.62 |
| E6-5 P1 rep2 | 15 899 | 2 639 (131.95) | 132.60 | 158.99 | 132.49 |
| E6-5 P0 rep2 | 3 979 | 665 (33.25) | 33.14 | 39.79 | 33.16 |
| E6-6 P1 rep1 | 14 555 | 290 (9.67) | 9.51 | 9.70 | 9.51 |
| E6-6 P0 rep1 | 10 996 | 217 (7.23) | 7.19 | 7.33 | 7.19 |
| E6-6 P1 rep2 | 14 566 | 291 (9.70) | 9.52 | 9.71 | 9.52 |
| E6-6 P0 rep2 | 11 865 | 343 (11.43) | 7.68 | 7.91 | 7.75 |

Certificate bucket (P1 arm only; P0 certificate is 0 everywhere):

| profile | A (historical) | C formal | C warmup |
|---|---:|---:|---:|
| E6-1 P1 rep1 | 103.79 | 86.38 | 87.05 |
| E6-1 P1 rep2 | 105.76 | 87.12 | 93.20 |
| E6-5 P1 rep1 | 103.72 | 86.21 | 87.55 |
| E6-5 P1 rep2 | 104.77 | 87.30 | 87.35 |
| E6-6 P1 rep1 | 1.30 | 1.28 | 1.23 |
| E6-6 P1 rep2 | 1.30 | 1.28 | 1.23 |

## Differential under the three normalizations (P1 − P0, samples/op)

| cell / rep | A excess → cert share | B excess → cert share | C excess → cert share |
|---|---|---|---|
| E6-1 rep1 | 118.52 → **87.6%** | 98.77 → 87.6% | 98.29 → 87.9% |
| E6-1 rep2 | 120.63 → 87.7% | 100.53 → 87.7% | 98.91 → 88.1% |
| E6-5 rep1 | 115.58 → **89.7%** | 96.32 → 89.7% | 95.80 → 90.0% |
| E6-5 rep2 | 119.20 → 87.9% | 99.33 → 87.9% | 99.46 → 87.8% |
| E6-6 rep1 | 2.37 → 55.0% | 2.33 → 55.0% | 2.32 → 55.1% |
| E6-6 rep2 | 1.80 → 72.4% | 1.77 → 72.4% | 1.84 → 69.7% |

### Reading rules (what may and may not be concluded)

1. **B's share is arithmetically identical to A's.** Both arms divide by
   the same total profiled ops, so the differential share is invariant
   under uniform rescaling. Warmup inclusion never changed the published
   *shares* — only the absolute samples/op magnitudes.
2. **Absolute magnitudes carry a uniform ≈ ×1.20 inflation on
   E6-1/E6-5** (warmup ops are ≈ 20/120 of profiled ops with rep1
   per-op rates within ±2.4%): e.g. certificate 103.79 → ≈ 86.4
   warmed-only samples/op;
   totals 158.99/40.47 → 132.07/33.78. On E6-6 the inflation is ≈ ×1.02.
3. **C moves the certificate share by at most 0.41 percentage points in
   absolute value**
   on E6-1/E6-5 (max: E6-1 rep2 87.7% → 88.1%; E6-5 rep2 moves
   −0.1 pp). The L2 report's
   qualitative localization — the P1−P0 construction excess is strongly
   concentrated in the certificate-barrier lookup context on E6-1/E6-5 —
   **survives the warmed-only reconstruction** (shares 87.8–90.0%).
4. **P1/P0 ratios are essentially unchanged**: E6-1 rep1 3.93 → 3.91
   (C); E6-5 rep1 3.78 → 3.76; E6-6 1.32 → 1.32.
5. E6-6's rep-to-rep share spread (55%–72%) reconfirms the original
   report's own scoping of E6-6 as qualitative-only.

## Status of the three quantities

```text
A. HISTORICAL_REPORTED   = PRESERVED as published (mixed-phase numerator,
                           formal denominator) — interpretation corrected
                           by the erratum, values unchanged
B. POOLED_NORMALIZED     = VALID pooled average (explicitly labeled);
                           sensitivity-only
C. WINDOW_WARMED_ONLY    = CONSTRAINED ESTIMATE (sensitivity-only);
                           TRUE_WARMED_ONLY = NOT RECOVERED strictly
                           (no recorded phase boundary exists in the
                           retained evidence)
```

Reproduce: `python3 erratum_phase_window.py && python3 make_warmup_audit.py`
(from this directory; reads only retained L2 evidence).
