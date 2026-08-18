# Phase A4 — Final Research Closeout → PocketJS Product Foundation

> ## SUPERSEDED ARCHITECTURE DECISION
>
> This report is a **historical research closeout**. Its measurements and
> experimental findings remain part of the evidence base.
>
> Its product-foundation decision ("PocketJS") was **superseded on
> 2026-08-18** by [ADR-008](adr/ADR-008-direct-gpui-product-substrate.md),
> which selects **direct GPUI** for Markit.
>
> Reason: subsequent PocketJS desktop work showed that the modern PocketJS
> desktop host itself delegates window/render/native-text/input integration
> to GPUI, making PocketJS an unnecessary intermediate runtime for Markit's
> product goals. Historical numbers below are unedited.

Status: **RESEARCH_PHASE_CLOSED** (A4-R exit gate passed, see §7).
**The product-foundation decision in this document is superseded —
see the notice above.**

## 0. The answer in one paragraph

Phase A4-R answered the last two open research questions and closed the
research phase:

- **R1 (PocketJS ~7 ms active-edit floor)**: the residual was decomposed
  with a byte-identical counterfactual (the CF bundle reproduces the
  Solid app's DrawList word-for-word and pixel-for-pixel). ~7.4 ms of the
  ~8.5 ms edit turn is **Solid-layer work amplification, not a framework
  intrinsic floor**: Solid's `For` reconciles by item-reference identity,
  and Markit's fresh per-render item objects re-mounted all 26 visible
  line components per edit (~90 native node creations + GC churn). The
  PocketJS lower rendering stack — document mutation, core tick (layout
  + text), DrawList, host, GPU — is **~0.3 ms**. The one obvious, safe,
  local, product-natural fix (stable item identity for the visible list,
  A3's changed-range discipline applied to the view layer) reduced the
  reactive/render preparation to ~1.0–1.5 ms with no semantic or visual
  change. So the ~7 ms was **Markit's use of Solid reconciliation
  semantics**, not the PocketJS stack.
- **R2 (Markdown L1 invalidation radius)**: an incremental L1 pipeline
  (Block Index → Incremental Parse → Affected Blocks → Styled Runs →
  Visible Layout → DrawList) was built and measured on fixed-seed
  Markdown corpora (10K/100K/1M, the L1 subset). For normal local edits
  (paragraph, inline emphasis, heading, list, off-viewport) the
  invalidation radius is **exactly one block at every document size**
  (10K→1M: blocks_reparsed 1→1, edit time 1.4→1.5 ms, viewport-constant).
  A fence-boundary edit (M5) honestly invalidates forward through the
  fence cascade (10K: 287 lines; 1M: 30 197 lines, 68.9 ms) — recorded,
  not benchmarked away, and flagged as a product-level L1 design cost.
  **No accidental O(document) hot path exists for normal edits.**

Research phase: **CLOSED**. From here Markit is a PocketJS-based product;
GPUI is frozen as a reference/performance oracle.

## 1. A1–A3 recap (what was established before A4)

| Phase | Question | Answer |
|-------|----------|--------|
| A1 | Do both substrates run a parity editor on Windows? | Yes (PocketJS windowed wgpu; GPUI; same window/font/workload contract) |
| A2 | What causes the 1M edit/frame cost? | PJS: Markit's per-edit full-document `lineStarts()` scan (94.6% of the turn); GPUI: Markit's full-content element sizing (98% of the frame). Counterfactual-verified. |
| A3 | Remove both root causes, re-measure | PJS 224.8→9.9 ms (incremental LineIndex); GPUI 52.4→1.21 ms (viewport-bounded). Both A2 predictions reproduced. Residual: PJS ~7–9 ms Solid visible-list re-render; GPUI ~1 ms edit-side line rebuild. |

## 2. A4-R R1 — PocketJS residual decomposition

### 2.1 Question

A3 left a ~7–9 ms PocketJS active-edit floor (the CF=1 residual). R1 asked:
what exactly is inside it — Solid/reactive reconstruction or the PocketJS
lower rendering stack?

### 2.2 Setup

Same machine/corpus conventions as A3 (win11_dt, Windows 11, AMD Ryzen 7
5800H, 1000×700 window, Consolas 18 px, 28 px line height, release
builds, fixed-seed corpora). The guest gained fine-grained instrumentation
(per-edit phase wall times at Date.now() resolution + per-op native call
counters via an ops-wrapped diagnostic bundle; the host's per-tick
`gf_us`/`ct_us`/`dl_us`/`r_us` remain the precise clocks).

### 2.3 The counterfactual (CF bundle)

`app/cf-boot.ts` drives the **same document model and the same visible
presentation** (same 26 lines, same text, same caret, same selection)
through the PocketJS mirror/native ops directly — no signals, no memos,
no component execution. Equivalence was verified three ways:

- DrawList word streams **identical** (word-for-word, every tick);
- screenshots **pixel-identical** (0 diff rows);
- state echo identical (caret/scroll/doc sequences).

A no-text CF variant (same tree, empty Text nodes) isolates text content
work from tree work.

### 2.4 Decomposition (windowed, p50 of run medians, 3 runs per cell)

| term | 10K | 100K | 1M |
|------|----:|-----:|---:|
| gf_us total (Solid app) | 9.44 ms | 8.79 ms | 9.67 ms |
| gf_us CF (imperative floor) | 0.32 ms | 0.49 ms | 2.28 ms |
| **Solid reactive reconstruction** | **9.1 ms** | **8.3 ms** | **7.4 ms** |
| cf-notext (no text work) | 0.31 ms | 0.46 ms | 2.23 ms |
| ct_us core tick (Solid app) | 72 µs | 66 µs | 75 µs |
| ct_us core tick (CF) | 29 µs | 30 µs | 27 µs |
| dl_us DrawList hash | ~50–60 µs | | |
| r_us GPU render | ~250 µs | | |

Notes:

- The CF floor scales 10K→1M (0.32→2.28 ms) because the typing workload
  edits at the document beginning: the **model's** concat + line-index
  suffix shift are O(lines after edit) — Markit-owned, position-dependent,
  not framework cost (cf-notext scales identically).
- Per-edit op counts (ops-wrapped bundles): the Solid app performs
  **~90 native node creations + ~30 detach/destroy per edit** (949
  createNode per 60-frame run); the CF performs 1 replaceText + 2 caret
  setProps.
- Guest-side phase timing agrees: the Solid synchronous re-render phase
  measures ~7.6 ms/edit before the fix, ~1.2–2.3 ms after.

### 2.5 Root cause

Solid's `For` reconciliation matches items by **reference identity**
(`items[i] === newItems[i]`; this Solid version's `mapArray` does not use
a key function). The app built fresh `{index, start, end}` item objects
per render, so every item looked new every edit: all 26 line components
re-mounted, recreating ~90 native nodes (plus QuickJS GC churn) per edit.

### 2.6 The intervention (one obvious, safe, local, product-natural fix)

`app.tsx` now:

1. emits items that carry **only the stable line number**, cached by
   position (`lineCache`) — the item reference is stable while the
   document shifts under it;
2. hoists every doc-dependent read (line text, selection rect) into
   **item-scoped memos**, so the item component itself never re-runs on a
   document change; the memos re-evaluate (26× per edit) but the
   components and their native nodes are created once.

This is standard Solid usage (stable identity + fine-grained reads), not
a benchmark trick; the same pattern is the correct shape for the
product's Incremental View Model (A4-P Layer 4).

### 2.7 Before/after (windowed, p50 of run medians, 3 runs per cell)

| corpus | gf_us before | gf_us after | Solid contribution after |
|-------:|-------------:|------------:|-------------------------:|
| 10K | 9.44 ms | **1.66 ms** | ~1.3 ms |
| 100K | 8.79 ms | **1.94 ms** | ~1.5 ms |
| 1M | 9.67 ms | **3.65 ms** | ~2.3 ms |

1M position cells (after; the position signature is the model's
suffix-shift term, Markit-owned):

| position | gf_us |
|----------|------:|
| begin | 4.18 ms |
| q1 | 3.66 ms |
| mid | 2.79 ms |
| q3 | 2.41 ms |
| end | 1.91 ms |

DrawList words unchanged (3046 at every corpus), screenshots
pixel-identical, state echo byte-identical, `bun test` 38/38
line-index tests still pass. The `by`-key and item-content-cache attempts
were measured and dropped (no effect — this Solid version's `For` ignores
the key and content caching is defeated by offset shifts).

### 2.8 R1 verdict

```text
Solid/reactive contribution:  ~7.4 ms  →  ~1.0–1.5 ms (viewport-constant)
visible text/layout (JS+core): ~0.3 ms (CF floor incl. model work at 10K)
DrawList:                      ~50–60 µs
host/GPU:                      ~250–300 µs
model (concat + suffix shift): 0–2.3 ms (position-dependent, Markit-owned)

CONFIRMED:
  Fresh-reference visible-item reconstruction caused the ~7 ms Solid-
  layer amplification (Solid's `For` reconciles by item-reference
  identity; 26 fresh objects per edit re-mount every visible line).

PocketJS lower-stack floor observed in this experiment:
  ~0.3 ms (model + core + DrawList + host + GPU).

After adopting stable Solid identities:
  reactive/render preparation ≈ 1.0–1.5 ms — viewport-constant and
  position-independent.

Not shown: a 7 ms "framework intrinsic floor". The ~7 ms was Markit's
use of Solid reconciliation semantics (fresh item references), not a
property of Solid or PocketJS itself.
```

## 3. A4-R R2 — Markdown L1 invalidation radius

### 3.1 Question

Under a real Markdown L1 workload, does PocketJS keep local invalidation?
What is the one-character-edit invalidation radius, and does it scale
with document size?

### 3.2 Pipeline (built for R2, product-shaped)

```text
Document → Block Index → Incremental Parse → Affected Blocks
         → Styled Runs → Visible Layout → DrawList
```

- `app/markdown.ts` (framework-free): `classifyLine` (L1 subset:
  heading, paragraph, bold, emphasis, inline code, link, blockquote,
  unordered/ordered list, fenced code), `scanBlocksFull` (load-time +
  test oracle), `BlockIndex.applyEdit` — an incremental rescan that
  re-classifies from the first affected block forward and **stops at the
  first stable boundary** (same kind + aligned boundary in old and new
  structure, beyond the edited lines; the scan carries fence state, so a
  fence-boundary edit extends the radius to the fence close). 10 unit
  tests incl. a 5000-edit randomized differential against the full-scan
  oracle on the real 10K corpus (0 failures).

  **Conformance scope (what the oracle proves):** the differential proves
  *incremental invalidation correctness* — the incremental rescan agrees
  with this same parser's full scan on 5000 random edits. It is **not** a
  CommonMark conformance proof: both sides could parse "wrong" together.
  The parser is the *Markit Markdown L1 subset* (the nine constructs
  above, syntax-visible), not a general Markdown parser; conformance
  golden fixtures (CommonMark-derived cases + known deviations) are
  deferred to Product P0 (`docs/product/issue-backlog.md`).
- `app/md-app.tsx`: the L0 editor plus the pipeline — the block index is
  maintained per edit (same changed-range discipline as the LineIndex),
  styled runs are computed per affected block (cached by block start
  line, invalidated for exactly the replaced blocks), each visible line
  renders its runs as colored Text nodes (L1 keeps the syntax visible;
  color-only styling keeps the caret/measure math font-uniform).
- `workloads/generate-markdown-corpus.py`: fixed-seed L1 corpora
  (`markdown-10k.md`/`100k`/`1m`, mixed heading/paragraph/lists/quotes/
  inline/fences) plus a positions manifest for the M1–M6 edit cases.

### 3.3 Edit cases (M1–M6) — one-character edits at manifest positions

Windowed, p50 of run medians, 3 runs per cell; counters from the edit's
per-edit record:

| case | 10K scan/reparse/inline | 10K gf | 1M scan/reparse/inline | 1M gf |
|------|------------------------:|-------:|------------------------:|------:|
| M1 paragraph | 4 / 1 / 1 | 1.38 ms | 2 / 1 / 1 | 1.54 ms |
| M2 inline emphasis | 3 / 1 / 1 | 1.44 ms | 5 / 1 / 1 | 1.41 ms |
| M3 heading | 1 / 1 / 1 | 1.37 ms | 1 / 1 / 1 | 1.45 ms |
| M4 list item | 5 / 1 / 1 | 1.52 ms | 5 / 1 / 1 | 1.64 ms |
| M5 fence boundary | 287 / 24 / 12 | 5.23 ms | 30 197 / 2 364 / 1 182 | 68.9 ms |
| M6 off-viewport | 2 / 1 / 1 | 1.34 ms | 2 / 1 / 1 | 1.45 ms |

### 3.4 Findings

1. **Normal local edits: the invalidation radius is exactly one block,
   independent of document size.** M1/M2/M3/M4/M6 reparse 1 block at
   10K and at 1M; the edit time is flat (~1.4–1.6 ms — the
   viewport-bounded Solid re-render; the Markdown parse itself adds
   ~0.05 ms). The R2 acceptance gate passes: 10K→1M does not scale
   blocks_reparsed or layout.
2. **Off-viewport edits (M6) cost the same as visible edits** (2/1/1,
   1.34 vs 1.38 ms) — the pipeline is viewport-agnostic by construction.
3. **Structural fence-boundary edits (M5) invalidate honestly and
   broadly**: breaking an opening fence turns the code lines into
   paragraphs, and each subsequent stray ``` line opens a new fence that
   consumes forward to the next close — at 1M the cascade re-parsed
   30 197 lines / 2 364 blocks and took 68.9 ms. Recorded as a genuine
   structural-edit cost (the L1 block model's "``` outside a fence opens
   one" rule), a **product-level design input** — tracked as the
   *bounded fence recovery* product item
   (`docs/product/issue-backlog.md`, P4) with parser-checkpoint /
   restart-state candidates; the honest 68.9 ms is kept, not optimized
   away for a prettier report.
4. **No accidental O(document) hot path exists for normal edits** — the
   A4-R exit-gate "no new O(document) amplification" criterion passes.

## 4. A4-R exit gate

```text
✅ PocketJS 7 ms residual decomposed (Solid reconstruction ~7.4 ms —
   Markit's fresh-reference reconciliation, not a framework intrinsic
   floor; lower stack ~0.3 ms; stable-item fix → ~1.0–1.5 ms)
✅ Markdown L1 pipeline built (BlockIndex + incremental rescan + styled
   runs + visible layout; 5000-edit differential-correct)
✅ 10K/100K/1M L1 corpora generated (fixed seed)
✅ Local edit invalidation radius measured (1 block at any size)
✅ Structural edit invalidation measured (fence cascade, honest)
✅ No accidental O(document) amplification for normal edits
✅ PocketJS core shows no product-blocking performance defect
   (the ~7 ms was Markit's Solid usage, fixed in Markit code; the
   remaining terms are Markit-owned model work and the viewport-constant
   Solid re-render)
```

```text
RESEARCH_PHASE_CLOSED
```

## 5. What we now know

**CONFIRMED**

- The A3 PocketJS residual was Solid reactive reconstruction (~7.4 ms of
  the ~8.5 ms edit turn) caused by Markit's fresh-reference visible-item
  objects, removable with a stable-item + memoized-read pattern (→
  ~1.0–1.5 ms; whole turn 1.66 ms at 10K, 3.65 ms at 1M).
- The PocketJS lower rendering stack (model + core + DrawList + host +
  GPU) is ~0.3 ms for a one-char edit, viewport-bound, scaling only with
  the model's O(lines-after) suffix shift.
- Markdown L1 local edits invalidate exactly one block at any document
  size; the pipeline is viewport-agnostic (off-viewport edits cost the
  same).
- A fence-boundary edit causes a broad, honest structural cascade (up to
  the whole document at 1M) under the L1 block model — a real product
  design input.

**SUPPORTED (measured, not fully isolated)**

- The remaining PocketJS edit cost is viewport-constant Solid re-render
  (~1.0–1.5 ms; the 26 line memos re-evaluate per edit) plus Markit's
  O(lines-after) line-index suffix shift (position-dependent, up to
  ~2.3 ms at 1M begin) and O(N) concat — all instrumented.

**UNKNOWN**

- L2+ projection costs (visual syntax hiding, tables, images, math);
- IME/real-input latency (Windows IME manual validation pending);
- real Linux desktop (Wayland/X11) and macOS host behavior (WSLg smoke
  only); CJK font discovery on all three platforms.

## 6. PocketJS decision

```text
PRIMARY PRODUCT FOUNDATION: PocketJS
```

GPUI remains the faster substrate on this machine's active-edit
benchmarks (A3: ~1.2 ms vs 9.9 ms per edit at 1M; after the A4 stable-item
fix the gap narrows to ~1.2 ms vs 3.65 ms). The decision is **not** a
benchmark win: it rests on the project's goals and architecture control:

- **Guest-side product logic**: editor behavior in JS on the PocketJS
  stack — iteration speed and product flexibility that a native Rust
  editor does not offer.
- **The residual cost is now understood and small**: the framework floor
  is ~0.3 ms and the remaining Solid cost (~1.0–1.5 ms, viewport-bound)
  is a known, bounded, addressable term (the product's Incremental View
  Model can cut it further) — not an unknown black box.
- **The measured bottlenecks have all been Markit-owned** (A2 scan, A3
  index, A4 Solid usage) and were fixed in Markit code; `vendor/pocketjs`
  SHA remains unchanged across A2–A4.
- **Single-stack product**: one retained UI tree + DrawList contract for
  Windows/Linux/macOS, with a clear host/svc boundary for platform work.

This is the honest trade: GPUI's measured latency/startup/memory
advantages are real and documented (A3 §16), and GPUI stays as the
reference/performance oracle for future product decisions.

## 7. GPUI status

```text
REFERENCE / PERFORMANCE ORACLE — no longer developed as a product backend.
```

GPUI is used only when a PocketJS number needs a substrate sanity check.
No GPUI Markdown product work was created in A4 (the R2 pipeline exists
only on the PocketJS side).

## 8. Product architecture (summary)

See `docs/product/architecture.md` for the full document. The target
shape:

```text
                    Markit Core (framework-free TS)
                        │
          ┌─────────────┼─────────────┐
      Document       Markdown       Commands
        Model          Engine          │
          │             │             │
          └──────┬──────┘             │
                 │                    │
          Incremental View Model      │
                 │                    │
            PocketJS UI               │
                 │                    │
             DrawList                 │
                 │                    │
          PocketJS Desktop Host  ─────┘
                 │
       ┌─────────┼─────────┐
    Windows    Linux     macOS
```

Platform differences live in thin capability-driven adapters
(ClipboardProvider, FontProvider, ImeProvider, FileDialogProvider,
ShortcutPolicy, PlatformPaths), never in the core.

## 9. Platform plan (Windows / Linux / macOS)

See `docs/product/platform-capability-matrix.md` for the full matrix
(PASS/FAIL/PARTIAL/NOT TESTED/DEFERRED — no guessing). Current status:

- **Windows**: the A1–A4 evidence base (windowed wgpu, keyboard, scroll,
  resize, demand rendering all PASS; CJK discovery, clipboard, IME and
  file dialogs are GAPs for the product — P1 scope; transparent window
  DEFERRED, not a requirement).
- **Linux**: WSLg smoke PASS only; the product runtime must be verified
  on a real Linux desktop (Wayland/X11, fontconfig, IBus/Fcitx,
  xdg portals) — P2 scope.
- **macOS**: not tested in this environment; PocketJS's macOS-first
  heritage is not evidence of product readiness — P3 scope.

Implementation sequencing: P0 shared core → P1 Windows → P2 Linux → P3
macOS (architecture keeps all three boundaries correct from day one).

## 10. Remaining foundation gaps (real, not speculative)

1. Windows system font discovery + CJK/emoji fallback (Tier-0 for a
   Chinese-facing product).
2. Windows clipboard (text) + IME composition model + validation.
3. Real-Linux-desktop validation (Wayland/X11, IBus/Fcitx).
4. macOS validation (Metal/wgpu, fonts, IME, clipboard, menus).
5. File dialogs / file association / atomic save / recovery tooling.
6. Packaging (portable exe → installer, AppImage/desktop entry, .app
   bundle + signing).
7. The fence-cascade cost for structural Markdown edits at 1M (L1 block
   model refinement — bounded fence recovery).
8. The L0 `caretFromX` click bug fixed in A4 (see §11.2) should gain a
   regression test in the core test suite.

None of these block starting the product foundation; they are P1–P3
scope items (see `docs/product/issue-backlog.md` for ready-to-paste
issue bodies).

## 11. Notable A4 findings outside the two questions

1. **Solid `For` reconciliation is reference-identity based in the
   bundled version** — fresh item objects per render re-mount every item.
   Product rule: visible-list items must carry stable identity; doc
   reads must be hoisted into item-scoped memos (this is the seed of the
   Incremental View Model).
2. **`caretFromX` click bug (pre-existing)**: `lineIndex * doc.length`
   clamped every click on a line ≥ 1 to the document end; it silently
   corrupted the A3-era click-position cells and was uncovered by R2's
   position cases. Fixed in `editor.ts` (direct `starts` lookup) and
   re-measured. **Provenance (A4 review gate):** the A4 erratum in
   `docs/phase-a3-intervention-validation.md` marks the affected A3
   cells; affected A3/A2 summary files are kept in place and marked
   superseded in `results/summary/a3/INVALIDATED-caretFromX.md` and
   `results/summary/a2/INVALIDATED-caretFromX.md`. A2's qualitative
   position conclusions are unaffected (see the A2 doc erratum).
3. **The R1 stable-item fix also applies to the md-app** — the R2 local
   edit times (~1.4 ms) already include it; the run-level Text nodes in
   md-app are recreated per edit (runs shift with the doc) — a known
   cost carried by the L1 prototype, to be addressed by the Incremental
   View Model (per-line run signals), not by more R-phase optimization.
4. **Stable visible-list identity is a stateless projection** — items
   are keyed by absolute line number and carry no state; every
   doc-dependent read lives in item-scoped memos (regression-tested in
   `app/view-slots.test.ts`, invariant in `app/view-slots.ts` and
   `docs/product/architecture.md` §11). If line widgets later gain state
   (IME, folding), identity must move to a stable block/content ID.

## 12. Product MVP and next phase

- MVP v0.1 scope: `docs/product/mvp-v0.1.md`.
- Roadmap: `docs/product/roadmap.md` (PocketJS Desktop Enablement →
  P0 Product Foundation → P1 Windows Desktop MVP → P2 Linux → P3 macOS →
  P4 Markdown Visual Editing L2 → P5 Rich Blocks → P6 Hardening).
- Next phase: **PocketJS Desktop Enablement / Windows Reference Host** —
  prove the generic desktop capabilities in PocketJS first (normal
  desktop window, CJK runtime fonts, clipboard, IME, platform
  capability truthfulness), then **Markit Product P0 starts after the
  required PocketJS desktop capabilities are proven on the target
  platform**. No A5 research is planned unless a real Markit product
  workload demonstrates a foundational blocker.

## 13. Limitations

- R1/R2 cells are same-day, same-machine (win11_dt), windowed, U0 ASCII
  corpora; run-to-run drift ±10–20% as in A1–A3.
- The guest phase timers are Date.now() ms-resolution (coarse); the
  host-side gf_us/ct_us/dl_us/r_us are the precise clocks.
- The CF's DrawList/pixel equivalence is verified on the typing workload;
  selection rendering parity is structural (same tree order) but not
  pixel-verified.
- The q1/mid/q3 position cells from the first (pre-fix) battery were
  corrupted by the caretFromX bug and discarded; the final cells were
  re-run with the fix. The A3 position and viewport tables carry the
  same latent corruption in their click-derived cells (begin valid, end
  valid by coincidence, middle cells invalid) — see the A4 erratum in
  `docs/phase-a3-intervention-validation.md`; affected A3/A2 summary
  files are marked `INVALIDATED` in place, not deleted.
- cf-notext's words (12) are not a presentation (diagnostic only).
- The R2 pipeline is the measurement surface, not the final product
  editor; md-app's run-level Text recreation is a known prototype cost.

## 14. Git state

```text
Branch:                  phase/a4-product-foundation
PocketJS submodule SHA:  cadffef50b0359e1a069586b9dc5574d65d7fb05 (UNCHANGED
                         across A2–A4 — every fix was Markit-owned)
Artifacts:
  results/raw/a4/        R1 baseline/cf/pos/ops + R2 M1–M6 raw logs
  results/summary/a4/    per-run summaries
  bench/run-a4.py        A4 driver (r1-scale/r1-pos/r1-ops/r2-case)
  workloads/corpus-md/   L1 corpora + positions manifests
  workloads/generate-markdown-corpus.py
```

## 15. Verdict

```text
RESEARCH_PHASE_CLOSED
READY_FOR_PRODUCT_FOUNDATION

Immediate next step:
  PocketJS Desktop Enablement / Windows Reference Host
  (normal desktop window, CJK runtime fonts, clipboard, IME,
   platform capability truthfulness — PocketJS-owned)

Product P0 starts after the required PocketJS desktop capabilities
are proven on the target platform.
```
