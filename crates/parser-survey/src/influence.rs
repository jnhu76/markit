//! Influence-vector observation schema (run-1.1 corrective, issue #19
//! §3): one edit's cost split across four axes instead of a single
//! I-class.
//!
//! Every classification below is DERIVED FROM OBSERVED COUNTERS of the
//! measured run (MarkdownWork + state observations), never from the
//! mutation's name. Where the current L1 representation cannot observe a
//! class at all, the schema says so explicitly (`...UNOBSERVABLE`) rather
//! than guessing. Semantic influence is UNKNOWN everywhere in run-1.1:
//! there is no semantic dependency layer to measure yet (run-4 adds one).

use markit_core::markdown::BlockKind;

use crate::text::LineInfo;

// --- Syntax influence (observed reparse radius) ---------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // the full class set is the schema; L1 cannot observe all of it
pub enum SyntaxInfluence {
    /// No block needed a reparse at all (ZERO_REPARSE_CONVERGENCE: parse
    /// work 0 — representation work may still be nonzero, see M-axis).
    S0NoReparse,
    /// Inline-level-only repair. UNOBSERVABLE in P0-02 L1: any touched
    /// block re-parses as a whole, so no run-1.1 row can honestly claim
    /// this class; the variant exists so later representations can.
    S1TokenOrInlineLocal,
    /// Exactly the covering block reparsed; convergence at/near its end.
    S2BlockLocal,
    /// Reparse spilled into the immediate neighbors only (split/merge/
    /// boundary convert: block-count delta or ≤2 blocks).
    S3NeighborLocal,
    /// Reparse stayed inside one container-extent block (quote/list) —
    /// in flat L1 the "container" is the whole flat block, so this is
    /// also the granularity floor E3, not proof of container awareness.
    S4ContainerScoped,
    /// Forward parser state carried the reparse far past the edit (fence
    /// opener/closer corruption): convergence ≫ edit region, restart local.
    S5StatePropagating,
    /// Restart at BOF and ≥ half the document rescanned.
    S6DocumentReparse,
}

impl SyntaxInfluence {
    pub fn name(&self) -> &'static str {
        match self {
            Self::S0NoReparse => "S0_NO_REPARSE",
            Self::S1TokenOrInlineLocal => "S1_TOKEN_OR_INLINE_LOCAL",
            Self::S2BlockLocal => "S2_BLOCK_LOCAL",
            Self::S3NeighborLocal => "S3_NEIGHBOR_LOCAL",
            Self::S4ContainerScoped => "S4_CONTAINER_SCOPED",
            Self::S5StatePropagating => "S5_STATE_PROPAGATING",
            Self::S6DocumentReparse => "S6_DOCUMENT_REPARSE",
        }
    }
}

// --- Semantic influence ----------------------------------------------------

/// Run-1.1 has no semantic dependency layer, so every observation is
/// `DUnknown`. The class set exists because run-4 (ReferenceIndex) will
/// fill it; emitting DUnknown instead of a guess keeps the schema honest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // classes fill in when run-4's ReferenceIndex exists
pub enum SemanticInfluence {
    D0None,
    D1Local,
    D2Neighbor,
    D3Container,
    D4IndexedDependents,
    D5Global,
    /// Not measured: no semantic index exists in this run.
    DUnknown,
}

impl SemanticInfluence {
    pub fn name(&self) -> &'static str {
        match self {
            Self::D0None => "D0_NONE",
            Self::D1Local => "D1_LOCAL",
            Self::D2Neighbor => "D2_NEIGHBOR",
            Self::D3Container => "D3_CONTAINER",
            Self::D4IndexedDependents => "D4_INDEXED_DEPENDENTS",
            Self::D5Global => "D5_GLOBAL",
            Self::DUnknown => "D_UNKNOWN",
        }
    }
}

// --- Representation influence ----------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // M2 arrives with the run-3 green-tree prototype
pub enum RepresentationInfluence {
    /// No metadata moved anywhere (survivor shifts and record moves 0)
    /// and nothing was reparsed.
    M0None,
    /// Reparse produced replacement records locally; nothing outside the
    /// edited region moved.
    M1LocalNodeRewrite,
    /// Ancestor-path rewrite. UNOBSERVABLE in flat L1 (no ancestor
    /// paths exist); kept for the green-tree run-3 comparison.
    M2AncestorPath,
    /// A small number of records near the edit moved (sequence splice)
    /// without a suffix-wide rewrite.
    M3LocalSequenceShift,
    /// Every (or a suffix-proportional share of) survivor block/inline
    /// records had absolute coordinates rewritten — the hidden-O(N) gate.
    M4SuffixMetadataRewrite,
    /// Essentially the whole record set moved (≈ full rebuild of the
    /// sequence, e.g. records-moved ≈ block count).
    M5GlobalRebuild,
}

