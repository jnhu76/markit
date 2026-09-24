## 14. Rust-like 逻辑 schemas（解释用途）

下列 `Owned<T>`、`SeqLink`、`SourceAccess`、`Scratch`、`OrderedChildren` 表示逻辑拥有关系，不是 Rust 标准类型，不决定 Box/arena/Rc/Arc/allocator/packed layout/public API/final lifetime。字段必须遵守前述契约；这里不提供可编译生产代码。

```rust
struct ReadyDocument {
    identity: InterpretationIdentity,
    source: SourceAssociation,
    owners: OwnerSeq,
    references: RefTable,
    // READY is a proven state, not a lazily checked boolean.
}

struct SourceAssociation {
    source_id: SourceId,
    revision: RevisionToken,       // whole-document association only
    byte_len: ByteLen,
    access: SourceAccess,          // current immutable substrate source
}

struct InterpretationIdentity {
    grammar_revision: GrammarRevision,
    normalized_result_revision: ResultRevision,
    parser_semantics_revision: ParserSemanticsRevision,
    options: SemanticOptionsIdentity,
    horse_model_revision: HorseModelRevision,
}

struct OwnerSeq {
    root: SeqLink,
}

struct SequenceNode {
    left: SeqLink,
    owner: Owned<Owner>,
    right: SeqLink,
    aggregate: Aggregate,
    height: Height,                // selected AVL realization
}

struct Aggregate {
    bytes: ByteLen,
    records: RecordCount,
    has_certified_safe_end: bool,
    payload_nodes: NodeCount,       // required local reuse accounting
}

struct Owner {
    coverage_len: NonZeroByteLen,
    body: OwnerBody,
    payload_nodes: NodeCount,       // cached when fresh payload completes
    end_certificate: Option<RestartCertificate>,
}

enum OwnerBody {
    Block(ASTPayload),             // exactly one complete top-level subtree
    TriviaOnly,                    // sole record iff L > 0 and no blocks
}

struct ASTPayload {
    root: SemanticNode,
}

struct RelativeSpan { start: ByteOffset, end: ByteOffset }

struct SemanticNode {
    span: RelativeSpan,            // same Owner base at every depth
    value: SemanticValue,
    children: OrderedChildren<SemanticNode>,
}

enum SemanticValue {
    Paragraph,
    Heading { level: HeadingLevel },
    BlockQuote,
    List,
    ListItem { marker: Marker },
    FencedCode { info: OwnedText, content: RelativeSpan },
    Text,
    Emphasis,
    CodeSpan,
    Link { destination: OwnedText },
    ReferenceLink { label: OwnedText, destination: OwnedText },
    ReferenceDefinition { label: OwnedText, destination: OwnedText },
}
// Document is synthesized once from ReadyDocument, never an Owner payload.

struct DefinitionFact {
    normalized_label: OwnedText,
    destination: OwnedText,
}

struct RefTable {
    entries: OrderedFacts<DefinitionFact>, // duplicates retained, first-match lookup
    // No owner pointers, source offsets, mutable winner cache or postings.
}

struct RestartCertificate {
    rule: RootBlankRuleRevision,
    support: RelativeSupport,      // relative to the LEFT Owner
    evidence: RootEmptyEvidence,   // only parser-semantic issuer may create
    // Position = Owner base + coverage_len; no stored absolute position.
    // No per-document generation requiring suffix-wide renewal.
}

struct RelativeSupport {
    blank_line: RelativeSpan,      // includes its terminating LF; ends at owner end
    preceding_lf: Option<RelativeSpan>, // one byte; None only for BOF support
}

struct RootEmptyEvidence {
    // Opaque witness of the predicate in §8, issued before EOF finalization.
    // It does NOT contain a resumable container snapshot.
}

struct UpdateStaging {
    old: Owned<ReadyDocument>,      // coherent and unmutated until frontier
    post: SourceAssociation,
    edit: ValidatedCanonicalEdit,
    mapping: EditMapping,
    region: Option<ReplacementRegion>,
    parsed: Option<ParsedReplacement>, // transient, never another retained Skel tree
    parse_scratch: Scratch,         // parser stack / inline-input workspaces
    prepared: Option<PreparedCommit>,
}

struct ParsedReplacement {
    blocks: Ordered<ParsedTopBlock>,
    facts: CompleteOrderedLocalFacts,
    boundary_observations: LocalBoundaryEvidence,
    // All items belong to the exact final [r, q_new) interval.
}

struct ParsedTopBlock {
    syntax: SharedSkel,
    physical_first_line_start: ByteOffset, // parser provenance, temporary absolute coord
    // Coverage/certificate assembly consumes this provenance before dropping it.
}

struct ReplacementRegion {
    restart_old: ByteOffset,
    restart_new: ByteOffset,       // equal to old restart in this single edit model
    end_old: ByteOffset,
    end_new: ByteOffset,
    old_cut_ranks: CutRanks,       // transaction-local, no stable locator map
    end: EndProof,
}

enum EndProof {
    Converged { old_boundary: OldBoundaryEvidence,
                new_boundary: RootEmptyEvidence,
                suffix: UnchangedSuffixProof,
                coverage: ExactCoverageCutProof },
    RealEof { completed: EofCompletionProof },
}

enum PreparedCommit {
    Splice {
        cuts: VerifiedCutPlan,
        fresh: Owned<OwnerSeq>,
        environment: ExactOrderedFactsEquality,
        resources: CommitResources,
    },
    ReplaceAll {
        fresh_ready: Owned<ReadyDocument>,
        resources: CommitResources,
    },
}

struct CommitResources {
    sequence_workspace: PreparedWorkspace,
    retirement_workspace: PreparedWorkspace,
    result_storage: PreparedStorage,
    counter_capacity: PreparedEventCapacity,
    arithmetic: CheckedBounds,
}
```

