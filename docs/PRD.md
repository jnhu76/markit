# Markit PRD v3 — Adversarial Audit

## Executive Verdict

**结论：CONDITIONALLY SOUND**

v3 的研究顺序已经明显优于此前方案：

```text
控制变量
→ 测量
→ attribution
→ scaling
→ intervention
→ mechanism
→ implementation
```

但目前仍存在几个足以让整个研究得到“漂亮但错误结论”的方法学风险。

其中最严重的不是技术实现。

而是：

1. **把可测量的东西误当成真正的用户延迟；**
2. **不同编辑器的 workload 实际并不等价；**
3. **profiler 本身改变系统行为；**
4. **cross-platform experiment 极容易被平台差异污染；**
5. **项目仍然存在为 PocketJS 寻找证明的 confirmation bias；**
6. **ASCII baseline 有被错误推广为 editor architecture 结论的风险。**

这些问题需要在 R0 阶段解决。

---

# Critical Finding 1

## F-01 — `input → present` 的定义仍然可能是假的

**Severity: Critical**

PRD 把 Interaction-to-Present 作为第一指标，这是正确方向。

但“present”有多个完全不同含义：

```text
CPU submitted frame
GPU completed frame
swap called
compositor accepted surface
vsync selected frame
photon actually changed
```

如果 System A 测：

```text
GPU submit
```

System B 测：

```text
actual compositor presentation
```

两者不能比较。

甚至同一个 OS 的 API：

> “present completed”

也不一定意味着用户已经看到像素。

### Attack scenario

GPUI：

```text
input → GPU submit = 5 ms
```

Electron：

```text
input → compositor present = 11 ms
```

报告得出：

> GPUI latency 是 Electron 一半。

实际上：

```text
GPUI compositor queue = +8 ms
```

真实 visibility：

```text
13 ms
vs
11 ms
```

结论完全反转。

### Required Fix

R0 必须定义至少三个独立 timestamp：

```text
Tinput
Tsubmit
Tpresent-observable
```

并明确每个平台：

```text
Windows
macOS
Linux
```

能够提供哪个层级。

跨系统报告只比较：

> 同语义 timestamp。

无法获得真正 display visibility 时必须标记：

```text
presentation proxy
```

不得伪装为真实 input-to-photon。

---

# Critical Finding 2

## F-02 — 相同“按键脚本”不代表相同 workload

**Severity: Critical**

不同 Markdown editor：

```text
输入 #
```

可能触发完全不同产品语义。

例如：

System A：

```text
plain source editor
```

System B：

```text
live Markdown projection
```

System C：

```text
syntax highlight + preview update
```

System D：

```text
outline + spellcheck + history + extension events
```

如果直接比较：

```text
keystroke latency
```

实际上测的是不同产品。

### Attack scenario

Markit profile 比某 Electron editor 快 4×。

原因不是架构更好。

而是 Electron baseline 同时执行：

```text
syntax
outline
spellcheck
plugin notification
preview
```

Markit 全部关闭。

### Required Fix

建立：

# Semantic Workload Levels

例如：

```text
L0 Plain text edit
L1 Markdown parse
L2 Syntax/projection update
L3 User-visible Markdown editing
L4 Normal product configuration
```

只有同 level 才能横向比较。

同时保留：

> realistic default configuration

作为独立实验。

---

# High Finding 3

## F-03 — Flame graph 可能因为 profiler overhead 改变瓶颈

**Severity: High**

Profiling text rendering、allocation、JS runtime 时，sampling 与 instrumentation 可能：

* 改变 timing；
* 改变 cache；
* 改变 scheduler；
* 改变 JIT；
* 增加 syscalls；
* 阻止某些优化。

### Attack scenario

没有 profiler：

```text
p99 = 9 ms
```

开启 trace：

```text
p99 = 24 ms
```

然后 flame graph 显示：

```text
allocator
logging
trace serialization
```

研究者误认为这是产品瓶颈。

### Required Fix

每个 profiler 必须记录：

```text
baseline latency
profiled latency
overhead ratio
```

正式性能数字和 profiling 数字分开采集。

Profile 用于 attribution。

Uninstrumented run 用于最终 latency。

---

# High Finding 4

## F-04 — Scaling experiment 容易把 corpus structure 当作 N

**Severity: High**

如果：

```text
10 KB
100 KB
1 MB
10 MB
```

由不同真实 Markdown 文件组成，那么增加的不只是 size。

还同时改变：

```text
block count
line length
code fences
links
Unicode
heading count
```

最终得到：

```text
T(N)
```

其实完全不是 document-size complexity。

### Required Fix

建立 synthetic scaling families。

例如：

## Family A

重复相同 paragraph：

```text
N × paragraph
```

仅改变 block count。

## Family B

一个 paragraph 不断增长。

仅改变 line length/text length。

## Family C

固定 block count，增加 block size。

## Family D

固定 size，增加 Markdown structural density。

必须把：

```text
N
B
L
V
Δ
```

