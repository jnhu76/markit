//! Clock abstraction: real clock for the harness, manual clock for
//! deterministic tests.
//!
//! No mechanism ever sees this type. The runner owns the only clock
//! reference and reads it exclusively at phase boundaries, which is what
//! makes the deterministic boundary tests in §7 of the R1 task possible.

use std::cell::Cell;
use std::cell::RefCell;
use std::time::Instant;

/// Monotonic nanosecond source. One method, one caller: the runner.
pub trait Clock {
    fn now_nanos(&self) -> u64;
}

/// Real harness clock: nanoseconds since this clock was constructed.
pub struct InstantClock {
    anchor: Instant,
}

impl InstantClock {
    pub fn new() -> Self {
        Self {
            anchor: Instant::now(),
        }
    }
}

impl Default for InstantClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for InstantClock {
    fn now_nanos(&self) -> u64 {
        // Saturate instead of wrapping: an Instant::elapsed beyond
        // u64::MAX nanos (~584 years) is not a measurement, it is a
        // broken environment.
        u64::try_from(self.anchor.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// Controllable clock for deterministic tests ONLY.
///
/// Records every [`Clock::now_nanos`] read so tests can assert the exact
/// read sequence of the runner (proving, e.g., that `complete()` ran
/// inside the `T_native` window and that oracle/serialization performed
/// no timed reads).
pub struct ManualClock {
    current_nanos: Cell<u64>,
    reads: RefCell<Vec<u64>>,
}

impl ManualClock {
    pub fn new() -> Self {
        Self {
            current_nanos: Cell::new(0),
            reads: RefCell::new(Vec::new()),
        }
    }

    pub fn set_nanos(&self, nanos: u64) {
        self.current_nanos.set(nanos);
    }

    pub fn advance_nanos(&self, delta: u64) {
        let next = self.current_nanos.get().saturating_add(delta);
        self.current_nanos.set(next);
    }

    /// Current value without recording a read (test introspection only).
    pub fn peek_nanos(&self) -> u64 {
        self.current_nanos.get()
    }

    /// All reads performed through the [`Clock`] trait, in order.
    pub fn recorded_reads(&self) -> Vec<u64> {
        self.reads.borrow().clone()
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn now_nanos(&self) -> u64 {
        let now = self.current_nanos.get();
        self.reads.borrow_mut().push(now);
        now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_records_read_sequence() {
        let clock = ManualClock::new();
        assert_eq!(clock.now_nanos(), 0);
        clock.set_nanos(11);
        assert_eq!(clock.now_nanos(), 11);
        clock.advance_nanos(23);
        assert_eq!(clock.now_nanos(), 34);
        assert_eq!(clock.recorded_reads(), vec![0, 11, 34]);
        assert_eq!(clock.peek_nanos(), 34);
    }

    #[test]
    fn real_clock_is_monotonic() {
        let clock = InstantClock::new();
        let a = clock.now_nanos();
        let b = clock.now_nanos();
        assert!(b >= a);
    }
}
