//! HORSE-A-STRUCTURAL-RAW-v1 — the frozen raw-row schema (readiness
//! record §9) plus the pre-treatment P2-2/P2-4 extensions required by
//! the #60 authorization record before the first decision-bearing row:
//!
//! - `frozen_contract_revision` (P2-4) — the synchronized #60 contract
//!   revision, distinct from StudyId / PR #72 / the execution commit;
//! - `study_mechanism_baseline` / `authorization_baseline` (P2-2) —
//!   named separately from `repository_commit` / `repository_tree`,
//!   which record the actual execution revision.
//!
//! The row is one JSON object per (cell, repetition). Unknown values stay
//! explicit (`"Unknown"`), never omitted. Rows are immutable once
//! written (see [`crate::producer::writer`]).
//!
//! The counters enumeration below is compile-time exhaustive over
//! [`HorseAStructuralCountersV1`]: it destructures the struct, so adding
//! a field to the counter record breaks this crate's build until the
//! enumeration — and therefore the adjudicator classification and the
//! completeness meta-test — is updated. That is the mechanical half of
//! the P2-6 guarantee; the meta-test is the other half.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::structural::{HorseAStructuralCountersV1, Observed};

use super::contract;

/// The canonical string form of one observed scalar.
pub fn observed_to_string(o: Observed) -> String {
    match o {
        Observed::Unknown => "Unknown".to_string(),
        Observed::Known(n) => format!("Known({n})"),
    }
}

/// Parse the canonical string form of one observed scalar.
pub fn observed_from_string(text: &str) -> Option<Observed> {
    if text == "Unknown" {
        return Some(Observed::Unknown);
    }
    let n = text
        .strip_prefix("Known(")
        .and_then(|rest| rest.strip_suffix(')'))?
        .parse::<u64>()
        .ok()?;
    Some(Observed::Known(n))
}

/// Every scalar field of the frozen counter record (the record identity
/// `schema` string is carried at row level as `counter_schema`). The
/// single field-list authority for row serialization, the adjudicator
/// classification sweep and the completeness meta-test.
pub const COUNTER_FIELDS: &[&str] = &[
    "m_old",
    "m_new",
    "h_old",
    "h_new",
    "restart_old",
    "convergence_old",
    "convergence_new",
    "replace_lo",
    "replace_hi",
    "full_build_selected",
    "full_build_reason",
    "locate_node_visits",
    "safe_predecessor_node_visits",
    "cursor_node_visits",
    "fact_range_node_visits",
    "split_node_visits",
    "pivot_extract_node_visits",
    "join_node_visits",
    "bulk_build_node_visits",
    "retire_node_visits",
    "sequence_link_writes",
    "avl_rotations",
    "aggregate_reads",
    "aggregate_writes",
    "certificate_reads",
    "certificate_writes",
    "candidate_checks",
    "cursor_advances",
    "owners_created",
    "owners_removed",
    "fresh_payload_nodes_final",
    "fresh_payload_nodes_temporary",
    "payload_nodes_retired",
    "old_fact_owner_visits",
    "old_facts_extracted",
    "new_facts_extracted",
    "facts_compared",
    "fact_compare_bytes",
    "reftable_entries_visited",
    "retirement_frames_entered",
    "max_retirement_depth",
    "forbidden_prefix_sequential_enumeration",
    "forbidden_suffix_sequential_enumeration",
    "forbidden_unaffected_payload_inspections",
    "forbidden_unaffected_coordinate_writes",
    "forbidden_unaffected_certificate_writes",
    "forbidden_global_fact_recollection",
    "forbidden_unaffected_old_retirement",
    "forbidden_attribution_tree_walk",
];

