# Markit Architecture — HOLD pending parser research

Status: **INTENTIONALLY UNFROZEN**

Authority gate: [Issue #19 — Incremental Markdown parsing: locality, convergence, and invalidation](https://github.com/jnhu76/markit/issues/19)

This file deliberately does **not** define the implementation architecture yet.

Markit is currently in a parser-research phase. The pre-reset implementation and architecture are retained as historical/experimental evidence, but they are not architectural authority for the new product.

The next architecture revision MUST be written from evidence produced by Issue #19, not by preserving the current implementation by default.

## Why architecture is on hold

The central unresolved mechanism is the Markdown editing engine:

> For lossless Markdown editing under arbitrary edits, how should Markit minimize reparse radius, tree reconstruction, memory movement, and downstream invalidation while preserving correctness?

Issue #19 is explicitly allowed to refine or replace that question when evidence shows that the current formulation is incomplete.

Until that experiment closes, Markit does not yet know enough to freeze choices such as:

- whole-document tree vs block-local representation;
- AST vs lossless CST vs hybrid/event representation;
- Tree-sitter, Lezer-like fragments, MD4C-like full parsing, or Markit-specific parsing;
- parser checkpoint/state-summary format;
- reusable-region and convergence rules;
- source storage structure (rope, piece table, other);
- position/index representation and offset-maintenance strategy;
- syntax dependency indexes;
- SemanticDelta shape;
- parser-to-render invalidation contract;
- rendering scheduler or UI backend architecture.

Any document or code that currently implies one of those choices is evidence, not authority.

---

## Frozen product-level constraints

These constraints do not depend on the parser experiment and remain authoritative.

### A1 — Markdown source is the document truth

```text
Markdown Source = authoritative document content
```

Source Mode, Live Mode, Preview, Browser output, Print, Mermaid, math, search results, caches, syntax trees, and render artifacts are derived from that source.

No rendered or rich-text representation may become a second independent document authority.

### A2 — Two first-class editing modes

Markit must support:

- **Source Mode** — direct lossless Markdown source editing;
- **Live Mode** — source-aware rendered editing that writes through to the same Markdown source.

Switching modes must not itself rewrite the document.

The mechanism for Live Mode is intentionally deferred until the Markdown representation is understood.

### A3 — One Markdown semantic authority

Different projections may exist, but they must not independently reinterpret Markdown with incompatible semantics.

Conceptually:

```text
Markdown Source
      |
      v
Markdown semantic authority
   /      |       \
Source   Live    Preview/Print
```

The exact representation behind the semantic authority is NOT frozen.

### A4 — Parser critical path excludes heavy rendering

Mermaid rendering, LaTeX/math rendering, syntax highlighting engines, browser generation, layout, shaping, and GPU/UI rendering are not part of the Markdown parse critical path.

The parser may identify their syntax and dependencies; it must not synchronously perform their heavy visual work.

### A5 — Interactive work and print completeness are different workloads

Interactive presentation may be viewport-aware, incremental, cancellable, or progressive.

Printing is full-document and completeness-first. See `print-browser-contract.md`.

```text
interactive: latency-first / partial publication allowed
print:       completeness-first / completion barrier required
```

### A6 — Workspace does not own document truth

Workspace functionality may discover files, enumerate them, and search them. An open document/buffer owns unsaved in-memory edits for that document.

Workspace search and filesystem state must not silently overwrite an active document authority.

### A7 — Extension boundaries must remain possible

Markit should remain extensible through stable semantic/query/command/provider boundaries rather than requiring plugins to mutate private parser or UI internals.

No general plugin runtime is selected yet.

Mermaid and math are useful built-in workloads for testing whether future extension seams are clean, but their current implementation must not dictate the parser architecture.

### A8 — Existing code must re-earn reuse

The current repository contains useful experiments, including document/change handling, line indexing, incremental Markdown work, GPUI probes, and benchmarks.

They are classified only as:

```text
EXPERIMENTAL / REFERENCE
```

After Issue #19, each relevant component must receive one of:

```text
ADOPT
ADAPT
REPLACE
DELETE
```

Prior implementation is not evidence of architectural necessity.

---

## Current research boundary

The active technical problem is intentionally narrower than the full editor:

```text
arbitrary source edit
        |
        v
Markdown parsing / reuse / convergence
        |
        v
correct incremental syntax + semantic change information
```

UI is not the research target yet.

The parser experiment should measure more than wall-clock latency, including where practical:

- bytes/lines rescanned;
- blocks or syntax regions reparsed;
- syntax nodes rebuilt/reused;
- allocations and allocated bytes;
- offset/index maintenance work;
- propagation distance;
- semantic dependency fan-out;
- equality with a clean authoritative parse.

The research must also distinguish:

```text
syntax invalidation
!=
semantic dependency invalidation
!=
downstream presentation invalidation
```

---

## Architecture gate

A new architecture document may be frozen only after Issue #19 produces a reviewed verdict.

Minimum required evidence before architecture freeze:

1. a mechanism survey covering the major comparison families;
2. a reproducible mutation corpus;
3. correctness comparison against clean parsing where applicable;
4. locality / propagation measurements;
5. representation and memory-movement evidence;
6. a verdict on the current research question itself (`KEEP`, `REFINE`, or `REPLACE`);
7. a parser-direction verdict.

Only then should architecture decide:

```text
Document storage
      -> incremental parser
      -> syntax/semantic representation
      -> dependency tracking
      -> downstream invalidation contract
      -> projection/render architecture
      -> UI backend integration
```

The order matters. Rendering and UI should consume the parser/semantic contract; they should not dictate it prematurely.

---

## Current authority order

During this research phase:

```text
docs/PRD.md
    = product requirements

Issue #19 + its generated evidence
    = Markdown parser research authority

docs/product/print-browser-contract.md
    = print/browser completeness contract

docs/product/mvp-v0.1.md
    = intended product scope, not implementation architecture

docs/product/roadmap.md
    = sequencing/status

this file
    = architecture HOLD / invariant boundary only

pre-reset docs + current implementation
    = archive/reference evidence
```

## Exit condition

This HOLD ends only after human review of the Issue #19 final report.

At that point, replace this document with an evidence-backed architecture rather than incrementally layering assumptions onto this placeholder.