拆开。

---

# High Finding 5

## F-05 — ASCII baseline 可能产生错误安全感

**Severity: High**

U0 非常适合作为实验 baseline。

但是 editor text architecture 的很多真实成本只在：

```text
CJK
fallback
graphemes
IME
bidi
complex shaping
```

中出现。

如果 U0 阶段冻结 architecture，再到 U3/U4 才发现：

```text
coordinate model错误
shape cache key错误
run model错误
```

返工非常大。

### Required Fix

区分：

```text
performance scope
```

和：

```text
architectural correctness constraints
```

R1 性能可以只测 U0。

但 architecture prototype 从一开始至少必须保持：

```text
UTF-8 safe offsets
grapheme-capable API
platform text abstraction
IME-compatible transaction model
```

也就是说：

> 暂时不 benchmark ≠ 架构允许假设 ASCII。

---

# High Finding 6

## F-06 — Cross-platform 抽象可能把真正的瓶颈隐藏掉

**Severity: High**

KMP 式：

```text
TextShaper trait
```

很漂亮。

但过早抽象可能强迫：

```text
DirectWrite
CoreText
HarfBuzz
```

适配一个最低公分母 API。

结果：

* 平台优化能力丢失；
* extra allocation；
* data conversion；
* hidden copies；
* 无法利用 native layout cache。

最后我们测到的可能是：

> abstraction tax。

而不是平台本身。

### Required Fix

Platform contract 必须允许：

```text
common semantic interface
+
platform-specific fast path
```

并要求 profile：

```text
adapter conversion cost
native call cost
copy cost
```

不要追求 API 形式完全统一。

---

# High Finding 7

## F-07 — ReferenceHost 很容易成为“虚假的确定性世界”

**Severity: High**

Headless deterministic host 很适合 correctness。

但不能证明：

* scheduler；
* compositor；
* real fonts；
* IME；
* GPU；
* OS input；
* frame pacing。

### Required Fix

明确：

```text
ReferenceHost
= core correctness + algorithmic scaling tool

Real Host
= user latency evidence
```

ReferenceHost 结果永远不能作为：

```text
desktop latency claim
```

---

# High Finding 8

## F-08 — PocketJS 仍然是研究中的既定答案

**Severity: High**

虽然 v3 增加 Architecture Review Escape Hatch，但项目标题与目标仍然是：

> 优化 PocketJS 来做。

团队非常容易产生：

```text
“我们需要证明 PocketJS 只差一个 EditorSurface。”
```

的心理预设。

### Attack scenario

实验发现：

```text
PocketJS host abstraction
+
render contract
+
text architecture
```

需要重写 60%。

团队仍然称之为：

> PocketJS optimization。

实际已经是另一个 runtime。

### Required Fix

R9 必须加入定量 architecture review：

至少比较：

```text
code reused
subsystems bypassed
platform code duplicated
FFI layers added
maintenance surface
performance delta
```

如果 Markit 绕过 PocketJS 大部分核心：

> 必须承认架构已经变化。

---

# Medium Finding 9

## F-09 — “关闭 subsystem”可能产生不真实 causal experiment

**Severity: Medium**

例如：

```text
disable parser
```

会同时减少：

* CPU；
* allocations；
* downstream invalidation；
* render changes。

所以：

```text
latency下降
```

只能证明：

> parser pipeline 总体相关。

不一定证明 parser 自身 CPU 是原因。

### Required Fix

Intervention 分层：

```text
parser real compute → fake equivalent output
parser output → frozen cached output
downstream notification → disabled
```

尽量保持其他路径不变。

原则：

> intervention 应改变一个 causal variable，而不是删除半条 pipeline。

---

# Medium Finding 10

## F-10 — Cache warm/cold 状态没有单独建模

**Severity: Medium**

GUI latency 高度依赖：

```text
font cache
glyph cache
parser cache
layout cache
filesystem cache
GPU pipeline cache
```

如果只 warm-up：

可能隐藏真实首次操作卡顿。

如果完全 cold：

又不像长期编辑。

### Required Fix

正式定义：

```text
Cold
Warm
Steady-state
Post-idle
After-large-navigation
```

不同 cache state。

---

# Medium Finding 11

## F-11 — Thermal / power state 可以轻易制造虚假回归

**Severity: Medium**

尤其 laptop：

```text
Turbo
thermal throttling
battery mode
background antivirus
OS indexing
```

都会影响 p99。

### Required Fix

记录：

```text
power plan
battery/AC
CPU frequency behavior
thermal state
```

正式 benchmark 应：

* 随机化版本执行顺序；
* 或 A/B/A/B interleave；

避免：

```text
old version先跑
new version热降频后跑
```

造成假回归。

---

# Medium Finding 12

## F-12 — p99 在样本太少时没有意义

**Severity: Medium**

100 次输入：

```text
p99
```

基本就是第二慢事件。

如果 workload 不够长，tail latency 会非常不稳定。

### Required Fix

