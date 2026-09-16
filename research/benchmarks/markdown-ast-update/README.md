# #22 — Controlled Rust Markdown Update Mechanism Benchmark

Status: **R2 CORRECTIVE PASS APPLIED — READY_FOR_ADVERSARIAL_R2_REVIEW (PR #27 open, awaiting human review)**

Issue authority: **#22 MARKIT-MARKDOWN-BENCHMARK-1**  
Execution roadmap: [`ROADMAP.md`](./ROADMAP.md)  
R0 methodology: [`protocol/R0-METHODOLOGY.md`](./protocol/R0-METHODOLOGY.md)  
R1 harness contract: [`protocol/R1-HARNESS-CONTRACT.md`](./protocol/R1-HARNESS-CONTRACT.md)  
R2 prior-art records: [`prior-art/`](./prior-art/README.md)

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
R1 = PASS（controlled Rust harness substrate；null mechanism 端到端通过 runner，
      schema/timer/case/seed/lane 全部由集成测试与 verify-r1.sh 强制）
NEXT = R2 Prior-art Mechanism Extraction
```

R1 只搭实验平台/schema；R2 只做 prior-art 机制提取。不得开始 horse tuning、
benchmark claim 或 Markit production algorithm。