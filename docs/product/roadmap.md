# Markit — Product Roadmap

Phase A* research naming ends with A4 (research phase closed). The
product phases below replace it. Each phase lists goal / scope /
acceptance / non-goals.

The cross-cutting execution model is now defined in
`docs/product/realtime-execution-model.md`:

```text
stable work is reused
invisible work is deferred
budget-exhausting work yields
stale work is cancelled/rejected
only coherent revision-compatible state is published
idle means no permanent update loop
```

The cross-cutting extension compatibility model is defined in
`docs/product/plugin-compatibility-contract.md`:

```text
plugins depend on a versioned semantic contract, not internals
capabilities are negotiated explicitly
stable opaque identity crosses the boundary
plugins consume snapshots/queries and submit commands/results
supported old plugins are exercised by compatibility fixtures
plugin latency/failure cannot poison the input hot path
```

These are product constraints, not separate optimization/framework phases.
Every phase that touches the hot path or future extension boundary must
preserve them.

```text
Architecture Pivot / Repository Cleanup   (ADR-008)
        ↓
G0  GPUI Baseline Selection
        ↓
P0  Markit Rust Core + Change/Revision Model
        ↓
P1  Windows Editor MVP + Realtime Render Scheduling + Markdown L1 (v0.1)
        ↓
P2  Incremental/Scheduling Hardening
        ↓
P3  Typora-style L2
        ↓
P4  Rich Blocks / Heavy Projection Jobs
        ↓
PX  Plugin Runtime (evidence-triggered, not calendar-triggered)
        ↓
P5  Cross-platform / Shipping Hardening
```

`PX` is intentionally conditional: the semantic extension boundary is
preserved from P0 onward, but a general plugin runtime is built only when
real extension workloads justify a concrete transport/runtime choice.

## Cross-cutting gate — Real-time execution laws

Before a product phase is considered complete, relevant hot-path changes
must answer:

```text
What changed?
What became dirty?
What is visible now?
What may be deferred?
What revision does this work belong to?
What makes stale work safe to cancel/reject?
What is the publication boundary?
What work is cached/reused?
What is the measured frame/interaction cost?
```

The acceptance laws are the invariants in
`performance-invariants.md`, especially INV-08 through INV-14.

Do not add a permanent frame/timer loop simply because the scheduler uses
game-engine-inspired techniques. Markit remains demand-driven.

## Cross-cutting gate — Extension compatibility laws

Before exposing a new extension-facing surface or changing an existing one,
answer:

```text
Is this public semantic API or an internal detail?
Is the change additive or breaking?
Which plugin API major/minor owns it?
Can an old supported plugin ignore the addition safely?
Which capability exposes it?
What stable identity/revision crosses the boundary?
Does the plugin receive a coherent snapshot/query rather than mutable internals?
Does mutation return through an explicit command/transaction?
What is the deprecation/migration path?
Which old-plugin compatibility fixture proves the claim?
Can plugin latency/crash block ordinary typing?
```

Do not expose `markit-core` Rust layout, GPUI entities, scheduler/cache
internals, or concrete Markdown IR memory representation as accidental plugin
ABI. Transport/runtime remains evidence-driven.

## G0 — GPUI Baseline Selection (next)

- Goal: select and pin a GPUI baseline suitable for Windows development
  and for a demand-driven, frame-observable editor scheduler.
- Scope: evaluate the current `mvp/gpui` prototype (gpui 0.2.2) and a
  current Zed GPUI revision on:

  ```text
  build
  release build
  window
  native text
  CJK
  IME
  clipboard
  resize
  HiDPI
  startup
  RSS
  basic latency
  frame request / redraw semantics
  access to clocks / scheduling hooks needed for instrumentation
  feasibility of deferred/cancellable work without blocking presentation
  ```

- Do not choose merely because newer is newer. Record evidence per item.
- The pinned GPUI revision is a dependency of the GPUI-facing crates
  (`markit-gpui` / the app crate, via the workspace dependency) — never
  of `markit-core` (P0 acceptance: core has no GPUI dependency).
