//! Campaign schedule: case order + horse order + cardinality policy
//! (task §10, §14, §15, §48).
//!
//! Case order (per `surface × session`):
//!
//! ```text
//! stable sort of case identities (CaseId bytes)
//!   -> seeded Fisher-Yates shuffle with the frozen algorithm
//!      splitmix64-v1 + fisher-yates-lemire-rejection-v2
//!      (REUSED from the runner's `order_cases`; never re-implemented)
//! ```
//!
//! Horse order (per scheduled case; task §15, deterministic and
//! position-balanced WITHOUT adaptivity):
//!
//! ```text
//! 1. one seeded base permutation of H0-H4, derived from the campaign
//!    root seed (identity order sorted by horse label, then the same
//!    frozen shuffle);
//! 2. for case order ordinal j (0-based) in session s (0-based):
//!    execution order = base permutation rotated by (j + s) mod 5.
//! ```
//!
//! Because the case order itself is a uniform shuffle, the rotation
//! provides deterministic near-exact position balance with no adaptive
//! procedure and no dependence on observed latency.
//!
//! The schedule is materialized BEFORE timing and never regenerated
//! after results are seen; running the generator twice from the same
//! frozen inputs must produce byte-identical output (task §48).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use markit_mdbench_runner::SplitMix64V1;

use crate::{Surface, HORSE_IDS, SCHEDULE_SCHEMA};

/// Identifier of the frozen horse-order policy.
pub const HORSE_ORDER_POLICY_ID: &str = "seeded-base-permutation-rotate-v1";

/// The frozen shuffle over an ALREADY-SORTED list: the exact
/// `splitmix64-v1 + fisher-yates-lemire-rejection-v2` frame from the
/// runner's `order_cases`, applied to fixed-length lowercase hex ids
/// (whose lexicographic string order equals their CaseId byte order).
/// Reusing the exported frozen PRNG primitive — never a second
/// implementation.
fn shuffle_sorted<T>(sorted: &mut [T], seed: u64) {
    let mut rng = SplitMix64V1::new(seed);
    for i in (1..sorted.len()).rev() {
        let j = rng.next_below(i + 1);
        sorted.swap(i, j);
    }
}

/// One schedule manifest row (JSONL): one scheduled case in one session
/// of one surface, with its exact horse execution order recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ScheduleRow {
    pub schema: String,
    pub campaign_spec_id: String,
    pub surface: String,
    /// 0-based session ordinal (0..session_count).
    pub session_ordinal: u32,
    /// 0-based position of the case inside the session's shuffled order.
    pub order_ordinal: u32,
    /// hex CaseId (sha256-of-casekey-v1) of the scheduled case.
    pub case_id: String,
    /// Payload/source identity: full-read payload id (`full-read:<key>`)
    /// or the frozen EDIT_WRITE `payload_id`.
    pub payload_id: String,
    /// Source key (`<source_id>/files/<upstream path>`).
    pub source_key: String,
    /// Trace id where applicable (EDIT_WRITE; `None` for CLEAN_STATE).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Exact horse execution order for THIS case (task §15: record the
    /// exact horse order for every case).
    pub horse_order: Vec<String>,
}

/// Derive the seeded base H0-H4 permutation (sorted identity order ->
/// frozen seeded shuffle).
pub fn base_horse_permutation(horse_base_seed: u64) -> Vec<String> {
    // Lexically sorted identity order first, so the input enumeration
    // order is irrelevant — exactly like `order_cases` sorts by CaseId.
    let mut horses: Vec<String> = HORSE_IDS.iter().map(|h| h.to_string()).collect();
    horses.sort();
    shuffle_sorted(&mut horses, horse_base_seed);
    horses
}

/// Rotate `base` by `k`: the rotated sequence starts at index `k` of the
/// base permutation and wraps around.
pub fn rotate<T: Clone>(base: &[T], k: usize) -> Vec<T> {
    let n = base.len();
    debug_assert!(n > 0);
    (0..n).map(|m| base[(k + m) % n].clone()).collect()
}

