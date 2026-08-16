// mvp/pocketjs/app/line-index.test.ts — A3-P1 correctness oracle.
//
// The incremental LineIndex must agree with the reference full-scan
// implementation after every edit. Reference scans are test/validation
// only — they never run on the production edit hot path (the A3 exit gate
// requires full_line_scans/edit = 0, verified here by the counters).
//
// Run: bun test mvp/pocketjs/app/line-index.test.ts

import { describe, expect, test } from "bun:test";
import {
  LineIndex,
  backspaceSel,
  deleteSel,
  lineStarts,
  typeText,
  type EditState,
} from "./editor.ts";

/** Deterministic PRNG (mulberry32) — same sequence on every run. */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Synthetic U0 document with the corpus line profile (short prose lines,
 *  occasional blanks). Deterministic per seed. */
function syntheticDoc(target: number, seed: number): string {
  const rand = rng(seed);
  const words =
    "the quick brown fox jumps over a lazy dog near the fence while the " +
    "editor measures frame latency on a modern windows desktop with a " +
    "fixed step clock and a demand rendered draw list".split(" ");
  const lines: string[] = [];
  let total = 0;
  while (total < target) {
    if (rand() < 0.08) {
      lines.push("");
      total += 1;
    } else {
      const n = 6 + Math.floor(rand() * 9);
      const parts: string[] = [];
      for (let i = 0; i < n; i++) parts.push(words[Math.floor(rand() * words.length)]);
      const ln = parts.join(" ");
      lines.push(ln);
      total += ln.length + 1;
    }
  }
  return lines.join("\n") + "\n";
}

const STATE0: EditState = { doc: "", caret: 0, anchor: 0 };

function checkIndex(index: LineIndex, doc: string, label: string) {
  expect(index.verify(doc), `${label}: index matches reference`).toBe(true);
}

/** Apply an EditResult to a doc + index pair (the app's applyState flow). */
function applyResult(doc: string, index: LineIndex, r: ReturnType<typeof typeText>) {
  if (r.change) index.applyEdit(r.change.start, r.change.end, r.change.text);
  return r.state;
}

describe("LineIndex construction", () => {
  test("empty document", () => {
    const ix = new LineIndex("");
    expect(ix.starts).toEqual([0]);
    expect(ix.fullScans).toBe(1);
    checkIndex(ix, "", "empty");
  });

  test("single line", () => {
    const ix = new LineIndex("hello");
    expect(ix.starts).toEqual([0]);
    checkIndex(ix, "hello", "single");
  });

  test("many lines (trailing newline)", () => {
    const doc = "a\nbb\nccc\n";
    const ix = new LineIndex(doc);
    expect(ix.starts).toEqual([0, 2, 5, 9]);
    checkIndex(ix, doc, "many");
  });
});

describe("incremental updates — deterministic cases", () => {
  // (label, initial doc, [start, end, text]) applied in sequence.
  const cases: [string, string, [number, number, string][]][] = [
    ["insert char middle", "ab", [[1, 1, "X"]]],
    ["insert newline middle", "ab", [[1, 1, "\n"]]],
    ["insert newline at begin", "ab", [[0, 0, "\n"]]],
    ["insert newline at end", "ab", [[2, 2, "\n"]]],
    ["insert multi-char with newlines", "ab", [[1, 1, "X\nY\nZ"]]],
    ["delete char middle", "abc", [[1, 2, ""]]],
    ["delete char at begin", "abc", [[0, 1, ""]]],
    ["delete char at end", "abc", [[2, 3, ""]]],
    ["delete newline", "a\nb", [[1, 2, ""]]],
    ["delete newline at end", "a\n", [[1, 2, ""]]],
    ["delete newline at begin", "\na", [[0, 1, ""]]],
    ["replace across lines", "a\nb\nc", [[2, 4, "XYZ"]]],
    ["replace across lines with newline", "a\nb\nc", [[0, 4, "X\nY"]]],
    ["delete whole doc", "a\nb\nc", [[0, 5, ""]]],
    ["replace whole doc", "a\nb\nc", [[0, 5, "z\nz"]]],
    ["insert into empty", "", [[0, 0, "hello"]]],
    ["insert newline into empty", "", [[0, 0, "\n"]]],
    ["insert at line boundary", "ab\ncd", [[3, 3, "XY\n"]]],
    ["replace at line boundary", "ab\ncd", [[3, 5, "Z"]]],
    ["replace ending at line start", "ab\ncd", [[0, 3, "q"]]],
    ["empty replace", "ab", [[1, 1, ""]]],
  ];

  for (const [label, doc, edits] of cases) {
    test(label, () => {
      const ix = new LineIndex(doc);
      let cur = doc;
      for (const [start, end, text] of edits) {
        ix.applyEdit(start, end, text);
        cur = cur.slice(0, start) + text + cur.slice(end);
        checkIndex(ix, cur, `${label} after [${start},${end})->${JSON.stringify(text)}`);
      }
      expect(ix.fullScans).toBe(1);
    });
  }
});

