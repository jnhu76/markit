# Markit Product Requirements

Status: **post-reset product authority**  
Reset: **MARKIT-PRODUCT-RESET-0 — 2026-09-16**

Markit is a local-first Markdown workspace editor. It exists to make ordinary Markdown work simple and reliable: open a file or workspace, edit the real Markdown source, search across the workspace, preview the document, render Mermaid and LaTeX math, and hand a complete browser document to the system browser for printing/PDF.

The pre-reset implementation and research documents are preserved as experimental evidence at repository revision `d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e`. They do not define the product after this reset.

## 1. Product statement

Markit has two first-class editing modes over **one authoritative Markdown source**:

1. **Source Mode** — direct plain-text Markdown editing. Syntax is visible and the file contents are edited directly.
2. **Live Mode** — source-aware rendered editing. Markdown presentation is editable, but all edits resolve back to the same Markdown source through explicit edit transactions. Live Mode is not a second rich-text document.

A separate read-only Preview can be shown beside Source Mode. Browser Preview and Print/PDF are output surfaces, not alternate document authorities.

The core product rule is:

> **Markdown Source is the single source of truth.**

The rendering rule is:

> **Parse incrementally; publish progressively; print completely.**

## 2. Original core requirements

The original Markit requirements remain mandatory and are the base of this reset.

### R1 — Plain-text Markdown editing

Markit MUST provide a complete Source Mode in which the user edits Markdown as plain text.

Acceptance intent:

- opening a `.md` file exposes its real source bytes/text, not a regenerated approximation;
- save does not rewrite unrelated syntax merely because a document was opened or previewed;
- Source Mode remains usable even if rich rendering, Mermaid, LaTeX, or a plugin fails;
- normal editor operations exist: caret, selection, navigation, copy/cut/paste, undo/redo, find, open/save/save-as;
- UTF-8, CJK, emoji, and IME input are first-class correctness requirements.

### R2 — Operating-system open integration

On Windows, installation/registration MUST support opening Markdown files through the operating system, including an **“Open with Markit”** path for `.md` files.

At minimum validate:

- `.md` file association / Open With registration;
- shell invocation with one or more file paths where supported;
- paths containing spaces, Unicode, and CJK characters;
- invocation when Markit is already running;
- no path reinterpretation caused by treating URLs as filesystem paths.

Other platforms may use their native equivalents later without changing document semantics.

### R3 — Workspace search

Markit MUST be able to open a folder as a workspace and search text across that workspace in a workflow comparable to VS Code’s basic text search.

V1 search results MUST identify at least:

- file path;
- line number;
- matching span;
- a short context/snippet;
- click-to-open/jump behavior.

A permanent index database is **not** required by default. The first implementation should prefer the smallest correct scanner/search mechanism and add indexing only when measured workloads justify it.

### R4 — Split preview

Source Mode MUST support a left/right split with a read-only Markdown Preview.

The preview:

- consumes the same document revision and Markdown semantics as other projections;
- must not parse an independent dialect of the document;
- may update incrementally and prioritize visible content;
- may fail a rich block visibly without making Source Mode unusable.

### R5 — Open in browser for print/PDF

Markit MUST be able to open the current document in the user’s system browser so the browser’s print function can produce paper output or PDF.

Markit does **not** need an independent PDF engine for V1.

Browser output must obey `docs/product/print-browser-contract.md`, especially:

- complete-document materialization for printing;
- output independent of editor scroll/viewport history;
- a coherent pinned document revision;
- Mermaid, LaTeX math, local images, fonts, and other required resources reach a terminal state before `PrintReady`;
- failed rich content is shown as an explicit fallback/error rather than silently disappearing;
- print CSS is a product-owned contract, not an accidental browser default.

### R6 — Mermaid

Mermaid is a mandatory built-in Markdown capability.

A fenced block such as:

````markdown
```mermaid
graph TD
  A --> B
```
````

must render in Live Mode where applicable, Preview, Browser Preview, and Print/PDF.

Mermaid rendering is a heavy/rich projection. It MUST NOT become synchronous per-keystroke work that blocks ordinary typing. Results are tied to block identity + document revision and stale results cannot overwrite newer content.

The initial provider may use Mermaid.js, but Mermaid.js itself is not the document authority or plugin ABI.

## 3. Added product requirements

### R7 — Live Mode

Markit MUST provide a source-aware Live Mode in addition to Source Mode.

Live Mode requirements:

- the Markdown source remains authoritative;
- switching Source Mode ↔ Live Mode does not serialize one document model into another;
- mode switching alone must not mutate the file;
- both modes share document revision, dirty state, undo history, and command semantics;
- visual selections/caret positions map explicitly through semantic/source coordinates;
- formatting actions produce Markdown-aware edit commands/transactions;
- syntax may be visually hidden when safe, but the user must always have a path to the underlying source;
- unsupported/ambiguous constructs degrade to source-visible editing rather than destructive normalization.

This is **source-aware WYSIWYG**, not a generic rich-text editor that later guesses Markdown.

### R8 — LaTeX-style mathematics

LaTeX-style math is a mandatory built-in rich projection.

V1 MUST support at least:

- inline math using `$...$`;
- display math using `$$...$$`;
- rendering in Live Mode where applicable, Preview, Browser Preview, and Print/PDF;
- explicit visible fallback on invalid expressions;
- CJK text surrounding math without corrupting layout or source offsets.

This requirement is for mathematical TeX/LaTeX syntax, not arbitrary TeX document execution.

A built-in provider may use KaTeX or another suitable renderer, but the renderer implementation is replaceable behind a semantic projection boundary.

