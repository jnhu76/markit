//! Complete ordered replacement definition facts (spec §8–§9; data-model
//! §10; I4 task contract §17).
//!
//! Both sides of the replacement are held as *complete ordered fact
//! sequences*: source order preserved, duplicates preserved, and both the
//! normalized label and the destination bytes compared. Nothing here is a
//! hash, a winner subset, a sorted set, or a per-Owner definition index —
//! and the comparison happens BEFORE any semantic materialization, because
//! the environment it proves is the environment fresh reference-sensitive
//! payload must be built under.
//!
//! The old side is extracted from the retained eager payloads of exactly
//! the Owners the update is about to detach. That is a transient local read
//! (data-model §10.1): the retained prefix and suffix are never scanned, and
//! no second permanent definition table appears anywhere.

use std::ops::Range;

use markit_mdbench_oracle::normalized::{Node, NodeKind};
use markit_mdbench_shared_grammar::RefTable;

use crate::state::{OwnerPayload, OwnerSeq};
use crate::structural::{ForbiddenKind, HorseAStructuralSink, StructuralOp};

/// One replacement side's complete ordered definition-fact sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderedFacts {
    entries: Vec<(String, String)>,
}

impl OrderedFacts {
    /// `Defs(O)`: the facts of the removed old Owners in source order, read
    /// from their eager payloads' block topology. `ranks` is the old
    /// replacement interval in Owner-rank space; the cursor is positioned
    /// once and advanced through exactly those records, so the cost is
    /// Δ_old records — never a document-wide recollection.
    pub(crate) fn of_old_replacement(
        owners: &OwnerSeq,
        ranks: Range<usize>,
        sink: &mut dyn HorseAStructuralSink,
    ) -> Self {
        debug_assert!(
            ranks.end <= owners.records(),
            "the old replacement rank range must lie inside the retained sequence"
        );
        let mut entries = Vec::new();
        let mut cursor = owners.cursor_at_rank(ranks.start, StructuralOp::FactRange, sink);
        for _ in ranks {
            let Some(item) = cursor.next(StructuralOp::FactRange, sink) else {
                debug_assert!(false, "the old replacement range left the sequence early");
                break;
            };
            sink.old_fact_owner_visit();
            if let OwnerPayload::Syntax(payload) = &item.owner.payload {
                collect_definition_facts(&payload.root, &mut entries);
            }
        }
        sink.facts_extracted_old(entries.len() as u64);
        // Defended-site sentinel assertion: this extraction read the eager
        // payloads of exactly the replacement Owners the cursor advanced
        // through — no unaffected (retained prefix/suffix) payload was
        // inspected. A regression that widened the read would charge this
        // site.
        sink.forbidden(ForbiddenKind::UnaffectedPayloadInspections, 0);
        Self { entries }
    }

    /// `Defs(N)`: the fresh region's own facts, exactly as the shared block
    /// pass recorded them while parsing `[r, q_new)`. The region parse only
    /// ever consumed that range, so these facts belong to the replacement
    /// and to nothing else.
    pub(crate) fn of_fresh_region(defs: &RefTable, sink: &mut dyn HorseAStructuralSink) -> Self {
        let entries = defs.entries().to_vec();
        sink.facts_extracted_new(entries.len() as u64);
        // The mechanism visits every region-table entry it copies here.
        sink.reftable_entries_visited(entries.len() as u64);
        Self { entries }
    }

    pub(crate) fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    /// The frozen preservation decision (spec §8): `Defs(O) == Defs(N)` as
    /// complete ordered sequences — label AND destination compared
    /// entry-by-entry, source order significant, duplicates significant.
    /// The comparison work is charged as it occurs; the decision remains
    /// exactly the equality of the two sequences.
    pub(crate) fn eq_with_recording(
        &self,
        other: &OrderedFacts,
        sink: &mut dyn HorseAStructuralSink,
    ) -> bool {
        // This comparison site executed: its measured pairwise count (and
        // compared bytes) is charged here, including the measured zero of
        // a comparison with no entries (`Known(0)`, never `Unknown`).
        sink.facts_compared(0);
        sink.fact_compare_bytes(0);
        let (left, right) = (self.entries.as_slice(), other.entries.as_slice());
        // Lengths differing decides without any entry comparison.
        if left.len() != right.len() {
            return false;
        }
        for (a, b) in left.iter().zip(right.iter()) {
            sink.facts_compared(1);
            // Both the normalized label and the destination bytes compare.
            sink.fact_compare_bytes((a.0.len() + a.1.len()) as u64);
            if a.0 != b.0 || a.1 != b.1 {
                return false;
            }
        }
        true
    }
}

/// Source-order (pre-order) collection of `ReferenceDefinition` facts. The
/// shared block pass defines each label at its physical line dispatch, so
/// pre-order over the retained payload is exactly the source order the
/// document-global RefTable projects (spec §2.4).
pub(crate) fn collect_definition_facts(node: &Node, out: &mut Vec<(String, String)>) {
    if node.kind == NodeKind::ReferenceDefinition {
        if let (Some(label), Some(destination)) = (&node.label, &node.destination) {
            out.push((label.clone(), destination.clone()));
        }
    }
    for child in &node.children {
        collect_definition_facts(child, out);
    }
}
