# Markit — V0.1 Product Scope

Status: **product scope only; implementation architecture pending Issue #19**

Authority: `docs/PRD.md`

This document says what the first useful Markit release must do. It deliberately does **not** prescribe parser, storage, AST/CST, rendering, scheduler, or UI-backend mechanisms.

Windows is the first shipping target.

## 1. V0.1 promise

A user can:

```text
Open a .md file or workspace
  -> edit the real Markdown source
  -> optionally edit through Live Mode
  -> search the workspace
  -> view Source + Preview side by side
  -> render Mermaid and LaTeX-style math
  -> open a complete browser representation
  -> Print / Save as PDF
```

Hard product law:

> **Markdown Source is the single source of truth.**

Performance requirement:

> **Ordinary edits should avoid unnecessary work; correctness determines how far work must propagate.**

The mechanism used to satisfy that requirement is intentionally deferred to Issue #19 and the architecture that follows it.

---

## 2. In scope

### Editing

- Source Mode with direct Markdown source editing;
- Live Mode over the same source authority;
- caret, selection, navigation, scroll;
- copy/cut/paste;
- undo/redo;
- find within the current file;
- open/save/save-as;
- dirty-state handling;
- UTF-8 source;
- Chinese/CJK/emoji;
- Windows Chinese IME correctness.

### Files and workspace

- open a single `.md` file;
- open a folder as a workspace;
- file navigation;
- workspace full-text search;
- result file/line/match/context;
- click result -> open/jump;
- spaces/Unicode/CJK paths;
- reliable save behavior.

### OS integration

- Windows `.md` Open With / file association;
- shell path opening;
- already-running-instance handling;
- system browser launch.

### Markdown capability

Baseline syntax must cover ordinary Markdown including headings, paragraphs, emphasis/strong, inline/fenced code, links, blockquotes, and ordered/unordered lists.

Mandatory extensions:

- Mermaid fenced blocks;
- inline math `$...$`;
- display math `$$...$$`.

Parser/render diagnostics must never destroy source text.

### Preview

- Source + read-only Preview split view;
- Preview reflects the same document revision/source interpretation;
- ordinary edits update without unnecessary unrelated recomputation;
- failure in rich content is visible and does not disable Source Mode.

### Live Mode

- same source/revision/dirty/undo authority as Source Mode;
- mode switching alone is source-neutral;
- representative formatting actions produce correct Markdown edits;
- ambiguous/unsupported syntax remains safely recoverable in Source Mode;
- no second independently authoritative rich-text document.

### Mermaid

- Preview support;
- Live presentation where applicable;
- Browser/Print support;
- invalid diagrams visibly fail;
- mandatory assets work locally/offline;
- heavy rendering must not unnecessarily block ordinary typing.

### LaTeX-style math

- inline and display math;
- Preview/Browser/Print support;
- Live presentation where applicable;
- invalid expressions visibly fail;
- CJK + math works;
- mandatory assets work locally/offline.

### Browser Preview / Print

- open current document in system browser;
- complete coherent document representation;
- output independent of editor scroll history;
- required resources complete or fail visibly before `PrintReady`;
- browser handles Print / Save as PDF;
- acceptance follows `print-browser-contract.md`.

### Future extension compatibility

V0.1 does not require a marketplace or general plugin runtime.

It must, however, avoid architectural choices that make future extensions depend on mutable/private parser or UI internals.

The exact extension contract is deferred until the core Markdown architecture is known.

---

## 3. Explicitly not required for V0.1

```text
cloud sync / accounts / collaboration
AI assistant
Git GUI
terminal / debugger / LSP IDE
plugin marketplace
general third-party plugin runtime
DOCX export
independent PDF engine
Notion-style block database
arbitrary TeX execution
full compatibility with every Markdown editor extension
macOS/Linux shipping certification
```

---

## 4. Acceptance gates

### G1 — Source truth

- [ ] Source Mode edits the actual Markdown source.
- [ ] open + save without edits does not normalize unrelated syntax.
- [ ] Source Mode remains usable when rich rendering fails.
- [ ] Live Mode does not create a second authoritative document.

### G2 — Editing correctness

