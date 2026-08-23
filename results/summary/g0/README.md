# G0 — GPUI baseline Windows real-host evidence (summary)

Committed summary of the G0 smoke matrix. Two layers of committed evidence:

- this file: verbatim `G0 …` lines + external sampling that
  `docs/product/g0-gpui-baseline.md` §5 references;
- `results/evidence/g0/`: curated key artifacts (canonical trace, build
  receipts, HiDPI + real-IME traces, screenshots, host summary) so the
  frozen-baseline PASS claims stay re-checkable when the pin ever changes.

Full raw captures stay local under `results/raw/g0/` (gitignored).

## Run metadata

- canonical run: 2026-08-23 (UTC timestamps in trace), display scale 100 %,
  **review-fixed probe binary** (corrected IME input-handler semantics;
  `next_frame_callbacks` counter naming)
- earlier run: 2026-08-22 (scale 100 %, first full matrix — superseded
  counter naming `frame_requests`; numbers quoted below where still cited)
- host: `HU` — AMD Ryzen 7 5800H, 29 GB RAM, AMD Radeon(TM) Graphics
  (integrated), Windows 11 Pro 10.0.26200, display 2560x1440 @ 60 Hz
- scale coverage: 100 % (both matrix runs) + 125 % (HiDPI validation,
  2026-08-23: `scale_factor=1.25`, DPI-aware physical screenshot; scale set
  via Settings and restored afterwards)
- build: native `cargo build --release -p markit --features g0-probe`
  ON THE WINDOWS HOST, 6m02s full + 7.3 s incremental after the review
  fixes, zero warnings (receipts: `results/evidence/g0/windows-build-receipt.txt`).
  Release cross-builds from WSL are unsupported under the current
  Linux-host cross path at this pinned revision (`gpui_windows` compiles
  HLSL shaders with `fxc.exe` only when the build host is Windows; the
  `shaders_bytes.rs` include is release-only — baseline doc §5).
  `cargo check`/clippy cross-builds are unaffected.
- rustc: `rustc 1.96.0 (ac68faa20 2026-05-25)` (host stable; the pin also
  builds under WSL rustc 1.97.1 for check/clippy — Zed's own toolchain
  file pins 1.97.1)
- GPUI: zed rev `eb8e1c8b5502b7007465fbbc465f4a736fa39210` (v1.16.1),
  Apache-2.0, zero Markit patches
- runs behind this summary:
  1. `markit.exe --g0-probe --smoke` scripted matrix, auto-quit,
     8.33 s wall (canonical, 2026-08-23) + external `Get-Clipboard`
     verification
  2. `markit.exe --g0-probe --smoke` at display scale 125 % (HiDPI matrix
     observation; per-line evidence preserved via `hidpi-125.log`)
  3. `markit.exe --g0-probe` interactive + external `Get-Process` idle
     CPU/RSS sampling (12 × 500 ms after settle, at 125 %; the 2026-08-22
     run sampled at 100 %)
  4. real Microsoft Pinyin IME pass (interactive probe, layout HKL
     0x08040804, raw key injection: n-i-h-a-o + SPACE commit,
     z-h-o-n-g-w-e-n + ESC cancel) — `results/evidence/g0/pinyin-ime.log`
  5. window screen captures at 100 % and 125 % (the 125 % capture is
     DPI-aware, physical resolution)

## Matrix (verbatim probe lines, canonical run 2026-08-23, scale 100 %)

```
[04:46:06.333279Z] G0 probe start smoke=true pid=6372
[04:46:06.628457Z] G0 scale_factor=1
[04:46:06.949324Z] G0 boot first_draw_ms=312.2
[04:46:08.954250Z] G0 idle_passive_2s draws=0 next_frame_callbacks=0 (draws/s=0.0 callbacks/s=0.0)
[04:46:10.957420Z] G0 idle_nextframe_2s draws=1 next_frame_callbacks=119 (draws/s=0.5 callbacks/s=59.5)
[04:46:12.184113Z] G0 demand_1p2s notifies=42 draws=42 next_frame_callbacks=1
[04:46:12.697855Z] G0 idle_sched idle_time_remaining=None ran_right_after_busy=0 ran_total=100 during_busy=0 (expect remaining=None during_busy=0)
[04:46:13.308057Z] G0 bg_priority first12=HHHHHHHHHHHH total=36 L=12 M=12 H=12
[04:46:13.369782Z] G0 timer 1 requested_ms=50 actual_ms=61.68
[04:46:13.432543Z] G0 timer 2 requested_ms=50 actual_ms=62.73
[04:46:13.495178Z] G0 timer 3 requested_ms=50 actual_ms=62.60
[04:46:13.556425Z] G0 timer 4 requested_ms=50 actual_ms=61.20
[04:46:13.959220Z] G0 cancel_by_drop ran=0 (expect 0)
[04:46:13.967867Z] G0 clipboard roundtrip=ok token=markit-g0-probe-7625989
[04:46:14.376390Z] G0 resize 720x480->900x600 draws_after=1
[04:46:14.376432Z] G0 MATRIX DONE
[04:46:14.577042Z] G0 dump draws=46 next_frame_callbacks=120 key_actions=1 mouse=0 ime_updates=0 ime_commits=0 first_draw_ms=312.2 input_latency[last_us=13016 max_us=13016 n=1] status=clipboard roundtrip OK (markit-g0-probe-7625989)
```

