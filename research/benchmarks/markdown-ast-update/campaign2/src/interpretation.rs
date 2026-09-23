//! Interpretation rules inherited from the PR #46 audit (task §6).
//!
//! These are FROZEN INTO Campaign-2 METADATA. They constrain how the raw
//! evidence may be read; they are not findings and not conclusions.
//!
//! The rules below are quoted verbatim in meaning (not in formatting)
//! from `audit/algorithm-identity/*` at authority SHA
//! `3762b7a42e1c284a4c2c2e0ebac8496e70c63431`, and are copied here so a
//! Campaign-2 reader cannot silently reinterpret a counter.

/// R1 — `blocks_reparsed` semantics.
pub const R1_BLOCKS_REPARSED: &str = "blocks_reparsed counts fresh block-structure / skeleton \
units produced by reparse. It is NOT a count of top-level blocks and NOT a byte count.";

/// R2 — H3 source coverage.
pub const R2_H3_SOURCE_COVERAGE: &str = "For H3, unique source inspection = parser work + \
reuse-vouch / consultation margin reads. Full source coverage therefore does NOT automatically \
mean full reparse.";

/// R3 — H4 `convergence_distance`.
pub const R3_H4_CONVERGENCE_DISTANCE: &str = "Interpret convergence_distance together with \
reuse: when nodes_reused > 0 it is the forward distance to accepted suffix reuse; when \
nodes_reused == 0 it is the forward parse distance to EOF. Never interpret the gauge alone.";

/// R4 — H1 `full_parse` fallback slot.
pub const R4_H1_FULL_PARSE_FALLBACK: &str = "If the frozen implementation reports Unknown on the \
H1 full_parse fallback slot because of the known applicability mismatch, treat it as \
not-applicable-by-phase. Do NOT convert it into update fallback evidence.";

/// All four rules, in order, for hashing into the campaign specification.
pub fn rules() -> [&'static str; 4] {
    [
        R1_BLOCKS_REPARSED,
        R2_H3_SOURCE_COVERAGE,
        R3_H4_CONVERGENCE_DISTANCE,
        R4_H1_FULL_PARSE_FALLBACK,
    ]
}

/// Rule identifiers paired with their text (stable order).
pub fn tagged_rules() -> Vec<(String, String)> {
    [("R1", R1_BLOCKS_REPARSED), ("R2", R2_H3_SOURCE_COVERAGE),
     ("R3", R3_H4_CONVERGENCE_DISTANCE), ("R4", R4_H1_FULL_PARSE_FALLBACK)]
    .into_iter()
    .map(|(id, text)| (id.to_string(), text.to_string()))
    .collect()
}
