//! markit-mdbench-old-tree-subtree-reuse — H3 OLD_TREE_SUBTREE_REUSE
//! (#22, stage R5; #80 donor-fidelity corrective).
//!
//! Mechanism identity (frozen in
//! `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §8 + the #80 amendment;
//! tree-sitter-inspired — ts_tree_edit change flags on the edited path,
//! the ReusableNode forward cursor, position-alignment + state-agreement
//! splice, and suffix reuse gated on context agreement): retain the OLD
//! TREE with edit/change flags; map the edit onto the tree by patching
//! ONLY the affected ancestry (copy-on-write: sizes along the path
//! absorb the delta, relative offsets of children after the edit shift,
//! nodes overlapping the edit are marked changed — the change flags ARE
//! the damage map); parse the post source in one forward pass
//! consulting the tree through a PERSISTENT FORWARD-ONLY PRE-ORDER
//! CURSOR (the reusable_node.h advance/descend shape: per-level child
//! indices that never move backward; no per-consult stateless table
//! rebuild): unmarked subtrees whose line aligns with the parse
//! position and whose entry `ContextKey` equals the live parser state
//! are spliced whole (shared Arc identity) — at ANY nesting level,
//! including INSIDE a damaged (changed-flag) ancestor: the rejected
//! composite DESCENDS to its unmarked children (ts_parser__reuse_node
//! parser.c:808-811), so damage is a path, not a wall. Rejected or
//! damaged candidates descend (children) or are advanced past and
//! reparsed. Open-edge exclusion windows refuse runs whose termination
//! boundary consumed bytes the edit changed (the H2 window analogue).
//! No fragment table.
//!
//! Position strategy (R2-H09 class "patch-path + derive-at-read",
//! reproduced honestly): top-level entries carry `{gap, node}` with NO
//! stored absolute offsets — absolute coordinates are the prefix sum at
//! read, and because the edited entry's size absorbs the delta, entries
//! after the edit derive their NEW coordinates without ever shifting
//! sibling entries. Rejected reuse is natural degradation, never a
//! fallback — `fallback_to_full_count` is NotApplicable to H3.
//!
//! This crate owns ONLY H3 mechanism state and policy. Grammar semantics
//! live in `markit-mdbench-shared-grammar` (the splice hook is
//! horse-supplied — the scanner never decides reuse); the result
//! vocabulary and validation in `markit-mdbench-oracle`. H3 never calls
//! H0; test crates may use H0 as the correctness oracle.

