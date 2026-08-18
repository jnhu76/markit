# ADR-002 — Core vs platform boundary

Status: accepted (A4); boundary restated for direct GPUI (2026-08-18).

## Observed problem

Desktop editor capabilities (clipboard, IME, fonts, file dialogs,
shortcuts, paths) differ per OS. Without an explicit boundary, platform
branches leak into the core and every platform change touches editor
logic.

## Evidence

A1–A4: every platform capability that entered through a well-defined
channel (input, scroll, resize, caret rect) stayed testable headlessly and
identically across platforms; the two host-only features (frame submit,
ime_cursor_area) needed no core change. The A2/A3 fixes were all core-side;
the host never needed a patch.

## Decision

- Core (`markit-core`, framework-free Rust) names capabilities, never
  OSes: `ClipboardProvider`, `FontProvider`, `ImeProvider`,
  `FileDialogProvider`, `ShortcutPolicy`, `PlatformPaths`.
- The GPUI/platform layer is the boundary: GPUI itself abstracts a large
  amount of OS behavior (window, input, text, presentation). Platform
  integration lives at the GPUI edge, and Markit adapters implement only
  the capabilities GPUI does not already provide (or where Markit needs
  semantics GPUI does not expose).
- Do not create duplicate wrapper abstractions unless Markit semantics
  need them (AGENTS.md §8, §9).
- No `if (platform == ...)` in core code; real three-platform differences
  are the only things abstracted (no abstraction for its own sake).

## Trade-offs

- Thin adapter layer per platform vs core complexity; capability-driven
  interfaces keep the boundary honest and the core portable.
- GPUI already isolates much platform behavior; the remaining adapters
  must stay small or they are evidence of a missing GPUI capability,
  which is a GPUI baseline question (roadmap G0), not a reason to grow a
  Markit-private OS compatibility layer.

## Note (post-pivot)

The original A4 wording said "the PocketJS host is the socket: svc
messages carry platform events". That mechanism is superseded: there is
no host/svc bridge in the direct-GPUI architecture. The principle it
served — *share editor policy/state; isolate platform mechanisms* —
remains the boundary contract of this ADR and is implemented at the
GPUI edge instead.
