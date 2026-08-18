# Markit — Platform Capability Matrix

Status: live document. Values are evidence-based only (PASS/FAIL/PARTIAL/
NOT TESTED/DEFERRED — never guessed). "PASS" requires a real run on the
platform; WSLg runs are labeled as such and do not certify Linux desktop.

The matrix tracks the **direct-GPUI product path** (ADR-008). The
A0–A4 rows below come from the GPUI Windows feasibility prototype
(`mvp/gpui`, gpui 0.2.2) and the parity MVPs; they are **prototype
evidence** — product acceptance must be re-validated on the selected GPUI
baseline (roadmap G0). Historical PocketJS-era rows are preserved in
`docs/research/`.

## Matrix

| Capability | Windows (prototype) | Linux (WSLg) | Linux (real desktop) | macOS |
| ---------- | ------------------- | ------------ | -------------------- | ----- |
| Build | PASS (GPUI MVP, cross-build from WSL) | PASS | NOT TESTED | NOT TESTED |
| Launch | PASS | PASS (WSLg windowed) | NOT TESTED | NOT TESTED |
| GPU render / presentation | PASS (GPUI Windows backend: DirectX 11 + DirectComposition) | PASS (WSLg, GPUI) | NOT TESTED | NOT TESTED |
| Resize + relayout | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| HiDPI / density | PASS (scale factor 1.0 verified; >100% DPI monitor not yet validated) | PASS (density 1) | NOT TESTED | NOT TESTED |
| Mouse / pointer | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| Keyboard | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| IME | NOT TESTED (GPUI IMM32 path exercised in prototype; manual Pinyin validation pending on the selected baseline) | NOT TESTED | NOT TESTED | NOT TESTED |
| CJK fonts | PARTIAL (DirectWrite fallback verified in prototype; product baseline validation pending) | NOT TESTED | NOT TESTED | NOT TESTED |
| Clipboard | NOT TESTED (GPUI Windows path to validate in Markit) | NOT TESTED | NOT TESTED | NOT TESTED |
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
- Product gaps to close (Tier-0) on the selected GPUI baseline: system
  font discovery (CJK/emoji), clipboard (text), IME validation, native
  file dialogs, file association, Ctrl shortcuts beyond Q/A.
- GPUI baseline selection (roadmap G0) must re-validate: build, release
  build, window, native text, CJK, IME, clipboard, resize, HiDPI,
  startup, RSS, basic latency.
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
