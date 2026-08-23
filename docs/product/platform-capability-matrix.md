# Markit — Platform Capability Matrix

Status: live document. Values are evidence-based only (PASS/FAIL/PARTIAL/
NOT TESTED/DEFERRED — never guessed). "PASS" requires a real run on the
platform; WSLg runs are labeled as such and do not certify Linux desktop.

The matrix tracks the **direct-GPUI product path** (ADR-008). The
A0–A4 rows below come from the GPUI Windows feasibility prototype
(`mvp/gpui`, gpui 0.2.2) and the parity MVPs; they are **prototype
evidence** — product acceptance is re-validated on the selected GPUI
baseline (roadmap G0). Historical PocketJS-era rows are preserved in
`docs/research/`.

## G0 product baseline (zed rev eb8e1c8, frozen 2026-08-22, re-validated 2026-08-23)

Evidence source: `apps/markit --g0-probe --smoke` on the Windows desktop
session (probe + external host sampling); details and raw numbers in
`docs/product/g0-gpui-baseline.md` §5, `results/summary/g0/`, and the
curated artifacts under `results/evidence/g0/`.

| Capability | Windows (product baseline) |
| ---------- | -------------------------- |
| Cross-build (WSL → x86_64-pc-windows-msvc) | PASS for `cargo check`/clippy; **release cross-build unsupported under the current Linux-host path** (gpui_windows compiles HLSL via host-Windows-only fxc) — release builds are native on the Windows host |
| Release build | PASS (native host, 6m02s + 7.3s incremental re-validation, zero warnings, rustc 1.96.0; receipts in `results/evidence/g0/windows-build-receipt.txt`) |
| Launch / show window | PASS (bounds 720x480, GPU AMD Radeon iGPU, D3D 11.1) |
| Resize + relayout | PASS (720x480→900x600, 1 draw) |
| HiDPI scale factor | PASS @ 1.0 (2560x1440 100 %) and @ 1.25 (Settings 125 %: `scale_factor=1.25`, DPI-aware physical capture 918×647 px, sharp text) |
| Latin rendering | PASS (screenshot-verified) |
| CJK rendering (fallback) | PASS (real glyphs, Microsoft YaHei UI via DirectWrite) |
| Emoji rendering (fallback) | PASS (color, screenshot-verified) |
| Keyboard actions | PASS (F1/F3 via synthesized input after click-to-focus) |
| Mouse events | PASS (click; drag/wheel not covered) |
| IME composition pipeline | PASS (real Microsoft Pinyin: composition start/update with marked range covering the whole composition, SPACE commit replacing the marked span, ESC cancel, candidate `bounds_for_range` docking — `results/evidence/g0/pinyin-ime.log`; product-level IME UX → P1-A) |
| Clipboard text write/read | PASS (both directions + external Get-Clipboard match) |
| Demand redraw | PASS (notify→draws track 1:1 in the canonical run; 0 draws at rest) |
| Idle behavior (no draw/present) | PASS (2 s passive: 0 draws, 0 delivered `on_next_frame` callbacks; external ~0–1 % CPU, 36–38 MB WS; on_next_frame armed: ~60 callbacks/s ≈ every vsync, ~0 draws) |
| Startup / first frame | PASS (312.2 ms process→first paint canonical; 309.6–314.5 ms across runs, incl. device+font init) |
| Memory sanity | PASS (36–38 MB WS idle) |

## Matrix (prototype evidence, crates.io gpui 0.2.2)

| Capability | Windows (prototype) | Linux (WSLg) | Linux (real desktop) | macOS |
| ---------- | ------------------- | ------------ | -------------------- | ----- |
| Build | PASS (GPUI MVP, cross-build from WSL) | PASS | NOT TESTED | NOT TESTED |
| Launch | PASS | PASS (WSLg windowed) | NOT TESTED | NOT TESTED |
| GPU render / presentation | PASS (GPUI Windows backend: DirectX 11 + DirectComposition) | PASS (WSLg, GPUI) | NOT TESTED | NOT TESTED |
| Resize + relayout | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| HiDPI / density | PASS (scale factor 1.0 verified; >100% DPI monitor not yet validated) | PASS (density 1) | NOT TESTED | NOT TESTED |
| Mouse / pointer | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| Keyboard | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| IME | NOT TESTED (GPUI IMM32 path exercised in prototype; product-baseline result now in the G0 table above: PASS) | NOT TESTED | NOT TESTED | NOT TESTED |
| CJK fonts | PARTIAL (DirectWrite fallback verified in prototype; product-baseline result now in the G0 table above: PASS) | NOT TESTED | NOT TESTED | NOT TESTED |
| Clipboard | NOT TESTED (prototype scope; product-baseline result now in the G0 table above: PASS) | NOT TESTED | NOT TESTED | NOT TESTED |
| Open dialog | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| Save dialog | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| Drag/drop | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| Shortcuts (Ctrl/Cmd) | PASS (Ctrl+Q/A in prototype; copy/paste pending clipboard) | PARTIAL (Ctrl works) | NOT TESTED | NOT TESTED |
| Window restore | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| File association | NOT TESTED (packaging P1+) | NOT TESTED | NOT TESTED | NOT TESTED |

## Per-platform notes

### Windows (implementation order P1)

- Prototype evidence (A0, `mvp/gpui`): windowed GPUI (Win32 + DirectX 11
  + DirectComposition), keyboard, scroll,
  resize, IME pipeline, demand rendering, headless determinism all PASS.
- Substrate capabilities closed by G0 on the product pin (2026-08-22/23):
  window/render/resize, HiDPI 100 %+125 %, system fonts incl. CJK/emoji
  fallback, clipboard text, IME pipeline (real Pinyin start/update/commit/
  cancel), demand redraw, idle behavior, startup, memory — see the G0
  table above.
- Remaining product gaps to close (Tier-0): native file dialogs, file
  association, Ctrl shortcuts beyond Q/A, drag/drop, window restore, and
  product-level IME UX (candidate presentation, arrow-key navigation —
  P1-A scope, substrate already validated).
- G0 re-validation on the product pin (2026-08-22): see the G0 table
  above and `docs/product/g0-gpui-baseline.md` §5.
- Transparent window: DEFERRED — not a product requirement.

### Linux (implementation order P5)

- WSLg runs prove the GPUI host pipeline but are NOT the product runtime.
  Real desktop validation required: Wayland (primary) with X11 fallback,
  fontconfig, IBus/Fcitx IME, xdg portals/file dialogs, XDG
  config/data/cache dirs.
- Packaging: portable binary + desktop entry; AppImage later.

### macOS (implementation order P5)

- Not tested in this environment; every row above must be validated on
  real hardware through GPUI: Metal, CoreText fonts, IME, clipboard,
  Cmd shortcuts, file dialogs, menu bar, window behavior, Retina scaling,
  .app bundle + codesign/notarization.

## Update rule

A cell moves to PASS only after a real run on that platform with the
evidence recorded (log/screenshot + date + machine + GPUI revision).
WSLg results are labeled; they certify the host pipeline, not the Linux
desktop runtime.
