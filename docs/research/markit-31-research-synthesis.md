# MARKIT-31 研究进展综合 — 从机制赛马到证据驱动的 V1 要求

Status: **DURABLE RESEARCH SYNTHESIS / CANONICAL HANDOFF** —— 本文件是本阶段研究的
规范交接文档。它记录研究如何一步步改变我们的理解，以及下一步应当回答什么问题。

关联 issue：

```text
#22    umbrella：Rust Markdown AST/CST 更新机制标准化 benchmark（ACTIVE）
#31    execution：project-driven performance evaluation for H0–H4（ACTIVE）
#33    roadmap：paper-style regime map study（ACTIVE）
#48    mechanism synthesis / A–J interpretation（ACTIVE，本文件是其后续）
#50    H4-LARGE-N-CAUSE-1 causal experiment（CLOSED，evidence authority = PR #51）
```

配套阅读：

1. [Campaign-2 完整机制评审](campaign-2-mechanism-synthesis.md) —— **历史文档**：
   Campaign-2 当时（PR #47）的 A–J 解释，保留原样。
2. [从实验到 AST 的实施指导](campaign-2-to-ast-guidance.md) —— 工程方向与顺序。
3. [Markit AST/更新算法设计候选 v0](markit-ast-update-design-v0.md) —— 设计候选，
   每条主要陈述已按 A/B/C/D 分级。

---

## 0. What is proven / what is not

这一节是全文的读法约定。**proven 只包含仓库内有证据权威的具体陈述**；
其余一律属于未证明，不能因为“看起来显然”或“别人都这么做”而升级。

### Proven

```text
- H0-H4 在冻结工作负载下的正确性（oracle 等价，R5 parity，Campaign-2 correctness gates）。
- regime-specific 的机制优势/劣势：没有通用赢家（Campaign-2 B 轴、真实 edit family）。
- restart + convergence 对局部编辑给出大幅收益（Campaign-2：H4 相对 H0 在 1 MiB 约 6.347×）。
- H4 的局部 parser work 可以随 N 保持常数，8/8 cells 恒定。两个 lane 各给出一个
  不能互换的数字：Campaign-2 的 post inspected bytes = 138；#50 W_COUNTERS 的
  `source_bytes_inspected_total` = 283（诊断 scanner 的 total inspection，含边界重读），
  其中 unique post-source = 138。283 不是 parser 的唯一输入尺寸，也不是 138 的替代。
- 当前 H4 的 retained representation 每次更新执行多个 O(M) 操作（#50 W_COUNTERS，8/8 cells）。
- 这些 O(M) retained-state 操作支配了 H4 的大 N 更新成本
  （16 MiB: 唯一 fundamental phase P2 = 0.02% of U_PHASE）。
- memory hierarchy 强烈放大该成本。CPI 1.12 -> 2.57、LLC-miss/block 0.007 -> 9.38
  与 2-4 MiB regime transition 重合（作为关联，不是算术闭合）；这些数来自
  PMU/perf lane 的**等价诊断副本**，属 qualitative / regime-level 支持，
  cycle 份额不精确转移到 U_PLAIN（scope 见报告 §4）。
- R1-R6 是下一个表示原型有证据支撑的、可被否证的目标。
```

证据权威：

```text
Campaign-2   PR #47 merge  334eea6201fc0258e35a7c5b21feb722641ddcbd
H4 根因      PR #51 merge  6cec47e9bb756affb0a4477bcb4962c3c78d8901
机制/计数语义 PR #46 merge  3762b7a42e1c284a4c2c2e0ebac8496e70c63431
```

符号约定（全文一致）：

```text
M = 常驻文档的 top-level block 数（本阶段 workload：128 B/块，
    128 KiB -> 1 024，16 MiB -> 131 072）。所有 O(M) 陈述都指这个量。
U_PLAIN   = 未插桩的原始 frozen H4（`run-plain`）测得的 update 时间 —— 延迟权威。
U_PHASE   = 插桩后互斥 phase lane 的时间；phase share 一律是 U_PHASE 的份额，
            不能当成 U_PLAIN 的份额。
```

