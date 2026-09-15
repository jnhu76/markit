# Markit

**Markit** is a local-first Markdown workspace editor with two first-class editing modes over one authoritative Markdown source:

- **Source Mode** — direct plain-text Markdown editing;
- **Live Mode** — source-aware WYSIWYG editing that still writes through the same Markdown source.

It also targets workspace search, split preview, Mermaid, LaTeX-style math, browser preview, and reliable browser Print/PDF.

> **Markdown Source is the single source of truth.**
>
> **Parse incrementally; publish progressively; print completely.**

## Product reset status

Markit is currently in **MARKIT-PRODUCT-RESET-0**.

The repository previously centered on architecture/performance experiments around GPUI/PocketJS, incremental Markdown, viewport rendering, and editor latency. Those experiments are preserved as evidence, but they no longer define the product.

The pre-reset authority boundary is repository revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

Existing code is treated as an **experimental/reference implementation** until each component earns reuse against the new product contracts.

See `docs/archive/product-reset-2026-09-16/README.md`.

## What Markit is for

The core workflow is deliberately small:

```text
Open Markdown file or workspace
        ↓
Edit in Source Mode or Live Mode
        ↓
Search across the workspace
        ↓
Preview beside source
        ↓
Render Mermaid + LaTeX math
        ↓
Open complete document in browser
        ↓
Browser Print / Save as PDF
```

### Source Mode

Source Mode edits the real Markdown text directly. It is the reliable escape hatch for every document and remains usable even if a rich renderer/plugin fails.

### Live Mode

Live Mode is editable rendered Markdown, but it is **not** an independent rich-text document that later serializes back to Markdown.

The model is:

```text
Markdown Source
  -> Markdown Semantics
  -> Live Projection
  -> user gesture / formatting command
  -> source-aware EditTransaction
  -> same Markdown Source
```

Switching Source <-> Live must not mutate the file by itself.

### Workspace search

A folder can be opened as a workspace. Markit provides file navigation and basic VS Code-like text search with file, line, matching span, context, and click-to-open/jump behavior.

The first implementation should prefer a direct scanner/search engine over a speculative permanent index database.

### Split Preview

Source Mode can be shown beside a read-only Preview. Preview and Live Mode share Markdown semantics but do not share mutable editor state.

### Mermaid

Fenced `mermaid` blocks are a first-class built-in rich projection. Heavy Mermaid rendering is revision-aware and stale-safe; it must not synchronously poison ordinary typing.

### LaTeX-style math

Inline `$...$` and display `$$...$$` math are first-class built-in projections. Math rendering must work in Preview, Live Mode where applicable, Browser Preview, and Print/PDF, with visible fallback for invalid expressions.

### Browser Preview and Print/PDF

Markit opens a complete local browser representation and delegates final pagination/PDF generation to the browser.

The print path intentionally follows different rules from interactive viewport rendering:

```text
Interactive: viewport-first, latency-first, progressive
Print:       full-document, completeness-first, completion barrier
```

Printing must not depend on whether the user scrolled to a region first. Offscreen Mermaid, math, images, and text are part of the Print Document before `PrintReady`.

See `docs/product/print-browser-contract.md`.

### OS integration

Windows is the first shipping target. V0.1 includes `.md` Open With/file-association integration and direct shell/path opening, including paths with spaces and Unicode/CJK characters.

## Architecture at a glance

```text
Workspace
   │ open
   ▼
Document  ← authoritative Markdown source
   │ ChangeSet
   ▼
Markdown Semantics
   │ SemanticDelta
   ├───────────────┬────────────────┐
   ▼               ▼                ▼
Source          Live             Preview
Projection      Projection       Projection
   │               │                │
   └───────────────┴────────────────┘
                   │
                 Desktop

Markdown Semantic Snapshot
          │
          ▼
 Browser / Print Projection
          │
          ▼
       HTML/CSS
          │
          ▼
   System Browser -> Print/PDF

Rich Projection Services: Mermaid / LaTeX math / later blocks
Plugin Boundary: semantic capabilities, never private authority
```

The architecture avoids a Source Document vs Live Document synchronization protocol. There is one source; every other representation is derived.

## Incremental / streaming rendering

Markit is an editor, so its rendering model must handle arbitrary insertion, deletion, and replacement — not only append-only token streams.

Conceptually:

```text
EditTransaction
  -> revision + ChangeSet
  -> incremental Markdown update
  -> SemanticDelta
  -> RenderPatch stream
  -> visible/current presentation first
```

Streaming Markdown projects are useful references for progressive publication and avoiding repeated full rebuilds. Markit generalizes that discipline to arbitrary document mutations.

Broad structural Markdown changes are allowed to propagate honestly. Work may be chunked/yielded, but semantics are not changed merely to manufacture a small invalidation radius.

## Plugin extensibility

Markit should remain extensible without making a plugin system the core editor.

Future providers/plugins operate through versioned semantic capabilities:

```text
snapshot/query -> plugin/provider -> result/command -> Markit validation
```

They do not receive mutable Document internals, GPUI entity identity, private Markdown IR memory layout, or scheduler/cache internals as their contract.

V0.1 does **not** require a marketplace or general third-party runtime. Built-in Mermaid, math, and browser/export workloads are used to keep the semantic seams clean so a runtime can be added later when real extension workloads justify it.

## Local-first

Core editing, workspace search, Preview, mandatory Mermaid/math rendering, Browser Preview preparation, and Print/PDF preparation work without a cloud account and without uploading document contents to a remote service.

## Documentation authority

Read current product documents in this order:

1. `docs/PRD.md` — product requirements and product laws;
2. `docs/product/architecture.md` — ownership and rendering architecture;
3. `docs/product/print-browser-contract.md` — browser/print completeness rules;
4. `docs/product/mvp-v0.1.md` — first shipping scope and acceptance gates;
5. `docs/product/roadmap.md` — implementation order and stop conditions.

`README.md` is an entry point, not an independent source of truth.

Pre-reset research/benchmark/ADR/product documents remain useful evidence, but if they conflict with the authority above they are historical until explicitly re-adopted.

## V0.1 target

V0.1 is complete when a Windows user can:

- open `.md` directly or through Open With;
- open a workspace and search it;
- edit reliably in Source Mode;
- edit the same source in Live Mode;
- switch modes without source mutation or divergent undo/revision state;
- use Source + Preview split view;
- render Mermaid and LaTeX-style math;
- open a complete browser representation;
- Print/Save PDF without viewport/lazy-render omissions;
- keep working locally when offline;
- do all of this without freezing private implementation details into future plugin contracts.

## Not the goal for V0.1

Markit is not trying to become VS Code, Notion, a cloud collaboration suite, or a PDF layout engine.

The first product does not require:

- cloud sync/accounts;
- AI assistant;
- Git GUI;
- terminal/debugger/LSP IDE;
- plugin marketplace;
- arbitrary TeX execution;
- DOCX export;
- an independent PDF engine.

The product should earn additional complexity from real workflows rather than from framework ambition.