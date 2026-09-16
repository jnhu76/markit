# MARKIT-MARKDOWN-BENCHMARK-1 — 实验 Roadmap

Status: **ACTIVE / PROTOCOL-FIRST**  
Authority: GitHub Issue #22  
Branch: `research/22-markdown-benchmark-1`

本文件只规定**执行顺序、阶段交付物与 Gate**。实验目标、研究问题、对象、payload、operation、指标和结论规则以 Issue #22 为研究协议权威。

核心纪律：

```text
协议冻结
  -> 实验基础设施
  -> correctness
  -> baseline measurements
  -> scaling
  -> attribution
  -> replication
  -> Weakness Map
  -> 人工 review

禁止：看到单点性能差异就直接提出 Markit 算法。
```

---

## R0 — Protocol Freeze

目标：在第一条正式性能数据产生前冻结实验语义。

交付：

- 6 个 baseline 的 capability / provenance manifest；
- B2（Comrak 或 cmark-gfm）二选一并记录理由；
- operation schema；
- payload schema；
- measurement boundary；
- result row schema；
- correctness oracle schema；
- timing / memory / allocation 三条 measurement lane；
- sampling / warmup / randomization 规则；
- Parse Amplification 定义与 `UNKNOWN` 规则；
- RW01 `CppCoreGuidelines.md` 的 pinned provenance；
- 目录和代码 ownership 冻结。

### Gate R0

只有以下全部满足才进入 R1：

```text
SAME_PAYLOAD semantics defined
SAME_OPERATION semantics defined
MEASUREMENT_BOUNDARY defined
CORRECTNESS_BEFORE_PERFORMANCE defined
CAPABILITY_CLASS defined
RESULT_SCHEMA defined
CONCLUSION_RULES defined
```

Stop：`READY_FOR_STAGE_1_IMPLEMENTATION_REVIEW`

---

## R1 — Harness Substrate

目标：只建立统一实验基础设施，不产生研究结论。

建议目录：

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/
│   ├── experiment.md
│   ├── operations.*
│   ├── metrics.*
│   └── result-schema.*
├── manifest/
│   ├── baselines.*
│   ├── environment.*
│   └── corpus.*
├── runner/
├── adapters/
│   ├── md4c/
│   ├── pulldown-cmark/
│   ├── retained-ast/
│   ├── tree-sitter-markdown/
│   ├── lezer-markdown/
│   └── mizchi-markdown/
├── corpus/
│   ├── generators/
│   ├── mutations/
│   ├── real-world/
│   └── materialized/        # generated / normally ignored
├── oracle/
├── instrumentation/
├── scripts/
├── results/
│   ├── raw/                 # ignored
│   ├── normalized/          # ignored or generated
│   └── summary/             # curated evidence only
└── report/
```

### Ownership rules

- `runner/`：唯一负责 case enumeration、顺序、measurement lane、统一结果 schema；
- `adapters/<baseline>/`：只负责把统一 operation 映射到该 baseline；不得自带私有 workload、评分规则或结论逻辑；
- `corpus/`：只产生/物化 source 与 edit descriptors；不得知道 parser；
- `oracle/`：只判断 correctness；不得读 timing 结果；
- `instrumentation/`：只采集可观测 work counters；不能改变 operation 语义；
- `results/raw/`：机器原始输出，默认不提交；
- `results/summary/`：经过 schema 校验、带 provenance 的精选证据；
- `report/`：最终论文式分析，不包含 executable benchmark logic。

第三方代码：优先用 manifest + pinned fetch/build；非必要不复制进仓库。若 vendor，必须记录 license、upstream、commit/tag、SHA256 和理由。

### Gate R1

- 一个 dummy/null adapter 可以完整走完 protocol；
- schema round-trip；
- case ID 稳定；
- 同 seed 重跑生成相同 payload/edit；
- timing lane 与 memory/allocation lane 分离。

Stop：`HARNESS_SUBSTRATE_PASS`

---

## R2 — Baseline Adapters + Capability Conformance

目标：逐个接入 baseline，但先验证能力和 correctness，不比较快慢。

顺序建议：

```text
B0 MD4C
B1 pulldown-cmark
B2 retained AST control
B3 tree-sitter-markdown
B4 @lezer/markdown
B5 mizchi/markdown
```

每个 adapter 必须产出 capability record：

```text
version / commit
runtime / compiler
Markdown dialect
output model
retained?
lossless?
incremental?
query support?
observable work counters?
```

### Gate R2

每个 baseline：

- FULL_PARSE smoke PASS；
- 对支持 incremental 的对象：SELF_EQUIVALENCE PASS；
- DIALECT_SEMANTIC_ORACLE 的适用范围已明确；
- 不支持能力明确记 `N/A`，不可模拟成“支持”；
- 可观察指标和 `UNKNOWN` 项记录完整。

Stop：`BASELINE_CONFORMANCE_PASS`

---

## R3 — Corpus + Mutation Freeze

目标：冻结 payload，不允许根据初步性能结果偷偷调整 workload。

Controlled synthetic：

```text
P1 PLAIN
P2 MANY_BLOCKS
P3 HUGE_BLOCK
P4 DEEP_CONTAINER
P5 INLINE_DENSE
P6 FENCE_HEAVY
P7 REFERENCE_FANOUT
P8 MIXED
```

尺寸：

```text
64 KiB / 256 KiB / 1 MiB / 4 MiB / 16 MiB
```

Real world：

```text
RW01 CppCoreGuidelines.md
```

必须 pin repo commit、file SHA256、bytes、lines、encoding。

Mutation families：

```text
INSERT
DELETE
REPLACE_EQ
REPLACE_GROW
REPLACE_SHRINK
STRUCTURAL_EDIT
QUERY
```

Structural families 必须覆盖：paragraph/boundary、heading、container、fence/forward-state、reference/semantic-global。

### Gate R3

- corpus deterministic；
- mutation deterministic；
- edit boundary 合法；
- payload metadata 完整；
- RW01 可重复 materialize；
- 所有 case 在正式 measurement 前冻结 ID。

Stop：`CORPUS_AND_MUTATION_FREEZE_PASS`

---

## R4 — Correctness Campaign

目标：性能前先排除“快但错”。

Oracle：

```text
SELF_EQUIVALENCE
incremental == same parser clean rebuild

