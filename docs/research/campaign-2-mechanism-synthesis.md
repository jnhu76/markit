# Markit Campaign-2：机制综合独立评审

```text
Historical Campaign-2 mechanism synthesis
Evidence authority:  PR #47 (merged, 334eea6201fc0258e35a7c5b21feb722641ddcbd)
Reviewed snapshot:   819928966e05ca00bb82c58b887014e54f3d0cd5 (PR #47 head at review time)
Later H4 large-N root-cause evidence: #50 / PR #51
                     (merged, 6cec47e9bb756affb0a4477bcb4962c3c78d8901)
```

**这是一份历史文档，按当时（2026-09-23）可获得的分析上下文保留原样。**
它记录的是 Campaign-2 解释，而不是最终解释；`#50 / PR #51` 之后才取得的
H4 大 N 根因证据**没有**被回写进正文。跨阶段综合、最终 Weakness Map 与
R1-R6 可追踪矩阵见
[markit-31-research-synthesis.md](markit-31-research-synthesis.md)。

后续证据对本文的具体影响，记录在文末的 **Later evidence** 一节 —— 包括 §C4 的
suffix-metadata 假设如何被 #50 扩展为“多条 O(M) retained-state 流共同支配”。

审查日期：2026-09-23。研究目的：识别局部性、传播、语义依赖与维护成本，决定 Markit 应保留哪些最小机制；不产生 H0–H4 排行榜。

- 实现与主证据：PR #47，`819928966e05ca00bb82c58b887014e54f3d0cd5`。
- 算法权威：`3762b7a42e1c284a4c2c2e0ebac8496e70c63431`，包含 PR #46 审计。
- 用户补充的原始证据：`research/31-campaign-2-review-inputs`，本次取得的提交 `7f2cdb2`，两个 tar.gz 包。
- 未使用 PR #45 作为研究权威；未修改仓库、issue 或 PR；未重新运行性能 campaign。
- 所有性能结论限于 BENCH-GRAMMAR-v1、冻结工作负载及本次机器/构建。不能外推为 Tree-sitter、Lezer、完整 CommonMark/GFM 或编辑器端到端性能结论。

**总判断：数据可以用于机制综合；候选架构应 MODIFY。Markit 应采用可局部寻址和拼接的语法状态、保守而精确的 restart/convergence、按实际语义依赖修复，以及有预算的全量重建。当前证据没有证明必须采用某一种 persistent tree，也没有证明维护更多复用索引总是值得。**

## A. Evidence verdict

```text
CAMPAIGN_2_ANALYSIS_USABLE = YES
CANDIDATE_SYNTHESIS       = MODIFY
PRODUCTION_DESIGN_PROVEN  = NO
```

### A1. 核验结果与分析口径

1. GitHub PR 元数据的 head/base 与用户给定值一致；权威到审查 head 的 mechanisms、runner/src、common/src、oracle/src、shared-grammar/src 没有改动。
2. 补充包的 **34/34 个原始文件**，合计 **84,428,616 bytes**，全部与审查 head 的原始清单 SHA-256 相符。包含 10 个 attribution 文件与 24 个区域 profiling 文件。
3. 10 个 attribution 文件共 **29,015 行**：construction 110、resident update 1,810、controlled 215、lifecycle 26,880。所有这些行报告 correctness pass。逐文件检查身份，并按源码规定的换行分隔顺序独立重算 RunId，**10/10 匹配**；并非只接受 corrective 的自述。
4. 生命周期三个 session 的 8,960 个 trace-step-horse 的计数、state_repr 与结果 checksum 完全一致。原始 attribution **每步都带 state_repr**；“只在链尾导出”的文档描述只适合原 timing 出口，不能覆盖新增 attribution 路径。
5. lifecycle checkpoint 表与 per-step p50 表的累加关系全部相符。这验证的是 **DERIVED 求和**，不是一个实测 enclosing wall-clock interval。
6. 主 timing 原始大文件未随此次补充包提供；本报告的主时间数值来自提交的 timing CSV，检查过 summarizer 的分组、分位数和会话处理。没有声称重新计算全部 1,091,615 行主证据。

三个时间面严格分开：

- `C_h`：clean parse + 本机制状态构建与 seal。
- `U_h`：已有 resident state 的 prepare + update + complete；旧状态构建在计时外。
- `L_h(128)`：同一状态上的 128 次链式更新；本文使用逐步计时及其明确标注的派生统计。

所有单个时间表中的 μs，除另行说明，都是三 session p50 的中位数。speedup 使用冻结的“逐 session H0/Hx，再几何平均”口径；因此不能用两个跨 session 中位数相除替代，尤其在接近 1 的点上。

### A2. 必须降级或修正的分析字段

这些限制不要求重跑 Campaign-2。

| 问题 | 本次处理 | 不能据此声称什么 |
|---|---|---|
| summarizer 对整轴复用第一个 `axis_value` | controlled 身份使用 `cell_id`，轴值由 raw cell/manifest 核对 | 不用错误 `axis_value` 拟合趋势 |
| attribution 数字变成 `KNOWN` | 所有数字归因直接读取 raw attribution | `KNOWN` 不是工作量 |
| 原 receipts 把 closure 时的 executable SHA 赋给所有文件 | 接受并独立验证补充包的 observation RunId 派生修正 | 哈希相等不等于所有 producer 描述都正确 |
| perf-stat 和 perf-record 并非同一 executable | stat/replay 区域文件为 `2b4ac6b8…`；record header/stdout 为 `b864fe53…`，分别对待 | corrective 中“所有 profiling 同一 binary”的说法仍不成立 |
| PMU 区域宽于主 update timer | 从源码确认包括 Source/参考结果 clone、checksum、oracle/projection，以及本轮临时对象销毁 | 不能将 PMU 数字当作纯 `U_h` 指令/周期分解 |
| 内嵌 PMU 无 enabled/running 时间校正 | 六个 event 独立打开，`read_format=0`，只读 u64；小 slot 有不可信计数 | 不能推导可靠 IPC、cache/branch 因果或 cycles→time；文档对该路径的“内核已缩放”解释不受源码支持 |
| `perf record` 覆盖整个 replay 进程 | 仅使用机制特有热点支持定性归因 | 不能把采样百分比当作主 update 时间占比 |
| profiling selection 某些真实 case 的数值/极值叙述错配 | 按 raw header `case_id` 回连主 timing；不按选择表文字归因 | P01 并未 profile 表中所称的 57.5→4.0 μs case |
| P12 是 late-step 文本的 fresh-state replay | 生命周期结论只用真正 chained-state 数据 | P12 不能单独验证历史效应 |
| H4 `nodes_reused>0` 同时包含 prefix 与 suffix | 结合 restart、edit offset、convergence 和源码判定是否 suffix take | 正复用数不等于后缀收敛 |

