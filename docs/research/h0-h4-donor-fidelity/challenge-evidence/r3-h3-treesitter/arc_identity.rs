//! Arc-identity probe: which OLD-tree nodes survive (by Arc pointer) into
//! the NEW tree after an update. No timing. Qualitative only.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Source,
};
use markit_mdbench_old_tree_subtree_reuse::{OldTreeSubtreeReuseMechanism, TNode};
use std::sync::Arc;

fn src(bytes: &[u8], id: u64) -> Source {
    Source::new(SourceId(id), String::from_utf8(bytes.to_vec()).unwrap())
}

fn full_parse(bytes: &[u8]) -> markit_mdbench_old_tree_subtree_reuse::H3State {
    let mut counters = markit_mdbench_common::WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = OldTreeSubtreeReuseMechanism::new();
    let pending = mech.full_parse(&src(bytes, 1), &mut cx).unwrap();
    mech.complete(pending).unwrap().state
}

fn collect(nodes: &mut Vec<Arc<TNode>>, entries: &[markit_mdbench_old_tree_subtree_reuse::TEntry]) {
    fn walk(n: &Arc<TNode>, out: &mut Vec<Arc<TNode>>) {
        out.push(n.clone());
        for (_, c) in &n.children {
            walk(c, out);
        }
    }
    for e in entries {
        walk(&e.node, nodes);
    }
}

fn report(name: &str, old: &str, es: usize, ee: usize, ins: &str) {
    let post = format!("{}{}{}", &old[..es], ins, &old[ee..]);
    let old_state = full_parse(old.as_bytes());
    let old_snapshot = old_state.clone();
    let edit = CanonicalEdit::new(es, ee, ins.to_string()).unwrap();
    let new_state;
    {
        let mut counters = markit_mdbench_common::WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = OldTreeSubtreeReuseMechanism::new();
        let old_source = src(old.as_bytes(), 10);
        let post_source = src(post.as_bytes(), 11);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .unwrap();
        let pending = mech
            .update(&old_source, &post_source, &edit, old_state, prepared, &mut cx)
            .unwrap();
        new_state = mech.complete(pending).unwrap().state;
    }
    let mut old_nodes = Vec::new();
    let mut new_nodes = Vec::new();
    collect(&mut old_nodes, &old_snapshot.tree().entries);
    collect(&mut new_nodes, &new_state.tree().entries);
    let mut survivors = Vec::new();
    for o in &old_nodes {
        if new_nodes.iter().any(|n| Arc::ptr_eq(n, o)) {
            survivors.push(o.kind);
        }
    }
    println!("== {name} ==");
    println!("  old nodes: {}, new nodes: {}, Arc-shared survivors: {}", old_nodes.len(), new_nodes.len(), survivors.len());
    let mut kinds = String::new();
    for s in &survivors {
        kinds.push_str(&format!("{s:?} "));
    }
    println!("  survivor kinds: {kinds}");
    // also: what does the old tree's first damaged entry look like?
    for (i, e) in old_snapshot.tree().entries.iter().enumerate() {
        println!("  old entry {i}: kind={:?} size={} children={}", e.node.kind, e.node.size, e.node.children.len());
    }
}

fn main() {
    // Damaged QUOTE containing undamaged earlier paragraphs.
    let qa = "> alpha beta gamma delta epsilon zeta eta theta iota kappa\n";
    let qb = "> second line with more words to fill the size out\n";
    let qc = "> third line with even more filler words to pass boundary\n";
    let doc = format!("{qa}{qb}{qc}\ntail para with additional words so the right side survives too\n");
    let inside_c = qa.len() + qb.len() + 5;
    report("QUOTE: edit inside 3rd para (one quote, 3 paras)", &doc, inside_c, inside_c + 6, "TWEAK");

    // Damaged LIST containing undamaged earlier items.
    let mut l = String::new();
    for i in 0..6 {
        l.push_str(&format!("- item number {i} with some filler text here\n"));
    }
    l.push_str("\nafter para\n");
    let item3 = l.find("item number 3").unwrap();
    report("LIST: edit inside item 3", &l, item3, item3 + 6, "CHANGE");

    // ONE quote with THREE real paragraphs (">" blank separators).
    let p1 = "> alpha beta gamma delta epsilon zeta eta theta\n";
    let blank = ">\n";
    let p2 = "> second paragraph with more words to fill size out\n";
    let p3 = "> third paragraph with even more filler words to pass\n";
    let onedoc = format!("{p1}{blank}{p2}{blank}{p3}\nafter the quote para\n");
    let in3 = onedoc.find("third paragraph").unwrap();
    report("QUOTE-3PARA: edit inside 3rd paragraph of one quote", &onedoc, in3, in3 + 6, "TWEAK");

    // Nested quote in quote, edit in the innermost.
    let n = "> outer one\n> > inner alpha beta gamma delta epsilon zeta\n> outer two\n\nafter para\n";
    let inner = n.find("inner alpha").unwrap();
    report("NESTED QQ: edit inside inner quote", n, inner, inner + 5, "TWK");
}
