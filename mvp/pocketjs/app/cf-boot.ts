// mvp/pocketjs/app/cf-boot.ts — A4-R1 diagnostic counterfactual (DIAGNOSTIC ONLY).
//
// Bypasses Solid's reactive reconstruction entirely: the same document
// model (editor.ts) drives the same visible presentation (the same 26 line
// Text nodes, caret and selection) through the PocketJS mirror/native ops
// directly, with no signals, no memos, no component re-execution.
//
// Purpose (Phase A4-R1 §6): with the same document, same viewport and the
// same ~25 visible lines, split the ~7 ms active-edit floor into
//
//   Solid / reactive reconstruction   vs   PocketJS lower rendering stack
//
// The CF must produce the same DrawList as the Solid app (same tree, same
// styles, same text) — the bench driver checks words/ct_us equivalence
// against the baseline bundle. NEVER a production implementation; the
// Solid path in app.tsx stays the product path. After the experiment this
// bundle is not part of any product build.
//
// Build (scripts/build-app.sh parameterized by APP_ENTRY):
//   APP_ENTRY=cf       bun tools/build.ts app/cf.ts        (text path)
//   APP_ENTRY=cf-notext bun tools/build.ts app/cf-notext.ts (no text)

import {
  detectHost,
  getOps,
  installFrameHandler,
  installHost,
} from "@pocketjs/framework/host";
import {
  createElement,
  detachNode,
  insertNode,
  replaceText,
  retain,
  rootMirror,
  runSweep,
  setProp,
  type NodeMirror,
} from "@pocketjs/framework/renderer";
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
import { connectSvc, type HostEvent } from "./svc.ts";
import { SAMPLE_DOC } from "./sample.ts";
import {
  perfEditCommit,
  perfInit,
  markitPhaseTimed,
  perfOpCounts,
  perfOpCalls,
  perfRecordGuestPhases,
  perfRecordMeasure,
  perfRecordSvcEvents,
  perfRecordVisible,
  perfRequest,
  perfTakeRequest,
  perfTickFrames,
  perfCounters,
} from "./perf.ts";

// Style enum values (contracts/spec/spec.ts ENUMS) — hardcoded here so the
// CF needs no framework module imports; the values are pinned by the spec.
const POS_ABSOLUTE = 1;
const OVERFLOW_HIDDEN = 1;
/** 18 px body font (slot 3 — 12/14/16/18/20/24/36), matches app.tsx. */
const FONT_SLOT = 3;
// The font-atlas baker selects slots from class literals in the source; the
// CF sets fontSlot via props, so this (unused) literal forces the slot-3
// atlas into the pak — the same atlas the baseline bundle bakes for "text-lg".
const BAKE_SLOT_3_LITERAL = "text-lg";
const LINE_H = 28;
const CARET_W = 2;
const CARET_H = LINE_H;
/** Line-node pool: covers a 1000x700 viewport (26 lines) plus headroom. */
const SLOTS = 32;
const DRAG_SLOP = 3;

