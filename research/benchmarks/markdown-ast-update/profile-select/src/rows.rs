//! Universe profiling: run REAL-MARKDOWN-PROFILER-v1 over every candidate
//! and reduce each full profile to a selection-grade [`CandidateRow`]
//! (CORRECTIVE-B §10-§13).
//!
//! The full profile records (with spans) are written to
//! `workloads/profiles/real-profile-v1.jsonl` — one canonical-JSON line
//! per candidate, in frozen lexical order. The compact rows are written
//! to `workloads/profiles/candidate-rows-v1.jsonl`. Both files are
//! derived data at candidate-universe scale and are gitignored; their
//! SHA-256 identities are recorded in the committed distributions
//! artifact, following the PR #34 storage model for candidate-scale
//! material.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use markit_mdbench_semantics::{
    profile as profile_source, LaneProfile, Profile, RecognitionStatus, SyntaxFact, SyntaxKind,
};

use crate::{
    universe, CandidateIdentity, CandidateRow, LaneRow, CANDIDATE_ROW_SCHEMA, PROFILED_GRAMMAR_IDS,
};

#[derive(Debug, Clone, Default)]
pub struct ProfilingOutcome {
    pub rows: Vec<CandidateRow>,
    pub profile_failures: u64,
    pub ambiguous_facts: u64,
    pub unknown_facts: u64,
    /// Total bytes of the emitted full-profile JSONL.
    pub profile_jsonl_bytes: u64,
}

/// Profile one candidate's bytes. `source` must be the exact materialized
/// bytes; the caller has already hash-verified them.
pub fn profile_candidate(identity: &CandidateIdentity, source: &str) -> CandidateRow {
    let full = profile_source(source, &PROFILED_GRAMMAR_IDS);
    reduce(identity, &full, None)
}

/// Build a row for a candidate whose bytes could not be profiled at all.
/// Materialization state is preserved honestly (a UTF-8-invalid file can
/// be fully materialized and hash-verified yet unprofilable).
pub fn failure_row(
    identity: &CandidateIdentity,
    reason: &str,
    materialized: bool,
    hash_match: bool,
) -> CandidateRow {
    CandidateRow {
        schema: CANDIDATE_ROW_SCHEMA.to_string(),
        identity: identity.clone(),
        source_sha256: identity.sha256.clone(),
        file_bytes: identity.bytes,
        line_count: 0,
        cjk_byte_share: 0.0,
        newline_form: "unknown".to_string(),
        profile_failure: Some(reason.to_string()),
        lanes: Vec::new(),
        materialized,
        hash_match,
    }
}

fn reduce(identity: &CandidateIdentity, full: &Profile, failure: Option<String>) -> CandidateRow {
    let source_row = &full.source;
    let mut row = CandidateRow {
        schema: CANDIDATE_ROW_SCHEMA.to_string(),
        identity: identity.clone(),
        source_sha256: source_row.source_sha256.clone(),
        file_bytes: source_row.file_bytes,
        line_count: source_row.line_count,
        cjk_byte_share: source_row.cjk_byte_share,
        newline_form: format!("{:?}", source_row.newline_form).to_lowercase(),
        profile_failure: failure,
        lanes: Vec::with_capacity(full.lanes.len()),
        materialized: true,
        hash_match: source_row.source_sha256 == identity.sha256,
    };
    for lane in &full.lanes {
        row.lanes.push(reduce_lane(lane));
    }
    row
}

fn bump(map: &mut BTreeMap<String, u64>, kind: SyntaxKind) {
    *map.entry(kind.name().to_string()).or_insert(0) += 1;
}

