# Phase A1 — PocketJS Windows Desktop Host

> Historical research record.
>
> This document reflects the state of PocketJS desktop work at the time it
> was written. Markit later pivoted to direct GPUI as the product
> foundation — see [ADR-008](adr/ADR-008-direct-gpui-product-substrate.md).
> Measurements and findings are unedited.

Status: **READY_FOR_PHASE_B**

## 1. Executive Summary

PocketJS now has a genuine, verifiable Windows desktop host. The same flat
widget shell that runs the macOS note widget builds, boots, renders, takes
real OS input, edits, scrolls, resizes, and autosaves on `windows-msvc` —
with Windows-native clipboard, CJK glyph baking, primary-modifier chords,
and a registered `windows-widget` platform target. A thin plain-text editor
(`apps/editor`, no markdown) runs on that host with a deterministic `--smoke`
self-test that is green, and a new Windows CI workflow guards the whole
chain. Real Microsoft Pinyin IME composition is the one item that remains a
manual verification checklist (the IME pipeline itself is exercised
deterministically); everything else in the Phase A1 acceptance list is
implemented and evidenced.

## 2. Baseline

| Item | Value |
| --- | --- |
| PocketJS upstream SHA | `73b784109af36ae1eb3cc0c0193710e5723f34be` (2026-08-15) |
| Markit baseline (GPUI A0) | `e1863bc` (Phase A0 report: `docs/phase-a0-windows-feasibility.md`) |
| Toolchain | rustc 1.96.0 stable `x86_64-pc-windows-msvc`, bun 1.2.8, cargo |
| Windows | Windows 11 (win11_dt), AMD Radeon(TM) Graphics (integrated), display dpi 96 |
| Runtime crates | winit 0.30.13, wgpu 25, rquickjs 0.12, ab_glyph 0.2, windows 0.58 (new), memmap2 |
| Working clone | `C:\Users\fred1\source\pocketjs` (upstream + 12 local commits) |

## 3. Initial Windows gaps

From the Phase A0 gap analysis (CONDITIONAL GO), plus new findings from the
code-level audit:

| Gap | Owning module | Fix |
| --- | --- | --- |
| `set_identity("macos-widget", 3)` hardcoded | note-widget boot | platform-resolved identity (`windows-widget`/abi 4 on Windows) |
| Clipboard = pbcopy/pbpaste, other platforms log-only | note-widget host glue | new `pocket-clipboard` crate (macOS kept, Win32 `CF_UNICODETEXT` added) |
| CJK fallback fonts = hardcoded macOS paths | note-widget `cjk.rs` | per-substrate candidate list (`%WINDIR%\Fonts`, Microsoft YaHei first) |
| Chords keyed on `super_down()` (⌘Q/W/Z/C/X/V) | note-widget + `pocket3d::input` | `Input::primary_down()` (Cmd on macOS, Ctrl elsewhere) + Ctrl+A/Ctrl+Y |
| No `windows-*` target in `platforms.ts` | contracts | `windows-widget` target (hostAbi 4, same capabilities as macos-widget) |
| `bun run note` hardwired to macos-widget + broken Windows paths | tools/note.ts, tools/widget.ts, tools/test.ts, framework/compiler/jsx-plugin.ts, tests | `--target`/`--app`/`--build-only` flags; `fileURLToPath` everywhere `URL.pathname` produced `/C:/…`; separator-agnostic test assertions |
| No Windows CI | .github/workflows | `windows.yml` gate (build + tests + smoke) |
| Windowed boot aborts when the surface lacks alpha (Vulkan) | pocket-widget shell, pocket3d app | `pick_alpha_mode` degrades to Opaque with a warning |
| No `frame_submit` observation point | pocket-widget shell | default no-op `FlatWidget::frame_submitted()` called after `present()` |

Pre-existing upstream issues found but NOT caused by this work (documented,
not fixed here): bun 1.2.8 crashes (`panic: unreachable`) on
`tests/pocket-package.test.ts` — reproduced on a pristine upstream clone;
the full bun suite contains macOS-only test failures on Windows (iPhone 2G,
iOS, npm-artifact tests).

## 4. Architecture changes

- **Platform contract first**: `windows-widget` was registered in
  `contracts/spec/platforms.ts` before any host code changed, so identity
  asserts, build-plan resolution and admission all keyed off the existing
  registry machinery. Ids stay labels; `platform: "windows"` is the
  queryable field. Hosts declare the profile matching their substrate.
- **Platform services behind the existing abstraction points**: clipboard is
  a crate with cfg backends (macOS behavior byte-identical — still
  pbcopy/pbpaste); the CJK probe keeps its candidate-list + glyph-coverage
  architecture and only the list is per-substrate; the primary modifier is
  one method on the existing `Input` vocabulary. No `if windows` hacks in
  app or framework code; no new platform framework was invented.