/// Execution horse order for case order ordinal `j` (0-based) in session
/// `s` (0-based): `rotate(base, (j + s) mod 5)` (task §15).
pub fn horse_order_for(base: &[String], j: u32, s: u32) -> Vec<String> {
    rotate(base, (j as usize + s as usize) % base.len())
}

/// Decode-check a hex CaseId (64 lowercase hex chars).
pub fn validate_case_id_hex(hex: &str) -> Result<(), String> {
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!("case id {hex:?} is not 64 lowercase hex chars"));
    }
    Ok(())
}

/// One case identity: (case_id_hex, payload_id, source_key, trace_id).
pub type CaseIdentity = (String, String, String, Option<String>);

/// Inputs to schedule generation: the frozen per-surface case identities.
#[derive(Debug, Clone)]
pub struct SurfaceCases {
    pub surface: Surface,
    pub cases: Vec<CaseIdentity>,
}

/// The deterministic per-session order of one surface: stable sort by
/// CaseId (hex strings of fixed length sort identically to the CaseId
/// bytes), then the frozen seeded shuffle. Shared by generation AND
/// verification, so any order corruption is recomputed against (task
/// §48).
pub fn session_case_order(
    surface: &SurfaceCases,
    root_seed: u64,
    session: u32,
) -> Result<Vec<CaseIdentity>, String> {
    let mut ordered: Vec<CaseIdentity> = surface.cases.clone();
    // Stable sorted list of case identities, sorted by CaseId bytes.
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    if ordered.is_empty() {
        return Err(format!(
            "surface {} has no cases; the frozen workload must be consumed as-is",
            surface.surface.as_str()
        ));
    }
    for (case_id, ..) in &ordered {
        validate_case_id_hex(case_id)?;
    }
    let seed = crate::identity::session_seed(root_seed, surface.surface.as_str(), session);
    // Shuffle the SORTED list through the frozen machinery (sort +
    // seeded Fisher-Yates); hex strings of fixed length sort identically
    // to the CaseId bytes they encode, so this is the runner's
    // `order_cases` permutation.
    shuffle_sorted(&mut ordered, seed);
    Ok(ordered)
}

/// Generate the full deterministic schedule (all surfaces × all
/// sessions). Byte-deterministic for identical inputs.
pub fn generate_schedule(
    campaign_spec_id: &str,
    session_count: u32,
    root_seed: u64,
    surfaces: &[SurfaceCases],
) -> Result<Vec<ScheduleRow>, String> {
    let horse_base = base_horse_permutation(crate::identity::horse_base_seed(root_seed));
    let mut rows = Vec::new();
    for surface in surfaces {
        for s in 0..session_count {
            // Materialize the shuffled order BEFORE timing, freezing
            // payload/source identity alongside each position.
            let entries = session_case_order(surface, root_seed, s)?;
            for (j, entry) in entries.iter().enumerate() {
                rows.push(ScheduleRow {
                    schema: SCHEDULE_SCHEMA.to_string(),
                    campaign_spec_id: campaign_spec_id.to_string(),
                    surface: surface.surface.as_str().to_string(),
                    session_ordinal: s,
                    order_ordinal: j as u32,
                    case_id: entry.0.clone(),
                    payload_id: entry.1.clone(),
                    source_key: entry.2.clone(),
                    trace_id: entry.3.clone(),
                    horse_order: horse_order_for(&horse_base, j as u32, s),
                });
            }
        }
    }
    Ok(rows)
}

/// Serialize schedule rows to canonical JSONL bytes.
pub fn schedule_to_jsonl(rows: &[ScheduleRow]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for row in rows {
        let line =
            serde_json::to_string(row).map_err(|e| format!("serialize schedule row: {e}"))?;
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

/// Parse schedule JSONL bytes.
pub fn schedule_from_jsonl(bytes: &[u8]) -> Result<Vec<ScheduleRow>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("schedule utf8: {e}"))?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(
            serde_json::from_str(line).map_err(|e| format!("schedule line {}: {e}", index + 1))?,
        );
    }
    Ok(rows)
}

