# Markit Architecture — HOLD pending benchmark-first research chain

Status: **INTENTIONALLY UNFROZEN**

Authority gate: **#22 MARKIT-MARKDOWN-BENCHMARK-1** — the standardized
Markdown AST/CST update benchmark (the only active Markdown research
campaign). Architecture synthesis may not start before that chain completes
(see "Architecture gate" below).

This file deliberately does **not** define the implementation architecture.

Markit has **no production implementation in the active tree**. The old
implementation and Experiment 0 are historical evidence (Experiment 0 is
archived at `research/experiments/experiment-0-parser-survey/`), not
architectural authority for the new product.

The next architecture revision MUST be written from evidence produced by the
benchmark-first chain, not by preserving any removed implementation by
default.

## Why architecture is on hold

The central unresolved mechanism is the Markdown editing engine:

> How should Markit update Markdown syntax/semantic state under arbitrary
> edits with minimal unnecessary reparse, tree reconstruction, memory
> movement, and downstream invalidation — at verified correctness?

The sequencing to answer it was corrected after Experiment 0:

```text
#19 / PR #20  Experiment 0 (archived evidence)
        ↓
#22           standardized benchmark over existing parsers/update designs
        ↓
              Weakness Map (what is slow, and why — attributed)
        ↓
              Markit-specific algorithm campaign (only for earned weaknesses)
        ↓
              formal/correctness work (as applicable)
        ↓
              architecture synthesis
```

Issue #21 (MARKIT-MARKDOWN-ARCHITECTURE-1), which tried to skip from
Experiment 0 to architecture synthesis, is **CLOSED / SUPERSEDED** and must
not be used as a gate.

Markit does not yet freeze choices such as:

- whole-document tree vs block-local representation;
- AST vs lossless CST vs hybrid/event representation;
- Tree-sitter, Lezer-like, MD4C-like, pulldown/Comrak-like, or Markit-specific parsing;
- parser checkpoint/state-summary format;
- reusable-region and convergence rules;
- source storage structure (rope, piece table, other);
- position/index representation and offset-maintenance strategy;
- syntax dependency indexes;
- SemanticDelta shape;
- parser-to-render invalidation contract;
- rendering scheduler or UI backend architecture.

Any document or archived code that currently implies one of those choices is
evidence, not authority.

---

## Frozen product-level constraints

These constraints do not depend on parser research and remain authoritative.

### A1 — Markdown source is the document truth

```text
Markdown Source = authoritative document content
```

Source Mode, Live Mode, Preview, Browser output, Print, Mermaid, math, search
results, caches, syntax trees, and render artifacts are derived from that
source.

No rendered or rich-text representation may become a second independent
document authority.

### A2 — Two first-class editing modes

Markit must support:

- **Source Mode** — direct lossless Markdown source editing;
- **Live Mode** — source-aware rendered editing that writes through to the
  same Markdown source.

Switching modes must not itself rewrite the document.

The mechanism for Live Mode is intentionally deferred until the Markdown
representation is understood.

### A3 — One Markdown semantic authority

Different projections may exist, but they must not independently reinterpret
Markdown with incompatible semantics.

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

Mermaid rendering, LaTeX/math rendering, syntax highlighting engines, browser
generation, layout, shaping, and GPU/UI rendering are not part of the Markdown
parse critical path.

The parser may identify their syntax and dependencies; it must not
synchronously perform their heavy visual work.

### A5 — Interactive work and print completeness are different workloads

Interactive presentation may be viewport-aware, incremental, cancellable, or
progressive.

Printing is full-document and completeness-first. See
`print-browser-contract.md`.

```text
interactive: latency-first / partial publication allowed
print:       completeness-first / completion barrier required
```

### A6 — Workspace does not own document truth

Workspace functionality may discover files, enumerate them, and search them.
An open document/buffer owns unsaved in-memory edits for that document.

Workspace search and filesystem state must not silently overwrite an active
document authority.

### A7 — Extension boundaries must remain possible

Markit should remain extensible through stable semantic/query/command/provider
boundaries rather than requiring plugins to mutate private parser or UI
internals.

No general plugin runtime is selected yet.

Mermaid and math are useful built-in workloads for testing whether future
extension seams are clean, but their current implementation must not dictate
the parser architecture.

### A8 — Removed code has no residual authority

The pre-reset implementation and the Experiment 0 harness were removed from
the active tree / archived (see `docs/research/repo-reset-inventory.md`).

They are classified only as:

```text
HISTORICAL EVIDENCE / GIT HISTORY
```

Nothing deleted from the active tree regains authority by having existed.
Any future reuse must be re-earned explicitly as:

```text
ADOPT / ADAPT / REPLACE / BUILD_NEW
```

in the architecture phase, from #22-chain evidence.

---

## Current research boundary

The active technical problem is intentionally narrower than the full editor:

```text
same payload + same edit
        |
        v
normalized benchmark across existing Markdown update designs
        |
        v
correct results + measured work amplification
        |
        v
Weakness Map (common vs Markit-specific opportunities)
```

UI is not the research target.

The #22 benchmark must record more than wall-clock latency, including where
practical:

- bytes/lines rescanned;
- nodes rebuilt / reused;
- changed source coverage;
- allocations and allocated bytes;
- offset/index maintenance work;
- propagation distance;
- semantic dependency fan-out;
- equality with a clean authoritative parse (hard gate).

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

This HOLD ends only through the benchmark-first chain:

```text
1. #22 benchmark runs on the fixed baseline set with the correctness oracle
        |
        v
2. Weakness Map reviewed by humans
        |
        v
3. Markit algorithm campaign (only for weaknesses worth solving)
        |
        v
4. formal / correctness review of the proposed algorithm (as applicable)
        |
        v
5. architecture synthesis (replaces this document)
```

Skipping steps 1–4 is how #21 went superseded; do not repeat it.

Only the architecture phase should decide:

```text
Document storage
      -> incremental parser
      -> syntax/semantic representation
      -> dependency tracking
      -> downstream invalidation contract
      -> projection/render architecture
      -> UI backend integration
```

The order matters. Rendering and UI should consume the parser/semantic
contract; they should not dictate it prematurely.

---

## Current authority order

During this research phase:

```text
docs/PRD.md + docs/product/**
    = product requirements

Issue #22 (MARKIT-MARKDOWN-BENCHMARK-1)
    = the ONLY active Markdown parser research authority

research/experiments/experiment-0-parser-survey/ (#19 / PR #20)
    = archived Experiment 0 — historical evidence only

Issue #21 (MARKIT-MARKDOWN-ARCHITECTURE-1)
    = SUPERSEDED / CLOSED — not a gate

docs/product/print-browser-contract.md
    = print/browser completeness contract

docs/product/mvp-v0.1.md
    = intended product scope, not implementation architecture

docs/product/roadmap.md
    = sequencing/status

this file
    = architecture HOLD / invariant boundary only

Git history
    = archive of all removed implementations and documents
```

## Exit condition

This HOLD lifts when the #22 chain (benchmark → Weakness Map review → Markit
algorithm campaign → formal/correctness review) completes and the resulting
architecture synthesis passes human review.

At that point, replace this document with an evidence-backed architecture
rather than incrementally layering assumptions onto this placeholder.
