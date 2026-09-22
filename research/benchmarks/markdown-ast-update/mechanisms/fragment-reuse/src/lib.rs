//! markit-mdbench-fragment-reuse — H2 FRAGMENT_REUSE (#22, stage R5).
//!
//! Mechanism identity (frozen in
//! `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §7; lezer-inspired —
//! @lezer/common `TreeFragment.applyChanges` lifecycle with openStart/
//! openEnd edges, offset deltas and minGap, plus the @lezer/markdown
//! FragmentCursor block-run reuse shape): retain the old tree as
//! parent-relative `Arc<FNode>` block nodes plus a fragment table; map
//! the edit through the fragment table (split / drop-below-minGap /
//! shift / open edges); parse the post source line-driven, consulting a
//! cursor at every block-start line — whole old BLOCK runs whose start
//! aligns with the live position, whose entry `ContextKey` vouches the
//! live container/fence state, and which fit the fragment's safe window
//! are taken whole (shared Arc identity); everything else parses
//! normally. Reuse authority comes ONLY from the fragment table.
//!
//! Soundness rests on the termination-input argument: a taken run entered
//! the live parse under a vouched-identical `ContextKey`, consists of
//! unchanged bytes inside an undamaged fragment window, and ends at an
//! old tree block boundary — so the deterministic BENCH-GRAMMAR-v1 line
//! loop reproduces the old parse's boundary decisions inside and after
//! the run. The one block adjacent to each open edge is excluded from
//! reuse (its termination consumed bytes the edit changed). Reuse
//! failure is natural degradation (the line parses normally), never a
//! fallback — `fallback_to_full_count` is NotApplicable to H2.
//!
//! This crate owns ONLY H2 mechanism state and policy. Grammar semantics
//! live in `markit-mdbench-shared-grammar` (the splice hook is
//! horse-supplied — the scanner never decides reuse); the result
//! vocabulary and validation in `markit-mdbench-oracle`. H2 never calls
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
pub const H2_MECHANISM_ID: &str = "fragment-reuse-h2";

/// A surviving fragment piece shorter than `MIN_GAP` bytes adjacent to a
/// change is dropped entirely.
///
/// PRIOR_ART_ANCHORED_PRE_MEASUREMENT_CONSTANT: 128 is the frozen
/// @lezer/common `TreeFragment` default minGap, adopted BEFORE any
/// measurement; it is never tuned from R5 runtime data.
pub const MIN_GAP: usize = 128;

// ---------------------------------------------------------------------------
// Retained native state (parent-relative; absolute positions derive at
// read time — the H2 position strategy, R2-H09 class "fragment offset +
// parent-relative")
// ---------------------------------------------------------------------------