### Not proven

```text
- 任何具体的新数据结构是最优的（persistent tree / B-tree / rope / piece tree /
  persistent vector / COW tree / hierarchical checkpoints / reverse label index）。
- 新机制一定会打败 H4。
- persistent tree 能消除全部成本（suffix copy、locators、defs、release 仍在）。
- dependency index 的收益一定超过它的构建与内存成本。
- 精确的硬件 stall 分解（PRECISE_STALL_DECOMPOSITION = UNRESOLVED）。
- 首次 full parse 会变快（本阶段证据支持的是更好的 *更新器*）。
- 生产集成架构（AGENTS.md 的不变量与 architecture HOLD 不变）。
- 任何具体 chunk size / fanout / balancing policy / rebuild selector。
```

---

## 1. Authority map（读证据的入口）

```text
docs/PRD.md, docs/product/**             产品要求权威（无实现授权）
docs/product/architecture.md             HOLD，不因本阶段解冻

research/benchmarks/markdown-ast-update/ 唯一的活跃 Markdown parser 研究权威
    protocol/ grammar/ corpus/ mutations/ workload-freeze/
        冻结研究权威（#35）
    common/ instrumentation/ oracle/ runner/ shared-grammar/ mechanisms/
        可执行 benchmark 基质；机制与计数语义由 PR #46 审计（#41, #46）
    protocol/R5-REAL-WORKLOAD-CORRECTNESS-CLOSURE-v1.md
        H2/H3/H4 在真实工作负载上的正确性 closure（#40，
        PR #40 merge 1af66c09135860f2e11128df646cf6db983acc52）
    results/primary-run/905427da/
        #31 primary performance campaign 的**采集 closure**
        （authority base = PR #43 merge c20d8efb0a864b6b7f458ca170f11532fee78e0d；
        只记录采集完整性与身份，PRIMARY-RUN-CLOSURE.md 明示
        PRIMARY_TIMING = NOT_STARTED，因此它不含 timing 结论）
    results/campaign-2/                  Campaign-2 证据（PR #47 / #31）
    results/h4-large-n-cause-1/          H4 大 N 根因（PR #51 / #50）

docs/research/campaign-2-mechanism-synthesis.md
        Campaign-2 当时的 A-J 解释（历史记录，不追改）
本文件   跨阶段进展综合 + 最终 Weakness Map + R1-R6 可追踪矩阵 + 下一步问题

docs/research/markit-ast-update-design-v0.md
        设计候选（A/B/C/D 分级）

research/experiments/experiment-0-parser-survey/
        归档 Experiment 0（#19 / PR #20），仅历史证据
issue #21
        SUPERSEDED / CLOSED，不得作为 gate
```

权威链：

```text
#35 frozen workload
      -> #40 correctness
      -> #41 measurement substrate
      -> #43 + #44 primary performance campaign（采集 closure；PRIMARY_TIMING = NOT_STARTED）
      -> #46 mechanism/counter semantics
      -> #47 Campaign-2 empirical evidence
      -> #48 mechanism synthesis / A-J interpretation
      -> #50 + PR #51 H4 large-N causal decomposition
      -> 本文件（durable research synthesis + V1 要求）
      -> NEXT PHASE: 新的 Markit 候选机制（尚未开始）
```

注意 `results/primary-run/` 与 `protocol/R5-REAL-WORKLOAD-CORRECTNESS-CLOSURE-v1.md`
是两件不同的事：前者是 #31 的 campaign 采集完整性记录（无 timing），
后者是 #40 的正确性权威。本阶段的 timing 证据来自 Campaign-2（PR #47）与
H4-LARGE-N-CAUSE-1（PR #51），不是来自 primary-run。

---

## 2. 研究进展：证据如何改变了理解

