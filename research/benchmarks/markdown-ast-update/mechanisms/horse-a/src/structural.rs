//! HORSE-A-STRUCTURAL-COUNTERS-v1 (#59 §10, §20; #60 §9.4–§9.5): the
//! study-specific, explicitly versioned Horse-A structural attribution
//! record and its two sinks.
//!
//! The record is logically separate from the legacy common attribution
//! schema (`ATTRIBUTION-SCHEMA-v2` / `WorkSink`): the common sink remains
//! authoritative for shared parser/source-inspection events and is NOT
//! reinterpreted here. All variable-size source-inspection recording
//! finishes before the commit frontier; only these fixed-size scalar
//! fields are updated after it (#59 §9.2).
//!
//! Recording rules that are not negotiable in this file:
//!
//! - **at the point of work** — a counter is charged where the work
//!   occurs, by the operator that performs it; no count is derived
//!   afterward from tree shapes, sequence sizes or result walks;
//! - **operator-exact visits** — one logical processing of one non-empty
//!   AVL node by the named operator is one visit of that operator's
//!   counter, and never of a second visit counter (#59 §20 routing);
//! - **Unknown vs Known** — a field that was not collected stays
//!   [`Observed::Unknown`]; `Known(0)` means the counter applied, recording
//!   was enabled, and the actual measured work was zero. Missing
//!   instrumentation can never masquerade as zero, and overflow demotes a
//!   field to `Unknown` so it can never satisfy a PASS gate;
//! - **one algorithm** — the recording lane and the no-op lane execute the
//!   identical mechanism through the identical code path; only the sink
//!   identity differs (#59 §9.2).

/// One recorded scalar: not collected ([`Observed::Unknown`]) or the
/// actually measured value ([`Observed::Known`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Observed {
    #[default]
    Unknown,
    Known(u64),
}

impl Observed {
    /// A deliberately recorded value (test/convenience constructor).
    pub fn known(n: u64) -> Self {
        Observed::Known(n)
    }

    /// Whether the quantity was actually collected.
    pub fn is_known(self) -> bool {
        matches!(self, Observed::Known(_))
    }

    /// The measured value; `None` while `Unknown`.
    pub fn value(self) -> Option<u64> {
        match self {
            Observed::Unknown => None,
            Observed::Known(n) => Some(n),
        }
    }

    /// Accumulate work into the field. The first charge turns `Unknown`
    /// into `Known(n)`; later charges add up. Overflow demotes the field
    /// to `Unknown`: an overflowed counter can never satisfy a PASS gate.
    pub(crate) fn add(self, n: u64) -> Self {
        match self {
            Observed::Unknown => Observed::Known(n),
            Observed::Known(total) => total
                .checked_add(n)
                .map(Observed::Known)
                .unwrap_or(Observed::Unknown),
        }
    }

    /// Merge a depth gauge: the maximum of the recorded depths. The first
    /// charge turns `Unknown` into `Known(n)`.
    pub(crate) fn deepen(self, n: u64) -> Self {
        match self {
            Observed::Unknown => Observed::Known(n),
            Observed::Known(depth) => Observed::Known(depth.max(n)),
        }
    }
}

/// The operator a node visit is charged to (#59 §20 routing). One logical
/// processing of one non-empty AVL node lands in exactly one of these
/// counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralOp {
    /// Weighted byte locate (`locate_by_byte` descent).
    Locate,
    /// Aggregate-pruned safe-predecessor search (phase-1 descent,
    /// phase-2 ancestor examinations, guided second descent).
    SafePredecessor,
    /// Candidate-walk cursor: positioning descent, successor-walk stack
    /// entries, and each candidate boundary whose predicate is evaluated.
    Cursor,
    /// The replacement-facts rank-range navigation: its weighted-descent
    /// seek plus the successor-walk nodes entering the fact cursor's stack
    /// during the exactly Δ_old sequential advances (the corrected §20
    /// charging rule, ACCOUNTING-CORRECTION-1 §5.1 / spec §15.6.1 — the
    /// fact cursor's own work, never the convergence-cursor budget).
    FactRange,
    /// `split` spine nodes, including its internal `join_with_pivot` work.
    Split,
    /// `remove_max` pivot extraction: descent, unwind reprocessing, and
    /// rotation-participant reprocessing.
    PivotExtract,
    /// `join_with_pivot` work charged by a top-level `join`.
    Join,
    /// `bulk_build` node creation.
    BulkBuild,
}