impl RepresentationInfluence {
    pub fn name(&self) -> &'static str {
        match self {
            Self::M0None => "M0_NONE",
            Self::M1LocalNodeRewrite => "M1_LOCAL_NODE_REWRITE",
            Self::M2AncestorPath => "M2_ANCESTOR_PATH",
            Self::M3LocalSequenceShift => "M3_LOCAL_SEQUENCE_SHIFT",
            Self::M4SuffixMetadataRewrite => "M4_SUFFIX_METADATA_REWRITE",
            Self::M5GlobalRebuild => "M5_GLOBAL_REBUILD",
        }
    }
}

// --- Downstream influence (split axes) ---------------------------------------

/// Which downstream records an edit touches, split by WHY. `P_semantic`
/// counts blocks whose content actually changed (a renderer must
/// re-project them); `P_coordinate` counts survivor records whose source
/// coordinates moved with unchanged content (pure representation cost a
/// position-keyed consumer pays only to re-key). The two can overlap on a
/// reparsed block that also shifted — that block pays both. `P_provider`
/// is UNKNOWN in run-1.1: no provider layer (mermaid renderer, reference
/// resolver) exists in the harness yet.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // split fields are consumed by report queries, not the binary
pub struct DownstreamSplit {
    pub p_semantic_blocks: u64,
    pub p_semantic_bytes: u64,
    pub p_coordinate_block_records: u64,
    pub p_coordinate_inline_nodes: u64,
    pub p_provider: ProviderInfluence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderInfluence {
    Unknown,
    /// The edit touches a mermaid fence body — a future provider
    /// consumer would be invalidated, but none is measured here.
    MermaidCandidate,
}

impl ProviderInfluence {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::MermaidCandidate => "MERMAID_CANDIDATE",
        }
    }
}

// --- The vector --------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct InfluenceVector {
    pub syntax: SyntaxInfluence,
    pub semantic: SemanticInfluence,
    pub representation: RepresentationInfluence,
    pub downstream: DownstreamSplit,
}

// --- Inputs (all observed) ----------------------------------------------------

/// Everything the classifier needs, all measured — nothing derived from
/// the mutation id.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    /// Old-state line index of the first byte of the edit.
    pub edit_start: usize,
    /// Old-state line index one past the last byte removed/replaced by
    /// the edit (for inserts: the line of the insert point).
    pub edit_end_line: usize,
    /// Line footprint of the inserted text itself (0 for pure deletes,
    /// ≥ 1 for multi-line inserts), so a large paste does not masquerade
    /// as forward propagation past its own content.
    pub changed_lines: u64,
    pub doc_bytes: u64,
    pub doc_blocks: u64,
    // MarkdownWork counters.
    pub bytes_scanned: u64,
    pub blocks_reparsed: u64,
    pub blocks_created: u64,
    pub blocks_removed: u64,
    pub survivor_blocks_shifted: u64,
    pub survivor_inline_nodes_shifted: u64,
    pub block_records_moved: u64,
    pub restart_line: u64,
    pub convergence_line: u64,
    // Covering block in the OLD state (contains edit_start), line span.
    pub cover_kind: BlockKind,
    pub cover_end_line: usize,
}

fn is_containerish(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::BlockQuote | BlockKind::UnorderedList | BlockKind::OrderedList
    )
}

