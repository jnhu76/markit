// Gate-A R2 local H2 qualitative probes — NO timing. Observables: fragment
// count, Arc identity of retained blocks (donor "shared subtree" analogue),
// nodes_reused / blocks_reparsed counters, equality with H0 clean parse.
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_fragment_reuse::{FragmentReuseMechanism, FNode, FTree, H2State};
use std::sync::Arc;

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(SourceId(id), String::from_utf8(bytes.to_vec()).unwrap())
}

fn h2_full(bytes: &[u8]) -> H2State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = FragmentReuseMechanism::new();
    let pending = mech.full_parse(&source_of(bytes, 1), &mut cx).unwrap();
    mech.complete(pending).unwrap().state
}

fn h2_update(
    old: &[u8],
    post: &[u8],
    start: usize,
    end: usize,
    ins: &str,
    old_state: H2State,
) -> (H2State, WorkCounters) {
    let edit = CanonicalEdit::new(start, end, ins).unwrap();
    let mut counters = WorkCounters::all_unknown();
    let new_state;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = FragmentReuseMechanism::new();
        let old_source = source_of(old, 10);
        let post_source = source_of(post, 11);
        let prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .unwrap();
        let pending = mech
            .update(&old_source, &post_source, &edit, old_state, prepared, &mut cx)
            .unwrap();
        new_state = mech.complete(pending).unwrap().state;
    }
    (new_state, counters)
}

/// Collect (kind, start, Arc-pointer) of every top-level slot node, plus
/// deep sharing stats vs the old tree.
fn top_slots(t: &FTree) -> Vec<(&'static str, usize, *const FNode)> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for s in &t.slots {
        let start = cursor + s.gap;
        out.push((kind_name(&s.node), start, Arc::as_ptr(&s.node)));
        cursor = start + s.node.size;
    }
    out
}

fn kind_name(n: &FNode) -> &'static str {
    use markit_mdbench_oracle::normalized::NodeKind as K;
    match n.kind {
        K::Paragraph => "Para",
        K::Heading => "Heading",
        K::FencedCode => "Fence",
        K::ReferenceDefinition => "Def",
        K::BlockQuote => "Quote",
        K::List => "List",
        K::ListItem => "Item",
        _ => "Other",
    }
}

fn deep_share(old: &FTree, new: &FTree) -> (usize, usize) {
    // count old nodes whose Arc address appears anywhere in the new tree
    let mut old_ptrs = Vec::new();
    fn walk(n: &Arc<FNode>, out: &mut Vec<*const FNode>) {
        out.push(Arc::as_ptr(n));
        for (_, c) in &n.children {
            walk(c, out);
        }
    }
    for s in &old.slots {
        walk(&s.node, &mut old_ptrs);
    }
    let mut new_ptrs = Vec::new();
    for s in &new.slots {
        walk(&s.node, &mut new_ptrs);
    }
    let mut shared = 0;
    for p in &old_ptrs {
        if new_ptrs.contains(p) {
            shared += 1;
        }
    }
    (shared, old_ptrs.len())
}