/// Why the same-target full build was selected. The frozen semantic rule
/// has exactly two triggers (`facts differ OR preservation unknown`, spec
/// §8); the encoded values are fixed so the record stays scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullBuildReason {
    /// The local path was selected; no full build occurred (`Known(0)`).
    None,
    /// The complete ordered replacement facts differed.
    FactsDiffer,
    /// Semantic preservation could not be proven.
    PreservationUnknown,
}

impl FullBuildReason {
    /// The scalar encoding recorded in `full_build_reason`
    /// (`Known(0)` = none, `Known(1)` = facts differ, `Known(2)` =
    /// preservation unknown).
    pub(crate) fn code(self) -> u64 {
        match self {
            FullBuildReason::None => 0,
            FullBuildReason::FactsDiffer => 1,
            FullBuildReason::PreservationUnknown => 2,
        }
    }
}

/// The forbidden-work sentinels (#59 §10.1; #60 §9.5). Each is a real
/// recording field, charged `Known(0)` at the defended mechanism site when
/// recording is enabled; a future regression that introduces the forbidden
/// path increments the same charge site (#60: they must not be fake
/// constants disconnected from execution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForbiddenKind {
    /// Sequential enumeration of the retained prefix.
    PrefixEnumeration,
    /// Sequential enumeration of the retained suffix.
    SuffixEnumeration,
    /// Inspection of unaffected (non-replacement) retained payload.
    UnaffectedPayloadInspections,
    /// Per-Owner coordinate rewrite of the retained suffix.
    UnaffectedCoordinateWrites,
    /// Per-Owner certificate rewrite of the retained suffix.
    UnaffectedCertificateWrites,
    /// Document-wide definition recollection.
    GlobalFactRecollection,
    /// Retirement of unaffected old records on the local path.
    UnaffectedOldRetirement,
    /// Extra retained-tree traversal solely for attribution.
    AttributionTreeWalk,
}