use std::sync::Arc;

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::Completed;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::NotApplicableSlot;
use markit_mdbench_common::Observed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::{normalized_checksum, Node, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_shared_grammar as sg;
use sg::parser::{parse_region_with_hook, ContextKey, Skel, SpliceHook};

/// Mechanism identifier.
pub const H3_MECHANISM_ID: &str = "old-tree-subtree-reuse-h3";

// ---------------------------------------------------------------------------
// Retained native state (patch-path + derive-at-read position strategy)
// ---------------------------------------------------------------------------

/// Leaf payload of a block node. Inline content is MATERIALIZED at node
/// construction (resolved against the then-current document table) and
/// stored with spans relative to the node's own start.
#[derive(Debug, Clone)]
pub enum TPayload {
    /// Scanned inline nodes (relative spans), links enabled.
    Para {
        inline: Vec<Node>,
        /// The paragraph's content intervals, node-relative (build
        /// metadata, `Skel::Para.segments` rebased). Retained so the
        /// payload can be RE-MATERIALIZED against a changed document
        /// definition table without reparsing the block (the
        /// reference-environment clause in `update`). Never read by a
        /// parse that reuses the payload unchanged.
        segments_rel: Vec<(usize, usize)>,
    },
    Heading {
        level: u8,
        /// Heading content interval, node-relative (build metadata).
        content_rel: (usize, usize),
        inline: Vec<Node>,
    },
    Fence {
        info: String,
        content_rel: (usize, usize),
    },
    Def {
        label: String,
        destination: String,
    },
    /// Quote/List/Item: structure only (children carry everything).
    Container,
}

/// One retained block node. `size` is the byte size of the block's span
/// `[start, start + size)`; children are `(rel_start, node)` pairs with
/// `rel_start` relative to THIS node's span start. `changed` is the
/// tree-sitter-style edit flag: set exactly on the patched ancestry whose
/// extent the edit touched.
#[derive(Debug, Clone)]
pub struct TNode {
    pub kind: NodeKind,
    pub size: usize,
    /// Distance from the block's LINE start to its span start (the
    /// container-prefix / indentation width on its first line). Alignment
    /// matches LINE starts, because blocks nested in containers start
    /// mid-line.
    pub line_offset: usize,
    pub changed: bool,
    pub marker: Option<u8>,
    pub children: Vec<(usize, Arc<TNode>)>,
    /// Entry `ContextKey` captured when the block STARTED (the
    /// state-agreement key, R5 freeze §2).
    pub ctx: ContextKey,
    /// Subtree facts: contains a ReferenceLink / a ReferenceDefinition.
    pub has_ref: bool,
    pub has_def: bool,
    /// Definitions recorded in this subtree, in document order.
    pub def_facts: Vec<(String, String)>,
    pub payload: TPayload,
}

/// One top-level entry: `gap` blank bytes precede the node. NO absolute
/// offset is stored — the absolute position is the prefix sum at read,
/// so patching the edited entry's size moves every later entry without
/// shifting any sibling entry's own fields.
#[derive(Debug, Clone)]
pub struct TEntry {
    pub gap: usize,
    pub node: Arc<TNode>,
}

/// The retained old tree: top-level entries under a Document wrapper (the
/// root is never reused) plus the retained definition facts.
#[derive(Debug, Clone, Default)]
pub struct TTree {
    pub entries: Vec<TEntry>,
    pub defs: Vec<(String, String)>,
    pub src_len: usize,
}

impl TTree {
    /// Number of block nodes in the tree (recursive).
    pub fn node_count(&self) -> u64 {
        self.entries.iter().map(|e| count_node(&e.node)).sum()
    }

    /// Number of nodes carrying the changed flag (recursive).
    pub fn changed_count(&self) -> u64 {
        self.entries.iter().map(|e| changed_node(&e.node)).sum()
    }
}

/// Native node count of one retained subtree under the corrective
/// counting rule (R5-CORRECTIVE-1 §8): the TNode itself PLUS every
/// retained inline syntax node in its payload (recursive). Reused
/// subtrees are shared `Arc`s — their inline payloads ride with the
/// identity, zero parser source reads.
fn count_node(n: &TNode) -> u64 {
    1 + payload_inline_nodes(&n.payload)
        + n.children.iter().map(|(_, c)| count_node(c)).sum::<u64>()
}

/// Recursive count of inline syntax nodes stored in one payload.
fn payload_inline_nodes(p: &TPayload) -> u64 {
    match p {
        TPayload::Para { inline, .. } => count_forest(inline),
        TPayload::Heading { inline, .. } => count_forest(inline),
        _ => 0,
    }
}

/// Recursive count of an inline node forest (each node once).
fn count_forest(nodes: &[Node]) -> u64 {
    nodes.iter().map(|n| 1 + count_forest(&n.children)).sum()
}

/// Rebase absolute content segments to node-relative coordinates.
fn rebase_segments(segments: &[(usize, usize)], start: usize) -> Vec<(usize, usize)> {
    segments
        .iter()
        .map(|(a, b)| (a - start, b - start))
        .collect()
}

fn changed_node(n: &TNode) -> u64 {
    (n.changed as u64) + n.children.iter().map(|(_, c)| changed_node(c)).sum::<u64>()
}

/// H3 retained state: the old tree (with whatever change flags the last
/// update's patch left on it — flags are mechanism state).
#[derive(Debug, Clone)]
pub struct H3State {
    tree: TTree,
}

impl H3State {
    pub fn tree(&self) -> &TTree {
        &self.tree
    }
}

/// Completed-state normalization (R5-CORRECTIVE-1, MAJOR-3): the pure
/// `project` traversal over the retained tree — no source, no sink, no
/// parser. The retained payloads carry every inline syntax fact.
impl NormalizeV1 for H3State {
    fn normalize_v1(&self) -> NormalizedDocument {
        project(&self.tree)
    }
}

/// Pending work handed to `complete()` — the complete patched tree
/// (eager completion boundary, R5 freeze §4).
///
/// MEASUREMENT-CORRECTIVE-1 §9: the normalized projection (`project`, a
/// pure traversal over retained payloads) is experiment/oracle EXPORT,
/// not H3 mechanism state — it is derived at the runner's post-timer
/// export boundary via `NormalizeV1`, never inside the timed update.
pub struct H3Pending {
    state: H3State,
}

/// `prepare_update` product: the PATCHED tree (edit metadata — the
/// damage map), the edit coordinates, and the patch accounting.
#[derive(Debug, Clone)]
pub struct H3Prepared {
    pub edit_start: usize,
    pub edit_end_new: usize,
    /// The patched tree: the edit-mapped damage map (change flags).
    pub tree: TTree,
    pub scanned_entries: u64,
    pub patched_nodes: u64,
    pub definition_changing: bool,
}

// ---------------------------------------------------------------------------
// Mechanism
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct OldTreeSubtreeReuseMechanism {
    /// The discovery-shape summary of the LAST `update` call (#80 §21:
    /// structural observability of the forward-cursor discovery work —
    /// diagnostic export only, never mechanism state). Interior
    /// mutability because the frozen `Mechanism` trait takes `&self`.
    last_discovery: std::cell::Cell<DiscoverySummary>,
}

impl OldTreeSubtreeReuseMechanism {
    pub fn new() -> Self {
        Self::default()
    }

    /// Discovery accounting of the most recent `update` (#80 §21):
    /// consults, entry visits, per-consult maximum, and table rebuilds
    /// (always 0 — the pre-#80 per-consult rebuild is removed).
    pub fn discovery_summary(&self) -> DiscoverySummary {
        self.last_discovery.get()
    }

    /// Clean parse into H3 state + result.
    fn parse_into_pending<W: WorkSink>(
        &self,
        src: &[u8],
        cx: &mut MechanismContext<'_, W>,
    ) -> H3Pending {
        let rp = sg::parse_region(src, 0, src.len(), cx.sink);
        let defs = rp.defs.entries().to_vec();
        let table = ref_table(&defs);
        let mut built = Built::default();
        let entries = build_entries(&rp.blocks, src, &table, &mut built, cx.sink);
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        cx.sink.add_nodes_reused(0);
        cx.sink.add_metadata_records_touched(entries.len() as u64);
        let tree = TTree {
            entries,
            defs,
            src_len: src.len(),
        };
        H3Pending {
            state: H3State { tree },
        }
    }
}

impl Mechanism for OldTreeSubtreeReuseMechanism {
    type State = H3State;
    type Prepared = H3Prepared;
    type Pending = H3Pending;

    fn id(&self) -> MechanismId {
        MechanismId(H3_MECHANISM_ID.to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        declare_not_applicable(cx);
        let pending = self.parse_into_pending(source.as_bytes(), cx);
        Ok(pending)
    }

    fn prepare_update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        let es = edit.start_byte() as usize;
        let ee = edit.end_byte() as usize;
        let delta = edit.inserted_text_len_bytes() as isize - (ee - es) as isize;
        let ee_new = es + edit.inserted_text_len_bytes() as usize;
        // DAMAGE MAPPING: locate the edited top-level entries by a linear
        // scan (reads only) and patch the ancestry chain whose extent the
        // edit touches. Everything else is untouched — the change flags
        // are the damage map (tree-sitter shape).
        let mut scanned = 0u64;
        let mut patched = 0u64;
        let tree = patch_tree(
            &old_state.tree,
            old_source.as_bytes(),
            post_source.as_bytes(),
            &EditFacts {
                es,
                ee,
                ee_new,
                delta,
            },
            &mut scanned,
            &mut patched,
            cx.sink,
        );
        cx.sink.add_metadata_records_touched(scanned + patched);

        // Broad conservative reference invalidation (as H2): the update
        // is definition-changing when a CHANGED (damaged) subtree
        // contains a ReferenceDefinition, or the reparsed region creates
        // one (any `]: ` occurrence inside the edited span's post bytes —
        // pre-computable and source-derived).
        let definition_changing = damaged_has_def(&tree);
        Ok(H3Prepared {
            edit_start: es,
            edit_end_new: (ee as isize + delta) as usize,
            definition_changing,
            tree,
            scanned_entries: scanned,
            patched_nodes: patched,
        })
    }

    fn update<W: WorkSink>(
        &self,
        _old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        _old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let post = post_source.as_bytes();
        let es = prepared.edit_start;
        let ee_new = prepared.edit_end_new;
        debug_assert_eq!(ee_new, es + edit.inserted_text_len_bytes() as usize);
        // The reparsed region may itself create a definition: flag on the
        // region's post bytes (any `]: ` occurrence). The probe reads the
        // edited span when it runs: report the scan (R5-CORRECTIVE-2).
        let definition_changing = prepared.definition_changing || {
            cx.sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Post,
                es as u64,
                ee_new.min(post.len()) as u64,
            );
            post[es..ee_new.min(post.len())]
                .windows(3)
                .any(|w| w == b"]: ")
        };
        declare_not_applicable(cx);

        // Forward pass with the reusable-node cursor (#80 corrective:
        // post-closure consults, persistent forward-only pre-order
        // descent, damaged-ancestor interior descent — no per-consult
        // stateless table rebuild). The hook NEVER decides grammar — it
        // only executes takes the cursor vouched.
        let mut cursor = Cursor::new(&prepared.tree, es, ee_new, definition_changing);
        let (rp, slot_count, takes, consultations, reused, discovery) = {
            let mut hook: Box<SpliceHook<'_>> =
                Box::new(|pos, key, starts_block| cursor.consult(pos, key, starts_block));
            let (rp, slot_count) = parse_region_with_hook(post, 0, post.len(), cx.sink, &mut hook);
            drop(hook); // end the cursor borrow before reading the take record
            (
                rp,
                slot_count,
                cursor.takes,
                cursor.consultations,
                cursor.reused,
                cursor.discovery,
            )
        };
        self.last_discovery.set(discovery);

        // Rebuild the document-global first-wins table from the assembled
        // structure (fresh Def entries + taken runs' recorded facts).
        let table = rebuilt_table(&rp.blocks, &takes);

        // REFERENCE-ENVIRONMENT CLAUSE (sound). A retained payload's
        // reference resolution is valid only while the document-global
        // definition environment is UNCHANGED. Unchanged definition
        // BYTES are not sufficient: an edit can change whether those
        // bytes are definitions at all (deleting a fence closer makes
        // the rest of the document fence content, so the definitions
        // inside it leave the table) — a forward-state change over a
        // long suffix. The rebuilt table is computed from the
        // mechanism's own assembled structure, so the comparison costs
        // no extra parse and is exact for the frozen first-wins
        // semantics.
        // (`prepared.tree` is the patched old tree; it carries the
        // retained table the surviving payloads were materialized against.)
        let reference_environment_changed = table.entries() != prepared.tree.defs.as_slice();
        if reference_environment_changed {
            cx.sink.add_metadata_records_touched(1);
        }

        // Assemble the new tree: fresh TNodes for parsed material, shared
        // Arcs for taken runs (containers wrapping takes are rebuilt; the
        // run members keep their identity — their retained inline payloads
        // are reused with ZERO parser source reads).
        let mut built = Built::default();
        let entries = assemble_entries(
            &rp.blocks,
            &takes,
            post,
            &table,
            reference_environment_changed,
            &mut built,
            cx.sink,
        );
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        // Honest reuse accounting: a re-materialized member was not
        // reused as a unit, but its reference-insensitive descendants
        // kept their identity and still count as reused.
        cx.sink
            .add_nodes_reused(reused - built.rematerialized_members + built.rematerialized_kept);
        // Discovery accounting (D11, #80): every consultation AND every
        // old-tree entry visit is attributed — the pre-#80 stateless
        // enumeration was invisible behind one consultation count.
        cx.sink.add_metadata_records_touched(
            consultations + discovery.total_visits + slot_count as u64,
        );

        let tree = TTree {
            entries,
            defs: table.entries().to_vec(),
            src_len: post.len(),
        };
        Ok(H3Pending {
            state: H3State { tree },
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        // Native-sealing ONLY (eager completion boundary, R5 freeze §4):
        // the projection + checksum are the runner's post-timer export
        // (MEASUREMENT-CORRECTIVE-1).
        Ok(Completed {
            state: pending.state,
        })
    }
}

/// Post-timer experiment export (MEASUREMENT-CORRECTIVE-1): the H3
/// checksum is the checksum of the pure `NormalizeV1` projection over
/// the retained tree.
impl markit_mdbench_common::ResultChecksum for H3State {
    fn result_checksum(&self) -> u64 {
        normalized_checksum(&self.normalize_v1())
    }
}

fn declare_not_applicable<W: WorkSink>(cx: &mut MechanismContext<'_, W>) {
    cx.sink
        .set_slot_not_applicable(NotApplicableSlot::FallbackToFullCount);
    cx.sink.set_restart_distance(Observed::NotApplicable);
    cx.sink.set_convergence_distance(Observed::NotApplicable);
}

// ---------------------------------------------------------------------------
// Edit patch (copy-on-write along the edited ancestry)
// ---------------------------------------------------------------------------

/// Patch the tree for the edit `[es, ee)` (+`delta`): every top-level
/// entry whose span intersects the edit is marked changed; the FIRST one
/// additionally absorbs the delta in its size (which moves every later
/// entry's derived position without shifting any sibling entry's fields)
/// and patches its inner ancestry; a gap edit shifts the following
/// entry's gap.
/// Edit facts for the patch (coordinates in both coordinate systems).
struct EditFacts {
    es: usize,
    ee: usize,
    ee_new: usize,
    delta: isize,
}

fn patch_tree<W: WorkSink>(
    tree: &TTree,
    old: &[u8],
    post: &[u8],
    f: &EditFacts,
    scanned: &mut u64,
    patched: &mut u64,
    sink: &mut W,
) -> TTree {
    let es = f.es;
    let ee = f.ee;
    let delta = f.delta;
    let ee_new = f.ee_new;
    let mut entries = tree.entries.clone();
    // Absolute entry spans by prefix sum.
    let mut cursor = 0usize;
    let mut first_hit: Option<usize> = None;
    let mut prev_idx: Option<usize> = None;
    let mut prev_end: Option<usize> = None;
    let mut next_idx: Option<usize> = None;
    for (i, e) in entries.iter().enumerate() {
        *scanned += 1;
        let start = cursor + e.gap;
        let end = start + e.node.size;
        if start < ee && end > es && first_hit.is_none() {
            first_hit = Some(i);
        }
        if end <= es {
            prev_idx = Some(i);
            prev_end = Some(end);
        }
        if start >= ee && next_idx.is_none() {
            next_idx = Some(i);
        }
        cursor = end;
    }
    // CONTINUATION MARGIN (the changed-flag analogue of H2's open-edge
    // exclusion and H1's F2/F3/F4 guards): the entry before the damage
    // can absorb the damaged region's first line by paragraph/list/quote
    // continuation, and the entry after it can merge with its last line,
    // whenever NO blank line separates them from the edit. The ContextKey
    // deliberately never encodes paragraph state (R5 freeze §2), so a
    // taken neighbor would be spliced with its continuation line already
    // closed. A blank line (>= 2 LFs across the separation) terminates
    // every continuation (§3, D5, §6) and keeps the neighbor reusable.
    let line_aligned = |i: usize, entries: &[TEntry]| -> usize {
        let mut c = 0usize;
        for e in entries.iter().take(i) {
            c += e.gap + e.node.size;
        }
        let start = c + entries[i].gap;
        start - entries[i].node.line_offset
    };
    if let Some(pi) = prev_idx {
        // The margin's LF count reads the separation bytes: report the
        // exact scanned range (R5-CORRECTIVE-2); `lfs` reads nothing
        // when the range is empty.
        let (a, b) = (prev_end.unwrap_or(0), es);
        if a < b {
            sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Old,
                a as u64,
                b as u64,
            );
        }
        let sep_lfs = lfs(old, a, b);
        if sep_lfs < 2 && !hit_index(first_hit, prev_idx) {
            let node = mark_changed(&entries[pi].node, patched);
            entries[pi] = TEntry {
                gap: entries[pi].gap,
                node,
            };
        }
    }
    if let Some(ni) = next_idx {
        let next_line_new = (line_aligned(ni, &tree.entries) as isize + delta).max(0) as usize;
        let (a, b) = (ee_new.min(post.len()), next_line_new.min(post.len()));
        if a < b {
            sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Post,
                a as u64,
                b as u64,
            );
        }
        let sep_lfs = lfs(post, a, b);
        if sep_lfs < 2 && !hit_index(first_hit, next_idx) {
            let node = mark_changed(&entries[ni].node, patched);
            entries[ni] = TEntry {
                gap: entries[ni].gap,
                node,
            };
        }
    }
    let Some(_) = first_hit else {
        // Gap edit: no entry SPAN intersects the edit. The following
        // entry's gap absorbs the delta (its derived position moves); the
        // continuation margin above marks the edge entries whose
        // separation carries no blank line.
        if let Some(i) = next_idx {
            entries[i].gap = (entries[i].gap as isize + delta) as usize;
        }
        return TTree {
            entries,
            defs: tree.defs.clone(),
            src_len: (tree.src_len as isize + delta) as usize,
        };
    };
    // Patch the first hit's ancestry; mark every other intersecting
    // entry changed (their spans lie in the damaged zone; their stale
    // positions can never align, and the flag refuses them outright).
    // An edit may span several top-level entries: the first hit absorbs
    // the delta in its size (clamped at zero), and any residual the
    // clamp could not take moves to the last hit, so the DERIVED
    // positions of the untouched entries after the edit stay shifted by
    // exactly delta. (If even the last hit must clamp, later positions
    // go stale — alignment then misses and those regions reparse
    // naturally; correctness never depends on stale positions.)
    let mut hits: Vec<usize> = Vec::new();
    let mut inner_cursor = 0usize;
    for (i, e) in entries.iter().enumerate() {
        let start = inner_cursor + e.gap;
        let end = start + e.node.size;
        if start < ee && end > es {
            hits.push(i);
        }
        inner_cursor = end;
    }
    if let Some(&first) = hits.first() {
        let node_abs = inner_cursor_zero(&entries, first);
        let node = patch_node(&entries[first].node, node_abs, es, ee, delta, patched);
        let old_size = entries[first].node.size as isize;
        let absorbed = (old_size + delta).max(0) as usize;
        let residual = (old_size + delta) - absorbed as isize; // <= 0 only when clamped
        entries[first] = TEntry {
            gap: entries[first].gap,
            node,
        };
        if residual < 0 {
            if let Some(&last) = hits.last() {
                if last != first {
                    let last_size = entries[last].node.size as isize;
                    let new_last = (last_size + residual).max(0) as usize;
                    let marked = mark_changed(&entries[last].node, patched);
                    entries[last] = TEntry {
                        gap: entries[last].gap,
                        node: Arc::new(TNode {
                            size: new_last,
                            ..TNode::clone(&marked)
                        }),
                    };
                }
            }
        }
    }
    for &i in hits.iter().skip(1) {
        let node = mark_changed(&entries[i].node, patched);
        entries[i] = TEntry {
            gap: entries[i].gap,
            node,
        };
    }
    TTree {
        entries,
        defs: tree.defs.clone(),
        src_len: (tree.src_len as isize + delta) as usize,
    }
}

