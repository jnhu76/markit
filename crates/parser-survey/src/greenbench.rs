//! RUN-3 driver — green-tree representation battery (plan §18 attacks).
//!
//! Same corpus family, same edit positions, same machine as run-1.1.
//! For every (size, position) the battery measures:
//! - markit control — `MarkdownState::update` wall + survivor counters
//!   (the M4 suffix cost, ~9 ns/record);
//! - G1 (naive position-free Vec) — replace wall + pointer clones;
//! - G2 (balanced persistent sequence) — replace wall + ancestors
//!   copied (≈ log2 B) — the H4 candidate;
//! - red-view position queries (plan §17) — G1 scan vs G2 O(log B) vs
//!   markit `block_at_offset`: the "did we make reads unacceptable?"
//!   check;
//! - huge paragraph (plan §21) — one leaf vs 64-byte chunks;
//! - recursive container (plan §22) — 200-deep quote chain vs flat.
//!
//! Scope: green rows carry NO parse work (see green.rs); markit rows
//! include real parse+inline work. Comparison is structural counters
//! first, wall-clock per column, never merged (plan §34).

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use markit_core::markdown::MarkdownState;
use markit_core::{ByteOffset, Document, EditTransaction, TextEdit};

use crate::corpus::{human_size, synthetic, SynthOptions};
use crate::green::{self, Green, GreenLeaf};

const QUERY_ROUNDS: usize = 2000;

fn timed_us<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t0 = Instant::now();
    let out = f();
    (out, t0.elapsed().as_secs_f64() * 1e6)
}

