# Phase A3 — Root-Cause Intervention Validation & Fair Re-benchmark

Status: **READY_FOR_PHASE_A4** — both A2 root causes removed with minimal
Markit-owned interventions; both A2 counterfactual predictions reproduced
by the production implementations (PJS 1M edit 224.8→9.9 ms vs CF=1
~7.5 ms; GPUI 1M frame ~52.4→1.21 ms vs CF ~1.3 ms); full A3 battery,
startup, memory and idle baselines captured on the A1 Windows machine;
one integration bug found and fixed during A3-M (see §3).

> ## ⚠️ A4 Erratum (2026-08-17) — click-position provenance
>
> Read this before quoting any A3 click-derived cell. A4's R2 position
> cases uncovered a pre-existing `caretFromX` bug (`mvp/pocketjs/app/
> editor.ts`, present since the A1 MVP): the click line was computed as
> `Math.min(Math.max(0, lineIndex * doc.length), doc.length)`, so every
> click on a line ≥ 1 placed the caret at the **end of the document**
> and ignored the x-coordinate. Fixed in A4 (direct `starts` lookup,
> commit `1aefbfd`) and re-measured.
>
> | A3 data | status | reason |
> |---------|--------|--------|
> | `pjs-pos-begin` (1M) | ✅ valid | click at line 0 maps correctly |
> | `pjs-pos-end` (1M) | ✅ valid by coincidence | caret at document end is the intended position |
> | `pjs-pos-q1/mid/q3` (1M) | ⛔ **INVALIDATED** | caret landed at document end; these cells measured end-position edits |
> | `pjs-vp-inside/near/far` (1M) | ⛔ **INVALIDATED** | all three clicks (lines 10/30/~9826) also landed at document end |
> | `pjs-scale-*` (all corpora) | ✅ valid | typing-only workload, no clicks |
> | GPUI cells (pos/vp/smoke/scale) | ✅ valid | Rust-side click path, unaffected |
>
> Affected summary files are kept in place but marked superseded in
> `results/summary/a3/INVALIDATED-caretFromX.md`. The A2 PJS pos/vp cells
> carry the same latent corruption (`results/summary/a2/
> INVALIDATED-caretFromX.md`); A2's qualitative conclusion — the per-edit
> scan is O(N) at every position — is unaffected because every cell
> measured the same O(N) scan cost.
>
> **A3 conclusions that still hold:** both counterfactual chains (PJS
> `lineStarts` full-scan removal 224.8→9.9 ms; GPUI viewport bounding
> ~52.4→1.2 ms) rest on typing workloads (`pjs-scale`, `gpui-smoke`),
> which contain no clicks. The begin/end position gap (~2.4 ms @1M — the
> suffix-shift term, O(lines after edit)) is anchored by the two valid
> cells and stands.
>
> **A3 numbers replaced by A4:** the position table's intermediate cells
> (8.2/8.2/7.3 ms) and the viewport table (8.9/9.3/8.8 ms) were all
> end-position edits — treat them as end-position data, not position
> gradients. A4 re-measured the full 1M gradient with the fix (after the
> stable-item fix): begin 4.18 / q1 3.66 / mid 2.79 / q3 2.41 / end
> 1.91 ms (`docs/phase-a4-final-research-closeout.md` §2.4).

## 0. The answer in one paragraph

Both A2-confirmed Markit-owned root causes were removed with minimal,
correct interventions, and both A2 counterfactual predictions were
reproduced by the production implementations:

- **PocketJS**: the per-edit full-document `lineStarts()` scan was replaced
  by an incrementally maintained `LineIndex` (one full scan at load, local
  updates per edit). The 1M one-character edit turn went from **224.8 ms →
  9.9 ms p50** (A2 CF=1 predicted ~7.5 ms — within 1.3×), full scans per
  edit went **1 → 0**, and the 10K→1M scaling amplification collapsed from
  ~25× to ~1.1× (within run-to-run noise). The remaining position
  signature (begin ~2.4 ms slower than end at 1M) is the instrumented
  suffix-shift term — O(lines after edit), the honest replacement for
  A2's O(chars) scan.
- **GPUI**: `EditorElement::request_layout` now sizes the element to the
  **viewport** instead of the full content height, so prepaint's shaped
  range is `visible + 2 overscan` lines instead of `[scroll_line ..
  document_end]`. The 1M edit frame went from **~52.4 ms → ~1.2 ms**
  (A2 26-line CF predicted ~1.3 ms), prepaint from **39.6 ms → 64 µs**
  (CF predicted 67 µs), and `lines_shaped` from **18,081 → 25**, while the
  logical scroll extent (19,653 lines, scroll to document end) is
  preserved.

