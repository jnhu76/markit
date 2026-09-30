# horse-a-v2-step2r-104 — Step 2R screening evidence (issue #104)

Retained raw + derived evidence for the Step 2R V-vs-R screening. All
derived receipts are regenerable from the raw JSONs by the included
scripts. Schemas: `HORSE-A-V2-STEP2R-104-*-v1`. Report:
`docs/research/horse-a-v2-step2r-certificate-repair.md`.

```text
v-only-cal/        5 V-only calibration runs (run1..run5/tlane.json) +
                   v_only_calibration.json (analyze_v_only.py)
pair-campaign/     paired campaign v1..v5 + r1..r5 (tlane.json) +
                   pair_campaign_result.json (analyze_pair.py)
alane-v/ alane-r/  A-LANE counter receipts, V and R binaries
validate-retention/ raw L0.5 validate outputs, both binaries (captured
                   post-campaign for retention; the PASS on both binaries
                   was also frozen in-issue pre-timing)
```

Identity binding (the raw tlane/alane JSONs embed no binary identity —
see #104 Protocol Event receipts):

```text
V_BINARY sha256 = 81f569d8349a52d4efb67c064eada519bb42899bbcb672cd62a03ec24f26ce86
  built from worktree /home/jnhu/Source/markit-v-frozen (detached master
  352e214); binary RETAINED at
  /home/jnhu/Source/markit-v-frozen/research/benchmarks/markdown-ast-update/target/release/mdbench-horse-a-v2-diag
R_BINARY sha256 = 692631acc10247cb39f6f46be96dd25adfea64f9bf550ae6ba41ea29e681bb46
  built from the R branch; binary RETAINED at
  /home/jnhu/Source/markit/research/benchmarks/markdown-ast-update/target/release/mdbench-horse-a-v2-diag
  (built at aa0efca; branch tip 15ce147 differs only in #[cfg(test)] code,
  so the release binary — and this sha256 — is unchanged; #104 PE-2)
BOTH            = re-verified by sha256 at report time; every timing run
                  invoked one of these two paths under `taskset -c 2`
                  (argv recorded inside each tlane.json)
```

Phase timeline (E5 local CST, 2026-09-30): V-only calibration
11:53:09..11:54:43; paired campaign 11:55:30..11:57:53. The protocol and
calibration-number freezes are timestamped in #104 BEFORE the respective
executions; the methodology review verified the artifact mtimes against
the comment trail.

## Amendment v3 corrective rerun (merge-authoritative)

The independent review found P1: the fast matcher did not preserve V's
frozen duplicate behavior on non-monotone defensive barrier slices.
Amendment v3 (#104, append-only receipt) added an allocation-free O(B)
strict-order eligibility gate: monotone slices take the unchanged fast
matcher; non-monotone slices route to `v1_defensive_fallback` — the
frozen 352e214 V matcher copied verbatim. Because production code
changed, the frozen screening was rerun as a protocol-preserving
corrective rerun (protocol, thresholds, repetitions, order, host
unchanged; thresholds NOT recalculated; V binary re-verified unchanged).

```text
corrective/             validate-retention/ (L0.5 PASS both binaries, all
                        13 cells) + alane-v/ alane-r/ (A-LANE receipts on
                        the corrected binary; V/R byte-identical on every
                        field)
corrective-campaign/    v1..v5 + r1..r5 tlane.json (interleaved V,R x 5,
                        2026-09-30T14:44:42..14:47:04+08:00, taskset -c 2)
                        + pair_campaign_result.json (analyze_pair.py
                        unchanged, same frozen thresholds)
R_BINARY_CORRECTIVE     sha256 25c13f70c840e2e28e3c51daacd09aa4375191687864
                        c3201b759c4dc1e2731f, built at corrective head
                        2ea09b7; RETAINED at the R target/release path
                        (pre-review binary 692631ac... also retained)
V_BINARY                sha256 81f569d8... (UNCHANGED, re-verified)
VERDICT                 challenges E6-1 -53.49% / E6-4 -62.48% /
                        E6-5 -59.05% all MATERIAL faster; no guardrail
                        regression (worst +1.17%); E6-6 crossed its
                        threshold FASTER (-3.70%) — recorded as report
                        protocol event PE-4. This screening is the
                        merge-authoritative Step 2R result; pair-campaign/
                        is preserved as the pre-review screening.
```
