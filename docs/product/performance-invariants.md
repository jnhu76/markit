# Markit — Performance Invariants

These are the A1–A4 research results turned into architecture invariants
for the direct-GPUI product (ADR-008), extended by the product real-time
execution model in `realtime-execution-model.md`.

They are **work-amplification and scheduling invariants** first — the
product must not accidentally do O(document) work in hot paths or turn
background work into interaction-blocking work. Exact numeric millisecond
budgets are calibrated on real product workloads and real Windows/GPUI
presentation evidence; CI should not flake on arbitrary wall-clock SLAs.

Evidence for the original invariants lives in `docs/research/` (historical
A2–A4 measurements) and the ADRs. New scheduling invariants are product
contracts to instrument and validate as the direct-GPUI implementation
lands.

## INV-01 — Normal single-character edits must not scan the entire document

The A2 root cause (per-edit `lineStarts()` full scan, 94.6% of the 1M
edit turn) is removed by the incremental LineIndex (A3-P1) and the
incremental Markdown BlockIndex (A4-R2). Guard: full-document scans per
edit == 0; blocks_reparsed for local edits == 1 at any document size
(measured 10K→1M).

## INV-02 — Normal frame work must be viewport-bounded

Only the visible range (+ overscan) is laid out and drawn (A3-G1 formula;
measured 25 lines shaped at 1M on the GPUI prototype). The view model
emits exactly the visible lines; nothing shapes or paints beyond the
presentation region unless explicitly classified as deferred prefetch.

## INV-03 — Materialized presentation work scales with the visible presentation, not the document

The equivalent of the A4 "DrawList words identical at 10K/100K/1M (3046)"
measurement: materialized GPUI elements / shaped text / paint work must
scale with the visible presentation, not total document size. A document
can be huge; the frame must not be.

## INV-04 — An idle document must not continuously request frames

Demand rendering: the idle editor must not request frames while nothing
changed (measured ~0 frames/s, ~1% CPU in A3/A4). No animation/timer loop
may force frames while nothing changed.

The game-engine analogy therefore stops at scheduling techniques: Markit
must not acquire a permanent fixed-rate game loop.

## INV-05 — A local Markdown edit reparses the smallest semantically valid region

The BlockIndex rescan stops at the first stable boundary; local edits
reparse exactly one block (measured). Structural edits (fence boundaries)
may invalidate broadly — that cost is owned, documented, and bounded by a
product strategy (see Notes and `architecture.md` §10).

## INV-06 — Platform integration must not add unnecessary work to the per-edit hot path

Keyboard/mouse/scroll events cross the platform edge per tick;
everything else (clipboard, IME candidates, dialogs, fonts, files)
arrives through capability paths that never run inside the per-edit
path unless the interaction itself requires them. GPUI itself sits under
the editor, but Markit must not add per-edit work (conversion, allocation,
re-shaping) at the GPUI edge beyond what the visible presentation
requires.

## INV-07 — Performance measurement prioritizes tails and long frames, not only averages

All A4 cells report p50/p95/max per tick. Product regression runs must
keep p95/p99/max and long-frame counts visible where sample sizes justify
them.

Throughput or total completion time alone is insufficient: spreading
background work over more frames can improve interaction quality even if
that work completes later overall.

## INV-08 — Normal interaction work is proportional to change + viewport

For normal local edits, the intended scaling law is:

```text
work ~= changed semantic region + visible presentation + bounded overhead
```

not:

```text
work ~= total document size
```

Instrumentation should record changed bytes/lines/blocks, blocks rescanned
or reparsed, visible lines, layout/shaping counts, and materialized render
work so hidden amplification is observable.

This invariant does not claim every structural operation is O(Δ + V).
When semantics require broader propagation, the broader radius must be
explicit and measured rather than silently becoming a normal-edit cost.

## INV-09 — Deferrable work must not monopolize the interaction/frame budget

Syntax highlighting, indexing, rich-block projection, distant parsing,
image decode, math/diagram work, and other non-critical tasks must be
schedulable in bounded chunks or worker jobs.

Required behavior:

```text
budget/quota reached
  -> yield
  -> keep the UI responsive
  -> resume later if still relevant
```

The exact budget is not frozen here. It must be derived from target
refresh rates and measured input-to-present behavior on the real host.
Regression instrumentation must expose at least work duration, budget
exhaustion/yield count, and long frames.

