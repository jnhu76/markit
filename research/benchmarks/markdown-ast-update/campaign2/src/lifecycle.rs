//! Surface C — LIFECYCLE traces (task §15-§17).
//!
//! A lifecycle run builds the horse state ONCE per (trace, horse, session)
//! and then applies `step_count` chained edits to the SAME state, in order,
//! with no reconstruction between edits (task §15).
//!
//! Two trace families:
//!
//! ```text
//! real_break_restore   a frozen #35 BREAK/RESTORE pair from
//!                      `workloads/payloads/trace-manifest-v1.jsonl`,
//!                      replayed as repeat(break, restore). Every step is a
//!                      real frozen payload; the pair returns the source to
//!                      its base bytes, so the chain can be repeated to any
//!                      length without inventing source bytes.
//! L1..L7               the controlled deterministic lifecycle family
//!                      (task §16): repeated local text, moving local edit,
//!                      paragraph split/restore, container mutation, fence
//!                      open/restore, reference definition change/restore,
//!                      mixed editor-style sequence.
//! ```
//!
//! Every trace freezes:
//!
//! ```text
//! initial source identity (sha256 + origin)
//! ordered edits (start, end, inserted text)
//! post-source sha256 after every edit
//! expected normalized checksum after every edit
//! ```
//!
//! The expected checksum is `normalized_checksum(H0 clean full parse(post
//! source))` — the SAME oracle authority Surface A/B use. For real traces
//! it is additionally cross-checked against the frozen manifest's
//! `post_source_sha256`.
//!
//! Traces are frozen BEFORE any timing and are never modified after
//! seeing a result.

use markit_mdbench_common::CanonicalEdit;

use crate::generators::{para_block, para_line, para_region};
use crate::sha256_hex;

/// Frozen schema tag of the lifecycle trace freeze artifact.
pub const LIFECYCLE_TRACE_SCHEMA: &str = "campaign2-lifecycle-trace-v1";

/// Frozen size of the controlled lifecycle base documents.
pub const CONTROLLED_TRACE_BASE_BYTES: usize = 65_536;

/// Frozen step count of every lifecycle trace (>= the largest K checkpoint).
pub const TRACE_STEPS: u32 = 128;

/// Frozen K checkpoints (task §15).
pub const K_CHECKPOINTS: [u32; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

/// One ordered lifecycle edit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceStepV1 {
    pub step: u32,
    pub edit_start: u64,
    pub edit_end: u64,
    pub inserted_text: String,
    pub post_source_sha256: String,
    pub expected_checksum: String,
    pub label: String,
    pub transition_label: String,
}

impl TraceStepV1 {
    pub fn edit(&self) -> CanonicalEdit {
        CanonicalEdit::new(self.edit_start as usize, self.edit_end as usize, &self.inserted_text)
            .expect("frozen trace step is a valid canonical edit")
    }
}

/// Origin of a trace's initial source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceOriginV1 {
    /// Deterministically regenerated from the controlled generator.
    Controlled {
        generator_id: String,
        family: String,
        base_bytes: u64,
    },
    /// A real frozen payload chain: the initial source is the step-0
    /// pre-source of the named frozen trace in the #35 workload.
    RealPayloadChain {
        workload_trace_id: String,
        base_source_sha256: String,
    },
}

/// One frozen lifecycle trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleTraceV1 {
    pub schema: String,
    pub trace_id: String,
    pub family: String,
    pub chain_construction: String,
    pub initial_source_origin: TraceOriginV1,
    pub initial_source_sha256: String,
    pub initial_source_bytes: u64,
    pub step_count: u32,
    pub steps: Vec<TraceStepV1>,
}

/// A trace plan: pure functions of the current source, so a trace can be
/// replayed forward deterministically without storing every intermediate
/// source.
pub trait TracePlan {
    fn trace_id(&self) -> String;
    fn family(&self) -> &'static str;
    fn chain_construction(&self) -> String;
    fn initial_source(&self) -> String;
    fn step_count(&self) -> u32;
    /// The `step`-th edit against the CURRENT source.
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String>;
}