DIALECT_SEMANTIC_ORACLE
normalized semantics == expected for supported dialect

LOSSLESS_ORACLE
reconstruct == exact source bytes, where supported
```

Structural edit 是重点，不允许只跑普通字符替换。

### Gate R4

- correctness failure 必须成为显式结果；
- failure case 不进入 headline performance ranking；
- 不允许为使 baseline 通过而改 payload semantics；
- 每个 known divergence 有 taxonomy：BUG / DIALECT_DIFFERENCE / UNSUPPORTED / ORACLE_LIMITATION。

Stop：`CORRECTNESS_GATE_PASS`

---

## R5 — Full Parse Baseline

目标：建立 rebuild floor。

测：

```text
throughput
latency
CPU
peak/retained memory
allocations where comparable
```

规则：event parser 与 retained AST/CST parser 必须标注 capability，不产生一个“总冠军”排名。

### Gate R5

- 所有 6 baseline 完成适用 payload；
- 同机器、同 build profile、同 corpus；
- timing lane 无 memory/allocation instrumentation 污染；
- provenance complete。

Stop：`FULL_PARSE_SURFACE_PASS`

---

## R6 — Arbitrary Edit Microbench

目标：db_bench 风格逐操作测成本。

矩阵：

```text
INSERT / DELETE / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK
× edit size
× position
× payload
× document size
```

主输出：

```text
latency p50/p95/p99 (p99 only when sample count is sufficient)
CPU/op
memory lane
allocation lane
edited bytes
bytes rescanned, if observable
Parse Amplification, if observable
```

Full-rebuild controls按“post-edit source -> clean rebuild”执行，是 control，不是 N/A。

### Gate R6

Stop：`ARBITRARY_EDIT_SURFACE_PASS`

---

## R7 — Structural Edit Campaign

目标：单独测 Markdown 结构传播，不与普通文本 update 混合。

重点：

- paragraph split/merge；
- ATX/Setext heading conversion；
- list/blockquote depth；
- lazy continuation；
- fence open/close/type/length；
- closed ↔ unclosed；
- reference definition/winner changes。

额外记录：

```text
logical edit bytes
observed affected syntax span
first stable/reusable suffix, if observable
```

PA 不作为“越小越好”的简单评分。

### Gate R7

Stop：`STRUCTURAL_EDIT_SURFACE_PASS`

---

## R8 — Syntax Representation + Query

目标：比较 retained representation 的成本，不强迫 event parser 假装有 AST。

只测第一版必要项：

```text
retained bytes / source bytes
node count
random offset -> node/block latency
full sequential traversal latency
```

Event-only parser：`N/A — no retained representation`。

### Gate R8

Stop：`REPRESENTATION_SURFACE_PASS`

---

## R9 — Semantic Resolution Surface

目标：把 syntax propagation 和 semantic fanout 分开。

第一轮只聚焦 reference definition/users；不扩展到完整 IDE semantics。

至少控制：

```text
fanout F
winner change
losing duplicate
add/delete definition
label rename
```

如果 baseline 本身不提供语义解析能力，必须把 parser timing 与外部统一 resolver timing 分栏，禁止混成“parser 性能”。

### Gate R9

Stop：`SEMANTIC_SURFACE_PASS`

---

## R10 — Scaling + RW01

目标：得到性能曲面而不是单点数字。

分析变量：

```text
N = document bytes
B = block count
L = enclosing/affected block size
D = container depth
K = edit size
F = semantic fanout
```

RW01 必须覆盖 Issue #22 的固定操作集合。

输出只描述 scaling signature；此阶段仍不允许直接宣称机制根因。

### Gate R10

Stop：`SCALING_AND_REALWORLD_SURFACE_PASS`

---

## R11 — Performance Attribution

仅对稳定差异触发，不做全量 profile。

触发条件：

```text
稳定 >20% 差异
OR
scaling shape 明显不同
OR
correctness/robustness cliff
```

归因顺序固定：

```text
1. scaling signature
2. 最小 work counters
3. controlled counterexample
4. 必要时 profiler
```

允许归因维度：

```text
O(N) document coupling
O(B) block-count coupling
O(L) reuse granularity floor
O(D) container/depth coupling
O(K) edit-size dominated
O(F) semantic fanout
allocation/reconstruction
position/range maintenance
runtime/IPC boundary
```

每个归因产出 `Weakness Card`：

```text
ID
payload
operation
observation
scaling signature
mechanism hypothesis
evidence
control/counterexample
confidence
Markit relevance
```

注意：`mechanism hypothesis` 不是结论，只有被 control/counterexample 支持后才能升级为 attributed weakness。

### Gate R11

Stop：`ATTRIBUTION_PASS`

---

## R12 — Replication / Robustness

目标：阻止偶然数据进入最终结论。

对所有拟进入 Weakness Map 的结论：

- fresh process rerun；
- case order shuffle / fixed seed；
- 至少一次独立 measurement session；
- 核查环境漂移；
- correctness gate 再跑；
- 对关键差异做最小 reproducer。

不能稳定复现：降级为 `INCONCLUSIVE`。

### Gate R12

Stop：`REPLICATION_PASS`

---

## R13 — Paper-style Report + Weakness Map

最终报告结构固定为：

```text
1. Abstract / Executive Summary
2. Research Questions
3. Experimental Subjects
4. Methodology
5. Correctness and Capability
6. Full-Parse Results
7. Arbitrary-Edit Results
8. Structural-Edit Results
9. Representation / Query Results
10. Semantic Resolution Results
11. Scaling Analysis
12. Performance Attribution
13. Weakness Map
14. Threats to Validity / Limitations
15. Conclusions
16. Reproducibility Appendix
```

结论等级只允许：

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_WEAKNESS
COMMON_WEAKNESS
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

禁止：

```text
单点快 -> 算法优越
相关性 -> 因果
一个 baseline 弱点 -> 所有现有算法共同弱点
Experiment 0 结果 -> #22 结论
benchmark weakness -> 直接写 Markit production architecture
```

最终 Weakness Map 必须逐项回答：

1. 弱点在哪个 payload/operation 出现？
2. correctness/capability 是否同级可比？
3. scaling signature 是什么？
4. 根因是否已经被控制实验支持？
5. 是单实现弱点、某一设计族弱点，还是跨设计共同弱点？
6. 对真实 Markdown 编辑是否重要？
7. 是否值得下一阶段提出 Markit-specific 算法？

### Final Stop Gate

```text
BENCHMARK_COMPLETE
CORRECTNESS_GATED
WEAKNESS_MAP_COMPLETE
ATTRIBUTION_REVIEWABLE
NO_MARKIT_ALGORITHM_IMPLEMENTED
NO_ARCHITECTURE_FROZEN
READY_FOR_HUMAN_WEAKNESS_MAP_REVIEW
```

到这里停止。

---

# Phase 2 — YCSB-like Mixed Workload（暂定，不属于第一轮）

只有 R13 完成且人工认为 microbench 已经解释主要机制后，才考虑：

```text
typing-heavy
read/query-heavy
structural-edit-heavy
paste/delete-heavy
real editing trace
```

Phase 2 的作用是验证 microbench 结论在混合真实工作负载下是否仍成立，不得反过来替代第一轮的机制归因。
