# Markit — Product Architecture

Status: **product foundation / GPUI architecture phase** (ADR-008).

Markit is a Rust-native Markdown editor built **directly on GPUI**.
Windows is the first product platform.

```text
                         Markit

                 ┌───────────────────┐
                 │   markit-core     │
                 │                   │
                 │ document          │
                 │ edit model        │
                 │ selection         │
                 │ undo / redo       │
                 │ markdown          │
                 │ block index       │
                 │ view model        │
                 │ dirty/version     │
                 │ scheduling model  │
                 └─────────┬─────────┘
                           │
                           ▼
                 ┌───────────────────┐
                 │ markit-gpui/app   │
                 │                   │
                 │ window            │
                 │ rendering         │
                 │ native text       │
                 │ keyboard          │
                 │ pointer           │
                 │ IME               │
                 │ clipboard         │
                 │ file dialogs      │
                 └─────────┬─────────┘
                           │
                           ▼
                         GPUI
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
           Windows       Linux        macOS
```

## 1. Why direct GPUI?

The A1–A4 research compared PocketJS and GPUI as candidate substrates.
A4 selected PocketJS at the time (ADR-001). The pivot to direct GPUI
(ADR-008) rests on new information from the PocketJS desktop audit:

- the modern PocketJS desktop path itself uses GPUI for window,
  rendering, native text, keyboard, pointer, IME and clipboard
  integration;
- Markit does not need PocketJS's QuickJS / DrawList / companion /
  capability runtime layers to build a native Markdown editor;
- those intermediate abstractions increase architecture surface and
  attribution complexity;

Markit's product principle is:

```text
Nothing gets between your input and the next frame.
```

An intermediate runtime between Markit and the platform works against
that principle when the platform path it abstracts is itself GPUI.

This is an architecture decision, not a claim that GPUI wins every
benchmark. The A1–A4 measurements remain valid within their original
setup (see `docs/research/`).

## 2. Core vs platform boundary

`markit-core` is a framework-independent Rust library. It owns editor
policy and state; it never branches on the platform and never depends on
GPUI.

```text
core (markit-core)               gpui layer (markit-gpui/app)
─────────────────────────────    ────────────────────────────
document                         window
edit model / EditTransaction     rendering / presentation
selection                        native text (shaping)
undo / redo                      keyboard
markdown / block index           pointer
incremental invalidation         IME
viewport / LOD model             clipboard
revision / dirty model           file dialogs
commands                         frame request / present edge
```

GPUI itself already abstracts a large amount of OS behavior. Do not
create duplicate wrapper abstractions unless Markit semantics need them
(ADR-002). Platform integration belongs at the GPUI edge.

The capability concepts from the A1–A4 era (`ClipboardProvider`,
`ImeProvider`, `FontProvider`, `FileDialogProvider`, `ShortcutPolicy`,
`PlatformPaths`) remain useful as **semantic boundaries** where GPUI does
not already provide the needed semantics — they must not exist merely
because a PocketJS svc adapter needed them. Prefer the smallest meaningful
abstraction.

## 3. Where does the document live?

In `markit-core`'s `Document`, owned by the model layer — never inside
the GPUI element tree. GPUI entities are a **projection** of the model,
never the canonical Markdown document model.

The document is a plain string plus the incremental `LineIndex`
(one full scan at load, local updates per edit — ADR-003). Future buffer
structures (piece table, rope, tree-based) are a decision for the real
product workload, not a pre-emptive choice.

**Coordinate semantics** (AGENTS.md §8): keep bytes / Unicode scalars /
grapheme boundaries / logical positions / display positions / platform
UTF-16 coordinates explicit as Unicode levels rise (U1+ CJK). Avoid
ambiguous `charOffset`-style APIs.

## 4. How does Markdown parsing become incremental?

The A4-R2 pipeline (proven on the PocketJS-era seed, see
`docs/research/pocketjs-mvp-knowledge-transfer.md`) is the blueprint for
the Rust core:

```text
Document → Block Index → Incremental Parse → Affected Blocks
         → Styled Runs → Visible Layout → GPUI presentation
```

