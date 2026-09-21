//! markit-mdbench-diagnostics — frozen-workload correctness diagnosis
//! (#22 REAL WORKLOAD CORRECTNESS CLOSURE).
//!
//! Read-only. No clock, no work counters, no research metric. The crate
//! answers two questions and nothing else:
//!
//! 1. [`isolate`] — for one frozen case and one horse, is the wrong
//!    result caused by the horse's own clean parser, by its retained old
//!    state, or by the incremental update? (Task §7's A/B/C/D
//!    isolation.)
//! 2. [`first_divergence`] — where exactly do two normalized results
//!    first differ? (Task §8's diagnostic comparator, owned by the
//!    oracle crate and re-exported here.)
//!
//! The mechanisms are untouched by this crate: it only calls their
//! frozen `Mechanism` phases with a discarding sink and compares the
//! normalized results they produce.

use markit_mdbench_common::{
    CanonicalEdit, Mechanism, MechanismContext, NoopWorkSink, Source, SourceId,
};
use markit_mdbench_oracle::divergence::{describe, first_divergence, Divergence};

/// Re-exported for the diagnostic binary (the oracle's stable one-line
/// divergence rendering).
pub use markit_mdbench_oracle::divergence::describe as describe_divergence;
use markit_mdbench_oracle::{NormalizeV1, NormalizedDocument};
use markit_mdbench_semantics::canonical_json;

pub use markit_mdbench_oracle::divergence::{is_equal, DivergenceKind};

/// Every horse's pending exposes its already-materialized normalized
/// result under a mechanism-specific method name; this local trait gives
/// the diagnostic one name for all of them. Implemented ONLY here, for
/// the five frozen horses.
pub trait PendingDocument {
    fn pending_document(&self) -> NormalizedDocument;
}

impl PendingDocument for markit_mdbench_full_rebuild::H0Pending {
    fn pending_document(&self) -> NormalizedDocument {
        self.document().clone()
    }
}

impl PendingDocument for markit_mdbench_block_local::H1Pending {
    fn pending_document(&self) -> NormalizedDocument {
        self.result().clone()
    }
}

impl PendingDocument for markit_mdbench_fragment_reuse::H2Pending {
    fn pending_document(&self) -> NormalizedDocument {
        self.result().clone()
    }
}

impl PendingDocument for markit_mdbench_old_tree_subtree_reuse::H3Pending {
    fn pending_document(&self) -> NormalizedDocument {
        self.result().clone()
    }
}

impl PendingDocument for markit_mdbench_restart_convergence::H4Pending {
    fn pending_document(&self) -> NormalizedDocument {
        self.result().clone()
    }
}

/// The §7 classification of one wrong dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationClass {
    /// Nothing diverges: the horse matches H0 through the harness path.
    Pass,
    /// The horse's own clean parse of the PRE or POST source disagrees
    /// with H0 — the defect is in the horse's parsing, not in reuse.
    FullParseWrong,
    /// The horse's clean parse of the PRE source disagrees with H0: the
    /// retained old state does not mean what a clean parse says it means.
    OldStateWrong,
    /// Both clean parses agree with H0, and the update does not: the
    /// defect is in the incremental mechanism.
    UpdateWrong,
    /// A phase failed (execution failure) rather than producing a
    /// divergent result.
    ExecutionFailed,
}

impl IsolationClass {
    pub fn name(self) -> &'static str {
        match self {
            IsolationClass::Pass => "PASS",
            IsolationClass::FullParseWrong => "FULL_PARSE_WRONG",
            IsolationClass::OldStateWrong => "OLD_STATE_WRONG",
            IsolationClass::UpdateWrong => "UPDATE_WRONG",
            IsolationClass::ExecutionFailed => "EXECUTION_FAILED",
        }
    }
}

