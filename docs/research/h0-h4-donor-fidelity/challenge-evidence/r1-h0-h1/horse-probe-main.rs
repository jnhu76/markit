// Gate-A R1 qualitative mechanism probes (round 2). NO timing of any kind.
use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::Mechanism as _;
use markit_mdbench_common::{CanonicalEdit, CounterSink, MechanismContext, Source, WorkCounters};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::NormalizeV1;

fn src(bytes: &[u8], id: u64) -> Source {
    Source::new(markit_mdbench_common::source::SourceId(id), String::from_utf8(bytes.to_vec()).unwrap())
}
fn h1_full(bytes: &[u8]) -> H1State {
    let mech = BlockLocalMechanism::new();
    let mut c = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut c);
    let mut cx = MechanismContext::new(&mut sink);
    let p = mech.full_parse(&src(bytes, 1), &mut cx).unwrap();
    mech.complete(p).unwrap().state
}
fn h1_update(old: &[u8], post: &[u8], s: usize, e: usize, ins: &str, st: H1State) -> (H1State, WorkCounters) {
    let mech = BlockLocalMechanism::new();
    let edit = CanonicalEdit::new(s, e, ins).unwrap();
    let mut c = WorkCounters::all_unknown();
    let st = {
        let mut sink = CounterSink::new(&mut c);
        let mut cx = MechanismContext::new(&mut sink);
        let prep = mech.prepare_update(&src(old, 10), &src(post, 11), &edit, &st, &mut cx).unwrap();
        let p = mech.update(&src(old, 10), &src(post, 11), &edit, st, prep, &mut cx).unwrap();
        sink.finalize_derived();
        mech.complete(p).unwrap().state
    };
    (st, c)
}
fn show(tag: &str, post: &[u8], st: &H1State, c: &WorkCounters) {
    let clean = parse_document(post);
    println!("== {tag}");
    println!("   result==H0: {}  entries={}", st.normalize_v1() == clean, st.entries().len());
    println!("   fallback={:?} blocks_reparsed={:?} nodes_rebuilt={:?} nodes_reused={:?} meta_touched={:?}", c.fallback_to_full_count, c.blocks_reparsed, c.nodes_rebuilt, c.nodes_reused, c.metadata_records_touched);
    println!("   unique_post={:?}/{} unique_old={:?}", c.unique_post_source_bytes, post.len(), c.unique_old_source_bytes);
}
fn main() {
    // H1-F5 (clean): 100 paragraphs; intra-line edit inside paragraph 50.
    let mut big = String::new();
    for i in 0..100 { big.push_str(&format!("paragraph number {i} body\n\n")); }
    let big_b = big.clone().into_bytes();
    // locate "body" inside paragraph 50 and replace with "BOdy" (eq-size, mid-line)
    let marker = format!("paragraph number 50 ");
    let mpos = big.find(&marker).unwrap() + marker.len();
    let (s, e) = (mpos, mpos + 4);
    let post2: String = format!("{}BOdy{}", &big[..s], &big[e..]);
    let st2 = h1_full(&big_b);
    let (st3, c3) = h1_update(&big_b, post2.as_bytes(), s, e, "BOdy", st2);
    show("H1-F5 100-block doc, intra-line edit in block 50", post2.as_bytes(), &st3, &c3);

    // H1-F5 (edit in FIRST block, intra-line): donor early-breaks its scan at the
    // first block past the edit; local scans all entries. Expect meta_touched to
    // cover the whole tiling (enumeration, no index) even for an early edit.
    let marker0 = "paragraph number 0 ";
    let m0 = big.find(marker0).unwrap() + marker0.len();
    let post0: String = format!("{}BOdy{}", &big[..m0], &big[m0 + 4..]);
    let st0 = h1_full(&big_b);
    let (st0b, c0) = h1_update(&big_b, post0.as_bytes(), m0, m0 + 4, "BOdy", st0);
    show("H1-F5 same doc, intra-line edit in block 0 (EARLY)", post0.as_bytes(), &st0b, &c0);
}
