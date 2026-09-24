### E16：fence 暴露 definition

```text
old = K · G · ⟦"```\n[x]: /u\n```\n\n"⟧Fence · Z · E
new = K · G · ⟦"[x]: /u\n\n"⟧Def · Z · E
edit = replace complete [9,26) with "[x]: /u\n\n"
```

coverage：旧 Fence `[9,26)`；新 Def `[9,18)`。r=6；旧 q26 support 被替换，向前到 Z 后 q=32→24。O=`G,Fence,Z`，N=`G,Def,Z`；facts `[]→[(x,/u)]`，full。definition 的文字先前作为 code body，不能从“文本是否修改”推导 facts 是否不变。反向将该 definition 放进 fence，facts 由一项变空，也 full。最终无 retained ranges，退休全部旧 representation。

### E17：definition spelling 改变，规范事实相等

```text
old = K · G · ⟦"[X]: /u\n\n"⟧Def · Z · E
new = K · G · ⟦"[x]: /u\n\n"⟧Def · Z · E
```

coverage 都是 Def `[9,18)`。r=6；q=18→18。O=`G,Def`，N=`G,Def'`；shared ASCII normalization 得 `[(x,/u)]==[(x,/u)]`，incremental。保留 K、Z/E 和原表；退休 G/Def 的旧 payload。新的 Definition AST/source association 仍必须正确，不能因 facts 相等保留被编辑的 Owner。

### E18：leading trivia 与 indentation

```text
old = ⟦"\n  a\n\n"⟧P [0,6) · ⟦"b\n"⟧P [6,8)
new = ⟦" \n  a\n\n"⟧P [0,7) · ⟦"b\n"⟧P [7,9)
edit = insert SPACE at 0
```

r=BOF；q=6→7，最后 blank/LF support 未被编辑。O=首 Owner，N=新首 Owner；facts 空，incremental。无 prefix，保留 b suffix，退休旧首 Owner。首块 semantic span `[3,4)→[4,5)`，coverage 起点始终 0；不能把行首缩进误当 span 起点。

### E19：首块消失，只剩 leading trivia——coverage 反例

```text
old = ⟦"\na\n\n"⟧A [0,4) · ⟦"b\n\n"⟧B [4,7) · ⟦"c\n"⟧C [7,9)
new = ⟦"\n\n\nb\n\n"⟧B [0,6) · ⟦"c\n"⟧C [6,8)
edit = delete only "a" at [1,2)
```

r=0。旧 q4 的直接 blank/LF support 能映射到新 q3，new live state 也为空 root；但新 `[0,3)` 只有非空 trivia，按唯一 coverage 规则必须归之后的 b Owner。**因此 q4→3 因 coverage cut 不成立而拒绝。**

继续到 q7→6；O=`A,B`，N=`B-with-leading-trivia`；facts 空，incremental。无 prefix，保留 C；退休旧 A/B。b 的 payload 必须重建/重新相对化，不能把旧 B 作为 untouched suffix 原样挂上去。这个例子验证为何语法收敛不等于可立即 ownership splice。

### E20：trailing trivia

```text
old = ⟦"a\n\n"⟧P [0,3)
new = ⟦"a\n\n  "⟧P [0,5)
edit = append two SPACES at EOF
```

t=3；old EOF 不作 restart cert，r=0；EOF 3→5。O=P，N=P'；facts 空，incremental-to-EOF。没有 retained prefix/suffix；退休旧 P。semantic span 仍 `[0,1)`，新增 trailing bytes 只扩大 coverage。查询 trailing gap 返回 `[Document]`。

### E21：empty 与 all-whitespace

```text
(a) old = empty OwnerSeq, source ""
    new = ⟦" \n"⟧TriviaOnly [0,2)
(b) old = ⟦" \n  "⟧TriviaOnly [0,4)
    new = ⟦" \na\n"⟧Paragraph [0,4)     // replace [2,4) by "a\n"
(c) old = ⟦" \n  "⟧TriviaOnly [0,4)
    new = empty OwnerSeq               // delete [0,4)
```

三者均 r=BOF、到真实 EOF（0→2、4→4、4→0），Defs `[]==[]`，incremental EOF completion。(a) O=空、N=TriviaOnly，没有旧 Owner 退休；(b) 退休唯一 TriviaOnly，创建完整 paragraph，semantic span `[2,3)`；(c) 退休唯一 TriviaOnly，没有新 Owner。无保留 ranges。Document root 分别 `[0,2)`、`[0,4)`、`[0,0)`。`"\t\n"` 则是普通 paragraph，不能使用 TriviaOnly。

