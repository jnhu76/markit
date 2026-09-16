# MARKIT-MARKDOWN-BENCHMARK-1 — 实验 Roadmap

Status: **ACTIVE / R0 FINAL REVIEW**  
Authority: GitHub Issue #22  
Branch: `research/22-markdown-benchmark-1`  
R0 methodology: `protocol/R0-METHODOLOGY.md`

本文件只规定执行顺序、阶段交付物与 Gate。实验方法学以 Issue #22 + `protocol/R0-METHODOLOGY.md` 为权威。

核心纪律：

```text
protocol
-> controlled Rust substrate
-> shared grammar/result contract
-> H0 control
-> H1-H3 mechanism implementations
-> correctness
-> microbench surfaces
-> scaling/work counters
-> attribution
-> replication
-> strength/weakness profiles
-> Weakness Map
-> human review
```

禁止：看到单点 timing 差异就直接提出 Markit algorithm。

---

## R0 — Protocol Freeze

目标：在任何正式 benchmark 代码/结果出现前冻结“我们到底在比较什么”。

### Primary subject

```text
Controlled Rust mechanism race
```

不是六个 upstream parser 的跨语言 absolute-time ranking。

Upstream projects：

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
```

只作为 prior-art/source reconnaissance、机制抽取和必要 sanity/fidelity probes。

### First-round horses

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL
H2 FRAGMENT_REUSE
H3 OLD_TREE_REUSE_CONVERGENCE
```

### R0 deliverables

必须冻结：

- `BENCH-GRAMMAR-v1` 的语义范围；
- normalized result contract；
- H0-H3 mechanism boundary / allowed state；
- canonical UTF-8 edit descriptor；
- payload + structural-edit families；
- `T_prepare / T_native / T_total` timer contract；
- headline metrics；
- algorithmic work-counter schema；
- PA 定义；
- sampling/repeatability；
- failure taxonomy；
- conclusion ladder；
- prior-art provenance/fidelity rules；
- 实验目录 ownership。

### Gate R0

```text
PRIMARY_SUBJECT = CONTROLLED_RUST_MECHANISM_RACE
UPSTREAM_ROLE = PRIOR_ART / SANITY_ONLY
BENCH_GRAMMAR_V1_DEFINED
NORMALIZED_RESULT_CONTRACT_DEFINED
H0_H3_BOUNDARIES_DEFINED
SAME_PAYLOAD_DEFINED
SAME_EDIT_DEFINED
SAME_SEMANTIC_TASK_DEFINED
TIMER_CONTRACT_DEFINED
CORRECTNESS_ORACLE_DEFINED
WORK_COUNTER_SCHEMA_DEFINED
STRUCTURAL_MINIMUM_DEFINED
SAMPLING_POLICY_DEFINED
CONCLUSION_RULES_DEFINED
```

Stop：`READY_FOR_R1_CONTROLLED_RUST_HARNESS`

---

## R1 — Controlled Rust Harness / Directory Substrate

目标：只搭统一实验平台，不实现 H0-H3 算法，不产生性能结论。

