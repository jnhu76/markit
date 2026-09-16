# MARKIT-MARKDOWN-BENCHMARK-1 — 实验 Roadmap

Status: **ACTIVE / R0 PROTOCOL CORRECTIVE**  
Authority: GitHub Issue #22  
Branch: `research/22-markdown-benchmark-1`  
R0 methodology: `protocol/R0-METHODOLOGY.md`

本文件规定执行顺序、阶段交付物与 Gate。实验目标、Research Questions、对象、payload、operation、measurement 和结论规则以 Issue #22 为研究协议权威。

核心纪律：

```text
协议冻结
  -> 实验基础设施
  -> capability/correctness
  -> native implementation measurements
  -> scaling
  -> attribution
  -> controlled Rust reproduction when needed
  -> replication
  -> Weakness Map
  -> human review
```

禁止：看到单点性能差异就直接提出 Markit 算法。

---

## R0 — Protocol Freeze

目标：在第一条正式性能数据产生前冻结实验语义、比较层级和 measurement boundary。

### R0.1 Baseline set

第一轮固定：

```text
B0 MD4C
B1 pulldown-cmark
B2 Comrak
B3 tree-sitter-markdown
B4 @lezer/markdown
B5 mizchi/markdown
```

每个 baseline 必须固定 exact version/tag/commit、runtime/compiler、feature flags、build profile、dialect、output model、retained/lossless/incremental/query capability 和可观察 work counters。

### R0.2 两条比较 Lane

#### Lane A — Native Implementation Performance

运行 upstream/native implementation，使用同一 canonical payload 和 operation contract。

测：latency / throughput / CPU / memory / allocation（适用时）。

允许结论：**现有 implementation 在冻结环境中的实际工程成本。**

禁止：从跨 runtime 的 raw wall-clock 直接推出 language 或 algorithm superiority。

#### Lane B — Rust Algorithm Reproduction

统一算法/机制复现语言：**Rust**。

理由：无 GC；allocation/layout/ownership 可控；C FFI 直接；Rust baseline 原生；适合 retained tree / incremental parsing mechanism 实验。

Rust reproduction 不是 upstream baseline。只用于 controlled causal attribution。

默认只复现已观察到差异/weakness 所需的机制，不为对称性全量重写六个 parser。

每个 reproduction 必须记录：

```text
upstream + exact commit/tag
mechanism/data-structure boundary
omitted features
representation differences
port-specific choices
differential/conformance cases
```

未通过 conformance：`REPRODUCTION_INVALID`，不得进入 algorithmic conclusion。

### R0.3 Same Operation Contract

Canonical edit：

```text
UTF-8 byte range [start,end)
+
inserted bytes
```

各 adapter 可转换成 native coordinate/change descriptor，但不得改变逻辑 edit。

第一轮 operation：

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

### R0.4 Timer Contract

#### FULL_PARSE

Outside timer：

```text
file IO
fixture/source materialization
process/runtime startup
one-time runtime init
correctness / normalization / logging
```

`T_native`：

```text
START
consume complete in-memory source
force native result to completion
  event/pull parser -> consume ALL events
  tree parser       -> complete native tree/result
STOP
```

禁止 lazy parser 只测 iterator/object construction。

#### UPDATE / STRUCTURAL_EDIT

Outside timer：old source、old parse state、canonical edit、post-edit source 已准备好。

```text
T_prepare:
  canonical edit -> native coordinate/change metadata

T_native:
  parser-required old-state maintenance
  + incremental parse/update to completion

T_total = T_prepare + T_native
```

正式结果同时保存 `T_prepare / T_native / T_total`。

Parser 必须付出的 coordinate/state maintenance 不得为了结果好看而移出计时区间。

#### Node/VM/IPC

如果 Rust runner 通过 IPC 调 Node/JS：

- `T_native` 在 Node/JS runtime 内部测；
- serialization/pipe/scheduler IPC 不进入 `T_native`；
- integration cost 可另报 `T_adapter_roundtrip`；
- process model、warmup、GC policy 必须预注册。