例如真实 P01 header 对应 `e46709f6e4c8…`，主表 H0/H4 为 **12.316/2.604 μs**；选择表引用的 **57.453/4.037 μs** 属于 `318adda97a09…` 的 paragraph merge。P02 同样应按 `c20431ee65a0…` 读取 **24.071/39.314 μs**。这个问题限制 profiling 与主时间的匹配叙述，不污染按 case_id 保存的主 timing。

**分析级 blocker：没有阻断机制综合的 blocker。** 仍阻断的是：精确 PMU 因果分摊、H4 16 MiB 额外退化的唯一主因判定、生产 persistent representation/依赖索引的收益与内存结论。它们应保持 unknown 或 hypothesis，而不是补造精确结论。

## B. Mechanism regime map

### B1. 五种机制分别提供什么信息

| Horse | 强区间 | 弱区间／改变符号的因素 | 主收益 | 主成本 |
|---|---|---|---|---|
| H0 | construction；增量需要重做大部分结果、global invalidation、广泛传播；一些小而结构密集的真实文件 | 局部变化且大部分高成本语法/语义可保留 | 不做 damage/reuse 搜索、无维护增量索引、无失败后的二次解析 | 每次丢弃旧树并全量解析/构造 |
| H1 | bounded region、无定义、边界 guard 通过；尤其受影响区域小而其余文档扫描昂贵 | 任意 retained definition 就触发 F1；边界合并/开 fence；大 suffix/大 B；大 N | 精确界定损伤，跳过其他块的 block/inline parse | 全 tiling scan；suffix skeleton/semantic 坐标重建；先 probe 再 fallback |
| H2 | 局部编辑可形成长合法 take run；C-F 低 occupied-slot 数 | 512 B 的固定成本；F 增大；候选多而接受少；C-D；容器边界保守拒绝 | 保留结构与 semantic payload；必要时可用保留的内容区间重做 inline | 重复枚举/clone old entries；全局 defs assembly；广泛 `[` probe；known-ref 拒绝复用 |
| H3 | 中等规模的安全局部编辑，changed ancestry 外的 run 可整段接受 | C-F 的位置映射退化，即使 F=0；C-D 的反复失败咨询；小 N/高重建比例 | 保留 unaffected subtree 身份及其 semantic payload；能下降穿过受损祖先寻找子树 | patched-tree 构建、stale alignment、每次 consult 重建候选 Vec、搜索与销毁 |
| H4 | 小 restart distance、短 forward parse、可安全 suffix take；真实局部编辑和 L1/L2/L3/L7 | 大 B/粗 container restart unit；定义变化的 restart-at-zero；到 EOF 无后缀；大 N 的全局状态维护 | prefix 不进 parser；收敛后 suffix 不进 parser；完整 skel+sem Arc 保留 | 线性 damage/restart 寻找；prefix/suffix slot 重建；defs traversal；全 checkpoint 注册 |

H0 应成为 **cost-selected normal path，同时承担 escape/fallback**。但“文件小”不足以单独触发 H0：C-N 512 B 时 H1/H4 仍优于它。

### B2. Controlled N：局部解析量不变，维护成本随文档增长

固定受影响块 128 B、插入 8 B、近中部编辑。

| N | H0 μs | H1 μs | H2 μs | H3 μs | H4 μs |
|---|---:|---:|---:|---:|---:|
| 512 B | 1.934 | 1.260 | 2.246 | 2.095 | 0.990 |
| 16 KiB | 62.014 | 19.348 | 23.803 | 18.833 | 9.273 |
| 128 KiB | 546.657 | 175.102 | 230.376 | 184.420 | 87.531 |
| 1 MiB | 4,340.716 | 1,450.949 | 2,593.961 | 1,922.759 | 679.345 |
| 16 MiB | 108,447.983 | 101,975.020 | 54,936.024 | 49,327.173 | 29,267.286 |

- H2/H3 在 512 B 输给 H0；H2 的 1 KiB 比值约 1.016，不能宣传为稳健 crossover。
- H1 到 16 MiB 只剩约 1.063× 的 H0-relative 比值。
- H4 在 1 MiB 约 6.347×，到 16 MiB 约 3.695×；仍保有优势，但不具备大小无关的局部更新时间。
- 此轴没有证明“大 N 必然应该选 H0”。它证明当前几种增量表示都有非局部维护工作。

### B3. Controlled B：解析区域增大，但没有测到必然翻转

N 固定 128 KiB。B 从 128 B 到 64 KiB：

| Horse | B=128 B μs | B=64 KiB μs | 解释 |
|---|---:|---:|---|
| H0 | 560.714 | 405.809 | padding 块数减少，结果节点数下降；N 固定不等于 full cost 固定 |
| H1 | 228.331 | 293.619 | region parse 增长，同时 suffix 节点数减少 |
| H2 | 239.869 | 261.777 | 大 leaf 需要重做，候选/metadata 工作反向减少 |
| H3 | 183.450 | 285.598 | large block 与 reuse consultation 叠加 |
| H4 | 86.927 | 170.984 | bounded parse 成本开始显著；仍保留约 2.374× 优势 |

H4 fresh blocks 始终 1、fresh nodes 始终 2；但 post inspected bytes 从 **138→65,546**，convergence distance 从 **136→65,544**。因此“只重建两个节点”不代表工作常数。这个轴不支持把 B/N 的某个比例固化成通用 rebuild 门槛。

### B4. Controlled D：名义 D 不是实际收敛距离

| 名义 D | H0 μs | H1 μs | H2 μs | H3 μs | H4 μs |
|---|---:|---:|---:|---:|---:|
| 64 B | 779.989 | 1,032.104 | 16,432.068 | 16,600.757 | 985.373 |
| 32 KiB | 637.469 | 885.229 | 23,054.237 | 22,984.856 | 712.515 |
| 64 KiB | 507.986 | 677.937 | 19,965.535 | 19,600.604 | 498.624 |

所有 D 点的 H4：restart distance **516**；`r=32,768`；forward distance **98,301**；post length **131,069**；因此 `r+distance=EOF`，**没有 suffix take**。`nodes_reused=1,024` 是前缀复用。用“nodes_reused>0”来判断后缀收敛会得出错误结论。

原因是生成器里的五反引号“第二 closer”，在旧文档中也会成为 opener，使后面 suffix 的解释发生翻转。名义 D 不单独控制语义损伤终点。它测试的是 fence-state 改变与广泛结果变化，不是纯粹可控的 bounded-D 收敛曲线。

H2/H3 相对 H0 约慢 **21–39 倍**。H4 在最大 D 的跨 session 几何比约 **0.984**，虽 case-estimate 中位数略低于 H0，也不应宣布明确获胜。

### B5. Controlled F：低扇出优势存在，但不是 O(F) 的纯语义修复

