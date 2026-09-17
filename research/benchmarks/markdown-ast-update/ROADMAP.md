# MARKIT-MARKDOWN-BENCHMARK-1 — 实验 Roadmap

Status: **R3 FREEZE READY_FOR_ADVERSARIAL_R3_REVIEW (2026-09-17; R2 merged
via PR #27, master c9bad01)**

Authority: GitHub Issue #22

- Branch: `research/22-markdown-benchmark-1` (R0/R1) /
  `research/22-prior-art-mechanism-extraction-1` (R2) /
  `research/22-r3-grammar-corpus-mutation-freeze-1` (R3)
- R0 methodology: `protocol/R0-METHODOLOGY.md`
- R1 harness contract: `protocol/R1-HARNESS-CONTRACT.md`

本文件只规定执行顺序、阶段交付物与 Gate。实验方法学以 Issue #22 + `protocol/R0-METHODOLOGY.md` 为权威。

核心纪律：

```text
protocol
-> controlled Rust substrate
-> shared grammar/result contract
-> prior-art mechanism extraction
-> H0 control
-> H1-H4 mechanism implementations
-> correctness
-> microbench surfaces
-> scaling/work counters
-> attribution
-> optimization-sensitivity check
-> replication
-> strength/weakness profiles
-> Weakness Map
-> human review
```

禁止：看到单点 timing 差异就直接提出 Markit algorithm。

## R0 — Protocol Freeze — PASS

Frozen horses:

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

Frozen R0 contracts:

```text
BENCH-GRAMMAR-v1
normalized result contract
canonical UTF-8 edit
IMPLEMENTATION_PARITY_CONTRACT
T_prepare / T_native / T_total
work-counter / PA definitions
sampling / repeatability
conclusion ladder
prior-art provenance / fidelity
```

Verdict: `PASS`

Next: `R1 CONTROLLED RUST HARNESS / DIRECTORY SUBSTRATE`

---

## R1 — Controlled Rust Harness / Directory Substrate

目标：只搭统一实验平台，不实现 H0-H4 算法，不产生性能结论。

R1 必须建立：

```text
one Rust workspace
pinned rust-toolchain / Cargo.lock
frozen release profile / LTO / codegen-units / panic / RUSTFLAGS
shared allocator policy
common Source/Edit/status/work/result-hook substrate types
(no Markdown Node/AST/CST/tree representation frozen in R1)
runner-owned timer API
black_box input/output protection
result checksum/full-work validation boundary
T-LANE / M-LANE / A-LANE separation
stable case ID / seed handling
result-schema round trip
```

R1 禁止：

```text
H1-H4 performance tuning
horse-specific optimization
formal performance claims
Markit production algorithm work
```

Gate R1：dummy/null mechanism 跑通 schema/timer/case/seed/lane，不产生性能结论。

R1-CORRECTIVE-1 (2026-09-16)：PR #26 对抗性 review 的 4 MAJOR + 3 IMPORTANT 已修复：prepare attribution、common PA authority、M-LANE per-case window、supervisor failure isolation、edit_meta operation contract、completion claim downgrade + EAGER gate、golden vectors、shuffle v2 rejection。

Fresh adversarial re-review：`PASS`。

R1 交付（`protocol/R1-HARNESS-CONTRACT.md` 为权威记录）：

```text
Cargo workspace（common / instrumentation / oracle / runner / mechanisms/null-r1）
pinned rustc 1.97.1 + frozen release-primary-v1 profile
Mechanism trait（full_parse / prepare_update / update / complete；prepare 也接受 MechanismContext）
runner-owned timer：T_total = T_prepare + T_native（算术恒等，无第三区间）
complete() + black_box 在 T_native 内，作为 completion authority boundary；
  real horses 在正式 measurement 前仍必须通过 EAGER_COMPLETION_VALIDATION_PASS
T / M / A 三 lane 结构性分离；M-LANE 使用 per-case begin/end window
Observed<T> = Known / UNKNOWN / NOT_APPLICABLE
source inspection 由 common collector 对 raw interval events 做 union/derive，horse 不拥有 PA 最终值
CaseId = SHA256(canonical_encode(CaseKeyV1))，无机制/lane/运行时身份
order = CaseId 字节排序 + splitmix64-v1+fisher-yates-lemire-rejection-v2
CaseId / SplitMix64 / final permutation 有 golden vectors 防止 silent drift
ResultRowV1 schema（Rust 类型为源，protocol/result-schema-v1.json 漂移防护）
worker/supervisor process boundary 保留 timeout/fatal-exit case；fatal signals 统一诚实归类 Crash
null mechanism __r1_null__ + R1_SMOKE_ONLY fixture 端到端通过
scripts/verify-r1.sh 验收门
```

Final R1 verdict:

```text
R1_FINAL_REVIEW_PASS
HARNESS_SUBSTRATE_PASS
PR_26_READY_FOR_MERGE
READY_FOR_R2_PRIOR_ART_EXTRACTION
```

Next: `R2 PRIOR-ART MECHANISM EXTRACTION`

---

## R2 — Prior-art Mechanism Extraction

Status: **READY_FOR_FINAL_R2_REVIEW（2026-09-17 人工评审要求的 corrective
pass MARKIT-R2-PRIOR-ART-CORRECTIVE-1 已完成：MAJOR-1 mizchi 复用语义修正
（parser-work reuse ≠ representation reuse）、MAJOR-2 状态最小性/普适性声明移除、
MAJOR-3 GLR reuse suppression 语义修正、IMPORTANT-1/2、MINOR 清理；
随后 MARKIT-R2-FINAL-CLEANUP-1 完成 MECHANISM_INTRINSIC_STATE 四分类
（mechanism state / common input / common instrumentation / model-defined
candidate state）、R2-H11 去过度概括、mizchi §7 措辞修复；
pins 不变。之前的 PRIOR_ART_EXTRACTION_PASS 自评已被人工对抗评审取代；
最终 PASS 由人工评审给出。）**

对 MD4C / pulldown-cmark / Comrak / Tree-sitter Markdown / Lezer / mizchi / Wagner & Graham / Swift incremental syntax 记录：

```text
source/version/paper
problem solved
mechanism
reuse unit
restart/convergence/fallback
retained representation
position/range strategy
strength/weakness hypotheses
relevance to H0-H4
not reproduced
```

R2 交付（`prior-art/` 为权威记录，2026-09-16）：

```text
9 个先验艺术提取记录（全部 15 节结构，OBSERVED/INFERRED/UNKNOWN 纪律，
  pinned tag/SHA + retrieval date，负证据一等公民）
SOURCE-MAP.md（来源清单 + 命名纠正 + 未增补来源的边界决定）
MECHANISM-SOURCE-MAP.md（每匹马 MECHANISM_SOURCE_MAP / PRIOR_ART_ANCHOR /
  FIDELITY_BOUNDARY / NON_GOALS / MECHANISM_INTRINSIC_STATE）
MECHANISM-MATRIX.md（行=机制 M1-M8 的对比矩阵 + confidence）
R2-HYPOTHESES.md（12 条假设，无数字、无排名）
manifest/prior-art.toml（14 个来源的机器可读 pin 集）
fresh adversarial review：A-L 失败模式 + scope/manifest/consistency，
  12/12 引用抽查 VERIFIED；verdict PRIOR_ART_REVIEW_IMPORTANT_ONLY，
  1 IMPORTANT + 4 MINOR 已全部修复，零 MAJOR
```

关键发现（reported, not resolved；详见
`prior-art/MECHANISM-SOURCE-MAP.md` HORSE_BOUNDARY_AMBIGUITIES）：

```text
H2/H3 边界 = 查找/担保策略而非复用粒度（Lezer 在 fragment gap 内做
  状态锚定的节点/块复用）
tree-sitter 是 H3+H4 混合体（复用受状态一致性门控；skip-based 而非
  checkpoint-based）
W&G 同时支撑 H3 与 H4；Swift 横跨两者
三个锚点族的 convergence authority 各不相同（batch-parser 定理匹配点 /
  字节局部检查点谓词 / 状态一致性门控）；没有任何先验实现
  "reparse 后比较新旧树" 的字面形状
mizchi 的 definition-fallback 使 H1 在含引用文档上按设计退化为 H0
```

H1-H4 各自必须有：

```text
MECHANISM_SOURCE_MAP
PRIOR_ART_ANCHOR
FIDELITY_BOUNDARY
NON_GOALS
MECHANISM_INTRINSIC_STATE
```

Stop：`READY_FOR_FINAL_R2_REVIEW`（corrective pass + final cleanup 已推送至
PR #27）→ Next: R3，等待 PR 人工评审合并。

---

## R3 — BENCH-GRAMMAR-v1 + Corpus / Mutation Freeze

Status: **READY_FOR_ADVERSARIAL_R3_REVIEW (2026-09-17)**. Deliverables:
`protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md` (stage record),
`grammar/BENCH-GRAMMAR-v1.md` + `NORMALIZED-RESULT-v1.md` + 43 golden
fixtures, `corpus/CORPUS-v1.md` + manifest (24 corpora, exact-byte
sizing, corpus-gen-v1 determinism contract), `mutations/MUTATION-v1.md` +
manifest (13 structural recipes across the six R0 families),
`cases/CASE-MATRIX-v1.md` + manifest (370 unique cases), and the static
gate `scripts/verify-r3.sh` (`R3 FREEZE GATE: PASS`). R2 hypotheses are
workload coverage only: 10 covered, 2 deferred, 0 out of scope. No
parser/mechanism code, no corpus bytes, no timing.

Grammar minimum：paragraph/text, blank-line boundary, ATX heading, basic list/blockquote, fenced code, emphasis, code span, link/reference basics, reference definition。

Synthetic shapes：

```text
PLAIN
MANY_BLOCKS
HUGE_BLOCK
DEEP_CONTAINER
INLINE_DENSE
FENCE_HEAVY
REFERENCE_FANOUT
MIXED
```

Sizes：`64 KiB / 1 MiB / 16 MiB`。

Operations：

```text
FULL_PARSE
INSERT
DELETE
REPLACE_EQ
REPLACE_GROW
REPLACE_SHRINK
STRUCTURAL_EDIT
QUERY
```

Structural minimum：local text / block boundary / container / forward state / inline delimiter / semantic dependency。

Stop：`GRAMMAR_CORPUS_MUTATION_FREEZE_PASS`

---

## R4 — H0 Reference Full Rebuild

建立 H0 clean parser 和 normalized correctness oracle。

```text
normalize(H1/H2/H3/H4.update(post-edit))
==
normalize(H0.clean_parse(post-edit source))
```

Stop：`H0_REFERENCE_PASS`

---

## R5 — H1/H2/H3/H4 Mechanism Implementation

顺序：H1 block-local → H2 fragment → H3 old-tree subtree → H4 restart-convergence。

每匹 horse 在正式 measurement 前必须通过：grammar fixtures、arbitrary/structural differential correctness、counter semantics、fallback、implementation parity、no timer-informed tuning、no undeclared horse-specific optimization。

Hard gate before formal horse measurement (R1-CORRECTIVE-1, IMPORTANT-1):

```text
EAGER_COMPLETION_VALIDATION_PASS
```

`complete()` + `black_box` 是 completion authority boundary，不是 lazy work 不可能性的机械证明。R4/R5 必须额外证明真实 horses 的 normalized result/state 是 eager materialization，才能进入正式 measurement。

Stop：`HORSE_CORRECTNESS_AND_PARITY_PASS`（且 `EAGER_COMPLETION_VALIDATION_PASS`）

---

## R6 — Full Parse / State Construction Surface

测 retained-state 固定成本：latency/throughput/CPU/memory/allocation/state size。

Stop：`STATE_CONSTRUCTION_SURFACE_PASS`

---

## R7 — Arbitrary Edit Microbench

`INSERT/DELETE/REPLACE_* × edit size × position × payload shape × size`。

输出：T_prepare/T_native/T_total、CPU、memory/allocation、PA、blocks/nodes/metadata、restart/convergence、fallback。

Stop：`ARBITRARY_EDIT_SURFACE_PASS`

---

## R8 — Structural Edit Campaign

覆盖 paragraph、container depth、fence、inline delimiter、link/reference delimiter、reference-definition changes。

Stop：`STRUCTURAL_EDIT_SURFACE_PASS`

---

## R9 — Representation + Query

测 retained bytes/source bytes、node count、offset→enclosing region、full traversal。

Stop：`REPRESENTATION_QUERY_PASS`

---

## R10 — Scaling Surface

分析 `N/B/L/D/K/F` empirical scaling signatures，不宣称复杂度证明。

Stop：`SCALING_SURFACE_PASS`

---

## R11 — Attribution + Optimization Sensitivity

顺序：timing → scaling → work counters → allocation/bytes moved → controlled ablation/counterexample → second compiler profile → optional PMU。

关键 Weakness Map 候选若在第二 compiler profile 下 materially flip，标 `OPTIMIZATION_SENSITIVE` 并降级结论。

Stop：`ATTRIBUTION_PASS`

---

## R12 — Replication / Robustness

```text
3 independent sessions
10 warmup/session
30 measured/session
recorded shuffled seed
p50 + p95
no p99
no outlier deletion
```

Stop：`REPLICATION_PASS`

---

## R13 — Paper-style Report + Weakness Map

最终报告：Research Question / Prior Art / H0-H4 / Grammar+Workloads / Methodology+Parity / Correctness / Edit Results / Structural Results / Representation / Scaling / Attribution+Optimization Sensitivity / Profiles / Weakness Map / Threats / Conclusions / Reproducibility。

Final Stop：

```text
BENCHMARK_COMPLETE
CORRECTNESS_GATED
IMPLEMENTATION_PARITY_AUDITED
STRENGTH_WEAKNESS_PROFILES_COMPLETE
WEAKNESS_MAP_COMPLETE
OPTIMIZATION_SENSITIVITY_RECORDED
NO_MARKIT_ALGORITHM_IMPLEMENTED
NO_ARCHITECTURE_FROZEN
READY_FOR_HUMAN_WEAKNESS_MAP_REVIEW
```

---

# Phase 2 — deferred

R13 + human review 后再考虑 YCSB-like mixed workloads / real editing traces。