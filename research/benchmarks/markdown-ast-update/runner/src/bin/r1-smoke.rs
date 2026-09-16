//! R1 smoke run: one deterministic case set through the REAL runner with
//! the null mechanism.
//!
//! Everything this binary prints or writes is
//!
//! ```text
//! R1_SMOKE_ONLY / NON_RESEARCH_RESULT
//! ```
//!
//! harness-validation output. It is NOT benchmark data; the timing values
//! are dummy values from a dummy mechanism and must never enter a
//! research table. Rows go to a temporary file, never to `results/`.

use std::env;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::ExitCode;

use markit_mdbench_common::CaseId;
use markit_mdbench_common::CaseKeyV1;
use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::ExecutionStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::OperationKind;
use markit_mdbench_common::Seed;
use markit_mdbench_instrumentation::InstantClock;
use markit_mdbench_null_r1::fixture::{
    smoke_fixture, smoke_payload_id, smoke_payload_shape, smoke_payload_size_bytes,
    R1_SMOKE_ONLY_GENERATOR_ID, SMOKE_EDIT_OPERATION,
};
use markit_mdbench_null_r1::{null_checksum, NullMechanism, NullPending};
use markit_mdbench_oracle::ScalarChecksumHook;
use markit_mdbench_runner::current_build_identity;
use markit_mdbench_runner::{
    assemble_row, build_initial_state, edit_meta, order_cases, run_full_parse_timed,
    run_update_timed, write_row, CaseFacts, PayloadMetaV1, SHUFFLE_ALGORITHM_ID,
};

/// Fixed smoke seed, recorded in the emitted rows.
const R1_SMOKE_SEED: u64 = 0x5EED_0000_0000_0001;

