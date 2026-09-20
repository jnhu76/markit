# CORRECTIVE-B-REPORT-1 — profile and select the frozen real Markdown workload

```text
task            MARKIT-WORKLOAD-PR-SETTLEMENT-AND-CORRECTIVE-B-1 (stage B)
corrective      CORRECTIVE-B-PROFILE-SELECT-v1 (issue #35)
generator       CORRECTIVE-B-PROFILE-SELECT-v1 / mdbench-corrective-b
                tool sha256 91426defcb42e61407d4c3e9d9142080d6d4a0070e5201a722164db54f171ecb
                (manifest-relative source digest; verifiable from any checkout)
verdict         CORRECTIVE_B_PROFILE_SELECT_PASS
stop point      Draft PR — human review; CORRECTIVE-B is NOT merged by this task
```

## Run header

```text
BASE_SHA         473c3e74f384ac10cbe75582596219db4d664de1 (settled master, post stage-0)
HEAD_SHA         the Draft PR head commit (this report is part of that commit)
BRANCH           research/35-corrective-b-profile-select-1
PR               Draft: "research: profile and select frozen real Markdown workload"
WORKTREE STATUS  clean at the PR head (committed: crate, tests, artifacts,
                 contract inputs, docs; local-only derived data gitignored)
```

## PR settlement (stage 0)

All three PRs were verified independently (offline test suites, fail-closed
replay, `verify --full`, artifact determinism, workspace tests) before
merging; each merge re-ran its gates rather than trusting the branch report.

```text
PR #34  MARKIT-REAL-WORKLOAD-ACQUISITION-1 (acquisition authority)
        merge d5f97a0b2153de581ad1311155c9365e71380b21          PASS
PR #36  R6 real-workload authority amendment
        merge 5a581a4728e9ffd2684f51808e3c64eac5d2f2ed          PASS
PR #37  CORRECTIVE-A semantic substrate (lanes/profiler/oracle/lifecycle)
        rebased onto master (diff contains only CORRECTIVE-A), then merged:
        merge 473c3e74f384ac10cbe75582596219db4d664de1          PASS
settled master = 473c3e74f384ac10cbe75582596219db4d664de1 = BASE_SHA
checkpoint comment posted to issue #35 after the stage-0 merges
```

## Corrections found and fixed during this stage

Both were found while assembling this report's evidence, fixed before the
adversarial review, and re-validated end to end.

1. **Syntax-inventory evidence-grade mislabel.** `syntax_inventory()` derived
   each target's grade from its owning lane id alone, stamping
   `strikethrough`, `task_list_item`, `front_matter` and `directive` as
   `strict_lane_coverage` under G0 — but
   `grammar/grammar-lanes-v1.json` marks all four `out_of_lane` under G0
   **and** disabled under G1. The occurrence counts were always honest
   (0 recognized); only the grade string lied. Fixed: the grade now mirrors
   the frozen `construct_scopes` table; regression test
   `syntax_inventory_grades_mirror_frozen_construct_scopes` added.
