# Markit — GPUI 与 PocketJS Benchmark / Mechanism Comparison

> **Status:** Research closeout
> **Scope:** Phase A1–A4
> **Primary product foundation:** PocketJS
> **Reference / performance oracle:** GPUI

---

## 1. 为什么需要这份文档

Markit 的早期研究并不是为了回答一个简单的“GPUI 和 PocketJS 谁跑分快”。

真正的问题是：

> 对一个追求低延迟、低卡顿、支持大文档的 Markdown 编辑器而言，两个技术栈分别会在哪里产生工作放大？这些成本来自框架本身，还是来自 Markit 的使用方式？修掉错误以后，还剩下什么不可避免的成本？

因此，本报告不把 benchmark 当排行榜，而是把 A1–A4 的实验结果串成一条因果链：

```text
观察性能问题
    ↓
定位工作放大
    ↓
构造 counterfactual
    ↓
实施最小修复
    ↓
重新 benchmark
    ↓
判断剩余成本归属
    ↓
做产品技术路线决策
```

---

# 2. 结论先行

如果问题只是：

> **哪个 substrate 在当前 Windows MVP 上更快、更省内存？**

答案很明确：

**GPUI。**

在最后一次同机、双框架 common benchmark（A3）中：

| 指标                           |         GPUI |  PocketJS |
| ---------------------------- | -----------: | --------: |
| 1M 单字符 edit                  |  **1.21 ms** |    9.9 ms |
| 1M edit p99                  |  **0.65 ms** |   13.5 ms |
| Startup → first usable frame |   **336 ms** |    750 ms |
| Working Set @1M              | **39.0 MiB** | 235.6 MiB |
| Private Bytes @1M            | **35.0 MiB** | 223.5 MiB |
| Idle CPU                     |        ~0–2% |       ~1% |
| Idle rendering               |       ~0 fps |    ~0 fps |

GPUI 在延迟、tail latency、启动和内存上都有显著优势。

但是 A4 回答了一个更重要的问题：

> PocketJS 慢下来的部分，到底是不是 PocketJS 本身的硬下限？

答案是：**不是。**

PocketJS A3 剩余约 7–9 ms 的大头来自 Markit 对 Solid visible list identity 的错误使用。每次编辑都会创建新的 item object，导致 26 个可见行组件全部被当成新节点重新挂载。

采用 stable item identity + item-scoped memo 后：

```text
PocketJS 1M edit

A2: 217.7 ms
        │
        │ remove full-document lineStarts scan
        ▼
A3:   9.9 ms
        │
        │ remove fresh-reference Solid remount amplification
        ▼
A4:   3.65 ms
```

从 A2 到 A4，总 edit latency 下降约 **98%**。

因此最终的产品判断不是：

> PocketJS benchmark 打败了 GPUI。

而是：

> **GPUI 仍然更快，但 PocketJS 的已知性能成本已经进入可控范围，而且没有发现 PocketJS lower stack 的产品级性能阻塞。**

Markit 因而选择：

```text
PocketJS = PRIMARY PRODUCT FOUNDATION

GPUI = REFERENCE / PERFORMANCE ORACLE
```

---

# 3. Benchmark 环境与口径

主要 Windows benchmark 环境：

```text
OS:        Windows 11
CPU:       AMD Ryzen 7 5800H
RAM:       32 GB
Window:    1000 × 700
Font:      Consolas 18 px
Line:      28 px
GPUI:      0.2.2
Rust:      rustc 1.96.0 x86_64-pc-windows-msvc
JS:        Bun 1.2.8
Corpus:    fixed-seed 10K / 100K / 250K / 500K / 1M
```

需要特别注意：

**GPUI 和 PocketJS 的 latency boundary 并不完全相同。**

GPUI 主要测：

```text
edit-side work
→ prepaint
→ paint
→ frame work
```

PocketJS 主要测：

```text
guest edit
→ reactive update
→ DrawList generation
```

因此数字适合判断：

* 数量级；
* scaling；
* 工作放大；
* intervention 前后变化；
* 产品是否跨过 16.7 ms 等体验阈值；

