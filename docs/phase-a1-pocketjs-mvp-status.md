# Phase A1 — Markit PocketJS integration & thin editor MVP

> Historical research record.
>
> This document describes the PocketJS integration and thin-editor MVP
> that supported the A1–A4 substrate comparison. Markit later pivoted to
> direct GPUI as the product foundation — see
> [ADR-008](adr/ADR-008-direct-gpui-product-substrate.md). Measurements and
> findings are unedited.

Status: **READY_FOR_PHASE_A2** — PocketJS integration + thin editor MVP
done and verified on Windows (native); first-round GPUI/PocketJS benchmark
completed on the same Windows machine with the same corpora. (Report
updated 2026-08-16 with benchmark results.)

This report covers the Markit-side Phase A1 work: bringing the PocketJS
fork in as a vendored submodule, and building a Markit-owned PocketJS thin
editor MVP that mirrors the GPUI Phase A0 prototype. (The earlier fork-side
A1 work — `support/windows-desktop` branch — is historical reference only,
per the branch discipline below.)

## 1. Baseline

```text
Markit SHA:                (see Git state below)

GPUI:                      gpui 0.2.2, mvp/gpui baseline (Phase A0)
PocketJS upstream main:    cadffef50b0359e1a069586b9dc5574d65d7fb05
PocketJS fork main:        cadffef50b0359e1a069586b9dc5574d65d7fb05
                           (clean mirror of upstream, verified equal)
PocketJS feat/windows-mvp: cadffef50b0359e1a069586b9dc5574d65d7fb05
                           (based on clean main — no cherry-picks)

PocketJS submodule SHA:    cadffef50b0359e1a069586b9dc5574d65d7fb05
PocketJS submodule SHA pushed remotely?  YES (it equals origin/main)
```

## 2. Repository policy applied

```text
pocket-stack/pocketjs  → upstream/main (fetch-only)
jnhu76/pocketjs        → origin/main  (mirror; ff-only, no merge commits)
                       → feat/windows-mvp (integration/evidence branch)
```

- `origin/main == upstream/main` verified byte-identical (both `cadffef`).
- `feat/windows-mvp` created from clean `main`; the old
  `support/windows-desktop` (12 commits) and `pr1a/*` branches were NOT
  cherry-picked — they remain as historical reference only. Per policy,
  PocketJS is modified only when a real Markit workload proves the need.
- **PocketJS fixes: NONE** (no commits on `feat/windows-mvp` — the MVP
  runs on stock upstream `main`).

## 3. What was built (Markit-owned)

```text
mvp/pocketjs/
├── app/          guest: editor.ts (framework-free model), app.tsx (UI),
│                 svc.ts (host protocol), sample.ts (GPUI-identical seed)
├── src/          host: main.rs (boot, svc bridge, FlatWidget, smoke),
│                 instrument.rs (7-stage trace, GPUI-identical contract)
├── scripts/build-app.sh   guest bundle (bun build, Consolas override)
└── dist/         bundle output (gitignored)

workloads/
├── generate-corpus.py     fixed-seed generator (0xA1C0FFEE)
└── corpus/                10k.txt / 100k.txt / 1m.txt (ASCII)
```

Parity contract with `mvp/gpui`: 1000x700 window, Consolas 18 px, 28 px
line height, no soft wrap, same visible-line formula, byte-identical seed
document, same palette, same 7-stage trace names/format, same `--smoke`
step order (minus IME steps — deferred).

## 4. PocketJS MVP status

Verified on Windows native (AMD Radeon integrated GPU, DX12) via
`--smoke` windowed run and headless scripted runs:

| Capability | Status |
| --- | --- |
| Windows launch | PASS (opaque window; transparent=false) |
| text rendering | PASS (ASCII; CJK lines = tofu, see Deferred) |
| caret | PASS |
| typing | PASS |
| delete/backspace | PASS |
| arrows (+home/end, vertical) | PASS |
| selection | PASS (SelectAll verified; shift+arrow extends) |
| scroll | PASS |
| resize + relayout | PASS |
| CJK | FAIL (tofu) — DEFERRED |
| IME | NOT TESTED — DEFERRED (protocol reserved) |
| clipboard | NOT TESTED — DEFERRED |

Smoke evidence (Windows, windowed, `--file 10k.txt`):
```text
[state] tick=2  caret=3  text_head="Hi!Quick a fence editor..."
[state] tick=4  caret=2  (Backspace)
[state] tick=8  caret=2  (Enter: "H\nQuick a fence...")
[state] tick=10 caret=10283 anchor=0 (SelectAll)
[state] tick=12 scroll_y=56
[state] tick=14 vp=1200x800 (resize)
pocket-widget: 61 ticks, 7 frames rendered (11.5%) — demand rendering
Trace: input_received=6 edit_applied=5 layout=61/61 render=7/7 frame_submit=7
```