### E22：EOF 插入接续最后一行

```text
old = K · G · ⟦"a"⟧P [9,10)
new = K · G · ⟦"ab"⟧P [9,11)
edit = insert "b" at 10
```

t=L=10；strict interior predecessor r=9。注意这里只须重做最后 P，因为 EOF anchor 的左侧已经包含它；old EOF 本身不是 restart。q=EOF 10→11；O=P，N=P'；facts 空，incremental。保留 K/G，无 suffix；退休旧 P。不能把 append 单独建一个 paragraph Owner，否则与 full parse 不等价。

### E23：UTF-8 编辑边界

```text
old = K · G · ⟦"中a\n\n"⟧P [9,15) · Z · E
new = K · G · ⟦"中😀a\n\n"⟧P [9,19) · Z · E
edit = insert emoji at 12              // after the 3-byte 中
```

r=6；q=15→19；O=`G,P`，N=`G,P'`；facts 空，incremental。保留 K、Z/E，退休旧 G/P；suffix bases +4，不是 +1。若请求删除 `[10,11)`，它切进“中”的 UTF-8 编码，validation 直接失败：不选 restart、不解析、不比较 facts、不提交；old 的完整 coverage/Owners/table 保留，没有旧状态退休，只释放已产生的输入临时物。

### E24：zero-length / block-start / block-end affinity

block-start：从 `K·G·⟦"a\n\n"⟧P·Z·E` 在 9 插入 `"b\n\n"`。RIGHT 命中旧 P；r=6。新 coverage 为 `K·G·⟦"b\n\n"⟧Pnew·⟦"a\n\n"⟧P·Z·E`；candidate q=12→15。O=`G,P`，N=`G,Pnew,P'`；facts 空，incremental；保留 K、Z/E，退休旧 G/P。新增字节实际形成一个新 Owner，而非强行附属 RIGHT 定位对象。

block semantic-end：从 `K·G·⟦"ab\n\n"⟧P·Z·E` 在 11（LF 前）插入 `"!"`。RIGHT 仍命中 P，不是 Z；新 P coverage `[9,14)`。r=6。因旧 blank certificate 支持包含被相触的 preceding LF，本文保守跳过 q13，继续到 q19→20。O=`G,P,Z`，N=`G,P',Z'`；facts 空，incremental；保留 K、E，退休旧 G/P/Z。这进一步区分 semantic end、Owner end 与可用 certificate。

### E25：没有 blank 的 paragraph continuation

```text
old = ⟦"a\nb\n"⟧Paragraph [0,4)
new = ⟦"a\n"⟧Paragraph [0,2) · ⟦"# b\n"⟧Heading [2,6)
edit = insert "# " at 2
```

r=0；旧 offset2 虽然 ContextKey 可为空，却不是 Owner boundary，且 paragraph 打开。新 offset2 成为 Owner cut，但无 root blank certificate。只能 EOF 4→6。O=旧 Paragraph，N=Paragraph+Heading；facts 空，incremental-to-EOF；无保留范围，退休整个旧 payload。到 EOF 完成整个文档不等于语义失败后的重复 full rebuild。

这些 traces 揭示的三项必要修正是：**coverage closure 独立于 syntax convergence；证书迁移独立于 raw blank bytes 是否存在；semantic admissibility 独立于 syntax P/S 是否可保留。** 三者缺一都能构造错误结果。

## 16. 复杂度与成本边界

固定记号：

| 符号 | 本文含义 |
|---|---|
| M | 当前 retained sequence records，包括全空白特例的 TriviaOnly |
| H | sequence height；局部公式取有关旧/新树高度的最大值，AVL 为 O(log(M+Δ+1)) |
| Δ | 实际 created/removed/seam records 总数；不能拿它替代 parser bytes 或 payload 大小 |
| R | restart 到 convergence/EOF 的实际共享 block-parser 工作，包括保护块、传播、prefix/line inspection |
| C | convergence discovery/checking 工作，不重复包含 R 的正常 block parsing |
| Q | 实际检查的 convergence candidates；无全局 candidate Vec |
| D | document definition facts / lookup environment 的规模；精确字符串成本另按实际字节计算 |
| SΔ | 实际 fresh/removed semantic payload 工作，分为 S_new 与 S_removed；不是 Owner 数 |

