# G0 — GPUI baseline Windows real-host evidence (summary)

Committed summary of the G0 smoke matrix. Raw logs (full probe traces,
external sampling files, build receipt) are archived locally under
`results/raw/g0/` (gitignored) and are NOT reproduced here; this file
quotes the verbatim `G0 …` lines that `docs/product/g0-gpui-baseline.md`
§5 references.

## Run metadata

- date: 2026-08-22 (UTC timestamps in trace; local 2026-08-23)
- host: `HU` — AMD Ryzen 7 5800H, 29 GB RAM, AMD Radeon(TM) Graphics
  (integrated), Windows 11 Pro 10.0.26100, display 2560x1440 @ scale 1
- build: native `cargo build --release -p markit --features g0-probe`
  ON THE WINDOWS HOST, 6m02s, zero warnings in receipt
  (cross-building release from WSL is impossible: `gpui_windows` compiles
  HLSL shaders with `fxc.exe` only when the build host is Windows — see
  baseline doc §5; `cargo check`/clippy cross-builds are unaffected)
- rustc: `rustc 1.96.0 (ac68faa20 2026-05-25)` (host stable; the pin also
  builds under WSL rustc 1.97.1 for check/clippy — Zed's own toolchain
  file pins 1.97.1)
- GPUI: zed rev `eb8e1c8b5502b7007465fbbc465f4a736fa39210` (v1.16.1),
  Apache-2.0, zero Markit patches
- runs:
  1. `markit.exe --g0-probe --smoke` (scripted matrix, auto-quit,
     8.98 s wall) + external `Get-Clipboard` verification
  2. `markit.exe --g0-probe` interactive + external `Get-Process`
     idle CPU/RSS sampling (12 × 500 ms after settle) — run 2a
  3. `markit.exe --g0-probe` interactive + synthesized input
     (AppActivate → click @ window center → SendKeys F1/F3/'a'/'b') — run 2b
  4. `markit.exe --g0-probe` interactive + window screen capture
     (CopyFromScreen of the window rect → `g0-window.png`) — run 2c

## Matrix (verbatim probe lines, run 1)

```
[22:45:52.713623Z] Using GPU: AMD Radeon(TM) Graphics
[22:45:52.747287Z] Created device with Direct3D 11.1 feature level.
[22:45:52.748968Z] Use Microsoft YaHei UI as UI font.
[22:45:52.969252Z] G0 bounds w=720 h=480
[22:45:52.969293Z] G0 scale_factor=1
[22:45:53.284370Z] G0 boot first_draw_ms=314.5
[22:45:55.285215Z] G0 idle_passive_2s draws=0 frame_requests=0 (draws/s=0.0 wakes/s=0.0)
[22:45:57.297280Z] G0 idle_nextframe_2s draws=1 frame_requests=25 (draws/s=0.5 wakes/s=12.5)
[22:45:58.526250Z] G0 demand_1p2s notifies=41 draws=15 frame_requests=1
[22:45:59.038376Z] G0 idle_sched idle_time_remaining=None ran_right_after_busy=0 ran_total=100 during_busy=0 (expect remaining=None during_busy=0)
[22:45:59.641496Z] G0 bg_priority first12=HHHHHHHHHHHH total=36 L=12 M=12 H=12
[22:45:59.700207Z] G0 timer 1 requested_ms=50 actual_ms=58.68
[22:45:59.761625Z] G0 timer 2 requested_ms=50 actual_ms=61.39
[22:45:59.825491Z] G0 timer 3 requested_ms=50 actual_ms=63.84
[22:45:59.887390Z] G0 timer 4 requested_ms=50 actual_ms=61.87
[22:46:00.295521Z] G0 cancel_by_drop ran=0 (expect 0)
[22:46:00.297833Z] G0 clipboard roundtrip=ok token=markit-g0-probe-7630633
[22:46:00.300227Z] G0 bounds w=900 h=600
[22:46:00.711283Z] G0 resize 720x480->900x600 draws_after=1
[22:46:00.711313Z] G0 MATRIX DONE
[22:46:00.915425Z] G0 dump draws=20 frame_requests=26 key_actions=1 mouse=0 ime_updates=0 ime_commits=0 first_draw_ms=314.5 input_latency[last_us=6292 max_us=6292 n=1]
```

## External host evidence

Clipboard (run 1, OS-level `Get-Clipboard` after auto-quit):

```
clipboard_token=markit-g0-probe-7630633      # matches probe roundtrip token
```

Idle CPU/RSS (run 2a, `Get-Process` 12 × 500 ms while window idle at
rest; first→last sample):

```
cpu_s=0.375 ws_mb=35.9 pm_mb=32   threads=27 handles=371
... (10 identical intermediate samples; cpu_s never advances)
cpu_s=0.375 ws_mb=36   pm_mb=32.1 threads=29 handles=374
```

→ **0.00 % CPU while idle** (TotalProcessorTime frozen at 0.375 s across
the full 6 s window; the process consumes no measurable CPU at rest),
working set ~36 MB, 27–29 threads.

Synthesized input (run 2b, click-to-focus then SendKeys; probe log):

```
G0 dump draws=4 frame_requests=0 key_actions=1 mouse=1 ...   # F1 after click
G0 clipboard read="hello-from-host"                          # F3 read host-written text
G0 ime composition start
G0 ime composition update text="a" marked=Some(1..1)
G0 ime composition update text="a'b" marked=Some(3..3)
```

→ click lands (`mouse=1`), bound keyboard action lands (`key_actions=1`),
clipboard host→probe read works, and plain synthesized text traverses the
IMM32 composition pipeline (start/update with UTF-16 marked ranges).

## Notes

- `first_draw_ms` instrumentation fix: the first run logged `0.0` because
  the process-entry `OnceLock` timestamp was lazily initialized inside
  the first paint. Fixed (eager init at probe entry) and re-run; 314.5 ms
  is the corrected, real number. The broken run is superseded.
- Rendering text (Latin/CJK/emoji lines in the window) verified by
  screen capture of the live window (`g0-window.png`, 736x519, visual
  inspection): Latin "the quick brown fox 0123", Chinese
  `中文渲染测试：汉字与标点` as real glyphs (no tofu), emoji
  `🙂👍🧑‍💻🌍` in color. Presented on D3D11 with DirectWrite
  (Microsoft YaHei UI selected).
- IME: composition start/update/marked-range exercised via the input
  pipeline; a true Pinyin IME composition+commit needs human IME input
  and remains PENDING for P0-03/manual validation on this baseline.
- Mouse: single synthesized click verified (counter + focus path);
  drag/wheel/pointer-routing not covered by G0.
- `idle_nextframe_2s`: with a self-sustaining `on_next_frame` callback,
  frame callbacks fire at ~12.5/s on this host while the window is
  clean, and painting is skipped (1 draw in 2 s) — consistent with the
  source-audit model (§4 of the baseline doc): wakeups below the API are
  platform behavior; above the API, no draw/present work occurs.
- Background timers: 50 ms requested → 58.7–63.8 ms actual (~+12 ms),
  consistent with default Windows timer granularity on this host.