但不应该把 1.21 ms 和 3.65 ms 理解成两个完全相同函数的 microbenchmark。

---

# 4. 第一阶段：两个 MVP 都曾经“非常慢”

A2 的 1M benchmark：

| Substrate | 1M edit/frame | 初始判断 |
| --------- | ------------: | ---- |
| PocketJS  |  **217.7 ms** | 不可接受 |
| GPUI      |   **52.4 ms** | 不可接受 |

如果研究在这里停止，很容易得到两个错误结论：

```text
PocketJS 很慢
GPUI 也不适合大文档
```

但 causal decomposition 表明：

**两边最大的问题都不是 framework intrinsic cost。**

---

# 5. PocketJS：217.7 ms 从哪里来

A2 分解发现：

```text
1M edit turn ≈ 217.7 ms

其中：

lineStarts() full document scan
≈ 206 ms
≈ 94.6%
```

也就是说，每输入一个字符，Markit 都重新扫描整个文档寻找换行符。

复杂度近似：

```text
edit
  ↓
scan entire document
  ↓
O(document bytes)
```

这不是 PocketJS 的问题，而是 Markit MVP 的 document model 问题。

### Counterfactual

跳过 full scan 后：

```text
217.7 ms
   ↓
约 7.5 ms
```

于是 A2 给出了一个非常明确、可以被 A3 验证的预测：

> 如果把 line index 改成 incremental maintenance，PocketJS edit latency 应该从两百多毫秒进入个位数毫秒级。

---

# 6. PocketJS A3：217.7 ms → 9.9 ms

A3 引入 Incremental `LineIndex`。

旧模型：

```text
edit
 → scan entire document
 → rebuild every line start
```

新模型：

```text
edit(start, end, text)
        ↓
update affected range
        ↓
insert/delete newline positions
        ↓
shift suffix offsets
```

结果：

| Corpus |           A2 |         A3 |
| -----: | -----------: | ---------: |
|    10K |      13.5 ms |     8.9 ms |
|   100K |      32.8 ms |    10.4 ms |
|     1M | **217.7 ms** | **9.9 ms** |

1M 延迟下降约 **95.5%**。

更重要的是 scaling：

```text
A2:
10K → 1M
约 25× amplification

A3:
10K → 1M
约 1.1×
```

原本最危险的：

```text
O(document bytes)
```

已经从 normal edit hot path 中消失。

---

# 7. GPUI：52.4 ms 从哪里来

GPUI 的问题完全不同。

A2 中 `EditorElement` 被设置成整个文档的 content height。

于是 framework 认为：

```text
viewport
≈ whole document
```

一个 1M 文档中：

```text
实际需要显示：约 26 行

实际被访问：18,081 行
实际被 shape：18,081 行
```

于是一次 edit frame：

```text
prepaint ≈ 39.4 ms
paint    ≈ 11.9 ms
total    ≈ 52.4 ms
```

本质仍然是：

> **一个本该 viewport-bounded 的操作，被错误扩大成了 O(document)。**

---

# 8. GPUI A3：52.4 ms → 1.21 ms

修复方式很简单：

```text
Element bounds
full-content-sized
        ↓
viewport-sized
        +
2 overscan lines
```

工作量变化：

| 1M document      |      A2 |          A3 |
| ---------------- | ------: | ----------: |
| visible          |  19,654 |          26 |
| lines visited    |  18,081 |          28 |
| lines shaped     |  18,081 |          25 |
| prepaint         | 39.4 ms |   **64 µs** |
| whole edit frame | 52.4 ms | **1.21 ms** |

降幅约 **97.7%**。

而且 static redraw：

```text
10K → 1M
≤ 1.8×
```

说明 rendering work 已经真正变成 viewport bounded。

---

# 9. A3：第一次公平看到两个 substrate 的形状

修掉双方最大的 O(document) 错误后：