/// Materialize a plan: replay every edit, record post-source hashes and
/// the expected normalized checksum (H0 clean parse of the post source).
pub fn materialize(plan: &dyn TracePlan) -> Result<LifecycleTraceV1, String> {
    let initial = plan.initial_source();
    let mut current = initial.clone();
    let mut steps = Vec::with_capacity(plan.step_count() as usize);
    for step in 0..plan.step_count() {
        let (edit, label, transition_label) = plan.next_edit(step, &current)?;
        let current_source =
            markit_mdbench_common::Source::new(markit_mdbench_common::SourceId(0), current.clone());
        let post = edit
            .apply(&current_source, markit_mdbench_common::SourceId(1))
            .map_err(|e| format!("{} step {step}: applying the trace edit: {e:?}", plan.trace_id()))?
            .as_str()
            .to_string();
        let expected = expected_checksum(&post)?;
        steps.push(TraceStepV1 {
            step,
            edit_start: edit.start_byte(),
            edit_end: edit.end_byte(),
            inserted_text: edit.inserted_text().to_string(),
            post_source_sha256: sha256_hex(post.as_bytes()),
            expected_checksum: format!("{expected:016x}"),
            label,
            transition_label,
        });
        current = post;
    }
    Ok(LifecycleTraceV1 {
        schema: LIFECYCLE_TRACE_SCHEMA.to_string(),
        trace_id: plan.trace_id(),
        family: plan.family().to_string(),
        chain_construction: plan.chain_construction(),
        initial_source_origin: TraceOriginV1::Controlled {
            generator_id: crate::generators::GENERATOR_ID.to_string(),
            family: plan.family().to_string(),
            base_bytes: initial.len() as u64,
        },
        initial_source_sha256: sha256_hex(initial.as_bytes()),
        initial_source_bytes: initial.len() as u64,
        step_count: plan.step_count(),
        steps,
    })
}

/// The oracle authority for every lifecycle step: the H0 clean full parse
/// of the post-edit source, normalized. Verified against
/// NORMALIZED-RESULT-V1 conformance so a malformed reference cannot be
/// silently used as truth.
pub fn expected_checksum(post_source: &str) -> Result<u64, String> {
    let doc = markit_mdbench_full_rebuild::parse_document(post_source.as_bytes());
    markit_mdbench_oracle::validate_normalized(&doc, None)
        .map_err(|e| format!("lifecycle reference gate (post source): {e:?}"))?;
    Ok(markit_mdbench_oracle::normalized_checksum(&doc))
}

// ---------------------------------------------------------------------------
// Controlled base documents
// ---------------------------------------------------------------------------

/// Base A — 64 KiB of 128-byte paragraph blocks. Used by L1, L2, L3, L7.
pub fn base_paragraph_document() -> String {
    let units = CONTROLLED_TRACE_BASE_BYTES / 128;
    let mut out = String::with_capacity(CONTROLLED_TRACE_BASE_BYTES);
    for i in 0..units {
        out.push_str(&para_block(126, i));
    }
    debug_assert_eq!(out.len(), CONTROLLED_TRACE_BASE_BYTES);
    out
}

/// Base B — a document with a container region, used by L4.
pub fn base_container_document() -> String {
    let prefix = 32_768usize;
    let mut out = String::with_capacity(CONTROLLED_TRACE_BASE_BYTES);
    out.push_str(&para_region(prefix, 0x91));
    // 128 quote lines, each its own paragraph inside the quote.
    for i in 0..128usize {
        out.push_str("> ");
        out.push_str(&para_line(62, 0x92 + i));
        out.push('\n');
        out.push_str("> \n");
    }
    let container = out.len() - prefix;
    let suffix = CONTROLLED_TRACE_BASE_BYTES - prefix - container;
    out.push_str(&para_region(suffix, 0x93));
    debug_assert_eq!(out.len(), CONTROLLED_TRACE_BASE_BYTES);
    out
}

/// Base C — a document with one fenced block, used by L5.
pub fn base_fence_document() -> String {
    let prefix = 32_768usize;
    let mut out = String::with_capacity(CONTROLLED_TRACE_BASE_BYTES);
    out.push_str(&para_region(prefix, 0x94));
    out.push_str("```\n");
    out.push_str(&para_block(510, 0x95)); // fence body (raw)
    out.push_str("```\n");
    let used = out.len();
    out.push_str(&para_region(CONTROLLED_TRACE_BASE_BYTES - used, 0x96));
    debug_assert_eq!(out.len(), CONTROLLED_TRACE_BASE_BYTES);
    out
}

