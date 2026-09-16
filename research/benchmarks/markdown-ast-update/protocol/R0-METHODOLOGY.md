# R0 Methodology — MARKIT-MARKDOWN-BENCHMARK-1

Status: **PROTOCOL CORRECTIVE / NO BENCHMARK EXECUTION**

Authority: GitHub Issue #22, comment `R0-METHODOLOGY-CORRECTIVE-1`.

This file exists so the methodology can be cited and reviewed without searching issue history. It does not supersede Issue #22; it mirrors the R0 decisions recorded there.

## 1. Two comparison lanes

### Lane A — Native Implementation Performance

Run each upstream implementation in its supported runtime/toolchain with the same canonical payload and operation contract.

Subjects:

- B0 MD4C
- B1 pulldown-cmark
- B2 Comrak
- B3 tree-sitter-markdown
- B4 @lezer/markdown
- B5 mizchi/markdown

Measures:

- latency
- throughput
- CPU
- memory
- allocation where observable/comparable

Allowed conclusion: implementation-level performance under the frozen environment.

Not allowed: a raw cross-runtime wall-clock difference is not, by itself, proof that one parsing algorithm is superior.

### Lane B — Rust Algorithm Reproduction

Rust is the single reproduction language for controlled algorithm/mechanism comparisons.

Why Rust:

- no GC confounder;
- explicit allocation/layout/ownership control;
- direct C FFI for native subjects;
- natural integration with Rust baselines;
- appropriate for retained-tree / incremental parsing mechanism experiments.

A Rust reproduction is not an upstream baseline. It is a controlled reproduction used for causal/algorithmic attribution.

Each reproduction must record:

- upstream project and exact version/commit;
- algorithm/data-structure boundary being reproduced;
- omitted features;
- representation differences;
- port-specific decisions;
- differential/conformance tests against upstream behavior.

Failure of this gate => `REPRODUCTION_INVALID` and no algorithmic conclusion.

Default policy: reproduce only mechanisms needed to test an observed difference/weakness. Do not port six entire parsers merely for symmetry.

## 2. Canonical operation contract

Canonical edit descriptor:

```text
UTF-8 byte range [start,end)
+
inserted bytes
```

Each native adapter may convert this descriptor into its official API shape. The conversion must not change the logical edit.

## 3. Timer contract

### FULL_PARSE

Outside timer:

- file IO;
- fixture/source materialization;
- process/runtime startup;
- one-time runtime initialization;
- correctness/serialization/logging.

`T_native`:

```text
START
parser consumes the complete in-memory source
force the native result to completion
  event/pull parser: consume all events
  tree parser: complete tree/result construction
STOP
```

A lazy parser may not stop timing after constructing an iterator/object.

### UPDATE / STRUCTURAL_EDIT

Outside timer:

- old source materialized;
- old parse state constructed;
- canonical edit chosen;
- post-edit source materialized.

`T_prepare`:

```text
canonical edit -> parser-native coordinate/change metadata
```

`T_native`:

```text
parser-required old-state maintenance
+
incremental parse/update to completion
```

Headline:

```text
T_total = T_prepare + T_native
```

Always retain all three columns: `T_prepare`, `T_native`, `T_total`.

Parser-required coordinate/state maintenance must not be moved outside the timer merely to improve the result.

### Node/VM adapters

If Rust runner talks to Node/JS through IPC:

- `T_native` is measured inside the Node/JS runtime;
- serialization/pipe/scheduler IPC is excluded from `T_native`;
- integration cost may be reported separately as `T_adapter_roundtrip`;
- process model, warmup and GC policy must be preregistered.

## 4. Correctness contract

### C1 — Capability Matrix

Use official CommonMark/GFM examples to record supported dialect/capabilities and known divergence. This is context for interpreting performance and need not be run inside every timing case.

### C2 — Incremental Self-Equivalence

For incremental subjects:

```text
normalize(incremental(post-edit))
==
normalize(same-parser clean full parse(post-edit))
```

This is the primary timing-case correctness gate.

Do not hand-author expected ASTs for every arbitrary payload.

If two implementations clearly perform different semantic work, mark `CAPABILITY_DIFFERENCE` and constrain the cross-implementation conclusion.

