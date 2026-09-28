//! The frozen #60 §14 adjudicator (P2-6): a PURE function over one
//! frozen cell definition plus one raw counter row, returning structured
//! reasons — never a single boolean, never a silent coercion.
//!
//! Complete sweep (every decision-bearing obligation of the
//! synchronized #60 contract):
//!
//! ```text
//! INVALID   schema/study/contract-revision/baseline/mechanism/cell
//!           identity/counters-map shape/repetition/provenance/row
//!           identity — external, producer or instrumentation failure
//! FAIL      correctness C1/C2/C3; Known sweep (Unknown can never PASS);
//!           route/geometry exact values; convergence before EOF; every
//!           §9.5 threshold (incl. the f1/f2 composites); every
//!           forbidden sentinel Known(0); the H_max invariant
//!           h(root) ≤ H_max(M) (an implementation invariant failure,
//!           never a larger budget)
//! PASS      only when no reason of either kind exists
//! ```
//!
//! The classification table at the bottom of this file drives both the
//! sweep and the completeness meta-test: every field of
//! HORSE-A-STRUCTURAL-COUNTERS-v1 is classified exactly once, and the
//! meta-test asserts set-equality with the compile-time-exhaustive
//! field enumeration in [`super::schema`].

use crate::structural::Observed;

use super::contract::{self, FrozenCell};
use super::schema::RawRowV1;

/// One adjudication verdict with structured reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adjudication {
    /// Every frozen obligation holds.
    Pass,
    /// Evidence against the frozen Horse-A claim or implementation
    /// conformance (#60 §13). Never relabelled INVALID for convenience.
    Fail(Vec<String>),
    /// External/producer/instrumentation failure (#60 §12). The row is
    /// still archived; it can never carry a PASS.
    Invalid(Vec<String>),
}

impl Adjudication {
    /// The status string recorded in the raw row.
    pub fn status(&self) -> &'static str {
        match self {
            Adjudication::Pass => "PASS",
            Adjudication::Fail(_) => "FAIL",
            Adjudication::Invalid(_) => "INVALID",
        }
    }

    /// The structured reasons (empty on PASS).
    pub fn reasons(&self) -> &[String] {
        match self {
            Adjudication::Pass => &[],
            Adjudication::Fail(r) | Adjudication::Invalid(r) => r,
        }
    }
}

/// How one counter field is adjudicated (the P2-6 classification). The
/// adjudication sweep is data-driven from this table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Must be `Known(expected)` — geometry/route/exact-value obligation.
    Exact(u64),
    /// Must be `Known(n)` with `n <= expected` (#60 §9.5 threshold).
    Threshold(u64),
    /// A forbidden-work sentinel: must be `Known(0)` (#60 §9.5).
    Sentinel,
    /// A witness-constant threshold shared by all cells.
    ConstThreshold(u64),
}

