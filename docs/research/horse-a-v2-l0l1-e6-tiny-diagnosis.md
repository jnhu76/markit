# Horse-A v2 L0+L1 diagnosis — E6 + tiny same-READY work/effect decomposition

> Issue: [#98](https://github.com/jnhu76/markit/issues/98) (execution contract)
> Parent authority: [#95](https://github.com/jnhu76/markit/issues/95) direction ·
> [#96](https://github.com/jnhu76/markit/issues/96) methodology ·
> [#97](https://github.com/jnhu76/markit/issues/97) umbrella
> Baseline authority: Horse-A v1 (immutable);
> capsule `markit-r0-rq8-research-record-v1.tar.gz`
> SHA256 `156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849`
> Status: **L0 PASS / L1 COMPLETE / L2 NOT_EXECUTED — escalation decision: L2
> (one paired-profile question, §K; to be authorized and designed separately)**
> Machine-readable evidence: `research/benchmarks/markdown-ast-update/results/horse-a-v2-diag-98/`

This is the first Horse-A v2 forensic pass (#95 Step 1). It separates, on a
small frozen diagnostic panel, what work Horse-A v1 performs versus what the
frozen eager semantics actually require, using a validated neutral control
(`HORSE_A_V1_DIRECT_READY_REBUILD`) plus the frozen H0/H2 references.

**Headline:** on the E6 semantic challenges the normal update and a direct
same-READY rebuild cost the same within noise (A/B = 0.993–1.000 on E6-1..5;
at most 0.7% apart). The E6 pathology is **not** duplicate pre-fallback work;
it is dominated by the cost of the full-build route itself — chiefly the
Horse-A retained-state construction (P1−P0 ≈ 2.7–2.9 ms of a ≈ 4.6 ms update,
≈ 57–62% of arm B), plus old-state retirement/validation (≈ 0.75–1.1 ms,
≈ 16–24%). Meanwhile the
frozen ordered-facts certification fires full rebuilds on semantically
irrelevant fact changes (E6-1: **1** output had to change; 2050 owners /
4994 nodes were rebuilt).

---

## A. Authority receipt

```text
HOST            = jnhu@192.168.31.75 (hostname E5; Intel Xeon E5-2666 v3, 20 cores, Haswell-EP)
CHECKOUT        = /home/jnhu/Source/markit
LIVE_MASTER     = a89a4c2723390d903edb1b38543c263e0d042f05 (matched the expected SHA; no intervening commits)
EXECUTION_BRANCH= research/98-horse-a-v2-l0-l1-diagnosis
EXECUTED_CODE_IDENTITY = 6b227b7 (the code state that produced the sealed evidence;
                     at collection start the last commit was 25849a7 and the 6b227b7
                     changes were uncommitted; verified by a bit-identical
                     release-binary rebuild — see PROVENANCE Errata 1/2/3)
FINAL_PR_HEAD   = documentation/audit tip of the execution branch (6f32664 audit
                  errata + closure pass); the exact SHA and the merge commit are
                  recorded in the #98 closure receipt and the post-merge master
                  receipt. Documentation heads do NOT change EXECUTED_CODE_IDENTITY.
AUTHORITY_DRIFT = NONE (live #98 body == prompt contract; #95/#96/#97 titles verified OPEN/consistent)
WORKTREE_STATE  = tracked-clean only after the 6b227b7 commit; at collection time
                  the tree held those changes uncommitted; unrelated untracked
                  #76-era result files left untouched
RUSTC           = 1.97.1 (workspace pin, rust-toolchain.toml; resolves in-workspace)
CARGO           = 1.97.1
LLVM            = 22.1.6 (bundled with rustc 1.97.1; the collector originally
                  captured host-default 1.98.1 / LLVM 22.1.8 strings — see the
                  dated erratum in PROVENANCE.md)
KERNEL          = Linux 7.2.5-200.fc44.x86_64 (Fedora 44)
CPU             = Intel Xeon E5-2666 v3 @ 2.90GHz (Haswell-EP; the v1 PMU host class)
PROFILE         = release (opt-level 3, lto=thin, codegen-units 1, incremental=false,
                  panic=unwind, target-cpu=default, empty RUSTFLAGS; frozen workspace profile)
```

Provenance, exact commands, and artifact SHA-256 sums:
`results/horse-a-v2-diag-98/PROVENANCE.md` + `SHA256SUMS`.

## B. Substrate / responsibility map

Existing frozen substrate was reused everywhere; **no new Horse-A counter was
invented**. The only code added inside a frozen crate is
`horse_a::direct_ready` (diagnostic control; see C) plus two `pub(crate)`
visibility widenings (`validate_association`, `Association`) that change no
behavior and are called by no frozen pipeline path.

| Responsibility | Existing implementation | Existing evidence/oracle | Diagnostic change |
|---|---|---|---|
| update entrypoint | `update::update` / `update_with_structural` | adapter `Mechanism` impl; campaign T-lane | none (arm A runs it verbatim) |
| validation (association) | `update.rs::validate_association` (stage Phase 1) | `UpdateError::InvalidAssociation` | reused BY the control (same fn) |
| damage / Owner location | `locate_damage` → `sequence::locate_by_byte` | `locate_node_visits` | none |
| restart selection / left guard | `select_restart`, `first_replaced_rank` | `safe_predecessor_node_visits`, `restart_old`, `replace_lo` | none |
| forward syntax work | `forward_parse` → shared `parse_region_observed` | `WorkCounters` inspections (CounterSink) | none |
| convergence / candidate rejection | `candidate::CandidateWalk` | `candidate_checks`, `cursor_advances`, `cursor_node_visits` | none |
| coordinate mapping / intervals | `complete_intervals` | `convergence_old/new`, `replace_hi` | none |
| facts extraction / comparison | `facts::OrderedFacts` | `old/new_facts_extracted`, `facts_compared`, `fact_range_node_visits`, `old_fact_owner_visits` | none |
| semantic reuse certification | stage Phase 10 (`facts_equal` decision) | `full_build_selected`, `full_build_reason` | none |
| full-build fallback | `full_build::full_build` | `owners_created`, `bulk_build_node_visits` | reused BY the control (same fn) |
| result/retained-state construction | `fresh::build_replacement_owners` / `full_build` | `fresh_payload_nodes_*`, coverage/certificate counters | none |
| old-state retirement | `retirement::retire_detached/retire_document` inside `PreparedCommit::commit` | `retire_node_visits`, `payload_nodes_retired`, `retirement_frames_entered`, `owners_removed` | reused BY the control (same fns) |
| normalized correctness oracle | oracle `NormalizeV1` (horse-a export) + H0 `parse_document` | `normalized_checksum`, structural equality | none |
| semantic/reference facts authority | `ReadyDocument.refs: RefTable` (ordered first-wins projection) | `RefTable::entries()/resolve()` | compared pre/post in the effect ledger |
| next-edit acceptance contract | unchanged `update()` on the result state | I4/I5 conformance tests | exercised by the 0C next-edit gate |
| allocation/resource counters | instrumentation crate, M-LANE | v1 campaign M-LANE | not re-collected (not needed for these hypotheses) |
| E6 cases | G0-REFDEF transitions; campaign C-F reference-fanout geometry | sealed v1 record §8 | #98 cells built ON the C-F geometry |
| tiny cases | campaign2 Axis::D points; G0 E1/E2/E5 transitions | sealed v1 record §9 | #98 cells at Axis::D sizes with G0 semantics |
| local/container sentinels | G0 list/BQ/E5 transitions | sealed v1 record §7 | #98 sentinel cells (128 KiB) |
| runner/case identity | campaign crate (heavy); h4diag precedent (#50) | frozen campaign2 generators | new small #98 diag crate (h4diag pattern) |

## C. L0 symptom + comparator receipt

```text
FROZEN_E6_SYMPTOM_BOUND  = YES  (sealed v1 §8: Horse-A E6 = 0.583× H0; not re-measured wholesale;
                                  bound here by A/C medians on the new panel: 3.2–3.7× H0 latency,
                                  same direction on same-regime cells)
FROZEN_TINY_SYMPTOM_BOUND= YES  (sealed v1 §9: bottom-decile 0.825×, POST_HOC 0.453×; bound here:
                                  TINY-64B A/C = 1.83, TINY-1K 0.54, TINY-4K 0.45)
DIRECT_READY_CONTROL_ID  = HORSE_A_V1_DIRECT_READY_REBUILD (horse_a::direct_ready::direct_ready_rebuild)
DIRECT_READY_RESULT_PARITY = PASS (13/13 cells: normalized A == B == H0)
DIRECT_READY_SEMANTIC_STATE= PASS (13/13: RefTable projections equal)
DIRECT_READY_NEXT_EDIT    = PASS (13/13: one unchanged v1 edit from EACH result state succeeds;
                                  second-generation results equal)
DIRECT_READY_LIFECYCLE    = PASS (13/13: owners_removed == M_old; retirement frames > 0 inside the
                                  same call boundary; invalid-association refused identically with no
                                  build/retirement work charged)
L0 = PASS
```

The control reuses, unchanged: `validate_association` (stage Phase 1), the
frozen `full_build`, and the `CommitPlan::Full` retirement sequence
(`owners_removed` → `retire_document` → `H_new`). It introduces no new
parser/representation/table/index/lazy state. Validation gates ran in debug
(validate_ready belt active) and again in the release collection session; both
PASS.

## D. Frozen case manifest

`HORSE-A-V2-DIAG-98-CELL-MANIFEST-v1` — 13 cells, IDs/roles/SHA-256 frozen in
`results/horse-a-v2-diag-98/manifest.json` (construction is deterministic and
byte-exact; E6/sentinel documents are 128 KiB on the campaign2 C-F skeleton;
tiny cells sit at frozen Axis::D points).

| Stable ID | Role | Arms |
|---|---|---|
| E6-1-LOSE-DUP-CHANGE | losing duplicate changes; effective binding unchanged | A B C D |
| E6-2-WINNER-DELETE | winning definition deleted; successor takes over | A B C D |
| E6-3A-DEF-CREATE | negative dependency: absence → existence | A B C D |
| E6-3B-DEF-DELETE | inverse leg of E6-3A (same short trace) | A B C D |
| E6-4-LOW-FANOUT-VALUE | effective value change, 10 consumers | A B C D |
| E6-5-HIGH-FANOUT-VALUE | effective value change, 320 consumers | A B C D |
| E6-6-FENCE-HIDE-DEF | syntax edit hides an unchanged definition | A B C D |
| TINY-64B-E1 / TINY-1K-E2 / TINY-4K-E5 | tiny boundary cells (G0 E1/E2/E5 semantics) | A B C |
| SENT-LIST-INDENT / SENT-BQ-NEST / SENT-E5-EMPH | v1 strength guardrails (E3/E3/E5) | A B |

Arms: `A = HORSE_A_V1_NORMAL`, `B = HORSE_A_V1_DIRECT_READY_REBUILD`,
`C = H0_REFERENCE`, `D = H2_REFERENCE`.

## E. Counter-admission table

Counters actually used (all pre-existing surfaces; the two derived quantities
and the two timing probes are computed OUTSIDE the mechanisms):

| Counter/probe | Responsibility | Hypothesis | Exact semantics |
|---|---|---|---|
| `source_bytes_inspected_total`, `unique_post_source_bytes` (WorkCounters, CounterSink) | forward parse vs full-build inspection | H-A | frozen ATTRIBUTION-SCHEMA-v2 derivation; repeated effort = total − union |
| `HORSE-A-STRUCTURAL-COUNTERS-v1` (recording sink) | full update attribution incl. `full_build_selected/reason`, `restart_old`, `convergence_*`, `owners_created`, `fresh_payload_nodes_*`, retirement fields | H-A, H-D | frozen v1 record, charged at the point of work |
| `blocks_reparsed` / `nodes_rebuilt` (H0/H2 arms only) | reference-arm work volume | context | Horse-A arms honestly leave these `Unknown` (frozen adapter policy) |
| effect-ledger signature diff + winner map (offline, two H0 parses) | required effects | H-C, H-D | node multiset diff on (kind+value) signatures; spans excluded (span-only movement counted separately); winner map = first-wins per label |
| `P1_FULL_BUILD_ONLY` / `P0_H0_FULL_PARSE_ONLY` timing probes (decompose.json) | state-construction vs retirement split of arm B | H-B | frozen public APIs only: `full_build(post)` alone; `H0::full_parse(post)+complete` alone; same warmup/round policy as T-LANE |

No other counter was added; the L1 counters could not split B−C (same
inspections, same node counts), which is precisely why the two timing probes
were admitted.

## F. Work ledger

Machine-readable: `results/horse-a-v2-diag-98/alane.json` (+ `tlane.json`,
`decompose.json`). Medians from 200 measured interleaved rounds × 30 warmup,
single process, release profile; IQR/median 0.8–10% (recorded per arm in the
artifacts), far below the 3–10× separations interpreted here.

| Cell | A µs | B µs | C µs | D µs | A/B | A/C | P1 µs | P0 µs | B−P1 µs | P1−P0 µs |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| E6-1 | 4719 | 4722 | 1444 | 756 | 1.00 | 3.27 | 3930 | 1074 | 792 | 2855 |
| E6-2 | 4734 | 4739 | 1463 | 701 | 1.00 | 3.24 | 3913 | 1076 | 826 | 2838 |
| E6-3A | 4595 | 4617 | 1369 | 689 | 1.00 | 3.36 | 3866 | 1038 | 752 | 2827 |
| E6-3B | 4606 | 4623 | 1322 | 527 | 1.00 | 3.48 | 3612 | 765 | 1012 | 2846 |
| E6-4 | 4444 | 4475 | 1205 | 512 | 0.99 | 3.69 | 3399 | 698 | 1076 | 2701 |
| E6-5 | 4739 | 4753 | 1479 | 754 | 1.00 | 3.20 | 3658 | 958 | 1095 | 2700 |
| E6-6 | 999 | 845 | 568 | 435 | 1.18 | 1.76 | 258 | 195 | 588 | 63 |
| TINY-64B | 1.0 | 0.8 | 0.6 | — | 1.25 | 1.83 | 0.5 | 0.4 | 0.3 | 0.1 |
| TINY-1K | 7.3 | 22.0 | 13.5 | — | 0.33 | 0.54 | 13.6 | 9.1 | 8.4 | 4.5 |
| TINY-4K | 42.5 | 135.1 | 93.6 | — | 0.31 | 0.45 | 66.8 | 48.7 | 68.3 | 18.1 |
| SENT-LIST | 971 | 4309 | — | — | 0.23 | — | 3197 | 1031 | 1112 | 2166 |
| SENT-BQ | 613 | 3200 | — | — | 0.19 | — | 2349 | 742 | 851 | 1607 |
| SENT-E5 | 548 | 5465 | — | — | 0.10 | — | 5293 | 1222 | 172 | 4070 |

P1 = Horse-A `full_build(post)` alone; P0 = H0 `full_parse(post)` alone;
B−P1 = validation + old-state retirement; P1−P0 = Horse-A retained-READY
construction over H0 document construction. C−P0 (H0's own old-state drop,
context) = 0.33–0.56 ms on E6.

Repeated/discarded work (E6, from the frozen counters): on E6-1..5 the normal
path's pre-decision work is **128–160 bytes of source inspection, 2–3 candidate
checks, 3 old-fact owner visits, and a ≤1-entry facts comparison** (restart
≈ 8128/10208, convergence ≈ 8288/10368) — arm A inspects 259 424 B vs arm B's
259 264 B (E6-1). On E6-6 (fence, convergence at real EOF) the discarded
forward parse is larger: 112 KB re-inspected inside the subsequent full build
(43% of arm A's inspection effort; A−B wall = 154 µs ≈ 15% of A, 18% above B).

## G. Required-effect ledger

Offline oracle (two independent H0 parses + node-signature multiset diff +
first-wins winner map); spans excluded from "changed" (span-only movement
reported separately, 0 on all cells except the structurally edited region).

| Cell | facts Δ | winner changed | existence Δ | Text↔RefLink | must-change outputs | owners/nodes rebuilt (A) |
|---|---|---|---|---|---:|---:|
| E6-1 | 1 value | NO | — | — | **1** | 2050 / 4994 |
| E6-2 | def removed | YES (successor) | — | — | 321 | 2049 / 4993 |
| E6-3A | def added | — | unresolved→resolved | 575 Text→320 RefLink (+re-segmentation) | 897 | 2049 / 4993 |
| E6-3B | def removed | — | resolved→unresolved | 576 RefLink→Text | 897 | 2048 / 4096 |
| E6-4 | 1 value | YES | — | — | **11** | 2049 / 4125 |
| E6-5 | 1 value | YES | — | — | 321 | 2049 / 4993 |
| E6-6 | def hidden (syntax) | — | resolved→unresolved | 320 RefLink→Text; 1792 paras + 2368 Text → 1 FencedCode | 4482 | 257 / 513 |
| TINY-1K / TINY-4K / sentinels | — | — | — | — | 1–3 | 1–2 owners (local path) |

## H. Hypothesis verdicts

```text
H-A_DUPLICATE_PATH_WORK      = NOT_SUPPORTED (E6 semantic regime) — with a bounded fence exception
H-B_READY_CONSTRUCTION_TAX   = SUPPORTED
H-C_UNAVOIDABLE_SEMANTIC_WORK= SUPPORTED (strongly cell-dependent)
H-D_CERTIFICATION_DEFICIENCY = SUPPORTED (refusal reason: FactsDiffer on a semantically
                               irrelevant replacement-region fact)
H-TINY_FALLBACK_OPPORTUNITY  = NOT_SUPPORTED (for this tested scope)
```

### H-A — duplicate/path-selection work: NOT_SUPPORTED (E6 semantic regime)

- OBSERVATION: E6-1..5 arm A vs arm B: wall medians equal within 0.7%
  (IQR/median 1.9–8.6%); pre-decision work 128–160 B inspection, 2–3 candidate
  checks, ≤1-entry facts comparison; both arms then run the identical full
  build (equal `owners_created`, `fresh_payload_nodes`, retirement frames).
- INFERENCE: on value/duplicate/delete/negative-dependency definition edits,
  the incremental attempt before the full-build branch is not a material loss
  source; the E6 loss is the full-build route itself.
- COUNTER_EVIDENCE / boundary: E6-6 (fence-induced, convergence only at real
  EOF) does discard a real forward parse: 112 KB re-inspected (43% of A's
  inspection), A−B = 154 µs (15% of A, 18% above B). The fence/propagation regime retains
  moderate duplicate work — consistent with the sealed W-A1 propagation
  finding, not with E6-value edits.
- MECHANISM IMPLICATION: "stop the incremental attempt earlier / skip it on
  E6" would recover ≈0–2% on E6-1..5 and ≈15% on the fence variant. Candidate
  A (earlier rebuild) is a minor lever for the E6 semantic problem; it is not
  the pathology.

### H-B — READY construction/lifecycle tax: SUPPORTED

- OBSERVATION: P1−P0 = 2.70–2.86 ms on every E6-1..5 cell (P1/P0 =
  3.6–4.9×), i.e. 57–62% of arm B's ≈ 4.5–4.7 ms; B−P1 (validation +
  old-state retirement) = 0.75–1.10 ms (16–24%); H0's own old-state drop
  (C−P0) is 0.33–0.56 ms for context. Inspection volumes are equal across
  B/C (259 KB both) and payload-node counts are equal (≈ 4993 both), so the
  delta is state construction + heavier retirement, not parser work.
- INFERENCE: producing an equivalent next-edit-ready Horse-A state is itself
  materially more expensive than producing an H0 document — on these cells the
  single largest component of the E6 pathology. The tax is NOT the whole
  story (retirement is a distinguishable second component), so `B − C` must
  not be quoted as a pure state tax.
- COUNTER_EVIDENCE / boundary: the probes do not split the P1−P0 interior
  (per-owner materialization, span rebasing, AVL bulk build,
  coverage/certificates, per-owner allocation); that split is the L2 question
  below. Tiny cells show the same tax at small scale (P1/P0 = 1.3–1.5×) —
  size-proportional, mechanism-inherent to v1's representation.
- MECHANISM IMPLICATION: any v2 path that still enters whole-document
  rebuilds keeps this cost; the retained representation's construction cost
  is now a first-class deletion-map row, not an assumed constant.

### H-C — unavoidable semantic work: SUPPORTED (strongly cell-dependent)

- OBSERVATION: E6-2/3A/3B/5 require 321–897 output changes; E6-6 requires
  4482 (the fence legitimately destroys/rebuilds the whole tail). E6-1
  requires exactly 1 (the loser definition node) and E6-4 exactly 11.
- INFERENCE: under frozen eager semantics a material fraction of E6 work is
  genuine obligation — but only on effective-change cells. On losing-duplicate
  and low-fanout cells the obligation is 0.02–0.27% of what was rebuilt.
- COUNTER_EVIDENCE / boundary: obligation ≠ optimality; changed outputs
  establish necessity of the change, not of a whole-document rebuild.
- MECHANISM IMPLICATION: coarser rebuild is defensible for high-fanout /
  existence-change / fence cells; targeted invalidation is defensible for
  ineffective-fact and low-fanout cells. Both must exist in any v2 story.

### H-D — reuse-certification deficiency: SUPPORTED

- OBSERVATION: all seven E6 legs take the full-build branch
  (`full_build_selected = 1`, `full_build_reason = 1` FactsDiffer). On E6-1
  the differing ordered fact is the LOSING duplicate's destination — a fact
  that cannot change any consumer's effective first-wins binding; 1 output
  had to change, 2050 owners / 4994 nodes were rebuilt. E6-4: 11 outputs vs
  2049/4125.
- INFERENCE: the frozen W-A2 rule (ordered replacement-facts equality)
  certifies preservation far below the effective-binding level; the concrete
  refusal condition on these cells is **facts differ**, firing on
  semantically irrelevant fact changes.
- COUNTER_EVIDENCE / boundary: certification repair alone does not remove
  H-B's construction tax where full builds remain legitimate (E6-2/5/6), and
  H-C obligations still stand. This verdict names the refusal reason; it does
  NOT authorize a reverse dependency index (sparse-set discovery cost is
  unmeasured here).
- MECHANISM IMPLICATION: the minimum repair candidate (#95 Candidate B —
  effective binding/value certification) is evidence-backed as the first
  challenger: it would convert E6-1/E6-4-class updates into local splices.

### H-TINY — legal fallback opportunity: NOT_SUPPORTED (this tested scope)

- OBSERVATION: TINY-64B: A/B = 1.25 (B 0.84 µs vs A 1.05 µs; B sub-µs, A ≈ 1 µs);
  TINY-1K: A/B = 0.33; TINY-4K: A/B = 0.31. A direct rebuild beats the normal
  path only at 64 B by an absolute 0.2 µs, and even there B remains ≈ 1.5× H0.
- INFERENCE: in the tested tiny regime a legal direct same-READY rebuild does
  NOT materially beat normal Horse-A update; where Horse-A loses to H0
  (TINY-64B), the cause is the same construction tax (P1/P0 = 1.3×, and B
  still builds Horse-A state), not path selection.
- COUNTER_EVIDENCE / boundary: these are paragraph-shaped synthetic cells at
  frozen Axis::D points; the sealed v1 bottom-decile cells were real-project
  files. The verdict is scoped to "this tested scope": tiny-selector research
  (predicting incremental-vs-rebuild) is stopped for it.
- MECHANISM IMPLICATION: per #95 §11, stop tiny-selector research for this
  scope; the tiny residual is a construction-tax problem, not a selector
  problem.

## I. Initial deletion / necessity map

Evidence-backed rows only (others remain `UNKNOWN` until earned):

| Responsibility / component | Obligation | Evidence / measured cost | Decision |
|---|---|---|---|
| source/edit validation | correctness | O(1), part of B−P1 | KEEP (challenge nothing yet) |
| damage locate / restart selection / left guard | correctness (restart certificates) | E6 pre-decision work 128–160 B / 2–3 checks | KEEP |
| forward syntax work + convergence | correctness | E6-1..5 immaterial; E6-6 112 KB discarded (154 µs ≈ 15% of A) | SHRINK candidate (fence/EOF convergence regime only — earned by E6-6, not by E6 semantic edits) |
| facts extraction/comparison | feeds W-A2 decision | 3 owner visits, ≤1-entry compares | SHRINK (its RESULT triggers whole-document rebuilds; see certification) |
| semantic reuse certification (W-A2 ordered-facts equality) | correctness-sensitive | fires FactsDiffer on semantically irrelevant facts (E6-1: 1/4994) | REPLACE with effective-binding/value certification (Candidate B; must carry the same eager-READY obligations incl. negative dependency) |
| full-build fallback | correctness/READY safety | the E6 cost center: dominates E6-1..5 update cost (construction 57–62% + retirement 16–24% + the shared full parse) | REFRAME — keep as safety; its cost profile (H-B) now gates how often v2 may enter it |
| retained representation construction (Owner payload + AVL + coverage + certificates) | READY representation | P1−P0 = 2.7–2.9 ms (57–62% of arm B; P1/P0 3.6–4.9×) | UNKNOWN→L2: interior split required before SHRINK/REPLACE can be decided |
| old-state retirement | lifecycle | 0.75–1.1 ms on E6 (heavier than H0's 0.33–0.56 ms drop) | SHRINK candidate (proportional to state size; representation-dependent) |
| payload reuse / local splice | correctness + v1 strength | sentinels: A beats B 4.4–10×; tiny 1K/4K 3× | KEEP (preserve-v1 guardrail confirmed) |
| tiny selector | none (was a research idea) | H-TINY NOT_SUPPORTED | DELETE the research branch for this scope |

## J. Main findings (ranked by mechanism-decision importance)

1. **The E6 pathology is a construction problem, not a path-selection
   problem.** Normal update ≈ direct rebuild (within 0.7%) on E6-1..5; the
   loss vs H0 (3.2–3.7×) lives inside the full-build route.
2. **Horse-A READY-state construction is 3.6–4.9× the cost of H0's document
   construction** for identical parser work and identical semantic-node
   counts (P1−P0 = 2.7–2.9 ms of ≈ 4.6 ms updates). This tax also explains
   the tiny residual (B never reaches H0 even at 64 B).
3. **The frozen ordered-facts certification over-fires**: E6-1 (losing
   duplicate; 1 output must change) and E6-4 (11 outputs) still rebuild
   2049–2050 owners / 4125–4994 nodes. Refusal reason: FactsDiffer on an
   ineffective fact.
4. **Obligation is strongly bimodal**: 321–4482 must-change outputs on
   effective-change/fence cells vs 1–11 on ineffective/low-fanout cells —
   v2 needs both targeted invalidation and an affordable coarse rebuild.
5. **v1 strengths are intact under the same-READY contract**: sentinels show
   normal update 4.4–10× faster than direct rebuild; tiny 1K/4K 3× faster.
   Any v2 selector that rebuilds eagerly would destroy these.

## K. Escalation decision

```text
ESCALATE_TO_L2_CONTEXT_PATH_PROFILING
```

```text
UNRESOLVED_RESPONSIBILITY =
    the full-build construction-route residual inside Horse-A full_build
    (the P1−P0 quantity). Candidate internal owners — per-Owner payload
    materialization, span rebasing, AVL bulk build, coverage/certificate
    construction, allocation, other full_build-only contexts — remain
    UNRESOLVED at L1; the list is a hypothesis set for L2, not a claim.

WHAT_L1_ESTABLISHED =
    P1_FULL_BUILD_ONLY materially exceeds P0_H0_PARSE_ONLY (2.7–2.9 ms,
    57–62% of arm B; P1/P0 = 3.6–4.9×), is size-proportional, and is not
    explained by parser work (equal inspections / equal semantic-node
    counts) or by pre-fallback path-selection work (H-A: A/B = 0.993–1.000
    on E6-1..5)

WHY_L1_CANNOT_RESOLVE_IT =
    the frozen counters attribute whole responsibility quantities
    (owners_created, fresh_payload_nodes, bulk_build visits, retirement
    frames) but do not localize cycle ownership among full_build
    sub-contexts, and doing so by ablation would require modifying the
    frozen v1 mechanism

EXACT_L2_QUESTION =
    which calling contexts account for the P1−P0 full-build construction
    residual inside full_build(post) on E6-1, E6-5 and E6-6?

MINIMUM_PROFILE_NEEDED =
    PAIRED context-sensitive (call-graph-aware) sample profiles for
    P1_FULL_BUILD_ONLY and P0_H0_PARSE_ONLY on the same selected cells,
    host, toolchain, profile, source identity and profiling protocol,
    followed by differential attribution of the residual. A profile of
    P1 alone is NOT sufficient: the target is a difference. No new
    counters, no mechanism modification, no broad PMU reopening, no
    cache/TLB hypothesis yet.
```

L2 is NOT performed in #98. The L2 outcome decides the deletion-map row
"retained representation construction" (SHRINK implementation cost vs REPLACE
representation) and therefore which second challenger (#95 Step 2) is honest.

## L. Repository / GitHub receipt

```text
BRANCH = research/98-horse-a-v2-l0-l1-diagnosis
EXECUTED_CODE_IDENTITY = 6b227b7 (produced every sealed artifact; immutable)
COMMITS= 25849a7 (crate + control + validation) · 6b227b7 (report + evidence) ·
         6f32664 (audit errata) · + this closure pass (documentation only)
PR     = #99 into master — L0/L1 evidence only, no L2 work
ISSUE_98_UPDATED = YES (L0/L1 result posted; closure receipt posted on merge)
MERGED = YES (closure pass); FINAL_PR_HEAD and MERGE_COMMIT are recorded exactly
         in the #98 closure receipt and the post-merge master receipt commit
```

## M. Final state

```text
L0 = PASS
L1 = COMPLETE
L2 = NOT_EXECUTED
DELETION_MAP = INITIALIZED
NEXT = L2 (paired-profile question per §K; to be designed, authorized,
       and executed separately — not in this record)
```

---

### Threats and boundaries

- Small diagnostic panel (13 cells, single host, single process, 200 reps):
  supports the comparator relationships quoted, not population-wide
  frequency claims; the sealed v1 campaign remains the population authority.
- B−P1 mixes validation with retirement (both small; retirement dominates by
  construction order); C−P0 gives the H0-side drop context, not an equality.
- The effect ledger is an OFFLINE diagnostic oracle (two H0 parses + diff);
  none of its knowledge is available to a mechanism for free.
- E6-6's structural blast radius is by design (fence swallows the tail); its
  obligation count (4482) must not be read as typical for definition edits.
- The two `pub(crate)` widenings and `horse_a::direct_ready` are #98
  diagnostic additions; no frozen pipeline path calls them, and Horse-A v1's
  mechanism identity is unchanged (arm A ran the untouched pipeline).
