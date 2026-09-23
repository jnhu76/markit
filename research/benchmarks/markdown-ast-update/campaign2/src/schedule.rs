//! Session scheduling: seeded case order + rotated horse order
//! (task §19).
//!
//! Reuses the FROZEN Campaign-1 order machinery
//! (`markit_mdbench_runner::case_order::order_cases`, `SplitMix64V1`) and
//! the frozen Campaign-1 horse policy (`seeded-base-permutation-rotate-v1`)
//! unchanged. Campaign-2 adds no new ordering algorithm — it only derives
//! NEW seeds from the Campaign-2 root seed.

pub use markit_mdbench_campaign::schedule::{
    base_horse_permutation, horse_order_for, rotate, HORSE_ORDER_POLICY_ID,
};
pub use markit_mdbench_runner::{order_cases, SplitMix64V1, SHUFFLE_ALGORITHM_ID};

/// One scheduled case of a Campaign-2 session.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleRow2 {
    pub schema: String,
    pub sub_campaign_tag: String,
    pub surface: String,
    pub session_ordinal: u32,
    pub case_order_ordinal: u32,
    pub case_id: String,
    pub payload_id: String,
    pub horse_order: Vec<String>,
    /// Controlled-cell identity when the row belongs to Surface D.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_label: Option<String>,
    /// Lifecycle trace identity when the row belongs to Surface C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// One case as the scheduler needs it.
#[derive(Debug, Clone)]
pub struct ScheduleCase {
    pub case_id: markit_mdbench_common::CaseId,
    pub case_id_hex: String,
    pub payload_id: String,
    pub axis: Option<(String, String)>,
    pub trace_id: Option<String>,
}

/// Build the deterministic session schedule for one sub-campaign.
///
/// Case order: `order_cases` (frozen Campaign-1 shuffle) over the case ids
/// with the sub-campaign's session seed. Horse order: a seeded base
/// permutation rotated once per case and once per session, exactly the
/// frozen Campaign-1 policy. The schedule guarantees that the campaign
/// never executes "all H0, then all H1, ...".
pub fn build_schedule(
    tag: &str,
    surface: &str,
    session_ordinal: u32,
    session_seed_value: u64,
    horse_base_seed: u64,
    cases: &[ScheduleCase],
) -> Vec<ScheduleRow2> {
    let mut ordered: Vec<&ScheduleCase> = cases.iter().collect();
    order_cases(
        &mut ordered,
        markit_mdbench_common::Seed(session_seed_value),
        |case| case.case_id,
    );
    let base = base_horse_permutation(horse_base_seed);
    let mut rows = Vec::with_capacity(ordered.len());
    for (ordinal, case) in ordered.iter().enumerate() {
        rows.push(ScheduleRow2 {
            schema: "campaign2-schedule-v1".to_string(),
            sub_campaign_tag: tag.to_string(),
            surface: surface.to_string(),
            session_ordinal,
            case_order_ordinal: ordinal as u32,
            case_id: case.case_id_hex.clone(),
            payload_id: case.payload_id.clone(),
            horse_order: horse_order_for(&base, ordinal as u32, session_ordinal),
            axis: case.axis.as_ref().map(|(a, _)| a.clone()),
            axis_label: case.axis.as_ref().map(|(_, l)| l.clone()),
            trace_id: case.trace_id.clone(),
        });
    }
    rows
}

/// Serialize a schedule to JSONL.
pub fn schedule_to_jsonl(rows: &[ScheduleRow2]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(rows.len() * 256);
    for row in rows {
        serde_json::to_writer(&mut out, row).map_err(|e| format!("serialize schedule row: {e}"))?;
        out.push(b'\n');
    }
    Ok(out)
}

/// Parse a schedule JSONL file.
pub fn schedule_from_jsonl(bytes: &[u8]) -> Result<Vec<ScheduleRow2>, String> {
    let mut rows = Vec::new();
    for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let row: ScheduleRow2 = serde_json::from_slice(line)
            .map_err(|e| format!("schedule row {index}: {e}"))?;
        rows.push(row);
    }
    Ok(rows)
}
