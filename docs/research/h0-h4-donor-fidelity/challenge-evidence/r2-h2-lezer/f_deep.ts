// Deep identity walk for the mid-container case (balanced wrappers hide items).
import {TreeFragment, Tree} from "@lezer/common"
import {parser as mdParser} from "@lezer/markdown"

function collect(t: any, out: any[]) { out.push(t); for (const c of t.children ?? []) collect(c, out) }

let doc = "intro paragraph of some length to clear margins for sure yes more words\n\n"
for (let i = 0; i < 30; i++) doc += `- item ${String(i).padStart(2, "0")} with some words to make it realistic\n`
doc += "\nafter list tail paragraph with words and more words here\n"
const tree0 = mdParser.parse(doc)
const at = doc.indexOf("item 03") + 6
const post = doc.slice(0, at) + "ZZ" + doc.slice(at + 2)
const chg = [{fromA: at, toA: at + 2, fromB: at, toB: at + 2}]
const frags = TreeFragment.applyChanges(TreeFragment.addTree(tree0), chg)
const tree1 = mdParser.parse(post, frags)

const oldNodes: any[] = [], newNodes: any[] = []
collect(tree0, oldNodes); collect(tree1, newNodes)
const shared = oldNodes.filter(n => newNodes.includes(n))
// classify shared nodes by type name
const byName: Record<string, number> = {}
for (const n of shared) { const k = n.type?.name ?? "?" ; byName[k] = (byName[k] ?? 0) + 1 }
console.log("MID-CONTAINER deep sharing:", `${shared.length}/${oldNodes.length} old node objects shared`)
console.log("   shared by type:", JSON.stringify(byName))
// ListItem count shared
const oldItems = oldNodes.filter(n => n.type?.name === "ListItem")
const sharedItems = oldItems.filter(n => newNodes.includes(n)).length
console.log(`   ListItem objects: ${sharedItems}/${oldItems.length} shared`)
console.log("   incremental==clean:", tree1.toString() === mdParser.parse(post).toString())
