// Gate-A R2 upstream qualitative probes — H2-F1..F5 (donor behavior, NO timing).
// Run: npx tsx f_case.ts  (or node --experimental-strip-types f_case.ts)
import {TreeFragment, Tree} from "@lezer/common"
import {parser as mdParser} from "@lezer/markdown"

function fragLine(f: TreeFragment) {
  const t = f.tree as any
  return `frag[doc ${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd} treeLen=${t.length} treeType=${t.type.name}`
}

function describe(title: string, fragments: readonly TreeFragment[]) {
  console.log(`\n== ${title} (${fragments.length} fragments)`)
  for (const f of fragments) console.log("   " + fragLine(f))
}

// ---------- shared document ----------
const doc0 = [
  "# Title",                       // line 1
  "", "para one text here padding padding padding padding padding padding padding padding padding padding padding padding",
  "", "- item one", "- item two", "- item three",
  "", "```", "code line A", "code line B", "```",
  "", "final paragraph tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail",
].join("\n")
console.log("doc0 length =", doc0.length)

const tree0 = mdParser.parse(doc0)
describe("F0 initial addTree (full parse)", TreeFragment.addTree(tree0))

// ---------- H2-F1: aligned fragment + compatible context -> reuse expected ----------
// Edit inside "para one" (a paragraph) far from other blocks; whole paragraph block forfeited,
// heading and later blocks reused if hashes match.
{
  const at = doc0.indexOf("para one")
  const doc1 = doc0.slice(0, at) + "PARA-EDIT " + doc0.slice(at)
  const chg: ChangedRange[] = [{fromA: at, toA: at, fromB: at, toB: at + 10}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  describe("F1 applyChanges after insertion inside paragraph", frags)
  const tree1 = mdParser.parse(doc1, frags)
  const clean1 = mdParser.parse(doc1)
  const eq = tree1.toString() === clean1.toString()
  console.log("F1 incremental==clean (toString):", eq)
  // Reuse observation: does the FencedCode subtree survive by object identity?
  let fencedOld: any = null, fencedNew: any = null
  const scan = (t: any, out: any[]) => { if (t.type && t.type.name === "FencedCode") out.push(t); (t.children||[]).forEach((c: any) => scan(c, out)) }
  const olds: any[] = [], news: any[] = []
  scan(tree0, olds); scan(tree1, news)
  fencedOld = olds[0]; fencedNew = news[0]
  console.log("F1 FencedCode object identity preserved:", news.length && olds.length && fencedNew === fencedOld)
  // Heading subtree identity too
  const tops0: any[] = [], tops1: any[] = []
  const c0 = tree0.cursor(), c1 = tree1.cursor()
  while (c0.firstChild()) {} // no-op descend guard
  const topChildren = (t: Tree) => { const out: any[] = []; let c = t.cursor(); while (c.nextSibling()) out.push(c.node); c.moveTo(t.from, 1); return out }
  // simpler: iterate first-level children
  const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("F1 top-level child counts old/new:", k0.length, k1.length)
  for (let i = 0; i < Math.min(k0.length, k1.length); i++) {
    if (k0[i] === k1[i]) console.log(`F1 top-level child ${i} SHARED (object identity) len=${k0[i].length}`)
  }
}

// ---------- H2-F2: fragment overlaps edit -> split/trim/drop ----------
{
  // insertion spanning into two "fragments" is impossible (fragments are per addTree);
  // test split: one edit in middle of doc splits the single whole-tree fragment.
  const at = doc0.indexOf("item two")
  const doc1 = doc0.slice(0, at) + "CHANGED" + doc0.slice(at + 8)
  const chg: ChangedRange[] = [{fromA: at, toA: at + 8, fromB: at, toB: at + 7}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  describe("F2 applyChanges after replacement spanning list item", frags)
  // Now with minGap tiny to show the drop policy boundary
  const fragsMinGap2 = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg, 2)
  describe("F2 same edit, minGap=2 (upstream tests use 2)", fragsMinGap2)
  const tree1 = mdParser.parse(doc1, frags)
  console.log("F2 incremental==clean:", tree1.toString() === mdParser.parse(doc1).toString())
}

// ---------- H2-F3: unchanged bytes + incompatible context -> reuse rejected ----------
{
  // Change a list marker char ( - -> * ) on item one: composite-stack hash of the
  // list subtree changes; blocks under it must NOT be reused despite identical text elsewhere.
  const at = doc0.indexOf("- item one")
  const doc1 = doc0.slice(0, at) + "* item one" + doc0.slice(at + 10)
  const chg: ChangedRange[] = [{fromA: at, toA: at + 10, fromB: at, toB: at + 10}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  describe("F3 applyChanges after list-marker change", frags)
  const tree1 = mdParser.parse(doc1, frags)
  console.log("F3 incremental==clean:", tree1.toString() === mdParser.parse(doc1).toString())
  // identity probe: which top-level children are shared?
  const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }
  const k0 = kids(tree0), k1 = kids(tree1)
  const shared = k1.filter((x: any) => k0.includes(x)).length
  console.log(`F3 top-level shared objects: ${shared}/${k1.length} (expect list NOT shared, code/final shared)`)
  // contrast: benign same-length edit inside the final paragraph (context compatible)
  const at2 = doc0.indexOf("final paragraph")
  const doc2 = doc0.slice(0, at2) + "FINAL paragraph" + doc0.slice(at2 + 15)
  const chg2: ChangedRange[] = [{fromA: at2, toA: at2 + 15, fromB: at2, toB: at2 + 15}]
  const tree2 = mdParser.parse(doc2, TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg2))
  const k2 = kids(tree2)
  const shared2 = k2.filter((x: any) => k0.includes(x)).length
  console.log(`F3-control top-level shared objects: ${shared2}/${k2.length}`)
}

