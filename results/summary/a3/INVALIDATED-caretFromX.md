# INVALIDATED / SUPERSEDED — `caretFromX` click-position bug

**Do not quote these files as position measurements.** A4's R2 position
cases uncovered a pre-existing bug in `mvp/pocketjs/app/editor.ts`
(`caretFromX` computed the click line as
`Math.min(Math.max(0, lineIndex * doc.length), doc.length)`): every
click on a line ≥ 1 placed the caret at the **end of the document** and
ignored the x-coordinate. The bug was present since the A1 MVP and was
fixed in A4 commit `1aefbfd` (direct `starts` lookup).

Files below are kept in place for auditability (per the A4 review
provenance requirement) but their cells do not measure the positions
their names claim. See the A4 erratum in
`docs/phase-a3-intervention-validation.md` and re-measurement in
`docs/phase-a4-final-research-closeout.md` §2.4.

## INVALIDATED (click landed at document end — end-position edits)

- `after/pjs-pos-q1-1m-{0,1,2}.summary.txt`
- `after/pjs-pos-mid-1m-{0,1,2}.summary.txt`
- `after/pjs-pos-q3-1m-{0,1,2}.summary.txt`
- `after/pjs-vp-inside-1m-{0,1,2}.summary.txt`
- `after/pjs-vp-near-1m-{0,1,2}.summary.txt`
- `after/pjs-vp-far-1m-{0,1,2}.summary.txt`

## VALID (kept; still usable as stated)

- `after/pjs-pos-begin-1m-{0,1,2}.summary.txt` — click at line 0 maps
  correctly under the bug.
- `after/pjs-pos-end-1m-{0,1,2}.summary.txt` — valid by coincidence:
  the bug's caret-at-document-end is exactly the intended position.
  Superseded as a *measured* cell by the A4 re-measurement (§2.4).

## Unaffected

- `after/pjs-scale-*`, `before/pjs-scale-*` — typing-only workloads
  (no clicks).
- All `gpui-*` files (including `after-buggy/summaries/gpui-*`) — the
  GPUI click path is Rust-side and independent of this JS bug.
- `after-buggy/summaries/pjs-*` — invalid for the separate A3-M
  load-order reason documented in `docs/phase-a3-intervention-validation.md`
  §3 (idle/shape cost), not this bug.

## A2 note

The A2 PJS pos/vp cells (`results/summary/a2/pjs-pos-*`,
`results/summary/a2/pjs-vp-*`) carry the same latent corruption; see
`results/summary/a2/INVALIDATED-caretFromX.md`. A2's qualitative
conclusion (per-edit scan is O(N) at every position) is unaffected.
