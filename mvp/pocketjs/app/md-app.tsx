// mvp/pocketjs/app/md-app.tsx — A4-R2 Markdown L1 styled editor (guest).
//
// The L0 editor (app.tsx) plus the Phase A4-R2 incremental Markdown L1
// pipeline: Document → Block Index → Incremental Parse → Affected Blocks
// → Styled Runs → Visible Layout → DrawList.
//
//   - BlockIndex (markdown.ts) is maintained incrementally in applyState,
//     exactly like the LineIndex (A3-P1): one full scan at load, then a
//     per-edit rescan whose consumed range is the structural invalidation
//     radius (blocksScanned / blocksReparsed).
//   - Styled runs are computed per affected block (parseInline, cached by
//     block start line) — inlineParsed counts the re-parses per edit.
//   - Each visible line renders its runs as Text nodes colored by style
//     (L1 keeps the Markdown syntax visible). Font slot is unchanged, so
//     the caret/measure math of the L0 editor stays valid.
//   - The R2 work counters (blocksScanned, blocksReparsed, inlineParsed,
//     runsRendered) ride the same perf reply as the R1 counters.
//
// This is the R2 measurement surface, not the final product editor — the
// A4-P architecture layers the same pipeline into markit-core.

import { createMemo, createSignal, For, Show } from "solid-js";
import { Focusable, Text, View } from "@pocketjs/framework/components";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { hitFocusable } from "@pocketjs/framework/input";
import { getOps, resizeViewport } from "@pocketjs/framework";
import {
  LineIndex,
  backspaceSel,
  caretFromX,
  caretRow,
  caretX,
  deleteSel,
  lineEnd,
  lineStart,
  moveVertical,
  selBounds,
  typeText,
  type EditChange,
  type EditState,
} from "./editor.ts";
import { resolveViewSlots, type LineSlot } from "./view-slots.ts";
import {
  BlockIndex,
  parseInline,
  type Block,
  type Run,
} from "./markdown.ts";
import { connectSvc, type HostEvent } from "./svc.ts";
import { SAMPLE_DOC } from "./sample.ts";
import {
  markitPhaseTimed,
  perfCounters,
  perfEditCommit,
  perfOpCalls,
  perfOpCounts,
  perfRecordBlockIndex,
  perfRecordGuestPhases,
  perfRecordMeasure,
  perfRecordRunsRendered,
  perfRecordSvcEvents,
  perfRecordSvcSend,
  perfRecordVisible,
  perfRequest,
  perfTakeRequest,
  perfTickFrames,
} from "./perf.ts";

const FONT_SLOT = 3;
const LINE_H = 28;
const CARET_W = 2;
const CARET_H = LINE_H;
const SCROLL_STEP = 56;
const DRAG_SLOP = 3;

const INK = {
  bg: "#ffffff",
  body: "#333333",
  caret: "#0000ff",
  sel: "#3311ff30",
  // L1 style colors (color-only styling keeps the caret math font-uniform).
  heading: "#1e3a8a",
  quote: "#64748b",
  code: "#0f766e",
  list: "#333333",
  bold: "#7c2d12",
  em: "#444444",
  link: "#1d4ed8",
};

const widthCache = new Map<string, number>();
function textWidth(text: string): number {
  if (text === "") return 0;
  const key = FONT_SLOT + "|" + text;
  let w = widthCache.get(key);
  if (w === undefined) {
    w = getOps().measureText(text, FONT_SLOT);
    perfRecordMeasure(text.length);
    widthCache.set(key, w);
  }
  return w;
}

/** Block base color for body runs. */
function blockColor(kind: Block["kind"]): string {
  switch (kind) {
    case "heading":
      return INK.heading;
    case "quote":
      return INK.quote;
    case "fenced":
      return INK.code;
    default:
      return INK.body;
  }
}

/** Inline style color (wins over the block base). */
function runColor(style: Run["style"], kind: Block["kind"]): string {
  switch (style) {
    case "bold":
      return INK.bold;
    case "em":
      return INK.em;
    case "code":
      return INK.code;
    case "link":
      return INK.link;
    default:
      return blockColor(kind);
  }
}