/// The frozen Horse-A structural record (#59 §10.1, verbatim unit names;
/// #60 §9.5 decision-bearing subset). Fixed-size scalar fields only, so
/// every post-frontier update is a fixed-size write (#59 §9.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorseAStructuralCountersV1 {
    /// The frozen record identity.
    pub schema: &'static str,

    // ---- scale/height diagnostics (#60 §9.5: Known values) ----
    /// Retained Owner records of the old state (`M_old`).
    pub m_old: Observed,
    /// Retained Owner records of the new state (`M_new`).
    pub m_new: Observed,
    /// Height of the old retained sequence (`H_old`).
    pub h_old: Observed,
    /// Height of the new retained sequence (`H_new`).
    pub h_new: Observed,

    // ---- geometry / route ----
    /// The restart cut `r` the forward parse started from.
    pub restart_old: Observed,
    /// The old replacement end (`q_old`, or `L_old` at real EOF).
    pub convergence_old: Observed,
    /// The new replacement end (`q_new`, or `L_new` at real EOF).
    pub convergence_new: Observed,
    /// The old replacement interval's lower Owner rank (`replace_lo`).
    pub replace_lo: Observed,
    /// The old replacement interval's upper Owner rank (`replace_hi`).
    pub replace_hi: Observed,
    /// Whether the same-target full build was selected (`Known(0/1)`).
    pub full_build_selected: Observed,
    /// Why the full build was selected (`Known(0/1/2)`, see
    /// [`FullBuildReason`]).
    pub full_build_reason: Observed,

    // ---- structural navigation/operator work (#59 §20) ----
    pub locate_node_visits: Observed,
    pub safe_predecessor_node_visits: Observed,
    pub cursor_node_visits: Observed,
    pub fact_range_node_visits: Observed,
    pub split_node_visits: Observed,
    pub pivot_extract_node_visits: Observed,
    pub join_node_visits: Observed,
    pub bulk_build_node_visits: Observed,
    pub retire_node_visits: Observed,

    // ---- structural mutations ----
    pub sequence_link_writes: Observed,
    pub avl_rotations: Observed,

    // ---- aggregate field work (#59 §7.1) ----
    pub aggregate_reads: Observed,
    pub aggregate_writes: Observed,

    // ---- certificate/candidate work ----
    pub certificate_reads: Observed,
    pub certificate_writes: Observed,
    pub candidate_checks: Observed,
    pub cursor_advances: Observed,

    // ---- fresh/replacement/fact work ----
    pub owners_created: Observed,
    pub owners_removed: Observed,
    /// Payload nodes created by the update's fresh materialization that
    /// survive into the returned state. In this realization no transient
    /// payload is discarded, so the temporary count below records the same
    /// events; a future engine that discards transients must split the
    /// charge at its own discard seam.
    pub fresh_payload_nodes_final: Observed,
    /// All payload nodes created by the update's fresh materialization.
    pub fresh_payload_nodes_temporary: Observed,
    pub payload_nodes_retired: Observed,

    pub old_fact_owner_visits: Observed,
    pub old_facts_extracted: Observed,
    pub new_facts_extracted: Observed,
    pub facts_compared: Observed,
    pub fact_compare_bytes: Observed,

    pub reftable_entries_visited: Observed,

    // ---- retirement (#59 §11.1: separated quantities, never summed) ----
    pub retirement_frames_entered: Observed,
    /// Resource-depth gauge: the maximum depth reached by the retirement
    /// recursion, charged per frame kind in its own recursion space (AVL
    /// frames and payload frames are separate recursion spaces, #59
    /// §11.1/#60 f5 — depth is never summed into the frame count). The
    /// actual combined call-stack bound remains `O(H_detached +
    /// D_payload)`, the frozen resource bound.
    pub max_retirement_depth: Observed,

    // ---- forbidden sentinels ----
    pub forbidden_prefix_sequential_enumeration: Observed,
    pub forbidden_suffix_sequential_enumeration: Observed,
    pub forbidden_unaffected_payload_inspections: Observed,
    pub forbidden_unaffected_coordinate_writes: Observed,
    pub forbidden_unaffected_certificate_writes: Observed,
    pub forbidden_global_fact_recollection: Observed,
    pub forbidden_unaffected_old_retirement: Observed,
    pub forbidden_attribution_tree_walk: Observed,
}

impl HorseAStructuralCountersV1 {
    /// The frozen schema identity.
    pub const SCHEMA: &'static str = "HORSE-A-STRUCTURAL-COUNTERS-v1";

