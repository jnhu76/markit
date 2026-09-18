# #22 — Controlled Rust Markdown Update Mechanism Benchmark

Status: **R5 READY_FOR_ADVERSARIAL_R5_REVIEW (2026-09-17). PR opened
(`research: implement R5 incremental mechanism horses (#22)`), NOT merged.
Prior: R4 H0_REFERENCE_PASS (2026-09-17). Human final review passed;
PR #29 merged into master as
`21d7d832fec84fceedb7600cccb4296745395fc1`. R3
GRAMMAR_CORPUS_MUTATION_FREEZE_PASS (human final review passed; PR #28
merged into master as `17f6040b21f59f452fa57d67b512d0f4858f431f`).**

Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**

- Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)
- R0 methodology: [`protocol/R0-METHODOLOGY.md`](./protocol/R0-METHODOLOGY.md)
- R1 harness contract: [`protocol/R1-HARNESS-CONTRACT.md`](./protocol/R1-HARNESS-CONTRACT.md)
- R2 prior-art records: [`prior-art/`](./prior-art/README.md)
- R3 freeze record:
  [`protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md`](./protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md)
  (grammar/, corpus/, mutations/, cases/)
- R4 stage record:
  [`protocol/R4-H0-REFERENCE-FULL-REBUILD.md`](./protocol/R4-H0-REFERENCE-FULL-REBUILD.md)
  (oracle/, corpusgen/, mechanisms/full-rebuild/, scripts/verify-r4.sh)
- R4 corrective record:
  [`protocol/R4-H0-REFERENCE-CORRECTIVE-1.md`](./protocol/R4-H0-REFERENCE-CORRECTIVE-1.md)
  (field-legality gate; F027/F028/F038 conformance repair; zero-length
  contract; `nodes_reused` = Known(0))
- R5 mechanism freeze + stage record:
  [`protocol/R5-HORSE-CORRECTNESS-PARITY.md`](./protocol/R5-HORSE-CORRECTNESS-PARITY.md),
  [`protocol/R5-HORSES-STAGE-RECORD.md`](./protocol/R5-HORSES-STAGE-RECORD.md)
  (`shared-grammar/`, `mechanisms/{block-local,fragment-reuse,old-tree-subtree-reuse,restart-convergence}/`,
  `scripts/verify-r5.sh`)

本目录是 #22 的唯一实验代码、实验数据和研究报告工作区。

第一阶段只比较统一 Rust 基底下的五种 update mechanism：

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

所有 horses 共享 BENCH-GRAMMAR-v1、normalized result contract、payload/edit、runner/timer、Rust toolchain/build profile 和 allocator policy。Prior-art projects（MD4C、pulldown-cmark、Comrak、Tree-sitter Markdown、Lezer、mizchi/markdown）只用于机制来源、provenance、sanity/fidelity probes；其原生绝对时间不进入 headline ranking。

R0 已冻结 `IMPLEMENTATION_PARITY_CONTRACT`：共享非研究代码；horse 只拥有机制固有状态；第一轮禁止 undeclared horse-specific allocator/unsafe/SIMD/prefetch/parallelism/string/hash/inline tuning；输入/输出用统一 `black_box`/full-work validation；最终 Weakness Map 候选要做 optimization-sensitivity check。

正确性：

```text
normalize(H1/H2/H3/H4 update result)
==
normalize(H0 clean full parse(post-edit source))
```

Correctness 不比较 pointer/NodeId/fragment/allocation identity。

Timer：

```text
T_prepare = mechanism-specific edit metadata preparation
T_native  = mechanism-required update work to valid new state
T_total   = T_prepare + T_native
```

Host text-buffer apply-edit 在 timer 外；机制自己的 state/index/reuse/restart/reconstruction 工作不得隐藏。

实验目录固定为 `protocol/ manifest/ runner/ common/ mechanisms/ corpus/ oracle/ instrumentation/ prior-art/ scripts/ results/ report/`，其中 `mechanisms/` 下为五匹 horse。

当前 Gate：

```text
R0 = PASS
R1 = PASS（controlled Rust harness substrate；verify-r1.sh 强制）
R2 = PASS（prior-art mechanism extraction；PR #27 已合并）
R3 = GRAMMAR_CORPUS_MUTATION_FREEZE_PASS（BENCH-GRAMMAR-v1 + corpus/mutation
      freeze；人工最终评审通过；PR #28 已合并为 17f6040；
      verify-r3.sh 静态门 + corrective regressions 强制）
R4 = H0_REFERENCE_PASS（H0 reference full rebuild：
      oracle/ + corpusgen/ + mechanisms/full-rebuild/；
      43/43 fixtures；differential/QUERY/eager/attribution 套件；
      verify-r4.sh 正确性门（含 R1/R3 回归 + 5/5 负向 mutation 检查）；
      冻结 158-slot 结构矩阵逐槽复现；
      CORRECTIVE-1 已应用：CodeSpan 字段权威矛盾修复（F027/F028/F038），
      共享 field-legality 门（Rust validate.rs + verify_r3.py FIELD_TABLE），
      零长度契约 start<end 对所有节点，nodes_reused = Known(0)；
      R3 语义未扩展——仅修复 fixture 工件一致性；
      人工最终评审通过，PR #29 已合并为 21d7d83）
NEXT = R5 H1/H2/H3/H4 Mechanism Implementation（correctness + identity +
       parity only；no measurement）
```

R4 只实现 H0 参考与正确性门，不做任何 measurement、不计时、不开始
horse tuning、不实现 Markit production algorithm。