/// Base D — a document with a reference definition and its use sites,
/// used by L6.
pub fn base_reference_document() -> String {
    let head = 16_384usize;
    let mut out = String::with_capacity(CONTROLLED_TRACE_BASE_BYTES);
    out.push_str(&para_region(head, 0x97));
    out.push_str("[rd]: /");
    out.push_str(&markit_mdbench_corpusgen::filler(24, 7));
    out.push('\n');
    out.push_str(&para_region(4_096, 0x98));
    for i in 0..64usize {
        let link = format!("[t{i:03}][rd] ");
        out.push_str(&format!("{}{}\n\n", link.repeat(5), markit_mdbench_corpusgen::filler(7, i)));
    }
    let used = out.len();
    out.push_str(&para_region(CONTROLLED_TRACE_BASE_BYTES - used, 0x99));
    debug_assert_eq!(out.len(), CONTROLLED_TRACE_BASE_BYTES);
    out
}

// ---------------------------------------------------------------------------
// L1 — repeated local text (task §16)
// ---------------------------------------------------------------------------

/// The same 8-byte window of the same paragraph is replaced, step after
/// step, with a rotating 8-byte payload. Total document size is constant.
pub struct L1RepeatedLocalText {
    base: String,
    offset: usize,
}

impl L1RepeatedLocalText {
    pub fn new() -> Self {
        let base = base_paragraph_document();
        // Middle of the line of the middle block.
        let offset = (CONTROLLED_TRACE_BASE_BYTES / 2 / 128) * 128 + 60;
        Self { base, offset }
    }
}

impl Default for L1RepeatedLocalText {
    fn default() -> Self {
        Self::new()
    }
}

impl TracePlan for L1RepeatedLocalText {
    fn trace_id(&self) -> String {
        "C2-L1-REPEATED-LOCAL-TEXT".to_string()
    }
    fn family(&self) -> &'static str {
        "L1"
    }
    fn chain_construction(&self) -> String {
        format!(
            "replace_eq of the fixed 8-byte window at offset {} with a rotating payload",
            self.offset
        )
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, _current: &str) -> Result<(CanonicalEdit, String, String), String> {
        // Rotating 8-byte payload over the frozen alphabet; byte 0 encodes
        // the step so consecutive edits differ deterministically.
        let a = (b'a' + (step % 26) as u8) as char;
        let payload = format!("{a}{}", "yyyyyyy");
        let edit = CanonicalEdit::new(self.offset, self.offset + 8, payload)
            .map_err(|e| format!("L1 step {step}: {e:?}"))?;
        Ok((edit, format!("step{step}"), "C2-L1-LOCAL-TEXT-REPLACE".to_string()))
    }
}

// ---------------------------------------------------------------------------
// L2 — moving local edit across blocks
// ---------------------------------------------------------------------------

/// A same-size local replace whose target block advances by one block each
/// step and wraps. The edit is always a replace_eq inside a paragraph.
pub struct L2MovingLocalEdit {
    base: String,
    blocks: usize,
}

impl L2MovingLocalEdit {
    pub fn new() -> Self {
        Self {
            base: base_paragraph_document(),
            blocks: CONTROLLED_TRACE_BASE_BYTES / 128,
        }
    }
}

impl TracePlan for L2MovingLocalEdit {
    fn trace_id(&self) -> String {
        "C2-L2-MOVING-LOCAL-EDIT".to_string()
    }
    fn family(&self) -> &'static str {
        "L2"
    }
    fn chain_construction(&self) -> String {
        "replace_eq of an 8-byte window, target block advancing by one each step".to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, _current: &str) -> Result<(CanonicalEdit, String, String), String> {
        // Advance through the document from the first block, wrapping.
        let block = (step as usize) % self.blocks;
        let offset = block * 128 + 59;
        let a = (b'a' + (step % 26) as u8) as char;
        let payload = format!("{a}{}", "xxxxxxx");
        let edit = CanonicalEdit::new(offset, offset + 8, payload)
            .map_err(|e| format!("L2 step {step}: {e:?}"))?;
        Ok((edit, format!("block{block}"), "C2-L2-LOCAL-TEXT-REPLACE".to_string()))
    }
}

