//! Gate-A R3 qualitative probes for H3 challenge cases. No timing of any
//! kind: only counters, change flags, and structural outcomes.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_old_tree_subtree_reuse::{OldTreeSubtreeReuseMechanism, H3State};

fn src(bytes: &[u8], id: u64) -> Source {
    Source::new(SourceId(id), String::from_utf8(bytes.to_vec()).unwrap())
}

fn full_parse(bytes: &[u8]) -> H3State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = OldTreeSubtreeReuseMechanism::new();
    let pending = mech.full_parse(&src(bytes, 1), &mut cx).unwrap();
    mech.complete(pending).unwrap().state
}

fn k(c: &WorkCounters, f: impl FnOnce(&WorkCounters) -> Observed<u64>) -> String {
    match f(c) {
        Observed::Known(n) => n.to_string(),
        o => format!("{o:?}"),
    }
}

fn run_update(name: &str, old: &[u8], post: &[u8], es: usize, ee: usize, ins: &str) {
    let old_state = full_parse(old);
    let edit = CanonicalEdit::new(es, ee, ins.to_string()).unwrap();
    let mut counters = WorkCounters::all_unknown();
    let prepared;
    let done;
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let mech = OldTreeSubtreeReuseMechanism::new();
        let old_source = src(old, 10);
        let post_source = src(post, 11);
        prepared = mech
            .prepare_update(&old_source, &post_source, &edit, &old_state, &mut cx)
            .unwrap();
        let pending = mech
            .update(&old_source, &post_source, &edit, old_state, prepared.clone(), &mut cx)
            .unwrap();
        done = mech.complete(pending).unwrap();
        cx.sink.finalize_derived();
    }
    let clean = parse_document(post);
    let eq = done.state.normalize_v1() == clean;
    let eq_ck = done.state.result_checksum()
        == checksum(&clean);
    println!("== {name} ==");
    println!("  result == H0 : {eq} (checksum {eq_ck})");
    println!(
        "  prepared: changed_count={} patched_nodes={} scanned_entries={} node_count={}",
        prepared.tree.changed_count(),
        prepared.patched_nodes,
        prepared.scanned_entries,
        prepared.tree.node_count(),
    );
    println!(
        "  new tree: changed_count={} node_count={}",
        done.state.tree().changed_count(),
        done.state.tree().node_count(),
    );
    println!(
        "  counters: nodes_reused={} nodes_rebuilt={} blocks_reparsed={} metadata_touched={} unique_bytes={}/{}",
        k(&counters, |c| c.nodes_reused),
        k(&counters, |c| c.nodes_rebuilt),
        k(&counters, |c| c.blocks_reparsed),
        k(&counters, |c| c.metadata_records_touched),
        k(&counters, |c| c.unique_source_bytes),
        post.len(),
    );
}

fn checksum(
    doc: &markit_mdbench_oracle::normalized::NormalizedDocument,
) -> u64 {
    markit_mdbench_oracle::normalized::normalized_checksum(doc)
}

fn main() {
    // ---------- F1: unchanged eligible subtree + compatible state ----------
    let mut doc = String::new();
    let para = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod.\n\n";
    for _ in 0..10 {
        doc.push_str(para);
    }
    let pos = para.len() * 5 + 10; // inside block 5
    let post = format!("{}{}{}", &doc[..pos], "XYZ", &doc[pos + 3..]);
    run_update("F1 local edit block5 of 10-para doc", doc.as_bytes(), post.as_bytes(), pos, pos + 3, "XYZ");

    // ---------- F2: unchanged bytes + incompatible parser state ----------
    // Insert "> " at the very start: every later byte is unchanged, but the
    // container context flips for all of them.
    let old2 = "alpha one\n\nfiller line one to push length beyond any boundary\n\nfiller line two keeps tail far away\n\nbeta two\n";
    let post2 = format!("> {old2}");
    run_update("F2 quote-prefix insert flips context", old2.as_bytes(), post2.as_bytes(), 0, 0, "> ");

    // ---------- F2b: unclosed fence insert flips state for ALL later bytes ----------
    let old2b = "alpha one\n\nfiller line one to push length beyond any boundary\n\nfiller line two keeps tail far away\n\nbeta two\n";
    let post2b = format!("```\\n{old2b}");
    run_update("F2b unclosed fence insert flips state", old2b.as_bytes(), post2b.as_bytes(), 0, 0, "```\\n");

    // ---------- F4b: damaged quote with reusable earlier paragraphs ----------
    let qa = "> alpha beta gamma delta epsilon zeta eta theta iota kappa\n";
    let qb = "> second line with more words to fill the size out\n";
    let qc = "> third line with even more filler words to pass boundary\n";
    let doc4b = format!("{qa}{qb}{qc}\ntail para with additional words so the right side survives too\n");
    let inside_c = qa.len() + qb.len() + 5; // inside qc text
    let post4b = format!("{}{}{}", &doc4b[..inside_c], "TWEAK", &doc4b[inside_c + 6..]);
    run_update("F4b edit inside 3rd para of one quote", doc4b.as_bytes(), post4b.as_bytes(), inside_c, inside_c + 6, "TWEAK");

    // ---------- F5b: 200 blocks + fence flip (worst-case consult path) ----------
    let mut doc5b = String::new();
    for i in 0..200u32 {
        doc5b.push_str(&format!("block {i:03} text fill fill fill fill fill fill fill.\n\n"));
    }
    let post5b = format!("```\\n{doc5b}");
    run_update("F5b 200 blocks + fence flip (no reuse)", doc5b.as_bytes(), post5b.as_bytes(), 0, 0, "```\\n");

    // ---------- F3: edit overlaps a candidate ----------
    let old3 = "para one\n\npara two\n\npara three\n\npara four\n";
    let p2 = old3.find("para two").unwrap();
    let post3 = format!("{}{}{}", &old3[..p2], "EDITED TWO", &old3[p2 + 8..]);
    run_update("F3 edit overlaps block two", old3.as_bytes(), post3.as_bytes(), p2, p2 + 8, "EDITED TWO");

    // ---------- F4: damaged ancestor w/ reusable descendant ----------
    let q1 = "> alpha beta gamma delta epsilon zeta eta theta iota kappa\n";
    let q2 = "> second line with more words to fill the fragment size out\n";
    let q3 = "> third line with even more filler words to get past boundary\n";
    let tail = "tail para with additional words so the right side survives too\n\n";
    let doc4 = format!("{q1}\n{q2}\n{q3}\n{tail}{tail}");
    let p4 = q1.len() + 1 + q2.len() + 1 + q3.len() + 2; // inside q3 (3rd para of the quote)
    let post4 = format!("{}{}{}", &doc4[..p4], "TWEAK", &doc4[p4 + 6..]);
    run_update("F4 damaged quote contains reusable earlier para", doc4.as_bytes(), post4.as_bytes(), p4, p4 + 6, "TWEAK");

    // ---------- F5: many top-level candidates ----------
    let mut doc5 = String::new();
    let blk = "block text number fill fill fill fill fill fill fill fill.\n\n";
    for i in 0..200u32 {
        doc5.push_str(&format!("block {i:03} text fill fill fill fill fill fill fill.\n\n"));
    }
    let _ = blk;
    let late = doc5.len() - 60; // edit near the end: 199 candidate blocks before
    let post5 = format!("{}{}{}", &doc5[..late], "ZED", &doc5[late + 3..]);
    run_update("F5 200 blocks, late edit", doc5.as_bytes(), post5.as_bytes(), late, late + 3, "ZED");
}