## 5. Instrumentation findings (PocketJS side)

- `frame_submit` is observable on PocketJS Windows (recorded after
  `queue.submit`; 7 samples in the smoke run). GPUI Phase A0 marked it
  unavailable — a real, measured architecture difference to quantify in
  the benchmark.
- `edit_applied` is a host-side proxy (recorded when the edit is
  forwarded to the guest over svc; the guest applies it in the same
  tick). GPUI records the mutation directly. Documented, not faked.
- Boot cost: first `layout_begin` at ~581 ms (rquickjs eval + pak feed +
  first guest frame) on the Windows headless run — the dominant startup
  term, to be compared against GPUI startup in the benchmark.
- Settled idle ticks: layout_begin→layout_end ~30-60 µs per tick on the
  Windows headless run (llvmpipe; real GPU numbers will differ — WSLg
  used llvmpipe software rendering).

## 6. Workloads

```text
corpus:   workloads/corpus/10k.txt (10 KB), 100k.txt (100 KB), 1m.txt (1 MB)
          fixed-seed ASCII prose, byte-identical on every machine
trace:    framework-independent traces — Phase B (schema drafted in README)
run count: Phase B (warmup + 5 measured runs per instruction)
```

## 7. Benchmark environment

```text
Windows:  Windows 11 (win11_dt), 100% scale
CPU/GPU:  AMD Radeon(TM) Graphics (integrated) — real GPU on Windows;
          llvmpipe (software) under WSLg
RAM/Display: see Windows host (recorded in Phase B report)
Markit SHA / PocketJS SHA: see Git state
Build:    release, cargo-xwin x86_64-pc-windows-msvc
```

## 8. Deferred issues (observed, not scanned)

```text
Windows CJK glyph rendering      DEFERRED — PocketJS main has no CJK
                                  system-font discovery; seed CJK lines
                                  render tofu. Blocks A1: NO (ASCII corpus)
clipboard (Ctrl+C/V)              DEFERRED — macOS-first on main
IME composition                   DEFERRED — protocol reserved; needs
                                  manual Microsoft Pinyin verification
transparent window                DEFERRED — Markit uses opaque; Windows
                                  alpha-mode degradation is a known
                                  PocketJS gap (not triggered here)
```

## 9. First-round benchmark (Windows, windowed, both MVPs)

Environment: Windows 11 Pro 10.0.26200, AMD Ryzen 7 5800H, AMD Radeon
(integrated), 32 GB RAM. Release builds. Warmup run + 5 measured runs per
corpus per MVP (36 runs total). Workload per run: 100 single-character
inserts (one per frame) + one backspace + one scroll, 1000x700 window,
Consolas 18 px / 28 px line height.

Results (median of run medians, µs; p50/p99):

| Metric | corpus | GPUI | PocketJS |
| --- | --- | ---: | ---: |
| input->edit | 10 KB | 11.8 / 107.7 | 0.2 / 0.9 |
| input->edit | 100 KB | 71.2 / 984.9 | 0.1 / 0.5 |
| input->edit | 1 MB | 1080.2 / 12535.3 | 0.1 / 1.6 |
| edit->layout | 10 KB | 7.3 / 29.2 | 8817.2 / 11419.3 |
| edit->layout | 100 KB | 14.0 / 63.9 | 20791.8 / 24523.5 |
| edit->layout | 1 MB | 23.5 / 98.6 | 142007.0 / 155837.8 |
| render | 10 KB | 394.2 / 928.8 | 357.8 / 477.6 |
| render | 100 KB | 3603.6 / 4640.6 | 339.8 / 424.9 |
| render | 1 MB | 48508.6 / 65617.0 | 360.8 / 549.8 |
| frames rendered / ticks | all | 106 / 106 | 13-47 / 541 |

Raw per-run logs: `results/raw/`; full table: `results/summary/`.

### Scaling

```text
10 KB -> 100 KB -> 1 MB (10x each step)

GPUI edit->layout:   7.3 -> 14.0 -> 23.5  µs   (near-flat; visible-line
                                                layout only)
PocketJS edit->layout: 8.8 ms -> 20.8 ms -> 142 ms   (grows with doc;
                                                ~2.4x per 10x doc at
                                                first step, ~6.8x at
                                                second — superlinear-ish
                                                per-edit DrawList work)
GPUI render:         0.39 -> 3.6 -> 48.5 ms   (grows with doc; renders
                                                every frame)
PocketJS render:     0.36 -> 0.34 -> 0.36 ms   (flat; demand rendering
                                                renders 13-47 of 541
                                                ticks)
```