The two interventions are **independent and separately validated** (A3-P1/P2
before A3-G1/G2), no PocketJS framework patch was needed (`vendor/pocketjs`
SHA unchanged), and no unrelated optimization was mixed in.

## 1. Baseline

```text
A2 merge/base SHA:        d859db1 (master, post-A2-merge)
A3 branch:                perf/phase-a3-intervention-validation
Markit HEAD:              75b3014 (A3 docs commit; branch
                          perf/phase-a3-intervention-validation — see §19)
PocketJS submodule SHA:   cadffef50b0359e1a069586b9dc5574d65d7fb05 (unchanged)
GPUI:                     gpui 0.2.2
Windows environment:      win11_dt, Windows 11, AMD Ryzen 7 5800H,
                          AMD Radeon (integrated), 32 GB RAM, 1000x700
                          window, Consolas 18 px, 28 px line height
Toolchain:                rustc 1.96.0 x86_64-pc-windows-msvc (Windows
                          native builds, same as A1/A2), bun 1.2.8
Corpora:                  10K/100K/250K/500K/1M fixed-seed ASCII (U0),
                          250K/500K added by A3 (same generator/seed)
```

## 2. A3-0 — Baseline revalidation (A3 BEFORE)

The A2 exes were re-run unchanged before any code modification
(`results/raw/a3/before/`). Causal signatures reproduced:

| probe | A2 | A3 BEFORE |
|-------|-----|-----------|
| PJS 1M edit turn (`gf_us` p50) | 217.7 ms | 224.8 ms |
| PJS per-edit `scanMs` @1M | 206 ms | 195–248 ms |
| PJS `lineStartsScans` per run | 104 | 104 (1 per edit) |
| GPUI 1M `prepaint_us` p50 | 39 446 µs | 39 569 µs |
| GPUI 1M `lines_shaped` | 18 081 | 18 081 |
| GPUI 1M `visible` (element-bounds) | 19 653+1 | 19 654 |

## 3. A3-P1 — PocketJS incremental line index

### Old behavior

Every edit ran `lineStarts(doc)` inside the Solid memo over `doc()`:
a full-document scan for `\n` (O(N) in bytes), 1 × per edit, called from
Markit's memo (A2: 94.6% of the 1M edit turn).

### Root cause (A2)

`lineStarts()` full rescan per edit — ownership `MARKIT_POCKETJS_IMPLEMENTATION`.

### New design

`LineIndex` (mvp/pocketjs/app/editor.ts), an explicit line-index
abstraction owned by the model:

- **Construction** does the one allowed full scan (load time, O(N)),
  counted by `fullScans`.
- **`applyEdit(start, end, text)`** updates the index locally for
  `replace [start, end) → text`:
  1. locate the affected line range (`lineOf(start)` … `lineOf(end)`);
  2. drop line-start entries inside the replaced range
     (`newlinesDeleted`);
  3. add entries for `\n` positions in the inserted text
     (`newlinesInserted`);
  4. shift the remaining suffix entries by the length delta
     (`entriesAdjusted`, O(lines after edit));
  5. never re-scans the document.
- **Changed-range propagation**: edit functions return
  `EditResult { state, change }`; `applyState` applies the change to the
  index *before* flipping the `doc` signal (AGENTS.md §13 explicit
  changed-range propagation). Caret-only moves carry `change: null` and
  touch neither the index nor the document signal.
- The app memo is now `createMemo(() => { void doc(); return lineIndex.starts; })`
  — O(1) per edit. Reactivity with the stable array reference was verified
  empirically (Solid re-evaluates downstream on doc change; see the test
  note in the P1 commit).
- Offset unit unchanged (JS code units); caret/selection semantics
  unchanged.
- `LineIndex.verify(doc)` is the reference full-scan oracle — **test-only**,
  never on the production path.

### Correctness strategy

`mvp/pocketjs/app/line-index.test.ts` (38 tests, `bun test`): deterministic
cases (insert char/newline/range at begin/middle/end, delete char/newline,
replace across lines, empty document), randomized differential testing
(1000 seeded random edits on 10K; 500 edit-function ops), and
10K/100K/1M smoke validation — the incremental index equals the reference
full-scan index after every edit, `fullScans` stays 1, and
`entriesAdjusted`/newline counters are non-zero where expected.

