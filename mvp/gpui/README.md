# mvp/gpui — Markit Phase A0 GPUI Windows feasibility prototype

Thin editable text surface proving GPUI 0.2.2's Windows baseline for the
Markit editor feasibility gate. Not an editor, not a benchmark — a capability
probe (see `../../docs/phase-a0-windows-feasibility.md`).

## Features verified

- Native window (Win32 + DirectX 11 + DirectComposition)
- Fixed-font text with Chinese via DirectWrite fallback
- Mouse: click-to-position-cursor, drag selection, wheel scroll
- Keyboard: insert, backspace/delete, arrows, home/end, enter, ctrl+a/c/v/x
- IME pipeline: composition/commit through gpui's input-handler contract
  (IMM32 on Windows; end-to-end composition needs a human to type)
- Resize + HiDPI (scale_factor / rem_size logged on every bounds change)
- Instrumentation skeleton: input_received / edit_applied / layout_begin /
  layout_end / render_begin / render_end (frame_submit unavailable, see report)

## Build & run

```bash
cargo build --release
./target/release/mvp-gpui.exe            # interactive
./target/release/mvp-gpui.exe --smoke    # deterministic self-test, dumps
                                         # editor state + trace, then exits
```

Controls: type to insert, arrows/backspace/delete/enter/home/end,
shift+arrows to select, ctrl+a/c/v/x, wheel to scroll, F1 dumps the
instrumentation ring buffer to stdout, ctrl+q quits.

## Layout

- `src/main.rs` — app bootstrap, key bindings, resize/HiDPI logging, `--smoke` driver
- `src/editor.rs` — ThinEditor (flat UTF-8 document + line index, cursor/selection,
  IME marked range, manual scroll) and its custom paint element
- `src/instrument.rs` — shared 7-stage timestamp contract (ring buffer, F1/quit dump)

## Known constraints (Phase A0 findings)

1. `cx.bind_keys` must run **before** `open_window`, or the keymap is empty.
2. `window.focus` is ignored while the window is inactive; a user click is the
   reliable focus path. The editor is focused on open (deferred), so typing
   works once the window is activated.
3. `frame_submit` is marked unavailable: `Window::on_next_frame` never fired on
   Windows in this spike; the pre-present moment is not observable from app code.
