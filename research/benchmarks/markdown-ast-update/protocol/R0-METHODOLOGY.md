# R0 Methodology — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R0 ADVERSARIAL CORRECTIVE / PROTOCOL ONLY / NO BENCHMARK EXECUTION**

Authority: GitHub Issue #22 and its R0 review comments.

本文件是 #22 第一轮实验的方法学冻结点。目标不是一次做完所有性能研究，而是保证第一轮已经形成一个可以复现、可以解释、不会从单点 timing 跳到算法结论的最小实验闭环。

核心研究链：

```text
Research Question
-> SAME payload / SAME operation
-> capability parity
-> correctness gate
-> native implementation measurement
-> scaling/work observation
-> controlled attribution
-> optional Rust mechanism reproduction
-> replication
-> bounded conclusion
```

任何一步证据不足都必须停在 `INCONCLUSIVE`。

---

## 1. Two comparison lanes

### Lane A — Native Implementation Performance

在各 upstream 官方支持的 runtime/toolchain 中运行原实现，对同一 canonical payload 和 operation contract 测量实际工程实现。

Subjects:

- B0 MD4C
- B1 pulldown-cmark
- B2 Comrak
- B3 tree-sitter-markdown
- B4 @lezer/markdown
- B5 mizchi/markdown

Measures:

- latency
- throughput
- CPU
- memory
- allocation（仅在可观测且定义可比时）

允许结论：

> 在冻结环境、版本、payload、operation、capability 条件下，某个**实现**的实际成本是多少。

禁止结论：

> 仅凭跨 runtime wall-clock，宣称某个 parsing algorithm 天生优于另一个 algorithm。

Lane A 比较的是 implementations，不是 languages，也不是未经控制的 abstract algorithms。

### Lane B — Rust Algorithm / Mechanism Reproduction

Rust 是后续 controlled attribution 的统一复现语言。

选择 Rust 的原因：

- 无 GC，减少额外 runtime confounder；
- allocation/layout/ownership 可显式控制；
- 可直接接 C FFI；
- 与 Rust baselines 自然集成；
- 适合 retained-tree / incremental parser mechanism 的局部复现和 ablation。

但 Rust reproduction **不是 upstream baseline，也不是翻译比赛**。

默认规则：

```text
先在 Lane A 观察稳定差异
-> scaling/work evidence 形成 mechanism hypothesis
-> 只有需要验证该 hypothesis 时才做最小 Rust reproduction
```

禁止为了形式对称把六个 parser 全量翻译成 Rust。

每个 reproduction 必须记录：

- upstream project + exact version/commit；
- 被复现的算法/数据结构边界；
- omitted features；
- representation differences；
- port-specific decisions；
- differential/conformance tests against upstream；
- reproduction-specific allocation/layout assumptions。

验证失败：

```text
REPRODUCTION_INVALID
```

不得产生算法结论。

### Cross-lane hard rule

```text
Native upstream absolute time
!=
Rust reproduction absolute time
```

两者禁止直接放进同一绝对性能排名。

Rust lane 只允许做：

- 同语言 controlled comparison；
- mechanism ablation；
- scaling shape validation；
- causal probe / counterexample。

---

## 2. Comparable-work gate

“相同字节”不自动等于“做了相同语义工作”。

因此第一轮结果分为两类 case。

### CORE-COMPARABLE

只有当参与比较的 implementations 对该 case：

```text
capability supported
AND
correctness gate PASS
AND
operation semantics equivalent
```

才允许进入跨 implementation headline comparison。

### CAPABILITY-SPECIFIC

如果某 implementation 对特性不支持、降级为普通文本、采用不同 dialect，或实际完成的语义工作明显不同：

```text
CAPABILITY_DIFFERENCE
```

该 case 可以保留实现级 timing，但不得和 CORE-COMPARABLE case 混成“谁更快”的算法结论。

Capability Matrix 的作用就是定义哪些 cases 有资格进入 shared comparison surface。

---

## 3. Canonical operation contract

Canonical source authority：UTF-8 source bytes。

Canonical edit descriptor：

```text
UTF-8 byte range [start,end)
+
inserted bytes
```

所有 payload、mutation、case ID 都基于这个 descriptor。

Adapter 可以转换成 native API 坐标，但不得改变 logical edit。

### Host text mutation boundary

本 benchmark 第一轮研究 parser / syntax update，而不是 text-buffer data structure。

因此以下统一在 parser timer 外：

- canonical source fixture 生成；
- host/source buffer 应用 edit；
- post-edit source materialization。

但 parser 为其 incremental API 必须执行的坐标转换、旧状态维护、私有输入复制/规范化不能借此逃出 timer；具体规则见下一节。

---

## 4. Timer contract

Flash / adapter implementation **不得自行解释 timer 范围**。以下为权威边界。