这里 F 是 **含引用的 64 B 段落槽数**；每个槽有 5 次 `[t…][rd]`，故实际引用出现数是 **5F**。这两种 fanout 单位必须区分。

| F（槽） | H0 μs | H2 μs | H2 fresh blocks | H2 rebuilt nodes | H2 reused nodes | H2 unique post bytes |
|---|---:|---:|---:|---:|---:|---:|
| 0 | 826.879 | 583.588 | 2 | 4 | 4,092 | 126,954 |
| 2 | 833.748 | 742.257 | 4 | 8 | 4,088 | 126,961 |
| 4 | 838.392 | 915.318 | 6 | 12 | 4,084 | 126,965 |
| 16 | 907.875 | 1,499.579 | 18 | 36 | 4,060 | 126,989 |
| 64 | 878.456 | 4,060.221 | 66 | 132 | 3,964 | 127,085 |

观测 crossover 位于本组 F=2 与 F=4 之间，但它不是通用阈值。描述性拟合 `U_H2≈641.45+53.61F μs`，仅对这 8 点成立，不能解释成“每个 use-site 的语义修复成本”。

解释：删除定义使 `definition_changing=true`，H2 的 `search_level` 对 `has_ref` 节点拒绝 take，于是该序列呈现 **fresh blocks=F+2**，并非仅 rematerialize F 份 retained payload。每次更多拒绝/新 take 都可能重新构造全部 old top-level entries；此外 env change 后 `mentions_reference` 扫描近整篇 source，即使 F=0。

H1/H4 在全部 F 点均做全量路径，基本不享受低 F。H3 在 F=0…64 大致 **117–123 ms**，始终 fresh blocks=1,920、rebuilt=3,840、reused=256、metadata=5,896；这不是 fanout 的必要成本。

### B6. Controlled K：首先是恢复粒度变化，而不是深度本身的单调灾难

| K | H0 μs | H1 μs | H2 μs | H3 μs | H4 μs |
|---|---:|---:|---:|---:|---:|
| 0 | 853.584 | 376.986 | 522.359 | 407.745 | 166.453 |
| 1 | 835.749 | 563.332 | 749.036 | 593.811 | 361.618 |
| 12 | 842.976 | 541.622 | 705.891 | 626.445 | 406.278 |

最大变化发生在 K=0→1：H4 fresh blocks **1→258**，forward distance **72→17,480**；K=12 为 269 blocks、28,744 B。编辑落在 256 sibling 的容器首行，H4 checkpoint 是 top-level；H2/H3 的 raw-source blank-line margin 也不能把带 `>` 的内部空行当成纯空白，因此内层复用保守退化。

结论：需要正确恢复 container context；但“必须重做全部 256 siblings”是当前安全边界/表示的选择，不是 Markdown 的普遍下界。当前数据未证明需要在每个嵌套节点放 checkpoint，更没证明收益值得状态成本。

### B7. 真实 edit family 与 construction

以下为每个 edit family 内 case-equal 的 H0-relative 几何平均，仅用于展示同一机制在不同 regime 的变化，不是总体工作负载排名或用户频率模型：

| family | cases | H1 | H2 | H3 | H4 |
|---|---:|---:|---:|---:|---:|
| ATX heading toggle | 12 | 0.703 | 2.236 | 2.337 | 3.987 |
| E1 local text | 51 | 1.281 | 1.750 | 1.887 | 3.280 |
| E2 split/merge | 83 | 0.980 | 1.734 | 1.887 | 3.414 |
| E3 container | 50 | 0.951 | 1.460 | 1.568 | 2.327 |
| E4 fence | 44 | 0.896 | 1.438 | 1.599 | 2.286 |
| E5 inline delimiter | 102 | 1.192 | 1.777 | 2.004 | 3.583 |
| E6 reference definition | 20 | 0.649 | 0.904 | 0.843 | 0.650 |

family 不是充分分类器：同一类既有大幅胜出也有失败。真实 H1 的 **234/362** 个 case fallback，**234 个全部比 H0 慢**；128 个非 fallback case 仍有 26 个较慢。H4 中：290 个能 suffix take（其中 18 个仍较慢）；42 个到 EOF 而无 suffix take（30 个较慢）；30 个从零解析且零复用（30 个全部较慢）。

构建单独看，22 个 FULL_READ 的 C/H0 几何平均为 H1 **1.301**、H2 **1.260**、H3 **1.238**、H4 **1.234**，每个文件均有构建溢价。这不能用来解释 resident update crossover。

若要计算 amortization，必须在同一个文档/编辑分布上比较：

`C_new − C_H0 < Σ(U_H0,i − U_new,i)`。

不能把这 22 个构建样本与另一组任意更新平均值拼起来宣布“第几次编辑回本”。

## C. Causal map：优势到底省掉了什么

置信标签：**观测**=原始计数/主 timing；**强支持**=计数、源路径与时序共同支持；**假设**=仍需 ablation；**未知**=本次证据无法判定。

| 观测 | 省掉／增加的工作 | 源码原因 | 置信度 |
|---|---|---|---|
| 44,914 B 的真实 local edit：H0 53.256 μs；H1 3.963；H4 4.006 | 不再扫描并重建其余文档；H1 只检查 35 post B，fresh blocks=1；H4 35 post B、fresh nodes=2、保留50 nodes | H1 region tiling；H4 restart+suffix take；保留 semantic payload | 强支持 |
| 真实 paragraph merge：H0 57.453 μs，H4 4.037；H1 61.210 | H4 前向修复 merge 后在55 B处停止；H1 guard 退回全量 | H4 安全 convergence；H1固定region+continuation guard | 强支持 |
| H1 C-N始终只 parse 1 block，却到16 MiB接近H0 | suffix没有重parse，但仍重建坐标表示 | `shift_entry_owned/shift_skel/shift_node_owned`；linear damage scan | 强支持 |
| H2低F比H0快，尽管扫描约127 KiB | 保留绝大多数 block结构和inline payload，避免全量materialization | fragment takes；但 semantic候选发现仍广泛扫描 | 强支持 |
| H2从F0的0.584ms升至F64的4.060ms | 更多ref block拒绝take、候选表反复构造与销毁；不是只增加inline修复 | `definition_changing && node.has_ref`；`find_run` 每次建old-entry Vec | 强支持；精确时间拆分未知 |
| H3 F0就约119ms，F增加计数不变 | 删定义整行时位置映射失准；suffix候选拒绝；随后大量consult+candidate Vec churn | 定义node span不含LF；32B删除作用于31B span，size夹到0，单hit的-1 residual未传给后续gap；后续位置差1 | 对源码算术及反常计数强支持；未做反事实修复实验 |
| H2/H3 C-D仅parse剩余变动区域，却比H0慢数十倍 | parser省下的工作被失败候选搜索/clone/drop覆盖 | forward hook + 每次find_run遍历old top-level entries | 强支持；record热点亦支持 |
| H4 C-N保持forward=136 B，但时间随N涨 | 节点身份保留不等于metadata免费：restart/damage、defs、slot/cp都走全局路径 | `.rposition`、全blocks damage scan、prefix/suffix pairs、重建cp向量、collect_defs | 强支持 |
| H4真实fence restore 37.076 μs，H0 17.405 | 先forward skeleton，再发现def环境变化后restart0；fresh blocks61 vs H0 57 | assembled table比较在forward后；discarded work计数未丢失 | 强支持 |
| K0→1开销跃升 | 单块粒度变为整container及siblings；更长context恢复 | top-level checkpoints；raw空白margin保守 | 强支持 |
| 16 MiB相对1 MiB的额外非线性退化 | allocator、working set、cache、内存搬移可能参与 | 已见大量Vec/Arc/metadata路径，但缺该点匹配profile与allocator数据 | 假设；不能指定为某种cache miss原因 |

