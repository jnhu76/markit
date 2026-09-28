//! mdbench-horse-a-structural — the minimal frozen #60 structural
//! producer CLI (readiness record §8; task §5).
//!
//! ONE process executes exactly ONE cell × ONE repetition × ONE raw
//! observation. The 3×3 schedule is launched externally as nine
//! independent OS processes — this binary contains no cell or
//! repetition loops.
//!
//! Usage:
//!
//! ```text
//! mdbench-horse-a-structural --cell <H4N-128KiB|H4N-1MiB|H4N-16MiB> \
//!     --repetition <1|2|3> --output <fresh raw destination>
//!
//! mdbench-horse-a-structural --validate-only [--cell <id>] [--output <path>]
//! mdbench-horse-a-structural --print-schema
//! mdbench-horse-a-structural --print-provenance
//! ```
//!
//! `--repetition` is the 1-based external schedule index; the raw row
//! records the schema repetition (`value − 1`, readiness record §9:
//! `0 | 1 | 2`).
//!
//! Exit codes: 0 = row PASS (continue); 3 = valid structural FAIL
//! (continue the remaining frozen observations); 4 = INVALID / producer
//! failure (stop); 5 = correctness or cell-identity failure (stop);
//! 2 = usage error (no treatment performed).
//!
//! There are NO post-freeze knobs: no threshold, restart-distance,
//! candidate-limit, full-build-override, ignore-unknown or
//! threshold-scale flags exist, and none may be added.

use std::path::PathBuf;
use std::process::ExitCode;

use markit_mdbench_horse_a::producer::contract::FrozenCell;
use markit_mdbench_horse_a::producer::run::{
    print_provenance_json, run_observation, validate_only,
};
use markit_mdbench_horse_a::producer::schema::schema_field_report;

const USAGE: &str = "\
mdbench-horse-a-structural — frozen #60 structural-locality producer

TREATMENT (one process = one cell x one repetition x one raw row):
  --cell <H4N-128KiB|H4N-1MiB|H4N-16MiB>
  --repetition <1|2|3>     1-based schedule index; recorded as schema repetition (value - 1)
  --output <path>          fresh raw destination; an existing path is refused (fail closed)

NON-TREATMENT:
  --validate-only [--cell <id>] [--output <path>]
                           verify identities/threshold table/schema/provenance; never records
  --print-schema           print the HORSE-A-STRUCTURAL-RAW-v1 field report
  --print-provenance       print the captured static provenance (JSON)

Exit codes: 0 PASS | 3 structural FAIL (continue) | 4 INVALID/producer (stop)
            | 5 correctness/identity failure (stop) | 2 usage error
No threshold or protocol flags exist. STRUCTURAL_COLLECTION is the only mode.";

enum Mode {
    Treatment {
        cell: &'static FrozenCell,
        repetition: u64,
        output: PathBuf,
    },
    ValidateOnly {
        cell: Option<&'static FrozenCell>,
        output: Option<PathBuf>,
    },
    PrintSchema,
    PrintProvenance,
}

fn usage_error(detail: &str) -> ! {
    eprintln!("mdbench-horse-a-structural: {detail}\n\n{USAGE}");
    std::process::exit(2);
}

fn parse_cell(value: &str) -> &'static FrozenCell {
    FrozenCell::by_id(value).unwrap_or_else(|| usage_error(&format!("unknown --cell {value:?}")))
}

fn parse_repetition(value: &str) -> u64 {
    let n: u64 = value
        .parse()
        .unwrap_or_else(|_| usage_error(&format!("--repetition must be 1, 2 or 3, got {value:?}")));
    if !(1..=3).contains(&n) {
        usage_error(&format!("--repetition must be 1, 2 or 3, got {n}"));
    }
    n - 1
}

fn next_value(args: &mut Vec<String>, flag: &str) -> String {
    if args.is_empty() {
        usage_error(&format!("{flag} requires a value"));
    }
    args.remove(0)
}

fn expect_flag(args: &mut Vec<String>, flag: &str) {
    if args.is_empty() || args[0] != flag {
        let got = args.first().map(|s| s.as_str()).unwrap_or("<end>");
        usage_error(&format!("expected {flag}, got {got:?}"));
    }
    args.remove(0);
}

fn parse_args() -> Mode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage_error("no arguments given");
    }
    let first = args.remove(0);
    match first.as_str() {
        "--print-schema" => {
            if !args.is_empty() {
                usage_error("--print-schema takes no further arguments");
            }
            Mode::PrintSchema
        }
        "--print-provenance" => {
            if !args.is_empty() {
                usage_error("--print-provenance takes no further arguments");
            }
            Mode::PrintProvenance
        }
        "--validate-only" => {
            let mut cell: Option<&'static FrozenCell> = None;
            let mut output: Option<PathBuf> = None;
            while !args.is_empty() {
                let flag = args.remove(0);
                match flag.as_str() {
                    "--cell" => {
                        if cell.is_some() {
                            usage_error("--cell given twice");
                        }
                        let value = next_value(&mut args, "--cell");
                        cell = Some(parse_cell(&value));
                    }
                    "--output" => {
                        if output.is_some() {
                            usage_error("--output given twice");
                        }
                        let value = next_value(&mut args, "--output");
                        output = Some(PathBuf::from(value));
                    }
                    other => usage_error(&format!("unknown flag for --validate-only: {other:?}")),
                }
            }
            Mode::ValidateOnly { cell, output }
        }
        "--cell" => {
            let cell_value = next_value(&mut args, "--cell");
            let cell = parse_cell(&cell_value);
            expect_flag(&mut args, "--repetition");
            let rep_value = next_value(&mut args, "--repetition");
            let repetition = parse_repetition(&rep_value);
            expect_flag(&mut args, "--output");
            let output = PathBuf::from(next_value(&mut args, "--output"));
            if !args.is_empty() {
                usage_error("unexpected extra arguments in treatment mode");
            }
            Mode::Treatment {
                cell,
                repetition,
                output,
            }
        }
        "--help" | "-h" => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        other => usage_error(&format!("unknown first argument {other:?}")),
    }
}

fn main() -> ExitCode {
    match parse_args() {
        Mode::PrintSchema => {
            print!("{}", schema_field_report());
            ExitCode::SUCCESS
        }
        Mode::PrintProvenance => match print_provenance_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(detail) => {
                eprintln!("mdbench-horse-a-structural: {detail}");
                ExitCode::from(4)
            }
        },
        Mode::ValidateOnly { cell, output } => match validate_only(cell, output.as_deref()) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(detail) => {
                eprintln!("mdbench-horse-a-structural: validate-only FAILED: {detail}");
                ExitCode::from(4)
            }
        },
        Mode::Treatment {
            cell,
            repetition,
            output,
        } => {
            let command = std::env::args().collect::<Vec<_>>().join(" ");
            let outcome = run_observation(cell, repetition, &command, &output);
            println!(
                "mdbench-horse-a-structural: {} rep{} -> {} ({}) [exit {}]",
                cell.cell_id,
                repetition + 1,
                outcome.status_line(),
                output.display(),
                outcome.exit_code()
            );
            ExitCode::from(outcome.exit_code().try_into().unwrap_or(2))
        }
    }
}
