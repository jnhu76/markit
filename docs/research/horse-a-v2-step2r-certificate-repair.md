# Horse-A v2 Step 2R — same-authority certificate-matching repair R

> Scope: issue [#104](https://github.com/jnhu76/markit/issues/104) (Step 2R
> contract, protocol freeze, calibration, screening receipt, decision) —
> the one Step-2R issue under the #95 umbrella. The L2 evidence this repair
> answers lives in
> [horse-a-v2-l2-context-profile-100.md](horse-a-v2-l2-context-profile-100.md)
> plus its append-only
> [erratum](horse-a-v2-l2-evidence-erratum-2026-09-30.md).
> Status: **CONTRACT AMENDMENT v3 APPLIED AND REVIEW-FIXED (independent
> reviews E/F round 1 resolved: gate extended to the cuts ordering,
> diagnostic tally made panic-free, retention claims corrected); FINAL
> screening complete on the final binary; fresh reviews E'/F' rerun on
> the final head before any merge; #95 is not closed; no S mechanism
> exists.**

```text
STEP_1_DIAGNOSIS   = COMPLETE (#97/#98/#100)
L2_ERRATUM         = MERGED (PR #103)
STEP_2R            = AMENDED_v3_FINAL_SCREENING_COMPLETE_FOR_REVIEW_GATE
L3/L4/L5           = NOT EXECUTED
HISTORICAL_V       = IMMUTABLE (master 352e214; nothing rewritten)
SEMANTIC_CHALLENGER_FROZEN = NO (S1 question EARNED and RECORDED only)
NEXT               = FRESH_POST_FIX_REVIEW_THEN_MERGE_GATE
```

## A. Authority

