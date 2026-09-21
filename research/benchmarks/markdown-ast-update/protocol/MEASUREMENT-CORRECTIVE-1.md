# R5-MEASUREMENT-CORRECTIVE-1 — Measurement substrate repair

Status: **IMPLEMENTED / VERIFIED / PRIMARY TIMING STILL NOT RUN**

Authority chain:

```text
#22 MARKIT-MARKDOWN-BENCHMARK-1
  -> protocol/R0-METHODOLOGY.md (baseline methodology)
  -> protocol/R5-HORSE-CORRECTNESS-PARITY.md (horse correctness/parity)
  -> protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md (workload authority)
  -> THIS corrective (measurement substrate only)
```

This corrective repairs the **measurement substrate** of the frozen benchmark:
the timer boundary, the attribution schema, the counter authority, and the
correctness-only qualification surface. It deliberately does **not** change:

```text
workload identity (cases, payloads, CaseId space, corpora, manifests)
BENCH-GRAMMAR-v1 semantics
NORMALIZED-RESULT-v1 result vocabulary
H0-H4 mechanism semantics (WHAT they compute)
sampling policy, conclusion ladder, optimization-sensitivity rule
```

Explicit base: because the PR #40 correctness work (reference-environment
soundness checks in H2/H3 re-materialization and H4 assembled-table
comparison) was still an unmerged draft when this corrective started, this
corrective is built on the **accepted PR #40 head `5bc2752`** as its explicit
base commit, not on `master`.

---

## 1. BLOCKER A — the timer boundary (§6-§11)

### 1.1 The defect

The pre-corrective runner derived the normalized result checksum from the
mechanism's Pending inside the timed region, and H0 alone ran
`validate_normalized` inside its timed parse path. Two asymmetric defects:

```text
- every horse was charged export/verification work (normalize + checksum)
  inside T_native, so "native update cost" was polluted by oracle work;
- H0 was charged MORE than H1-H4 (it additionally validated), so the H0
  control was not comparable.
```

### 1.2 The corrected semantics (frozen)

> **Work required for the horse's usable native state stays inside timing.
> Work required only to prove equality to the experiment's normalized
> oracle stays outside timing.**

Concretely:

```text
T_prepare  edit-coordinate preparation
T_native   update + native-sealing complete() + black_box(state)
post-timer NormalizeV1 projection, validation, result checksum,
           oracle comparison — never timed
```

### 1.3 The API change

- `Mechanism::complete()` is now **native-sealing ONLY**: it turns the
  `Pending` into `Completed { state }`. It must not normalize, validate,
  checksum, or otherwise serve the oracle.
- The experiment's result checksum is a **post-timer export**: every horse
  `State` implements the new `common::ResultChecksum` trait
  (`fn result_checksum(&self) -> u64`), and the runner's `finish_*`
  functions call it strictly after every timer has stopped.
- H0's `parse_with_attribution` (measurement path) no longer calls
  `validate_normalized`. The reference-authority path (`parse_document`)
  keeps the frozen NORMALIZED-RESULT-v1 conformance gate — the gate lives
  with the REFERENCE, not inside the measured mechanism.

### 1.4 Normalized-projection audit (A = intrinsic vs B = export)

Every horse's normalized projection was audited against the corrected
boundary:

```text
H0  B (export)  retained Document is the native state; checksum = its
                normalized checksum, derived post-timer
H1  B (export)  retained tiling carries per-block semantic subtrees;
                NormalizeV1 is a pure traversal + assembly (R5-CORRECTIVE-1
                MAJOR-3 design, now honored at the boundary)
H2  B (export)  fragment store assembles the document by pure traversal
H3  B (export)  retained old tree + patch table assemble by pure traversal
H4  B (export)  block forest + convergence state assemble by pure traversal
```

No horse needs the normalized projection as retained mechanism state; all
five are B-class. The `Pending.result()`-style in-timer export R5 §4/§11.5
described is **superseded**; R5 §11.6's counting rule (native node = 1
structural block unit + retained inline nodes) is unchanged.

### 1.5 Regression tests

