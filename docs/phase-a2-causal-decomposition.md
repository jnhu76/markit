# Phase A2 — Causal Decomposition (PocketJS / GPUI)

Status: **READY_FOR_PHASE_A3** — both MVPs' per-edit latency is decomposed
into measured components, the dominant component of each is proven causal
by a counterfactual intervention (not inferred), ownership is assigned, and
every diagnostic change was removed or disabled before this report was
written. Raw logs in `results/raw/a2/`, per-run summaries in
`results/summary/a2/`.

Scope: **measurement and decomposition only.** No optimization was shipped.
All code changes in this phase are instrumentation (gated by `--perf` /
`--a2` flags) or diagnostic counterfactuals that were reverted and verified
(§9). Per the phase spec: *A2 ends with explanations, not faster numbers.*

## 0. The answer in one paragraph

Both MVPs pay for the **entire document** on every keystroke, through two
different mechanisms, and both mechanisms live in **Markit-owned code**:

- **PocketJS**: every edit triggers a full-document `lineStarts()` scan in
  the guest (`editor.ts`), `O(N)` in total bytes, called from our Solid
  memo. At 1 MB it is 206 ms of the 217.7 ms edit turn (94.6%). Deleting
  the scan (counterfactual CF=1) drops the turn to 7.5 ms — a 29× change
  caused by removing one function call. Position of the edit and viewport
  position have no effect; the host side (core tick, DrawList, wgpu) is
  constant and viewport-sized.
- **GPUI**: our `EditorElement::request_layout` sizes the element to the
  **full content height**, so `prepaint` computes a "visible" range of
  `[first_scroll_line .. last_document_line]` and calls
  `text_system().shape_line` on every one of those lines **every frame** —
  at 1 MB, 18,081 lines, 39.4 ms of the 52.4 ms frame (75%) plus 11.9 ms of
  paint (23%) over the same range. Truncating the shaped range to 26 lines
  (counterfactual, semantics deliberately broken) drops prepaint to 67 µs
  and the whole frame to ~1.3 ms — a 40× change caused by the range, not
  by shaping itself.

Framework-side costs (QuickJS turn, gpui shaping, DrawList, wgpu render)
are real but are **not** what scales: they are constant or viewport-sized.
The scaling behavior of both MVPs is produced by Markit's document-model
choices.

## 1. Questions

| # | Question | Experiment | Result (§) |
|---|----------|-----------|------------|
| Q1 | What does a single-character edit cost end-to-end in each MVP, and where does the time go? | per-tick counters, both MVPs, 10K/100K/1M | §5.1, §6.1 |
| Q2 | Does the cost depend on the edit position in the document? | P1–P5 (begin/q1/mid/q3/end), 1M | §5.2, §6.2 |
| Q3 | Does the cost depend on where the viewport is? | V1–V3 (inside/near/far), 1M | §5.3, §6.3 |
| Q4 | What is the overhead of a turn/frame when nothing changes? | noop/same-value control (PJS), static-redraw control (GPUI) | §5.4, §6.4 |
| Q5 | Which component *causes* the scaling? | counterfactuals: PJS CF=1/CF=2 (skip scan / skip concat), GPUI 26-line range truncation | §5.6, §6.5 |
| Q6 | Does the instrumentation change the numbers? | calibration orig vs off vs on, same workload | §7 |
| Q7 | Who owns the cause — Markit or the framework? | ownership classification of each measured component | §10 |

## 2. Setup

- Host: the A1 Windows machine (native windowed runs, 1000×700), driven
  from WSL. Same machine, toolchain and corpora as the A1 benchmark
  (`docs/phase-a1-pocketjs-windows.md` §7) so A1 numbers remain comparable.
- Toolchain: unchanged from A1 (recorded in A1 report); release builds,
  symbols retained, no profiler attached to the latency runs.