    /// A fresh record with every field `Unknown` — never evidence by
    /// itself (#59 §10.2: Unknown can never satisfy PASS).
    pub fn new() -> Self {
        Self {
            schema: Self::SCHEMA,
            m_old: Observed::Unknown,
            m_new: Observed::Unknown,
            h_old: Observed::Unknown,
            h_new: Observed::Unknown,
            restart_old: Observed::Unknown,
            convergence_old: Observed::Unknown,
            convergence_new: Observed::Unknown,
            replace_lo: Observed::Unknown,
            replace_hi: Observed::Unknown,
            full_build_selected: Observed::Unknown,
            full_build_reason: Observed::Unknown,
            locate_node_visits: Observed::Unknown,
            safe_predecessor_node_visits: Observed::Unknown,
            cursor_node_visits: Observed::Unknown,
            fact_range_node_visits: Observed::Unknown,
            split_node_visits: Observed::Unknown,
            pivot_extract_node_visits: Observed::Unknown,
            join_node_visits: Observed::Unknown,
            bulk_build_node_visits: Observed::Unknown,
            retire_node_visits: Observed::Unknown,
            sequence_link_writes: Observed::Unknown,
            avl_rotations: Observed::Unknown,
            aggregate_reads: Observed::Unknown,
            aggregate_writes: Observed::Unknown,
            certificate_reads: Observed::Unknown,
            certificate_writes: Observed::Unknown,
            candidate_checks: Observed::Unknown,
            cursor_advances: Observed::Unknown,
            owners_created: Observed::Unknown,
            owners_removed: Observed::Unknown,
            fresh_payload_nodes_final: Observed::Unknown,
            fresh_payload_nodes_temporary: Observed::Unknown,
            payload_nodes_retired: Observed::Unknown,
            old_fact_owner_visits: Observed::Unknown,
            old_facts_extracted: Observed::Unknown,
            new_facts_extracted: Observed::Unknown,
            facts_compared: Observed::Unknown,
            fact_compare_bytes: Observed::Unknown,
            reftable_entries_visited: Observed::Unknown,
            retirement_frames_entered: Observed::Unknown,
            max_retirement_depth: Observed::Unknown,
            forbidden_prefix_sequential_enumeration: Observed::Unknown,
            forbidden_suffix_sequential_enumeration: Observed::Unknown,
            forbidden_unaffected_payload_inspections: Observed::Unknown,
            forbidden_unaffected_coordinate_writes: Observed::Unknown,
            forbidden_unaffected_certificate_writes: Observed::Unknown,
            forbidden_global_fact_recollection: Observed::Unknown,
            forbidden_unaffected_old_retirement: Observed::Unknown,
            forbidden_attribution_tree_walk: Observed::Unknown,
        }
    }
}

impl Default for HorseAStructuralCountersV1 {
    fn default() -> Self {
        Self::new()
    }
}

/// The Horse-A structural accounting sink. Threaded through the whole
/// update — staging, frontier crossing and retirement — so every charge
/// lands in one record; the no-op and recording implementations execute
/// the identical mechanism.
pub trait HorseAStructuralSink {
    /// One logical processing of one non-empty AVL node by the named
    /// operator (#59 §20 routing).
    fn node_visit(&mut self, op: StructuralOp);

    /// `n` persistent structural Link-slot mutations (rotation primitives
    /// charge the frozen flat convention instead of their raw slots).
    fn link_writes(&mut self, n: u64);

    /// `units` rotation units (single = 1, double = 2).
    fn rotations(&mut self, units: u64);

    /// `n` persistent aggregate-field reads (#59 §7.1).
    fn aggregate_reads(&mut self, n: u64);

    /// `n` persistent aggregate-field writes during recomputation.
    fn aggregate_writes(&mut self, n: u64);

    /// One inspection of a persistent RestartCertificate (or its presence)
    /// sufficient for one mechanism decision. Reading `subtree_has_safe`
    /// is an aggregate read, never this.
    fn certificate_read(&mut self);

    /// One creation/installation/update of one persistent outgoing
    /// RestartCertificate at the real persistence seam.
    fn certificate_write(&mut self);

    /// One full convergence-predicate evaluation of one offered candidate.
    fn candidate_check(&mut self);

    /// One monotone movement to the next offered candidate boundary.
    fn cursor_advance(&mut self);

    /// `n` fresh Owners created.
    fn owners_created(&mut self, n: u64);

    /// `n` old Owners disposed of by this commit.
    fn owners_removed(&mut self, n: u64);

    /// One fresh payload node created by the update's materialization.
    fn fresh_payload_node_created(&mut self);