- **The shell stayed portable**: winit 0.30's `ApplicationHandler` loop was
  already substrate-neutral (IME cursor area, scale factor, occlusion,
  demand rendering). The only shell change is the `frame_submitted` hook and
  the alpha-mode degradation.

## 5. Windows host

`engine/pocket3d/examples/note-widget` is the stock host on both macOS and
Windows:

- native window (winit), wgpu surface (DX12/Vulkan), demand rendering
  (FNV-1a DrawList hash → render only on dirt)
- identity: `windows-widget`/4 on Windows, `macos-widget`/3 elsewhere
- verified on this machine: headless proof run (130 frames, autosave
  round-trip, 18 rendered text bands pixel-verified), windowed run
  (1000×700 @ dpi 96; PrintWindow captures show the seed document; resize to
  900×640 re-renders), release build in ~36 s incremental
- windowed trace observes all seven stages:
  `trace: events=7678 input=1796 edit=3/3 layout=1801/1801 render=758/758 frame=758`

## 6. Clipboard

`engine/crates/pocket-clipboard` — `copy(&str)` / `paste() -> Option<String>`:

- macOS: pbcopy/pbpaste (unchanged behavior)
- Windows: Win32 clipboard, `CF_UNICODETEXT` (UTF-16), GMEM_MOVEABLE
  allocation; `GlobalUnlock`'s inverted BOOL semantics handled (windows-rs
  wraps it as `Result`, the success case returns FALSE)
- other platforms: explicit `Unsupported` result, no silent log
- tests (serialized through a mutex — the clipboard is process-global and
  the harness runs tests in parallel): ASCII round-trip, 中文 round-trip,
  multiline (incl. CJK line), empty clipboard → `None` — all pass

## 7. Shortcut semantics

`pocket3d::input::Input` gained `control_down()` and `primary_down()` —
Command on macOS, Control everywhere else. The host's quit/undo/copy/cut/
paste chords and its char-suppression under chords use `primary_down()`;
Ctrl+A (SelectAll) and Ctrl+Y (Redo) joined the chord set. The guest never
sees a platform modifier — chords arrive as named keys (`Copy`, `Undo`,
`SelectAll`, …) over the svc protocol. macOS behavior is unchanged.

## 8. Font/CJK

`cjk.rs` keeps the runtime glyph-baking probe (ab_glyph over an mmap'd
system font, coverage check on `中`, append to the FONT ATLAS v3 blob) but
the candidate list is per-substrate:

- Windows: `C:\Windows\Fonts\msyh.ttc`, `msyhbd.ttc`, `simsun.ttc`,
  `simhei.ttf`, `Deng.ttf`, `Dengb.ttf`, `msyhl.ttc`
- macOS: the original system collection

Verified on Windows: `msyh.ttc#0` (Microsoft YaHei) wins the probe; typing
8 unseen hanzi extended 7 atlas slots at runtime; pixel analysis of the
headless screenshot shows irregular glyph clusters (real strokes), not tofu
boxes. ASCII/Latin glyphs come from the baked Inter atlas.

## 9. Input/focus

The winit → `Input` → svc path was already substrate-neutral; verified
end-to-end on Windows with real OS-level input:

- click (WM_LBUTTONDOWN/UP) → winit → svc mouse → caret placement
- keys (WM_KEYDOWN + WM_CHAR + WM_KEYUP triples) → winit `KeyEvent` → svc
  `ch` → document edit → debounced autosave → file on disk
  (the saved document contains the typed text — verified)
- wheel → svc scroll; resize → live viewport relayout; scale factor from
  winit (dpi 96 here; the code path is identical at other scales)
- focus: winit `Focused(true)` observed; window must be foreground for real
  keys (standard Windows behavior; the automation sets topmost + foreground)

Automation note: winit maps `WM_CHAR` onto keys it has tracked down, so
bare `WM_CHAR` posts and `VK_PACKET`/SendInput chars are dropped (winit's
fake-key detection); the working injection is the KEYDOWN/CHAR/KEYUP triple.
This is an automation detail, not a product gap — real typing produces the
same triple through the keyboard layout.

## 10. IME