Windows verification: windowed smoke state echoes byte-identical to A1
(`Hi!` typing, backspace ×2, Enter, SelectAll, scroll, resize — same
caret/docHead sequence, 61 ticks / 7 frames); headless scripted check
confirmed click-caret, typing, Backspace, Enter, scroll semantics.

### Work-count change

| counter | A2 | A3 |
|---------|----|----|
| `lineStartsScans` / edit | 1 | 0 (2 total per run: seed + load) |
| suffix `entriesAdjusted` / edit @1M begin | — | 19 652 |
| `newlinesInserted` / `newlinesDeleted` | — | counted |

### Before/after timing (Windows, p50 of run medians, 3 runs per cell)

| corpus | A2 `gf_us` | A3 `gf_us` |
|-------:|-----------:|-----------:|
| 10K | 13.5 ms | 8.9 ms |
| 100K | 32.8 ms | 10.4 ms |
| 250K | — | 9.0 ms |
| 500K | — | 9.8 ms |
| 1M | 217.7 ms | 9.9 ms |

(Run-to-run drift between batteries is ±10–20% on this machine — the same
order as the A2 calibration observed; the 100K cell above is within that
band. The per-edit scan — the A2 scaling driver — is gone at every size.)

### Position scaling (1M, `gf_us` p50)

| position | A2 | A3 |
|----------|-----|-----|
| begin | 218.3 ms | 9.8 ms |
| q1 | 189.4 ms | 8.2 ms |
| mid | 182.0 ms | 8.2 ms |
| q3 | 198.4 ms | 7.3 ms ⛔ |
| end | 192.7 ms | 7.4 ms ✅¹ |

> ⛔ q1/mid/q3 A3 cells are **INVALIDATED** by the `caretFromX` bug (A4
> erratum above): the click landed the caret at the document end, so they
> measured end-position edits. ✅¹ end is valid by coincidence (caret at
> document end is its intended position). The begin/end anchor cells
> support the suffix-shift claim below; the intermediate "gradient" does
> not exist in this data. The full gradient was re-measured in A4: begin
> 4.18 / q1 3.66 / mid 2.79 / q3 2.41 / end 1.91 ms @1M (post-fix).

A2 was position-independent because the scan is O(N) everywhere. A3 shows
the expected suffix-shift signature: begin edits shift all 19 653 entries
(~2.4 ms), end edits shift none — i.e. A2's O(chars) became O(lines after
edit), per the phase spec §11, recorded honestly rather than hidden. Both
ends stay inside the CF-predicted order of magnitude.

### Viewport (1M, `gf_us` p50)

| viewport | A2 | A3 |
|----------|-----|-----|
| inside | 183.2 ms | 8.9 ms ⛔ |
| near | 182.3 ms | 9.3 ms ⛔ |
| far | 185.2 ms | 8.8 ms ⛔ |

> ⛔ All three A3 viewport cells are **INVALIDATED** by the same bug (A4
> erratum above): every click landed the caret at the document end, so
> this table is three repeats of "edit at end with different click
> coordinates", not a viewport comparison.
### Tail latency (edit→layout trace span, us)

| corpus | A2 p50/p99 | A3 p50/p95/p99 |
|-------:|-----------:|----------------|
| 10K | 8 817 / 11 419 | 8 175 / 9 305 / 10 211 |
| 100K | 20 792 / 24 524 | 8 190 / 11 528 / 14 396 |
| 1M | 142 007 / 155 838 | 9 225 / 13 059 / 13 456 |

Long frames (>16.7 ms layout): A2 ~11–40 per run at 10K–1M → A3 0–1 per run.

### Residual bottleneck

The suffix offset update: O(lines after edit), ~19 652 entries shifted per
begin-position edit at 1M (~2 ms, measured by the begin/end position gap).
Recorded as the A3 residual for PocketJS; a candidate for a later
index-design revision (A4), not fixed in A3 per the phase rules.

### Integration bug found and fixed during A3-M (idle measurement)

The first A3-P1 bundle had a load-order bug: the `load` handler flipped
the `doc` signal **before** rebuilding the line index. Solid re-renders
synchronously on the signal write, so the memo re-evaluated with the old
(seed) index against the new document — the last visible line's slice then
extended to the document end, leaving a **whole-document text run** in the
retained tree. That node was shaped on every `ui.draw()` call:
`layout_run` is O(run length) and the word output is clipped to the
viewport, so the DrawList stayed viewport-sized (2457 words) but the build
cost became O(document): 206 µs / 1.7 ms / 16.8 ms at 10K / 100K / 1M
(A2: ~85 µs flat), and idle CPU read ~1.2 cores at 1M. The exe was
identical (A2 bundle on the same exe measured 85 µs) — the bundle was the
cause.