- [ ] type/delete/newline/navigation/selection work.
- [ ] copy/cut/paste work.
- [ ] undo/redo behavior is sane.
- [ ] Chinese IME commit/cancel/composition works on Windows.
- [ ] CJK/emoji display works.

### G3 — File/path correctness

- [ ] open/save/save-as work for UTF-8 Markdown.
- [ ] spaces/CJK/Unicode paths work.
- [ ] save does not silently corrupt a good file.
- [ ] URL/path conversion is explicit at relevant boundaries.

### G4 — Workspace

- [ ] open-folder flow works.
- [ ] file navigation works.
- [ ] workspace search reports file/line/match/context.
- [ ] result click opens/jumps.

### G5 — OS Open With

- [ ] `.md` opens through Windows Open With/file association.
- [ ] shell path invocation works with spaces/CJK.
- [ ] an already-running instance opens the requested file correctly.

### G6 — Split Preview

- [ ] Source + Preview works.
- [ ] Preview reflects the same source revision/interpretation.
- [ ] ordinary local edits do not trigger obviously unnecessary whole-document work on supported workloads.
- [ ] Preview state never mutates source merely by being viewed.

This gate intentionally does not prescribe how locality is achieved.

### G7 — Live Mode single-truth conformance

- [ ] Source -> Live mode switch is byte-neutral.
- [ ] Live -> Source mode switch is byte-neutral.
- [ ] both modes share dirty/undo/document authority.
- [ ] representative Live edits generate expected Markdown source.
- [ ] unsupported constructs remain safely recoverable.

### G8 — Mermaid

- [ ] valid Mermaid renders in Preview.
- [ ] valid Mermaid renders in Browser/Print.
- [ ] Live Mode handles it without a second source authority.
- [ ] invalid Mermaid is visibly diagnosed.
- [ ] ordinary typing is not forced to wait on unnecessary full Mermaid rendering.

### G9 — Math

- [ ] inline math renders.
- [ ] display math renders.
- [ ] Browser/Print math represents the same source.
- [ ] invalid formulas are visible rather than omitted.
- [ ] CJK around math remains correct.

### G10 — Arbitrary-edit responsiveness

For representative small/medium/large documents:

- [ ] ordinary local edits remain responsive;
- [ ] work unrelated to the edit is not repeated without a semantic reason;
- [ ] structural edits converge to the same correct interpretation as a clean parse;
- [ ] large-file position/index bookkeeping does not hide pathological linear work;
- [ ] no performance optimization changes Markdown semantics.

The exact parser/storage mechanism used to pass this gate is determined after Issue #19.

### G11 — Browser/Print completeness

The `print-browser-contract.md` regression corpus passes, including:

- [ ] print immediately after open without scrolling;
- [ ] print content set unchanged after scrolling through the document;
- [ ] offscreen Mermaid included;
- [ ] offscreen math included;
- [ ] CJK printable;
- [ ] local Unicode-path resources resolve;
- [ ] slow resource does not produce incomplete `PrintReady`;
- [ ] failed rich block remains visibly represented;
- [ ] edit during preparation resolves to one coherent revision.

### G12 — Local-first

- [ ] source editing works offline;
- [ ] workspace search works offline;
- [ ] core Preview works offline;
- [ ] mandatory Mermaid/math rendering works offline;
- [ ] browser/print preparation does not upload source remotely.

### G13 — Future extension viability

- [ ] the chosen architecture does not require future plugins to mutate private Document/parser/UI internals;
- [ ] rich providers can plausibly be replaceable;
- [ ] no plugin runtime is built merely to satisfy this gate.

---

## 5. Architecture dependency

V0.1 implementation work after basic research is blocked on the parser architecture gate.

Do not infer from this scope that V0.1 requires any particular:

```text
text buffer
parser library
AST/CST representation
incremental algorithm
render delta type
worker pool
scheduler
GPUI backend
plugin runtime
```

The order is:

```text
Issue #19 experiment
  -> parser/research verdict
  -> evidence-backed architecture
  -> implementation
  -> V0.1 acceptance
```

## 6. Exit statement

Markit V0.1 is ready when it is a useful, source-faithful Markdown editor/workspace—not merely a parser or rendering experiment—and when its performance architecture has been earned from evidence rather than inherited from the pre-reset prototype.