- Instrumentation: gated by `--perf` (PocketJS host) / `--a2` (GPUI).
  A1's 7-stage ring trace is untouched (PocketJS reuses it; GPUI's A1
  dump format byte-identical).
- Workload: A1-typing — 100 single-char inserts at ticks 340–439,
  backspace @450, scroll @460, auto-quit. Same for both MVPs
  (`bench/run-a2.py`). Semantic workload level L0–L1 (plain ASCII edit +
  line index maintenance; no Markdown projection — see §12).
- Corpus: fixed-seed ASCII, 10K / 100K / 1M bytes
  (190 / 1941 / 19653 lines, ~53 chars/line). U0 per the Unicode ladder.
- Metric boundaries (§3); medians of run medians, 3–5 runs per cell,
  p50 unless noted. All durations are host-clock µs.
- Run driver: `bench/run-a2.py`; parsers `bench/parse-a2.py`
  (JSONL aggregate) and `bench/parse-trace.py` (A1 ring).

## 3. Measurement boundary audit (A2-0)

What each number is, and what it is a proxy for (A1 contract, unchanged):

| Metric | Definition | Proxy status |
|--------|-----------|--------------|
| PocketJS `gf_us` | guest JS turn: `guest.frame()` (svc drain → Solid re-render → DrawList regen) | edit-to-DrawList, host clock |
| PocketJS `ct_us` | core tick (`surface.tick`) | layout/retained-tree update |
| PocketJS `dl_us` | DrawList build + FNV-1a hash | demand-render keying work |
| PocketJS `words` | DrawList word count | work proxy for the GPU path |
| PocketJS `scanMs` | `lineStarts()` full scan, per edit (guest, `Date.now()`, ms) | O(N) scan duration |
| PocketJS `copyMs` | `typeText()` concat copy (guest, ms) | coarse (<1 ms resolution) |
| GPUI `prepaint_us` | `EditorElement::prepaint`, i.e. the line-shaping loop | shaping + cache work |
| GPUI `shape_us` | the `first..last` shaping loop only | shaping |
| GPUI `paint_us` | paint loop over shaped lines (quads, glyph quads) | paint work |
| GPUI `lines_shaped` | lines visited by prepaint (non-empty) | range-size proxy |
| GPUI `concat_us` / `lines_us` | `apply_edit` concat + `rebuild_line_starts` | edit-side O(N) work |
| trace spans | A1 7-stage ring (`input->edit`, `edit->layout`, `layout`, `render`) | unchanged semantics |
| `frame_submit` | GPUI Windows | unavailable (A1-known); layout->submit not reported |

The end-to-end metric of interest is **edit → DrawList / frame**, i.e.
`gf_us` (PJS) and `prepaint_us + paint_us + concat_us + lines_us` (GPUI).
Presentation (compositor/display) is out of scope for both MVPs; per
benchmark integrity rules, no timestamps with different semantics are
compared as if equivalent (e.g. PJS `input->edit` and GPUI `input->edit`
are different input paths — programmatic keystroke vs scripted forward —
and are not compared).

## 4. PocketJS decomposition

Edit tick (windowed, `--perf`, p50 across runs):

### 4.1 Scaling (Q1)

| corpus | `gf_us` | `scanMs` (per edit) | `copyMs` | visible lines | words |
|-------:|--------:|--------------------:|---------:|--------------:|------:|
| 10K | 13.5 ms | 2 ms | ~0 | 26 | 3046 |
| 100K | 32.8 ms | 22 ms | ~0 | 26 | 3046 |
| 1M | 217.7 ms | 206 ms | ~0 | 26 | 3046 |

- The guest turn scales with total bytes `N`; the scan is linear
  (2/22/206 ms ≈ 10× per 10× bytes) and is ~95% of the turn at 1M.
- The concat copy is negligible (<1 ms at 1M, memcpy-rate; the `Date.now()`
  ms clock can only bound it — see §12).