目录：

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
│   └── old-tree-convergence/
├── corpus/
│   ├── generators/
│   ├── mutations/
│   ├── real-world/
│   └── materialized/
├── oracle/
├── instrumentation/
├── prior-art/
├── scripts/
├── results/
│   ├── raw/
│   ├── normalized/
│   └── summary/
└── report/
```

### Ownership

- `runner/`：唯一 case enumeration / sequencing / result schema；
- `common/`：仅共享与被测机制无关的代码；
- `mechanisms/`：H0-H3 的研究变量实现；
- `corpus/`：source + edit descriptors，不知道 horse；
- `oracle/`：correctness，不读取 timing；
- `instrumentation/`：work/memory/allocation counters；
- `prior-art/`：上游源码/设计笔记、probe/fidelity evidence，不参与 headline ranking；
- `report/`：分析，不包含 executable benchmark logic。

### Gate R1

用 dummy/null mechanism 验证：

- case ID 稳定；
- corpus/mutation deterministic；
- result schema round-trip；
- timer API 可强制边界；
- T-LANE / M-LANE / A-LANE 分离；
- same seed -> same cases/order；
- no actual performance claim。

Stop：`HARNESS_SUBSTRATE_PASS`

---

## R2 — Prior-art Mechanism Extraction

目标：先理解源码再写 horse，避免“凭印象实现 Tree-sitter/Lezer/mizchi”。

对每个 prior-art source 记录：

```text
UPSTREAM / version / commit
problem solved
parser/update mechanism
restart authority
reuse unit
convergence/fallback rule
retained representation
position/range strategy
known strengths
known weaknesses
what is relevant to H0-H3
what is NOT reproduced
```

允许跑少量 upstream payload probe，但结果标 `REFERENCE_ONLY`。

### Gate R2

H1-H3 每个都必须有：

```text
MECHANISM_SOURCE_MAP
FIDELITY_BOUNDARY
NON_GOALS
```

Stop：`PRIOR_ART_EXTRACTION_PASS`

---

## R3 — BENCH-GRAMMAR-v1 + Corpus / Mutation Freeze

目标：保证四匹 horse 真正解决同一个问题。

### Grammar v1 minimum

```text
plain paragraph/text
blank-line block boundary
ATX heading
basic list/blockquote
fenced code
emphasis delimiter
code-span delimiter
inline/reference link basics
reference definition
```

不宣称完整 CommonMark。

### Synthetic shapes

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

第一轮 sizes：

```text
64 KiB
1 MiB
16 MiB
```

只有观察到 cliff/crossover 后才补点。

### Operations

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

Structural minimum covers：

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

### Gate R3

- grammar deterministic；
- corpus deterministic；
- mutations deterministic/legal；
- normalized result fixtures frozen；
- all case IDs frozen before measurement。

Stop：`GRAMMAR_CORPUS_MUTATION_FREEZE_PASS`

---

## R4 — H0 Reference Full Rebuild

目标：先建立正确的 control/oracle，不做“增量算法”。

H0 必须：

```text
source -> BENCH-GRAMMAR-v1 normalized syntax state
```

要求：

- deterministic；
- all grammar fixtures pass；
- source spans/relations obey result contract；
- no hidden incremental state；
- FULL_PARSE timer boundary verified。

H0 是 H1-H3 correctness oracle：

```text
horse.update(post-edit)
==
H0.clean_parse(post-edit)
```

Stop：`H0_REFERENCE_PASS`

---

## R5 — H1/H2/H3 Mechanism Implementation

顺序：

```text
H1 BLOCK_LOCAL
H2 FRAGMENT_REUSE
H3 OLD_TREE_REUSE_CONVERGENCE
```

每个 horse 单独 commit/section，不能一次写三套后再调数据。

每个 horse 在性能 measurement 前必须：

- grammar fixtures PASS；
- arbitrary edit differential correctness vs H0 PASS；
- structural edit correctness PASS；
- work-counter instrumentation semantics documented；
- fallback behavior explicit；
- no timing-based heuristic tuning yet。

Stop：`HORSE_CORRECTNESS_PASS`

---

## R6 — Full Parse / State Construction Surface

测：

```text
latency p50/p95
throughput
CPU
peak/retained memory
allocation
state size
```

主要用于理解 retained-state 固定成本，不产生 update 机制优越性结论。

Stop：`STATE_CONSTRUCTION_SURFACE_PASS`

---

## R7 — Arbitrary Edit Microbench

矩阵：

```text
INSERT / DELETE / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK
× edit size
× position
× payload shape
× size
```

主输出：

```text
T_prepare
T_native
T_total
CPU/op
memory/allocation lane
source coverage / PA
blocks reparsed
nodes rebuilt/reused
metadata touched
fallback count
```

Stop：`ARBITRARY_EDIT_SURFACE_PASS`

---

## R8 — Structural Edit Campaign

重点：

```text
paragraph split/merge
list/blockquote depth
fence open/close
emphasis/code-span delimiter
link/reference delimiter
reference-definition changes
```

必须同时记录传播/收敛特征，不把所有 1-byte edit 当成同类。

Stop：`STRUCTURAL_EDIT_SURFACE_PASS`

---

## R9 — Representation + Query

第一轮只测：

```text
retained bytes/source bytes
node count
Q1 offset -> enclosing syntax/block region
Q2 full syntax traversal
```

Stop：`REPRESENTATION_QUERY_PASS`

---

## R10 — Scaling Surface

分析：

```text
N document bytes
B block count
L affected/largest block
D container depth
K edit size
F semantic fanout
```

这里只描述 empirical scaling signature，不把有限实验曲线宣称成复杂度证明。

Stop：`SCALING_SURFACE_PASS`

---

## R11 — Attribution

固定顺序：

```text
1 timing difference
2 scaling signature
3 work counters
4 allocation/metadata/bytes moved
5 controlled mutation/ablation/counterexample
6 optional PMU only if residual remains unexplained
```

PMU 非第一轮硬要求；可选：

```text
cycles
instructions
branch misses
cache misses
L1/LLC misses if available
```

每个 horse 形成：

```text
MECHANISM
BEST REGIME
WORST REGIME
SCALING SIGNATURE
WORK AMPLIFICATION
MEMORY / ALLOCATION COST
FAILURE / FALLBACK REGIME
KEY STRENGTH
KEY WEAKNESS
EVIDENCE
CONFIDENCE
```

Stop：`ATTRIBUTION_PASS`

---

## R12 — Replication / Robustness

所有拟进入最终结论的数据：

```text
3 independent sessions
10 warmup/session
30 measured/session
recorded shuffled seed
p50 + p95
no p99 first round
no outlier deletion
```

关键结论至少 fresh-run 再现一次；失败则降级 `INCONCLUSIVE`。

Stop：`REPLICATION_PASS`

---

## R13 — Paper-style Report + Weakness Map

结构：

```text
1 Abstract
2 Research Question / Prior Art
3 Controlled Mechanism Models
4 BENCH-GRAMMAR-v1 / Workloads
5 Methodology / Timer / Metrics
6 Correctness
7 Arbitrary Edit Results
8 Structural Edit Results
9 Representation / Query
10 Scaling
11 Attribution
12 Strength/Weakness Profiles
13 Weakness Map / Pareto Gaps
14 Threats to Validity
15 Conclusions
16 Reproducibility Appendix
```

结论只允许：

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_STRENGTH
ATTRIBUTED_WEAKNESS
CROSS_MECHANISM_PATTERN
PARETO_GAP
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

### Final Stop

```text
BENCHMARK_COMPLETE
CORRECTNESS_GATED
STRENGTH_WEAKNESS_PROFILES_COMPLETE
WEAKNESS_MAP_COMPLETE
NO_MARKIT_ALGORITHM_IMPLEMENTED
NO_ARCHITECTURE_FROZEN
READY_FOR_HUMAN_WEAKNESS_MAP_REVIEW
```

到这里停止。

---

# Phase 2 — Mixed / real editing workloads (deferred)

只有 R13 + human review 之后，才考虑 YCSB-like mixed workloads / real editing traces，用来检查 microbench 得出的机制结论在混合 workload 中是否仍成立。