/// Compile-time-exhaustive enumeration of the counter record into a
/// name→value map. Destructuring every field makes a future added field
/// a hard compile error here.
pub fn counters_to_map(counters: &HorseAStructuralCountersV1) -> BTreeMap<String, Observed> {
    // Exhaustive destructure: adding a field to
    // HorseAStructuralCountersV1 without extending COUNTER_FIELDS (and
    // the adjudicator classification) fails this very binding.
    let HorseAStructuralCountersV1 {
        schema: _,
        m_old,
        m_new,
        h_old,
        h_new,
        restart_old,
        convergence_old,
        convergence_new,
        replace_lo,
        replace_hi,
        full_build_selected,
        full_build_reason,
        locate_node_visits,
        safe_predecessor_node_visits,
        cursor_node_visits,
        fact_range_node_visits,
        split_node_visits,
        pivot_extract_node_visits,
        join_node_visits,
        bulk_build_node_visits,
        retire_node_visits,
        sequence_link_writes,
        avl_rotations,
        aggregate_reads,
        aggregate_writes,
        certificate_reads,
        certificate_writes,
        candidate_checks,
        cursor_advances,
        owners_created,
        owners_removed,
        fresh_payload_nodes_final,
        fresh_payload_nodes_temporary,
        payload_nodes_retired,
        old_fact_owner_visits,
        old_facts_extracted,
        new_facts_extracted,
        facts_compared,
        fact_compare_bytes,
        reftable_entries_visited,
        retirement_frames_entered,
        max_retirement_depth,
        forbidden_prefix_sequential_enumeration,
        forbidden_suffix_sequential_enumeration,
        forbidden_unaffected_payload_inspections,
        forbidden_unaffected_coordinate_writes,
        forbidden_unaffected_certificate_writes,
        forbidden_global_fact_recollection,
        forbidden_unaffected_old_retirement,
        forbidden_attribution_tree_walk,
    } = *counters;

    let fields: Vec<(&str, Observed)> = vec![
        ("m_old", m_old),
        ("m_new", m_new),
        ("h_old", h_old),
        ("h_new", h_new),
        ("restart_old", restart_old),
        ("convergence_old", convergence_old),
        ("convergence_new", convergence_new),
        ("replace_lo", replace_lo),
        ("replace_hi", replace_hi),
        ("full_build_selected", full_build_selected),
        ("full_build_reason", full_build_reason),
        ("locate_node_visits", locate_node_visits),
        ("safe_predecessor_node_visits", safe_predecessor_node_visits),
        ("cursor_node_visits", cursor_node_visits),
        ("fact_range_node_visits", fact_range_node_visits),
        ("split_node_visits", split_node_visits),
        ("pivot_extract_node_visits", pivot_extract_node_visits),
        ("join_node_visits", join_node_visits),
        ("bulk_build_node_visits", bulk_build_node_visits),
        ("retire_node_visits", retire_node_visits),
        ("sequence_link_writes", sequence_link_writes),
        ("avl_rotations", avl_rotations),
        ("aggregate_reads", aggregate_reads),
        ("aggregate_writes", aggregate_writes),
        ("certificate_reads", certificate_reads),
        ("certificate_writes", certificate_writes),
        ("candidate_checks", candidate_checks),
        ("cursor_advances", cursor_advances),
        ("owners_created", owners_created),
        ("owners_removed", owners_removed),
        ("fresh_payload_nodes_final", fresh_payload_nodes_final),
        (
            "fresh_payload_nodes_temporary",
            fresh_payload_nodes_temporary,
        ),
        ("payload_nodes_retired", payload_nodes_retired),
        ("old_fact_owner_visits", old_fact_owner_visits),
        ("old_facts_extracted", old_facts_extracted),
        ("new_facts_extracted", new_facts_extracted),
        ("facts_compared", facts_compared),
        ("fact_compare_bytes", fact_compare_bytes),
        ("reftable_entries_visited", reftable_entries_visited),
        ("retirement_frames_entered", retirement_frames_entered),
        ("max_retirement_depth", max_retirement_depth),
        (
            "forbidden_prefix_sequential_enumeration",
            forbidden_prefix_sequential_enumeration,
        ),
        (
            "forbidden_suffix_sequential_enumeration",
            forbidden_suffix_sequential_enumeration,
        ),
        (
            "forbidden_unaffected_payload_inspections",
            forbidden_unaffected_payload_inspections,
        ),
        (
            "forbidden_unaffected_coordinate_writes",
            forbidden_unaffected_coordinate_writes,
        ),
        (
            "forbidden_unaffected_certificate_writes",
            forbidden_unaffected_certificate_writes,
        ),
        (
            "forbidden_global_fact_recollection",
            forbidden_global_fact_recollection,
        ),
        (
            "forbidden_unaffected_old_retirement",
            forbidden_unaffected_old_retirement,
        ),
        (
            "forbidden_attribution_tree_walk",
            forbidden_attribution_tree_walk,
        ),
    ];

    debug_assert_eq!(fields.len(), COUNTER_FIELDS.len());
    fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Alias mirroring the readiness record's name for the enumeration.
pub fn counters_field_names() -> &'static [&'static str] {
    COUNTER_FIELDS
}