- Host stages are constant across `N` (`ct_us` 68–100 µs, `dl_us` 52–84 µs,
  `words` 2976–3046, render pass 361–452 µs, idle `gf_us` 57–59 µs).
  The PocketJS host is viewport-sized, not document-sized.

### 4.2 Position (Q2)

| position | caret line | `gf_us` |
|----------|-----------:|--------:|
| begin | 0 | 218.3 ms |
| q1 | 4913 | 189.4 ms |
| mid | 9826 | 182.0 ms |
| q3 | 14739 | 198.4 ms |
| end | 19652 | 192.7 ms |

Edit position does not change the cost: the scan is always over the whole
document (the per-edit `scanMs` was 166–190 ms across positions vs 206 ms
in the scaling cell — run-to-run noise, same order). The residual spread
(±10%) is machine noise, not a position effect.

> **A4 erratum (2026-08-17):** the `caretFromX` click bug (see the
> erratum in `docs/phase-a3-intervention-validation.md`) was present in
> this battery: every click on a line ≥ 1 placed the caret at the
> document end, so the q1/mid/q3/end cells above are end-position edits
> and begin is the only genuine non-end position. The qualitative
> conclusion (cost independent of position) is unaffected — every cell
> exercised the same O(N) scan; no finer position gradient may be read
> from this table. Affected files are marked in
> `results/summary/a2/INVALIDATED-caretFromX.md`. A corrected gradient
> exists only in the A4 re-measurement (§2.4 of the A4 closeout).

### 4.3 Viewport (Q3)

| viewport | edit at line | `gf_us` |
|----------|-------------:|--------:|
| inside | 10 | 183.2 ms |
| near | 30 | 182.3 ms |
| far | 9826 | 185.2 ms |

Viewport position does not change the cost either. The scan ignores the
viewport entirely.

> **A4 erratum:** the three viewport cells above are repeats of
> "edit at document end" for the same reason (clicks at lines 10/30/9826
> all landed the caret at the end). The conclusion is unaffected; the
> cells are not a viewport comparison.

### 4.4 Noop / same-value control (Q4)

| control | what happens | `gf_us` |
|---------|--------------|--------:|
| `noop-empty` | 50 empty inserts (`--type ""`), guest skips mutation | 0.14 ms |
| `noop-left` | caret moves left 50×, document unchanged | 0.22 ms |

A turn in which the document does not change costs ~0.2 ms. The guest
memo pipeline (Solid `createMemo` over `doc`) short-circuits on unchanged
documents: no scan, no visible-list rebuild. So of the 217.7 ms edit turn,
~217.5 ms is work that only runs because the document changed.

### 4.5 Per-tick counters — supporting evidence

Per-tick JSONL confirms the shape of the turn: `ev` (edits forwarded)
matches the script (101), idle turns are 57–59 µs, `words` is constant
(3046) — the DrawList regeneration is not what grows with the document.

### 4.6 Counterfactual interventions (Q5)

The dist bundle was rebuilt with `perf.ts` compile-time toggles (`CF`, 0..3)
and the same instrumented exe was re-run at 1M with the identical workload:

| bundle | change | `gf_us` (ev>0 ticks) | vs CF=0 |
|--------|--------|---------------------:|--------:|
| CF=0 | baseline (current tree) | 217.7 ms | — |
| CF=1 | `lineStarts` scan skipped (load-time index returned) | 7.5 ms | **29× faster** |
| CF=2 | concat skipped (document never changes) | 1.4 ms | 155× faster |

- **CF=1 removes the scan and 96.5% of the turn disappears.** The scan is
  the cause. The remaining 7.5 ms is the Solid re-render of the 26 visible
  line views + text measurement + svc sends (viewport-constant).
- CF=2 skips the concat, so the document identity never changes and the
  Solid memo pipeline short-circuits; the residual 1.4 ms is the edit
  plumbing (selBounds, applyState, caret signal, svc sends) with no
  document work. It independently confirms that the concat itself
  (measured ≤1 ms) is not a causal factor.
