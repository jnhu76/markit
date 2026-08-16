# ADR-002 — Core vs platform boundary

Status: accepted (A4).

## Observed problem

Desktop editor capabilities (clipboard, IME, fonts, file dialogs,
shortcuts, paths) differ per OS. Without an explicit boundary, platform
branches leak into the core and every platform change touches editor
logic.

## Evidence

A1–A4: every host capability that entered via the svc channel (input,
scroll, resize, caret rect) stayed testable headlessly and identically
across platforms; the two host-only features (frame submit, ime_cursor_area)
needed no core change. The A2/A3 fixes were all core-side; the host never
needed a patch.

## Decision

- Core (`markit-core`, framework-free TS) names capabilities, never OSes:
  `ClipboardProvider`, `FontProvider`, `ImeProvider`, `FileDialogProvider`,
  `ShortcutPolicy`, `PlatformPaths`.
- The PocketJS host is the socket: svc messages carry platform events;
  adapters implement the providers per platform (windows/linux/macos).
- No `if (platform === ...)` in core code; real three-platform
  differences are the only things abstracted (no abstraction for its own
  sake).

## Trade-offs

- Thin adapter layer per platform (three small implementations) vs core
  complexity; capability-driven interfaces keep the boundary honest and
  the core portable.