/// Leaf payload of a block node. Inline content is MATERIALIZED at node
/// construction (resolved against the then-current document table) and
/// stored with spans relative to the node's own start.
#[derive(Debug, Clone)]
pub enum FPayload {
    /// Scanned inline nodes (relative spans), links enabled.
    Para {
        inline: Vec<Node>,
        /// The paragraph's content intervals, node-relative (build
        /// metadata, `Skel::Para.segments` rebased). Retained so the
        /// payload can be RE-MATERIALIZED against a changed document
        /// definition table without reparsing the block (see the
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
/// `rel_start` relative to THIS node's span start.
#[derive(Debug, Clone)]
pub struct FNode {
    pub kind: NodeKind,
    pub size: usize,
    /// Distance from the block's LINE start to its span start (the
    /// container-prefix / indentation width on its first line). Alignment
    /// matches LINE starts, because blocks nested in containers start
    /// mid-line.
    pub line_offset: usize,
    pub marker: Option<u8>,
    pub children: Vec<(usize, Arc<FNode>)>,
    /// Entry `ContextKey` captured when the block STARTED (the vouching
    /// key, R5 freeze §2).
    pub ctx: ContextKey,
    /// Subtree facts: contains a ReferenceLink / a ReferenceDefinition.
    pub has_ref: bool,
    pub has_def: bool,
    /// Definitions recorded in this subtree, in document order.
    pub def_facts: Vec<(String, String)>,
    pub payload: FPayload,
}

/// One top-level slot: `gap` blank bytes precede the node.
#[derive(Debug, Clone)]
pub struct FSlot {
    pub gap: usize,
    pub node: Arc<FNode>,
}

/// The retained old tree: top-level slots under a Document wrapper (the
/// root is never reused) plus the retained definition facts.
#[derive(Debug, Clone, Default)]
pub struct FTree {
    pub slots: Vec<FSlot>,
    pub defs: Vec<(String, String)>,
    pub src_len: usize,
}

impl FTree {
    /// Number of block nodes in the tree (recursive).
    pub fn node_count(&self) -> u64 {
        self.slots.iter().map(|s| count_node(&s.node)).sum()
    }
}

/// Native node count of one retained subtree under the corrective
/// counting rule (R5-CORRECTIVE-1 §8): the FNode itself PLUS every
/// retained inline syntax node in its payload (recursive through the
/// whole block and inline structure). Reused subtrees are shared
/// `Arc`s — their inline payloads ride with the identity, zero parser
/// source reads.
fn count_node(n: &FNode) -> u64 {
    1 + payload_inline_nodes(&n.payload)
        + n.children.iter().map(|(_, c)| count_node(c)).sum::<u64>()
}

/// Recursive count of inline syntax nodes stored in one payload.
fn payload_inline_nodes(p: &FPayload) -> u64 {
    match p {
        FPayload::Para { inline, .. } => count_forest(inline),
        FPayload::Heading { inline, .. } => count_forest(inline),
        _ => 0,
    }
}

/// Recursive count of an inline node forest (each node once).
fn count_forest(nodes: &[Node]) -> u64 {
    nodes.iter().map(|n| 1 + count_forest(&n.children)).sum()
}

/// One fragment range in UPDATED-document coordinates; `to_old` maps a
/// position inside the fragment to old-tree coordinates
/// (`old = new + to_old`). Open edges face the change.
#[derive(Debug, Clone)]
pub struct Fragment {
    pub new_start: usize,
    pub new_end: usize,
    pub to_old: isize,
    pub open_start: bool,
    pub open_end: bool,
}

/// H2 retained state: the old tree plus the fragment table.
#[derive(Debug, Clone)]
pub struct H2State {
    tree: FTree,
    fragments: Vec<Fragment>,
}

impl H2State {
    pub fn tree(&self) -> &FTree {
        &self.tree
    }

    pub fn fragment_count(&self) -> usize {
        self.fragments.len()
    }
}

/// Completed-state normalization (R5-CORRECTIVE-1, MAJOR-3): the pure
/// `project` traversal over the retained tree — no source, no sink, no
/// parser. The retained payloads carry every inline syntax fact.
impl NormalizeV1 for H2State {
    fn normalize_v1(&self) -> NormalizedDocument {
        project(&self.tree)
    }
}

/// Pending work handed to `complete()` — the complete new tree and
/// fragment table (eager completion boundary, R5 freeze §4).
///
/// MEASUREMENT-CORRECTIVE-1 §9: the normalized projection (`project`, a
/// pure traversal over retained payloads) is experiment/oracle EXPORT,
/// not H2 mechanism state — it is derived at the runner's post-timer
/// export boundary via `NormalizeV1`, never inside the timed update.
pub struct H2Pending {
    state: H2State,
}

/// `prepare_update` product: the fragment table after the applyChanges
/// mapping plus the edit coordinates (R5 freeze §7 DAMAGE rule).
#[derive(Debug, Clone)]
pub struct H2Prepared {
    edit_start: usize,
    edit_end: usize,
    delta: isize,
    fragments: Vec<Fragment>,
}

// ---------------------------------------------------------------------------
// Mechanism
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct FragmentReuseMechanism;

impl FragmentReuseMechanism {
    pub fn new() -> Self {
        Self
    }

    /// Clean parse into H2 state + result: block pass over the shared
    /// grammar, node construction with the completed definition table,
    /// one whole-document fragment (the Lezer addTree lifecycle).
    fn parse_into_pending<W: WorkSink>(
        &self,
        src: &[u8],
        cx: &mut MechanismContext<'_, W>,
    ) -> H2Pending {
        let rp = sg::parse_region(src, 0, src.len(), cx.sink);
        let defs = rp.defs.entries().to_vec();
        let table = ref_table(&defs);
        let mut built = Built::default();
        let slots = build_slots(&rp.blocks, src, &table, &mut built, cx.sink);
        cx.sink.add_blocks_reparsed(built.fnodes);
        cx.sink.add_nodes_rebuilt(built.nodes);
        cx.sink.add_nodes_reused(0);
        cx.sink.add_metadata_records_touched(slots.len() as u64);
        let tree = FTree {
            slots,
            defs,
            src_len: src.len(),
        };
        H2Pending {
            state: H2State {
                fragments: vec![Fragment {
                    new_start: 0,
                    new_end: src.len(),
                    to_old: 0,
                    open_start: false,
                    open_end: false,
                }],
                tree,
            },
        }
    }
}

impl Mechanism for FragmentReuseMechanism {
    type State = H2State;
    type Prepared = H2Prepared;
    type Pending = H2Pending;

    fn id(&self) -> MechanismId {
        MechanismId(H2_MECHANISM_ID.to_string())
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
        _old_source: &Source,
        _post_source: &Source,
        edit: &CanonicalEdit,
        _old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        let es = edit.start_byte() as usize;
        let ee = edit.end_byte() as usize;
        let delta = edit.inserted_text_len_bytes() as isize - (ee - es) as isize;
        // applyChanges: the fragment covering the edit is split; the
        // edited span is dropped; later coordinates shift by delta; the
        // edges adjacent to the change are marked open; a surviving piece
        // shorter than MIN_GAP adjacent to the change is dropped entirely.
        // The left-of-cut piece is registered here; the right-of-cut
        // piece needs the old source length and is completed in `update`.
        let mut fragments = Vec::new();
        if es >= MIN_GAP {
            fragments.push(Fragment {
                new_start: 0,
                new_end: es,
                to_old: 0,
                open_start: false,
                open_end: true,
            });
        }
        cx.sink.add_metadata_records_touched(1); // the covering fragment record mapped
        Ok(H2Prepared {
            edit_start: es,
            edit_end: ee,
            delta,
            fragments,
        })
    }

    fn update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        _edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let old = old_source.as_bytes();
        let post = post_source.as_bytes();
        let es = prepared.edit_start;
        let ee = prepared.edit_end;
        let delta = prepared.delta;
        // Post-edit coordinates of the edited span: [es, ee_new), where
        // ee_new = es + inserted byte length.
        let ee_new = es + ee_new_len(delta, ee - es);
        debug_assert_eq!(post.len() as i64, old.len() as i64 + delta as i64);
        declare_not_applicable(cx);

        // Complete the fragment table with the right-of-cut piece.
        let mut fragments = prepared.fragments;
        if old.len() - ee >= MIN_GAP {
            fragments.push(Fragment {
                new_start: ee_new,
                new_end: (old.len() as isize + delta) as usize,
                to_old: -delta,
                open_start: true,
                open_end: false,
            });
        }
        cx.sink.add_metadata_records_touched(fragments.len() as u64);

        // Safe windows (open-edge rule): at an OPEN edge the one adjacent
        // whole block is excluded from reuse. The left edge's paragraph
        // margin reads source bytes: report them (R5-CORRECTIVE-2).
        let left_window_end = left_window_end(&old_state.tree, old, es, cx.sink);
        let right_window_start = right_window_start(&old_state.tree, ee, old.len());

        // Broad conservative reference invalidation (R5 freeze §7): the
        // update is definition-changing when a damaged old block contains
        // a ReferenceDefinition, or the reparsed region creates one.
        // (Pre-computable and source-derived: any `]: ` occurrence inside
        // the edited span flags the region — a refdef line carries that
        // sequence at any container depth.) The probe reads the edited
        // span: report the scan (R5-CORRECTIVE-2).
        let damaged_has_def = any_def_in_range(&old_state.tree, es, ee);
        cx.sink.record_source_inspection(
            markit_mdbench_common::SourceVersion::Post,
            es as u64,
            ee_new as u64,
        );
        let region_may_create_def = post[es..ee_new].windows(3).any(|w| w == b"]: ");
        let definition_changing = damaged_has_def || region_may_create_def;

        // Line-driven parse with the fragment cursor. The hook NEVER
        // decides grammar — it only executes takes the cursor vouched.
        let mut cursor = Cursor {
            fragments: &fragments,
            tree: &old_state.tree,
            post,
            ee_new,
            left_window_end,
            right_window_start,
            old_len: old.len(),
            definition_changing,
            takes: Vec::new(),
            consultations: 0,
            reused: 0,
            margin_checks: Vec::new(),
        };
        let (rp, slot_count, takes, consultations, reused, margin_checks) = {
            let mut hook: Box<SpliceHook<'_>> = Box::new(|pos, key| cursor.consult(pos, key));
            let (rp, slot_count) = parse_region_with_hook(post, 0, post.len(), cx.sink, &mut hook);
            drop(hook); // end the cursor borrow before reading the take record
            (
                rp,
                slot_count,
                cursor.takes,
                cursor.consultations,
                cursor.reused,
                cursor.margin_checks,
            )
        };
        for (a, b) in &margin_checks {
            cx.sink
                .record_source_inspection(markit_mdbench_common::SourceVersion::Post, *a, *b);
        }

        // Rebuild the document-global first-wins table from the assembled
        // structure (surviving fragments' recorded facts + new facts), in
        // document order. Every surviving definition appears in the
        // skeleton (fresh) or in a taken run's recorded facts, so this
        // table is complete.
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
        let reference_environment_changed = table.entries() != old_state.tree.defs.as_slice();
        if reference_environment_changed {
            cx.sink.add_metadata_records_touched(1);
        }

        // Assemble the new tree: fresh FNodes for parsed material, shared
        // Arcs for taken runs (containers wrapping takes are rebuilt; the
        // run members keep their identity — their retained inline payloads
        // are reused with ZERO parser source reads).
        let mut built = Built::default();
        let slots = assemble_slots(
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
        cx.sink
            .add_metadata_records_touched(consultations + slot_count as u64);

        let tree = FTree {
            slots,
            defs: table.entries().to_vec(),
            src_len: post.len(),
        };
        Ok(H2Pending {
            state: H2State {
                fragments: vec![Fragment {
                    new_start: 0,
                    new_end: post.len(),
                    to_old: 0,
                    open_start: false,
                    open_end: false,
                }],
                tree,
            },
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

/// Post-timer experiment export (MEASUREMENT-CORRECTIVE-1): the H2
/// checksum is the checksum of the pure `NormalizeV1` projection over
/// the retained tree.
impl markit_mdbench_common::ResultChecksum for H2State {
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

fn ee_new_len(delta: isize, removed: usize) -> usize {
    (removed as isize + delta) as usize
}

// ---------------------------------------------------------------------------
// Safe windows (the open-edge exclusion)
// ---------------------------------------------------------------------------

/// LEFT window: reuse must end before the old-tree block adjacent to the
/// cut. That is the deepest block containing the last byte before the
/// cut; when that byte falls in a blank gap, the last block ending at or
/// before the cut is still excluded (conservative).
fn left_window_end<W: WorkSink>(tree: &FTree, old: &[u8], es: usize, sink: &mut W) -> usize {
    if es == 0 {
        return 0;
    }
    if let Some((boundary_start, _boundary)) = deepest_node_containing(tree, es - 1) {
        // LEFT-EDGE PARAGRAPH MARGIN (the H1-F2 / H4-restart-margin
        // analogue on the left fragment; found by the adversarial
        // small-model generator): the retained block ending at the
        // window edge can paragraph-continue into the reparsed region
        // when no blank line separates it from the boundary block. That
        // edge is safe only while the boundary block's FIRST LINE still
        // interrupts — true when its bytes are untouched (an interrupting
        // dispatch is determined by the unchanged line prefix). When the
        // edit reaches into that first line, the boundary may have
        // vanished (e.g. the edit broke a ``` fence opener into paragraph
        // text): the left fragment is dropped entirely and the merge
        // happens inside the live parse. The margin's backward scan and
        // terminator scan read source bytes: reported exactly
        // (R5-CORRECTIVE-2); the blank check's span is covered by the
        // backward-scan report.
        if boundary_start >= 2 && node_kind_at(tree, boundary_start - 2) == NodeKind::Paragraph {
            // These scans read the OLD source: reported under
            // `SourceVersion::Old` (MEASUREMENT-CORRECTIVE-1 §18).
            let prev_ls = sg::parser::line_start_of_reported_in(
                markit_mdbench_common::SourceVersion::Old,
                old,
                boundary_start - 1,
                sink,
            );
            let blank_before = sg::parser::all_spaces(old, prev_ls, boundary_start - 1);
            let first_lf = sg::parser::memchr_lf_reported_in(
                markit_mdbench_common::SourceVersion::Old,
                old,
                boundary_start,
                sink,
            );
            if !blank_before && es < first_lf {
                return 0;
            }
        }
        return boundary_start;
    }
    let mut best = 0usize;
    let mut cursor = 0usize;
    for slot in &tree.slots {
        let start = cursor + slot.gap;
        if start + slot.node.size <= es {
            best = start;
        }
        cursor = start + slot.node.size;
    }
    best
}

/// RIGHT window: reuse must start at or after the end of the old-tree
/// block adjacent to the cut (the deepest block containing the cut start;
/// when that byte falls in a blank gap, the first block at/after it).
fn right_window_start(tree: &FTree, ee: usize, old_len: usize) -> usize {
    if ee >= old_len {
        return old_len;
    }
    if let Some(end) = block_end_containing(tree, ee) {
        return end;
    }
    let mut cursor = 0usize;
    for slot in &tree.slots {
        let start = cursor + slot.gap;
        if start >= ee {
            return start + slot.node.size;
        }
        cursor = start + slot.node.size;
    }
    old_len
}

// ---------------------------------------------------------------------------
// Fragment cursor (the FragmentCursor.moveTo analogue)
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    fragments: &'a [Fragment],
    tree: &'a FTree,
    /// The post source, for the live-side paragraph margin at take start.
    post: &'a [u8],
    ee_new: usize,
    left_window_end: usize,
    right_window_start: usize,
    old_len: usize,
    definition_changing: bool,
    takes: Vec<TakeRun>,
    consultations: u64,
    reused: u64,
    /// Inspected byte ranges of the consult-time paragraph margins,
    /// buffered during the parse (the cursor borrow excludes the sink)
    /// and reported by the caller afterwards (R5-CORRECTIVE-2). Every
    /// consult that performs the read reports it, pass or fail.
    margin_checks: Vec<(u64, u64)>,
}

/// One taken run: the placeholder covers POST bytes `[pos, pos + len)`;
/// the members are shared old-tree nodes whose old spans tile that range
/// (unchanged bytes, inter-member gaps included).
struct TakeRun {
    pos: usize,
    old_start: usize,
    members: Vec<(usize, Arc<FNode>)>,
}

impl Cursor<'_> {
    /// Consultation at a block-start line: find a vouched run of old-tree
    /// blocks starting exactly here. Returns the take end (exclusive) or
    /// `None` (the line parses normally — natural degradation).
    fn consult(&mut self, pos: usize, key: &ContextKey) -> Option<usize> {
        self.consultations += 1;
        // PARAGRAPH MARGIN at take start (the ContextKey deliberately
        // excludes paragraph state, R5 freeze section 2): the line
        // immediately before the splice must be blank (>= the blank-line
        // separation that terminates every continuation), so no paragraph
        // is open in the live parse. A blank-separated line start is the
        // only splice point where a taken fragment's first block is
        // guaranteed to start a fresh block in the live parse too.
        // Conservative: takes at interruptor lines (legal but
        // para-state-dependent) degrade to reparse instead.
        if pos > 0 {
            let prev_ls = line_start_of(self.post, pos - 1);
            // The margin read happens on every consult that reaches it
            // (pass or fail): buffer the inspected range — the backward
            // scan [prev_ls-1, pos-1) covers the blank check's span.
            self.margin_checks
                .push((prev_ls.saturating_sub(1) as u64, (pos - 1) as u64));
            if !sg::parser::all_spaces(self.post, prev_ls, pos - 1) {
                return None;
            }
        }
        // The fragment whose range covers the live position; positions in
        // the edited span belong to no fragment.
        let frag = self
            .fragments
            .iter()
            .find(|f| pos >= f.new_start && pos < f.new_end)?;
        let p_old = (pos as isize + frag.to_old) as usize;
        let (win_start, win_end) = if pos < self.ee_new {
            (0, self.left_window_end)
        } else {
            (self.right_window_start, self.old_len)
        };
        let run = self.find_run(p_old, key, win_start, win_end)?;
        let new_end = pos + (run.old_end - run.old_start);
        self.reused += run.members.iter().map(|(_, n)| count_node(n)).sum::<u64>();
        self.takes.push(TakeRun {
            pos,
            old_start: run.old_start,
            members: run.members,
        });
        Some(new_end)
    }

    /// Find a run of sibling blocks starting exactly at `p_old`: the
    /// OUTERMOST block starting there whose entry `ContextKey` equals the
    /// live key is the candidate (the root is never reused — the search
    /// starts below the Document wrapper); deeper levels are tried only
    /// through a non-matching shell. The run extends over following
    /// siblings (their inter-block gaps included) while they fit the safe
    /// window and pass the reference clause. Sibling blocks of one parent
    /// share their entry ContextKey in BENCH-GRAMMAR-v1, so vouching the
    /// candidate vouches the run.
    fn find_run(
        &self,
        p_old: usize,
        key: &ContextKey,
        win_start: usize,
        win_end: usize,
    ) -> Option<Run> {
        let mut entries: Vec<(usize, Arc<FNode>)> = Vec::new();
        let mut cursor = 0usize;
        for slot in &self.tree.slots {
            let start = cursor + slot.gap;
            entries.push((start, slot.node.clone()));
            cursor = start + slot.node.size;
        }
        self.search_level(&entries, p_old, key, win_start, win_end)
    }

    fn search_level(
        &self,
        entries: &[(usize, Arc<FNode>)],
        p_old: usize,
        key: &ContextKey,
        win_start: usize,
        win_end: usize,
    ) -> Option<Run> {
        // LINE-aligned matching: a block nested in containers starts
        // mid-line (after its container prefix), so the candidate is the
        // entry whose first LINE starts at the live position.
        let idx = entries
            .iter()
            .position(|(start, n)| start - n.line_offset == p_old)?;
        let (start, node) = &entries[idx];
        if node.ctx != *key {
            // Context mismatch at this level: descend into this node and
            // try the child living on the same line.
            let children: Vec<(usize, Arc<FNode>)> = node
                .children
                .iter()
                .map(|(rel, c)| (start + rel, c.clone()))
                .collect();
            return self.search_level(&children, p_old, key, win_start, win_end);
        }
        // Vouched candidate. Window + reference clause on the whole run
        // (span-based: the run may begin mid-line, after the prefix).
        if *start < win_start || *start + node.size > win_end {
            return None;
        }
        if self.definition_changing && node.has_ref {
            return None;
        }
        let mut members = vec![(*start, node.clone())];
        let mut end = start + node.size;
        for (s2, n2) in entries.iter().skip(idx + 1) {
            if s2 + n2.size > win_end || (self.definition_changing && n2.has_ref) {
                break;
            }
            members.push((*s2, n2.clone()));
            end = s2 + n2.size;
        }
        Some(Run {
            // The take starts at the LINE start; the prefix bytes between
            // the line start and the first member's span are unchanged.
            old_start: p_old,
            old_end: end,
            members,
        })
    }
}

struct Run {
    old_start: usize,
    old_end: usize,
    members: Vec<(usize, Arc<FNode>)>,
}

/// Absolute old-tree span of the deepest block whose span contains `p`.
/// Kind of the deepest block whose span contains `p`.
fn node_kind_at(tree: &FTree, p: usize) -> NodeKind {
    deepest_node_containing(tree, p)
        .map(|(_, n)| n.kind)
        .unwrap_or(NodeKind::Document)
}

/// Absolute old-tree START and NODE of the deepest block whose span
/// contains `p`.
fn deepest_node_containing(tree: &FTree, p: usize) -> Option<(usize, Arc<FNode>)> {
    fn walk(entries: &[(usize, Arc<FNode>)], p: usize) -> Option<(usize, Arc<FNode>)> {
        for (start, node) in entries {
            if *start <= p && p < start + node.size {
                let children: Vec<(usize, Arc<FNode>)> = node
                    .children
                    .iter()
                    .map(|(rel, c)| (start + rel, c.clone()))
                    .collect();
                return walk(&children, p).or(Some((*start, node.clone())));
            }
        }
        None
    }
    fn abs_entries(tree: &FTree) -> Vec<(usize, Arc<FNode>)> {
        let mut entries = Vec::new();
        let mut cursor = 0usize;
        for slot in &tree.slots {
            let start = cursor + slot.gap;
            entries.push((start, slot.node.clone()));
            cursor = start + slot.node.size;
        }
        entries
    }
    walk(&abs_entries(tree), p)
}

/// Absolute old-tree END of the deepest block whose span contains `p`.
fn block_end_containing(tree: &FTree, p: usize) -> Option<usize> {
    fn walk(entries: &[(usize, Arc<FNode>)], p: usize) -> Option<usize> {
        for (start, node) in entries {
            if *start <= p && p < start + node.size {
                let children: Vec<(usize, Arc<FNode>)> = node
                    .children
                    .iter()
                    .map(|(rel, c)| (start + rel, c.clone()))
                    .collect();
                return walk(&children, p).or(Some(start + node.size));
            }
        }
        None
    }
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    for slot in &tree.slots {
        let start = cursor + slot.gap;
        let size = slot.node.size;
        entries.push((start, slot.node.clone()));
        cursor = start + size;
    }
    walk(&entries, p)
}

/// Whether any old-tree block whose span intersects `[a, b)` contains a
/// ReferenceDefinition.
fn any_def_in_range(tree: &FTree, a: usize, b: usize) -> bool {
    fn walk(entries: &[(usize, Arc<FNode>)], a: usize, b: usize) -> bool {
        for (start, node) in entries {
            let end = start + node.size;
            if *start < b && end > a {
                if node.has_def {
                    return true;
                }
                let children: Vec<(usize, Arc<FNode>)> = node
                    .children
                    .iter()
                    .map(|(rel, c)| (start + rel, c.clone()))
                    .collect();
                if walk(&children, a, b) {
                    return true;
                }
            }
        }
        false
    }
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    for slot in &tree.slots {
        let start = cursor + slot.gap;
        entries.push((start, slot.node.clone()));
        cursor = start + slot.node.size;
    }
    walk(&entries, a, b)
}

// ---------------------------------------------------------------------------
// Node construction (fresh parse material -> FNode)
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

/// Build top-level slots from completed blocks (absolute spans in the
/// parsed source). Fresh inline scanning is instrumented: every scanned
/// segment is reported to `sink` (R5-CORRECTIVE-1, MAJOR-2).
fn build_slots<W: WorkSink>(
    blocks: &[Skel],
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Vec<FSlot> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for sk in blocks {
        let start = sk.start();
        out.push(FSlot {
            gap: start - cursor,
            node: build_fnode(sk, src, table, built, sink),
        });
        cursor = sk.end();
    }
    out
}

fn build_fnode<W: WorkSink>(
    sk: &Skel,
    src: &[u8],
    table: &sg::RefTable,
    built: &mut Built,
    sink: &mut W,
) -> Arc<FNode> {
    built.fnodes += 1;
    let start = sk.start();
    let size = sk.end() - start;
    let mut marker = None;
    let (kind, payload, children): (NodeKind, FPayload, Vec<(usize, Arc<FNode>)>) = match sk {
        Skel::Para { segments, .. } => (
            NodeKind::Paragraph,
            FPayload::Para {
                inline: rebased(scan_inlines_abs(src, segments, table, sink), start),
                segments_rel: rebase_segments(segments, start),
            },
            Vec::new(),
        ),
        Skel::Heading { level, content, .. } => (
            NodeKind::Heading,
            FPayload::Heading {
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
            FPayload::Fence {
                info: info.clone(),
                content_rel: (content.0 - start, content.1 - start),
            },
            Vec::new(),
        ),
        Skel::Def {
            label, destination, ..
        } => (
            NodeKind::ReferenceDefinition,
            FPayload::Def {
                label: label.clone(),
                destination: destination.clone(),
            },
            Vec::new(),
        ),
        Skel::Quote { children, .. } => (
            NodeKind::BlockQuote,
            FPayload::Container,
            build_level(children, src, table, start, built, sink),
        ),
        Skel::List { items, .. } => (
            NodeKind::List,
            FPayload::Container,
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
                FPayload::Container,
                build_level(children, src, table, start, built, sink),
            )
        }
        Skel::Spliced { .. } => unreachable!("initial construction has no splices"),
    };
    built.nodes += 1 + payload_inline_nodes(&payload);
    let has_ref = payload_has_ref(&payload) || children.iter().any(|(_, c)| c.has_ref);
    let mut def_facts = Vec::new();
    if kind == NodeKind::ReferenceDefinition {
        if let FPayload::Def { label, destination } = &payload {
            def_facts.push((label.clone(), destination.clone()));
        }
    }
    for (_, c) in &children {
        def_facts.extend(c.def_facts.iter().cloned());
    }
    let has_def = kind == NodeKind::ReferenceDefinition || children.iter().any(|(_, c)| c.has_def);
    Arc::new(FNode {
        kind,
        size,
        // The line-offset derivation is a mechanism source read: reported
        // (R5-CORRECTIVE-2).
        line_offset: start - sg::parser::line_start_of_reported(src, start, sink),
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
) -> Vec<(usize, Arc<FNode>)> {
    blocks
        .iter()
        .map(|sk| {
            (
                sk.start() - level_start,
                build_fnode(sk, src, table, built, sink),
            )
        })
        .collect()
}

/// A re-materialized retained subtree plus its honest native-node
/// accounting.
struct Rematerialized {
    node: Arc<FNode>,
    /// Native nodes reconstructed (this node + re-scanned inline syntax).
    rebuilt: u64,
    /// Native nodes that kept their `Arc` identity (reference-insensitive
    /// descendants, and the children of non-reference payloads).
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
fn mentions_reference<W: WorkSink>(node: &FNode, post: &[u8], base: usize, sink: &mut W) -> bool {
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
/// line offset, context key, children layout, definition facts — is
/// retained): the paragraph's retained content segments are re-scanned
/// with the new first-wins table. This is the "rebuild affected
/// dependency consumers" repair, not a reparse and not a full rebuild.
fn rematerialize<W: WorkSink>(
    node: &Arc<FNode>,
    post: &[u8],
    base: usize,
    table: &sg::RefTable,
    sink: &mut W,
) -> Rematerialized {
    let mut rebuilt = 1; // this node is reconstructed
    let payload = match &node.payload {
        FPayload::Para { segments_rel, .. } => {
            let segments: Vec<(usize, usize)> = segments_rel
                .iter()
                .map(|(a, b)| (base + a, base + b))
                .collect();
            let inline = rebased(scan_inlines_abs(post, &segments, table, sink), base);
            rebuilt += count_forest(&inline);
            FPayload::Para {
                inline,
                segments_rel: segments_rel.clone(),
            }
        }
        FPayload::Heading {
            level, content_rel, ..
        } => {
            let content = (base + content_rel.0, base + content_rel.1);
            let inline = rebased(
                scan_inlines_abs(post, std::slice::from_ref(&content), table, sink),
                base,
            );
            rebuilt += count_forest(&inline);
            FPayload::Heading {
                level: *level,
                content_rel: *content_rel,
                inline,
            }
        }
        FPayload::Fence { info, content_rel } => FPayload::Fence {
            info: info.clone(),
            content_rel: *content_rel,
        },
        FPayload::Def { label, destination } => FPayload::Def {
            label: label.clone(),
            destination: destination.clone(),
        },
        FPayload::Container => FPayload::Container,
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
    let node = Arc::new(FNode {
        kind: node.kind,
        size: node.size,
        line_offset: node.line_offset,
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

/// Shift every span in an inline node forest by `delta` (rebase to
/// block-relative coordinates at build time, to document coordinates at
/// projection time).
/// Rebase absolute content segments to node-relative coordinates.
fn rebase_segments(segments: &[(usize, usize)], start: usize) -> Vec<(usize, usize)> {
    segments
        .iter()
        .map(|(a, b)| (a - start, b - start))
        .collect()
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

fn payload_has_ref(p: &FPayload) -> bool {
    fn forest_has_ref(nodes: &[Node]) -> bool {
        nodes
            .iter()
            .any(|n| n.kind == NodeKind::ReferenceLink || forest_has_ref(&n.children))
    }
    match p {
        FPayload::Para { inline, .. } => forest_has_ref(inline),
        FPayload::Heading { inline, .. } => forest_has_ref(inline),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Assembly (parse skeleton + takes -> new tree)
// ---------------------------------------------------------------------------

/// Rebuild the document-global first-wins definition table in document
/// order from the assembled skeleton: fresh Def entries and taken runs'
/// recorded facts. Every surviving definition appears in one of the two,
/// so the table is complete.
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
fn assemble_slots<W: WorkSink>(
    blocks: &[Skel],
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Vec<FSlot> {
    let entries = assemble_level(
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
    for (start, node) in entries {
        let size = node.size;
        out.push(FSlot {
            gap: start - cursor,
            node,
        });
        cursor = start + size;
    }
    out
}

/// Assemble one container level: fresh Skels convert to fresh FNodes
/// (fresh inline scans reported to `sink`); Spliced placeholders expand
/// to their shared member runs (new member positions derive from the
/// placeholder start plus the members' old internal layout — unchanged
/// bytes, and the members' retained inline payloads are reused without
/// any source read). Returns `(new_start, node)` pairs in order.
#[allow(clippy::too_many_arguments)]
fn assemble_level<W: WorkSink>(
    blocks: &[Skel],
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Vec<(usize, Arc<FNode>)> {
    let mut out = Vec::new();
    for sk in blocks {
        match sk {
            Skel::Spliced { slot, start, .. } => {
                let run = &takes[*slot as usize];
                for (old_start, node) in run.members.iter() {
                    let new_start = run.pos + (old_start - run.old_start);
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
                assemble_fnode(
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
fn assemble_fnode<W: WorkSink>(
    sk: &Skel,
    takes: &[TakeRun],
    post: &[u8],
    table: &sg::RefTable,
    reference_environment_changed: bool,
    built: &mut Built,
    sink: &mut W,
) -> Arc<FNode> {
    built.fnodes += 1;
    let start = sk.start();
    let size = sk.end() - start;
    let mut marker = None;
    let (kind, payload, children): (NodeKind, FPayload, Vec<(usize, Arc<FNode>)>) = match sk {
        Skel::Para { segments, .. } => (
            NodeKind::Paragraph,
            FPayload::Para {
                inline: rebased(scan_inlines_abs(post, segments, table, sink), start),
                segments_rel: rebase_segments(segments, start),
            },
            Vec::new(),
        ),
        Skel::Heading { level, content, .. } => (
            NodeKind::Heading,
            FPayload::Heading {
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
            FPayload::Fence {
                info: info.clone(),
                content_rel: (content.0 - start, content.1 - start),
            },
            Vec::new(),
        ),
        Skel::Def {
            label, destination, ..
        } => (
            NodeKind::ReferenceDefinition,
            FPayload::Def {
                label: label.clone(),
                destination: destination.clone(),
            },
            Vec::new(),
        ),
        Skel::Quote { children, .. } => (
            NodeKind::BlockQuote,
            FPayload::Container,
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
            FPayload::Container,
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
                FPayload::Container,
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
    let children: Vec<(usize, Arc<FNode>)> = children
        .into_iter()
        .map(|(abs, node)| (abs - start, node))
        .collect();
    let has_ref = payload_has_ref(&payload) || children.iter().any(|(_, c)| c.has_ref);
    let mut def_facts = Vec::new();
    if kind == NodeKind::ReferenceDefinition {
        if let FPayload::Def { label, destination } = &payload {
            def_facts.push((label.clone(), destination.clone()));
        }
    }
    for (_, c) in &children {
        def_facts.extend(c.def_facts.iter().cloned());
    }
    let has_def = kind == NodeKind::ReferenceDefinition || children.iter().any(|(_, c)| c.has_def);
    Arc::new(FNode {
        kind,
        size,
        line_offset: start - sg::parser::line_start_of_reported(post, start, sink),
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

fn project(tree: &FTree) -> NormalizedDocument {
    let mut children = Vec::new();
    let mut cursor = 0usize;
    for slot in &tree.slots {
        let start = cursor + slot.gap;
        children.push(project_node(&slot.node, start));
        cursor = start + slot.node.size;
    }
    let mut root = Node::new(NodeKind::Document, 0, tree.src_len);
    root.children = children;
    NormalizedDocument::new(root)
}

fn project_node(node: &FNode, base: usize) -> Node {
    let mut n = Node::new(node.kind, base, base + node.size);
    if let Some(m) = node.marker {
        n.marker = Some(if m == b'-' { "-" } else { "*" }.to_string());
    }
    match &node.payload {
        FPayload::Para { inline, .. } => {
            n.children = shift_forest(inline.clone(), base as isize);
        }
        FPayload::Heading { level, inline, .. } => {
            n.level = Some(*level);
            n.children = shift_forest(inline.clone(), base as isize);
        }
        FPayload::Fence { info, content_rel } => {
            n.info = Some(info.clone());
            n.content = Some((base + content_rel.0, base + content_rel.1));
        }
        FPayload::Def { label, destination } => {
            n.label = Some(label.clone());
            n.destination = Some(destination.clone());
        }
        FPayload::Container => {
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

fn line_start_of(src: &[u8], pos: usize) -> usize {
    src[..pos]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |p| p + 1)
}

fn ref_table(defs: &[(String, String)]) -> sg::RefTable {
    let mut t = sg::RefTable::new();
    t.extend_from(defs);
    t
}
