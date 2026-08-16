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
  LineIndex,
  backspaceSel,
  caretFromX,
  caretRow,
  caretX,
  deleteSel,
  lineEnd,
  lineOf,
  lineStart,
  moveVertical,
  selBounds,
  typeText,
  type EditChange,
  type EditState,
} from "./editor.ts";
import { resolveViewSlots, type LineSlot } from "./view-slots.ts";
import { connectSvc, type HostEvent } from "./svc.ts";
import { SAMPLE_DOC } from "./sample.ts";
// Phase A2/A3 instrumentation (Markit-owned): work counters + perfreq reply.
import {
  perfCounters,
  perfEditCommit,
  perfOpCalls,
  perfOpCounts,
  perfRecordGuestPhases,
  perfRecordMeasure,
  perfRecordSvcEvents,
  perfRecordSvcSend,
  perfRecordVisible,
  perfRequest,
  perfTakeRequest,
  perfTickFrames,
  markitPhaseTimed,
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
    // A3-P1: the index is maintained incrementally by applyState (and
    // rebuilt once at load). This memo only makes the index reactive to
    // document changes — no document work per edit.
    void doc();
    return lineIndex.starts;
  });
  const totalH = () => starts().length * LINE_H;
  const maxScroll = () => Math.max(0, totalH() - vp().h);
  const viewH = () => vp().h;

  // Visible line range — GPUI's paint formula exactly: first line from
  // scroll, one viewport's worth plus one; nothing else is shaped or drawn.
  // A4-R1: items carry ONLY the stable absolute line number, cached by
  // position, so Solid's For reconciliation (item-reference identity —
  // this Solid version's mapArray matches `items[i] === newItems[i]`)
  // reuses every component while the document shifts under it; the line's
  // start/end and text are derived inside the item from doc()/starts().
  // Without the cache, fresh per-render objects made mapArray re-mount
  // all 26 line components per edit — ~90 native node creations per edit,
  // the dominant term of the measured Solid reconstruction cost. Items
  // are stateless projections (view-slots.ts): identity is the absolute
  // line number; content is always re-derived.
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
  /** Normalized selection bounds, null when collapsed. */
  const editSel = (): [number, number] | null => {
    if (caret() === anchor()) return null;
    return selBounds({ doc: doc(), caret: caret(), anchor: anchor() });
  };
  /** Selection highlight rect for one display line, null outside. */
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
    // Explicit changed-range propagation: the incremental line index is
    // updated locally before the document signal flips (A3-P1).
    markitPhaseTimed(
      () => {
        if (change) lineIndex.applyEdit(change.start, change.end, change.text);
      },
      (ms) => perfRecordGuestPhases(0, ms, 0),
    );
    // The Solid synchronous re-render runs inside these signal writes —
    // this phase is the guest-side measure of reactive reconstruction
    // (A4-R1, Date.now() ms, coarse).
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
        // A3-P1: rebuild the index BEFORE the doc signal flips — the memo
        // re-evaluates synchronously on setDoc and must see the matching
        // index, or the last visible line's slice extends to the document
        // end (an O(doc) text run in the retained tree, shaped every draw).
        lineIndex = new LineIndex(ev.text ?? "");
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
          {(item) => {
            // A4-R1: the item carries only the stable line number; every
            // doc-dependent read lives in item-scoped memos, so the item
            // component itself never re-runs on a document change (its
            // children getter used to read doc() directly, which
            // re-mounted all 26 line components per edit). The memos
            // re-evaluate (26× per edit) but the components and their
            // native nodes are created once.
            const line = item.line;
            const sel = createMemo(() => lineSelRect(line));
            const text = createMemo(() => {
              const start = starts()[line];
              const end = lineEnd(doc(), starts(), start);
              return doc().slice(start, end);
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
                  {text()}
                </Text>
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

// Local helpers kept out of the component body for clarity.

/** A3-P1: the incremental line index. Built once at load (one full scan);
 *  edits update it locally via applyState — full scans never run on the
 *  edit hot path. */
let lineIndex: LineIndex = new LineIndex(SAMPLE_DOC);

function caretRowOf(starts: number[], caret: number): number {
  return lineOf(starts, caret).line;
}