/// The field classification for every decision-bearing counter of one
/// frozen cell (built from the frozen tables — never from results).
pub fn classification_for(cell: &FrozenCell) -> Vec<(&'static str, FieldKind)> {
    vec![
        // ---- scale diagnostics (#60 §9.5 exact + the H_max invariant) ----
        ("m_old", FieldKind::Exact(cell.m)),
        ("m_new", FieldKind::Exact(cell.m)),
        // h_old / h_new: recorded diagnostics with the H_max invariant —
        // handled directly in the sweep (Known + <= H_max), classified
        // here as the H_max threshold so the meta-test sees them.
        ("h_old", FieldKind::Threshold(cell.h_max)),
        ("h_new", FieldKind::Threshold(cell.h_max)),
        // ---- geometry / route (#60 §4, §9.5 exact) ----
        ("restart_old", FieldKind::Exact(cell.restart)),
        ("convergence_old", FieldKind::Exact(cell.convergence_old)),
        ("convergence_new", FieldKind::Exact(cell.convergence_new)),
        ("replace_lo", FieldKind::Exact(cell.replace_lo)),
        ("replace_hi", FieldKind::Exact(cell.replace_hi)),
        ("full_build_selected", FieldKind::Exact(0)),
        ("full_build_reason", FieldKind::Exact(0)),
        // ---- operator visits (#60 §9.5 thresholds) ----
        (
            "locate_node_visits",
            FieldKind::Threshold(cell.locate_visits_max),
        ),
        (
            "safe_predecessor_node_visits",
            FieldKind::Threshold(cell.safe_predecessor_visits_max),
        ),
        (
            "cursor_node_visits",
            FieldKind::Threshold(cell.cursor_visits_max),
        ),
        (
            "fact_range_node_visits",
            FieldKind::Threshold(cell.fact_range_visits_max),
        ),
        (
            "split_node_visits",
            FieldKind::Threshold(cell.split_visits_max),
        ),
        (
            "pivot_extract_node_visits",
            FieldKind::Threshold(cell.pivot_visits_max),
        ),
        (
            "join_node_visits",
            FieldKind::Threshold(cell.join_visits_max),
        ),
        (
            "bulk_build_node_visits",
            FieldKind::Exact(contract::DELTA_NEW),
        ),
        (
            "retire_node_visits",
            FieldKind::ConstThreshold(contract::DELTA_OLD),
        ),
        // ---- structural mutations ----
        (
            "sequence_link_writes",
            FieldKind::Threshold(cell.link_writes_max),
        ),
        ("avl_rotations", FieldKind::Threshold(cell.rotations_max)),
        // ---- aggregate field work ----
        (
            "aggregate_reads",
            FieldKind::Threshold(cell.aggregate_reads_max),
        ),
        (
            "aggregate_writes",
            FieldKind::Threshold(cell.aggregate_writes_max),
        ),
        // ---- certificate / candidate work ----
        (
            "certificate_reads",
            FieldKind::ConstThreshold(contract::CERTIFICATE_READS_MAX),
        ),
        (
            "certificate_writes",
            FieldKind::ConstThreshold(contract::CERTIFICATE_WRITES_MAX),
        ),
        (
            "candidate_checks",
            FieldKind::Exact(contract::CANDIDATE_CHECKS),
        ),
        (
            "cursor_advances",
            FieldKind::Exact(contract::CURSOR_ADVANCES),
        ),
        // ---- fresh / replacement / fact work (#60 §9.5 exact values) ----
        ("owners_created", FieldKind::Exact(contract::DELTA_NEW)),
        ("owners_removed", FieldKind::Exact(contract::DELTA_OLD)),
        (
            "fresh_payload_nodes_final",
            FieldKind::Exact(contract::FRESH_PAYLOAD_NODES_FINAL),
        ),
        (
            "fresh_payload_nodes_temporary",
            FieldKind::ConstThreshold(contract::FRESH_PAYLOAD_NODES_TEMPORARY_MAX),
        ),
        (
            "payload_nodes_retired",
            FieldKind::Exact(contract::P_REMOVED),
        ),
        (
            "old_fact_owner_visits",
            FieldKind::ConstThreshold(contract::OLD_FACT_OWNER_VISITS_MAX),
        ),
        ("old_facts_extracted", FieldKind::Exact(0)),
        ("new_facts_extracted", FieldKind::Exact(0)),
        ("facts_compared", FieldKind::Exact(0)),
        ("fact_compare_bytes", FieldKind::Exact(0)),
        (
            "reftable_entries_visited",
            FieldKind::Exact(contract::REFTABLE_ENTRIES_VISITED),
        ),
        // ---- retirement (#59 §11.1 separated quantities) ----
        (
            "retirement_frames_entered",
            FieldKind::ConstThreshold(contract::RETIREMENT_FRAMES_MAX),
        ),
        (
            "max_retirement_depth",
            FieldKind::Threshold(cell.max_retirement_depth_max),
        ),
        // ---- forbidden sentinels (Known(0)) ----
        (
            "forbidden_prefix_sequential_enumeration",
            FieldKind::Sentinel,
        ),
        (
            "forbidden_suffix_sequential_enumeration",
            FieldKind::Sentinel,
        ),
        (
            "forbidden_unaffected_payload_inspections",
            FieldKind::Sentinel,
        ),
        (
            "forbidden_unaffected_coordinate_writes",
            FieldKind::Sentinel,
        ),
        (
            "forbidden_unaffected_certificate_writes",
            FieldKind::Sentinel,
        ),
        ("forbidden_global_fact_recollection", FieldKind::Sentinel),
        ("forbidden_unaffected_old_retirement", FieldKind::Sentinel),
        ("forbidden_attribution_tree_walk", FieldKind::Sentinel),
    ]
}

