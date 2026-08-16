// mvp/pocketjs/app/svc.ts — Markit host ↔ guest protocol.
//
// The Markit desktop host (src/main.rs) is the guest's companion process:
// it forwards real keyboard/mouse/wheel/resize as JSON lines; the guest
// sends intents back. One poll per frame, per the HostOps contract. Hosts
// without the channel feature-detect to null and the app renders the seed
// read-only.
//
// host → guest lines:
//   {t:"hello", w, h}          logical viewport at boot
//   {t:"resize", w, h}         live window resize (relayout follows)
//   {t:"load", text}           document content (seed, or --file)
//   {t:"ch", s}                typed characters (batched)
//   {t:"key", k, sh}           named key: Backspace Delete Enter Tab Left
//                              Right Up Down Home End PageUp PageDown Escape
//                              SelectAll Copy Cut (Ctrl-chords arrive as
//                              named keys)
//   {t:"paste", text}          insert text (Ctrl+V — host reads clipboard;
//                              clipboard is DEFERRED on Phase A1)
//   {t:"ime", s, c}            IME composition (DEFERRED on Phase A1;
//                              protocol reserved)
//   {t:"mouse", x, y, d, sh}   pointer moved / pressed / released
//   {t:"scroll", dy}           wheel delta in logical px
//
// guest → host lines:
//   {t:"caret", x, y, h}       caret rect (logical px) — IME docking
//   {t:"quit"}                 close the window
//   {t:"state", caret, anchor, scrollY, docHead}  smoke/state echo
//   {t:"perf", ...}            Phase A2 counter dump (reply to "perfreq")

import { getOps } from "@pocketjs/framework";
import { perfRecordSvcSend } from "./perf.ts";

export interface HostEvent {
  t: "hello" | "resize" | "load" | "ch" | "key" | "mouse" | "scroll" | "paste" | "ime" | "perfreq";
  w?: number;
  h?: number;
  text?: string;
  s?: string;
  k?: string;
  x?: number;
  y?: number;
  /** Primary mouse button held ("mouse" events). */
  d?: boolean;
  /** Shift held — extends selections. */
  sh?: boolean;
  dy?: number;
  /** IME preedit caret (char index into s), null when composition ends. */
  c?: number | null;
}

export interface Svc {
  poll(): HostEvent[];
  send(
    line:
      | { t: "quit" }
      | { t: "caret"; x: number; y: number; h: number }
      | { t: "state"; caret: number; anchor: number; scrollY: number; w: number; h: number; docHead: string }
      | ({ t: "perf" } & Record<string, number>),
  ): void;
}

/** Probe the channel; null = standalone (no host on the other end). */
export function connectSvc(): Svc | null {
  const ops = getOps();
  if (!ops.svcOpen || !ops.svcPoll || !ops.svcSend || !ops.svcOpen("markit")) return null;
  const poll = ops.svcPoll.bind(ops);
  const send = ops.svcSend.bind(ops);
  return {
    poll() {
      const batch = poll();
      if (!batch) return [];
      const events: HostEvent[] = [];
      for (const line of batch.split("\n")) {
        if (line === "") continue;
        try {
          events.push(JSON.parse(line) as HostEvent);
        } catch {
          // A malformed line is a host bug; skip it rather than wedge.
        }
      }
      return events;
    },
    send(line) {
      perfRecordSvcSend();
      send(JSON.stringify(line));
    },
  };
}
