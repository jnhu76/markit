// mvp/pocketjs/app/app.tsx — Markit PocketJS thin-editor guest.
//
// The window IS the editor: a plain-text editing surface over the Markit
// flat widget host. No markdown, no chrome. Layout mirrors the GPUI Phase
// A0 prototype (Consolas 18 px, 28 px line height, no soft wrap, visible
// lines only) so both MVPs render the same corpus the same way; the seed
// document is app/sample.ts. The host feeds real keyboard/mouse/scroll/
// resize through the svc channel (app/svc.ts); without a host the bundle
// renders the seed read-only.

import { createMemo, createSignal, For, Show } from "solid-js";
import { Focusable, Text, View } from "@pocketjs/framework/components";
import { onFrame } from "@pocketjs/framework/lifecycle";
import { hitFocusable } from "@pocketjs/framework/input";
import { getOps, resizeViewport } from "@pocketjs/framework";
import {
  backspaceSel,
  caretFromX,
  caretRow,
  caretX,
  deleteSel,
  lineEnd,
  lineOf,
  lineStart,
  lineStarts,
  moveVertical,
  selBounds,
  typeText,
  type EditState,
} from "./editor.ts";
import { connectSvc, type HostEvent } from "./svc.ts";
import { SAMPLE_DOC } from "./sample.ts";
// Phase A2 instrumentation (Markit-owned): work counters + perfreq reply.
import {
  cfSkipScan,
  perfCounters,
  perfEditCommit,
  perfOpCalls,
  perfRecordMeasure,
  perfRecordSvcEvents,
  perfRecordSvcSend,
  perfRecordVisible,
  perfRequest,
  perfTakeRequest,
  perfTickFrames,
} from "./perf.ts";

/** 18 px body font (slot 3 — 12/14/16/18/20/24/36). */
const FONT_SLOT = 3;
/** Line height in logical px — mirrors GPUI's LINE_HEIGHT (28). */
const LINE_H = 28;
/** Cursor width px — mirrors GPUI's CURSOR_WIDTH (2). */
const CARET_W = 2;
/** Cursor height px — GPUI paints the caret full line height. */
const CARET_H = LINE_H;
/** PageUp/PageDown step — one viewport. */
const SCROLL_STEP = 56;
/** Pointer movement (logical px) that turns a press into a drag. */
const DRAG_SLOP = 3;

// GPUI Phase A0 palette: white surface, #333333 ink, blue caret,
// rgba(0x3311ff30) selection.
const INK = {
  bg: "#ffffff",
  body: "#333333",
  caret: "#0000ff",
  sel: "#3311ff30",
};

/** The measure injected into the caret math: body-font width in px. */
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