```text
Stage 0  question
    Which AST/CST update mechanisms cost what, where, and why?

Stage 1  H0-H4 mechanism experiment
    五个机制（H0 FULL_REBUILD / H1 BLOCK_LOCAL_REPARSE / H2 FRAGMENT_REUSE /
    H3 OLD_TREE_SUBTREE_REUSE / H4 RESTART_CONVERGENCE）在冻结 workload 下
    正确性完整，并各自暴露不同的成本结构。

Stage 2  Campaign-2 regime map
    no universal winner。H2/H3 在 512 B 输给 H0；H1 到 16 MiB 只剩约 1.063x 的
    H0-relative 比值；H4 在 1 MiB 约 6.347x、到 16 MiB 约 3.695x；
    真实 edit family 内既有大幅胜出也有失败。family 不是充分分类器。

Stage 3  mechanism synthesis（#48 / campaign-2-mechanism-synthesis.md）
    H4 restart/convergence 是最强的结构性正向思想；
    H2 的 syntax/semantic 分离有用；
    H0 full rebuild 仍是必要的 cost-selected normal path 与 escape；
    H3 证明“发现/证明复用”本身可能比解析更贵。

Stage 4  unresolved observation
    H4 的 parser work 保持不变（Campaign-2：1 fresh block、2 fresh nodes、
    138 post bytes），但大 N latency 仍增长：Campaign-2 的 controlled-N lane 在
    1 -> 16 MiB 时 M 约 16x，时间 679.345 us -> 29,267.286 us（约 43x）。
    当时只有假设，没有该点的匹配 profile 与 allocator 数据。

Stage 5  issue #50 causal experiment
    parser work 常数；多个 retained-state 操作是 O(M)：
    damage_records_visited = M、definition_nodes_visited = M、
    prefix_slots_visited = M/2、suffix_slots_visited = M/2 - 1、
    slots_created = M、checkpoint_records_created = M、
    checkpoint_key_clones = M、Arc handle clone/release = M、
    nodes_reused retained walk = 2M - 2。

Stage 6  root cause
    incremental parsing succeeded; incremental state representation did not。
    即："H4 localized parsing but did not localize retained-state maintenance."

Stage 7  design requirements
    R1-R6（见 §5）。它们是有证据支撑的 requirement，不是实现选择。

Stage 8  next research question
    Can a new Markit mechanism satisfy R1-R6 while retaining correctness,
    losslessness, restart/convergence semantics, reasonable construction cost,
    memory footprint, and robust fallback behaviour?
```

**这个顺序本身就是结论的一部分**：Stage 3 的机制解释是正确的方向判断，
但 Stage 4/5 表明“H4 复用不足”不能只归因于 suffix metadata —— 当时的 C4 假设
被 #50 扩展为“多条 O(M) retained-state 流共同支配”，其中没有任何一条单独支配。

---

## 3. Final H0-H4 Weakness Map

| Mechanism | Valuable idea | Confirmed weakness |
|---|---|---|
| **H0** FULL_REBUILD | 简单的全局正确性；rebuild 是正常算法逃逸路径 | 定义上就是 O(N) parsing |
| **H1** BLOCK_LOCAL_REPARSE | bounded local damage（精确损伤界定 + 边界 continuation soundness） | flat/suffix 状态维护，以及 fallback 成本（真实 234/362 case fallback，234 个全部比 H0 慢） |
| **H2** FRAGMENT_REUSE | structural reuse 与 semantic rematerialization 的分离 | 全局咨询 / reference fanout 敏感；候选表反复构造与销毁 |
| **H3** OLD_TREE_SUBTREE_REUSE | subtree reuse 概念本身 | reuse discovery/vouching 自身可以是全局且昂贵的（consult + candidate Vec） |
| **H4** RESTART_CONVERGENCE | safe restart + forward parse + convergence —— 观察到的最强结构性正向机制 | **parser locality ≠ representation locality**：retained state 仍被 O(M) 重建 / 重走 / 退役 |

H4 行在 #50 之后必须读成完整的一句：

