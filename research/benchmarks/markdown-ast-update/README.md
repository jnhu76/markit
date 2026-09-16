# #22 — Standardized Markdown AST/CST Update Benchmark

Status: **ACTIVE / PROTOCOL-FIRST**  
Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**  
Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)  
R0 methodology: [`protocol/R0-METHODOLOGY.md`](./protocol/R0-METHODOLOGY.md)

本目录是 #22 的**唯一实验代码与实验数据工作区**。所有 benchmark harness、baseline adapter、corpus、mutation、oracle、instrumentation、raw/normalized results 与最终 report 都必须在这里管理；不得重新向产品 `crates/`、Experiment 0 archive 或仓库其他目录散落实验实现。

## 研究对象

第一轮固定 6 个 design points：

```text
B0 MD4C                       full-rebuild control / SAX-callback
B1 pulldown-cmark             full-rebuild control / Rust events
B2 Comrak                     full-rebuild control / retained AST
B3 tree-sitter-markdown       generic incremental CST
B4 @lezer/markdown            Markdown-specific incremental fragments
B5 mizchi/markdown            lossless/incremental CST
```

这些对象不是“六个完全同类 parser”。只有 `CORE-COMPARABLE` cases（capability supported + correctness PASS + operation semantics equivalent）才允许进入跨实现 headline comparison；其他 case 必须标 `CAPABILITY-SPECIFIC` / `CAPABILITY_DIFFERENCE`。

## 两条比较 Lane

```text
Lane A — Native Implementation Performance
  upstream 原实现 / 原 runtime
  -> latency / throughput / CPU / memory / allocation
  -> 回答“现实实现成本是多少”

Lane B — Rust Algorithm / Mechanism Reproduction
  仅在归因需要时做最小 controlled reproduction / ablation
  -> work / reuse / scaling / causal probe
  -> 回答“机制为什么这样”
```

禁止把 Lane A 的 upstream absolute time 与 Lane B 的 Rust reproduction absolute time 放进同一排名。

## 目录权责

#22 实现阶段按以下结构演进：

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/                 # 冻结实验语义、schema、timer / measurement boundary
├── manifest/                 # baseline/corpus/environment provenance
├── runner/                   # 唯一 case enumeration + orchestration
├── adapters/                 # baseline-specific glue；禁止私有 workload/评分逻辑
├── corpus/
│   ├── generators/
│   ├── mutations/
│   ├── real-world/
│   └── materialized/         # generated; normally ignored
├── oracle/                   # correctness only；不得读取 timing 结果
├── instrumentation/          # work counters / memory / allocation lanes
├── scripts/                  # reproducible setup/fetch/build helpers
├── results/
│   ├── raw/                  # machine output; ignored
│   ├── normalized/           # generated normalized data
│   └── summary/              # curated, provenance-bearing evidence
└── report/                   # paper-style analysis + Weakness Map
```

### 强制边界

- `runner/` 定义统一 operation、case ID、运行顺序、measurement lane 与结果 schema；
- `adapters/` 只做 API 映射，不允许各自发明 workload、计时边界或结论；
- `corpus/` 不依赖任何 parser；
- `oracle/` 只决定 correctness；
- `protocol/R0-METHODOLOGY.md` 是 timer/capability/self-equivalence/PA/结论晋级规则的当前冻结点；
- wall-clock timing 与 memory/allocation/work instrumentation 分 lane；
- 第三方代码优先通过 pinned manifest + reproducible fetch/build 获取；非必要不 vendor；
- Experiment 0 (`research/experiments/experiment-0-parser-survey/`) 只允许作为历史证据，不得成为 #22 的隐式第七个 baseline 或 Markit candidate。

## 实验方法

第一轮只执行两套主方法：

1. **Swift incremental syntax parsing 风格**：incremental result 必须与同 parser 的 clean full parse 等价；同时观察真实重新处理工作量与 scaling。
2. **db_bench 风格**：按 `FULL_PARSE / INSERT / DELETE / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK / STRUCTURAL_EDIT / QUERY` 分操作测试成本。

YCSB-like mixed workload 只保留为 Phase 2；第一轮不混入。

### 第一轮 Structural Edit 最低覆盖

必须覆盖六类传播机制：

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

不是只测 block-level Markdown。

## Headline metrics

```text
Throughput
Latency
CPU
Memory
Allocation
Correctness
Parse Amplification (diagnostic / A-LANE only)
```

不可观测的内部 work counter 必须记 `UNKNOWN`，不能用 wall-clock、changed_ranges 或 node count 倒推。

## Timer 摘要

正式细节见 R0 methodology；adapter 不得自行改变。

```text
T_prepare = canonical edit -> parser-native coordinate/change metadata
T_native  = parser-required state/input work + parse/update to completion
T_total   = T_prepare + T_native
```

Host text-buffer 应用 edit 在 parser timer 外；parser 私有 copy/normalize/coordinate/state maintenance 不得隐藏到 timer 外。

## 第一次测量策略

第一轮故意简单但可复现：

```text
3 independent sessions
10 warmup iterations/session
30 measured iterations/session
fixed recorded shuffle seed
report p50 + p95
no p99
no outlier deletion
```

任何失败都必须显式保留，不能静默删除 case。

## 结论纪律

#22 不选“冠军 parser”，也不直接产出 Markit 算法。结论只能逐级晋升：

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

Raw cross-runtime timing 最多直接建立 implementation observation；算法归因必须依赖 scaling/work evidence + controlled probe/counterexample，必要时才用 Rust reproduction。

最终产物是 Performance Surface、Weakness Cards 和 Weakness Map。只有经人工 review 后，下一独立 campaign 才允许针对挣得的 weakness 设计 Markit-specific Markdown algorithm。

## 当前 Gate

当前阶段：**R0 Protocol Freeze / final review**。

禁止在 R0 review 通过前：

- 搭建正式 baseline adapter；
- 跑正式性能结果；
- 因 smoke 数字调整 payload；
- 写 Rust algorithm reproduction；
- 实现 Markit-specific parser；
- 写形式化证明；
- 冻结 architecture。

历史前身：Experiment 0 (#19 / PR #20)，只作为 reconnaissance / historical evidence。
