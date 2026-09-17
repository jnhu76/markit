//! Generation receipts (CORPUS-v1 §7): the schema was frozen in R3; the
//! receipts themselves are produced by the FIRST generation — this R4
//! reference generator — and committed under `corpus/receipt-<id>.toml`.
//! `actual_bytes` and `source_sha256` exist only here, never hand-written.

use std::fmt::Write as _;

use markit_mdbench_common::PayloadShape;

use crate::{generate, ALL_SHAPES, ALL_SIZES, CORPUS_GENERATOR_VERSION, GLOBAL_SEED};

/// One generation receipt, rendered as frozen-schema TOML.
pub struct Receipt {
    pub shape: PayloadShape,
    pub size: usize,
    pub actual_bytes: usize,
    pub source_sha256: String,
}

impl Receipt {
    pub fn corpus_id(&self) -> String {
        crate::corpus_id(self.shape, self.size)
    }

    /// Per-shape generation parameters (CORPUS-v1 §4), as ordered
    /// `(key, value)` pairs for the inline `shape_parameters` table.
    fn shape_parameters(&self) -> Vec<(&'static str, String)> {
        match self.shape {
            PayloadShape::Plain => vec![("unit_bytes", "64"), ("cjk_period_units", "16")],
            PayloadShape::ManyBlocks => vec![("unit_bytes", "16"), ("cjk_period_units", "16")],
            PayloadShape::HugeBlock => {
                vec![("unit_bytes", "65536"), ("cjk_head_every_unit", "true")]
            }
            PayloadShape::DeepContainer => vec![
                ("mountain_bytes", "2048"),
                ("lines_per_mountain", "32"),
                ("max_depth", "16"),
                ("cjk_line_period", "8"),
                ("mountain_kinds", "even=list, odd=blockquote"),
            ],
            PayloadShape::InlineDense => vec![("unit_bytes", "64"), ("cjk_period_units", "16")],
            PayloadShape::FenceHeavy => vec![
                ("unit_bytes", "512"),
                ("body_bytes", "250"),
                ("cjk_body_on_odd_units", "true"),
            ],
            PayloadShape::ReferenceFanout => vec![
                ("header_bytes", "512"),
                ("definitions", "16"),
                ("unit_bytes", "64"),
                ("cjk_period_units", "16"),
                ("link_ordinal", "unit_index mod 1000"),
            ],
            PayloadShape::Mixed => vec![
                ("tile_bytes", "4096"),
                ("structural_prefix_bytes", "1344"),
                ("plain_units_per_tile", "43"),
                ("link_ordinal", "tile_index*8+line mod 1000"),
            ],
        }
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect()
    }

    /// The exact receipt file content (byte-stable across runs).
    pub fn render(&self) -> String {
        let mut params = String::new();
        for (i, (k, v)) in self.shape_parameters().into_iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            if v.parse::<i64>().is_ok() || v == "true" || v == "false" {
                let _ = write!(params, "{k} = {v}");
            } else {
                let _ = write!(params, "{k} = \"{v}\"");
            }
        }
        let mut out = String::with_capacity(512);
        let _ = writeln!(out, "schema = \"corpus-receipt-v1\"");
        let _ = writeln!(out, "corpus_id = \"{}\"", self.corpus_id());
        let _ = writeln!(out, "generator_version = \"{CORPUS_GENERATOR_VERSION}\"");
        let _ = writeln!(out, "seed = {:#x}", GLOBAL_SEED);
        let _ = writeln!(out, "target_bytes = {}", self.size);
        let _ = writeln!(out, "actual_bytes = {}", self.actual_bytes);
        let _ = writeln!(out, "source_sha256 = \"{}\"", self.source_sha256);
        let _ = writeln!(out, "shape_parameters = {{ {params} }}");
        out
    }
}

/// Generate every CORPUS-v1 member and its receipt, in canonical
/// (shape x size) order — 24 receipts.
pub fn all_receipts() -> Result<Vec<Receipt>, crate::CorpusError> {
    let mut out = Vec::with_capacity(24);
    for shape in ALL_SHAPES {
        for size in ALL_SIZES {
            let bytes = generate(shape, size)?;
            let digest = sha2::Sha256::digest(&bytes);
            out.push(Receipt {
                shape,
                size,
                actual_bytes: bytes.len(),
                source_sha256: format!("{digest:x}"),
            });
        }
    }
    Ok(out)
}

use sha2::Digest;
