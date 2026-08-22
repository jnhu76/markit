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

## G0 product baseline (zed rev eb8e1c8, 2026-08-22)

Evidence source: `apps/markit --g0-probe --smoke` on the Windows desktop
session (probe + external host sampling); details and raw numbers in
`docs/product/g0-gpui-baseline.md` §5 and `results/summary/g0/`.

| Capability | Windows (product baseline) |
| ---------- | -------------------------- |
| Cross-build (WSL → x86_64-pc-windows-msvc) | PASS for `cargo check`/clippy; **release cross-build impossible** (gpui_windows compiles HLSL via host-Windows-only fxc) — release builds are native on the Windows host |
| Release build | PASS (native host, 6m02s, zero warnings, rustc 1.96.0) |
| Launch / show window | PASS (bounds 720x480, GPU AMD Radeon iGPU, D3D 11.1) |
| Resize + relayout | PASS (720x480→900x600, 1 draw) |
| HiDPI scale factor | PASS @ 1.0 (2560x1440 100 %); >100 % DPI PENDING (no such display) |
| Latin rendering | PASS (screenshot-verified) |
| CJK rendering (fallback) | PASS (real glyphs, Microsoft YaHei UI via DirectWrite) |
| Emoji rendering (fallback) | PASS (color, screenshot-verified) |
| Keyboard actions | PASS (F1/F3 via synthesized input after click-to-focus) |
| Mouse events | PASS (click; drag/wheel not covered) |
| IME composition pipeline | PARTIAL (start/update/UTF-16 marked ranges exercised; human Pinyin commit PENDING → P0-03) |
| Clipboard text write/read | PASS (both directions + external Get-Clipboard match) |
| Demand redraw | PASS (notify→draws track; 0 draw/0 request at rest) |
| Idle behavior (no draw/present) | PASS (2 s passive: 0 draws 0 requests; external 0 % CPU, ~36 MB WS; on_next_frame armed: ~12.5 callbacks/s, ~0 draws) |
| Startup / first frame | PASS (314.5 ms process→first paint, incl. device+font init) |
| Memory sanity | PASS (~36 MB WS idle) |

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
| IME | NOT TESTED (GPUI IMM32 path exercised in prototype; product-baseline result now in the G0 table above: PARTIAL) | NOT TESTED | NOT TESTED | NOT TESTED |
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
- Product gaps to close (Tier-0) on the G0-frozen baseline: system
  font discovery (CJK/emoji), clipboard (text), IME validation, native
  file dialogs, file association, Ctrl shortcuts beyond Q/A.
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