/// Expected schedule cardinality for the FROZEN primary campaign (task
/// §10): rows are per scheduled case (the per-horse expansion happens
/// at execution, not in the manifest).
pub fn expected_frozen_schedule_rows(session_count: u32) -> usize {
    session_count as usize
        * (Surface::CleanState.frozen_case_count() + Surface::EditWrite.frozen_case_count())
}

/// Verify a schedule against the frozen policy: exact cardinality, no
/// duplicates, no missing cases, contiguous ordinals, correct horse
/// orders, session count, spec id. Returns a list of blockers (empty =
/// valid).
pub fn verify_schedule(
    rows: &[ScheduleRow],
    campaign_spec_id: &str,
    session_count: u32,
    root_seed: u64,
    surfaces: &[SurfaceCases],
) -> Result<(), Vec<String>> {
    let mut blockers: Vec<String> = Vec::new();
    let horse_base = base_horse_permutation(crate::identity::horse_base_seed(root_seed));

    let expected_total = session_count as usize
        * surfaces
            .iter()
            .map(|surface| surface.cases.len())
            .sum::<usize>();
    if rows.len() != expected_total {
        blockers.push(format!(
            "schedule row count {} != expected {expected_total} \
             (session_count={session_count})",
            rows.len()
        ));
    }

    for surface in surfaces {
        let surface_name = surface.surface.as_str();
        for s in 0..session_count {
            let expected_entries = match session_case_order(surface, root_seed, s) {
                Ok(entries) => entries,
                Err(error) => {
                    blockers.push(error);
                    continue;
                }
            };
            let mut session_rows: Vec<&ScheduleRow> = rows
                .iter()
                .filter(|r| r.surface == surface_name && r.session_ordinal == s)
                .collect();
            session_rows.sort_by_key(|r| r.order_ordinal);
            let expected_n = surface.cases.len();
            if session_rows.len() != expected_n {
                blockers.push(format!(
                    "{surface_name} session {s}: {} scheduled cases != {expected_n}",
                    session_rows.len()
                ));
                continue;
            }
            // Contiguous 0-based ordinals.
            for (j, row) in session_rows.iter().enumerate() {
                if row.order_ordinal != j as u32 {
                    blockers.push(format!(
                        "{surface_name} session {s}: order_ordinal {} at position {j}",
                        row.order_ordinal
                    ));
                }
                if row.schema != SCHEDULE_SCHEMA {
                    blockers.push(format!(
                        "{surface_name} session {s} ordinal {j}: schema {:?} != {SCHEDULE_SCHEMA}",
                        row.schema
                    ));
                }
                if row.campaign_spec_id != campaign_spec_id {
                    blockers.push(format!(
                        "{surface_name} session {s} ordinal {j}: campaign_spec_id drift"
                    ));
                }
                // Exact horse order recomputation (unknown horse included).
                let expected_order = horse_order_for(&horse_base, j as u32, s);
                if row.horse_order != expected_order {
                    blockers.push(format!(
                        "{surface_name} session {s} ordinal {j}: horse order {:?} != recomputed {:?}",
                        row.horse_order, expected_order
                    ));
                }
                // Exact case identity recomputation: the whole shuffled
                // order is derived from the seed, so any order
                // corruption or identity drift is caught positionally.
                let expected_entry = &expected_entries[j];
                if (
                    row.case_id.as_str(),
                    row.payload_id.as_str(),
                    row.source_key.as_str(),
                    row.trace_id.as_ref(),
                ) != (
                    expected_entry.0.as_str(),
                    expected_entry.1.as_str(),
                    expected_entry.2.as_str(),
                    expected_entry.3.as_ref(),
                ) {
                    blockers.push(format!(
                        "{surface_name} session {s} ordinal {j}: case identity mismatch (order corruption or identity drift): on disk (case {:?}, payload {:?}, source {:?}, trace {:?}) vs recomputed (case {:?}, payload {:?}, source {:?}, trace {:?})",
                        row.case_id,
                        row.payload_id,
                        row.source_key,
                        row.trace_id,
                        expected_entry.0,
                        expected_entry.1,
                        expected_entry.2,
                        expected_entry.3
                    ));
                }
                for horse in &row.horse_order {
                    if !HORSE_IDS.contains(&horse.as_str()) {
                        blockers.push(format!(
                            "{surface_name} session {s} ordinal {j}: unknown horse {horse:?}"
                        ));
                    }
                }
                if row.horse_order.len() != HORSE_IDS.len() {
                    blockers.push(format!(
                        "{surface_name} session {s} ordinal {j}: horse order has {} entries != 5",
                        row.horse_order.len()
                    ));
                }
            }
            // No duplicate / missing case ids in this session.
            let seen: BTreeSet<&str> = session_rows.iter().map(|r| r.case_id.as_str()).collect();
            if seen.len() != session_rows.len() {
                blockers.push(format!(
                    "{surface_name} session {s}: duplicate case ids ({} unique of {})",
                    seen.len(),
                    session_rows.len()
                ));
            }
            let expected: BTreeSet<&str> =
                surface.cases.iter().map(|(id, ..)| id.as_str()).collect();
            let missing: Vec<&str> = expected.difference(&seen).copied().collect();
            let extra: Vec<&str> = seen.difference(&expected).copied().collect();
            if !missing.is_empty() {
                blockers.push(format!(
                    "{surface_name} session {s}: missing cases {}",
                    missing.join(",")
                ));
            }
            if !extra.is_empty() {
                blockers.push(format!(
                    "{surface_name} session {s}: unknown cases {}",
                    extra.join(",")
                ));
            }
        }
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_surface(surface: Surface, n: usize) -> SurfaceCases {
        let cases = (0..n)
            .map(|i| {
                let case_id = synthetic_case_id_hex(i);
                (
                    case_id,
                    format!("payload-{i}"),
                    format!("proj/files/f{i}.md"),
                    if surface == Surface::EditWrite {
                        Some(format!("trace-{i}"))
                    } else {
                        None
                    },
                )
            })
            .collect();
        SurfaceCases { surface, cases }
    }

    /// Deterministic synthetic CaseId hexes: distinct per index, stable
    /// across calls (sha256 of the index label — NOT a real case
    /// identity; schedule machinery only needs a total order).
    fn synthetic_case_id_hex(i: usize) -> String {
        crate::sha256_hex(format!("synthetic-schedule-case-{i}").as_bytes())
    }

    #[test]
    fn schedule_generation_is_deterministic() {
        let surfaces = vec![
            synthetic_surface(Surface::CleanState, 22),
            synthetic_surface(Surface::EditWrite, 30),
        ];
        let a = generate_schedule("spec", 3, 0xA1B2C3D4E5F60718, &surfaces).unwrap();
        let b = generate_schedule("spec", 3, 0xA1B2C3D4E5F60718, &surfaces).unwrap();
        assert_eq!(
            schedule_to_jsonl(&a).unwrap(),
            schedule_to_jsonl(&b).unwrap()
        );
        assert_eq!(a.len(), 3 * (22 + 30));
        assert!(verify_schedule(&a, "spec", 3, 0xA1B2C3D4E5F60718, &surfaces).is_ok());
    }

    #[test]
    fn seed_drift_changes_the_schedule() {
        let surfaces = vec![synthetic_surface(Surface::EditWrite, 40)];
        let a = generate_schedule("spec", 3, 1, &surfaces).unwrap();
        let b = generate_schedule("spec", 3, 2, &surfaces).unwrap();
        let orders_a: Vec<String> = a.iter().map(|r| r.case_id.clone()).collect();
        let orders_b: Vec<String> = b.iter().map(|r| r.case_id.clone()).collect();
        assert_ne!(
            orders_a, orders_b,
            "different root seeds must reorder cases"
        );
        // The drifted schedule must FAIL verification under the original
        // seed (order corruption / seed drift detection, task §48).
        assert!(verify_schedule(&b, "spec", 3, 1, &surfaces).is_err());
    }

    #[test]
    fn horse_order_is_position_balanced_and_deterministic() {
        let base = base_horse_permutation(0x5EED_0000_0000_0005);
        assert_eq!(base.len(), 5);
        let mut sorted = base.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            HORSE_IDS.to_vec(),
            "base permutation is a permutation"
        );
        // Rotation by (j + s): for a full cycle of 5 consecutive (j+s)
        // values each horse occupies each position exactly once.
        for m in 0..5 {
            let order = horse_order_for(&base, m, 0);
            let mut sorted_order = order.clone();
            sorted_order.sort();
            assert_eq!(sorted_order, HORSE_IDS.to_vec());
        }
        // (j + s) arithmetic: s advances the rotation like j does.
        assert_eq!(horse_order_for(&base, 0, 2), horse_order_for(&base, 2, 0));
        assert_eq!(horse_order_for(&base, 4, 1), horse_order_for(&base, 0, 0));
    }

    #[test]
    fn negative_missing_duplicate_unknown_horse_wrong_sessions() {
        let surfaces = vec![synthetic_surface(Surface::CleanState, 22)];
        let seed = 7u64;
        let rows = generate_schedule("spec", 3, seed, &surfaces).unwrap();
        assert!(verify_schedule(&rows, "spec", 3, seed, &surfaces).is_ok());

        // Missing case (drop one row of session 1).
        let mut missing = rows.clone();
        missing.retain(|r| {
            !(r.surface == "clean_state" && r.session_ordinal == 1 && r.order_ordinal == 3)
        });
        let blockers = verify_schedule(&missing, "spec", 3, seed, &surfaces).unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| b.contains("missing cases") || b.contains("row count")));

        // Duplicate case (clone a row over another position).
        let mut dup = rows.clone();
        let clone_from = dup
            .iter()
            .position(|r| r.session_ordinal == 2 && r.order_ordinal == 0)
            .unwrap();
        let cloned_row = dup[clone_from].clone();
        let target = dup
            .iter_mut()
            .find(|r| r.session_ordinal == 2 && r.order_ordinal == 1)
            .unwrap();
        *target = cloned_row;
        let blockers = verify_schedule(&dup, "spec", 3, seed, &surfaces).unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| b.contains("duplicate case ids") || b.contains("missing cases")));

        // Unknown horse inside the recorded order.
        let mut unknown = rows.clone();
        unknown[0].horse_order[0] = "H9".to_string();
        let blockers = verify_schedule(&unknown, "spec", 3, seed, &surfaces).unwrap_err();
        assert!(blockers.iter().any(|b| b.contains("unknown horse")));

        // Wrong session count (only 2 sessions materialized).
        let short: Vec<ScheduleRow> = rows
            .iter()
            .filter(|r| r.session_ordinal < 2)
            .cloned()
            .collect();
        let blockers = verify_schedule(&short, "spec", 3, seed, &surfaces).unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| b.contains("session 2") || b.contains("row count")));

        // Order corruption: two cases exchange ordinals, so each
        // position carries the wrong case identity. Verification must
        // recompute the seeded order and catch it positionally.
        let mut corrupt = rows.clone();
        let a = corrupt
            .iter()
            .position(|r| r.session_ordinal == 0 && r.order_ordinal == 5)
            .unwrap();
        let b = corrupt
            .iter()
            .position(|r| r.session_ordinal == 0 && r.order_ordinal == 6)
            .unwrap();
        let (ordinal_a, ordinal_b) = (corrupt[a].order_ordinal, corrupt[b].order_ordinal);
        corrupt[a].order_ordinal = ordinal_b;
        corrupt[b].order_ordinal = ordinal_a;
        let blockers = verify_schedule(&corrupt, "spec", 3, seed, &surfaces).unwrap_err();
        assert!(blockers
            .iter()
            .any(|b| b.contains("order corruption") || b.contains("identity drift")));
    }
}