- The **Block Index** maps lines to L1 blocks (heading, paragraph, quote,
  ulist/olist, fenced, blank). `applyEdit(startLine, endLine, ...)`
  rescans from the first affected block forward and stops at the first
  stable boundary (kind + alignment match beyond the edited lines),
  carrying fence state. The consumed range is the structural
  invalidation radius (measured: 1 block for local edits at any size;
  the full fence cascade for fence-boundary edits — a known product cost
  to bound, see §9).
- **Styled runs** are computed per affected block (inline parse, cached
  by block start line, invalidated for exactly the replaced blocks) and
  sliced per visible line.
- The full-document scan is the load-time and test-oracle path only.

Incrementality is necessary but not sufficient. A fast incremental parser
can still cause bad frames if every changed result synchronously fans out
into layout, highlighting, rich-block work, and presentation. The next
section defines the execution model that constrains that fan-out.

## 5. Real-time execution model: incremental + non-blocking

Markit treats editing as a **continuous real-time workload**, not as a
single "update everything" function call.

The detailed design lives in
`docs/product/realtime-execution-model.md`. The architectural contract is:

```text
OS / GPUI input
      ↓
EditTransaction + revision
      ↓
explicit changed range
      ↓
precise dirty propagation
      ↓
BlockIndex / Markdown IR
      ↓
priority + cancellable derived work
      ↓
Viewport / Document LOD
      ↓
visible layout / shaping
      ↓
coherent committed presentation
      ↓
frame-budgeted GPUI work
```

The design laws are:

1. **Stable work is not repeated.** A local edit invalidates the smallest
   semantically valid region; unrelated parse/layout/render artifacts are
   reused.
2. **Invisible work is deferred.** Current interaction and visible
   viewport outrank near-viewport work; distant work is background.
3. **Deferrable work yields.** The UI path is not allowed to monopolize a
   frame merely to empty a queue. Exact numeric budgets are calibrated by
   measurement, not copied from another system.
4. **Stale work is disposable.** Derived jobs are revision-aware;
   obsolete work is cancelled when profitable or rejected at commit.
5. **Only coherent state is published.** Presentation must not silently
   mix incompatible document/parse/layout/highlight revisions.
6. **Demand-driven idle.** No permanent game-style tick exists; when
   nothing changes, Markit should do almost nothing.

The useful game-engine ideas are dirty flags, frame budgets, job priority,
visibility culling, LOD, coherent publication, and caches. Markit does
**not** adopt ECS, archetype storage, a scene graph as the canonical
document model, or a fixed-rate update loop by default.

## 6. How does the UI consume only visible state?

The view model (markit-core) computes exactly the visible line range
(viewport formula + overscan), and the GPUI layer renders only those
lines. Frame work is viewport-bounded whenever semantics permit
(ADR-005):

- materialized GPUI elements / shaped text / paint work scale with the
  **visible presentation**, not the total document size;
- the idle editor must not continuously request frames;
- the document may be huge; the frame must not be.

The viewport rule extends into **Document LOD**:

```text
far       → source range + block metadata + estimated extent
near      → parsed/lightweight derived state
visible   → exact layout + shaped text
presented → render primitives / GPU-facing state
```

These are semantic levels, not mandatory concrete structs. The point is
that derived-state materialization is proportional to user observability.
Height estimation/correction must preserve logical scroll extent and be
measured for scroll drift.

The A4-R1 stateless-projection discipline transfers as a principle:
per-line presentation derives statelessly from the model's visible
range, and identity for any stateful line widget must be a stable
block/content ID, not the absolute line number.

## 7. How is asynchronous work kept correct?

Background or deferred work (parser follow-ups, highlighting, indexing,
image decode, math/diagram projection, or later layout work) must be tied
to an explicit document/semantic revision.

Conceptually:

```text
Committed presentation v101  ← safe to show
Working derived state v102   ← jobs in flight
New edit creates v103        ← v102 work must prove reuse or become stale
```

A stale job never earns the right to commit merely because it already
consumed CPU. Markit either cancels it or rejects the stale result at the
commit boundary.

"Snapshot" means a coherent version boundary for derived presentation;
it does **not** require copying the full document. Caches may reuse older
artifacts only when their dependencies prove compatibility with the
current revision.

## 8. IME / clipboard / fonts / file dialogs at the GPUI edge

- **IME** is a Tier-0 editor-model concept (ADR-007): composition
  start/update/commit/cancel have distinct semantics, composition never
  enters the undo stack as keystrokes, commit is one undo transaction,
  candidates dock at the caret rect. The model side lives in markit-core;
  the platform path is GPUI's IME integration (IMM32/TSF on Windows).
  Chinese IME is validated first (P1), JA/KO architecture present.