| Property            | GPUI                         | PocketJS                   |
| ------------------- | ---------------------------- | -------------------------- |
| Active-edit latency | **更低**                       | 较高                         |
| p99                 | **明显更低**                     | 较高                         |
| Memory              | **明显更低**                     | 较高                         |
| Startup             | **更快**                       | 较慢                         |
| Idle work           | 很低                           | 很低                         |
| Rendering scaling   | viewport bounded             | viewport bounded           |
| 主要 residual         | edit-side line-index rebuild | Solid visible-list work    |
| Residual owner      | Markit                       | 当时认为 Solid/Markit boundary |
| Product logic       | Native Rust                  | Guest-side JS/TS           |

所以 A3 的合理结论确实是：

```text
LEAN_GPUI
```

如果研究到这里停止，GPUI 是更稳妥的性能选择。

---

# 10. A4：PocketJS 的“7 ms floor”到底是什么

A4 专门追踪 PocketJS A3 剩余的约 7–9 ms。

结果非常关键。

Counterfactual bundle 保持：

* 同一个 document model；
* 同样的 26 个 visible lines；
* 同样的 caret；
* 同样的 selection；
* DrawList word-for-word 相同；
* screenshot pixel-identical；

但绕过 Solid component reconstruction。

结果显示：

```text
PocketJS edit turn
≈ 8–10 ms

其中：

Solid reactive reconstruction
≈ 7.4 ms

PocketJS lower stack
≈ 0.3 ms
```

进一步检查 native operations 后发现，原来的 Solid app 每次 edit 大约发生：

```text
~90 native node creations
~30 detach/destroy
```

而 counterfactual 只需要：

```text
1 replaceText
2 caret setProps
```

---

# 11. 根因：不是 Solid 慢，而是 identity 用错了

Markit 当时每次 render 都生成新的：

```ts
{
  index,
  start,
  end
}
```

即便代表同一行，也拥有新的 object reference。

而当前 Solid `For` reconciliation 按 reference identity 判断 item 是否相同。

于是：

```text
edit
 ↓
26 个全新 object
 ↓
Solid 判断 26 个 item 全变了
 ↓
全部 remount
 ↓
native node recreation
 ↓
GC + bridge + tree work
```

这就是 A3 所看到的约 7 ms。

它不是：

```text
PocketJS intrinsic floor
```

也不是：

```text
Solid intrinsic floor
```

而是：

```text
Markit's use of Solid reconciliation semantics
```

---

# 12. A4 stable identity 修复

产品自然的解决方式是：

```text
Visible item
    ↓
stable identity

Document-dependent values
    ↓
item-scoped memos
```

修改之后：

| Corpus |  Before |       After |
| -----: | ------: | ----------: |
|    10K | 9.44 ms | **1.66 ms** |
|   100K | 8.79 ms | **1.94 ms** |
|     1M | 9.67 ms | **3.65 ms** |

A3 → A4 又降低约 **63%**。

1M 不同位置：

| Position | PocketJS A4 |
| -------- | ----------: |
| begin    |     4.18 ms |
| q1       |     3.66 ms |
| middle   |     2.79 ms |
| q3       |     2.41 ms |
| end      | **1.91 ms** |

这个 gradient 不再来自 rendering。

它主要来自 model 中：

```text
LineIndex suffix offset shift
```

也就是说，越靠近文档开头编辑，需要移动的后续 line offsets 越多。

这是：

```text
O(lines after edit)
```

而不再是：

```text
O(document bytes)
```

---

# 13. 最终 latency 该怎么比较

这里必须避免制造一个不存在的“完全 apples-to-apples A4 benchmark”。

GPUI 在 A4 没有继续优化，也没有重新跑完整 common battery。

因此有两组值得保存的数据。

### 最后一次严格 common benchmark：A3

```text
GPUI       ≈ 1.21 ms
PocketJS   ≈ 9.9 ms
```

GPUI 大约快 **8×**。

### PocketJS A4 final state

```text
GPUI A3 reference      ≈ 1.21 ms
PocketJS A4            ≈ 3.65 ms @1M scale
PocketJS A4 end edit   ≈ 1.91 ms
```

如果用 A3 GPUI 作为 reference：

