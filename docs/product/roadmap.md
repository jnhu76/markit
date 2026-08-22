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
P0-01  Document core + change/revision model            ✅ DONE (PR #11)
              │
              ├──────────────────────────────────┐
              ▼                                  ▼
G0  GPUI baseline freeze               P0-02  Markdown BlockIndex + IR
    (Windows evidence,                        (GPUI-independent: grammar
     dependency pin)                           contract, golden fixtures,
              │                                differential oracle)
              │                                  │
              └──────────────┬───────────────────┘
                             ▼
                   P0-03  First product vertical slice
                          keystroke → Document → Markdown IR → pixels
                             │
                             ▼
                   P1-A  Dogfood editor
                          (edit Markit's own docs with Markit)
                             │
                             ▼
                   P1-B  v0.1 hardening
                          (real-host matrix, buffer/index gate,
                           reliability, release artifact) → v0.1
                             │
                             ▼
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

This is a **dependency graph with evidence-triggered gates, not a waterfall**
(issue #12). G0 and P0-02 depend only on P0-01 and may proceed independently
and in parallel. Both are prerequisites for P0-03, the first real
`keystroke → Markdown semantics → pixels` product path. P1-A dogfooding gets
a real workload early without weakening the final v0.1 gates, which P1-B
owns. Phases may proceed when their dependencies are ready; do not create
artificial serialization, and do not start a phase whose prerequisites are
missing.

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

## G0 — GPUI Baseline Selection (✅ DONE 2026-08-22)

- Outcome: the product GPUI baseline is **frozen** —
  `zed-industries/zed` rev `eb8e1c8b5502b7007465fbbc465f4a736fa39210`
  (Zed v1.16.1), pinned in the workspace root `Cargo.toml` and consumed only
  by GPUI-facing crates. The full capability audit, Windows runtime evidence,
  idle-scheduling verdict, provenance, and update policy live in
  `docs/product/g0-gpui-baseline.md`.
- Key facts established (details + evidence in that document):
  - the pinned revision provides demand-driven draw/present, view-granular
    invalidation, real Windows background priorities, threadpool timers, and
    drop-cancels-future task semantics — all behaviorally confirmed on the
    real Windows host (`results/summary/g0/`: 0 draws/0 frame requests at
    rest, 0 % idle CPU, ~36 MB WS, threadpool priority order H-first,
    50 ms timers → ~59–64 ms actual);
  - **Windows has no metered idle scheduling**: `spawn_when_idle` falls back
    to low-priority main-thread work and `idle_time_remaining()` is `None`
    (source audit, confirmed behaviorally: 100 idle tasks all ran after a
    200 ms main-thread busy window, none during); any future Markit idle
    budget must be self-metered;
  - a vsync thread wakes every window each refresh; clean windows skip
    draw/present, so idle evidence is stated as "no draw/present work", not
    "no wakeups" (with an always-armed `on_next_frame`, callbacks arrive at
    ~12.5/s on the test host while painting stays ~0);
  - foreground executor priority is ignored — Markit priority semantics must
    not assume foreground priority works;
  - release builds for Windows must be produced **natively on a Windows
    host** (gpui_windows compiles HLSL shaders with fxc only when the build
    host is Windows); cross-builds from WSL cover `cargo check`/clippy only.
- Original scope (for history): evaluate the `mvp/gpui` prototype baseline
  (crates.io gpui 0.2.2) and a current Zed revision on build, release build,
  window, native text, CJK, IME, clipboard, resize, HiDPI, startup, RSS,
  basic latency, frame request/redraw semantics, instrumentation hooks, and
  deferred/cancellable work — recording evidence per item, never choosing
  merely because newer is newer.
- The pinned GPUI revision is a dependency of the GPUI-facing crates only
  — never of `markit-core` (P0 acceptance: core has no GPUI dependency).
- Non-goals (preserved): editor features, worker-topology tuning,
  Linux/macOS hosts, plugin runtime/transport decisions.

## P0 — Markit Rust Core + Change/Revision Model

P0 is three nodes with different dependency profiles (issue #12 R1/R2).

### P0-01 — Product workspace + document/change/revision core ✅ DONE

Implemented (PR #11, 2026-08): the product Rust workspace
(`crates/markit-core` + `apps/markit` skeleton), the incremental document core
(private storage, revision/change model, incremental LineIndex per ADR-003,
selection / edit-transaction / snapshot seams, stable `DocumentId`), and the
differential test battery including work-amplification counters. See
`docs/product/p0-01-implementation-note.md`.

Known retained costs (deliberate P0-01 choices, revisited only through the
P1-B buffer/index decision gate):

```text
String splice               = O(document suffix bytes) movement
LineIndex suffix adjustment = O(lines after edit)
```

### P0-02 — Markdown BlockIndex + internal IR (next)

GPUI-independent; does not wait for G0.

- Goal: the framework-independent Markdown layer whose update semantics are
  explicit enough for incremental, cancellable, coherent presentation.
- Scope:

  ```text
  supported L1 grammar contract (explicit, incl. recorded deviations)
  BlockIndex (ADR-004)
  block/inline internal IR
  incremental resynchronization (stable-boundary rescan)
  internal block identity stable enough for incremental reuse
  changed-region consumption from P0-01 EditResults
  full-parse oracle + randomized differential
  minimal semantic golden suite for every supported L1 construct
    (paragraph, heading, blockquote, ul/ol list, fenced code,
     emphasis, strong, inline code, link)
  work-amplification counters (bytes/blocks rescanned, blocks reparsed)
  ```

- **P0-02 must not self-certify Markdown correctness** (issue #12 R5): the
  differential oracle proves `incremental == full scan of the same parser`
  only. The minimal golden suite uses CommonMark-derived cases where the
  supported subset intends CommonMark semantics and explicitly records
  deviations. P2 may expand conformance breadth; P0-02 cannot ship only a
  differential oracle.
- **"Local edit = exactly one block" is evidence, not law** (issue #12 R6):
  the invariant is *a change invalidates the smallest semantically valid
  region and stops when parser state safely converges*. Allowed claim:
  ordinary non-structural local edits remain one-block in the measured L1
  workload. A one-character edit may legitimately change block structure.
- **Fence recovery must not change Markdown semantics to manufacture a
  bound** (issue #12 R7): broad honest propagation is allowed; what must be
  bounded/chunkable/yieldable is synchronous user-blocking work. Checkpoints
  and restart states may reduce repeated work but cannot lie about
  semantics; any dialect rule change (e.g. what counts as a fence opener)
  must be an explicit documented dialect decision, not a performance fix.
- **Internal block identity is not plugin identity** (issue #12 R9): the
  internal `BlockId` may be implementation-owned and evolve; public plugin
  semantic identity is defined later through the versioned adapter contract.
- Acceptance: golden fixtures green; differential oracle green; local-edit
  invalidation is the smallest semantically valid region with counters
  proving unrelated blocks are untouched; no GPUI dependency; no rendering,
  scheduler, files, IME, plugin runtime, rich blocks, or syntax hiding.
- Non-goals: everything GPUI (that is P0-03), view model, commands beyond
  what P0-01 already owns.

### P0-03 — First product vertical slice

Begins only after **both** G0 and P0-02 are ready.

- Goal: prove the first real product path:

  ```text
  keystroke
    → EditTransaction
    → DocumentVersion / change regions
    → BlockIndex / Markdown IR
    → visible projection
    → GPUI layout / shaping / paint (pinned G0 baseline)
    → presented frame
  ```

- Scope: the smallest window + input + projection path on the frozen GPUI
  baseline that makes one keystroke visible as styled Markdown. Success is
  the slice, not feature completeness.
- Must preserve (from the execution laws): demand-driven redraw,
  viewport-bounded visible work, revision identity, stale-result-safe
  publication boundary, instrumentation seams. Must **not** pre-build a
  generic scheduler (issue #12 R3): use GPUI primitives directly and keep the
  policy seam thin until a real product job needs priority/deadline machinery
  (trigger: a real job that can exceed the interaction budget synchronously,
  may finish out of order and need cancellation, or competes with visible
  work under sustained typing/scroll).
- Acceptance: keystroke→styled-pixels works on Windows on the pinned
  baseline; no permanent frame loop; the P0-01/P0-02 counters are observable
  end-to-end; no editor feature work beyond the slice.
- Non-goals: files, undo history beyond the existing transaction seam,
  IME (P1-A), plugins, rich blocks.

## P1-A — Dogfood Editor

- Goal: enough Markit to edit Markit's own Markdown docs daily.

  ```text
  window + visible Markdown L1
  caret / selection / navigation / scroll
  open / save / save-as
  undo / redo
  clipboard
  Chinese IME (composition model, candidate docking)
  CJK + emoji fallback
  basic find
  ```

- Runs on the frozen G0 baseline with the execution laws preserved
  (demand-driven frames, viewport-bounded work, revision-safe deferred
  results where they exist).
- Dogfooding does **not** weaken final v0.1 acceptance (issue #12 R10): it
  gets a real workload earlier so P1-B hardens against evidence, not
  speculation.
- Non-goals: generalized scheduler framework, plugin runtime, rich blocks,
  tables/images/math.

## P1-B — v0.1 Hardening

- Goal: close the `docs/product/mvp-v0.1.md` acceptance gates on Windows and
  ship v0.1.

  ```text
  atomic save correctness (write→tmp→fsync→rename)
  minimal crash recovery (snapshot + clean-shutdown marker + startup recovery)
  real-host performance matrix (p50/p95/p99/max + long frames)
  Buffer / Index Decision Gate (see below)
  scheduler extraction only where real workloads require it (issue #12 R3)
  queue/cancellation/coherent-publication tests where async work exists
  portable release artifact
  ```

- **Buffer / Index Decision Gate (issue #12 R8):** before v0.1 performance
  certification, measure real product workloads at positions
  `begin / q1 / middle / q3 / end`, content mixes (ASCII / CJK / emoji /
  long lines / mixed Markdown / fences), and sizes (small / medium / large +
  the synthetic 1M-line case), observing at least: bytes scanned, bytes
  moved/copied where measurable, line entries shifted, blocks reparsed,
  visible materialization, and real input→visible-frame tails / long frames.
  v0.1 certification may not claim "1M local edit is flat" from
  `bytes_scanned == Δ ∧ full_rebuilds == 0` alone — those counters exclude
  String suffix movement and LineIndex suffix shifting. If String/flat
  LineIndex suffix movement becomes the dominant user-visible term, reopen
  the ADR-003 implementation decision and evaluate Rope/PieceTree/tree
  variants; if not, keep the simple representation. The synthetic 1M-line
  test is an algorithmic regression guard, not by itself a product
  performance certification.
- Scheduler extraction remains evidence-triggered (issue #12 R3): build
  priority/deadline/cancellation machinery only when at least one real
  product job can exceed the interaction/frame budget synchronously, may
  finish out of order and need stale rejection, or competes with visible
  work under sustained typing/scroll.
- Acceptance: all `mvp-v0.1.md` gates PASS on Windows with evidence;
  invariants battery green; normal local edits remain Δ + viewport
  proportional rather than document proportional under the full gate
  measurement (not just the counters); idle editor does no draw/present
  work; deferrable work can yield without blocking caret/input/visible
  text; synthetic out-of-order jobs cannot publish stale state; no GPUI
  code leaked into `markit-core`.
- Non-goals: tabs, images/tables/math, installer/MSIX, Linux/macOS,
  generalized ECS/job framework, plugin runtime/marketplace.

## P2 — Incremental / Scheduling Hardening

- Goal: harden the L1 pipeline and real-time execution model to
  product-grade conformance, bounded recovery, and stable scheduling.
- Scope:
  - L1 conformance breadth (expanding the P0-02 minimal golden suite:
    adversarial cases, structural recovery, CommonMark-derived where
    applicable);
  - bounded fence recovery via semantics-preserving means (parser
    checkpoints, resumable propagation — do not hide honest structural
    propagation and do not change Markdown interpretation to cap the
    radius);
  - incremental-parser robustness across large structural edits;
  - priority inversion / queue growth tests;
  - cancellation vs stale-result-rejection policy measurement;
  - cache-key/invalidation correctness tests;
  - Document LOD height estimation/correction and scroll-drift tests;
  - adaptive work chunking only if evidence shows a fixed policy causes
    long frames or under-utilization.
- Acceptance:
  - ordinary non-structural local edits remain one-block invalidation at any
    document size in the measured workload (evidence-based claim, not a
    correctness law — structural edits may legitimately propagate further,
    issue #12 R6);
  - structural-edit propagation radius is honest, measured, and documented;
    any bounding strategy (checkpoints/resumable propagation) preserves
    Markdown semantics (issue #12 R7 — never alter interpretation to cap
    invalidation);
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