```text
H4 achieved incremental parsing (local parser work, constant in N),
but not an incremental representation: the retained state still
undergoes multiple O(M) operations per local edit.

因此：
    incremental parser          = achieved
    incremental representation  = not achieved
```

五条共同弱点（Campaign-2 §E，仍然成立）：

```text
1. 局部语法工作配上全局维护。
2. 复用发现的成本没有被约束。
3. 安全边界过粗或位置映射退化。
4. 语法有效性与语义环境有效性耦合过强。
5. 没有经济停止条件。
```

---

## 4. H4-LARGE-N-CAUSE-1：#50 确立了什么

完整证据：`research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/`
（`H4-LARGE-N-CAUSE-REPORT.md`、`H4-LARGE-N-CAUSE-1-MANIFEST.md`、
`PHASE-MAP.md`、`RAW-HASH-CLOSURE.txt` = 1004/1004 PASS）。

```text
result class          POST_HOC_EXPLANATORY / H4_ONLY / RESIDENT_SINGLE_RESET
producing executables 4d23df55 (U_PLAIN/ablations/PMU/record/eBPF),
                      8185f825 (U_PHASE), aa731757 (W_COUNTERS),
                      fd81e27d (A_ALLOCATOR)  <- executable authority
U_PLAIN latency       86.3 us (128 KiB) -> 24.99 ms (16 MiB); x36.0 over 1->16 MiB
                      （这是 #50 的采集，M x16。Campaign-2 是另一次采集：
                       1->16 MiB 为 679.345 -> 29,267.286 us（约 43x），
                       128 KiB->16 MiB 为 87.531 -> 29,267.286 us（约 334x）；
                       两次采集绝对值不可互换。）
H4_LARGE_N_CAUSE_RESULT = PASS
```

冻结 §14 规则下的分类（DOMINANT 需 >= 50%）：

```text
DOMINANT under the frozen rule: none

LARGEST MATERIAL CONTRIBUTORS
    P6 pairs -> slots/checkpoints        26.6% of U_PHASE, 28.2% of increment
    P5 fresh materialization + suffix    21.5%, 22.8%
    P3 prefix pair assembly              20.2%, 21.2%
MATERIAL
    P7 seal + retirement                 12.4%, 12.9%
    P1 damage scan + restart selection   11.6%, 12.4%
SECONDARY
    P4 definition traversal                8.6%,  9.1%  (Adefs -8.7%)
    Vec capacity churn                    ~2%  (Acapacity; direction-INCONSISTENT at 2/4 MiB)
NEGLIGIBLE
    P2 forward parse + convergence        0.02%
```

没有任何单一操作独占主导；三条 O(M) 重建流合计 68.3% of U_PHASE；
唯一 fundamental 的 phase 是 P2 (0.02%)。**约 99.98% 的 16 MiB update 不是解析工作。**

解释框架（三层，必须分开陈述）：

```text
ALGORITHMIC WORK       linear in M；measured regime 内没有第二个超线性工作项。
COST PER RECORD        large-N regime 上升：cycles/block 294 -> 678。
MEMORY HIERARCHY       STRONGLY SUPPORTED：CPI 与 LLC-miss/block 的上升与同一个
                       2-4 MiB regime transition 重合（关联，不是算术闭合）。
EXACT STALL PARTITION  UNRESOLVED：现有 PMU 数据没有把 excess cycles 分解为
                       LLC-latency / MLP / prefetch / downstream / TLB / allocator。
```

`PRECISE_STALL_DECOMPOSITION = UNRESOLVED` 不阻塞 V1：表示层分类只依赖
phase、counter 与 ablation 证据。

---

## 5. 证据到要求的可追踪矩阵

每个 requirement 都指向仓库内的证据权威，而不是聊天记录或转述。

⚠ **命名空间**：这里的 `R1`–`R6` 是 **V1 行为要求**。`protocol/` 下的
`R0`–`R7` 文件是**另一个命名空间**（研究阶段记录）。引用时写
“V1 requirement R2” 与 `protocol/R7-…` 以区分；两者编号无对应关系。

