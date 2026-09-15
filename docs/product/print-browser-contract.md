# Markit Browser / Print Contract

Status: **normative product contract**  
Authority: `docs/PRD.md` -> `docs/product/architecture.md` -> this document

Markit deliberately delegates paper/PDF generation to the system browser in V1. That subtraction is only valid if Markit supplies a complete, coherent, print-ready browser document.

This contract exists to prevent a common class of editor/export defects: content looks correct while editing, but offscreen/lazy materialized blocks, diagrams, formulas, fonts, or images silently disappear or differ when printed.

The governing rule is:

> **Interactive rendering may be partial. Printing may not be.**

## 1. Definitions

### Browser Preview

A browser-hosted representation of a Markdown document revision. It may update while the source changes, but each published state must identify the revision it represents.

### Print Document

A whole-document representation built from one coherent `DocumentSnapshot` and its Markdown semantics. It is not the GPUI viewport, not a screenshot of Preview, and not a traversal of only materialized editor nodes.

### Print Resource

Any resource whose absence changes printed content, including at least:

- Mermaid diagrams;
- LaTeX-style math output;
- local images;
- required fonts/fallback fonts;
- syntax/style assets needed for visible code rendering;
- plugin/provider output declared print-relevant.

### PrintReady

A terminal publication state for one pinned document revision meaning:

- the full document structure has been materialized for browser/print;
- every required Print Resource is either `Ready` or `FailedVisible`;
- no required content is merely pending because it was offscreen or never visited;
- print CSS and resource URLs are installed;
- the browser page may be printed without relying on editor viewport history.

`FailedVisible` means the printed/browser document contains an explicit diagnostic/fallback in the location of failed content. It does **not** mean silently omit it.

## 2. Core invariants

### PRINT-01 — Pinned revision

A Print Document is generated from one explicit `DocumentSnapshot(revision=N)`.

Async results from another revision cannot be inserted into it unless a documented compatibility rule proves them semantically identical for that resource.

If the source changes to revision N+1 while the browser still displays N, the page either:

- remains a clearly identified snapshot of N; or
- rebuilds coherently for N+1.

It must not silently mix them.

### PRINT-02 — Full-document materialization

Print materialization covers the complete document regardless of:

- current editor scroll position;
- current Preview scroll position;
- viewport height;
- overscan;
- which blocks have previously been visible;
- interactive LOD/cache state.

The following is forbidden:

```text
print output = whatever editor blocks currently happen to be materialized
```

### PRINT-03 — Scroll-history independence

Given the same source revision and print configuration, these histories must not change document completeness:

```text
A. open document -> print immediately
B. open document -> scroll to end -> print
C. open document -> visit every Mermaid/math block -> print
```

A, B, and C may differ in incidental browser timing, but not in which semantic content exists in the final Print Document.

### PRINT-04 — Completion barrier

Markit must not declare `PrintReady` while a required rich resource is only queued/running.

Conceptually:

```text
Build semantic Print Document
        ↓
resolve local resources
        ↓
render Mermaid
render LaTeX math
load required fonts
load/decode images
run print-relevant providers
        ↓
Ready | FailedVisible for every required resource
        ↓
PrintReady(revision=N)
```

Internal generation may stream and run concurrently. Publication of `PrintReady` is the barrier.

### PRINT-05 — No silent omission

A failed Mermaid block, math expression, image, or provider result must occupy an explicit visible fallback region with enough information for the user to notice the failure.

Blank space or disappearance is not an acceptable error path.

### PRINT-06 — Shared semantics, separate projection

Desktop Preview, Live Mode, Browser Preview, and Print consume the same Markdown semantic authority.

They may use different layout/rendering mechanisms.

Browser/Print must not run an independently defined Markdown dialect that can reinterpret the same source differently without a documented compatibility rule.

### PRINT-07 — Browser print is the PDF engine

V1 does not implement a second PDF layout engine.

Markit owns:

- semantic HTML/document structure;
- bundled/local renderer assets;
- resource readiness;
- print stylesheet;
- print diagnostics;
- browser launch.

The browser owns:

- paged layout implementation;
- printer selection;
- final PDF serialization / physical printing.

### PRINT-08 — Local-first resources

Core browser/print output must work offline.

Mandatory Mermaid/math runtime assets and product CSS/fonts that Markit itself requires are bundled or available locally. Opening a document must not require sending Markdown to a remote rendering service.

### PRINT-09 — Stable local asset resolution

Relative local resources are resolved from an explicit document/workspace context.

Resolution must be deterministic and safe. Filesystem paths are not manipulated as URL strings without explicit conversion.

Missing/denied resources become visible diagnostics.

### PRINT-10 — Print CSS is product behavior

Markit owns a versioned print stylesheet. Browser defaults are not sufficient product specification.

At minimum the stylesheet must define behavior for:

- printable content width and margins;
- heading/page-break behavior;
- paragraphs and blockquotes;
- fenced code / long lines / wrapping policy;
- tables if present in the dialect;
- images / diagrams max printable width;
- Mermaid diagrams;
- display math;
- links and link text;
- light/dark theme conversion for print readability;
- avoidance of obviously unusable page breaks where CSS paged-media controls can express intent.

Markit should prefer standard `@media print` / `break-*` CSS and let the browser paginate rather than implementing its own paginator.

### PRINT-11 — Font completeness

The print path must validate text with:

- Latin;
- CJK;
- emoji/fallback where printable;
- mixed CJK + Latin + math;
- code blocks containing CJK.