fn report(name: &str, old: &str, start: usize, end: usize, ins: &str) {
    let old_b = old.as_bytes();
    let post = format!("{}{}{}", &old[..start], ins, &old[end..]);
    let post_b = post.as_bytes();
    let old_state = h2_full(old_b);
    let o = top_slots(old_state.tree());
    let mut old_tree_ptrs = Vec::new();
    {
        fn walk(n: &Arc<FNode>, out: &mut Vec<*const FNode>) {
            out.push(Arc::as_ptr(n));
            for (_, c) in &n.children {
                walk(c, out);
            }
        }
        for s in &old_state.tree().slots {
            walk(&s.node, &mut old_tree_ptrs);
        }
    }
    let total = old_tree_ptrs.len();
    let (new_state, counters) = h2_update(old_b, post_b, start, end, ins, old_state);

    // correctness vs H0
    let clean = markit_mdbench_full_rebuild::parse_document(post_b);
    let eq = new_state.normalize_v1() == clean;

    println!("== {name}");
    println!("   incremental==clean: {eq}   checksum eq: {}", new_state.result_checksum()
        == markit_mdbench_oracle::normalized::normalized_checksum(&clean));
    let n = top_slots(new_state.tree());
    let descr: Vec<String> = n
        .iter()
        .map(|(k, s, p)| {
            let shared = o.iter().any(|(_, _, op)| op == p);
            format!("{k}@{s}{}", if shared { "=SHARED" } else { "" })
        })
        .collect();
    println!("   new top slots: {}", descr.join(" "));
    let mut new_tree_ptrs = Vec::new();
    {
        fn walk2(n: &Arc<FNode>, out: &mut Vec<*const FNode>) {
            out.push(Arc::as_ptr(n));
            for (_, c) in &n.children {
                walk2(c, out);
            }
        }
        for s in &new_state.tree().slots {
            walk2(&s.node, &mut new_tree_ptrs);
        }
    }
    let shared = old_tree_ptrs.iter().filter(|p| new_tree_ptrs.contains(p)).count();
    println!(
        "   deep node sharing: {shared}/{total} old nodes retained-by-identity",
    );
    if let Observed::Known(r) = counters.nodes_reused {
        println!("   nodes_reused={r}");
    }
    if let Observed::Known(b) = counters.blocks_reparsed {
        println!("   blocks_reparsed={b}");
    }
    if let Observed::Known(m) = counters.metadata_records_touched {
        println!("   metadata_records_touched={m} (fragment mappings + cursor consultations)");
    }
    println!("   new fragment_count={}", new_state.fragment_count());
}

fn main() {
    let doc0 = "# Title\n\npara one text here padding padding padding padding padding padding padding padding padding padding padding padding\n\n- item one\n- item two\n- item three\n\n```\ncode line A\ncode line B\n```\n\nfinal paragraph tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail tail";

    // F1: insertion inside paragraph
    let at = doc0.find("para one").unwrap();
    report("F1 insert inside paragraph", doc0, at, at, "PARA-EDIT ");

    // F2: replacement inside list item
    let at2 = doc0.find("item two").unwrap();
    report("F2 replace inside list item", doc0, at2, at2 + 8, "CHANGED");

    // F3: list marker change (context change)
    let at3 = doc0.find("- item one").unwrap();
    report("F3 list marker change", doc0, at3, at3 + 10, "* item one");

    // F4: deletion shifting the tail
    let at4 = doc0.find("para one").unwrap();
    report("F4 delete 20 bytes in paragraph", doc0, at4, at4 + 20, "");

    // F5-analogue: single-edit contract -> two surviving pieces when the doc
    // is long on both sides; probe consultation count on a many-block doc.
    let mut many = String::new();
    let blk = "filler block with enough bytes to exceed any margin comfortably here\n\n";
    for _ in 0..40 {
        many.push_str(blk);
    }
    let mid = many.len() / 2;
    report("F5 long doc, mid edit (2 fragments expected)", &many, mid, mid + 2, "XY");

    // Mid-container salvage probe (donor takes item-level runs after damage):
    let mut list_doc = String::new();
    list_doc.push_str("intro paragraph of some length to clear margins\n\n");
    for i in 0..30 {
        list_doc.push_str(&format!("- item {i:02} with some words to make it realistic\n"));
    }
    list_doc.push_str("\nafter list tail paragraph with words\n");
    let edit_at = list_doc.find("item 03").unwrap() + 6;
    report(
        "MID-CONTAINER: edit inside item 3 of a 30-item tight list",
        &list_doc,
        edit_at,
        edit_at + 2,
        "ZZ",
    );

    // Interruptor-boundary probe (undamaged list directly after a damaged para):
    let ip = "para one that is long enough to be more than one hundred and twenty eight bytes long so that fragments survive the mingap rule on the left side\n- a\n- b\n- c\n- d\n- e\nmore tail text to make the right side long enough as well yes more words here\n";
    let pat = ip.find("that is long").unwrap();
    report(
        "INTERRUPTOR: undamaged tight list right after damaged para",
        ip,
        pat,
        pat + 4,
        "THAT IS LONG",
    );

    // Short-tail probe (donor keeps after-last-change fragment however short):
    let st = "head paragraph long enough alone to clear the left margin rule with more words here indeed\n\ntail";
    let at5 = st.find("head").unwrap();
    report("SHORT-TAIL: 4-byte tail after edit (donor keeps tail fragment)", st, at5, at5 + 4, "HEAD");
}
