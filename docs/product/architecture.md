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
                  private projections
                           │
          ┌────────────────┴────────────────┐
          │                                 │
          ▼                                 ▼
┌──────────────────────┐          ┌──────────────────────┐
│   markit-gpui/app     │          │ Plugin API Boundary  │
│                      │          │                      │
│ window / rendering   │          │ snapshots / queries  │
│ native text / IME    │          │ commands / results   │
│ input / clipboard    │          │ stable ids/revisions │
│ file dialogs         │          │ capabilities/version │
└──────────┬───────────┘          └──────────┬───────────┘
           │                                  │
           ▼                                  ▼
         GPUI                         future plugin runtime
           │                         (transport not chosen)
  ┌────────┼────────┐
  ▼        ▼        ▼
Windows  Linux    macOS
```

The Plugin API Boundary is semantic, not a frozen Rust ABI. Internal
representation may change without becoming a plugin-breaking event.

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

A separate extension boundary exists beside the GPUI boundary. Plugins do
not become another way to reach internal core/platform objects; they consume
stable semantic snapshots/queries and return documented commands/results.

## 3. Where does the document live?

In `markit-core`'s `Document`, owned by the model layer — never inside
the GPUI element tree. GPUI entities are a **projection** of the model,
never the canonical Markdown document model.

The document is a plain string plus the incremental `LineIndex`
(one full scan at load, local updates per edit — ADR-003). Future buffer
structures (piece table, rope, tree-based) are a decision for the real
product workload, not a pre-emptive choice.

Implemented as P0-01 in `crates/markit-core::document` (storage private;
reads go through byte/line queries; every mutation returns an explicit
changed-range `EditResult` at mutation time).

**Coordinate semantics** (AGENTS.md §8): keep bytes / Unicode scalars /
grapheme boundaries / logical positions / display positions / platform
UTF-16 coordinates explicit as Unicode levels rise (U1+ CJK). Avoid
ambiguous `charOffset`-style APIs.

Document storage representation is private. A future plugin cannot depend on
`String`, Rope, Piece Table, tree node, or raw pointer identity being stable.

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
  to bound, see §10).
- **Styled runs** are computed per affected block (inline parse, cached
  by block start line, invalidated for exactly the replaced blocks) and
  sliced per visible line.
- The full-document scan is the load-time and test-oracle path only.

Incrementality is necessary but not sufficient. A fast incremental parser
can still cause bad frames if every changed result synchronously fans out
into layout, highlighting, rich-block work, and presentation. The next
section defines the execution model that constrains that fan-out.

The internal Markdown IR is allowed to evolve for performance/correctness.
Future plugins that need Markdown semantics receive a versioned public
semantic view/snapshot; the concrete internal IR Rust layout is not the plugin
contract.

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

Plugin work inherits the same law: a slow extension cannot make normal typing
wait on unbounded work. Extension results carry revision/identity information
and stale results are rejected before mutation/publication.

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

Stable block/content IDs are also the natural extension identity. Plugins must
not treat a current Vec index, line number, or GPUI Entity ID as durable
content identity.

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

The same model applies to plugins:

```text
plugin receives snapshot revision 412
        ↓
plugin computes
        ↓
document reaches revision 415
        ↓
plugin result(base_revision=412)
        ↓
Markit validates → reuse/rebase if explicitly supported, otherwise reject
```

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

A future plugin that needs filesystem/network/clipboard authority receives an
explicit capability rather than direct access merely because Markit itself has
that platform service.

## 9. How does the view model stay bounded and testable?

- A deterministic/headless host (or core unit tests) validates
  correctness, algorithmic scaling, dirty propagation, revision
  compatibility, queue policy, and controlled interventions — it is
  **not** evidence of real desktop interaction latency (AGENTS.md §12).
- Real OS hosts are required for claims about input delivery, IME, fonts,
  scheduling, compositor behavior, GPU/presentation, and actual frame
  budget calibration.
- Instrumentation must make work amplification visible: changed bytes /
  lines / blocks, blocks rescanned/reparsed, layout lines, visible/near/far
  materialization, frame work, yielded work, stale jobs, cache hit/miss,
  and long frames.

When a stable plugin API exists, compatibility testing adds a second form of
bounded testability: representative old-plugin fixtures must execute against
new hosts, not merely compile against the newest SDK.

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

Rich-block/provider APIs that later become extensible expose stable semantic
inputs/outputs, not GPUI render nodes or internal Markdown IR ownership.

## 11. How do plugins survive Markit updates?

The detailed contract lives in
`docs/product/plugin-compatibility-contract.md`. The architecture law is:

```text
private Markit implementation
        ↓ adapter
versioned semantic Plugin API Boundary
        ↓ negotiated capability
plugin
```

Never:

```text
plugin → markit-core private structs
plugin → GPUI Entity tree
plugin → scheduler/cache implementation
plugin → Rust memory layout of Markdown IR
```

### 11.1 Semantic API before runtime ABI

Markit freezes the concepts and compatibility semantics before choosing a
transport. The same public operation should remain meaningful whether the
runtime eventually uses an in-process adapter, Wasm, subprocess/IPC, or a
hybrid.

This lets Markit change buffer storage, parser data structures, GPUI versions,
scheduler internals, or caches without converting every internal refactor into
a plugin ecosystem migration.

### 11.2 Version + capability negotiation

Plugin loading begins with an explicit API major/minor and capability
negotiation. Additive features are optional by default. Required missing
capabilities fail closed with an explanation. Breaking semantic changes require
a compatibility boundary/API major change rather than silent reinterpretation.

### 11.3 Snapshot/query + command/result

Plugins read coherent snapshots or explicit queries. Mutations return as
commands/transactions so Markit remains the authority for undo, dirty
propagation, revision checking, IME/input invariants, and scheduling.

Print/PDF is the model case:

```text
DocumentSnapshot + Public Markdown Semantic View
        ↓
Print provider
        ↓
Print/Export result
```

It does not require the provider to inspect the live GPUI render tree.

### 11.4 Compatibility is a CI property

Once the stable plugin API exists, Markit retains old-plugin fixtures and runs
them against new hosts. Version declarations alone do not prove compatibility.
A host update is not extension-compatible until representative supported old
plugins still perform their documented operations or are rejected through a
documented compatibility boundary.

### 11.5 MVP restraint

P0/P1 preserve the seam but do not build a general plugin framework. A plugin
runtime is triggered by real extension workloads, not by architecture
enthusiasm. See roadmap PX.

## 12. Reference systems are lessons, not dependencies

Markstream (`Simon-He95/markstream-vue`) is now an explicit reference for
streaming Markdown scheduling. Its current renderer demonstrates useful
ideas such as incremental batches, adaptive work based on measured render
cost, idle/frame-boundary scheduling, append-vs-replacement semantics,
and document virtualization.

Markit borrows those **execution principles**, not Vue, DOM, VDOM, or its
numeric defaults. Direct GPUI lets Markit make dirty ranges, revisions,
viewport priority, and publication boundaries explicit in the editor
model.

The same rule applies to Zed, game engines, and plugin ecosystems: reference
implementations are hypotheses and design vocabulary, not proof. Do not copy a
plugin loading model merely because another product uses it.

## 13. Reference documents

- Real-time execution model:
  `docs/product/realtime-execution-model.md`.
- Plugin compatibility contract:
  `docs/product/plugin-compatibility-contract.md`.
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