### 4.1 FULL_PARSE

Outside timer：

- file IO；
- fixture/corpus materialization；
- process/runtime startup；
- one-time parser/runtime initialization；
- correctness oracle；
- logging / result serialization。

#### Native source representation

在计时前，source 可以被 materialize 为该 runtime 的正常 host text representation，例如 Rust `&str` / JS string。

这意味着第一轮 `FULL_PARSE` 测的是 **parser-core from already-in-memory native text**，不是 UTF-8 file loading/decoding benchmark。

但是：

> 如果 parser API 在收到正常 host text 后仍必须复制、转码、normalize 或构造 parser-private input buffer，这属于 parser-required work，必须计入 `T_native`。

`T_native`：

```text
START
parser receives the already-in-memory native text
parser performs all parser-required input preparation
parser consumes the complete source
force native result to completion
  event/pull parser: consume ALL events
  tree parser: complete tree/result construction
STOP
```

Lazy parser 不能只构造 iterator/object 就停表。

### 4.2 UPDATE / STRUCTURAL_EDIT

Outside timer：

- old source materialized；
- old parse state 已在 case setup 阶段构造；
- canonical edit selected；
- host text mutation applied；
- post-edit source materialized。

`T_prepare`：

```text
canonical UTF-8 edit
-> parser-native coordinates / change descriptor
```

包括 parser API 要求的：

- byte -> row/column conversion；
- byte -> UTF-16/code-unit conversion；
- parser-specific edit metadata construction。

`T_native`：

```text
START
parser-required old-state maintenance
parser-required private input preparation/copy/normalization
incremental parse/update to completion
STOP
```

Headline：

```text
T_total = T_prepare + T_native
```

必须保存：

```text
T_prepare
T_native
T_total
```

任何 parser-required coordinate/state/input work 都不得为了让数据好看被偷偷移到 timer 外。

### 4.3 Full-rebuild controls under UPDATE

B0–B2 的 edit/update control：

```text
host applies canonical edit outside timer
post-edit native text is ready
T_native = clean full rebuild to completion
```

其 `T_prepare = 0`，除非该 parser API 自身还要求 parser-specific preparation。

### 4.4 Node/VM adapters

如果 Rust runner 通过 IPC 调用 Node/JS：

- `T_native` 必须在 Node/JS runtime 内部计时；
- IPC / pipe / serialization / scheduler delay 不进入 `T_native`；
- 可以单独报告 `T_adapter_roundtrip`；
- process model / warmup / GC policy 必须固定。

`T_adapter_roundtrip` 只能作为 integration diagnostic，不能冒充 parser-core latency。

### 4.5 QUERY

Outside timer：

- query case/offset 已预生成；

`T_prepare`：

- canonical byte offset -> native query coordinate；

`T_native`：

- actual query / traversal。

第一轮只要求：

```text
Q1 offset -> enclosing block/syntax region
Q2 full sequential syntax traversal
```

不在第一轮强求 deepest-CST-node 等更细 API。

---

## 5. Correctness contract

第一轮不人工维护每个 arbitrary payload 的完整 expected AST。

### C1 — Capability Matrix

用官方 CommonMark/GFM examples 和 upstream documented capabilities 建立：

```text
supported
divergent
unsupported
unknown
```

Capability Matrix 是跨 implementation comparison 的上下文和资格表，不在每次 timing loop 内执行。

### C2 — Incremental Self-Equivalence

对 incremental subjects：

```text
normalize(incremental(post-edit))
==
normalize(same-parser clean full parse(post-edit))
```

这是 timing case 的 primary correctness gate。

如果 self-equivalence 失败：

```text
WRONG_RESULT
```

该 case 不能进入 headline performance conclusion。

### C3 — Cross-implementation semantic parity

跨 implementation 比较前必须检查该 case 是否属于 `CORE-COMPARABLE`。

如果 outputs / capability 表明 implementations 实际执行不同语义工作：

```text
CAPABILITY_DIFFERENCE
```

允许保留数据，但结论必须限定为实现观察，不能偷换成 algorithm superiority。

---

## 6. Structural Edit minimum set

第一轮可以小，但必须覆盖不同传播机制。

### Block/boundary

- paragraph split / merge；
- blank-line insert/delete；
- ATX or Setext heading conversion。

### Container/state

- list indent +/-1；
- blockquote depth change；
- lazy continuation boundary change。

### Forward-state

- fence opener/closer insert/delete；
- closed <-> unclosed fence。

### Inline structural

至少：

- emphasis delimiter insert/delete；
- code-span delimiter insert/delete；
- link/reference bracket or destination delimiter edit。

这是为了覆盖 HUGE_BLOCK 中 inline delimiter propagation，不能只测 block-level structure。

### Semantic-global

