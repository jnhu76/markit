//! markit-mdbench-full-rebuild — H0 FULL_REBUILD (the control horse).
//!
//! R4 reference semantics (task contract §2–§3, `ROADMAP.md` R4): for
//! every update H0 ignores all old parse state for semantic reuse,
//! clean-parses the ENTIRE post-edit source, materializes a complete new
//! state, and returns it. It never reuses nodes/fragments/checkpoints,
//! keeps no incremental index, and caches nothing. It is simple,
//! deterministic, eager, total over BENCH-GRAMMAR-v1, and byte-
//! coordinate correct — deliberately not clever and not fast.
//!
//! Boundaries held:
//!
//! - the frozen R1 `Mechanism` contract is implemented as-is; timing is
//!   runner-owned and this crate never names a clock;
//! - work facts are reported only through `MechanismContext`'s sink;
//! - the normalized result is the oracle crate's frozen
//!   NORMALIZED-RESULT-v1 vocabulary — the H0 parse algorithm itself is
//!   horse-private (`pub(crate)` modules; no other crate links against
//!   this one), so later horses cannot accidentally depend on it.

mod inline;
mod parser;

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
use markit_mdbench_oracle::normalized::{normalized_checksum, NodeKind, NormalizedDocument};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_oracle::NormalizeV1;

/// Reserved H0 mechanism id.
pub const H0_MECHANISM_ID: &str = "h0-full-rebuild";

/// The H0 reference entry point: clean parse of a complete document into
/// the frozen normalized vocabulary. Used by the R4 correctness suites;
/// no other mechanism crate may depend on this crate.
///
/// The result is checked against the shared NORMALIZED-RESULT-v1
/// conformance gate (exact field-kind legality, zero-length rule,
/// `FencedCode.content` interval) before it is returned — every H0
/// result is gated, not only the test surfaces (R4-CORRECTIVE-1).
pub fn parse_document(src: &[u8]) -> NormalizedDocument {
    let document = parser::parse(src);
    validate_normalized(&document, Some(src)).expect("H0 result violates NORMALIZED-RESULT-v1");
    document
}

/// Retained H0 state: the fully materialized normalized document of the
/// last parse plus the source length. No lazy members, no parser
/// handles — the normalized result is plain owned data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H0State {
    pub source_len_bytes: usize,
    pub document: NormalizedDocument,
}

impl NormalizeV1 for H0State {
    fn normalize_v1(&self) -> NormalizedDocument {
        self.document.clone()
    }
}

impl H0State {
    /// The eager-completion proof surface (task contract §14): after
    /// `complete()`, the normalized result can be traversed, compared,
    /// hashed, and queried without invoking any parser work. These are
    /// plain operations over owned data.
    pub fn node_count(&self) -> usize {
        fn count(n: &markit_mdbench_oracle::Node) -> usize {
            1 + n.children.iter().map(count).sum::<usize>()
        }
        count(&self.document.root)
    }
}

/// Trivial prepared state: H0 requires NO mechanism-specific edit
/// preparation (a full rebuild reads the edit only through the
/// already-materialized post-edit source). Nothing is invented here to
/// populate `T_prepare` (task contract §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H0Prepared;

/// Pending work handed to `complete()`: the fully parsed document.
/// Materialization is EAGER — `update`/`full_parse` already built the
/// complete tree; `complete()` consumes it, computes the checksum, and
/// seals the new state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H0Pending {
    document: NormalizedDocument,
}

impl H0Pending {
    /// Eager-completion proof surface (R4 §14): the pending result is
    /// already the COMPLETE normalized document before `complete()` runs;
    /// reading it performs no parse work.
    pub fn document(&self) -> &NormalizedDocument {
        &self.document
    }
}

/// The H0 mechanism.
#[derive(Debug, Clone, Default)]
pub struct FullRebuildMechanism;

impl FullRebuildMechanism {
    pub fn new() -> Self {
        Self
    }
}

