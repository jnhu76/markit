# Markit Product Architecture

Status: **post-reset product architecture**  
Authority: subordinate to `docs/PRD.md`  
Reset: **MARKIT-PRODUCT-RESET-0 — 2026-09-16**

The pre-reset implementation is an experimental/reference implementation. This document defines the architecture required by the product; existing code is reused only where it conforms.

Markit is built around one rule:

> **Markdown Source is the single source of truth.**

And one rendering rule:

> **Parse incrementally; publish progressively; print completely.**

## 1. Architecture overview

```text
                              Markit

                     ┌──────────────────┐
                     │    Workspace     │
                     │ files / search   │
                     └────────┬─────────┘
                              │ open
                              ▼
                     ┌──────────────────┐
                     │     Document     │
                     │ SOURCE AUTHORITY │
                     │ revision / edits │
                     │ selection / undo │
                     └────────┬─────────┘
                              │ ChangeSet
                              ▼
                 ┌──────────────────────────┐
                 │   Markdown Semantics     │
                 │ blocks / inline / spans  │
                 │ semantic identity        │
                 └───────────┬──────────────┘
                             │ SemanticDelta
                ┌────────────┼───────────────┐
                │            │               │
                ▼            ▼               ▼
         Source Projection  Live Projection  Preview Projection
                │            │               │
                └────────────┼───────────────┘
                             │ interactive presentation
                             ▼
                           GPUI

                 Markdown Semantics / snapshot
                             │
                             ▼
                  Browser / Print Projection
                             │
                             ▼
                          HTML/CSS
                             │
                             ▼
                       System Browser
                             │
                             ▼
                        Print / PDF

        Rich Projection Services: Mermaid / LaTeX math / future blocks
        Desktop Host: OS open / file association / IME / clipboard / browser launch
        Plugin Boundary: versioned semantic capabilities beside, never inside, authority
```

The boxes are responsibility domains. They do not require a framework class or crate for every box.

## 2. Authority map

### 2.1 Workspace owns discovery, not documents

Workspace owns:

- workspace root(s) and path discovery;
- file enumeration/tree presentation;
- text search across files;
- search result coordinates and open/jump requests.

Workspace does **not** own an editor buffer and does not mutate a `Document` behind the editor’s back.

```text
Workspace result(path, line, match)
        -> OpenDocument(path)
        -> Document authority
```

V1 should avoid a permanent search index unless measured workloads justify one. A direct scanner/ripgrep-like mechanism is preferable to inventing a second content database.

### 2.2 Document owns source truth

Document owns:

- Markdown source text/bytes;
- document identity;
- revision identity;
- edit transactions;
- selection/caret logical state where framework-independent;
- undo/redo transaction history;
- dirty/save state;
- coherent immutable/query snapshots.

There is exactly one authoritative source for a file in an editing session.

No projection is allowed to become a second document authority.

### 2.3 Markdown Semantics owns interpretation

Markdown Semantics consumes a Document snapshot/change and owns the framework-independent interpretation required by product projections:

- block structure;
- inline semantics;
- source spans;
- stable-enough semantic identities for incremental reuse;
- recognized extension blocks such as Mermaid;
- recognized math spans/blocks;
- explicit parse diagnostics.

The concrete IR layout is private. No GPUI entity, DOM node, JavaScript object, plugin object, or renderer handle is semantic identity.

The exact Markdown dialect must be explicit and testable. V1 targets a CommonMark-compatible baseline plus product-defined extensions such as Mermaid and LaTeX-style math; compatibility extensions are added deliberately rather than by accident.

## 3. Two editing modes, one document

### 3.1 Source Mode

Source Mode is the direct text projection of Document source.

```text
Document source
   -> Source Projection
   -> text layout / syntax decoration
   -> user text edit
   -> EditTransaction
   -> Document
```

Source Mode must remain operational even when Markdown parsing or a rich projection is temporarily unavailable. Syntax highlighting/styling is derived decoration, never required to preserve the source.

### 3.2 Live Mode

Live Mode is an editable semantic projection of the same source.

```text
Document source
   -> Markdown Semantics
   -> Live Projection
   -> visual editing gesture
   -> semantic/source-aware command
   -> EditTransaction
   -> Document source
```

Live Mode MUST NOT follow this architecture:

```text
Markdown -> independent RichTextDocument -> serialize back to Markdown
```

That model creates two truths, round-trip normalization, ambiguous identity, and divergent undo/selection semantics.

Instead a Live Mode action such as bolding text conceptually becomes:

```text
ToggleStrong(source_range)
   -> Markdown-aware source edit
   -> EditTransaction
```

Mode switching:

- reprojects the same Document revision;
- does not itself mutate source;
- preserves dirty state and undo history;
- maps caret/selection through explicit source/semantic/visual coordinates;
- degrades ambiguous constructs to source-visible editing rather than guessing destructively.