    /// One Owner visited while extracting the old replacement facts.
    fn old_fact_owner_visit(&mut self);

    /// `n` fact entries extracted from the old replacement side.
    fn facts_extracted_old(&mut self, n: u64);

    /// `n` fact entries taken from the fresh region's table.
    fn facts_extracted_new(&mut self, n: u64);

    /// `n` ordered fact-entry comparisons performed.
    fn facts_compared(&mut self, n: u64);

    /// `n` bytes of label/destination text actually compared.
    fn fact_compare_bytes(&mut self, n: u64);

    /// `n` RefTable entries actually visited by the mechanism.
    fn reftable_entries_visited(&mut self, n: u64);

    /// One retirement recursion frame entered for a retired AVL record,
    /// at `avl_depth` in the AVL recursion space.
    fn retire_avl_frame(&mut self, avl_depth: u64);

    /// One retirement recursion frame entered for a destroyed payload
    /// node, at `payload_depth` in the payload recursion space.
    fn retire_payload_frame(&mut self, payload_depth: u64);

    /// The forbidden-work sentinel charge: the defended site executed and
    /// measured `n` occurrences of the forbidden work (correct sites
    /// measure 0; a regression increments the same site).
    fn forbidden(&mut self, kind: ForbiddenKind, n: u64);

    // ---- geometry / route recording ----

    /// Old-state scale/height diagnostics (`M_old`, `H_old`).
    fn record_scale_old(&mut self, m_old: u64, h_old: u64);

    /// New-state record count (`M_new`).
    fn record_scale_new(&mut self, m_new: u64);

    /// New-state height (`H_new`), recorded once the new root exists.
    fn record_height_new(&mut self, h_new: u64);

    /// The restart cut the forward parse started from.
    fn record_restart_old(&mut self, cut: u64);

    /// The accepted replacement end on both sides (`q_old/q_new`, or the
    /// real `L_old/L_new` at EOF).
    fn record_convergence(&mut self, old_cut: u64, new_cut: u64);

    /// The old replacement interval in Owner ranks.
    fn record_replace_interval(&mut self, lo: u64, hi: u64);

    /// The frozen semantic full-build decision.
    fn record_full_build(&mut self, selected: bool, reason: FullBuildReason);
}

/// The timing/no-op structural lane: identical algorithm, no recording.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopHorseAStructuralSink;

impl HorseAStructuralSink for NoopHorseAStructuralSink {
    fn node_visit(&mut self, _op: StructuralOp) {}
    fn link_writes(&mut self, _n: u64) {}
    fn rotations(&mut self, _units: u64) {}
    fn aggregate_reads(&mut self, _n: u64) {}
    fn aggregate_writes(&mut self, _n: u64) {}
    fn certificate_read(&mut self) {}
    fn certificate_write(&mut self) {}
    fn candidate_check(&mut self) {}
    fn cursor_advance(&mut self) {}
    fn owners_created(&mut self, _n: u64) {}
    fn owners_removed(&mut self, _n: u64) {}
    fn fresh_payload_node_created(&mut self) {}
    fn old_fact_owner_visit(&mut self) {}
    fn facts_extracted_old(&mut self, _n: u64) {}
    fn facts_extracted_new(&mut self, _n: u64) {}
    fn facts_compared(&mut self, _n: u64) {}
    fn fact_compare_bytes(&mut self, _n: u64) {}
    fn reftable_entries_visited(&mut self, _n: u64) {}
    fn retire_avl_frame(&mut self, _avl_depth: u64) {}
    fn retire_payload_frame(&mut self, _payload_depth: u64) {}
    fn forbidden(&mut self, _kind: ForbiddenKind, _n: u64) {}
    fn record_scale_old(&mut self, _m_old: u64, _h_old: u64) {}
    fn record_scale_new(&mut self, _m_new: u64) {}
    fn record_height_new(&mut self, _h_new: u64) {}
    fn record_restart_old(&mut self, _cut: u64) {}
    fn record_convergence(&mut self, _old_cut: u64, _new_cut: u64) {}
    fn record_replace_interval(&mut self, _lo: u64, _hi: u64) {}
    fn record_full_build(&mut self, _selected: bool, _reason: FullBuildReason) {}
}

