# ADR-007 — IME composition model

Status: accepted (A4; implementation P1).

## Observed problem

IME text must not be treated as ordinary key events: composition
start/update/commit/cancel have distinct semantics, composition must not
enter the undo stack per keystroke, and candidates dock at the caret.

## Evidence

The A1 host protocol reserved the composition path (`{t:"ime"}` +
`{t:"caret"}` caret-rect docking); the editor model currently ignores it
(deferred). Windows IME validation is a P1 gate (platform-capability
matrix: IME NOT TESTED everywhere).

## Decision

- Editor model distinguishes composition start / update / commit /
  cancel; commit is one edit transaction (grouped for undo), updates are
  preedit state only.
- The host docks candidates at the caret rect the view reports (the wire
  contract is live since A1).
- `ImeProvider` abstracts platform IME; Chinese IME is validated first
  (P1), JA/KO architecture present (roadmap P1–P2).