fn is_hex_64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_hex_40(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The complete #60 §14 sweep over one row (pure; touches nothing).
pub fn adjudicate(cell: &FrozenCell, row: &RawRowV1) -> Adjudication {
    let mut invalid: Vec<String> = Vec::new();
    let mut fail: Vec<String> = Vec::new();

    // ---- INVALID: schema / study / contract / baselines / mechanism ----
    if row.schema != contract::RAW_SCHEMA {
        invalid.push(format!("schema:row:{}", row.schema));
    }
    if row.study_id != contract::STUDY_ID {
        invalid.push("study_id_mismatch".to_string());
    }
    if row.protocol != contract::PROTOCOL {
        invalid.push(format!("protocol:{}", row.protocol));
    }
    if row.frozen_contract_revision != contract::FROZEN_CONTRACT_REVISION {
        invalid.push("frozen_contract_revision_mismatch".to_string());
    }
    if row.study_mechanism_baseline != contract::STUDY_MECHANISM_BASELINE {
        invalid.push("study_mechanism_baseline_mismatch".to_string());
    }
    if row.authorization_baseline != contract::AUTHORIZATION_BASELINE {
        invalid.push("authorization_baseline_mismatch".to_string());
    }
    if !is_hex_40(&row.repository_commit) {
        invalid.push(format!(
            "provenance:repository_commit:{}",
            row.repository_commit
        ));
    }
    if !is_hex_40(&row.repository_tree) {
        invalid.push(format!(
            "provenance:repository_tree:{}",
            row.repository_tree
        ));
    }
    if row.mechanism != contract::MECHANISM
        || row.mechanism_design_head != contract::MECHANISM_DESIGN_HEAD
        || row.mechanism_merge != contract::MECHANISM_MERGE
    {
        invalid.push("mechanism_identity_mismatch".to_string());
    }

    // ---- INVALID: cell identity ----
    if row.cell_id != cell.cell_id {
        invalid.push(format!("cell_id:{}!={}", row.cell_id, cell.cell_id));
    }
    if row.case_id_hex != cell.case_id_hex {
        invalid.push("cell_identity:case_id_hex".to_string());
    }
    if row.n_bytes != cell.n_bytes || row.m != cell.m || row.target != cell.target {
        invalid.push("cell_identity:scale".to_string());
    }
    if row.edit_start != cell.edit_start || row.edit_end != cell.edit_start {
        invalid.push("cell_identity:edit_range".to_string());
    }
    if row.inserted_text_sha256 != contract::INSERTED_TEXT_SHA256 {
        invalid.push("cell_identity:inserted_text_sha256".to_string());
    }
    if row.pre_sha256 != cell.pre_sha256 {
        invalid.push("cell_identity:pre_sha256".to_string());
    }
    if row.post_sha256 != cell.post_sha256 {
        invalid.push("cell_identity:post_sha256".to_string());
    }
    if row.repetition > 2 {
        invalid.push(format!("repetition_out_of_range:{}", row.repetition));
    }

    // ---- INVALID: provenance capture completeness ----
    if !is_hex_64(&row.executable_sha256) {
        invalid.push("provenance:executable_sha256".to_string());
    }
    for (name, value) in [
        ("rustc", &row.rustc),
        ("cargo", &row.cargo),
        ("toolchain_channel", &row.toolchain_channel),
        ("build_profile", &row.build_profile),
        ("features", &row.features),
        ("execution_command", &row.execution_command),
        ("host.os", &row.host.os),
        ("host.kernel", &row.host.kernel),
        ("host.arch", &row.host.arch),
        ("host.cpu_model", &row.host.cpu_model),
    ] {
        if value.trim().is_empty() {
            invalid.push(format!("provenance_missing:{name}"));
        }
    }
    if row.counter_schema != contract::COUNTER_SCHEMA {
        invalid.push(format!("counter_schema:{}", row.counter_schema));
    }

    // ---- INVALID: counters map shape (never silently dropped fields) ----
    for name in super::schema::COUNTER_FIELDS {
        if !row.counters.contains_key(*name) {
            invalid.push(format!("counters_schema:missing_field:{name}"));
        }
    }
    for name in row.counters.keys() {
        if !super::schema::COUNTER_FIELDS.contains(&name.as_str()) {
            invalid.push(format!("counters_schema:unknown_field:{name}"));
        }
    }

    // ---- INVALID: row identity ----
    let expected_identity = RawRowV1::compute_row_identity(
        &row.study_id,
        &row.cell_id,
        row.repetition,
        &row.executable_sha256,
    );
    if row.row_identity != expected_identity {
        invalid.push("row_identity_mismatch".to_string());
    }

    // Invalid identity/provenance means the counters cannot be trusted as
    // decision-bearing evidence at all (#60 §12): INVALID wins.
    if !invalid.is_empty() {
        return Adjudication::Invalid(invalid);
    }

    // ---- FAIL: correctness gates (#60 §7/§14) ----
    if row.correctness.c1 != "PASS" {
        fail.push(format!("correctness:c1:{}", row.correctness.c1));
    }
    if row.correctness.c2 != "PASS" {
        fail.push(format!("correctness:c2:{}", row.correctness.c2));
    }
    if row.correctness.c3 != "PASS" {
        fail.push(format!("correctness:c3:{}", row.correctness.c3));
    }
    if !row.correctness.normalized_structural_equality {
        fail.push("correctness:normalized_structural_equality".to_string());
    }

    // ---- FAIL: route / fallback result must match the frozen witness ----
    if row.route != "local" {
        fail.push(format!("route:{}", row.route));
    }
    if row.full_build_reason != "none" {
        fail.push(format!("full_build_reason:{}", row.full_build_reason));
    }

    // ---- FAIL: the complete Known sweep (Unknown can never PASS) ----
    for (name, kind) in classification_for(cell) {
        let value = row.counters.get(name).copied();
        match (kind, value) {
            (_, None) => fail.push(format!("unknown:{name}")),
            (_, Some(Observed::Unknown)) => fail.push(format!("unknown:{name}")),
            (FieldKind::Exact(expected), Some(Observed::Known(n))) => {
                if n != expected {
                    fail.push(format!(
                        "exact_mismatch:{name}:Known({n}):expected:Known({expected})"
                    ));
                }
            }
            (FieldKind::Threshold(bound), Some(Observed::Known(n)))
            | (FieldKind::ConstThreshold(bound), Some(Observed::Known(n))) => {
                if n > bound {
                    fail.push(format!("threshold_exceeded:{name}:{n}>{bound}"));
                }
            }
            (FieldKind::Sentinel, Some(Observed::Known(n))) => {
                if n != 0 {
                    fail.push(format!("forbidden_sentinel:{name}:{n}"));
                }
            }
        }
    }

    // ---- FAIL: H_max invariant (implementation invariant failure, never
    // a larger budget — #60 §9.4). Distinct reason so it can never be
    // mistaken for an ordinary threshold event. ----
    for name in ["h_old", "h_new"] {
        if let Some(Observed::Known(h)) = row.counters.get(name).copied() {
            if h > cell.h_max {
                fail.push(format!(
                    "implementation_invariant_failure:{name}:{h}>H_max:{}",
                    cell.h_max
                ));
            }
        }
    }

    // ---- FAIL: convergence before EOF (#60 §4 frozen expectation) ----
    if let Some(Observed::Known(q)) = row.counters.get("convergence_old").copied() {
        if q >= cell.n_bytes {
            fail.push("eof_convergence:convergence_old".to_string());
        }
    }
    if let Some(Observed::Known(q)) = row.counters.get("convergence_new").copied() {
        if q >= cell.n_bytes + contract::INSERTED_TEXT.len() as u64 {
            fail.push("eof_convergence:convergence_new".to_string());
        }
    }

    // ---- FAIL: composite bounds (f1, f2 — #60 §9.5) ----
    let known = |name: &str| -> Option<u64> { row.counters.get(name).and_then(|o| o.value()) };
    if let (Some(l), Some(s)) = (
        known("locate_node_visits"),
        known("safe_predecessor_node_visits"),
    ) {
        if l + s > cell.f1_visits_max {
            fail.push(format!(
                "f1_composite_exceeded:{l}+{s}>{}",
                cell.f1_visits_max
            ));
        }
    }
    if let (Some(sp), Some(pv), Some(jv)) = (
        known("split_node_visits"),
        known("pivot_extract_node_visits"),
        known("join_node_visits"),
    ) {
        if sp + pv + jv > cell.f2_visits_max {
            fail.push(format!(
                "f2_composite_exceeded:{sp}+{pv}+{jv}>{}",
                cell.f2_visits_max
            ));
        }
    }

    if fail.is_empty() {
        Adjudication::Pass
    } else {
        Adjudication::Fail(fail)
    }
}

// ---------------------------------------------------------------------------
// Determinism and verdict derivation (#60 §15; task §42/§43) — mechanical,
// never averaged, never tolerance-based.
// ---------------------------------------------------------------------------

/// The determinism comparison payload of one row (#60 §15): every
/// structural work counter plus the branch/fallback reason.
fn determinism_key(row: &RawRowV1) -> (Vec<(String, Observed)>, String, String) {
    (
        row.counters.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        row.route.clone(),
        row.full_build_reason.clone(),
    )
}

/// Compare one cell's VALID structural observations (#60 §15): exact
/// equality of all structural work counters and the branch/fallback
/// reason. Rows adjudicated INVALID do not participate (their
/// instrumentation is not evidence); a missing participant makes
/// determinism unverifiable and the answer is `false`.
pub fn counters_deterministic(rows: &[&RawRowV1], adjudications: &[Adjudication]) -> bool {
    if rows.is_empty() || rows.len() != adjudications.len() {
        return false;
    }
    let participants: Vec<&RawRowV1> = rows
        .iter()
        .zip(adjudications.iter())
        .filter(|(_, a)| !matches!(a, Adjudication::Invalid(_)))
        .map(|(r, _)| *r)
        .collect();
    if participants.len() != rows.len() {
        return false;
    }
    let reference = determinism_key(participants[0]);
    participants[1..]
        .iter()
        .all(|r| determinism_key(r) == reference)
}

/// One cell's frozen verdict (task §43): all three repetitions must
/// satisfy the exact #60 rules and agree deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellVerdict {
    pub cell_id: String,
    /// Per-repetition adjudication statuses in repetition order.
    pub rep_statuses: Vec<String>,
    /// #60 §15 exact counter/route equality across the three valid rows.
    pub counters_deterministic: bool,
    /// PASS only when all repetitions PASS and the counters agree.
    pub verdict: String,
}

