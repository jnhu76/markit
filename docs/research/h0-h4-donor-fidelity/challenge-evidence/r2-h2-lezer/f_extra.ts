// Gate-A R2 extra upstream probes: F3 identity detail + fenced-code content edit + NotLast observation.
import {TreeFragment} from "@lezer/common"
import {parser as mdParser} from "@lezer/markdown"

const doc0 = [
  "# Title", "",
  "para one text here padding padding padding padding padding padding padding padding padding padding padding padding",
  "", "- item one", "- item two", "- item three",
  "", "```", "code line A", "code line B", "```",
  "", "final paragraph tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail",
].join("\n")
const tree0 = mdParser.parse(doc0)

const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }
const nameOf = (c: any) => c.type ? c.type.name : "buffer"
const at = doc0.indexOf("- item one")
const doc1 = doc0.slice(0, at) + "* item one" + doc0.slice(at + 10)
const chg = [{fromA: at, toA: at + 10, fromB: at, toB: at + 10}]
const tree1 = mdParser.parse(doc1, TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg))
const k0 = kids(tree0), k1 = kids(tree1)
console.log("F3 old children:", k0.map((c: any, i: number) => `${i}:${nameOf(c)}[${c.from ?? "?"},${c.to ?? "?"}]`).join(" "))
console.log("F3 new children:", k1.map((c: any, i: number) => `${i}:${nameOf(c)}${k0.includes(c) ? "=SHARED" : ""}[${c.from ?? "?"},${c.to ?? "?"}]`).join(" "))

// fenced code content edit (whole-fence reuse expected? no: damaged block forfeited)
const at2 = doc0.indexOf("code line A")
const doc2 = doc0.slice(0, at2) + "CODE-EDIT line A" + doc0.slice(at2 + 11)
const chg2 = [{fromA: at2, toA: at2 + 11, fromB: at2, toB: at2 + 15}]
const frags2 = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg2)
console.log("\nfence-edit fragments:")
for (const f of frags2) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
const tree2 = mdParser.parse(doc2, frags2)
console.log("fence-edit incremental==clean:", tree2.toString() === mdParser.parse(doc2).toString())
const k2 = kids(tree2)
console.log("fence-edit new children:", k2.map((c: any, i: number) => `${i}:${nameOf(c)}${k0.includes(c) ? "=SHARED" : ""}`).join(" "))

// trailing-partial-line: fragment.to lands mid-line (edit at end without newline coverage)
// openEnd effect observed via fragmentEnd shrinking: emit an openEnd fragment by making the
// parse partial is not directly possible via public API here; instead show line-rounded reuse
// by editing the last line partially.
const at3 = doc0.indexOf("tail tail")
const doc3 = doc0.slice(0, at3) + "T" + doc0.slice(at3 + 4) // same length
const chg3 = [{fromA: at3, toA: at3 + 4, fromB: at3, toB: at3 + 4}]
const frags3 = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg3)
console.log("\nlast-para edit fragments:")
for (const f of frags3) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
const tree3 = mdParser.parse(doc3, frags3)
console.log("last-para incremental==clean:", tree3.toString() === mdParser.parse(doc3).toString())
const k3 = kids(tree3)
console.log("last-para new children:", k3.map((c: any, i: number) => `${i}:${nameOf(c)}${k0.includes(c) ? "=SHARED" : ""}`).join(" "))