| Requirement | Evidence | Failure prevented |
|---|---|---|
| **R1** 局部编辑不得要求 O(M) damage / restart search | #50 P1 = 11.6% of U_PHASE；`damage_records_visited = M`（PR #51） | large-N locate tax |
| **R2** 未变 prefix/suffix 不得仅为保留而逐块重走 | #50 P3 = 20.2%（prefix pair assembly）与 P5 = 21.5%（fresh materialization + suffix assembly）；这两个 phase 在冻结证据中各自是整体，**没有** P5 的 suffix/ fresh 数值拆分，不得引用“半个 P5”；counters：`prefix_slots_visited = M/2`、`suffix_slots_visited = M/2 - 1`（PR #51） | 局部编辑重建 retained sequence |
| **R3** 未变 suffix 状态不得要求逐记录重建 slot / checkpoint / `ContextKey` | #50 P6 = 26.6%；`slots_created = checkpoint_records_created = checkpoint_key_clones = M`；A_ALLOCATOR（**诊断副本**lane）在该流上测得约 80 B x M 的 requested payload 字节（requested bytes，不是 DRAM traffic；mechanism scope 见报告 §4）（PR #51） | slots/checkpoints/ContextKey 的 O(M) |
| **R4** 未变语义/定义环境不得要求全语法遍历才能发现它没变 | #50 P4 = 8.6%；`definition_nodes_visited = M` 而表为空；Adefs -8.7%；Campaign-2 的 F 轴（PR #47） | 不必要的全局语义遍历 |
| **R5** 局部编辑不得强制 O(M) 退役整个先前 retained representation | #50 P7 = 12.4%；Adrop -12.4%，且 DERIVED(U_deferred + T_drain) = 42.1 ms >> 24.99 ms | O(M) destruction |
| **R6** 生产 attribution / statistics 不得在热更新路径内要求 O(M) retained-tree walk | frozen source `mechanisms/restart-convergence/src/lib.rs:511,606`（`retained_block_nodes`）+ 诊断副本采样 21.1% self（PR #51；`bin/mdbench-h4diag` 中确认存在 `markit_mdbench_restart_convergence::inline_node_count`） | measurement machinery 变成算法成本 |
| **restart + convergence** 必须保留 | Campaign-2 H4 的 N / B / K / lifecycle 轴（PR #47） | 局部损伤后仍需全文解析 |
| **full rebuild 退路** 必须保留 | Campaign-2 D / F / global-semantic regime（PR #47） | 增量路径可能比全量更贵 |

R1-R6 与后两条共同构成 V1 的**行为要求**。它们不指定数据结构：

> 一个机制，其更新工作与实际语法/语义损伤及 restart/convergence 距离成比例，
> 同时未改动的 retained state 被保留而不承担逐记录维护。

**不得**在此之前写成 “Markit is a persistent B-tree parser” 或任何等价说法。

---

## 6. 方法论教训（本阶段真正学到的）

```text
1. 不要只从时间曲线推断原因。
   Stage 4 的 43x 曲线催生了多个互相竞争的解释，只有显式 counter + ablation
   才能区分它们。

2. mechanism identity、正确性、timer boundary 与 work-counter 语义必须在
   性能解释之前闭合。
   Campaign-2 的 `axis_value` / `KNOWN` attribution 缺陷，以及本阶段的
   provenance/窗口缺陷都属于这一类。

3. 大幅加速不意味着通用赢家；regime map 才是结论。

4. "AST reuse" 不够：发现、表示与退役被复用状态的**成本**同样要计入。
   H3 与 H4 各自以不同方式证明了这一点。

5. 当 parser work 固定而 latency 增长时，先分解 representation work，
   再选择数据结构。

6. 单靠 phase timing 不是因果证明。组合：
       phase timing
     + direct work counters
     + targeted ablation
     + allocator evidence
     + PMU / perf record / eBPF（在有明确 scope 时）
   并且每一层都必须说明自己的 scope。

7. 硬件效应会放大软件自有的工作。先消除不必要的工作，再微优化它的 cache 行为。
   #50 的 O(M) retained-state 流就是“先消除”的对象。

8. derived-summary 缺陷不会使有效的 raw evidence 失效，
   但在结果成为权威之前必须修正，并且要能机械复现修正。

9. 失败的测量尝试必须连同其缺陷一起归档，不能静默替换。

10. producer identity 必须在生产时捕获，而不是在 closure 时用最新 binary 回填。
```