// ---------- H2-F4: edit shifts retained fragment -> mapping ----------
{
  // delete a chunk early; everything after shifts left; after-fragment must remap and still reuse.
  const delFrom = doc0.indexOf("para one")
  const delLen = 20
  const doc1 = doc0.slice(0, delFrom) + doc0.slice(delFrom + delLen)
  const chg: ChangedRange[] = [{fromA: delFrom, toA: delFrom + delLen, fromB: delFrom, toB: delFrom}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  describe("F4 applyChanges after deletion (delta shift)", frags)
  const tree1 = mdParser.parse(doc1, frags)
  console.log("F4 incremental==clean:", tree1.toString() === mdParser.parse(doc1).toString())
  const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }
  const k0 = kids(tree0), k1 = kids(tree1)
  let shared = 0
  for (const x of k1) if (k0.includes(x)) shared++
  console.log(`F4 top-level shared objects: ${shared}/${k1.length}`)
}

// ---------- H2-F5: multiple candidate fragments -> discovery ----------
{
  // two distant edits -> >=3 fragments; middle kept only if stretch >= minGap.
  const at1 = doc0.indexOf("para one")
  const at2 = doc0.indexOf("code line A")
  const doc1 = doc0.slice(0, at1) + "X" + doc0.slice(at1 + 1) // same-length edit 1
  const e2 = at2 + 1 // net delta 0 from edit 1
  const doc2 = doc1.slice(0, e2) + "Y" + doc1.slice(e2 + 1)
  const chg: ChangedRange[] = [
    {fromA: at1, toA: at1 + 1, fromB: at1, toB: at1 + 1},
    {fromA: at2, toA: at2 + 1, fromB: e2, toB: e2 + 1},
  ]
  const fragsDefault = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  describe("F5 applyChanges, two edits, default minGap=128", fragsDefault)
  const fragsSmall = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg, 2)
  describe("F5 applyChanges, two edits, minGap=2", fragsSmall)
  const tree2 = mdParser.parse(doc2, fragsSmall)
  console.log("F5 incremental==clean:", tree2.toString() === mdParser.parse(doc2).toString())
}
