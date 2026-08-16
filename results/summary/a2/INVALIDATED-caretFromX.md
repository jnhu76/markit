# INVALIDATED / SUPERSEDED — `caretFromX` click-position bug (A2 batch)

**Do not quote these files as position measurements.** Same latent bug
as the A3 batch (see `results/summary/a3/INVALIDATED-caretFromX.md` and
the A4 erratum in `docs/phase-a3-intervention-validation.md`): every
click on a line ≥ 1 placed the caret at the end of the document.

## INVALIDATED (click landed at document end — end-position edits)

- `pjs-pos-q1-1m-{0,1,2}.summary.txt`
- `pjs-pos-mid-1m-{0,1,2}.summary.txt`
- `pjs-pos-q3-1m-{0,1,2}.summary.txt`
- `pjs-vp-inside-1m-{0,1,2}.summary.txt`
- `pjs-vp-near-1m-{0,1,2}.summary.txt`
- `pjs-vp-far-1m-{0,1,2}.summary.txt`

## VALID (kept; still usable as stated)

- `pjs-pos-begin-1m-{0,1,2}.summary.txt` — click at line 0 maps
  correctly under the bug.
- `pjs-pos-end-1m-{0,1,2}.summary.txt` — valid by coincidence: the
  bug's caret-at-document-end is exactly the intended position.

## Conclusion status

A2's qualitative conclusion — the per-edit `lineStarts` scan is O(N) at
every position, so edit position does not change the cost — is
unaffected: every cell (valid or not) exercised the same full-document
scan. A2's position/viewport tables are therefore still valid *as
evidence that the scan is position-independent*; they must not be used
for any finer position gradient. The corrected gradient exists only in
the A4 re-measurement (`docs/phase-a4-final-research-closeout.md` §2.4).
