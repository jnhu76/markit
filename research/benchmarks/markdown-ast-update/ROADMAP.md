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

---

## R0 — Protocol Freeze — PASS

目标：冻结“我们到底在比较什么”。

Primary subject：

```text
Controlled Rust mechanism race
```

Frozen horses：

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

Frozen fairness rules：

```text
BENCH-GRAMMAR-v1
normalized semantic result contract
canonical UTF-8 edit descriptor
one Rust workspace/toolchain/build/allocator policy
IMPLEMENTATION_PARITY_CONTRACT
T_prepare / T_native / T_total timer contract
shared non-research substrate
horse-owned mechanism-intrinsic state only
first-round horse-specific optimization ban
black_box + full-work validation
key-conclusion optimization-sensitivity check
```

Gate：

```text
PRIMARY_SUBJECT = CONTROLLED_RUST_MECHANISM_RACE
UPSTREAM_ROLE = PRIOR_ART / SANITY_ONLY
BENCH_GRAMMAR_V1_DEFINED
NORMALIZED_RESULT_CONTRACT_DEFINED
H0_H4_BOUNDARIES_DEFINED
IMPLEMENTATION_PARITY_CONTRACT_DEFINED
SAME_PAYLOAD_DEFINED
SAME_EDIT_DEFINED
SAME_SEMANTIC_TASK_DEFINED
TIMER_CONTRACT_DEFINED
CORRECTNESS_ORACLE_DEFINED
WORK_COUNTER_SCHEMA_DEFINED
STRUCTURAL_MINIMUM_DEFINED
SAMPLING_POLICY_DEFINED
CONCLUSION_RULES_DEFINED
METHODOLOGY_REFERENCES_RECORDED
```

Verdict：`PASS`

Next：`R1 CONTROLLED RUST HARNESS / DIRECTORY SUBSTRATE`

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

- `runner/`：唯一 case enumeration / sequencing / timer API / result schema；
- `common/`：仅共享与被测机制无关的 Source/Edit/grammar/scanner/Node/result/oracle/counter infrastructure；
- `mechanisms/`：只放 H0-H4 的机制变量实现；
- `corpus/`：source + edit descriptors，不知道 horse；
- `oracle/`：correctness，不读取 timing；
- `instrumentation/`：work/memory/allocation counters；
- `prior-art/`：源码/设计笔记、source map、sanity/fidelity evidence；
- `report/`：分析，不包含 executable benchmark logic。

### R1 platform requirements

必须建立并测试：

```text
one Rust workspace
pinned rust-toolchain / Cargo.lock
frozen release profile / LTO / codegen-units / panic / RUSTFLAGS policy
shared allocator policy
common Source/Edit types
common normalized Node/result types
runner-owned timer abstraction
black_box input/output protection
result checksum/full-work validation
T-LANE / M-LANE / A-LANE separation
stable case ID / seed handling
result-schema round trip
```

R1 不允许针对任何 H1-H4 写性能优化。

### Gate R1

用 dummy/null mechanism 验证：

- case ID stable；
- corpus/mutation deterministic；
- result schema round-trip；
- timer API 能强制 boundary；
- black_box/checksum 路径不会让工作被静默省略；
- same seed -> same cases/order；
- T-LANE / M-LANE / A-LANE 分离；
- compiler/build/allocator policy 能由 manifest 记录；
- no actual performance claim。

Stop：`HARNESS_SUBSTRATE_PASS`

---

## R2 — Prior-art Mechanism Extraction

目标：先理解 source/design 再写 horses。