---

## 7. Provenance 与失败历史（为什么这些证据可信）

原则：**未来研究者应当能够重建“为什么最终证据值得信任”**，
而不是只看到一个干净结论。原始 incident 日志不放进设计文档正文，
集中记录在这里，并指向仓库内的原始文件。

### 7.1 Campaign-2（PR #47）已记录的缺陷

```text
- summarizer 对整轴复用第一个 axis_value：controlled 身份必须用 cell_id，
  不能用 derived axis_value 拟合趋势。
- attribution 数字被压成 KNOWN：不能当成工作量。
- 原 receipts 把 closure 时的 executable SHA 赋给所有文件：
  哈希相等不等于 producer 描述正确。
- perf-stat 与 perf-record 不是同一个 executable：必须分别对待。
- PMU 区域宽于主 update timer；内嵌 PMU 无 enabled/running 校正。
- perf-record 覆盖整个 replay 进程：采样百分比只作定性。
- profiling selection 的某些 case 数值/极值叙述与 raw header 不匹配
  （例如真实 P01 对应 e46709f6… 的 12.316/2.604 µs，
   而不是选择表引用的 57.453/4.037 µs）。
- P12 是 late-step 文本的 fresh-state replay，不能单独验证历史效应。
- H4 `nodes_reused>0` 同时包含 prefix 与 suffix：正复用数 != suffix take。
```

这些限制**不要求重跑 Campaign-2**；它们限制的是可声称的结论范围。

### 7.2 H4-LARGE-N-CAUSE-1（#50）的采集尝试

```text
attempt 1  FAILED       driver ROOT 错误；日志保留 collection-driver-attempt1-failed.log
attempt 2  ABORTED      部分采集；日志保留 collection-driver-attempt2-aborted.log
attempt 3  superseded   mixed provenance：lane 由 SHA 不匹配冻结集合的 binary 产生；
                        整个树保留在 attempt-3-mixed-provenance/
attempt 4  superseded   PMU window defect：pmu-run 在构建 fresh state 之前就打开了
                        perf control window，使计数窗口包含 construction
                        （约 20x instruction inflation）；保留在 attempt-4-superseded/
attempt 5  FINAL        正式采集；全部 13 个 timing lane exit 0
```

正式采集期间的 workspace/source state 是 **HEAD `f52fe5a` + 未提交的 h4diag
working-tree 改动**；这些改动在采集结束 16 分钟之后才提交为 `6b2c2a4`。
因此**任何 commit SHA 都不是测量的 provenance**：可执行文件 SHA256 才是。

### 7.3 结果提交之后的 corrective（全部只动文档/派生层）

```text
c6c52bb   结果提交 SHA 记入 manifest
84bc333   root PMU derived summary 全零 —— 其 inline generator 只按 user-mode
          事件名匹配（cycles:u），使 root 文件的 plain name（cycles）静默落成 0；
          raw root 文件始终有效。修正为 committed 的确定性脚本
          perf/derive-stat-summary.py + 回归测试（7/7）。
          同时撤回 "<=1.6% 残差" 的微架构闭合声明，
          改为 PRECISE_STALL_DECOMPOSITION = UNRESOLVED。
f6b7856   独立复核之后的修正（PR #51）：
          - P1：PRODUCER-RECEIPTS.csv 在 84bc333 被修正但未重算其记录 hash，
            closure 只通过 1003/1004；改为重新生成 closure，并新增 committed
            的确定性生成器 hash-closure.py。
          - closure 曾覆盖 46 个被仓库 `*.log` ignore 规则挡在 git 之外的
            evidence log，因此在任何 clone 中都无法验证；改为 scoped
            `.gitignore` negation 纳入跟踪。结果 1004/1004 PASS，
            并在全新 clone 中同样 PASS。
          - provenance / PMU 机制范围 / cache 措辞 / eBPF scope /
            分类阈值 / allocator 算术 / Campaign-2 与 #50 数字混用 等
            P2 类修正（明细见 PR #51 的 manifest §12）。
```

