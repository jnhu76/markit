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
viewport model                   clipboard
commands                         file dialogs
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
  to bound, see §8).
- **Styled runs** are computed per affected block (inline parse, cached
  by block start line, invalidated for exactly the replaced blocks) and
  sliced per visible line.
- The full-document scan is the load-time and test-oracle path only.

## 5. How does the UI consume only visible state?

The view model (markit-core) computes exactly the visible line range
(viewport formula + overscan), and the GPUI layer renders only those
lines. Frame work is viewport-bounded whenever semantics permit
(ADR-005):

- materialized GPUI elements / shaped text / paint work scale with the
  **visible presentation**, not the total document size;
- the idle editor must not continuously request frames;
- the document may be huge; the frame must not be.

The A4-R1 stateless-projection discipline transfers as a principle:
per-line presentation derives statelessly from the model's visible
range, and identity for any stateful line widget must be a stable
block/content ID, not the absolute line number.

## 6. IME / clipboard / fonts / file dialogs at the GPUI edge

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

## 7. How does the view model stay bounded and testable?

- A deterministic/headless host (or core unit tests) validates
  correctness, algorithmic scaling, and controlled interventions — it is
  **not** evidence of real desktop interaction latency (AGENTS.md §11).
- Real OS hosts are required for claims about input delivery, IME, fonts,
  scheduling, compositor behavior, and GPU/presentation.

## 8. How do future rich Markdown blocks join without breaking the hot path?

1. **Explicit change-range propagation**: every edit carries its changed
   range; every layer consumes the range, never the whole document.
2. **Block-granular invalidation**: a new block kind registers its
   classifier + inline parser + style mapper; the incremental rescan
   treats it like any other kind. The block index stays line-based so
   the stable-boundary logic keeps working.
3. **Viewport-bounded presentation**: rich blocks render only in the
   visible range; heavy projections (images, syntax highlight) are
   computed lazily per visible block and cached, with explicit
   invalidation on edit.
4. **Structural-edit cost is owned**: block kinds whose edits can
   invalidate broadly (fences today; tables with row-span semantics
   later) must document their invalidation radius and provide a bounded
   recovery strategy. The fence cascade (30K lines at 1M, measured) is
   the first such case — bound fence recovery (e.g. only treat ``` as an
   opener when a matching close exists ahead) and re-measure.
5. **Regression gates**: the work-amplification invariants
   (`docs/product/performance-invariants.md`) are checked by the
   regression battery, not by wall-clock thresholds.

## 9. Reference documents

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