/// The A/B/C/D isolation evidence for one (case, horse).
#[derive(Debug, Clone)]
pub struct Isolation {
    /// A: the authority — H0 clean full parse of the post source.
    pub reference: NormalizedDocument,
    /// B: the horse's clean full parse of the POST source.
    pub hx_full_post: Option<Divergence>,
    /// C: the horse's clean full parse of the PRE source vs H0's.
    pub hx_full_pre: Option<Divergence>,
    /// D: the horse's update(pre_state, edit) vs A.
    pub hx_update: Option<Divergence>,
    /// The horse's pending result and completed-state projection agree
    /// (they must: the harness oracle reads the completed state).
    pub pending_matches_state: bool,
    /// Set when a phase failed outright; both sides are then absent.
    pub execution_failure: Option<String>,
}

impl Isolation {
    pub fn class(&self) -> IsolationClass {
        if self.execution_failure.is_some() {
            return IsolationClass::ExecutionFailed;
        }
        if self.hx_full_pre.is_some() {
            return IsolationClass::OldStateWrong;
        }
        if self.hx_full_post.is_some() {
            return IsolationClass::FullParseWrong;
        }
        if self.hx_update.is_some() || !self.pending_matches_state {
            return IsolationClass::UpdateWrong;
        }
        IsolationClass::Pass
    }
}

/// Clean full parse of `bytes` through `mech`, returning the completed
/// state's projection and the pending's already-materialized result.
fn clean_parse<M>(
    mech: &M,
    bytes: &[u8],
) -> Result<(NormalizedDocument, NormalizedDocument), String>
where
    M: Mechanism,
    M::State: NormalizeV1,
    M::Pending: PendingDocument,
{
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let source = Source::new(SourceId(0), String::from_utf8_lossy(bytes).into_owned());
    let pending = mech
        .full_parse(&source, &mut cx)
        .map_err(|failure| format!("{failure:?}"))?;
    let pending_doc = pending.pending_document();
    let completed = mech
        .complete(pending)
        .map_err(|failure| format!("{failure:?}"))?;
    Ok((completed.state.normalize_v1(), pending_doc))
}

/// The A/B/C/D isolation for one (case, horse).
///
/// `reference` is H0's clean parse of the POST source (A). `h0_pre` is
/// H0's clean parse of the PRE source, used as the authority for C.
pub fn isolate<M>(
    mech: &M,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
    reference: &NormalizedDocument,
    h0_pre: &NormalizedDocument,
) -> Isolation
where
    M: Mechanism,
    M::State: NormalizeV1,
    M::Pending: PendingDocument,
{
    let mut out = Isolation {
        reference: reference.clone(),
        hx_full_post: None,
        hx_full_pre: None,
        hx_update: None,
        pending_matches_state: true,
        execution_failure: None,
    };

    // C: does the horse's own clean parse of PRE agree with H0's?
    match clean_parse(mech, pre.as_bytes()) {
        Ok((state_doc, pending_doc)) => {
            if pending_doc != state_doc {
                out.pending_matches_state = false;
            }
            out.hx_full_pre = first_divergence(h0_pre, &state_doc);
        }
        Err(failure) => {
            out.execution_failure = Some(format!("Hx full_parse(pre): {failure}"));
            return out;
        }
    }

    // B: does the horse's own clean parse of POST agree with H0's?
    match clean_parse(mech, post.as_bytes()) {
        Ok((state_doc, pending_doc)) => {
            if pending_doc != state_doc {
                out.pending_matches_state = false;
            }
            out.hx_full_post = first_divergence(reference, &state_doc);
        }
        Err(failure) => {
            out.execution_failure = Some(format!("Hx full_parse(post): {failure}"));
            return out;
        }
    }

    // D: the incremental update from the freshly parsed PRE state.
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let state = match mech.full_parse(pre, &mut cx) {
        Ok(pending) => match mech.complete(pending) {
            Ok(completed) => completed.state,
            Err(failure) => {
                out.execution_failure = Some(format!("complete(pre): {failure:?}"));
                return out;
            }
        },
        Err(failure) => {
            out.execution_failure = Some(format!("full_parse(pre): {failure:?}"));
            return out;
        }
    };
    let mut sink = NoopWorkSink;
    let mut cx = MechanismContext::new(&mut sink);
    let prepared = match mech.prepare_update(pre, post, edit, &state, &mut cx) {
        Ok(prepared) => prepared,
        Err(failure) => {
            out.execution_failure = Some(format!("prepare_update: {failure:?}"));
            return out;
        }
    };
    let pending = match mech.update(pre, post, edit, state, prepared, &mut cx) {
        Ok(pending) => pending,
        Err(failure) => {
            out.execution_failure = Some(format!("update: {failure:?}"));
            return out;
        }
    };
    let pending_doc = pending.pending_document();
    let completed = match mech.complete(pending) {
        Ok(completed) => completed,
        Err(failure) => {
            out.execution_failure = Some(format!("complete: {failure:?}"));
            return out;
        }
    };
    let state_doc = completed.state.normalize_v1();
    if pending_doc != state_doc {
        out.pending_matches_state = false;
    }
    out.hx_update = first_divergence(reference, &state_doc);
    out
}