### C1. H1 的核心与可以删除的机制

H1 最有用的思想确实接近 **精确且可证明的 damage bounding**，但还包括边界 continuation soundness。不能只保留“找最近块”而删掉合并、容器和 fence 检查。

新设计不需要继承：全 tiling scan、按绝对坐标逐节点平移、独立全局 definitions Vec 的无条件失效策略、边界一不确定就先做完整 probe 后全量重跑。Blank coverage 的语义可用 source sequence 的 gap/trivia 区间表达，不必复制 H1 的一套并行对象。

### C2. H2 的 structural reuse / semantic reuse / rematerialization

三者不是同一件事：

- structural reuse：块边界、容器结构与内容区间仍有效。
- semantic reuse：之前解析出的 inline/reference 结果也仍有效。
- rematerialization：利用保留结构和内容区间，再执行受影响 inline 解析；可保留不相关后代，但被修复节点本身不一定保持 Arc 身份。

H2 的 `rematerialize` 和已有 audit/reference-environment fixture 支持这种分离是可实现的，尤其“fence变化使远处definition出现/消失”和 unresolved→resolved 的情形。

**但 C-F 本身不证明纯 rematerialization 的成本曲线。** 已知ref块在那里被拒绝结构复用，计数正是 F+2 fresh blocks；同时 `mentions_reference` 仍按全环境变化扫描广泛源区间。

所以反向依赖索引是有依据的改进假设，不是已经获证的免费组件。它应同时消除：无关块的 `[` 扫描、拒绝所有含ref块的粗失效，以及每个消费者引起的全候选枚举。只加一个 `definition→uses` 表，却保留这三条路径，不能保证获得一般性收益。

### C3. H3 的负面证据边界

P09 H3 whole-process record 中 consult、candidate-vector drop、search_level 的 self samples 分别约 **44.17%、40.69%、7.82%**；P07 H2/H3的同类热点也占多数。这支持“发现/证明复用本身极贵”，而不是“语义修复必然极贵”。采样覆盖整个进程，百分比不能迁移到主 update timer。

必须修正一个过强命题：**全局线性 reuse discovery 并非永远无法胜过 full parse**，H3 的 C-N 128 KiB 就约有2.97×优势。正确命题是：若 Markit 要使固定局部编辑的成本不随全文增大，reuse discovery 和 metadata maintenance 必须局部；频繁 O(M)候选构造不能藏在“复用”之下。

不能据 H3 评价 Tree-sitter；这里存在本实现的delta残差退化与候选列表构造方式。

### C4. H4 是否主要输在 suffix metadata

证据能确认：**存在不可忽略的全局表示维护税；不能收窄成只有 suffix relocation，更不能把大N加速比回落全部归给它。**

| N | H4 fresh blocks/nodes | H4 reused nodes | unique post bytes | metadata records | U μs |
|---|---|---:|---:|---:|---:|
| 512 B | 1 / 2 | 6 | 138 | 18 | 0.990 |
| 128 KiB | 1 / 2 | 2,046 | 138 | 3,588 | 87.531 |
| 1 MiB | 1 / 2 | 16,382 | 138 | 28,676 | 679.345 |
| 16 MiB | 1 / 2 | 262,142 | 138 | 458,756 | 29,267.286 |

这里 metadata约为 `3.5M+4`，M为top-level block数。它包含horse自定义的触碰规则，不是硬件访存总数；H2在C-N中metadata一直13，却也有线性候选列表和结果装配，已经说明不能跨horse直接按此数字比较“全部metadata工作”。

1→16 MiB 时M增长16倍，而H4时间增长约43倍。因此数据支持非局部维护限制扩展性，但无法仅凭计数解释额外非线性。没有C-N大点的匹配perf-record，也没有allocation计量来分离这部分。

### C5. 生命周期：没有观测到累积型退化，周期性坏路径仍很明显

统计方法：对每个step取三个session p50的中位数；比较step0–31与96–127的均值；对交替trace保持break/restore相位均衡。均值和累计值都标为派生描述，不是一次完整链的墙钟计时。

| trace | 观测及机制解释 |
|---|---|
| L1 repeated local text | 每horse工作计数恒定。H4始终512 blocks/512 checkpoints；H2 fragment count始终1。晚/早时间比H1≈1.000、H2≈1.002、H3≈0.995、H4≈0.976 |
| L2 moving edit | 状态记录数不变。H1晚/早约0.892，与编辑逐步后移、要平移的suffix变短一致；不是“越用越快”的缓存机制证明。H4计数也随位置改变 |
| L3 split/restore | H1每128步fallback64次；两相派生均值117.38/351.76 μs；H4约44.51/43.61。慢恢复重复发生，没有逐周期增长 |
| L4 container mutation | 表示计数只在两个有效结构间切换；H2/H3晚/早约1.015/1.016，未见持续增长证据 |
| L5 fence open/restore | H2约235.52/5,452.14 μs，H3约193.22/5,523.64，H4约125.98/341.03；显著的方向不对称，不能误称历史累积。H1 fallback64次 |
| L6 definition destination change/restore | H1 fallback128次；H4持续restart全量。H2/H3工作计数恒定，约1.85/1.81ms；H0约0.60ms。低效模式稳定存在 |
| L7 mixed | 是移动的四步局部replace/insert/delete循环，source size 65,536–65,540；不是包含所有语法编辑的大杂烩。H2 fragments=1，H4 checkpoints=512；晚/早未见持续退化 |
| 7条真实BREAK/RESTORE | 每个horse每条trace只有两个work signatures；同一checksum对应的state_repr未漂移。H1 ATX、refdef各128次fallback；quote、fence各64次；codespan、emphasis、list为0 |

