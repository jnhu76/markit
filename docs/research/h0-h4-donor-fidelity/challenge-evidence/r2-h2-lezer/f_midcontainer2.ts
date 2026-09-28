// Corrected decisive donor probes.
import {TreeFragment} from "@lezer/common"
import {parser as mdParser} from "@lezer/markdown"

const kids = (t: any) => { const out: any[] = []; for (let i = 0; i < t.children.length; i++) out.push(t.children[i]); return out }

// ---- MID-CONTAINER corrected: same-length replacement inside item 3 ----
{
  let doc = "intro paragraph of some length to clear margins for sure yes more words\n\n"
  for (let i = 0; i < 30; i++) doc += `- item ${String(i).padStart(2, "0")} with some words to make it realistic\n`
  doc += "\nafter list tail paragraph with words and more words here\n"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("item 03") + 6
  const post = doc.slice(0, at) + "ZZ" + doc.slice(at + 2) // replace 2 chars with 2 chars
  const chg = [{fromA: at, toA: at + 2, fromB: at, toB: at + 2}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("MID-CONTAINER(corr) fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
  const list0: any = k0.find((c: any) => c.type.name === "BulletList")
  const lists1 = k1.filter((c: any) => c.type.name === "BulletList")
  const oldItems = kids(list0)
  for (const l of lists1) {
    const its = kids(l)
    const shared = its.filter((it: any) => oldItems.includes(it)).length
    console.log(`   new BulletList: ${its.length} items, ${shared} shared`)
  }
  const allShared = oldItems.filter((it: any) => lists1.some((l: any) => kids(l).includes(it))).length
  console.log(`   old items reused: ${allShared}/${oldItems.length}`)
}

// ---- INTERRUPTOR corrected: blank-separated tail para after the list ----
{
  const doc = "para one that is long enough to be more than one hundred and twenty eight bytes long so that fragments survive the mingap rule on the left side\n- a\n- b\n- c\n- d\n- e\n\nmore tail text to make the right side long enough as well yes more words here\n"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("that is long")
  const post = doc.slice(0, at) + "THAT IS LONG" + doc.slice(at + 4)
  const chg = [{fromA: at, toA: at + 4, fromB: at, toB: at + 12}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("\nINTERRUPTOR(corr) fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
}

// ---- SHORT-TAIL with trailing newline (fragmentEnd can round to the last line) ----
{
  const doc = "head paragraph long enough alone to clear the left margin rule with more words here indeed\n\ntail\n"
  const tree0 = mdParser.parse(doc)
  const at = doc.indexOf("head")
  const post = doc.slice(0, at) + "HEAD" + doc.slice(at + 4)
  const chg = [{fromA: at, toA: at + 4, fromB: at, toB: at + 4}]
  const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
  console.log("\nSHORT-TAIL(+LF) fragments:")
  for (const f of frags) console.log(`   frag[${f.from},${f.to}] off=${f.offset} openStart=${f.openStart} openEnd=${f.openEnd}`)
  const tree1 = mdParser.parse(post, frags)
  console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
  const k0 = kids(tree0), k1 = kids(tree1)
  console.log("   top-level:", k1.map((c: any) => c.type.name + (k0.includes(c) ? "=SHARED" : "")).join(" "))
}
