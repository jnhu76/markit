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
  /** Suffix line-index entries shifted by this edit's length delta. */
  adjusts: number;
  /** '\n' inserted by this edit. */
  newlinesIn: number;
  /** '\n' removed by this edit. */
  newlinesDel: number;
  /** A4-R1: document-mutation wall time, ms (Date.now() — coarse). */
  modelMs: number;
  /** A4-R1: line-index + signal-write (Solid sync render) wall time, ms. */
  indexMs: number;
  /** A4-R1: Solid synchronous re-render wall time, ms (inside indexMs). */
  solidMs: number;
  /** A4-R2: lines examined by the Markdown block-index rescan. */
  blocksScanned: number;
  /** A4-R2: blocks created by the rescan (the invalidation radius). */
  blocksReparsed: number;
  /** A4-R2: blocks inline-parsed (styled runs recomputed). */
  inlineParsed: number;
  /** A4-R2: styled run Text nodes rendered for the visible range. */
  runsRendered: number;
}

export interface PerfCounters {
  /** Guest turns (frames) since boot. */
  frames: number;
  /** Document mutations applied. */
  edits: number;
  /** Full-document line-index rebuilds (load-time only after A3). */
  lineStartsScans: number;
  /** Chars scanned by lineStarts (== doc length per scan). */
  lineStartsChars: number;
  /** Wall time spent in lineStarts, ms (coarse; 0 when Date is unavailable). */
  lineStartsMs: number;
  /** Line-index suffix entries shifted by incremental updates. */
  lineIndexAdjusts: number;
  /** '\n' inserted by edits. */
  newlinesInserted: number;
  /** '\n' removed by edits. */
  newlinesDeleted: number;
  /** typeText concat copies. */
  typeCopies: number;
  /** Chars copied by typeText concat (≈ doc length per edit). */
  typeCopyChars: number;
  /** Wall time spent in typeText, ms. */
  typeMs: number;
  /** A4-R2: lines examined by the Markdown block-index rescan. */
  blocksScanned: number;
  /** A4-R2: blocks created by the rescan (invalidation radius). */
  blocksReparsed: number;
  /** A4-R2: blocks inline-parsed (styled runs recomputed). */
  inlineParsed: number;
  /** A4-R2: styled run Text nodes rendered for the visible range. */
  runsRendered: number;
  /** A4-R1: document-mutation wall time (concat), ms. */
  modelMs: number;
  /** A4-R1: line-index + Solid signal-write wall time, ms. */
  indexMs: number;
  /** A4-R1: Solid synchronous re-render wall time, ms. */
  solidMs: number;
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
  lineIndexAdjusts: 0,
  newlinesInserted: 0,
  newlinesDeleted: 0,
  typeCopies: 0,
  typeCopyChars: 0,
  typeMs: 0,
  blocksScanned: 0,
  blocksReparsed: 0,
  inlineParsed: 0,
  runsRendered: 0,
  modelMs: 0,
  indexMs: 0,
  solidMs: 0,
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
let cur: PerfEdit = { scanChars: 0, scanMs: 0, copyChars: 0, copyMs: 0, visibleLines: 0, measures: 0, ops: 0, adjusts: 0, newlinesIn: 0, newlinesDel: 0, modelMs: 0, indexMs: 0, solidMs: 0, blocksScanned: 0, blocksReparsed: 0, inlineParsed: 0, runsRendered: 0 };
/** Op count at the beginning of the current frame. */
let opsAtFrameStart = 0;

export function perfEditBegin(): void {
  cur = { scanChars: 0, scanMs: 0, copyChars: 0, copyMs: 0, visibleLines: 0, measures: 0, ops: 0, adjusts: 0, newlinesIn: 0, newlinesDel: 0, modelMs: 0, indexMs: 0, solidMs: 0, blocksScanned: 0, blocksReparsed: 0, inlineParsed: 0, runsRendered: 0 };
  opsAtFrameStart = opCalls;
}

export function perfEditCommit(): void {
  cur.ops = opCalls - opsAtFrameStart;
  counters.edits += 1;
  editsRing.push(cur);
  if (editsRing.length > EDITS_RING) editsRing.shift();
  counters.editsRing = [...editsRing];
}

/** A4-R1: guest-side phase wall times for the last edit (Date.now() ms —
 *  coarse by design; the host's gf_us/ct_us are the precise clocks). */
export function perfRecordGuestPhases(modelMs: number, indexMs: number, solidMs: number): void {
  counters.modelMs += modelMs;
  counters.indexMs += indexMs;
  counters.solidMs += solidMs;
  cur.modelMs += modelMs;
  cur.indexMs += indexMs;
  cur.solidMs += solidMs;
}

/** A4-R2: record one edit's Markdown block-index + inline-parse work. */
export function perfRecordBlockIndex(scanned: number, reparsed: number, inline: number): void {
  counters.blocksScanned += scanned;
  counters.blocksReparsed += reparsed;
  counters.inlineParsed += inline;
  cur.blocksScanned += scanned;
  cur.blocksReparsed += reparsed;
  cur.inlineParsed += inline;
}

/** A4-R2: record styled-run Text nodes rendered for the visible range. */
export function perfRecordRunsRendered(n: number): void {
  counters.runsRendered += n;
  cur.runsRendered += n;
}

/**
 * A4-R1: run `fn` and record its wall time into the current edit's phase
 * accumulators. Keeps every markitNow() call inside this module's scope —
 * the bundler must not be relied on to rewrite cross-module timer refs.
 */
export function markitPhaseTimed<T>(fn: () => T, record: (ms: number) => void): T {
  const t0 = markitNow();
  const r = fn();
  record(markitNow() - t0);
  return r;
}

/** Host asked for a dump: the next svc reply carries the counters. */

/** Wall clock, ms since epoch. Falls back to 0 when the runtime lacks Date. */
export function markitNow(): number {
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

/** Incremental line-index update: suffix entries shifted + newlines moved. */
export function perfRecordLineIndex(adjusted: number, inserted: number, deleted: number): void {
  counters.lineIndexAdjusts += adjusted;
  counters.newlinesInserted += inserted;
  counters.newlinesDeleted += deleted;
  cur.adjusts += adjusted;
  cur.newlinesIn += inserted;
  cur.newlinesDel += deleted;
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

/** A4-R1: per-op-name call counts (WRAP_OPS=1 runs only). */
const opCounters = new Map<string, number>();

export function perfOpCounts(): Record<string, number> {
  const out: Record<string, number> = {};
  for (const [name, n] of opCounters) out[name] = n;
  return out;
}

/**
 * Wrap the native ui ops with per-call counters. Costs ~2 ms per edit turn
 * at 10K (calibration), so it is OFF for the main battery and enabled only
 * for the dedicated op-churn run. The dedicated entries (main-ops.tsx,
 * cf-ops.ts) flip it before the first render via perfSetWrapOps.
 */
let WRAP_OPS: boolean = false;

export function perfSetWrapOps(v: boolean): void {
  WRAP_OPS = v;
}

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
        opCounters.set(name, (opCounters.get(name) ?? 0) + 1);
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
