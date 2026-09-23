//! `mdbench-h4diag-counters` — built with `--features counters`
//! (W_COUNTERS lane).
//!
//! Contains the direct representation-operation counters and nothing
//! else: no phase timers, no allocator wrapper, and no headline timing
//! (this binary runs the attribution lane only).

fn main() -> std::process::ExitCode {
    markit_mdbench_h4diag::cli::main()
}
