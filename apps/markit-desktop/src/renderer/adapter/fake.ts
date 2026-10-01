// FAKE ADAPTER (UI track only — the integration slice deletes this file
// and switches the workbench to the real Rust bridge). It mirrors ONLY the
// already-frozen product semantics: document identity, monotonic
// revisions, explicit stale-base rejection, linear undo/redo as new
// commits, saved baseline / dirty relation, byte-offset coordinates, and
// the pinned/heading query shape. It invents no new domain authority.

import type {
  DocumentSession,
  DomainAdapter,
  DocumentView,
  EditResultView,
  OutlineView,
  SemanticStatusView,
} from "./types";

/** UTF-8 byte length of `text`. */
export function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

/** UTF-16 position (UI currency) -> UTF-8 byte offset (domain currency). */
export function utf16ToByte(text: string, utf16Pos: number): number {
  if (utf16Pos <= 0) return 0;
  if (utf16Pos >= text.length) return byteLength(text);
  return byteLength(text.slice(0, utf16Pos));
}

/** UTF-8 byte offset (domain currency) -> UTF-16 position (UI currency). */
export function byteToUtf16(text: string, bytePos: number): number {
  if (bytePos <= 0) return 0;
  if (bytePos >= byteLength(text)) return text.length;
  const bytes = new TextEncoder().encode(text);
  return new TextDecoder().decode(bytes.slice(0, bytePos)).length;
}

export interface FakeAdapterOptions {
  /**
   * Semantic-failure injection for deterministic UI testing: when set,
   * every semantic query reports failure with this message. The source is
   * never touched — provider failure cannot mutate documents.
   */
  injectSemanticFailure?: string;
}

export function createFakeAdapter(
  options: FakeAdapterOptions = {},
): DomainAdapter {
  return {
    async openFile() {
      const desktop = globalThis.window?.markitDesktop;
      if (!desktop) return null;
      return desktop.openFile();
    },

    async openScratch(source: string): Promise<DocumentSession> {
      const state: FakeDocState = {
        documentId: nextDocumentId++,
        source,
        revision: 0,
        saved: null,
        history: [],
        cursor: 0,
        outlines: new Map(),
        sources: new Map(),
      };
      state.outlines.set(0, { revision: 0, entries: scanHeadings(source) });
      state.sources.set(0, source);
      return fakeSession(state, options);
    },
  };
}

let nextDocumentId = 1;

interface FakeDocState {
  documentId: number;
  source: string;
  revision: number;
  saved: number | null;
  history: Array<{ pos: number; prev: string; next: string }>;
  cursor: number;
  /** Outline + source per published revision (the fake's minimum
   * retention; the real bridge pins engine state). */
  outlines: Map<number, OutlineView>;
  sources: Map<number, string>;
}

function statusOf(
  options: FakeAdapterOptions,
  revision: number,
): SemanticStatusView {
  if (options.injectSemanticFailure !== undefined) {
    return { kind: "failed", revision, message: options.injectSemanticFailure };
  }
  return { kind: "available", revision };
}

