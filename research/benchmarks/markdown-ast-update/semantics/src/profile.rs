//! REAL-MARKDOWN-PROFILER-v1 — the executable profiler.
//!
//! Authority: `workloads/profiles/PROFILER-CONTRACT-v1.md`. One profile
//! record per source: parser-independent [`SourceFacts`] plus one
//! [`LaneProfile`] per requested grammar lane, each carrying syntax facts,
//! structural facts and eligibility facts for that lane only.
//!
//! Everything here is deterministic and measurement-free: no clock, no
//! horse, no H0-H4 state. The profiler is a selection/characterization
//! instrument, never a performance instrument.

use serde::{Deserialize, Serialize};

use crate::canonical::sha256_hex;
use crate::facts::{
    EligibilityFacts, FactReason, LaneScopeGrade, NewlineForm, RecognitionStatus, SourceFacts,
    Span, StructuralFacts, SyntaxFact, SyntaxKind, PROFILER_VERSION, PROFILE_SCHEMA,
};
use crate::g0::parse_g0;
use crate::g1::parse_g1;
use crate::lanes::{
    g0_lane, g1_lane, lane_spec, ConstructStatus, LaneSpec, G0_GRAMMAR_ID, G1_GRAMMAR_ID,
};
use crate::parse::LaneParse;
use crate::probes::{probe_candidates, RawCandidate};

/// CJK byte-share ranges, frozen by the profiler contract.
pub const CJK_RANGES: &[(char, char)] = &[
    ('\u{3000}', '\u{303F}'),   // CJK symbols and punctuation
    ('\u{3040}', '\u{309F}'),   // Hiragana
    ('\u{30A0}', '\u{30FF}'),   // Katakana
    ('\u{3400}', '\u{4DBF}'),   // CJK unified ideographs extension A
    ('\u{4E00}', '\u{9FFF}'),   // CJK unified ideographs
    ('\u{AC00}', '\u{D7AF}'),   // Hangul syllables
    ('\u{F900}', '\u{FAFF}'),   // CJK compatibility ideographs
    ('\u{FF00}', '\u{FFEF}'),   // halfwidth and fullwidth forms
    ('\u{20000}', '\u{2FFFF}'), // CJK unified ideographs extension B and beyond
];

/// A complete profile of one source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Profile {
    pub schema: String,
    pub profiler_version: String,
    pub source: SourceFacts,
    pub lanes: Vec<LaneProfile>,
}

/// Per-lane profile record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LaneProfile {
    pub lane_id: String,
    pub grammar_id: String,
    pub lane_version: String,
    pub configuration: String,
    pub eligibility: EligibilityFacts,
    pub syntax_facts: Vec<SyntaxFact>,
    pub structural: StructuralFacts,
    /// Constructs the lane oracle emitted that the frozen configuration
    /// cannot produce (recorded, never silently relabeled).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unexpected_oracle_tags: Vec<String>,
    /// Registry/parse inconsistencies observed while assembling facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consistency_warnings: Vec<String>,
}

/// SOURCE_FACT computation (parser-independent).
pub fn source_facts(source: &str) -> SourceFacts {
    let bytes = source.as_bytes();
    let file_bytes = bytes.len() as u64;
    let lf_count = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
    let cr_count = bytes.iter().filter(|byte| **byte == b'\r').count() as u64;
    let crlf_count = source.matches("\r\n").count() as u64;
    let final_line_terminated = bytes.last() == Some(&b'\n');
    let newline_form = match (lf_count, cr_count, crlf_count) {
        (0, 0, _) => NewlineForm::None,
        (0, _, _) => NewlineForm::Cr,
        (_lf, 0, _) => NewlineForm::Lf,
        (lf, cr, crlf) if lf == crlf && cr == crlf => NewlineForm::Crlf,
        _ => NewlineForm::Mixed,
    };
    let line_count = if source.is_empty() {
        0
    } else {
        lf_count + u64::from(!final_line_terminated)
    };
    let ascii_bytes = bytes.iter().filter(|byte| **byte < 0x80).count() as u64;
    let non_ascii_bytes = file_bytes - ascii_bytes;
    let cjk_bytes = source
        .chars()
        .filter(|c| in_cjk(*c))
        .map(|c| c.len_utf8() as u64)
        .sum::<u64>();

    SourceFacts {
        source_sha256: sha256_hex(bytes),
        file_bytes,
        line_count,
        lf_count,
        cr_count,
        crlf_count,
        final_line_terminated,
        newline_form,
        ascii_bytes,
        non_ascii_bytes,
        cjk_bytes,
        cjk_byte_share: crate::parse::ratio(cjk_bytes as f64, file_bytes as f64),
        utf8_valid: true, // `source` is a `&str` by construction
    }
}