fn hit_index(first_hit: Option<usize>, i: Option<usize>) -> bool {
    first_hit == i
}

/// Deepest node whose derived (post-coordinate) span contains `p` —
/// returns `(start, node)`. The open-edge window boundary (#80): the
/// block adjacent to the edit consumed bytes the edit changed, so its
/// termination decision is not vouched (the H2 window analogue).
fn deepest_node_containing(tree: &TTree, p: usize) -> Option<(usize, Arc<TNode>)> {
    fn walk(entries: &[(usize, Arc<TNode>)], p: usize) -> Option<(usize, Arc<TNode>)> {
        for (start, node) in entries {
            if *start <= p && p < start + node.size {
                let children: Vec<(usize, Arc<TNode>)> = node
                    .children
                    .iter()
                    .map(|(rel, c)| (start + rel, c.clone()))
                    .collect();
                return walk(&children, p).or(Some((*start, node.clone())));
            }
        }
        None
    }
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    for e in &tree.entries {
        let start = cursor + e.gap;
        entries.push((start, e.node.clone()));
        cursor = start + e.node.size;
    }
    walk(&entries, p)
}

/// Derived (post-coordinate) END of the deepest node whose span
/// contains `p`.
fn block_end_containing(tree: &TTree, p: usize) -> Option<usize> {
    deepest_node_containing(tree, p).map(|(start, node)| start + node.size)
}