## 4. Coordinate model

Markit must distinguish at least:

```text
Source coordinates
  bytes / Unicode boundaries / logical source positions

Semantic coordinates
  block identity / inline identity / source span

Visual coordinates
  line / run / glyph / x,y / platform text coordinates
```

Conversions are explicit. APIs must not use an ambiguous generic `offset` when the coordinate space matters.

IME and platform UTF-16 coordinates terminate at the platform/text boundary and are converted into Markit’s source coordinate model before becoming source edits.

## 5. Preview is not Live Mode

Preview is read-only. Live Mode is editable.

They may share semantic inputs, style tokens, rich-block providers, and reusable rendering artifacts, but they do not share mutable editor state.

```text
Markdown Semantics
  ├─ Live Projection: caret + selection + editing commands
  └─ Preview Projection: read-only observation
```

This separation avoids turning a read-only renderer into a second editor engine.

## 6. Incremental semantic pipeline

Interactive editing is an arbitrary-mutation workload, not an append-only stream.

The intended logical pipeline is:

```text
EditTransaction
     ↓
Document revision N -> N+1
     ↓
ChangeSet
     ↓
Incremental Markdown update
     ↓
SemanticDelta
     ↓
RenderPatch stream
```

`ChangeSet`, `SemanticDelta`, and `RenderPatch` are semantic concepts here, not frozen Rust ABIs.

### 6.1 Incremental parse law

For a local edit, Markit should recompute the smallest semantically valid affected region and reuse unrelated semantic state.

A structural edit may honestly propagate far. The parser must not change Markdown meaning merely to manufacture a small invalidation radius.

### 6.2 Progressive publication law

Rendering does not need to wait for every derived region in a large document before visible interaction can progress.

Priority is:

```text
current input / caret
  > currently visible affected content
  > near-visible content
  > distant/background presentation
```

Long work may yield/chunk. Stale derived results carry revision/dependency identity and are cancelled or rejected at publication.

No permanent 60/120 Hz application loop is required; rendering remains demand-driven.

### 6.3 Coherence law

Progressive does not mean incoherent.

A projection may reuse prior artifacts only when dependencies prove compatibility. A block/result from revision N cannot silently overwrite or masquerade as revision N+1.

When a broad update is incomplete, the UI may show a prior-good representation or explicit pending state where safe; it must not publish a fabricated mixture as fully current.

## 7. Rich Projection Services

Mermaid and LaTeX-style math are mandatory built-in rich projections, but they are not Markdown/document authorities.

The generic shape is:

```text
Semantic rich block/span
        + source/revision identity
        ↓
Rich Projection Provider
        ↓
Ready(renderable) | Failed(visible diagnostic)
        ↓
projection validates revision/identity
        ↓
publish
```

### 7.1 Mermaid

Mermaid blocks originate from Markdown fence semantics. Rendering may use Mermaid.js or another compatible implementation.

Rules:

- do not execute the full Mermaid renderer synchronously on every keystroke;
- coalesce/cancel/reject stale requests;
- a result belongs to a semantic block identity + source revision/configuration;
- Preview, Live, Browser, and Print consume the same semantic Mermaid source even if backend rendering mechanisms differ;
- failure is visible and source remains editable.

### 7.2 LaTeX-style math

Math spans/blocks are semantic projections over source ranges.

Rules mirror Mermaid:

- inline `$...$` and display `$$...$$` are mandatory V1 syntax;
- provider results are revisioned;
- invalid syntax yields an explicit fallback/diagnostic;
- provider implementation may be KaTeX-compatible or otherwise replaceable;
- arbitrary TeX execution is outside the math renderer authority.

## 8. Browser and print architecture

Interactive projection and print projection optimize for different truths.

```text
                  Markdown Semantic Snapshot
                         /             \
                        /               \
      Interactive Projection         Print Projection
      viewport-first                 full-document
      latency-first                  completeness-first
      progressive                    completion barrier
      cancellable                    pinned revision
```

The full contract is `docs/product/print-browser-contract.md`.

The critical boundary is:

> **editor viewport state must never define print completeness.**

Browser/Print consumes one coherent Document snapshot, materializes the full document, waits for required rich resources to become Ready or visibly Failed, then publishes `PrintReady`.

V1 delegates pagination/PDF generation to the system browser. Markit owns semantic HTML/output structure, resource readiness, and print CSS; the browser owns its print engine.

## 9. Browser transport

How Markit hands a page to the browser is a replaceable host mechanism.

A loopback HTTP server is a reasonable V1 candidate because it handles local resources and browser refresh cleanly, but the architecture does not make `localhost` URLs part of document semantics.

Any implementation must:

