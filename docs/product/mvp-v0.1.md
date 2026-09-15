# Markit — v0.1 Product Scope

Status: **post-reset first shippable product**  
Authority: `docs/PRD.md` and `docs/product/architecture.md`

Markit v0.1 is no longer defined as a single-document GPUI experiment. It is the first usable product that satisfies the original Markit workflow plus the later-added Live Mode, Mermaid, LaTeX-style math, streaming/incremental rendering, and browser-print correctness requirements.

Windows is the first shipping target. Existing pre-reset code is an experimental/reference implementation and must not weaken these gates merely because a feature already exists in some form.

## 1. V0.1 product promise

A user can:

```text
Open a .md file or workspace
  -> edit the real Markdown source
  -> optionally edit in Live Mode
  -> search the workspace
  -> view Source + Preview side by side
  -> render Mermaid and LaTeX-style math
  -> open a complete browser representation
  -> Print / Save as PDF
```

The two product laws remain:

> **Markdown Source is the single source of truth.**

> **Parse incrementally; publish progressively; print completely.**

## 2. In scope

### Editing

- Source Mode with direct Markdown source editing.
- Live Mode with source-aware rendered editing over the same Document.
- caret, selection, navigation, scroll;
- copy/cut/paste;
- undo/redo with transaction grouping;
- find within the current file;
- open/save/save-as;
- dirty-state handling;
- UTF-8 source;
- Chinese/CJK input and rendering;
- emoji fallback;
- Chinese IME commit/cancel/composition correctness on Windows.

### Files and workspace

- open a single `.md` file;
- open a folder as a workspace;
- workspace file tree sufficient to navigate Markdown/text files;
- workspace full-text search;
- search result file/line/match/context;
- click search result -> open/jump;
- paths containing spaces/Unicode/CJK;
- atomic/reliable save behavior appropriate for the platform.

### OS integration

- Windows `.md` Open With / file association path;
- launch Markit with a Markdown path from the shell;
- already-running-instance handling sufficient to open the requested document;
- system default browser launch for Browser Preview/Print.

### Markdown semantics

- CommonMark-compatible baseline sufficient for ordinary Markdown documents;
- headings, paragraphs, emphasis/strong, inline code, fenced code, links, blockquotes, ordered/unordered lists as baseline constructs;
- extension semantics are explicit rather than accidental;
- Mermaid fenced blocks are mandatory;
- LaTeX-style inline `$...$` and display `$$...$$` math are mandatory;
- parser/semantic diagnostics do not destroy source text.

### Preview

- Source Mode can be shown beside a read-only Preview in a left/right split;
- Preview consumes the same document revision and semantic authority;
- local edits update Preview incrementally where semantics allow;
- Preview may prioritize visible content and progressively publish derived work;
- rich projection failure is visible without breaking Source Mode.

### Live Mode

- same Document/revision/undo/dirty state as Source Mode;
- source-aware rendered editing, not an independent rich-text document;
- switching modes does not mutate source;
- source/semantic/visual coordinate mapping is explicit;
- common formatting/editing gestures produce Markdown-aware edit transactions;
- ambiguous or unsupported syntax remains safely source-editable rather than normalized destructively.

### Mermaid

- visible in Preview;
- visible in Live Mode where applicable;
- visible in Browser Preview and Print/PDF;
- expensive render work is deferred/coalesced and stale-safe;
- invalid diagrams produce visible diagnostics/fallbacks;
- mandatory renderer assets are local/offline-capable.

### LaTeX-style math

- inline and display math;
- Preview, Live, Browser Preview, and Print/PDF support;
- invalid formula -> visible fallback/diagnostic;
- CJK + math mixed layout correctness;
- renderer work is revision-aware and does not make normal source typing wait on unnecessary computation.

### Browser Preview / Print

- open current document in the system browser;
- Browser/Print uses a coherent document snapshot;
- full-document materialization independent of editor viewport/scroll history;
- wait for Mermaid, math, local images, fonts, and other required print resources before `PrintReady`;
- failed required resources become visible `FailedVisible` content;
- product-owned print stylesheet;
- browser handles Print / Save as PDF;
- acceptance follows `docs/product/print-browser-contract.md`.

### Incremental / streaming rendering

- document edits produce explicit change/revision information;
- Markdown semantics update incrementally for ordinary local edits;
- projections consume semantic deltas/patches rather than requiring full-document rebuilds by default;
- visible interaction outranks distant rendering work;
- stale async/deferred results cannot publish over a newer revision;
- broad structural changes may propagate honestly and may be chunked/yielded;
- no permanent fixed-rate render loop is required.

### Extension boundary

V0.1 does not need a marketplace or general third-party plugin runtime, but must preserve a future plugin/provider boundary:

- semantic snapshots/queries rather than mutable internals;
- commands/transactions for mutation;
- revisioned provider results;
- no GPUI/private Markdown IR/cache identity as public contract;
- built-in Mermaid/math/browser-output implementation does not make later provider extraction impossible.

## 3. Explicitly not required for v0.1

```text
cloud sync / accounts / collaboration
AI assistant
Git GUI
terminal / debugger / LSP IDE
plugin marketplace
arbitrary third-party plugin runtime
DOCX export
independent PDF engine
Notion-style block database
arbitrary TeX execution
full compatibility with every Obsidian/Typora extension
macOS/Linux shipping certification
```

macOS/Linux architecture must remain possible, but Windows is the real-host acceptance target for v0.1.

## 4. Acceptance gates

V0.1 ships only when the following gates pass on the product implementation.

### G1 — Source truth

