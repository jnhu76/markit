//! mdbench-horse-a-v2-diag — the Issue #98 L0+L1 diagnostic driver.
//!
//! Subcommands:
//!
//! ```text
//! manifest    dump the frozen cell manifest receipt (IDs + SHA-256)
//! validate    L0.5 comparator validation (must ALL pass before treatment)
//! tlane       T-LANE comparator timing (release build)
//! alane       A-LANE work counters (un-timed, attributed)
//! effects     offline required-effect ledger (H0 oracle)
//! all         validate, then (only if every gate passes) tlane + alane
//!             + effects, writing one JSON artifact per lane
//! ```

use std::io::Write;
use std::path::PathBuf;

use markit_mdbench_common::{NoopWorkSink, Source, SourceId};
use markit_mdbench_horse_a::full_build::full_build;
use markit_mdbench_horse_a::structural::NoopHorseAStructuralSink;
use markit_mdbench_horse_a::update::update;

use markit_mdbench_horse_a_v2_diag::{alane, cells, decompose, effects, tlane, validate_control};

fn usage() -> ! {
    eprintln!(
        "usage: mdbench-horse-a-v2-diag <manifest|validate|tlane|alane|effects|all> [--out DIR]"
    );
    std::process::exit(2);
}

fn write_json(path: &PathBuf, value: &serde_json::Value) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    let s = serde_json::to_string_pretty(value).unwrap();
    f.write_all(s.as_bytes())?;
    f.write_all(b"\n")?;
    println!("wrote {}", path.display());
    Ok(())
}

fn run_metadata() -> serde_json::Value {
    serde_json::json!({
        "driver": "mdbench-horse-a-v2-diag",
        "issue": "98",
        "schema": "HORSE-A-V2-DIAG-98-RUN-v1",
        "argv": std::env::args().collect::<Vec<_>>(),
    })
}