fn median(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn block_list(state: &MarkdownState) -> Vec<(u8, usize)> {
    state
        .blocks()
        .map(|b| {
            let r = b.source_range();
            (b.kind() as u8, r.end.as_usize() - r.start.as_usize())
        })
        .collect()
}

/// Deterministic xorshift for query offsets.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

struct ScaleRow {
    position: &'static str,
    blocks: usize,
    markit_inc_us: f64,
    markit_surv_shift: u64,
    g1_us: f64,
    g1_pointers: usize,
    g2_us: f64,
    g2_ancestors: usize,
    coord_rewrites: u64,
    q_g1_ns: f64,
    q_g2_ns: f64,
    q_markit_ns: f64,
}

fn scaling(doc: &str) -> Result<Vec<ScaleRow>, String> {
    let d = Document::new(doc.to_string());
    let snap = d.snapshot();
    let base_state = MarkdownState::build(&snap);
    let blocks = block_list(&base_state);
    let b = blocks.len();
    let g2_root = green::g2_build(&blocks);
    let g1 = green::G1Doc {
        children: blocks
            .iter()
            .map(|&(kind, len)| Arc::new(GreenLeaf { kind, len }))
            .collect(),
    };
    let total = doc.len();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut query_offsets = Vec::with_capacity(QUERY_ROUNDS);
    for _ in 0..QUERY_ROUNDS {
        query_offsets.push((rng.next() as usize) % total);
    }

    // Red-view query costs (representation only; independent of edit).
    let q_g2 = {
        let t0 = Instant::now();
        let mut acc = 0u8;
        for &o in &query_offsets {
            if let Some((k, _)) = green::red_query(&g2_root, o) {
                acc = acc.wrapping_add(k);
            }
        }
        std::hint::black_box(acc);
        t0.elapsed().as_secs_f64() * 1e9 / QUERY_ROUNDS as f64
    };
    let q_g1 = {
        let t0 = Instant::now();
        let mut acc = 0u8;
        for &o in &query_offsets {
            // linear scan over the position-free Vec (no stored offsets)
            let mut off = 0usize;
            for leaf in &g1.children {
                if o < off + leaf.len {
                    acc = acc.wrapping_add(leaf.kind);
                    break;
                }
                off += leaf.len;
            }
        }
        std::hint::black_box(acc);
        t0.elapsed().as_secs_f64() * 1e9 / QUERY_ROUNDS as f64
    };
    let q_markit = {
        let t0 = Instant::now();
        let mut acc = 0u8;
        for &o in &query_offsets {
            if let Some(v) = base_state.block_at_offset(ByteOffset(o)) {
                acc = acc.wrapping_add(v.kind() as u8);
            }
        }
        std::hint::black_box(acc);
        t0.elapsed().as_secs_f64() * 1e9 / QUERY_ROUNDS as f64
    };

    let mut rows = Vec::new();
    for (pos, offset) in [("bof", 0), ("mid", total / 2), ("eof", total.saturating_sub(1))] {
        let idx = green::g2_locate(&g2_root, offset).index;

        // markit control: median of 5 incremental updates.
        let mut mk_us = Vec::new();
        let mut surv = 0u64;
        for it in 0..6 {
            let mut doc2 = Document::new(doc.to_string());
            let mut st = MarkdownState::build(&doc2.snapshot());
            let applied = EditTransaction::typing()
                .with_edit(TextEdit::insert(ByteOffset(offset), "X"))
                .apply(&mut doc2)
                .map_err(|e| format!("apply: {e}"))?;
            let snap2 = doc2.snapshot();
            let (r, us) = timed_us(|| st.update(&snap2, &applied.result));
            r.map_err(|e| format!("update: {e:?}"))?;
            if it > 0 {
                mk_us.push(us);
            }
            surv = st.last_work().survivor_blocks_shifted;
        }
        let markit_inc_us = median(&mut mk_us);

        // G1: persistent replace via full Vec clone.
        let leaf = Arc::new(GreenLeaf {
            kind: blocks[idx].0,
            len: blocks[idx].1 + 1,
        });
        let mut g1_times = Vec::new();
        let mut g1_out = None;
        for _ in 0..5 {
            let (out, us) = timed_us(|| green::g1_replace(&g1, idx, leaf.clone()));
            g1_times.push(us);
            g1_out = Some(out);
        }
        let (g1_doc, g1_c) = g1_out.unwrap();
        let g1_us = median(&mut g1_times);
        green::check(
            &green::g2_build(
                &g1_doc
                    .children
                    .iter()
                    .map(|l| (l.kind, l.len))
                    .collect::<Vec<_>>(),
            ),
            total + 1,
            b,
        )
        .map_err(|e| format!("g1 invariant: {e}"))?;

        // G2: path-copy replace.
        let mut g2_times = Vec::new();
        let mut last = None;
        let mut g2_ancestors = 0;
        for _ in 0..5 {
            let repl = Green::Leaf(Arc::new(GreenLeaf {
                kind: blocks[idx].0,
                len: blocks[idx].1 + 1,
            }));
            let ((root, c), us) = timed_us(|| green::g2_replace_at(&g2_root, idx, repl));
            g2_times.push(us);
            g2_ancestors = c.ancestors_copied;
            last = Some((root, c));
        }
        let (g2_new, g2_c) = last.unwrap();
        let g2_us = median(&mut g2_times);
        green::check(&g2_new, total + 1, b).map_err(|e| format!("g2 invariant: {e}"))?;

        rows.push(ScaleRow {
            position: pos,
            blocks: b,
            markit_inc_us,
            markit_surv_shift: surv,
            g1_us,
            g1_pointers: g1_c.pointers_cloned,
            g2_us,
            g2_ancestors,
            coord_rewrites: g2_c.coord_rewrites,
            q_g1_ns: q_g1,
            q_g2_ns: q_g2,
            q_markit_ns: q_markit,
        });
    }
    Ok(rows)
}

fn huge_paragraph() -> Result<(f64, f64, f64, usize, f64), String> {
    // §21: one 100 KB leaf vs 64-byte chunks, mid edit (+1 byte).
    let big = 100 * 1024;
    // flat: 1 leaf
    let (out, flat_us) = timed_us(|| {
        let repl = Green::Leaf(Arc::new(GreenLeaf { kind: 0, len: big + 1 }));
        green::g2_replace_at(&green::g2_build(&[(0, big)]), 0, repl)
    });
    green::check(&out.0, big + 1, 1).map_err(|e| e.to_string())?;
    // chunked: G2 over 64B chunks
    let chunk = 64usize;
    let n = (big + chunk - 1) / chunk;
    let blocks: Vec<(u8, usize)> = (0..n)
        .map(|i| (0u8, chunk.min(big - i * chunk)))
        .collect();
    let root = green::g2_build(&blocks);
    let (out2, chunk_us) = timed_us(|| {
        let _loc = green::g2_locate(&root, big / 2);
        green::chunked_replace_len(&root, chunk, big / 2 + 1, 1)
    });
    let (new_root, c) = out2?;
    green::check(&new_root, big + 1, n).map_err(|e| e.to_string())?;
    // red query cost on chunked
    let mut rng = Rng(42);
    let q = {
        let t0 = Instant::now();
        for _ in 0..QUERY_ROUNDS {
            let _ = green::red_query(&root, (rng.next() as usize) % big);
        }
        t0.elapsed().as_secs_f64() * 1e9 / QUERY_ROUNDS as f64
    };
    Ok((flat_us, chunk_us, q, n, c.ancestors_copied as f64))
}

fn deep_quote() -> Result<(f64, usize, f64, f64, f64), String> {
    // §22: 200-deep nested container chain vs one flat leaf; inner edit
    // (+1 byte at the innermost content).
    let depth = 200;
    let inner_len = 200; // contents of innermost block
    let nested = green::nested_quote_build(depth, inner_len);
    let (out, nested_us) = timed_us(|| green::nested_replace_inner(&nested, 1));
    let (new_root, c) = out?;
    green::check(&new_root, inner_len + 1, 1).map_err(|e| e.to_string())?;
    // flat leaf: representation cost is a single leaf replace
    let flat = green::g2_build(&[(0, 42109)]);
    let (out2, flat_us) = timed_us(|| {
        green::g2_replace_at(
            &flat,
            0,
            Green::Leaf(Arc::new(GreenLeaf { kind: 0, len: 42_110 })),
        )
    });
    green::check(&out2.0, 42_110, 1).map_err(|e| e.to_string())?;
    Ok((
        nested_us,
        c.ancestors_copied,
        flat_us,
        out2.1.ancestors_copied as f64,
        0.0,
    ))
}

fn fence_cascade(doc: &str) -> Result<(f64, usize, f64, u64), String> {
    // §18 fence cascade shape: one block absorbs the whole suffix.
    // Green: truncate at the fence block index (path copy).
    let d = Document::new(doc.to_string());
    let state = MarkdownState::build(&d.snapshot());
    let blocks = block_list(&state);
    // find a fence block with a large suffix
    let idx = blocks
        .iter()
        .position(|&(kind, _)| kind == markit_core::markdown::BlockKind::FencedCode as u8)
        .unwrap_or(blocks.len() / 2);
    let root = green::g2_build(&blocks);
    let (out, g_us) = timed_us(|| green::g2_truncate(&root, idx + 1));
    let (new_root, c) = out;
    let expected_len: usize = blocks[..idx + 1].iter().map(|&(_, l)| l).sum();
    green::check(&new_root, expected_len, idx + 1).map_err(|e| e.to_string())?;
    // markit control: real fence_len_grow update cost from run-1.1 is
    // re-measured here for the same machine: collapse the fence region
    // by mutating the opener length is out of scope for this driver —
    // return only the green side plus the block stats.
    let _ = (g_us, c.ancestors_copied);
    Ok((g_us, c.ancestors_copied, 0.0, 0))
}

// --- CORRECTIVE-1: G2-HISTORY-STABILITY gate (adversarial review
// MAJOR-2) ---------------------------------------------------------------
//
// The run-3 T-G1..T-G4 rows measured ONE path-copy edit each, always
// starting from the originally balanced root. They prove single-edit
// locality, not long-lived balance. This gate applies a mixed structural
// edit history (char insert/delete, block split/merge at a hotspot,
// uniform positions, BOF) and observes height / memory / query cost over
// time, in three arms:
// - REPLACE_ONLY: 1→1 leaf edits only (no structure change) — control.
// - NAIVE: mixed ops, path-copy only, no rebalance — the degradation
//   probe (review's prediction: hotspot splits grow height unboundedly).
// - REBUILD_ON_HEIGHT: same mix + rebuild the whole sequence balanced
//   whenever height exceeds 2·log2(B)+4 — the cheapest possible
//   balance-maintenance policy, measured for its amortized cost. This
//   does NOT freeze the strategy (WB tree / B-tree / finger tree stay
//   open); it only proves a trivial bound exists and what it costs.
//
// Arms run on a big-stack thread: NAIVE's height is the thing being
// measured and recursive walks (audit/drop) must survive it.

#[derive(Clone, Copy, PartialEq, Debug)]
enum HistArm {
    ReplaceOnly,
    Naive,
    RebuildOnHeight,
}

impl HistArm {
    fn label(&self) -> &'static str {
        match self {
            HistArm::ReplaceOnly => "replace-only",
            HistArm::Naive => "naive-path-copy",
            HistArm::RebuildOnHeight => "rebuild-on-height",
        }
    }
}