// ---------------------------------------------------------------------------
// L3 — paragraph split / restore
// ---------------------------------------------------------------------------

/// Alternating split (insert a blank line inside a paragraph) and restore
/// (delete it) at the same document position.
pub struct L3ParagraphSplitRestore {
    base: String,
    offset: usize,
}

impl L3ParagraphSplitRestore {
    pub fn new() -> Self {
        let base = base_paragraph_document();
        let offset = (CONTROLLED_TRACE_BASE_BYTES / 2 / 128) * 128 + 60;
        Self { base, offset }
    }
}

impl TracePlan for L3ParagraphSplitRestore {
    fn trace_id(&self) -> String {
        "C2-L3-PARAGRAPH-SPLIT-RESTORE".to_string()
    }
    fn family(&self) -> &'static str {
        "L3"
    }
    fn chain_construction(&self) -> String {
        "alternating insert/delete of a two-byte blank-line boundary at a fixed offset".to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let split = step % 2 == 0;
        // Guard: the split edit is only valid while the window is still a
        // paragraph interior.
        if !split {
            let bytes = current.as_bytes();
            if self.offset + 2 > bytes.len() || &bytes[self.offset..self.offset + 2] != b"\n\n" {
                return Err(format!("L3 step {step}: restore anchor missing"));
            }
        }
        let edit = if split {
            CanonicalEdit::new(self.offset, self.offset, "\n\n")
        } else {
            CanonicalEdit::new(self.offset, self.offset + 2, "")
        }
        .map_err(|e| format!("L3 step {step}: {e:?}"))?;
        Ok((
            edit,
            if split { "split".to_string() } else { "restore".to_string() },
            if split {
                "C2-L3-PARAGRAPH-SPLIT".to_string()
            } else {
                "C2-L3-PARAGRAPH-RESTORE".to_string()
            },
        ))
    }
}

// ---------------------------------------------------------------------------
// L4 — container mutation
// ---------------------------------------------------------------------------

/// Alternating deepen/restore of a blockquote container marker at a fixed
/// container line (the frozen `nest_deepen` / `nest_restore` shape).
pub struct L4ContainerMutation {
    base: String,
    offset: usize,
}

impl L4ContainerMutation {
    pub fn new() -> Self {
        let base = base_container_document();
        // Start of the 64th container content line.
        let container_start = 32_768usize;
        let offset = container_start + 64 * (2 + 62 + 1 + 2 + 1);
        Self { base, offset }
    }
}

impl TracePlan for L4ContainerMutation {
    fn trace_id(&self) -> String {
        "C2-L4-CONTAINER-MUTATION".to_string()
    }
    fn family(&self) -> &'static str {
        "L4"
    }
    fn chain_construction(&self) -> String {
        "alternating nest_deepen/nest_restore of one blockquote marker".to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let deepen = step % 2 == 0;
        if !deepen {
            let bytes = current.as_bytes();
            if self.offset + 2 > bytes.len() || &bytes[self.offset..self.offset + 2] != b"> " {
                return Err(format!("L4 step {step}: container marker anchor missing"));
            }
        }
        let edit = if deepen {
            CanonicalEdit::new(self.offset, self.offset, "> ")
        } else {
            CanonicalEdit::new(self.offset, self.offset + 2, "")
        }
        .map_err(|e| format!("L4 step {step}: {e:?}"))?;
        Ok((
            edit,
            if deepen { "deepen".to_string() } else { "restore".to_string() },
            if deepen {
                "C2-L4-CONTAINER-DEEPEN".to_string()
            } else {
                "C2-L4-CONTAINER-RESTORE".to_string()
            },
        ))
    }
}

// ---------------------------------------------------------------------------
// L5 — fence open / restore
// ---------------------------------------------------------------------------

/// Alternating removal and restoration of a fence closer (the frozen
/// `close_break` / `close_restore` shape).
pub struct L5FenceOpenRestore {
    base: String,
    closer_offset: usize,
}