export default function Editor(): ReturnType<typeof View> {
  const svc = connectSvc();
  const [vp, setVp] = createSignal({ w: 1000, h: 700 });
  const [doc, setDoc] = createSignal(SAMPLE_DOC);
  const [caret, setCaret] = createSignal(0);
  const [anchor, setAnchor] = createSignal(0);
  const [scrollE, setScrollE] = createSignal(0);

  const starts = createMemo(() => {
    // DIAGNOSTIC (CF=1/3): return the load-time index without scanning —
    // document lines go stale. Not a valid product configuration.
    if (cfSkipScan()) {
      void doc();
      return startsCache;
    }
    const s = lineStarts(doc());
    startsCache = s;
    return s;
  });
  const totalH = () => starts().length * LINE_H;
  const maxScroll = () => Math.max(0, totalH() - vp().h);
  const viewH = () => vp().h;

  // Visible line range — GPUI's paint formula exactly: first line from
  // scroll, one viewport's worth plus one; nothing else is shaped or drawn.
  const visibleLines = createMemo(() => {
    const from = Math.max(0, Math.floor(scrollE() / LINE_H));
    const to = Math.min(starts().length, from + Math.ceil(viewH() / LINE_H) + 1);
    const out: { index: number; start: number; end: number }[] = [];
    for (let i = from; i < to; i++) out.push({ index: i, start: starts()[i], end: lineEnd(doc(), starts(), starts()[i]) });
    perfRecordVisible(to - from);
    return out;
  });

  const caretPx = () => caretX(doc(), starts(), caret(), textWidth);
  const caretRow = () => caretRowOf(starts(), caret());
  /** Normalized selection bounds, null when collapsed. */
  const editSel = (): [number, number] | null => {
    if (caret() === anchor()) return null;
    return selBounds({ doc: doc(), caret: caret(), anchor: anchor() });
  };
  /** Selection highlight rect for one display line, null outside. */
  const lineSelRect = (line: { index: number; start: number; end: number }) => {
    const sel = editSel();
    if (!sel) return null;
    const ls = line.start;
    const le = line.end;
    if (sel[1] <= ls || sel[0] >= le) return null;
    const x0 = sel[0] <= ls ? 0 : textWidth(doc().slice(ls, Math.min(sel[0], le)));
    const x1 = sel[1] >= le ? textWidth(doc().slice(ls, le)) : textWidth(doc().slice(ls, Math.min(sel[1], le)));
    return { x0, x1 };
  };

  const selState = (): EditState => ({ doc: doc(), caret: caret(), anchor: anchor() });
  const applyState = (s: EditState) => {
    setDoc(s.doc);
    setCaret(s.caret);
    setAnchor(s.anchor);
  };
  const mutate = (f: (s: EditState) => EditState) => {
    applyState(f(selState()));
  };

  const handleKey = (k: string, shift = false) => {
    // Shift + navigation extends the selection: the caret moves, the
    // anchor holds.
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
        case "Down": {
          extend(moveVertical(doc(), starts(), caret(), k === "Up" ? -1 : 1, caretPx(), textWidth));
          return;
        }
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
        mutate((s) => {
          const at = Math.max(0, s.caret - 1);
          return { doc: s.doc, caret: at, anchor: at };
        });
        break;
      case "Right":
        mutate((s) => {
          const at = Math.min(s.doc.length, s.caret + 1);
          return { doc: s.doc, caret: at, anchor: at };
        });
        break;
      case "Home":
        mutate((s) => {
          const at = lineStart(starts(), s.caret);
          return { doc: s.doc, caret: at, anchor: at };
        });
        break;
      case "End":
        mutate((s) => {
          const at = lineEnd(s.doc, starts(), s.caret);
          return { doc: s.doc, caret: at, anchor: at };
        });
        break;
      case "Up":
      case "Down":
        mutate((s) => {
          const at = moveVertical(s.doc, starts(), s.caret, k === "Up" ? -1 : 1, caretPx(), textWidth);
          return { doc: s.doc, caret: at, anchor: at };
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
        setDoc(ev.text ?? "");
        setCaret(0);
        setAnchor(0);
        // DIAGNOSTIC (CF=1/3): the index is snapshotted at load; edits
        // afterwards keep it stale (no per-edit scan).
        startsCache = lineStarts(ev.text ?? "");
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
        // IME composition is DEFERRED on Phase A1 (protocol reserved).
        break;
    }
  };

  // ---- pointer gestures over the content (svc mouse stream) --------------
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
    // Shift-click keeps the anchor: the span to the clicked point
    // becomes the selection.
    if (!shift) setAnchor(pos);
  };
  const pointerMove = (x: number, y: number) => {
    if (!press) return;
    if (!press.dragged && Math.abs(x - press.x) + Math.abs(y - press.y) < DRAG_SLOP) return;
    press.dragged = true;
    setCaret(editPosAt(x, y)); // anchor stays: the selection
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
      // Phase A2: one counter dump per request (host prints it).
      const c = perfCounters();
      svc.send({
        t: "perf",
        frames: c.frames,
        edits: c.edits,
        lineStartsScans: c.lineStartsScans,
        lineStartsChars: c.lineStartsChars,
        lineStartsMs: c.lineStartsMs,
        typeCopies: c.typeCopies,
        typeCopyChars: c.typeCopyChars,
        typeMs: c.typeMs,
        visibleVisits: c.visibleVisits,
        visibleLines: c.visibleLines,
        measures: c.measures,
        measureChars: c.measureChars,
        svcEvents: c.svcEvents,
        svcSends: c.svcSends,
        ops: perfOpCalls(),
        editsRing: JSON.stringify(c.editsRing),
      });
    }
    if (events.length > 0) {
      // State echo for the host's smoke driver (ignored otherwise).
      svc.send({
        t: "state",
        caret: caret(),
        anchor: anchor(),
        scrollY: scrollE(),
        w: vp().w,
        h: vp().h,
        docHead: doc().slice(0, 64),
      });
      // Caret rect (logical px) — host docks IME candidates here
      // (deferred Phase A1, but the wire contract is live).
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
          {(line) => (
            <View class="absolute" style={{ insetL: 0, insetT: line.index * LINE_H, height: LINE_H }}>
              <Show when={lineSelRect(line) != null}>
                <View
                  class="absolute"
                  style={{
                    insetL: lineSelRect(line)?.x0 ?? 0,
                    insetT: 0,
                    width: Math.max(2, (lineSelRect(line)?.x1 ?? 0) - (lineSelRect(line)?.x0 ?? 0)),
                    height: LINE_H,
                    bgColor: INK.sel,
                  }}
                />
              </Show>
              <Text
                class="absolute text-lg"
                style={{
                  insetL: 0,
                  insetT: 0,
                  height: LINE_H,
                  lineHeight: LINE_H,
                  textColor: INK.body,
                }}
              >
                {doc().slice(line.start, line.end)}
              </Text>
            </View>
          )}
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

// Local helpers kept out of the component body for clarity.

/** DIAGNOSTIC (CF=1/3): load-time line index cache (never updated by edits). */
let startsCache: number[] = [0];

function caretRowOf(starts: number[], caret: number): number {
  return lineOf(starts, caret).line;
}