PRD 必须给出 minimum event count。

例如：

```text
typing interaction >= thousands
scroll frames >= thousands
```

正式 threshold 根据 R0 方差确定。

同时报告：

```text
long-frame count
max
histogram
```

而不迷信单独 p99。

---

# Medium Finding 13

## F-13 — 用户输入自动化可能绕过真实 OS path

**Severity: Medium**

直接调用：

```text
editor.applyEdit()
```

测不到：

```text
window event queue
keyboard dispatch
IME
OS scheduler
```

但使用真实 OS key injection：

又增加 nondeterminism。

### Required Fix

建立两层 workload：

```text
Engine workload
→ deterministic internal command

End-to-end workload
→ OS/platform input
```

两者回答不同问题。

禁止混用结果。

---

# Medium Finding 14

## F-14 — Markdown parser 的“增量性”不能只看 parse 时间

**Severity: Medium**

一个 incremental parser 可能：

```text
parse = 0.4 ms
```

但生成的大量 changed nodes 导致：

```text
projection/layout = 15 ms
```

### Required Fix

changed-region 必须贯穿：

```text
buffer
parse
projection
wrap
layout
render
```

记录每层：

```text
input delta size
invalidated logical range
invalidated display range
materialized visible range
```

这样才能发现：

> 小 edit 被哪一层重新放大成 global invalidation。

---

# Medium Finding 15

## F-15 — “visible only”也可能不是正确复杂度模型

**Severity: Medium**

Markdown layout 存在：

```text
offscreen height estimates
scrollbar mapping
fold state
block dependency
```

因此目标不应机械规定：

```text
O(visible)
```

### Required Fix

更准确的原则：

> **Per-interaction work must be bounded and proportional to information genuinely affected by the interaction.**

它可能是：

```text
O(V)
O(Δ)
O(log N)
O(changed block chain)
```

而不是永远 O(V)。

---

# Medium Finding 16

## F-16 — 多平台测试矩阵后期仍可能爆炸

**Severity: Medium**

未来矩阵：

```text
3 OS
× several editors
× Unicode levels
× document sizes
× workloads
× cache states
× hardware
```

不可持续。

### Required Fix

建立三层测试集：

## Tier A — Commit

很小：

```text
one platform
core regression
```

## Tier B — Nightly

代表性：

```text
3 OS
selected workloads
```

## Tier C — Research / Release

完整矩阵。

不要把 research benchmark 变成普通 CI。

---

# Required PRD Amendments

在正式接受 v3 前，我建议至少增加以下 10 条硬性规则：

1. **Presentation timestamp 必须按语义分类，禁止混比。**
2. **Benchmark workload 必须定义 semantic equivalence level。**
3. **Profiling run 与 latency run 分离，并量化 profiler overhead。**
4. **Scaling corpus 必须控制 N/B/L/V/Δ。**
5. **ASCII 只简化 benchmark，不允许 core architecture 假设 ASCII。**
6. **Platform abstraction 必须允许 native fast path。**
7. **ReferenceHost 不得作为真实桌面 latency 证据。**
8. **PocketJS 必须保留真正的 stop/review gate。**
9. **Causal intervention 尽可能保持 pipeline 输出等价。**
10. **必须区分 engine-level benchmark 与 OS end-to-end benchmark。**

---

# Revised Research Spine

经过红队后，更稳妥的流程应当是：

```text
Define semantic workload
        ↓
Control variables
        ↓
Validate measurement
        ↓
Measure real latency
        ↓
CPU / off-CPU / GPU attribution
        ↓
Scaling experiment
        ↓
Form root-cause hypothesis
        ↓
Controlled intervention
        ↓
Reproduce on another workload
        ↓
Reproduce on another platform/system
        ↓
Extract mechanism
        ↓
Optimize PocketJS
        ↓
Re-run original end-to-end experiment
```

最后多出两个非常重要的步骤：

```text
another workload
another platform/system
```

因为只在一个 benchmark 上成立：

> 还不能称为设计原则。

---

# Final Audit Verdict

## What v3 Gets Right

它已经避开三个最危险的错误：

* 一开始就选择实现方案；
* 把 flame graph 当作 causality；
* 一开始同时研究所有 Unicode / OS / framework 变量。

## What Must Change Before Implementation

R0 必须先把：

```text
measurement semantics
workload equivalence
profiler overhead
scaling corpus
experiment tiers
```

定义清楚。

否则后面的 profiler 数据越丰富：

> **越可能让错误结论看起来很科学。**

## Gate

**PRD v3 可以作为研究方向基线，但建议先吸收 F-01～F-08，再冻结为 v3.1。**

真正开始 PocketJS editor implementation 前，至少应该经过：

```text
R0 Methodology
R1 Controlled baseline
R2 Scaling
R3 Attribution
R4 Causal validation
```

四个证据 gate。

在那之前：

> **不允许因为“某个设计看起来像 Zed/Typora/GPUI”而进入大规模实现。**