- Acceptance: a documented, evidence-based GPUI baseline selection with
  build + Windows runtime evidence; the product's GPUI dependency (in
  `markit-gpui` / the app crate, not `markit-core`) is frozen to the
  selected revision; the selected baseline has enough scheduling and
  redraw observability to implement P1 without inventing an opaque busy
  loop.
- Non-goals: editor features, worker-topology tuning, Linux/macOS hosts,
  plugin runtime/transport decisions.

## P0 — Markit Rust Core + Change/Revision Model

Status note (2026-08): **P0-01 is implemented** — the product Rust
workspace (`crates/markit-core` + `apps/markit` skeleton, no GPUI yet),
the incremental document core (private storage, revision/change model,
incremental LineIndex per ADR-003, selection / edit-transaction /
snapshot seams, stable `DocumentId`), and the differential test battery
including work-amplification counters. The remaining P0 scope below
(BlockIndex, Markdown L1, commands, view model) is open. See
`docs/product/p0-01-implementation-note.md`.

- Goal: framework-independent core built as a Rust library whose update
  semantics are explicit enough for incremental, cancellable, coherent
  presentation and future stable extension snapshots/commands.
- Scope:

  ```text
  Document
  LineIndex (ADR-003)
  Selection
  EditTransaction / undo / redo
  Commands (ADR-006)
  BlockIndex
  Markdown L1 (ADR-004)
  view model
  explicit changed-range propagation
  document/derived-state revision identity
  stable document/block identity where semantics require it
  dirty/invalidation model
  viewport / near / far semantic priority model
  Unicode-aware coordinate semantics (AGENTS.md §8)
  instrumentation seams
  regression battery
  ```

- Required semantic distinctions must not collapse into one generic
  "document changed" path. The core must preserve enough information to
  distinguish local edit / append / delete / paste / structural edit /
  document replacement / viewport change / style change where those
  distinctions affect invalidation.
- Public extension semantics are **not** defined by simply marking core Rust
  types `pub`. Future plugin access must go through an adapter/contract layer.
- Acceptance:
  - core tests green (incl. randomized differentials vs full-scan oracles,
    caretFromX regression);
  - local edits invalidate the smallest semantically valid block region;
  - revision tests prove stale derived results cannot commit over newer
    state;
  - dirty propagation is testable without GPUI;
  - work-amplification counters exist for changed bytes/lines/blocks and
    blocks rescanned/reparsed;
  - stable identity/revision semantics are sufficient to construct coherent
    read-only snapshots without exposing storage pointers/indexes;
  - commands/transactions provide the mutation seam future plugins can reuse;
  - invariants battery defined for INV-01/05/08/10/11/13;
  - core has no GPUI dependency.
- Non-goals: final buffer choice, worker thread pool design, platform
  integration, packaging, public plugin ABI/runtime.

## P1 — Windows Editor MVP + Realtime Render Scheduling + Markdown L1 (v0.1)

- Goal: Markit Desktop v0.1 runs on Windows, built directly on GPUI: a
  single-document, L1-styled Markdown editor whose critical interaction
  path is incremental, viewport-bounded, and non-blocking.
- Product scope (from `docs/product/mvp-v0.1.md`): window, editing,
  Markdown L1 styled pipeline (ADR-004: heading, paragraph, bold,
  emphasis, inline code, link, blockquote, ul/ol list, fenced code), IME
  (Chinese composition model, ADR-007), clipboard (text), files
  (open/save/atomic save, crash recovery minimal), CJK + emoji fallback,
  undo/redo, shortcuts (Ctrl), resize/HiDPI, large-document stability
  (1M+ flat), regression battery in CI.