- Both bundles were replaced by the CF=0 bundle afterwards (SHA-verified,
  §9).

**Decomposition of the 1M edit turn (217.7 ms):**

| component | time | share | evidence |
|-----------|-----:|------:|----------|
| `lineStarts` full scan | ~206 ms | 94.6% | linear scaling; CF=1 removes it |
| Solid re-render of 26 visible lines + measure | ~7–11 ms | 3–5% | CF=1 residual; viewport-constant |
| edit plumbing (concat, selBounds, svc) | ~1.4 ms | 0.6% | CF=2 residual |
| host tick + DrawList + render | ~0.6 ms | 0.3% | constant counters |
| turn overhead with nothing to do | 0.14–0.22 ms | <0.1% | noop control |

## 5. GPUI decomposition

Per-frame counters (`--a2`, windowed, p50 across runs). Edit frames are
frames that follow a document edit.

### 5.1 Scaling (Q1) — smoke workload, scroll at top (`first=0`)

| corpus | `concat_us` | `lines_us` | `prepaint_us` | `paint_us` | lines shaped | glyphs | visible/last/total |
|-------:|------------:|-----------:|--------------:|-----------:|-------------:|-------:|--------------------:|
| 10K | 5 µs | 5 µs | 227 µs | 142 µs | 177 | 10 144 | 191/190/190 |
| 100K | 9 µs | 53 µs | 2623 µs | 1010 µs | 1782 | 100 549 | 1942/1941/1941 |
| 1M | 489 µs | 549 µs | 39 446 µs | 11 905 µs | 18 081 | 1 029 043 | 19 654/19 653/19 653 |

- `visible` = `ceil(bounds.height / line_height) + 1` where
  `bounds.height` is the **content height** (our `request_layout` sizes the
  element to `line_height × line_count`), so `visible ≈ total + 1` and
  `last = line_count`: prepaint shapes from the scroll position to the end
  of the document on every frame.
- Prepaint is linear in the shaped range (227 µs/2.6 ms/39.4 ms;
  ~2.2 µs/line). Paint is linear too (~0.66 µs/line + constant).
- Edit-side work (concat + line-index rebuild, both O(N) memcpy/scan) is
  0.5+0.55 ms at 1M — real, but 1/40 of prepaint.

### 5.2 Position (Q2) — caret placed at fraction, then 50 edits

| position | `first` (scrolled line) | lines shaped | `prepaint_us` |
|----------|------------------------:|-------------:|--------------:|
| begin | 0 | 18 081 | 44 335 |
| q1 | 4871 | 13 579 | 34 982 |
| mid | 9763 | 9075 | 19 786 |
| q3 | 14 719 | 4539 | 9857 |
| end | 19 629 | 24 | 58 |

The cost is exactly `(document_end − scroll_first)`: the element claims the
full content height, so "visible" is "everything below the scroll
position". Moving the caret changes `first` (the editor reveals the caret),
which changes the range — but the range is *below the scroll line*, not
the viewport.

### 5.3 Viewport (Q3)

| viewport | `first` | lines shaped | `prepaint_us` |
|----------|--------:|-------------:|--------------:|
| inside (click line 10) | 0 | 18 081 | 40 347 |
| near (click line 30) | 6 | 18 076 | 41 570 |
| far (click line 9826) | 9763 | 9075 | 20 188 |

Viewport location is irrelevant: the shaped range is `[first..doc_end]`
and `first` only changes if the caret reveal scrolls. The viewport size
(≈26 lines) never enters the calculation.

### 5.4 Static-redraw control (Q4)

`--a2-mode static` dispatches 50 `cx.notify()` redraw requests with no
document change:

- The run produced **exactly one frame** (initial paint): 50 back-to-back
  notifies coalesced into zero additional frames in this app's wiring
  (gpui 0.2.2 only redraws when a tracked entity invalidates the window;
  the deferred notify chain did not). A pure no-change "redraw storm" is
  therefore not observable in this wiring.
