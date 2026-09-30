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

## Amendment v3 corrective reruns (round 1 = history; FINAL = merge-authoritative)

The independent review found P1: the fast matcher did not preserve V's
frozen duplicate behavior on non-monotone defensive barrier slices.
Amendment v3 (#104, append-only receipt) added an allocation-free
strict-order eligibility gate: eligible slices take the unchanged fast
matcher; violators route to `v1_defensive_fallback` — the frozen 352e214
V matcher copied verbatim. Round-1 review E then found (and demonstrated
empirically) a P0 on the barriers-only gate — non-monotone CUTS with
gate-eligible barriers took the fast path where V's filter loop is
defined — so the gate was extended to BOTH orderings (head a9270ba).
Because production code changed each time, the frozen screening was
rerun as a protocol-preserving corrective rerun after each fix
(protocol, thresholds, repetitions, order, host unchanged; thresholds
NOT recalculated; V binary re-verified before every campaign).

```text
corrective/             round-1 (binary 25c13f70): validate-retention/ +
                        alane-v/ alane-r/ (V/R byte-identical)
corrective-campaign/    round-1 screening (14:44:42..14:47:04+08:00) —
                        HISTORY, superseded by the FINAL screening
final/                  FINAL (binary 734cccba, head a9270ba):
                        validate-retention/ (L0.5 PASS both binaries,
                        13/13 cells) + alane-v/ alane-r/ (byte-identical
                        V/R on every field)
final-campaign/         FINAL screening (15:49:54..15:52:17+08:00),
                        interleaved V,R x 5, taskset -c 2 +
                        pair_campaign_result.json (analyze_pair.py
                        unchanged, same frozen thresholds) —
                        MERGE-AUTHORITATIVE
final-campaign-quarantine-PE7/   one aborted 15:46 campaign run against
                        an accidentally rebuilt V arm (see PE-7); kept
                        for the record, used for NOTHING
R_BINARY_FINAL          sha256 734cccba37717074e67e99c507a9939087e62e08327
                        45d9c338e851297c450fe, built from the final
                        production-code tree (head a9270ba; later
                        commits touch no .rs file); RETAINED at the R
                        target/release path AND at
                        /home/jnhu/retained-bench-binaries/
                        R-final-a9270ba-734cccba
V_BINARY                sha256 81f569d8... (UNCHANGED through every
                        campaign; after the PE-7 overwrite incident the
                        original bytes were recovered from the cargo dep
                        artifact, restored, and dual-retained at
                        /home/jnhu/retained-bench-binaries/
                        V-frozen-352e214-81f569d8)
EARLIER R BINARIES      692631ac (pre-review) and 25c13f70 (round 1) are
                        NO LONGER ON HOST — single-copy retention inside
                        a cargo target dir was overwritten by later
                        rebuilds (PE-7's lesson; review F round-1 P1).
                        Their screenings are retained as history; the
                        FINAL screening is the merge-authoritative
                        result.
VERDICT (FINAL)         challenges E6-1 -53.86% / E6-4 -62.35% /
                        E6-5 -59.02% all MATERIAL faster; no guardrail
                        regression (worst +3.52% vs 31.44% threshold);
                        E6-6 crossed its threshold FASTER (-3.29%) —
                        report protocol event PE-4.
```