### R9 — Incremental / streaming rendering

Interactive rendering MUST be designed for arbitrary editor mutations, not only append-only streams.

The intended flow is conceptual, not a frozen ABI:

```text
EditTransaction
  -> Document revision + ChangeSet
  -> incremental Markdown semantics
  -> SemanticDelta
  -> RenderPatch stream
  -> Source / Live / Preview projections
```

For ordinary local edits, unchanged document regions must not be reparsed/rebuilt merely because the document is large.

For expensive or broad changes:

- work may be chunked and progressively published;
- current interaction and visible content outrank distant presentation work;
- stale results are cancelled or rejected;
- broad structural Markdown effects must be represented honestly rather than hidden behind a false local invalidation rule;
- no permanent fixed-rate render loop is required.

Streaming Markdown projects are references for incremental publication discipline, not proof that an append-tail parser is sufficient for an editor.

### R10 — Plugin-extensible product boundaries

Markit MUST remain extensible without making plugins part of the document authority.

V1 does not require a marketplace or a fully general third-party runtime. It DOES require that product boundaries do not prevent later plugin implementations.

Extension-facing rules:

- plugins/providers consume versioned semantic snapshots or explicit queries;
- mutations return as commands/transactions, never direct mutable document access;
- plugin identity must not become document/block/source identity;
- plugins do not depend on GPUI entity identity, private parser memory layout, scheduler internals, or cache layout;
- slow/crashed extension work cannot indefinitely block ordinary text editing;
- results carry enough revision/identity information to reject stale output;
- capabilities are explicit and can be versioned.

Built-in Mermaid, math, browser/export, and future rich-block facilities SHOULD be shaped so they could later be implemented/replaced through these same semantic provider seams without forcing V1 to build the full plugin runtime first.

## 4. Product domains

Markit has six responsibility domains. These are ownership boundaries, not a requirement to create six frameworks.

```text
Workspace
  files / roots / search / path discovery

Document
  source / revision / transactions / selection / undo / dirty state

Markdown Semantics
  blocks / inline semantics / source spans / semantic identity

Projection
  Source / Live / Preview / Browser-Print representations

Rich Projection Services
  Mermaid / LaTeX math / later heavy blocks

Desktop Host
  window / OS open / file association / clipboard / IME / browser launch
```

A future Plugin Boundary sits beside these domains and receives explicit semantic capabilities. It is not a back door into their internals.

## 5. Key user flows

### File editing

```text
OS / Markit Open
  -> Document Source
  -> Source Mode or Live Mode
  -> EditTransaction
  -> Save
```

### Workspace editing

```text
Open Folder
  -> Workspace tree/search
  -> select result/file
  -> Document
  -> Source or Live editing
```

### Source + Preview

```text
Source Mode | Preview
     same Document revision
     same Markdown semantics
```

### Browser / PDF

```text
DocumentSnapshot(revision N)
  -> whole-document Browser/Print representation
  -> await required rich resources
  -> PrintReady(revision N)
  -> system browser
  -> browser Print / Save as PDF
```

## 6. Rendering correctness laws

The following are product laws:

1. **One source truth** — no Source document vs Live document synchronization protocol.
2. **No silent normalization** — projections do not rewrite source simply by observing it.
3. **Revisioned derived work** — every expensive derived result proves which source state produced it.
4. **Interactive work is incremental** — stable unaffected work is reused where semantics allow.
5. **Publication is progressive but coherent** — partial readiness must not publish internally incompatible state as if complete.
6. **Print is exhaustive** — print correctness is not bounded by the interactive viewport.
7. **Rich failure is visible** — Mermaid/math/image failure becomes an error/fallback block, not missing content.
8. **Source Mode survives projection failure** — editing the file is more fundamental than rendering it.

## 7. Local-first behavior

Core editing, workspace search, Markdown rendering, Mermaid/math rendering required for normal use, Browser Preview generation, and printing MUST work without a cloud account and without sending document contents to a remote service.

Network-backed plugins may exist later only through explicit capabilities and user-visible policy.

## 8. V1 non-goals

The following are not required for the first product release unless separately justified:

- cloud sync or collaboration;
- account system;
- AI assistant;
- Git GUI;
- terminal/debugger/LSP IDE features;
- plugin marketplace;
- a general plugin runtime before real provider workloads require it;
- an independent PDF layout/serialization engine;
- DOCX export;
- Notion-style database/block workspace semantics;
- arbitrary TeX execution;
- silently importing every Obsidian/Typora/GFM extension.

## 9. Product authority and experimental reuse

The current implementation predates this reset and is an experimental/reference implementation. Existing Document, revision, parser, GPUI, viewport, IME, benchmark, and scheduling work may be valuable, but each component must be evaluated against this PRD and the post-reset architecture before becoming product authority.

Do not preserve an old abstraction merely because code already exists.

Do not discard a proven component merely because it came from an experiment.

Reuse must be earned by semantic fit, correctness evidence, and measured product value.

## 10. V1 definition

Markit V1 is complete only when a user can, on the primary Windows target:

- open a Markdown file directly or through OS Open With;
- open a workspace and search across it;
- edit reliably in Source Mode;
- edit the same source in Live Mode;
- switch modes without source mutation or state divergence;
- use split Source + Preview;
- render Mermaid and LaTeX-style math;
- open a complete browser representation;
- print/save PDF without viewport/lazy-render omissions;
- recover visibly from rich-render failures;
- perform all core workflows locally;
- do all of the above without product APIs that make future semantic plugins depend on private implementation details.