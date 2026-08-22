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

These are product constraints, not a separate optimization phase. Every
phase that touches the hot path must preserve them.

```text
Architecture Pivot / Repository Cleanup   (ADR-008)
        ↓
G0  GPUI Baseline Selection
        ↓
P0  Markit Rust Core + Change/Revision Model
        ↓
P1  Windows Editor MVP + Realtime Render Loop + Markdown L1 (v0.1)
        ↓
P2  Incremental/Scheduling Hardening
        ↓
P3  Typora-style L2
        ↓
P4  Rich Blocks / Heavy Projection Jobs
        ↓
P5  Cross-platform / Shipping Hardening
```

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
- Non-goals: editor features, worker-topology tuning, Linux/macOS hosts.

## P0 — Markit Rust Core + Change/Revision Model

- Goal: framework-independent core built as a Rust library whose update
  semantics are explicit enough for incremental, cancellable, coherent
  presentation.
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
- Acceptance:
  - core tests green (incl. randomized differentials vs full-scan oracles,
    caretFromX regression);
  - local edits invalidate the smallest semantically valid block region;
  - revision tests prove stale derived results cannot commit over newer
    state;
  - dirty propagation is testable without GPUI;
  - work-amplification counters exist for changed bytes/lines/blocks and
    blocks rescanned/reparsed;
  - invariants battery defined for INV-01/05/08/10/11/13;
  - core has no GPUI dependency.
- Non-goals: final buffer choice, worker thread pool design, platform
  integration, packaging.

## P1 — Windows Editor MVP + Realtime Render Loop + Markdown L1 (v0.1)

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
  - no GPUI code leaked into `markit-core`.
- Non-goals: tabs, images/tables/math, installer/MSIX (later if needed),
  Linux/macOS, generalized ECS/job framework.

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
  - layout-stability measurement when a heavy result becomes ready.
- Acceptance:
  - each block kind has a measured invalidation radius;
  - expensive projection does not synchronously run for distant blocks
    during normal typing;
  - stale heavy-job results cannot commit;
  - cache dependency/invalidation rules are documented and tested;
  - placeholder→final transitions stay within accepted scroll/layout
    stability bounds;
  - rich blocks stay out of the critical interaction path unless the
    current visible interaction genuinely requires them.
- Non-goals: unrestricted plugin runtime / marketplace.

## P5 — Cross-platform / Shipping Hardening

- Goal: shipping quality on Windows, then Linux/macOS while preserving
  the same semantic execution contract.
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
  calibrated frame/interaction budgets.
- Non-goals: store submission specifics until a platform is release-ready.

## Working rule (the anti-foundation rule, from A4 §59)

> Once a foundation is sufficient for the next product feature, stop
> improving the foundation and build the product feature.

The real-time execution model does **not** authorize an endless scheduler
or engine rewrite. Build the smallest mechanism required by the next
product phase, instrument it, measure it, and keep it only if the product
workload justifies it.

Real product workload is the only judge that can reopen research (e.g.
an A5-style investigation) — a synthetic benchmark alone cannot. Do not
let platform or scheduling work prevent building the editor once the
Windows foundation is adequate.