```text
LIVE_MASTER_AT_START = 352e214fc43ff22a89c835c5407a107d83cbe33e
                       (task-creation authority re-verified live at every
                       round: origin/master still 352e214; no drift)
ISSUE                = #104 (created for Step 2R; body + comments carry the
                       full contract/protocol/calibration/result trail plus
                       the append-only CONTRACT AMENDMENT v3 and corrective
                       receipts)
BRANCH               = research/95-step2r-certificate-match-r1
R_HEADS              = pre-review screening head 9854f7c (binary 692631ac…);
                       corrective round-1 head 2ea09b7/d9d613b (binary
                       25c13f70…); FINAL head a9270ba (binary 734cccba…).
                       Research identity is logically ONE mechanism,
                       HORSE-A-V2-STEP2R-CERT-MATCH-R1, across all heads.
V_IDENTITY           = frozen Horse-A v1 = master 352e214
                       certificate.rs blob 564b84dc8ae0374e8c3508b560ef5f0
                       eb42ca626 (sha256 259f776bf9e44623e4d2d3358569871a
                       811150a2a1d3726b8327e6830ac00dbc)
V_BINARY             = sha256 81f569d8349a52d4efb67c064eada519bb42899bbcb67
                       2cd62a03ec24f26ce86, built from worktree
                       /home/jnhu/Source/markit-v-frozen (detached 352e214).
                       UNCHANGED through all campaigns. Retention is now
                       DUAL-LOCATION after incident PE-7: the original
                       bytes live at the worktree path AND at
                       /home/jnhu/retained-bench-binaries/
                       V-frozen-352e214-81f569d8 (outside any cargo
                       target dir).
R_IDENTITY           = HORSE-A-V2-STEP2R-CERT-MATCH-R1
                       (CERTIFICATE_MATCH_REPAIR_ID, certificate.rs only;
                       never enters ReadyDocument/InterpretationId)
R_BINARY_FINAL       = sha256 734cccba37717074e67e99c507a9939087e62e08327
                       45d9c338e851297c450fe, built from the final
                       production-code tree (head a9270ba; the docs/evidence
                       commits after it touch no .rs file, so the binary is
                       valid for the final PR head); RETAINED at the R
                       target/release path AND at
                       /home/jnhu/retained-bench-binaries/
                       R-final-a9270ba-734cccba
R_BINARY_PRE_REVIEW  = sha256 692631acc10247cb39f6f46be96dd25adfea64f9bf55
                       0ae6ba41ea29e681bb46 — NO LONGER ON HOST: the
                       original single-copy retention (the R target/release
                       path) was overwritten by the corrective rebuild
                       (PE-7's lesson). Its identity is attested by the
                       #104 protocol receipt + the PE-2 test-only-delta
                       chain; the pre-review screening it produced is
                       explicitly NOT merge-authoritative.
R_BINARY_CORRECTIVE  = sha256 25c13f70c840e2e28e3c51daacd09aa4375191687864
                       c3201b759c4dc1e2731f (corrective round-1; also no
                       longer on host, same overwrite class — its
                       screening is retained as history, superseded by
                       the FINAL screening)
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

R-C2a (amendment v3, extended after review E) ORDERING_ELIGIBILITY_GATE =
      the fast traversal runs only after an allocation-free scan
      confirms BOTH orderings it relies on:
      `barriers.windows(2).all(|w| w[0].cut < w[1].cut)` AND
      `cuts[..=boundary_count].windows(2).all(|w| w[0] < w[1])`
      (vacuously true for single-element windows; at most
      `(B - 1) + boundary_count` comparisons). Review E demonstrated a
      P0 on the barriers-only gate: with NON-MONOTONE CUTS and a
      gate-eligible barrier, the fast traversal's exhaustion break
      installs nothing where V's filter loop installs a certificate
      (cuts [0,10,3,6], barrier cut 6 -> V persists owner 2,
      {Some(0), 1..3}). Legal production always passes both (R-C2 +
      CoveragePlan cuts checks). A violation of either ordering routes
      to the frozen-V fallback (`v1_defensive_fallback`, the 352e214
      matcher loop copied verbatim, defined on arbitrary input,
      including its forbidden-sentinel tail and early-Err shape).
      NO sorting, deduplication, assertion, panic, new retained state,
      heap allocation, or new error class. Totality (R-C9): every input
      now has V's exact defined behavior — the gate covers the FULL
      assumption set of the fast traversal, so V-parity holds on EVERY
      input, legal or defensive.

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
  R:  an eligibility gate first (R-C2a): allocation-free scans confirm
      BOTH orderings (barriers strictly increasing by cut; cuts
      strictly increasing). Eligible (every legal production call): one
      function-local usize cursor
      advanced monotonically over the barriers as the examined
      boundaries increase; exact-equality match at the cursor; the
      adjacent-element duplicate peek retained as a defended site
      (provably inert under the gate); then V's support checks,
      installation, charges and error paths VERBATIM. The matched
      barrier is consumed (cursor += 1); exhaustion breaks (nothing
      later can match).
      NOT eligible (violation of either ordering, unreachable from
      legal parsing): the frozen V matcher verbatim
      (`v1_defensive_fallback`), restoring V's exact seam semantics on
      every input shape (AMENDMENT v3, extended after review E).
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
  battery            = every enumerated seam shape (see `covers`)
                       x (Result + exact message + full Owner vector +
                       certificate_write parity): ALL identical; plus the
                       separate cuts-precondition test
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
  DEFENSIVE BATTERY (amendment v3, D1–D9): non-adjacent duplicate
                       (exact V error text, fallback route charges 0
                       fast-matcher inspections); duplicate after a larger
                       out-of-order event; unsorted
                       without duplicate (full Result/Owner/writes parity
                       plus the same-shape monotone twin proving both
                       routes converge on the same state); unsorted
                       duplicates at an unexamined cut stay silent; lowest
                       examined duplicate boundary errors first under
                       unsorted input; duplicate precedence over a
                       support-failing first candidate under unsorted
                       geometry; support-rejected exact match skips
                       identically on both routes; NON-MONOTONE CUTS
                       (review E's demonstrated P0 shapes: cuts
                       [0,10,3,6] and [0,100,5,6] with a single barrier —
                       V's installed certificate is reproduced exactly by
                       the fallback route; plus combined cuts+barriers
                       violations); legal cuts shapes stay on the fast
                       path (gate routing both ways)
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
CORRECTIVE RERUNS (protocol-preserving; see F for PE-6/PE-7):
  ROUND 1 (amendment v3, binary 25c13f70): unit + workspace green
                       (horse-a 256 tests incl. D1–D7); L0.5 validate
                       PASS both binaries 13/13; A-LANE V/R
                       byte-identical; screening under corrective-
                       campaign/ (retained as history).
  FINAL (review-E fixes, binary 734cccba, head a9270ba): horse-a 258
                       tests green (incl. D8/D9, the non-monotone-CUTS
                       differential); workspace green except the sealed
                       #100 campaign crate, whose code is byte-identical
                       on both trees (0 diff lines vs 352e214) and whose
                       failing tests are environment/state-dependent,
                       differing in NAME between trees and runs on both
                       sides — classified not-R-caused, outside this
                       PR's scope; L0.5 validate PASS both binaries
                       13/13 (final/validate-retention/); A-LANE V/R
                       byte-identical on all 13 cells
                       (final/alane-v, final/alane-r); FINAL screening
                       under final-campaign/ (MERGE-AUTHORITATIVE).
PRE-REVIEW SUITES (head 9854f7c, binary 692631ac — retained for the
  record)            = horse-a crate 249 tests green; full workspace
                       green except the sealed campaign crate
                       (environment-dependent, not R-caused); L0.5 PASS
                       on both binaries (retained under
                       validate-retention/)
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
ORDER_CHECK_WORK  = max(B - 1, 0) + boundary_count cut comparisons for
                    the eligibility gate (barriers scan + cuts scan;
                    extended after review E); allocation-free; NOT
                    charged to the diagnostic counter (fixed scope,
                    below).
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
TOTAL_LEGAL_PATH  = ORDER_CHECK + FAST_MATCH <= 2B + 2*boundary_count
                    + R - 1 (empty shape B = 0, boundary_count = 0: no
                    work) — linear in B + N_b. On the dense E6 geometry
                    (~2050 barriers x ~2050 examined boundaries):
                    V performed ~4.2M matching inspections per full
                    build; R performs ~4.1k fast-match inspections plus
                    ~4.1k order-check comparisons.
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

### Corrective reruns (amendment v3 — protocol-preserving)

Production code changed twice after the pre-review screening (the v3
fallback fix; then review E's gate extension), so pre-review timing was
NOT reused. The frozen screening protocol was rerun unchanged each time;
nothing was recalculated after seeing R numbers.

```text
CORRECTIVE_RERUN_REASON          = independent-review fixes (round 1: P1
                                   fallback fix; round 2: review-E P0
                                   gate extension)
