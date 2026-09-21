//! Adversarial small-model differential for H1 BLOCK_LOCAL_REPARSE (stage R5).
//!
//! 504 deterministic edit sequences (>= the frozen 500) across the SIX
//! edit families (local_text, block_boundary, container_state,
//! forward_state, inline_delimiter_state, semantic_dependency) with
//! ASCII and CJK/full-width material. Every intermediate step must
//! satisfy `H1 == H0` (structural + checksum). The generator is a
//! splitmix64 PRNG seeded from fixed constants — no `random_device`, no
//! external RNG crate, byte-identical sequences forever. No timing.

use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Source, WorkCounters, ResultChecksum,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::NormalizeV1;

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — the small-model driver
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

// ---------------------------------------------------------------------------
// Material (ASCII + CJK/full-width, per AGENTS.md section 8)
// ---------------------------------------------------------------------------

const ASCII_WORDS: &[&str] = &[
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "lorem", "ipsum", "dolor", "sit", "amet",
    "markdown", "block", "inline",
];
const CJK_WORDS: &[&str] = &[
    "段落",
    "标题",
    "引用",
    "列表",
    "代码",
    "链接",
    "定义",
    "测试",
    "编辑器",
    "中文",
];
const CJK_LABELS: &[&str] = &["一", "二人", "三番", "参考"];

fn words(rng: &mut Rng, cjk: bool) -> String {
    let n = 3 + rng.below(6);
    let mut out = String::new();
    for i in 0..n {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(if cjk {
            rng.pick(CJK_WORDS)
        } else {
            rng.pick(ASCII_WORDS)
        });
    }
    out
}

fn label(rng: &mut Rng, cjk: bool) -> String {
    if cjk {
        (*rng.pick(CJK_LABELS)).to_string()
    } else {
        (*rng.pick(ASCII_WORDS)).to_string()
    }
}

// ---------------------------------------------------------------------------
// Base documents: 8-14 blocks mixing the family's structures
// ---------------------------------------------------------------------------

fn base_doc(rng: &mut Rng, cjk: bool) -> String {
    let mut blocks: Vec<String> = Vec::new();
    for _ in 0..(6 + rng.below(6)) {
        blocks.push(match rng.below(8) {
            0 => format!("# {}", words(rng, cjk)),
            1 => format!("```\n{}\n```", words(rng, cjk)),
            2 => format!("> {}", words(rng, cjk)),
            3 => format!("- {}", words(rng, cjk)),
            4 => format!(
                "[{}]: /dest/{}",
                label(rng, cjk),
                words(rng, cjk).replace(' ', "-")
            ),
            5 => format!("see [{}] here", label(rng, cjk)),
            6 => format!("**{}** and `code`", words(rng, cjk)),
            _ => words(rng, cjk),
        });
    }
    let mut doc = blocks.join("\n\n");
    doc.push('\n');
    doc
}

// ---------------------------------------------------------------------------
// Family-targeted edits against the CURRENT document (the small model)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    LocalText,
    BlockBoundary,
    ContainerState,
    ForwardState,
    InlineDelimiter,
    SemanticDependency,
}

const FAMILIES: [Family; 6] = [
    Family::LocalText,
    Family::BlockBoundary,
    Family::ContainerState,
    Family::ForwardState,
    Family::InlineDelimiter,
    Family::SemanticDependency,
];