/// The recording structural lane: accumulates every charge into one
/// [`HorseAStructuralCountersV1`] record.
#[derive(Debug, Clone, Default)]
pub struct RecordingHorseAStructuralSink {
    counters: HorseAStructuralCountersV1,
}

impl RecordingHorseAStructuralSink {
    /// Start recording with every field `Unknown`.
    pub fn new() -> Self {
        Self {
            counters: HorseAStructuralCountersV1::new(),
        }
    }

    /// The record accumulated so far.
    pub fn counters(&self) -> &HorseAStructuralCountersV1 {
        &self.counters
    }

    /// Consume the sink and hand back the record.
    pub fn into_counters(self) -> HorseAStructuralCountersV1 {
        self.counters
    }
}

impl HorseAStructuralSink for RecordingHorseAStructuralSink {
    fn node_visit(&mut self, op: StructuralOp) {
        let field = match op {
            StructuralOp::Locate => &mut self.counters.locate_node_visits,
            StructuralOp::SafePredecessor => &mut self.counters.safe_predecessor_node_visits,
            StructuralOp::Cursor => &mut self.counters.cursor_node_visits,
            StructuralOp::FactRange => &mut self.counters.fact_range_node_visits,
            StructuralOp::Split => &mut self.counters.split_node_visits,
            StructuralOp::PivotExtract => &mut self.counters.pivot_extract_node_visits,
            StructuralOp::Join => &mut self.counters.join_node_visits,
            StructuralOp::BulkBuild => &mut self.counters.bulk_build_node_visits,
        };
        *field = field.add(1);
    }

    fn link_writes(&mut self, n: u64) {
        let field = &mut self.counters.sequence_link_writes;
        *field = field.add(n);
    }

    fn rotations(&mut self, units: u64) {
        let field = &mut self.counters.avl_rotations;
        *field = field.add(units);
    }

    fn aggregate_reads(&mut self, n: u64) {
        let field = &mut self.counters.aggregate_reads;
        *field = field.add(n);
    }

    fn aggregate_writes(&mut self, n: u64) {
        let field = &mut self.counters.aggregate_writes;
        *field = field.add(n);
    }

    fn certificate_read(&mut self) {
        let field = &mut self.counters.certificate_reads;
        *field = field.add(1);
    }

    fn certificate_write(&mut self) {
        let field = &mut self.counters.certificate_writes;
        *field = field.add(1);
    }

    fn candidate_check(&mut self) {
        let field = &mut self.counters.candidate_checks;
        *field = field.add(1);
    }

    fn cursor_advance(&mut self) {
        let field = &mut self.counters.cursor_advances;
        *field = field.add(1);
    }

    fn owners_created(&mut self, n: u64) {
        let field = &mut self.counters.owners_created;
        *field = field.add(n);
    }

    fn owners_removed(&mut self, n: u64) {
        let field = &mut self.counters.owners_removed;
        *field = field.add(n);
    }

    fn fresh_payload_node_created(&mut self) {
        // This realization discards no transient payload, so the final and
        // temporary counts record the same creation events (see the field
        // docs on the record).
        let final_field = &mut self.counters.fresh_payload_nodes_final;
        *final_field = final_field.add(1);
        let temp_field = &mut self.counters.fresh_payload_nodes_temporary;
        *temp_field = temp_field.add(1);
    }

    fn old_fact_owner_visit(&mut self) {
        let field = &mut self.counters.old_fact_owner_visits;
        *field = field.add(1);
    }