- The no-edit frames that *did* render (initial frame, and the frame after
  the a2 driver moves the caret in pos/vp modes) re-shape the **full**
  `[first..doc_end]` range at the same cost as an edit frame:
  - initial frame (cold): 491 ms prepaint, 18 081 lines — 27 µs/line
  - caret-set frame (warm, no edit, inferred from the two-frame p50
    of the pos/vp runs): ~33 ms prepaint, 18 081 lines
  - edit frames (warm): 2.2 µs/line (39.4 ms for the same 18 081 lines)
- So a redraw with **no document change** costs the same as an edit frame;
  there is no content-unchanged fast path in our element. The 12×
  cold-vs-warm difference is first-shape cost inside the framework's
  shaping stack (font/glyph initialization; hypothesis, see §12).

### 5.5 Counterfactual intervention (Q5)

The exe was rebuilt with the shaped range truncated to 26 lines
(`let last = (first + 26).min(last)` — deliberately breaks semantics:
lines below the viewport would not be shaped; diagnostic only) and the
identical smoke workload re-run at 1M:

| metric | clean exe | 26-line truncation | Δ |
|--------|----------:|-------------------:|----:|
| `prepaint_us` | 39 446 µs | 67 µs | **588×** |
| `paint_us` | 11 905 µs | 100 µs | 119× |
| `concat_us` + `lines_us` | 1038 µs | 1108 µs | unchanged |
| lines shaped | 18 081 | 23 | — |
| whole frame | ~52.4 ms | ~1.3 ms | **40×** |

- Narrowing the range to viewport-size removes the entire frame cost
  except the fixed edit-side work. The range, not the shaping per se, is
  the cause.
- The exe was rebuilt clean afterwards and verified: 1M smoke re-run
  reproduces 18 081 shaped lines / 38.7 ms prepaint (the same data as the
  original battery, §9).

**Decomposition of the 1M edit frame (52.4 ms):**

| component | time | share | evidence |
|-----------|-----:|------:|----------|
| prepaint shaping `[first..doc_end]` | 39.4 ms | 75% | linear in range; truncation removes it |
| paint over shaped lines | 11.9 ms | 23% | drops to 100 µs with truncation |
| `apply_edit` concat + line-index rebuild | 1.1 ms | 2% | unchanged by truncation |
| cold first-frame extra | ~12× warm | — | framework first-shape (hypothesis) |

## 6. Calibration — instrumentation overhead (Q6)

Same workload, three builds: original (A1 exe, no instrumentation),
instrumented-JSONL-off, instrumented-JSONL-on (same binary, gated).

PocketJS (`edit->layout` trace span, p50):

| build | 10K | 1M |
|-------|----:|---:|
| orig (A1 exe) | 11.7 ms | 200.5 ms |
| off | 13.7 ms | 219.8 ms |
| on | 12.3 ms | 224.5 ms |

GPUI (render trace span, p50):

| build | 10K | 1M |
|-------|----:|---:|
| orig (A1 exe) | 400 µs | 50 835 µs |
| off | 407 µs | 50 465 µs |
| on | 457 µs | 51 026 µs |

Instrumentation adds ≤17% (PJS, small absolute) and ≤14% (GPUI @10K,
+57 µs; <1% @1M). The conclusions of §4–§5 are orders of magnitude larger
than the instrumentation error. Note the machine drift vs the A1 report:
the same corpus/workload today measures 200.5 ms (PJS 1M) vs 142 ms in A1
(~1.4× slower today); GPUI is within 5% (50.8 ms vs 48.5 ms). A1 numbers
are cited only as reference, never mixed into A2 cells.

## 7. Complexity classification

Per AGENTS.md §7 (controlled scaling: N total bytes, B blocks, L line
length, V visible region, Δ changed region):