struct HistPoint {
    edit: usize,
    blocks: usize,
    doc_len: usize,
    height: usize,
    avg_depth: f64,
    repr_kib: f64,
    query_ns: f64,
    copies_per_edit: f64,
    rebuilds: usize,
    rebuild_leaf_work: u64,
    rebuild_us: f64,
    rebuild_worst_us: f64,
}

fn height_budget(count: usize) -> usize {
    let lg = (usize::BITS - count.max(2).leading_zeros()) as usize;
    2 * lg + 4
}

fn history_gate(doc: &str, edits: usize, arm: HistArm) -> Result<Vec<HistPoint>, String> {
    let doc = doc.to_string();
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || history_gate_inner(doc, edits, arm))
        .map_err(|e| format!("spawn: {e}"))?
        .join()
        .map_err(|_| "history thread panicked".to_string())?
}

fn history_gate_inner(
    doc: String,
    edits: usize,
    arm: HistArm,
) -> Result<Vec<HistPoint>, String> {
    let d = Document::new(doc);
    let state = MarkdownState::build(&d.snapshot());
    let blocks = block_list(&state);
    let mut root = green::g2_build(&blocks);
    let mut rng = Rng(0xC0FFEE19_70A1_2026);
    let (mut copies, mut rebuilds, mut rebuild_work) = (0usize, 0usize, 0u64);
    let (mut rebuild_us, mut rebuild_worst_us) = (0.0f64, 0.0f64);
    let mut points = Vec::new();
    let mut query_rng = Rng(0xABCDEF01_23456789);

    let checkpoint = |root: &Green, e: usize, copies: usize, rebuilds: usize,
                          rebuild_work: u64, rebuild_us: f64, rebuild_worst_us: f64,
                          points: &mut Vec<HistPoint>, query_rng: &mut Rng|
          -> Result<(), String> {
        green::g2_audit(root).map_err(|e| format!("audit @{e}: {e}"))?;
        let count = root.count();
        let mut acc = 0u8;
        let t0 = Instant::now();
        let span = root.len().max(1);
        for _ in 0..500 {
            if let Some((k, _)) = green::red_query(root, (query_rng.next() as usize) % span) {
                acc = acc.wrapping_add(k);
            }
        }
        std::hint::black_box(acc);
        let query_ns = t0.elapsed().as_secs_f64() * 1e9 / 500.0;
        points.push(HistPoint {
            edit: e,
            blocks: count,
            doc_len: root.len(),
            height: green::g2_height(root),
            avg_depth: green::g2_total_leaf_depth(root) as f64 / count.max(1) as f64,
            repr_kib: green::g2_repr_bytes(root) as f64 / 1024.0,
            query_ns,
            copies_per_edit: if e == 0 { 0.0 } else { copies as f64 / e as f64 },
            rebuilds,
            rebuild_leaf_work: rebuild_work,
            rebuild_us,
            rebuild_worst_us,
        });
        Ok(())
    };

    checkpoint(&root, 0, copies, rebuilds, rebuild_work, rebuild_us, rebuild_worst_us,
               &mut points, &mut query_rng)?;

    for e in 1..=edits {
        let count = root.count();
        if count < 8 {
            return Err(format!(
                "doc collapsed at edit {e} (arm {arm:?}): {count} blocks"
            ));
        }
        // Position: 40% a 16-leaf hotspot band at 40% of the doc, 40%
        // uniform, 20% near BOF.
        let band = count * 2 / 5;
        let idx = {
            let bucket = rng.next() % 100;
            if bucket < 40 {
                band + (rng.next() as usize) % 16
            } else if bucket < 80 {
                (rng.next() as usize) % count
            } else {
                (rng.next() as usize) % (count / 20).max(1)
            }
        };
        let idx = idx.min(count - 1);
        let roll = (rng.next() % 100) as u32;
        let before_len = root.len();
        let leaf = green::leaf_at(&root, idx).ok_or("leaf missing")?;
        let kind = leaf.kind;
        let len = leaf.len;

        #[derive(Debug)]
        enum Op {
            Char(i8),
            Split,
            Merge(usize),
        }
        // Mix (of ALL ops): 30% char insert / 20% char delete / 25%
        // split / 20% merge / 5% append — net block drift +0.05/edit so
        // a long history grows instead of burning the document down.
        let op = if arm == HistArm::ReplaceOnly || roll < 30 {
            Op::Char(1)
        } else if roll < 50 {
            Op::Char(if len > 2 { -1 } else { 1 })
        } else if roll < 75 {
            if len >= 4 { Op::Split } else { Op::Char(1) }
        } else if roll < 95 && idx + 1 < count {
            let other = green::leaf_at(&root, idx + 1).ok_or("merge leaf missing")?;
            Op::Merge(other.len)
        } else {
            Op::Char(1)
        };
        if e % 5000 == 0 {
            eprintln!("hist[{arm:?}] e={e} blocks={count} len={}", root.len());
        }

        let mut expect_len = root.len() as isize;
        let mut expect_count = count as isize;
        match op {
            Op::Char(d) => {
                let nl = (len as isize + d as isize).max(1) as usize;
                let (nr, c) = green::g2_replace_at(
                    &root,
                    idx,
                    Green::Leaf(Arc::new(GreenLeaf { kind, len: nl })),
                );
                copies += c.ancestors_copied;
                expect_len += d as isize;
                root = nr;
            }
            Op::Split => {
                let repl = green::g2_build(&[(kind, len / 2), (kind, len - len / 2)]);
                let (nr, c) = green::g2_replace_range(&root, idx, 1, &repl);
                copies += c.ancestors_copied;
                expect_count += 1;
                root = nr;
            }
            Op::Merge(other_len) => {
                let repl = Green::Leaf(Arc::new(GreenLeaf {
                    kind,
                    len: len + other_len,
                }));
                let (nr, c) = green::g2_replace_range(&root, idx, 2, &repl);
                copies += c.ancestors_copied;
                expect_count -= 1;
                root = nr;
            }
        }
        if let Err(err) = green::check(&root, expect_len as usize, expect_count as usize) {
            return Err(format!(
                "invariant at edit {e} (arm {arm:?}): {err}; op={op:?} idx={idx} \
                 leaf_len={len} before_len={before_len} after_len={} \
                 expect_len={expect_len} expect_count={expect_count}",
                root.len()
            ));
        }
        let after_count = root.count() as isize;
        if (after_count - count as isize).abs() > 1 {
            return Err(format!(
                "count jump at edit {e} (arm {arm:?}): {count} -> {after_count}; \
                 op={op:?} idx={idx}"
            ));
        }

        if arm == HistArm::RebuildOnHeight && green::g2_height(&root) > height_budget(root.count())
        {
            let t0 = Instant::now();
            let mut leaves = Vec::with_capacity(root.count());
            green::g2_collect_leaves(&root, &mut leaves);
            rebuild_work += leaves.len() as u64;
            rebuilds += 1;
            root = green::g2_build(&leaves);
            rebuild_us += t0.elapsed().as_secs_f64() * 1e6;
            rebuild_worst_us = rebuild_worst_us.max(t0.elapsed().as_secs_f64() * 1e6);
            green::g2_audit(&root).map_err(|e| format!("post-rebuild: {e}"))?;
        }

        if e % 1000 == 0 || e == edits {
            checkpoint(&root, e, copies, rebuilds, rebuild_work, rebuild_us,
                       rebuild_worst_us, &mut points, &mut query_rng)?;
        }
    }
    Ok(points)
}