fn cmd_effects(out: Option<PathBuf>) -> Result<i32, String> {
    let mut ledgers = Vec::new();
    for cell in cells::frozen_cells() {
        let pre_source = Source::new(SourceId(0), cell.pre_source.clone());
        let edit = cell.edit();
        let post_source = edit
            .apply(&pre_source, SourceId(1))
            .map_err(|e| format!("cell {}: {e}", cell.id))?;
        let mut sink = NoopWorkSink;
        let pre_state = full_build(&pre_source, &mut sink, &mut NoopHorseAStructuralSink)
            .map_err(|e| format!("cell {}: pre full_build: {e}", cell.id))?;
        let mut sink = NoopWorkSink;
        let post_a = update(
            pre_state.clone(),
            &pre_source,
            &post_source,
            &edit,
            &mut sink,
        )
        .map_err(|e| format!("cell {}: arm A: {e}", cell.id))?;
        ledgers.push(effects::effect_ledger(
            cell.id,
            &cell.pre_source,
            post_source.as_str(),
            &pre_state,
            &post_a,
        ));
    }
    let value = serde_json::json!({
        "meta": run_metadata(),
        "note": "offline diagnostic oracle (two H0 parses + tree diff); NOT a free online mechanism",
        "ledgers": ledgers,
    });
    if let Some(dir) = out {
        write_json(&dir.join("effects.json"), &value).map_err(|e| e.to_string())?;
    } else {
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    }
    Ok(0)
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cmd: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(
                    args.get(i).unwrap_or_else(|| usage()).clone(),
                ));
            }
            other => {
                if cmd.is_some() {
                    usage();
                }
                cmd = Some(other.to_string());
            }
        }
        i += 1;
    }
    let cmd = cmd.unwrap_or_else(|| usage());
    if let Some(dir) = &out {
        std::fs::create_dir_all(dir).expect("create output dir");
    }

    let result: Result<i32, String> = (|| {
        match cmd.as_str() {
            "manifest" => {
                let value = serde_json::json!({
                    "meta": run_metadata(),
                    "manifest": cells::manifest_receipt(),
                });
                if let Some(dir) = out {
                    write_json(&dir.join("manifest.json"), &value).map_err(|e| e.to_string())?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                Ok(0)
            }
            "validate" => {
                let rows = validate_control::validate_all()?;
                let all_pass = rows.iter().all(|r| r.passed);
                let value = serde_json::json!({
                    "meta": run_metadata(),
                    "all_pass": all_pass,
                    "cells": rows,
                });
                if let Some(dir) = out {
                    write_json(&dir.join("validation.json"), &value).map_err(|e| e.to_string())?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                println!("L0 comparator validation: {}", if all_pass { "PASS" } else { "FAIL" });
                Ok(if all_pass { 0 } else { 1 })
            }
            "tlane" => {
                let rows = tlane::run()?;
                let value = serde_json::json!({
                    "meta": run_metadata(),
                    "policy": {
                        "warmup_rounds": tlane::WARMUP_ROUNDS,
                        "measured_rounds": tlane::MEASURED_ROUNDS,
                        "process_model": "single process, cells in manifest order, arms interleaved per round",
                        "timed_window": "complete arm call incl. pre-state clone exclusion and result drop exclusion",
                        "mechanisms": tlane::arm_mechanism_ids(),
                    },
                    "cells": rows,
                });
                if let Some(dir) = out {
                    write_json(&dir.join("tlane.json"), &value).map_err(|e| e.to_string())?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                Ok(0)
            }
            "alane" => {
                let rows = alane::run()?;
                let value = serde_json::json!({
                    "meta": run_metadata(),
                    "policy": {
                        "lane": "A-LANE, one attributed pass per cell/arm, untimed",
                        "surfaces": ["ATTRIBUTION-SCHEMA-v2 WorkCounters", "HORSE-A-STRUCTURAL-COUNTERS-v1 (arms A/B)"],
                    },
                    "cells": rows,
                    "compact": alane::compact_table(&rows),
                });
                if let Some(dir) = out {
                    write_json(&dir.join("alane.json"), &value).map_err(|e| e.to_string())?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                Ok(0)
            }
            "effects" => cmd_effects(out),
            "decompose" => {
                let rows = decompose::run()?;
                let value = serde_json::json!({
                    "meta": run_metadata(),
                    "policy": {
                        "probes": ["P1_FULL_BUILD_ONLY", "P0_H0_FULL_PARSE_ONLY"],
                        "warmup_rounds": tlane::WARMUP_ROUNDS,
                        "measured_rounds": tlane::MEASURED_ROUNDS,
                    },
                    "cells": rows,
                });
                if let Some(dir) = out {
                    write_json(&dir.join("decompose.json"), &value).map_err(|e| e.to_string())?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                }
                Ok(0)
            }
            "all" => {
                // 0C first: no treatment data unless every gate passes.
                let rows = validate_control::validate_all()?;
                let all_pass = rows.iter().all(|r| r.passed);
                write_json(
                    &out
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("validation.json"),
                    &serde_json::json!({
                        "meta": run_metadata(),
                        "all_pass": all_pass,
                        "cells": rows,
                    }),
                )
                .map_err(|e| e.to_string())?;
                if !all_pass {
                    eprintln!("L0 comparator validation FAILED; treatment collection refused.");
                    return Ok(1);
                }
                println!("L0 comparator validation: PASS");
                for (cmd, fname) in [
                    ("tlane", "tlane.json"),
                    ("alane", "alane.json"),
                    ("effects", "effects.json"),
                ] {
                    // rerun through the same paths as the standalone
                    // subcommands (single-process, sequential)
                    match cmd {
                        "tlane" => {
                            let rows = tlane::run()?;
                            write_json(
                                &out.clone().unwrap().join(fname),
                                &serde_json::json!({
                                    "meta": run_metadata(),
                                    "policy": {
                                        "warmup_rounds": tlane::WARMUP_ROUNDS,
                                        "measured_rounds": tlane::MEASURED_ROUNDS,
                                        "process_model": "single process, cells in manifest order, arms interleaved per round",
                                        "mechanisms": tlane::arm_mechanism_ids(),
                                    },
                                    "cells": rows,
                                }),
                            )
                            .map_err(|e| e.to_string())?;
                        }
                        "alane" => {
                            let rows = alane::run()?;
                            write_json(
                                &out.clone().unwrap().join(fname),
                                &serde_json::json!({
                                    "meta": run_metadata(),
                                    "cells": rows,
                                    "compact": alane::compact_table(&rows),
                                }),
                            )
                            .map_err(|e| e.to_string())?;
                        }
                        "effects" => {
                            cmd_effects(out.clone())?;
                        }
                        _ => unreachable!(),
                    }
                }
                Ok(0)
            }
            _ => usage(),
        }
    })();

    match result {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}
