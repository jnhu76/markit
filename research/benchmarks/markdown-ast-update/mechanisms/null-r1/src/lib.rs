//! markit-mdbench-null-r1 — the R1 harness-validation mechanism.
//!
//! This mechanism is OBVIOUSLY NON-RESEARCH and NON-PARSER. It does not
//! understand paragraphs, headings, lists, quotes, fences, emphasis, code
//! spans, links, references, or any Markdown syntax. It never scans the
//! document body; it only combines O(1) scalar facts (lengths/offsets).
//! Its timing is dummy timing and must never enter a research table.
//!
//! It exists solely to prove: common runner -> timer boundaries ->
//! completion boundary -> oracle hook -> schema-valid result ->
//! deterministic case identity/order.
//!
//! It must not come to resemble H0-H4: it retains no structure derived
//! from the document, detects no damage, reuses nothing, restarts
//! nothing, and re-parses nothing.

pub mod fixture;

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::Completed;
use markit_mdbench_common::FailureStatus;
use markit_mdbench_common::Mechanism;
use markit_mdbench_common::MechanismContext;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::Observed;
use markit_mdbench_common::Source;
use markit_mdbench_common::WorkSink;

pub use fixture::smoke_fixture;

/// Reserved null-mechanism id.
pub const NULL_R1_MECHANISM_ID: &str = "__r1_null__";

/// Test/injection-only failure configuration, LOCAL to this mechanism so
/// the future horse API is never contaminated by injection plumbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailureMode {
    /// Normal deterministic null behavior.
    #[default]
    Pass,
    /// Every phase reports UNSUPPORTED.
    Unsupported,
    /// Every phase reports CRASH.
    Crash,
}

/// O(1) retained state: source length plus a revision counter. No
/// document structure of any kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullState {
    pub source_len_bytes: u64,
    pub revision: u64,
}

/// Trivial prepared metadata derived from edit offsets/lengths only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullPrepared {
    pub edit_start_byte: u64,
    pub edit_end_byte: u64,
    pub inserted_len_bytes: u64,
    pub mix: u64,
}

/// Fully scalar work handed through the explicit completion boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullPending {
    pub old_len_bytes: u64,
    pub post_len_bytes: u64,
    pub edit_start_byte: u64,
    pub edit_end_byte: u64,
    pub inserted_len_bytes: u64,
    pub revision: u64,
}

/// The null mechanism.
#[derive(Debug, Clone, Default)]
pub struct NullMechanism {
    failure_mode: FailureMode,
}

impl NullMechanism {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injection config for orchestration tests only.
    pub fn with_failure_mode(failure_mode: FailureMode) -> Self {
        Self { failure_mode }
    }

    fn check(&self) -> Result<(), FailureStatus> {
        match self.failure_mode {
            FailureMode::Pass => Ok(()),
            FailureMode::Unsupported => Err(FailureStatus::Unsupported),
            FailureMode::Crash => Err(FailureStatus::Crash),
        }
    }
}

/// SplitMix64 finalizer (the documented scalar mix; also used, in stream
/// form, by the runner's shuffle — same primitive, different use).
pub fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// The documented null checksum formula (MEASUREMENT-CORRECTIVE-1): a
/// deterministic scalar mix of exactly the scalars the SEALED null state
/// carries. The checksum is a POST-TIMER experiment export derived from
/// the completed state — never computed inside a mechanism phase — so it
/// is a function of `NullState`, not of the consumed pending scalars.
pub fn null_checksum(state: &NullState) -> u64 {
    let mut h = 0x9E37_79B9_7F4A_7C15u64;
    for v in [state.source_len_bytes, state.revision] {
        h = mix64(h ^ v);
    }
    h
}

impl markit_mdbench_common::ResultChecksum for NullState {
    fn result_checksum(&self) -> u64 {
        null_checksum(self)
    }
}

impl Mechanism for NullMechanism {
    type State = NullState;
    type Prepared = NullPrepared;
    type Pending = NullPending;

    fn id(&self) -> MechanismId {
        MechanismId(NULL_R1_MECHANISM_ID.to_string())
    }

    fn full_parse<W: WorkSink>(
        &self,
        source: &Source,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        self.check()?;
        Ok(NullPending {
            old_len_bytes: source.len_bytes() as u64,
            post_len_bytes: source.len_bytes() as u64,
            edit_start_byte: 0,
            edit_end_byte: 0,
            inserted_len_bytes: 0,
            revision: 0,
        })
    }

    fn prepare_update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: &Self::State,
        _cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Prepared, FailureStatus> {
        self.check()?;
        Ok(NullPrepared {
            edit_start_byte: edit.start_byte(),
            edit_end_byte: edit.end_byte(),
            inserted_len_bytes: edit.inserted_text_len_bytes(),
            mix: mix64(
                edit.start_byte()
                    ^ mix64(edit.end_byte())
                    ^ mix64(edit.inserted_text_len_bytes())
                    ^ mix64(old_source.len_bytes() as u64)
                    ^ mix64(post_source.len_bytes() as u64)
                    ^ mix64(old_state.revision),
            ),
        })
    }

