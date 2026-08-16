# Markit Desktop — MVP v0.1 Scope

The first shippable product milestone: a single-document, L1-styled
Markdown editor on Windows (P1), then Linux (P2), then macOS (P3), built
on the PocketJS foundation decided in A4. Prerequisite: **PocketJS
Desktop Enablement** (roadmap E0) proves the consumed desktop
capabilities (window, CJK fonts, clipboard, IME, dialogs) on the target
platform first — this milestone consumes them, it does not re-implement
them Markit-side.

## In scope

```text
Platforms:          Windows (P1), Linux (P2), macOS (P3) — one shared core
Editing:            single document, UTF-8 (BOM detection if needed)
Files:              open / save / save-as, dirty state, atomic save
Markdown:           L1 styled editing (heading, paragraph, bold, emphasis,
                    inline code, link, blockquote, ul/ol list, fenced
                    code) with syntax visible
Editing primitives: caret, selection, scroll, undo/redo (transactions,
                    typing/delete coalescing, IME-commit grouping)
Text:               Latin + CJK + emoji fallback (system font discovery)
IME:                Chinese IME (composition model, candidate docking);
                    JA/KO architecture present, Chinese validated
Clipboard:          text copy/cut/paste
Shortcuts:          Copy/Paste/Undo/Redo/SelectAll/Save/Open/Find
                    (Ctrl on Win/Linux, Cmd on macOS via ShortcutPolicy)
Window:             resize, HiDPI, window-state restore (basic)
Stability:          large documents (1M+ measured flat), crash recovery
                    (minimal: periodic recovery snapshot + clean-shutdown
                    marker + startup recovery)
```

## Not in scope (v0.1)

```text
tabs / workspace / file tree   plugins
images / tables / math / Mermaid
cloud sync / collaboration / Git integration
PDF export / themes marketplace
rich HTML clipboard / images / custom MIME
transparent windows
legacy encodings (UTF-8 only)
full Typora-style syntax hiding (L2 is a later phase)
```

## Acceptance gates (per platform, in order)

1. Launch, render, resize, HiDPI — PASS on real hardware (no WSLg-only
   certification for Linux).
2. Open/save/save-as round-trip byte-identical for UTF-8 (incl. BOM),
   atomic save (no torn file on kill -9 / TerminateProcess).
3. L1 editing: type, backspace, enter, arrows, home/end, selection,
   scroll, undo/redo — smoke-verified byte-identical state sequences
   (the existing deterministic smoke pattern).
4. CJK: Chinese text renders (system font discovery + fallback), Chinese
   IME composes, commits and cancels correctly; composition never enters
   the undo stack as keystrokes.
5. Clipboard: copy/cut/paste round-trip with the OS.
6. Performance invariants: `docs/product/performance-invariants.md`
   battery passes (work-amplification checks; loose wall-clock
   guardrails).
7. 1M-document stability: edit latency flat vs 10K (within noise band),
   DrawList viewport-bound, no idle redraw.

## Exit criteria

- P1 (Windows): gates 1–7 on Windows, installer-free portable exe,
  crash recovery smoke.
- P2 (Linux): gates 1–7 on a real Linux desktop (Wayland primary, X11
  fallback), portable binary + desktop entry.
- P3 (macOS): gates 1–7 on real hardware, .app bundle (codesign/
  notarization deferred).