| component | complexity | driver | evidence |
|-----------|-----------|--------|----------|
| PJS `lineStarts` scan | **O(N)** | total bytes, per edit | 2/22/206 ms at 10K/100K/1M; position/viewport independent; CF=1 removes |
| PJS Solid re-render + measure | **O(V)** | visible lines (26) | constant across N; CF=1 residual ~7 ms |
| PJS edit plumbing | O(Δ)-ish | edit path | CF=2 residual 1.4 ms, constant |
| PJS host tick/DrawList/render | O(V) | viewport | `words`=3046, `ct/dl/r` constant across N |
| GPUI prepaint range | **O(N − scroll)** | document below scroll | linear 227 µs/2.6/39.4 ms @first=0; position table; truncation |
| GPUI paint | O(shaped) | same range | 142/1010/11 905 µs |
| GPUI concat + line index | O(N) | bytes | 10/62/1038 µs (10K/100K/1M), memcpy+scan rate |
| GPUI per-line shape | O(1) | line length | 2.2 µs/line warm (cache hit), 27 µs cold |

Neither MVP's scaling is driven by `B`, `L` (beyond per-line work) or `V`
— both are driven by **N** (PJS: scan; GPUI: shaped range), i.e. a local
edit whose cost grows with total document size — the strong signal
AGENTS.md §7 calls out. The GPUI range is additionally a function of
scroll position (`first`), i.e. it *shrinks* as you scroll down — which is
why `pos-end`/`vp-far` look fast: not because the viewport matters, but
because there is nothing left below the scroll line to shape.

## 8. Causal graph

```
PocketJS keystroke (1M):
  host forwards edit ──► guest turn (gf_us 217.7 ms)
                            │
                            ├─ lineStarts(doc) 206 ms  ◄─ O(N) scan, every edit
                            │     └─ called by our Solid memo over doc()
                            ├─ Solid re-render of 26-line map  ~7-11 ms  ◄─ O(V)
                            ├─ typeText concat + plumbing      ~1.4 ms
                            └─ svc sends / state               ~0.2 ms
  host: core tick 97 µs + DrawList 74 µs + render ~0.45 ms  ◄─ O(V), constant

  CF=1 (skip scan) → 7.5 ms   ⇒  scan is the cause of 94.6% of the turn

GPUI keystroke (1M):
  keystroke → apply_edit: concat 0.47 ms + rebuild_line_starts 0.55 ms
  frame → EditorElement::request_layout sizes element to content height
       → prepaint: visible = ceil(content_h/lh)+1 = total+1
                   last = line_count  ⇒  shape [first..doc_end] every frame
                   18081 lines × 2.2 µs (warm cache hit) = 39.4 ms
       → paint over shaped lines = 11.9 ms
  frame total 52.4 ms

  truncate range to 26 lines → 1.3 ms  ⇒  the range is the cause of 98%
```

Both graphs converge on the same design principle (observation, not
prescription): **the per-edit cost of each MVP is set by how much of the
document the edit path re-derives, not by the framework's dispatch,
layout, or render machinery.** PocketJS re-derives line boundaries from
the whole document; GPUI re-shapes every line below the scroll position.

## 9. Ownership