A font used by the interactive GPUI renderer is not automatically evidence that the browser print path has a correct fallback chain.

The PrintReady barrier includes required web/system font readiness when asynchronous loading is involved.

### PRINT-12 — Rich content must not depend on interaction

Mermaid/math output may not require the user to click, focus, hover, expand, or scroll a block before it becomes print-relevant.

Interactive controls may be removed/simplified for print, but semantic content remains.

## 3. Browser Preview behavior

Browser Preview is useful for two workflows:

1. inspect the output in a normal browser;
2. use browser Print / Save as PDF.

The browser page SHOULD expose a small Markit status surface outside printed content:

```text
Document: <name>
Revision: <N>
Rendering: Preparing | Ready | Ready with errors
```

When `Ready with errors`, failures are visible in-document and the status makes them obvious before printing.

The browser page may live-update for source changes, but a print action must always target a coherent ready revision.

## 4. Browser transport requirements

A loopback HTTP transport is permitted and is the likely V1 mechanism, but it is not a semantic contract.

If a loopback server is used:

- bind only to loopback by default;
- do not expose the workspace as a generic static file server;
- use explicit resource routes/capabilities rather than arbitrary path traversal;
- use an unguessable session/document token when appropriate;
- stop/revoke stale sessions when the document/window lifecycle ends;
- apply an appropriate Content Security Policy;
- do not fetch remote scripts for mandatory Mermaid/math rendering;
- escape/sanitize generated HTML according to the chosen raw-HTML Markdown policy.

A temporary-file implementation is also possible if it satisfies the same resource/security/completeness contract.

## 5. Mermaid contract

For each print-relevant Mermaid block:

```text
(block identity, source revision, Mermaid source, render config)
        ↓
Mermaid provider
        ↓
Ready(SVG/print representation)
  OR
FailedVisible(diagnostic/source excerpt)
```

`PrintReady` waits for the terminal state.

Tests must include:

- Mermaid block near the top;
- Mermaid block far below the initial viewport;
- multiple diagrams;
- invalid Mermaid source;
- source edited while a prior render is running;
- print immediately after opening without scrolling.

## 6. LaTeX-style math contract

For inline `$...$` and display `$$...$$` math:

- formulas are parsed from the same Markdown semantic source spans used by other projections;
- formula rendering is revision checked;
- browser/print uses a locally available renderer/runtime;
- invalid math becomes a visible diagnostic/source fallback;
- `PrintReady` waits for required math output and fonts.

Tests must include:

- inline math in a paragraph;
- display math below the initial viewport;
- many formulas in a long document;
- CJK surrounding inline math;
- invalid expression;
- source changed while rendering.

## 7. Image/resource contract

For local Markdown images/resources:

- resolve relative paths against explicit document/workspace context;
- do not grant the browser arbitrary filesystem browsing authority;
- preserve aspect ratio;
- constrain printable width;
- wait for load/decode success or produce `FailedVisible`;
- test Unicode/CJK filenames and paths containing spaces.

Remote resources, if later supported, require separate policy for privacy, offline behavior, timeout, and print readiness. They are not required to make V1 core printing work.

## 8. Plugin/provider participation

A future provider can declare print-relevant output only through a versioned semantic capability.

The provider contract must include enough information for Markit to know:

- which semantic block/span the result belongs to;
- source/document revision/dependency identity;
- whether the result is still pending;
- terminal `Ready` vs `FailedVisible`;
- whether it contributes browser CSS/assets;
- any bounded resource lifetime needed for the print session.

A provider does not get to mark the whole document `PrintReady`; Markit owns that aggregate decision.

## 9. Page-break policy

Browser engines make final pagination decisions, but Markit should express sensible intent:

- avoid orphaning a heading at the bottom of a page where supported;
- avoid splitting small code blocks/diagrams/math blocks when they fit on one page;
- allow very large blocks to split/scale rather than overflow invisibly;
- keep table rows together when practical;
- ensure wide code/diagrams do not disappear beyond printable bounds;
- prefer readable light print colors even when the editor uses a dark theme.

These are CSS/product tests, not justification for a custom paged-layout engine.

## 10. Required adversarial regression corpus

Print acceptance must include at least these fixtures:

1. **No-scroll long document** — open and print immediately; content at end exists.
2. **Scroll-equivalence** — print before and after scrolling through whole file; semantic content set is identical.
3. **Offscreen Mermaid** — diagram far below viewport renders in print.
4. **Offscreen math** — display/inline math far below viewport renders in print.
5. **Mixed rich content** — Mermaid + math + local image + code + CJK.
6. **CJK font fallback** — Chinese headings/paragraphs/code are present and readable.
7. **Slow provider** — PrintReady waits; user sees Preparing rather than incomplete ready state.
8. **Provider failure** — failed block prints an explicit fallback, not blank output.
9. **Stale completion** — old Mermaid/math result cannot enter a newer print revision.
10. **Edit during preparation** — page resolves to one coherent revision, never a mixed document.
11. **Unicode paths** — local resources under paths with spaces/CJK resolve correctly.
12. **Page-break stress** — headings, code, diagrams, display math, and long content remain visible across pages.

## 11. Acceptance statement

The Browser/Print feature is acceptable only when the following statement is true:

> For a fixed Markdown source revision, the semantic content available to browser printing is complete and independent of whether any region was ever visible in the desktop editor; every print-relevant rich resource reaches an explicit terminal state, and no failure is silently omitted.