function fakeSession(
  state: FakeDocState,
  options: FakeAdapterOptions,
): DocumentSession {
  // Byte-currency splice: convert domain bytes to UTF-16 positions before
  // touching the JS string (String.slice counts units, not bytes).
  const applyReplacement = (
    base: string,
    range: { start: number; end: number },
    replacement: string,
  ): string =>
    base.slice(0, byteToUtf16(base, range.start)) +
    replacement +
    base.slice(byteToUtf16(base, range.end));

  /** One successful commit: new revision, pinned data published. */
  const commit = (source: string): number => {
    state.source = source;
    state.revision += 1;
    state.outlines.set(state.revision, {
      revision: state.revision,
      entries: scanHeadings(source),
    });
    state.sources.set(state.revision, source);
    return state.revision;
  };

  const view = (): DocumentView => ({
    documentId: state.documentId,
    revision: state.revision,
    source: state.source,
    dirty: state.saved !== state.revision,
  });

  return {
    handle: { documentId: state.documentId },

    async view() {
      return view();
    },

    async edit(
      baseRevision: number,
      range: { start: number; end: number },
      replacement: string,
    ): Promise<EditResultView> {
      if (baseRevision !== state.revision) {
        return {
          kind: "rejected",
          error: { kind: "staleBase", currentRevision: state.revision },
        };
      }
      const len = byteLength(state.source);
      if (range.start > range.end || range.end > len) {
        return { kind: "rejected", error: { kind: "invalidRange" } };
      }
      if (
        !isBoundary(state.source, range.start) ||
        !isBoundary(state.source, range.end)
      ) {
        return { kind: "rejected", error: { kind: "notCharBoundary" } };
      }
      const prevText = bytesToString(state.source, range.start, range.end);
      pushHistory(state, range.start, prevText, replacement);
      const revision = commit(
        applyReplacement(state.source, range, replacement),
      );
      return { kind: "committed", revision };
    },

    async undo() {
      if (state.cursor === 0) return null;
      const record = state.history[state.cursor - 1]!;
      state.cursor -= 1;
      const revision = commit(
        applyReplacement(state.source, {
          start: record.pos,
          end: record.pos + byteLength(record.next),
        }, record.prev),
      );
      return revision;
    },

    async redo() {
      if (state.cursor === state.history.length) return null;
      const record = state.history[state.cursor]!;
      state.cursor += 1;
      const revision = commit(
        applyReplacement(state.source, {
          start: record.pos,
          end: record.pos + byteLength(record.prev),
        }, record.next),
      );
      return revision;
    },

    async pin(revision: number): Promise<OutlineView | null> {
      return state.outlines.get(revision) ?? null;
    },

    async save(revision: number, path: string | null) {
      if (revision > state.revision) {
        return { kind: "rejected", error: { kind: "documentMismatch" } };
      }
      const bytes = stringToBytes(
        state.sources.get(revision) ?? state.source,
      );
      let target = path;
      if (!target) {
        const desktop = globalThis.window?.markitDesktop;
        if (!desktop) {
          return {
            kind: "rejected",
            error: { kind: "saveFailed", message: "no file dialog available" },
          };
        }
        const picked = await desktop.saveMarkdown(
          "untitled.md",
          bytesToBase64(bytes),
        );
        if (!picked) {
          return {
            kind: "rejected",
            error: { kind: "saveFailed", message: "save canceled" },
          };
        }
        target = picked;
      }
      // Dirty means current != saved: marking revision N saved never
      // cleans N+1.
      state.saved = revision;
      return { kind: "saved", savedRevision: revision, path: target };
    },

    async semanticStatus() {
      return statusOf(options, state.revision);
    },
  };
}

function pushHistory(
  state: FakeDocState,
  pos: number,
  prev: string,
  next: string,
): void {
  state.history = state.history.slice(0, state.cursor);
  state.history.push({ pos, prev, next });
  state.cursor += 1;
}

// --- byte-currency helpers (UTF-8 is the domain coordinate system) ---

function isBoundary(text: string, bytePos: number): boolean {
  if (bytePos === 0 || bytePos === byteLength(text)) return true;
  const bytes = new TextEncoder().encode(text);
  return (bytes[bytePos]! & 0xc0) !== 0x80;
}

function bytesToString(
  text: string,
  startByte: number,
  endByte: number,
): string {
  const bytes = new TextEncoder().encode(text);
  return new TextDecoder().decode(bytes.slice(startByte, endByte));
}

function stringToBytes(text: string): Uint8Array {
  return new TextEncoder().encode(text);
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/** ATX heading scan over one revision's source; byte-range bounds. */
export function scanHeadings(source: string): OutlineView["entries"] {
  const entries: OutlineView["entries"] = [];
  const bytes = new TextEncoder().encode(source);
  let lineStart = 0;
  for (let i = 0; i < bytes.length; i++) {
    if (bytes[i] !== 0x0a) continue;
    pushLine(bytes, lineStart, i + 1, entries);
    lineStart = i + 1;
  }
  if (lineStart < bytes.length) {
    pushLine(bytes, lineStart, bytes.length, entries);
  }
  return entries;
}

function pushLine(
  bytes: Uint8Array,
  start: number,
  endWithLf: number,
  entries: OutlineView["entries"],
): void {
  const line = new TextDecoder().decode(bytes.slice(start, endWithLf));
  const withoutLf = line.replace(/\n$/, "");
  const match = /^(#{1,6}) (.*)$/.exec(withoutLf);
  if (match) {
    entries.push({
      level: match[1]!.length,
      text: match[2]!.trim(),
      range: { start, end: endWithLf },
    });
  }
}
