//! The frozen mechanism phase boundary (R1 harness contract §4, as
//! corrected by MEASUREMENT-CORRECTIVE-1).
//!
//! Authority boundary (must not drift):
//!
//! ```text
//! prepare_update != update/native != complete
//! ```
//!
//! `complete()` is the explicit completion AUTHORITY boundary: the runner
//! requires the mechanism to hand over a fully consumed, fully EAGER
//! native [`Completed`] state inside `T_native` and `black_box`es it.
//! This is an authority requirement, NOT a mechanical proof that lazily
//! deferred work (iterators, closures, `OnceCell`s, interior mutability,
//! lazy indexes/trees) has been forced — `black_box` cannot look through
//! such structures. The R4/R5 eager-completion gates prove that the
//! completed state is complete without reservation.
//!
//! MEASUREMENT-CORRECTIVE-1 timing-boundary rule (frozen):
//!
//! > Work required for the mechanism's usable native state stays inside
//! > timing. Work required only to prove equality to the experiment's
//! > normalized oracle stays outside timing.
//!
//! `complete()` therefore SEALS the native state only. The normalized
//! projection, its validation, and the deterministic result checksum are
//! experiment EXPORT work: the runner derives them strictly AFTER every
//! timer has stopped, through the state's frozen pure
//! [`ResultChecksum`] export (for the real horses: `NormalizeV1` +
//! `normalized_checksum`). No parsing may be deferred past the timer
//! stop, and no mechanism-required state/index construction may move
//! outside timing.
//!
//! Every mechanism phase that can do mechanism-owned work — including
//! `prepare_update` — receives a [`MechanismContext`], so attribution
//! can never miss preparation work that is timed in `T_prepare`
//! (R1-CORRECTIVE-1, MAJOR-1).
//!
//! `MechanismContext` exposes work-counter hooks only. It must never
//! expose a timer or clock — timer placement belongs to
//! `runner`/`instrumentation`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::edit::CanonicalEdit;
use crate::source::Source;
use crate::status::FailureStatus;
use crate::work::WorkSink;

/// Identifier of a mechanism (horse). `"__r1_null__"` is reserved for the
/// R1 null mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct MechanismId(pub String);

impl MechanismId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Work-counter hooks handed to a mechanism. Generic over the sink so the
/// timing lane can pass a zero-cost discarding sink while the attribution
/// lane records.
pub struct MechanismContext<'a, W: WorkSink> {
    pub sink: &'a mut W,
}

impl<'a, W: WorkSink> MechanismContext<'a, W> {
    pub fn new(sink: &'a mut W) -> Self {
        Self { sink }
    }
}

/// The completed result of a mechanism run: the mechanism's new retained
/// native state, sealed at the explicit completion boundary.
///
/// MEASUREMENT-CORRECTIVE-1: `Completed` carries NO result checksum.
/// Deriving a checksum means serializing + hashing the normalized result
/// — verification/export work, not native mechanism completion — so the
/// runner computes it strictly AFTER every timer has stopped, via the
/// state's frozen [`ResultChecksum`] export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed<State> {
    pub state: State,
}

/// Post-timer experiment export (MEASUREMENT-CORRECTIVE-1): the frozen,
/// PURE derivation of the deterministic result checksum from a sealed
/// native state.
///
/// Contract:
///
/// - the runner calls this strictly AFTER every timer has stopped; it is
///   experiment verification/export work and never mechanism timing;
/// - implementations are pure functions of already-complete state: no
///   source input, no parsing, no reparsing, no dependency repair, no
///   index update, no restart/reuse decision, no interior mutability;
/// - the derivation is deterministic and stable across runs.
///
/// The five real horses implement this over their frozen
/// `NormalizeV1` projection + the oracle's `normalized_checksum`. The R1
/// null mechanism implements it over its own documented scalar mix.
pub trait ResultChecksum {
    fn result_checksum(&self) -> u64;
}

/// Phase boundary every mechanism (future horses and the R1 null
/// mechanism alike) must implement.
///
/// Type parameters on the phase methods let the runner choose the work
/// sink per measurement lane without the mechanism knowing about lanes.
pub trait Mechanism {
    /// Retained state carried between updates.
    type State;
    /// Mechanism-specific edit-metadata prepared during `T_prepare`.
    type Prepared;
    /// Mechanism work handed to `complete()`; must be fully consumed
    /// there.
    type Pending;

    fn id(&self) -> MechanismId;

    /// Clean full parse of `source` (H0-style control operation).
    /// Runs inside `T_native`; `T_prepare` is `NotApplicable`.
    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus>;

    /// Mechanism-specific edit-coordinate / edit-metadata preparation.
    /// Runs inside `T_prepare` and only there.
    ///
    /// Receives a [`MechanismContext`] like every other working phase:
    /// work performed here is timed in `T_prepare`, so it must be
    /// attributable in the A-LANE too (R1-CORRECTIVE-1, MAJOR-1) — a
    /// mechanism can never report work in one lane that vanishes in the
    /// other.
    fn prepare_update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus>;

    /// Mechanism-required state maintenance / damage detection / restart
    /// / reuse / reparse / reconstruction / index work. Runs inside
    /// `T_native` together with `complete()`.
    fn update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus>;

    /// Explicit completion boundary: consumes all pending work and seals
    /// the new retained native state. Native-sealing ONLY — no export,
    /// no checksum, no validation (MEASUREMENT-CORRECTIVE-1). The runner
    /// calls this inside `T_native` and derives the result checksum
    /// strictly after every timer has stopped.
    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus>;
}
