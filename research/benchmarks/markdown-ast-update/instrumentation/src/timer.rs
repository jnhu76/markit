//! Phase timing primitives.
//!
//! The frozen measurement model (R0 §7, R1 task §6):
//!
//! ```text
//! T_prepare = mechanism-specific edit metadata preparation
//! T_native  = mechanism-required update work up to a completed state
//! T_total   = T_prepare + T_native   (arithmetic, never a third interval)
//! ```
//!
//! For FULL_PARSE, `T_prepare` is `NotApplicable` and
//! `T_total == T_native` by documented arithmetic identity (the
//! NotApplicable phase contributes additive zero).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use markit_mdbench_common::Observed;

/// One timed phase: `start` reads the clock once, [`PhaseGuard::stop`]
/// reads it once more. Nothing else in the runner may read the clock
/// between the two reads.
pub struct PhaseGuard<'a, C: Clock> {
    clock: &'a C,
    start_nanos: u64,
}

impl<'a, C: Clock> PhaseGuard<'a, C> {
    pub fn start(clock: &'a C) -> Self {
        let start_nanos = clock.now_nanos();
        Self { clock, start_nanos }
    }

    /// Stop the phase: exactly one further clock read; returns elapsed
    /// nanoseconds (saturating at zero against a clock that moves
    /// backwards, which the deterministic clocks never do).
    pub fn stop(self) -> u64 {
        self.clock.now_nanos().saturating_sub(self.start_nanos)
    }
}

/// Timing arithmetic failure (u64 nanosecond overflow). Maps to
/// `ExecutionStatus::InstrumentationUnavailable` — the run is not a
/// benchmark data point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    Overflow,
}

impl core::fmt::Display for TimingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TimingError::Overflow => f.write_str("nanosecond arithmetic overflow in T_total"),
        }
    }
}

impl std::error::Error for TimingError {}

/// A timing record for one case run.
///
/// `T_total` is always the arithmetic sum: `prepare + native` (update) or
/// `native` (full parse, with prepare `NotApplicable`). It is never an
/// independently measured enclosing wall-clock interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimingRecord {
    pub prepare_ns: Observed<u64>,
    pub native_ns: Observed<u64>,
    pub total_ns: Observed<u64>,
}

impl TimingRecord {
    /// UPDATE record: both phases measured, `T_total = T_prepare +
    /// T_native` with checked arithmetic.
    pub fn update(prepare_ns: u64, native_ns: u64) -> Result<Self, TimingError> {
        let total = prepare_ns
            .checked_add(native_ns)
            .ok_or(TimingError::Overflow)?;
        Ok(Self {
            prepare_ns: Observed::Known(prepare_ns),
            native_ns: Observed::Known(native_ns),
            total_ns: Observed::Known(total),
        })
    }

    /// FULL_PARSE record: `T_prepare` is `NotApplicable` (no preparation
    /// phase exists), `T_total = T_native` by documented identity.
    pub fn full_parse(native_ns: u64) -> Self {
        Self {
            prepare_ns: Observed::NotApplicable,
            native_ns: Observed::Known(native_ns),
            total_ns: Observed::Known(native_ns),
        }
    }

    /// Record for a run that produced no measurable timing (failed
    /// execution or timing-arithmetic overflow). The lane is preserved;
    /// the values are honestly `Unknown`.
    pub fn unknown() -> Self {
        Self {
            prepare_ns: Observed::Unknown,
            native_ns: Observed::Unknown,
            total_ns: Observed::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_is_arithmetic_sum() {
        let rec = TimingRecord::update(11, 23).expect("no overflow");
        assert_eq!(rec.prepare_ns, Observed::Known(11));
        assert_eq!(rec.native_ns, Observed::Known(23));
        assert_eq!(rec.total_ns, Observed::Known(34));
    }

    #[test]
    fn overflow_is_checked_not_silent() {
        assert_eq!(
            TimingRecord::update(u64::MAX, 1),
            Err(TimingError::Overflow)
        );
        assert_eq!(
            TimingRecord::update(1, u64::MAX),
            Err(TimingError::Overflow)
        );
    }

    #[test]
    fn full_parse_has_not_applicable_prepare() {
        let rec = TimingRecord::full_parse(23);
        assert_eq!(rec.prepare_ns, Observed::NotApplicable);
        assert_eq!(rec.native_ns, Observed::Known(23));
        assert_eq!(rec.total_ns, Observed::Known(23));
    }

    #[test]
    fn serde_is_strict_and_round_trips() {
        let rec = TimingRecord::update(11, 23).unwrap();
        let json = serde_json::to_value(rec).unwrap();
        assert_eq!(json["prepare_ns"], 11);
        let back: TimingRecord = serde_json::from_value(json).unwrap();
        assert_eq!(back, rec);
        let mut bad = serde_json::to_value(TimingRecord::full_parse(1)).unwrap();
        bad["blocks_reparsed"] = serde_json::json!(0);
        assert!(serde_json::from_value::<TimingRecord>(bad).is_err());
    }
}