对所有trace，重复出现同一result checksum时，没有观测到state_repr字段漂移。H4 checkpoints与blocks始终1:1；H3 `old_tree_index_entries`只是tree.node_count出口，不代表还存了一个独立索引。

**可下结论：这些128步链中，没有观测到表示记录累积、复用逐步恶化或fallback hysteresis。不可下结论：不存在heap retention/allocator碎片、没有任何长期历史问题、单调增长或长非周期编辑已获验证。** state_repr不是分配字节，RSS也不能替代它。

## D. Fundamental vs accidental costs

| Cost | Horse | Markdown语义必要性 | 当前表示/算法成本 | 可避免性与证据 |
|---|---|---|---|---|
| 确认实际受影响语法及continuation | H1–H4 | 必须保证精确边界；最坏情况可传播很远 | 以整个top-level块为单位可能过粗 | 保留证明责任，不能笼统删掉；K/B表明当前粒度有改进空间 |
| fence状态改变后解析直到安全等价点/EOF | H0–H4 | 真实状态和结果改变时必要 | 多次失败咨询不是必要 | C-D证实广泛损伤；Q次全表搜索可去除 |
| 局部编辑导致整个suffix坐标重写 | H1，H4 metadata | 否；位置必须可正确查询，不要求立即逐条改写 | absolute spans、flat Vec、parallel checkpoints | 可用相对长度/聚合索引/lazy delta避免；实际收益待测 |
| 全量damage scan / restart线性查找 | H1/H3/H4 | 否 | flat layout和查询实现 | 单一位置索引可使查询局部；不用新全树复用索引 |
| 每次consult重建old candidate Vec | H2/H3 | 否 | O(M)枚举、Arc clone/drop，可能乘consult数Q | 强负面证据；本设计直接删除 |
| 更新后收集全文definition facts | H2/H3/H4 | 有效定义环境必须正确；全文重收集不必要 | repeated tree walk、Vec assembly、whole-table compare | 按变动语法区维护定义发生位置和winner |
| 某label的实际消费者语义改变 | H2/H3，H1/H4以全量覆盖 | 对已发生变化的输出必须修复；最坏F可为全局 | 按所有ref或所有`[`块失效过宽 | 受影响label→候选consumer；下界按实际输出变化计，不是无条件Ω(F) |
| 删除首定义后寻找同label后继定义 | 全部 | first-wins语义必要 | 扫全文不是必要 | 需要有序定义候选，或接受查询时扫描的成本 |
| unresolved→resolved修复 | H2/H3等 | 必要，光存已解析link不够 | 广泛`[`扫描是保守实现 | 保留潜在label查询/consumer记录；精度与体积需测 |
| 整个旧树销毁、重建新结果 | H0；新设计的rebuild | rebuild路径本身需要 | 分配布局影响常数 | 不可只比较parse bytes；新full path须计相同目标状态的建造/释放 |
| 先做增量，再发现必须全量 | H1/H4 | 某些长程变化事前未知 | 过晚gate、无budget会放大成本 | 前置已知不利条件；其余bounded attempt；不能保证零浪费 |
| 内存分配/branch/cache损失 | 多个horse | 不是Markdown语义 | 表示和机器相互作用 | 目前只部分定位到clone/drop；无足够证据给出精确成本/根因比例 |

## E. Weakness Map：最小解释集合

1. **局部语法工作配上全局维护。** H1的suffix节点坐标；H4的slot/checkpoint与defs traversal；H2/H3的候选与assembly。
2. **复用发现的成本没有被约束。** 特别是Q次consult乘M个候选的反复枚举；复用率或source inspection不能反映这种成本。
3. **安全边界过粗或位置映射退化。** top-level容器单位、原始空白margin、H3删除后坐标残差，使潜在可复用内容不能被接受。
4. **语法有效性与语义环境有效性耦合过强。** 全局定义变化引发重启/全体ref拒绝；缺少label级实际依赖和未解析引用登记。
5. **没有经济停止条件。** 小输出、高重建比例、已知全量路径仍先付增量准备与尝试成本。

不需要引入“Markdown天生不能增量”或“某语言/库不够快”来解释这些现象。

## F. Design synthesis：MODIFY

建议的最小研究原型：

> **一个可按位置查找、按区间拼接的源位置相对语法表示；在安全块边界restart并前向收敛；语法重建和label依赖修复分开；用同一目标表示的full builder作为正常候选与有预算的退路。**

### F1. 对原候选逐项裁决

| 候选项 | 裁决 | 修改理由 |
|---|---|---|
| persistent block sequence | MODIFY | 证据要求局部split/splice与不遍历全部suffix，不要求保留任意历史版本。单当前版本可用可变平衡chunk sequence；需要共享旧快照时才加persistence/COW |
| sparse/persistent checkpoints | MODIFY | 先把安全边界上下文嵌入块/chunk索引，去掉独立平行Vec。任意稀疏间距尚未获证；稀疏化增加restart距离，必须显式计入 |
| restart-and-converge | ACCEPT作为主语法机制 | 最直接避免前缀parser工作与收敛后的suffix工作；但必须保留正确parser continuation条件 |
| selective semantic repair | ACCEPT语义分层；索引实现为待测候选 | H2证明可分离，C-F同时暴露粗失效和全局发现；还未测出真正O(affected consumers)路径 |
| cost-based full rebuild | ACCEPT，增加online budget | 不能准确预知所有D/F；已知条件预选，未知条件小预算尝试；不增量到底 |
| universal fragment index / global old-tree reuse search | REJECT作为默认状态 | 当前工作负载没有证明其额外搜索/状态是最小必要机制；保留旧语法树不等于保留另一套全局复用索引 |

### F2. Persistent sequence究竟获得了多少授权

**已获证的需求：** stable local spans、O(log M)附近的定位、保留未改动子树、无需逐条suffix rebase，以及局部checkpoint维护。

**仍未获证的选择：** persistent B-tree vs mutable chunk tree vs其他平衡interval sequence；chunk大小、Arc策略、checkpoint密度、同等状态下的construction成本与内存。

不能只把`Vec<BlockSlot>`换成树：如果仍全树`collect_defs_skel`、重建所有checkpoint、逐节点计数或完整导出到绝对坐标树放进critical path，局部性仍不存在。

简化实现首先允许一个top-level有序sequence；大container内部的递归局部更新是后续可独立加入的能力，不预先建立所有深度的通用subtree索引。当前K证据足以指出损失，尚不足以选定其解决机制。

### F3. 最小semantic dependency状态

不是物理definition-node→已解析link，而是：

```text
normalized label
    → 按文档顺序排列的 definition occurrences（决定first winner）
    → 可能受该label影响的 consumer block IDs
```

consumer记录必须包含当前未解析成功、但定义出现后会生效的引用尝试。块内需保留足够内容区间，以便重新运行该块的inline逻辑。用block而非每个link作为最初修复粒度，可共享一次inline扫描；只有证明use-site细粒度有价值才细分。