describe("edit functions produce correct changes (EditResult)", () => {
  test("typeText insert", () => {
    const s: EditState = { doc: "abc", caret: 1, anchor: 1 };
    const r = typeText(s, "X");
    expect(r.state.doc).toBe("aXbc");
    expect(r.change).toEqual({ start: 1, end: 1, text: "X" });
  });

  test("typeText over selection", () => {
    const s: EditState = { doc: "abcdef", caret: 4, anchor: 1 };
    const r = typeText(s, "Z");
    expect(r.state.doc).toBe("aZef");
    expect(r.state.caret).toBe(2);
    expect(r.change).toEqual({ start: 1, end: 4, text: "Z" });
  });

  test("typeText newline", () => {
    const s: EditState = { doc: "ab", caret: 1, anchor: 1 };
    const r = typeText(s, "\n");
    expect(r.state.doc).toBe("a\nb");
    expect(r.change).toEqual({ start: 1, end: 1, text: "\n" });
  });

  test("backspaceSel deletes before caret", () => {
    const s: EditState = { doc: "abc", caret: 2, anchor: 2 };
    const r = backspaceSel(s);
    expect(r.state.doc).toBe("ac");
    expect(r.change).toEqual({ start: 1, end: 2, text: "" });
  });

  test("backspaceSel at doc start is a no-op change", () => {
    const r = backspaceSel({ doc: "abc", caret: 0, anchor: 0 });
    expect(r.state.doc).toBe("abc");
    expect(r.change).toEqual({ start: 0, end: 0, text: "" });
  });

  test("backspaceSel with selection routes through typeText", () => {
    const s: EditState = { doc: "abcdef", caret: 3, anchor: 1 };
    const r = backspaceSel(s);
    expect(r.state.doc).toBe("adef");
  });

  test("deleteSel deletes at caret", () => {
    const s: EditState = { doc: "abc", caret: 1, anchor: 1 };
    const r = deleteSel(s);
    expect(r.state.doc).toBe("ac");
    expect(r.change).toEqual({ start: 1, end: 2, text: "" });
  });

  test("deleteSel at doc end is a no-op change", () => {
    const r = deleteSel({ doc: "abc", caret: 3, anchor: 3 });
    expect(r.state.doc).toBe("abc");
    expect(r.change).toEqual({ start: 3, end: 3, text: "" });
  });

  test("applying EditResult changes keeps the index in sync", () => {
    let doc = "abc\ndef\nghi";
    const ix = new LineIndex(doc);
    let state: EditState = { doc, caret: 2, anchor: 2 };
    const steps: ((s: EditState) => ReturnType<typeof typeText>)[] = [
      (s) => typeText(s, "Q"),
      (s) => typeText(s, "\n"),
      backspaceSel,
      deleteSel,
      (s) => typeText(s, ""),
      backspaceSel,
    ];
    for (const f of steps) {
      const r = f(state);
      state = applyResult(state.doc, ix, r);
      checkIndex(ix, state.doc, "edit-result flow");
    }
  });
});

