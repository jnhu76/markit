# Input provenance — Horse-A v1 baseline figures

Every file in this directory is a **sealed derived authority artifact** copied
verbatim from the verified R0–RQ8 research capsule. Nothing here is raw
evidence (no raw timing/perf counter rows), and nothing here was edited.

## Chain of custody

```text
BASELINE_CAPSULE        = markit-r0-rq8-research-record-v1.tar.gz
BASELINE_CAPSULE_SHA256 = 156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849
verified against        = docs/research/horse-a-v1-baseline-experimental-record.md §3
verification date       = 2026-09-29 (capsule SHA256 recomputed on the formal host
                          at /home/jnhu/markit-research-archive/ and matched)
```

Inside the capsule, `SHA256SUMS` lists the SHA-256 of every member file. The
`SHA256SUMS` file in this directory lists the capsule's entries for the copied
artifacts; the copies were verified byte-for-byte against those entries. The
figure pipeline (`generate-all.mjs`) re-verifies every input against
`SHA256SUMS` at generation time and fails closed on any mismatch.

## Files and capsule paths

| File | Capsule path | Role |
|---|---|---|
| `stage-b-facts-v1.json` | `result-analysis/R7-PRIMARY-RESULT-ANALYSIS-v1/stage-b/stage-b-facts-v1.json` | R7 primary T/A/M analysis facts: project-macro EDIT_WRITE speedups (Fig 1, 4) and per-family FAMILY_MACRO aggregates (Fig 3) |
| `r8-values.json` | `synthesis/R8-FINAL-SYNTHESIS-v1/r8-values.json` | R8 sealed aggregate values: EDIT_WRITE median allocation counts / allocated bytes (Fig 4) |
| `regime-map.csv` | `synthesis/R8-FINAL-SYNTHESIS-v1/regime-map.csv` | R8 sealed regime map: size-quartile geomean cuts Q1–Q4 (Fig 2), list/blockquote E3 sub-stratum cuts (Fig 3), bottom-decile cut (Fig 8 context) |
| `rq8-sensitivity-facts-v1.json` | `sensitivity/analysis/rq8-sensitivity-facts-v1.json` | RQ8 sealed sensitivity facts: K1 macro under primary ThinLTO and LTO-off (Fig 6); Q3/Q4 full-precision cross-check (Fig 2) |
| `pmu-cells-v1.json` | `pmu/pmu-v1/analysis/pmu-cells-v1.json` | PMU sealed analysis cells; EDIT_WRITE representative-cell medians reproduce the published PMU-EXPLANATION-v1 table (Fig 5) |

Capsule SHA-256 entries (also mirrored in `SHA256SUMS`):

```text
742e9f3665559515fd2f6914e232ad9f6e0360ab9be3bf401356016e474a3806  stage-b-facts-v1.json
f752745d7274da0b00bae82b6eb673aa4d917b83fde55fdd95fd35b039a74130  r8-values.json
def050a76a82eb011cca6aa8dd27b541dd50f45cb01310ba34cb1892fef98e52  regime-map.csv
c86602e9a0cb9757de0909ce99026ce60c587836b18536675105eda5138b1352  rq8-sensitivity-facts-v1.json
23c7cbbba6a66d67222f8b3d9773f1ef252d4e59bc009773cdcd91e7049d2750  pmu-cells-v1.json
```

## Qualitative input

`qualitative-verdicts.json` is **not** a sealed artifact. It is a faithful,
verbatim-as-possible transcription of the frozen qualitative verdicts and
priority ordering already recorded in the merged baseline record
(`docs/research/horse-a-v1-baseline-experimental-record.md` §13 and §16) and
frozen earlier in `docs/research/horse-a-v1-algorithm.md` §20 (W-A1/W-A2/W-A3
definitions). It contains **no numeric values**: every number rendered in
Figures 7–8 is injected from the machine-readable extraction of the sealed
artifacts above. If §13/§16 ever change, this file must be updated to match
and the figures regenerated.