新增/删除/修改definition、fence使definition出现/消失，都先得到确切的有效winner差异。若删除一个shadowed duplicate没有改变有效destination，则不应修复所有消费者。删除首定义时必须知道下一候选；只存当前winner不足以避免全文查找。

无需通用依赖DAG、传播调度器或永久全局generation失效。存source order与label lookup即可覆盖本次reference语义。为精确删除失效postings，可把反向edge handle/消费label集放在其owner block中；不是再存一份独立语义图。

索引不是保证正确性的唯一方法——扫描全文也正确。它是欲获得低fanout更新时间时要支付的额外状态；若构建/维护/内存成本无法回收，应删掉该快速路径，选择全量修复。

## G. State budget

| 持久状态 | 解决的失败/解锁的区间 | 可派生部分 | 是否可以删除 |
|---|---|---|---|
| 权威source buffer及位置度量 | lossless Markdown真值；保持edit/语法坐标一致 | 具体piece/rope由编辑器source层决定；本campaign没比较其成本 | 真值不可删；不要在parser再拥有独立全文副本 |
| 单一有序块/子树sequence，含gap/trivia长度、聚合byte length | 避免suffix绝对坐标重写、全表damage寻找；大N局部编辑 | 绝对位置从prefix sums/路径派生 | 可退回Vec但放弃该局部性；无需先承诺历史持久化 |
| 每块必要语法与semantic payload、块内相对span/inline内容区间 | 保留AST语义，支持仅重做受影响inline | 公共export绝对span按需派生；不保留第二个normalized整树副本 | retained结果不可删；双份Skel+semantic树应尽量合并 |
| 安全边界的restart/convergence上下文 | 防止paragraph/container/fence错误拼接 | 安全top-level边界的空context可隐含；边界位置由sequence派生 | 安全证明不可删；独立`Vec<Checkpoint>`可删 |
| label→有序definition occurrences及当前winner | 定义winner切换；避免每次全树收集 | winner从每label有序候选首项派生/缓存 | selective路径需要；否则接受全扫描/全量策略 |
| label→consumer block集合，含unresolved；块拥有edge删除信息 | F小而N大时避免全文语义扫描 | 更细的use-site表可不存；consumer去重到块 | **条件性状态，必须由下一实验赚取**；无需通用graph |
|少量root/chunk统计及有界成本参数|无需O(N)扫描的selector；region字节/节点/引用规模估计|统计在本地split/splice时维护；无逐case历史库|复杂模型可删；保留简单预算也可工作 |

**瞬时而非持久状态：** edit delta/mapping、damage描述、forward parser stack、新middle、changed labels、待修复consumer集合、工作预算与临时分配。完成后释放。

**不加入：** universal fragment table、H3式全局old candidate index、全局changed flags永久层、全suffix绝对offset缓存、与block重复维护的checkpoint向量、每次更新都重算的全局语义generation、为selector保存大规模编辑历史。

计数只按本机制自己的语义比较；H0的retained_blocks含root/native nodes，H1为tiling entries，H2/H3计数还含inline节点，H4是top-level slots，不能把state_repr数字横向相加来决定内存最优。

## H. Concrete update algorithm

```text
BUILD(source):
    用同一Markdown语义核心全量扫描block结构
    建立definition occurrences并确定每个label的winner
    解析inline，同时登记成功/未成功的reference查询所关联consumer
    以bulk builder建立单一sequence及相对span、边界上下文、聚合统计
    seal为可用状态；不留下必须在读取时补做的语义修复

PREPARE_EDIT(old, edit):
    source层验证/应用edit；保存旧→新位置映射
    用sequence定位覆盖区间及邻接边界
    读取必要的continuation margin，向前选择安全restart
    读取局部语法种类、覆盖大小、已知definition/consumer摘要
    不扫描全文；damage classifier是保守分类，不能用词法probe代替证明

PRESELECT:
    若已知成本表明full builder更便宜，直接FULL_BUILD(post)
    否则建立临时middle和尝试预算

RESTART_AND_PARSE:
    从可恢复状态的安全边界开始forward parse
    在旧边界映射到新坐标的位置检查：
        后续source确实未编辑
        已越过全部syntax damage
        parser continuation/context等价，不能遗漏open paragraph/container/fence
        边界本身合法
    匹配则停止，保留suffix；不匹配则继续
    语义环境的变化单独记录，不用全局generation自动否决全部syntax reuse
    若到EOF仍未匹配，当前middle已经包含必要的forward结果
    若工作预算耗尽/估计剩余增量工作更贵，则转FULL_BUILD(post)

SPLICE:
    split(old, restart)
    split(old, accepted convergence or EOF)
    concat(prefix, new_middle, suffix)
    更新边界路径与长度聚合；不遍历suffix逐条平移位置/重建checkpoints

SEMANTIC_REPAIR:
    从removed/inserted syntax内的definition facts更新对应label候选
    对比有效winner，只得到真正变化的labels
    找出这些labels的consumer blocks（包含prefix中的远端使用者）
    剔除已被语法重建/删除的consumer，按块去重
    对保留syntax的受影响块重做inline；更新该块的依赖登记
    new-middle的inline也必须在最终definition环境下完成
    若此时选择全量更便宜，使用相同target-state的FULL_BUILD

COMPLETE:
    修复必须完成后再发布新root和semantic状态
    释放被淘汰状态及临时索引；释放成本属于实现成本
    oracle/benchmark checksum放在测量外；真实API不得偷偷把必要语义推到读取时
```

说明：步骤文字中的splice可先形成tentative root，直到语义修复完成才发布，不表示允许用户读取语义陈旧状态。若只有一个当前版本，可用一次受控mutation而无需保留整套历史快照；算法正确性仍要求在旧状态被破坏前保留足够fallback信息/source真值。

预判不是预言。Fence打开后的最终D、definition是否被结构隐藏等，可能只有forward parse后才知道。selector必须同时具备事前选择和有限探索预算。

在online决策点，比较 **remaining incremental work** 与 **从当前状态转full builder的成本**；已发生的探索是sunk cost。另设总尝试上限限制后悔成本。不要仅因“已花很多”继续，也不要用spent+remaining重复触发不理性的选择。未校准前不能声称有严格competitive bound。

## I. Complexity and cost selection

仅用N/B/D/F/K不足以描述目前的失败：必须显式加入metadata与输出规模。

- N：source bytes。
- M：可寻址block/sequence records。
- R：restart点至edit的回溯/重走字节数。
- B：直接受损语法区域；D：越过该区域后直到真正convergence/EOF的额外距离。三者定义不重叠时forward bytes `W=R+B+D`；若D定义为整个forward距离，就不能再加B。
- K：需恢复/比较的container depth。
- Q：convergence/reuse候选次数。
- A：新建/删除/改变的结构记录数。
- L：label数，E：需要插入/删除的definition occurrences。
- F_b：受有效winner变化影响的distinct consumer blocks；S_F：这些块实际重做的inline bytes；I_F：它们产生的inline结果规模；U：需更新的dependency edges。