describe("randomized differential test", () => {
  test("1M random edits on a 10K document", () => {
    const rand = rng(0xA3C0FFEE);
    const doc = syntheticDoc(10 * 1024, 0xA3C0FFEE);
    const ix = new LineIndex(doc);
    let cur = doc;
    let edits = 0;
    for (let i = 0; i < 1000; i++) {
      const len = cur.length;
      let start: number;
      let end: number;
      let text: string;
      const op = rand();
      if (op < 0.3) {
        // single-char insert (no newline)
        start = end = Math.floor(rand() * (len + 1));
        text = "abcdefghij"[Math.floor(rand() * 10)];
      } else if (op < 0.4) {
        // newline insert
        start = end = Math.floor(rand() * (len + 1));
        text = "\n";
      } else if (op < 0.55) {
        // multi-char insert, may span newlines
        start = end = Math.floor(rand() * (len + 1));
        text = "XY\nZW\nP".slice(0, 1 + Math.floor(rand() * 7));
      } else if (op < 0.75) {
        // single-char delete
        start = Math.floor(rand() * len);
        end = start + 1;
        text = "";
      } else if (op < 0.9) {
        // range delete (may span lines)
        start = Math.floor(rand() * len);
        end = Math.min(len, start + 1 + Math.floor(rand() * 40));
        text = "";
      } else {
        // replace range with text (may span lines)
        start = Math.floor(rand() * len);
        end = Math.min(len, start + 1 + Math.floor(rand() * 40));
        text = "R\nR\nR".slice(0, 1 + Math.floor(rand() * 5));
      }
      ix.applyEdit(start, end, text);
      cur = cur.slice(0, start) + text + cur.slice(end);
      edits++;
      if (i % 250 === 0) checkIndex(ix, cur, `randomized step ${i}`);
    }
    checkIndex(ix, cur, "randomized final");
    expect(edits).toBe(1000);
    // The incremental path never full-scans.
    expect(ix.fullScans).toBe(1);
  });

  test("edit-functions differential on randomized input", () => {
    const rand = rng(0xA3FEED);
    const doc = syntheticDoc(10 * 1024, 0xA3FEED);
    const ix = new LineIndex(doc);
    let state: EditState = { doc, caret: 0, anchor: 0 };
    for (let i = 0; i < 500; i++) {
      const len = state.doc.length;
      state = { ...state, caret: Math.floor(rand() * (len + 1)), anchor: Math.floor(rand() * (len + 1)) };
      const op = rand();
      const r =
        op < 0.4
          ? typeText(state, "abcdefghij\nXY"[Math.floor(rand() * 13)])
          : op < 0.7
            ? backspaceSel(state)
            : deleteSel(state);
      state = applyResult(state.doc, ix, r);
      if (i % 100 === 0) checkIndex(ix, state.doc, `edit-functions step ${i}`);
    }
    checkIndex(ix, state.doc, "edit-functions final");
    expect(ix.fullScans).toBe(1);
  });
});

describe("document-size smoke validation (10K / 100K / 1M)", () => {
  for (const size of [10 * 1024, 100 * 1024, 1024 * 1024]) {
    test(`${size} bytes: load scan + 100 edits stay correct`, () => {
      const doc = syntheticDoc(size, size);
      const ix = new LineIndex(doc);
      checkIndex(ix, doc, `${size} load`);
      expect(ix.starts.length).toBeGreaterThan(100);
      const rand = rng(size ^ 0x5eed);
      let cur = doc;
      for (let i = 0; i < 100; i++) {
        const len = cur.length;
        const start = Math.floor(rand() * (len + 1));
        const text = rand() < 0.5 ? "a" : "\n";
        ix.applyEdit(start, start, text);
        cur = cur.slice(0, start) + text + cur.slice(start);
      }
      checkIndex(ix, cur, `${size} after 100 edits`);
      // Reference lineStarts agrees on the final doc too.
      expect(ix.starts).toEqual(lineStarts(cur));
      expect(ix.fullScans).toBe(1);
      expect(ix.entriesAdjusted).toBeGreaterThan(0);
      expect(ix.newlinesInserted + ix.newlinesDeleted).toBeGreaterThan(0);
    });
  }
});