- Execution-model scope:

  ```text
  demand-driven frame requests
  visible-range materialization
  near/far priority boundary
  coherent committed presentation revision
  bounded/cooperative scheduler for deferrable UI work
  revision-safe background/deferred job seam
  cache/invalidation seam for parsed/layout/shaped artifacts
  instrumentation for frame work, yields, queue depth, stale results
  ```

- Extension-boundary scope:

  ```text
  explicit snapshot/query seam
  explicit command/transaction mutation seam
  no extension-facing dependency on GPUI/private core representation
  built-in extension-like features use boundaries reusable by future plugins
  ```

- The exact numeric frame budget, worker count, batch size, overscan, and
  cache sizes are **not** specified in advance. Calibrate them with real
  Windows/GPUI input-to-present evidence and target refresh rates.
- Acceptance:
  - `docs/product/mvp-v0.1.md` gates PASS on Windows with evidence;
  - invariants battery green;
  - normal local edits remain Δ + viewport proportional rather than
    document proportional;
  - idle editor requests no continuous frames;
  - deferrable work can yield without blocking caret/input/visible text;
  - synthetic out-of-order jobs cannot publish stale state;
  - p50/p95/p99/max (where statistically meaningful) and long-frame
    counts are visible in real-host performance runs;
  - future extension semantics can be expressed without handing plugins
    mutable `Document`/IR/GPUI internals;
  - no GPUI code leaked into `markit-core`.
- Non-goals: tabs, images/tables/math, installer/MSIX (later if needed),
  Linux/macOS, generalized ECS/job framework, plugin runtime/marketplace.

## P2 — Incremental / Scheduling Hardening

- Goal: harden the L1 pipeline and real-time execution model to
  product-grade conformance, bounded recovery, and stable scheduling.
- Scope:
  - L1 conformance golden fixtures (CommonMark-derived where applicable);
  - bounded fence recovery + parser checkpoints (do not hide honest
    structural propagation);
  - incremental-parser robustness across large structural edits;
  - priority inversion / queue growth tests;
  - cancellation vs stale-result-rejection policy measurement;
  - cache-key/invalidation correctness tests;
  - Document LOD height estimation/correction and scroll-drift tests;
  - adaptive work chunking only if evidence shows a fixed policy causes
    long frames or under-utilization.
- Acceptance:
  - local-edit radius 1 block at any size;
  - fence edits bounded and documented;
  - differential oracle green;
  - conformance fixtures green;
  - stale/out-of-order jobs cannot corrupt presentation;
  - queue depth remains bounded under sustained typing/scroll workloads;
  - no unacceptable scroll jumps from LOD/height correction;
  - frame/interaction tail metrics and work-amplification counters show
    that background completion does not dominate visible interaction.
- Non-goals: syntax hiding (P3), copying Markstream's numeric defaults,
  building a generic game engine.

## P3 — Typora-style L2

- Goal: source-aware presentation (syntax hidden outside the active
  line, revealed on caret entry) without regressing the real-time
  execution laws.
- Scope: per-line syntax visibility, run-level virtualization, richer
  inline styling (weights/sizes), precise Style/Layout/Paint dirty
  propagation.
- Acceptance:
  - L2 editing measured against the full invariant battery;
  - local-edit radius still 1 block where semantics permit;
  - caret-line reveal is critical visible work;
  - offscreen syntax visibility changes do not force full-document layout
    or paint;
  - cached shaping/layout is reused when dependencies remain stable.
- Non-goals: rich blocks (P4).

## P4 — Rich Markdown Blocks / Heavy Projection Jobs

- Goal: images, tables, math, code highlighting (and later diagrams) as
  viewport-aware projections that cannot poison the typing hot path.
- Scope:
  - block-kind registry extension;
  - lazy per-visible/near-block projection;
  - versioned/cancellable or stale-result-safe heavy jobs;
  - cache + explicit invalidation;
  - lightweight fallback/placeholder presentation where appropriate;
  - memory bounds / eviction;
  - layout-stability measurement when a heavy result becomes ready;
  - built-in exporter/print/provider seams shaped so they can later map to
    versioned plugin capabilities without exposing render internals.