为避免隐藏实际成本，另用 `B_O` 表示旧 replacement 中被访问的 block topology 节点数，`DΔ_bytes` 表示 local fact stream/比较实际处理的 label/destination 字节，`I_new` 表示新 inline/values 的构造工作，`L_lookup` 表示全部 fresh reference 查询的实际工作，`V_i` 表示 query 在已定位 Owner 内访问的节点数，`S_all` 表示全量输出/退役 payload 工作。这些都是测量/分析量，不自动成为持久字段。

| 操作 | 结构/查找成本 | parser / semantic / source 成本与边界 |
|---|---|---|
| validate / locate | 常数次 O(H) locate；identity/range arithmetic 有界 | 依赖共同可信 edit association；不再全 source hash。输入 UTF-8 边界检查按 source API |
| exact boundary | O(H) | 不解析 Markdown |
| safe predecessor | O(H) | 不向后线性查找 blank；所选点距 edit 很远的代价在 R |
| block parse | 与 M 无直接界 | R；巨大 container/长行/传播允许很大，不能写成 O(Δ) |
| convergence | cursor 版本 `C=O(H+Δ_old+Q)`，另加确实发生的 support/mapping 检查 | 新 live evidence 随 R 产生；一个 hook 没运行不是一次成功检查；root-seek 版本须改报 O(QH) |
| fact extraction | sequence 范围入口 O(H)；旧 O 顺序 block traversal O(B_O) | 新 facts 已由 shared block pass 发出；流化/取值 O(DΔ_bytes)。若实现再次遍历新 block，要显式加 B_N |
| fact comparison | 无 P/S 或全表遍历 | exact ordered comparison O(DΔ_bytes) 最坏；可以提前发现不等，但不能据此跳过已要求的完整 region parse |
| semantic materialization | 不访问 retained P/S payload | `S_new = I_new + L_lookup + fresh relative-span/value conversion`；现有线性 resolve 最坏每次扫描 D 项，label 比较字节不能省略 |
| fresh sequence bulk build | O(Δ_new) | 只消费 fresh Owners，不扫描 retained ranges |
| splice / split / join | O(H + H_fresh)，含常数次 split/join | 不解析 payload；不逐 retained record reinsert |
| aggregate repair | O(H + Δ_new)，其中 fresh 节点已可计入 bulk build | 每节点 O(1) combine；不从后代重新计算 node counts |
| certificate creation/repair | O(Δ_new + 实际 seam 数) 的 attach/metadata；path repair O(H) | 证据观察已在 R/C；不重发全部 suffix 证书；实际 support 读取必须计费 |
| retirement | O(Δ_old + destroyed transient records) 的结构销毁 | S_removed + 实际临时 Skel/facts/values/workspace 清理；preserved 表不重建/释放 |
| same-target full build | OwnerSeq bulk O(M_new) | `R_full + Defs_full + S_full + certificate construction`；old 全量 retirement 另加，放弃 attempt 另加 |
| NODE_PATH_AT | O(H) owner locate | O(V_i + path length)，无 owner 内索引时最坏扫描整个 owner payload |
| whole export | 顺序 O(M)，不是 O(MH) | 加所有节点/span/value 的输出工作；不得 parse/repair |
| document destruction | O(M) 结构 | O(S_all + table/owned resources 的实际销毁工作)；不是 R5 禁止的 incremental maintenance |

表中 bulk build、aggregate repair、parser observation 与 certificate creation 有联合执行部分；总时间不能把同一工作重复相加。归因时为实际事件归类，理论总界可写为：

```text
U_incremental =
    O(H + Δ)
  + R + C
  + extraction(O,N) + exact_fact_comparison
  + S_new + S_removed
  + actually incurred transient/resource management

S_new contains actual RefTable lookup cost, not just changed bytes.
```

固定短段落、无定义、bounded guard/restart/propagation witness 中，Δ/R/Q/SΔ 都可有与 M 无关的边界，此时模型预期不存在 Θ(M) 的 retained maintenance。**这是可被否证的机制预测，不是已经获得的实验结果。**

memory：常驻 overhead 为 O(M) 的 sequence/coverage/certificate/count metadata，加完整 AST semantic values、D 的环境值和当前 source association。局部 staging 峰值包括完整 old、post source、fresh block skeleton、fresh payload/sequence、local facts 和 commit/drain workspace；full path 还同时具有 old+完整 new。不能只报告 commit 后 retained bytes，不能把预留 drain workspace 或已放弃 attempt 排除。