fn in_cjk(c: char) -> bool {
    CJK_RANGES.iter().any(|(lo, hi)| c >= *lo && c <= *hi)
}

/// Profile `source` under every lane in `grammar_ids`.
pub fn profile(source: &str, grammar_ids: &[&str]) -> Profile {
    Profile {
        schema: PROFILE_SCHEMA.to_string(),
        profiler_version: PROFILER_VERSION.to_string(),
        source: source_facts(source),
        lanes: grammar_ids
            .iter()
            .map(|grammar_id| {
                lane_profile(source, grammar_id)
                    .unwrap_or_else(|| panic!("unknown grammar_id {grammar_id}"))
            })
            .collect(),
    }
}

/// Profile `source` under one lane.
pub fn lane_profile(source: &str, grammar_id: &str) -> Option<LaneProfile> {
    let lane = lane_spec(grammar_id)?;
    Some(lane_profile_with(source, &lane))
}

/// Profile `source` under one already-resolved lane.
pub fn lane_profile_with(source: &str, lane: &LaneSpec) -> LaneProfile {
    let parse = parse_for_lane(source, lane);
    let mut warnings: Vec<String> = Vec::new();

    // 1. Facts for constructs the lane oracle recognized.
    let mut facts: Vec<SyntaxFact> = Vec::new();
    for node in parse.nodes() {
        let status = lane.construct_status(node.kind);
        let grade = match status {
            ConstructStatus::Frozen => LaneScopeGrade::StrictLaneCoverage,
            ConstructStatus::DeclaredNotQualified => LaneScopeGrade::ContractDeclaredNotQualified,
            ConstructStatus::OutOfLane => {
                warnings.push(format!(
                    "lane {} recognized {} although the registry marks it out_of_lane",
                    lane.grammar_id,
                    node.kind.name()
                ));
                LaneScopeGrade::ContractDeclaredNotQualified
            }
            ConstructStatus::Deferred => {
                warnings.push(format!(
                    "lane {} recognized {} although the registry marks it deferred",
                    lane.grammar_id,
                    node.kind.name()
                ));
                LaneScopeGrade::ContractDeclaredNotQualified
            }
        };
        facts.push(SyntaxFact {
            grammar_id: lane.grammar_id.clone(),
            syntax_kind: node.kind,
            source_start: node.span.start,
            source_end: node.span.end,
            recognition_status: RecognitionStatus::Recognized,
            lane_scope_grade: grade,
            host_context: !parse.is_strictly_inside_non_host(node.span),
            reason: FactReason::RecognizedUnderLaneOracle,
            occurrence: 0,
            detail: node.detail.clone(),
        });
    }

    // 2. Out-of-band recognized facts (G1 reference definitions).
    for fact in &parse.out_of_band_facts {
        facts.push(fact.clone());
    }

    // 3. Candidate evidence. Two sources of candidates exist: constructs
    // this lane does not recognize at all, and shapes the lane oracle
    // declined to recognize (inside literal content, or failing a lane
    // rule). Neither is ever silently dropped.
    let recognized = lane.recognized_kinds();
    for candidate in probe_candidates(source) {
        let host_context = !parse.is_inside_non_host(candidate.span);
        if host_context && recognized.contains(&candidate.kind) {
            let claimed = facts.iter().any(|fact| {
                fact.syntax_kind == candidate.kind
                    && fact.recognition_status == RecognitionStatus::Recognized
                    && fact.span().intersects(&candidate.span)
            });
            if claimed {
                // The lane oracle owns this construct here; the recognized
                // fact is the authority and the lexical candidate would be
                // a duplicate of it.
                continue;
            }
            facts.push(SyntaxFact {
                grammar_id: lane.grammar_id.clone(),
                syntax_kind: candidate.kind,
                source_start: candidate.span.start,
                source_end: candidate.span.end,
                recognition_status: RecognitionStatus::NotRecognized,
                lane_scope_grade: grade_of(lane.construct_status(candidate.kind)),
                host_context: true,
                reason: FactReason::CandidateRejectedByLaneRule,
                occurrence: 0,
                detail: candidate.detail.clone(),
            });
            continue;
        }
        facts.push(candidate_fact(source, lane, &parse, &candidate));
    }

    facts.sort_by(|a, b| {
        (
            a.source_start,
            a.source_end,
            a.syntax_kind.name(),
            a.recognition_status as u8,
        )
            .cmp(&(
                b.source_start,
                b.source_end,
                b.syntax_kind.name(),
                b.recognition_status as u8,
            ))
    });
    assign_occurrences(&mut facts);

    let scope_blockers: Vec<SyntaxFact> = facts
        .iter()
        .filter(|fact| fact.is_scope_blocker())
        .cloned()
        .collect();

    let (block_kinds, container_kinds) = match lane.grammar_id.as_str() {
        G0_GRAMMAR_ID => (crate::g0::G0_BLOCK_KINDS, crate::g0::G0_CONTAINER_KINDS),
        G1_GRAMMAR_ID => (crate::g1::G1_BLOCK_KINDS, crate::g1::G1_CONTAINER_KINDS),
        _ => (&[][..], &[][..]),
    };

    // A lane with no frozen semantics has no defined interpretation, so
    // `lane_valid` is false and the zero-filled structural record is
    // explicitly flagged as not defined. Never report `lane_valid: true`
    // for a lane that cannot interpret the source.
    let implemented = lane_has_implementation(lane);
    if !implemented {
        warnings.push(format!(
            "lane {} has no frozen semantics/oracle: lane_valid=false and the structural record is not defined (grammar/GRAMMAR-LANES-v1.md §4)",
            lane.grammar_id
        ));
    }

    LaneProfile {
        lane_id: lane.lane_id.clone(),
        grammar_id: lane.grammar_id.clone(),
        lane_version: lane.lane_version.clone(),
        configuration: lane
            .reference_oracle
            .as_ref()
            .map(|oracle| oracle.configuration.clone())
            .unwrap_or_else(|| "no oracle".to_string()),
        eligibility: EligibilityFacts {
            grammar_id: lane.grammar_id.clone(),
            lane_valid: implemented,
            strict_scope_clean: implemented && scope_blockers.is_empty(),
            scope_blockers,
            strict_scope_clean_rule: lane.eligibility_rules.strict_scope_clean_rule.clone(),
        },
        syntax_facts: facts,
        structural: crate::parse::structural_facts(&parse, block_kinds, container_kinds),
        unexpected_oracle_tags: parse.extras.unexpected_oracle_tags.clone(),
        consistency_warnings: warnings,
    }
}