/// One machine-readable inventory row for a wrong dispatch (task §6).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailureRow {
    pub payload_id: String,
    pub case_id: String,
    pub horse: String,
    pub transition_id: String,
    pub step: u32,
    pub edit_family: String,
    pub operation_variant: String,
    pub source_id: String,
    pub source_path: String,
    pub pre_source_sha256: String,
    pub post_source_sha256: String,
    pub edit_start: u64,
    pub edit_end: u64,
    pub inserted_bytes: u64,
    pub requested_positions: Vec<String>,
    pub actual_anchor_byte: u64,
    pub isolation_class: String,
    /// `describe(first_divergence(...))` of the divergent phase (D when
    /// the class is UPDATE_WRONG, else the phase that failed first).
    pub first_divergence: String,
    /// UTF-8 lossy excerpt of the post source around the divergent span.
    pub divergence_excerpt: String,
    pub reference_sha: String,
}

/// Render a bounded excerpt of `source` around `[start, end)`, with the
/// span markers left in place. Deterministic and read-only.
pub fn excerpt(source: &[u8], span: Option<(usize, usize)>) -> String {
    let Some((start, end)) = span else {
        return String::new();
    };
    let window_start = start.saturating_sub(120);
    let window_end = (end + 120).min(source.len());
    let mut text = String::new();
    if window_start > 0 {
        text.push_str("...");
    }
    text.push_str(&String::from_utf8_lossy(
        &source[window_start..start.min(source.len())],
    ));
    text.push_str("【");
    text.push_str(&String::from_utf8_lossy(
        &source[start.min(source.len())..end.min(source.len())],
    ));
    text.push_str("】");
    text.push_str(&String::from_utf8_lossy(
        &source[end.min(source.len())..window_end],
    ));
    if window_end < source.len() {
        text.push_str("...");
    }
    text
}

/// Render one isolation report as a human-readable block.
pub fn render(
    horse: &str,
    payload_id: &str,
    transition_id: &str,
    source_path: &str,
    isolation: &Isolation,
    post: &[u8],
) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "{horse} {payload_id} {transition_id} {source_path}\n"
    ));
    text.push_str(&format!("  class={}\n", isolation.class().name()));
    if let Some(failure) = &isolation.execution_failure {
        text.push_str(&format!("  execution_failure={failure}\n"));
    }
    for (label, divergence) in [
        ("C hx_full_parse(pre) vs H0(pre)", &isolation.hx_full_pre),
        ("B hx_full_parse(post) vs H0(post)", &isolation.hx_full_post),
        ("D hx_update vs H0(post)", &isolation.hx_update),
    ] {
        if let Some(divergence) = divergence {
            text.push_str(&format!("  {label}: {}\n", describe(divergence)));
            text.push_str(&format!(
                "    excerpt: {}\n",
                excerpt(post, divergence.source_span)
            ));
        }
    }
    if !isolation.pending_matches_state {
        text.push_str("  NOTE: pending result != completed-state projection\n");
    }
    text
}

/// Canonical JSON one-liner of a failure row (stable field order).
pub fn row_json(row: &FailureRow) -> String {
    canonical_json(row)
}
