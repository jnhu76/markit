# Markit — Real-time Editor Execution Model

Status: **product design contract / implementation details still evidence-driven**.

This document turns Markit's existing incremental and viewport-bounded
invariants into one execution model. It is inspired by real-time systems
and game engines, but Markit is **not** adopting a game-engine architecture.
The useful ideas are dirty propagation, bounded per-frame work, priority,
cancellation, visibility, coherent snapshots, and cache reuse.

The product rule is:

> **Never do work the user cannot observe.**

Expanded:

> If it did not change, do not recompute it. If it is not visible, defer it.
> If the frame budget is exhausted, yield. If work is stale, discard it.
> Never publish a half-coherent document state.

This refines the existing product principle:

> **Nothing gets between your input and the next frame.**

## 1. What problem this model solves

A Markdown editor can become slow even when its parser is fast. A local
edit can accidentally trigger work proportional to the whole document:

```text
input
  -> document replacement
  -> full parse
  -> full layout
  -> full render tree update
  -> syntax / math / image work
  -> paint
```

Markit instead targets work proportional to the changed region and the
visible presentation:

```text
Cost(normal local edit)
    ~= Cost(changed semantic region)
     + Cost(viewport)
     + bounded scheduling overhead
```

not:

```text
Cost(normal local edit) ~= Cost(document size)
```

This is a design target, not a claim that every Markdown operation can be
strictly bounded. Structural edits such as fence changes may have a larger
semantic invalidation radius; those cases must be explicit, measured, and
recovered with bounded strategies where possible.

## 2. The execution pipeline

```text
                    OS / GPUI input
                          |
                          v
                 EditTransaction
                          |
                    changed range
                          v
                 Document / LineIndex
                          |
                    dirty propagation
                          v
                      BlockIndex
                          |
                 changed semantic blocks
                          v
             +------------+-------------+
             |            |             |
             v            v             v
          parser       highlighter    indexer
           jobs           jobs          jobs
             |            |             |
             +------ versioned results -+
                          |
                     Markdown IR
                          |
                    dirty layout
                          v
                 Viewport / LOD model
                          |
             visible / near / far priority
                          v
                   text shaping / layout
                          |
                        cache
                          v
                    render scene
                          |
                  frame-budget scheduler
                          v
                         GPUI
                          |
                          v
                          GPU
```

The arrows are semantic dependencies, not necessarily one synchronous
call stack. The scheduler decides what must happen now, what can happen
later, and what is already obsolete.

## 3. Dirty propagation is the default update model

A single `document_changed` bit is too coarse. Markit should distinguish
at least the semantic reason for invalidation:

```text
TextDirty
ParseDirty
StructureDirty
StyleDirty
LayoutDirty
PaintDirty
ViewportDirty
```

The exact Rust representation is not fixed by this document. What is
fixed is the propagation rule: **invalidate the smallest downstream region
whose semantics may have changed**.

Examples:

```text
local text edit
  -> TextDirty
  -> affected inline/block parse
  -> affected layout
  -> affected paint

style-only change
  -> StyleDirty
  -> layout only if metrics changed
  -> paint

scroll
  -> ViewportDirty
  -> visible-range materialization/layout/paint
  -> no document reparse
```

A feature that cannot state its invalidation radius is not ready to enter
the hot path.

## 4. Mutation semantics must stay explicit

The system must not collapse every state change into "replace document".
At minimum, the architecture must preserve distinctions such as:

```text
Append
LocalEdit
Delete
Paste
StructuralEdit
ReplaceDocument
ViewportMove
ThemeChange
```

The exact API can evolve, but these differences matter because they imply
different invalidation and scheduling behavior.

This is especially important for streaming or append-heavy workloads:
append growth is not a dataset replacement.

## 5. Frame-budgeted, cooperative work

The UI thread must not treat "all pending work completed" as the success
condition for an interaction.

The success condition is:

> the next user-observable frame is coherent and produced within the
> measured interaction budget, while lower-priority work remains resumable.

Conceptually:

```text
Frame N

critical
  input / caret / selection / IME

high
  visible text mutation
  visible layout / shaping
  visible paint preparation

medium
  near-viewport parse/layout
  cheap projection work

low
  distant parsing
  syntax highlighting
  search indexing
  image decode
  math / Mermaid / other rich projections

budget exhausted
  -> yield
  -> continue later
```

Do **not** hard-code a universal 6 ms or 8 ms Markit budget from another
project. The actual budget must be calibrated on Markit's target refresh
rates, GPUI behavior, Windows presentation path, and measured input-to-
present latency.

The required mechanism is a scheduler that can stop at a deadline or work
quota and resume later. The implementation may be cooperative on one
thread, use worker threads, or mix both; that choice remains evidence-
driven.

## 6. Priority is based on user observability

A simple priority order is:

```text
1. current interaction
2. visible viewport
3. near viewport / likely next interaction
4. background document state
```

The priority model should prefer what the user can see or is about to see,
not FIFO completion of historical work.

This means completion order may intentionally differ from submission
order.

## 7. Jobs are versioned and cancellable

Background work must carry enough identity to prove that its output still
belongs to the current document state.

Conceptually:

```text
Document revision 101
  -> parse job 101

Document revision 102 arrives
  -> job 101 may continue only if reusable
  -> otherwise cancel / ignore result

Document revision 103 arrives
  -> publish only results valid for 103
```

A stale task is not valuable merely because CPU time has already been
spent on it.

Required rule:

> **Old work must never block newer user-visible work, and stale results
> must never commit over a newer state.**

Cancellation can be eager or lazy (result rejection), depending on cost.
Correctness requires version validation at commit either way.

## 8. Viewport culling and Document LOD

