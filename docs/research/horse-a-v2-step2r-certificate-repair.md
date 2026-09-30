# Horse-A v2 Step 2R — same-authority certificate-matching repair R

> Scope: issue [#104](https://github.com/jnhu76/markit/issues/104) (Step 2R
> contract, protocol freeze, calibration, screening receipt, decision) —
> the one Step-2R issue under the #95 umbrella. The L2 evidence this repair
> answers lives in
> [horse-a-v2-l2-context-profile-100.md](horse-a-v2-l2-context-profile-100.md)
> plus its append-only
> [erratum](horse-a-v2-l2-evidence-erratum-2026-09-30.md).
> Status: **CONTRACT AMENDMENT v3 APPLIED (independent-review P1 fix:
> defensive non-monotone barrier slices now route to a verbatim frozen-V
> fallback); corrective rerun complete; the R PR is open — merged only
> after the fresh post-fix reviews; #95 is not closed; no S mechanism
> exists.**

```text
STEP_1_DIAGNOSIS   = COMPLETE (#97/#98/#100)
L2_ERRATUM         = MERGED (PR #103)
STEP_2R            = AMENDED_v3_CORRECTIVE_RERUN_COMPLETE_FOR_REVIEW
L3/L4/L5           = NOT EXECUTED
HISTORICAL_V       = IMMUTABLE (master 352e214; nothing rewritten)
SEMANTIC_CHALLENGER_FROZEN = NO (S1 question EARNED and RECORDED only)
NEXT               = FRESH_POST_FIX_REVIEW_THEN_MERGE_GATE
```

## A. Authority

```text
LIVE_MASTER_AT_START = 352e214fc43ff22a89c835c5407a107d83cbe33e
                       (task-creation authority re-verified live; local
                       checkout fast-forwarded fb359de -> 352e214; no drift;
                       RE-VERIFIED at amendment-v3 time: origin/master still
                       352e214, PR head still 9854f7c, no commits added)
ISSUE                = #104 (created for Step 2R; body + comments carry the
                       full contract/protocol/calibration/result trail plus
                       the append-only CONTRACT AMENDMENT v3 receipt)
BRANCH               = research/95-step2r-certificate-match-r1
R_HEADS              = pre-review screening head 9854f7c (campaign binary
                       692631ac...); corrective amendment-v3 head 2ea09b7
                       (campaign binary 25c13f70...). Research identity is
                       logically ONE mechanism, HORSE-A-V2-STEP2R-CERT-MATCH-
                       R1, across both heads.
V_IDENTITY           = frozen Horse-A v1 = master 352e214
                       certificate.rs blob 564b84dc8ae0374e8c3508b560ef5f0
                       eb42ca626 (sha256 259f776bf9e44623e4d2d3358569871a
                       811150a2a1d3726b8327e6830ac00dbc)
V_BINARY             = built from worktree /home/jnhu/Source/markit-v-frozen
                       (detached 352e214); sha256 81f569d8349a52d4efb67c06
                       4eada519bb42899bbcb672cd62a03ec24f26ce86; RETAINED
                       on host (path above), re-verified at both campaign
                       times — UNCHANGED by amendment v3
R_IDENTITY           = HORSE-A-V2-STEP2R-CERT-MATCH-R1
                       (CERTIFICATE_MATCH_REPAIR_ID, certificate.rs only;
                       never enters ReadyDocument/InterpretationId)
R_BINARY_PRE_REVIEW  = sha256 692631acc10247cb39f6f46be96dd25adfea64f9bf55
                       0ae6ba41ea29e681bb46 (head 9854f7c; RETAINED)
R_BINARY_CORRECTIVE  = sha256 25c13f70c840e2e28e3c51daacd09aa4375191687864
                       c3201b759c4dc1e2731f (head 2ea09b7; RETAINED) — the
                       merge-authoritative Step 2R binary
HOST                 = E5 (jnhu@192.168.31.75; Xeon E5-2666 v3, 20 cores),
                       Linux 7.2.5-200.fc44.x86_64, schedutil untouched
TOOLCHAIN            = workspace pin 1.97.1 (rust-toolchain.toml; verified
                       in-workspace: rustc 1.97.1 8bab26f4)
PROFILE              = frozen [profile.release] (opt-level 3, lto=thin,
                       codegen-units 1, incremental=false, panic=unwind)
DIFF SCOPE           = production-code diff vs 352e214 = 4 files, all under
                       mechanisms/horse-a/src/ (certificate.rs, lib.rs,
                       structural.rs, step2r_tests.rs). Every harness and
                       measurement crate byte-identical across V/R. The
                       complete PR additionally carries reports and the
                       retained evidence tree.
```

