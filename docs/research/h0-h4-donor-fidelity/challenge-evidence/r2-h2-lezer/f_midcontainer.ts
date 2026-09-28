// Decisive donor probes for the local deviations: mid-container salvage,
// interruptor-boundary take, short-tail fragment retention.
import {TreeFragment} from "@lezer/common"
import {parser as mdParser} from "@lezer/markdown"

const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }

// ---- MID-CONTAINER: edit inside item 3 of a 30-item tight list ----
{
  let doc = "intro paragraph of some length to clear margins\n\n"
  for (let i = 0; i < 30; i++) doc += `- item ${String(i).padStart(2, "0")} with some words to make it realistic\n`
  doc += "\nafter list tail paragraph with words\n"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("item 03") + 6
  const post = doc.slice(0, at) + "ZZ" + doc.slice(at + 2)
  const chg = [{fromA: at, toA: at + 2, fromB: at, toB: at + 4}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("MID-CONTAINER fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
  // item-level sharing inside the (new) list
  const list0 = k0.find((c: any) => c.type.name === "BulletList")
  const lists1 = k1.filter((c: any) => c.type.name === "BulletList")
  for (const l of lists1) {
    const items = kids(l)
    const shared = items.filter((it: any) => kids(list0).includes(it)).length
    console.log(`   new BulletList: ${items.length} items, ${shared} shared with old list`)
  }
  const totalOld = kids(list0).length
  const allShared = kids(list0).filter((it: any) => lists1.some((l: any) => kids(l).includes(it))).length
  console.log(`   old items reused: ${allShared}/${totalOld}`)
}

// ---- INTERRUPTOR: undamaged tight list directly after a damaged para ----
{
  const doc = "para one that is long enough to be more than one hundred and twenty eight bytes long so that fragments survive the mingap rule on the left side\n- a\n- b\n- c\n- d\n- e\nmore tail text to make the right side long enough as well yes more words here\n"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("that is long")
  const post = doc.slice(0, at) + "THAT IS LONG" + doc.slice(at + 4)
  const chg = [{fromA: at, toA: at + 4, fromB: at, toB: at + 12}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("\nINTERRUPTOR fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any, i: number) => `${c.type.name}${k0.includes(c) ? "=SHARED" : ""}`).join(" "))
}

// ---- SHORT-TAIL: tiny tail after edit (donor keeps after-last-change fragment) ----
{
  const doc = "head paragraph long enough alone to clear the left margin rule with more words here indeed\n\ntail"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("head")
  const post = doc.slice(0, at) + "HEAD" + doc.slice(at + 4)
  const chg = [{fromA: at, toA: at + 4, fromB: at, toB: at + 4}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("\nSHORT-TAIL fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
}

// ---- TIGHT-LIST left-edge: edit inside item 2 (does donor keep the before-fragment?) ----
{
  const doc = "# Title\n\npara one text here padding padding padding padding padding padding padding padding padding padding padding padding\n\n- item one\n- item two\n- item three\n\n```\ncode line A\ncode line B\n```\n\nfinal paragraph tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("item two")
  const post = doc.slice(0, at) + "CHANGED" + doc.slice(at + 8)
  const chg = [{fromA: at, toA: at + 8, fromB: at, toB: at + 7}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("\nTIGHT-LIST-EDGE fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
}