    fn update<W: WorkSink>(
        &self,
        old_source: &Source,
        post_source: &Source,
        edit: &CanonicalEdit,
        old_state: Self::State,
        prepared: Self::Prepared,
        cx: &mut MechanismContext<'_, W>,
    ) -> Result<Self::Pending, FailureStatus> {
        self.check()?;
        // Honest, trivial attribution facts: the null mechanism inspects
        // no source bytes (no inspection events at all — the common
        // collector derives the measured zero), reparses no blocks, never
        // falls back, and has no restart/convergence concept at all.
        cx.sink.add_blocks_reparsed(0);
        cx.sink.set_restart_distance(Observed::NotApplicable);
        cx.sink.set_convergence_distance(Observed::NotApplicable);
        cx.sink.add_fallback_to_full(0);
        let _ = (prepared, old_source);
        Ok(NullPending {
            old_len_bytes: old_source.len_bytes() as u64,
            post_len_bytes: post_source.len_bytes() as u64,
            edit_start_byte: edit.start_byte(),
            edit_end_byte: edit.end_byte(),
            inserted_len_bytes: edit.inserted_text_len_bytes(),
            revision: old_state.revision + 1,
        })
    }

    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus> {
        self.check()?;
        // Native-sealing ONLY (MEASUREMENT-CORRECTIVE-1): no checksum
        // here — the runner derives it post-timer via NullState's
        // ResultChecksum export.
        Ok(Completed {
            state: NullState {
                source_len_bytes: pending.post_len_bytes,
                revision: pending.revision,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use markit_mdbench_common::OperationKind;
    use markit_mdbench_common::ResultChecksum as _;
    use markit_mdbench_common::SourceId;

    #[test]
    fn id_is_the_reserved_null_id() {
        assert_eq!(NullMechanism::new().id().as_str(), "__r1_null__");
    }

    #[test]
    fn checksum_is_deterministic_and_sensitive() {
        // The checksum is a function of the SEALED state (post-timer
        // export, MEASUREMENT-CORRECTIVE-1).
        let p = |revision| NullState {
            source_len_bytes: 14,
            revision,
        };
        assert_eq!(null_checksum(&p(1)), null_checksum(&p(1)));
        assert_ne!(null_checksum(&p(1)), null_checksum(&p(2)));
    }

    #[test]
    fn null_full_parse_and_update_round_trip_scalars() {
        let mech = NullMechanism::new();
        let (old, edit, post) = fixture::smoke_fixture();
        assert_eq!(
            OperationKind::classify(&edit),
            fixture::SMOKE_EDIT_OPERATION
        );

        let mut counters = markit_mdbench_common::WorkCounters::all_unknown();
        let mut sink = markit_mdbench_common::CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);

        let pending = mech.full_parse(&old, &mut cx).expect("full parse");
        let done = mech.complete(pending).expect("complete");
        assert_eq!(
            done.state,
            NullState {
                source_len_bytes: old.len_bytes() as u64,
                revision: 0
            }
        );

        let prepared = mech
            .prepare_update(&old, &post, &edit, &done.state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(&old, &post, &edit, done.state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        // The checksum is the post-timer export of the sealed state.
        assert_eq!(
            done.state.result_checksum(),
            null_checksum(&NullState {
                source_len_bytes: post.len_bytes() as u64,
                revision: 1,
            })
        );
        assert_eq!(counters.blocks_reparsed, Observed::Known(0));
        assert_eq!(counters.restart_distance, Observed::NotApplicable);
        assert_eq!(counters.nodes_rebuilt, Observed::Unknown);
    }

    #[test]
    fn injected_failures_return_statuses() {
        let (old, edit, post) = fixture::smoke_fixture();
        for (mode, expected) in [
            (FailureMode::Unsupported, FailureStatus::Unsupported),
            (FailureMode::Crash, FailureStatus::Crash),
        ] {
            let mech = NullMechanism::with_failure_mode(mode);
            assert_eq!(mech.full_parse(&old, &mut no_ctx()), Err(expected));
            assert_eq!(
                mech.prepare_update(
                    &old,
                    &post,
                    &edit,
                    &NullState {
                        source_len_bytes: 0,
                        revision: 0
                    },
                    &mut no_ctx(),
                ),
                Err(expected)
            );
        }
    }

    fn no_ctx() -> MechanismContext<'static, markit_mdbench_common::NoopWorkSink> {
        // A leak-free way to get a context for full_parse in tests: a
        // scoped sink is fine here because the call fails immediately.
        let sink = Box::leak(Box::new(markit_mdbench_common::NoopWorkSink));
        MechanismContext::new(sink)
    }

    #[test]
    fn fixture_sources_have_distinct_ids() {
        let (old, _edit, post) = fixture::smoke_fixture();
        assert_ne!(old.id(), SourceId(0));
        assert_ne!(old.id(), post.id());
    }
}