| component | owner | basis |
|-----------|-------|-------|
| PJS `lineStarts` full rescan per edit | **MARKIT** (`app/editor.ts`, `app/app.tsx` memo) | our algorithm + call site; nothing in the framework forces a full rescan |
| PJS Solid re-render of visible map | MIXED — framework reactivity (FRAMEWORK) re-renders our 26-line map (MARKIT structure) | count is viewport-constant, bounded |
| PJS edit plumbing | MIXED — our `typeText`/`applyState` + Solid signal diffing | small, bounded |
| PJS host tick, DrawList, wgpu | FRAMEWORK (pocketjs core engine + renderer) | constant, viewport-sized — not the problem |
| PJS idle turn overhead | FRAMEWORK (QuickJS turn + host dispatch) | 0.2 ms |
| GPUI element sizing → full-content range | **MARKIT** (`EditorElement::request_layout`: `content_h = lh × line_count`; `visible` math uses bounds height; the "skip off-screen" comment is defeated by the sizing) | the counterfactual changes only this and removes the cost |
| GPUI per-line shaping (`shape_line`) | FRAMEWORK (gpui TextSystem / platform text system; two-frame `LineLayoutCache` — hits ~2.2 µs, full layout ~27 µs) | framework API + cache semantics verified in gpui 0.2.2 source and docs |
| GPUI paint loop | MIXED — our loop over our shaped lines; framework quad building | linear in range |
| GPUI edit-side concat + line index | **MARKIT** (`apply_edit`, `rebuild_line_starts`) | our code; same algorithm as PJS scan, 375× faster (see §11) |
| GPUI cold-first-frame 12× | FRAMEWORK (hypothesis: font/glyph first-use) | UNRESOLVED detail, bounded, not causal for per-edit scaling |

Markit owns both dominant terms. Neither the PocketJS core nor gpui is
the reason either MVP scales with document size; both frameworks'
machinery (DrawList regeneration, shaping) is either constant or
viewport-sized when driven through a viewport-bounded range.

## 10. Cross-framework comparison (same workload, L0–L1)

Only same-workload numbers are compared (AGENTS.md §6):

| 1M, one char edit | PocketJS | GPUI |
|--------------------|---------:|-----:|
| edit-side (concat + line index) | ≤1 ms + **206 ms scan** | 0.47 + 0.55 ms |
| visible-content redraw | ~7–11 ms (Solid, 26 lines) | 51.3 ms (shape 18 081 + paint) |
| host/frame fixed | ~0.6 ms | — |
| total | 217.7 ms | 52.4 ms |

- The same line-index operation (`scan for '\n'`, rebuild from scratch
  per edit) costs 206 ms in QuickJS (JS loop + array alloc) vs 549 µs in
  Rust — a 375× measured difference for this workload. Observation, not a
  law: the engines differ in loop and allocation behavior; the algorithm
  is identical (AGENTS.md §2: "Rust/native code is automatically faster"
  is not assumed — this is a measured single point, and the dominant PJS
  term is removable algorithmically either way).
- Both MVPs make the same structural choice: **full-document re-derivation
  per edit** (PJS: line starts; GPUI: shaping range). The frameworks'
  own fast paths (Solid memoization, gpui `LineLayoutCache` two-frame
  reuse) are either bypassed (gpui `reuse_layouts`/`truncate_layouts` are
  never called by our element) or defeated by the element sizing.
- Viewport virtualization is **not** the dominant optimization for
  PocketJS (its render path is already viewport-sized; the scan is not a
  rendering problem). For GPUI, viewport-bounding the shaped range *is*
  the intervention the counterfactual isolated.

## 11. Exit gate — READY_FOR_PHASE_A3

