// mvp/pocketjs/app/perf.ts — Phase A2 instrumentation (Markit-owned).
//
// Work counters + coarse wall-clock timing for the guest edit path. This
// module changes NO behavior: counters are cheap integer increments, timing
// uses Date.now() (ms resolution — only meaningful on multi-ms stages at
// 1M-scale documents), and nothing is emitted unless the host asks with a
// {"t":"perfreq"} svc message. Removing this module restores the exact
// original guest behavior.

/** One edit's counter snapshot (ring of the last EDITS records). */
export interface PerfEdit {
  scanChars: number;
  scanMs: number;
  copyChars: number;
  copyMs: number;
  visibleLines: number;
  measures: number;
  ops: number;
}

export interface PerfCounters {
  /** Guest turns (frames) since boot. */
  frames: number;
  /** Document mutations applied. */
  edits: number;
  /** Full-document line-index rebuilds. */
  lineStartsScans: number;
  /** Chars scanned by lineStarts (== doc length per scan). */
  lineStartsChars: number;
  /** Wall time spent in lineStarts, ms (coarse; 0 when Date is unavailable). */
  lineStartsMs: number;
  /** typeText concat copies. */
  typeCopies: number;
  /** Chars copied by typeText concat (≈ doc length per edit). */
  typeCopyChars: number;
  /** Wall time spent in typeText, ms. */
  typeMs: number;
  /** visibleLines recomputes. */
  visibleVisits: number;
  /** Lines visited by visibleLines. */
  visibleLines: number;
  /** measureText host calls (cache misses). */
  measures: number;
  /** Chars measured by measureText. */
  measureChars: number;
  /** Host svc events consumed. */
  svcEvents: number;
  /** Guest -> host svc lines sent. */
  svcSends: number;
  /** Per-edit records (ring, oldest first). */
  editsRing: PerfEdit[];
}

/** Ring of the most recent edit records (perf reply only). */
const EDITS_RING = 32;
const editsRing: PerfEdit[] = [];

const ZERO: PerfCounters = {
  frames: 0,
  edits: 0,
  lineStartsScans: 0,
  lineStartsChars: 0,
  lineStartsMs: 0,
  typeCopies: 0,
  typeCopyChars: 0,
  typeMs: 0,
  visibleVisits: 0,
  visibleLines: 0,
  measures: 0,
  measureChars: 0,
  svcEvents: 0,
  svcSends: 0,
  editsRing: [],
};

let counters: PerfCounters = { ...ZERO };
let requested = false;

/** Current per-edit record being accumulated (set by perfEditBegin). */
let cur: PerfEdit = { scanChars: 0, scanMs: 0, copyChars: 0, copyMs: 0, visibleLines: 0, measures: 0, ops: 0 };
/** Op count at the beginning of the current frame. */
let opsAtFrameStart = 0;

export function perfEditBegin(): void {
  cur = { scanChars: 0, scanMs: 0, copyChars: 0, copyMs: 0, visibleLines: 0, measures: 0, ops: 0 };
  opsAtFrameStart = opCalls;
}

export function perfEditCommit(): void {
  cur.ops = opCalls - opsAtFrameStart;
  editsRing.push(cur);
  if (editsRing.length > EDITS_RING) editsRing.shift();
  counters.editsRing = [...editsRing];
}

/** Host asked for a dump: the next svc reply carries the counters. */

/** Wall clock, ms since epoch. Falls back to 0 when the runtime lacks Date. */
export function perfNow(): number {
  try {
    return Date.now();
  } catch {
    return 0;
  }
}

/** A per-edit timing+counting helper: returns an object to record into. */
export function perfEditStart(): PerfCounters {
  return counters;
}

export function perfCounters(): PerfCounters {
  return counters;
}

/** Host asked for a dump: the next svc reply carries the counters. */
export function perfRequest(): void {
  requested = true;
}

export function perfTakeRequest(): boolean {
  const r = requested;
  requested = false;
  return r;
}

export function perfTickFrames(): void {
  counters.frames += 1;
  perfEditBegin();
}

export function perfRecordEdit(): void {
  counters.edits += 1;
}

export function perfRecordLineStarts(chars: number, ms: number): void {
  counters.lineStartsScans += 1;
  counters.lineStartsChars += chars;
  counters.lineStartsMs += ms;
  cur.scanChars += chars;
  cur.scanMs += ms;
}

export function perfRecordTypeCopy(chars: number, ms: number): void {
  counters.typeCopies += 1;
  counters.typeCopyChars += chars;
  counters.typeMs += ms;
  cur.copyChars += chars;
  cur.copyMs += ms;
}

export function perfRecordVisible(lines: number): void {
  counters.visibleVisits += 1;
  counters.visibleLines += lines;
  cur.visibleLines += lines;
}

export function perfRecordMeasure(chars: number): void {
  counters.measures += 1;
  counters.measureChars += chars;
  cur.measures += 1;
}

export function perfRecordSvcEvents(n: number): void {
  counters.svcEvents += n;
}

export function perfRecordSvcSend(): void {
  counters.svcSends += 1;
}

/** Host ui.* op calls (perfInit wraps the native ops). */
let opCalls = 0;

// ---- Phase A2 DIAGNOSTIC counterfactuals (compile-time; NEVER a product
// configuration). One rebuild per variant; reverted before the phase ends.
//   0 = production behavior
//   1 = skip the lineStarts scan (line index goes stale)
//   2 = skip the typeText concat (document does not change)
//   3 = skip both
export const CF: number = 0;

export function cfSkipScan(): boolean {
  return CF === 1 || CF === 3;
}

export function cfSkipConcat(): boolean {
  return CF === 2 || CF === 3;
}

/**
 * Wrap the native ui ops with per-call counters. Costs ~2 ms per edit turn
 * at 10K (calibration), so it is OFF for the main battery and enabled only
 * for the dedicated op-churn run (WRAP_OPS=1).
 */
export const WRAP_OPS: boolean = false;

/**
 * Wrap the native `globalThis.ui` ops with counters. Must run before
 * `mount()` renders the tree. The framework resolves ops lazily from the
 * same object (host.ts getOps), so wrapped properties are seen by every
 * renderer call. Overhead: one JS call + increment per op.
 */
export function perfInit(): void {
  if (!WRAP_OPS) return;
  try {
    const ui = (globalThis as unknown as { ui?: Record<string, unknown> }).ui;
    if (!ui || typeof ui.createNode !== "function") return;
    const names = [
      "createNode", "destroyNode", "insertBefore", "removeChild",
      "setStyle", "setProp", "setText", "replaceText", "setImage",
      "setSprite", "measureText", "hitTest", "hitTestBounds",
      "setFocus", "setActive",
    ];
    for (const name of names) {
      const orig = ui[name] as (...args: unknown[]) => unknown;
      if (typeof orig !== "function") continue;
      ui[name] = (...args: unknown[]) => {
        opCalls += 1;
        return orig(...args);
      };
    }
  } catch {
    // No native ui (web/test host): counters stay at 0.
  }
}

export function perfOpCalls(): number {
  return opCalls;
}
