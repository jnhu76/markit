// mvp/pocketjs/app/markdown.ts — A4-R2 incremental Markdown L1 structure.
//
// Framework-free Markdown L1 pipeline pieces for the Markit guest:
//
//   Document → Block Index → Incremental Parse → Affected Blocks
//            → Styled Runs → Visible Layout → DrawList
//
// This module owns the STRUCTURE stages (block boundaries, kinds,
// incremental rescan) plus the inline run parser. It deliberately
// implements only the Phase A4 L1 subset (A4 spec §8):
//
//   heading, paragraph, bold, emphasis, inline code, link, blockquote,
//   unordered/ordered list, fenced code block
//
// and nothing else (no tables/images/math/HTML/frontmatter/nesting).
//
// ## Block model (L1 simplification)
//
// - A blank line ends the current block; blank lines are their own
//   "blank" blocks.
// - Consecutive paragraph / quote / list-item lines merge into one block
//   of that kind. Every ATX heading line is its own block.
// - A fenced block = open fence line + content lines + close fence line
//   (or EOF). Content inside a fence is code — no inline parse. A "```"
//   line OUTSIDE a fence opens a fence (CommonMark behavior; a stray
//   close is therefore an open, which is what makes M5 structural edits
//   invalidate forward until the next fence boundary).
// - Loose lists (blank between items) split; nesting is out of scope.
//
// ## Incremental rescan
//
// `applyEdit` re-classifies lines from the start of the first affected
// block FORWARD, and stops at the first line that is a block boundary in
// BOTH the old and new structure with the same kind (the "stable point").
// The consumed range is the structural invalidation radius, measured by
// RescanStats. `scanBlocksFull` is the load-time and test-oracle path; it
// never runs per edit in production.

export type BlockKind =
  | "para"
  | "heading"
  | "quote"
  | "ulist"
  | "olist"
  | "fenced"
  | "blank";

export interface Block {
  kind: BlockKind;
  /** ATX heading level (heading blocks only). */
  level: number;
  /** First line index (inclusive). */
  startLine: number;
  /** Last line index (inclusive). */
  endLine: number;
}

/** Inline run style (L1). Syntax characters stay inside the run — L1
 *  keeps Markdown source visible (Typora-style hiding is L2). */
export type RunStyle = "body" | "bold" | "em" | "code" | "link";

/** A styled run over a block's text; offsets are code units relative to
 *  the block's first line start. */
export interface Run {
  start: number;
  end: number;
  style: RunStyle;
}

/** Lines examined / blocks created by the last incremental rescan. */
export interface RescanStats {
  linesScanned: number;
  blocksReparsed: number;
}

export type LineText = (line: number) => string;