- `r1_acceptance`: a `ClockProbe` mechanism records the clock value observed
  in each phase; `complete_read == update_read` proves `complete()` performs
  no observable phase work, and a `ClockAdvancingHook` proves oracle
  execution happens outside every timer.
- Every mechanism test that previously asserted in-timer checksums now
  asserts the post-timer export (`ResultChecksum`), keeping the R5 §11.5
  invariant `checksum(normalize(state)) == state.result_checksum()` with the
  derivation moved outside timing.

---

## 2. BLOCKER B — FULL_READ parity (§12-§13)

FULL_READ is renamed in all reporting to **clean parse + native-state
construction** and is extended from H0-only to **all five horses**:

```text
22 G0-strict files x 5 horses = 110/110 clean-state constructions PASS
   (correctness-only; no timing, no counters-as-results)
```

`mdbench-corrective-c dry-run` dispatches every G0-strict file through H0-H4
via the runner's correctness-only path, each against the H0 reference
(`parse_document`, which keeps the normalized conformance gate). Verified
this corrective: `full_read horse_dispatches=110 pass=110 failed=0`,
`edit_write horse_dispatches=1810 pass=1810 failed=0`.

---

## 3. Counter authority (§14)

Attribution counters report **cumulative ACTUAL work that occurred**,
including work later discarded by a fallback or a restart-at-zero. Discarded
work is never subtracted; one scan of one record is one charge. A fallback
or restart therefore never *reduces* a counter — the delivered result's
counters accumulate on top of the discarded attempt's.

---

## 4. H1 counter repairs (§15)

- **H1-A (duplicate metadata charge)**: the damage scan over the retained
  tiling is charged EXACTLY ONCE, at the scan site in `update`. The F1/F6
  total fallback no longer re-charges the same scan.
- **H1-B (discarded regional work)**: when the region reparse runs and a
  fallback then fires, the discarded region's block units REMAIN in
  `blocks_reparsed` AND `nodes_rebuilt` (skeleton-only: the discarded
  attempt did not reach inline materialization). The full parse's counts
  accumulate on top.

## 5. H4 counter repairs (§16)

- **H4-A (retained prefix reuse)**: the retained prefix's block nodes are
  counted in `nodes_reused` (they ARE reused state), delivered as
  `add_nodes_reused(prefix_reused + reused)`.
- **H4-B (discarded forward pass)**: when the assembled-table comparison
  rejects the forward pass, the discarded pass's block units stay in
  `blocks_reparsed`/`nodes_rebuilt` (plus the checkpoint-table
  consultations in `metadata_records_touched`), and the restart-at-zero
  work accumulates on top.

## 6. H2/H3 audit (§17)

Both horses were audited for the H1-A/H1-B defect class; fixes were applied
only where evidenced:

- **H2 (fragment-reuse)**: each fragment-slot consultation is charged once;
  the covering-fragment mapping charge and the total-parse charges are
  separate events; no duplicate charging found. §18 version stamping was the
  only evidenced repair (prev-margin reads stamped `Old`).
- **H3 (old-tree-subtree-reuse)**: the patch-margin scans are charged once
  at the patch site (`scanned + patched`); H3 has no fallback lane. §18
  version stamping was the only evidenced repair (prev-margin `Old`,
  next-margin `Post`).

---

## 7. Source-inspection coordinates (§18) and unique vs cumulative (§19-§20)