## Real Microsoft Pinyin IME evidence (2026-08-23, verbatim excerpts)

Full trace: `results/evidence/g0/pinyin-ime.log`.

```
G0 ime bounds_for_range requested_utf16=108..108 element=688x0   # candidate docking on composition start
G0 ime composition start
G0 ime composition update text="n" marked_utf16=Some(108..109) selected_utf16=109..109
G0 ime composition update text="n'ni'hao" marked_utf16=Some(108..116) selected_utf16=116..116
G0 ime commit text="拿你号" marked_replaced=144..152 tail="type here (IME ok): 拿你号"
G0 ime composition start
G0 ime composition update text="zhong'wen" marked_utf16=Some(111..120) selected_utf16=120..120
G0 ime composition cleared (cancel path)
G0 ime composition update text="" marked_utf16=None selected_utf16=111..111
G0 dump ... ime_updates=15 ime_commits=1 ...
```

→ the marked range covers the **whole** composing text and grows per key;
SPACE commits via `replace_text_in_range(None, result)` which **replaces**
the marked span (GCS_RESULTSTR semantics); ESC clears the composition and
the selection returns to the composition start; candidate-window docking
reaches the handler (`bounds_for_range`). Product-level IME UX (candidate
visuals, arrow-key navigation) is P1-A scope, not G0.

## External host evidence

Clipboard (canonical run, OS-level `Get-Clipboard` after auto-quit):

```
clipboard_token=markit-g0-probe-7625989      # matches probe roundtrip token
```

Idle CPU/RSS (2026-08-23 @125 %, `Get-Process` 12 × 500 ms while window
idle at rest; 2026-08-22 @100 % showed the same shape):

```
cpu_s=0.344 → 0.406 (advance ≈ 0.06 s over 6 s wall ≈ 1 % of one core)
ws_mb=38.2–38.3   threads=27–29
```

2026-08-22 @100 %: `cpu_s` frozen at 0.375 across the full 6 s window
(0.00 % measurable CPU at rest), WS ~36 MB, handles 371–374.

Synthesized input (2026-08-22 run, click-to-focus then synthesized keys;
field names from the superseded binary):

```
G0 dump draws=4 frame_requests=0 key_actions=1 mouse=1 ...   # F1 after click
G0 clipboard read="hello-from-host"                          # F3 read host-written text
```

→ click lands (`mouse=1`), bound keyboard action lands (`key_actions=1`),
clipboard host→probe read works. (The 2026-08-22 log's
`ime composition update text="a" …` lines came from the same MS Pinyin
layout with the OLD, incorrect marked-range semantics — superseded by the
real Pinyin trace above.)

## Notes

- `first_draw_ms` instrumentation fix (2026-08-22): the first run logged
  `0.0` because the process-entry `OnceLock` timestamp was lazily
  initialized inside the first paint. Fixed (eager init at probe entry) and
  re-run; corrected numbers: 314.5 ms (08-22), 312.2 ms (08-23 canonical,
  100 %), 309.6 ms (08-23, 125 %).
- Rendering text (Latin/CJK/emoji lines) verified by screen capture of the
  live window at both 100 % (`g0-window-100.png`) and 125 %
  (`g0-window-125.png`, DPI-aware physical capture, 918×647 px for a
  720×480 logical window): Latin "the quick brown fox 0123", Chinese
  `中文渲染测试：汉字与标点` as real glyphs (no tofu, sharp at both
  scales), emoji `🙂👍🧑‍💻🌍` in color. Presented on D3D11 with
  DirectWrite (Microsoft YaHei UI selected).
- `idle_passive_2s`: with NO pending `on_next_frame` callback the app
  observes zero callback deliveries and zero draws. The Windows vsync
  thread still wakes every window below the API (baseline doc §4); idle
  CPU sampling is the external proxy for that wake cost.
- `idle_nextframe_2s`: with a self-sustaining `on_next_frame`,
  callbacks deliver at ~60/s (≈ every 60 Hz vsync) while painting is
  skipped (1 draw in 2 s). The superseded 2026-08-22 binary observed
  ~12.5/s in its run; delivery rate is run-dependent and NOT a platform
  contract — the stable claim is "no draw/present work while idle".
- Background timers: 50 ms requested → 61.2–62.7 ms actual (canonical
  run; the 08-22 run observed 58.7–63.8 ms), consistent with default
  Windows timer granularity on this host.
- Mouse: single synthesized click verified (counter + focus path);
  drag/wheel/pointer-routing not covered by G0.