fn reduce_lane(lane: &LaneProfile) -> LaneRow {
    let mut row = LaneRow {
        grammar_id: lane.grammar_id.clone(),
        lane_valid: lane.eligibility.lane_valid,
        strict_scope_clean: lane.eligibility.strict_scope_clean,
        blocker_kinds: BTreeMap::new(),
        block_count: lane.structural.block_count,
        largest_block_bytes: lane.structural.largest_block_bytes,
        max_container_depth: lane.structural.max_container_depth,
        fence_density_per_kib: lane.structural.fence_density_per_kib,
        code_occupancy: lane.structural.code_occupancy,
        fenced_code_content_bytes: lane.structural.fenced_code_content_bytes,
        reference_definition_count: lane.structural.reference_definition_count,
        reference_use_count: lane.structural.reference_use_count,
        reference_density_per_kib: lane.structural.reference_density_per_kib,
        table_count: lane
            .structural
            .table
            .as_ref()
            .map(|table| table.table_count)
            .unwrap_or(0),
        max_table_columns: lane
            .structural
            .table
            .as_ref()
            .map(|table| table.max_table_columns)
            .unwrap_or(0),
        max_table_rows_including_header: lane
            .structural
            .table
            .as_ref()
            .map(|table| table.max_table_rows_including_header)
            .unwrap_or(0),
        strict_kinds: BTreeMap::new(),
        declared_kinds: BTreeMap::new(),
        candidate_kinds: BTreeMap::new(),
        nonhost_kinds: BTreeMap::new(),
        ambiguous_kinds: BTreeMap::new(),
        unknown_kinds: BTreeMap::new(),
        table_alignment_marker: false,
        table_candidate_rejected: false,
        inline_inside_table: false,
        fact_count: lane.syntax_facts.len() as u64,
    };

    for blocker in &lane.eligibility.scope_blockers {
        *row.blocker_kinds
            .entry(blocker.syntax_kind.name().to_string())
            .or_insert(0) += 1;
    }

    // Recognized table spans, for the inline-inside-table evidence check.
    let mut table_spans: Vec<(usize, usize)> = Vec::new();
    for fact in &lane.syntax_facts {
        match fact.recognition_status {
            RecognitionStatus::Recognized => {
                if !fact.host_context {
                    bump(&mut row.nonhost_kinds, fact.syntax_kind);
                } else if fact.is_strict_coverage() {
                    bump(&mut row.strict_kinds, fact.syntax_kind);
                    if fact.syntax_kind == SyntaxKind::Table {
                        table_spans.push((fact.source_start, fact.source_end));
                    }
                } else {
                    // Recognized under a declared-but-not-qualified lane
                    // scope (G1 CommonMark base kinds).
                    bump(&mut row.declared_kinds, fact.syntax_kind);
                    if fact.syntax_kind == SyntaxKind::Table {
                        table_spans.push((fact.source_start, fact.source_end));
                    }
                }
            }
            RecognitionStatus::NotRecognized => {
                if fact.host_context {
                    bump(&mut row.candidate_kinds, fact.syntax_kind);
                    if fact.syntax_kind == SyntaxKind::Table {
                        row.table_candidate_rejected = true;
                    }
                } else {
                    bump(&mut row.nonhost_kinds, fact.syntax_kind);
                }
            }
            RecognitionStatus::Ambiguous => {
                if fact.host_context {
                    bump(&mut row.ambiguous_kinds, fact.syntax_kind);
                } else {
                    bump(&mut row.nonhost_kinds, fact.syntax_kind);
                }
            }
            RecognitionStatus::Unknown => {
                if fact.host_context {
                    bump(&mut row.unknown_kinds, fact.syntax_kind);
                } else {
                    bump(&mut row.nonhost_kinds, fact.syntax_kind);
                }
            }
        }
    }

    // Alignment-marker evidence: the G1 oracle emits the table's column
    // alignments as the fact detail (`alignments=[None, Left, …]`);
    // any non-None alignment is an alignment marker.
    for fact in &lane.syntax_facts {
        if fact.syntax_kind == SyntaxKind::Table
            && fact.recognition_status == RecognitionStatus::Recognized
        {
            if let Some(detail) = &fact.detail {
                if detail.contains("Left") || detail.contains("Center") || detail.contains("Right")
                {
                    row.table_alignment_marker = true;
                }
            }
        }
    }

    // Inline-inside-table evidence: a recognized host-context inline fact
    // (code span / emphasis / strong / inline link, strict or declared
    // grade — G1 inline kinds are contract-declared) whose span lies
    // inside a recognized table span (containment, per the profiler's
    // span discipline).
    if !table_spans.is_empty() {
        'facts: for fact in &lane.syntax_facts {
            if fact.recognition_status != RecognitionStatus::Recognized || !fact.host_context {
                continue;
            }
            if !matches!(
                fact.syntax_kind,
                SyntaxKind::CodeSpan
                    | SyntaxKind::Emphasis
                    | SyntaxKind::Strong
                    | SyntaxKind::LinkInline
            ) {
                continue;
            }
            for (start, end) in &table_spans {
                if *start <= fact.source_start && fact.source_end <= *end {
                    row.inline_inside_table = true;
                    break 'facts;
                }
            }
        }
    }

    row
}