| requirement | status |
|-------------|--------|
| Complete experiment set: PJS scaling/position/viewport/noop/per-tick counters; GPUI scaling/position/viewport/static control | ✅ all cells run, raw + summaries archived |
| Counterfactual for each dominant term | ✅ PJS CF=1/CF=2; GPUI range truncation |
| ≥80% of end-to-end variance explained | ✅ PJS: 94.6% of the edit turn in one measured component, proven by CF=1; residual (7–11 ms) bounded between noop (0.2 ms) and CF=1 (7.5 ms) with the split identified but not fully separated (see §12). GPUI: 98% of the frame (75% prepaint + 23% paint) in the range, proven by truncation. |
| Ownership assigned | ✅ §9 — both dominant terms MARKIT |
| No optimization shipped; diagnostics removed/disabled | ✅ §12 audit |
| A3 gate | ✅ **READY_FOR_PHASE_A3** — A3 interventions should target the two MARKIT terms: PJS incremental line-index maintenance; GPUI viewport-bounded shaped range (e.g. engage the framework's cache seams). These are the *minimum* interventions the data supports; nothing else measured warrants one. |

## 12. Audit — nothing leaked into the product

Every code change in this phase was instrumentation (flag-gated) or a
diagnostic counterfactual. Post-experiment state:

- PocketJS: `perf.ts` CF toggles exist with `CF = 0` (all off); the A2
  dist bundle was restored to the CF=0 build and SHA-verified
  (`3287a938…` vs `/tmp/dist-cf0`). Host `--perf` gated; A1 ring trace
  untouched.
- GPUI: the 26-line truncation was removed from `editor.rs` and the exe
  rebuilt; a post-restore smoke run reproduces the original battery
  (18 081 lines shaped, 38.7 ms prepaint). The A2 workspace source is
  byte-identical to the repo tree.
- `git diff` vs the A1 merge shows only instrumentation modules and
  counters; no layout, buffer, or renderer behavior changed with
  instrumentation off (calibration §6: OFF ≈ ORIG within noise).
- No pushes, no PRs (per phase spec). Commits are local instrumentation
  commits.

## 13. Limitations

- `copyMs` uses the guest `Date.now()` clock (ms resolution): the concat
  is only bounded (≤1 ms at 1M), not measured precisely. The CF=2 result
  does not depend on it.
- CF=2 raw logs are contaminated by mouse-hover input (`in=1` on most
  ticks — the cursor happened to be over the window during those runs);
  the reported CF=2 numbers were recomputed from `ev>0` ticks only
  (101 per run). Noted in `results/raw/a2/README.md`.
- `gpui-smoke-1m-0` raw log was overwritten by the post-restore
  verification run (identical workload/exe); the original run's summary
  block is retained in the summary file. Noted in `results/raw/a2/README.md`.
- The GPUI static control (50 notifies) produced one frame in this app
  wiring; the no-edit redraw cost is measured via the pos/vp caret-set
  frames instead (same range, no edit).
- The 12× cold-first-frame factor is a framework hypothesis (font/glyph
  first-use in the platform text system), not fully attributed — it is a
  one-frame constant, not a per-edit scaling driver.
- The residual PJS 7–11 ms (Solid re-render vs our memo code) is bounded
  but not split; `ops` counting was disabled (`WRAP_OPS=false`) so the
  op-level split is not available. Not causal for the scaling conclusion.
- Corpus is U0 ASCII with fixed line lengths (B/L not varied); the
  O(N)/O(N−scroll) classifications are for this corpus family.
- Machine drift vs A1 (~1.4× for PJS) — A1 numbers cited as reference
  only. GPUI within 5%.
- `frame_submit` remains unavailable on GPUI Windows (A1-known); the
  presentation path is not part of this decomposition.

## 14. Artifacts

- `results/raw/a2/` — all raw logs (PJS: scale/pos/vp/noop/cf1/cf2/cal;
  GPUI: smoke/pos/vp/static/cf/cal), 120 files, plus README.md.
- `results/summary/a2/` — per-run parse-a2 + parse-trace summaries.
- `bench/run-a2.py` (driver), `bench/parse-a2.py` (JSONL aggregate),
  `bench/parse-trace.py` (A1 ring).
- Instrumentation: `mvp/pocketjs/app/perf.ts`, `app/editor.ts`,
  `app/app.tsx`, `app/svc.ts`, `app/main.tsx`, `src/main.rs`;
  `mvp/gpui/src/a2.rs`, `src/editor.rs`, `src/main.rs`.
- Framework references: gpui 0.2.2 `text_system.rs`,
  `text_system/line_layout.rs` (two-frame `LineLayoutCache`,
  `layout_line` keyed by text+font+size, previous-frame reuse),
  `app.rs` notify→invalidate wiring, `window.rs` dirty/draw scheduling.
