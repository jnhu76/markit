# #22 — Standardized Markdown AST/CST Update Benchmark

Status: **ACTIVE / PROTOCOL-FIRST**  
Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**  
Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)

本目录是 #22 的**唯一实验代码与实验数据工作区**。所有 benchmark harness、baseline adapter、corpus、mutation、oracle、instrumentation、raw/normalized results 与最终 report 都必须在这里管理；不得重新向产品 `crates/`、Experiment 0 archive 或仓库其他目录散落实验实现。

## 研究对象

第一轮固定 6 个设计点：

```text
B0 MD4C                       full-rebuild control / SAX-callback
B1 pulldown-cmark             full-rebuild control / Rust events
B2 Comrak or cmark-gfm        full-rebuild control / retained AST (Stage 0 二选一)
B3 tree-sitter-markdown       generic incremental CST
B4 @lezer/markdown            Markdown-specific incremental fragments
B5 mizchi/markdown            lossless/incremental CST
```

这些对象不是“六个完全同类 parser”。比较必须先经过 capability classification 与 correctness gate。

## 目录权责

#22 实现阶段按以下结构演进：

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/                 # 冻结实验语义、schema、measurement boundary
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
- wall-clock timing 与 memory/allocation instrumentation 分 lane，避免 instrumentation 污染 headline latency；
- 第三方代码优先通过 pinned manifest + reproducible fetch/build 获取；非必要不 vendor；
- Experiment 0 (`research/experiments/experiment-0-parser-survey/`) 只允许作为历史证据，不得成为 #22 的隐式第七个 baseline 或 Markit candidate。

## 实验方法

第一轮只执行两套主方法：

1. **Swift incremental syntax parsing 风格**：incremental result 必须与同 parser 的 clean full parse 等价；同时观察实际重新处理工作量与 scaling。
2. **db_bench 风格**：按 `FULL_PARSE / INSERT / DELETE / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK / STRUCTURAL_EDIT / QUERY` 分操作测试成本。

YCSB-like mixed workload 只保留为 Phase 2；第一轮不混入。

## Headline metrics

```text
Throughput
Latency
CPU
Memory
Allocation
Correctness
Parse Amplification (diagnostic)
```

不可观测的内部 work counter 必须记 `UNKNOWN`，不能用 wall-clock 倒推。

## 结论纪律

#22 不选“冠军 parser”，也不直接产出 Markit 算法。最终只允许从以下等级逐级推进：

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_WEAKNESS
COMMON_WEAKNESS
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

最终产物是性能曲面、Weakness Cards 和 Weakness Map。只有经人工 review 后，下一独立 campaign 才允许针对挣得的 weakness 设计 Markit-specific Markdown algorithm。

## 当前 Gate

当前阶段：**R0 Protocol Freeze**。

禁止在 R0 review 前：

- 跑正式性能结果；
- 因单个 smoke 数字调整 payload；
- 实现 Markit-specific parser；
- 写形式化证明；
- 冻结 architecture。

历史前身：Experiment 0 (#19 / PR #20)，只作为 reconnaissance / historical evidence。
