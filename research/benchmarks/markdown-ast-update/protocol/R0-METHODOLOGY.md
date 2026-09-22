# R0 Methodology — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R0 FROZEN / PASS / READY FOR R1**

Authority: GitHub Issue #22 + this file. This file is the authoritative R0 methodology for the first experiment. Older #22 text describing native cross-runtime parser ranking, a dual-lane primary design, or four horses is superseded.

## 0. Research question

The first experiment asks:

> **Under one Rust experimental substrate, one Markdown semantic core, the same payload, the same edit, and the same normalized result contract, what do different AST/CST update mechanisms cost, where do they degrade, and why?**

The primary object is the **update mechanism**, not the source language/runtime or a product implementation.

Research chain:

```text
prior-art/source reconnaissance
-> extract mechanism
-> common Rust experimental substrate
-> SAME parser semantics / SAME payload / SAME edit / SAME result contract
-> mechanism race
-> timing + scaling + work counters
-> controlled attribution
-> replication
-> strength / weakness profile
-> Weakness Map
```

Evidence that cannot advance this chain remains `INCONCLUSIVE`.

### 0.1 Canonical research question and frozen question set: DQ1-DQ7 + MQ1-MQ7 (MEASUREMENT-CORRECTIVE-1 §2; split restored in review round 2)

The canonical research question this benchmark ultimately serves is the #33
question, unchanged:

> **Under the same grammar, result contract, Rust substrate, machine/profile, project/file set, and edit workload, how do different Markdown AST update mechanisms differ in performance, behavior, resource cost, and failure/degradation regime; and under what concrete conditions does each mechanism work best or worst, and why?**

The R0 question above is this question's measurement-phase form. The
benchmark exists to answer **decision questions** — questions whose
answers change what Markit does next. A measurement that cannot move one
of these decisions is bookkeeping, not evidence. The question set is
frozen here so that later stages cannot quietly re-scope it, and it is
split into two layers with a fixed relationship:

```text
DQ1-DQ7  DECISION QUESTIONS — what the benchmark must ultimately answer
   ↓ answered by
measurement (frozen workload + frozen protocol)
   ↓ made credible by
MQ1-MQ7  MEASUREMENT / ATTRIBUTION QUALIFICATION QUESTIONS — is the
         evidence good enough to answer a DQ
   ↓
answers to DQ1-DQ7
```

MQ questions qualify evidence; they never replace or re-scope the DQ
questions. The split was restored in MEASUREMENT-CORRECTIVE-1 review
round 2 after the first freeze had promoted the qualification questions
over the decision questions.

DQ1-DQ7 (DECISION QUESTIONS, frozen):

```text
DQ1  Local text edit inside an existing block: which mechanism is best?
                                 (decides: whether an incremental
                                 mechanism wins the common case)
DQ2  As the affected block grows, which mechanism degrades first?
                                 (decides: the degradation order as the
                                 edit blast radius grows)
DQ3  List / blockquote container edit: which mechanism is best?
                                 (decides: the container-edit strategy)
DQ4  Under fence forward propagation, which mechanisms degrade, and why?
                                 (decides: whether forward propagation is
                                 a cliff, and for whom)
DQ5  When a reference dependency changes, which mechanism pays the most?
                                 (decides: reference-edit cost attribution)
DQ6  On small files, is full rebuild (H0) the outright cheaper choice?
                                 (decides: the small-file strategy)
DQ7  As the N/B/L/K/F regimes vary, where are the crossovers?
                                 (decides: the axes and crossover points
                                 of the regime map)
```

MQ1-MQ7 (MEASUREMENT / ATTRIBUTION QUALIFICATION QUESTIONS, frozen):

```text
MQ1  Does any incremental mechanism (H1-H4) deliver a measured end-to-end
     update win over H0 full rebuild on the primary real workload at
     realistic file scales — or is H0 already adequate for Markit's
     document sizes?            (qualifies: the aggregate evidence behind
                                 DQ1/DQ6)
MQ2  Which mechanism family's avoided work (blocks reparsed, nodes
     rebuilt/reused, metadata touched) explains its wins, and is that
     avoided work stable across regimes rather than timing noise?
                                 (qualifies: the causal reading of any
                                 DQ1-DQ7 ranking)
MQ3  How often do conservative soundness fallbacks fire on real edits, and
     what does fallback-inclusive cumulative work look like compared with
     the incremental happy path?
                                 (qualifies: whether measured wins survive
                                 fallback-inclusive accounting)
MQ4  Which structural regimes (file size, block count, largest block,
     container depth, fence density, reference density) flip the measured
     ranking — where do the measured crossovers and cliffs land?
                                 (qualifies: the measurement behind
                                 DQ2/DQ4/DQ7)
MQ5  Is the measured source-inspection effort (unique vs cumulative, Old
     vs Post) consistent with each mechanism's claimed locality — does
     the counter evidence survive attribution audit?
                                 (qualifies: the locality claims behind a
                                 DQ1/DQ5 answer)
MQ6  How much of each mechanism's measured time and counters is spent
     producing the usable native state, versus proving the result equals
     the normalized oracle?
                                 (qualifies: the timer-boundary reading of
                                 every DQ answer)
MQ7  Which metric families are QUALIFIED for decision-making under the
     current substrate, and can a candidate conclusion be stated without
     leaning on UNAVAILABLE metrics?
                                 (qualifies: whether a conclusion is
                                 writable at all)
```