/// Profile the whole universe in a single pass: read bytes, verify the
/// hash, profile, reduce to a row, and (when `profile_jsonl` is given)
/// append the full canonical-JSON profile line to the JSONL as it is
/// produced. Rows are collected in frozen lexical order.
pub fn profile_universe(
    workloads_root: &Path,
    identities: &[CandidateIdentity],
    profile_jsonl: Option<&Path>,
) -> Result<ProfilingOutcome, String> {
    let mut jsonl_file = match profile_jsonl {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            Some(fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?)
        }
        None => None,
    };
    let mut outcome = ProfilingOutcome::default();
    for identity in identities {
        let path = universe::materialized_path(workloads_root, identity);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                outcome.profile_failures += 1;
                outcome.rows.push(failure_row(
                    identity,
                    &format!("materialization missing: {error}"),
                    false,
                    false,
                ));
                continue;
            }
        };
        let digest = crate::sha256_hex(&bytes);
        if digest != identity.sha256 || bytes.len() as u64 != identity.bytes {
            outcome.profile_failures += 1;
            outcome.rows.push(failure_row(
                identity,
                "materialization hash mismatch (§9 hard failure)",
                true,
                false,
            ));
            continue;
        }
        match std::str::from_utf8(&bytes) {
            Ok(source) => {
                let full = profile_source(source, &PROFILED_GRAMMAR_IDS);
                if let Some(file) = jsonl_file.as_mut() {
                    let line = crate::canonical_json(&full);
                    file.write_all(line.as_bytes())
                        .map_err(|error| format!("profile jsonl: {error}"))?;
                    file.write_all(b"\n")
                        .map_err(|error| format!("profile jsonl: {error}"))?;
                    outcome.profile_jsonl_bytes += line.len() as u64 + 1;
                }
                let row = reduce(identity, &full, None);
                for lane in &row.lanes {
                    outcome.ambiguous_facts += lane.ambiguous_kinds.values().sum::<u64>();
                    outcome.unknown_facts += lane.unknown_kinds.values().sum::<u64>();
                }
                outcome.rows.push(row);
            }
            Err(error) => {
                outcome.profile_failures += 1;
                outcome.rows.push(failure_row(
                    identity,
                    &format!("not valid UTF-8: {error}"),
                    true,
                    true,
                ));
            }
        }
    }
    if let Some(file) = jsonl_file.as_mut() {
        file.flush()
            .map_err(|error| format!("profile jsonl: {error}"))?;
    }
    Ok(outcome)
}

/// Write the compact candidate-row JSONL.
pub fn write_rows_jsonl(rows: &[CandidateRow], path: &Path) -> Result<u64, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut file =
        fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut total = 0u64;
    for row in rows {
        let line = crate::canonical_json(row);
        file.write_all(line.as_bytes())
            .map_err(|error| format!("{}: {error}", path.display()))?;
        file.write_all(b"\n")
            .map_err(|error| format!("{}: {error}", path.display()))?;
        total += line.len() as u64 + 1;
    }
    file.flush()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(total)
}

/// Load rows back from a candidate-rows JSONL (deterministic order is the
/// file order).
pub fn load_rows_jsonl(path: &Path) -> Result<Vec<CandidateRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(
            serde_json::from_str(line).map_err(|error| format!("{}: {error}", path.display()))?,
        );
    }
    Ok(rows)
}

/// Check a syntax fact's span evidence against actual bytes (spot-check
/// primitive, §45): char-boundary safety plus kind-specific delimiter
/// presence.
pub fn span_evidence_ok(source: &str, fact: &SyntaxFact) -> Result<(), String> {
    let bytes = source.as_bytes();
    let span = fact.span();
    if span.start > bytes.len() || span.end > bytes.len() || span.start > span.end {
        return Err(format!("span out of range: {}..{}", span.start, span.end));
    }
    if !source.is_char_boundary(span.start) || !source.is_char_boundary(span.end) {
        return Err(format!(
            "span not on char boundaries: {}..{}",
            span.start, span.end
        ));
    }
    let text = &source[span.start..span.end];
    let ok = match fact.syntax_kind {
        SyntaxKind::HeadingAtx => text.trim_start().starts_with('#'),
        SyntaxKind::CodeBlockFenced => {
            text.trim_start().starts_with('`') || text.trim_start().starts_with('~')
        }
        SyntaxKind::HeadingSetext => {
            text.trim_end().ends_with('=') || text.trim_end().ends_with('-')
        }
        SyntaxKind::ThematicBreak => text.contains("---") || text.contains("***"),
        SyntaxKind::Table | SyntaxKind::TableHeaderRow | SyntaxKind::TableRow => text.contains('|'),
        SyntaxKind::CodeSpan => text.starts_with('`') && text.ends_with('`') && text.len() >= 2,
        SyntaxKind::Emphasis | SyntaxKind::Strong => {
            text.starts_with('*') || text.starts_with('_') || text.starts_with("**")
        }
        SyntaxKind::LinkInline => text.starts_with('['),
        SyntaxKind::Image => text.starts_with("!["),
        SyntaxKind::LinkAutolink => text.starts_with('<'),
        SyntaxKind::ReferenceDefinition => text.starts_with('['),
        SyntaxKind::HtmlBlock => text.trim_start().starts_with('<'),
        SyntaxKind::RawHtmlInline => text.starts_with('<'),
        SyntaxKind::InlineMath => text.starts_with('$'),
        SyntaxKind::DisplayMath => text.starts_with("$$") || text.starts_with('$'),
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "kind {} span {}..{} does not carry its delimiter: {:?}",
            fact.syntax_kind.name(),
            span.start,
            span.end,
            &text[..text.len().min(48)]
        ))
    }
}
