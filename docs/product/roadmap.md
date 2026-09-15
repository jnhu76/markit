# Markit Product Roadmap

Status: **post-reset execution roadmap**  
Authority: `docs/PRD.md` -> `docs/product/architecture.md` -> `docs/product/mvp-v0.1.md`

This roadmap starts from product requirements, not from the shape of the pre-reset experiments.

Existing work from revision `d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e` is retained as experimental evidence. Reuse is allowed, but no old phase/ADR/implementation is automatically a prerequisite or product authority after `MARKIT-PRODUCT-RESET-0`.

The product laws are:

```text
Markdown Source is the single source of truth.
Parse incrementally; publish progressively; print completely.
```

## Roadmap graph

```text
R0  Product Truth Reset                         ← this documentation reset
 │
 ├──────────────┐
 ▼              ▼
R1 Source Core  R2 Workspace + OS Open
 │              │
 └──────┬───────┘
        ▼
R3 Markdown Semantics + Split Preview
        │
        ├───────────────┐
        ▼               ▼
R4 Rich Projection    R5 Browser / Print
Mermaid + Math        completeness contract
        │               │
        └───────┬───────┘
                ▼
          R6 Live Mode
       source-aware editing
                │
                ▼
       R7 Extension Boundary
       conformance / providers
                │
                ▼
          R8 v0.1 Hardening
                │
                ▼
              v0.1
                │
                ▼
        R9 Cross-platform / later plugins
```

This is a dependency graph, not a forced waterfall. Independent nodes may run in parallel when their prerequisites are satisfied.

## R0 — Product Truth Reset

### Goal

Replace the research-first product narrative with a requirement-first one while preserving old work as evidence.

### Scope

- rewrite `docs/PRD.md`;
- rewrite `docs/product/architecture.md`;
- rewrite `docs/product/mvp-v0.1.md`;
- rewrite this roadmap;
- rewrite repository `README.md`;
- add `docs/product/print-browser-contract.md`;
- record the pre-reset authority boundary under `docs/archive/`;
- declare the current implementation experimental/reference until conformance is re-earned.

### Acceptance

- documents agree on Source Mode + Live Mode;
- Workspace Search, split Preview, Browser/Print, Mermaid, LaTeX math, and OS Open With are V0.1 product requirements;
- one source truth is explicit;
- interactive incremental/streaming rendering is separated from exhaustive print;
- plugin extensibility is preserved without requiring a plugin marketplace/runtime now;
- old product/research documents cannot silently override the reset authority.

### Non-goal

No production-code rewrite in R0.

## R1 — Source Core

### Goal

Produce the smallest trustworthy product path for editing the real Markdown source.

```text
open file
  -> Document source
  -> Source Mode
  -> EditTransaction
  -> revision / undo / dirty
  -> save
```

### Reuse audit

Before writing replacements, audit the experimental implementation for candidates such as:

- document storage;
- revision/change model;
- transactions;
- selection;
- line index;
- GPUI text/input integration;
- IME evidence;
- save path.

For each candidate classify:

```text
ADOPT
ADAPT
DELETE / REPLACE
```

Do not preserve old types merely to minimize diff size.

### Required scope

- authoritative Document;
- explicit revision/change result;
- Source Mode plain-text editing;
- caret/selection/navigation;
- copy/cut/paste;
- undo/redo;
- open/save/save-as;
- UTF-8/CJK/emoji;
- Windows Chinese IME;
- source editing remains usable without Markdown rich rendering.

### Acceptance

- one source truth;
- round-trip does not rewrite unrelated syntax;
- common editing operations work on real Windows host;
- path/encoding tests include CJK;
- no Live/Preview shadow document is introduced;
- idle UI remains demand-driven.

## R2 — Workspace + OS Open

May proceed in parallel with later R1 work once Document open semantics are stable.

### Goal

Make Markit useful as a local Markdown workspace tool.

### Scope

```text
Workspace root
├─ file tree
├─ file open
└─ text search
```

and Windows host integration:

```text
.md -> Open With Markit
shell path -> Markit open
existing process -> open requested document
```