ADR-005 already requires viewport-bounded presentation. This document
extends that into a document level-of-detail model.

Far-away content does not need the same materialized state as visible
content.

A possible quality ladder is:

```text
Far
  source range + block metadata + estimated extent

Near
  parsed block / lightweight projection

Visible
  parsed + exact layout + shaped text

Presented
  render primitives / GPU-facing state
```

This is a semantic model, not a requirement to create four concrete object
types. The important rule is that **distance from observability controls
how much derived state must stay materialized**.

Scroll correctness requires stable logical extents. Height estimation and
correction must therefore avoid visible scroll jumps and must be measured
for drift.

## 9. Coherent snapshots instead of half-updated UI

The UI should render a coherent committed view while newer derived state is
being prepared.

Conceptually:

```text
Committed snapshot v101  <- UI may present
Working state      v102  <- parse/layout jobs in flight

ready visible subset for v102
  -> validate revision
  -> atomic/logical publish
  -> request frame
```

This does not require copying the whole document. "Snapshot" here means a
coherent version boundary for derived presentation state.

The architecture must prevent mixtures such as:

```text
AST from v102
layout from v101
highlight from v099
```

unless each derived artifact is independently proven compatible with the
current revision.

## 10. Cache stable artifacts; invalidate precisely

Useful cache candidates include:

```text
block parse / inline parse
syntax highlighting
text shaping / glyph runs
line layout
image decode
math / diagram projection
```

Each cache must define:

- key;
- dependency set;
- invalidation trigger;
- revision compatibility;
- memory bound / eviction policy;
- observability of hit/miss behavior for performance analysis.

The goal is not "cache everything". It is:

> **reuse stable derived work without making invalidation correctness
> mysterious.**

A cache whose invalidation cannot be explained is a correctness risk.

## 11. Heavy Markdown blocks stay off the critical interaction path

Rich blocks such as code highlighting, images, math, and diagrams may cost
far more than plain text. Their product contract is:

```text
lazy
viewport-aware
cancellable or stale-result-safe
cacheable
measurable
```

Fallback presentation is allowed while a heavy projection is pending, as
long as document semantics remain correct and the transition does not
cause unacceptable layout/scroll instability.

## 12. What we borrow from game engines

Useful ideas:

- dirty flags / dependency invalidation;
- frame budgets and cooperative scheduling;
- job systems and priority queues;
- visibility culling;
- level of detail;
- coherent frame publication;
- reusable render/layout caches;
- profiling by frame and subsystem.

Not adopted by default:

- ECS;
- archetype storage;
- a permanent 60 Hz update loop;
- a scene graph as the canonical document model;
- rebuilding Markit around game-engine abstractions.

Markit is a demand-driven editor. When nothing changes, it should do
almost nothing (INV-04).

## 13. Reference: Markstream's useful lesson

`Simon-He95/markstream-vue` is a useful external reference for streaming
Markdown scheduling, not proof of Markit's architecture.

Its current NodeRenderer scheduler demonstrates several relevant ideas:

- incremental batches rather than "render everything now";
- a render-time budget used to adapt batch size;
- scheduling through idle / animation-frame boundaries when available;
- treating pure append growth differently from dataset replacement;
- virtualization / live-node bounds for document-style rendering.

Markit should borrow the **discipline**, not the Vue/DOM implementation.
In a direct-GPUI editor we can make change ranges, versions, viewport
priority, and invalidation explicit instead of relying on a reactive DOM
framework to infer them.

Reference files at the time of this design note:

- <https://github.com/Simon-He95/markstream-vue/blob/main/src/components/NodeRenderer/composables/useBatchRenderingScheduler.ts>
- <https://github.com/Simon-He95/markstream-vue/blob/main/src/components/NodeRenderer/rendererModeDefaults.ts>

## 14. Required instrumentation

The execution model is only useful if it is observable. Product builds
used for performance work should be able to report at least:

```text
revision
changed bytes / lines / blocks
blocks rescanned / reparsed
layout lines / shaped runs
visible / near / far materialization counts
frame work duration
budget overruns / yielded work
queue depth by priority
job cancellation / stale-result count
cache hit/miss by subsystem
long-frame count
input -> frame/present timing where available
```

These counters are not all required in every release build, but the
architecture must retain seams for them.

## 15. Acceptance laws

Every product phase that touches the editor hot path must preserve these
laws:

1. **Work proportionality** — normal local edits do not scale with total
   document size when semantics do not require it.
2. **Viewport boundedness** — presentation work scales with visible
   content plus controlled overscan.
3. **Non-blocking progress** — deferrable work cannot monopolize the UI
   thread across the interaction budget.
4. **Freshness over completion** — stale work is cancelled or rejected.
5. **Coherent publication** — only revision-compatible derived state is
   presented.
6. **Precise invalidation** — unrelated stable work is reused.
7. **Demand-driven idle** — no permanent update loop when nothing changed.
8. **Measurement before tuning** — exact budgets, worker counts, queue
   policies, buffer structures, and cache sizes are selected by evidence.

## 16. Relationship to other documents

- `architecture.md` defines the product/core/platform structure and uses
  this document as the execution model for the editor pipeline.
- `performance-invariants.md` defines regression invariants that make this
  model testable.
- `roadmap.md` makes the scheduler/dirty/version/LOD foundations explicit
  phase acceptance work rather than optional future optimization.
- ADR-003/004/005 remain the accepted evidence-backed decisions for
  incremental line indexing, Markdown invalidation, and viewport-bounded
  rendering.
- No new ADR is created here for worker topology, ECS, a buffer structure,
  numeric frame budgets, or cache algorithms. Those remain open until the
  product workload produces evidence.