/// Derive one cell's verdict from its three raw rows (mechanical).
pub fn derive_cell_verdict(cell: &FrozenCell, rows: [&RawRowV1; 3]) -> CellVerdict {
    let adjudications: Vec<Adjudication> = rows.iter().map(|r| adjudicate(cell, r)).collect();
    let rep_statuses: Vec<String> = adjudications
        .iter()
        .map(|a| a.status().to_string())
        .collect();
    let refs: Vec<&RawRowV1> = rows.to_vec();
    let deterministic = counters_deterministic(&refs, &adjudications);
    let all_pass = rep_statuses.iter().all(|s| s == "PASS");
    CellVerdict {
        cell_id: cell.cell_id.to_string(),
        rep_statuses,
        counters_deterministic: deterministic,
        verdict: if all_pass && deterministic {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        },
    }
}

/// The overall frozen structural verdict (task §43): PASS requires ALL
/// three cells PASS. One valid FAIL means STRUCTURAL_FAIL. No weighting,
/// no averaging, no latency rescue.
pub fn derive_overall_verdict(cells: &[CellVerdict]) -> String {
    if cells.iter().all(|c| c.verdict == "PASS") {
        "STRUCTURAL_PASS".to_string()
    } else {
        "STRUCTURAL_FAIL".to_string()
    }
}
