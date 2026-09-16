# R0 Methodology — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R0 CORRECTIVE-2 / CONTROLLED RUST MECHANISM RACE / AUTHORITATIVE**

Authority: GitHub Issue #22 plus the latest R0 corrective comment. Where older #22 body text still describes native cross-runtime baseline ranking or a dual-lane primary design, **this file supersedes that older methodology**.

## 0. Research question

#22 第一阶段不再以“六个 upstream parser 谁更快”为主问题。

主问题改为：

> **在同一 Rust 实验基底、同一 Markdown 语义核心、同一 payload、同一 edit、同一输出契约下，不同 AST/CST 更新机制分别付出什么成本、在什么结构上失效、为什么失效？**

目标是研究 mechanism，而不是语言/runtime/工程优化差异。

研究链：

```text
prior-art/source reconnaissance
-> extract mechanism
-> common Rust experimental substrate
-> SAME parser semantics / SAME payload / SAME edit / SAME result contract
-> mechanism race
-> scaling + work counters
-> attribution
-> replication
-> strength / weakness profile
-> Weakness Map
```

证据不足必须停在 `INCONCLUSIVE`。

---

## 1. Primary experiment: one Rust substrate

第一阶段 headline benchmark 只比较统一 Rust 实验实现。

共享并冻结的非研究变量：

```text
Rust toolchain / build profile
source representation
canonical edit descriptor
benchmark grammar / semantic contract
normalized syntax output contract
node semantic vocabulary
corpus + mutations
timer implementation
allocator/instrumentation policy
result schema
machine/environment
```

允许不同的研究变量：

```text
damage detection
restart strategy
reuse strategy
convergence strategy
retained state
reconstruction strategy
position/range maintenance strategy
fallback policy
```

原则：**能共享的非研究代码尽量共享；会改变机制语义的部分不得为了“统一”而共享。**

统一 substrate 的目的，是尽可能把 implementation choices 从“算法机制比较”中消掉；与此同时，所有 mechanism reproduction 必须显式记录 fidelity boundary，避免把 Flash 自己的重实现误称为 upstream 原算法。

---

## 2. Prior art role

