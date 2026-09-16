//! The frozen mechanism phase boundary (R1 harness contract §5).
//!
//! Authority boundary (must not drift):
//!
//! ```text
//! prepare_update != update/native != complete
//! ```
//!
//! `complete()` is an explicit full-work boundary: a mechanism cannot
//! return an unconsumed iterator / lazy parser state and have the runner
//! force the real work later outside `T_native`, because the runner calls
//! `complete()` inside `T_native` and `black_box`es the completed state.
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
/// state plus a deterministic scalar checksum of the produced result.
///
/// R1's oracle hook compares checksums; the real normalized-result
/// oracle arrives with H0 (R4). The checksum is a fact for correctness
/// checking, never a performance claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed<State> {
    pub state: State,
    pub result_checksum: u64,
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
    /// Deliberately has no `MechanismContext`: the R1 interface freezes
    /// counter reporting on `full_parse`/`update` only. If a later horse
    /// needs prepare-phase attribution, that is a protocol question for
    /// its stage — not something to add silently here.
    fn prepare_update(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
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

    /// Explicit completion boundary: consumes all pending work and
    /// produces the new retained state plus the result checksum. The
    /// runner calls this inside `T_native`.
    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus>;
}
