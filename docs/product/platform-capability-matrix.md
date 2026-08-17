# Markit Desktop — Platform Capability Matrix

Status: live document. Values are evidence-based only (PASS/FAIL/PARTIAL/
NOT TESTED/DEFERRED — never guessed). "PASS" requires a real run on the
platform; WSLg runs are labeled as such and do not certify Linux desktop.

Evidence sources: A1–A4 MVP runs (`docs/phase-a1-*`, `docs/phase-a2-*`,
`docs/phase-a3-*`, `docs/phase-a4-final-research-closeout.md`) and E0
capability slices (`docs/phase-e0-desktop-enablement.md`).

## Matrix

| Capability | Windows | Linux (WSLg) | Linux (real desktop) | macOS |
| ---------- | ------- | ------------ | -------------------- | ----- |
| Build | PASS (cargo-xwin from WSL) | PASS | NOT TESTED | NOT TESTED |
| Launch | PASS | PASS (WSLg windowed) | NOT TESTED | NOT TESTED |
| GPU render (wgpu) | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| Resize + relayout | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| HiDPI / density | PASS (density 1; Retina untested) | PASS (density 1) | NOT TESTED | NOT TESTED |
| Mouse / pointer | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| Keyboard | PASS | PASS (WSLg) | NOT TESTED | NOT TESTED |
| IME | NOT TESTED (protocol reserved; manual Pinyin validation pending) | NOT TESTED | NOT TESTED | NOT TESTED |
| CJK fonts | PARTIAL (discovery+build PASS via cargo-xwin; msyh coverage verified; real-machine run pending — E0 slice 1) | PASS (WSLg: wqy-zenhei discovered, no-tofu screenshot; E0 slice 1) | NOT TESTED | NOT TESTED |
| Clipboard | NOT TESTED (protocol reserved; A1 gap) | NOT TESTED | NOT TESTED | NOT TESTED |
| Open dialog | NOT TESTED (no FileDialogProvider yet) | NOT TESTED | NOT TESTED | NOT TESTED |
| Save dialog | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| Drag/drop | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| Shortcuts (Ctrl/Cmd) | PASS (Ctrl+Q/A; copy/paste pending clipboard) | PARTIAL (Ctrl works) | NOT TESTED | NOT TESTED |
| Window restore | NOT TESTED | NOT TESTED | NOT TESTED | NOT TESTED |
| File association | NOT TESTED (packaging P1+) | NOT TESTED | NOT TESTED | NOT TESTED |

## Per-platform notes

### Windows (implementation order P1)

- Evidence base is the strongest: windowed wgpu, keyboard, scroll,
  resize, demand rendering, headless determinism all PASS (A1–A4).
- Product gaps to close (Tier-0): system font discovery (CJK/emoji) —
  E0 slice 1 landed upstream (jnhu76/pocketjs#5, Windows registry +
  Linux dir discovery); the real-machine Windows run is staged and
  pending, emoji/COLR remains open; clipboard (text), IME validation,
  native file dialogs, file association, Ctrl shortcuts beyond Q/A.
- Transparent window: DEFERRED — not a product requirement.
- Startup ~750 ms at 1M (A3): dominated by QuickJS eval + pak feed; a
  product target is to reduce this (bytecode/qjsc is a PocketJS-side
  direction; measure before optimizing).

### Linux (implementation order P2)

- WSLg runs prove the host pipeline but are NOT the product runtime.
  Real desktop validation required: Wayland (primary) with X11 fallback,
  fontconfig, IBus/Fcitx IME, xdg portals/file dialogs, XDG
  config/data/cache dirs.
- Packaging: portable binary + desktop entry; AppImage later.

### macOS (implementation order P3)

- PocketJS's macOS-first heritage (note-widget) is NOT product evidence;
  every row above must be validated on real hardware: Metal/wgpu,
  CoreText fonts, IME, clipboard, Cmd shortcuts, file dialogs, menu bar,
  window behavior, Retina scaling, .app bundle + codesign/notarization.

## Update rule

A cell moves to PASS only after a real run on that platform with the
evidence recorded (log/screenshot + date + machine). WSLg results are
labeled; they certify the host pipeline, not the Linux desktop runtime.
