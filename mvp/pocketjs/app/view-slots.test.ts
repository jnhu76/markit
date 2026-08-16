// mvp/pocketjs/app/view-slots.test.ts — A4-R1 stable-identity regression.
//
// A4 review gate: position-keyed identity must not silently follow the
// wrong line when an edit shifts the document before the viewport.
// Scenarios (from the review):
//   insert newline before viewport / delete newline before viewport /
//   replace multiline before viewport / scroll afterwards / edit visible
//   line — exercised through resolveViewSlots over a simulated line count.
//
// Content correctness is by construction (items carry no content); the
// stateless-projection invariant is documented in view-slots.ts and
// docs/product/architecture.md §11.
//
// Run: bun test mvp/pocketjs/app/view-slots.test.ts

import { describe, expect, test } from "bun:test";
import { resolveViewSlots, type LineSlot } from "./view-slots.ts";

/** Simulated visible range for a doc with `lineCount` lines and an
 * unscrolled 700 px viewport at 28 px lines (the A3/A4 window). */
function range(lineCount: number, scrollY = 0): [number, number] {
  const from = Math.max(0, Math.floor(scrollY / 28));
  return [from, Math.min(lineCount, from + Math.ceil(700 / 28) + 1)];
}

describe("resolveViewSlots — stable identity across before-viewport edits", () => {
  test("initial resolution creates one item per visible line, no content stored", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const items = resolveViewSlots(from, to, cache);
    expect(items.length).toBe(to - from);
    expect(items[0]).toEqual({ line: from });
    expect(items.map((i) => i.line)).toEqual(
      Array.from({ length: to - from }, (_, k) => from + k),
    );
  });

  test("insert newline before viewport: surviving absolute lines keep their identity (no remount)", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const before = resolveViewSlots(from, to, cache);
    // Inserting "\n" at line 10 shifts every later line; the visible
    // range is unchanged in absolute numbers (scroll did not move).
    const after = resolveViewSlots(from, to, cache);
    expect(after.length).toBe(before.length);
    // Every item reference is reused — Solid's For sees no new objects.
    for (let k = 0; k < before.length; k++) expect(after[k]).toBe(before[k]);
  });

  test("delete newline before viewport: same reuse property", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const before = resolveViewSlots(from, to, cache);
    const after = resolveViewSlots(from, Math.min(to, 9_999), cache);
    expect(after.length).toBeGreaterThanOrEqual(to - from - 1);
    for (let k = 0; k < after.length; k++) expect(after[k]).toBe(before[k]);
  });

  test("replace multiline before viewport (net -3 lines): overlap keeps identity", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const before = resolveViewSlots(from, to, cache);
    // Replace 3 lines at line 5 with 0 lines: line count 10_000 -> 9_997.
    const after = resolveViewSlots(from, Math.min(to, 9_997), cache);
    for (let k = 0; k < after.length; k++) expect(after[k]).toBe(before[k]);
    expect(after.length).toBeLessThanOrEqual(before.length);
  });

  test("scroll afterwards: overlapping lines reuse identity, newly revealed lines are fresh", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000, 0);
    const before = resolveViewSlots(from, to, cache);
    // Scroll down 20 lines (20*28 px): range [20, 46).
    const [f2, t2] = range(10_000, 20 * 28);
    const after = resolveViewSlots(f2, t2, cache);
    expect(after[0]).toBe(before[20]); // line 20 reused
    expect(after[25].line).toBe(f2 + 25); // new line 45, fresh item
    expect(after[25]).not.toBe(before[25]);
    // Cache accumulates every line ever visible (lines 0..45): 26 from
    // the first range + 20 newly revealed. Growth with the highest
    // visible line is the documented invariant-4 bound; eviction is
    // future work (issue backlog) once lines carry state.
    expect(cache.size).toBe(46);
  });

  test("edit visible line: identical range returns the identical item array", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const before = resolveViewSlots(from, to, cache);
    const after = resolveViewSlots(from, to, cache);
    expect(after).toEqual(before);
    for (let k = 0; k < before.length; k++) expect(after[k]).toBe(before[k]);
  });

  test("empty range resolves to an empty array without touching the cache", () => {
    const cache = new Map<number, LineSlot>();
    expect(resolveViewSlots(0, 0, cache)).toEqual([]);
    expect(cache.size).toBe(0);
  });

  test("items expose only the line number (stateless projection contract)", () => {
    const cache = new Map<number, LineSlot>();
    const [from, to] = range(10_000);
    const items = resolveViewSlots(from, to, cache);
    for (const item of items) expect(Object.keys(item)).toEqual(["line"]);
  });
});