fn main() -> ExitCode {
    println!("== R1_SMOKE_ONLY / NON_RESEARCH_RESULT ==");
    println!("harness validation only -- NOT benchmark data, NOT a performance claim");

    let out_path = match env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => env::temp_dir().join("markit-mdbench-r1-smoke-nonresearch.jsonl"),
    };

    let build_identity = current_build_identity();
    let clock = InstantClock::new();
    let mechanism = NullMechanism::new();
    let (old, edit, post) = smoke_fixture();

    // ---- case identity (contains no mechanism/lane facts) -------------
    let payload_id = smoke_payload_id();
    let key_full_parse = CaseKeyV1 {
        payload_id: payload_id.0.clone(),
        payload_shape: smoke_payload_shape(),
        payload_size_bytes: smoke_payload_size_bytes(),
        old_source_sha256: old.sha256(),
        operation: OperationKind::FullParse,
        edit_start_byte: None,
        edit_end_byte: None,
        inserted_text_sha256: None,
        generator_id: Some(R1_SMOKE_ONLY_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .expect("smoke full-parse case key must validate");

    use sha2::{Digest, Sha256};
    let mut inserted_hasher = Sha256::new();
    inserted_hasher.update(edit.inserted_text().as_bytes());
    let key_update = CaseKeyV1 {
        payload_id: payload_id.0.clone(),
        payload_shape: smoke_payload_shape(),
        payload_size_bytes: smoke_payload_size_bytes(),
        old_source_sha256: old.sha256(),
        operation: SMOKE_EDIT_OPERATION,
        edit_start_byte: Some(edit.start_byte()),
        edit_end_byte: Some(edit.end_byte()),
        inserted_text_sha256: Some(inserted_hasher.finalize().into()),
        generator_id: Some(R1_SMOKE_ONLY_GENERATOR_ID.to_string()),
        generator_seed: None,
    }
    .validated()
    .expect("smoke update case key must validate");

    // ---- deterministic case order ------------------------------------
    let case_id_full_parse = CaseId::from_key(&key_full_parse);
    let case_id_update = CaseId::from_key(&key_update);
    let mut case_keys = vec![key_full_parse, key_update];
    order_cases(&mut case_keys, Seed(R1_SMOKE_SEED), CaseId::from_key);
    println!("seed = {R1_SMOKE_SEED:#018x}  shuffle_algorithm_id = {SHUFFLE_ALGORITHM_ID}");
    for (position, key) in case_keys.iter().enumerate() {
        println!(
            "case[{position}] = {} ({})",
            CaseId::from_key(key),
            key.operation.canonical_name()
        );
    }

    // ---- run both operations through the real runner (T-LANE) --------
    let expected_full_parse = null_checksum(&NullPending {
        old_len_bytes: old.len_bytes() as u64,
        post_len_bytes: old.len_bytes() as u64,
        edit_start_byte: 0,
        edit_end_byte: 0,
        inserted_len_bytes: 0,
        revision: 0,
    });
    let report_full_parse = run_full_parse_timed(
        &mechanism,
        &old,
        &clock,
        &ScalarChecksumHook::new(expected_full_parse),
    );

    // Old state is built BEFORE the edit case, outside every timer.
    let old_state = match build_initial_state(&mechanism, &old) {
        Ok(state) => state,
        Err(failure) => {
            eprintln!("R1_SMOKE_FAILED: initial state construction failed: {failure:?}");
            return ExitCode::FAILURE;
        }
    };
    let expected_update = null_checksum(&NullPending {
        old_len_bytes: old.len_bytes() as u64,
        post_len_bytes: post.len_bytes() as u64,
        edit_start_byte: edit.start_byte(),
        edit_end_byte: edit.end_byte(),
        inserted_len_bytes: edit.inserted_text_len_bytes(),
        revision: 1,
    });
    let report_update = run_update_timed(
        &mechanism,
        &old,
        &post,
        &edit,
        old_state,
        &clock,
        &ScalarChecksumHook::new(expected_update),
    );

    // ---- emit exactly two clearly-labeled NON_RESEARCH rows ----------
    let environment_ref = "manifest/environment.toml#r1-smoke";
    let provenance_ref = "R1_SMOKE_ONLY/NON_RESEARCH_RESULT";
    let payload_meta = PayloadMetaV1 {
        payload_id: payload_id.0.clone(),
        shape: smoke_payload_shape(),
        size_bytes: smoke_payload_size_bytes(),
    };

    let facts_full_parse = CaseFacts {
        case_id: case_id_full_parse,
        seed: Seed(R1_SMOKE_SEED),
        mechanism_id: mechanism.id(),
        operation: OperationKind::FullParse,
        payload: payload_meta.clone(),
        edit: edit_meta(None),
    };
    let facts_update = CaseFacts {
        case_id: case_id_update,
        seed: Seed(R1_SMOKE_SEED),
        mechanism_id: mechanism.id(),
        operation: SMOKE_EDIT_OPERATION,
        payload: payload_meta,
        edit: edit_meta(Some(&edit)),
    };

    let rows = [
        assemble_row(
            &facts_full_parse,
            &report_full_parse,
            &build_identity,
            environment_ref,
            provenance_ref,
        ),
        assemble_row(
            &facts_update,
            &report_update,
            &build_identity,
            environment_ref,
            provenance_ref,
        ),
    ];

    let write_result = fs::File::create(&out_path).and_then(|file| {
        let mut writer = BufWriter::new(file);
        for row in &rows {
            write_row(&mut writer, row)?;
        }
        Ok(())
    });
    if let Err(err) = write_result {
        eprintln!("R1_SMOKE_FAILED: could not write smoke rows to {out_path:?}: {err}");
        return ExitCode::FAILURE;
    }

    // ---- summary (still NON_RESEARCH) --------------------------------
    for row in &rows {
        println!(
            "row case={} op={} execution={:?} correctness={:?} (NON_RESEARCH)",
            &row.case_id[..12],
            row.operation.canonical_name(),
            row.execution_status,
            row.correctness_status,
        );
    }
    println!(
        "smoke rows written to {} (NON_RESEARCH_RESULT)",
        out_path.display()
    );

    let all_pass = rows.iter().all(|row| {
        row.execution_status == ExecutionStatus::Pass
            && row.correctness_status == CorrectnessStatus::Pass
    });
    if all_pass {
        println!("R1_SMOKE_OK");
        ExitCode::SUCCESS
    } else {
        println!("R1_SMOKE_FAILED");
        ExitCode::FAILURE
    }
}