### Search policy

Start with the smallest direct scanner/search engine that satisfies the product. Do not create an index database until evidence shows it is necessary.

### Acceptance

- workspace search returns file/line/match/context;
- click result opens/jumps;
- path spaces/CJK/Unicode pass;
- `.md` Open With passes;
- no URL/path confusion;
- Workspace owns discovery, not a duplicate editor buffer.

## R3 — Markdown Semantics + Split Preview

### Goal

Build the first shared semantic rendering path for arbitrary edits.

```text
Document revision/change
  -> Markdown Semantics
  -> SemanticDelta
  -> Preview RenderPatch
  -> read-only Preview
```

### Scope

- explicit Markdown dialect baseline;
- source spans and semantic identity;
- incremental update for ordinary local edits;
- honest propagation for structural edits;
- Source + Preview left/right layout;
- revision-safe progressive publication;
- instrumentation for changed semantic region and projection work.

### Streaming rule

Borrow streaming Markdown ideas only at the publication/scheduling level. An editor must support arbitrary insertion/deletion/replacement, not only append-tail streams.

### Acceptance

- Source + Preview share one semantic authority;
- normal local edits reuse unaffected semantic/projection state where valid;
- large/broad work can yield/progressively publish;
- stale work cannot overwrite newer presentation;
- no permanent update loop;
- parser correctness is not weakened to manufacture a small invalidation radius.

## R4 — Rich Projection: Mermaid + LaTeX Math

### Goal

Add the two mandatory rich rendering workloads through clean provider-shaped boundaries.

### Mermaid scope

- fenced `mermaid` block semantics;
- asynchronous/deferred/coalesced rendering;
- Preview output;
- Live-compatible projection input for R6;
- browser/print representation;
- visible failure fallback;
- offline/local mandatory assets.

### Math scope

- inline `$...$`;
- display `$$...$$`;
- Preview output;
- Live-compatible projection input for R6;
- browser/print output;
- CJK + math fixtures;
- visible invalid-expression fallback;
- offline/local mandatory assets.

### Acceptance

- typing is not synchronously blocked by unnecessary heavy renderer execution;
- provider results carry semantic identity/revision;
- old completions cannot publish over new source;
- renderer failure does not make Source Mode unavailable;
- built-in implementation does not leak a renderer-specific object as public semantic identity.

## R5 — Browser Preview + Print Completeness

Can begin once R3 semantic output exists; complete Mermaid/math tests after R4.

### Goal

Make Browser Preview and browser Print/PDF a reliable first-class output workflow.

### Scope

Implement `docs/product/print-browser-contract.md`:

- coherent `DocumentSnapshot(revision=N)`;
- whole-document browser/print materialization;
- `PrintReady` barrier;
- `Ready | FailedVisible` terminal resources;
- product print CSS;
- local image/resource resolution;
- CJK/browser font validation;
- browser launch;
- local/offline mandatory assets;
- secure local transport if loopback HTTP is used.

### Critical adversarial gate

Printing must be invariant to interactive scroll history:

```text
print immediately after open
== semantic content ==
scroll through whole document, then print
```

Offscreen Mermaid and offscreen math are required regression fixtures.

### Acceptance

All tests in `print-browser-contract.md` pass on Windows reference browser configuration, and browser printing never depends on which editor blocks were previously materialized.

## R6 — Live Mode

### Goal

Add source-aware WYSIWYG editing without creating a second document truth.

### Required model

```text
Document source
  -> Markdown Semantics
  -> Live Projection
  -> gesture/command
  -> source-aware EditTransaction
  -> same Document
```

### Initial scope

- switch Source <-> Live;
- heading/paragraph presentation;
- emphasis/strong;
- inline/fenced code presentation;
- links/lists/blockquote sufficient for the baseline dialect;
- Mermaid/math rendered as rich blocks while their source remains authoritative;
- formatting commands produce Markdown source edits;
- source reveal/fallback for ambiguous/unsupported syntax;
- explicit source/semantic/visual coordinate mapping;
- selection/caret behavior;
- IME correctness in Live editing paths where text insertion occurs.

