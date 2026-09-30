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