`RootEmptyEvidence` 是算法正确性前提的类型化表达，不意味着 Rust 类型系统已经证明 parser 正确。内部 producer 必须对应 §8 的真实 scanner 控制流，后续 correctness fixtures 才能检验实现。`ExactOrderedFactsEquality` 同样不能由 hash coincidence 或一个任意 bool 构造。

实现必须允许 payload/表内字符串具有独立于即将退休对象的有效期；可以有等价的安全表示，但不能引入跨 edit source borrowing 债务。最终 lifetime 实现另审，本次不选地址布局。

## 15. Worked semantic traces：用反例检查模型

以下是按已读取共享语法手工推导的设计 traces；仅核对了字节长度算术，没有运行 parser 或基准。后续实现须用独立 oracle 重放，不能把本节当已通过的测试。

记号 `⟦bytes⟧Kind` 表示一个 Owner 的**完整 coverage**，不是 semantic span。相邻 `⟦…⟧` 无缝拼接成整个 source；`\n` 是一个 LF byte。所有 incremental trace 都退休表中 O 的 records/payload/certificates 和本轮临时物；保留的表按所有权移动。所有 full trace 都退休**整个旧状态**及放弃的 attempt，P/S 仅为暂时建立过的 syntax proof，不实际保留。

多数例子使用同一 source frame：

```text
K = ⟦"keep\n\n"⟧Paragraph   length 6
G = ⟦"g\n\n"⟧Paragraph      length 3
Z = ⟦"tail\n\n"⟧Paragraph   length 6
E = ⟦"end\n"⟧Paragraph      length 4
source = K · G · X · Z · E
```

此时 K=[0,6)、G=[6,9)、X 从 9 开始，后面的 bases 是前面 coverage lengths 的前缀和。除特别标明，编辑命中 X，r=6（纳入 G），保留 prefix K。`q_old→q_new` 明确给出两个版本的切点。

### E01：paragraph edit，不改变边界

```text
old = K · G · ⟦"ab\n\n"⟧P · Z · E
new = K · G · ⟦"aXb\n\n"⟧P · Z · E
edit = insert "X" at byte 10
```

coverage：X `[9,13)→[9,14)`；Z/E 的 base 均 +1。r=6；q=13→14，最后 blank/LF support 未动，新 live root 为空。O=`G,P`，N=`G,P'`；Defs `[]==[]`，incremental。保留 K 和 Z/E；退休 G/P 的旧 payload。Z/E 的 relative spans、values、内部证书不变。这里明确付出 guard G 的额外重建。

### E02：paragraph split

```text
old = K · G · ⟦"a\nb\n\n"⟧P · Z · E
new = K · G · ⟦"a\n\n"⟧P1 · ⟦"b\n\n"⟧P2 · Z · E
edit = insert LF at byte 11
```

coverage：旧 X `[9,14)`；新 X `[9,12)+[12,15)`。r=6；q=14→15。新 P1/P2 之间多出一个 certified cut，但旧 source 在对应处没有真实 Owner boundary，故不能在那里提前 convergence。O=`G,P`，N=`G,P1,P2`；facts 空且相等，incremental；保留 K、Z/E，退休旧 G/P。

### E03：删除 blank separator，paragraph merge