Fix: rebuild the index **before** `setDoc` (one line reorder, in the P1
commit). Verified: idle `dl_us` 84–87 µs at 10K/100K/1M, words 2980, idle
CPU ~1% (0.01 cores), 901 ticks / 1 frame rendered on a clean exit —
matching the A2 bundle's idle behavior. The buggy-bundle raw logs are
archived separately (`results/raw/a3/after-buggy/`); all final A3-P2 and
A3-M numbers below are from the fixed bundle.

## 4. A3-G1 — GPUI viewport-bounded rendering

### Old range contract

`EditorElement::request_layout` sized the element to
`line_height × line_count` (full content height). Prepaint then computed
`visible = ceil(bounds.height / lh) + 1` from that content height, so the
shaped range was `[first_scroll_line .. document_end]` — 18 081 lines at
1M, every frame, edit or not.

### New viewport contract

`request_layout` sizes the element to the viewport (`relative(1.)` on
both axes — the element is the child of the full-size root div). Prepaint's
`visible` is then the real viewport count (26 at 700 px / 28 px), plus an
explicit `OVERSCAN_LINES = 2`:

```rust
let visible = (pxf(bounds.size.height) / lh).ceil() as usize + 1;
let last = (first + visible + OVERSCAN_LINES).min(line_count);
```

No hardcoded 26; the workset is derived from the live element bounds, so
it is resize-aware.

### Logical extent preservation

The document's logical extent lives in the ThinEditor model and is
unchanged: `line_count`, and the `scroll_y` clamps in `on_scroll_wheel`
and `ensure_cursor_visible` (both use `content_h − viewport_h` from the
model). Verified by the 1M smoke: after select-all at the document end,
`scroll_y` reaches ~549 700 px (document end) and `lines_total` stays
19 653. Only the paint/layout workset changed.

### Work-count change

| counter @1M | A2 | A3 |
|-------------|----|----|
| `visible` | 19 654 (content-based) | 26 (viewport-based) |
| `lines_visited` (last−first) | 18 081 | 28 |
| `lines_shaped` (non-empty) | 18 081 | 25 |
| `lines_total` | 19 653 | 19 653 |
| `overscan` | — | 2 |

### Before/after timing (Windows, p50 of run medians, 3 runs per cell)

Edit frames (smoke workload):

| corpus | A2 prepaint | A3 prepaint | A2 paint | A3 paint |
|-------:|------------:|------------:|---------:|---------:|
| 10K | 227 µs | 55 µs | 142 µs | ~100 µs |
| 100K | 2 623 µs | 54 µs | 1 010 µs | ~100 µs |
| 250K | — | 57 µs | — | ~100 µs |
| 500K | — | 96 µs | — | ~100 µs |
| 1M | 39 446 µs | 64 µs | 11 905 µs | 106 µs |

Whole 1M edit frame (prepaint + paint + concat + line-index rebuild):
**~52.4 ms → ~1.21 ms** (A2 26-line CF predicted ~1.3 ms).

### Static redraw (no-edit frames)

| corpus | A2 prepaint | A3 prepaint |
|-------:|------------:|------------:|
| 10K | ~230 µs | ~47–55 µs |
| 1M | 42 712 µs (static mode) / ~33 ms (caret-set) | 47–102 µs |

10K→1M static redraw ratio: A2 ~180× → A3 ≤ 1.8× (spec gate ≤ 2× PASS).

### Position scaling (1M edit frames, `prepaint_us` p50)

| position | A2 | A3 |
|----------|-----|-----|
| begin | 44 335 µs | 67 µs |
| q1 | 34 982 µs | 69 µs |
| mid | 19 786 µs | 98 µs |
| q3 | 9 857 µs | 95 µs |
| end | 58 µs | 51 µs |

A2's "faster at the end" signature (cost = document below the scroll
line) is gone; A3 is flat within noise (51–98 µs, no monotonic trend).

### Viewport (1M, `prepaint_us` p50)

| viewport | A2 | A3 |
|----------|-----|-----|
| inside | 40 347 µs | 69 µs |
| near | 41 570 µs | 63 µs |
| far | 20 188 µs | 102 µs |

### Input-side secondary bottleneck (NOT fixed, per spec §28)

`apply_edit` concat + `rebuild_line_starts` at 1M: ~465 + ~578 µs —
now the largest single GPUI term, still O(N) per edit. The A3 phase spec
explicitly defers it: listed as an **A4 candidate**, not touched here.

