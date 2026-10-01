// Fake-adapter semantics tests: the fake mirrors the frozen document
// semantics (monotonic revisions, stale-base rejection, undo/redo as new
// commits, saved/dirty relation, byte coordinates over CJK/emoji) so the
// UI track exercises exactly the contract the Rust domain defines.

import { describe, expect, it } from "vitest";
import {
  byteLength,
  createFakeAdapter,
  scanHeadings,
  utf16ToByte,
} from "../src/renderer/adapter/fake";

const WELCOME = "# 欢迎使用\n\n## 世界 🌍\n";

function range(text: string, start: number, end: number) {
  return { start, end };
}

describe("fake adapter document semantics", () => {
  it("revision starts at 0 and every commit advances it exactly once", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch(WELCOME);
    const view = await session.view();
    expect(view.revision).toBe(0);
    expect(view.dirty).toBe(true);

    const len = byteLength(WELCOME);
    const result = await session.edit(0, range(WELCOME, len - 1, len), "x\n");
    expect(result).toEqual({ kind: "committed", revision: 1 });
  });

  it("stale base revision is rejected explicitly and nothing mutates", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch(WELCOME);
    const len = byteLength(WELCOME);
    await session.edit(0, range(WELCOME, len - 1, len), "\n");
    const before = await session.view();

    const rejected = await session.edit(0, range(WELCOME, 0, 0), "# Stale\n");
    expect(rejected).toEqual({
      kind: "rejected",
      error: { kind: "staleBase", currentRevision: 1 },
    });
    expect(await session.view()).toEqual(before);
  });

  it("edits are byte-range based and CJK/emoji survive", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch(WELCOME);
    // replace the emoji line's heading text: find "# 世界 🌍" text bytes
    const source = (await session.view()).source;
    const heading = "## 世界 🌍";
    const at = source.indexOf(heading);
    expect(at).toBeGreaterThanOrEqual(0);
    // byte offset of the line start: the first line is "# 欢迎使用\n"
    const firstLineBytes = byteLength("# 欢迎使用\n\n");
    const result = await session.edit(
      0,
      range(source, firstLineBytes, firstLineBytes + byteLength(heading)),
      "## 地球 🗺️",
    );
    expect(result).toEqual({ kind: "committed", revision: 1 });
    expect((await session.view()).source).toBe("# 欢迎使用\n\n## 地球 🗺️\n");
  });

  it("undo and redo are new commits that restore content", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch("# A\n");
    // replace the heading character A (bytes [2,3)) with B
    await session.edit(0, range("# A\n", 2, 3), "B");
    expect((await session.view()).source).toBe("# B\n");

    expect(await session.undo()).toBe(2);
    expect((await session.view()).source).toBe("# A\n");
    expect(await session.redo()).toBe(3);
    expect((await session.view()).source).toBe("# B\n");
    expect(await session.undo()).toBe(4);
    expect((await session.view()).source).toBe("# A\n");
    expect(await session.undo()).toBeNull();
  });

  it("save marks exactly the saved revision; later edits leave it dirty", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch("# A\n");
    const saved = await session.save(0, "/tmp/x.md");
    expect(saved).toEqual({ kind: "saved", savedRevision: 0, path: "/tmp/x.md" });
    expect((await session.view()).dirty).toBe(false);

    await session.edit(0, range("# A\n", 2, 3), "B");
    expect((await session.view()).dirty).toBe(true);
    // a late save of the OLD revision never cleans the new one
    await session.save(0, "/tmp/x.md");
    expect((await session.view()).dirty).toBe(true);
  });

  it("pinned outlines stay at their revision", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch(WELCOME);
    const pinned = await session.pin(0);
    expect(pinned?.revision).toBe(0);
    expect(pinned?.entries.map((e) => e.text)).toEqual(["欢迎使用", "世界 🌍"]);

    const len = byteLength(WELCOME);
    await session.edit(0, range(WELCOME, len - 1, len), "\n### 新的\n");
    const fresh = await session.pin(1);
    expect(fresh?.entries.map((e) => e.text)).toEqual(["欢迎使用", "世界 🌍", "新的"]);
    expect((await session.pin(0))?.entries).toHaveLength(2);
    expect(await session.pin(99)).toBeNull();
  });

  it("semantic failure injection never mutates the source", async () => {
    const adapter = createFakeAdapter({ injectSemanticFailure: "H4 unavailable" });
    const session = await adapter.openScratch(WELCOME);
    expect(await session.semanticStatus()).toEqual({
      kind: "failed",
      revision: 0,
      message: "H4 unavailable",
    });
    expect((await session.view()).source).toBe(WELCOME);
  });
});

describe("byte coordinate helpers", () => {
  it("utf16 positions convert to UTF-8 byte offsets over CJK and emoji", () => {
    const text = "# 世界 🌍 x";
    // "# " = 2 bytes; 世/界 3 bytes each; " " 1; emoji 4
    expect(utf16ToByte(text, 0)).toBe(0);
    expect(utf16ToByte(text, 2)).toBe(2); // before 世
    expect(utf16ToByte(text, 3)).toBe(5); // after 世
    expect(utf16ToByte(text, 4)).toBe(8); // after 界
    expect(utf16ToByte(text, 5)).toBe(9); // before emoji
    expect(utf16ToByte(text, 7)).toBe(13); // after emoji (surrogate pair = 2 units)
    expect(utf16ToByte(text, text.length)).toBe(byteLength(text));
  });

  it("mid-codepoint edits are rejected (notCharBoundary)", async () => {
    const adapter = createFakeAdapter();
    const session = await adapter.openScratch("# 世\n");
    const bad = byteLength("# ") + 1; // inside 世
    const result = await session.edit(0, range("# 世\n", bad, bad + 1), "x");
    expect(result).toEqual({
      kind: "rejected",
      error: { kind: "notCharBoundary" },
    });
  });
});

describe("heading scan", () => {
  it("scans ATX headings with byte ranges", () => {
    const source = "# One\n\ntext\n\n## Two 2\n";
    const entries = scanHeadings(source);
    expect(entries).toEqual([
      { level: 1, text: "One", range: { start: 0, end: 6 } },
      { level: 2, text: "Two 2", range: { start: 13, end: 22 } },
    ]);
  });
});