/// Attribution semantics for H0 (task contract §15 — no fabricated
/// counters; R4-H0-REFERENCE-CORRECTIVE-1 §4):
///
/// - `unique_source_*`: derived by the common collector from the real
///   per-line (block pass) and per-region (inline pass) inspection
///   events the parser emits — for a clean parse their union is the
///   complete source;
/// - `blocks_reparsed` / `nodes_rebuilt`: measured counts of the block
///   nodes and of all nodes the rebuild constructed;
/// - `nodes_reused`: H0 intentionally reuses no old parse node — the
///   precise fact is exactly zero, reported through the ordinary
///   cumulative counter path so the slot reads `Known(0)` (add(0) = a
///   MEASURED zero; R1 law: `Known(0)` != `Unknown` !=
///   `NotApplicable`);
/// - `metadata_records_touched` / `fallback_to_full_count`: intrinsically
///   meaningless for a full rebuild (no metadata records, no degraded
///   mode) -> `NotApplicable`, never a fabricated zero;
/// - `restart_distance` / `convergence_distance`: no restart/convergence
///   concept -> `NotApplicable` gauges.
fn report_attribution<W: WorkSink>(cx: &mut MechanismContext<'_, W>, blocks: u64, nodes: u64) {
    cx.sink.add_blocks_reparsed(blocks);
    cx.sink.add_nodes_rebuilt(nodes);
    cx.sink.add_nodes_reused(0);
    cx.sink
        .set_slot_not_applicable(NotApplicableSlot::MetadataRecordsTouched);
    cx.sink
        .set_slot_not_applicable(NotApplicableSlot::FallbackToFullCount);
    cx.sink.set_restart_distance(Observed::NotApplicable);
    cx.sink.set_convergence_distance(Observed::NotApplicable);
}

/// Parse `src` with attribution events; returns the document plus the
/// measured block/node counts. The result passes the shared
/// NORMALIZED-RESULT-v1 conformance gate like every other H0 result.
fn parse_with_attribution<W: WorkSink>(
    src: &[u8],
    cx: &mut MechanismContext<'_, W>,
) -> (NormalizedDocument, u64, u64) {
    let document = parser::parse_with_inspection(src, cx.sink);
    validate_normalized(&document, Some(src)).expect("H0 result violates NORMALIZED-RESULT-v1");
    let (blocks, nodes) = count_blocks_nodes(&document.root);
    (document, blocks, nodes)
}

/// Measured construction counts: every node in the result was built by
/// this rebuild. `blocks` counts block-level nodes (Paragraph, Heading,
/// BlockQuote, List, ListItem, FencedCode, ReferenceDefinition);
/// `nodes` counts all normalized nodes.
fn count_blocks_nodes(root: &markit_mdbench_oracle::Node) -> (u64, u64) {
    const INLINE: [NodeKind; 5] = [
        NodeKind::Text,
        NodeKind::Emphasis,
        NodeKind::CodeSpan,
        NodeKind::Link,
        NodeKind::ReferenceLink,
    ];
    fn walk(n: &markit_mdbench_oracle::Node, blocks: &mut u64, nodes: &mut u64) {
        *nodes += 1;
        if !INLINE.contains(&n.kind) {
            *blocks += 1;
        }
        for c in &n.children {
            walk(c, blocks, nodes);
        }
    }
    let mut blocks = 0;
    let mut nodes = 0;
    walk(root, &mut blocks, &mut nodes);
    // the Document root itself is not a parse product
    blocks -= 1;
    nodes -= 1;
    (blocks, nodes)
}

impl Mechanism for FullRebuildMechanism {
    type State = H0State;
    type Prepared = H0Prepared;
    type Pending = H0Pending;

    fn id(&self) -> MechanismId {
        MechanismId(H0_MECHANISM_ID.to_string())
    }

    /// FULL_PARSE: clean parse of the complete source. `T_prepare` is
    /// runner-recorded `NotApplicable` for this operation.
    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let (document, blocks, nodes) = parse_with_attribution(source.as_bytes(), cx);
        report_attribution(cx, blocks, nodes);
        Ok(H0Pending { document })
    }

    /// Trivial by design: a full rebuild needs no edit metadata.
    fn prepare_update<W: WorkSink>(
        &self,
        _old_source: &Source,
        _post_source: &Source,
        _edit: &CanonicalEdit,
        _old_state: &Self::State,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        Ok(H0Prepared)
    }

    /// UPDATE: full clean parse of the post-edit source. The old state
    /// is dropped, never consulted for semantics — proven by the
    /// differential tests (same post source from different old states
    /// yields identical results).
    fn update<W: WorkSink>(
        &self,
        _old_source: &Source,
        post_source: &Source,
        _edit: &CanonicalEdit,
        old_state: Self::State,
        _prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        drop(old_state);
        let (document, blocks, nodes) = parse_with_attribution(post_source.as_bytes(), cx);
        report_attribution(cx, blocks, nodes);
        Ok(H0Pending { document })
    }

    /// Explicit completion boundary: the pending document is already
    /// fully materialized; this seals the new state and derives the
    /// deterministic checksum from the normalized semantic content.
    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        let checksum = normalized_checksum(&pending.document);
        Ok(Completed {
            state: H0State {
                source_len_bytes: pending.document.len_bytes(),
                document: pending.document,
            },
            result_checksum: checksum,
        })
    }
}