fn line_starts(src: &str) -> Vec<usize> {
    let b = src.as_bytes();
    let mut v = vec![0usize];
    for (i, &c) in b.iter().enumerate() {
        if c == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

/// Snap a byte position to a UTF-8 char boundary (never splits a rune).
fn snap(src: &str, mut p: usize) -> usize {
    while p < src.len() && !src.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// One family-targeted edit: (start, end, inserted).
fn family_edit(family: Family, rng: &mut Rng, src: &str) -> (usize, usize, String) {
    let len = src.len();
    let starts = line_starts(src);
    let line = starts[rng.below(starts.len())];
    let cjk = rng.below(2) == 1;
    let (s, e, ins) = match family {
        Family::LocalText => match rng.below(5) {
            0 => {
                // mid-line word insert (para-internal, continuation-safe)
                let p = snap(src, line + rng.below(len.saturating_sub(line).min(40)));
                (p, p, format!("{} ", words(rng, cjk)))
            }
            1 => {
                // small mid-line delete
                let a = snap(src, line + rng.below(len.saturating_sub(line).min(30)));
                let b = snap(src, (a + 1 + rng.below(10)).min(len));
                (a, b, String::new())
            }
            2 => {
                // same-length replace inside a line
                let a = snap(src, line + rng.below(len.saturating_sub(line).min(30)));
                let b = snap(src, (a + 2 + rng.below(6)).min(len));
                let same = "x".repeat(b - a);
                (a, b, same)
            }
            3 => {
                // EOF append: paragraph continuation at the document end
                (len, len, words(rng, cjk))
            }
            _ => {
                // insert two words at the line start
                (line, line, format!("{} ", words(rng, cjk)))
            }
        },
        Family::BlockBoundary => match rng.below(4) {
            0 => {
                // delete ONE LF of the separator after the chosen line
                // (paragraph-merge hazard)
                let lf = src.as_bytes()[line..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .map_or(len, |p| line + p);
                (
                    lf.min(len.saturating_sub(1)),
                    lf.min(len.saturating_sub(1)) + 1,
                    String::new(),
                )
            }
            1 => {
                // insert a heading at a line start (split + interruptor)
                (line, line, format!("# {}\n", words(rng, cjk)))
            }
            2 => {
                // insert a blank line at a line start
                (line, line, "\n".to_string())
            }
            _ => {
                // downgrade a heading: strip its marker if present
                if src.as_bytes()[line..].len() >= 2 && src.as_bytes()[line] == b'#' {
                    (line, line + 2, String::new())
                } else {
                    (line, line, "# h\n".to_string())
                }
            }
        },
        Family::ContainerState => match rng.below(3) {
            0 => (line, line, "> ".to_string()),
            1 => {
                // remove a leading quote marker if present
                if src.as_bytes()[line..].len() >= 2 && src.as_bytes()[line] == b'>' {
                    (line, line + 2, String::new())
                } else {
                    (line, line, "> ".to_string())
                }
            }
            _ => (line, line, format!("- {}\n", words(rng, cjk))),
        },
        Family::ForwardState => match rng.below(4) {
            0 => (line, line, "```\n".to_string()), // unclosed fence
            1 => (line, line, format!("```\n{}\n```\n", words(rng, cjk))),
            2 => {
                // delete the NEXT line wholesale if any (may eat a closer)
                let nl = starts
                    .get(starts.iter().position(|&p| p == line).unwrap_or(0) + 1)
                    .copied()
                    .unwrap_or(len);
                (line, nl.min(len), String::new())
            }
            _ => (line, line, "# intr\n".to_string()),
        },
        Family::InlineDelimiter => {
            const DELIMS: &[&str] = &["*", "`", "[", "]", "(", ")", "**"];
            match rng.below(3) {
                0 => {
                    let p = snap(src, line + rng.below(len.saturating_sub(line).min(40)));
                    (p, p, (*rng.pick(DELIMS)).to_string())
                }
                1 => {
                    let a = snap(src, line + rng.below(len.saturating_sub(line).min(30)));
                    let b = snap(src, (a + 1).min(len));
                    (a, b, String::new())
                }
                _ => {
                    let p = snap(src, line + rng.below(len.saturating_sub(line).min(40)));
                    (p, p, format!("[{}](/x)", label(rng, cjk)))
                }
            }
        }
        Family::SemanticDependency => match rng.below(4) {
            0 => (
                line,
                line,
                format!("[{}]: /d-{}\n", label(rng, cjk), rng.below(1000)),
            ),
            1 => {
                // delete the first refdef line if one exists
                let mut found = None;
                for &st in &starts {
                    let end = src.as_bytes()[st..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map_or(len, |p| st + p + 1);
                    let l = &src[st..end.min(src.len())];
                    if l.contains("]:") {
                        found = Some((st, end.min(len)));
                        break;
                    }
                }
                found.map_or_else(
                    || (line, line, "[k]: /v\n".to_string()),
                    |(a, b)| (a, b, String::new()),
                )
            }
            2 => {
                let p = snap(src, line + rng.below(len.saturating_sub(line).min(40)));
                (p, p, format!("[{}]", label(rng, cjk)))
            }
            _ => {
                // duplicate the first refdef at the top (first-wins probe)
                let first = src
                    .lines()
                    .find(|l| l.contains("]:"))
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "[d]: /x".to_string());
                (0, 0, format!("{first}\n"))
            }
        },
    };
    // Keep the edit valid: ordered, in range, char boundaries.
    let s = s.min(e).min(len);
    let e = e.min(len);
    debug_assert!(src.is_char_boundary(s) && src.is_char_boundary(e));
    (s, e, ins)
}

// ---------------------------------------------------------------------------
// The differential harness
// ---------------------------------------------------------------------------

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn full_parse_state(bytes: &[u8]) -> H1State {
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let mech = BlockLocalMechanism::new();
    let pending = mech
        .full_parse(&source_of(bytes, 1), &mut cx)
        .expect("full_parse");
    mech.complete(pending).expect("complete").state
}

/// One adversarial sequence: chained edits, every step == H0.
fn run_sequence(family: Family, seed: u64, cjk: bool) {
    let mut rng = Rng::new(seed);
    let mut doc = base_doc(&mut rng, cjk);
    let mut state = full_parse_state(doc.as_bytes());
    let steps = 3 + rng.below(4);
    for step in 0..steps {
        let (s, e, ins) = family_edit(family, &mut rng, &doc);
        let edit = CanonicalEdit::new(s, e, ins).expect("edit in range");
        let post = {
            let mut v = doc.as_bytes().to_vec();
            v.splice(s..e, edit.inserted_text().bytes());
            String::from_utf8(v).expect("post is UTF-8")
        };
        let mut counters = WorkCounters::all_unknown();
        let next_state;
        {
            let mut sink = CounterSink::new(&mut counters);
            let mut cx = MechanismContext::new(&mut sink);
            let mech = BlockLocalMechanism::new();
            let old_source = source_of(doc.as_bytes(), 10);
            let post_source = source_of(post.as_bytes(), 11);
            let prepared = mech
                .prepare_update(&old_source, &post_source, &edit, &state, &mut cx)
                .unwrap_or_else(|err| {
                    panic!("family {family:?} seed {seed} step {step}: prepare: {err:?}")
                });
            let pending = mech
                .update(&old_source, &post_source, &edit, state, prepared, &mut cx)
                .unwrap_or_else(|err| {
                    panic!("family {family:?} seed {seed} step {step}: update: {err:?}")
                });
            let clean = parse_document(post.as_bytes());
            // MEASUREMENT-CORRECTIVE-1: complete() seals the state; the
            // normalized projection and checksum are post-complete exports.
            let done = mech.complete(pending).expect("complete");
            assert_eq!(
                done.state.normalize_v1(),
                clean,
                "family {family:?} seed {seed} step {step}: structural mismatch vs H0"
            );
            assert_eq!(
                done.state.result_checksum(),
                normalized_checksum(&clean),
                "family {family:?} seed {seed} step {step}: checksum mismatch"
            );
            next_state = done.state;
        }
        state = next_state;
        doc = post;
    }
}

#[test]
fn adversarial_small_model_differential_504_sequences() {
    const SEEDS_PER_FAMILY: u64 = 84; // 6 x 84 = 504 >= 500 (frozen floor)
    let mut total = 0u64;
    for family in FAMILIES {
        for i in 0..SEEDS_PER_FAMILY {
            // Deterministic per-(family, iteration) seed; even seeds use
            // CJK material.
            let seed = 1000 * (family as u64 + 1) + i;
            run_sequence(family, seed, i % 2 == 0);
            total += 1;
        }
    }
    assert_eq!(total, 504);
    assert!(total >= 500, "frozen adversarial floor is 500 sequences");
}