PocketJS 与 GPUI 的 gap 已从约：

```text
8×
```

缩小到约：

```text
3×
```

但这不是一次新的双边 common benchmark，因此只能作为工程比较，不能声称为新的公平 benchmark 排名。

---

# 14. Memory 和 startup：GPUI 的优势仍然是真实的

A3 measured：

| Metric @1M         |         GPUI |  PocketJS | Ratio |
| ------------------ | -----------: | --------: | ----: |
| Working Set        | **39.0 MiB** | 235.6 MiB |   ~6× |
| Private Bytes      | **35.0 MiB** | 223.5 MiB |   ~6× |
| First usable frame |   **336 ms** |    750 ms | ~2.2× |

这部分不能因为 A4 的 latency 优化就忽略。

PocketJS 的 runtime baseline 明显更重：

```text
QuickJS
+ Solid
+ retained UI tree
+ baked resources / atlases
+ PocketJS runtime
```

GPUI 的 native Rust stack 在：

* memory；
* startup；
* raw latency；

三个指标上都有明显优势。

A4 没有重新跑完整 memory/startup battery，因此不能声称 stable-identity fix 改变了这个结论。

---

# 15. Markdown L1：真正产品 workload 的第一次测试

A4 不再只测纯文本。

实验构造了：

```text
Document
 → Block Index
 → Incremental Parse
 → Affected Blocks
 → Styled Runs
 → Visible Layout
 → DrawList
```

测试：

* paragraph；
* inline emphasis；
* heading；
* list；
* fenced code；
* off-viewport edit。

普通 local edit：

| Case         |     10K |      1M | Blocks reparsed |
| ------------ | ------: | ------: | --------------: |
| paragraph    | 1.38 ms | 1.54 ms |               1 |
| emphasis     | 1.44 ms | 1.41 ms |               1 |
| heading      | 1.37 ms | 1.45 ms |               1 |
| list         | 1.52 ms | 1.64 ms |               1 |
| off-viewport | 1.34 ms | 1.45 ms |               1 |

关键结果：

```text
10K → 1M

blocks reparsed:
1 → 1
```

也就是说：

> **普通 Markdown 局部编辑没有重新引入 O(document) hot path。**

这比某个单独的 1.4 ms 数字更重要。

---

# 16. 一个不能藏起来的坏结果：Fence cascade

A4 同时发现一个真实的产品级 worst case。

编辑 fenced code boundary 时：

```text
10K:
287 lines
24 blocks
5.23 ms

1M:
30,197 lines
2,364 blocks
68.9 ms
```

原因不是渲染框架。

Markdown fence 是一种结构状态：

```text
opening fence
    ↓
后续 parsing state
    ↓
closing fence
```

破坏一个 opening fence 后，parser state 可能一直传播到很远的位置。

所以：

```text
local character edit
≠
local structural effect
```

这个 68.9 ms 不应该被 benchmark 技巧隐藏掉。

它应该成为产品设计输入。

未来需要：

```text
parser checkpoint
+
restart state
+
bounded recovery
```

目标不是假装 fence edit 是 O(1)，而是给 structural cascade 一个可控的恢复机制。

---

# 17. 两个框架真正的 mechanism comparison

| Dimension                             | GPUI                 | PocketJS                  |
| ------------------------------------- | -------------------- | ------------------------- |
| Runtime model                         | Native Rust          | JS/TS guest + native host |
| Raw latency                           | **更优**               | 可接受，但更高                   |
| Tail latency                          | **更优**               | 较高                        |
| Memory                                | **显著更优**             | runtime baseline 较重       |
| Startup                               | **更优**               | 较慢                        |
| Idle behavior                         | 优                    | 优                         |
| Viewport-bounded rendering            | 已验证                  | 已验证                       |
| Incremental document seam             | 可以实现                 | 已验证                       |
| Incremental Markdown                  | 未继续产品化               | **已验证 L1**                |
| Guest-side iteration                  | 弱                    | **强**                     |
| UI/product iteration                  | Native rebuild cycle | **JS/TS iteration**       |
| Main historical bottlenecks           | Markit integration   | Markit integration        |
| Framework/core blocking defect found? | No                   | **No**                    |
| Product role                          | Performance oracle   | **Primary foundation**    |

