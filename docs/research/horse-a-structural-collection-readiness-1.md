# HORSE-A-STRUCTURAL-COLLECTION-READINESS-1 — pre-treatment readiness record

```text
STATUS                             = PRE-TREATMENT READINESS RECORD
STUDY                              = HORSE-A-STRUCTURAL-LOCALITY-1
STRUCTURAL_COLLECTION_RUN          = NO
PERFORMANCE_COLLECTION_RUN         = NO
STRUCTURAL_COLLECTION_AUTHORIZED   = (recorded separately on issue #60;
                                      this document does not authorize)
PERFORMANCE_COLLECTION_AUTHORIZED  = NO
TREATMENT_OBSERVATIONS_CONSULTED   = NONE (none exists)
```

This document freezes the pre-treatment facts a future structural-collection
task must obey. It is not a treatment observation: it contains no counter
value produced by a recording-mode run of any primary cell, no timing, no
PMU data, no allocator economics. The only implementation-side diagnostic
quoted anywhere below is the correctness-only fact-range value in §6, which
is explicitly non-decision-bearing and was never used to set a bound. Its only numeric content is mechanical transcription of
already merged authority (`docs/research/horse-a-v1-algorithm.md` §16 as
corrected by merged `ACCOUNTING-CORRECTION-1` / PR #72) and the frozen
workload identities.

Authority for the experiment itself remains issue
[#60](https://github.com/jnhu76/markit/issues/60) (structural-locality
falsification contract, authority-synchronized 2026-09-28) and the durable
spec `docs/research/horse-a-v1-algorithm.md`. If this document and #60
disagree, #60 wins.

---

## 1. Reviewed baseline

```text
MASTER_SHA    = 8e40932239273cc798795a625244ffb3e12b5f0e
MASTER_TREE   = 265bfc75af9b5b32ce76cfc72ce20df0a81f3756
IDENTITY      = "Merge pull request #74 from jnhu76/test/horse-a-c123-closure"
                ("test(horse-a): close C1 C2 C3 correctness gates (#62)")

ISSUE_62      = CLOSED / completed (2026-09-28T01:50:48Z)
ISSUE_60      = CLOSED / completed (2026-09-25T10:40:29Z), body
                authority-synchronized 2026-09-28T02:12:11Z
```

#62 closure evidence (its final comment) records, on all three primary cells:
`C1 = PASS`, `C2 = PASS`, `C3 = PASS`, `C3_SAME_RETURNED_STATE = YES`,
`FULL_NORMALIZED_STRUCTURAL_EQUALITY = PASS`,
`READY_CONTINUATION = PASS`, `IMPLEMENTATION_CONFORMANCE = PASS`,
`MECHANISM_IDENTITY_CHANGED = NO`, `COUNTER_SCHEMA_CHANGED = NO`,
`STRUCTURAL_COLLECTION_RUN = NO`.

## 2. Frozen mechanism identity

```text
MECHANISM            = HORSE-A v1 (frozen by #55 / merged PR #54)
FROZEN DESIGN HEAD   = d44f32345674f1e1db88fb832adb40e291da0120
MERGE COMMIT         = 3b9ff484e5da73f3f4510afdbcc1f3bc3d877a48
IMPLEMENTATION       = mechanisms/horse-a (markit-mdbench-horse-a)
IMPLEMENTATION STATE = complete for the frozen mechanism; #62 CLOSED
W-A1 / W-A2 / W-A3   = preserved (root-only restart + left guard /
                       facts-differ-or-unknown -> same-target full build /
                       one Owner per AVL node)
```

No collection lane, treatment registration, or performance measurement
exists in the crate, by design (`mechanisms/horse-a/src/lib.rs`: "any #60
treatment registration or collection lane … later slices").

## 3. StudyId

```text
StudyId = 2e061da9cb6fbe57f9ce139ca02382fda4cad672fd9422e6c7d7b885f42f4e17
```

Derivation (the repository's canonical form: `hex(SHA256(preimage))` over
newline-separated frozen fields, mirroring `campaign2::identity::study_id`;
no result value is an input). Reproduce exactly:

```bash
cat > /tmp/horse-a-studyid-preimage.txt <<'EOF'
HORSE-A-STRUCTURAL-LOCALITY-1
master=8e40932239273cc798795a625244ffb3e12b5f0e
tree=265bfc75af9b5b32ce76cfc72ce20df0a81f3756
mechanism=HORSE-A-v1
mechanism_design_head=d44f32345674f1e1db88fb832adb40e291da0120
mechanism_merge=3b9ff484e5da73f3f4510afdbcc1f3bc3d877a48
counter_schema=HORSE-A-STRUCTURAL-COUNTERS-v1
threshold_authority=ACCOUNTING-CORRECTION-1:PR-72:04b6496:horse-a-v1-algorithm.md-16
raw_schema=HORSE-A-STRUCTURAL-RAW-v1
execution=resident-single-reset,one-break-insertion,3-independent-process-invocations
cell=H4N-128KiB,case=22e51081397aebb06a1c4c3f85bd08a6e7041a438f4aae416970027e1ddbb24e,pre=58b39c389bfe5d5cbcfdbbd74150d276c7996e1b25a84a467fa4deb3cec29a30,post=1ca6bb1f138507758230ba868c6a55bb9bc824b94fe0b9c75ff3d39c8fb777ea,start=65599,end=65599,inserted=c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429
cell=H4N-1MiB,case=337a179ec729c20c01309d98317f7ca00691d12ca10285b55ade71fb880e191c,pre=681c2c032bd1585deef29cf2ce9f9c2a3b8c4bf163ca54c8001d45309462b294,post=e3e0d2ddf4717cc9bbd0e60848ada7b542500d143134b39609cd2537da8c174c,start=524351,end=524351,inserted=c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429
cell=H4N-16MiB,case=0c00f8138e0e622f6fc7215fc0f0d135e79bf60450e4c79d30e30624a8842117,pre=0fdca64e71df4386d4407afa1dd5e72aa0faaf60d6854a1fab730d580be6c78d,post=c8014b5ecae8ca489cb083e6a1a1f8170b3fd2d8d115a85ffa4c58ab4bb29eb0,start=8388671,end=8388671,inserted=c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429
EOF
sha256sum /tmp/horse-a-studyid-preimage.txt
# 2e061da9cb6fbe57f9ce139ca02382fda4cad672fd9422e6c7d7b885f42f4e17
```

It binds: protocol/study identity, reviewed repository revision, mechanism
identity, counter schema, threshold authority, raw-row schema, execution
policy and the three canonical workload identities. It must not be derived
from result values and must not change after treatment exposure.

## 4. Primary cells and edit identity (exact)

The three primary cells are the frozen #50 / Campaign-2 controlled-N cells,
anchored by the tracked catalog
`research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/cells.jsonl`
(git blob `98d04bc9eb6f4f11c7da8fc92aa9c62568d78c65`). Construction:
`filler(126, b) + "\n\n"` per 128-byte block (`markit_mdbench_corpusgen::filler`,
in-repo and deterministic), edit = insertion of `"zzzzzzzz"` at
`target * 128 + 63`.

| cell | N | M | target | edit_start = edit_end | case_id_hex |
|---|---:|---:|---:|---:|---|
| H4N-128KiB | 131,072 | 1,024 | 512 | 65,599 | `22e51081…dbb24e` |
| H4N-1MiB | 1,048,576 | 8,192 | 4,096 | 524,351 | `337a179e…0e191c` |
| H4N-16MiB | 16,777,216 | 131,072 | 65,536 | 8,388,671 | `0c00f813…842117` |

Byte-identity gates (full values in §3 / #60 §3):

```text
inserted_text_sha256 = c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429
128KiB pre/post = 58b39c38…c29a30 / 1ca6bb1f…b777ea
1MiB   pre/post = 681c2c03…462b294 / e3e0d2dd…7da8c174c
16MiB  pre/post = 0fdca64e…be6c78d / c8014b5e…4bb29eb0
```

Canonical edit identity = (cell identity, pre_sha256, post_sha256, start,
end, inserted_text_sha256). No prose-equivalent replacement workload is
acceptable without byte-identity proof.

## 5. Frozen geometry (pre-result static derivation)

```text
Δ_old = 2, Δ_new = 2, candidate_checks Q = 2, cursor k_crossed = 2
owners_created = 2, owners_removed = 2, fresh_payload_nodes_final = 4
P_removed = 4, D_payload = 2, ordered facts old/new = [] / []
full_build_selected = false, convergence before EOF = true

cuts            128KiB      1MiB        16MiB
restart          65,408      524,160     8,388,480
q_old            65,664      524,416     8,388,736
q_new            65,672      524,424     8,388,744
replace_lo/hi    511/513     4,095/4,097 65,535/65,537

H_max(M)         14          18          24     (legal bound; h(root) > H_max
                                                is an implementation invariant
                                                failure, never a larger budget)
```

## 6. Corrected threshold authority (active)

`ACCOUNTING-CORRECTION-1` (#71 / PR #72 @ `04b6496`) as reflected in the
durable spec §16 and transcribed into the #60 §9.5 table. Changed rows:

| counter | frozen formula | 128 KiB | 1 MiB | 16 MiB |
|---|---|---:|---:|---:|
| fact_range_node_visits | ≤ H + Δ_old(H−1) = 3H − 2 | 40 | 52 | 70 |
| join_node_visits (×2) | ≤ 8H − 2 | 110 | 142 | 190 |
| f2 visits | ≤ 36H − 4 | 500 | 644 | 860 |
| avl_rotations | ≤ 18H − 4 | 248 | 320 | 428 |
| sequence_link_writes | ≤ 72H − 3 | 1,005 | 1,293 | 1,725 |
| aggregate_reads | ≤ 406H − 34 | 5,650 | 7,274 | 9,710 |
| aggregate_writes | ≤ 168H − 24 | 2,328 | 3,000 | 4,008 |

All other rows, the exact call counts, certificate `≤ 5 / ≤ 2`, retirement
rows, exact `Known` values and the FAIL/PASS/result-to-action rules are
unchanged from #60. Superseded pre-correction values exist only as labelled
provenance (`horse-a-accounting-correction-1.md`; #60 §9.2/§9.5/§22) and are
never execution authority.

```text
NO_THRESHOLD_DERIVED_FROM_FROZEN60_OBSERVED_VALUES = YES
(no bound was tightened or loosened from any observed implementation value;
 correctness-only diagnostics such as the observed fact_range_node_visits
 19/25/33 were never substituted for the 40/52/70 authority)
```

## 7. Counter schema

```text
COUNTER_SCHEMA = HORSE-A-STRUCTURAL-COUNTERS-v1
```

Emitted by `mechanisms/horse-a/src/structural.rs` (`HorseAStructuralCountersV1`),
charged at the point of work through `HorseAStructuralSink`; the recording
lane and the no-op lane execute the identical algorithm and differ only in
sink identity. Every decision-bearing field is `Observed::Unknown` or
`Observed::Known(n)`; the first charge turns `Unknown` into `Known`, and
overflow demotes a field back to `Unknown`, so a missing or overflowed
counter can never satisfy a PASS gate. All eight forbidden sentinels are real
fields charged at defended mechanism sites:
`forbidden_prefix_sequential_enumeration`,
`forbidden_suffix_sequential_enumeration`,
`forbidden_unaffected_payload_inspections`,
`forbidden_unaffected_coordinate_writes`,
`forbidden_unaffected_certificate_writes`,
`forbidden_global_fact_recollection`,
`forbidden_unaffected_old_retirement`,
`forbidden_attribution_tree_walk` — PASS requires `Known(0)`.

## 8. Producer / build identity policy

> Status note (pre-treatment, same PR as the producer): the producer NOW
> EXISTS as `mechanisms/horse-a` bin `mdbench-horse-a-structural`
> (module `producer`), implementing exactly the policy and recipe frozen
> below; every other word of this section is unchanged from the frozen
> record. The policy below remains binding on it.

The producer policy and recipe it must satisfy were frozen by this
record before implementation; no raw treatment row may be produced
before its executable identity is captured.

```text
REQUIRED BINARY  markit-mdbench-horse-a bin (new), e.g.
                 mdbench-horse-a-structural
BUILD COMMAND    cargo build --release -p markit-mdbench-horse-a \
                     --bin mdbench-horse-a-structural
TOOLCHAIN        rust-toolchain.toml pin: 1.97.1
                 (rustc 1.97.1 (8bab26f4f 2026-07-14);
                  cargo 1.97.1 (c980f4866 2026-06-30))
PROFILE          release-primary-v1 (workspace [profile.release]:
                 opt-level 3, lto "thin", codegen-units 1,
                 incremental false, panic "unwind")
FEATURES         none
TARGET_CPU       default; RUSTFLAGS empty
ALLOCATOR        rust system default (no custom allocator)
```

Producer requirements (binding, testable):

```text
- the recording window is exactly stage/prepare/commit of ONE update from a
  freshly constructed pre-edit READY state (full_build and every oracle /
  export / restore step are outside the recording window and must use the
  no-op sink);
- no external materialization is required: the pre source is constructed
  from markit_mdbench_corpusgen::filler and admitted only if its sha256 and
  the post sha256 equal the frozen §4 values;
- exactly 3 independent process invocations per primary cell, each with a
  fresh ready-state construction and the same source/edit identities;
- raw rows are append-only: a written row is never rewritten; a failed or
  inconvenient cell is recorded, never deleted.
```

Producer identity captured with every raw row / attribution file:

```text
repository commit SHA + tree SHA
executable SHA256 (sha256sum of the built binary)
rustc/cargo version strings + toolchain channel
build profile id + features
host identity (OS kernel, CPU model, architecture)
counter schema version
StudyId (§3) and protocol identity
exact execution command
```

## 9. Raw evidence schema

```text
RAW_SCHEMA = HORSE-A-STRUCTURAL-RAW-v1
```

One JSON object per (cell, repetition). Required fields (unknown values stay
explicit, never omitted):

```text
study_id                  (§3 StudyId)
protocol                  "HORSE-A-FAILURE-FIRST-1 / #60"
frozen_contract_revision  (pre-treatment P2-4 extension — see below)
study_mechanism_baseline  (pre-treatment P2-2 extension — see below)
authorization_baseline    (pre-treatment P2-2 extension — see below)
repository_commit, repository_tree
mechanism                 "HORSE-A_v1"
mechanism_design_head, mechanism_merge
cell_id                   H4N-128KiB | H4N-1MiB | H4N-16MiB
case_id_hex, n_bytes, m, target, edit_start, edit_end
inserted_text_sha256, pre_sha256, post_sha256
repetition                0 | 1 | 2
executable_sha256, rustc, cargo, toolchain_channel
build_profile, features, host, execution_command
counter_schema            "HORSE-A-STRUCTURAL-COUNTERS-v1"
counters                  every field of HorseAStructuralCountersV1, each
                          "Known(n)" or "Unknown" — never silently dropped
route                     local | same_target_full_build
                          full_build_reason: none | facts_differ | preservation_unknown
correctness               c1/c2/c3 status + normalized-structural-equality
                          boolean (the oracle decision; checksums are
                          provenance only)
result_checksum           oracle/result checksum (provenance, not the oracle)
```

### 9.1 Pre-treatment schema extensions (P2-2 / P2-4 of the #60 authorization record)

Frozen BEFORE the first decision-bearing raw row (no treatment
observation existed when these values were fixed; they are never derived
from results and never changed after the first row):

```text
frozen_contract_revision
  = HORSE-A-FAILURE-FIRST-1/#60@sync-20260928T021211Z
    /master-f7fdcdaabc5d761435f3c0e8c17611973642934b
    /tree-9ec58ed228972c323da5cccf0b991235a0bc0e8a

  Identifies the synchronized #60 contract authority (the
  authority-synchronized issue body, 2026-09-28T02:12:11Z, as reviewed by
  the authorization record at that master/tree) DISTINCTLY from the
  StudyId (a content hash), from the PR #72 accounting authority
  (04b6496…) and from the repository execution commit (which changes
  when the producer lands).

study_mechanism_baseline
  = 8e40932239273cc798795a625244ffb3e12b5f0e
  (the StudyId-bound reviewed mechanism revision, §1 MASTER_SHA)

authorization_baseline
  = f7fdcdaabc5d761435f3c0e8c17611973642934b
  (the authorization master, §1; differs from the study baseline by this
   readiness record only)

repository_commit / repository_tree
  (the ACTUAL execution revision — the merged producer commit — captured
   per row; distinct from both baselines above)
```

The external collection schedule's 1-based repetition index
(`--repetition 1|2|3`) maps to the schema repetition as `value − 1`
(recorded 0 | 1 | 2).

Raw rows are immutable; derived tables must be regenerable from them.

## 10. Repetition policy (frozen)

```text
3 independent process invocations per primary cell
fresh ready-state construction in each invocation
same exact source/edit identities

exact agreement required across repetitions for:
  normalized result equality
  every structural counter
  the branch/fallback reason

counters are NEVER averaged; differing counters mean NO STRUCTURAL PASS
until explained. H0/H4 context work may use the same deterministic policy.
No timing schedule exists in this study.
```

## 11. Phase boundary (frozen)

```text
INCLUDED in primary structural observation
  prepare/stage attribution + native update (commit) + retirement
  (retirement required before READY completion stays inside)

EXCLUDED
  fresh initial ready-state construction (full_build)
  oracle normalization / export
  the C3 restore probe
  final document destruction
```

The executable instrumentation can enforce this because the recording sink is
threaded through `stage -> prepare -> commit` only, while `full_build`, the
C3 restore and every oracle export take the no-op sink.

## 12. PASS/FAIL and result-to-action rules

Unchanged and owned by #60 (§13 FAIL rules, §14 PASS rules, §15 determinism,
§19 result-to-action). Summary of the action rules:

```text
incorrect C1/C2/C3              -> no authorization / implementation defect
mechanism non-conformance       -> implementation defect (fix implementation)
conforming Horse-A structural FAIL -> reject the first structural-locality
                                      claim; do NOT rescue with A-R / B / A-P
structural PASS                 -> only then authorize a separate
                                      economics / broad-regime study
```

Wall-clock performance cannot rescue a structural FAIL, and this study
adjudicates no latency. `full_build_selected = true` on this fixed local
witness is itself a structural FAIL; it may never be reinterpreted as a legal
alternate route after seeing results.

## 13. Instrumentation preflight (recorded 2026-09-28, pre-treatment)

Verified statically on `8e409322…` and by the merged conformance tests
(no treatment rows produced):

```text
point-of-work charging          counters are charged by the operator that
                                performs the work; no post-hoc derivation
                                from tree shapes or result walks
post-frontier policy            commit() is infallible, returns no Result,
                                takes no WorkSink (no source inspection can
                                be recorded after the frontier), allocates
                                nothing (bounded pre-reserved stacks; the
                                allocation-denial guard in frozen60 asserts
                                0 attempts post-frontier)
workspace reservation           logical limit derived and try_reserve_exact
                                performed in staging, before the frontier;
                                refusal is an ordinary pre-frontier error and
                                the old READY state survives it
fact-range charging             initial rank seek + successor-walk nodes
                                entering the fact cursor stack, routed to
                                fact_range_node_visits — never the
                                convergence cursor
operator ledger                 single rotation = 1 unit / 3 link writes;
                                double = 2 units / 6; same-frame
                                take/reinstall = 1 logical write (L1);
                                root: take uncharged, final installation = 1
                                write
forbidden sentinels             all eight charged Known(0) at their defended
                                sites; PASS fails on Unknown
deferred retirement             retirement stays inside commit (no semantic
                                completion after READY)
```

Known non-blocking build observation (P3, recorded not repaired): a plain
`cargo build --release -p markit-mdbench-horse-a` (no test cfg) emits one
`unused_imports` warning for `use crate::validate;` in `full_build.rs`,
because that validator call is intentionally `#[cfg(debug_assertions)]`-gated
("validators are test/debug gates only"). `cargo clippy --all-targets --
-D warnings` is clean and no test, counter, oracle or mechanism behaviour
differs. It is left unrepaired because touching the crate would move the
reviewed baseline; a fix belongs to a separately reviewed change.

Conformance evidence on the frozen master: `cargo test -p
markit-mdbench-horse-a` (debug: 142 lib + 78 integration tests PASS; release:
141 lib + 78 integration tests PASS), including `i5_tests::frozen60` (C1–C5
per cell) and `i5_tests::c123` (the unbroken C1→C2→C3 ownership chain,
debug 3/3 and release 3/3). The one-test debug/release difference is the
`#[cfg(debug_assertions)]`-gated structural test. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` PASS on the package.

## 14. Environment: existing workspace failures — classification

#62 recorded workspace-only environmental failures. Reevaluated here for
collection relevance:

```text
OBSERVED (full workspace run, --no-fail-fast, on the reviewed master)

  exactly three test binaries fail, all outside the Horse-A crate:

  1. markit-mdbench-campaign --lib
     preflight::tests::real_scopes_enumerate_the_frozen_campaign_cardinalities
     preflight::tests::timing_scope_rejects_a_session_outside_the_frozen_count
     root cause: materialized corpus missing, e.g.
     workloads/sources/cpp-core-guidelines/files/CppCoreGuidelines.md

  2. markit-mdbench-campaign --test campaign_freeze  (7 tests)
     frozen_workload_consumes_exactly_22_and_362,
     campaign_manifest_verifies_against_live_state,
     schedule_is_deterministic_and_verifies,
     campaign_receipt_binds_every_artifact,
     preflight_passes_and_enumerates_unique_observation_ids,
     attribution_counters_are_deterministic_on_representative_cases,
     non_research_fake_clock_smoke_end_to_end
     root causes — TWO, independently present:
       (a) corpus materialization (the frozen G0-strict clean_state/
           edit_write workload loads real source bytes) — the SOLE cause in
           frozen_workload_consumes_exactly_22_and_362,
           campaign_manifest_verifies_against_live_state,
           schedule_is_deterministic_and_verifies and
           attribution_counters_are_deterministic_on_representative_cases;
       (b) stale receipt-bound artifact hashes: the frozen campaign receipt
           binds Cargo.lock = e42e1a91… / Cargo.toml = 3ad241ff… while the
           tracked master files hash to b78e67b8… / ce1531f7… — present
           TOGETHER WITH (a) in campaign_receipt_binds_every_artifact,
           preflight_passes_and_enumerates_unique_observation_ids and
           non_research_fake_clock_smoke_end_to_end, which fail on both
           causes at once; materializing the corpus alone will NOT clear
           those three.

  3. markit-mdbench-semantics --test contracts
     committed_pilot_artifacts_match_the_driver
     root cause: stale workloads/pilots/semantic-pilot-results-v1.json
     ("is stale; re-run ... mdbench-semantic-pilot --write")

  These are a documented storage state, not corruption: candidate-stage
  source bytes are deliberately NOT tracked in Git (workloads/.gitignore) and
  are deterministically reconstructed with `python3 tools/acquire.py
  materialize` and verified against the hash-pinned SOURCE.json manifests.
  Every Horse-A test target passes in the same run (debug and release).

RELEVANCE TO #60
  COLLECTION_IRRELEVANT.

  Reason: the #60 primary cells are synthetic controlled-N constructions that
  do not come from the real-project corpus. Their pre sources are built by
  `filler(126, b) + "\n\n"` and are admitted only by sha256 equality with the
  frozen pre/post hashes; the frozen catalog cells.jsonl is tracked in Git.
  The Horse-A crate's dependency closure is
  markit-mdbench-{common,oracle,shared-grammar,corpusgen} — none of which
  reads workloads/sources/*/files/, the campaign preflight, or the pilot JSON.
  (corpusgen is currently a `[dev-dependencies]` entry; the producer bin must
  promote it to a normal dependency, which stays inside the same closure.
  The promotion is part of implementing the producer, and the producer must
  still be built and identified under the §8 policy.)

BINDING CONDITION
  The collection producer must stay inside that closure. A future producer
  that routes through the campaign preflight or requires the real-project
  corpus is out of contract and must not be used for this study.
```

Note: #62's closure recorded the same three environment-only failures as a
non-blocking P2. This record does not inherit that classification by
inheritance: it is re-derived here from the actual failing targets and their
root causes against the #60 primary-cell path, and the binding condition
above is what keeps it `COLLECTION_IRRELEVANT`.

## 15. Fresh-clone execution path

From a clean checkout of `8e409322…` (or the merge commit of this record's
PR, which must be recorded when it lands):

```text
1. git clone / checkout the recorded commit; toolchain auto-pins via
   research/benchmarks/markdown-ast-update/rust-toolchain.toml (1.97.1)
2. cd research/benchmarks/markdown-ast-update
3. cargo build --release -p markit-mdbench-horse-a --bin <producer>
   (release-primary-v1 profile; no features; no RUSTFLAGS)
4. sha256sum the built binary -> executable_sha256
5. the producer constructs each frozen pre source in-process, verifies
   pre/post/inserted sha256 against §4, and only then runs the recording
   window
6. raw rows are written under
   results/horse-a-structural-locality-1/raw/<cell_id>-rep<N>.json
   with the executable attribution of §8 archived beside them
   (results/horse-a-structural-locality-1/EXECUTABLE-ATTRIBUTION.txt)
7. adjudication applies #60 §14 against §6; derived tables are regenerated
   from the raw rows
```

No undocumented local filesystem dependency is required. No corpus
materialization step is part of this path.

## 16. What this record does not do

```text
STRUCTURAL_COLLECTION_RUN          = NO
PERFORMANCE_COLLECTION_RUN         = NO
```

No treatment cell was executed in recording mode to produce this record; no
structural counter value of the three primary cells appears here; no
threshold was chosen, tightened or loosened from any observation of the
implementation's behaviour. Authorization to run the collection is recorded
separately on #60 by the independent authorization review.