### I1. 当前算法中的额外项

H1包含全tiling定位`O(M)`和suffix表示重建`O(nodes_suffix)`。H2/H3在当前find_run实现下包含约`O(Q·M)`的候选列表构造/搜索，加上assembly；低Q时仍可能胜过full parse。H4在局部parse以外仍包含`O(M)`的slots/checkpoints和可能的全树definition遍历。

这比“增量是O(B)”更能解释观测。

### I2. 新设计能有条件承诺的项

假设选择了**真正支持按权重split/concat的平衡chunk tree**，并满足：相对坐标、不flat-copy全部leaf、不全树重建defs/checkpoints，且旧状态回收只处理失去最后引用的changed子树：

```text
T_inc = T_source_edit
      + T_locate/restart(log M, K, local margin)
      + P_block(W, K)
      + T_convergence(Q, K, index traversal)
      + T_splice(log M + changed/boundary chunks)
      + T_definition_delta(E, L)
      + T_consumer_lookup(changed labels, F_b)
      + P_inline(S_F, K) + output construction(I_F)
      + T_dependency_maintenance(U)
      + T_retirement(changed/removed state)
```

若每候选做一次树查询，convergence项是`O(Q(log M+K))`；若实现单调cursor，可避免重复根查找，但必须由具体数据结构证明，不能仅因叫rope就写成O(Q)。块内容解析P_block/P_inline也不能无证明直接写O(bytes)：当前shared grammar的某些扫描、嵌套inline与definition lookup有自己的复杂度。

label查找若用平衡树为O(log L)，若hash为期望O(1)。每个consumer独立更新sequence会产生O(F_b log M)路径成本；批量有序更新可优化，但本报告不预支该收益。

Full path应建造**相同未来可更新表示**：

```text
T_full = P_block(N,K) + P_inline(all relevant content,K)
       + full output/index construction + old-state retirement
```

它不等于现有H0的计时常数，因为现有H0不维护新设计的sequence/dependency状态。不能拿H0现有数值当成“切过去就能免费获得”的产品性能。

### I3. O(F)需要怎样限定

纯粹写`O(F)`不成立：F_b个块长短不同，一个新definition可能影响多个未解析尝试；inline树形状可能改变。合理目标是 **与affected consumer blocks及其实际内容/输出工作成比例，加上局部索引成本**，而非与全文N成比例。

对必须立即完成的eager结果，真正变化的输出需要产生/替换；若winner没有变化，F很大也可以不修复。不能把“所有潜在F”当作语义下界。

### I4. 便宜的selector

事前可用：root的N/M/节点类别统计；edit位置、长度、被覆盖block大小；安全restart位置；大container/fence特征；直接改变label的posting计数/内容权重。来源应是已有树聚合和本地更新，不是每次预测先扫O(N)。

无法便宜精确知道：实际D、隐含definition变化、最终inline输出。对此使用保守估计与预算。比较项包括：

`restart/search + affected parse + convergence + splice/retirement + semantic repair`

与相同target-state的full builder。F门槛由当前consumer字节/节点量、label索引开销、N及full-build估计共同决定；**不硬编码本次F=2/4交叉点**。selector本身应是常数/对数加本地输入成本；复杂预测模型只有测到净收益才有资格存在。

## J. Smallest discriminating experiment

**只做一个6-cell的mechanism ablation，不再做大campaign，也不重采Campaign-2。**

固定现有grammar、oracle、source/edit和计时边界；使用四个variant：

1. V0：当前H4，提供已知metadata/global-semantic路径。
2. V1：只替换为可局部寻址/拼接的sequence和局部definition事实维护；保留现有语义全量重启政策，保持checkpoint安全粒度，不同时做nested reuse优化。
3. V2：V1加label级consumer repair；这是唯一新增semantic dependency变量。
4. VF：直接full build到与V2相同的ready state；是selector应比较的真实退路成本。

六个计时cell：

| witness | cells | 要区分的假设 |
|---|---|---|
| 固定B=128 B、相同中部8 B编辑，无ref/fence | N=128 KiB、1 MiB、16 MiB | V1能否真正去掉O(M)damage/suffix/checkpoint/defs工作，而非只把Vec改名 |
| 固定128 KiB、删除同一32 B definition | C-F slots=0、4、64 | V2是否避免全局`[`扫描与ref块重parse；新索引维护是否比VF划算 |

每个cell仍按session独立计算p50，固定预先选定的重复数；不用profiling代替主时间，不临时增删点。工作计数单独测，新增能直接判别的计数：真正访问/clone/重写的sequence records、suffix records、checkpoint records、candidate枚举成员数、semantic probe bytes、fresh block/inline bytes、dependency edges变更、实际分配/释放的chunks与bytes。需要区分函数调用计数和该调用内部访问M个对象的成本。

正确性fixture只补几种必要边界：unresolved→resolved、删除first definition后duplicate接替、shadowed duplicate删除不改winner、fence隐藏/恢复definition、paragraph/container接缝、CJK字节坐标。它们是正确性门，不扩展为新性能矩阵。

预先写明可推翻结果：

- 若V1仍访问与M线性增长的未变suffix/defs/checkpoints，representation设计失败，不因时间偶然下降就接受。
- 若访问量已局部但16 MiB时间仍显著随N长，转查回收/分配/source或导出边界；不能继续声称“metadata问题已解释完”。
- 若V2在F=0仍扫描整篇source，dependency discovery未解决。
- 若V2保持syntax但实际semantic工作仍广泛超出changed labels的consumer集合，依赖记录不够精确。
- 若V2的构建/常驻内存/无ref局部路径开销吞掉其目标regime收益，删除或延后该索引。
- 若V2在F=64败给VF，这可以是正常选择边界，不自动判定索引失败；必须同时存在可服务的低F区间，且selector不靠O(N)预测。

这个实验首先给两个新增状态组件一个可被否定的理由：**sequence是否消掉全局维护；label consumer index是否消掉全局语义发现。** 不以“打败H4多少倍”作为唯一成功条件。

## Evidence references

以下路径均相对于 `research/benchmarks/markdown-ast-update/`；实现引用固定到审查head。