impl L5FenceOpenRestore {
    pub fn new() -> Self {
        let base = base_fence_document();
        // "```\n" opener at 32768, then `para_block(510)` = 512 bytes of
        // body, so the closer starts at 32768 + 4 + 512.
        let closer_offset = 32_768 + 4 + 512;
        Self { base, closer_offset }
    }
}

impl TracePlan for L5FenceOpenRestore {
    fn trace_id(&self) -> String {
        "C2-L5-FENCE-OPEN-RESTORE".to_string()
    }
    fn family(&self) -> &'static str {
        "L5"
    }
    fn chain_construction(&self) -> String {
        "alternating removal/restoration of the three backticks of one fence closer".to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let remove = step % 2 == 0;
        let bytes = current.as_bytes();
        if self.closer_offset + 3 > bytes.len() {
            return Err(format!("L5 step {step}: closer anchor out of range"));
        }
        let has_backticks = &bytes[self.closer_offset..self.closer_offset + 3] == b"```";
        // The chain is only valid while the document is in the expected
        // phase: backticks present before a remove, absent before a restore.
        if remove != has_backticks {
            return Err(format!(
                "L5 step {step}: closer anchor in the wrong phase (backticks present = {has_backticks})"
            ));
        }
        let edit = if remove {
            CanonicalEdit::new(self.closer_offset, self.closer_offset + 3, "")
        } else {
            CanonicalEdit::new(self.closer_offset, self.closer_offset, "```")
        }
        .map_err(|e| format!("L5 step {step}: {e:?}"))?;
        Ok((
            edit,
            if remove { "close_break".to_string() } else { "close_restore".to_string() },
            if remove {
                "C2-L5-FENCE-CLOSER-REMOVE".to_string()
            } else {
                "C2-L5-FENCE-CLOSER-RESTORE".to_string()
            },
        ))
    }
}

// ---------------------------------------------------------------------------
// L6 — reference definition change / restore
// ---------------------------------------------------------------------------

/// Alternating change and restore of the reference definition destination
/// (same length, so the document size is constant).
pub struct L6ReferenceDefineRestore {
    base: String,
    dest_offset: usize,
}

const C2_L6_DEST_BYTES: usize = 24;

impl L6ReferenceDefineRestore {
    pub fn new() -> Self {
        let base = base_reference_document();
        // "[rd]: /" is 7 bytes and starts right after the head padding.
        let dest_offset = 16_384 + 7;
        Self { base, dest_offset }
    }
}

impl TracePlan for L6ReferenceDefineRestore {
    fn trace_id(&self) -> String {
        "C2-L6-REFDEF-CHANGE-RESTORE".to_string()
    }
    fn family(&self) -> &'static str {
        "L6"
    }
    fn chain_construction(&self) -> String {
        "alternating replace_eq of the reference definition destination".to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let changed = step % 2 == 0;
        if !changed {
            let bytes = current.as_bytes();
            if self.dest_offset + C2_L6_DEST_BYTES > bytes.len() {
                return Err(format!("L6 step {step}: destination anchor missing"));
            }
        }
        let payload = if changed {
            markit_mdbench_corpusgen::filler(C2_L6_DEST_BYTES, 7 + step as usize + 1)
        } else {
            markit_mdbench_corpusgen::filler(C2_L6_DEST_BYTES, 7)
        };
        let edit = CanonicalEdit::new(
            self.dest_offset,
            self.dest_offset + C2_L6_DEST_BYTES,
            payload,
        )
        .map_err(|e| format!("L6 step {step}: {e:?}"))?;
        Ok((
            edit,
            if changed { "change".to_string() } else { "restore".to_string() },
            if changed {
                "C2-L6-REFDEF-DEST-CHANGE".to_string()
            } else {
                "C2-L6-REFDEF-DEST-RESTORE".to_string()
            },
        ))
    }
}

// ---------------------------------------------------------------------------
// L7 — mixed deterministic editor-style sequence
// ---------------------------------------------------------------------------

/// A deterministic rotation over the six edit shapes above, at rotating
/// positions: the editor-style "not one kind of edit" sequence.
pub struct L7MixedSequence {
    base: String,
}