完整明细与逐条修正理由：
`research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/H4-LARGE-N-CAUSE-1-MANIFEST.md` §12。

### 7.4 独立复核记录（两次，必须分开读）

本阶段经历了两次独立复核：修正前的一次（它产生了 `f6b7856`）与放行合并的一次。
两次结论对应两个不同状态，**不能混引**。下表逐行给出记录位置；除此以外没有别的
复核来源。

| 记录 | 位置 | 内容 |
|---|---|---|
| 修正前复核 | commit message `f6b7856`（首段，逐字） | `ISSUE50_FINAL_CAUSAL_CLOSURE = PASS`、`PR51_EVIDENCE_INTEGRITY = CONDITIONAL_PASS`、exactly one P1 blocker、`rerun_required = NO`；该 P1 即 `RAW-HASH-CLOSURE.txt` 只通过 1003/1004 |
| 放行合并复核 | PR #51 review comment，2026-09-24T10:16:48Z，reviewed head `f6b7856d39dff84bc2b1a5f90b4903f4149a9a8c` | 下方 PASS 表 |
| 合并结果 | merge commit `6cec47e9bb756affb0a4477bcb4962c3c78d8901`（本文件的 base） | 合并与 #50 关闭 |

放行合并复核的逐字内容：

```text
ISSUE50_FINAL_CAUSAL_CLOSURE            = PASS
PR51_EVIDENCE_INTEGRITY                 = PASS
P0 = 0   P1 = 0
RERUN_REQUIRED                          = NO
V1_REPRESENTATION_PROTOTYPE_AUTHORIZED  = YES
PR51_MERGE_AUTHORIZED                   = YES
ISSUE50_CLOSE_AUTHORIZED                = YES
```

`RERUN_REQUIRED = NO` 是本阶段的一条重要判断：**文档层缺陷不构成重跑实验的理由**，
但这要求修正本身可机械复现（因此 closure 现在由 committed 生成器产生）。
两次复核都只覆盖 #50/PR #51 的证据；`V1_REPRESENTATION_PROTOTYPE_AUTHORIZED = YES`
的授权范围是“可以为表示原型开一个新的 scoped 研究”，**不是**“V1 已获准实现”，
也不为任何具体数据结构授权。

---

## 8. 下一个研究问题

```text
NEXT_RESEARCH_QUESTION =
    Can a new Markit candidate mechanism satisfy R1-R6 while retaining
    correctness, losslessness, restart/convergence semantics, reasonable
    construction cost, memory footprint, and robust fallback behaviour —
    and does it beat H0-H4 under the same semantic core, oracle,
    payload/edit contract, workload and measurement protocol?
```

下一阶段的工作（**尚未开始**）：

```text
1. 从 R1-R6 设计一个新的 Markit 候选机制，给它新的实验身份。
2. 在相同的 semantic core / oracle / payload-edit contract / workload /
   measurement protocol 下与 H0-H4 比较。
3. 不以“打败 H4 多少倍”作为唯一成功条件；判别的是**工作是否被消除**。
```

**下一个 horse 不得被定义为 “H4 but optimized”。** 在其机制身份被显式设计之前，
不得进入实现。设计必须先回答：

```text
- What state does it own?
- How is edit location found?
- How is restart chosen?
- How is convergence proven?
- How are unchanged prefix/suffix retained without O(M) work?
- How are coordinates represented?
- How are definitions / semantic dependencies invalidated?
- How is old state retired?
- When does it choose full rebuild?
- What construction / memory tax does the state impose?
```