/** Classify one line's L1 block kind (fence state is the caller's job). */
export function classifyLine(text: string): { kind: BlockKind; level: number } {
  if (text === "") return { kind: "blank", level: 0 };
  if (text.startsWith("```")) return { kind: "fenced", level: 0 };
  const h = /^(#{1,6}) /.exec(text);
  if (h) return { kind: "heading", level: h[1].length };
  if (text.startsWith(">")) return { kind: "quote", level: 0 };
  if (/^[-*+] /.test(text)) return { kind: "ulist", level: 0 };
  if (/^\d+\. /.test(text)) return { kind: "olist", level: 0 };
  return { kind: "para", level: 0 };
}

/** Full-document block scan — load-time and test oracle. */
export function scanBlocksFull(lines: LineText, lineCount: number): Block[] {
  const blocks: Block[] = [];
  let i = 0;
  while (i < lineCount) {
    const cls = classifyLine(lines(i));
    if (cls.kind === "blank") {
      blocks.push({ kind: "blank", level: 0, startLine: i, endLine: i });
      i++;
      continue;
    }
    if (cls.kind === "fenced") {
      // Open fence: consume content until a close fence or EOF.
      let j = i + 1;
      while (j < lineCount && lines(j) !== "```") j++;
      const end = j < lineCount ? j : lineCount - 1;
      blocks.push({ kind: "fenced", level: 0, startLine: i, endLine: end });
      i = end + 1;
      continue;
    }
    if (cls.kind === "heading") {
      blocks.push({ kind: "heading", level: cls.level, startLine: i, endLine: i });
      i++;
      continue;
    }
    // para/quote/ulist/olist: merge consecutive same-kind lines.
    let j = i + 1;
    while (j < lineCount && classifyLine(lines(j)).kind === cls.kind) j++;
    blocks.push({ kind: cls.kind, level: cls.level, startLine: i, endLine: j - 1 });
    i = j;
  }
  return blocks;
}

/**
 * Incremental block index. Maintained locally across edits like the
 * LineIndex (A3-P1): one full scan at rebuild, then per-edit rescans that
 * stop at the first stable boundary. The rescan radius IS the structural
 * invalidation radius R2 measures.
 */
export class BlockIndex {
  /** All lines covered; block boundaries are exactly the list boundaries. */
  blocks: Block[];
  /** Lines examined by the last applyEdit rescan. */
  lastScanned: number;
  /** Blocks created by the last applyEdit rescan. */
  lastReparsed: number;

  constructor(lines: LineText, lineCount: number) {
    this.blocks = scanBlocksFull(lines, lineCount);
    this.lastScanned = 0;
    this.lastReparsed = 0;
  }

  /** (index, block) of the block containing `line` (clamped). */
  blockAt(line: number): { index: number; block: Block } {
    const n = this.blocks.length;
    const lastEnd = n > 0 ? this.blocks[n - 1].endLine : 0;
    const target = Math.max(0, Math.min(line, lastEnd));
    let lo = 0;
    let hi = n - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (this.blocks[mid].startLine <= target) lo = mid;
      else hi = mid - 1;
    }
    return { index: lo, block: this.blocks[lo] };
  }

  /**
   * Incremental rescan for: lines [startLine, endLine] changed.
   * `lineCount` is the CURRENT line count (the caller's LineIndex owns it
   * — the old block list may cover fewer or more lines than the new doc).
   * `lines` reads CURRENT line texts (after the edit). Returns the stats
   * and the replaced block index range [lo, hi] in the OLD list, so the
   * caller can invalidate derived state (e.g. cached inline runs).
   */
  applyEdit(
    startLine: number,
    endLine: number,
    lineCount: number,
    lines: LineText,
  ): { stats: RescanStats; lo: number; hi: number; replacedStartLines: number[] } {
    const n = this.blocks.length;
    if (n === 0) {
      this.blocks = scanBlocksFull(lines, lineCount);
      this.lastScanned = lineCount;
      this.lastReparsed = this.blocks.length;
      return {
        stats: { linesScanned: lineCount, blocksReparsed: this.blocks.length },
        lo: 0,
        hi: 0,
        replacedStartLines: [],
      };
    }
    let lo = this.blockAt(startLine).index;
    let fromLine = this.blocks[lo].startLine;

    // A kind change at the edit can open a merge with the PRECEDING block
    // (e.g. a list item line becoming a paragraph that then merges with
    // the paragraph above). Extend the rescan start backward while the
    // previous block is mergeable and its kind matches the new
    // classification of the first line.
    while (lo > 0) {
      const prev = this.blocks[lo - 1];
      const k = prev.kind;
      if (k !== "para" && k !== "quote" && k !== "ulist" && k !== "olist") break;
      if (classifyLine(lines(fromLine)).kind !== k) break;
      lo -= 1;
      fromLine = prev.startLine;
    }

    const newBlocks: Block[] = [];
    let i = fromLine;
    let scanned = 0;

    // Line-count delta between the new and old documents: lines beyond
    // the edit region shift by this much, so old-block line numbers must
    // be translated before comparing with the new structure.
    const oldLineCount = this.blocks[n - 1].endLine + 1;
    const delta = lineCount - oldLineCount;
    const oldLastEnd = oldLineCount - 1;

    // Stable point: line `at` (new numbering) corresponds to old line
    // `at - delta`, which must start a block in the OLD structure with the
    // same kind as the NEW classification (then the old structure from
    // `at` on is still valid — its interior lines are unchanged, and a
    // fenced block's close lies among unchanged lines). `at` must also be
    // strictly beyond the last edited line — a block the edit still
    // reaches into can never be a stable point. EOF is always stable; an
    // open fence never is (the scan must reach its close).
    const isStable = (at: number, fenceOpen: boolean): boolean => {
      if (at > endLine && at >= lineCount) return true;
      if (fenceOpen || at <= endLine) return false;
      const oldLine = at - delta;
      if (oldLine < 0 || oldLine > oldLastEnd) return false;
      const ob = this.blocks[this.blockAt(oldLine).index];
      // The old block must lie entirely beyond the edited lines (old
      // numbering) — a block the edit still reaches into has changed
      // content and can never be a stable point.
      if (ob.startLine <= endLine) return false;
      if (ob.startLine + delta !== at) return false;
      return classifyLine(lines(at)).kind === ob.kind;
    };

    while (i < lineCount) {
      const cls = classifyLine(lines(i));
      scanned++;
      if (cls.kind === "fenced") {
        // Open fence (a "```" line outside a fence always opens one):
        // consume content until a close fence or EOF.
        let j = i + 1;
        while (j < lineCount && lines(j) !== "```") {
          j++;
          scanned++;
        }
        const end = j < lineCount ? j : lineCount - 1;
        newBlocks.push({ kind: "fenced", level: 0, startLine: i, endLine: end });
        i = end + 1;
        if (isStable(i, false)) break;
        continue;
      }
      if (cls.kind === "blank") {
        newBlocks.push({ kind: "blank", level: 0, startLine: i, endLine: i });
        i++;
        if (isStable(i, false)) break;
        continue;
      }
      if (cls.kind === "heading") {
        newBlocks.push({ kind: "heading", level: cls.level, startLine: i, endLine: i });
        i++;
        if (isStable(i, false)) break;
        continue;
      }
      // para/quote/ulist/olist: merge consecutive same-kind lines.
      let j = i + 1;
      while (j < lineCount && classifyLine(lines(j)).kind === cls.kind) {
        j++;
        scanned++;
      }
      newBlocks.push({ kind: cls.kind, level: cls.level, startLine: i, endLine: j - 1 });
      i = j;
      if (isStable(i, false)) break;
    }

    // Splice: replace the old blocks overlapping the consumed range
    // (their new-numbered start lines lie at or before the last new
    // block's end). The scan stopped at a stable point, so every kept
    // old block starts exactly at `lastNewEnd + 1` in the new numbering.
    const lastNewEnd = newBlocks.length > 0
      ? newBlocks[newBlocks.length - 1].endLine
      : fromLine - 1;
    let hi = lo;
    while (hi < n - 1 && this.blocks[hi + 1].startLine + delta <= lastNewEnd) hi++;
    const replacedStartLines = this.blocks.slice(lo, hi + 1).map((b) => b.startLine);
    this.blocks.splice(lo, hi - lo + 1, ...newBlocks);
    // The kept old blocks (beyond the consumed range) are the same
    // physical lines, but their line numbers must follow the document's
    // line-count delta (the stable point guaranteed their structure is
    // still valid; only the numbering moved).
    if (delta !== 0) {
      for (let k = lo + newBlocks.length; k < this.blocks.length; k++) {
        this.blocks[k].startLine += delta;
        this.blocks[k].endLine += delta;
      }
    }
    this.lastScanned = scanned;
    this.lastReparsed = newBlocks.length;
    return {
      stats: { linesScanned: scanned, blocksReparsed: newBlocks.length },
      lo,
      hi: lo + newBlocks.length - 1,
      replacedStartLines,
    };
  }

  /** Reference oracle for tests: full rescan from scratch. */
  verify(lines: LineText, lineCount: number): boolean {
    const ref = scanBlocksFull(lines, lineCount);
    if (ref.length !== this.blocks.length) return false;
    for (let i = 0; i < ref.length; i++) {
      const a = ref[i];
      const b = this.blocks[i];
      if (a.kind !== b.kind || a.level !== b.level ||
          a.startLine !== b.startLine || a.endLine !== b.endLine) {
        return false;
      }
    }
    return true;
  }
}