## 17. 剩余 gate、不能隐含的实现选择

本文没有留下必须通过 winner index、nested checkpoint、COW 或 packed layout 才能解决的逻辑义务。A-01—A-05 的 closure 是：

| 义务 | 本文闭合内容 | 对应章节 |
|---|---|---|
| A-01 | 唯一 partition/affinity；guard 和 leading-trivia coverage closure；EOF/empty 特例 | §§3、6、E19/E21/E24 |
| A-02 | pre-EOF root-empty predicate；source support 与 parse provenance；迁移归纳；真实 convergence | §§8–9、E05–E10/E25 |
| A-03 | 完整 O/N facts；环境判定后 eager materialization；first-wins projection 单一真值 | §10、E11–E17 |
| A-04 | fallible staging、资源预备、唯一 frontier、无 fallback commit、按所有权 retirement | §§4、11–12、14 |
| A-05 | scoped R1–R6 与每个 enforcing operator；真实替换/传播/全建成本公开 | §§13、16 |

仍须完成的步骤不是新增设计机制：

1. 把本文候选及 §19 修订纳入 #55 的一个固定版本；live #55 不能只改一个 YES 而保留旧算法顺序。
2. 进行所要求的 fresh independent review，特别攻击 guard coverage lemma、certificate induction、EOF/empty exceptions 和 no-fail split/join/drain resource bounds。本文作者的 PASS 不是这轮独立审查本身。
3. 若独立审查通过，固定 mechanism identity、同一 logical target、chosen realization、restart/candidate policy、counter units 与 parser observation boundary。
4. 再单独冻结 falsification contract：输入/edit、oracle、失败模式、成本/峰值/retirement 边界、预注册阈值及结果到行动规则。本文没有代选这些数值、样本或阈值。
5. 只有以上 gate 通过后，才设计/实现具体 Rust lifetime、allocation/split/join/resource strategy 并验证本节 traces、连续 edits 和 oracle equivalence。测试是在实现授权后检验契约，不是本次预跑未来机制。

实现若发现共享 parser 的实际 callback 不足以取得 §8 evidence，必须补齐**同一共享语义的 observer/adapter**并审查其边界，或承认候选实现不符合 contract。不能将空 ContextKey 降格当作等价证据，也不能把缺少观察接口变成另写一个 Markdown parser 的理由。

在采用本方案前，需明确接受 guard policy 的可测成本和 support-touch 候选拒绝规则。它们已经在本文固定，而不是留给实现者按测量结果自由调整。与 A-R/B/A-P 的后续轴仍可能耦合，本文不承诺三轴收益可相加。

## 18. 最终设计裁决

下面是对**本文完整候选**的 author-side design assessment；不是对尚未修订的 #55 正文作无条件 PASS。P 计数是本次模型中仍未解决的设计缺陷数，不是运行结果，也不表示已经批准合并/冻结/实现。

```text
COVERAGE_CONTRACT = PASS
READYDOCUMENT_CONTRACT = PASS
OWNERSEQ_CONTRACT = PASS
OWNER_CONTRACT = PASS
ASTPAYLOAD_CONTRACT = PASS
REFTABLE_RELATION = PASS
RESTART_CERTIFICATE = PASS
CONVERGENCE_CONTRACT = PASS
STAGING_COMMIT_MODEL = PASS
FULL_BUILD_EQUIVALENCE = PASS
R1_R6_MAPPING = PASS

P0 = 0
P1 = 0
P2 = 0
P3 = 0

MECHANISM_IDENTITY_READY_TO_FREEZE = YES
READY_TO_FREEZE_FALSIFICATION_CONTRACT = YES
READY_FOR_IMPLEMENTATION = NO
```

两个 YES 的含义是：**逻辑候选已具体到可以交给独立审查并作冻结决定，且足以据此制定/审查 falsification contract。** 不表示已经冻结，亦不跳过 #55 明确要求的 fresh review。当前外部权威状态仍为 `DESIGN CANDIDATE / IMPLEMENTATION NOT AUTHORIZED`。

W-A1（coarse restart/atomic owner）、W-A2（保守 facts 证书）、W-A3（binary per-record layout）全部保留。后续若要缩短保护段/增加边界集合、改变 semantic decision 或换布局，必须明确记录机制版本；不能在获得有利/不利数据后悄悄修订身份。