### Residual bottleneck

GPUI: the edit-side line-start rebuild (~1 ms at 1M). Everything
render-side is viewport-bounded (~0.2 ms).

## 5. A2 prediction validation

| Root cause | A2 prediction | A3 observed | Validated? |
|------------|---------------|-------------|------------|
| PJS full scan removed | turn approaches CF=1 ≈ 7.5 ms @1M | 9.9 ms begin / 7.4 ms end (1.3× CF at worst position) | ✅ |
| PJS scans per edit | 0 | 0 (2 total: seed+load) | ✅ |
| GPUI bound to viewport | frame approaches CF ≈ 1.3 ms @1M | ~1.21 ms | ✅ |
| GPUI prepaint | approaches CF ≈ 67 µs | 64 µs | ✅ |
| GPUI lines shaped | ≈ visible + overscan | 25 (28 visited) | ✅ |
| GPUI position dependence | removed | flat 51–98 µs | ✅ |
| PJS 10K→1M amplification | gone (CF: no O(N) term) | ~1.1× (noise band) | ✅ |
| GPUI 10K→1M static redraw | ≤ ~2× | ≤ 1.8× | ✅ |

## 6. Common Windows re-benchmark (A3-R)

Same machine (win11_dt), same GPU, same 1000×700 window, same corpora,
release builds (rustc 1.96.0, Windows-native), fixed bundle/exes. PJS
workload: the A1 typing trace (100 single-char inserts + backspace +
scroll). GPUI workload: `--smoke` (same shape + IME/select-all steps, A2
convention) and `--a2-mode scale` (100 keystrokes only — the closest
match to the PJS trace).

| Metric | corpus | A2 GPUI | A3 GPUI | A2 PocketJS | A3 PocketJS |
| ------ | ------ | ------: | ------: | ----------: | ----------: |
| one-char edit (frame/turn) p50 | 10K | 0.37 ms | 0.12 ms | 13.5 ms | 8.9 ms |
| one-char edit (frame/turn) p50 | 100K | 3.6 ms | 0.17 ms | 32.8 ms | 10.4 ms |
| one-char edit (frame/turn) p50 | 1M | 52.4 ms | 1.21 ms | 217.7 ms | 9.9 ms |
| edit p99 | 1M | 65.6 ms (render) | 0.65 ms | 155.8 ms | 13.5 ms |
| long frames (>16.7 ms) | 1M | many (40 ms frames) | 0 | ~40 | 0–1 |
| startup → first usable frame | — | n/a (A1 gap) | 336 ms | n/a (A1 gap) | 750 ms |
| working set @1M (idle) | — | n/a | 39.0 MiB | n/a | 235.6 MiB |
| private bytes @1M (idle) | — | n/a | 35.0 MiB | n/a | 223.5 MiB |
| idle CPU | — | n/a | 0.5–2.1% | n/a | ~1% |
| idle rendering | — | n/a | ~0 frames/s | n/a | ~0 frames/s (1/901 ticks) |

Boundary note (unchanged from A2): GPUI columns are edit-frame
prepaint+paint+edit-side work; PocketJS columns are the guest turn
(`gf_us`, edit → DrawList). PJS `input→edit` (~0.1–0.2 µs) and GPUI
`input→edit` (~1.05 ms @1M, the UTF-16/line-index input path) are
different input paths and are not compared (A2 §3).

## 7. Startup (A3-M)

Both MVPs print `MARKIT_FIRST_USABLE_FRAME <ms>` — process-internal
delta from main entry to the first application-level frame-ready
(document visible, editor wired, first buffer submitted/painted). No OS
present timestamp is available (A1-known; labeled frame-ready). External
runner: `bench/startup-memory.py`, 1 warmup + 5 measured launches per
cell.

| MVP | corpus | median | min | max |
|-----|--------|-------:|----:|----:|
| PocketJS | 100K | 609 ms | 597 | 638 |
| PocketJS | 1M | 750 ms | 739 | 773 |
| GPUI | 100K | 342 ms | 334 | 360 |
| GPUI | 1M | 336 ms | 333 | 344 |

The dominant PocketJS term is rquickjs eval + pak feed + first guest
frame (A1-observed ~581 ms boot); GPUI's is the platform/window init.
GPUI is ~2.2× faster at 1M. (The first A3-P1 bundle measured 691/927 ms —
the load-order bug's O(doc) text run also delayed the first frame; the
fixed bundle numbers are above.)