- Acceptance:
  - each block kind has a measured invalidation radius;
  - expensive projection does not synchronously run for distant blocks
    during normal typing;
  - stale heavy-job results cannot commit;
  - cache dependency/invalidation rules are documented and tested;
  - placeholder→final transitions stay within accepted scroll/layout
    stability bounds;
  - rich blocks stay out of the critical interaction path unless the
    current visible interaction genuinely requires them;
  - extension-like providers consume documented semantic inputs instead of
    GPUI element/layout internals.
- Non-goals: unrestricted plugin runtime / marketplace.

## PX — Plugin Runtime (evidence-triggered)

- Trigger: at least two materially different extension workloads need a
  distributable third-party boundary (for example Print/PDF plus an
  independent lint/export/provider class), and built-in-only seams no longer
  provide enough evidence.
- Goal: implement the smallest runtime that satisfies the already-defined
  semantic compatibility contract.
- Required evaluation before choosing runtime/transport:

  ```text
  failure isolation
  hot-path latency
  startup cost
  memory overhead
  cross-platform support
  dependency isolation
  security/capability enforcement
  upgrade compatibility
  debugging/developer experience
  packaging/signing implications
  ```

- Candidate transports may include in-process adapters, Wasm/component
  models, subprocess/IPC, or hybrids. None is preselected.
- Scope:
  - manifest + plugin identity/version;
  - API major/minor negotiation;
  - required/optional capability negotiation;
  - snapshot/query + command/result boundary;
  - stale-result/revision validation;
  - crash/hang/incompatibility handling;
  - compatibility fixtures with older supported plugins;
  - deprecation/migration machinery;
  - dependency isolation appropriate to the chosen runtime.
- Acceptance:
  - a host update that changes private Markit implementation details does
    not break representative supported old plugins;
  - incompatible plugins are disabled with an explicit reason, not crash;
  - missing optional capability degrades cleanly;
  - stale plugin results cannot overwrite newer state;
  - slow/crashed plugin cannot block ordinary typing indefinitely;
  - at least one compatibility test runs an old-plugin fixture against the
    new host in CI.
- Non-goals: marketplace economics, broad permission UX, arbitrary plugin
  capabilities not justified by real workloads.

## P5 — Cross-platform / Shipping Hardening

- Goal: shipping quality on Windows, then Linux/macOS while preserving
  the same semantic execution and extension compatibility contracts.
- Scope: Windows hardening first (packaging, crash reporting, recovery
  maturity, performance regression CI, accessibility basics, i18n); then
  Linux (Wayland/X11, fontconfig, IBus/Fcitx) and macOS (CoreText, IME,
  clipboard, Cmd, .app bundle) — each gated by the same MVP acceptance on
  real hardware.
- Each platform may use platform-specific scheduling/presentation fast
  paths if the common semantics remain intact. Do not force lowest-common-
  denominator timing mechanisms.
- Acceptance: release-ready on each platform it claims; each real host
  validates input/presentation semantics, idle demand rendering,
  cancellation/revision safety, viewport-bounded work, and the platform's
  calibrated frame/interaction budgets. If the plugin runtime has shipped,
  supported plugin compatibility must also hold across host platform updates.
- Non-goals: store submission specifics until a platform is release-ready.

## Working rule (the anti-foundation rule, from A4 §59)

> Once a foundation is sufficient for the next product feature, stop
> improving the foundation and build the product feature.

The real-time execution model and plugin compatibility contract do **not**
authorize an endless scheduler/engine/plugin-framework rewrite. Build the
smallest mechanism required by the next product phase, instrument it, measure
it, and keep it only if the product workload justifies it.

Real product workload is the only judge that can reopen research (e.g.
an A5-style investigation) or trigger PX — a synthetic benchmark or desire for
a generic ecosystem alone cannot. Do not let platform, scheduling, or plugin
framework work prevent building the editor once the Windows foundation is
adequate.