```text
old = K · G · ⟦"a\n\n"⟧P1 · ⟦"b\n\n"⟧P2 · Z · E
new = K · G · ⟦"a\nb\n\n"⟧P · Z · E
edit = delete [11,12)
```

coverage：`[9,12)+[12,15)→[9,14)`。r=6；旧 cut 12 的支持 LF 已被删除，且新段落未闭合，必须拒绝；q=15→14。O=`G,P1,P2`，N=`G,P`；facts 空，相等，incremental；保留 K、Z/E，退休旧 G/P1/P2。不能只替换命中的 P1。

### E04：heading → text，旧 Owner 边界消失

```text
old = K · G · ⟦"a\n"⟧P · ⟦"# b\n\n"⟧H · Z · E
new = K · G · ⟦"a\nb\n\n"⟧P · Z · E
edit = delete "# " at [11,13)
```

coverage：旧 `[9,11)+[11,16)`，新 `[9,14)`。命中 H，t=11，strict safe predecessor r=9；P/H 之间虽然是 Owner cut，却无 certificate。q=16→14。O=`P,H`，N=`P'`；facts 空，incremental。保留 K/G 和 Z/E；退休旧 P/H。

反向 text→heading：从 `K·G·⟦"a\nb\n\n"⟧P·Z·E` 在 11 插入 `"# "`，r=6，q=14→16，O=`G,P`，N=`G,⟦"a\n"⟧P,⟦"# b\n\n"⟧H`；facts 空，incremental；保留 K、Z/E，退休 G/P。两个方向的 restart 可以不同，不能以新 AST 形状倒推旧 damage。

### E05：root blank boundary 的直接修改

```text
old = K · G · ⟦"a\n\n"⟧P · Z · E
new = K · G · ⟦"a\n \n"⟧P · Z · E
edit = insert SPACE at byte 11
```

coverage：P `[9,12)→[9,13)`。r=6。虽然新 blank 仍结束 root，旧 q=12 的直接 support 被 edit 触及；本文的保守 policy 跳过它。继续处理 Z，在 q=18→19 convergence。O=`G,P,Z`，N=`G,P',Z'`；facts 空，相等，incremental。保留 K 和 E；退休 G/P/Z。证明安全的模型可以有 conservative extra work，但不能隐藏它。

### E06：blockquote 内 blank line

```text
old = K · G · ⟦"> a\n>\n> b\n\n"⟧Quote · Z · E
new = K · G · ⟦"> a\n>\n>\n> b\n\n"⟧Quote · Z · E
edit = insert ">\n" at byte 15
```

coverage：Quote `[9,20)→[9,22)`。r=6；q=20→22，依赖 quote 之后的**外层** blank。quote 内 `>` blank 不提供 root certificate。O=`G,Quote`，N=`G,Quote'`；facts 空，incremental；保留 K、Z/E；退休整个旧 Quote payload（含全部后代）和 G。即使只多了一个空行，也不细分这个 Owner。

### E07：fence 内 blank line

```text
old = K · G · ⟦"```\nx\n\ny\n```\n\n"⟧Fence · Z · E
new = K · G · ⟦"```\nx\n\n\ny\n```\n\n"⟧Fence · Z · E
edit = insert LF at byte 15, inside the existing fence body
```

coverage：Fence `[9,23)→[9,24)`。r=6；q=23→24，只在 closer 之后的 root blank 处成立。body blank 不发证书；Fence.content 相对区间长度 +1，绝对投影随后得到。O=`G,Fence`，N=`G,Fence'`；facts 空，incremental；保留 K、Z/E；退休 G/整个旧 Fence。

### E08：插入 fence opener，吞掉原来的 suffix

```text
old = K · G · ⟦"a\n\n"⟧P · Z · E
new = K · G · ⟦"```\n\ntail\n\nend\n"⟧UnclosedFence
edit = replace [9,10) with "```"
```

coverage：旧 X/Z/E 共 `[9,22)`；新 Fence `[9,24)`。r=6；旧的 cut 12/18 即使有旧证书，新 live fence 仍打开，不能 convergence。真实 EOF 22→24 合法结束。O=`G,P,Z,E`，N=`G,Fence`；facts 空，incremental-to-EOF，**不是必须 full**。保留 K，无 retained suffix；退休旧 G/P/Z/E。新 unclosed fence span/content 按共享 EOF 规则结束。

### E09：删除 fence closer

```text
old = K · G · ⟦"```\nx\n```\n\n"⟧Fence · Z · E
new = K · G · ⟦"```\nx\n\ntail\n\nend\n"⟧UnclosedFence
edit = delete closer line [15,19)
```

coverage：旧 Fence `[9,20)`、Z/E 到 30；新 Fence `[9,26)`。r=6；没有 live root convergence，EOF 30→26。O=`G,Fence,Z,E`，N=`G,Fence'`；facts 空，相等，incremental-to-EOF。保留 K；退休其他旧 Owners。不能靠旧的 blank 证书跳过仍处于新 fence 内的 source。

