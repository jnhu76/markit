//! MARKIT-31 primary analysis cross-check (task §58).
//!
//! Second implementation of the Stage-A statistics for 10 predetermined
//! EDIT_WRITE CaseIds x all 5 horses x all 3 sessions, calling the
//! FROZEN primitives in `campaign::stats` directly (no re-derivation).
//! Emits JSON on stdout; a Python comparator checks agreement with the
//! committed CSV artifacts (integer quantiles exact; ratios within
//! floating-point tolerance).

use std::collections::BTreeMap;
use std::path::PathBuf;

use markit_mdbench_campaign::stats::{
    geometric_mean, median_of_three, session_cell_summary, session_speedup,
    session_unstable,
};

use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: stats-crosscheck <raw-run-dir> <case-list-file>");
        std::process::exit(2);
    }
    let run_dir = PathBuf::from(&args[1]);
    let case_list: Vec<String> = std::fs::read_to_string(&args[2])
        .expect("case list")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let selected: std::collections::BTreeSet<&str> =
        case_list.iter().map(String::as_str).collect();

    // (case, mech, session) -> {prepare, native, total} measured samples
    let mut cells: BTreeMap<(String, String, u32), (Vec<u64>, Vec<u64>, Vec<u64>)> =
        BTreeMap::new();
    let timing_dir = run_dir.join("timing");
    let mut files: Vec<_> = std::fs::read_dir(&timing_dir)
        .expect("timing dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
        .collect();
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("timing file");
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let obs: serde_json::Value = serde_json::from_str(line).expect("json");
            if obs["sample_kind"] != "measured" {
                continue;
            }
            let case = obs["result_row_v2"]["case_id"].as_str().expect("case");
            if !selected.contains(case) {
                continue;
            }
            let mech = obs["result_row_v2"]["mechanism_id"]
                .as_str()
                .expect("mech");
            let session = obs["session_ordinal"].as_u64().expect("session") as u32;
            let metrics = &obs["result_row_v2"]["measurement"]["metrics"];
            let entry = cells
                .entry((case.to_string(), mech.to_string(), session))
                .or_default();
            if let Some(p) = metrics["prepare_ns"].as_u64() {
                entry.0.push(p);
            }
            entry
                .1
                .push(metrics["native_ns"].as_u64().expect("native_ns"));
            entry.2.push(metrics["total_ns"].as_u64().expect("total_ns"));
        }
    }

    // Cell summaries via the frozen primitive.
    let mut cell_rows = Vec::new();
    let mut per_case_horse: BTreeMap<(String, String), BTreeMap<u32, (u64, u64, u64)>> =
        BTreeMap::new();
    for ((case, mech, session), (prep, nat, tot)) in &cells {
        let native = session_cell_summary(nat).expect("native cell");
        let total = session_cell_summary(tot).expect("total cell");
        let prepare = if prep.len() == 30 {
            let v = session_cell_summary(prep).expect("prepare cell");
            json!([v.0, v.1])
        } else {
            // whole-cell NOT_APPLICABLE (schema semantics)
            assert!(prep.is_empty(), "partial prepare coverage");
            serde_json::Value::Null
        };
        per_case_horse
            .entry((case.clone(), mech.clone()))
            .or_default()
            .insert(*session, (native.0, native.1, total.0));
        cell_rows.push(json!({
            "case": case, "mech": mech, "session": session,
            "prepare": prepare,
            "native_p50": native.0, "native_p95": native.1,
            "total_p50": total.0, "total_p95": total.1,
        }));
    }
    cell_rows.sort_by_key(|r| {
        (
            r["case"].as_str().unwrap().to_string(),
            r["mech"].as_str().unwrap().to_string(),
            r["session"].as_u64().unwrap(),
        )
    });

    // Case estimates + H0-relative speedups via the frozen primitives.
    let mut est_rows = Vec::new();
    for ((case, mech), sessions) in &per_case_horse {
        if sessions.len() != 3 {
            panic!("case {case} mech {mech}: missing sessions");
        }
        let get = |s: u32| sessions[&s];
        let med = |f: fn((u64, u64, u64)) -> u64| {
            median_of_three(f(get(0)), f(get(1)), f(get(2)))
        };
        let native_p50 = med(|t| t.0);
        let total_p50 = med(|t| t.2);
        let unstable = session_unstable(&[get(0).2, get(1).2, get(2).2]).is_some();
        let speedup = if mech == "h0-full-rebuild" {
            1.0
        } else {
            let h0 = &per_case_horse[&(case.clone(), "h0-full-rebuild".to_string())];
            let ratios: Vec<f64> = (0u32..3)
                .map(|s| session_speedup(h0[&s].2, get(s).2).expect("speedup"))
                .collect();
            geometric_mean(&ratios).expect("geomean")
        };
        est_rows.push(json!({
            "case": case, "mech": mech,
            "native_p50": native_p50, "total_p50": total_p50,
            "unstable": unstable,
            "speedup_geomean": speedup,
        }));
    }
    est_rows.sort_by_key(|r| {
        (
            r["case"].as_str().unwrap().to_string(),
            r["mech"].as_str().unwrap().to_string(),
        )
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "cells": cell_rows,
            "estimates": est_rows,
        }))
        .expect("json")
    );
}