---

# 18. 最重要的研究发现其实不是“谁快”

A1–A4 更重要的结果，是发现了四种完全不同的 work amplification。

### ① Document amplification

```text
每次 edit
→ full-document scan
```

修复：

```text
Incremental LineIndex
```

### ② Viewport amplification

```text
本来只需 26 行
→ shape 18,081 行
```

修复：

```text
viewport + overscan
```

### ③ Reactive identity amplification

```text
26 个 logically identical items
→ fresh object references
→ 26 个 component remount
```

修复：

```text
stable identity
+
item-scoped memo
```

### ④ Structural invalidation amplification

```text
1 character
→ Markdown state cascade
→ thousands of blocks
```

这一个不能简单靠 viewport clipping 解决。

需要：

```text
incremental parser architecture
+
bounded recovery
```

这四种 amplification 比“GPUI 1.2 ms、PocketJS 3.6 ms”更值得成为 Markit 后续设计的核心知识。

---

# 19. 从 benchmark 提炼出的性能不变量

Markit 后面的代码不应该只是“尽量快”。

应该维护几个明确的不变量。

## INV-1 — Local edit 不得 full-document scan

普通输入不能出现：

```text
O(document bytes)
```

扫描。

## INV-2 — Rendering work 必须 viewport bounded

文档从：

```text
10K
→
100K
→
1M
```

增长时，可见区域 rendering work 不应同步增长。

## INV-3 — Visible object identity 必须稳定

不能因为 document mutation：

```text
重新创建整个 visible UI subtree
```

## INV-4 — Changed range 必须显式传播

Document mutation 应产生：

```text
{start, end, inserted}
```

而不是要求下游重新比较整个文档。

## INV-5 — Parser invalidation radius 必须可观察

每次 Markdown edit 都应该能够回答：

```text
scanned lines?
reparsed blocks?
restyled blocks?
visible nodes rebuilt?
```

## INV-6 — Structural worst case 必须单独 benchmark

平均输入快不能掩盖：

* fence；
* table；
* list nesting；
* math；
* image/block；
* future L2 projection；

等 structural edit 的 cascade。

---

# 20. 为什么最终仍然选择 PocketJS

如果 Markit 的唯一目标是：

```text
minimum possible memory
+
minimum possible native latency
```

目前的数据会支持 GPUI。

但是 Markit 的目标不是做一个 editor benchmark。

它要做的是一个长期演进的 Markdown 产品。

PocketJS 已经证明：

```text
normal Markdown local edit
≈ 1.4–1.6 ms

1M plain-text edit
≈ 2–4 ms depending on position

lower rendering stack
≈ 0.3 ms

no normal-edit O(document) rendering path
```

也就是说：

> 性能已经不再构成否决 PocketJS 的理由。

与此同时，PocketJS 允许大部分：

* document behavior；
* Markdown logic；
* command logic；
* incremental view model；
* editor product behavior；

留在 JS/TS guest side。

这带来了更大的：

```text
iteration speed
+
architecture control
+
experimentation freedom
```

因此最终决策是一个产品工程 trade-off，而不是 benchmark victory：

```text
PRIMARY PRODUCT FOUNDATION
        =
      PocketJS

REFERENCE / PERFORMANCE ORACLE
        =
        GPUI
```

GPUI 不应该被删除或遗忘。

恰恰相反，它非常有价值。

以后如果 PocketJS 出现：

```text
20 ms edit
50 ms layout
300 MiB unexpected growth
```

我们可以问：

> 同样的 workload，GPUI oracle 是多少？

如果 GPUI 也是慢：

```text
可能是 workload 本身
```

如果 GPUI 仍然 1 ms：

```text
优先寻找 PocketJS/Markit stack 的 amplification
```

这比凭感觉优化有效得多。

---

# 21. Benchmark 没有证明什么

当前结果不能被扩大解释。

尚未完整证明：