/// LEFT open-edge window (#80): a run entirely before the edited span
/// must end at/before the START of the deepest block containing the
/// last unchanged byte before the edit — that adjacent block's
/// termination consumed bytes the edit changed. When that byte falls in
/// a gap, the boundary is the last block's END: a blank separation
/// terminates every continuation deterministically, and a NO-blank
/// separation is already refused by the patch-time continuation margin
/// (the changed flag), so the window does not need to double-exclude.
fn left_window_end(tree: &TTree, es: usize) -> usize {
    if es == 0 {
        return 0;
    }
    if let Some((boundary_start, _)) = deepest_node_containing(tree, es - 1) {
        return boundary_start;
    }
    let mut best = 0usize;
    let mut cursor = 0usize;
    for e in &tree.entries {
        let start = cursor + e.gap;
        let end = start + e.node.size;
        if end <= es {
            best = end;
        }
        cursor = end;
    }
    best
}

/// RIGHT open-edge window (#80): a run entirely after the edited span
/// must start at/after the END of the deepest block containing the
/// edit's last post byte. When that byte falls in a gap, the boundary
/// is the next block's START (blank separations are deterministic; a
/// no-blank separation is already refused by the patch-time margin's
/// changed flag).
fn right_window_start(tree: &TTree, ee_new: usize, post_len: usize) -> usize {
    if ee_new >= post_len {
        return post_len;
    }
    if let Some(end) = block_end_containing(tree, ee_new) {
        return end;
    }
    let mut cursor = 0usize;
    for e in &tree.entries {
        let start = cursor + e.gap;
        if start >= ee_new {
            return start;
        }
        cursor = start + e.node.size;
    }
    post_len
}

fn lfs(src: &[u8], a: usize, b: usize) -> usize {
    if a >= b || b > src.len() {
        return 0;
    }
    src[a..b].iter().filter(|&&c| c == b'\n').count()
}

/// Absolute position of entry `i` by prefix sum (reads only).
fn inner_cursor_zero(entries: &[TEntry], i: usize) -> usize {
    let mut cursor = 0usize;
    for e in entries.iter().take(i) {
        cursor += e.gap + e.node.size;
    }
    cursor + entries[i].gap
}

/// Copy-on-write patch of one node whose span intersects the edit: the
/// size absorbs the delta (clamped at zero — a small node hit by a large
/// edit stays a valid, if stale, span), the node is marked changed,
/// children after the edit shift their relative offsets, intersecting
/// children recurse.
fn patch_node(
    node: &Arc<TNode>,
    abs: usize,
    es: usize,
    ee: usize,
    delta: isize,
    patched: &mut u64,
) -> Arc<TNode> {
    *patched += 1;
    let mut children = node.children.clone();
    for (rel, child) in children.iter_mut() {
        let child_abs = abs + *rel;
        let child_end = child_abs + child.size;
        if child_abs >= ee {
            *rel = (*rel as isize + delta).max(0) as usize;
        } else if child_end > es {
            let patched_child = patch_node(child, child_abs, es, ee, delta, patched);
            if child_abs >= es {
                *rel = (*rel as isize + delta).max(0) as usize;
            }
            *child = patched_child;
        }
    }
    let base = TNode::clone(node);
    let size = (base.size as isize + delta).max(0) as usize;
    Arc::new(TNode {
        size,
        changed: true,
        children,
        ..base
    })
}

/// Copy-on-write change mark (the whole subtree becomes suspect without
/// touching any other entry).
fn mark_changed(node: &Arc<TNode>, patched: &mut u64) -> Arc<TNode> {
    *patched += 1;
    let base = TNode::clone(node);
    Arc::new(TNode {
        changed: true,
        ..base
    })
}

/// Whether any CHANGED (damaged) subtree contains a ReferenceDefinition.
fn damaged_has_def(tree: &TTree) -> bool {
    fn walk(n: &TNode) -> bool {
        if n.changed && (n.has_def || n.kind == NodeKind::ReferenceDefinition) {
            return true;
        }
        n.children.iter().any(|(_, c)| walk(c))
    }
    tree.entries.iter().any(|e| walk(&e.node))
}

// ---------------------------------------------------------------------------
// Reusable-node cursor (#80 corrective: the persistent forward-only
// pre-order cursor of tree-sitter reusable_node.h — advance/descend
// only, monotone with the parse position; damaged-ancestor interior
// descent is a real operative path)
// ---------------------------------------------------------------------------

/// Discovery-shape accounting (#80 §21; structural, never timing):
/// every old-tree entry visit during candidate discovery is counted,
/// so the forward-progress property is observable rather than hidden
/// behind one consultation count (the pre-#80 D11 attribution blind
/// spot).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiscoverySummary {
    /// Line-start consultations received.
    pub consultations: u64,
    /// Total old-tree entry visits over the whole update (skip/descend/
    /// candidate reads). Forward-only: bounded by forward traversal.
    pub total_visits: u64,
    /// The largest number of entry visits any single consult performed.
    pub max_consult_visits: u64,
    /// Top-level tables rebuilt after cursor construction. The cursor
    /// materializes the top level ONCE per update; any further rebuild
    /// would be the pre-#80 stateless-enumeration shape.
    pub table_rebuilds: u64,
}

/// One persistent descent level of the forward cursor: the entries of
/// one old-tree level (borrowed — no per-consult cloning), with the
/// current entry index and its absolute derived position. The top level
/// walks the Document's entries (gaps folded into the running
/// position); nested levels walk a container node's `children`
/// (relative starts over the parent's patched span start).
struct LevelCursor<'a> {
    entries: Option<&'a [TEntry]>,
    children: &'a [(usize, Arc<TNode>)],
    /// Parent span start (nested levels; entries are `base + rel`).
    base: usize,
    /// The current entry's absolute derived position — maintained
    /// incrementally on advance (top: prefix-summed with gaps).
    cur: usize,
    /// The current entry index (monotone; never moves backward).
    idx: usize,
}

