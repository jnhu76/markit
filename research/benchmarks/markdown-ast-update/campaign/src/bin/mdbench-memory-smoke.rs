//! NON_RESEARCH memory-lane smoke binary (#76 Gate B).
//!
//! The DEDICATED instrumented binary: it installs the
//! [`CountingAllocator`] as the process allocator, so the M-LANE windows
//! are backed by real allocation counting. The timing/attribution
//! campaign binary (`mdbench-campaign`) never installs it — timing
//! sessions always run under the plain system allocator.
//!
//! Output is NON_RESEARCH instrument validation only
//! (`CAMPAIGN_MEMORY_SMOKE/NON_RESEARCH_RESULT`): one predetermined
//! micro case × six horses, proving the campaign wiring can measure
//! incremental latency, initial build, allocation count/bytes, and
//! retained memory. It collects NO comparative latency evidence and
//! writes NO campaign raw rows. Exit code 0 = MEMORY_SMOKE_PASS.

use std::process::ExitCode;

#[global_allocator]
static COUNTING: markit_mdbench_instrumentation::CountingAllocator =
    markit_mdbench_instrumentation::CountingAllocator;

fn main() -> ExitCode {
    match markit_mdbench_campaign::memory_smoke::run_and_report() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("MEMORY_SMOKE_FAIL (NON_RESEARCH): {error}");
            ExitCode::FAILURE
        }
    }
}