- Every `record_source_inspection` event now carries a `SourceVersion`:
  `Old` (a consultation of the retained OLD source, e.g. H3's prev-margin)
  or `Post` (a scan of the source being parsed). The two versions are
  separate coordinate spaces after any length-changing edit and are never
  unioned into each other.
- `WorkCounters` v2 adds: per-version unions
  (`unique_old_/unique_post_source_intervals/bytes`), the combined
  `unique_source_intervals/bytes` (SUM of the per-version unions, never a
  cross-version merge), and cumulative `source_bytes_inspected_total`
  (every event counts, duplicates included).
- **Parse Amplification (PA)** is defined explicitly: the combined primary
  quantity is the sum of the per-version unique unions, divided by the post
  source bytes; PA is never computed from cumulative effort and never
  merges the Old and Post coordinate spaces.
- The result schema is versioned: `RESULT_SCHEMA_VERSION = 2`
  (`protocol/result-schema-v2.json`; v1 removed). Workload CaseId/payload
  identity is untouched.

## 8. Exact counter regression fixtures (§21)

`attribution_exact_fixtures` (H1 and H4) pin EXACT derived counter values
with the derivation in the test comments — e.g. H1's F1 fallback charges
`metadata_records_touched = 6 (damage scan, once) + 6 (new tiling)`; the F6
guard path charges `blocks_reparsed = 1 (discarded region) + 2 (delivered
full parse)`; H4's discarded-forward-pass restart charges
`blocks_reparsed = 3, nodes_rebuilt = 4, nodes_reused = 0`. "Counter != 0"
is not the gate; the frozen rules derive the number.

---

## 9. Metric qualification (§22)

What the repaired substrate can honestly support, and what it cannot. Any
conclusion that needs an UNAVAILABLE family is not writable in this
environment; the missing instrumentor is future work, not a silent zero.

| Metric family | Status | Grounds |
|---|---|---|
| LATENCY (T_prepare / T_native / wall-clock) | **QUALIFIED** | T-LANE instrumented; timer contract regression-proven (phase clocks cannot leak; oracle work is post-timer). Primary timing not yet RUN. |
| WORK_COUNTERS (blocks/nodes/metadata/reuse/inspection) | **QUALIFIED** | A-LANE v2 schema; per-version attribution; exact fixtures H0/H1/H4 + H2/H3 audits; cumulative authority frozen. |
| Parse Amplification | **QUALIFIED** (as a WORK_COUNTERS derived ratio) | Defined from per-version unique unions (§7); never from cumulative effort. |
| CPU_TIME | **UNAVAILABLE** | No CPU-time instrumentor exists in the substrate. |
| ALLOC_COUNT / ALLOC_BYTES | **UNAVAILABLE** | M-LANE contract (`MemoryRecord`) exists; no allocator backend is wired (fields carry `Observed::Unknown`). |
| PEAK_BYTES | **UNAVAILABLE** | Same as ALLOC_*. |
| RETAINED_BYTES | **UNAVAILABLE** | Same as ALLOC_*. |

## 10. Strict-file profile export (§23)

`mdbench-corrective-c profile-export` writes
`workloads/profiles/strict-surface-profile-v1.jsonl`: one row per
G0-strict file (the frozen manifest is the authority — the strict surface is
NEVER reselected), carrying mechanism-neutral G0 `StructuralFacts`
(file bytes, block count, largest block, max container depth, fence
density, code occupancy, reference density, with their counting-rule
echoes). No timing, no counters, no horse fields.

---

## 11. Discipline constraints (§25-§27)

- **No formal timing**: this corrective runs correctness-only surfaces. No
  p50/p95, no throughput, no ranking, no "X is faster" statement may be
  produced from it. Primary timing begins only under the R6-R11 gates.
- **Horse fidelity**: no mechanism was optimized, tuned, or restructured
  beyond what the corrective's own repairs required; a horse whose
  measurement changed (H1, H4) changed its COUNTERS, not its algorithm.

## 12. Workload identity receipt (§28)

Verified after all changes, against the pre-change baseline:

```text
workloads/sources tree        IDENTICAL (sha256 d38fd1ac...)
transition-registry-v1.json   IDENTICAL
applicability-matrix-v1.jsonl IDENTICAL
full-read-manifest-v1.jsonl   IDENTICAL
edit-write-manifest-v1.jsonl  IDENTICAL
trace-manifest-v1.jsonl       IDENTICAL
coverage-final-v1.json        IDENTICAL
freeze-receipt-v1.json        IDENTICAL
dry-run-cases-v1.jsonl        REGENERATED (derived; reflects 110 FULL_READ rows)
dry-run-report-v1.json        REGENERATED (derived; same reason)

WORKLOAD_IDENTITY_CHANGED: NO
```

The only workload-tree changes are the two derived dry-run artifacts, which
are regenerated outputs of the correctness-only dry-run, not workload
definition.