/// Front-collapse probe (review MAJOR-2's exact `Node{left, Empty}`
/// concern): repeated g2_truncate on one root — the suffix-absorption
/// shape — and the height it reaches without rebalance.
fn truncate_spine(blocks_n: usize, rounds: usize) -> Result<Vec<(usize, usize, usize)>, String> {
    let blocks: Vec<(u8, usize)> = (0..blocks_n).map(|i| (0u8, 40 + (i % 7) * 3)).collect();
    let mut root = green::g2_build(&blocks);
    let mut out = Vec::new();
    out.push((0, root.count(), green::g2_height(&root)));
    for r in 1..=rounds {
        let count = root.count();
        if count < 16 {
            break;
        }
        let keep = count - count / 8;
        let (nr, _c) = green::g2_truncate(&root, keep);
        root = nr;
        if r % (rounds / 8).max(1) == 0 || r == rounds {
            green::g2_audit(&root).map_err(|e| format!("spine audit r{r}: {e}"))?;
            out.push((r, root.count(), green::g2_height(&root)));
        }
    }
    Ok(out)
}

fn history_summary(rows: &[(HistArm, Vec<HistPoint>)]) -> String {
    let mut s = String::new();
    s.push_str(
        "| arm | edits | blocks | height | avg_depth | repr_KiB | q_ns | copies/edit \
         | rebuilds | rebuild_ms | worst_ms |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for (arm, pts) in rows {
        // first, middle, last checkpoint
        let picks = [0usize, pts.len() / 2, pts.len() - 1];
        for (j, &i) in picks.iter().enumerate() {
            let p = &pts[i];
            let arm_cell = if j == 0 { arm.label() } else { "" };
            s.push_str(&format!(
                "| {arm_cell} | {} | {} | {} | {:.1} | {:.0} | {:.0} | {:.1} | {} | {:.2} | {:.2} |\n",
                p.edit,
                p.blocks,
                p.height,
                p.avg_depth,
                p.repr_kib,
                p.query_ns,
                p.copies_per_edit,
                p.rebuilds,
                p.rebuild_us / 1000.0,
                p.rebuild_worst_us / 1000.0,
            ));
        }
    }
    s
}