## INV-10 — Stale background work must never commit over newer state

Deferred/parallel derived work is revision-aware. When document revision
or semantic dependencies change, an old job must either:

- prove that its output remains reusable;
- be cancelled; or
- have its result rejected at commit.

A stale task cannot block newer user-visible work merely to preserve FIFO
completion.

Regression tests must cover out-of-order completion and verify that
stale parse/highlight/layout/projection results cannot overwrite a newer
revision.

## INV-11 — Dirty propagation and cache invalidation must be precise

A local edit must not invalidate unrelated stable artifacts.

Each derived cache (parse, highlight, layout, shaping, image/math/diagram
projection) must have an explainable key, dependency set, invalidation
trigger, and memory bound. Cache reuse is allowed only when revision and
dependency compatibility are established.

The testable signal is not merely cache hit rate. It is that unrelated
work remains reusable and that invalidation never serves stale output.

## INV-12 — Derived-state materialization follows viewport priority / Document LOD

Markit may retain different amounts of derived state for far, near, and
visible content. The semantic target is:

```text
far       -> lightweight metadata / estimated extent
near      -> parsed or prefetched lightweight state
visible   -> exact layout + shaped text
presented -> render primitives
```

The concrete representation is not fixed. The invariant is that distant
content cannot force visible-frame work proportional to total document
size.

Any height estimation/correction mechanism must also track scroll drift or
visible jumps; virtualization is not considered successful if it trades
CPU for unstable scrolling.

## INV-13 — Presentation publishes coherent revision-compatible state

The user-visible frame must not silently mix incompatible derived state,
for example:

```text
AST v103 + layout v102 + highlight v099
```

unless each reused artifact independently proves compatibility with the
current revision.

"Snapshot" is a logical publication boundary; it does not require a
full-document copy. Tests must exercise edits arriving while parse/layout/
highlight work is in flight and verify coherent publication.

## INV-14 — User-observable work outranks background completion

Scheduling priority follows observability:

```text
current interaction
  > visible viewport
  > near viewport / likely next interaction
  > distant/background document work
```

The scheduler may complete tasks out of submission order. Queue depth,
priority inversion, stale-result count, and cancellation/rejection count
should be observable during performance investigation.

## Notes

- INV-05's fence cascade is the known exception: a fence-boundary edit
  re-parsed up to 30K lines at 1M (68.9 ms, honest, A4 measurement). The
  product must bound fence recovery (only treat ``` as an opener when a
  matching close exists ahead) and re-measure before shipping L1.
- The model's O(lines-after) line-index suffix shift (up to ~2.3 ms at
  1M begin-position edits, A3 measurement) is instrumented and
  position-dependent; a buffer redesign is a real-workload decision, not
  a pre-emptive one.
- The A4-R1 reactive-identity finding (~90 native node recreations per
  edit from fresh visible-list objects) is a design warning for the GPUI
  layer: per-line presentation must derive statelessly from the model's
  visible range (see `issue-backlog.md` — viewport identity discipline).
- `Simon-He95/markstream-vue` is an external reference for adaptive
  incremental rendering and append-vs-replacement semantics. Its numeric
  batch sizes/budgets are not Markit defaults and must not be copied as
  product evidence.

## Regression battery

Core unit tests + invariants battery (to be re-expressed for the Rust
core; the A2–A4 battery `bench/run-a4.py` remains as historical tooling):

| check | invariant covered |
|-------|-------------------|
| full_document_scans == 0 per local edit | INV-01 |
| blocks_reparsed == 1 for local edits at 10K and 1M | INV-01/05/08 |
| materialized presentation work flat across document sizes | INV-02/03/12 |
| op churn per edit viewport-bound | INV-02/03 |
| idle: no frame requests | INV-04 |
| structural edit radius recorded (bounded recovery to come) | INV-05/08 |
| frame-work duration + yield/budget-overrun counters recorded | INV-07/09 |
| old revision result cannot commit after a newer edit | INV-10/13 |
| unrelated block cache survives local edit | INV-11 |
| far/near/visible materialization counts remain bounded | INV-12 |
| out-of-order jobs preserve visible revision correctness | INV-10/13/14 |
| scheduler queue/cancellation/stale-result counters observable | INV-09/10/14 |