### Work amplification — one character edit

```text
GPUI one-char edit (1 MB doc):
  input handling (UTF-16 offset math + line-starts rebuild, O(n)):
      ~1.1 ms p50
  layout (shape only visible lines, ~25):      ~1 µs
  render (full frame paint):                   ~49 ms p50 — the cost
      scales with document size despite visible-only shaping; root cause
      is a Phase A2 question (text_system/atlas? paint walk?)
  per-edit total:                              ~50 ms on a 1 MB doc

PocketJS one-char edit (1 MB doc):
  input forwarding (svc, host side):           ~0.2 µs
  guest turn (JS edit + solid update + DrawList rebuild):
      ~142 ms p50 — the DrawList/UI work scales with document size;
      root cause is a Phase A2 question (full UI-tree walk? measure FFI?
      atlas/hash?)
  render (words pass):                         ~0.36 ms (flat)
  per-edit total:                              ~142 ms on a 1 MB doc
```

### Tail latency

```text
PocketJS edit->layout p99: 11.4 ms (10 KB) / 24.5 ms (100 KB) / 156 ms (1 MB)
GPUI     edit->layout p99: 29.2 / 63.9 / 98.6 µs
GPUI     input->edit p99:  108 µs / 985 µs / 12.5 ms (1 MB)
```

### Architecture findings (first round, hypotheses not yet root-caused)

1. PocketJS's dominant cost is the per-edit guest turn (JS update +
   DrawList regeneration), which grows with document size (8.8 ms ->
   142 ms). GPUI's layout stays flat (~1 µs) because it shapes visible
   lines only.
2. GPUI's dominant costs are input handling (grows with doc, 1.1 ms at
   1 MB — line-starts rebuild + UTF-16 conversion) and full-frame render
   (grows with doc, 48.5 ms at 1 MB, every frame). PocketJS renders
   rarely (demand rendering) and flat.
3. Per edit on a 1 MB doc: PocketJS does ~2.9x the work of GPUI
   (142 ms vs ~50 ms) — but the split is opposite: PocketJS pays in the
   guest turn (JS/layout), GPUI pays in input+render (native paint).
4. PocketJS p99 edit latency is 3-4 orders of magnitude worse than GPUI's
   on small docs (8.8 ms vs 7 µs); GPUI's p99 input handling degrades
   with doc size (12.5 ms at 1 MB).
5. Baseline memory: not yet measured (working set sampling is a Phase A2
   item).
6. Local invalidation: GPUI already does visible-line-only shaping;
   PocketJS's DrawList rebuild is the obvious invalidation target.
7. `frame_submit`: observable on PocketJS (windowed), unavailable on
   GPUI — a genuine measurement asymmetry, not a performance claim.

### Limitations

- edit->layout on PocketJS is a host-side proxy (edit forwarded -> layout
  end); the guest's mutation itself is inside that span.
- GPUI's smoke runs 106 frames (fixed steps incl. IME/select-all); the
  PocketJS run is 541 ticks with 101 scripted edits — same workload shape
  (100 chars + backspace + scroll), not identical event counts.
- Windowed GPUI renders every frame (no demand rendering); this is the
  framework's behavior, not a harness artifact.
- GPU is an integrated AMD iGPU shared with the display; absolute numbers
  are machine-specific, the scaling trends are the evidence.
- Memory (working set) and startup-to-first-frame were not captured in
  this first round; startup boot cost observed separately: ~581 ms to
  first layout on PocketJS (rquickjs eval + pak feed + first frame).
- Power mode / refresh rate were not controlled beyond default (recorded
  for Phase B).

## 10. Verdict

**READY_FOR_PHASE_A2.** The Phase A1 completion criteria are met:
PocketJS thin MVP (Windows launch, text render, typing, delete/backspace,
scroll, resize) verified PASS on Windows native; same corpus, same trace
schema, same machine, release builds; first-round benchmark completed on
10 KB / 100 KB / 1 MB. Next phase must NOT auto-optimize: propose Top-3
confirmed bottlenecks and Top-3 experiments for the human to choose.
Candidate bottleneck leads from this round: (1) PocketJS per-edit
DrawList rebuild cost vs doc size; (2) GPUI full-frame render cost vs doc
size; (3) GPUI input handling (line-starts rebuild) cost vs doc size.

## 11. Git state

```text
Markit commits:            chore(pocketjs) submodule; feat(pocketjs) MVP;
                           bench(corpus); docs(a1); feat(gpui) --file +
                           typing smoke; bench + report (this commit)
PocketJS commits:          NONE on feat/windows-mvp (stock upstream main)

Markit working tree:       clean
PocketJS working tree:     clean (submodule)

Push performed: NO
PR created: NO
```