impl L7MixedSequence {
    pub fn new() -> Self {
        Self {
            base: base_paragraph_document(),
        }
    }
}

impl TracePlan for L7MixedSequence {
    fn trace_id(&self) -> String {
        "C2-L7-MIXED-EDITOR-SEQUENCE".to_string()
    }
    fn family(&self) -> &'static str {
        "L7"
    }
    fn chain_construction(&self) -> String {
        "rotation of replace_eq / insert / delete / split-restore over rotating paragraph blocks"
            .to_string()
    }
    fn initial_source(&self) -> String {
        self.base.clone()
    }
    fn step_count(&self) -> u32 {
        TRACE_STEPS
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let blocks = CONTROLLED_TRACE_BASE_BYTES / 128;
        // One 4-step cycle per block: the insert (phase 1) and its paired
        // delete (phase 2) always land on the SAME block and offset, so
        // the document returns to its base size every four steps.
        let cycle = (step / 4) as usize;
        let phase = step % 4;
        let block = (cycle * 7) % blocks;
        let base_offset = block * 128;
        match phase {
            0 => {
                let a = (b'a' + (step % 26) as u8) as char;
                let payload = format!("{a}wwwwwww");
                let edit = CanonicalEdit::new(base_offset + 40, base_offset + 48, payload)
                    .map_err(|e| format!("L7 step {step}: {e:?}"))?;
                Ok((edit, format!("block{block}/replace_eq"), "C2-L7-REPLACE-EQ".to_string()))
            }
            1 => {
                let edit = CanonicalEdit::new(base_offset + 30, base_offset + 30, "qqqq")
                    .map_err(|e| format!("L7 step {step}: {e:?}"))?;
                Ok((edit, format!("block{block}/insert"), "C2-L7-INSERT".to_string()))
            }
            2 => {
                let bytes = current.as_bytes();
                if base_offset + 34 > bytes.len()
                    || &bytes[base_offset + 30..base_offset + 34] != b"qqqq"
                {
                    return Err(format!("L7 step {step}: paired delete anchor missing"));
                }
                let edit = CanonicalEdit::new(base_offset + 30, base_offset + 34, "")
                    .map_err(|e| format!("L7 step {step}: {e:?}"))?;
                Ok((edit, format!("block{block}/delete"), "C2-L7-DELETE".to_string()))
            }
            _ => {
                // Same-length replace over a wider window at a third
                // position of the same block (disjoint from phases 0-2).
                let edit = CanonicalEdit::new(base_offset + 20, base_offset + 36, "vvvvvvvvvvvvvvvv")
                    .map_err(|e| format!("L7 step {step}: {e:?}"))?;
                Ok((edit, format!("block{block}/replace_eq_wide"), "C2-L7-REPLACE-EQ-WIDE".to_string()))
            }
        }
    }
}

/// Every controlled lifecycle plan, in a fixed order.
pub fn controlled_plans() -> Vec<Box<dyn TracePlan>> {
    vec![
        Box::new(L1RepeatedLocalText::new()),
        Box::new(L2MovingLocalEdit::new()),
        Box::new(L3ParagraphSplitRestore::new()),
        Box::new(L4ContainerMutation::new()),
        Box::new(L5FenceOpenRestore::new()),
        Box::new(L6ReferenceDefineRestore::new()),
        Box::new(L7MixedSequence::new()),
    ]
}

// ---------------------------------------------------------------------------
// Real break/restore chains
// ---------------------------------------------------------------------------

/// A frozen real BREAK/RESTORE pair replayed as `repeat(break, restore)`.
pub struct RealPairChain {
    pub workload_trace_id: String,
    pub base_source: String,
    pub base_sha256: String,
    pub broken_source: String,
    /// SHA256 the frozen manifest records for the break step's post source.
    pub broken_source_sha256: String,
    pub break_edit: CanonicalEdit,
    pub restore_edit: CanonicalEdit,
    pub break_transition: String,
    pub restore_transition: String,
    pub steps: u32,
}

