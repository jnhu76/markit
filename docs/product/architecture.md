# Markit Desktop — Product Architecture

Status: frozen enough to implement (A4-P). This document answers the ten
architecture questions of the A4 phase spec (§53). The PocketJS decision
and GPUI status are documented in `docs/phase-a4-final-research-closeout.md`.

## 1. Why PocketJS?

1. **Guest-side product logic.** Editor behavior (document, commands,
   Markdown pipeline, view model) lives in JS on the PocketJS stack:
   iterate without native rebuilds, ship one bundle across platforms.
2. **The measured cost is understood and small.** A4-R decomposed the
   residual floor: the PocketJS lower rendering stack is ~0.3 ms for a
   one-character edit; the Solid reconstruction term was Markit's usage
   (stable-item fix → ~1.0–1.5 ms, viewport-bound, addressable by the
   Incremental View Model).
3. **Every measured bottleneck so far was Markit-owned** (A2 scan, A3
   line index, A4 Solid usage) and fixed in Markit code;
   `vendor/pocketjs` is unchanged across A2–A4.
4. **One retained tree + DrawList contract** for Windows/Linux/macOS,
   with an explicit host/svc boundary for platform work.

GPUI remains the reference/performance oracle. Its measured advantages
(active-edit latency, startup, memory) are documented in the A3 report;
the product decision rests on iteration speed and architecture control,
not on a benchmark win.

## 2. What is GPUI's role now?