/// Whether this lane has a frozen implementation (an oracle-backed parser).
///
/// G2 (math) is identity-only in CORRECTIVE-A: it has no semantics, no
/// normalized vocabulary, and no oracle, so it cannot interpret any source.
pub fn lane_has_implementation(lane: &LaneSpec) -> bool {
    matches!(lane.lane_id.as_str(), "G0" | "G1")
}

fn parse_for_lane(source: &str, lane: &LaneSpec) -> LaneParse {
    match lane.lane_id.as_str() {
        "G0" => parse_g0(source),
        "G1" => parse_g1(source),
        // A lane without an implementation cannot be profiled; the caller
        // must use a lane whose registry entry declares an oracle.
        _ => LaneParse {
            grammar_id: lane.grammar_id.clone(),
            root: crate::parse::LaneNode::new(SyntaxKind::Document, Span::new(0, source.len())),
            non_host_spans: Vec::new(),
            literal_spans: Vec::new(),
            extras: crate::parse::LaneExtras {
                fenced_code_content_bytes: 0,
                reference_definition_count: 0,
                reference_definition_counting: "not implemented".to_string(),
                reference_use_count: 0,
                reference_use_counting: "not implemented".to_string(),
                table: None,
                math_occupancy_note: "lane has no implementation".to_string(),
                unexpected_oracle_tags: Vec::new(),
            },
            out_of_band_facts: Vec::new(),
        },
    }
}

