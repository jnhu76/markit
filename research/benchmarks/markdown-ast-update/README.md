# #22 — Controlled Rust Markdown Update Mechanism Benchmark

Status: **R0 PASS / READY FOR R1**  
Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**  
Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)  
R0 methodology: [`protocol/R0-METHODOLOGY.md`](./protocol/R0-METHODOLOGY.md)

本目录是 #22 的唯一实验代码、实验数据和研究报告工作区。

## 研究目标

第一阶段主实验是在**同一个 Rust 实验基底**中比较 Markdown AST/CST 更新机制：

```text
SAME Rust toolchain / build profile
SAME source representation
SAME BENCH-GRAMMAR-v1
SAME normalized result contract
SAME payload / edit
SAME runner / timer / instrumentation
        │
        ├── H0 FULL_REBUILD
        ├── H1 BLOCK_LOCAL_REPARSE
        ├── H2 FRAGMENT_REUSE
        ├── H3 OLD_TREE_SUBTREE_REUSE
        └── H4 RESTART_CONVERGENCE
```

目标不是选一个“冠军 parser”，而是得到每种机制在不同输入结构下的 strength/weakness profile，并解释这些差异。

## Prior-art / source subjects

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
Wagner & Graham incremental parsing
Swift incremental syntax parsing
```

它们用于源码/设计审计、机制抽取、provenance、sanity probe 和 fidelity 检查。原生绝对时间只作为 `REFERENCE_ONLY`，不进入统一 Rust 机制赛马的 headline ranking。

## 第一轮 horses

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

这些是 mechanism models，不得冒充“Rust 版 Tree-sitter/Lezer/mizchi”。如果只能复现思想而不能证明 faithful reproduction，必须使用 `*-inspired` 描述并记录 fidelity boundary。

## BENCH-GRAMMAR-v1

所有 horses 解决同一个受控 Markdown problem。

最低覆盖：

```text
paragraph / text
blank-line boundary
ATX heading
basic list / blockquote
fenced code
emphasis delimiter
code-span delimiter
inline/reference link basics
reference definition
```

第一轮不宣称完整 CommonMark conformance。

正确性：

```text
normalize(H1/H2/H3/H4 update result)
==
normalize(H0 clean full parse(post-edit source))
```

normalized correctness 只比较 semantic kind / tree topology / ordered children / UTF-8 spans / reference facts，不比较 pointer identity、NodeId、fragment ID 或 allocation identity。

## IMPLEMENTATION_PARITY_CONTRACT

为了避免实验变成“Flash 编程水平赛马”，R0 已冻结以下规则：

```text
one Rust workspace
one rustc/toolchain + Cargo.lock
one build profile / LTO / codegen-units / RUSTFLAGS policy
one allocator policy
shared Source/Edit/grammar/scanner/Node/result/runner/counters where semantics permit
horse-specific state only when mechanism requires it
```

第一轮禁止 horse-specific：

```text
custom allocator
unsafe unchecked fast path
SIMD/manual prefetch
parallelism
specialized hash/string representation
one-horse-only inline/cold tuning
```

机制不可分割的优化必须标 `MECHANISM_INTRINSIC`。

输入与结果统一使用 `black_box`/runner protection，关键 Weakness Map 候选还要做一次第二 compiler profile 的 optimization-sensitivity check。

## 目录权责

```text
research/benchmarks/markdown-ast-update/
├── README.md
├── ROADMAP.md
├── protocol/
├── manifest/
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

`common/` 只放非研究变量。若共享代码会抹掉机制成本，该代码必须留在对应 mechanism 内。

## Timer

正式边界以 `protocol/R0-METHODOLOGY.md` 为权威：

```text
T_prepare = mechanism-specific edit metadata preparation
T_native  = mechanism-required state maintenance + damage/restart/reuse/reparse/reconstruction/index work
T_total   = T_prepare + T_native
```

Host text-buffer apply-edit 在 timer 外；机制自己必须做的工作不得提前隐藏。

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

每种 mechanism 最终必须有：

```text
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

```text
R0 = PASS
NEXT = R1 Controlled Rust Harness / Directory Substrate
```

R1 只搭实验平台和 schema，不得开始 horse performance tuning 或 Markit production algorithm。