/// The correctness block of one raw row (#60 §7; readiness record §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectnessBlock {
    /// C1 — pre-edit full build equals the H0 clean parse of the pre
    /// source (`"PASS"` / `"FAIL"` / `"NOT_RUN"`).
    pub c1: String,
    /// C2 — the primary incremental update equals the H0 clean parse of
    /// the post source.
    pub c2: String,
    /// C3 — the restore probe on the SAME returned state equals the H0
    /// clean parse of the pre source.
    pub c3: String,
    /// The oracle decision itself: full normalized structural equality
    /// held on every gate that ran (checksums are provenance only).
    pub normalized_structural_equality: bool,
}

/// The provenance checksums of the three oracle gates (provenance, never
/// the oracle decision — readiness record §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultChecksums {
    pub c1: u64,
    pub c2: u64,
    pub c3: u64,
}

/// The adjudication block of one raw row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjudicationBlock {
    /// `"PASS"` | `"FAIL"` | `"INVALID"`.
    pub status: String,
    /// Structured reasons, one per failed/invalid obligation (empty on
    /// PASS).
    pub reasons: Vec<String>,
}

/// The host identity block (readiness record §8: OS/kernel,
/// architecture, CPU model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBlock {
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub cpu_model: String,
}

/// One HORSE-A-STRUCTURAL-RAW-v1 row (readiness record §9 + P2-2/P2-4
/// pre-treatment extensions). One JSON object per (cell, repetition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRowV1 {
    /// The frozen raw-schema identity.
    pub schema: String,
    /// Stable row identity (task §13): sha256 over
    /// study_id | cell_id | repetition | executable_sha256.
    pub row_identity: String,
    /// §3 StudyId.
    pub study_id: String,
    /// Protocol identity.
    pub protocol: String,
    /// P2-4 — the synchronized #60 contract revision.
    pub frozen_contract_revision: String,
    /// P2-2 — the StudyId-bound reviewed mechanism revision.
    pub study_mechanism_baseline: String,
    /// P2-2 — the authorization master.
    pub authorization_baseline: String,
    /// P2-2 — the actual execution revision (commit).
    pub repository_commit: String,
    /// P2-2 — the actual execution revision (tree).
    pub repository_tree: String,
    /// Mechanism identity (#55; readiness record §2).
    pub mechanism: String,
    pub mechanism_design_head: String,
    pub mechanism_merge: String,
    /// Cell identity.
    pub cell_id: String,
    pub case_id_hex: String,
    pub n_bytes: u64,
    pub m: u64,
    pub target: u64,
    pub edit_start: u64,
    pub edit_end: u64,
    /// The observed (computed) byte identities — the adjudicator compares
    /// these against the frozen gates.
    pub inserted_text_sha256: String,
    pub pre_sha256: String,
    pub post_sha256: String,
    /// Schema repetition (readiness record §9: 0 | 1 | 2). The external
    /// schedule's `--repetition` is 1-based and maps to this value.
    pub repetition: u64,
    /// Build identity (readiness record §8).
    pub executable_sha256: String,
    pub rustc: String,
    pub cargo: String,
    pub toolchain_channel: String,
    pub build_profile: String,
    pub features: String,
    pub host: HostBlock,
    pub execution_command: String,
    /// Counter schema identity.
    pub counter_schema: String,
    /// Every scalar field of HorseAStructuralCountersV1, explicit
    /// Known/Unknown.
    #[serde(
        serialize_with = "serialize_observed_map",
        deserialize_with = "deserialize_observed_map"
    )]
    pub counters: BTreeMap<String, Observed>,
    /// The selected route (readiness record §9).
    pub route: String,
    /// The fallback reason when the full build was selected.
    pub full_build_reason: String,
    /// Correctness gates.
    pub correctness: CorrectnessBlock,
    /// Oracle/result checksums (provenance only).
    pub result_checksums: ResultChecksums,
    /// The producer-time adjudication (Phase D re-derives mechanically
    /// from the raw rows; the embedded verdict is never edited).
    pub adjudication: AdjudicationBlock,
}

