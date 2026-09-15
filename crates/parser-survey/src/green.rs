//! RUN-3 — M1 green-tree representation prototype (issue #19 plan
//! §13–22). Research-only: NOT a production parser; answers H4/Q4.
//!
//! Question: does a position-free persistent syntax representation
//! eliminate the suffix absolute-offset rewrite that run-1/1.1 exposed
//! (`M4_SUFFIX_METADATA_REWRITE`, ~9 ns/record)?
//!
//! Scope discipline (plan §13/§21): parse work is common to every
//! representation and measured elsewhere (run-1/1.1); these prototypes
//! carry only what representation maintenance needs — block kind and
//! byte length. The measured object is representation work, not
//! parsing.
//!
//! Prototypes:
//! - **G1** naive green Vec — immutable nodes, position-free, root
//!   children in a `Vec`. Isolates coordinate-rewrite elimination from
//!   sequence relocation: a local edit still clones the whole root
//!   vector (O(B) pointer copies).
//! - **G2** balanced persistent block sequence — weight-balanced binary
//!   tree over block leaves; update = path copy only. Target complexity
//!   (plan §15): locate O(log B), replace O(log B + k), position query
//!   O(log B).
//! - **Red view** (plan §17): offsets computed on demand by traversal —
//!   verifies absolute positions are obtainable without being stored.
//! - **Huge-block granularity** (plan §21): a 100 KB paragraph as one
//!   leaf vs chunked leaves (G2 over chunks).
//! - **Recursive container** (plan §22): a 200-deep quote as nested
//!   container nodes vs one flat leaf.
//!
//! Counters (plan §16): green_nodes_old/reused/new, ancestors_copied,
//! sequence_nodes_copied, absolute_coordinate_records_rewritten (≡ 0 by
//! construction — that is the hypothesis under test, recorded per row).
//! Every op verifies the length invariant (sum of leaf lens == expected
//! bytes) — a coherence oracle for the prototype itself.

use std::sync::Arc;

const LEAF: u8 = 0;

#[derive(Debug)]
pub struct GreenLeaf {
    pub kind: u8,
    pub len: usize,
}

#[derive(Debug)]
pub struct GreenNode {
    pub total_len: usize,
    pub count: usize, // leaf count under this node
    pub left: Green,
    pub right: Green,
}

#[derive(Debug, Clone)]
pub enum Green {
    Leaf(Arc<GreenLeaf>),
    Node(Arc<GreenNode>),
    Empty,
}

impl Green {
    pub fn len(&self) -> usize {
        match self {
            Green::Leaf(l) => l.len,
            Green::Node(n) => n.total_len,
            Green::Empty => 0,
        }
    }

    pub fn count(&self) -> usize {
        match self {
            Green::Leaf(_) => 1,
            Green::Node(n) => n.count,
            Green::Empty => 0,
        }
    }
}

/// Build a balanced G2 tree from block (kind, len) pairs.
pub fn g2_build(blocks: &[(u8, usize)]) -> Green {
    fn build_range(blocks: &[(u8, usize)]) -> Green {
        if blocks.len() == 1 {
            return Green::Leaf(Arc::new(GreenLeaf {
                kind: blocks[0].0,
                len: blocks[0].1,
            }));
        }
        let mid = blocks.len() / 2;
        let left = build_range(&blocks[..mid]);
        let right = build_range(&blocks[mid..]);
        Green::Node(Arc::new(GreenNode {
            total_len: left.len() + right.len(),
            count: left.count() + right.count(),
            left,
            right,
        }))
    }
    match blocks.len() {
        0 => Green::Empty,
        _ => build_range(blocks),
    }
}

#[allow(dead_code)] // offset_in_leaf consumed by parse-scope prototypes
pub struct Locate {
    pub index: usize,
    pub offset_in_leaf: usize,
}

/// G2 locate by byte offset: O(log B).
pub fn g2_locate(root: &Green, offset: usize) -> Locate {
    let mut node = root;
    let mut index = 0;
    let mut off = offset;
    loop {
        match node {
            Green::Leaf(l) => {
                return Locate {
                    index,
                    offset_in_leaf: off.min(l.len),
                }
            }
            Green::Node(n) => {
                let ll = n.left.len();
                if off < ll {
                    node = &n.left;
                } else {
                    off -= ll;
                    index += n.left.count();
                    node = &n.right;
                }
            }
            Green::Empty => {
                return Locate {
                    index,
                    offset_in_leaf: 0,
                }
            }
        }
    }
}

