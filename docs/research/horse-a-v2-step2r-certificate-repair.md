# Horse-A v2 Step 2R — same-authority certificate-matching repair R

> Scope: issue [#104](https://github.com/jnhu76/markit/issues/104) (Step 2R
> contract, protocol freeze, calibration, screening receipt, decision) —
> the one Step-2R issue under the #95 umbrella. The L2 evidence this repair
> answers lives in
> [horse-a-v2-l2-context-profile-100.md](horse-a-v2-l2-context-profile-100.md)
> plus its append-only
> [erratum](horse-a-v2-l2-evidence-erratum-2026-09-30.md).
> Status: **IMPLEMENTATION + SCREENING COMPLETE FOR INDEPENDENT REVIEW —
> the R PR is open, unmerged; #95 is not closed; no S mechanism exists.**

```text
STEP_1_DIAGNOSIS   = COMPLETE (#97/#98/#100)
L2_ERRATUM         = MERGED (PR #103)
STEP_2R            = COMPLETE_FOR_REVIEW (this report + open PR)
L3/L4/L5           = NOT EXECUTED
HISTORICAL_V       = IMMUTABLE (master 352e214; nothing rewritten)
SEMANTIC_CHALLENGER_FROZEN = NO (S1 question EARNED and RECORDED only)
NEXT               = INDEPENDENT_STEP2R_REVIEW
```

## A. Authority

```text
LIVE_MASTER_AT_START = 352e214fc43ff22a89c835c5407a107d83cbe33e
                       (task-creation authority re-verified live; local
                       checkout fast-forwarded fb359de -> 352e214; no drift)
ISSUE                = #104 (created for Step 2R; body + 6 comments carry
                       the full contract/protocol/calibration/result trail)
BRANCH               = research/95-step2r-certificate-match-r1
R_HEAD_AT_REPORT     = 15ce147 (see Protocol Events: the campaign ran at
                       sibling aa0efca; production code byte-identical,
                       test-only delta)
V_IDENTITY           = frozen Horse-A v1 = master 352e214
                       certificate.rs blob 564b84dc8ae0374e8c3508b560ef5f0
                       eb42ca626 (sha256 259f776bf9e44623e4d2d3358569871a
                       811150a2a1d3726b8327e6830ac00dbc)
V_BINARY             = built from worktree /home/jnhu/Source/markit-v-frozen
                       (detached 352e214); sha256 81f569d8349a52d4efb67c06
                       4eada519bb42899bbcb672cd62a03ec24f26ce86; RETAINED
                       on host (path above), re-verified at report time
R_IDENTITY           = HORSE-A-V2-STEP2R-CERT-MATCH-R1
                       (CERTIFICATE_MATCH_REPAIR_ID, certificate.rs only;
                       never enters ReadyDocument/InterpretationId)
R_BINARY             = sha256 692631acc10247cb39f6f46be96dd25adfea64f9bf55
                       0ae6ba41ea29e681bb46; RETAINED on host, re-verified
HOST                 = E5 (jnhu@192.168.31.75; Xeon E5-2666 v3, 20 cores),
                       Linux 7.2.5-200.fc44.x86_64, schedutil untouched
TOOLCHAIN            = workspace pin 1.97.1 (rust-toolchain.toml; verified
                       in-workspace: rustc 1.97.1 8bab26f4)
PROFILE              = frozen [profile.release] (opt-level 3, lto=thin,
                       codegen-units 1, incremental=false, panic=unwind)
DIFF SCOPE           = git diff 352e214..15ce147 = 4 files, all
                       mechanisms/horse-a/src/ (certificate.rs, lib.rs,
                       structural.rs, step2r_tests.rs). Every harness and
                       measurement crate byte-identical across V/R.
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
      check, lowest examined boundary errors first (R-C3b). Duplicate
      detection under R's traversal = one adjacent-element peek (under
      monotonicity a second same-cut event is exactly the next element).

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
      behavior untestable and would panic where V does not). Behavior on
      violative input is defined by the traversal, unreachable from both
      proven callers, and not relied on.

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
  R:  one function-local usize cursor advanced monotonically over the
      barriers as the examined boundaries increase; exact-equality match
      at the cursor; duplicate detection by one adjacent-element peek
      BEFORE support checks; then V's support checks, installation,
      charges and error paths VERBATIM. The matched barrier is consumed
      (cursor += 1); exhaustion breaks (nothing later can match).
JUSTIFICATION = R-C1 + R-C2 (both sequences strictly increasing at every
      legal call site; all four call sites enumerated: full_build.rs,
      fresh.rs, two single-element test callers).

NEW_RETAINED_STATE           = 0 (no field added to any persisted type;
                                state.rs byte-identical)
NEW_TEMPORARY_STATE          = 2 scalars (cursor, batch tally) — no heap
                                allocation in the matcher; the only
                                allocations are the pre-existing error
                                format! Strings V also allocates
NEW_PERSISTENT_INDEX         = none
CERTIFICATE_AUTHORITY_CHANGED= NO
CERTIFICATE_SET_CHANGED      = NO
READY_SEMANTICS_CHANGED      = NO
NEXT_EDIT_CONTRACT_CHANGED   = NO
PARSER_EVIDENCE_CHANGED      = NO (A-LANE counters byte-identical V/R)

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
                       exact-asserted; <= B + N_b = 111 bound)
FROZEN SUITES       = horse-a crate 249 tests green (i2/i4/i5 +
                       conformance C1-C3 unchanged and green); full
                       workspace green except the sealed campaign crate,
                       whose failing set on R is a STRICT SUBSET of the
                       failing set on frozen V (pre-existing,
                       environment-dependent, not R-caused)
COMPARATOR GATE     = L0.5 `validate` PASS on BOTH binaries (raw outputs
                       retained under results/horse-a-v2-step2r-104/
                       validate-retention/)
A-LANE PARITY       = alane.json V vs R: byte-identical counters on all
                       13 cells for every arm (except the recorded argv
                       path), incl. certificate_writes and
                       forbidden_unaffected_certificate_writes = 0
```