### Acceptance

- mode switch alone is byte-identical;
- revision/dirty/undo are shared, not synchronized copies;
- representative Live edits produce expected Markdown source;
- unsupported constructs cannot be silently normalized/dropped;
- deleting all Live projection state loses no document semantics;
- Source Mode remains the escape hatch for every document.

## R7 — Extension Boundary Conformance

### Goal

Prove the architecture remains plugin/provider-extensible before private implementation leaks harden into public contracts.

This phase does **not** automatically build a general plugin runtime.

### Scope

Exercise built-in workloads as if they were provider candidates:

- Mermaid projection;
- math projection;
- browser/export projection;
- at least one read-only semantic query;
- at least one controlled command/edit boundary.

For each verify:

- semantic version/capability shape;
- snapshot/query input rather than mutable internals;
- command/result output;
- revision/identity validation;
- no GPUI/private IR/cache/scheduler identity in the contract;
- provider failure does not poison Source Mode.

### Decision gate

Only after at least two materially different external extension workloads exist should Markit choose a concrete third-party runtime/transport such as in-process, Wasm, subprocess/IPC, or another mechanism.

Valid outcomes before that are:

```text
KEEP SEMANTIC SEAM ONLY
SPIKE RUNTIME
DEFER
```

## R8 — v0.1 Hardening

### Goal

Close every gate in `mvp-v0.1.md` on a real Windows host and ship the first useful product.

### Scope

- reliability/save/crash recovery appropriate for v0.1;
- installer/portable packaging decision sufficient for OS registration;
- real-host performance evidence;
- workspace/search stress;
- large-document interaction behavior;
- Mermaid/math failure/stale cases;
- full print adversarial corpus;
- CJK/IME/path regression matrix;
- memory/cache bounds;
- accessibility basics needed for shipping;
- product docs/help sufficient to explain Source vs Live vs Preview.

### Performance decision gates

Do not preselect Rope/PieceTree, worker counts, cache sizes, or a custom scheduler.

Measure:

- source mutation cost by edit position/size;
- bytes moved/copied where measurable;
- semantic work amplification;
- projection/layout/shaping work;
- rich provider latency/cancellation;
- input -> visible frame tails;
- workspace search latency;
- Browser/Print preparation latency and failures;
- idle CPU/memory.

If an experimental component is not the bottleneck, do not rewrite it for fashion.

## R9 — Cross-platform and later plugin runtime

After Windows v0.1:

### Cross-platform

Bring up macOS/Linux host mechanisms while preserving:

- Document semantics;
- Markdown semantics;
- Source/Live command behavior;
- Preview/Print contracts;
- provider capability semantics.

Platform-specific differences stay at host edges: IME, fonts, shortcuts, dialogs, packaging, browser launch, file association, scheduling primitives.

### Plugin runtime

Build a concrete plugin runtime only when actual external extension workloads justify it. Runtime selection must evaluate:

```text
failure isolation
latency / hot-path contamination
startup and memory
security/capabilities
cross-platform packaging
version compatibility
debugging/developer experience
```

The semantic boundary comes first; transport is replaceable.

## Cross-cutting stop conditions

Stop and open a focused architecture review if any change requires:

1. a Source Document and Live Document with synchronization between them;
2. mode switching that rewrites source merely to change presentation;
3. a renderer/provider object becoming durable document identity;
4. Preview or Browser reparsing a separate undocumented Markdown dialect;
5. print completeness depending on viewport/scroll/lazy cache state;
6. synchronous Mermaid/math work on every ordinary keystroke without evidence;
7. a permanent search index before direct search proves insufficient;
8. a plugin/provider mutating Document internals directly;
9. GPUI/private IR/cache/scheduler types entering public semantic contracts;
10. changing Markdown semantics to make a performance benchmark look bounded;
11. network/cloud dependence for core local editing/rendering/printing;
12. a major data copy/serialization boundary that cannot name the product value it buys.

## Working rule

> Build the smallest boundary that preserves the product truth, measure the real user path, and delete experimental machinery that no longer earns its place.