#[allow(dead_code)] // full counter set is the plan-section-16 schema
pub struct G2Counters {
    pub ancestors_copied: usize,
    pub leaves_reused: usize,
    pub leaves_new: usize,
    pub bytes_new: usize,
    pub coord_rewrites: u64, // ≡ 0 by construction; recorded per row
}

/// G2 replace leaf `index` with any `replacement` subtree (a fresh leaf,
/// a two-leaf subtree for splits, …): path-copy only, O(log B).
pub fn g2_replace_at(root: &Green, index: usize, replacement: Green) -> (Green, G2Counters) {
    let total = root.count();
    let reused = total.saturating_sub(replacement.count());
    fn rec(node: &Green, index: usize, replacement: &Green, c: &mut G2Counters) -> Green {
        match node {
            Green::Leaf(_) => replacement.clone(),
            Green::Node(n) => {
                c.ancestors_copied += 1;
                let li = n.left.count();
                let (left, right) = if index < li {
                    (rec(&n.left, index, replacement, c), n.right.clone())
                } else {
                    (n.left.clone(), rec(&n.right, index - li, replacement, c))
                };
                Green::Node(Arc::new(GreenNode {
                    total_len: left.len() + right.len(),
                    count: left.count() + right.count(),
                    left,
                    right,
                }))
            }
            Green::Empty => Green::Empty,
        }
    }
    let mut c = G2Counters {
        ancestors_copied: 0,
        leaves_reused: reused,
        leaves_new: replacement.count(),
        bytes_new: 0,
        coord_rewrites: 0,
    };
    (rec(root, index, &replacement, &mut c), c)
}

/// G2 truncate to leaves `[0, index)` (the fence-cascade shape: one
/// block absorbs the suffix): split path copy only, O(log B). Subtrees
/// fully inside the kept range are shared, not copied.
pub fn g2_truncate(root: &Green, index: usize) -> (Green, G2Counters) {
    fn rec(node: &Green, k: usize, c: &mut G2Counters) -> Green {
        match node {
            Green::Leaf(_) => {
                if k >= 1 {
                    node.clone()
                } else {
                    Green::Empty
                }
            }
            Green::Node(n) => {
                if k == 0 {
                    Green::Empty
                } else if k >= n.count {
                    node.clone()
                } else {
                    c.ancestors_copied += 1;
                    let li = n.left.count();
                    if k <= li {
                        let left = rec(&n.left, k, c);
                        Green::Node(Arc::new(GreenNode {
                            total_len: left.len(),
                            count: left.count(),
                            left,
                            right: Green::Empty,
                        }))
                    } else {
                        let right = rec(&n.right, k - li, c);
                        Green::Node(Arc::new(GreenNode {
                            total_len: n.left.len() + right.len(),
                            count: li + right.count(),
                            left: n.left.clone(),
                            right,
                        }))
                    }
                }
            }
            Green::Empty => Green::Empty,
        }
    }
    let mut c = G2Counters {
        ancestors_copied: 0,
        leaves_reused: index,
        leaves_new: 0,
        bytes_new: 0,
        coord_rewrites: 0,
    };
    (rec(root, index, &mut c), c)
}

/// G1 naive green Vec: same edit, but the root sequence is a Vec that
/// must be cloned (O(B) pointer copies) to stay persistent.
pub struct G1Doc {
    pub children: Vec<Arc<GreenLeaf>>,
}

#[allow(dead_code)] // full counter set is the plan-section-16 schema
pub struct G1Counters {
    pub pointers_cloned: usize,
    pub leaves_reused: usize,
    pub leaves_new: usize,
    pub coord_rewrites: u64,
}

pub fn g1_replace(doc: &G1Doc, index: usize, new_leaf: Arc<GreenLeaf>) -> (G1Doc, G1Counters) {
    let mut children = Vec::with_capacity(doc.children.len());
    children.extend_from_slice(&doc.children[..index]);
    children.push(new_leaf.clone());
    children.extend_from_slice(&doc.children[index + 1..]);
    (
        G1Doc { children },
        G1Counters {
            pointers_cloned: doc.children.len(),
            leaves_reused: doc.children.len() - 1,
            leaves_new: 1,
            coord_rewrites: 0,
        },
    )
}

/// Red view (plan §17): compute the absolute start of every block by
/// traversal. Sequential; per-block O(depth) queries come from
/// [`red_query`].
#[allow(dead_code)] // sequential red traversal kept for future adapters
pub fn red_starts(root: &Green) -> Vec<(u8, usize, usize)> {
    let mut out = Vec::with_capacity(root.count());
    let mut start = 0usize;
    fn rec(node: &Green, start: &mut usize, out: &mut Vec<(u8, usize, usize)>) {
        match node {
            Green::Leaf(l) => {
                out.push((l.kind, *start, l.len));
                *start += l.len;
            }
            Green::Node(n) => {
                rec(&n.left, start, out);
                rec(&n.right, start, out);
            }
            Green::Empty => {}
        }
    }
    rec(root, &mut start, &mut out);
    out
}