impl<'a> LevelCursor<'a> {
    fn top(entries: &'a [TEntry]) -> Self {
        LevelCursor {
            entries: Some(entries),
            children: &[],
            base: 0,
            cur: entries.first().map_or(0, |e| e.gap),
            idx: 0,
        }
    }
    fn nested(base: usize, children: &'a [(usize, Arc<TNode>)]) -> Self {
        LevelCursor {
            entries: None,
            children,
            base,
            cur: children.first().map_or(base, |(rel, _)| base + rel),
            idx: 0,
        }
    }
    fn len(&self) -> usize {
        match self.entries {
            Some(entries) => entries.len(),
            None => self.children.len(),
        }
    }
    fn node(&self) -> &'a Arc<TNode> {
        match self.entries {
            Some(entries) => &entries[self.idx].node,
            None => &self.children[self.idx].1,
        }
    }
    /// Advance past the current entry (forward-only).
    fn advance(&mut self) {
        let size = self.node().size;
        self.idx += 1;
        if self.idx < self.len() {
            self.cur = match self.entries {
                Some(entries) => self.cur + size + entries[self.idx].gap,
                None => self.base + self.children[self.idx].0,
            };
        }
    }
}

struct Cursor<'a> {
    tree: &'a TTree,
    /// The edited span in POST coordinates: `[es, ee_new)`. A take may
    /// never cover any of these bytes (they have no old-tree origin).
    es: usize,
    ee_new: usize,
    /// Open-edge exclusion windows (#80): the block adjacent to each
    /// side of the edit is excluded — its termination/entry decision
    /// consumed bytes the edit changed (the H2 window analogue, in the
    /// patched tree's post coordinates).
    left_window_end: usize,
    right_window_start: usize,
    definition_changing: bool,
    takes: Vec<TakeRun>,
    consultations: u64,
    reused: u64,
    discovery: DiscoverySummary,
    /// The persistent forward pre-order path (top level first). The
    /// donor's ReusableNode advance/descend become index advances and
    /// level pushes here; every consult only moves FORWARD from where
    /// the previous one left the path. No per-consult top-level table
    /// rebuild — the pre-#80 stateless-enumeration shape is gone.
    path: Vec<LevelCursor<'a>>,
}

/// One taken run: the placeholder covers POST bytes `[pos, pos + len)`;
/// the members are shared patched-tree nodes whose (new-coordinate)
/// spans tile that range (unchanged bytes, inter-member gaps included).
struct TakeRun {
    /// The run's LINE start in POST coordinates (placeholder start).
    pos: usize,
    members: Vec<(usize, Arc<TNode>)>,
}

impl<'a> Cursor<'a> {
    fn new(tree: &'a TTree, es: usize, ee_new: usize, definition_changing: bool) -> Self {
        let left_window_end = left_window_end(tree, es);
        let right_window_start = right_window_start(tree, ee_new, tree.src_len);
        Cursor {
            tree,
            es,
            ee_new,
            left_window_end,
            right_window_start,
            definition_changing,
            takes: Vec::new(),
            consultations: 0,
            reused: 0,
            discovery: DiscoverySummary::default(),
            path: vec![LevelCursor::top(&tree.entries)],
        }
    }

    /// Consultation at a line start (post-closure live state — the
    /// donor's consultation point): find a run of unmarked old-tree
    /// blocks whose line aligns with the live position and whose entry
    /// state agrees with the live parser, AT THE LIVE LEVEL — including
    /// INSIDE a damaged (changed-flag) ancestor, whose rejection
    /// descends to its unmarked children (tree-sitter
    /// `ts_parser__reuse_node` parser.c:808-811). Returns the take end
    /// (span end — the scanner rounds it to the next line start) or
    /// `None` (the line parses normally — natural degradation).
    fn consult(&mut self, pos: usize, key: &ContextKey, starts_block: bool) -> Option<usize> {
        self.consultations += 1;
        self.discovery.consultations += 1;
        // SOUNDNESS GATE (the blank-line margin's donor-faithful
        // replacement, #80): an open live paragraph absorbs a
        // plain-text line, so no old block boundary at this line is
        // live — refuse (the ContextKey deliberately excludes paragraph
        // state, R5 freeze §2). At every other line the live parse
        // starts a fresh block, exactly where a run may splice.
        if !starts_block {
            return None;
        }
        let visits_before = self.discovery.total_visits;
        let level = self.seek(pos)?;
        // Vouch gates on the candidate the descent settled on.
        let (cstart, cnode) = {
            let lvl = self.path.last().expect("non-empty path");
            (lvl.cur, lvl.node())
        };
        let _ = level;
        if cnode.changed || cnode.ctx != *key {
            // A damaged candidate is not taken whole — the descent in
            // `seek` already went past changed ancestry; a leaf that is
            // still changed here reparses. A state disagreement refuses
            // the line (the donor's entry-state mismatch: advance /
            // ordinary lexing); the cursor stays for the next line.
            self.bump_max(visits_before);
            return None;
        }
        // STALE-POSITION GUARD (found by the adversarial small-model
        // generator): a patched tree's derived positions can go stale
        // (clamped sizes, residual shifts). A run whose members do not
        // tile monotonically inside the post document is refused —
        // natural degradation, never a correctness risk.
        if cstart.checked_sub(cnode.line_offset) != Some(pos) {
            self.bump_max(visits_before);
            return None;
        }
        // OPEN-EDGE WINDOW (#80): the run must lie fully inside its
        // side's safe window — the block adjacent to the edit is
        // excluded (its termination consumed bytes the edit changed).
        let (win_start, win_end) = if cstart < self.ee_new {
            (0, self.left_window_end)
        } else {
            (self.right_window_start, self.post_len())
        };
        if cstart < win_start || cstart + cnode.size > win_end {
            self.bump_max(visits_before);
            return None;
        }
        let mut members = vec![(cstart, cnode.clone())];
        let mut end = cstart + cnode.size;
        let mut last_idx = self.path.last().expect("path").idx;
        let lvl_len = self.path.last().expect("path").len();
        // Running position of the next entry (top level: prefix-summed
        // with the inter-member gaps — O(1) per extension step).
        let mut next_pos = end;
        for i in (last_idx + 1)..lvl_len {
            let (s2, n2) = {
                let lvl = self.path.last().expect("path");
                match lvl.entries {
                    Some(entries) => (next_pos + entries[i].gap, &entries[i].node),
                    None => (lvl.base + lvl.children[i].0, &lvl.children[i].1),
                }
            };
            if n2.changed || (self.definition_changing && n2.has_ref) || s2 + n2.size > win_end {
                break;
            }
            // Extending covers the bytes up to this member's end; the
            // run must never cross the edited span (a boundary insert
            // marks no node, but its bytes sit in the gap between
            // members). The refused member may still be taken by its
            // own consultation at its own line start.
            if end < self.ee_new && s2 + n2.size > self.es {
                break;
            }
            members.push((s2, n2.clone()));
            end = s2 + n2.size;
            next_pos = end;
            self.discovery.total_visits += 1;
            last_idx = i;
        }
        let len = end.checked_sub(pos)?;
        if pos + len > self.post_len() {
            self.bump_max(visits_before);
            return None;
        }
        let mut expect = pos;
        for (ms, m) in &members {
            if *ms < expect {
                self.bump_max(visits_before);
                return None; // stale: member starts before the covered range
            }
            let mend = ms.checked_add(m.size)?;
            if mend > self.post_len() {
                self.bump_max(visits_before);
                return None;
            }
            expect = mend;
        }
        let new_end = pos + len;
        // The covered range must be disjoint from the edited span.
        if pos < self.ee_new && new_end > self.es {
            self.bump_max(visits_before);
            return None;
        }
        // Consume the run (forward-only): the level index moves past
        // the last taken member.
        for _ in 0..=(last_idx - self.path.last().expect("path").idx) {
            self.path.last_mut().expect("path").advance();
        }
        self.bump_max(visits_before);
        self.reused += members.iter().map(|(_, n)| count_node(n)).sum::<u64>();
        self.takes.push(TakeRun { pos, members });
        Some(new_end)
    }