Governance classification (frozen by #95, unchanged):

```text
CERTIFICATE_FINDING       = REQUIRED_RESPONSIBILITY_WITH_BAD_IMPLEMENTATION
GOVERNANCE_CLASSIFICATION = MECHANISM_SHRINK
FROZEN_V1_IN_PLACE_REPAIR = NOT_ALLOWED (V untouched; R is a distinct identity)
```

## B. Derived contract

Derived from code at 352e214 BEFORE any algorithm choice, then independently
reviewed (pre-implementation reviews A: semantic/correctness, P0=1 P1=2
P2=4; B: minimal-mechanism, P0=0 P1=1 P2=2 — all findings resolved in the
frozen contract v2 on #104 before implementation).

```text
R-C0  MATCH_PREDICATE = exact `ev.cut == cuts[i+1]` equality (A-P0-1:
      never a predecessor/range lookup; this is what makes the persisted
      support end exactly at the coverage end, validate.rs
      `blank_line.end == coverage_len`).

R-C1  CUTS_MONOTONE = YES (strictly increasing).
      Authority: coverage.rs `build_with_base` hard-errors unless the
      TopLevelStart physical starts are strictly increasing in
      [base, region_end); cuts = [base] ++ starts[1..] ++ [region_end];
      both seam callers construct the plan immediately before the call.

R-C2  BARRIERS_MONOTONE_BY_CUT = YES for every legal execution (strictly
      increasing). Authority chain: shared-grammar parser.rs `run()` is a
      single forward `while pos < end` line loop (pos strictly increases;
      each physical line dispatched exactly once); `observe_root_blank_
      barrier` has exactly one call site (root B1), at most one event per
      dispatched line, cut = line_lf + 1 with one LF per line; both
      collectors fill their Vec append-only from exactly ONE
      parse_region_observed call per persist (full_build.rs; update.rs
      ForwardObserver — the convergence barrier is pushed before the Stop).
      Caller-side invariant, documented at the seam, not re-checked
      (R-C9).

R-C3  DUPLICATE_CUT_SEMANTICS = same hard error, same scope, same order.
      Duplicates are unreachable from a legal parse (R-C2) — defensive
      only — but are frozen seam behavior: hard error
      "multiple root blank barriers certify the same boundary {boundary}",
      scoped to EXAMINED interior boundaries only (duplicates at
      unexamined cuts stay silently ignored), checked BEFORE any support
      check, lowest examined boundary errors first (R-C3b).
      AMENDMENT v3: V detects duplicates by a whole-slice search per
      examined boundary. The fast traversal's adjacent peek is provably
      inert under the eligibility gate (see R-C2a) but cannot see
      non-adjacent duplicates on NON-monotone defensive slices; such
      slices are therefore routed to the verbatim V fallback, restoring
      V's exact whole-slice duplicate semantics on every input.

R-C2a (amendment v3) BARRIER_ORDER_ELIGIBILITY_GATE = the fast traversal
      runs only after an allocation-free O(B) scan confirms
      `barriers.windows(2).all(|w| w[0].cut < w[1].cut)` (strictly
      increasing; vacuously true for B <= 1; B - 1 comparisons). Legal
      production always passes (R-C2). A duplicate (equal cut) or a
      decreasing cut fails the gate and takes the frozen-V fallback
      (`v1_defensive_fallback`, the 352e214 matcher loop copied
      verbatim, including its forbidden-sentinel tail and early-Err
      shape). NO sorting, deduplication, assertion, panic, new retained
      state, heap allocation, or new error class. Totality (R-C9) is
      strengthened: violative input now has V's exact defined behavior
      instead of "undefined matching behavior".

R-C4  UNMATCHED_EVENT_SEMANTICS = three silent skip classes (corrected
      from four in review): (1) cut matches no examined boundary (mid-gap,
      leading trivia); (2) line_start - base underflows or is 0 (blank not
      strictly inside left Owner coverage — includes every BOF-shaped
      event); (3) preceding_lf < base underflow. `preceding_lf = None` is
      NOT a skip class: it is preserved/persisted by the shared support
      code, is unreachable for production-shaped events, and is preserved
      as-is (neither fixed nor widened). No new error class anywhere.

R-C5  SUPPORT_PERSISTENCE = byte-for-byte equality-equivalent:
      rel_blank_start = line_start - base (>= 1 enforced),
      rel_preceding = preceding_lf.map(-base) with None preserved,
      rel_blank_end = cut - base (V's arithmetic verbatim, including the
      pre-existing unchecked subtraction), adjacency invariant untouched,
      exactly one certificate_write per installation, retained-suffix
      certificates never rewritten (UnaffectedCertificateWrites = 0).

R-C6  FULL_BUILD_FINAL_BOUNDARY = document EOF never certified (last
     _boundary_is_interior = false).
      LOCAL_CONVERGENCE_FINAL_BOUNDARY = the convergence barrier is a real
      interior boundary of the new document and IS certified on the last
      fresh Owner (end < src.len()). Shared seam, flag distinguishes.

R-C7  Relied-upon parser-side invariants, documented not verified (as in
      V): preceding_lf == line_start - 1 (or None at line_start == 0);
      cut == line_lf + 1 > line_start.

R-C8  The seam runs before OwnerSeq::bulk_build at both callers (aggregates
      derive from installed certificates at bulk-build time).

R-C9  TOTALITY: no new panic modes; NO sortedness assertion in release or
      debug (a strict-order assert would make the duplicate-cut seam
      behavior untestable and would panic where V does not). AMENDMENT
      v3: behavior on violative (non-monotone) input is DEFINED — it is
      exactly the frozen V fallback semantics — and remains unreachable
      from both proven callers.

CONTRACT_REVIEW = Subagent A (semantic): P0=1 P1=2 P2=4 P3=4 — all
      resolved (R-C0 pinned; duplicate scope pinned; R-C4 corrected to
      three classes; error precedence/error-text/parser invariants/seam
      ordering/totality pinned).
      Subagent B (minimal-mechanism): P0=0 P1=1 P2=2 P3=4 — all resolved
      (counter moved OUTSIDE the frozen counters-v1 record; identity
      constraints pinned; V-work stated as exact N_b x B, R as a bound).
```

## C. Repair mechanism

Exactly one function's matcher changed.

```text
MECHANISM_CHANGE = persist_interior_certificates:
  V:  for every examined boundary, a fresh lazy filter scan over ALL
      barriers (first match + whole-remainder second-match search) —
      exactly N_b x B barrier-cut inspections per call.
  R:  an eligibility gate first (R-C2a): one allocation-free O(B) scan
      confirms the barriers are strictly increasing by cut. Eligible
      (every legal production call): one function-local usize cursor
      advanced monotonically over the barriers as the examined
      boundaries increase; exact-equality match at the cursor; the
      adjacent-element duplicate peek retained as a defended site
      (provably inert under the gate); then V's support checks,
      installation, charges and error paths VERBATIM. The matched
      barrier is consumed (cursor += 1); exhaustion breaks (nothing
      later can match).
      NOT eligible (non-monotone, unreachable from legal parsing): the
      frozen V matcher verbatim (`v1_defensive_fallback`), restoring
      V's exact seam semantics on defensive input (AMENDMENT v3).
JUSTIFICATION = R-C1 + R-C2 + R-C2a (both sequences strictly increasing
      at every legal call site; all four call sites enumerated:
      full_build.rs, fresh.rs, two single-element test callers; the gate
      re-verifies the barrier ordering so the traversal's assumptions
      are checked, not presumed).

NEW_RETAINED_STATE           = 0 (no field added to any persisted type;
                                state.rs byte-identical)
NEW_TEMPORARY_STATE          = a function-local cursor + batch tally + one
                                gate bool — no heap allocation anywhere in
                                the matcher or the gate (windows(2) and the
                                V fallback's iterator filter allocate
                                nothing); the only allocations are the
                                pre-existing error format! Strings V also
                                allocates
NEW_PERSISTENT_INDEX         = none
CERTIFICATE_AUTHORITY_CHANGED= NO
CERTIFICATE_SET_CHANGED      = NO
READY_SEMANTICS_CHANGED      = NO
NEXT_EDIT_CONTRACT_CHANGED   = NO
PARSER_EVIDENCE_CHANGED      = NO (A-LANE counters byte-identical V/R,
                                re-verified on the corrected binary)

Additive diagnostic seam (outside the mechanism's semantics):
  HorseAStructuralSink::certificate_barrier_inspections(n) — default
  no-op body; overridden only by RecordingHorseAStructuralSink, which
  tallies a SIDE FIELD (its own accessor) outside the frozen
  HORSE-A-STRUCTURAL-COUNTERS-v1 record. Producer/adjudicator machinery
  byte-identical; T-LANE (no-op lane) pays only the virtual calls.
  Charged once per examined boundary (seek batch) + once per duplicate
  peek — see Protocol Event PE-1.
```

## D. Correctness receipt

Every gate below was green before any timing was collected.

```text
SEAM DIFFERENTIAL (V-replica vs R, src/step2r_tests.rs)
  V-replica          = verbatim 352e214 traversal (one predicate
                       evaluation at a time), audited faithful by review C
  battery            = 15 seam cases x (Result + exact message + full
                       Owner vector + certificate_write parity): ALL
                       identical
  covers             = no-barriers; single certificate; all-certified;
                       mid-gap events; leading trivia; EOF cut;
                       BOF-shaped skip; blank-at-base skip; before-base
                       skip; duplicate at first examined boundary;
                       duplicate at non-first boundary (exact first-error
                       message); duplicates at unexamined cut silently
                       ignored; duplicate where the first candidate would
                       fail support (error still fires); local-convergence
                       last boundary certified; single-owner (both flags);
                       empty owners; cuts-length precondition error +
                       sentinel-not-charged
  DEFENSIVE BATTERY (amendment v3, D1–D7): non-adjacent duplicate
                       (exact V error text, fallback route charges 0
                       fast-matcher inspections); duplicate after a larger
                       out-of-order event at a later boundary; unsorted
                       without duplicate (full Result/Owner/writes parity
                       plus the same-shape monotone twin proving both
                       routes converge on the same state); unsorted
                       duplicates at an unexamined cut stay silent; lowest
                       examined duplicate boundary errors first under
                       unsorted input; duplicate precedence over a
                       support-failing first candidate under unsorted
                       geometry; support-rejected exact match skips
                       identically on both routes
NEXT_EDIT GATE
  chains             = edit1 -> READY1 -> edit2 -> READY2, every stage
                       held to the clean from-scratch full-build
                       authority as FULL LOGICAL STATE (in-order Owner
                       sequence incl. persisted certificates + RefTable +
                       ids + interpretation; AVL node shape excluded as
                       derived structure, its invariants gated by
                       validate_ready): certificate-adjacent edit1
                       (support-touching blank-line deletion),
                       support-untouched edit1, CJK/emoji chain — all
                       equal
RETAINED SUFFIX     = local update leaves ranks beyond the guard
                       certificate-identical; forbidden sentinel
                       Known(0); state equals clean authority
WORK ACCOUNTING     = hand-traced exact: small case V=10, R=6; dense case
                       V=2480, R=109 (= 93 seek iterations + 16 peeks,
                       exact-asserted; fast-match bound B + N_b = 111,
                       rejected-match term R = 0 on these shapes)
CORRECTIVE RERUN (amendment v3, corrected binary 25c13f70):
  unit + workspace   = horse-a crate 256 tests green (incl. the D1-D7
                       battery); full workspace green
  L0.5 validate      = PASS on BOTH binaries, all 13 cells (raw outputs
                       under results/horse-a-v2-step2r-104/corrective/
                       validate-retention/)
  A-LANE PARITY      = alane.json V vs R on the corrected binary:
                       byte-identical counters on all 13 cells for every
                       field, incl. certificate_writes and
                       forbidden_unaffected_certificate_writes = 0
PRE-REVIEW SUITES (head 9854f7c, binary 692631ac — retained for the
  record)            = horse-a crate 249 tests green; full workspace
                       green except the sealed campaign crate, whose
                       failing set on R is a STRICT SUBSET of the failing
                       set on frozen V (pre-existing,
                       environment-dependent, not R-caused); L0.5 PASS on
                       both binaries (retained under validate-retention/)
```

## E. Work-reduction receipt (restated by amendment v3)

The v2 statement `R_MATCHING_WORK <= B + N_b` was re-derived against the
actual code instead of being assumed (amendment-v3 receipt on #104). The
corner case is real: an exact-cut match whose support is not persistable
stays at the cursor and is re-inspected once at the next examined
boundary, so the bound needs an explicit rejected-match term.

```text
V_MATCHING_WORK   = exactly N_b x B barrier-cut predicate evaluations per
                    persist call, absent the duplicate abort — proven
                    structurally (the lazy filter is exhausted for every
                    examined boundary: first-match scan k+1 evals +
                    remainder scan B-k-1 evals = B; no-match scan = B).
                    V charges no counter; this number is derived, not
                    measured.
ORDER_CHECK_WORK  = max(B - 1, 0) cut comparisons for the amendment-v3
                    eligibility gate; allocation-free; NOT charged to the
                    diagnostic counter (fixed scope, below).
FAST_MATCH_INSPECTIONS (the diagnostic tally: seek iterations + duplicate
                    peeks) <= B + N_b + R, where R = support-rejected
                    exact matches (each contributes one later seek
                    inspection and one peek beyond the B + N_b core).
                    Derivation: cursor movement is exact — every barrier
                    is advanced past at most once (A = B - U - I, U =
                    never reached, I = installed), every examined
                    boundary breaks at most once (T <= N_b), every match
                    peeks at most once (P <= M = I + R), so
                    A + T + P <= B + N_b + R - U.
                    Measured by the tally: hand-traced exact 6 (vs V 10);
                    dense-case exact 109 (vs V 2480, a 22.8x reduction;
                    bound 111 with R = U = 0 on those shapes). R = 0 on
                    every measured evidence path.
TOTAL_LEGAL_PATH  = ORDER_CHECK + FAST_MATCH <= 2B + N_b + R - 1 —
                    linear in B + N_b. On the dense E6 geometry
                    (~2050 barriers x ~2050 examined boundaries):
                    V performed ~4.2M matching inspections per full
                    build; R performs ~4.1k fast-match inspections plus
                    ~2k order-check comparisons.
DEFENSIVE_FALLBACK (non-monotone input only, unreachable from legal
                    production generation) = exactly V's N_b x B
                    predicate evaluations — the frozen algorithm's own
                    cost, by construction. No fast-matcher inspections
                    are charged on this route.
ASYMPTOTIC_CLAIM  = Theta(N_b x B) -> O(B + N_b) on the legal production
                    path. The scientific claim is linearity in barriers +
                    examined boundaries with identical authority, not a
                    cosmetically tight formula.
DIAGNOSTIC_COUNTER_SCOPE (fixed) = certificate_barrier_inspections counts
                    FAST-MATCH seek + peek inspections ONLY. Order-check
                    comparisons are not included; the V fallback charges
                    nothing (V has no such diagnostic); total work is
                    accounted analytically above. Recording lane only;
                    outside the frozen record. Evidence order per
                    contract: structural proof leading, counter
                    confirmation only.
```

## F. Frozen screening protocol

Frozen in #104 BEFORE any timing existed (comment timestamps verified by
the methodology review against artifact mtimes):

```text
PROTOCOL_FROZEN_BEFORE_R_TIMING = YES
  03:07:17Z contract v2 -> 03:52:59Z protocol freeze -> 03:53:28Z first
  timing artifact (V-only) -> 03:54:54Z calibration end -> 03:55:19Z
  calibration numbers frozen -> 03:55:48Z campaign start -> 03:57:58Z
  campaign end -> 04:00:35Z result posted. No r*.json existed before the
  freeze; the analysis script predates the first R run.
V_ONLY_NOISE_CALIBRATION = 5 independent V processes (taskset -c 2,
  2026-09-30T11:53:09..11:54:43+08:00); R numbers unread during
  calibration (no R run existed).
MATERIALITY_RULE = per cell THRESHOLD = max(2 x V-only spread, 2%),
  frozen before the campaign: E6-1 2.53%, E6-4 2.00%, E6-5 2.00%,
  E6-6 2.50%, TINY-64B 17.75%, TINY-1K 31.44%, SENT-E5 5.98%,
  SENT-LIST 3.78%, SENT-BQ 13.51%.
REPETITIONS      = paired campaign 5 V + 5 R independent processes,
  interleaved V,R x 5, 30 warmup + 200 measured rounds per cell x arm
  per run, per-run medians, decision statistic = median of 5.
RUN_ORDER        = V,R interleaved; frozen manifest cell order; same
  core (taskset -c 2) as #100; no perf sampling.
DECLARED ASYMMETRY = R's no-op lane pays extra batched virtual sink
  calls per persist call (see PE-1 for the corrected count); V pays
  none. Orders of magnitude below the measured windows.
EVIDENCE         = results/horse-a-v2-step2r-104/ (raw tlane JSONs,
                   alane JSONs, validate retention, analysis scripts +
                   derived receipts, all regenerable; schemas HORSE-A-V2-
                   STEP2R-104-*-v1).
```

### Corrective rerun (amendment v3 — protocol-preserving)

The amendment-v3 fix changed production code (the eligibility gate), so
the pre-review timing was NOT reused as final. The frozen screening
protocol was rerun unchanged; nothing was recalculated after seeing R
numbers.

```text
CORRECTIVE_RERUN_REASON          = independent-review P1 same-authority fix
SCREENING_PROTOCOL_CHANGED       = NO
MATERIALITY_THRESHOLDS_CHANGED   = NO (same v_only_calibration.json; the
                                   analysis script is unchanged and reads
                                   the same frozen thresholds file)
V_ARM_CHANGED                    = NO (binary 81f569d8... re-verified
                                   unchanged; fresh V runs rerun in the
                                   same interleaved structure)
R_IMPLEMENTATION_CHANGED         = YES (binary 25c13f70..., head 2ea09b7)
CORRECTNESS_BEFORE_TIMING        = YES (unit + workspace + L0.5 PASS +
                                   A-LANE parity, all on the corrected
                                   binary, before any tlane run)
CAMPAIGN                         = 2026-09-30T14:44:42..14:47:04+08:00,
                                   5 V + 5 R independent processes,
                                   interleaved V,R x 5, taskset -c 2,
                                   one process per run, no perf sampling
EVIDENCE                         = results/horse-a-v2-step2r-104/
                                   corrective-campaign/ (raw tlane JSONs
                                   + derived receipt); results/horse-a-v2-
                                   step2r-104/corrective/ (validate + alane)
BOTH_SCREENINGS_PRESERVED        = YES (pair-campaign/ = pre-review;
                                   corrective-campaign/ = merge-
                                   authoritative)
```

## G. V vs R screening (all nine frozen cells)

Two complete screenings are retained. The pre-review screening
(binary 692631ac) is historical; the corrective screening (binary
25c13f70) is the merge-authoritative Step 2R result. Decision statistic
in both: median of the 5 paired-campaign per-run ARM_A medians.

Pre-review screening (head 9854f7c; retained unchanged):

| frozen cell | V median ns | R median ns | relative effect | frozen threshold | verdict |
|---|---|---|---|---|---|
| E6-1-LOSE-DUP-CHANGE | 4 657 227 | 2 164 441 | **−53.53%** | 2.53% | MATERIAL, faster |
| E6-4-LOW-FANOUT-VALUE | 3 997 977 | 1 529 045 | **−61.75%** | 2.00% | MATERIAL, faster |
| E6-5-HIGH-FANOUT-VALUE | 4 222 666 | 1 735 597 | **−58.90%** | 2.00% | MATERIAL, faster |
| E6-6-FENCE-HIDE-DEF (guardrail) | 910 054 | 907 958 | −0.23% | 2.50% | within noise |
| TINY-64B-E1 (guardrail) | 952 | 944 | −0.84% | 17.75% | within noise |
| TINY-1K-E2 (guardrail) | 6 996 | 7 004 | +0.11% | 31.44% | within noise |
| SENT-E5-EMPH (guardrail) | 556 261 | 553 663 | −0.47% | 5.98% | within noise |
| SENT-LIST-INDENT (guardrail) | 895 092 | 899 955 | +0.54% | 3.78% | within noise |
| SENT-BQ-NEST (guardrail) | 536 263 | 542 787 | +1.22% | 13.51% | within noise |

Corrective screening (head 2ea09b7; MERGE-AUTHORITATIVE):

| frozen cell | V median ns | R median ns | relative effect | frozen threshold | verdict |
|---|---|---|---|---|---|
| E6-1-LOSE-DUP-CHANGE | 4 632 743 | 2 154 608 | **−53.49%** | 2.53% | MATERIAL, faster |
| E6-4-LOW-FANOUT-VALUE | 3 993 632 | 1 498 464 | **−62.48%** | 2.00% | MATERIAL, faster |
| E6-5-HIGH-FANOUT-VALUE | 4 210 524 | 1 724 013 | **−59.05%** | 2.00% | MATERIAL, faster |
| E6-6-FENCE-HIDE-DEF (guardrail) | 903 573 | 870 158 | −3.70% | 2.50% | MATERIAL, FASTER (see PE-4) |
| TINY-64B-E1 (guardrail) | 946 | 943 | −0.32% | 17.75% | within noise |
| TINY-1K-E2 (guardrail) | 7 025 | 7 107 | +1.17% | 31.44% | within noise |
| SENT-E5-EMPH (guardrail) | 554 035 | 553 011 | −0.18% | 5.98% | within noise |
| SENT-LIST-INDENT (guardrail) | 897 295 | 888 948 | −0.93% | 3.78% | within noise |
| SENT-BQ-NEST (guardrail) | 537 801 | 534 611 | −0.59% | 13.51% | within noise |

Guardrail reading: the frozen rule requires a material REGRESSION on a
guardrail to be explained before any positive E6 reading is used. No
guardrail regressed in either screening (worst slower-side movement
+1.22% pre-review, +1.17% corrective). E6-6 crossed its threshold in the
FASTER direction in the corrective screening; the frozen rule does not
block on that, and the deviation is recorded as protocol event PE-4 with
its interpretation rather than silently absorbed.

Resource guardrails: NEW_RETAINED_STATE = 0 (structural); temporary
repair allocations = 0 (structural, gate included); A-LANE counters
byte-identical V/R on both binaries (parser evidence and route parity);
certificate_writes identical.
Context-only, same-run, not decision-bearing: H0_REFERENCE 1.21 ms,
H2_REFERENCE 0.68 ms on the dense cells (corrective screening; H0/H2 are
byte-identical mechanisms on both binaries and moved <1% between arms).

## H. Residual interpretation (corrective screening numbers)

```text
E6-1 (dense, equality-class): 4.63 -> 2.15 ms. The removed ~2.5 ms matches
    the removed matching work (L2 erratum E2 bounded certificate-barrier
    lookup as the robust qualitative localization of the P1-P0 excess).
    The remaining total cost of the legal same-target rebuild is
    ~0.95 ms above the same-run H0 clean parse (CONTEXT ONLY — see I),
    for an edit whose consumer-visible effective environment is
    UNCHANGED. This residual is the largest dense-cell number remaining
    and it is a SEMANTIC residual, not a matching-work residual.
E6-4 (low fanout, real change): 3.99 -> 1.50 ms.
E6-5 (high fanout, real change): 4.21 -> 1.72 ms.
    The E6-5 vs E6-4 total-price delta is ~0.23 ms after R; V showed
    ~0.22 ms on the same pair in the same campaign. This is recorded as
    a descriptive observation about THIS cell pair only; no general
    fanout model is claimed (see I, S2).
E6-6 (low-boundary contrast): pre-review -0.23%, corrective -3.70%
    (FASTER; PE-4). With few boundaries there is little matching work to
    remove; the corrective movement direction is favorable and no
    guardrail regressed.
tiny controls: unchanged (certificate lookup absent/small).
sentinels: unchanged (local/container strengths protected).
COST MIGRATION: none observed — matching work fell and latency fell with
    it; no allocation/retirement anomaly; no L3 question needed.
```

## I. Post-R decision and the S1/S2 reading (tightened by amendment v3)

```text
POST_R_DECISION = EARN_S1_QUESTION

S1_OPPORTUNITY = REAL
  E6-1 remains a genuine semantic over-invalidation counterexample:
  complete producer facts change while the relevant consumer-visible
  effective environment is unchanged. After R, executing the legal
  same-target rebuild still has material total cost (~2.15 ms on the
  dense geometry), so the question of unchanged-consumer early cutoff
  deserves independent study.

S1_RECOVERABLE_PRIZE = NOT YET MEASURED
  R_E6_1_TOTAL_COST ~= 2.15 ms. The same-run H0 difference (~0.95 ms)
  is CONTEXT ONLY, NOT an estimate of removable semantic work: H0 does
  not carry Horse-A's complete eager READY / next-edit obligations, so
  no statement of the form "S1 prize = R - H0", "S1 can recover 0.95 ms",
  "perfect S1 should reach H0", or "H0 is the lower bound for legal S1"
  is made or implied. The future S1 experiment must separate:
  required producer-state refresh, avoidable unchanged-consumer
  propagation/rebuild work, and new certification/maintenance cost.
  The S1 question is recorded, NOT implemented, NOT frozen; it is priced
  against the REPAIRED baseline (~2.15 ms), not the historical V
  baseline (~4.63 ms), per the #95 interaction rule.

S2 = NOT_SELECTED
  Under the frozen #95/#104 decision rule, a dependency-discovery
  question (R-C) is earned only if the E6-4-style residual DOMINATES.
  It does not (E6-4 1.50 ms < E6-1 2.15 ms), so S2 was not the dominant
  post-R question under the frozen screening rule. This is a selection
  statement for THIS screening, not a claim that dependency structure
  can never matter; no general fanout claim is made from the E6-4/E6-5
  pair.

R_ONLY_CANDIDATE / NO_NEW_MECHANISM_HEADROOM = not declared: the
  remaining dense residual is dominated by the semantic rebuild
  opportunity, not by further same-authority shrink.
TARGETED_DIAGNOSIS_REQUIRED = NO. R_REJECTED = NO.
```

## J. Independent reviews

```text
CONTRACT_REVIEW_A (semantic/correctness, pre-implementation)
  = P0=1 P1=2 P2=4 P3=4 -> all resolved in frozen contract v2 (#104)
CONTRACT_REVIEW_B (minimal-mechanism, pre-implementation)
  = P0=0 P1=1 P2=2 P3=4 -> all resolved (counter channel relocated
  outside the frozen record; identity constraints pinned)
IMPLEMENTATION_REVIEW_C (same-authority, post-implementation)
  = P0=0 P1=0 P2=1 P3=4. P2-1: the frozen protocol's declared virtual-
    call asymmetry said ~N_b; the implementation charges a matched
    boundary twice (seek batch + peek), i.e. up to ~2xN_b — recorded as
    Protocol Event PE-1, not silently edited. P3s: sink-tally overflow
    discipline (unreachable), duplicate-before-support case added
    (battery 11b), screened-commit vs reviewed-HEAD sha relationship
    (PE-2), evidence tree must land with the PR (done).
METHODOLOGY_REVIEW_D (research/process, post-implementation)
  = P0=0 P1=0 P2=2 P3=6. P2-1: branch tip moved aa0efca -> 15ce147
    after the campaign without a protocol event — recorded as PE-2
    (test-only delta; release binary byte-identical, so the frozen
    R_BINARY sha256 remains valid). P2-2: raw artifacts do not embed
    binary sha/host/affinity — mitigated by the retained binaries
    (re-verified sha256 at report time), the issue trail, argv paths,
    and the retention addendum; future receipts should embed identity.
    P3s: affinity attested in-issue only; calibration argv relative
    path; validate raw outputs now retained; E6-6 role label in the
    derived JSON realigned to the frozen text (PE-3); narrative
    precision tightened in this report (0.93 ms / 0.50-0.51 ms exact
    values replace the earlier rounded ranges); "every sample kept"
    describes the live run, while retained tlane.json stores summary
    statistics only.
```

## K. Protocol events (post-freeze deviations, recorded not repaired)

```text
PE-1  The frozen contract v2 / protocol described the diagnostic charge
      as "once per boundary". The implementation charges a matched
      boundary twice (seek batch + duplicate peek): up to ~2xN_b virtual
      calls per persist call instead of ~N_b. Still negligible against
      the measured windows and declared here per the protocol's own
      no-silent-repair rule. No decision-bearing number is affected.
PE-2  The branch tip was rewritten aa0efca -> 15ce147 AFTER the paired
      campaign (strengthening a test assertion from a bound to the exact
      hand-traced 109 and adding battery case 11b after reviews). The
      delta is entirely in #[cfg(test)] code: the release binary — and
      therefore the pre-frozen R_BINARY sha256 692631ac… — is
      byte-identical; the timing evidence transfers unchanged.
PE-3  pair_campaign_result.json labeled E6-6 role "challenge"; the frozen
      protocol text groups E6-6 with the guardrails. The derived JSON was
      regenerated (script one-line change) to match the frozen text; raw
      run data untouched; E6-6 is within noise either way.
PE-4  (amendment v3 corrective screening) guardrail E6-6 moved -3.70%,
      crossing its 2.50% materiality threshold in the FASTER direction
      (pre-review screening: -0.23%, within). The frozen rule requires
      explanation only for material REGRESSIONS on guardrails; none
      occurred on any cell in either screening. Interpretation recorded,
      not silently absorbed: E6-6's update path also runs the certificate
      matcher, so a small favorable movement is consistent with the
      repair; the between-campaign swing (R median 907958 -> 870158 ns)
      exceeds the V-only calibration spread and may include code-layout
      effects of the added gate on this one binary; no decision-bearing
      reading uses E6-6, and both raw screenings are retained.
PE-5  (amendment v3 corrective rerun) the pre-review screening was NOT
      reused as final after the P1 fix changed production code. The
      frozen protocol, thresholds, repetitions, order and host were
      rerun unchanged (SCREENING_PROTOCOL_CHANGED = NO,
      MATERIALITY_THRESHOLDS_CHANGED = NO, V_ARM_CHANGED = NO,
      R_IMPLEMENTATION_CHANGED = YES); both screenings are preserved in
      the evidence tree and the corrective screening is the
      merge-authoritative Step 2R result.
```

## L. GitHub receipt

```text
STEP2R_ISSUE        = #104
PR                  = (implementation/evidence PR; OPEN, unmerged)
PR_STATE            = OPEN
MERGED              = NO
ISSUE_95_CLOSED     = NO
SEMANTIC_CHALLENGER_CREATED = NO
HISTORICAL_V        = IMMUTABLE (all #98/#100/erratum paths byte-unchanged
                      on the branch; verified by the methodology review)
NEXT                = FRESH_POST_FIX_REVIEW_THEN_MERGE_GATE
```
