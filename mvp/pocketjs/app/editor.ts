// mvp/pocketjs/app/editor.ts — Markit PocketJS thin-editor model.
//
// Deliberately framework-free and mirroring the GPUI Phase A0 prototype's
// semantics so the two MVPs edit the same corpus the same way:
//   - flat document string, line index rebuilt on every mutation;
//   - caret/selection in code-unit offsets (U0/ASCII corpus: code unit ==
//     byte; a Unicode ladder (U1+) will need byte/scalar/UTF-16 awareness);
//   - NO soft wrap (GPUI paints full lines; the window clips long ones);
//   - manual scroll offset, fixed line height, visible-line virtualization
//     is the app's job (app.tsx), not the model's.
//   - no undo (GPUI Phase A0 has none; parity before features).
//
// The measure function is injected (getOps().measureText behind a cache in
// app.tsx) so this module stays pure and deterministic.

export type Measure = (text: string) => number;

export interface EditState {
  doc: string;
  /** Caret position (code-unit offset) — the moving end of a selection. */
  caret: number;
  /** Selection anchor; equal to caret when collapsed. */
  anchor: number;
}

/** Byte/code-unit offset of each line start, mirroring GPUI's line_starts. */
export function lineStarts(doc: string): number[] {
  const starts = [0];
  for (let i = 0; i < doc.length; i++) {
    if (doc[i] === "\n") starts.push(i + 1);
  }
  return starts;
}

/** (line index, line start) for a code-unit offset. */
export function lineOf(starts: number[], offset: number): { line: number; start: number } {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (starts[mid] <= offset) lo = mid;
    else hi = mid - 1;
  }
  return { line: lo, start: starts[lo] };
}

export function lineCount(starts: number[]): number {
  return starts.length;
}

/** Replace the current selection (or insert at the caret) with `text`. */
export function typeText(s: EditState, text: string): EditState {
  const [lo, hi] = selBounds(s);
  return {
    doc: s.doc.slice(0, lo) + text + s.doc.slice(hi),
    caret: lo + text.length,
    anchor: lo + text.length,
  };
}

/** Backspace: delete the selection, or the code unit before the caret. */
export function backspaceSel(s: EditState): EditState {
  if (s.caret !== s.anchor) return typeText(s, "");
  const at = Math.max(0, s.caret - 1);
  return {
    doc: s.doc.slice(0, at) + s.doc.slice(s.caret),
    caret: at,
    anchor: at,
  };
}

/** Delete: delete the selection, or the code unit at the caret. */
export function deleteSel(s: EditState): EditState {
  if (s.caret !== s.anchor) return typeText(s, "");
  const at = Math.min(s.doc.length, s.caret + 1);
  return {
    doc: s.doc.slice(0, s.caret) + s.doc.slice(at),
    caret: s.caret,
    anchor: s.caret,
  };
}

/** First code-unit offset of the display line containing `offset`. */
export function lineStart(starts: number[], offset: number): number {
  return lineOf(starts, offset).start;
}

/** One past the last code unit of the display line (excludes the '\n'). */
export function lineEnd(doc: string, starts: number[], offset: number): number {
  const { line, start } = lineOf(starts, offset);
  const next = line + 1 < starts.length ? starts[line + 1] : doc.length;
  return next - (next > start && doc[next - 1] === "\n" ? 1 : 0);
}

/** Normalized selection bounds [lo, hi]. */
export function selBounds(s: EditState): [number, number] {
  return s.caret <= s.anchor ? [s.caret, s.anchor] : [s.anchor, s.caret];
}

/** Caret x in px on its display line. */
export function caretX(doc: string, starts: number[], caret: number, measure: Measure): number {
  const { start } = lineOf(starts, caret);
  const end = lineEnd(doc, starts, caret);
  return measure(doc.slice(start, Math.min(caret, end)));
}

/** Place a caret from a click at (x, lineIndex) by char midpoint. */
export function caretFromX(
  doc: string,
  starts: number[],
  lineIndex: number,
  x: number,
  measure: Measure,
): number {
  const { start } = lineOf(starts, Math.min(Math.max(0, lineIndex * doc.length), doc.length));
  const end = lineEnd(doc, starts, start);
  const text = doc.slice(start, end);
  let acc = 0;
  for (let i = 0; i < text.length; i++) {
    const cw = measure(text[i]);
    if (x < acc + cw / 2) return start + i;
    acc += cw;
  }
  return end;
}

/** Caret y in px (line index * line height, caller adds padding). */
export function caretRow(starts: number[], caret: number): number {
  return lineOf(starts, caret).line;
}

/**
 * Vertical caret movement keeping goalX. Returns the code-unit offset on
 * the target display line nearest to goalX; the caret clamps to the doc.
 */
export function moveVertical(
  doc: string,
  starts: number[],
  caret: number,
  delta: number,
  goalX: number,
  measure: Measure,
): number {
  const { line } = lineOf(starts, caret);
  const target = Math.max(0, Math.min(starts.length - 1, line + delta));
  const { start } = lineOf(starts, starts[target]);
  const end = lineEnd(doc, starts, start);
  const text = doc.slice(start, end);
  let acc = 0;
  let best = end;
  for (let i = 0; i < text.length; i++) {
    const cw = measure(text[i]);
    if (goalX <= acc + cw / 2) {
      best = start + i;
      break;
    }
    acc += cw;
  }
  return best;
}