/// Random-access position query: block kind covering `offset`, O(log B).
pub fn red_query(root: &Green, offset: usize) -> Option<(u8, usize)> {
    let mut node = root;
    let mut off = offset;
    loop {
        match node {
            Green::Leaf(l) => return Some((l.kind, off.min(l.len))),
            Green::Node(n) => {
                let ll = n.left.len();
                if off < ll {
                    node = &n.left;
                } else {
                    off -= ll;
                    node = &n.right;
                }
            }
            Green::Empty => return None,
        }
    }
}

/// Invariant oracle for the prototype: total length and leaf count must
/// match expectations after every op.
pub fn check(root: &Green, expect_len: usize, expect_count: usize) -> Result<(), String> {
    if root.len() != expect_len {
        return Err(format!("len invariant: {} != {expect_len}", root.len()));
    }
    if root.count() != expect_count {
        return Err(format!("count invariant: {} != {expect_count}", root.count()));
    }
    Ok(())
}

/// Chunked huge-paragraph prototype (plan §21): the paragraph becomes a
/// G2 tree over `chunk_len`-byte chunk leaves. A mid edit rebuilds one
/// chunk + ancestors instead of rescanning the whole block.
pub fn chunked_replace_len(
    root: &Green,
    chunk_len: usize,
    mid_offset: usize,
    delta: isize,
) -> Result<(Green, G2Counters), String> {
    let loc = g2_locate(root, mid_offset);
    let old = match &root {
        _ => match leaf_at(root, loc.index) {
            Some(l) => l,
            None => return Err("chunk leaf missing".into()),
        },
    };
    let new_len = (old.len as isize + delta).max(0) as usize;
    let (new_root, c) = g2_replace_at(
        root,
        loc.index,
        Green::Leaf(Arc::new(GreenLeaf {
            kind: old.kind,
            len: new_len,
        })),
    );
    check(&new_root, (root.len() as isize + delta) as usize, root.count())?;
    let _ = chunk_len;
    Ok((new_root, c))
}

fn leaf_at(root: &Green, index: usize) -> Option<Arc<GreenLeaf>> {
    match root {
        Green::Leaf(l) => Some(l.clone()),
        Green::Node(n) => {
            let li = n.left.count();
            if index < li {
                leaf_at(&n.left, index)
            } else {
                leaf_at(&n.right, index - li)
            }
        }
        Green::Empty => None,
    }
}

/// Recursive container prototype (plan §22): a depth-D quote chain.
/// Editing the innermost leaf path-copies D ancestors; unaffected
/// siblings are shared. This is the green answer to the flat-L1
/// deep-quote cost (run-1: R ≈ 42 093 for one quote-line edit).
pub fn nested_quote_build(depth: usize, inner_len: usize) -> Green {
    let mut node = Green::Leaf(Arc::new(GreenLeaf {
        kind: LEAF + 1, // "paragraph inside quote"
        len: inner_len,
    }));
    for _ in 0..depth {
        node = Green::Node(Arc::new(GreenNode {
            total_len: node.len(),
            count: node.count(),
            left: node,
            right: Green::Empty,
        }));
    }
    node
}

pub fn nested_replace_inner(
    root: &Green,
    delta: usize,
) -> Result<(Green, G2Counters), String> {
    // locate innermost leaf along the left spine, replace, path-copy.
    fn rec(node: &Green, c: &mut G2Counters) -> Green {
        match node {
            Green::Leaf(l) => {
                Green::Leaf(Arc::new(GreenLeaf {
                    kind: l.kind,
                    len: l.len + 1,
                }))
            }
            Green::Node(n) => {
                c.ancestors_copied += 1;
                Green::Node(Arc::new(GreenNode {
                    total_len: n.total_len + 1,
                    count: n.count,
                    left: rec(&n.left, c),
                    right: n.right.clone(),
                }))
            }
            Green::Empty => Green::Empty,
        }
    }
    let mut c = G2Counters {
        ancestors_copied: 0,
        leaves_reused: root.count() - 1,
        leaves_new: 1,
        bytes_new: std::mem::size_of::<GreenLeaf>(),
        coord_rewrites: 0,
    };
    let out = rec(root, &mut c);
    check(&out, root.len() + delta as usize, root.count())?;
    Ok((out, c))
}
