// mvp/pocketjs/app/editor.ts — Markit PocketJS thin-editor model.
//
// Deliberately framework-free and mirroring the GPUI Phase A0 prototype's
// semantics so the two MVPs edit the same corpus the same way:
//   - flat document string, line index maintained incrementally (A3-P1:
//     one full scan at load, local updates per edit — no full-document
//     rescans on the edit hot path);
//   - caret/selection in code-unit offsets (U0/ASCII corpus: code unit ==
//     byte; a Unicode ladder (U1+) will need byte/scalar/UTF-16 awareness);
//   - NO soft wrap (GPUI paints full lines; the window clips long ones);
//   - manual scroll offset, fixed line height, visible-line virtualization
//     is the app's job (app.tsx), not the model's.
//   - no undo (GPUI Phase A0 has none; parity before features).
//
// The measure function is injected (getOps().measureText behind a cache in
// app.tsx) so this module stays pure and deterministic.

// Phase A2/A3 instrumentation (Markit-owned): work counters around the
// line-index and concat paths. Removing these lines restores the original
// behavior (at the cost of the A2-measured full-document scan per edit).
import { markitNow, perfRecordLineIndex, perfRecordLineStarts, perfRecordTypeCopy } from "./perf.ts";

export type Measure = (text: string) => number;

export interface EditState {
  doc: string;
  /** Caret position (code-unit offset) — the moving end of a selection. */
  caret: number;
  /** Selection anchor; equal to caret when collapsed. */
  anchor: number;
}

/** A document change: replace [start, end) (code-unit offsets) with text. */
export interface EditChange {
  start: number;
  end: number;
  text: string;
}

/** An edit outcome: the new state plus the change that produced it (null =
 *  caret-only move; the document did not change). Explicit changed-range
 *  propagation lets the caller maintain the line index locally. */
export interface EditResult {
  state: EditState;
  change: EditChange | null;
}

/** Byte/code-unit offset of each line start, mirroring GPUI's line_starts.
 *  Full-document scan — the A2 root cause when called per edit. A3 keeps it
 *  for load-time index construction and test oracles only. */
export function lineStarts(doc: string): number[] {
  const t0 = markitNow();
  const starts = [0];
  for (let i = 0; i < doc.length; i++) {
    if (doc[i] === "\n") starts.push(i + 1);
  }
  perfRecordLineStarts(doc.length, markitNow() - t0);
  return starts;
}

/**
 * Incremental line index (Phase A3-P1): line-start offsets maintained
 * locally across edits instead of re-derived from the whole document.
 *
 * - Construction does the one allowed full scan (load time, O(N)).
 * - `applyEdit(start, end, text)` rescans only the affected region: the
 *   entries inside the replaced range are dropped, newline positions in the
 *   inserted text are added, and the remaining suffix entries are shifted
 *   by the length delta. The suffix shift is O(lines after the edit) —
 *   instrumented as `entriesAdjusted` (A3 reports it as the residual
 *   position-dependent term, replacing A2's O(chars) full scan).
 * - `verify` is the reference oracle (full scan) for tests only; it never
 *   runs on the production edit path.
 *
 * Offset unit: JS string code units, matching the pre-A3 semantics.
 */
export class LineIndex {
  /** Code-unit offset of each line start; starts[0] === 0. */
  starts: number[];
  /** Full-document scans performed (construction only in production). */
  fullScans: number;
  /** Suffix entries shifted by an edit's length delta (O(lines-after)). */
  entriesAdjusted: number;
  /** '\n' inserted by edits. */
  newlinesInserted: number;
  /** '\n' removed by edits. */
  newlinesDeleted: number;

  constructor(doc: string) {
    this.starts = lineStarts(doc);
    this.fullScans = 1;
    this.entriesAdjusted = 0;
    this.newlinesInserted = 0;
    this.newlinesDeleted = 0;
  }

  /** (line index, line start) for a code-unit offset. */
  lineOf(offset: number): { line: number; start: number } {
    return lineOf(this.starts, offset);
  }

  /** Update for: replace [start, end) with `text` (code-unit offsets). */
  applyEdit(start: number, end: number, text: string): void {
    const lineLo = this.lineOf(start).line;
    const lineHi = this.lineOf(end).line;
    // Line starts strictly inside (start, end] mark lines that vanish with
    // the replaced text.
    const removed = lineHi - lineLo;
    // Line starts opened by '\n' inside the inserted text.
    const opened: number[] = [];
    for (let i = 0; i < text.length; i++) {
      if (text.charCodeAt(i) === 10 /* \n */) opened.push(start + i + 1);
    }
    // Suffix entries (after the replaced region) shift by the length delta.
    const delta = text.length - (end - start);
    const suffixFrom = lineLo + 1 + removed;
    const adjusted = this.starts.length - suffixFrom;
    for (let i = suffixFrom; i < this.starts.length; i++) {
      this.starts[i] += delta;
    }
    this.entriesAdjusted += adjusted;
    // Replace the affected slice with the opened entries.
    this.starts.splice(lineLo + 1, removed, ...opened);
    this.newlinesDeleted += removed;
    this.newlinesInserted += opened.length;
    perfRecordLineIndex(adjusted, opened.length, removed);
  }

  /** Reference comparison against a full scan — TEST/validation only. */
  verify(doc: string): boolean {
    const ref = lineStarts(doc);
    if (ref.length !== this.starts.length) return false;
    for (let i = 0; i < ref.length; i++) {
      if (ref[i] !== this.starts[i]) return false;
    }
    return true;
  }
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
export function typeText(s: EditState, text: string): EditResult {
  const [lo, hi] = selBounds(s);
  const t0 = markitNow();
  const doc = s.doc.slice(0, lo) + text + s.doc.slice(hi);
  perfRecordTypeCopy(s.doc.length + text.length, markitNow() - t0);
  return {
    state: {
      doc,
      caret: lo + text.length,
      anchor: lo + text.length,
    },
    change: { start: lo, end: hi, text },
  };
}

/** Backspace: delete the selection, or the code unit before the caret. */
export function backspaceSel(s: EditState): EditResult {
  if (s.caret !== s.anchor) return typeText(s, "");
  const at = Math.max(0, s.caret - 1);
  return {
    state: {
      doc: s.doc.slice(0, at) + s.doc.slice(s.caret),
      caret: at,
      anchor: at,
    },
    change: { start: at, end: s.caret, text: "" },
  };
}

/** Delete: delete the selection, or the code unit at the caret. */
export function deleteSel(s: EditState): EditResult {
  if (s.caret !== s.anchor) return typeText(s, "");
  const at = Math.min(s.doc.length, s.caret + 1);
  return {
    state: {
      doc: s.doc.slice(0, s.caret) + s.doc.slice(at),
      caret: s.caret,
      anchor: s.caret,
    },
    change: { start: s.caret, end: at, text: "" },
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
  // A4-R2 correctness fix: the line index maps DIRECTLY into `starts` —
  // the previous `lineIndex * doc.length` clamp sent every click on a
  // line >= 1 to the document end (uncovered by the R2 position cases;
  // the caret landed at the last line's start instead of the clicked
  // line, which also corrupted the R1 q1/mid/q3 position cells).
  const line = Math.max(0, Math.min(lineIndex, starts.length - 1));
  const start = starts[line];
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