## 8. Memory (A3-M)

Sampled at marker+4 s (idle, 3 s after load), 6 launches per cell,
median of per-run samples:

| MVP | corpus | Working Set | Private Bytes |
|-----|--------|------------:|--------------:|
| PocketJS | 100K | 234.3 MiB | 222.4 MiB |
| PocketJS | 1M | 235.6 MiB | 223.5 MiB |
| GPUI | 100K | 37.9 MiB | 33.8 MiB |
| GPUI | 1M | 39.0 MiB | 35.0 MiB |

GPUI holds the document + shaping data in ~40 MiB regardless of corpus
(100K→1M: +1.1 MiB); PocketJS's baseline runtime (QuickJS, Solid, baked
atlases, retained tree) is ~230 MiB before the document matters
(100K→1M: +1.3 MiB). ~6× WS difference.

## 9. Idle behavior (A3-M)

| MVP | CPU (3 s window) | rendering |
|-----|-----------------:|-----------|
| PocketJS | ~1% (0.010 cores) | demand rendering: 901 ticks, 1 frame (0.1%) |
| GPUI | 0.5–2.1% | no redraw without notify: 2 frames per idle run |

Both MVPs do essentially no idle work after the initial frame. (The first
A3-P1 bundle measured 1.19 cores idle — caused by the load-order bug,
§3; fixed and re-measured.)

## 10. Work amplification table (A3)

| PocketJS | A2 | A3 |
|----------|----|----|
| full scans / edit | 1 | 0 |
| index entries adjusted / edit @1M begin | — | 19 652 |
| visible lines | 26 | 26 |
| lines rendered | 26 | 26 |
| DrawList words | 2 980 | 2 980 (edit: 3 046) |
| idle DrawList rebuild | ~85 µs | 84–87 µs |

| GPUI | A2 | A3 |
|------|----|----|
| visible lines | 19 654 (content) | 26 (viewport) |
| lines visited | 18 081 | 28 |
| lines shaped | 18 081 | 25 |
| lines painted | 18 081 | 25 |

Core conclusion:

```text
A2: one edit/frame touched O(document)
A3: one edit/frame touches local (PJS: changed range + suffix shift;
    GPUI: viewport + overscan) work
```

Residual amplification: PJS O(lines after edit) suffix shift (position
dependent, instrumented); GPUI O(N) edit-side line-index rebuild
(1 ms @1M, A4 candidate).

## 11. Correctness regression matrix

| check | PocketJS | GPUI |
|-------|----------|------|
| Windows launch | PASS (smoke + marker runs) | PASS (smoke + marker runs) |
| text rendering | PASS (smoke state + headless) | PASS (smoke) |
| caret | PASS (byte-identical A1 echo) | PASS (smoke cursor math) |
| typing | PASS | PASS |
| Backspace/Delete | PASS | PASS (backspace ×2) |
| Enter | PASS (H\n echo) | PASS (lines 19653→19654) |
| arrows | PASS (headless) | PASS (end keystroke) |
| selection | PASS (SelectAll echo) | PASS (cmd-a sel=0..N) |
| scroll | PASS (scroll_y echo, clamp) | PASS (scroll to doc end) |
| resize | PASS (1200x800 echo) | PASS (1200x800) |
| IME | not tested (deferred A1) | PASS (smoke ime-commit) |
| line index vs reference | PASS (38 bun tests) | — (rebuild unchanged) |

## 12. Framework ownership

| component | owner |
|-----------|-------|
| PJS LineIndex (new) | MARKIT |
| PJS scan removal | MARKIT (root cause fixed; no PocketJS patch) |
| GPUI element sizing (viewport) | MARKIT |
| GPUI shaping stack | FRAMEWORK (now viewport-sized; not a bottleneck) |
| GPUI edit-side line-index rebuild | MARKIT (A4 candidate, untouched) |
| PJS Solid visible-list re-render | FRAMEWORK+MARKIT (now the top PJS term, A4) |
| `vendor/pocketjs` SHA | **unchanged** — first giant PocketJS MVP
  bottleneck was entirely Markit integration, not the PocketJS runtime |

## 13. Residual bottlenecks / A4 candidates

1. **PJS Solid visible-list re-render ~7–9 ms** (26-line map + measure) —
   now the dominant PocketJS term (was the CF=1 residual).
2. **GPUI edit-side line-start rebuild ~1 ms @1M** (O(N) per edit).
3. **Real Markdown workload** (L1+) — both substrates unmeasured at L1+.