来源：

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
Wagner & Graham
Swift incremental syntax parsing
```

对每个来源记录：

```text
UPSTREAM / version / commit / paper
problem solved
parse/update mechanism
restart authority
reuse unit
convergence/fallback rule
retained representation
position/range strategy
known strengths / weaknesses
relevance to H0-H4
what is NOT reproduced
```

允许少量 upstream sanity probes，但数据标 `REFERENCE_ONLY`。

### Gate R2

H1-H4 每个必须有：

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

目标：保证五匹 horse 解决同一个 parsing problem。

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

### Normalized result contract

正确性只比较：

```text
semantic node kind
ordered parent/child topology
UTF-8 source spans/source relation
ordered block/inline structure
reference-definition facts
```

以下不参与 correctness：

```text
pointer identity
NodeId persistence
fragment ID
allocation address
reuse count
```

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

First-round sizes：

```text
64 KiB
1 MiB
16 MiB
```

只有观察到 cliff/crossover 后才补点，并记录为 follow-up sampling，不重写已有协议。

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
- normalized fixtures frozen；
- all case IDs frozen before measurement。

Stop：`GRAMMAR_CORPUS_MUTATION_FREEZE_PASS`

---

## R4 — H0 Reference Full Rebuild

目标：建立正确 control/oracle。

H0：

```text
source -> BENCH-GRAMMAR-v1 normalized syntax state
```

要求：

- deterministic；
- all grammar fixtures PASS；
- spans/relations obey result contract；
- no hidden incremental state；
- FULL_PARSE timer boundary verified；
- uses same common scanner/grammar/result substrate intended for all horses where semantics permit。

Correctness authority：

```text
normalize(H1/H2/H3/H4.update(post-edit))
==
normalize(H0.clean_parse(post-edit))
```

Stop：`H0_REFERENCE_PASS`

---

## R5 — H1/H2/H3/H4 Mechanism Implementation

顺序：

```text
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

每匹 horse 独立 commit/section，不能一次写完再根据 benchmark 调优。

每匹 horse 在正式 measurement 前必须：

- grammar fixtures PASS；
- arbitrary-edit differential correctness vs H0 PASS；
- structural-edit correctness PASS；
- mechanism state / counter semantics documented；
- fallback explicit；
- implementation-parity checklist PASS；
- no timer-informed tuning；
- no undeclared horse-specific optimization。

Stop：`HORSE_CORRECTNESS_AND_PARITY_PASS`

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

用途：理解 retained-state 固定成本，不单独产生 update mechanism superiority 结论。

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

输出至少：

```text
T_prepare / T_native / T_total
CPU/op
memory/allocation lane
source coverage / PA
blocks reparsed
nodes rebuilt/reused
metadata touched
restart/convergence distance where applicable
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

第一轮：

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

这里只描述 empirical scaling signature，不把有限曲线宣称成复杂度证明。

Stop：`SCALING_SURFACE_PASS`

---

## R11 — Attribution + Optimization Sensitivity

固定顺序：

```text
1 timing difference
2 scaling signature
3 work counters
4 allocation/metadata/bytes moved
5 controlled mutation/ablation/counterexample
6 key-case optimization-sensitivity check
7 optional PMU only if residual remains unexplained
```

关键 Weakness Map 候选至少挑一个代表 case 用第二 frozen compiler profile 重跑。

如果 ranking/weakness materially flips：

```text
OPTIMIZATION_SENSITIVE
```

该结论必须降级/限定。

PMU 非第一轮硬要求；可选：

```text
cycles
instructions
branch misses
cache misses
L1/LLC misses if available
```

每匹 horse 形成：

```text
MECHANISM
PRIOR_ART_ANCHOR
BEST REGIME
WORST REGIME
SCALING SIGNATURE
WORK AMPLIFICATION
MEMORY / ALLOCATION COST
FAILURE / FALLBACK REGIME
KEY STRENGTH
KEY WEAKNESS
OPTIMIZATION_SENSITIVITY
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

关键结论 fresh-run 再现；失败则降级 `INCONCLUSIVE`。

Stop：`REPLICATION_PASS`

---

## R13 — Paper-style Report + Weakness Map

结构：

```text
1 Abstract
2 Research Question / Prior Art
3 Controlled Mechanism Models H0-H4
4 BENCH-GRAMMAR-v1 / Workloads
5 Methodology / Implementation Parity / Timer / Metrics
6 Correctness
7 Arbitrary Edit Results
8 Structural Edit Results
9 Representation / Query
10 Scaling
11 Attribution / Optimization Sensitivity
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
IMPLEMENTATION_PARITY_AUDITED
STRENGTH_WEAKNESS_PROFILES_COMPLETE
WEAKNESS_MAP_COMPLETE
OPTIMIZATION_SENSITIVITY_RECORDED
NO_MARKIT_ALGORITHM_IMPLEMENTED
NO_ARCHITECTURE_FROZEN
READY_FOR_HUMAN_WEAKNESS_MAP_REVIEW
```

到这里停止。

---

# Phase 2 — Mixed / real editing workloads (deferred)

只有 R13 + human review 后，才考虑 YCSB-like mixed workloads / real editing traces，用来检查 microbench 得出的机制结论在混合 workload 中是否仍成立。