## 5. Parse Amplification

First-version definition:

```text
PA = unique source-byte coverage re-inspected / logical edited bytes
```

Repeated reads of the same source byte are not double-counted in PA. Total byte-read work may be recorded separately as an attribution diagnostic when observable.

If coverage is not genuinely observable:

```text
PA = UNKNOWN
```

Do not infer PA from latency, changed ranges, or node counts.

Structural edits should additionally record affected syntax span and first stable/reusable suffix when observable.

## 6. Conclusion levels

```text
OBSERVATION
REPRODUCED_OBSERVATION
ATTRIBUTED_WEAKNESS
CROSS_IMPLEMENTATION_WEAKNESS
COMMON_WEAKNESS
PARETO_GAP
DESIGN_OPPORTUNITY
INCONCLUSIVE
REFUTED
```

Raw native timing can establish implementation observations only.

Algorithmic attribution requires scaling evidence + work/reuse evidence + controlled probe/counterexample, and where useful a valid Rust reproduction.

`COMMON_WEAKNESS` must state its target class, for example `B3+B4+B5`.

`PARETO_GAP` means no measured baseline simultaneously satisfies the declared target constraints; it does not require all implementations to share one mechanism failure.

## 7. R0 preregistration checklist

Before R1:

- baseline exact versions/commits/features/build profiles;
- B2 = Comrak;
- native implementation lane;
- Rust reproduction lane;
- canonical operation/edit coordinate;
- `T_prepare/T_native/T_total` boundaries;
- VM/Node process and warmup policy;
- CPU/OS/affinity/frequency policy;
- sampling count and p99 eligibility;
- case-order randomization + fixed seed;
- outlier policy;
- memory definitions;
- failure taxonomy: timeout/OOM/crash/wrong/unsupported;
- capability matrix policy;
- self-equivalence policy;
- PA definition and UNKNOWN rule;
- conclusion ladder.

## 8. Methodology references

1. Tim A. Wagner, Susan L. Graham. **Efficient and Flexible Incremental Parsing**. ACM TOPLAS 20(5), 1998. DOI: https://doi.org/10.1145/293677.293678
   - incremental work, reuse, scaling, retained parse structure.

2. Alex Hoppen. **Swift incremental syntax parsing** proposal/discussion, 2018. https://forums.swift.org/t/incremental-syntax-parsing/12368
   - incremental vs clean parse, reuse/work amount, source-size scaling.

3. Stefan Marr, Benoit Daloze, Hanspeter Mössenböck. **Cross-Language Compiler Benchmarking: Are We Fast Yet?** DLS 2016. DOI: https://doi.org/10.1145/2989225.2989232
   - common problems/abstractions across heterogeneous implementations; distinguish implementation comparisons from language claims.

4. Andy Georges, Dries Buytaert, Lieven Eeckhout. **Statistically Rigorous Java Performance Evaluation**. OOPSLA 2007. DOI: https://doi.org/10.1145/1297027.1297033
   - repeated runs and statistically disciplined runtime benchmarking.

5. Edd Barrett et al. **Virtual Machine Warmup Blows Hot and Cold**. OOPSLA 2017. DOI: https://doi.org/10.1145/3133876
   - warmup/steady-state assumptions are unsafe for JIT VMs.

6. Todd Mytkowicz et al. **Producing Wrong Data Without Doing Anything Obviously Wrong**. ASPLOS 2009. https://research.ibm.com/publications/producing-wrong-data-without-doing-anything-obviously-wrong
   - measurement bias and setup randomization.

7. Brian F. Cooper et al. **Benchmarking Cloud Serving Systems with YCSB**. SoCC 2010. DOI: https://doi.org/10.1145/1807128.1807152
   - common workload contracts for heterogeneous implementations; mixed workloads are Phase 2 here.

8. RocksDB `db_bench`. https://github.com/facebook/rocksdb/wiki/Benchmarking-tools
   - operation-oriented microbenchmarks (fill/overwrite/delete/read), adapted here to parse/edit/query operations.

These references justify methodology; they do not establish novelty for a future Markit algorithm.