- remain local by default;
- not upload document contents;
- restrict file/resource access to explicitly resolved document/workspace assets;
- bundle mandatory renderer assets locally for offline use;
- make the printed revision identifiable;
- avoid treating filesystem paths as URLs or vice versa.

## 10. Desktop Host boundary

Desktop Host owns mechanisms such as:

- window lifecycle;
- GPUI/native rendering integration;
- OS keyboard/pointer/IME delivery;
- clipboard;
- native file dialogs where used;
- `.md` file association / Open With integration;
- startup arguments and already-running-instance handoff;
- launching the system browser;
- platform-specific paths/packaging.

These mechanisms do not redefine Document or Markdown semantics.

Windows is the first product target; other platforms may provide different host mechanisms behind the same semantic behavior.

## 11. Plugin / provider boundary

Markit is designed to remain plugin-extensible without prematurely building a universal plugin framework.

The extension architecture is:

```text
private implementation
      ↓ adapter
versioned semantic capability
      ↓
provider/plugin
      ↓
result / command
      ↓ validation
Markit authority
```

Never:

```text
plugin -> mutable Document internals
plugin -> GPUI Entity tree as semantic state
plugin -> private Markdown IR memory layout
plugin -> scheduler/cache internals
plugin object identity -> document/block identity
```

### 11.1 Candidate semantic capabilities

The following seams should remain feasible:

- rich block/span projection provider;
- Markdown extension parser/semantic provider where safely specifiable;
- command provider;
- read-only document/workspace query;
- controlled document-edit command;
- preview/browser stylesheet contribution;
- exporter/output provider;
- sidebar/panel UI provider later.

Not all need a V1 public API.

### 11.2 Built-ins as boundary tests

Mermaid, math, and Browser/Print are useful built-in workloads for testing whether the semantic seams are clean. They should not be forced through a third-party runtime in V1, but their implementation must not require private representation leakage that would make later provider extraction impossible.

### 11.3 Failure isolation

Extension/provider work that is not required for direct source editing must not indefinitely block Source Mode input. Stale, failed, or crashed providers fail visibly at their projection boundary.

## 12. Data movement rules

Markit should minimize avoidable memory movement without freezing a complicated storage structure before measurement.

Rules:

- source text is not copied wholesale between Source, Live, Preview, and Print just to cross module boundaries;
- semantic nodes prefer source spans/references over duplicated strings where safe;
- projection layers consume snapshots/views and copy only what their backend lifetime requires;
- rich providers receive the smallest stable semantic/source payload required for their work;
- no JSON/IPC serialization is inserted into the local hot path merely to imitate a future plugin boundary;
- a future Rope/PieceTree/other buffer is adopted only when measured source mutation costs justify it.

Semantic ownership is more important than premature zero-copy tricks: lifetimes must remain explicit and safe.

## 13. Caches and derived state

All caches and backend projection objects are disposable derived state.

For every cache, the implementation must be able to answer:

```text
What is the key?
What source/semantic revision/dependencies produced it?
What invalidates it?
Can it be reused across a local edit?
How is stale publication prevented?
What bounds memory growth?
```

Deleting all caches/projection objects must not destroy the authoritative Markdown source.

## 14. Existing experimental code

The pre-reset code contains useful candidates: Document/revision/change work, incremental Markdown experiments, GPUI integration, viewport work, IME validation, and performance instrumentation.

They are not automatically discarded and not automatically canonical.

For each reused component, a post-reset implementation task should record:

1. which current product responsibility it satisfies;
2. which old assumptions were removed;
3. whether its public types leak old experimental architecture;
4. correctness tests against the new contract;
5. whether reuse avoids or creates extra copying/translation;
6. measured user-visible value where performance is the reason for reuse.

## 15. Architecture invariants

A product change must not violate these without a new architecture decision:

- **A1** — Markdown source is the only document truth.
- **A2** — Source Mode and Live Mode mutate the same Document through transactions.
- **A3** — mode switching alone does not mutate source.
- **A4** — Workspace does not own a shadow editor buffer.
- **A5** — Markdown semantics are backend-independent.
- **A6** — interactive local changes reuse unaffected work where semantics permit.
- **A7** — stale derived work cannot publish over newer state.
- **A8** — Source Mode remains usable when optional/rich projections fail.
- **A9** — Mermaid/math results are revision/identity checked.
- **A10** — print completeness is independent of editor viewport/scroll history.
- **A11** — `PrintReady` means all required print resources reached Ready or visible Failed terminal state for one pinned revision.
- **A12** — plugins/providers cross a semantic capability boundary, not private implementation identity.
- **A13** — core product workflows do not require cloud/network access.
- **A14** — platform path/URL conversions are explicit at the host boundary.
- **A15** — performance optimization may change representation, not silently change Markdown/product semantics.