const INK = {
  bg: "#ffffff",
  body: "#333333",
  caret: "#0000ff",
  sel: "#3311ff30",
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

interface Slot {
  view: NodeMirror;
  text: NodeMirror;
  /** Visible line index currently assigned, -1 = detached. */
  line: number;
  /** Text content last pushed to the native node. */
  cached: string;
}

export interface CfOptions {
  /** Render empty text nodes (isolates text/layout work from tree work). */
  noText: boolean;
}

export function bootCf(opts: CfOptions): void {
  perfInit();
  const host = detectHost();
  installHost(host);
  const ops = getOps();

  // ---- state (plain fields; the CF has no reactive layer) ---------------
  let doc = SAMPLE_DOC;
  let caret = 0;
  let anchor = 0;
  let scroll = 0;
  let vp = { w: 1000, h: 700 };
  let lineIndex: LineIndex = new LineIndex(SAMPLE_DOC);
  const svc = connectSvc();

  // ---- native tree --------------------------------------------------------
  // Root: the app layer, sized to the viewport (index.ts render()'s
  // appLayer: full-size, clipped). Inside it, the Focusable-equivalent
  // (app.tsx: class "relative flex-1 overflow-hidden" + bgColor) — same
  // two wrapper layers, so the DrawList command structure (nested
  // scissors + background quad) matches the Solid app exactly.
  const root = createElement("view");
  const rootStyle = () => ({
    width: vp.w,
    height: vp.h,
    overflow: OVERFLOW_HIDDEN,
  });
  setProp(root, "style", rootStyle());
  insertNode(rootMirror, root);
  const focusable = createElement("view");
  const focusableStyle = () => ({
    posType: POS_ABSOLUTE,
    insetL: 0,
    insetT: 0,
    width: vp.w,
    height: vp.h,
    overflow: OVERFLOW_HIDDEN,
    bgColor: INK.bg,
  });
  setProp(focusable, "style", focusableStyle());
  insertNode(root, focusable);

  // Content: the full-document-height layer translated by -scroll (app.tsx's
  // inner View; posType absolute).
  const content = createElement("view");
  const contentStyle = () => ({
    posType: POS_ABSOLUTE,
    insetL: 0,
    insetT: 0,
    width: vp.w,
    height: lineIndex.starts.length * LINE_H,
    translateY: -scroll,
  });
  setProp(content, "style", contentStyle());
  insertNode(focusable, content);

  // Line pool: SLOTS view+text pairs, attached to `content` on demand.
  const slots: Slot[] = [];
  for (let i = 0; i < SLOTS; i++) {
    const view = createElement("view");
    setProp(view, "style", {
      posType: POS_ABSOLUTE,
      insetL: 0,
      insetT: 0,
      height: LINE_H,
    });
    const text = createElement("text");
    setProp(text, "style", {
      posType: POS_ABSOLUTE,
      insetL: 0,
      insetT: 0,
      height: LINE_H,
      lineHeight: LINE_H,
      fontSlot: FONT_SLOT,
      textColor: INK.body,
    });
    insertNode(view, text);
    retain(view); // detached slots survive the end-of-frame sweep
    slots.push({ view, text, line: -1, cached: "" });
  }

  // Caret (app.tsx: Show when the selection is collapsed).
  const caretNode = createElement("view");
  const caretStyle = () => ({
    posType: POS_ABSOLUTE,
    width: CARET_W,
    insetL: caretX(doc, lineIndex.starts, caret, textWidth),
    insetT: caretRow(lineIndex.starts, caret) * LINE_H - scroll,
    height: CARET_H,
    bgColor: INK.caret,
  });
  setProp(caretNode, "style", caretStyle());
  insertNode(content, caretNode);
  retain(caretNode);

  // Selection rects: created on demand, pooled for reuse.
  const selPool: NodeMirror[] = [];
  function selRect(): NodeMirror {
    let n = selPool.pop();
    if (!n) {
      n = createElement("view");
      setProp(n, "style", { posType: POS_ABSOLUTE, insetT: 0, height: LINE_H, bgColor: INK.sel });
      retain(n);
    }
    return n;
  }
  const activeSel: NodeMirror[] = [];

  // ---- model helpers (parity with app.tsx) --------------------------------
  const selState = (): EditState => ({ doc, caret, anchor });
  function applyState(s: EditState, change?: EditChange | null) {
    markitPhaseTimed(
      () => {
        if (change) lineIndex.applyEdit(change.start, change.end, change.text);
      },
      (ms) => perfRecordGuestPhases(0, ms, 0),
    );
    markitPhaseTimed(() => {
      doc = s.doc;
      caret = s.caret;
      anchor = s.anchor;
    }, (ms) => perfRecordGuestPhases(0, 0, ms));
  }
  function mutate(f: (s: EditState) => EditResult) {
    const r = markitPhaseTimed(
      () => f(selState()),
      (ms) => perfRecordGuestPhases(ms, 0, 0),
    );
    applyState(r.state, r.change);
  }

  // ---- svc event handling (parity with app.tsx handleEvent) ----------------
  function handleEvent(ev: HostEvent) {
    switch (ev.t) {
      case "perfreq":
        perfRequest();
        break;
      case "hello":
      case "resize": {
        vp = { w: ev.w ?? 1000, h: ev.h ?? 700 };
        setProp(root, "style", rootStyle());
        scroll = Math.max(0, Math.min(maxScroll(), scroll));
        break;
      }
      case "load":
        lineIndex = new LineIndex(ev.text ?? "");
        doc = ev.text ?? "";
        caret = 0;
        anchor = 0;
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
        const x = ev.x ?? -1;
        const y = ev.y ?? -1;
        const down = ev.d ?? false;
        if (down) {
          const line = Math.floor((y + scroll) / LINE_H);
          const pos = caretFromX(doc, lineIndex.starts, line, x, textWidth);
          caret = pos;
          if (!(ev.sh ?? false)) anchor = pos;
        }
        break;
      }
      case "scroll":
        scroll = Math.max(0, Math.min(maxScroll(), scroll + (ev.dy ?? 0)));
        break;
      case "ime":
        break;
    }
  }

  function maxScroll(): number {
    return Math.max(0, lineIndex.starts.length * LINE_H - vp.h);
  }

  function handleKey(k: string, shift: boolean) {
    if (shift) {
      const extend = (pos: number) => {
        caret = Math.max(0, Math.min(pos, doc.length));
      };
      switch (k) {
        case "Left":
          extend(caret - 1);
          return;
        case "Right":
          extend(caret + 1);
          return;
        case "Home":
          extend(lineStart(lineIndex.starts, caret));
          return;
        case "End":
          extend(lineEnd(doc, lineIndex.starts, caret));
          return;
        case "Up":
        case "Down":
          extend(moveVertical(doc, lineIndex.starts, caret, k === "Up" ? -1 : 1, caretX(doc, lineIndex.starts, caret, textWidth), textWidth));
          return;
      }
    }
    if (k === "Escape") {
      anchor = caret;
      return;
    }
    if (k === "SelectAll") {
      caret = doc.length;
      anchor = 0;
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
          const at = lineStart(lineIndex.starts, s.caret);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "End":
        mutate((s) => {
          const at = lineEnd(s.doc, lineIndex.starts, s.caret);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "Up":
      case "Down":
        mutate((s) => {
          const at = moveVertical(s.doc, lineIndex.starts, s.caret, k === "Up" ? -1 : 1, caretX(s.doc, lineIndex.starts, s.caret, textWidth), textWidth);
          return { state: { doc: s.doc, caret: at, anchor: at }, change: null };
        });
        break;
      case "PageUp":
        scroll = Math.max(0, Math.min(maxScroll(), scroll - vp.h));
        break;
      case "PageDown":
        scroll = Math.max(0, Math.min(maxScroll(), scroll + vp.h));
        break;
    }
  }

  // ---- per-frame presentation update (the CF's "render") ------------------
  function updateVisible() {
    const starts = lineIndex.starts;
    const from = Math.max(0, Math.floor(scroll / LINE_H));
    const to = Math.min(starts.length, from + Math.ceil(vp.h / LINE_H) + 1);
    const count = to - from;
    perfRecordVisible(count);
    for (let k = 0; k < SLOTS; k++) {
      const slot = slots[k];
      const attached = slot.view.parent === content;
      if (k < count) {
        const line = from + k;
        const start = starts[line];
        const end = lineEnd(doc, starts, start);
        const text = opts.noText ? "" : doc.slice(start, end);
        // Insert before the caret node so the caret stays the LAST child
        // (the Solid app's JSX order: lines, then caret — the caret paints
        // over text, which the DrawList word stream also reflects).
        if (!attached) insertNode(content, slot.view, caretNode);
        if (!attached || text !== slot.cached) {
          replaceText(slot.text, text);
          slot.cached = text;
        }
        if (!attached || line !== slot.line) {
          setProp(slot.view, "style", {
            posType: POS_ABSOLUTE,
            insetL: 0,
            insetT: line * LINE_H,
            height: LINE_H,
          });
          slot.line = line;
        }
      } else if (attached) {
        detachNode(content, slot.view);
        slot.line = -1;
        slot.cached = "";
      }
    }
  }

  function updateCaret() {
    setProp(caretNode, "style", caretStyle());
  }

  function updateSelection() {
    // app.tsx parity: selection rects exist only while a selection spans a
    // visible line (inserted inside the line view, before its text — the
    // baseline's JSX order). The bench typing workload never selects, so
    // this is normally zero work.
    for (const n of activeSel.splice(0)) {
      detachNode(content, n);
      selPool.push(n);
    }
    if (caret === anchor) return;
    const [lo, hi] = selBounds({ doc, caret, anchor });
    const from = Math.max(0, Math.floor(scroll / LINE_H));
    const to = Math.min(lineIndex.starts.length, from + Math.ceil(vp.h / LINE_H) + 1);
    for (let line = from; line < to; line++) {
      const start = lineIndex.starts[line];
      const end = lineEnd(doc, lineIndex.starts, start);
      if (hi <= start || lo >= end) continue;
      const x0 = lo <= start ? 0 : textWidth(doc.slice(start, Math.min(lo, end)));
      const x1 = hi >= end ? textWidth(doc.slice(start, end)) : textWidth(doc.slice(start, Math.min(hi, end)));
      const rect = selRect();
      setProp(rect, "style", {
        posType: POS_ABSOLUTE,
        insetL: x0,
        insetT: 0,
        width: Math.max(2, x1 - x0),
        height: LINE_H,
        bgColor: INK.sel,
      });
      // Inside the line view, before its text child (baseline JSX order).
      const slot = slots[line - from];
      insertNode(slot.view, rect, slot.text);
      activeSel.push(rect);
    }
  }

  // ---- frame loop -----------------------------------------------------------
  installFrameHandler(() => {
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

    markitPhaseTimed(() => {
      setProp(content, "style", contentStyle());
      updateVisible();
      updateCaret();
      updateSelection();
    }, (ms) => perfRecordGuestPhases(0, 0, ms));

    if (perfTakeRequest()) {
      const c = perfCounters();
      svc.send({
        t: "perf",
        frames: c.frames,
        edits: c.edits,
        docChars: doc.length,
        docLines: lineIndex.starts.length,
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
        caret,
        anchor,
        scrollY: scroll,
        w: vp.w,
        h: vp.h,
        docHead: doc.slice(0, 64),
      });
      svc.send({
        t: "caret",
        x: Math.round(caretX(doc, lineIndex.starts, caret, textWidth)),
        y: Math.round(caretRow(lineIndex.starts, caret) * LINE_H - scroll),
        h: LINE_H,
      });
    }
    runSweep();
  });
}