export default function MdEditor(): ReturnType<typeof View> {
  const svc = connectSvc();
  const [vp, setVp] = createSignal({ w: 1000, h: 700 });
  const [doc, setDoc] = createSignal(SAMPLE_DOC);
  const [caret, setCaret] = createSignal(0);
  const [anchor, setAnchor] = createSignal(0);
  const [scrollE, setScrollE] = createSignal(0);

  const starts = createMemo(() => {
    void doc();
    return lineIndex.starts;
  });
  const totalH = () => starts().length * LINE_H;
  const maxScroll = () => Math.max(0, totalH() - vp().h);
  const viewH = () => vp().h;

  // A4-R2: cached styled runs per block, keyed by the block's start line.
  // Invalidated for exactly the blocks the rescan replaced.
  const blockRuns = new Map<number, Run[]>();

  /** Styled runs covering a block's text (lazy for fenced/blank). */
  function runsFor(block: Block): Run[] {
    const cached = blockRuns.get(block.startLine);
    if (cached) return cached;
    if (block.kind === "fenced") {
      const s = starts()[block.startLine];
      const e = lineEnd(doc(), starts(), starts()[block.endLine]);
      const runs: Run[] = [{ start: 0, end: e - s, style: "body" }];
      blockRuns.set(block.startLine, runs);
      return runs;
    }
    if (block.kind === "blank") return [];
    const s = starts()[block.startLine];
    const e = lineEnd(doc(), starts(), starts()[block.endLine]);
    const runs = parseInline(doc().slice(s, e));
    blockRuns.set(block.startLine, runs);
    return runs;
  }

  /** Runs for one visible line: the block's runs sliced to [ls, le), each
   *  carrying its x offset (font-uniform measure of the line prefix). */
  function lineRuns(line: number): { start: number; end: number; color: string; x: number }[] {
    const bi = blockIndex.blockAt(line);
    if (bi.block.kind === "blank") return [];
    const runs = runsFor(bi.block);
    const ls = starts()[line];
    const le = lineEnd(doc(), starts(), ls);
    const bs = starts()[bi.block.startLine];
    const lo = ls - bs;
    const hi = le - bs;
    const out: { start: number; end: number; color: string; x: number }[] = [];
    for (const r of runs) {
      const s = Math.max(r.start, lo);
      const e = Math.min(r.end, hi);
      if (e > s) out.push({
        start: bs + s,
        end: bs + e,
        color: runColor(r.style, bi.block.kind),
        x: textWidth(doc().slice(ls, bs + s)),
      });
    }
    return out;
  }

  // Visible line range (same formula as app.tsx; see view-slots.ts for
  // the stateless-projection invariant).
  const lineCache = new Map<number, LineSlot>();
  const visibleLines = createMemo(() => {
    const from = Math.max(0, Math.floor(scrollE() / LINE_H));
    const to = Math.min(starts().length, from + Math.ceil(viewH() / LINE_H) + 1);
    const out = resolveViewSlots(from, to, lineCache);
    perfRecordVisible(to - from);
    return out;
  });

  const caretPx = () => caretX(doc(), starts(), caret(), textWidth);
  const caretRow = () => caretRowOf(starts(), caret());
  const editSel = (): [number, number] | null => {
    if (caret() === anchor()) return null;
    return selBounds({ doc: doc(), caret: caret(), anchor: anchor() });
  };
  const lineSelRect = (line: number) => {
    const sel = editSel();
    if (!sel) return null;
    const ls = starts()[line];
    const le = lineEnd(doc(), starts(), ls);
    if (sel[1] <= ls || sel[0] >= le) return null;
    const x0 = sel[0] <= ls ? 0 : textWidth(doc().slice(ls, Math.min(sel[0], le)));
    const x1 = sel[1] >= le ? textWidth(doc().slice(ls, le)) : textWidth(doc().slice(ls, Math.min(sel[1], le)));
    return { x0, x1 };
  };

  const selState = (): EditState => ({ doc: doc(), caret: caret(), anchor: anchor() });
  const applyState = (s: EditState, change?: EditChange | null) => {
    markitPhaseTimed(
      () => {
        if (!change) return;
        // LineIndex first (the block rescan reads the new line offsets).
        lineIndex.applyEdit(change.start, change.end, change.text);
        const lineLo = lineIndex.lineOf(change.start).line;
        const lineHi = lineIndex.lineOf(Math.max(change.start, change.end)).line;
        const res = blockIndex.applyEdit(lineLo, lineHi, lineIndex.starts.length, (l) => {
          const st = lineIndex.starts[l] ?? s.doc.length;
          const en = l + 1 < lineIndex.starts.length ? lineIndex.starts[l + 1] : s.doc.length;
          return s.doc.slice(st, en - (en > st && s.doc[en - 1] === "\n" ? 1 : 0));
        });
        // Invalidate + eagerly re-parse exactly the replaced blocks
        // (inlineParsed = the inline invalidation radius).
        let inline = 0;
        for (const oldStart of res.replacedStartLines) blockRuns.delete(oldStart);
        for (let k = res.lo; k <= res.hi; k++) {
          const b = blockIndex.blocks[k];
          if (b.kind !== "fenced" && b.kind !== "blank") {
            const st = starts()[b.startLine];
            const en = lineEnd(s.doc, lineIndex.starts, starts()[b.endLine]);
            blockRuns.set(b.startLine, parseInline(s.doc.slice(st, en)));
            inline += 1;
          }
        }
        perfRecordBlockIndex(res.stats.linesScanned, res.stats.blocksReparsed, inline);
      },
      (ms) => perfRecordGuestPhases(0, ms, 0),
    );
    markitPhaseTimed(
      () => {
        setDoc(s.doc);
        setCaret(s.caret);
        setAnchor(s.anchor);
      },
      (ms) => perfRecordGuestPhases(0, 0, ms),
    );
  };
  const mutate = (f: (s: EditState) => EditResult) => {
    const r = markitPhaseTimed(
      () => f(selState()),
      (ms) => perfRecordGuestPhases(ms, 0, 0),
    );
    applyState(r.state, r.change);
  };

  const handleKey = (k: string, shift = false) => {
    if (shift) {
      const extend = (pos: number) => setCaret(Math.max(0, Math.min(pos, doc().length)));
      switch (k) {
        case "Left":
          extend(caret() - 1);
          return;
        case "Right":
          extend(caret() + 1);
          return;
        case "Home":
          extend(lineStart(starts(), caret()));
          return;
        case "End":
          extend(lineEnd(doc(), starts(), caret()));
          return;
        case "Up":
        case "Down":
          extend(moveVertical(doc(), starts(), caret(), k === "Up" ? -1 : 1, caretPx(), textWidth));
          return;
      }
    }
    if (k === "Escape") {
      setAnchor(caret());
      return;
    }
    if (k === "SelectAll") {
      setCaret(doc().length);
      setAnchor(0);
      return;
    }
    switch (k) {
      case "Backspace":
        mutate(backspaceSel);
        break;
      case "Delete":
        mutate(deleteSel);
        break;
      case "Enter":
        mutate((s) => typeText(s, "\n"));
        break;
      case "Tab":
        mutate((s) => typeText(s, "  "));
        break;
      case "Left":
        mutate((s) => ({
          state: { doc: s.doc, caret: Math.max(0, s.caret - 1), anchor: Math.max(0, s.caret - 1) },
          change: null,
        }));
        break;
      case "Right":
        mutate((s) => ({
          state: { doc: s.doc, caret: Math.min(s.doc.length, s.caret + 1), anchor: Math.min(s.doc.length, s.caret + 1) },
          change: null,
        }));
        break;
      case "Home":
        mutate((s) => {
          const at = lineStart(starts(), s.caret);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "End":
        mutate((s) => {
          const at = lineEnd(s.doc, starts(), s.caret);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "Up":
      case "Down":
        mutate((s) => {
          const at = moveVertical(s.doc, starts(), s.caret, k === "Up" ? -1 : 1, caretPx(), textWidth);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "PageUp":
        setScrollE(Math.max(0, Math.min(maxScroll(), scrollE() - viewH())));
        break;
      case "PageDown":
        setScrollE(Math.max(0, Math.min(maxScroll(), scrollE() + viewH())));
        break;
    }
  };

  const handleEvent = (ev: HostEvent) => {
    switch (ev.t) {
      case "perfreq":
        perfRequest();
        break;
      case "hello":
      case "resize":
        setVp({ w: ev.w ?? 1000, h: ev.h ?? 700 });
        resizeViewport(ev.w ?? 1000, ev.h ?? 700);
        setScrollE(Math.max(0, Math.min(maxScroll(), scrollE())));
        break;
      case "load":
        lineIndex = new LineIndex(ev.text ?? "");
        blockIndex = new BlockIndex((l) => {
          const st = lineIndex.starts[l] ?? ev.text!.length;
          const en = l + 1 < lineIndex.starts.length ? lineIndex.starts[l + 1] : ev.text!.length;
          return ev.text!.slice(st, en - (en > st && ev.text![en - 1] === "\n" ? 1 : 0));
        }, lineIndex.starts.length);
        blockRuns.clear();
        setDoc(ev.text ?? "");
        setCaret(0);
        setAnchor(0);
        break;
      case "ch":
        if (ev.s) mutate((s) => typeText(s, ev.s!));
        break;
      case "paste":
        if (ev.text) mutate((s) => typeText(s, ev.text!));
        break;
      case "key":
        if (ev.k) handleKey(ev.k, ev.sh ?? false);
        break;
      case "mouse": {
        const p = { x: ev.x ?? -1, y: ev.y ?? -1 };
        const down = ev.d ?? false;
        if (down && !prevDown) pointerDown(p.x, p.y, ev.sh ?? false);
        else if (down) pointerMove(p.x, p.y);
        if (!down && prevDown) pointerUp();
        prevDown = down;
        const n = hitFocusable(p.x, p.y);
        if (n) n.focus?.();
        break;
      }
      case "scroll":
        setScrollE(Math.max(0, Math.min(maxScroll(), scrollE() + (ev.dy ?? 0))));
        break;
      case "ime":
        break;
    }
  };

  let press: { x: number; y: number; dragged: boolean } | null = null;
  let prevDown = false;

  const editPosAt = (x: number, y: number): number => {
    const line = Math.floor((y + scrollE()) / LINE_H);
    return caretFromX(doc(), starts(), line, x, textWidth);
  };

  const pointerDown = (x: number, y: number, shift: boolean) => {
    press = { x, y, dragged: false };
    const pos = editPosAt(x, y);
    setCaret(pos);
    if (!shift) setAnchor(pos);
  };
  const pointerMove = (x: number, y: number) => {
    if (!press) return;
    if (!press.dragged && Math.abs(x - press.x) + Math.abs(y - press.y) < DRAG_SLOP) return;
    press.dragged = true;
    setCaret(editPosAt(x, y));
  };
  const pointerUp = () => {
    press = null;
  };

  onFrame(() => {
    if (!svc) return;
    perfTickFrames();
    const events = svc.poll();
    perfRecordSvcEvents(events.length);
    let anyEdit = false;
    for (const ev of events) {
      handleEvent(ev);
      if (ev.t === "ch" || ev.t === "key" || ev.t === "paste" || ev.t === "load") anyEdit = true;
    }
    if (anyEdit) perfEditCommit();
    if (perfTakeRequest()) {
      const c = perfCounters();
      svc.send({
        t: "perf",
        frames: c.frames,
        edits: c.edits,
        docChars: doc().length,
        docLines: starts().length,
        lineStartsScans: c.lineStartsScans,
        lineStartsChars: c.lineStartsChars,
        lineStartsMs: c.lineStartsMs,
        lineIndexAdjusts: c.lineIndexAdjusts,
        newlinesInserted: c.newlinesInserted,
        newlinesDeleted: c.newlinesDeleted,
        typeCopies: c.typeCopies,
        typeCopyChars: c.typeCopyChars,
        typeMs: c.typeMs,
        modelMs: c.modelMs,
        indexMs: c.indexMs,
        solidMs: c.solidMs,
        blocksScanned: c.blocksScanned,
        blocksReparsed: c.blocksReparsed,
        inlineParsed: c.inlineParsed,
        runsRendered: c.runsRendered,
        visibleVisits: c.visibleVisits,
        visibleLines: c.visibleLines,
        measures: c.measures,
        measureChars: c.measureChars,
        svcEvents: c.svcEvents,
        svcSends: c.svcSends,
        ops: perfOpCalls(),
        opCounts: JSON.stringify(perfOpCounts()),
        editsRing: JSON.stringify(c.editsRing),
      });
    }
    if (events.length > 0) {
      svc.send({
        t: "state",
        caret: caret(),
        anchor: anchor(),
        scrollY: scrollE(),
        w: vp().w,
        h: vp().h,
        docHead: doc().slice(0, 64),
      });
      svc.send({
        t: "caret",
        x: Math.round(caretPx()),
        y: Math.round(caretRow() * LINE_H - scrollE()),
        h: LINE_H,
      });
    }
  });

  return (
    <Focusable class="relative flex-1 overflow-hidden" style={{ bgColor: INK.bg }}>
      <View
        class="absolute"
        style={{ insetL: 0, insetT: 0, width: vp().w, height: totalH(), translateY: -scrollE() }}
      >
        <For each={visibleLines()}>
          {(item) => {
            // Same stable-item discipline as app.tsx (A4-R1): the item
            // carries only the line number; doc-dependent reads live in
            // item-scoped memos.
            const line = item.line;
            const sel = createMemo(() => lineSelRect(line));
            const runs = createMemo(() => {
              const out = lineRuns(line);
              perfRecordRunsRendered(out.length);
              return out;
            });
            return (
              <View class="absolute" style={{ insetL: 0, insetT: line * LINE_H, height: LINE_H }}>
                <Show when={sel() != null}>
                  <View
                    class="absolute"
                    style={{
                      insetL: sel()?.x0 ?? 0,
                      insetT: 0,
                      width: Math.max(2, (sel()?.x1 ?? 0) - (sel()?.x0 ?? 0)),
                      height: LINE_H,
                      bgColor: INK.sel,
                    }}
                  />
                </Show>
                <For each={runs()}>
                  {(run) => (
                    <Text
                      class="absolute text-lg"
                      style={{
                        insetL: run.x,
                        insetT: 0,
                        height: LINE_H,
                        lineHeight: LINE_H,
                        textColor: run.color,
                      }}
                    >
                      {doc().slice(run.start, run.end)}
                    </Text>
                  )}
                </For>
              </View>
            );
          }}
        </For>
        <Show when={editSel() == null}>
          <View
            class="absolute"
            style={{
              width: CARET_W,
              insetL: caretPx(),
              insetT: caretRow() * LINE_H - scrollE(),
              height: CARET_H,
              bgColor: INK.caret,
            }}
          />
        </Show>
      </View>
    </Focusable>
  );
}

let lineIndex: LineIndex = new LineIndex(SAMPLE_DOC);
let blockIndex: BlockIndex = new BlockIndex((l) => {
  const st = lineIndex.starts[l] ?? SAMPLE_DOC.length;
  const en = l + 1 < lineIndex.starts.length ? lineIndex.starts[l + 1] : SAMPLE_DOC.length;
  return SAMPLE_DOC.slice(st, en - (en > st && SAMPLE_DOC[en - 1] === "\n" ? 1 : 0));
}, lineIndex.starts.length);

function caretRowOf(startsArr: number[], caretPos: number): number {
  return caretRow(startsArr, caretPos);
}