- [PR #47 metadata](https://github.com/jnhu76/markit/pull/47)
- [Freeze](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/CAMPAIGN-2-FREEZE.md)
- [Evidence gaps](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/EVIDENCE-GAPS.md)
- [Closure](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/CAMPAIGN-2-CLOSURE.md)
- [Handoff](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/GPT6-FULL-EVIDENCE-HANDOFF.md)
- [Summarizer](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/tools/summarize.py)
- [Timing summaries](https://github.com/jnhu76/markit/tree/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/summary)
- [H0 implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/mechanisms/full-rebuild/src/lib.rs)
- [H1 implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/mechanisms/block-local/src/lib.rs)
- [H2 implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/mechanisms/fragment-reuse/src/lib.rs)
- [H3 implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/mechanisms/old-tree-subtree-reuse/src/lib.rs)
- [H4 implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/mechanisms/restart-convergence/src/lib.rs)
- [Controlled generators](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/campaign2/src/generators.rs)
- [Replay scope](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/campaign2/src/bin/mdbench-replay/main.rs)
- [PMU implementation](https://github.com/jnhu76/markit/blob/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/campaign2/src/bin/mdbench-replay/perfcount.rs)
- [Profiling artifacts](https://github.com/jnhu76/markit/tree/819928966e05ca00bb82c58b887014e54f3d0cd5/research/benchmarks/markdown-ast-update/results/campaign-2/profiling)
- [Raw supplement](https://github.com/jnhu76/markit/blob/research/31-campaign-2-review-inputs/markit-c2-review-inputs.tar.gz)
- [Executable-derivation supplement](https://github.com/jnhu76/markit/blob/research/31-campaign-2-review-inputs/markit-c2-executable-derivation.tar.gz)

本报告中的derived family统计以schedule的case_id→payload_id与冻结edit manifest连接；数字work counters来自补充包的raw JSONL，不来自压成KNOWN的attribution CSV。没有以全局平均值替代regime解释。

---

## Later evidence（#50/PR #51 之后补充）

本节**不修改上文任何结论**；它记录后来的证据如何确认、细化或改写了本文的哪些陈述。
本文正文保持 Campaign-2 当时（2026-09-23，PR #47）的解读。

新增的证据权威：

```text
#50 H4-LARGE-N-CAUSE-1  ->  PR #51 merge 6cec47e9bb756affb0a4477bcb4962c3c78d8901
    research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/
    U_PLAIN（原始 frozen H4，`run-plain`）86.3 us (128 KiB) -> 24.99 ms (16 MiB)
    （这是与 Campaign-2 不同的一次采集；绝对值不同，regime 相同。）
```

### 被确认的陈述

| 本文原陈述 | #50 的结果 |
|---|---|
| §C 表末行：“16 MiB 相对 1 MiB 的额外非线性退化 … 假设；不能指定为某种 cache miss 原因” | algorithmic work 侧被确定：**工作线性**、指令数线性；非线性是 memory hierarchy 的放大（`CACHE_CAPACITY_AMPLIFICATION = STRONGLY_SUPPORTED`，作为与 2–4 MiB regime transition 的关联）。`PRECISE_STALL_DECOMPOSITION` 仍为 **UNRESOLVED** —— 本文当时的谨慎判断在这一点上没有被推翻。 |
| §C4：“存在不可忽略的全局表示维护税；不能收窄成只有 suffix relocation” | **确认并量化**。H4 每次局部编辑执行多条 O(M) retained-state 流：P6 26.6%、P5 21.5%、P3 20.2%、P7 12.4%、P1 11.6%、P4 8.6%（share of U_PHASE at 16 MiB）。冻结 §14 规则下 **DOMINANT = none**：没有任何单条流独占主导。 |
| §I1：“H4 在局部 parse 以外仍包含 `O(M)` slots/checkpoints 和可能的全树 definition 遍历” | 两者都确认：P6 `slots_created = checkpoint_records_created = checkpoint_key_clones = M`；P4 `definition_nodes_visited = M` 而定义表为空（Adefs -8.7%）。 |
| §C 表：“H4 C-N 保持 forward=136 B，但时间随 N 涨” | 确认：`blocks_reparsed = 1`、`nodes_rebuilt = 2`、source inspection 283 total / 138 unique bytes 在 8/8 cells 恒定，而 `damage_records_visited = M`、`nodes_reused = 2M - 2`。 |
| §E Weakness Map 第 1 条：“局部语法工作配上全局维护” | 由 #50 的 phase/counter/ablation 证据具体化；H4 行补上 “parser locality != representation locality”。 |
| §B2 的 H4 大 N 优势回落到 3.695× | regime 复现（不同采集、不同绝对值）；本文的“不具备大小无关的局部更新时间”结论保持。 |

### 被改写或推进的陈述

- **“H4 16 MiB 额外退化的唯一主因判定”**（本文 §A2 列为仍阻断项之一）：
  #50 的结论是**不存在单一主因**。可回答的部分已经回答（多条 O(M) 流共同支配、
  其中三条合计 68.3% of U_PHASE）；不可回答的部分（精确 stall 分解）明确保留为
  UNRESOLVED，并且不阻塞 V1。
- **当时缺失的匹配数据现已存在**：本文 §C 表与 §C4 指出缺少 C-N 大点的匹配
  perf-record 与 allocation 计量。#50 补齐了 allocator lane、窗口化的 perf stat、
  perf record（1 MiB / 16 MiB）与一个 scope 受限的 eBPF/kernel lane
  （明细与 scope 限制见 `h4-large-n-cause-1/H4-LARGE-N-CAUSE-REPORT.md` §4–§5）。
- **§J 的六 cell 消融**：本文把它当作“最小判别实验”提出。#50 之后它被**推迟**，
  不是被删除：它的形状（少量 cell、预先写明可推翻结果、对照 VF 全量退路）仍然有效，
  但它的 V1/V2 variant **预设了具体的表示组件**，因此在新候选机制的机制身份被显式
  设计出来之前不得启动。当前顺序是先确立 R1-R6、再设计候选机制身份，然后才谈
  判别实验；§J 的 V1/V2 在新分级下属于 C（candidate design choice），不属于已获证据。
  见 [markit-31-research-synthesis.md](markit-31-research-synthesis.md) §8 与
  [markit-ast-update-design-v0.md](markit-ast-update-design-v0.md) §0.4 / §18。
- **§F1 的逐项裁决**（persistent block sequence = MODIFY、restart-and-converge =
  ACCEPT、cost-based full rebuild = ACCEPT 等）与 **§F2** 的“已获证需求 /
  仍未获证选择”划分，与 #50 的结论一致，并被提升为行为要求 R1-R6
  （综合文档 §5）。具体数据结构仍然 **UNDECIDED**。

### 未被改变的陈述

- 本文的全部 Campaign-2 数值、regime map、源码级解释与限制说明。
- §G 的 state budget 与 §F3 的最小 semantic dependency 状态形状。
- §H 的算法骨架与 §I 的复杂度项清单（R1-R6 与之一致）。
- 本文 §A2 中除“H4 唯一主因”之外的所有降级项。
