# MARKIT-MARKDOWN-BENCHMARK-1 — 实验 Roadmap

Status: **R0 PASS / READY FOR R1**  
Authority: GitHub Issue #22  
Branch: `research/22-markdown-benchmark-1`  
R0 methodology: `protocol/R0-METHODOLOGY.md`

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

目标目录：

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/
│   ├── R0-METHODOLOGY.md
│   ├── grammar.md
│   ├── result-contract.md
│   ├── operations.md
│   ├── metrics.md
│   ├── implementation-parity.md
│   └── result-schema.md
├── manifest/
│   ├── environment.*
│   ├── prior-art.*
│   └── corpus.*
├── runner/
├── common/
├── mechanisms/
│   ├── full-rebuild/
│   ├── block-local/
│   ├── fragment-reuse/
│   ├── old-tree-subtree-reuse/
│   └── restart-convergence/
├── corpus/
├── oracle/
├── instrumentation/
├── prior-art/
├── scripts/
├── results/
└── report/
```

R1 必须建立：

```text
one Rust workspace
pinned rust-toolchain / Cargo.lock
frozen release profile / LTO / codegen-units / panic / RUSTFLAGS
shared allocator policy
common Source/Edit types
common normalized Node/result types
runner-owned timer API
black_box input/output protection
result checksum/full-work validation
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

Stop：`HARNESS_SUBSTRATE_PASS`

---

## R2 — Prior-art Mechanism Extraction

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

H1-H4 各自必须有：

```text
MECHANISM_SOURCE_MAP
PRIOR_ART_ANCHOR
FIDELITY_BOUNDARY
NON_GOALS
MECHANISM_INTRINSIC_STATE
```

Stop：`PRIOR_ART_EXTRACTION_PASS`

---

## R3 — BENCH-GRAMMAR-v1 + Corpus / Mutation Freeze

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
normalize(H0.clean_parse(post-edit))
```

Stop：`H0_REFERENCE_PASS`

---

## R5 — H1/H2/H3/H4 Mechanism Implementation

顺序：H1 block-local → H2 fragment → H3 old-tree subtree → H4 restart-convergence。

每匹 horse 在正式 measurement 前必须通过：grammar fixtures、arbitrary/structural differential correctness、counter semantics、fallback、implementation parity、no timer-informed tuning、no undeclared horse-specific optimization。

Stop：`HORSE_CORRECTNESS_AND_PARITY_PASS`

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