- reference definition target change；
- add/delete/rename definition；
- duplicate definition / winner change。

第一轮不要求覆盖所有 Markdown 语法；但必须至少覆盖：

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

---

## 7. Measurement lanes

为避免 instrumentation 改变 headline latency：

```text
T-LANE  timing + CPU
M-LANE  memory
A-LANE  allocation + parser/work instrumentation
```

禁止在 A-LANE 的 instrumented build 上报告 headline latency。

### First-round sampling policy

第一轮故意保持简单：

```text
independent sessions: 3
warmup iterations/session: 10
measured iterations/session: 30
case order: shuffled with recorded fixed seed
headline latency: p50 + p95
p99: NOT REPORTED in first round
outlier deletion: NONE
```

Node/JIT runtime：

- 使用 persistent process per session；
- warmup policy 与 native 保持可记录、可复现；
- 不宣称 10 次 warmup 必然达到 steady state；
- 保存 iteration-order raw timing，若存在明显 warmup/non-stationarity 则结论降级并进入 attribution/replication。

第一轮的目标是得到稳定方向和 scaling surface，不是生产级 tail-latency SLO。

### Environment record

每次正式 run 至少记录：

```text
OS / kernel
CPU model
CPU affinity
SMT state if known
frequency/turbo policy (record; not necessarily disabled)
runtime/compiler/toolchain
build profile
baseline commit/version/features
runner commit
corpus manifest version
operation manifest version
sampling policy version
case-order seed
```

---

## 8. Parse Amplification

第一版：

```text
PA = unique source-byte coverage re-inspected / logical edited bytes
```

`unique source-byte coverage` 的操作定义：

> edit 后 parser/tokenizer/scanner 实际重新进入的 source byte intervals 的并集大小。

它不是：

- changed AST node range；
- Tree-sitter changed_ranges；
- wall-clock 的反推；
- node count 的反推。

PA 只能在 A-LANE / attribution instrumentation 下取得。

如果 parser 没有可靠 instrumentation：

```text
PA = UNKNOWN
```

不同 parser 的 PA 只有在 instrumentation 语义等价时才能横向比较；否则只用于同 implementation 内 scaling/ablation。

Repeated reads 不在 PA 中重复计数；如可观察，可另记 `total_byte_read_work`。

Structural edit 还应记录：

```text
observed affected syntax span
first stable/reusable suffix
```

如果不可观察则 `UNKNOWN`。

---

## 9. Failure taxonomy

任何 case 都不能因为失败而静默删除。

第一轮至少分类：

```text
PASS
WRONG_RESULT
CAPABILITY_DIFFERENCE
UNSUPPORTED
TIMEOUT
OOM
STACK_OVERFLOW
CRASH
ADAPTER_FAILURE
INSTRUMENTATION_UNAVAILABLE
```

默认单 case timeout：30 s；如后续 protocol amendment 修改，必须重新标版本。

---

## 10. Conclusion levels

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

### Promotion rules

`OBSERVATION`
- 单次实验面上的事实，例如 implementation A 在 case X 的 median latency 高于 B。

`REPRODUCED_OBSERVATION`
- 至少跨独立 measurement session 保持方向和量级。

`ATTRIBUTED_WEAKNESS`
- scaling evidence + work/reuse evidence + controlled probe/counterexample 支持一个机制根因。

`CROSS_IMPLEMENTATION_WEAKNESS`
- 至少两个独立 implementations 出现同类 scaling/mechanism evidence。

`COMMON_WEAKNESS`
- 必须显式声明 target class，例如 `B3+B4+B5`，且该 class 全部满足证据条件。

`PARETO_GAP`
- 已测实现中没有对象同时满足预先声明的目标约束；不要求所有实现拥有同一个机制缺陷。

`DESIGN_OPPORTUNITY`
- 只能从具备真实编辑相关性的 `COMMON_WEAKNESS` 或 `PARETO_GAP` 升级，并仍然不是 Markit algorithm 设计本身。

Raw native timing 永远最多直接建立 implementation observation。

Rust reproduction 只为 attribution 提供 controlled evidence；不能单独创造 `COMMON_WEAKNESS`。

---

## 11. R0 preregistration checklist

进入 R1 前必须已有：

- exact baseline versions/commits/features/build profiles 的登记槽位；
- B2 = Comrak；
- Lane A native implementation / Lane B Rust reproduction boundary；
- CORE-COMPARABLE / CAPABILITY-SPECIFIC rule；
- canonical UTF-8 operation/edit contract；
- source-representation/copy/encoding timer rule；
- `T_prepare/T_native/T_total` contract；
- VM/Node process model；
- first-round sampling policy；
- case randomization seed rule；
- no-outlier-deletion rule；
- memory lane / allocation lane 与 timing lane 分离；
- failure taxonomy；
- capability matrix / self-equivalence policy；
- minimal Structural Edit six-mechanism coverage；
- PA operational definition + UNKNOWN rule；
- conclusion promotion ladder。