### R0.5 Correctness Contract

不为每个 arbitrary payload 手工写 expected AST。

#### C1 Capability Matrix

使用官方 CommonMark/GFM examples 记录支持范围和 known divergence。

#### C2 Incremental Self-Equivalence

```text
normalize(incremental(post-edit))
==
normalize(same-parser clean full parse(post-edit))
```

这是 incremental timing case 的主要 correctness gate。

语义工作明显不同的对象标 `CAPABILITY_DIFFERENCE` 并限制横向结论。

### R0.6 Parse Amplification

第一版：

```text
PA = unique source-byte coverage re-inspected / logical edited bytes
```

同一 byte 被重复读取不重复计入 PA；total byte-read work 可作为 attribution diagnostic。

不可真实观察：`PA = UNKNOWN`。禁止由 latency / changed range / node count 倒推。

Structural edit 同时报 affected syntax span / first stable suffix（可观察时）。

### R0.7 Structural families

至少覆盖六类传播机制：

```text
local text
block boundary
container state
forward state / fence
inline delimiter state
semantic dependency
```

Inline structural 至少包含 emphasis/code-span/link-bracket/escaped delimiter；block ambiguity 至少包含 Setext/thematic-break/list/HTML 等代表 case。

### R0.8 Sampling / Environment / Failure preregistration

正式 measurement 前冻结：

```text
OS/kernel
CPU
CPU affinity
SMT/frequency/turbo policy
Rust/C/Node/other runtime versions
process lifetime model
warmup rule
GC policy where relevant
sample count
p99 eligibility
case order randomization + seed
outlier rule
memory definitions
failure taxonomy: timeout/OOM/crash/wrong/unsupported
```

### R0.9 Conclusion ladder

只允许：

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_WEAKNESS
CROSS_IMPLEMENTATION_WEAKNESS
COMMON_WEAKNESS
PARETO_GAP
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

Raw native timing -> implementation observation only。

Algorithmic attribution 需要 scaling + work/reuse evidence + controlled probe/counterexample；必要时再用 valid Rust reproduction。

`COMMON_WEAKNESS` 必须声明 target class。`PARETO_GAP` 表示没有 baseline 同时满足声明的目标约束。

### R0.10 Methodology references

详见 `protocol/R0-METHODOLOGY.md` 与 Issue #22 `R0-METHODOLOGY-CORRECTIVE-1`。固定来源包括：

```text
Wagner & Graham, TOPLAS 1998
Swift Incremental Syntax Parsing proposal, 2018
Marr et al., DLS 2016
Georges et al., OOPSLA 2007
Barrett et al., OOPSLA 2017
Mytkowicz et al., ASPLOS 2009
YCSB, SoCC 2010
RocksDB db_bench
```

### Gate R0

全部满足才进入 R1：

```text
BASELINE_SET_FROZEN (B2=Comrak)
NATIVE_IMPLEMENTATION_LANE_DEFINED
RUST_ALGORITHM_REPRODUCTION_LANE_DEFINED
SAME_PAYLOAD_DEFINED
SAME_OPERATION_CONTRACT_DEFINED
TIMER_CONTRACT_DEFINED
VM_PROCESS_AND_WARMUP_POLICY_DEFINED
CAPABILITY_MATRIX_RULE_DEFINED
SELF_EQUIVALENCE_RULE_DEFINED
PA_DEFINITION_DEFINED
STRUCTURAL_FAMILIES_DEFINED
SAMPLING/RANDOMIZATION/FAILURE_POLICY_DEFINED
RESULT_SCHEMA_DEFINED
CONCLUSION_RULES_DEFINED
METHODOLOGY_REFERENCES_RECORDED
```

Stop：`READY_FOR_R0_CORRECTIVE_REVIEW`

**R0 未经人工 PASS，不得进入 R1。**

---

## R1 — Harness Substrate

目标：建立统一实验基础设施，不产生研究结论。