    fn facts_extracted_old(&mut self, n: u64) {
        let field = &mut self.counters.old_facts_extracted;
        *field = field.add(n);
    }

    fn facts_extracted_new(&mut self, n: u64) {
        let field = &mut self.counters.new_facts_extracted;
        *field = field.add(n);
    }

    fn facts_compared(&mut self, n: u64) {
        let field = &mut self.counters.facts_compared;
        *field = field.add(n);
    }

    fn fact_compare_bytes(&mut self, n: u64) {
        let field = &mut self.counters.fact_compare_bytes;
        *field = field.add(n);
    }

    fn reftable_entries_visited(&mut self, n: u64) {
        let field = &mut self.counters.reftable_entries_visited;
        *field = field.add(n);
    }

    fn retire_avl_frame(&mut self, avl_depth: u64) {
        let visits = &mut self.counters.retire_node_visits;
        *visits = visits.add(1);
        let frames = &mut self.counters.retirement_frames_entered;
        *frames = frames.add(1);
        let depth = &mut self.counters.max_retirement_depth;
        *depth = depth.deepen(avl_depth);
    }

    fn retire_payload_frame(&mut self, payload_depth: u64) {
        let nodes = &mut self.counters.payload_nodes_retired;
        *nodes = nodes.add(1);
        let frames = &mut self.counters.retirement_frames_entered;
        *frames = frames.add(1);
        let depth = &mut self.counters.max_retirement_depth;
        *depth = depth.deepen(payload_depth);
    }

    fn forbidden(&mut self, kind: ForbiddenKind, n: u64) {
        let field = match kind {
            ForbiddenKind::PrefixEnumeration => {
                &mut self.counters.forbidden_prefix_sequential_enumeration
            }
            ForbiddenKind::SuffixEnumeration => {
                &mut self.counters.forbidden_suffix_sequential_enumeration
            }
            ForbiddenKind::UnaffectedPayloadInspections => {
                &mut self.counters.forbidden_unaffected_payload_inspections
            }
            ForbiddenKind::UnaffectedCoordinateWrites => {
                &mut self.counters.forbidden_unaffected_coordinate_writes
            }
            ForbiddenKind::UnaffectedCertificateWrites => {
                &mut self.counters.forbidden_unaffected_certificate_writes
            }
            ForbiddenKind::GlobalFactRecollection => {
                &mut self.counters.forbidden_global_fact_recollection
            }
            ForbiddenKind::UnaffectedOldRetirement => {
                &mut self.counters.forbidden_unaffected_old_retirement
            }
            ForbiddenKind::AttributionTreeWalk => {
                &mut self.counters.forbidden_attribution_tree_walk
            }
        };
        *field = field.add(n);
    }

    fn record_scale_old(&mut self, m_old: u64, h_old: u64) {
        self.counters.m_old = Observed::Known(m_old);
        self.counters.h_old = Observed::Known(h_old);
    }

    fn record_scale_new(&mut self, m_new: u64) {
        self.counters.m_new = Observed::Known(m_new);
    }

    fn record_height_new(&mut self, h_new: u64) {
        self.counters.h_new = Observed::Known(h_new);
    }

    fn record_restart_old(&mut self, cut: u64) {
        self.counters.restart_old = Observed::Known(cut);
    }

    fn record_convergence(&mut self, old_cut: u64, new_cut: u64) {
        self.counters.convergence_old = Observed::Known(old_cut);
        self.counters.convergence_new = Observed::Known(new_cut);
    }

    fn record_replace_interval(&mut self, lo: u64, hi: u64) {
        self.counters.replace_lo = Observed::Known(lo);
        self.counters.replace_hi = Observed::Known(hi);
    }

    fn record_full_build(&mut self, selected: bool, reason: FullBuildReason) {
        self.counters.full_build_selected = Observed::Known(u64::from(selected));
        self.counters.full_build_reason = Observed::Known(reason.code());
    }
}