impl TracePlan for RealPairChain {
    fn trace_id(&self) -> String {
        format!("C2-REAL-PAIR|{}", self.workload_trace_id)
    }
    fn family(&self) -> &'static str {
        "real_break_restore"
    }
    fn chain_construction(&self) -> String {
        format!(
            "repeat(break, restore) of the frozen #35 pair {:?}; \
             each step is a real frozen payload",
            self.workload_trace_id
        )
    }
    fn initial_source(&self) -> String {
        self.base_source.clone()
    }
    fn step_count(&self) -> u32 {
        self.steps
    }
    fn next_edit(&self, step: u32, current: &str) -> Result<(CanonicalEdit, String, String), String> {
        let breaking = step % 2 == 0;
        let expected = if breaking { &self.base_source } else { &self.broken_source };
        if current != expected.as_str() {
            return Err(format!(
                "real pair {} step {step}: chain state diverged from the frozen pair",
                self.workload_trace_id
            ));
        }
        if breaking {
            Ok((
                self.break_edit.clone(),
                "break".to_string(),
                self.break_transition.clone(),
            ))
        } else {
            Ok((
                self.restore_edit.clone(),
                "restore".to_string(),
                self.restore_transition.clone(),
            ))
        }
    }
}

/// Deterministic preregistered selection of real pair traces (task §16).
///
/// Rule (frozen before any timing, never re-selected from results):
///
/// - a candidate is a frozen #35 trace whose two steps form an exact
///   `base -> broken -> base` pair (`post(step 0) == pre(step 1)` and
///   `pre(step 0) == post(step 1)`);
/// - candidates are grouped by `(edit_family, break_transition)`, so a
///   family carrying several frozen transitions contributes one trace per
///   TRANSITION rather than several for whichever transition sorts first;
/// - within a group the lexicographically smallest `trace_id` wins;
/// - at most `per_transition` traces per group, and only traces whose base
///   source is at most `max_base_bytes`.
pub fn select_real_pair_traces(
    workload: &markit_mdbench_campaign::workload::CampaignWorkload,
    per_transition: usize,
    steps: u32,
    max_base_bytes: usize,
) -> Vec<RealPairChain> {
    use std::collections::BTreeMap;
    let mut by_trace: BTreeMap<&str, Vec<&markit_mdbench_campaign::workload::EditWriteCase>> =
        BTreeMap::new();
    for case in &workload.edit_write {
        by_trace.entry(case.trace_id.as_str()).or_default().push(case);
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for (trace_id, group) in by_trace {
        if group.len() != 2 {
            continue;
        }
        // The frozen #35 naming rule: a trace is named after its BREAK
        // transition, so the break step is the one whose transition is the
        // trace id's prefix. Deriving the phase from the payload hash order
        // would be arbitrary, and both assignments satisfy the
        // pre/post symmetry checks below.
        let prefix = trace_id.split('|').next().unwrap_or_default();
        let breaking: Vec<_> = group
            .iter()
            .filter(|case| case.expected_transition == prefix)
            .collect();
        if breaking.len() != 1 {
            continue;
        }
        let break_case = breaking[0];
        let restore_case = group
            .iter()
            .find(|case| case.payload_id != break_case.payload_id);
        let Some(restore_case) = restore_case else {
            continue;
        };
        // The frozen pair must be exactly base -> broken -> base.
        if break_case.pre_source_text != restore_case.post_source_text {
            continue;
        }
        if break_case.post_source_text != restore_case.pre_source_text {
            continue;
        }
        if break_case.pre_source_text.len() > max_base_bytes {
            continue;
        }
        let key = format!("{}|{}", break_case.edit_family, break_case.expected_transition);
        let taken = counts.entry(key).or_insert(0);
        if *taken >= per_transition {
            continue;
        }
        *taken += 1;
        out.push(RealPairChain {
            workload_trace_id: trace_id.to_string(),
            base_source: break_case.pre_source_text.clone(),
            base_sha256: crate::sha256_hex(break_case.pre_source_text.as_bytes()),
            broken_source: break_case.post_source_text.clone(),
            broken_source_sha256: crate::sha256_hex(
                break_case.post_source_text.as_bytes(),
            ),
            break_edit: break_case.edit.clone(),
            restore_edit: restore_case.edit.clone(),
            break_transition: break_case.expected_transition.clone(),
            restore_transition: restore_case.expected_transition.clone(),
            steps,
        });
    }
    out

}