### E10：补上 closer，暴露普通后文

E09 反向：old 是 `K·G·⟦"```\nx\n\ntail\n\nend\n"⟧Fence`，在 15 插入 `"```\n"`；new 是 `K·G·⟦"```\nx\n```\n\n"⟧Fence·Z·E`。

coverage：旧大 Fence `[9,26)`；新 Fence `[9,20)`、Z `[20,26)`、E `[26,30)`。r=6。虽然新 parser 在 20/26 可为空 root，旧大 Fence 内没有对应 Owner boundary，不能把新 Z/E 冒充保留旧 subtree。只能 EOF 26→30。O=`G,Fence`，N=`G,Fence',Z,E`；facts 空，incremental-to-EOF。保留 K，无 suffix；退休旧 G/Fence。

### E11：definition insertion

```text
old = K · G · ⟦"ab\n\n"⟧P · Z · E
new = K · G · ⟦"[x]: /u\n\n"⟧Def · ⟦"ab\n\n"⟧P · Z · E
edit = insert "[x]: /u\n\n" at 9
```

coverage：旧 P `[9,13)`；新 Def `[9,18)`、P `[18,22)`。r=6；syntax candidate q=13→22。旧 cut9 未跨过 inserted damage 且其 seam 被触及，不能作为完成点。O=`G,P`，N=`G,Def,P`；Defs `[]→[(x,/u)]`，**full**。语法上 K、Z/E 可保留不代表语义上获准；最终全部新建，退休所有旧 state/table 和 attempt。

### E12：winner deletion，后继定义胜出

```text
U = ⟦"[v][x]\n\n"⟧Paragraph  [0,8)
W = ⟦"[x]: /one\n\n"⟧Def     [8,19)
V = ⟦"[x]: /two\n\n"⟧Def     [19,30)
old = U · W · V · Z · E
new = U · V · Z · E
edit = delete [8,19)
```

新 coverage：U `[0,8)`、V `[8,19)`、Z `[19,25)`、E `[25,29)`。t=8，r=0；旧 q19 的 support 被删，candidate q=30→19。O=`U,W,V`，N=`U,V`；facts `[(x,/one),(x,/two)]→[(x,/two)]`，full。完整新表解析 U 得 `/two`。最终无保留 prefix/suffix；退休全部 old，不只是 W。

### E13：shadowed destination 改变，winner 没变

使用 E12 的 old，将 `[24,28)` 的 `/two` 替换为等长 `/xxx`。前后 coverage 完全相同。t=19，r=8；q=30→30，O=`W,V`，N=`W,V'`。

facts `[(x,/one),(x,/two)]→[(x,/one),(x,/xxx)]`，不等，full。有效 winner 仍为 `/one`，U 的最终 reference value 不变；这不能使 Horse-A 偷用 preserved branch。syntax prefix U、suffix Z/E 最终也重建；退休完整旧 state。报告理由必须是 `ordered_facts_differ`，不能写 `winner_changed`。

### E14：unresolved → resolved，引用位于定义之前

```text
old = ⟦"[v][x]\n\n"⟧U · Z · E
new = ⟦"[v][x]\n\n"⟧U · ⟦"[x]: /u\n\n"⟧Def · Z · E
edit = insert definition at 8
```

coverage：旧 U `[0,8)`、Z `[8,14)`、E `[14,18)`；新 U 同前、Def `[8,17)`、Z `[17,23)`、E `[23,27)`。r=0；candidate q=14→23，O=`U,Z`，N=`U,Def,Z`。facts `[]→[(x,/u)]`，full。U 从 literal Text 变为 ReferenceLink，且在完整新表建立后才 materialize。无最终 retained ranges；退休全部 old 和 attempt。

### E15：resolved → unresolved

E14 反向，删除 `[8,17)` 的唯一 Def。coverage 回到 U/Z/E；r=0；candidate q=23→14，O=`U,Def,Z`，N=`U,Z`。facts `[(x,/u)]→[]`，full。U 的 ReferenceLink 消失并按共享规则成为 literal Text，不能只清空 destination 字段。无最终保留范围；退休整个旧 state/table。