/** Parse one block's text into inline runs (L1: **bold**, *em*, `code`,
 *  [link](url)). Unclosed openers degrade to body text. Runs cover
 *  [0, text.length); the syntax characters are included (L1 keeps the
 *  source visible). Block-level styling is the caller's concern. */
export function parseInline(text: string): Run[] {
  const runs: Run[] = [];
  const n = text.length;
  const push = (s: number, e: number, style: RunStyle) => {
    if (e > s) runs.push({ start: s, end: e, style });
  };
  let pos = 0;
  while (pos < n) {
    // Earliest opener among the inline constructs.
    let best: { at: number; style: RunStyle } | null = null;
    const b = text.indexOf("**", pos);
    if (b >= 0) best = { at: b, style: "bold" };
    const c = text.indexOf("`", pos);
    if (c >= 0 && (!best || c < best.at)) best = { at: c, style: "code" };
    const l = text.indexOf("[", pos);
    if (l >= 0 && (!best || l < best.at)) {
      const closeBracket = text.indexOf("]", l + 1);
      if (closeBracket >= 0 && text[closeBracket + 1] === "(" &&
          text.indexOf(")", closeBracket + 2) >= 0) {
        best = { at: l, style: "link" };
      }
    }
    const s = text.indexOf("*", pos);
    if (s >= 0 && text[s + 1] !== "*" && (!best || s < best.at)) {
      best = { at: s, style: "em" };
    }
    if (!best) {
      push(pos, n, "body");
      break;
    }
    push(pos, best.at, "body");
    let end = -1;
    if (best.style === "bold") {
      const close = text.indexOf("**", best.at + 2);
      end = close >= 0 ? close + 2 : -1;
    } else if (best.style === "code") {
      const close = text.indexOf("`", best.at + 1);
      end = close >= 0 ? close + 1 : -1;
    } else if (best.style === "link") {
      const closeBracket = text.indexOf("]", best.at + 1);
      const closeParen = text.indexOf(")", closeBracket + 2);
      end = closeParen >= 0 ? closeParen + 1 : -1;
    } else {
      // em: next bare "*" (not the first char of "**").
      let q = best.at + 1;
      while (q < n && !(text[q] === "*" && text[q - 1] !== "*")) q++;
      end = q < n ? q + 1 : -1;
    }
    if (end < 0) {
      // Unclosed opener: degrade to body, advance past the opener.
      const adv = best.style === "bold" ? 2 : 1;
      push(best.at, best.at + adv, "body");
      pos = best.at + adv;
      continue;
    }
    push(best.at, end, best.style);
    pos = end;
  }
  return runs;
}
