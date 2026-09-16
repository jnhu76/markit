# Markit Product Requirements

Status: **post-reset product authority**  
Reset: **MARKIT-PRODUCT-RESET-0 — 2026-09-16**

Markit is a local-first Markdown editor/workspace. It exists to make ordinary Markdown work simple, fast, and reliable: open a file or workspace, edit the real Markdown source, search across the workspace, preview the document, render Mermaid and LaTeX-style math, and hand a complete browser document to the system browser for printing/PDF.

The pre-reset implementation and research material is preserved at repository revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

It is historical/experimental evidence, not current product or architecture authority.

## 1. Product statement

Markit has two first-class editing modes over **one authoritative Markdown source**:

1. **Source Mode** — direct plain-text Markdown editing. Syntax is visible and the real source is edited directly.
2. **Live Mode** — source-aware rendered editing. User actions still modify the same Markdown source; Live Mode is not an independent rich-text document later serialized back to Markdown.

A read-only Preview can be shown beside Source Mode. Browser Preview and Print/PDF are output surfaces, not alternate document authorities.

The core product law is:

> **Markdown Source is the single source of truth.**

The performance/output law is intentionally mechanism-neutral:

> **Avoid unnecessary work on interactive edits; publish visible results responsively; print the complete document.**

Exactly how Markdown parsing, reuse, storage, syntax representation, and invalidation achieve this is **not** specified by the PRD. That is the subject of the benchmark-first research chain (issue #22 and what follows it).

---

## 2. Core requirements

### R1 — Plain-text Markdown editing

Markit MUST provide a complete Source Mode in which the user edits Markdown as plain text.

Acceptance intent:

- opening a `.md` file exposes its real source rather than a regenerated approximation;
- save does not rewrite unrelated syntax merely because a document was opened or previewed;
- Source Mode remains usable if rich rendering, Mermaid, math, or a plugin/provider fails;
- normal editor operations exist: caret, selection, navigation, copy/cut/paste, undo/redo, find, open/save/save-as;
- UTF-8, CJK, emoji, and IME input are first-class correctness requirements.

### R2 — Live Mode

Markit MUST provide a source-aware Live Mode in addition to Source Mode.

Live Mode requirements:

- the same Markdown source remains authoritative;
- switching Source Mode ↔ Live Mode alone does not mutate the file;
- the two modes do not maintain independently authoritative documents that require synchronization;
- formatting/editing actions produce source changes with predictable undo/dirty behavior;
- unsupported or ambiguous syntax degrades visibly and remains recoverable in Source Mode;
- source fidelity is preferred over destructive normalization.

The exact source/syntax/visual mapping architecture is deferred until parser research is complete.

### R3 — Operating-system open integration

On Windows, installation/registration MUST support opening Markdown files through the operating system, including **Open with Markit** for `.md` files.

Validate at minimum:

- `.md` file association / Open With;
- shell invocation with file paths;
- paths containing spaces, Unicode, and CJK characters;
- invocation when Markit is already running;
- filesystem paths are not incorrectly treated as URLs.

### R4 — Workspace search

Markit MUST be able to open a folder as a workspace and search text across it in a workflow comparable to basic VS Code text search.

V0.1 search results MUST provide at least:

- file path;
- line number;
- matching span;
- short context/snippet;
- click-to-open/jump behavior.

A permanent search-index database is not required by the product specification. Use one only if later evidence justifies it.

### R5 — Split Preview

Source Mode MUST support a left/right split with a read-only Markdown Preview.

Preview must:

- represent the same source revision as the editor;
- use the same Markdown interpretation as other Markit projections;
- update responsively after edits;
- allow rich-block failure to be visible without making Source Mode unusable.

The PRD does not prescribe the incremental parser or rendering mechanism used to meet that requirement.

### R6 — Browser Preview and Print/PDF

Markit MUST be able to open a complete representation of the current document in the system browser so the browser can Print or Save as PDF.

Markit does **not** require its own PDF engine for V0.1.

Browser output must obey `docs/product/print-browser-contract.md`, including:

- complete-document materialization for printing;
- output independent of editor scroll/viewport history;
- one coherent document revision;
- required Mermaid, math, images, fonts, and other resources reach an explicit terminal state before `PrintReady`;
- failed rich content is represented visibly rather than silently omitted.

### R7 — Mermaid

Mermaid is a mandatory built-in Markdown capability.

A fenced block such as:

````markdown
```mermaid
graph TD
  A --> B
```
````

must render in Preview, Browser Preview, Print/PDF, and Live Mode where applicable.

Mermaid rendering MUST NOT be required synchronous work for every ordinary text keystroke.

The implementation may use Mermaid.js or another compatible renderer, but that renderer is not document authority and must remain replaceable.

### R8 — LaTeX-style mathematics

V0.1 MUST support mathematical markup including at least:

- inline `$...$`;
- display `$$...$$`;
- rendering in Preview;
- rendering in Browser Preview / Print;
- Live Mode presentation where applicable;
- explicit visible fallback on invalid expressions;
- CJK text around math without corrupting source behavior.

This is mathematical TeX/LaTeX-style syntax, not arbitrary TeX document execution.

---

## 3. Performance requirements

### R9 — Responsive arbitrary editing

Markit MUST remain responsive under arbitrary insertion, deletion, replacement, and paste—not merely append-only streams.

For ordinary local edits, work unrelated to the changed semantics should not be repeated merely because the document is large.

For edits whose Markdown meaning genuinely propagates farther, Markit may perform broader work. Correctness is more important than manufacturing an artificially small invalidation radius.

The implementation is intentionally undecided pending the #22 benchmark chain.

Possible mechanisms under research include existing incremental parsers, reusable syntax fragments/trees, small-region parsing, state checkpoints/convergence, full-parse baselines, and Markit-specific approaches. None is a product requirement.

The product-level success condition is observable:

- normal typing remains responsive on supported workloads;
- large-document size does not cause unnecessary work amplification for genuinely local edits;
- arbitrary structural edits converge to correct Markdown interpretation;
- no performance optimization silently changes document semantics.

### R10 — Correctness before benchmark wins

A faster incremental result that differs from the authoritative clean interpretation of the supported Markdown dialect is a failure.

A mechanism that appears local but performs hidden linear metadata/copy work is not proven scalable merely because parser time is low.

The research/architecture phase must therefore consider total work, not only parser wall time.

---

## 4. Extension requirement

### R11 — Plugin/provider extensibility

Markit MUST remain extensible, but V0.1 does not require a plugin marketplace or a fully general third-party runtime.

Future extension design must be able to avoid making private parser/UI implementation details into public authority.

At a product level, future extensions should be able to:

- read appropriate document/semantic information through explicit capabilities;
- request controlled edits/actions;
- return results that can be rejected if stale;
- fail without destroying basic source editing.

The concrete plugin API, ABI, transport, runtime, snapshot type, semantic node identity, and process model are deferred until the core Markdown architecture is known.

Mermaid and math are useful built-in workloads for testing whether future boundaries remain replaceable.

---

## 5. Local-first requirement

Core editing, workspace search, Markdown interpretation, required Mermaid/math rendering, Browser Preview preparation, and Print/PDF preparation MUST work without a cloud account and without uploading document contents to a remote service.

Network-backed extensions may exist later only through explicit product policy/capabilities.

---

## 6. Product truth vs architecture

This PRD defines **what Markit must do**, not the internal mechanism used to do it.

The following are deliberately **not frozen here**:

```text
Rope vs Piece Table vs another source store
AST vs CST vs event/hybrid representation
Tree-sitter vs Lezer-like vs MD4C-like vs custom parsing
block-local vs subtree/fragment reuse
parser checkpoint/state format
absolute vs relative position/index structures
SemanticDelta / RenderPatch concrete types
scheduler / worker topology
GPUI vs another future UI mechanism
plugin runtime / ABI / transport
```

These choices must be earned through the benchmark-first research chain and the architecture phase.

Current architecture status is documented in `docs/product/architecture.md` and is intentionally `HOLD` until the #22 benchmark chain (benchmark -> Weakness Map review -> Markit algorithm campaign -> formal/correctness review) completes.

---

## 7. V0.1 non-goals

The first release does not require:

- cloud sync or collaboration;
- account system;
- AI assistant;
- Git GUI;
- terminal/debugger/LSP IDE features;
- plugin marketplace;
- a general plugin runtime before real workloads justify one;
- an independent PDF engine;
- DOCX export;
- Notion-style database/workspace semantics;
- arbitrary TeX execution;
- every Markdown extension used by every other editor.

---

## 8. V0.1 definition

Markit V0.1 is complete only when a user on the primary Windows target can:

- open a Markdown file directly or through OS Open With;
- open a workspace and search across it;
- edit reliably in Source Mode;
- edit the same source through Live Mode;
- switch modes without source mutation or divergent document authority;
- use Source + Preview split view;
- render Mermaid and LaTeX-style math;
- open a complete browser representation;
- Print/Save PDF without viewport/lazy-materialization omissions;
- recover visibly from rich-render failures;
- perform all core workflows locally;
- retain a credible path to future extension mechanisms without exposing private implementation details as permanent API.

The implementation path to this product is deliberately postponed until Markit has experimental evidence about the Markdown parsing problem.
