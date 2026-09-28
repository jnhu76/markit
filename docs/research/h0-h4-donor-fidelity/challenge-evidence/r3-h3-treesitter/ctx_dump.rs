use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{CounterSink, Mechanism, MechanismContext, Source};
use markit_mdbench_old_tree_subtree_reuse::{OldTreeSubtreeReuseMechanism, TNode};
use std::sync::Arc;

fn srcf(bytes: &[u8], id: u64) -> Source {
    Source::new(SourceId(id), String::from_utf8(bytes.to_vec()).unwrap())
}
fn full_parse(bytes: &[u8]) -> markit_mdbench_old_tree_subtree_reuse::H3State {
    let mut counters = markit_mdbench_common::WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = OldTreeSubtreeReuseMechanism::new();
    let pending = mech.full_parse(&srcf(bytes, 1), &mut cx).unwrap();
    mech.complete(pending).unwrap().state
}
fn walk(n: &Arc<TNode>, depth: usize) {
    println!(
        "{:indent$}kind={:?} size={} line_offset={} ctx.frames={} ctx.fence={:?}",
        "",
        n.kind, n.size, n.line_offset, n.ctx.frames.len(), n.ctx.fence,
        indent = depth * 2
    );
    for (_, c) in &n.children { walk(c, depth + 1); }
}
fn main() {
    let doc = "> alpha beta gamma\n>\n> second paragraph words\n>\n> third paragraph words\n\nafter para\n";
    let st = full_parse(doc.as_bytes());
    for e in st.tree().entries.iter() {
        walk(&e.node, 0);
    }
}
