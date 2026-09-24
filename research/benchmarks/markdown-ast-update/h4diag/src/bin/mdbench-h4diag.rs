//! `mdbench-h4diag` — default build: no phase timers, no counters, no
//! allocator wrapper. This is the ablation / A0-copy binary.

fn main() -> std::process::ExitCode {
    markit_mdbench_h4diag::cli::main()
}