pub fn history_run(out: &Path) -> Result<String, String> {
    let mut s = String::new();
    s.push_str("# CORRECTIVE-1 B — G2 edit-history stability gate (review MAJOR-2)\n\n");
    s.push_str("Run-3 measured single edits from a balanced root; this gate applies a \
        MIXED STRUCTURAL history (30% char insert / 20% char delete / 25% block \
        split / 20% block merge / 5% append — net block drift +0.05 per edit; \
        positions: 40% a 16-leaf hotspot band, 40% uniform, 20% near-BOF) \
        and tracks height, average leaf depth, live green allocation (Arc header + \
        payload — NOT total editor state), red-view query cost, and path-copy work. \
        `rebuild-on-height` = rebuild the whole sequence balanced when height exceeds \
        2·log2(B)+4 — the cheapest possible balance policy, measured to prove a bound \
        exists, NOT a frozen strategy.\n\n");

    for &(doc_bytes, edits) in &[(1024 * 1024usize, 10_000usize), (100 * 1024, 100_000)] {
        let doc = synthetic(SynthOptions {
            target_bytes: doc_bytes,
            cjk: false,
            crlf: false,
        });
        s.push_str(&format!(
            "## T-G6 — synth-{}, {edits} edits\n\n",
            human_size(doc_bytes)
        ));
        let mut rows = Vec::new();
        for arm in [
            HistArm::ReplaceOnly,
            HistArm::Naive,
            HistArm::RebuildOnHeight,
        ] {
            let pts = history_gate(&doc, edits, arm)?;
            rows.push((arm, pts));
        }
        s.push_str(&history_summary(&rows));
        s.push('\n');

        // degradation ratios
        for (arm, pts) in &rows {
            let first = &pts[0];
            let last = &pts[pts.len() - 1];
            match arm {
                HistArm::ReplaceOnly => {
                    s.push_str(&format!(
                        "replace-only control: height {} → {} (structure never changes; \
                         doc {} → {} B, blocks constant at {})\n\n",
                        first.height, last.height, first.doc_len, last.doc_len, last.blocks,
                    ));
                }
                HistArm::Naive => {
                    let balanced = (last.blocks as f64).log2().ceil().max(1.0);
                    s.push_str(&format!(
                        "naive arm: height {} → {} after {edits} edits (B = {}; balanced \
                         height would be ≈ {balanced:.0}; ratio {:.1}×; copies/edit \
                         {:.1} vs balanced ≈ {:.0})\n\n",
                        first.height,
                        last.height,
                        last.blocks,
                        last.height as f64 / balanced,
                        last.copies_per_edit,
                        balanced,
                    ));
                }
                HistArm::RebuildOnHeight => {
                    s.push_str(&format!(
                        "rebuild-on-height: height bounded at ≤ {} (budget 2·log2(B)+4 = \
                         {} at final B); {} rebuilds, {} leaves walked total, {:.1} ms \
                         total, worst single {:.2} ms, amortized {:.0} ns/edit over \
                         {edits} edits\n\n",
                        last.height,
                        height_budget(last.blocks),
                        last.rebuilds,
                        last.rebuild_leaf_work,
                        last.rebuild_us / 1000.0,
                        last.rebuild_worst_us / 1000.0,
                        last.rebuild_us * 1000.0 / edits as f64,
                    ));
                }
            }
        }
    }

    s.push_str("## T-G7 — repeated front-collapse (g2_truncate spine probe)\n\n\
        The review flagged g2_truncate's `Node{left, Empty}` shape. One-shot use \
        (T-G4) is fine; this probe applies it repeatedly on one root.\n\n\
        | round | blocks | height |\n|---:|---:|---:|\n");
    for (r, b, h) in truncate_spine(4096, 32)? {
        s.push_str(&format!("| {r} | {b} | {h} |\n"));
    }

    s.push_str("\nReading guide: the review's prediction is confirmed if `naive-\
        path-copy` height grows far beyond ≈ log2(B) while `replace-only` stays at \
        the build height and `rebuild-on-height` stays under its 2·log2(B)+4 budget. \
        That would support the corrected claim: *position-free persistent sequence \
        with single-edit locality is proven; BALANCE MAINTENANCE IS A REQUIRED \
        MECHANISM for structural churn, and a trivial policy bounds it — concrete \
        balancing strategy remains unearned/open.*\n");

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(out.join("summary.md"), &s).map_err(|e| format!("write: {e}"))?;
    Ok(s)
}

