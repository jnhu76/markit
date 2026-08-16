// mvp/pocketjs/app/markdown.test.ts — BlockIndex + parseInline tests.
//
// Deterministic cases + randomized differential testing against the
// full-scan oracle (scanBlocksFull), the same discipline as
// line-index.test.ts (A3-P1): the incremental index must equal the
// reference index after every edit, and the rescan radius must stay
// local for non-structural edits.

import { describe, expect, test } from "bun:test";
import { BlockIndex, classifyLine, parseInline, scanBlocksFull } from "./markdown.ts";

function linesOf(doc: string): string[] {
  return doc.split("\n");
}

function lineTextOf(lines: string[]) {
  return (line: number) => lines[line] ?? "";
}

/** Apply a char-range replace to a doc (model-parity with editor.ts). */
function apply(doc: string, start: number, end: number, text: string): string {
  return doc.slice(0, start) + text + doc.slice(end);
}

/** Char offset of the start of `line` (0-based) in the doc. */
function lineStart(doc: string, line: number): number {
  let off = 0;
  for (let i = 0; i < line; i++) {
    off = doc.indexOf("\n", off) + 1;
  }
  return off;
}

describe("classifyLine", () => {
  test("L1 kinds", () => {
    expect(classifyLine("").kind).toBe("blank");
    expect(classifyLine("## title").kind).toBe("heading");
    expect(classifyLine("#### deep").level).toBe(4);
    expect(classifyLine("plain text").kind).toBe("para");
    expect(classifyLine("> quoted").kind).toBe("quote");
    expect(classifyLine("- item").kind).toBe("ulist");
    expect(classifyLine("* item").kind).toBe("ulist");
    expect(classifyLine("+ item").kind).toBe("ulist");
    expect(classifyLine("1. item").kind).toBe("olist");
    expect(classifyLine("42. item").kind).toBe("olist");
    expect(classifyLine("```").kind).toBe("fenced");
    expect(classifyLine("```js").kind).toBe("fenced");
    expect(classifyLine("a```").kind).toBe("para");
  });
});

describe("scanBlocksFull", () => {
  test("merges and splits", () => {
    const doc = "para one\npara two\n\n## head\n\n- a\n- b\n\n> q1\n> q2\n\n```\ncode\n```\n\nlast";
    const lines = linesOf(doc);
    const blocks = scanBlocksFull(lineTextOf(lines), lines.length);
    const kinds = blocks.map((b) => b.kind);
    expect(kinds).toEqual(["para", "blank", "heading", "blank", "ulist", "blank", "quote", "blank", "fenced", "blank", "para"]);
    const fence = blocks[8];
    expect(fence.startLine).toBe(11);
    expect(fence.endLine).toBe(13);
  });

  test("unclosed fence extends to EOF", () => {
    const doc = "```\ncode\nmore";
    const lines = linesOf(doc);
    const blocks = scanBlocksFull(lineTextOf(lines), lines.length);
    expect(blocks).toHaveLength(1);
    expect(blocks[0].kind).toBe("fenced");
    expect(blocks[0].endLine).toBe(2);
  });
});

describe("parseInline", () => {
  test("bold / em / code / link", () => {
    const runs = parseInline("a **bold** b *em* c `code` d [link](https://x.io) e");
    const styles = runs.map((r) => r.style);
    expect(styles).toContain("bold");
    expect(styles).toContain("em");
    expect(styles).toContain("code");
    expect(styles).toContain("link");
    // runs cover the whole text contiguously
    let pos = 0;
    for (const r of runs) {
      expect(r.start).toBe(pos);
      pos = r.end;
    }
    expect(pos).toBe("a **bold** b *em* c `code` d [link](https://x.io) e".length);
  });

  test("unclosed opener degrades to body", () => {
    const runs = parseInline("x **unclosed");
    expect(runs.every((r) => r.style === "body")).toBe(true);
  });

  test("no false em inside bold", () => {
    const runs = parseInline("**bold**");
    expect(runs.map((r) => r.style)).toEqual(["bold"]);
  });
});

describe("BlockIndex incremental vs oracle", () => {
  const SEED_DOC = [
    "## Title one",
    "",
    "A paragraph with **bold** and `code`.",
    "Second paragraph line.",
    "",
    "- item one",
    "- item two with *em*",
    "",
    "1. first",
    "2. second",
    "",
    "> quote line",
    "> quote two",
    "",
    "```",
    "let a = 1;",
    "let b = 2;",
    "```",
    "",
    "## Title two",
    "",
    "Final paragraph.",
    "",
  ].join("\n");

  function run(doc: string): { index: BlockIndex; lines: string[] } {
    const lines = linesOf(doc);
    return { index: new BlockIndex(lineTextOf(lines), lines.length), lines };
  }

  test("rebuild equals oracle", () => {
    const { index, lines } = run(SEED_DOC);
    expect(index.verify(lineTextOf(lines), lines.length)).toBe(true);
  });

  test("local edits stay local (radius)", () => {
    const { index, lines } = run(SEED_DOC);
    // M1: mid-paragraph char insert at line 2.
    const at = lineStart(SEED_DOC, 2) + 5;
    lines[2] = lines[2].slice(0, 5) + "x" + lines[2].slice(5);
    void at;
    const r = index.applyEdit(2, 2, lines.length, lineTextOf(lines));
    expect(r.stats.blocksReparsed).toBe(1);
    expect(r.stats.linesScanned).toBeLessThanOrEqual(2);
    expect(index.verify(lineTextOf(lines), lines.length)).toBe(true);
  });

  test("fence-open edit extends radius to the close (M5)", () => {
    const { index, lines } = run(SEED_DOC);
    // Break the fence-open line: "```" -> "a```" (line 14).
    lines[14] = "a" + lines[14];
    const r = index.applyEdit(14, 14, lines.length, lineTextOf(lines));
    expect(r.stats.linesScanned).toBeGreaterThan(3);
    expect(index.verify(lineTextOf(lines), lines.length)).toBe(true);
  });

  test("randomized differential vs oracle", () => {
    let doc = SEED_DOC;
    let { index, lines } = run(doc);
    let seed = 0xA4A4;
    const rnd = () => {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      return seed / 0x7fffffff;
    };
    for (let e = 0; e < 400; e++) {
      const pos = Math.floor(rnd() * (doc.length + 1));
      const len = Math.floor(rnd() * 5);
      const insert = rnd() < 0.5 ? "x" : rnd() < 0.25 ? "\n" : "";
      const newDoc = apply(doc, pos, pos + len, insert);
      // map the edit to a line range in the NEW doc
      const linesNew = linesOf(newDoc);
      const lineLo = linesNew.slice(0, newDoc.slice(0, pos).split("\n").length).length - 1;
      const lineHi = Math.max(
        lineLo,
        linesNew.slice(0, newDoc.slice(0, pos + len).split("\n").length).length - 1,
      );
      const res = index.applyEdit(Math.max(0, lineLo), Math.max(0, lineHi), linesNew.length, lineTextOf(linesNew));
      expect(res.stats.linesScanned).toBeGreaterThan(0);
      expect(index.verify(lineTextOf(linesNew), linesNew.length)).toBe(true);
      doc = newDoc;
      lines = linesNew;
    }
  });
});