Pipeline (winit's native path — TSF on Windows):

- shell: `Window::set_ime_allowed(true)`; `ime_cursor_area` polled each tick
  → `Window::set_ime_cursor_area` docks the candidate window at the caret
  rect the guest reports (`{t:"caret"}` intent, sent on caret move)
- events: `Ime::Preedit`/`Commit`/`Enabled`/`Disabled` → `Input::ime_events`
  → host converts byte cursor → char index → svc `{t:"ime", s, c}`; commits
  arrive as `{t:"ch"}`; the guest splices preedit at the caret with an
  underline and blocks navigation while composing
- deterministic verification: the smoke drives preedit → commit →
  backspace through the same protocol; the windowed candidate-window
  docking code path is exercised on every caret move
- **manual checklist (Microsoft Pinyin on Windows 11)** — not automatable
  without the real input method:
  1. launch `note-widget.exe --app editor-main --width 1000 --height 700`
  2. switch to Microsoft Pinyin; click into the text; type `ni`
  3. candidate window appears next to the caret; composition underline shows
  4. select 你 → commit; text inserts; Ctrl+Backspace inside composition
  5. Esc cancels composition; focus loss and window resize during
     composition behave

## 11. Windows target/contracts

`contracts/spec/platforms.ts` registers `windows-widget`:

```ts
"windows-widget": {
  hostAbi: 4,
  platform: "windows",
  form: "widget",
  display: { physicalViewport: [840, 1120], logicalViewports: [[420, 560]],
             dynamicViewport: { min: [240, 180], max: [4096, 4096] },
             presentations: ["native"], rasterDensity: 2 },
  capabilities: ["input.buttons", "input.ime", "input.pointer", "input.text",
                 "host.clipboard", "display.viewport.live",
                 "text.glyphs.baked", "text.glyphs.runtime"],
}
```

Only capabilities the stock host implements and tests are advertised. The
registry test pins the target list and the new profile; build plans resolve
against it (`bun tools/note.ts --target=windows-widget`); the host asserts
identity (`__host`/`__hostAbi`) at eval, so a bundle built for the wrong
target refuses to boot.

## 12. CI

`.github/workflows/windows.yml` (windows-latest, 45 min cap):

1. checkout with `core.autocrlf false` (byte-exact fixtures)
2. bun + rustup (stable, wasm32 target for the sim-boot tests)
3. cargo cache
4. `bun install`
5. `bun test tests/platform-contracts.test.ts tests/widget-args.test.ts tests/note.test.ts` (79 tests)
6. `cargo test -p pocket-clipboard -p pocket3d -p pocket-widget -p pocket-ui-surface -p pocket-ui-wgpu -p pocket-mod`
7. `bun tools/note.ts --target=windows-widget --app=editor --build-only` (release bundle + host)
8. `note-widget.exe --app editor-main --smoke` (deterministic, exit code gated)

Excluded with evidence: `tests/pocket-package.test.ts` (bun 1.2.8 crashes
before assertions — reproduced on pristine upstream); the macOS-only toolchain
suites. All steps were dry-run locally in order and are green.

## 13. Thin editor MVP

`apps/editor` — plain-text editing surface (no markdown), 1000×700 logical,
16 px font / 28 px line height, seed document of 10 lines with 5 Chinese
lines (ASCII+CJK mixed):

- soft wrap, caret, click-to-focus, drag selection, undo/redo (coalescing),
  Backspace/Delete/Enter/Tab/Home/End/arrows/PageUp/PageDown, SelectAll,
  Copy/Cut, paste, wheel scroll, live resize, IME composition with
  caret-docked candidates, debounced autosave
- runs on the stock host: `note-widget.exe --app editor-main`
  (`bun run note --app=editor --target=windows-widget`)

`--smoke` (headless, deterministic): a fixed script drives click-to-focus,
ASCII + CJK typing, IME preedit+commit, backspace, enter, home/end,
select-all, paste, scroll and a live viewport resize through the same svc
protocol as real input, then asserts the saved document and prints the
instrumentation summary:

```
smoke: ok   typed + pasted text in the saved document
smoke: ok   select-all replaced the seed document
smoke: ok   document is exactly the edited content
smoke: trace: events=275 input=13 edit=10/10 layout=120/120 render=1/1 frame=0 unavailable=[frame_submit]
smoke: PASS
```

`frame_submit` is honestly `unavailable` under headless (no swapchain
present) and observed in the windowed shell via the hook.

## 14. GPUI parity matrix

| Capability | GPUI (A0) | PocketJS (A1) |
| --- | --- | --- |
| Native window | ✓ Win32/DX11 | ✓ winit/DX12-Vulkan |
| Text | ✓ DirectWrite | ✓ baked atlases + runtime baking |
| CJK | ✓ | ✓ msyh.ttc fallback (pixel-verified, no tofu) |
| Keyboard | ✓ WM_CHAR | ✓ winit KeyEvent (real-input verified) |
| Mouse | ✓ | ✓ (real-input verified) |
| Cursor | ✓ | ✓ |
| Editing | ✓ | ✓ (insert/delete/enter/select-all verified) |
| Selection | ✓ | ✓ (drag + shift-extend + select-all) |
| Scroll | ✓ | ✓ |
| Resize | ✓ | ✓ (pixel-verified re-render) |
| HiDPI | ✓ scale factor | ✓ winit scale (dpi 96 verified; code path identical at other scales) |
| Clipboard | ✓ | ✓ Win32 CF_UNICODETEXT + pbcopy/pbpaste (4 tests) |
| IME | ✓ IMM32 pipeline (manual Pinyin pending) | ✓ winit/TSF pipeline (manual Pinyin pending) |
| Trace hooks | 6/7 (frame_submit unavailable) | 7/7 windowed, 6/7 headless (frame_submit unavailable, honest) |
| Windows CI | none | ✓ windows.yml |

## 15. Known limitations

- Real Microsoft Pinyin composition: manual checklist (see §10) — the
  pipeline is verified deterministically but not against the real IME.
- `input_received` counts a tick as input when the cursor is merely over the
  window (cursor-presence semantics); a per-frame cursor-moved edge would
  tighten it. Documented, not gamed.
- HiDPI verified at dpi 96; higher-DPI displays use the identical winit
  scale path but were not pixel-verified on a 150%/200% display.
- Transparency: surfaces without alpha (Vulkan on this machine) degrade to
  Opaque with a warning — the window is square-cornered, not see-through.
- Scripted caret placement landed ~2 lines below the clicked line (a
  coordinate-path quirk in the automation/window interaction); input
  delivery itself is verified. To be investigated if Phase B caret parity
  demands it.
- bun 1.2.8 crashes on `tests/pocket-package.test.ts` (upstream bug,
  reproduced on pristine); the Windows CI excludes it with this evidence.
- The full bun suite has pre-existing macOS-only failures on Windows
  (iPhone 2G / iOS / npm-artifact suites); the Windows gate runs the
  Windows-relevant subset.

## 16. Benchmark readiness

**Can GPUI and PocketJS now consume the same workload? YES.**

The svc protocol already accepts the Phase B trace vocabulary: `insert_text`
→ `{t:"ch"}`, `delete_backward` → `{t:"key","k":"Backspace"}`, `move_cursor`
→ arrow/Home/End keys, `select` → SelectAll + mouse drag, `scroll` →
`{t:"scroll"}`, `resize` → live viewport, `paste` → `{t:"paste"}`. Both
hosts run the same corpus (10-line CJK seed), window size (1000×700 logical),
font size (16 px) and line height (28 px). The instrumentation contract
(event_id, input_received, edit_begin/end, layout_begin/end,
render_begin/end, frame_submit where observable) is implemented on both
sides. Phase B itself (corpus freeze, trace schema, ETW, statistics) is a
separate, explicitly excluded step.

## 17. Exact commands

```text
# build bundle + host (Windows)
bun tools/note.ts --target=windows-widget --app=editor --build-only
# or macOS default
bun tools/note.ts

# host tests
cargo test -p pocket-clipboard -p pocket3d -p pocket-widget \
  -p pocket-ui-surface -p pocket-ui-wgpu -p pocket-mod   (from engine/)

# bun tests (Windows-relevant subset)
bun test tests/platform-contracts.test.ts tests/widget-args.test.ts tests/note.test.ts

# deterministic smoke (exit 0 = pass)
engine/target/release/note-widget.exe --app editor-main --smoke

# headless proof (note app acceptance)
bun tools/note.ts --target=windows-widget --proof

# windowed run
engine/target/release/note-widget.exe --app editor-main --width 1000 --height 700

# headless scripted run with screenshot
engine/target/release/note-widget.exe --app editor-main \
  --file dist/x.md --screenshot dist/x.png --frames 70 \
  --click 350,15@10 --type "你好世界"@30 --auto-quit 2

# windowed automated verification (real OS input + resize + captures)
powershell -File dist/windowed-check.ps1
```

## 18. Commits

PocketJS (`73b7841` upstream + 12 commits, branch `main`):

| SHA | Purpose |
| --- | --- |
| `7d7fbb1` | feat(contracts): register windows-widget target profile |
| `3c02894` | fix(tools): resolve repo root for Windows checkouts (+ note.ts --target) |
| `2d4bf1b` | feat(clipboard): platform clipboard abstraction with a Win32 backend |
| `318544c` | refactor(clipboard): cfg-scope the macOS-only imports |
| `47a3804` | feat(input): primary shortcut modifier — Cmd on macOS, Ctrl elsewhere |
| `0e378dc` | feat(fonts): Windows CJK fallback font discovery |
| `917ec04` | feat(widget): frame_submitted hook after present |
| `aa0cecd` | feat(editor): PocketJS thin editor MVP + deterministic smoke |
| `5fd84ee` | fix(tools): Windows-safe root paths and path assertions |
| `726c77e` | ci: add Windows desktop checks |
| `b3f82dd` | fix(widget): degrade transparent windows to Opaque instead of failing |
| `614c41c` | docs(widget): document Windows as a supported widget substrate |

Markit: this report (`docs/phase-a1-pocketjs-windows.md`).