fn candidate_fact(
    _source: &str,
    lane: &LaneSpec,
    parse: &LaneParse,
    candidate: &RawCandidate,
) -> SyntaxFact {
    let host_context = !parse.is_inside_non_host(candidate.span);
    let status = lane.construct_status(candidate.kind);
    let (recognition_status, lane_scope_grade, reason) = if !host_context {
        (
            RecognitionStatus::NotRecognized,
            grade_of(status),
            FactReason::NonHostContext,
        )
    } else {
        match status {
            // Owned by a lane whose semantics are not frozen yet. A
            // complete candidate is genuinely ambiguous between the lanes
            // that would read it differently; a partial candidate has no
            // reading at all, so it stays unknown. Neither is resolved.
            ConstructStatus::Deferred => {
                if candidate.complete {
                    (
                        RecognitionStatus::Ambiguous,
                        LaneScopeGrade::LaneDeferred,
                        FactReason::AmbiguousAcrossDeclaredLanes,
                    )
                } else {
                    (
                        RecognitionStatus::Unknown,
                        LaneScopeGrade::LaneDeferred,
                        FactReason::LaneSemanticsDeferred,
                    )
                }
            }
            ConstructStatus::OutOfLane => (
                RecognitionStatus::NotRecognized,
                LaneScopeGrade::OutOfLaneCandidate,
                FactReason::OutsideLaneConstructSet,
            ),
            // Recognized kinds never reach this function.
            ConstructStatus::Frozen | ConstructStatus::DeclaredNotQualified => (
                RecognitionStatus::NotRecognized,
                grade_of(status),
                FactReason::InterpretsAsTextUnderLane,
            ),
        }
    };
    SyntaxFact {
        grammar_id: lane.grammar_id.clone(),
        syntax_kind: candidate.kind,
        source_start: candidate.span.start,
        source_end: candidate.span.end,
        recognition_status,
        lane_scope_grade,
        host_context,
        reason,
        occurrence: 0,
        detail: candidate.detail.clone(),
    }
}

fn grade_of(status: ConstructStatus) -> LaneScopeGrade {
    match status {
        ConstructStatus::Frozen => LaneScopeGrade::StrictLaneCoverage,
        ConstructStatus::DeclaredNotQualified => LaneScopeGrade::ContractDeclaredNotQualified,
        ConstructStatus::OutOfLane => LaneScopeGrade::OutOfLaneCandidate,
        ConstructStatus::Deferred => LaneScopeGrade::LaneDeferred,
    }
}

fn assign_occurrences(facts: &mut [SyntaxFact]) {
    let mut counters: Vec<((SyntaxKind, RecognitionStatus), u32)> = Vec::new();
    for fact in facts.iter_mut() {
        let key = (fact.syntax_kind, fact.recognition_status);
        match counters.iter_mut().find(|(existing, _)| *existing == key) {
            Some((_, count)) => {
                *count += 1;
                fact.occurrence = *count;
            }
            None => {
                counters.push((key, 0));
                fact.occurrence = 0;
            }
        }
    }
}

/// Convenience: the single-lane profiles the pilot and tests need most.
pub fn profile_g0(source: &str) -> LaneProfile {
    lane_profile_with(source, &g0_lane())
}

pub fn profile_g1(source: &str) -> LaneProfile {
    lane_profile_with(source, &g1_lane())
}