具体 baseline SHA、toolchain version、CPU 等值在 R1/R2 materialization 时填入 manifest，但**字段和语义不能在看到性能结果后才发明**。

---

## 12. Methodology references and what we borrow

1. Tim A. Wagner, Susan L. Graham. **Efficient and Flexible Incremental Parsing**. ACM TOPLAS 20(5), 1998. DOI: https://doi.org/10.1145/293677.293678
   - incremental work、reuse、retained structure、scaling。

2. Alex Hoppen. **Swift incremental syntax parsing** proposal/discussion, 2018. https://forums.swift.org/t/incremental-syntax-parsing/12368
   - incremental vs clean parse、reuse/work amount、source-size scaling。

3. Stefan Marr, Benoit Daloze, Hanspeter Mössenböck. **Cross-Language Compiler Benchmarking: Are We Fast Yet?** DLS 2016. DOI: https://doi.org/10.1145/2989225.2989232
   - heterogeneous implementations 上使用 common abstraction/workload；比较 implementations，避免把语言/运行时差异偷换成算法结论。

4. Andy Georges, Dries Buytaert, Lieven Eeckhout. **Statistically Rigorous Java Performance Evaluation**. OOPSLA 2007. DOI: https://doi.org/10.1145/1297027.1297033
   - repeated runs、runtime variation、统计纪律。

5. Edd Barrett et al. **Virtual Machine Warmup Blows Hot and Cold**. OOPSLA 2017. DOI: https://doi.org/10.1145/3133876
   - JIT/VM warmup 不能想当然地视为稳定 steady state。

6. Todd Mytkowicz et al. **Producing Wrong Data Without Doing Anything Obviously Wrong**. ASPLOS 2009. https://research.ibm.com/publications/producing-wrong-data-without-doing-anything-obviously-wrong
   - measurement bias、setup randomization、避免偶然布局/执行顺序影响结论。

7. Brian F. Cooper et al. **Benchmarking Cloud Serving Systems with YCSB**. SoCC 2010. DOI: https://doi.org/10.1145/1807128.1807152
   - heterogeneous systems 共享 operation/workload contract；mixed workload 留到 Phase 2。

8. RocksDB `db_bench`. https://github.com/facebook/rocksdb/wiki/Benchmarking-tools
   - operation-oriented microbenchmark；这里映射为 full parse / insert / delete / replace / query / structural edit。

9. Catherine C. McGeoch. **A Guide to Experimental Algorithmics**. Cambridge University Press, 2012.
   - experimental algorithmics 强调用受控实验理解 algorithms/programs，而不是只看单一 runtime 数字；实验数据和 test environment 本身是研究资产。

10. Tomas Kalibera, Richard E. Jones. **Rigorous Benchmarking in Reasonable Time**. ISMM 2013. DOI: https://doi.org/10.1145/2464157.2464160
    - 系统存在多层不确定性；独立重复、effect-size/uncertainty 和合理的实验预算比一次大量循环更可信。

11. Catherine McGeoch et al. **Using Finite Experiments to Study Asymptotic Performance**. Experimental Algorithmics, 2002. DOI: https://doi.org/10.1007/3-540-36383-1_5
    - 有限输入实验可以支持 scaling hypothesis，但不能把经验 scaling 曲线自动升级为数学复杂度证明。

这些来源支撑实验方法，不支撑未来 Markit algorithm 的 novelty claim。

---

## 13. R0 adversarial-review verdict

第一次实验允许缩减 scope，但基础链必须完整。

本 corrective 后，R0 的最低逻辑闭环为：

```text
same logical problem
-> comparable capability
-> fixed timer contract
-> correctness/self-equivalence
-> implementation measurements
-> scaling/work evidence
-> controlled attribution if needed
-> independent rerun
-> bounded conclusion
```

R0 不要求：

- 完整实现所有 Markdown dialect；
- 所有 parser 都有 PA；
- 第一轮报告 p99；
- 第一轮就做 YCSB mixed workload；
- 第一轮把六个 parser 全部重写成 Rust；
- 第一轮直接设计 Markit parser。

R0 要求：

```text
NO HIDDEN TIMER WORK
NO CAPABILITY MISMATCH IN HEADLINE COMPARISON
NO SELF-EQUIVALENCE FAILURE IN PERFORMANCE CLAIM
NO CROSS-LANE ABSOLUTE RANKING
NO SINGLE-POINT -> ALGORITHM CAUSALITY
NO SILENT FAILURE DROPPING
```

Verdict after corrective: **READY_FOR_R0_FINAL_REVIEW**.