/// Derives the syntax class. Rules, in order (first match wins):
///
/// 1. `blocks_reparsed == 0` → S0 (ZERO_REPARSE_CONVERGENCE).
/// 2. `bytes_scanned ≥ doc_bytes/2` → S6 if restart at BOF else S5
///    (half-document scans are state/document events regardless of how
///    few blocks were actually rebuilt — the fence cascade re-builds 3
///    blocks but scans 1 MB).
/// 3. Escape distance: convergence past the farther of (covering block
///    end, edit end + the inserted line footprint) by > 16 lines → S5.
///    16 ≫ synth-neighbor distance (≤ 3 lines) but ≪ document scale;
///    the raw conv_Δ sits next to the class in every table so the
///    threshold is auditable per row.
/// 4. Covering block is a container-extent block and the reparse stayed
///    within it (≤ 2 blocks) → S4 (flat-block caveat above).
/// 5. Neighbor effects (2–4 blocks reparsed, or a small block-count
///    delta ≤ 8) → S3. A large block-count delta is fresh content (a
///    paste), not neighbor repair — radius == changed region → S2.
/// 6. Otherwise → S2.
pub fn classify_syntax(o: &Observed) -> SyntaxInfluence {
    if o.blocks_reparsed == 0 {
        return SyntaxInfluence::S0NoReparse;
    }
    if o.bytes_scanned * 2 >= o.doc_bytes {
        return if o.restart_line <= 1 {
            SyntaxInfluence::S6DocumentReparse
        } else {
            SyntaxInfluence::S5StatePropagating
        };
    }
    let edit_footprint_end =
        o.edit_end_line + o.changed_lines as usize;
    let escape = (o.convergence_line as usize)
        .saturating_sub(o.cover_end_line.max(edit_footprint_end));
    if escape > 16 {
        return SyntaxInfluence::S5StatePropagating;
    }
    if is_containerish(o.cover_kind) && o.blocks_reparsed <= 2 {
        return SyntaxInfluence::S4ContainerScoped;
    }
    let count_delta = o.blocks_created + o.blocks_removed;
    if (2..=4).contains(&o.blocks_reparsed) || (1..=8).contains(&count_delta) {
        return SyntaxInfluence::S3NeighborLocal;
    }
    SyntaxInfluence::S2BlockLocal
}

/// Derives the representation class:
///
/// 1. no shifts AND no record moves → M1 if something was reparsed else
///    M0;
/// 2. shifted ≥ 0.9 × doc_blocks (or record moves ≥ 0.9 × doc_blocks) →
///    M5;
/// 3. shifted ≥ half the suffix-proportional expectation
///    (suffix_fraction × doc_blocks, suffix = bytes after the edit) →
///    M4;
/// 4. else → M3.
///
/// M2 is never produced by flat L1 (no ancestor paths exist).
pub fn classify_representation(o: &Observed) -> RepresentationInfluence {
    let shifted = o.survivor_blocks_shifted + o.survivor_inline_nodes_shifted;
    if shifted == 0 && o.block_records_moved == 0 {
        return if o.blocks_reparsed > 0 {
            RepresentationInfluence::M1LocalNodeRewrite
        } else {
            RepresentationInfluence::M0None
        };
    }
    let n = o.doc_blocks as f64;
    if shifted as f64 >= 0.9 * n || o.block_records_moved as f64 >= 0.9 * n {
        return RepresentationInfluence::M5GlobalRebuild;
    }
    let suffix_fraction =
        (o.doc_bytes.saturating_sub(o.edit_start as u64)) as f64 / o.doc_bytes.max(1) as f64;
    if shifted as f64 >= 0.5 * suffix_fraction * n {
        RepresentationInfluence::M4SuffixMetadataRewrite
    } else {
        RepresentationInfluence::M3LocalSequenceShift
    }
}

/// Full vector for one scenario. `mermaid_candidate` flags edits whose
/// edit point sits inside a mermaid fence (provider invalidation a
/// future provider layer would pay; unmeasured here).
pub fn classify(o: &Observed, mermaid_candidate: bool) -> InfluenceVector {
    InfluenceVector {
        syntax: classify_syntax(o),
        semantic: SemanticInfluence::DUnknown,
        representation: classify_representation(o),
        downstream: DownstreamSplit {
            p_semantic_blocks: 0,
            p_semantic_bytes: 0,
            p_coordinate_block_records: o.survivor_blocks_shifted,
            p_coordinate_inline_nodes: o.survivor_inline_nodes_shifted,
            p_provider: if mermaid_candidate {
                ProviderInfluence::MermaidCandidate
            } else {
                ProviderInfluence::Unknown
            },
        },
    }
}

/// Old-state line index containing byte `off` (clamped to the last line).
pub fn line_of_byte(lines: &[LineInfo], off: usize) -> usize {
    match lines.binary_search_by(|l| {
        if off < l.start {
            std::cmp::Ordering::Greater
        } else if off >= l.end {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }) {
        Ok(i) => i,
        Err(i) => i.min(lines.len().saturating_sub(1)),
    }
}