- [ ] Source Mode edits the actual Markdown source.
- [ ] opening + saving without edits does not normalize unrelated syntax.
- [ ] source remains recoverable/editable when rich rendering fails.
- [ ] one authoritative Document/revision exists per open editing session.

### G2 — Source editing

- [ ] type/delete/newline/navigation/selection work.
- [ ] copy/cut/paste work with the OS.
- [ ] undo/redo grouping is sane for typing, deletion, paste, and IME commit.
- [ ] Chinese IME commit/cancel/composition is correct on Windows.
- [ ] CJK/emoji display works with system fallback.

### G3 — File and path correctness

- [ ] open/save/save-as work for UTF-8 Markdown.
- [ ] paths with spaces/CJK/Unicode work.
- [ ] save is crash-safe enough not to silently replace a good file with a torn write.
- [ ] URL/path conversion is explicit at platform/browser boundaries.

### G4 — Workspace

- [ ] open-folder workspace flow works.
- [ ] file tree opens documents.
- [ ] workspace text search reports file, line, match, and context.
- [ ] clicking a result opens/jumps to it.
- [ ] search does not require a persistent index database merely to pass the gate.

### G5 — OS Open With

- [ ] `.md` can be opened via Windows Open With/file association integration.
- [ ] shell path invocation works with spaces/CJK.
- [ ] invoking an already-running Markit instance opens the requested file without corrupting session state.

### G6 — Split Preview

- [ ] Source + Preview left/right mode works.
- [ ] Preview represents the same source revision/Markdown semantics.
- [ ] local edits do not require a full-document semantic/render rebuild by default.
- [ ] Preview scroll/viewport state does not mutate source.

### G7 — Live Mode single-truth conformance

- [ ] Source -> Live switch does not change file bytes.
- [ ] Live -> Source switch does not change file bytes.
- [ ] both modes share revision, dirty state, and undo history.
- [ ] representative formatting edits in Live Mode create correct Markdown source edits.
- [ ] caret/selection mappings do not rely on ambiguous mixed coordinate spaces.
- [ ] unsupported/ambiguous constructs degrade safely to source-visible handling.

### G8 — Mermaid

- [ ] valid Mermaid renders in Preview.
- [ ] valid Mermaid renders in Browser/Print.
- [ ] Live Mode handles Mermaid without creating a second source authority.
- [ ] invalid Mermaid is visibly diagnosed.
- [ ] stale Mermaid completion cannot overwrite newer source.
- [ ] typing outside/inside Mermaid is not synchronously blocked by unnecessary full renderer execution.

### G9 — LaTeX-style math

- [ ] `$...$` inline math renders.
- [ ] `$$...$$` display math renders.
- [ ] Browser/Print math matches the same semantic source.
- [ ] invalid formulas are visible rather than omitted.
- [ ] CJK surrounding math remains correct.
- [ ] stale formula output cannot publish over newer source.

### G10 — Incremental / streaming interaction

For representative small/medium/large documents:

- [ ] ordinary local edits reuse unaffected semantic/projection state where semantics permit;
- [ ] interactive work is demand-driven;
- [ ] visible/current work outranks distant presentation work;
- [ ] long/broad work can yield or progressively publish without lying about Markdown semantics;
- [ ] stale/out-of-order work is rejected;
- [ ] idle editor does not continuously redraw merely because a scheduler exists.

Do not certify this gate solely from synthetic counters; include real-host interaction evidence.

### G11 — Browser/Print completeness

The full regression corpus in `print-browser-contract.md` must pass, including:

- [ ] print immediately after open without scrolling;
- [ ] print content set unchanged by scrolling through the document first;
- [ ] offscreen Mermaid included;
- [ ] offscreen math included;
- [ ] CJK printable;
- [ ] local images/resources with Unicode paths resolve;
- [ ] slow resource keeps state at Preparing, not incomplete Ready;
- [ ] failed rich block is visibly represented;
- [ ] edit during preparation resolves to one coherent revision;
- [ ] page-break stress content remains visible.

### G12 — Local-first

- [ ] Source editing works offline.
- [ ] workspace search works offline.
- [ ] Preview works offline for core syntax.
- [ ] mandatory Mermaid/math rendering works offline.
- [ ] Browser/Print preparation does not require uploading Markdown to a remote service.

### G13 — Extension-boundary preservation

- [ ] no plugin/provider would need mutable Document internals to implement an extension;
- [ ] no GPUI entity/private IR/cache layout is required by the semantic provider boundary;
- [ ] rich results include revision/identity needed for stale rejection;
- [ ] Markit owns aggregate publication/PrintReady authority;
- [ ] no speculative third-party runtime is required just to satisfy this gate.

## 5. Performance and memory policy

V0.1 does not freeze arbitrary millisecond budgets, worker counts, cache sizes, or a specific text-buffer data structure before measurement.

It does freeze these qualitative requirements:

```text
ordinary local edit work should track changed semantics + visible presentation,
not total document size, where Markdown semantics permit;

unaffected source is not copied through every projection layer;

heavy providers do not run synchronously merely because their source exists;

print may intentionally materialize the whole document because completeness,
not interaction latency, is its authority.
```

Real-host p50/p95/p99/long-frame data should be collected where statistically meaningful, but correctness gates cannot be traded away to improve a benchmark.

## 6. Exit statement

Markit v0.1 is ready when it is a useful Markdown workspace editor rather than a rendering experiment: the user can choose source editing or source-aware Live editing, search a workspace, preview beside source, use Mermaid and LaTeX math, and reliably hand a complete document to the browser for Print/PDF — all while one Markdown source remains authoritative and future plugins are not forced to depend on private implementation details.