Reference / performance oracle. When a PocketJS product number needs a
substrate sanity check (e.g. "is this Markdown parsing cost or
integration cost?"), a minimal GPUI control may be built; no GPUI
product backend is developed.

## 3. Where does the document live?

In `markit-core`'s `Document` (framework-free TS), owned by the model
layer — never inside the PocketJS component tree. The document is a
plain string plus the incremental `LineIndex` (one full scan at load,
local updates per edit — A3-P1). Future buffer structures (piece table,
rope, tree-based) are a decision for the real product workload, not a
pre-emptive choice (A4 phase spec §20, Layer 1).

Offset semantics: the current code uses JS code units. The product core
must keep the byte / scalar / grapheme / logical / display / UTF-16
distinctions explicit (AGENTS.md §8) as Unicode levels rise (U1+ CJK).

## 4. How does Markdown parsing become incremental?

The R2 pipeline (`app/markdown.ts`) is the seed of the product engine:

```text
Document → Block Index → Incremental Parse → Affected Blocks
         → Styled Runs → Visible Layout → DrawList
```

- The **Block Index** maps lines to L1 blocks (heading, paragraph, quote,
  ulist/olist, fenced, blank). `applyEdit(startLine, endLine, ...)`
  rescans from the first affected block forward and stops at the first
  stable boundary (kind + alignment match beyond the edited lines),
  carrying fence state. The consumed range is the structural
  invalidation radius (measured: 1 block for local edits at any size;
  the full fence cascade for fence-boundary edits — a known product cost
  to bound, see §10).
- **Styled runs** are computed per affected block (inline parse, cached
  by block start line, invalidated for exactly the replaced blocks) and
  sliced per visible line.
- The full-document scan is the load-time and test-oracle path only.

## 5. How does the UI consume only visible state?

The view model (Layer 4) computes exactly the visible line range
(GPUI's paint formula: first line from scroll + one viewport + overscan),
and the UI renders only those lines. A4-R1 added the discipline that
makes this cheap in Solid: visible-list items carry **stable identity**
and doc reads are **hoisted into item-scoped memos**, so a document
change re-evaluates the visible memos (26× per edit) without re-mounting
components (~90 native node creations per edit before the fix).

**Stateless-projection invariant (A4 review gate):** visible-line items
are keyed by their **absolute document line number** and carry **no
state** — every doc-dependent read happens inside item-scoped memos.
Absolute line numbers are *not* stable document identity (inserting a
`\n` before the viewport shifts every later line), so correctness comes
exclusively from stateless re-derivation, never from stored per-item
state. If a future line widget needs state (IME, folding, inline
widgets), identity must move to a stable block/content ID, not the
absolute line number. The invariant and its regression tests live in
`mvp/pocketjs/app/view-slots.ts` + `view-slots.test.ts`; the identity
cache grows with the highest line ever visible (bounded eviction is a
backlog item once lines gain state).

## 6. Where is the boundary between the PocketJS component tree and the
document model?

The component tree is a **projection** of the model, never the model
itself:

- the document, line index, block index, selection, caret and commands
  live in `markit-core` (framework-free);
- components read derived view-model state (visible lines, runs) and
  emit ui.* ops; the DrawList is the only rendering authority;
- no editor state lives in Solid component state (A4 phase spec §20,
  Layer 3).

## 7. How does the DrawList stay viewport-bound?

- Only visible lines are laid out and drawn (viewport formula + overscan,
  measured: 25 lines shaped at 1M, A3).
- DrawList size scales with the visible presentation, not the document
  (measured: 3046 words at 10K and 1M).
- Demand rendering: the host redraws only when the DrawList hash
  changes (idle: ~0 frames/s).
- The model's remaining O(lines-after) suffix shift is instrumented and
  position-dependent; a buffer redesign is a real-workload decision.

## 8. Where are the Windows/Linux/macOS differences isolated?

In thin, capability-driven platform adapters behind explicit
interfaces (`docs/adr/ADR-002`). The core names capabilities, not OSes:

```text
ClipboardProvider   copy/cut/paste (text first)
FontProvider        system font discovery + fallback chain + runtime glyph
ImeProvider         composition start/update/commit/cancel
FileDialogProvider  open/save dialogs (native)
ShortcutPolicy      Command ←→ platform binding (Ctrl vs Cmd)
PlatformPaths       config/cache/recovery/logs/recent files
```

No `if (platform === "windows")` in the core. The host/svc boundary
(`mvp/pocketjs/src/main.rs` + `app/svc.ts`) is the socket where desktop
capabilities enter; new capabilities are added as svc messages + adapter
implementations, not core branches.

**Ownership of the implementations (A4 review decision):** `app/platform.ts`
is a **Markit-facing interface sketch** — it names the capabilities the
core needs and is deliberately not an OS implementation layer. The
generic OS implementations (Windows GDI/Linux fontconfig/macOS CoreText
font discovery, clipboard backends, IME host binding, native dialogs)
belong to **PocketJS** (Desktop Enablement, the immediate next phase).
Markit must not grow a private Windows/Linux/macOS compatibility layer
behind these interfaces: the contract is the seam where
PocketJS-provided implementations plug in, and Markit's P1 acceptance
explicitly checks that no such private layer was introduced.

## 9. How are IME / clipboard / fonts / file dialogs abstracted?

- **IME** is a Tier-0 editor-model concept: composition start/update/
  commit/cancel, composition text never enters the undo stack as
  keystrokes, the host docks candidates at the caret rect (the wire
  contract exists since A1: `{t:"ime"}` + `{t:"caret"}`). The product
  must implement the model side and validate Windows IME (P1).
- **Clipboard** is a Tier-0 capability: text-only copy/cut/paste via
  `ClipboardProvider`; rich HTML/images/custom MIME deferred.
- **Fonts**: system font discovery per platform (Windows GDI
  enumeration, Linux fontconfig, macOS CoreText), a fallback chain
  (Latin → CJK → emoji), runtime glyph extension for text outside the
  baked atlases (`text.glyphs.runtime` — already a macos-widget
  capability; the desktop hosts must implement it).
- **File dialogs**: native dialogs per platform behind
  `FileDialogProvider`; the MVP may start with a minimal path input
  until P1 native dialogs land.

## 10. How do future rich Markdown blocks (tables, images, math, Mermaid)
join without breaking the hot path?

The L1 rules of the road stay:

1. **Explicit change-range propagation**: every edit carries its changed
   range; every layer consumes the range, never the whole document.
2. **Block-granular invalidation**: a new block kind registers its
   classifier + inline parser + style mapper; the incremental rescan
   treats it like any other kind. The block index must stay line-based
   so the stable-boundary logic keeps working.
3. **Viewport-bounded presentation**: rich blocks render only in the
   visible range; heavy projections (images, syntax highlight) are
   computed lazily per visible block and cached, with explicit
   invalidation on edit.
4. **Structural-edit cost is owned**: block kinds whose edits can
   invalidate broadly (fences today; tables with row-span semantics
   later) must document their invalidation radius and provide a bounded
   recovery strategy. The R2 fence cascade (30K lines at 1M) is the
   first such case — the product should bound fence recovery (e.g. only
   treat ``` as an opener when a matching close exists ahead) and
   re-measure.
5. **Regression gates**: the work-amplification invariants
   (`docs/product/performance-invariants.md`) are checked by the
   regression battery (`bench/run-a4.py`), not by wall-clock thresholds.

## 11. Runtime identity and targets

The Markit desktop product does **not** impersonate PocketJS's
`macos-widget` target. Per the A4 phase spec (§26–§27): the target model
is being designed as `markit-desktop` (product identity) over platform
capability profiles (windows / linux / macos), with identity separated
from capability. The vendor registry change is an upstream PocketJS
proposal (Markit-independent reproduction + regression test), tracked in
`docs/product/issue-backlog.md`.

## 12. Reference documents

- Research closeout + PocketJS decision: `docs/phase-a4-final-research-closeout.md`
- ADRs: `docs/adr/` (001 PocketJS substrate, 002 core/platform boundary,
  003 incremental document+line index, 004 incremental Markdown
  invalidation, 005 viewport-bounded rendering, 006 command/shortcut
  abstraction, 007 IME composition model)
- Invariants: `docs/product/performance-invariants.md`
- Capability matrix: `docs/product/platform-capability-matrix.md`
- MVP scope: `docs/product/mvp-v0.1.md`
- Roadmap: `docs/product/roadmap.md`