fn serialize_observed_map<S: Serializer>(
    map: &BTreeMap<String, Observed>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let string_map: BTreeMap<&str, String> = map
        .iter()
        .map(|(k, v)| (k.as_str(), observed_to_string(*v)))
        .collect();
    string_map.serialize(s)
}

fn deserialize_observed_map<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, Observed>, D::Error> {
    let string_map = BTreeMap::<String, String>::deserialize(d)?;
    let mut map = BTreeMap::new();
    for (k, v) in string_map {
        let observed = observed_from_string(&v).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "malformed Observed value for {k}: {v:?} (expected \"Known(n)\" or \"Unknown\")"
            ))
        })?;
        map.insert(k, observed);
    }
    Ok(map)
}

impl RawRowV1 {
    /// The row identity (task §13): sha256 over the four identity
    /// components, newline-separated.
    pub fn compute_row_identity(
        study_id: &str,
        cell_id: &str,
        repetition: u64,
        executable_sha256: &str,
    ) -> String {
        let preimage = format!("{study_id}\n{cell_id}\n{repetition}\n{executable_sha256}");
        contract::sha256_hex(preimage.as_bytes())
    }

    /// The counters map serialized from a live record.
    pub fn counters_map(counters: &HorseAStructuralCountersV1) -> BTreeMap<String, Observed> {
        counters_to_map(counters)
    }

    /// Fetch one counter (decision-bearing lookup; `None` when absent).
    pub fn counter(&self, name: &str) -> Option<Observed> {
        self.counters.get(name).copied()
    }
}

/// The static schema descriptor printed by `--print-schema` (frozen
/// field list; used by tests to assert required-field coverage).
pub fn schema_field_report() -> String {
    let mut out = String::new();
    out.push_str("HORSE-A-STRUCTURAL-RAW-v1\n");
    out.push_str("one JSON object per (cell, repetition); rows immutable once written\n");
    out.push_str("counters schema: HORSE-A-STRUCTURAL-COUNTERS-v1 (explicit Known(n)/Unknown)\n");
    out.push_str("counter fields:\n");
    for name in counters_field_names() {
        out.push_str(&format!("  counters.{name}\n"));
    }
    out.push_str("row fields:\n");
    for f in [
        "schema",
        "row_identity",
        "study_id",
        "protocol",
        "frozen_contract_revision",
        "study_mechanism_baseline",
        "authorization_baseline",
        "repository_commit",
        "repository_tree",
        "mechanism",
        "mechanism_design_head",
        "mechanism_merge",
        "cell_id",
        "case_id_hex",
        "n_bytes",
        "m",
        "target",
        "edit_start",
        "edit_end",
        "inserted_text_sha256",
        "pre_sha256",
        "post_sha256",
        "repetition",
        "executable_sha256",
        "rustc",
        "cargo",
        "toolchain_channel",
        "build_profile",
        "features",
        "host.os",
        "host.kernel",
        "host.arch",
        "host.cpu_model",
        "execution_command",
        "counter_schema",
        "counters",
        "route",
        "full_build_reason",
        "correctness.c1",
        "correctness.c2",
        "correctness.c3",
        "correctness.normalized_structural_equality",
        "result_checksums.c1",
        "result_checksums.c2",
        "result_checksums.c3",
        "adjudication.status",
        "adjudication.reasons",
    ] {
        out.push_str(&format!("  {f}\n"));
    }
    out
}