## 14. Limitations

- A3-M startup marker is application-level frame-ready (no OS present
  timestamp on GPUI Windows; A1-known). Both MVPs use the same boundary.
- Same as A2: corpus is U0 ASCII; B/L/V not independently varied;
  `copyMs`/`scanMs` guest clocks are ms-resolution (bounded only);
  machine drift vs A1 (~1.4× for PJS) — all cells in this report are
  same-day, same-machine.
- PJS position cell edits at the document beginning shift the full
  suffix (worst case); end-position edits are cheaper — the residual
  position dependence is honest and instrumented, not hidden.
- The A3-R comparison reuses the A2 conventions (PJS a1-typing vs GPUI
  smoke) plus a same-keystroke `--a2-mode scale` cell where noted.

## 15. Substrate comparison (A3-D)

Answers to the A3-D questions (all measured, same machine/corpus/workload,
after both interventions):

1. **Active-edit latency after removing the MVP O(N) bugs**: GPUI lower —
   ~1.2 ms vs ~9.9 ms per one-char edit at 1M (8×). Both are now
   flat-scaling; the gap is PocketJS's framework-owned visible-list
   re-render (~7 ms, the A2 CF=1 residual) vs GPUI's Markit-owned
   edit-side line rebuild (~1 ms).
2. **p99**: GPUI 0.65 ms vs PocketJS 13.5 ms at 1M (~20×).
3. **Scaling stability**: both flat — PocketJS 10K→1M ≈ 1.1× (noise band),
   GPUI 10K→1M static redraw ≤ 1.8×.
4. **Startup**: GPUI 336 ms vs PocketJS 750 ms at 1M (~2.2×).
5. **Memory**: GPUI 39.0 MiB vs PocketJS 235.6 MiB working set at 1M
   (~6×); private bytes 35.0 vs 223.5 MiB.
6. **Idle work**: equal — both ~0–2% CPU, ~0 frames/s after the initial
   frame (PocketJS demand rendering: 1 frame per 901 ticks; GPUI: no
   redraw without notify).
7. **Local invalidation seams**: both now viewport/local-bounded in the
   render path. PocketJS gained an explicit changed-range seam
   (`EditResult.change` → `LineIndex.applyEdit`); its remaining O(lines)
   term is the suffix shift. GPUI's remaining O(N) is Markit's own
   `rebuild_line_starts` on edit (a clean A4 target, no framework
   involvement).
8. **Framework workarounds**: one per MVP — GPUI: the element must be
   sized to the viewport in `request_layout` (a one-line contract fix,
   but the framework's content-height sizing is the trap that caused the
   A2 finding); PocketJS: the model must own a maintained index (Markit
   code, no framework change — `vendor/pocketjs` SHA unchanged).
9. **Windows capability gaps**: PocketJS exposes `frame_submit` (windowed)
   and CJK; GPUI does not expose `frame_submit` (A1-known) and its IME
   path is IMM32 (manual Pinyin verification pending for both). For the
   current MVP scope (L0–L1 ASCII, scripted input) neither gap blocks the
   comparison.

## 16. Decision

```text
RECOMMENDATION: LEAN_GPUI
```

based on the measured substrate properties after both A2 root causes were
removed:

- active edit latency: ~8× lower at 1M (1.2 vs 9.9 ms);
- p99: ~20× lower (0.65 vs 13.5 ms);
- memory: ~6× lower working set (39 vs 244 MiB);
- startup: ~2.2× faster (336 vs 750 ms);
- residual bottleneck ownership: GPUI's remaining O(N) term is
  Markit-owned and small (~1 ms, clean A4 target); PocketJS's remaining
  term is framework-owned (Solid visible-list re-render, ~7 ms) and
  larger.

tradeoffs:

- PocketJS retains genuine advantages: guest-side iteration (editor logic
  in JS, editable without a native rebuild), demand rendering with
  observable `frame_submit` on Windows, and the changed-range seam that
  A3 added. GPUI's frame_submit stays unavailable and its input path
  (UTF-16 conversion + line-index rebuild) is currently ~1 ms @1M
  (A4 candidate).
- The PocketJS decision-relevant cost is not a Markit integration bug any
  more; it is the framework's per-edit re-render cost for 26 visible
  lines. A4 should first decide whether that ~7 ms is a hard framework
  floor (measure a minimal Solid re-render without text) before any
  PocketJS-based product work.
