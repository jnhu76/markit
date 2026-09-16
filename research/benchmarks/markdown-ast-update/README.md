# #22 — Controlled Rust Markdown Update Mechanism Benchmark

Status: **ACTIVE / R0 FINAL REVIEW**  
Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**  
Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)  
R0 methodology: [`protocol/R0-METHODOLOGY.md`](./protocol/R0-METHODOLOGY.md)

本目录是 #22 的唯一实验代码、实验数据和研究报告工作区。

## 研究目标

第一阶段主实验不是比较六个 upstream parser 的工程实现，而是在**同一个 Rust 实验基底**中比较 Markdown AST/CST 更新机制：

```text
SAME Rust toolchain
SAME source representation
SAME BENCH-GRAMMAR-v1
SAME normalized result contract
SAME payload
SAME edit
SAME timer / instrumentation
        │
        ├── H0 FULL_REBUILD
        ├── H1 BLOCK_LOCAL
        ├── H2 FRAGMENT_REUSE
        └── H3 OLD_TREE_REUSE_CONVERGENCE
```

目标是形成每种机制的 strength/weakness profile，并解释差异来自哪里，而不是只产生一个速度排名。

## Prior-art / source subjects

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
```

这些项目用于：源码/设计审计、机制抽取、优势/劣势假设、必要的 sanity probe 和 fidelity 检查。

它们的原生跨语言绝对时间可以作为 `REFERENCE_ONLY` 数据，但不进入统一 Rust 机制赛马的 headline ranking。

## 第一轮 horses

```text
H0 FULL_REBUILD
  clean full parse + rebuild state

H1 BLOCK_LOCAL
  locate affected block region + local reparse + sequence/index repair

H2 FRAGMENT_REUSE
  retained fragment/subtree reuse + damaged gaps reparse

H3 OLD_TREE_REUSE_CONVERGENCE
  restart + changed-region parse + convergence + reusable suffix
```

这些是 mechanism models，不得冒充“Rust 版 Tree-sitter/Lezer/mizchi”。如果只能复现思想而不能证明 faithful reproduction，必须使用 `*-inspired` 描述。

## 公平性

所有 horses 必须解决同一个受控 parsing problem：`BENCH-GRAMMAR-v1`，并输出相同 normalized semantic/syntax contract。

第一轮 grammar 最低覆盖：

```text
paragraph / text
blank-line boundary
ATX heading
basic list / blockquote
fenced code
emphasis delimiter
code span delimiter
inline/reference link basics
reference definition
```

正确性：

```text
H1/H2/H3 update result
==
H0 clean full parse(post-edit source)
```

不要求第一轮实现完整 CommonMark；扩大 standards scope 必须另做 protocol amendment。

## 目录权责

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/                 # grammar/result/timer/metric schema
├── manifest/                 # toolchain/corpus/environment/prior-art provenance
├── runner/                   # Rust case enumeration + orchestration
├── mechanisms/               # H0-H3；只放被测机制
│   ├── full-rebuild/
│   ├── block-local/
│   ├── fragment-reuse/
│   └── old-tree-convergence/
├── common/                   # 所有 horses 共享且不属于研究变量的代码
├── corpus/
│   ├── generators/
│   ├── mutations/
│   ├── real-world/
│   └── materialized/
├── oracle/                   # H0/full-parse equivalence + schema checks
├── instrumentation/          # work/memory/allocation counters
├── prior-art/                # upstream notes/probes/fidelity records，不放 headline 实现
├── scripts/
├── results/
│   ├── raw/
│   ├── normalized/
│   └── summary/
└── report/
```

`common/` 只能放非研究变量；如果共享某段代码会抹平机制差异，该代码必须留在对应 mechanism 内。

## Timer

正式边界以 `protocol/R0-METHODOLOGY.md` 为权威：

```text
T_prepare = mechanism-specific edit metadata preparation
T_native  = damage/restart/reuse/reparse/reconstruction/index maintenance
T_total   = T_prepare + T_native
```

Host text-buffer apply-edit 在 timer 外；mechanism 自己必须做的工作不得提前隐藏。

## 第一轮 measurements

Headline：

```text
latency p50 / p95
full-parse throughput
CPU time
peak / retained memory
allocation count / bytes
```

Mechanism counters：

```text
source coverage / PA
blocks reparsed
nodes rebuilt / reused
metadata records touched
restart distance
convergence distance
fallback-to-full count
```

第一次不强制 PMU/cache profiling。只有 algorithmic work 已接近、wall-clock 仍有稳定残差时，才进入 cycles/instructions/cache/branch attribution。

## Structural minimum

至少覆盖：

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

## Sampling

```text
3 independent sessions
10 warmup iterations/session
30 measured iterations/session
fixed recorded shuffled case order
report p50 + p95
no p99
no outlier deletion
```

## 结论

最终不是“冠军 parser”，而是每种 mechanism 的：

```text
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

结论等级：

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

Weakness Map 人工 review 前，不允许设计或实现 Markit-specific production parser。

## 当前 Gate

当前仍是 **R0 final review**。

R0 通过后才进入：

```text
R1 — Controlled Rust harness / directory substrate
```

在此之前禁止跑正式结果或写 H0-H3 实现。