以下项目仍然是 #22 的主要 prior-art/source subjects：

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
```

它们的作用：

1. 读取源码/设计文档，抽取 parsing/update mechanism；
2. 记录各自的 restart/reuse/tree/position/semantic strategy；
3. 必要时在少量相同 payload 上运行 upstream，实现 sanity probe；
4. 帮助验证 Rust mechanism model 是否抓住了原设计的关键行为；
5. 提供优势/劣势假设，不直接提供 #22 的最终 algorithm ranking。

Upstream timing 可以记录为 `REFERENCE_ONLY`，但不进入统一 Rust race 的 headline table。

禁止：

```text
upstream C/Rust/JS absolute timing
-> 直接推断 algorithm superiority
```

---

## 3. First-round horses

第一次实验可以少做，但必须形成完整闭环。第一阶段先固定 4 个 mechanism models。

### H0 — FULL_REBUILD

Control。

```text
post-edit source
-> clean full parse
-> rebuild normalized syntax state
```

用途：建立“什么都不复用”的成本对照。

Prior-art inspiration：clean full-parse routes such as MD4C / pulldown-cmark / Comrak。

### H1 — BLOCK_LOCAL

```text
identify affected Markdown block region
-> reparse affected block(s)
-> preserve unaffected block states
-> repair document sequence/index
```

重点变量：block count `B`、largest/affected block length `L`、block boundary mutation。

Prior-art inspiration：block-oriented incremental Markdown approaches，包括 mizchi/markdown 所代表的设计点。

### H2 — FRAGMENT_REUSE

```text
retain reusable syntax fragments/subtrees
-> map edit through retained fragments
-> parse gaps/damaged regions
-> compose new syntax state
```

重点变量：fragment granularity、edit position、fragment invalidation、large leaf/block。

Prior-art inspiration：Lezer-style reusable fragments/tree reuse。

### H3 — OLD_TREE_REUSE_CONVERGENCE

```text
old syntax state + edit
-> restart from valid context
-> incrementally parse changed region
-> detect convergence / reusable suffix
-> reuse old tree/state beyond convergence
```

重点变量：restart distance、forward-state propagation、container depth、convergence point。

Prior-art inspiration：incremental parsing literature、Tree-sitter old-tree reuse concepts、Swift incremental syntax ideas，以及 Markdown-specific convergence observations。

### Fidelity naming rule

这些名字描述 **mechanism models**，不是宣称“我们已经重写了 Tree-sitter/Lezer/mizchi”。

如果模型与某 upstream 算法不能达到足够 fidelity：

```text
<project>-inspired
```

而不是直接使用项目名作为 horse 名称。

---

## 4. Shared parser semantic core

机制赛马最重要的公平性规则：每匹马必须解决同一个 parsing problem。

第一阶段必须先定义 `BENCH-GRAMMAR-v1`：一个有明确语义、足以覆盖更新机制风险的 Markdown 子集/核心。

最低覆盖：

```text
plain paragraph / text
blank-line block boundary
ATX heading
list / blockquote container basics
fenced code block
emphasis delimiter
code span delimiter
inline/reference link basics
reference definition
```

第一阶段目标不是声明完整 CommonMark conformance，而是构造**同一个受控 Markdown problem**。

每匹 horse 必须输出同一个 normalized semantic/syntax contract，例如：

```text
node kind
parent/child semantic relation
source span or source-slice identity required by protocol
ordered block/inline structure
reference-definition facts where included
```

内部 representation 可以不同；最终 normalized result 必须可比较。

Correctness oracle：

```text
horse incremental/update result
==
H0 clean full parse of post-edit source
```

因此 `expected` 不需要人工逐 case 编写。

如果未来扩大到完整 CommonMark/GFM，则另开 protocol amendment；第一轮不要把 standards implementation 本身变成实验主体。

---

## 5. Canonical operation contract

Source authority：UTF-8 bytes。

Edit：

```text
[start,end) byte range
+
inserted UTF-8 bytes
```

Host source mutation 在 parser timer 外，因为第一轮研究 syntax update，不研究 rope/piece-tree/text-buffer。

所有 horses 接收相同：

```text
old source
old mechanism state
post-edit source
canonical edit
```

不得各自生成私有 workload。

---

## 6. Timer contract

所有 Rust horses 由同一个 Rust runner 在同一进程模型下计时。

### FULL_PARSE / H0

Outside timer：

```text
file IO
corpus generation
post-edit source construction
case enumeration
logging / serialization / oracle comparison
```

`T_native`：

```text
START
horse parses already-in-memory UTF-8 source
all horse-required parsing/representation construction completes
black_box(result/state)
STOP
```

### UPDATE / H1-H3

Outside timer：

```text
old source already materialized
old horse state already constructed for the case
canonical edit selected
host applies edit / post-edit source materialized
```

`T_prepare`：

```text
only horse-specific edit-coordinate / metadata preparation required by that mechanism
```

`T_native`：

```text
START
mechanism-specific old-state maintenance
damage/restart/reuse/convergence logic
reparse work
representation reconstruction/index maintenance
all work required to produce new valid horse state
STOP
```

Headline：

```text
T_total = T_prepare + T_native
```

必须保存 `T_prepare / T_native / T_total`。

禁止把某 horse 必须做的 work 提前放到 timer 外。

### Timing fairness hard rule

如果一个 helper 对所有 horses 相同，原则上放在共同 boundary 的同一侧。

如果 helper 只因为某机制需要，则属于该 mechanism cost。

---

## 7. First-round operations

第一次只冻结足以构成逻辑闭环的 micro-operations：

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

### Structural minimum

必须至少覆盖六种传播机制：

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

代表 edits：

```text
paragraph split/merge
list or blockquote depth change
fence open/close
emphasis/code-span delimiter edit
link/reference delimiter edit
reference definition change
```

第一轮不求穷尽 Markdown syntax。

---

## 8. Payload

Synthetic payload 继续以 shape 为主，而非只有 file size：

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

第一轮 size 可以缩减为：

```text
64 KiB
1 MiB
16 MiB
```

只有在观察到 crossover / cliff 时再补 256 KiB / 4 MiB 或其它点。

真实 Markdown（CppCoreGuidelines 等）第一阶段主要作为 realism/sanity corpus；只有 BENCH-GRAMMAR-v1 能定义其相关 slice/cases 时才进入严格 horse comparison。禁止把 unsupported syntax 静默算进 headline result。

---

## 9. Measurements

Headline：

```text
latency p50 / p95
throughput for full parse
CPU time
peak / retained memory
allocation count / bytes
```

Algorithmic work counters（优先于微架构 profiling）：

```text
bytes / source intervals re-inspected
blocks reparsed
nodes rebuilt
nodes reused
metadata / range records touched
restart distance
convergence distance
fallback-to-full count
```

按 mechanism 可观察性记录；不可观察则 `UNKNOWN`。

### Parse Amplification

```text
PA = unique source-byte coverage re-inspected / logical edited bytes
```

只能由显式 instrumentation 得到，不得由 latency/changed_ranges/node count 反推。

---

## 10. Attribution ladder

看到时间差后，不立刻下“算法更好”的结论。

固定顺序：

```text
1. timing difference
2. scaling signature across N/B/L/D/K/F
3. algorithmic work counters
4. allocation / bytes moved / representation maintenance
5. controlled mutation / ablation / counterexample
6. only if still unexplained: microarchitectural profiling
```

### PMU / memory hierarchy

第一次实验**不要求**分析访存延迟、cache miss、branch miss。

只有当两个 mechanisms 的 algorithmic work 接近但 wall-clock 仍存在稳定显著差异时，才进入可选 attribution：

```text
cycles
instructions
branches / branch-misses
cache references / misses
L1/LLC misses if available
```

PMU 是解释残差的工具，不是第一轮 headline requirement。

---

## 11. Strength / Weakness profile

最终不是只给排名，而是每匹 horse 形成 profile：

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

目标是回答：

> 哪个机制在哪些输入上擅长？为什么？在哪些输入上退化？为什么？这些机制能否在下一阶段扬长避短地组合或重新设计？

“组合”只能在 Weakness Map 之后发生；不能在第一轮看到一个好点子就直接写 Markit algorithm。

---

## 12. Sampling / repeatability

第一轮保持简单：

```text
3 independent sessions
10 warmup iterations/session
30 measured iterations/session
case order shuffled with recorded fixed seed
report p50 + p95
no p99
no outlier deletion
```

统一 Rust substrate 后 JIT/GC 不再是主变量，但仍记录 OS/kernel/CPU/affinity/turbo/toolchain/build profile/runner commit/corpus manifest/seed。

失败不得静默删除：

```text
PASS
WRONG_RESULT
UNSUPPORTED
TIMEOUT
OOM
STACK_OVERFLOW
CRASH
INSTRUMENTATION_UNAVAILABLE
```

---

## 13. Conclusion ladder

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

规则：

- 单点 timing 只能建立 `OBSERVATION`；
- 重跑稳定才能 `REPRODUCED_OBSERVATION`；
- scaling + work counters + controlled probe 才能 `ATTRIBUTED_*`；
- 多个独立 mechanisms 出现同类证据才能 `CROSS_MECHANISM_PATTERN`；
- `PARETO_GAP` 表示没有 horse 同时满足预先声明的目标约束；
- `DESIGN_OPPORTUNITY` 必须建立在真实编辑相关的 weakness/pattern/Pareto gap 上。

---

## 14. Methodology references

1. Wagner & Graham, **Efficient and Flexible Incremental Parsing**, ACM TOPLAS 1998 — incremental work / reuse / scaling。
2. Swift incremental syntax parsing proposal — incremental vs clean parse / reuse / source-size scaling。
3. Catherine McGeoch, **A Guide to Experimental Algorithmics**, 2012 — 通过受控计算实验获得对 algorithm/program 的机制洞察，而非只收集 runtime 数字。
4. Mendling et al., **Methodology of Algorithm Engineering**, ACM Computing Surveys 2025, DOI 10.1145/3769071 — algorithm design、implementation 与 execution environment 是不同研究层次；implementation decisions 可能造成巨大性能差异。
5. Angriman et al., **Guidelines for Experimental Algorithmics: A Case Study in Network Analysis**, Algorithms 2019 — reimplementation bias、repeatability/replicability、实验程序/输入/参数应可复现。
6. Georges et al., **Statistically Rigorous Java Performance Evaluation**, OOPSLA 2007 — repeated measurement / statistical discipline。
7. Mytkowicz et al., **Producing Wrong Data Without Doing Anything Obviously Wrong**, ASPLOS 2009 — setup bias / randomization。
8. YCSB, SoCC 2010 — shared workload contract。
9. RocksDB `db_bench` — operation-oriented microbenchmark。
10. Kalibera & Jones, **Rigorous Benchmarking in Reasonable Time**, ISMM 2013 — independent repetition / uncertainty under finite experimental budget。

这些文献支撑研究方法，不支撑未来 Markit algorithm 的 novelty claim。

---

## 15. R0 first-round gate

进入 R1 前，只要求以下基础完整：

```text
PRIMARY_SUBJECT = controlled Rust mechanism race
UPSTREAM_ROLE = prior-art / source reconnaissance / sanity only
BENCH_GRAMMAR_V1 defined before measurement
NORMALIZED_RESULT_CONTRACT defined
H0-H3 mechanism boundaries documented
SAME payload / SAME edit / SAME semantic task
T_prepare / T_native / T_total frozen
correctness = horse result == H0 clean parse
structural minimum covers six propagation families
headline metrics frozen
work counters schema frozen
sampling/repeatability frozen
conclusion ladder frozen
```

第一次不要求：

```text
full CommonMark implementation
six full upstream ports
PMU/cache analysis
YCSB mixed workloads
all payload sizes
production architecture
Markit-specific algorithm
```

Verdict target：`READY_FOR_R1_CONTROLLED_RUST_HARNESS`。
