//! `mdbench-h4diag-phases` — built with `--features phases` (U_PHASE lane).
//!
//! Contains the mutually-exclusive phase timers and nothing else: no
//! diagnostic counters, no allocator wrapper.

fn main() -> std::process::ExitCode {
    markit_mdbench_h4diag::cli::main()
}
