//! U_PHASE — mutually exclusive phase timers (Issue #50 §5, §6).
//!
//! Only compiled under the `phases` feature. The phase scopes are
//! **disjoint by construction**: each scope opens and closes exactly once
//! per update, and no scope is entered while another is open, so
//! `sum(disjoint phases)` is a legitimate arithmetic quantity. The one
//! deliberately *inclusive* measurement — the convergence hook, which
//! lives inside the forward parser — is recorded in a separate slot and is
//! never added to the disjoint sum.
//!
//! Clock: `std::time::Instant` (Linux `CLOCK_MONOTONIC` via vDSO). Each
//! scope reads the clock exactly twice. The frozen runner's outer
//! `T_prepare`/`T_native` timers are untouched and remain what U_PHASE
//! reports; these inner times are a decomposition of that outer window,
//! and the perturbation they cause is measured (`U_PHASE / U_PLAIN`)
//! before any share is transferred to U_PLAIN.

#[cfg(feature = "phases")]
use std::cell::RefCell;
#[cfg(feature = "phases")]
use std::time::Instant;

/// The frozen phase map (see `PHASE-MAP.md`). Order is the report order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// P1 — restart selection, margin back-up, damage scan
    /// (`prepare_update` body). Timed in the runner's `T_prepare`.
    P1PrepareDamageRestart,
    /// P2 — forward parse from the restart checkpoint, INCLUDING every
    /// convergence consult the hook performs.
    P2ForwardParseAndConvergence,
    /// P3 — retained-prefix pair assembly (Arc clone + key clone + push).
    P3PrefixPairAssembly,
    /// P4 — definition collection, table build, assembled-vs-retained
    /// comparison.
    P4DefinitionCollectTableCompare,
    /// P5 — fresh inline materialization + retained-suffix pair assembly.
    P5FreshMaterializationAndSuffixAssembly,
    /// P6 — pairs -> slots/checkpoints construction.
    P6PairsToSlotsCheckpoints,
    /// P7 — native sealing plus the explicit retirement of the consumed
    /// old state.
    P7SealAndRetirement,
    /// INCLUSIVE sub-measure of P2: the convergence hook only. Never added
    /// to the disjoint sum.
    HookInclusiveSubMeasure,
}

impl Phase {
    pub const ALL: [Phase; 7] = [
        Phase::P1PrepareDamageRestart,
        Phase::P2ForwardParseAndConvergence,
        Phase::P3PrefixPairAssembly,
        Phase::P4DefinitionCollectTableCompare,
        Phase::P5FreshMaterializationAndSuffixAssembly,
        Phase::P6PairsToSlotsCheckpoints,
        Phase::P7SealAndRetirement,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Phase::P1PrepareDamageRestart => "P1_prepare_damage_restart",
            Phase::P2ForwardParseAndConvergence => "P2_forward_parse_and_convergence",
            Phase::P3PrefixPairAssembly => "P3_prefix_pair_assembly",
            Phase::P4DefinitionCollectTableCompare => "P4_definition_collect_table_compare",
            Phase::P5FreshMaterializationAndSuffixAssembly => {
                "P5_fresh_materialization_and_suffix_assembly"
            }
            Phase::P6PairsToSlotsCheckpoints => "P6_pairs_to_slots_checkpoints",
            Phase::P7SealAndRetirement => "P7_seal_and_retirement",
            Phase::HookInclusiveSubMeasure => "hook_inclusive_sub_measure",
        }
    }

    fn index(self) -> usize {
        match self {
            Phase::P1PrepareDamageRestart => 0,
            Phase::P2ForwardParseAndConvergence => 1,
            Phase::P3PrefixPairAssembly => 2,
            Phase::P4DefinitionCollectTableCompare => 3,
            Phase::P5FreshMaterializationAndSuffixAssembly => 4,
            Phase::P6PairsToSlotsCheckpoints => 5,
            Phase::P7SealAndRetirement => 6,
            Phase::HookInclusiveSubMeasure => 7,
        }
    }
}

/// Per-observation phase times in nanoseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhaseTimes {
    pub slots: [u64; 8],
    /// Clock reads performed by phase instrumentation in this
    /// observation (measurement-overhead accounting, Issue #50 §5).
    pub clock_reads: u64,
}

impl PhaseTimes {
    pub fn get(&self, phase: Phase) -> u64 {
        self.slots[phase.index()]
    }

    /// Sum of the seven DISJOINT phases. The inclusive hook sub-measure is
    /// excluded by construction.
    pub fn disjoint_sum(&self) -> u64 {
        Phase::ALL.iter().map(|p| self.get(*p)).sum()
    }

    /// The inclusive hook sub-measure (already contained in P2).
    pub fn hook(&self) -> u64 {
        self.get(Phase::HookInclusiveSubMeasure)
    }
}

#[cfg(feature = "phases")]
thread_local! {
    static ACTIVE: RefCell<PhaseTimes> = RefCell::new(PhaseTimes::default());
    static ARMED: RefCell<bool> = const { RefCell::new(false) };
}

/// Begin a phase-time window (arming the recorder).
#[cfg(feature = "phases")]
pub fn arm() {
    ACTIVE.with(|t| *t.borrow_mut() = PhaseTimes::default());
    ARMED.with(|a| *a.borrow_mut() = true);
}

/// End a phase-time window and return the recorded times.
#[cfg(feature = "phases")]
pub fn disarm() -> PhaseTimes {
    ARMED.with(|a| *a.borrow_mut() = false);
    ACTIVE.with(|t| *t.borrow())
}

/// Record one phase duration.
#[cfg(feature = "phases")]
#[inline]
pub fn record(phase: Phase, nanos: u64) {
    let armed = ARMED.with(|a| *a.borrow());
    if !armed {
        return;
    }
    ACTIVE.with(|t| {
        let mut t = t.borrow_mut();
        t.slots[phase.index()] = t.slots[phase.index()].saturating_add(nanos);
        t.clock_reads += 2;
    });
}

/// RAII phase scope. Compiles to nothing without the `phases` feature.
#[cfg(feature = "phases")]
pub struct PhaseGuard {
    phase: Phase,
    start: Instant,
}

#[cfg(feature = "phases")]
impl PhaseGuard {
    #[inline]
    pub fn start(phase: Phase) -> Self {
        Self {
            phase,
            start: Instant::now(),
        }
    }
}

#[cfg(feature = "phases")]
impl Drop for PhaseGuard {
    #[inline]
    fn drop(&mut self) {
        let nanos = u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        record(self.phase, nanos);
    }
}

/// Open a disjoint phase scope (no-op without the `phases` feature).
#[macro_export]
macro_rules! phase {
    ($phase:ident, $body:block) => {{
        #[cfg(feature = "phases")]
        let __phase_guard = $crate::phases::PhaseGuard::start($crate::phases::Phase::$phase);
        let __phase_result = $body;
        #[cfg(feature = "phases")]
        drop(__phase_guard);
        __phase_result
    }};
}

/// Open an INCLUSIVE sub-measure scope (never added to the disjoint sum).
#[macro_export]
macro_rules! phase_inclusive {
    ($phase:ident, $body:block) => {{
        #[cfg(feature = "phases")]
        let __phase_guard = $crate::phases::PhaseGuard::start($crate::phases::Phase::$phase);
        let __phase_result = $body;
        #[cfg(feature = "phases")]
        drop(__phase_guard);
        __phase_result
    }};
}
