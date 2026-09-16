# AGENTS.md

This file defines the current working rules for humans and AI coding agents contributing to Markit.

## 1. Mission

Markit is a local-first Markdown editor/workspace whose document truth is Markdown source.

The intended product includes:

- Source Mode;
- source-aware Live Mode;
- workspace search;
- split Preview;
- Mermaid;
- LaTeX-style math;
- browser preview and reliable browser Print/PDF;
- Windows file association / Open With;
- future plugin/provider extensibility through stable boundaries.

The project is currently **not** in UI implementation mode.

## 2. Current phase — Markdown parser research

The active technical campaign is:

> **Issue #19 — Incremental Markdown parsing: locality, convergence, and invalidation**

The provisional research north star is:

> **For lossless Markdown editing under arbitrary edits, how can Markit minimize reparse radius, tree reconstruction, memory movement, and downstream render invalidation while preserving correctness?**

This question is provisional. Issue #19 must explicitly decide whether to:

```text
KEEP
REFINE
REPLACE
```

it after the experiments.

Do not start production parser replacement, rendering architecture, or UI redesign before the Issue #19 stop gate is reviewed.

## 3. Authority map

During this phase:

```text
docs/PRD.md
    = product requirements authority

Issue #19
+ docs/research/markdown-parser/README.md
    = parser research authority

docs/product/architecture.md
    = HOLD / invariant boundary only

docs/product/print-browser-contract.md
    = print/browser completeness contract

docs/product/mvp-v0.1.md
    = intended V0.1 product scope

docs/product/roadmap.md
    = work sequencing

docs/research/code-baseline.md
    = treatment of existing code during research
```

The complete pre-reset repository is archived at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

Old ADRs, GPUI/PocketJS decisions, realtime execution rules, old Markdown contracts, implementation notes, and benchmark conclusions are **historical evidence**, not current architecture authority.

## 4. Existing code is a baseline, not architecture

Current source paths remain in place to preserve reproducibility.

Treat them as:

```text
EXPERIMENTAL / REFERENCE
```

In particular:

- `crates/markit-core` is a useful document/Markdown comparison baseline for Issue #19;
- `apps/markit` is a frozen UI/GPUI reference;
- `mvp/gpui` is a historical feasibility prototype;
- old benchmark/results/tooling may be reused only after checking workload semantics.

Do not broadly move, rename, optimize, or delete the baseline before the experiment has used it.

After Issue #19, relevant components must explicitly receive one of:

```text
ADOPT
ADAPT
REPLACE
DELETE
```

Prior existence is not justification for adoption.

## 5. Frozen product invariants

These are valid even while architecture is on hold.

### 5.1 One document truth

```text
Markdown Source = authoritative document content
```

Source Mode, Live Mode, Preview, syntax trees, semantic views, browser output, print output, search results, caches, Mermaid, and math output are derived state.

Do not introduce a Source Document and a separate Rich/Live Document that require synchronization.

### 5.2 Losslessness

Markit must preserve source syntax needed for editing and exact round-trip behavior.

Do not choose a representation that silently loses Markdown markers or normalizes unrelated source merely to simplify rendering.

### 5.3 Heavy rendering is outside parser critical path

Markdown parsing may identify Mermaid/math syntax and dependencies.

It must not synchronously perform Mermaid rendering, LaTeX/math rendering, layout, shaping, browser generation, or GPU/UI work as part of ordinary parsing.

### 5.4 Print completeness is separate from interactive laziness

Interactive presentation may later be incremental, lazy, viewport-aware, progressive, or cancellable.

Print preparation must represent the complete coherent document. It must not depend on scroll history or which interactive regions were previously materialized.

Read `docs/product/print-browser-contract.md` for print work.

### 5.5 Future extension seams must remain possible

Do not make private parser nodes, Rust memory layout, UI entities, scheduler tasks, or caches the future plugin contract by accident.

The concrete plugin runtime is not selected.

## 6. Parser research rules

Do not assume any of the following before evidence:

- Rope is the right source storage;
- Piece Table is the right source storage;
- AST is the right persistent representation;
- CST is the right persistent representation;
- block-local CST is necessarily optimal;
- Tree-sitter is required;
- Lezer should be copied;
- MD4C full parsing is too slow;
- custom parsing is necessary;
- every local edit affects only one block;
- every structural edit must parse to EOF;
- absolute offsets are harmless metadata;
- parser latency alone determines editor latency.

All are hypotheses.

## 7. Comparison map

Issue #19 should study at least these families:

```text
Theory
  Wagner & Graham
  incremental parsing / reuse

Systems
  Tree-sitter
  Lezer
  Roslyn
  rust-analyzer / rowan

Markdown-specific
  @lezer/markdown
  tree-sitter-markdown
  mizchi/markdown
  MD4C
```

Streaming Markdown systems may be studied for stable-prefix / progressive-publication mechanisms, but append-only performance must not be confused with arbitrary-edit performance.

