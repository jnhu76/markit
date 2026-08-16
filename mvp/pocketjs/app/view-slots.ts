// mvp/pocketjs/app/view-slots.ts — A4-R1 stable visible-line identity.
//
// Visible-line items are STATELESS projections keyed by ABSOLUTE document
// line number. Solid's `For` reconciles by item-reference identity
// (`items[i] === newItems[i]` in the bundled mapArray), so reusing the
// same object for a line that stays visible avoids per-edit remounts
// (~90 native node creations per edit measured before this fix).
//
// Invariants (product rule — docs/product/architecture.md §11):
//   1. Items carry NO state: each component derives its slice from
//      doc()/starts() inside item-scoped memos, so a reused item shows
//      the CURRENT content of its absolute line number.
//   2. Absolute line numbers are NOT stable document identity: an edit
//      before the viewport (e.g. inserting "\n" at line 10) shifts every
//      later line, so the item for "line 100" is allowed to change what
//      it shows. Correctness comes from stateless re-derivation, never
//      from stored per-item state.
//   3. If a future line widget needs state (IME, folding, inline
//      widgets), identity must move to a stable block/content ID, not
//      the absolute line number.
//   4. The cache grows with the highest line number ever visible; bound
//      or evict it when lines gain state (issue backlog).

export interface LineSlot {
  /** Absolute document line number this slot projects. */
  line: number;
}

/**
 * Resolve the visible range [from, to) into cached line-slot items,
 * reusing the item for each absolute line number that already has one.
 * Pure: deterministic; the only mutation is filling the cache.
 */
export function resolveViewSlots(
  from: number,
  to: number,
  cache: Map<number, LineSlot>,
): LineSlot[] {
  const out: LineSlot[] = [];
  for (let i = from; i < to; i++) {
    let item = cache.get(i);
    if (!item) {
      item = { line: i };
      cache.set(i, item);
    }
    out.push(item);
  }
  return out;
}