现有设计候选
[markit-ast-update-design-v0.md](markit-ast-update-design-v0.md)
已经给出这些问题的一个候选答案与 A/B/C/D 分级；它是**输入**，不是结论。

### 不要过早优化的事项

```text
- 不要在测量之前选定 chunk size / fanout / balancing policy。
- 不要在测量之前承诺 persistent / COW。
- 不要为尚不存在的快照读者增加 persistence。
- 不要先建通用 fragment table 或第二棵全局复用索引。
- 不要把 cache 微优化放在消除 O(M) 工作之前。
- 不要把 BENCH-GRAMMAR-v1 原型当成产品方言。
- 不要重开 #21、复活 archived core、开始 UI/runtime 设计，
  或为尚不存在的算法先做形式化。
```

---

## 9. Canonical references

```text
frozen workload / protocol        research/benchmarks/markdown-ast-update/protocol/
                                  workload-freeze/ grammar/ corpus/ mutations/
mechanism + counter semantics     PR #46 merge 3762b7a42e1c284a4c2c2e0ebac8496e70c63431
H0-H4 correctness parity          PR #30 merge 12952a561c79fa051bca6bae7409ef536cd43011
real-workload correctness (#40)   protocol/R5-REAL-WORKLOAD-CORRECTNESS-CLOSURE-v1.md
                                  (PR #40 merge 1af66c09135860f2e11128df646cf6db983acc52)
primary campaign collection       results/primary-run/905427da/PRIMARY-RUN-CLOSURE.md
                                  (#43 authority base c20d8efb…; PRIMARY_TIMING = NOT_STARTED)
Campaign-2 evidence authority     PR #47 merge 334eea6201fc0258e35a7c5b21feb722641ddcbd
    research/benchmarks/markdown-ast-update/results/campaign-2/
    CAMPAIGN-2-FREEZE.md / CAMPAIGN-2-CLOSURE.md / EVIDENCE-GAPS.md
    GPT6-FULL-EVIDENCE-HANDOFF.md / EXECUTABLE-ATTRIBUTION-CORRECTIVE-1.md
H4 root-cause evidence authority  PR #51 merge 6cec47e9bb756affb0a4477bcb4962c3c78d8901
    research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/
    H4-LARGE-N-CAUSE-REPORT.md / H4-LARGE-N-CAUSE-1-MANIFEST.md / PHASE-MAP.md
    RAW-HASH-CLOSURE.txt (1004/1004 PASS) / derived-classification.csv
Campaign-2 historical synthesis   docs/research/campaign-2-mechanism-synthesis.md
engineering direction             docs/research/campaign-2-to-ast-guidance.md
design candidate (A/B/C/D)        docs/research/markit-ast-update-design-v0.md
```

命名空间提醒：`protocol/R0`–`R7` 是研究阶段记录；本文件的 `R1`–`R6` 是 V1 行为要求。
两者编号无关，引用时分别写 `protocol/R7-…` 与 “V1 requirement R3”。

本阶段的实验实现（`h4diag/`）是**诊断 instrumentation**，不是产品代码，
也不是未来的 benchmark baseline；它随 #50 冻结，不继续演进。

```text
CAMPAIGN2_EVIDENCE               = CLOSED   (evidence stage; PR #47 merged)
H4_LARGE_N_CAUSAL_DIAGNOSIS      = CLOSED   (issue #50 closed on PR #51 merge)
MECHANISM_SYNTHESIS              = RECORDED (issue #48 stays OPEN — only the
                                             maintainer closes it)
RESEARCH_HANDOFF_DOCUMENTATION   = RECORDED (this document)
UMBRELLA / EXECUTION ISSUES      = OPEN     (#22, #31, #33, #48)

V1_REQUIREMENTS                  = FROZEN_AS_EVIDENCE_BACKED
V1_DATA_STRUCTURE                = UNDECIDED
NEW_HORSE_IMPLEMENTATION         = NOT_STARTED
```
