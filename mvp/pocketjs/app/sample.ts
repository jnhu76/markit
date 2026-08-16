// mvp/pocketjs/app/sample.ts — seed document.
//
// Byte-for-byte the GPUI Phase A0 prototype's seed corpus
// (mvp/gpui/src/editor.rs "Seed corpus (plain-10k style)"), so the two
// MVPs render and edit the same document. ASCII + regular Chinese; the
// Phase A1 first-round benchmarks use the ASCII workload (CJK rendering
// is a separate capability item — PocketJS has no CJK system-font
// discovery on main).

export const SAMPLE_DOC = [
  "Markit Phase A0 - GPUI feasibility spike",
  "Fixed font, mouse & keyboard input, insert/delete, cursor,",
  "selection, scroll, resize, HiDPI baseline, Chinese text.",
  "",
  "中文文本显示验证：这是常规中文段落。",
  "低延迟、低卡顿的 Markdown 编辑器是最终目标。",
  "本行用于验证 DirectWrite 字体回退与中日韩字形。",
  "",
  "The quick brown fox jumps over the lazy dog. 0123456789",
  "A short line.",
  "Last line of the seed document.",
].join("\n");
