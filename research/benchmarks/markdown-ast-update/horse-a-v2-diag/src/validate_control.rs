//! L0.5 comparator validation: prove `HORSE_A_V1_DIRECT_READY_REBUILD`
//! (arm B) satisfies the equivalent-READY contract against
//! `HORSE_A_V1_NORMAL` (arm A) on every frozen cell BEFORE any
//! treatment data is collected.
//!
//! Gates per cell (#98 §L0.5):
//!
//! ```text
//! RESULT_PARITY     normalized result equality A == B (and both == H0)
//! SEMANTIC_STATE    definition-facts equality (RefTable projection)
//! STRUCTURAL        validate_ready passes on both resulting states
//! NEXT_EDIT         one unchanged normal v1 edit from EACH resulting
//!                   state succeeds and produces equal results
//! LIFECYCLE         old-state retirement charged inside arm B's
//!                   boundary (owners_removed == M_old; retirement
//!                   frames > 0 for non-empty old states); recoverable
//!                   refusal parity on an invalid association
//! ```

use markit_mdbench_common::{
    CanonicalEdit, NoopWorkSink, Source, SourceId,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_horse_a::direct_ready::direct_ready_rebuild;
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::structural::{
    NoopHorseAStructuralSink, RecordingHorseAStructuralSink,
};
use markit_mdbench_horse_a::update::update;
use markit_mdbench_horse_a::validate::validate_ready;
use markit_mdbench_oracle::NormalizeV1;

use crate::cells::frozen_cells;

#[derive(serde::Serialize)]
pub struct CellValidation {
    pub cell_id: String,
    pub result_parity_ab: bool,
    pub result_parity_h0: bool,
    pub semantic_state: bool,
    pub structural_a: bool,
    pub structural_b: bool,
    pub next_edit_ok: bool,
    pub lifecycle_owners_removed_eq_m_old: bool,
    pub lifecycle_retirement_charged: bool,
    pub refusal_parity: bool,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_detail: Option<String>,
}

/// A deterministic second edit (for the NEXT_EDIT gate): replace one
/// byte at a safe interior offset of the post source with a different
/// ASCII letter, staying inside one text region.
fn second_edit(post: &Source) -> Option<CanonicalEdit> {
    let bytes = post.as_bytes();
    // find a lowercase letter well inside the document (offset 30..)
    for off in 30..bytes.len().min(60) {
        if bytes[off].is_ascii_lowercase() {
            let alt = if bytes[off] == b'z' { b'y' } else { b'z' };
            let s = (alt as char).to_string();
            return CanonicalEdit::new(off, off + 1, s).ok();
        }
    }
    None
}

pub fn validate_all() -> Result<Vec<CellValidation>, String> {
    let mut out = Vec::new();
    for cell in frozen_cells() {
        out.push(validate_one(&cell)?);
    }
    Ok(out)
}

fn validate_one(cell: &crate::cells::DiagCell) -> Result<CellValidation, String> {
    let fail = |v: &mut CellValidation, d: String| {
        v.passed = false;
        if v.failure_detail.is_none() {
            v.failure_detail = Some(d);
        }
    };

    let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
    let edit = cell.edit();
    let post_source = edit
        .apply(&pre_source, SourceId(1))
        .map_err(|e| format!("cell {}: post derivation failed: {e}", cell.id))?;

    // Fresh identical pre-states.
    let mut sink = NoopWorkSink;
    let pre_state = full_build(&pre_source, &mut sink, &mut NoopHorseAStructuralSink)
        .map_err(|e| format!("cell {}: pre full_build failed: {e}", cell.id))?;

    let mut v = CellValidation {
        cell_id: cell.id.to_string(),
        result_parity_ab: false,
        result_parity_h0: false,
        semantic_state: false,
        structural_a: false,
        structural_b: false,
        next_edit_ok: false,
        lifecycle_owners_removed_eq_m_old: false,
        lifecycle_retirement_charged: false,
        refusal_parity: false,
        passed: true,
        failure_detail: None,
    };

    // Arm A — the frozen normal path (no-op lane).
    let mut sink = NoopWorkSink;
    let post_a = match update(
        pre_state.clone(),
        &pre_source,
        &post_source,
        &edit,
        &mut sink,
    ) {
        Ok(s) => s,
        Err(e) => {
            fail(&mut v, format!("arm A failed: {e}"));
            return Ok(v);
        }
    };

    // Arm B — the diagnostic control, with structural recording so the
    // lifecycle gate can read the retirement ledger.
    let mut sink = NoopWorkSink;
    let mut structural = RecordingHorseAStructuralSink::new();
    let post_b = match direct_ready_rebuild(
        pre_state.clone(),
        &pre_source,
        &post_source,
        &edit,
        &mut sink,
        &mut structural,
    ) {
        Ok(s) => s,
        Err(e) => {
            fail(&mut v, format!("arm B failed: {e}"));
            return Ok(v);
        }
    };
    let b_counters = structural.into_counters();

    // Gate 1: normalized result parity A == B.
    let norm_a = post_a.normalize_v1();
    let norm_b = post_b.normalize_v1();
    v.result_parity_ab = norm_a == norm_b;
    if !v.result_parity_ab {
        fail(&mut v, "normalized results A != B".into());
    }

    // Gate 1b: both equal the independent H0 parse of the post source
    // (cell qualification — the oracle the campaign itself uses).
    let h0_post = parse_document(post_source.as_bytes());
    v.result_parity_h0 = norm_a == h0_post && norm_b == h0_post;
    if !v.result_parity_h0 {
        fail(&mut v, "normalized result disagrees with H0 oracle".into());
    }

    // Gate 2: semantic/reference state equality (definition facts).
    v.semantic_state = post_a.refs == post_b.refs;
    if !v.semantic_state {
        fail(&mut v, "RefTable projections differ".into());
    }

    // Gate 3: retained structural invariants.
    v.structural_a = validate_ready(&post_a).is_ok();
    v.structural_b = validate_ready(&post_b).is_ok();
    if !v.structural_a || !v.structural_b {
        fail(
            &mut v,
            format!(
                "validate_ready: A={:?} B={:?}",
                validate_ready(&post_a),
                validate_ready(&post_b)
            ),
        );
    }

    // Gate 4: one subsequent unchanged normal v1 edit from EACH state.
    let edit2 = match second_edit(&post_source) {
        Some(e) => e,
        None => {
            fail(&mut v, "no safe second edit found".into());
            return Ok(v);
        }
    };
    let post2_source = match edit2.apply(&post_source, SourceId(2)) {
        Ok(s) => s,
        Err(e) => {
            fail(&mut v, format!("second edit derivation failed: {e}"));
            return Ok(v);
        }
    };
    let run_next = |from: markit_mdbench_horse_a::state::ReadyDocument| -> bool {
        let mut s = NoopWorkSink;
        update(from, &post_source, &post2_source, &edit2, &mut s).is_ok()
    };
    let next_a_ok = run_next(post_a.clone());
    let next_b_ok = run_next(post_b.clone());
    v.next_edit_ok = next_a_ok && next_b_ok;
    if !v.next_edit_ok {
        fail(&mut v, format!("next edit: A ok = {next_a_ok}, B ok = {next_b_ok}"));
    }
    // and the two second-generation results must also agree
    if v.next_edit_ok {
        let mut s = NoopWorkSink;
        let r1 = update(post_a, &post_source, &post2_source, &edit2, &mut s)
            .expect("checked above");
        let mut s = NoopWorkSink;
        let r2 = update(post_b, &post_source, &post2_source, &edit2, &mut s)
            .expect("checked above");
        if r1.normalize_v1() != r2.normalize_v1() {
            v.next_edit_ok = false;
            fail(&mut v, "second-generation results differ".into());
        }
    }

    // Gate 5: lifecycle — retirement charged INSIDE arm B's boundary.
    let m_old = pre_state.owners.records() as u64;
    let owners_removed = b_counters
        .owners_removed
        .value()
        .unwrap_or(u64::MAX);
    v.lifecycle_owners_removed_eq_m_old = owners_removed == m_old;
    let retirement_frames = b_counters
        .retirement_frames_entered
        .value()
        .unwrap_or(0);
    v.lifecycle_retirement_charged = retirement_frames > 0 || m_old == 0;
    if !v.lifecycle_owners_removed_eq_m_old || !v.lifecycle_retirement_charged {
        fail(
            &mut v,
            format!(
                "lifecycle: owners_removed={owners_removed} (M_old={m_old}), frames={retirement_frames}"
            ),
        );
    }

    // Gate 6: recoverable-refusal parity — an invalid association is
    // refused identically by both arms, and arm B refuses WITHOUT
    // building (no owners_created charged on refusal).
    let bad_edit = CanonicalEdit::new(pre_source.len_bytes() + 10, pre_source.len_bytes() + 11, "x")
        .expect("out-of-range canonical edit constructs");
    let mut s = NoopWorkSink;
    let a_refused = update(
        pre_state.clone(),
        &pre_source,
        &post_source,
        &bad_edit,
        &mut s,
    )
    .is_err();
    let mut s = NoopWorkSink;
    let mut structural = RecordingHorseAStructuralSink::new();
    let b_result = direct_ready_rebuild(
        pre_state,
        &pre_source,
        &post_source,
        &bad_edit,
        &mut s,
        &mut structural,
    );
    let b_counters_refusal = structural.into_counters();
    let b_refused = b_result.is_err();
    // Frozen record semantics: a counter that never charged stays
    // `Unknown` (not `Known(0)`). Refusal cleanliness = NO evidence of
    // any build/retirement work: value is Unknown or Known(0).
    let clean = |v: markit_mdbench_horse_a::structural::Observed| {
        !matches!(v, markit_mdbench_horse_a::structural::Observed::Known(n) if n > 0)
    };
    let b_built_nothing = clean(b_counters_refusal.owners_created)
        && clean(b_counters_refusal.retirement_frames_entered);
    v.refusal_parity = a_refused && b_refused && b_built_nothing;
    if !v.refusal_parity {
        fail(
            &mut v,
            format!(
                "refusal: A refused={a_refused}, B refused={b_refused}, B clean={}",
                b_counters_refusal.owners_created.value() == Some(0)
            ),
        );
    }

    Ok(v)
}