- **Clipboard** is a Tier-0 capability: text-only copy/cut/paste first;
  rich HTML/images/custom MIME deferred. Validate GPUI's Windows
  clipboard in Markit; require runtime evidence.
- **Fonts**: validate GPUI/DirectWrite CJK + emoji fallback for Markit
  (system font discovery + fallback chain). Do not assume GPUI already
  satisfies acceptance.
- **File dialogs**: native dialogs via the GPUI/platform edge; the MVP
  may start with a minimal path input until native dialogs land.

These platform services do not get to bypass the real-time execution
contract. For example, IME commit is critical visible work; spellcheck or
rich clipboard post-processing is not.

## 9. How does the view model stay bounded and testable?

- A deterministic/headless host (or core unit tests) validates
  correctness, algorithmic scaling, dirty propagation, revision
  compatibility, queue policy, and controlled interventions — it is
  **not** evidence of real desktop interaction latency (AGENTS.md §11).
- Real OS hosts are required for claims about input delivery, IME, fonts,
  scheduling, compositor behavior, GPU/presentation, and actual frame
  budget calibration.
- Instrumentation must make work amplification visible: changed bytes /
  lines / blocks, blocks rescanned/reparsed, layout lines, visible/near/far
  materialization, frame work, yielded work, stale jobs, cache hit/miss,
  and long frames.

## 10. How do future rich Markdown blocks join without breaking the hot path?

1. **Explicit change-range propagation**: every edit carries its changed
   range; every layer consumes the range, never the whole document.
2. **Block-granular invalidation**: a new block kind registers its
   classifier + inline parser + style mapper; the incremental rescan
   treats it like any other kind. The block index stays line-based so
   the stable-boundary logic keeps working.
3. **Viewport-bounded presentation**: rich blocks render only in the
   visible range; heavy projections (images, syntax highlight, math,
   diagrams) are computed lazily, assigned observable priority, cached,
   and made cancellable or stale-result-safe.
4. **Structural-edit cost is owned**: block kinds whose edits can
   invalidate broadly (fences today; tables with row-span semantics
   later) must document their invalidation radius and provide a bounded
   recovery strategy. The fence cascade (30K lines at 1M, measured) is
   the first such case — bound fence recovery (e.g. only treat ``` as an
   opener when a matching close exists ahead) and re-measure.
5. **No synchronous rich-block tax on typing**: expensive projections
   cannot become mandatory critical-path work merely because a block is
   present elsewhere in the document.
6. **Regression gates**: the work-amplification and scheduling invariants
   (`docs/product/performance-invariants.md`) are checked by the
   regression battery, not only by wall-clock thresholds.

## 11. Reference systems are lessons, not dependencies

Markstream (`Simon-He95/markstream-vue`) is now an explicit reference for
streaming Markdown scheduling. Its current renderer demonstrates useful
ideas such as incremental batches, adaptive work based on measured render
cost, idle/frame-boundary scheduling, append-vs-replacement semantics,
and document virtualization.

Markit borrows those **execution principles**, not Vue, DOM, VDOM, or its
numeric defaults. Direct GPUI lets Markit make dirty ranges, revisions,
viewport priority, and publication boundaries explicit in the editor
model.

The same rule applies to Zed and game engines: reference implementations
are hypotheses and design vocabulary, not proof.

## 12. Reference documents

- Real-time execution model:
  `docs/product/realtime-execution-model.md`.
- Substrate decision: `docs/adr/ADR-008-direct-gpui-product-substrate.md`
  (supersedes ADR-001).
- Core/platform boundary: `docs/adr/ADR-002-core-platform-boundary.md`.
- Editor principles: ADR-003 (document + line index), ADR-004
  (incremental Markdown invalidation), ADR-005 (viewport-bounded
  rendering), ADR-006 (command/shortcut abstraction), ADR-007 (IME
  composition model).
- Invariants: `docs/product/performance-invariants.md`.
- Capability matrix: `docs/product/platform-capability-matrix.md`.
- MVP scope: `docs/product/mvp-v0.1.md`.
- Roadmap: `docs/product/roadmap.md`.
- Historical evidence: `docs/research/README.md`.