Result contract for the decision questions: every promoted benchmark
conclusion must name the DQ(s) it answers, the MQ(s) that qualify its
evidence, the evidence class supporting it (#33 §1), and the metric
families it relies on, each QUALIFIED under
`protocol/MEASUREMENT-CORRECTIVE-1.md` §9. A conclusion requiring an
UNAVAILABLE family is not writable in this environment.

---

## 1. Primary experiment: one Rust substrate

Headline results compare controlled Rust mechanism models only.

Shared and frozen non-research variables:

```text
Rust toolchain / Cargo.lock / build profile
source representation
canonical edit descriptor
BENCH-GRAMMAR-v1 semantic contract
normalized syntax result contract
node semantic vocabulary
corpus + mutations
runner / timer implementation
allocator policy
instrumentation semantics
result schema
machine/environment
```

Allowed research variables:

```text
damage detection
restart strategy
reuse unit / reuse lookup
convergence strategy
retained state required by the mechanism
reconstruction strategy
position/range maintenance required by the mechanism
fallback policy
```

Rule:

> Share non-research code whenever doing so preserves the semantics of every mechanism. Do not share code when sharing would erase a mechanism-specific cost or state requirement.

The purpose of the common substrate is to reduce implementation variance. It does not permit claiming that a mechanism model is a byte-for-byte Rust port of an upstream parser.

---

## 2. Prior-art role

Primary source/prior-art subjects:

```text
MD4C
pulldown-cmark
Comrak
Tree-sitter Markdown
@lezer/markdown
mizchi/markdown
Wagner & Graham incremental parsing
Swift incremental syntax parsing
```

Their roles are:

1. inspect source/design documents;
2. extract restart/reuse/tree/position/semantic mechanisms;
3. provide provenance for each horse;
4. run small upstream sanity probes when useful;
5. check whether a Rust mechanism model exhibits obviously impossible behavior relative to its inspiration;
6. supply hypotheses about strengths/weaknesses.

Upstream absolute timing is `REFERENCE_ONLY`. It does not enter the controlled Rust headline ranking.

---

## 3. First-round horses — FROZEN

The first round contains **five** mechanism models.

### H0 — FULL_REBUILD

Control.

```text
post-edit source
-> clean full parse
-> rebuild normalized syntax state
```

Purpose: cost floor for zero reuse.

Prior-art anchors: clean full-parse routes such as MD4C / pulldown-cmark / Comrak.

### H1 — BLOCK_LOCAL_REPARSE

```text
identify affected Markdown block region
-> reparse affected block(s)
-> preserve unaffected block states
-> repair document sequence/index
```

Primary variables:

```text
B = block count
L = affected/largest block length
block-boundary mutations
```

Prior-art anchor: block-oriented incremental Markdown designs such as mizchi/markdown.

### H2 — FRAGMENT_REUSE

```text
retain reusable syntax fragments/subtrees
-> map edit through retained fragments
-> invalidate damaged fragments
-> parse gaps/damaged regions
-> compose new syntax state
```

Primary variables:

```text
fragment granularity
fragment invalidation
edit position
large leaf/block
fragment metadata maintenance
```

Prior-art anchor: Lezer-style reusable fragments/tree fragments.

### H3 — OLD_TREE_SUBTREE_REUSE

```text
retain old syntax tree
-> apply edit mapping to old-tree coordinates/state
-> parse post-edit source while consulting old tree
-> reuse unchanged compatible subtrees
-> rebuild only unreused structure required for the new result
```

Primary variables:

```text
subtree match/reuse granularity
old-tree navigation
changed-region shape
nested structure
position/range maintenance
```

Prior-art anchor: Tree-sitter-style old-tree reuse concepts.

### H4 — RESTART_CONVERGENCE

```text
retain restart/checkpoint state
-> choose a valid restart before the damage
-> parse forward from restart
-> detect semantic/parser-state convergence
-> reuse stable suffix after convergence
```

Primary variables:

```text
restart distance
container/state propagation
forward-state propagation
convergence distance
checkpoint density
```

Prior-art anchors: classical incremental parsing, Swift incremental syntax concepts, and Markdown-specific restart/convergence observations.

### Fidelity naming rule

Horse names describe **mechanism models**, not reimplementations of products.

If provenance supports only inspiration rather than faithful reproduction, documentation must use `<project>-inspired` wording and state the fidelity boundary.

---

## 4. IMPLEMENTATION_PARITY_CONTRACT — FROZEN

This contract prevents the experiment from becoming a contest in Flash coding quality or horse-specific Rust micro-optimization.

### 4.1 One workspace / one compiler regime

All horses live in one Rust workspace and are measured by the same runner.

Freeze and record:

```text
rustc version
Cargo.lock
opt-level
LTO policy
codegen-units
panic strategy
target / target-cpu policy
RUSTFLAGS
allocator policy
```

A horse may not use a different compiler/profile/allocator merely to improve its result.

### 4.2 Shared substrate

The following should be implemented once and shared unless sharing would erase a mechanism-specific cost:

```text
Source / Edit descriptor
BENCH-GRAMMAR-v1 definitions
scanner/token primitives where semantics are identical
semantic actions
normalized Node / result vocabulary
common source/span utilities
corpus/mutation generation
runner/timer
result schema
common counters
oracle normalization
```

### 4.3 Horse-owned code

A horse owns only state/work that is intrinsic to its mechanism, for example:

```text
H1 block damage/index state
H2 fragment table / edit mapping
H3 old-tree cursor / subtree reuse state
H4 restart checkpoints / convergence state
```

A horse must not silently replace shared scanning/grammar/output code with a faster private implementation.

### 4.4 First-round optimization ban

Unless a feature is mechanism-intrinsic and explicitly declared, first-round horse code may not introduce horse-specific:

```text
custom allocator
unsafe unchecked indexing
SIMD / vector intrinsics
manual prefetch
parallelism
specialized hash function
horse-specific source/string representation
#[inline(always)] / #[cold] policy used only for one horse
hand-written assembly
```

If an optimization is mechanism-intrinsic, mark it `MECHANISM_INTRINSIC`, explain why removing it changes the mechanism, and make it eligible for later ablation where feasible.

### 4.5 black_box / dead-work protection

Runner inputs and produced states/results must be protected from trivial optimizer elimination using `std::hint::black_box` or an equivalent common runner mechanism.

Minimum shape:

```text
source/edit -> black_box
horse operation
result/state -> black_box
```

`black_box` is necessary but not treated as a formal optimizer barrier; deterministic checksums/state validation outside the timed region must additionally prove that full work was completed.

### 4.6 Optimization-sensitivity check

A result may enter the final Weakness Map only after at least one **key representative case** for that conclusion is rerun under a second frozen compiler profile, e.g. primary profile vs LTO-off.

Interpretation:

```text
absolute time changes, mechanism ordering/scaling remains
-> stronger mechanism evidence

ordering or claimed weakness flips materially
-> OPTIMIZATION_SENSITIVE
-> conclusion is bounded/downgraded, not promoted as strong algorithmic evidence
```

This check is for final candidate conclusions, not the full Cartesian matrix.

---

## 5. Shared parser semantic core

Every horse must solve the same parsing problem.

### BENCH-GRAMMAR-v1

First-round minimum:

```text
plain paragraph / text
blank-line block boundary
ATX heading
basic list / blockquote container
fenced code block
emphasis delimiter
code-span delimiter
inline/reference link basics
reference definition
```

The first round does **not** claim full CommonMark conformance.

Expansion to full CommonMark/GFM requires a protocol amendment after the first experiment.

### Normalized result contract

Every horse must normalize to the same semantic result shape:

```text
node kind
ordered parent/child topology
UTF-8 source byte spans/source-slice relation
ordered block/inline structure
reference-definition facts included by BENCH-GRAMMAR-v1
```

Correctness compares semantics/topology/spans, **not reuse identity**.

The following must never be part of correctness equivalence:

```text
pointer identity
allocation address
NodeId persistence
fragment ID
Arc/Rc identity
reuse count
```

Those are work/reuse diagnostics only.

Correctness oracle:

```text
normalize(H1/H2/H3/H4 update result)
==
normalize(H0 clean full parse(post-edit source))
```

Therefore arbitrary benchmark cases do not require hand-authored expected ASTs.

---

## 6. Canonical operation contract

Source authority: UTF-8 bytes.

Canonical edit:

```text
[start,end) byte range
+
inserted UTF-8 bytes
```

Host text mutation is outside parser timing because the first experiment studies syntax update, not Rope/PieceTree/text-buffer design.

All horses receive the same logical inputs:

```text
old source
old mechanism state
post-edit source
canonical edit
```

No horse may generate private workloads.

---

## 7. Timer contract — FROZEN

All horses are measured by the same Rust runner and timing API.

### 7.1 H0 / FULL_PARSE

Outside timer:

```text
file IO
corpus generation
post-edit source construction
case enumeration
logging / serialization
oracle comparison
```

`T_native`:

```text
START
horse receives already-in-memory UTF-8 source
all horse-required parsing and representation construction completes
black_box(result/state)
STOP
```

### 7.2 H1-H4 / UPDATE

Outside timer:

```text
old source materialized
old horse state constructed before the edit case
canonical edit chosen
host applies edit / post-edit source materialized
```

`T_prepare`:

```text
only mechanism-specific edit-coordinate / edit-metadata preparation
required after receiving the canonical edit
```

`T_native`:

```text
START
mechanism-specific old-state maintenance
damage / restart / fragment / subtree / convergence logic
reparse work
representation reconstruction
index/range maintenance required by the mechanism
all work required to produce a valid new horse state
black_box(new_state)
STOP
```

Headline:

```text
T_total = T_prepare + T_native
```

Always retain:

```text
T_prepare
T_native
T_total
```

### 7.3 Timer fairness hard rule

If a helper is identical and required by all horses, keep it on the same side of the timer boundary for all horses.

If work exists only because a mechanism requires it, it is part of that mechanism's cost.

No horse-required coordinate/state/index/input work may be moved outside timing merely to improve its number.

---

## 8. First-round operations

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

Structural minimum must cover six propagation families:

```text
local text
block boundary
container state
forward state
inline delimiter state
semantic dependency
```

Representative edits:

```text
paragraph split/merge
list or blockquote depth change
fence open/close
emphasis/code-span delimiter edit
link/reference delimiter edit
reference definition change
```

The first round does not attempt exhaustive Markdown syntax coverage.

---

## 9. Payload

Synthetic shapes:

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

First-round sizes:

```text
64 KiB
1 MiB
16 MiB
```

Add intermediate points such as 256 KiB / 4 MiB only after a crossover/cliff is observed and record the addition as an analysis follow-up, not a rewrite of earlier results.

Real Markdown such as `CppCoreGuidelines.md` is a realism/sanity corpus. Strict horse comparison uses only cases/slices whose semantics are defined by BENCH-GRAMMAR-v1.

---

## 10. Measurements

Headline:

```text
latency p50 / p95
throughput for full parse
CPU time
peak / retained memory
allocation count / bytes
```

Algorithmic work counters:

```text
unique source intervals/bytes re-inspected
blocks reparsed
nodes rebuilt
nodes reused
metadata/range records touched
restart distance
convergence distance
fallback-to-full count
```

Mechanism-specific counters may be added if their semantics are documented before use in a conclusion.

### Parse Amplification

```text
PA = unique source-byte coverage re-inspected / logical edited bytes
```

PA is derived only from explicit instrumentation. Do not infer it from latency, changed ranges, or node counts.

---

## 11. Attribution ladder

```text
1. timing difference
2. scaling signature across N/B/L/D/K/F
3. algorithmic work counters
4. allocation / bytes moved / representation maintenance
5. controlled mutation / ablation / counterexample
6. optimization-sensitivity check for key conclusions
7. only if residual remains unexplained: microarchitectural profiling
```

First-round PMU/cache analysis is optional and only used when algorithmic work is similar but wall-clock remains materially different.

Possible residual probes:

```text
cycles
instructions
branches / branch-misses
cache references / misses
L1/LLC misses if available
```

Empirical scaling signatures are not promoted to asymptotic proofs.

---

## 12. Strength / Weakness profile

Each horse ultimately receives:

```text
MECHANISM
PRIOR_ART_ANCHOR
BEST REGIME
WORST REGIME
SCALING SIGNATURE
WORK AMPLIFICATION
MEMORY / ALLOCATION COST
FAILURE / FALLBACK REGIME
KEY STRENGTH
KEY WEAKNESS
EVIDENCE
OPTIMIZATION_SENSITIVITY
CONFIDENCE
```

The purpose is to identify regimes in which each mechanism helps or fails, so the next independent campaign can investigate combinations or new designs.

No Markit production algorithm may be designed or implemented before the Weakness Map human review.

---

## 13. Sampling / repeatability

First round:

```text
3 independent sessions
10 warmup iterations/session
30 measured iterations/session
case order shuffled with recorded fixed seed
report p50 + p95
no p99
no outlier deletion
```

Record at minimum:

```text
OS/kernel
CPU model
affinity
turbo/frequency policy
rustc/toolchain
Cargo.lock / runner commit
build profile / RUSTFLAGS
allocator policy
corpus manifest
case-order seed
```

Failures are never silently dropped:

```text
PASS
WRONG_RESULT
UNSUPPORTED
TIMEOUT
OOM
STACK_OVERFLOW
CRASH
INSTRUMENTATION_UNAVAILABLE
OPTIMIZATION_SENSITIVE
```

---

## 14. Conclusion ladder

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

Promotion rules:

- single timing fact -> `OBSERVATION`;
- stable independent sessions -> `REPRODUCED_OBSERVATION`;
- scaling + work counters + controlled probe -> `ATTRIBUTED_*`;
- similar evidence across independent mechanism models -> `CROSS_MECHANISM_PATTERN`;
- no horse satisfies the preregistered target constraints -> `PARETO_GAP`;
- only real editing-relevant weakness/pattern/gap may become `DESIGN_OPPORTUNITY`.

A conclusion that materially flips under the optimization-sensitivity check cannot be promoted as a strong algorithmic weakness/strength without explicit `OPTIMIZATION_SENSITIVE` qualification.

---

## 15. Methodology references and what #22 borrows

1. Tim A. Wagner, Susan L. Graham. **Efficient and Flexible Incremental Parsing**. ACM TOPLAS 20(5), 1998. DOI: https://doi.org/10.1145/293677.293678
   - incremental work, reuse, retained structure, scaling.

2. Alex Hoppen. **Swift incremental syntax parsing** proposal/discussion, 2018. https://forums.swift.org/t/incremental-syntax-parsing/12368
   - incremental vs clean parse, reuse/work amount, source-size scaling.

3. Catherine McGeoch. **A Guide to Experimental Algorithmics**, 2012.
   - controlled computational experiments for mechanism insight, not runtime-only scoreboards.

4. Stefan Marr, Benoit Daloze, Hanspeter Mössenböck. **Cross-Language Compiler Benchmarking: Are We Fast Yet?** DLS 2016. DOI: https://doi.org/10.1145/2989225.2989232
   - common problem/abstraction and careful interpretation of implementation results.

5. Andy Georges, Dries Buytaert, Lieven Eeckhout. **Statistically Rigorous Java Performance Evaluation**. OOPSLA 2007. DOI: https://doi.org/10.1145/1297027.1297033
   - repeated runs and disciplined performance measurement.

6. Edd Barrett et al. **Virtual Machine Warmup Blows Hot and Cold**. OOPSLA 2017. DOI: https://doi.org/10.1145/3133876
   - do not assume warmup automatically yields a stable state.

7. Todd Mytkowicz et al. **Producing Wrong Data Without Doing Anything Obviously Wrong**. ASPLOS 2009.
   - measurement bias and execution-order/setup sensitivity.

8. Brian F. Cooper et al. **Benchmarking Cloud Serving Systems with YCSB**. SoCC 2010. DOI: https://doi.org/10.1145/1807128.1807152
   - common operation/workload contracts; mixed workload deferred to Phase 2.

9. RocksDB `db_bench`.
   - operation-oriented microbenchmarks, adapted here to parse/update/query operations.

10. Experimental algorithmics / algorithm engineering methodology work referenced in #22 review.
   - implementation choices are a validity threat; preregistration, controlled implementations, and reproducibility matter.

These sources justify methodology. They do not establish novelty for any future Markit algorithm.

---

## 16. R0 final verdict

```text
BENCH_GRAMMAR_V1:             PASS
NORMALIZED_RESULT_CONTRACT:   PASS
HORSE_SET_H0_H4:              PASS
IMPLEMENTATION_PARITY:        PASS
TIMER_CONTRACT:               PASS
PAYLOAD_MODEL:                PASS
WORK_COUNTERS:                PASS
ATTRIBUTION_LADDER:           PASS
SAMPLING_POLICY:              PASS
METHODOLOGY_BASIS:            PASS

R0 VERDICT: PASS
NEXT: R1 CONTROLLED RUST HARNESS / DIRECTORY SUBSTRATE
```

R1 may build infrastructure only. It may not tune horses using benchmark results or start Markit-specific production algorithm design.