SCREENING_PROTOCOL_CHANGED       = NO (both reruns)
MATERIALITY_THRESHOLDS_CHANGED   = NO (both reruns; the analysis script
                                   is unchanged and reads the same
                                   v_only_calibration.json)
V_ARM_CHANGED                    = NO (binary 81f569d8... re-verified
                                   before every campaign; fresh V runs
                                   in the same interleaved structure)
R_IMPLEMENTATION_CHANGED         = YES (round-1 binary 25c13f70..., head
                                   2ea09b7; FINAL binary 734cccba...,
                                   head a9270ba)
CORRECTNESS_BEFORE_TIMING        = YES (both reruns: unit + workspace +
                                   L0.5 PASS + A-LANE parity, all on the
                                   round's binary, before any tlane run)
ROUND-1 CAMPAIGN                 = 2026-09-30T14:44:42..14:47:04+08:00
FINAL CAMPAIGN                   = 2026-09-30T15:49:54..15:52:17+08:00,
                                   5 V + 5 R independent processes each,
                                   interleaved V,R x 5, taskset -c 2,
                                   one process per run, no perf sampling
EVIDENCE                         = results/horse-a-v2-step2r-104/
                                   {corrective-campaign, final-campaign}/
                                   plus corrective/ and final/ (validate
                                   + alane); ALL screenings preserved:
                                   pair-campaign/ = pre-review;
                                   corrective-campaign/ = round 1
                                   (history); final-campaign/ =
                                   MERGE-AUTHORITATIVE
QUARANTINE                       = final-campaign-quarantine-PE7/ — one
                                   aborted 15:46 campaign started against
                                   an accidentally rebuilt V binary before
                                   the recovery in PE-7 was complete;
                                   retained for the record, used for
                                   nothing
```

## G. V vs R screening (all nine frozen cells)

Three complete screenings are retained: the pre-review screening
(binary 692631ac, table preserved in the #104 corrective receipt and in
`corrective-campaign`'s sibling history), the round-1 corrective
screening (binary 25c13f70 — history; its table is in the #104 corrective
receipt), and the FINAL screening below (binary 734cccba,
MERGE-AUTHORITATIVE). Decision statistic in all: median of the 5
paired-campaign per-run ARM_A medians.

FINAL screening (head a9270ba; MERGE-AUTHORITATIVE):

| frozen cell | V median ns | R-final median ns | relative effect | frozen threshold | verdict |
|---|---|---|---|---|---|
| E6-1-LOSE-DUP-CHANGE | 4 653 919 | 2 147 378 | **−53.86%** | 2.53% | MATERIAL, faster |
| E6-4-LOW-FANOUT-VALUE | 3 987 410 | 1 501 198 | **−62.35%** | 2.00% | MATERIAL, faster |
| E6-5-HIGH-FANOUT-VALUE | 4 205 224 | 1 723 155 | **−59.02%** | 2.00% | MATERIAL, faster |
| E6-6-FENCE-HIDE-DEF (guardrail) | 902 953 | 873 207 | −3.29% | 2.50% | MATERIAL, FASTER (see PE-4) |
| TINY-64B-E1 (guardrail) | 958 | 942 | −1.67% | 17.75% | within noise |
| TINY-1K-E2 (guardrail) | 6 882 | 7 124 | +3.52% | 31.44% | within noise |
| SENT-E5-EMPH (guardrail) | 551 576 | 554 633 | +0.55% | 5.98% | within noise |
| SENT-LIST-INDENT (guardrail) | 889 951 | 897 884 | +0.89% | 3.78% | within noise |
| SENT-BQ-NEST (guardrail) | 531 161 | 543 461 | +2.32% | 13.51% | within noise |

The round-1 corrective screening (binary 25c13f70) showed the same
shape: −53.49/−62.48/−59.05% on the challenges, no guardrail regression,
E6-6 −3.70% favorable — consistent across both post-fix binaries.

Guardrail reading: the frozen rule requires a material REGRESSION on a
guardrail to be explained before any positive E6 reading is used. No
guardrail regressed in any screening (worst slower-side movement +1.22%
pre-review, +1.17% round 1, +3.52% final — all far below their
thresholds). E6-6 crossed its threshold in the FASTER direction in both
post-fix screenings; the frozen rule does not block on that, and the
deviation is recorded as protocol event PE-4 with its interpretation
rather than silently absorbed.

Resource guardrails: NEW_RETAINED_STATE = 0 (structural); temporary
repair allocations = 0 (structural, gate included); A-LANE counters
byte-identical V/R on every binary (parser evidence and route parity);
certificate_writes identical.
Context-only, same-run, not decision-bearing (FINAL screening):
H0_REFERENCE 1.24 ms, H2_REFERENCE 0.68 ms on the dense cells; H0/H2 are
byte-identical mechanisms on both binaries and moved <2% between arms.

## H. Residual interpretation (FINAL screening numbers)

```text
E6-1 (dense, equality-class): 4.65 -> 2.15 ms. The removed ~2.5 ms matches
    the removed matching work (L2 erratum E2 bounded certificate-barrier
    lookup as the robust qualitative localization of the P1-P0 excess).
    The remaining total cost of the legal same-target rebuild is
    ~0.91 ms above the same-run H0 clean parse (CONTEXT ONLY — see I),
    for an edit whose consumer-visible effective environment is
    UNCHANGED. This residual is the largest dense-cell number remaining
    and it is a SEMANTIC residual, not a matching-work residual.
E6-4 (low fanout, real change): 3.99 -> 1.50 ms.
E6-5 (high fanout, real change): 4.21 -> 1.72 ms.
    The E6-5 vs E6-4 total-price delta is ~0.22 ms after R; V showed
    ~0.22 ms on the same pair in the same campaign. This is recorded as
    a descriptive observation about THIS cell pair only; no general
    fanout model is claimed (see I, S2).
E6-6 (low-boundary contrast): pre-review -0.23%; -3.70% and -3.29%
    (FASTER) in the two post-fix screenings (PE-4). With few boundaries
    there is little matching work to remove; the movement direction is
    favorable and no guardrail regressed.
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
  R_E6_1_TOTAL_COST ~= 2.15 ms. The same-run H0 difference (~0.91 ms)
  is CONTEXT ONLY, NOT an estimate of removable semantic work: H0 does
  not carry Horse-A's complete eager READY / next-edit obligations, so
  no statement of the form "S1 prize = R - H0", "S1 can recover 0.91 ms",
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
POST_FIX_REVIEW_E (semantic/exact-equivalence adversary, fresh context,
  round 1 on corrective head d9d613b)
  = P0=1 P1=1 P2=2 P3=1. P0 (EMPIRICALLY DEMONSTRATED): the
    amendment-v3 gate checked only the BARRIERS ordering, but the fast
    traversal's exhaustion break additionally assumes the CUTS strictly
    increasing — with non-monotone cuts [0,10,3,6] and a single barrier
    cut 6, V installs owner 2's certificate while the fast path installs
    nothing (unreachable from every in-tree caller, but inside the
    seam's parity claim). P1: the battery's Owner helper panicked on
    decreasing cuts, making the whole battery structurally unable to
    test cuts-ordering violations. P2s: the doc's parity sentence was
    broader than the mechanism; the diagnostic tally's checked_add
    .expect() was a new panic mode. ALL RESOLVED in head a9270ba: the
    gate now verifies BOTH orderings (R-C2a); differential D8 pins E's
    exact shapes over a saturating Owner builder (D9 pins legal-shape
    routing); the tally is saturating_add (no new panic mode); the doc
    now claims exactly what the mechanism does; the fallback's dropped
    V comment sentences restored (P3).
POST_FIX_REVIEW_F (methodology/claim audit, fresh context, round 1 on
  corrective head d9d613b)
  = P0=0 P1=1 P2=1 P3=5. Explicit answers: tighter work bound anywhere
    = NO (the R term proved necessary and sufficient);
    R−H0-as-S1-prize anywhere = NO; S2 overclaim in active surfaces =
    NO; thresholds unchanged (programmatically equal to both derived
    receipts); alane V/R byte-identical; all 18 published screening
    numbers recomputed from raw and exact; historical evidence pristine.
    P1: the "RETAINED" claim for the pre-review binary 692631ac had
    become false (overwritten by the corrective rebuild) — report/README
    reworded to state the overwrite honestly (and PE-7 dual-location
    retention added). P2: corrective-head identity string divergence
    (d9d613b vs 2ea09b7) — resolved by stating the exact tree the binary
    was built from and the no-.rs-delta fact. P3s: "examined at most
    once" wording (fixed), empty-shape bound note (added), immutable
    #04 comment S2 phrasing (superseded by receipt), argv-without-
    taskset (known, attested in-issue), trait-doc batching wording
    (fixed).
POST_FIX_REVIEW_ROUND_2 (E' semantic, F' methodology — fresh contexts)
  = rerun on the FINAL head; verdicts recorded on #104 before any merge
    gate. Merge requires P0 = P1 = P2 = 0 on both.
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
PE-6  (review-E gate extension, head a9270ba) round-1 review E found a
      P0 (cuts-ordering gap) requiring a production-code change, so the
      corrective round-1 screening was NOT reused as final either: the
      frozen protocol was rerun a second time, unchanged, on the FINAL
      binary 734cccba (see the corrective-reruns block in F). Three
      complete screenings are retained; final-campaign/ is
      merge-authoritative.
PE-7  (V-binary retention incident + recovery) during the review-E fix
      round, a `cargo test --release` executed inside the frozen V
      worktree rebuilt and overwrote the retained V binary at its
      target/release path (sha temporarily 34aca634...). One final-
      campaign attempt (15:46..15:48) started against that wrong V arm
      and was QUARANTINED unused (final-campaign-quarantine-PE7/,
      retained for the record, used for nothing). The original bytes
      were recovered intact from the cargo dep artifact
      mdbench_horse_a_v2_diag-5a2c026aa87d37c2 (sha re-verified
      81f569d8...), restored to the retention path, and copied to a
      second location outside any cargo target dir
      (/home/jnhu/retained-bench-binaries/); the R-final binary got the
      same dual retention. The FINAL campaign (15:49:54..15:52:17) ran
      with the re-verified frozen V binary. Lesson recorded: benchmark
      binaries are never single-copied inside a cargo target dir, and
      cargo test is never run in a worktree that holds a retained
      benchmark binary. (The same overwrite class had earlier silently
      destroyed the pre-review and round-1 R binaries — review F's P1 —
      which is why their retention claims were reworded and dual
      location introduced.)
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
