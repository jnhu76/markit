# ADR-007 — IME composition model

Status: accepted (A4; implementation P1).

> Post-pivot note (2026-08-18): the model-side semantics transfer —
> composition start/update/commit/cancel are distinct, commit is one undo
> transaction, updates are preedit state only. The original "host protocol
> / wire contract" evidence referred to the PocketJS svc channel; in the
> direct-GPUI architecture IME arrives through GPUI's platform IME path
> and candidate docking uses the caret rect the view reports to GPUI.
> `ImeProvider` remains the semantic boundary; Chinese IME is validated
> first (P1), JA/KO architecture present.

## Observed problem

IME text must not be treated as ordinary key events: composition
start/update/commit/cancel have distinct semantics, composition must not
enter the undo stack per keystroke, and candidates dock at the caret.

## Evidence

The A1 PocketJS host protocol had reserved the composition path
(`{t:"ime"}` + `{t:"caret"}` caret-rect docking); that editor model
ignored it (deferred). Windows IME validation is a P1 gate
(platform-capability matrix: IME NOT TESTED everywhere).

## Decision

- Editor model distinguishes composition start / update / commit /
  cancel; commit is one edit transaction (grouped for undo), updates are
  preedit state only.
- Candidates dock at the caret rect: the GPUI view reports caret bounds
  and GPUI's platform IME positions the candidate UI there (no custom
  host/wire contract in the direct-GPUI architecture).
- `ImeProvider` abstracts platform IME; Chinese IME is validated first
  (P1), JA/KO architecture present.
