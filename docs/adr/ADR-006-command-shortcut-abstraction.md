# ADR-006 — Command/shortcut abstraction

Status: accepted (A4).

## Observed problem

Platform shortcut bindings differ (Ctrl on Windows/Linux, Cmd on macOS);
editor logic must not branch on the modifier.

## Evidence

A1–A4 host protocol already delivers named keys (`SelectAll`, `Copy`,
`Cut`, …) rather than raw chords; the editor core consumes commands.

## Decision

- Core consumes `Command::Undo` / `Redo` / `Copy` / `Paste` /
  `SelectAll` / `Save` / `Open` / `Find`, never raw Ctrl-vs-Cmd.
- `ShortcutPolicy` (a platform adapter) maps bindings per platform.
- New commands register in the command table; shortcuts bind outside the
  core.