* CommonMark conformance；
* Markdown L2 visual syntax hiding；
* table/image/math 等 rich block；
* Windows real IME latency；
* CJK font fallback；
* clipboard path；
* real Linux Wayland/X11；
* Linux IBus/Fcitx；
* macOS host；
* mobile/tablet behavior；
* 极大文档下 undo history；
* multi-cursor；
* folding；
* syntax-aware selection；
* background parsing；
* file IO / save / recovery。

尤其 A4 的 randomized differential test 证明的是：

```text
incremental parser
==
its own full-scan oracle
```

它证明 incremental invalidation correctness。

它**不等于**：

```text
CommonMark correctness
```

两者必须继续分开。

---

# 22. 后续使用 benchmark 的原则

从现在开始，不应该继续做“为了研究而研究”的 A5、A6 benchmark。

下一阶段首先应该把 PocketJS Desktop Enablement 做成真正产品 foundation。

只有当真实产品 workload 出现问题时，再启动 targeted experiment。

推荐流程：

```text
real product symptom
      ↓
record trace + counters
      ↓
identify amplification dimension
      ↓
build counterfactual
      ↓
minimal intervention
      ↓
re-benchmark
      ↓
write invariant / regression test
```

不要再回到：

```text
“感觉这里可能慢，所以优化一下”
```

的模式。

---

# 23. Final verdict

最终 benchmark 给出的答案并不是：

```text
PocketJS > GPUI
```

实际上，就当前 measured native efficiency 而言：

```text
GPUI > PocketJS
```

这一点应当一直保留在文档里。

但研究同样证明：

```text
PocketJS 的早期巨大延迟
不是 PocketJS intrinsic limitation

217.7 ms
↓
9.9 ms
↓
3.65 ms
```

绝大多数问题来自：

```text
Markit integration
+
document architecture
+
reactive identity discipline
```

而不是 PocketJS lower stack。

同时，真正 Markdown L1 workload 已证明普通局部编辑：

```text
1 block invalidation
+
viewport-bounded rendering
+
~1.4–1.6 ms edit
```

因此对于 Markit：

> **GPUI 是更快、更轻的 native substrate；PocketJS 是已经达到产品性能门槛、同时提供更高产品迭代自由度的 substrate。**

于是最终技术路线是：

```text
PocketJS
    │
    ├── Markit Product Foundation
    │
    ├── Incremental Document Model
    │
    ├── Incremental Markdown
    │
    └── Incremental View Model
    │
    ▼
Windows → Linux → macOS


GPUI
    │
    └── performance/reference oracle
```

研究阶段到这里应该结束。

下一步不是继续证明 PocketJS。

下一步是：

```text
PocketJS Desktop Enablement
        ↓
Windows Reference Host
        ↓
Markit Product P0
```

只有真实产品 workload 再次违反性能不变量时，才重新进入 causal performance research。

---

## Evidence / source map

本报告数字与结论对应：

```text
docs/phase-a2-causal-decomposition.md
    └── initial bottleneck decomposition + counterfactual

docs/phase-a3-intervention-validation.md
    └── root-cause fixes
    └── common Windows re-benchmark
    └── startup / memory / idle
    └── GPUI vs PocketJS A3 comparison

docs/phase-a4-final-research-closeout.md
    └── PocketJS Solid residual decomposition
    └── stable identity intervention
    └── Markdown L1 invalidation benchmark
    └── final product-foundation decision

results/raw/a2/
results/raw/a3/
results/raw/a4/
    └── raw experiment evidence

results/summary/a2/
results/summary/a3/
results/summary/a4/
    └── summarized benchmark evidence
```

任何未来更新这份文档的人，都应继续遵守：

```text
OBSERVATION
≠
MEASUREMENT
≠
INFERENCE
≠
CAUSAL EVIDENCE
≠
DESIGN DECISION
```

不要因为产品已经选择 PocketJS，就改写 GPUI 的 benchmark 优势；也不要因为 GPUI benchmark 更快，就忽略 A4 已经证明的 PocketJS 产品可行性。