    fn post_len(&self) -> usize {
        self.tree.src_len
    }

    fn bump_max(&mut self, visits_before: u64) {
        let delta = self.discovery.total_visits - visits_before;
        self.discovery.max_consult_visits = self.discovery.max_consult_visits.max(delta);
    }

    /// Synchronize the persistent descent path to `pos` (POST
    /// coordinates — the patched tree's derived positions ARE post
    /// coordinates) and leave the cursor AT the candidate the descent
    /// settles on. This is tree-sitter's ReusableNode position gate
    /// (parser.c:770-811) adapted to the local line-alignment
    /// contract:
    ///
    /// - entries ending at/before `pos` are skipped (monotone
    ///   forward); an exhausted level pops and advances its parent;
    /// - an entry whose span CONTAINS `pos` is descended into when it
    ///   is damaged (the changed flag — the interior-descent path:
    ///   damage is a path, not a wall) or when its line starts before
    ///   `pos` (an interior line of the container);
    /// - an aligned entry — damaged or not — is the candidate: a
    ///   damaged one descends (children first); the consult's ctx gate
    ///   then decides the take;
    /// - an entry whose line starts after `pos` stays in place for the
    ///   next line (the donor's wait / per-line retry).
    fn seek(&mut self, pos: usize) -> Option<usize> {
        loop {
            let exhausted = {
                let level = self.path.last_mut()?;
                // Monotone skip: entries ending at/before pos are gone.
                while level.idx < level.len() {
                    let (s, n) = (level.cur, level.node());
                    if s + n.size <= pos {
                        level.advance();
                        self.discovery.total_visits += 1;
                    } else {
                        break;
                    }
                }
                level.idx >= level.len()
            };
            if exhausted {
                // Level exhausted: pop and continue at the parent,
                // advancing past the child we were inside.
                self.path.pop();
                self.discovery.total_visits += 1;
                if let Some(parent) = self.path.last_mut() {
                    parent.advance();
                }
                if self.path.is_empty() {
                    return None;
                }
                continue;
            }
            let (cstart, cnode) = {
                let level = self.path.last().expect("non-empty path");
                (level.cur, level.node())
            };
            let aligned = cstart
                .checked_sub(cnode.line_offset)
                .is_some_and(|ls| ls == pos);
            // The containment test is LINE-start-based: a child living
            // on a prefixed line (quote/list content) has its LINE
            // start before its span start — the consult sits on the
            // line, and the prefix bytes belong to the ancestor's span.
            let line_contains = cstart
                .checked_sub(cnode.line_offset)
                .is_some_and(|ls| ls <= pos)
                && cstart + cnode.size > pos;
            if aligned {
                if cnode.changed && !cnode.children.is_empty() {
                    // DAMAGED-ANCESTOR INTERIOR DESCENT (the #80
                    // repair): reject the changed composite and
                    // descend to its children — unmarked descendants
                    // at aligned interior positions are still
                    // reusable (donor parser.c:808-811).
                    self.discovery.total_visits += 1;
                    let child_entries: &'a [(usize, Arc<TNode>)] = &cnode.children[..];
                    let base = cstart;
                    self.path.push(LevelCursor::nested(base, child_entries));
                    continue;
                }
                // Undamaged aligned candidate (or a damaged leaf):
                // settle here — the consult's gates decide.
                self.discovery.total_visits += 1;
                return Some(self.path.len() - 1);
            }
            if line_contains {
                // An INTERIOR line of this entry: descend to the child
                // living at/after pos (containers only — an interior
                // line of a leaf has no candidates).
                if !cnode.children.is_empty() {
                    self.discovery.total_visits += 1;
                    let child_entries: &'a [(usize, Arc<TNode>)] = &cnode.children[..];
                    let base = cstart;
                    self.path.push(LevelCursor::nested(base, child_entries));
                    continue;
                }
                self.discovery.total_visits += 1;
                return None;
            }
            // The entry's line starts after pos (unaligned, later —
            // e.g. the consult sits in the gap before it): no candidate
            // at this consult; the cursor stays AT the entry and the
            // next line retries (the donor's per-line retry).
            self.discovery.total_visits += 1;
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Node construction (fresh parse material -> TNode)
// ---------------------------------------------------------------------------

/// Construction counters (R5-CORRECTIVE-1 §8): `fnodes` counts block
/// structure nodes (the `blocks_reparsed` unit); `nodes` counts every
/// constructed native node INCLUDING freshly scanned inline syntax
/// (the `nodes_rebuilt` unit).
#[derive(Debug, Default)]
struct Built {
    fnodes: u64,
    nodes: u64,
    /// Native nodes of take members RE-MATERIALIZED against a changed
    /// document definition table: the cursor counted these as reused, so
    /// they are subtracted from `nodes_reused` (the subtree was not
    /// reused as a unit).
    rematerialized_members: u64,
    /// Native nodes INSIDE a re-materialized member that kept their
    /// `Arc` identity (reference-insensitive descendants): added back to
    /// `nodes_reused`. `rematerialized_members == nodes + rematerialized_kept`
    /// holds for every rebuilt member, so no node is ever double-counted.
    rematerialized_kept: u64,
}

/// Build top-level entries from completed blocks (absolute spans in the
/// parsed source). Fresh inline scanning is instrumented: every scanned
/// segment is reported to `sink` (R5-CORRECTIVE-1, MAJOR-2).
fn build_entries<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Vec<TEntry> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for sk in blocks {
        let start = sk.start();
        out.push(TEntry {
            gap: start - cursor,
            node: build_tnode(sk, src, table, built, sink),
        });
        cursor = sk.end();
    }
    out
}

fn build_tnode<W: WorkSink>(
    sk: &Skel,
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Arc<TNode> {
    built.fnodes += 1;
    let start = sk.start();
    let size = sk.end() - start;
    let mut marker = None;
    let (kind, payload, children): (NodeKind, TPayload, Vec<(usize, Arc<TNode>)>) = match sk {
        Skel::Para { segments, .. } => (
            NodeKind::Paragraph,
            TPayload::Para {
                inline: rebased(scan_inlines_abs(src, segments, table, sink), start),
                segments_rel: rebase_segments(segments, start),
            },
            Vec::new(),
        ),
        Skel::Heading { level, content, .. } => (
            NodeKind::Heading,
            TPayload::Heading {
                level: *level,
                content_rel: (content.0 - start, content.1 - start),
                inline: rebased(
                    scan_inlines_abs(src, std::slice::from_ref(content), table, sink),
                    start,
                ),
            },
            Vec::new(),
        ),
        Skel::Fence { info, content, .. } => (
            NodeKind::FencedCode,
            TPayload::Fence {
                info: info.clone(),
                content_rel: (content.0 - start, content.1 - start),
            },
            Vec::new(),
        ),
        Skel::Def {
            label, destination, ..
        } => (
            NodeKind::ReferenceDefinition,
            TPayload::Def {
                label: label.clone(),
                destination: destination.clone(),
            },
            Vec::new(),
        ),
        Skel::Quote { children, .. } => (
            NodeKind::BlockQuote,
            TPayload::Container,
            build_level(children, src, table, start, built, sink),
        ),
        Skel::List { items, .. } => (
            NodeKind::List,
            TPayload::Container,
            build_level(items, src, table, start, built, sink),
        ),
        Skel::Item {
            marker: m,
            children,
            ..
        } => {
            marker = Some(*m);
            (
                NodeKind::ListItem,
                TPayload::Container,
                build_level(children, src, table, start, built, sink),
            )
        }
        Skel::Spliced { .. } => unreachable!("initial construction has no splices"),
    };
    built.nodes += 1 + payload_inline_nodes(&payload);
    let has_ref = payload_has_ref(&payload) || children.iter().any(|(_, c)| c.has_ref);
    let mut def_facts = Vec::new();
    if kind == NodeKind::ReferenceDefinition {
        if let TPayload::Def { label, destination } = &payload {
            def_facts.push((label.clone(), destination.clone()));
        }
    }
    for (_, c) in &children {
        def_facts.extend(c.def_facts.iter().cloned());
    }
    let has_def = kind == NodeKind::ReferenceDefinition || children.iter().any(|(_, c)| c.has_def);
    Arc::new(TNode {
        kind,
        size,
        // The line-offset derivation is a mechanism source read: reported
        // (R5-CORRECTIVE-2).
        line_offset: start - sg::parser::line_start_of_reported(src, start, sink),
        changed: false,
        marker,
        children,
        ctx: sk.ctx().clone(),
        has_ref,
        has_def,
        def_facts,
        payload,
    })
}

fn build_level<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    level_start: usize,
    built: &mut Built,
    sink: &mut W,
) -> Vec<(usize, Arc<TNode>)> {
    blocks
        .iter()
        .map(|sk| {
            (
                sk.start() - level_start,
                build_tnode(sk, src, table, built, sink),
            )
        })
        .collect()
}

/// A re-materialized retained subtree plus its honest native-node
/// accounting.
struct Rematerialized {
    node: Arc<TNode>,
    /// Native nodes reconstructed (this node + re-scanned inline syntax).
    rebuilt: u64,
    /// Native nodes that kept their `Arc` identity (reference-insensitive
    /// descendants).
    kept: u64,
}

/// Whether a retained subtree's materialized inline payload could carry
/// a reference resolution.
///
/// Sound over-approximation: a reference link — resolved OR resolvable —
/// always contains a `[` in its source bytes, so a subtree whose span has
/// no `[` cannot change meaning when the definition table changes. The
/// `has_ref` subtree fact short-circuits the common case with no read at
/// all; otherwise the probe reads the subtree's own bytes and reports
/// every inspected range (R5-CORRECTIVE-2).
fn mentions_reference<W: WorkSink>(node: &TNode, post: &[u8], base: usize, sink: &mut W) -> bool {
    if node.has_ref {
        return true;
    }
    let end = (base + node.size).min(post.len());
    if base >= end {
        return false;
    }
    sink.record_source_inspection(
        markit_mdbench_common::SourceVersion::Post,
        base as u64,
        end as u64,
    );
    post[base..end].contains(&b'[')
}

/// Re-materialize a retained subtree's reference-sensitive inline
/// payloads against `table`, keeping every reference-insensitive
/// descendant's `Arc` identity.
///
/// Only the PAYLOAD is reconstructed (the block structure — kind, size,
/// line offset, context key, children layout, definition facts, changed
/// flag — is retained): the paragraph's retained content segments are
/// re-scanned with the new first-wins table. This is the "rebuild
/// affected dependency consumers" repair, not a reparse and not a full
/// rebuild.
fn rematerialize<W: WorkSink>(
    node: &Arc<TNode>,
    post: &[u8],
    base: usize,
    table: &sg::RefTable,
    sink: &mut W,
) -> Rematerialized {
    let mut rebuilt = 1; // this node is reconstructed
    let payload = match &node.payload {
        TPayload::Para { segments_rel, .. } => {
            let segments: Vec<(usize, usize)> = segments_rel
                .iter()
                .map(|(a, b)| (base + a, base + b))
                .collect();
            let inline = rebased(scan_inlines_abs(post, &segments, table, sink), base);
            rebuilt += count_forest(&inline);
            TPayload::Para {
                inline,
                segments_rel: segments_rel.clone(),
            }
        }
        TPayload::Heading {
            level, content_rel, ..
        } => {
            let content = (base + content_rel.0, base + content_rel.1);
            let inline = rebased(
                scan_inlines_abs(post, std::slice::from_ref(&content), table, sink),
                base,
            );
            rebuilt += count_forest(&inline);
            TPayload::Heading {
                level: *level,
                content_rel: *content_rel,
                inline,
            }
        }
        TPayload::Fence { info, content_rel } => TPayload::Fence {
            info: info.clone(),
            content_rel: *content_rel,
        },
        TPayload::Def { label, destination } => TPayload::Def {
            label: label.clone(),
            destination: destination.clone(),
        },
        TPayload::Container => TPayload::Container,
    };
    let mut kept = 0u64;
    let mut children = Vec::with_capacity(node.children.len());
    for (rel, child) in &node.children {
        let child_base = base + rel;
        if mentions_reference(child, post, child_base, sink) {
            let sub = rematerialize(child, post, child_base, table, sink);
            rebuilt += sub.rebuilt;
            kept += sub.kept;
            children.push((*rel, sub.node));
        } else {
            kept += count_node(child);
            children.push((*rel, child.clone()));
        }
    }
    let has_ref = payload_has_ref(&payload) || children.iter().any(|(_, c)| c.has_ref);
    let node = Arc::new(TNode {
        kind: node.kind,
        size: node.size,
        line_offset: node.line_offset,
        changed: node.changed,
        marker: node.marker,
        children,
        ctx: node.ctx.clone(),
        has_ref,
        has_def: node.has_def,
        def_facts: node.def_facts.clone(),
        payload,
    });
    Rematerialized {
        node,
        rebuilt,
        kept,
    }
}

/// Scan inline content for one segment list with absolute spans,
/// reporting every scanned segment to `sink` (R5-CORRECTIVE-1, MAJOR-2).
fn scan_inlines_abs<W: WorkSink>(
    src: &[u8],
    segments: &[(usize, usize)],
    table: &sg::RefTable,
    sink: &mut W,
) -> Vec<Node> {
    sg::inline::scan_inlines_with_sink(src, segments, true, table, sink)
}

fn rebased(nodes: Vec<Node>, block_start: usize) -> Vec<Node> {
    shift_forest(nodes, -(block_start as isize))
}

fn shift_forest(nodes: Vec<Node>, delta: isize) -> Vec<Node> {
    let sh = |p: usize| (p as isize + delta) as usize;
    nodes
        .into_iter()
        .map(|mut n| {
            n.start = sh(n.start);
            n.end = sh(n.end);
            n.children = shift_forest(n.children, delta);
            n
        })
        .collect()
}

fn payload_has_ref(p: &TPayload) -> bool {
    fn forest_has_ref(nodes: &[Node]) -> bool {
        nodes
            .iter()
            .any(|n| n.kind == NodeKind::ReferenceLink || forest_has_ref(&n.children))
    }
    match p {
        TPayload::Para { inline, .. } => forest_has_ref(inline),
        TPayload::Heading { inline, .. } => forest_has_ref(inline),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Assembly (parse skeleton + takes -> new tree)
// ---------------------------------------------------------------------------

/// Rebuild the document-global first-wins definition table in document
/// order from the assembled skeleton: fresh Def entries and taken runs'
/// recorded facts.
fn rebuilt_table(blocks: &[Skel], takes: &[TakeRun]) -> sg::RefTable {
    let mut table = sg::RefTable::new();
    fn walk(skels: &[Skel], takes: &[TakeRun], table: &mut sg::RefTable) {
        for sk in skels {
            match sk {
                Skel::Def {
                    label, destination, ..
                } => {
                    table.define(label.clone(), destination.clone());
                }
                Skel::Spliced { slot, .. } => {
                    if let Some(run) = takes.get(*slot as usize) {
                        for (_, m) in &run.members {
                            for (l, d) in &m.def_facts {
                                table.define(l.clone(), d.clone());
                            }
                        }
                    }
                }
                Skel::Quote { children, .. } => walk(children, takes, table),
                Skel::List { items, .. } => walk(items, takes, table),
                Skel::Item { children, .. } => walk(children, takes, table),
                _ => {}
            }
        }
    }
    walk(blocks, takes, &mut table);
    table
}

#[allow(clippy::too_many_arguments)]
fn assemble_entries<W: WorkSink>(
    blocks: &[Skel],
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Vec<TEntry> {
    let leveled = assemble_level(
        blocks,
        takes,
        post,
        table,
        reference_environment_changed,
        built,
        sink,
    );
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for (start, node) in leveled {
        let size = node.size;
        out.push(TEntry {
            gap: start - cursor,
            node,
        });
        cursor = start + size;
    }
    out
}

/// Assemble one container level: fresh Skels convert to fresh TNodes
/// (fresh inline scans reported to `sink`); Spliced placeholders expand
/// to their shared member runs (new member positions derive from the
/// line-start placeholder position plus the members' internal layout —
/// unchanged bytes, and the members' retained inline payloads are reused
/// without any source read).
#[allow(clippy::too_many_arguments)]
fn assemble_level<W: WorkSink>(
    blocks: &[Skel],
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Vec<(usize, Arc<TNode>)> {
    let mut out = Vec::new();
    for sk in blocks {
        match sk {
            Skel::Spliced { slot, start, .. } => {
                let run = &takes[*slot as usize];
                debug_assert_eq!(*start, run.pos);
                for (member_start, node) in &run.members {
                    // Member positions derive from the placeholder (line)
                    // start plus the unchanged internal layout.
                    let new_start = run.pos + (member_start - run.pos);
                    debug_assert!(new_start >= *start);
                    // A retained member whose payload could carry a
                    // reference resolution is only valid while the
                    // definition environment it was materialized
                    // against still holds; otherwise its payload is
                    // re-materialized against the rebuilt table (the
                    // block STRUCTURE stays shared — no reparse).
                    if reference_environment_changed
                        && mentions_reference(node, post, new_start, sink)
                    {
                        let rebuilt = rematerialize(node, post, new_start, table, sink);
                        built.nodes += rebuilt.rebuilt;
                        built.rematerialized_members += count_node(node);
                        built.rematerialized_kept += rebuilt.kept;
                        out.push((new_start, rebuilt.node));
                    } else {
                        out.push((new_start, node.clone()));
                    }
                }
            }
            other => out.push((
                other.start(),
                assemble_tnode(
                    other,
                    takes,
                    post,
                    table,
                    reference_environment_changed,
                    built,
                    sink,
                ),
            )),
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn assemble_tnode<W: WorkSink>(
    sk: &Skel,
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Arc<TNode> {
    built.fnodes += 1;
    let start = sk.start();
    let size = sk.end() - start;
    let mut marker = None;
    let (kind, payload, children): (NodeKind, TPayload, Vec<(usize, Arc<TNode>)>) = match sk {
        Skel::Para { segments, .. } => (
            NodeKind::Paragraph,
            TPayload::Para {
                inline: rebased(scan_inlines_abs(post, segments, table, sink), start),
                segments_rel: rebase_segments(segments, start),
            },
            Vec::new(),
        ),
        Skel::Heading { level, content, .. } => (
            NodeKind::Heading,
            TPayload::Heading {
                level: *level,
                content_rel: (content.0 - start, content.1 - start),
                inline: rebased(
                    scan_inlines_abs(post, std::slice::from_ref(content), table, sink),
                    start,
                ),
            },
            Vec::new(),
        ),
        Skel::Fence { info, content, .. } => (
            NodeKind::FencedCode,
            TPayload::Fence {
                info: info.clone(),
                content_rel: (content.0 - start, content.1 - start),
            },
            Vec::new(),
        ),
        Skel::Def {
            label, destination, ..
        } => (
            NodeKind::ReferenceDefinition,
            TPayload::Def {
                label: label.clone(),
                destination: destination.clone(),
            },
            Vec::new(),
        ),
        Skel::Quote { children, .. } => (
            NodeKind::BlockQuote,
            TPayload::Container,
            assemble_level(
                children,
                takes,
                post,
                table,
                reference_environment_changed,
                built,
                sink,
            ),
        ),
        Skel::List { items, .. } => (
            NodeKind::List,
            TPayload::Container,
            assemble_level(
                items,
                takes,
                post,
                table,
                reference_environment_changed,
                built,
                sink,
            ),
        ),
        Skel::Item {
            marker: m,
            children,
            ..
        } => {
            marker = Some(*m);
            (
                NodeKind::ListItem,
                TPayload::Container,
                assemble_level(
                    children,
                    takes,
                    post,
                    table,
                    reference_environment_changed,
                    built,
                    sink,
                ),
            )
        }
        Skel::Spliced { .. } => unreachable!("splices expand at the level above"),
    };
    built.nodes += 1 + payload_inline_nodes(&payload);
    // Relativize child positions to this node's span start.
    let children: Vec<(usize, Arc<TNode>)> = children
        .into_iter()
        .map(|(abs, node)| (abs - start, node))
        .collect();
    let has_ref = payload_has_ref(&payload) || children.iter().any(|(_, c)| c.has_ref);
    let mut def_facts = Vec::new();
    if kind == NodeKind::ReferenceDefinition {
        if let TPayload::Def { label, destination } = &payload {
            def_facts.push((label.clone(), destination.clone()));
        }
    }
    for (_, c) in &children {
        def_facts.extend(c.def_facts.iter().cloned());
    }
    let has_def = kind == NodeKind::ReferenceDefinition || children.iter().any(|(_, c)| c.has_def);
    Arc::new(TNode {
        kind,
        size,
        line_offset: start - sg::parser::line_start_of_reported(post, start, sink),
        changed: false,
        marker,
        children,
        ctx: sk.ctx().clone(),
        has_ref,
        has_def,
        def_facts,
        payload,
    })
}

// ---------------------------------------------------------------------------
// Projection (tree -> NORMALIZED-RESULT-v1, pure traversal)
// ---------------------------------------------------------------------------

fn project(tree: &TTree) -> NormalizedDocument {
    let mut children = Vec::new();
    let mut cursor = 0usize;
    for entry in &tree.entries {
        let start = cursor + entry.gap;
        children.push(project_node(&entry.node, start));
        cursor = start + entry.node.size;
    }
    let mut root = Node::new(NodeKind::Document, 0, tree.src_len);
    root.children = children;
    NormalizedDocument::new(root)
}

fn project_node(node: &TNode, base: usize) -> Node {
    let mut n = Node::new(node.kind, base, base + node.size);
    if let Some(m) = node.marker {
        n.marker = Some(if m == b'-' { "-" } else { "*" }.to_string());
    }
    match &node.payload {
        TPayload::Para { inline, .. } => {
            n.children = shift_forest(inline.clone(), base as isize);
        }
        TPayload::Heading { level, inline, .. } => {
            n.level = Some(*level);
            n.children = shift_forest(inline.clone(), base as isize);
        }
        TPayload::Fence { info, content_rel } => {
            n.info = Some(info.clone());
            n.content = Some((base + content_rel.0, base + content_rel.1));
        }
        TPayload::Def { label, destination } => {
            n.label = Some(label.clone());
            n.destination = Some(destination.clone());
        }
        TPayload::Container => {
            n.children = node
                .children
                .iter()
                .map(|(rel, c)| project_node(c, base + rel))
                .collect();
        }
    }
    n
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ref_table(defs: &[(String, String)]) -> sg::RefTable {
    let mut t = sg::RefTable::new();
    t.extend_from(defs);
    t
}
