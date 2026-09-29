# Horse-A v2 L2 — paired context-profile localization of the full-build construction residual

> Issue: [#100](https://github.com/jnhu76/markit/issues/100) (execution contract)
> Parent authority: [#95](https://github.com/jnhu76/markit/issues/95) direction ·
> [#96](https://github.com/jnhu76/markit/issues/96) methodology ·
> [#97](https://github.com/jnhu76/markit/issues/97) umbrella ·
> [#98](https://github.com/jnhu76/markit/issues/98) CLOSED L0/L1
> (`docs/research/horse-a-v2-l0l1-e6-tiny-diagnosis.md`; its evidence
> directory is sealed and untouched)
> Status: **L2 COMPLETE — escalation decision: L2_SUFFICIENT_FOR_MECHANISM_DECISION**
> Machine-readable evidence:
> `research/benchmarks/markdown-ast-update/results/horse-a-v2-l2-100/`

L2 answers exactly one question (#98 §K, #100):

> Which calling contexts account for the
> `P1_FULL_BUILD_ONLY - P0_H0_FULL_PARSE_ONLY` construction residual
> inside `full_build(post)`?

**Headline:** the residual is **not diffuse and not the representation**.
On E6-1 and E6-5 ≈ **90% of the construction excess executes in one calling
context** — the `RootBlankEvent` **barrier lookup inside
`persist_interior_certificates`** (`full_build → attach_interior_certificates
→ barriers.iter().filter(|ev| ev.cut == boundary)` for every interior
boundary; `certificate.rs:140-149`). Owner payload materialization
**cancels against H0's document construction** (28.5 vs 29.2 samples/op on
E6-1); AVL bulk build is secondary (≈5%); shared block parse largely
cancels; span rebase and coverage build are below the sampling floor. The
certificate lookup cost scales with the **boundary count** (E6-1 with 1
must-change output pays the same ≈104 samples/op as E6-5 with 321), which
is precisely the #98 finding that the full-build tax is
obligation-independent — now localized to a mechanism path.

Sampling units are `samples/op` under one frozen `cycles:u` period
(1 sample = 100 000 cycles) on one pinned core; attribution shares are the
evidence, absolute wall latency remains #98 T-LANE authority.

---

## A. Authority receipt

```text
HOST             = jnhu@192.168.31.75 (hostname E5; Xeon E5-2666 v3, 20 cores)
LIVE_MASTER      = 6d88becbb94993618321bd71803ce95bdc09e454 (matched the
                   expected SHA in the #100 contract; no intervening commits)
EXECUTION_BRANCH = research/100-horse-a-v2-l2-context-profile
EXECUTION_REVISION= the commit carrying this report (documentation/evidence
                   only; the executed code identity for the MECHANISM is
                   master 6d88bec — the L2 crate adds no mechanism code)
TOOLCHAIN        = rustc/cargo 1.97.1 (workspace pin; LLVM 22.1.6)
KERNEL           = Linux 7.2.5-200.fc44.x86_64 (Fedora 44)
CPU              = Intel Xeon E5-2666 v3 @ 2.90GHz (Haswell-EP)
PERF_VERSION     = perf version 7.2.5-200.fc44.x86_64
HOST_TWEAK       = kernel.perf_event_mlock_kb 516 -> 8192 (ring capacity only)
```

Live bodies of #95/#96/#97/#100 were fetched and read before execution;
live #100 agrees with the execution prompt (the prompt's report skeleton
A–M is a superset of the issue's A–J; both are satisfied below). Frozen
L0/L1 verdicts were not re-litigated.

## B. Profiling build receipt

```text
PROFILE_BUILD        = release-l2-profile-v1 (inherits frozen [profile.release];
                       adds debug = 2 ONLY)
EXTRA_PROFILE_FLAGS  = RUSTFLAGS="-C force-frame-pointers=yes"
PROFILE_BUILD_SHA256 = ee9b680085128993507fecbeb9fbbb141d0358490a455af56d051de63592de09
PAIR_IDENTITY        = ONE binary family for P1 and P0, all cells, both repeats
CALLCHAIN_METHOD     = --call-graph fp (frame-pointer lane), perf script --inline
WHY_THIS_METHOD      = the DWARF-unwind attempt was rejected by the quality
                       pilot (see C); FP chains do not truncate, and the
                       debug=2 build lets --inline expand thin-LTO-inlined
                       parser frames with their DWARF names
P-LANE RULE          = this build's wall times are NOT latency evidence
                       (#98 T-LANE stays authoritative); only sampled
                       attribution under the frozen cycles:u contract is used
```

The harness crate `horse-a-v2-l2` (new, diagnostic-only) re-runs the #98
decompose probes verbatim inside a `#[inline(never)]` marker region; cells
come from the sealed `horse-a-v2-diag` manifest byte-for-byte. No frozen
crate's mechanism code changed; the only shared-manifest additions are the
workspace member entry and the new `release-l2-profile-v1` profile
(precedent: #33's `release-sensitivity-lto-off-v1`, #98's diag crate).

## C. Call-chain quality gate

Two-gate history (both recorded in `PROVENANCE.md`):

```text
ATTEMPT 1  existing release binary + --call-graph dwarf
           DWARF/eh_frame unwinding worked, but thin-LTO inlining left
           construction internals unnamed (E6-1/P1: 14 full_build frames vs
           425 entry frames), and with debug=2 the 16 KiB stack dumps
           truncated deep parser chains arm-asymmetrically
           (E6-1/P0: 188/3852 samples kept the entry frame) -> REJECT
           (biased samples are inadmissible for a differential claim)

ATTEMPT 2  release-l2-profile-v1 (debug=2 + force-frame-pointers) + fp
PILOT_CELL            = E6-1 / P1 and E6-1 / P0 (reps 40/100)
REGION_MARKER_FRACTION= 96.8% (P1) / 96.0% (P0) of all samples
RESOLVED_FRACTION     = 99.0% (P1) / 96.2% (P0)  (leaf-unknown 0.97% / 3.77%)
BROKEN_STACK_EVIDENCE = none (FP chains reach the entry frame on every
                        in-region stack; no empty chains in-region)
KERNEL_CONTAMINATION  = 0 kernel frames in-region (cycles:u)
EXPECTED_CONTEXTS_VISIBLE = YES: full_build, parse_region_observed /
                        BlockScanner run/classify (block pass),
                        materialize_one_with_sink / scan_inlines,
                        persist_interior_certificates (+ attach),
                        OwnerSeq bulk_build, CoveragePlan (below floor),
                        shift_spans (below floor), allocator/memmove, drop
                        glue (teardown)
PILOT_VERDICT         = PASS (after the substrate fix; attempt 1 FAIL is
                        part of the record)
```

Campaign adequacy (all twelve collections): region samples 3 979–16 104
(gate ≥ 1000), resolved 95.6–100%, kernel frames 0. The P0-side 3–4%
leaf-unknown on E6-1/E6-5 lands at libc leaves; classification still
attributes those stacks via their surviving caller frames.

## D. Frozen L2 manifest

Exactly the six contract profiles, plus one full independent repeat each
(`MANIFEST.md` has the per-profile artifact receipts):

| # | Cell (frozen #98 identity) | Arm | Ops | Region samples rep1/rep2 |
|---|---|---|---:|---|
| 1 | E6-1-LOSE-DUP-CHANGE | P1_FULL_BUILD_ONLY | 100 | 15 899 / 16 104 |
| 2 | E6-1-LOSE-DUP-CHANGE | P0_H0_FULL_PARSE_ONLY | 100 | 4 047 / 4 041 |
| 3 | E6-5-HIGH-FANOUT-VALUE | P1_FULL_BUILD_ONLY | 100 | 15 713 / 15 899 |
| 4 | E6-5-HIGH-FANOUT-VALUE | P0_H0_FULL_PARSE_ONLY | 100 | 4 155 / 3 979 |
| 5 | E6-6-FENCE-HIDE-DEF | P1_FULL_BUILD_ONLY | 1500 | 14 555 / 14 566 |
| 6 | E6-6-FENCE-HIDE-DEF | P0_H0_FULL_PARSE_ONLY | 1500 | 10 996 / 11 865 |

Repetition model per pair: E6-1/E6-5 = 100 ops (warmup 20), E6-6 = 1500 ops
(warmup 30) — identical within every P1/P0 pair, one frozen sampling period.

## E. Absolute context results (rep1; rep2 agrees — see stability note)

All numbers are `samples/op` (region samples ÷ frozen region ops).
`full_build_other` = inside `full_build` with no more specific frame;
`region_loop_other` = entry/loop frames and runtime frees whose Rust caller
frames are not preserved; `teardown` = the in-region per-iteration drop
(excluded from construction attribution, matching the #98 probe semantics
where the drop is untimed).

**E6-1 / P1** (total 158.99 samples/op):

| Context bucket | samples/op | share |
|---|---:|---:|
| certificate (persist_interior_certificates + attach) | 103.79 | 65.3% |
| owner_materialize (materialize_one_with_sink / scan_inlines) | 28.49 | 17.9% |
| block_parse_shared (parse_region_observed internals) | 9.19 | 5.8% |
| avl_ownerseq (OwnerSeq bulk_build) | 6.18 | 3.9% |
| region_loop_other | 5.44 | 3.4% |
| full_build_other | 3.81 | 2.4% |
| teardown (ReadyDocument drop) | 1.98 | 1.2% |
| coverage (CoveragePlan::build) | 0.08 | 0.05% |
| span_coordinate (shift_spans) | 0.03 | 0.02% |

Dominant path (leaf-side frames): `persist_interior_certificates ←
attach_interior_certificates ← full_build ← l2_p1_build`, leaves
`add / find / eq` over `Iter<RootBlankEvent>` — i.e. the per-boundary
`barriers.iter().filter(|ev| ev.cut == boundary)` scan.

**E6-1 / P0** (total 40.47 samples/op):

| Context bucket | samples/op | share |
|---|---:|---:|
| h0_document_materialize (finish_document_with_sink) | 29.15 | 72.0% |
| block_parse_shared (BlockScanner internals) | 7.53 | 18.6% |
| teardown (NormalizedDocument drop) | 3.52 | 8.7% |
| region_loop_other | 0.27 | 0.7% |

**E6-5 / P1** (total 157.13): certificate 103.72 (66.0%), owner_materialize
26.84 (17.1%), block_parse_shared 9.04 (5.8%), avl_ownerseq 6.30 (4.0%),
region_loop_other 5.30, full_build_other 4.03, teardown 1.74, coverage
0.10, span_coordinate 0.06.

**E6-5 / P0** (total 41.55): h0_document_materialize 30.31 (73.0%),
block_parse_shared 7.76 (18.7%), teardown 3.24 (7.8%), region_loop_other
0.24.

**E6-6 / P1** (total 9.70): block_parse_shared 4.42 (45.6%),
owner_materialize 2.54 (26.2%), certificate 1.30 (13.4%), avl_ownerseq
0.55, region_loop_other 0.42, full_build_other 0.38, teardown 0.08,
coverage 0.006, span_coordinate 0.005.

**E6-6 / P0** (total 7.33): block_parse_shared 4.19 (57.2%),
h0_document_materialize 2.91 (39.7%), teardown 0.21 (2.8%),
region_loop_other 0.03.

Structural reading: on E6-6 the post-edit source's tail is one giant
fenced-code block, so the document has few Owners/boundaries — the
certificate context collapses (1.30/op) and both arms are dominated by the
shared parse of 128 KiB. E6-1/E6-5 build ~2 050 Owners and pay the
certificate lookup in full.

## F. Differential attribution (P1 − P0, samples/op, sign preserved)

From `receipts/differential_rep1.{json,md}` (rep2 in parentheses in the
text below where it differs materially). Normalization: counts ÷ frozen
ops under ONE frozen sampling period — this is a samples/op delta, never a
percentage-of-flamegraph delta.

**E6-1** (totals: P1 158.99, P0 40.47, excess +118.52; rep2 +120.63;
P1/P0 = 3.93 — T-LANE P1/P0 = 3.66):

| Context/responsibility | P1 samples/op | P0 samples/op | Excess | Interpretation |
|---|---:|---:|---:|---|
| certificate (barrier lookup) | 103.79 | 0 | **+103.79** | dominant residual context (87.6% of total excess) |
| avl_ownerseq | 6.18 | 0 | +6.18 | secondary (5.2%) |
| region_loop_other | 5.44 | 0.27 | +5.17 | entry/loop + unattributed runtime frees; small |
| full_build_other | 3.81 | 0 | +3.81 | residual full_build interior, no single owner |
| block_parse_shared | 9.19 | 7.53 | +1.66 | shared parse ≈ cancels (+1.4% of P1) |
| coverage | 0.08 | 0 | +0.08 | below floor |
| span_coordinate | 0.03 | 0 | +0.03 | below floor |
| owner_materialize vs h0_document_materialize | 28.49 | 29.15 | −0.66 | **cancels** — Horse-A payload materialization ≈ H0 document construction |
| teardown | 1.98 | 3.52 | −1.54 | H0's recursive Node drop costs more than ReadyDocument's |

**E6-5** (totals: P1 157.13, P0 41.55, excess +115.58; rep2 +119.21;
P1/P0 = 3.78 — T-LANE 3.82): same structure as E6-1 — certificate
**+103.72** (89.7% of total excess), avl_ownerseq +6.30, full_build_other
+4.03, loop/other +5.06, block parse +1.28, materialization −3.47
(cancels), teardown −1.50, coverage +0.10, span +0.06.

**E6-6** (totals: P1 9.70, P0 7.33, excess +2.37; rep2 +1.80; P1/P0 = 1.32
— T-LANE 1.32): the residual is small and near run noise; its largest
construction components are certificate +1.30, avl +0.55,
full_build_other +0.38, parse +0.24, materialization −0.38. No
E6-6-specific construction context exists; the fence regime's known cost
(the discarded forward parse, #98 H-A boundary) is outside P1/P0 by
construction.

**Validation:** the machine-readable attribution regenerates from the
retained raw profiles via the recorded `perf script → perf_fold.py →
classify.py` commands; independent repeats reproduce bucket order in all
six profiles (top-3 order identical; max bucket delta ≤ 2% of the top
bucket on E6-1/E6-5; E6-6 P0 totals drift ±8% on small counts, which is
why E6-6 construction claims are qualitative).

## G. Cross-cell synthesis

| Excess (samples/op) | E6-1 | E6-5 | E6-6 | reading |
|---|---:|---:|---:|---|
| certificate (barrier lookup) | +103.79 | +103.72 | +1.30 | **stable across E6-1/E6-5**; scales with boundary count, not obligation |
| avl_ownerseq | +6.18 | +6.30 | +0.55 | stable, secondary, boundary-count-proportional |
| full_build_other | +3.81 | +4.03 | +0.38 | small, stable |
| block_parse_shared | +1.66 | +1.28 | +0.24 | cancels on all cells |
| materialization pair | −0.66 | −3.47 | −0.38 | cancels on all cells |
| teardown | −1.54 | −1.50 | −0.13 | H0-side drop slightly heavier |
| coverage / span | ≤ +0.10 | ≤ +0.10 | ≤ +0.01 | below floor everywhere |

```text
STABLE RESIDUAL CONTEXTS          = certificate barrier lookup; AVL bulk
                                    build (secondary); both track the
                                    Owner/boundary count
SEMANTIC-OBLIGATION-SENSITIVE     = NONE found — E6-1 (1 must-change output)
                                    pays the same certificate/op as E6-5
                                    (321 outputs)
E6-6-SPECIFIC CONTEXTS            = none in construction; E6-6's residual is
                                    small because its post document has few
                                    boundaries (fence swallows the tail)
UNEXPECTED CONTEXTS               = the residual owner is certificate
                                    persistence LOOKUP, not certificate
                                    WRITES and not coverage; and Horse-A
                                    materialization is H0-parity rather
                                    than a tax
```

One general full-build construction tax, one dominant interior owner —
not several regime-specific costs.

## H. L2 hypothesis verdicts

```text
L2-H1 MATERIALIZATION_DOMINANT        = NOT_SUPPORTED
L2-H2 AVL_OWNERSEQ_DOMINANT           = NOT_SUPPORTED (present, secondary)
L2-H3 COVERAGE_CERTIFICATE_DOMINANT   = SUPPORTED (certificate persistence;
                                        coverage build itself is below floor)
L2-H4 COORDINATE_SPAN_DOMINANT        = NOT_SUPPORTED
L2-H5 ALLOCATION_RUNTIME_DOMINANT     = NOT_SUPPORTED
L2-H6 DIFFUSE_MULTI_CONTEXT           = NOT_SUPPORTED (E6-1/E6-5)
L2-H7 SHARED_PARSE_RESIDUAL_LARGER_THAN_EXPECTED = NOT_SUPPORTED
```

- **H1 NOT_SUPPORTED.** OBSERVATION: owner materialization 28.49/op (E6-1
  P1) vs H0 document materialization 29.15/op (P0); difference −0.66/op
  (E6-5: 26.84 vs 30.31). INFERENCE: building the Horse-A payload tree
  costs ≈ the same as building the H0 normalized tree for identical
  source. COUNTER_EVIDENCE: none observed; runtime frames were attributed
  into the responsibilities that caused them, so allocation inside
  materialization is counted here. BOUNDARY: shares, not wall-time
  components; payload *representation* maintenance costs outside
  `full_build` are outside L2 scope.
- **H2 NOT_SUPPORTED (secondary).** OBSERVATION: bulk_build 6.18–6.30/op =
  3.9–4.0% of P1, +6.2/+6.3 of excess ≈ 5%. INFERENCE: AVL/OwnerSeq
  construction is real but second-order. BOUNDARY: its cost also scales
  with boundary count (E6-6: 0.55/op).
- **H3 SUPPORTED.** OBSERVATION: +103.79/+103.72 samples/op of certificate
  context on E6-1/E6-5 = 87.6–89.7% of the total P1−P0 excess, all under
  `persist_interior_certificates`'s per-boundary `barriers.iter().filter`
  scan (`certificate.rs:140-149`); CoveragePlan::build itself ≤ 0.13/op.
  INFERENCE: the known 2.7–2.9 ms construction residual is, in calling-
  context space, the certificate barrier lookup. COUNTER_EVIDENCE: none in
  the sampled pairs. BOUNDARY: localization, not causal leverage (§I/§K);
  O(boundaries × barriers) is the observed shape on these cells, not a
  complexity proof.
- **H4 NOT_SUPPORTED.** span rebase ≤ 0.07/op everywhere (below floor).
- **H5 NOT_SUPPORTED.** no standalone runtime context; allocator/memmove
  frames attribute inside the responsibilities; loop/other ≤ 3.4% of P1.
- **H6 NOT_SUPPORTED (E6-1/E6-5).** the excess is concentrated in ONE
  context, not diffuse. E6-6's tiny residual is spread thin, but its
  absolute size (~2 samples/op) is negligible.
- **H7 NOT_SUPPORTED.** shared parse differential +1.28..+1.66/op ≈ 1%
  of P1; parse work is common to both arms, as #98 inferred from equal
  inspection volumes.

## I. Deletion / necessity map update

Only the #98 row `retained-representation construction interior =
UNKNOWN / L2` changes; it splits into evidence-backed sub-rows. Outcomes
stay within the allowed vocabulary; no hotspot is promoted to REPLACE by
heat alone (§22), and no mechanism is designed here.

| Sub-row (of #98's retained-construction interior) | L2 evidence | cost localization | mechanism decision |
|---|---|---|---|
| certificate persistence — barrier lookup (`persist_interior_certificates`) | ≈ 88–90% of the E6-1/E6-5 construction excess; boundary-count-proportional; obligation-independent | **HIGH** | **DEFERRED → #95 Step 2** as the primary SHRINK candidate owner (repair belongs to challenger design, not L2) |
| Owner payload materialization | cancels vs H0 document construction (−0.7..−3.5 samples/op) | HIGH (and ≈ H0 parity) | **KEEP** on construction-cost grounds — the payload tree is not the tax |
| AVL / OwnerSeq bulk build | ≈ 5% of excess | LOW | DEFERRED (secondary SHRINK candidate at most) |
| coordinate/span rebase (`shift_spans`) | ≤ 0.07 samples/op — below floor | ~0 | no change (UNKNOWN, now bounded on these cells) |
| coverage plan build | ≤ 0.13 samples/op — below floor | ~0 | no change |
| full_build interior remainder (`full_build_other` + loop/other) | ≈ 8–9% of excess, no single owner | LOW | UNKNOWN (no evidence to act on) |

All other #98 deletion-map rows are unchanged by L2 (no evidence produced
here touches them).

## J. Main findings (ranked by explanatory importance)

1. **The construction residual has one dominant owner: the certificate
   barrier lookup.** ≈ 88–90% of the P1−P0 excess on E6-1/E6-5 executes in
   `persist_interior_certificates`'s per-boundary linear scan over
   `RootBlankEvent` barriers; the rest of the retained-state pipeline does
   not explain the tax.
2. **Horse-A payload materialization is H0-parity.** Owner materialization
   and H0's document construction cost the same within noise on every
   cell — the "retained-READY construction tax" is NOT the payload
   representation being inherently expensive to build.
3. **The tax is boundary-count-proportional and obligation-independent.**
   E6-1 (1 must-change output) and E6-5 (321) pay the same ≈ 104
   samples/op; E6-6's few-boundary post document collapses the residual to
   the T-LANE's ≈ 63 µs scale. One general construction tax, not
   regime-specific costs.
4. **AVL bulk build is secondary (≈5%); parse, spans, coverage, and
   allocation are non-owners.** Shared parse and materialization cancel;
   span rebase and coverage build sit below the sampling floor.
5. **The profiled ratios reproduce the T-LANE without being timings.**
   P1/P0 = 3.93 / 3.78 / 1.32 vs T-LANE 3.66 / 3.82 / 1.32, and two
   independent repeats reproduce bucket order everywhere — the paired
   attribution is stable evidence, not a sampling accident.

## K. Escalation decision

```text
L2_SUFFICIENT_FOR_MECHANISM_DECISION
```

Rationale: L2 localizes the #98 residual to a specific mechanism
responsibility — the certificate persistence interior (barrier lookup) —
strongly enough that the #95 Step-2 challenger question is now implied by
L1+L2 evidence together: #98 already established the cost exists
(T-LANE P1−P0 = 2.7–2.9 ms, 57–62% of arm B) and is not parser work; L2
establishes where it executes and that it tracks boundaries, not
obligation. That combination is exactly the input #95 Step 2 needs to
choose its ≤ 2 challengers (the #98 Candidate B certification repair and a
construction-interior repair aimed at the localized owner). A causal
virtual-speedup pass (L3) would not change the challenger choice; per
#100's rule, location is recorded as location — the map update in §I
keeps HIGH COST ≠ REPLACE, and every intervention claim remains #95
Step-2/L6 business.

Not executed here, by contract: L3–L5, any candidate implementation, any
mechanism redesign.

## L. Repository receipt

```text
BRANCH            = research/100-horse-a-v2-l2-context-profile
COMMITS           = harness crate + frozen profile + campaign evidence +
                    this report (list fixed at push; see the branch tip)
PR                = opened against master, left OPEN for review (L2
                    evidence only; no L3/L4/L5 work, no mechanism change)
ISSUE_100_UPDATED = YES (execution result posted on the issue)
MERGED            = NO
SEALED_98_EVIDENCE= untouched
```

## M. Final state

```text
L2               = COMPLETE
PROFILE_EVIDENCE = VALID (all 6 manifest profiles + 6 repeats; quality gate
                   PASS; attribution regenerable from retained raw data)
DELETION_MAP     = UPDATED (retained-construction interior split into
                   evidence-backed sub-rows; §I)
NEXT             = #95_STEP2 (challenger selection over the localized
                   owners; L3 not required for that choice)
```

---

### Threats and boundaries

- **P-LANE only.** The profiling build adds frame pointers and debug info;
  sampled shares are attribution units. Absolute P-LANE wall times were
  never compared to T-LANE numbers; only P1/P0 ratios are cross-checked
  (§J.5) and they agree.
- **Sampling, not tracing.** Contexts below the sampling floor
  (span rebase, coverage build) are reported as "not observed", not
  proven zero; their combined ceiling on E6-1/E6-5 is ≲0.2% of P1.
- **`region_loop_other` is a mixed small bucket** (entry/loop frames,
  black_box, runtime frees whose Rust caller frames are not preserved):
  3.4% of E6-1 P1, +5.2/op of excess — reported, not decomposed further.
- **E6-6 construction numbers are qualitative** (small counts: ~2.4
  excess samples/op, rep2 ±0.3); they are used only for the
  absence-of-E6-6-specific-context claim, which both repeats support.
- **The O(boundaries × barriers) reading is an observed shape** on the
  E6 geometry (~2 050 boundaries/barriers), not an algorithmic proof, and
  says nothing about whether a different certificate representation would
  pay elsewhere (next-edit obligations stay with #95 Step 2).
- **Single host, single core, schedutil governor**, as in #98; pairing is
  within-host and the repeats bound the drift.