- If Markit targets very large documents with heavy L2+ projections,
  GPUI's shape-on-demand + Rust model has the shorter remaining path
  (fix `rebuild_line_starts`); if it targets rapid iteration and
  guest-side product logic, PocketJS is viable once the visible-list cost
  is understood.

## 17. A4 candidates (max 3)

1. **PocketJS Solid visible-list re-render** (~7 ms per edit at 1M —
   now the dominant PocketJS term; measure the framework floor with a
   minimal re-render experiment before optimizing).
2. **GPUI edit-side line-index rebuild + UTF-16 input path** (~1.5 ms at
   1M — Markit-owned, same incremental-index pattern as A3-P1 applies).
3. **Real Markdown workload (L1+)** — neither substrate is measured at
   L1+; both frameworks' fast paths are untested against projection
   churn.

Phase A3 does not execute A4.

## 18. Verdict

```text
READY_FOR_PHASE_A4
```

All A3 exit gates pass:

**PocketJS** — full scan removed from the edit hot path (0 per edit, 2 per
run: seed + load); LineIndex correctness tests PASS (38/38, incl.
randomized differential and 10K/100K/1M reference comparison); Windows
10K/100K/250K/500K/1M PASS; begin/middle/end PASS (position signature
matches the instrumented suffix shift); viewport experiments PASS;
1M latency inside the CF-predicted order of magnitude (7.4–9.9 ms vs
CF=1 ≈ 7.5 ms); all editor interactions remain correct (smoke echo
byte-identical to A1; headless caret/typing checks).

**GPUI** — no hardcoded 26-line hack (viewport-derived workset +
OVERSCAN_LINES = 2); logical full-document scroll extent preserved
(scroll to document end verified, lines_total unchanged); work range
derived from the live viewport (resize-aware); lines_shaped ≈ visible +
overscan (25 of 28 visited); 10K/100K/250K/500K/1M static redraw flat
(≤ 1.8×); position dependence removed (51–98 µs); scroll/resize/caret/
selection/IME regression checks PASS; 1M frame inside the CF-predicted
order of magnitude (1.21 ms vs CF ≈ 1.3 ms).

**Cross-cutting** — A3 common Windows benchmark completed (same machine/
GPU/power/display/window/corpora/release mode); startup captured
(MARKIT_FIRST_USABLE_FRAME, same semantic boundary both MVPs); memory
captured (WS + private bytes, 6 runs per cell); tail latency captured
(p50/p95/p99/max, long frames); A2 predictions checked (all ✅, §5);
correctness preserved (regression matrix §11); no unrelated optimization
mixed in; report complete.

## 19. Git state

```text
A3 branch:              perf/phase-a3-intervention-validation
Markit commits:
  46075d6 perf(pocketjs-mvp): maintain line index incrementally
  d3884d1 perf(gpui-mvp): bound editor work to viewport
  (bench(a3) + docs(a3) commits follow this report)
PocketJS submodule SHA: cadffef50b0359e1a069586b9dc5574d65d7fb05 (UNCHANGED —
                        no framework patch; the first giant PocketJS MVP
                        bottleneck was entirely Markit integration)
Working tree:           clean (post-audit)
Artifacts:
  results/raw/a3/before/      A2 exes revalidated (A3-0)
  results/raw/a3/after/       fixed-bundle intervention battery
  results/raw/a3/after-buggy/ first A3-P1 bundle (load-order bug; archived)
  results/summary/a3/         per-run summaries for all of the above
  bench/run-a3.py             A3 driver (before/after stages)
  bench/startup-memory.py     A3-M runner (marker + WS/private bytes + idle)
```

## 20. Limitations

- Same as A2: corpus is U0 ASCII; B/L/V not independently varied;
  `copyMs`/`scanMs` guest clocks are ms-resolution (bounded only);
  machine drift vs A1 (~1.4× for PJS) — all cells in this report are
  same-day, same-machine.
- The A3-P1 load-order bug (§3) affected only the first A3 bundle's idle
  behavior; the final numbers are from the fixed bundle, and the buggy
  logs are archived separately.
- PJS position cell edits at the document beginning shift the full
  suffix (worst case); end-position edits are cheaper — the residual
  position dependence is honest and instrumented, not hidden.
- The A3-R comparison reuses the A2 conventions (PJS a1-typing vs GPUI
  smoke) plus a same-keystroke `--a2-mode scale` cell where noted.
- Idle CPU is a coarse 3 s Get-Process CPU delta (single-sample noise
  ±1%); both MVPs' values are near the noise floor.
- Startup marker is application-level frame-ready for both MVPs (no OS
  present timestamp available on GPUI Windows).