## E. Work-reduction receipt

```text
V_MATCHING_WORK   = exactly N_b x B barrier-cut predicate evaluations per
                    persist call, absent the duplicate abort — proven
                    structurally (the lazy filter is exhausted for every
                    examined boundary: first-match scan k+1 evals +
                    remainder scan B-k-1 evals = B; no-match scan = B).
                    V charges no counter; this number is derived, not
                    measured.
R_MATCHING_WORK   <= B + N_b inspections (each barrier entered at most
                    once by the seek, +1 duplicate peek per boundary with
                    a candidate), measured by the diagnostic tally:
                    hand-traced exact 6 (vs V 10) and dense-case exact
                    109 (vs V 2480, a 22.8x reduction; bound 111).
ASYMPTOTIC_CLAIM  = Theta(N_b x B) -> O(B + N_b) on the proved monotone
                    preconditions; claimed ONLY under those preconditions
                    (documented caller invariants, R-C1/R-C2).
DIAGNOSTIC_COUNTERS = one: certificate_barrier_inspections (batched per
                    boundary; recording lane only; outside the frozen
                    record). Evidence order per contract: structural proof
                    leading, counter confirmation only.
DENSE GEOMETRY    = E6 documents have ~2050 boundaries/barriers, so V
                    performed ~4.2M matching inspections per full build;
                    R performs ~4.1k.
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
  alane JSONs, validate retention, analysis scripts + derived receipts,
  all regenerable; schemas HORSE-A-V2-STEP2R-104-*-v1).
```

## G. V vs R screening (all nine frozen cells)

Decision statistic: median of the 5 paired-campaign per-run ARM_A medians.

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

Resource guardrails: NEW_RETAINED_STATE = 0 (structural); temporary
repair allocations = 0 (structural); A-LANE counters byte-identical V/R
(parser evidence and route parity); certificate_writes identical.
Context-only, same-run, not decision-bearing: H0_REFERENCE 1.02–1.24 ms,
H2_REFERENCE 0.46–0.68 ms on the dense cells.

## H. Residual interpretation

```text
E6-1 (dense, equality-class): 4.66 -> 2.16 ms. The removed ~2.5 ms matches
    the removed matching work (L2 erratum E2 bounded certificate-barrier
    lookup as the robust qualitative localization of the P1-P0 excess).
    Residual = the legal same-target rebuild itself: 0.93 ms above the
    same-run H0 clean parse, for an edit whose consumer-visible effective
    environment is UNCHANGED. This residual is the largest dense-cell
    number remaining and it is a SEMANTIC residual, not a matching-work
    residual.
E6-4 (low fanout, real change): 4.00 -> 1.53 ms; 0.50 ms above H0.
E6-5 (high fanout, real change): 4.22 -> 1.74 ms; 0.51 ms above H0.
    The fanout delta between E6-4 and E6-5 is ~0.21 ms after R — the same
    absolute delta V showed (~0.22 ms): the rebuild price is
    document-size-dominated, not fanout-dominated.
E6-6 (low-boundary contrast): unchanged (-0.23%), as predicted — with few
    boundaries there is little matching work to remove.
tiny controls: unchanged (certificate lookup absent/small).
sentinels: unchanged (local/container strengths protected).
COST MIGRATION: none observed — matching work fell and latency fell with
    it; no allocation/retirement anomaly; no L3 question needed.
```

## I. Post-R decision

```text
POST_R_DECISION = EARN_S1_QUESTION
  The E6-1-class residual remains material AFTER the repair: an
  equality-class same-target rebuild still costs ~2.16 ms on the dense
  geometry while the effective environment is unchanged. The earned
  question (recorded, NOT implemented, NOT frozen): can
  consumer-visible environment equality / early cutoff preserve unchanged
  consumer results while complete producer semantic state is still
  updated correctly, priced against the REPAIRED baseline (~2.16 ms), not
  the historical V baseline (~4.66 ms), per the #95 interaction rule.
EARN_S2_QUESTION = NO — E6-4-style residual does not dominate (1.53 ms <
  2.16 ms) and the repaired rebuild price is fanout-insensitive.
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
NEXT                = INDEPENDENT_STEP2R_REVIEW
```
