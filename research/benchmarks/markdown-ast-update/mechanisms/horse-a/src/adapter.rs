//! Common-`Mechanism` adapter for the frozen Horse-A pipeline (#76 Gate B).
//!
//! This module adds NO mechanism semantics. It presents the existing
//! frozen pipeline through the shared campaign phase boundary so the
//! campaign executor can dispatch Horse-A exactly like H0–H4:
//!
//! ```text
//! Mechanism::full_parse     = full_build + seal
//! Mechanism::prepare_update = update::stage()   (fallible pre-frontier
//!                             staging: association validation, damage
//!                             locate, restart selection, forward parse,
//!                             candidate walk, replacement facts, commit
//!                             workspace reservation) -> T_prepare
//! Mechanism::update         = UpdateStaging::prepare (infallible
//!                             ownership move) + PreparedCommit::commit
//!                             (frontier crossing: splice, retire,
//!                             state installation) -> T_native
//! Mechanism::complete       = native seal only (MEASUREMENT-CORRECTIVE-1)
//! ```
//!
//! The structural recording lane (`update_with_structural` /
//! `HORSE-A-STRUCTURAL-COUNTERS-v1`) is NOT part of this adapter: the
//! campaign's A-LANE carries the common WorkCounters schema, and this
//! adapter reports only the facts Horse-A produces through the ordinary
//! shared-parser seam (`record_source_inspection` via `cx.sink`) plus the
//! exact zero facts its slots demand. Horse-A's own frozen attribution
//! record stays in its producer lane; no counter is re-derived or
//! invented here.

use markit_mdbench_common::{
    CanonicalEdit, Completed, FailureStatus, Mechanism, MechanismContext, MechanismId, Source,
    WorkSink,
};

use crate::full_build::{full_build, BuildError};
use crate::prepared::UpdateStaging;
use crate::state::ReadyDocument;
use crate::structural::NoopHorseAStructuralSink;
use crate::update::{stage, UpdateError};

/// Public wrapper over the crate-private pre-frontier staging state
/// ([`UpdateStaging`]): the `Mechanism::Prepared` carrier between
/// `prepare_update` (T_prepare) and `update` (T_native). The interior
/// stays crate-private; nothing outside this crate can inspect it.
pub struct HorseAPrepared(UpdateStaging);

/// Campaign identity of the sixth horse (campaign roster order 6).
pub const HORSE_A_MECHANISM_ID: &str = "horse-a-v1";

/// Common-`Mechanism` frontend over the frozen Horse-A update pipeline.
#[derive(Debug, Default, Clone, Copy)]
pub struct HorseAMechanism;

impl HorseAMechanism {
    pub fn new() -> Self {
        HorseAMechanism
    }
}

/// `Pending`: the sealed new READY state awaiting the runner's explicit
/// completion boundary.
pub struct HorseAPending {
    state: ReadyDocument,
}

/// Build/Update refusals mapped to the shared failure schema. The mapping
/// is per-variant and honest:
///
/// - `InvalidAssociation` / `InvalidTriviaDocument` — the input class has
///   no legal retained state for this mechanism: `Unsupported`;
/// - `ResourceRefused` — an allocation the commit requires was refused:
///   `Oom`;
/// - `InconsistentObservation` / `InvariantViolation` — parser/READY
///   invariant evidence contradicts the frozen contracts: `Crash`.
fn build_failure(e: BuildError) -> FailureStatus {
    match e {
        BuildError::InvalidTriviaDocument { .. } => FailureStatus::Unsupported,
        BuildError::InconsistentObservation { .. } | BuildError::InvariantViolation(_) => {
            FailureStatus::Crash
        }
    }
}

fn update_failure(e: UpdateError) -> FailureStatus {
    match e {
        UpdateError::InvalidAssociation { .. } => FailureStatus::Unsupported,
        UpdateError::ResourceRefused => FailureStatus::Oom,
        UpdateError::InconsistentObservation { .. } => FailureStatus::Crash,
        UpdateError::FullBuild(e) => build_failure(e),
    }
}

impl Mechanism for HorseAMechanism {
    type State = ReadyDocument;
    type Prepared = HorseAPrepared;
    type Pending = HorseAPending;

    fn id(&self) -> MechanismId {
        MechanismId(HORSE_A_MECHANISM_ID.to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        let state =
            full_build(source, cx.sink, &mut NoopHorseAStructuralSink).map_err(build_failure)?;
        Ok(HorseAPending { state })
    }

    fn prepare_update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        stage(
            old_state,
            old_source,
            post_source,
            edit,
            cx.sink,
            &mut NoopHorseAStructuralSink,
        )
        .map(HorseAPrepared)
        .map_err(update_failure)
    }

    fn update<W: WorkSink>(
        &self,
        _old_source: &Source,
        _post_source: &Source,
        _edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        // Frontier crossing: infallible by type (no Result, no fallback,
        // no allocation); the only failure mode is a process-level panic
        // on an invariant bug, which the runner's catch_unwind records.
        let commit = prepared.0.prepare(old_state);
        Ok(HorseAPending {
            state: commit.commit(&mut NoopHorseAStructuralSink),
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        // Native-sealing ONLY: the frontier already returned a complete,
        // eager READY state; the checksum is the runner's post-timer
        // export (ReadyDocument's NormalizeV1 + ResultChecksum).
        Ok(Completed {
            state: pending.state,
        })
    }
}