External systems are mechanism references, not proof.

## 8. Candidate hypothesis to test

A candidate direction is:

```text
arbitrary edit
    |
    v
small affected source region
    |
    v
block / inline parsing
    |
    v
boundary context / state summary
    |
    v
earliest safe convergence
    |
    v
reuse unaffected syntax
    |
    v
separate semantic dependency invalidation
```

This is deliberately a hypothesis, not architecture.

Important distinction:

```text
syntax invalidation
!=
semantic dependency invalidation
!=
downstream presentation invalidation
```

For example, changing a reference definition may be syntax-local while semantically affecting distant users. Do not automatically turn semantic fan-out into syntax reparsing.

## 9. Benchmark integrity

Wall-clock latency alone is insufficient.

Where practical record:

```text
full_parse_time
incremental_parse_time
bytes_rescanned
lines_rescanned
syntax regions / blocks reparsed
nodes rebuilt
nodes reused
allocation count
allocated bytes
memory movement / copies
offset/index maintenance work
propagation distance
semantic dependents changed
```

The correctness oracle is mandatory:

```text
incremental result == clean authoritative parse
```

for every mutation where the comparison is defined.

A fast wrong parser fails.

A parser that reparses one block but then rewrites metadata for every following block has not demonstrated local total work.

## 10. Mutation corpus

Use controlled edits, not only full-file throughput.

Cover at least:

- ordinary paragraph word/character edits;
- CJK/Unicode edits;
- emphasis/code/link delimiter edits;
- blank-line insert/delete;
- paragraph merge/split;
- Setext heading cases;
- list indentation/nesting;
- blockquote/lazy continuation;
- fenced-code opener/closer/length/info changes;
- reference-definition edits;
- Mermaid fence/body edits;
- inline/display math delimiter/body edits;
- edits near file start/middle/end;
- large paste/delete;
- CRLF/LF cases.

The corpus should reveal the real propagation distribution rather than presupposing locality.

## 11. Experiment hygiene

Prefer:

```text
question
  -> controlled workload
  -> instrumentation
  -> measurement
  -> differential / intervention
  -> correctness check
  -> conclusion
```

Do not justify a mechanism with:

- “this should be faster”;
- “native is faster”;
- “all editors use this”;
- “Zed/VS Code/Tree-sitter does it this way”;
- “the old Markit implementation already has it”.

Record toolchain, commit SHA, hardware/OS, corpus version, and benchmark mode for performance evidence.

Separate measurement overhead from the metric being measured.

## 12. Research implementation policy

Allowed during Issue #19:

- isolated benchmark harnesses;
- mutation corpus generation;
- parser adapters;
- correctness oracles;
- measurement instrumentation;
- research-only prototypes;
- result tables and reports.

Do not during Issue #19:

- replace the production parser;
- redesign GPUI/UI;
- implement Live Mode;
- implement final Preview rendering;
- choose plugin runtime;
- refactor the baseline merely for style;
- change parser semantics to improve benchmark locality.

If instrumentation changes measured behavior, document it.

## 13. Unicode and source coordinates

Production-facing ideas must remain compatible with Unicode/CJK/emoji even if an early microbenchmark isolates ASCII.

Do not conflate:

- byte offsets;
- Unicode scalar positions;
- grapheme boundaries;
- logical source positions;
- visual/display positions;
- platform UTF-16 coordinates.

Any persistent position/index design should state which coordinate system it represents and what edit-shift cost it creates.

## 14. Documentation rules

Keep the active documentation tree small.

Current product files belong under `docs/product/`; active research belongs under `docs/research/`.

Do not resurrect old ADRs into `docs/adr/` merely because a decision seems plausible. Write a new ADR only after the new architecture is evidence-backed and reviewed.

Historical wording is available from the archive revision; do not duplicate large historical documents into active paths.

When research changes the root question, parser direction, or authority boundary, update:

```text
docs/research/markdown-parser/README.md
docs/product/architecture.md
docs/product/roadmap.md
README.md
AGENTS.md
```

as applicable.

## 15. Architecture gate

Do not write the next full architecture until Issue #19 provides:

1. reproducible corpus;
2. mechanism comparison;
3. correctness oracle results;
4. locality/propagation evidence;
5. representation/memory-work evidence;
6. research-question verdict (`KEEP`, `REFINE`, `REPLACE`);
7. parser-direction verdict.

Only then should architecture decide:

```text
source storage
  -> parsing/reuse
  -> lossless syntax / semantic representation
  -> dependency tracking
  -> downstream invalidation contract
  -> rendering/layout
  -> UI backend
```

## 16. Stop rule

When uncertain during the current phase, prefer:

```text
measure the Markdown edit
```

over:

```text
design the editor around an unproven parser
```

The current objective is not to make the repository look finished. It is to earn the mechanism that future Markit architecture will be built on.