目标目录：

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/
├── manifest/
├── runner/
├── adapters/
│   ├── md4c/
│   ├── pulldown-cmark/
│   ├── comrak/
│   ├── tree-sitter-markdown/
│   ├── lezer-markdown/
│   └── mizchi-markdown/
├── reproductions/          # Rust controlled algorithm/mechanism reproductions
├── corpus/
│   ├── generators/
│   ├── mutations/
│   ├── real-world/
│   └── materialized/
├── oracle/
├── instrumentation/
├── scripts/
├── results/
│   ├── raw/
│   ├── normalized/
│   └── summary/
└── report/
```

Ownership：

- `runner/`：case enumeration、顺序、measurement lane、result schema；
- `adapters/`：只做 canonical operation -> native API mapping；
- `reproductions/`：只放通过独立 provenance/conformance gate 的 Rust controlled reproduction；
- `corpus/`：source + edit descriptor，不知道 parser；
- `oracle/`：correctness，不读 timing；
- `instrumentation/`：work/memory/allocation counters，不改变 operation；
- `results/raw/`：原始机器输出，默认不提交；
- `results/summary/`：schema-valid、带 provenance 的精选证据；
- `report/`：论文式分析，不包含 executable benchmark logic。

第三方代码优先 manifest + pinned fetch/build；vendor 必须记录 license/upstream/commit/SHA256/理由。

### Gate R1

- dummy/null adapter 完整走通 protocol；
- schema round-trip；
- case ID 稳定；
- 同 seed 生成相同 payload/edit；
- `T_prepare/T_native/T_total` schema 固定；
- timing 与 memory/allocation lane 分离。

Stop：`HARNESS_SUBSTRATE_PASS`

---

## R2 — Native Baseline Adapters + Capability Conformance

逐个接入 B0–B5，但先验证 capability/correctness，不比较快慢。

每个 adapter 产出 capability/provenance record。

### Gate R2

- FULL_PARSE smoke PASS；
- incremental subject SELF_EQUIVALENCE PASS；
- capability differences 明确；
- N/A / UNKNOWN 不伪装成支持；
- adapter 不拥有私有 workload、timer 或 scoring rule。

Stop：`BASELINE_CONFORMANCE_PASS`

---

## R3 — Corpus + Mutation Freeze

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

尺寸：`64 KiB / 256 KiB / 1 MiB / 4 MiB / 16 MiB`。

Real-world 至少固定 RW01 `CppCoreGuidelines.md` 的 repo commit、file SHA256、bytes、lines、encoding。额外 real-world corpus 如增加，必须在正式 measurement 前冻结。

Mutation families 覆盖 arbitrary edit + structural families。

### Gate R3

- corpus deterministic；
- mutation deterministic；
- edit boundary 合法；
- metadata 完整；
- real-world corpus 可重复 materialize；
- 正式 measurement 前 case ID 冻结。

Stop：`CORPUS_AND_MUTATION_FREEZE_PASS`

---

## R4 — Correctness Campaign

目标：性能前排除“快但做了不同工作”。

使用：

```text
CAPABILITY_MATRIX
SELF_EQUIVALENCE
LOSSLESS_ORACLE where supported
```

failure taxonomy：`BUG / CAPABILITY_DIFFERENCE / UNSUPPORTED / ORACLE_LIMITATION`。

### Gate R4

correctness/capability status 完整，失败 case 不进入不受限定的 headline comparison。

Stop：`CORRECTNESS_GATE_PASS`

---

## R5 — Native Full Parse Surface

建立真实 implementation rebuild floor：throughput / latency / CPU / memory / allocation（适用时）。

Event parser 与 retained AST/CST parser 分 capability 报告，不产生一个无条件“总冠军”。

Stop：`FULL_PARSE_SURFACE_PASS`

---

## R6 — Native Arbitrary Edit Microbench

矩阵：

```text
INSERT / DELETE / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK
× edit size
× position
× payload
× document size
```

输出至少：

```text
T_prepare / T_native / T_total
latency p50/p95/p99 (eligible only)
CPU/op
memory lane
allocation lane
edited bytes
PA if observable
```

Full-rebuild controls执行 post-edit clean rebuild。

Stop：`ARBITRARY_EDIT_SURFACE_PASS`

---

## R7 — Structural Edit Campaign

单独测 Markdown 结构传播。至少包括：

- paragraph/boundary；
- heading/ambiguity flips；
- list/blockquote/lazy continuation；
- fence forward-state；
- inline delimiter state；
- reference/semantic-global。

记录 logical edit bytes、affected syntax span、first stable suffix（可观察时）。

Stop：`STRUCTURAL_EDIT_SURFACE_PASS`

---

## R8 — Syntax Representation + Query

只针对 retained representation：

```text
retained bytes / source bytes
node count
offset -> enclosing block latency
full sequential traversal latency
```

Event-only parser：`N/A`，不得人为包 AST 参加。

Stop：`REPRESENTATION_SURFACE_PASS`

---

## R9 — Semantic Resolution Surface

第一轮聚焦 reference definition/users，分离 syntax propagation 与 semantic fanout。

控制 `F`、winner change、losing duplicate、add/delete definition、label rename。

外部 resolver 成本必须单列，不能算成 parser 成本。

Stop：`SEMANTIC_SURFACE_PASS`

---

## R10 — Scaling + Real World

变量：

```text
N document bytes
B block count
L enclosing/affected block size
D container depth
K edit size
F semantic fanout
```

输出先描述 scaling signature，不直接宣称根因。

Stop：`SCALING_AND_REALWORLD_SURFACE_PASS`

---

## R11 — Performance Attribution + Controlled Rust Reproduction

仅对稳定差异触发：

```text
稳定 >20%
OR scaling shape 明显不同
OR correctness/robustness cliff
```

归因顺序：

```text
1 scaling signature
2 minimal work/reuse counters
3 controlled counterexample/probe
4 Rust mechanism reproduction when it can isolate the cause
5 profiler only if still unresolved
```

Rust reproduction 必须先过 provenance + differential/conformance gate。

每个 candidate weakness 形成 Weakness Card：

```text
ID
payload
operation
native observation
scaling signature
mechanism hypothesis
work evidence
control/counterexample
Rust reproduction evidence (if used)
confidence
target class
Markit relevance
```

Stop：`ATTRIBUTION_PASS`

---

## R12 — Replication / Robustness

所有拟进入 Weakness Map 的结论：

- fresh process rerun；
- shuffled case order + fixed seed；
- 至少一次独立 measurement session；
- 环境漂移检查；
- correctness gate 重跑；
- 关键差异最小 reproducer。

不能稳定复现：`INCONCLUSIVE`。

Stop：`REPLICATION_PASS`

---

## R13 — Paper-style Report + Weakness Map

报告结构：

```text
1 Abstract / Executive Summary
2 Research Questions
3 Experimental Subjects
4 Methodology
5 Correctness and Capability
6 Native Full-Parse Results
7 Native Arbitrary-Edit Results
8 Structural-Edit Results
9 Representation / Query Results
10 Semantic Resolution Results
11 Scaling Analysis
12 Performance Attribution
13 Rust Controlled Reproductions
14 Weakness Map / Pareto Gaps
15 Threats to Validity / Limitations
16 Conclusions
17 Reproducibility Appendix
```

结论等级：

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_WEAKNESS
CROSS_IMPLEMENTATION_WEAKNESS
COMMON_WEAKNESS
PARETO_GAP
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

禁止：

```text
single timing -> algorithm superiority
correlation -> causality
one implementation weakness -> common weakness
Experiment 0 result -> #22 conclusion
benchmark weakness -> production architecture
invalid Rust port -> algorithmic evidence
```

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

# Phase 2 — YCSB-like Mixed Workload（暂定）

只有 R13 完成且 microbench 已解释主要机制后再考虑：

```text
typing-heavy
read/query-heavy
structural-edit-heavy
paste/delete-heavy
real editing trace
```

Phase 2 用于验证 microbench 结论在混合工作负载下是否仍成立，不替代第一轮机制归因。
