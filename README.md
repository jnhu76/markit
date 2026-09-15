# Markit

**Markit** is a local-first Markdown editor/workspace built around one authoritative Markdown source.

The intended product has two first-class editing modes:

- **Source Mode** — direct lossless Markdown source editing;
- **Live Mode** — source-aware rendered editing that writes through to the same Markdown source.

The product also targets workspace search, split preview, Mermaid, LaTeX-style math, browser preview, reliable browser Print/PDF, and Windows file association/Open With.

> **Markdown Source is the single source of truth.**

## Current status

Markit is **not currently implementing the UI architecture**.

The active phase is:

> **Incremental Markdown parser research — Issue #19**

The immediate question is how an arbitrary source edit should propagate through Markdown parsing while minimizing unnecessary work and preserving exact correctness.

The provisional research north star is:

> **For lossless Markdown editing under arbitrary edits, how can Markit minimize reparse radius, tree reconstruction, memory movement, and downstream render invalidation while preserving correctness?**

That wording itself is subject to experiment. Issue #19 must ultimately decide whether to `KEEP`, `REFINE`, or `REPLACE` it.

## Why parser research comes first

The old repository already contains experiments in document editing, incremental Markdown, GPUI, viewport rendering, and performance measurement. They are useful evidence, but they no longer define the new architecture.

Before designing rendering and UI around those assumptions, Markit will first determine:

```text
arbitrary edit
    |
    v
what syntax is actually invalid?
    |
    v
how far must parsing propagate?
    |
    v
where can old syntax safely be reused?
    |
    v
what semantic dependencies really changed?
```

Only after that evidence exists should Markit define the parser/semantic contract consumed by rendering.

## Research comparison map

Issue #19 compares ideas and mechanisms from:

```text
Theory
  Wagner & Graham
  incremental parsing / optimal reuse

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

Candidate ideas such as small-region parsing, block/inline separation, boundary-state checkpoints, earliest safe convergence, lossless CSTs, green-tree-like reuse, and separate semantic dependency indexes are hypotheses to test—not architecture commitments.

## Architecture status

`docs/product/architecture.md` is intentionally a **HOLD document**.

It currently freezes only product-independent invariants such as:

- one Markdown source authority;
- Source and Live Mode cannot become two synchronized documents;
- heavy Mermaid/math rendering is outside the Markdown parser critical path;
- interactive presentation and full-document printing are different workloads;
- future plugins/providers must not depend directly on private parser/UI internals.

It intentionally does **not** freeze:

- AST vs CST;
- Tree-sitter vs Lezer-like vs custom parser;
- Rope vs Piece Table;
- block checkpoint format;
- semantic dependency representation;
- RenderPatch shape;
- GPUI or another UI backend architecture.

Those decisions wait for Issue #19.

## Product scope

The intended V0.1 still includes:

- Source Mode;
- Live Mode;
- file open/save;
- workspace file navigation and text search;
- split Source + Preview;
- Mermaid;
- LaTeX-style math;
- complete browser preview;
- browser Print / Save as PDF;
- Windows `.md` file association / Open With;
- local-first operation;
- extension-friendly semantic boundaries.

See `docs/PRD.md` and `docs/product/mvp-v0.1.md`.

## Print rule

Browser printing is a product requirement, but it does not decide the interactive parser/UI architecture.

The output invariant is:

```text
print immediately after opening
== semantic content ==
scroll through the entire document, then print
```

Offscreen text, Mermaid, math, images, and other required resources must not disappear merely because interactive UI never materialized them.

See `docs/product/print-browser-contract.md`.

## Existing code

Current code remains in the repository so experiments are reproducible and useful components can be evaluated.

For the new architecture, it is classified as:

```text
EXPERIMENTAL / REFERENCE
```

After the parser research, relevant components should explicitly receive one of:

```text
ADOPT
ADAPT
REPLACE
DELETE
```

Do not preserve an old mechanism merely because it already exists.

## Historical archive

The complete pre-reset repository is preserved at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

That revision is the historical archive for old ADRs, GPUI/PocketJS research, previous architecture, the old Markdown implementation, benchmark material, and previous implementation notes.

See `docs/archive/product-reset-2026-09-16/README.md`.

## Current documentation

Read in this order:

1. `docs/PRD.md` — product requirements;
2. Issue #19 + `docs/research/markdown-parser/README.md` — current parser research;
3. `docs/product/architecture.md` — architecture HOLD/invariants;
4. `docs/product/print-browser-contract.md` — browser/print completeness;
5. `docs/product/mvp-v0.1.md` — intended first shipping scope;
6. `docs/product/roadmap.md` — current sequencing.

The current working rule is:

> **Question -> experiment -> evidence -> verdict -> architecture -> implementation.**