2. **Fenced-content interval ignored container prefixes.**
   `fenced_content_interval()` (the G1 data source for
   `fenced_code_content_bytes` and fenced non-host spans, and a G0 debug
   guard) failed to recognize a **quoted closing fence** (`> ``` `) because
   it only stripped spaces. A debug-build `debug_assert_eq!` in `g0.rs`
   caught the disagreement (5-byte drift on the first affected file; a
   release run compiled the check out, which is why earlier runs passed).
   A corpus-wide probe found all 60 affected fences — every one inside a
   blockquote (rust-rfcs, mystmd). The G0 parser's stored field was correct;
   the derived rule was fixed to mirror the oracle state machines (closer =
   fence-char run ≥ opener run preceded only by container-prefix bytes),
   corpus-validated at 0 mismatches across all 3,970 files, with fixture
   tests (`fenced_content_interval_handles_container_prefixed_closers`).

Consequence: G1 structural facts changed slightly for 13 quoted-fence
files; across this checkout's runs the selection membership and all set
sizes were unchanged by the fix; determinism was re-verified after the fix
(below).

## Candidate universe

```text
files                     3,970
bytes                    66,391,919
sources                       20
materialization/hash          3,970 / 3,970 materialized, 0 missing,
verification                  0 hash mismatches (sha256 + byte size,
                              fail-closed; analysis/candidate-universe-verification-v1.json)
```

Universe = the merged PR #34 selected snapshot, byte-exact at the pins.
Derived data is local-only and hash-bound
(`distributions-v1.json → derived_data_binding`):

```text
profiles/real-profile-v1.jsonl     1,255,213,034 bytes
                                   sha256 cfc3854500e0829240422894b903417ac83a52640a6503f371db2af206550816
profiles/candidate-rows-v1.jsonl   reduced per-candidate rows (gitignored)
```

## Profiling

```text
profiler version       REAL-MARKDOWN-PROFILER-v1 (owned contract; not reimplemented)
lanes                  G0 BENCH-GRAMMAR-v1
                       G1 COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1
                       G2 MARKIT-EXT-MATH-v1 (identity reserved, deferred)
profiles generated     3,970 rows (every candidate profiled under all three lanes)
profile failures       0
ambiguous/unknown      10,076 ambiguous + 6 unknown host-context facts
                       (recorded, never resolved silently, never coverage)
```

Four fact classes stay separate (SOURCE / SYNTAX / STRUCTURAL /
ELIGIBILITY). Non-recognition is never read back as absence: candidate
evidence is recorded with its grade, and non-host context occurrences are
counted separately from host syntax everywhere (§14 inventory split).

## Eligibility

### G0 — BENCH-GRAMMAR-v1 (12 frozen constructs are the only strict scope)

```text
strict eligible     530 files /   2,872,622 bytes  (13.4% of files, 4.3% of bytes)
ineligible        3,440 files /  63,519,297 bytes   (blocker-bearing)
top blocker kinds (host-context out-of-lane candidates)
  strong 24,104 · html_block 19,464 · raw_html_inline 11,813 ·
  inline_math 8,880 · task_list_item 8,124 · directive 7,082 ·
  code_block_indented 3,660 · table 1,999 · display_math 1,213 · image 1,127
```

### G1 — CommonMark 0.31.2 + GFM tables (qualified scope = tables ONLY)

Stated per the actual qualified scope, without overclaiming: only the table
construct is semantically qualified in this corrective; every recognized
CommonMark base kind is `contract_declared_not_qualified` and is never
counted as strict coverage.

```text
strict-scope clean  1,995 files / 27,654,304 bytes (no G1-scope blocker present)
blocker-bearing     1,975 files
G1 blockers         only math (lane-deferred) + disabled GFM extensions
                    (strikethrough, task list items) + front matter + directives
                    block scope cleanliness; rejected base-construct probes
                    (e.g. unmatched emphasis runs) are ordinary text and do not.
```

### G2 — deferred

`lane_valid = 0/3,970`. No eligibility claim exists or may be inferred.

### Per-project eligibility

```text
source_id               domain                    cand   g0-strict  g1-scope-clean
cpp-core-guidelines     LARGE_GUIDE_REFERENCE       1        0          1
crafting-interpreters   OTHER_DECLARED             50       32         50
cs231n                  ML_SCIENTIFIC_TECHNICAL    30        1          2
d2l-en                  ML_SCIENTIFIC_TECHNICAL   191       20         44
ethereum-eips           RFC_PROPOSAL_DESIGN       956        0          0
graphql-spec            STANDARD_SPECIFICATION     15        1          7
kubernetes-keps         RFC_PROPOSAL_DESIGN       678       12        147
myst-parser             API_TECHNICAL_DOC          27        7          8
mystmd                  API_TECHNICAL_DOC          89        1          2
node                    API_TECHNICAL_DOC          70        0         11
oci-image               STANDARD_SPECIFICATION     18        5         16
oci-runtime             STANDARD_SPECIFICATION     21        1         19
openapi                 STANDARD_SPECIFICATION     23       11         11
openmlsys               ML_SCIENTIFIC_TECHNICAL   234       90        167
opentelemetry-spec      STANDARD_SPECIFICATION     99        8         95
owasp-cheatsheets       SECURITY_OPERATIONAL_DOC  122       14        113
progit                  OTHER_DECLARED              3        2          3
rust-book               BOOK_TUTORIAL             112       17        111
rust-rfcs               RFC_PROPOSAL_DESIGN       651      307        620
swift-evolution         RFC_PROPOSAL_DESIGN       580        1        568
```

Notable: `ethereum-eips` is 0-strict under **both** lanes (YAML front matter
on essentially every EIP) — the largest single acquisition source
contributes zero strict-eligible files. G1 scope-cleanliness is much wider
than G0 strictness, but its only *strict* evidence remains tables.

## Bias (mandatory eligibility-bias report)

Full artifact: `analysis/eligibility-bias-v1.json` +
`analysis/ELIGIBILITY-BIAS-REPORT-v1.md` (9 feature dimensions, each with
universe vs G0-strict distributions).

### Project / domain distribution

Per-project candidate counts and domains are in the table above; per-domain
candidate counts: RFC_PROPOSAL_DESIGN 2,865 · ML_SCIENTIFIC_TECHNICAL 455 ·
API_TECHNICAL_DOC 186 · STANDARD_SPECIFICATION 176 · BOOK_TUTORIAL 112 ·
SECURITY_OPERATIONAL_DOC 122 · OTHER_DECLARED 53 · LARGE_GUIDE_REFERENCE 1
(`CJK_TECHNICAL` is a declared stratum with no source in the sampling
frame — reported, not hidden).

### Proposal/RFC concentration

```text
sources      ethereum-eips, kubernetes-keps, rust-rfcs, swift-evolution
candidates   2,865 files = 72.17% of candidates (46,858,602 bytes = 70.58%)
G0-eligible  60.38% of the strict-eligible population
selected     representative-core share 22.22% (4 of 18); realism share 38.89%
```

The concentration survives eligibility and is explicitly re-measured after
selection; the domain/project caps hold it to 4 of 18 core slots.

### Candidate → eligible → selected distortion

| domain | candidate share | G0-eligible share | selected core | selected realism |
|---|---|---|---|---|
| RFC_PROPOSAL_DESIGN | 72.17% | 60.38% | 22.22% | 38.89% |
| ML_SCIENTIFIC_TECHNICAL | 11.46% | 20.94% | 22.22% | 22.22% |
| OTHER_DECLARED | 1.34% | 6.42% | 16.67% | 0.00% |
| STANDARD_SPECIFICATION | 4.43% | 4.91% | 16.67% | 11.11% |
| API_TECHNICAL_DOC | 4.69% | 1.51% | 5.56% | 11.11% |
| BOOK_TUTORIAL | 2.82% | 3.21% | 11.11% | 5.56% |
| SECURITY_OPERATIONAL_DOC | 3.07% | 2.64% | 5.56% | 5.56% |
| LARGE_GUIDE_REFERENCE | 0.03% | 0.00% | 0.00% | 5.56% |

The distortion is reported, not hidden: G0 eligibility shrinks the corpus
13.4× and re-weights it (RFC share drops, small-prose domains rise), and
the selection deliberately over-weights under-represented domains relative
to their candidate share.

## Redundancy

Method `SHINGLE-W5-MINHASH64-JACCARD-v1`: word 5-gram shingles, 64
deterministic sha256-derived permutations (no seed, no embeddings), pair
threshold 0.75, union-find grouping. **Diagnostic only** — no candidate is
removed anywhere; groups feed tie-break information and this report.

```text
exact duplicate groups   3 (all openapi editors variants)
  065c3bd8… ×3 (3.0.2/3.0.3/3.1.0-editors) · 73cf20f0… ×2 (3.0.4/3.1.1-editors)
  fc88736c… ×3 (3.1.2/3.2.0/3.2.1-editors); exact groups cross-checked
  against the acquisition registry (disagreement is a hard error)
near-duplicate groups    10 (largest: eip-3041/3044/3045/3046 @ 0.734;
  oci-image/oci-runtime GOVERNANCE @ 0.938 cross-project;
  openapi 3.0.0–3.0.3 @ 0.891)
selection impact         tie-break input only (RepKey near-dup flags);
  exactly-duplicated byte content never occupies two selected slots
```

## Selected sets

All four logical sets are outputs of the frozen `SELECTION-CONTRACT-v1`
executed by `selection::select`; the complete decision trace is
`selections/selection-trace-v1.jsonl`. Freeze-order evidence: the contract
was frozen in the working tree before the first selection ran (its mtime
precedes every selection output), its content matches the executed code and
`selection-config-v1.json`, and its sha256 is bound into every artifact's
identity block — the §48 reviewer mechanically recomputed all three
full-document winners, the seventh document, the caps, and every
representative tie-break from current sources and reproduced the recorded
outputs.

```text
REPRESENTATIVE_SET   18 logical files  (target 20–30; UNDER_TARGET_DUE_TO_COVERAGE_PLATEAU:
                     greedy stopped at 18 — every attainable mandatory cell covered,
                     no compliant candidate covers a new cell; plateau reported, not padded)
EXTREMAL_SET         12 logical files  (8 OBSERVED_MAXIMUM + 7 TAIL_REPLICATE roles;
                     the 7 replicates live on 6 files — node/fs carries two —
                     cjk_byte_share replicate NONE_AVAILABLE — no second CJK project exists)
SYNTAX_COVERAGE_SET   2 logical files
FULL_DOCUMENT_SET     7 logical files
UNIQUE PHYSICAL      36 files
```

Cross-set physical overlap (logical memberships may share a file):
REPRESENTATIVE∩EXTREMAL 1 · EXTREMAL∩FULL_DOCUMENT 2 · all other pairs 0.

Mechanical-relaxation events: **none** (`cap_relaxations = []`; no project
or domain cap ever needed relaxation). Full-document rejections: **none**.
Seventh full document: mechanical rule triggered by the uncovered
BOOK_TUTORIAL regime (largest-bytes choice), not hand-picked.

### Why every selected file was selected

```text
cpp-core-guidelines/files/CppCoreGuidelines.md        extremal: file_bytes+block_count OBSERVED_MAXIMUM;
                                                      full_document: FULL-CPP-CORE named hard candidate (G0 no / G1 clean)
crafting-interpreters/…/chapter01_introduction/1.md   representative: 13 new mandatory cells + OTHER_DECLARED stratum
crafting-interpreters/…/chapter13_inheritance/1.md    representative: 1 new mandatory cell
crafting-interpreters/…/chapter20_hash/1.md           representative: 1 new mandatory cell
d2l-en/…/chapter_appendix-tools-for-deep-learning/utils.md
                                                      representative: 6 new cells + ML stratum; extremal: code_occupancy TAIL_REPLICATE
d2l-en/…/chapter_generative-adversarial-networks/index.md
                                                      extremal: fence_density TAIL_REPLICATE
d2l-en/…/chapter_installation/index.md                representative: 1 new mandatory cell
d2l-en/…/chapter_preliminaries/linear-algebra.md      full_document: FULL-D2L-LINEAR-ALGEBRA named hard candidate
ethereum-eips/files/EIPS/eip-3198.md                  syntax_coverage: 1 new syntax/context cell at candidate grade only
                                                      (labels REALISM_ONLY + GRAMMAR_EXTENSION_REQUIRED — never strict)
ethereum-eips/files/EIPS/eip-7643.md                  extremal: largest_block_bytes + code_occupancy OBSERVED_MAXIMUM
kubernetes-keps/…/sig-api-machinery/1040-…/README.md  full_document: FULL-KEP equal-weight percentile-rank formula
kubernetes-keps/files/keps/sig-cli/FAQ.md             representative: 1 new mandatory cell
kubernetes-keps/…/sig-cluster-lifecycle/wgs/783-…/README.md
                                                      extremal: max_container_depth OBSERVED_MAXIMUM
myst-parser/files/docs/develop/_changelog.md          extremal: fence_density OBSERVED_MAXIMUM
myst-parser/files/docs/develop/contributing.md        representative: 3 new cells + API_TECHNICAL_DOC stratum
node/files/doc/api/fs.md                              extremal: file_bytes+block_count TAIL_REPLICATE;
                                                      full_document: FULL-NODE-FS named hard candidate
oci-image/files/HACKING.md                            representative: 1 new mandatory cell
oci-image/files/considerations.md                     representative: 11 new cells + STANDARD_SPECIFICATION stratum
oci-image/files/implementations.md                    representative: 1 new mandatory cell
oci-runtime/files/implementations.md                  extremal: reference_density OBSERVED_MAXIMUM
openmlsys/…/chapter_accelerator/accelerator_practise.md
                                                      full_document: FULL-OPENMLSYS percentile formula incl. CJK + math as
                                                      occurrence-only candidate evidence (G2 deferred — no fabricated occupancy)
openmlsys/…/chapter_accelerator/index.md              representative: 3 new mandatory cells
openmlsys/…/chapter_recommender_system/index.md       representative: 4 new mandatory cells
openmlsys/…/chapter_rl_sys/summary.md                 extremal: cjk_byte_share OBSERVED_MAXIMUM
opentelemetry-spec/files/spec-compliance-matrix.md    syntax_coverage: 1 new syntax/context cell at strict grade (G1 table)
owasp-cheatsheets/…/Email_Validation_and_Verification_Cheat_Sheet.md
                                                      representative: 5 new cells + SECURITY stratum
owasp-cheatsheets/…/Kubernetes_Security_Cheat_Sheet.md
                                                      extremal: largest_block_bytes TAIL_REPLICATE
rust-book/files/src/appendix-05-editions.md           representative: 5 new cells + BOOK_TUTORIAL stratum
rust-book/files/src/appendix-06-translation.md        representative: 2 new mandatory cells
rust-book/files/src/ch02-00-guessing-game-tutorial.md full_document: mechanical 7th (uncovered BOOK_TUTORIAL regime, largest bytes)
rust-rfcs/files/text/0404-change-prefer-dynamic.md    representative: 2 new mandatory cells
rust-rfcs/files/text/1123-str-split-at.md             representative: 13 new cells + RFC_PROPOSAL_DESIGN stratum
rust-rfcs/files/text/3289-source_replacement_ambiguity.md
                                                      representative: 3 new mandatory cells
rust-rfcs/files/text/3672-Project-Goals-2024h2.md     extremal: reference_density TAIL_REPLICATE
rust-rfcs/files/text/3935-Project-Goals-2026.md       full_document: FULL-RUST-RFC equal-weight percentile-rank formula
swift-evolution/files/proposals/0509-swift-sboms-via-swiftpm.md
                                                      extremal: max_container_depth TAIL_REPLICATE
```

## Coverage

Feature cells (8 features × ZERO/tertile bins over the G0-strict-eligible
population; `selections/coverage-v1.json`):

```text
28 feature cells       every cell with eligible > 0 has selected > 0 (no silent zeroes);
                       smallest selected margin: 1 file (cjk=MEDIUM and cjk=HIGH)
4 mandatory joint cells (file_bytes×block_count, file_bytes×largest_block,
  fence_density×code_occupancy, file_bytes×reference_density)  all covered;
  the triggered optional fifth (depth×block_count) also covered — per-cell
  winners are in selections/selection-trace-v1.jsonl (63 remaining cells → 0)
domain coverage        all 7 populated strata have ≥1 selected file
                       (LARGE_GUIDE_REFERENCE via extremal/full-document); CJK_TECHNICAL empty by frame
```

Syntax/context cells (33 cells, 26-target inventory with
recognized/candidate/ambiguous/unknown/non-host split):

```text
strict (G0 12-kind set + G1 table)   covered, e.g. paragraph 326,527 recognized
                                     occurrences over 3,969 files; table 618 files
declared-not-qualified (G1 base)     covered at declared grade, never counted strict
candidate-only (strikethrough, task_list_item, front_matter, directive,
  escaped-pipe)                      covered at candidate grade, labeled as realism only
ambiguous_or_unknown (math inline/display candidates)  covered as occurrence evidence
                                     ONLY, labeled LANE_DEFERRED (G2 owns the semantics)
```

Extreme-role coverage: all 8 dimension maxima selected; replicates selected
for 7 of 8 dimensions (distinct project in the top 1%); `cjk_byte_share`
replicate is `NONE_AVAILABLE` (reported as an uncovered-space entry).
Interpretation caveat (recorded, not hidden): the per-KiB density maxima
(`fence_density`, and code-occupancy tails) sit on small genuine MyST
`{include}` stub files (76–89 bytes) — real bytes, contract-frozen rules;
they are extremal facts about the corpus, not about typical documents.

Remaining uncovered space (`UNCOVERED-WORKLOAD-SPACE-v1.md`) — reported as
a **result**, not padded away:

```text
syntax_observed_lane_deferred   math inline/display semantics are G2's; candidate evidence only
domain_missing_from_sampling_frame  CJK_TECHNICAL: no acquisition source maps to it
extreme_without_cross_project_replicate  cjk_byte_share NONE_AVAILABLE
ambiguous_unknown_facts         10,076 ambiguous + 6 unknown host-context facts exist;
                                they block strict scope and are never counted as coverage
```

## Spot checks (13/13 all pass)

Re-profiled samples with ≤12 facts each, sha256 re-verified per file:
ordinary G0-strict prose (rust-rfcs/2476) · fence-heavy (myst-parser
changelog) · reference-heavy (oci-runtime implementations) · deep
container (keps 783) · CJK (openmlsys summary) · recognized G1 table
(openapi 3.1.1) · table-looking text inside a fence (mystmd tables) ·
raw-HTML-bearing (node quic) · image/autolink-bearing (owasp k8s) ·
math-candidate (d2l raden) · G0-ineligible large doc (CppCoreGuidelines) ·
exact-duplicate group (openapi editors) · near-duplicate group (eip-3041).

## Determinism, identity, and tests

```text
determinism (§44)   DETERMINISM PASS — 19 artifacts byte-identical across
                    two clean runs (fresh profiling both times)
artifact identity   every artifact carries: generator version + tool sha256,
                    candidate-manifest sha256 f8a9917b…, source-lock sha256
                    7884c61b…, domain-strata sha256 814c17be…,
                    selection-config sha256 259f0cb3…, profiler/lane/
                    contract versions
tests               cargo test --workspace: 288 passed / 0 failed across 58
                    binaries; includes 18 profile-select contract tests
                    (bins/collapse, hash binding, caps, tie-breaks, evidence
                    grades, dedup, MinHash determinism, fail-closed universe
                    loading, no-horse dependency guard, selection stability,
                    percentile tie-stability, span-evidence rejection) and
                    the new inventory-grade + fence-content regressions
performance-blind   selection crate deps are exactly semantics/serde/
                    serde_json/sha2 (test-enforced); no horse, runner,
                    timing, or mechanism code exists on this path; no
                    frequency model — ordering ladders are in the contract
hard guard          no edit_start/edit_end/inserted_text/post_source_sha256/
                    payload_id/CaseId fields anywhere in outputs (grep clean);
                    the transition taxonomy stays with CORRECTIVE-C — only
                    boolean transition_opportunity flags are carried
```

## Scope statement

This corrective explicitly does **not** deliver:

```text
NO final edit payload freeze
NO REAL_WORKLOAD_FREEZE_PASS
NO CORE_REAL_WORKLOAD_FREEZE_PASS
NO H0-H4 timing
NO #31 performance execution
```

PASS means only: the frozen 3,970-candidate acquisition universe has been
mechanism-neutrally profiled, its eligibility/bias/redundancy are
understood, and the four logical workload file sets have been selected
deterministically with a complete selection trace. Everything downstream —
transition taxonomy, canonical edits, BREAK/RESTORE pairs, payload
identity, runner load, and the workload freeze review — belongs to
CORRECTIVE-C and the final G1–G8 review.

## Adversarial review (§48)

A fresh-context reviewer (no part in building this stage, read-only)
answered all 20 required questions before the stage commit. Method:
full read of the crate + semantics profiler + contracts + all 19 artifacts;
independent recompute of bins, cells, caps, winners, hashes and every
headline number from candidate-rows-v1.jsonl; `verify` PASS; both crate
suites green; a non-writing `run --dry` reproduced the recorded profile
JSONL sha256 (cfc3854500e0…) from current sources.

```text
Q1–Q20  all SOUND          REVIEW: no MAJOR findings
MINOR   3 findings — all fixed before the verdict (report number errors,
        freeze-order wording, checkout-stable generator_tool_sha256)
record  workloads/REVIEW-CORRECTIVE-B-ADVERSARIAL-v1.md
```

## Verdict

```text
CORRECTIVE_B_PROFILE_SELECT_PASS
```

This task stops at the Draft PR. CORRECTIVE-B is not merged automatically,
and CORRECTIVE-C is not started.
