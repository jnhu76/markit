# Phase A1 — Markit PocketJS integration & thin editor MVP

Status: **A1_IN_PROGRESS** — PocketJS integration + thin editor MVP done
and verified on Windows (native); first-round GPUI/PocketJS benchmark is
the remaining Phase A1 item.

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

## 9. Remaining for Phase A1 completion

1. GPUI MVP: load corpus via `--file` (seed is currently built-in) so
   both MVPs run the same 10 KB / 100 KB / 1 MB documents.
2. First-round benchmark on the same Windows machine: startup, working
   set, idle CPU, input→layout/edit latency (p50/p95/p99), frame
   latency, long frames, per-corpus scaling.
3. Comparison table + architecture findings (work amplification:
   one-character edit cost per framework).

## 10. Git state

```text
Markit commits:            (see below)
PocketJS commits:          NONE on feat/windows-mvp (stock upstream main)

Markit working tree:       clean (after commit)
PocketJS working tree:     clean (submodule)

Push performed: NO
PR created: NO
```