pub fn run(out: &Path) -> Result<String, String> {
    let mut s = String::new();
    s.push_str("# RUN-3 — green-tree representation prototype (plan §13–22)\n\n");
    s.push_str("Scope: representation maintenance ONLY — green nodes carry kind + byte \
        length, no offsets; parse work is common to all representations and measured in \
        runs 1/1.1. G1 = naive position-free Vec (isolates sequence cost); G2 = balanced \
        persistent sequence (path-copy updates); coord_rewrites ≡ 0 by construction — \
        that IS the H4 claim, recorded per row. Every op verifies the length/count \
        invariant.\n\n");

    // T-G1 scaling
    s.push_str("## T-G1 — length-changing edit at bof/mid/eof\n\n\
        | corpus | pos | B | markit inc_us | markit survivors | G1 us | G1 ptrs | G2 us | G2 ancestors | coord_rw | q_G1_ns | q_G2_ns | q_markit_ns |\n\
        |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for &size in &[10 * 1024, 100 * 1024, 1024 * 1024] {
        let doc = synthetic(SynthOptions {
            target_bytes: size,
            cjk: false,
            crlf: false,
        });
        let label = format!("synth-{}", human_size(size));
        for r in scaling(&doc)? {
            s.push_str(&format!(
                "| {label} | {} | {} | {:.1} | {} | {:.3} | {} | {:.3} | {} | {} | {:.1} | {:.1} | {:.1} |\n",
                r.position,
                r.blocks,
                r.markit_inc_us,
                r.markit_surv_shift,
                r.g1_us,
                r.g1_pointers,
                r.g2_us,
                r.g2_ancestors,
                r.coord_rewrites,
                r.q_g1_ns,
                r.q_g2_ns,
                r.q_markit_ns,
            ));
        }
    }

    // T-G2 huge paragraph
    let (flat_us, chunk_us, q_ns, chunks, anc) = huge_paragraph()?;
    s.push_str(&format!(
        "\n## T-G2 — huge paragraph granularity (§21, 100 KB paragraph, mid +1B edit)\n\n\
        | representation | edit_us | nodes | red_query_ns |\n|---|---:|---:|---:|\n\
        | one flat leaf | {flat_us:.3} | 1 | — |\n\
        | 64B chunks (G2) | {chunk_us:.3} | {chunks} | {q_ns:.1} (ancestors {anc:.0}) |\n\
        \nReference point (run-1.1, parse included): markit L1 rescans the whole 100 KB \
        block (R ≈ 100 003, 215 µs at 1 MB-position corpus). Chunked green pays \
        ancestors + one chunk instead — at the price of ~{chunks} extra nodes.\n"
    ));

    // T-G3 deep quote
    let (nested_us, depth_copied, flat_us, flat_anc, _) = deep_quote()?;
    s.push_str(&format!(
        "\n## T-G3 — recursive container (§22, 200-deep quote, inner edit)\n\n\
        | representation | edit_us | structure cost |\n|---|---:|---|\n\
        | nested chain (200 levels) | {nested_us:.3} | {depth_copied} ancestors copied |\n\
        | one flat leaf | {flat_us:.3} | {flat_anc:.0} ancestors (parse scope stays 42 KB — E3) |\n\
        \nReference point (run-1.1): markit flat L1 quote_char pays R ≈ 42 093 bytes \
        reparsed, 131 µs. The nested chain bounds future parse scope to the innermost \
        block at O(depth) representation cost.\n"
    ));

    // T-G4 fence cascade (green side only)
    let doc1m = synthetic(SynthOptions {
        target_bytes: 1024 * 1024,
        cjk: false,
        crlf: false,
    });
    let (g_us, anc, _, _) = fence_cascade(&doc1m)?;
    s.push_str(&format!(
        "\n## T-G4 — suffix-absorbing block growth (§18 fence cascade shape)\n\n\
        green g2_truncate at the fence block: {g_us:.3} µs, {anc} ancestors copied \
        (vs run-1.1 markit fence_len_grow: inc ≈ 1.9–2.3 ms, R ≈ 1 048 257 — parse-\
        dominated but with the same O(B) metadata tail).\n"
    ));

    // Red-view queries are already in T-G1; summarize them.
    s.push_str("\n## T-G5 — red-view position queries (§17)\n\n\
        Per-query cost over 2 000 uniform random offsets (T-G1 rows carry per-corpus \
        values): G1 (position-free Vec scan) is O(B) per query; G2 O(log B); markit \
        `block_at_offset` as the product control. The G2 red view must not be slower \
        than the product's existing query path by an unacceptable factor — the T-G1 \
        `q_*` columns are the evidence.\n");

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(out.join("summary.md"), &s).map_err(|e| format!("write: {e}"))?;
    Ok(s)
}
