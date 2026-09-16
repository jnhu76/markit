//! markit-mdbench-oracle — the correctness hook only.
//!
//! R1 deliberately does NOT implement H0 semantic authority and does not
//! define the future normalized Markdown node structure. The R1 hook
//! compares a deterministic scalar/checksum produced by
//! [`markit_mdbench_common::Completed`]. The real BENCH-GRAMMAR-v1
//! normalization oracle (`normalize(update result) == normalize(H0 clean
//! parse(post-source))`) arrives with R4 and will implement the same
//! hook trait.
//!
//! Oracle execution is strictly OUTSIDE `T_native` — the runner invokes
//! hooks only after all timing has stopped.

use markit_mdbench_common::Completed;
use markit_mdbench_common::CorrectnessStatus;

/// Generic correctness hook over a mechanism's completed result.
pub trait CorrectnessHook<S> {
    fn verify(&self, completed: &Completed<S>) -> CorrectnessStatus;
}

/// R1 hook: compare the deterministic scalar checksum recorded in
/// [`Completed::result_checksum`] against the expected value derived by
/// the same documented formula.
pub struct ScalarChecksumHook {
    expected: u64,
}

impl ScalarChecksumHook {
    pub fn new(expected: u64) -> Self {
        Self { expected }
    }
}

impl<S> CorrectnessHook<S> for ScalarChecksumHook {
    fn verify(&self, completed: &Completed<S>) -> CorrectnessStatus {
        if completed.result_checksum == self.expected {
            CorrectnessStatus::Pass
        } else {
            CorrectnessStatus::WrongResult
        }
    }
}

/// Hook that records `NOT_CHECKED`: used when a lane intentionally skips
/// correctness (never silently passes).
pub struct NotCheckedHook;

impl<S> CorrectnessHook<S> for NotCheckedHook {
    fn verify(&self, _completed: &Completed<S>) -> CorrectnessStatus {
        CorrectnessStatus::NotChecked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markit_mdbench_common::Completed;

    #[test]
    fn scalar_hook_reports_pass_and_wrong_result() {
        let done = Completed {
            state: (),
            result_checksum: 42,
        };
        assert_eq!(
            ScalarChecksumHook::new(42).verify(&done),
            CorrectnessStatus::Pass
        );
        assert_eq!(
            ScalarChecksumHook::new(43).verify(&done),
            CorrectnessStatus::WrongResult
        );
        assert_eq!(NotCheckedHook.verify(&done), CorrectnessStatus::NotChecked);
    }
}
