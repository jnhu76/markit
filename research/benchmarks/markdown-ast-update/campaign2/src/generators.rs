//! Controlled workload generators (task §20-§25).
//!
//! Campaign-2 CORE controls exactly five structural variables:
//!
//! ```text
//! N  document size                (C-N)
//! B  affected block size          (C-B)
//! D  fence propagation span       (C-D)
//! F  reference fanout             (C-F)
//! K  container depth              (C-K)
//! ```
//!
//! Every generated document is BENCH-GRAMMAR-v1 valid and every generated
//! edit is a `LOCAL_TEXT` (or the axis's single designated structural
//! edit), so N/B/D/F/K are the only variables that move inside an axis.
//!
//! ## Frozen construction decisions (recorded, not discovered)
//!
//! - Padding is always made of the SAME paragraph unit kind the axis uses,
//!   so a padding block and an unmodified content block are mechanically
//!   equivalent (`generators::para_region`).
//! - The generated text alphabet is the frozen corpus alphabet
//!   (`corpusgen::filler`, `'a'..'z'`), so no line can accidentally open a
//!   fence, a list, a quote, a heading or a definition.
//! - Exact byte sizes are reached by trimming the LAST unit of a padding
//!   region, never by truncating a structural element.
//! - `LOCAL_TEXT` edits insert a FIXED 8-byte run at a FIXED relative
//!   offset inside a paragraph line, so the edit bytes are identical
//!   across every point of every axis.
//!
//! ## Known entanglements (recorded in EVIDENCE-GAPS.md, not hidden)
//!
//! With `N` fixed, one region must absorb the differing axis value:
//!
//! - C-N: nothing is absorbed — `N` itself is the variable.
//! - C-B: absorbed by the padding block count (`N - B`, always a multiple
//!   of the 128-byte padding unit for every frozen `B`).
//! - C-D: absorbed by the suffix length, which sits AFTER the
//!   reconvergence fence. `D` is exactly the pre-edit byte distance from
//!   the edited closer to the reconvergence closer.
//! - C-F: nothing is absorbed — the 64 use-site slots are a fixed-size
//!   region; `F` selects how many of them hold a reference use.
//! - C-K: absorbed by the trailing padding length, which sits after the
//!   container region.

use markit_mdbench_common::{CanonicalEdit, CaseId, CaseKeyV1, OperationKind, PayloadShape};
use markit_mdbench_corpusgen::filler;

use crate::sha256_hex;

/// Frozen generator identity (task §7 "controlled workload generator
/// identity").
pub const GENERATOR_ID: &str = "CAMPAIGN-2-CONTROLLED-GENERATOR";
/// Frozen generator version.
pub const GENERATOR_VERSION: &str = "campaign-2-controlled-v1";

/// Fixed `LOCAL_TEXT` insertion payload: exactly 8 bytes, no Markdown
/// metacharacter (`'a'..'z'`).
pub const LOCAL_TEXT_INSERT: &str = "zzzzzzzz";

/// The five controlled axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// Document size crossover.
    N,
    /// Affected block size.
    B,
    /// Fence propagation span.
    D,
    /// Reference fanout.
    F,
    /// Container depth.
    K,
}

impl Axis {
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::N => "N",
            Axis::B => "B",
            Axis::D => "D",
            Axis::F => "F",
            Axis::K => "K",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "N" | "n" => Ok(Axis::N),
            "B" | "b" => Ok(Axis::B),
            "D" | "d" => Ok(Axis::D),
            "F" | "f" => Ok(Axis::F),
            "K" | "k" => Ok(Axis::K),
            other => Err(format!("unknown controlled axis {other:?} (N|B|D|F|K)")),
        }
    }

    /// The sub-campaign tag this axis belongs to.
    pub fn sub_campaign_tag(self) -> &'static str {
        match self {
            Axis::N => "controlled_n",
            Axis::B => "controlled_b",
            Axis::D => "controlled_d_fence",
            Axis::F => "controlled_f_reference",
            Axis::K => "controlled_k_container",
        }
    }

    /// Frozen scale points, in ascending order. The exact byte value of
    /// each human-readable label is in [`Axis::points`].
    pub fn point_labels(self) -> &'static [&'static str] {
        match self {
            Axis::N => &[
                "512B", "1KiB", "2KiB", "4KiB", "8KiB", "16KiB", "32KiB", "64KiB", "128KiB", "1MiB",
                "16MiB",
            ],
            Axis::B => &[
                "128B", "512B", "1KiB", "2KiB", "4KiB", "8KiB", "16KiB", "32KiB", "64KiB",
            ],
            Axis::D => &["64B", "256B", "1KiB", "4KiB", "16KiB", "32KiB", "64KiB"],
            Axis::F => &["0", "1", "2", "4", "8", "16", "32", "64"],
            Axis::K => &["0", "1", "2", "3", "4", "6", "8", "12"],
        }
    }

    /// Frozen numeric value of every scale point (bytes for N/B/D, count
    /// for F/K), aligned with [`Axis::point_labels`].
    pub fn points(self) -> &'static [u64] {
        match self {
            Axis::N => &[
                512,
                1024,
                2048,
                4096,
                8192,
                16384,
                32768,
                65536,
                131072,
                1_048_576,
                16_777_216,
            ],
            Axis::B => &[128, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536],
            Axis::D => &[64, 256, 1024, 4096, 16384, 32768, 65536],
            Axis::F => &[0, 1, 2, 4, 8, 16, 32, 64],
            Axis::K => &[0, 1, 2, 3, 4, 6, 8, 12],
        }
    }

    /// The fixed document size of this axis (0 for the C-N axis, whose
    /// size IS the variable).
    pub fn fixed_n_bytes(self) -> u64 {
        FIXED_N_BYTES
    }
}

/// `N` for every axis other than C-N (task §21-§25 "N = 128 KiB").
pub const FIXED_N_BYTES: u64 = 131_072;

/// The 128-byte padding unit used by C-N and C-B: 126 content bytes + LF +
/// blank LF.
pub const PAD128_TEXT: usize = 126;
/// The 64-byte padding unit used by C-D: 62 content bytes + LF + blank LF.
pub const PAD64_TEXT: usize = 62;

/// One paragraph line of `len` content bytes drawn from the frozen corpus
/// alphabet.
pub fn para_line(len: usize, phase: usize) -> String {
    filler(len, phase)
}

/// One paragraph block: `line + "\n" + "\n"` (`len + 2` bytes).
pub fn para_block(len: usize, phase: usize) -> String {
    format!("{}\n\n", para_line(len, phase))
}

/// A paragraph region of EXACTLY `total` bytes: whole 64-byte units while
/// at least 64 remain, then one trimmed final unit.
///
/// The last unit keeps the paragraph + blank-line shape, so the region
/// always ends at a block start and the region's block count is the only
/// thing that changes with `total`.
pub fn para_region(total: usize, phase: usize) -> String {
    let mut out = String::with_capacity(total + 64);
    let mut remaining = total;
    let mut index = 0usize;
    while remaining >= 64 {
        out.push_str(&para_block(PAD64_TEXT, phase.wrapping_add(index)));
        remaining -= 64;
        index += 1;
    }
    if remaining > 0 {
        assert!(
            remaining >= 3,
            "para_region remainder {remaining} cannot hold a paragraph unit"
        );
        out.push_str(&para_block(remaining - 2, phase.wrapping_add(index)));
    }
    assert_eq!(out.len(), total, "para_region must be byte exact");
    out
}

/// One controlled case: pre-edge source, canonical edit, post-edit source
/// and the identity facts of the cell.
#[derive(Debug, Clone)]
pub struct ControlledCase {
    pub axis: Axis,
    pub axis_point_index: u32,
    pub axis_label: String,
    pub axis_value: u64,
    pub cell_id: String,
    pub case_id: CaseId,
    pub case_id_hex: String,
    pub pre_source: String,
    pub edit: CanonicalEdit,
    pub post_source: String,
    pub edit_start: u64,
    pub edit_end: u64,
    /// Informational transition label of this controlled edit (never a
    /// frozen-registry id — controlled documents are synthetic).
    pub transition_label: String,
    /// Absolute byte offset of the edit inside the pre-edit source.
    pub target_offset: u64,
    /// Relative position of the edit inside the pre-edit source.
    pub target_relative: f64,
    /// Free-form construction facts recorded with the cell.
    pub construction: Vec<(String, String)>,
}

impl ControlledCase {
    /// Deterministic `CaseId` over the frozen `CaseKeyV1` machinery.
    fn make_case_id(
        payload_id: &str,
        pre_len: u64,
        pre_sha256: &str,
        operation: OperationKind,
        edit: &CanonicalEdit,
        seed: u64,
    ) -> Result<CaseId, String> {
        let digest = hex_to_32(pre_sha256)?;
        let inserted = if operation.has_inserted_text() {
            Some(hex_to_32(&sha256_hex(edit.inserted_text().as_bytes()))?)
        } else {
            None
        };
        let key = CaseKeyV1 {
            payload_id: payload_id.to_string(),
            payload_shape: PayloadShape::Mixed,
            payload_size_bytes: pre_len,
            old_source_sha256: digest,
            operation,
            edit_start_byte: operation.has_edit().then_some(edit.start_byte()),
            edit_end_byte: operation.has_edit().then_some(edit.end_byte()),
            inserted_text_sha256: inserted,
            generator_id: Some(GENERATOR_ID.to_string()),
            generator_seed: Some(seed),
        }
        .validated()
        .map_err(|e| format!("controlled case key for {payload_id}: {e:?}"))?;
        Ok(CaseId::from_key(&key))
    }
}

fn hex_to_32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("digest {hex:?} is not 64 hex chars"));
    }
    (0..32)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|e| format!("digest {hex:?} byte {i}: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "digest length".to_string())
}

/// Finish a case: apply the edit to obtain the post source, derive the
/// case id, and record the position facts. The post source is ALWAYS
/// `edit.apply(pre)`, so pre/post/case-id can never disagree.
#[allow(clippy::too_many_arguments)]
fn finish(
    axis: Axis,
    point_index: u32,
    label: &str,
    value: u64,
    pre: String,
    edit: CanonicalEdit,
    transition_label: &str,
    seed: u64,
    construction: Vec<(String, String)>,
) -> Result<ControlledCase, String> {
    let pre_source = markit_mdbench_common::Source::new(markit_mdbench_common::SourceId(0), pre.clone());
    let post = edit
        .apply(&pre_source, markit_mdbench_common::SourceId(1))
        .map_err(|e| format!("{} {label}: applying the controlled edit: {e:?}", axis.as_str()))?
        .as_str()
        .to_string();
    let operation = OperationKind::classify(&edit);
    let pre_sha = sha256_hex(pre.as_bytes());
    let cell_id = format!("C2-{}-{}", axis.as_str(), label);
    let case_id = ControlledCase::make_case_id(
        &format!("c2:{}:{label}", axis.as_str().to_lowercase()),
        pre.len() as u64,
        &pre_sha,
        operation,
        &edit,
        seed,
    )?;
    let start = edit.start_byte();
    let relative = start as f64 / pre.len().max(1) as f64;
    Ok(ControlledCase {
        axis,
        axis_point_index: point_index,
        axis_label: label.to_string(),
        axis_value: value,
        cell_id,
        case_id_hex: case_id.hex(),
        case_id,
        edit_start: start,
        edit_end: edit.end_byte(),
        target_offset: start,
        target_relative: relative,
        pre_source: pre,
        edit,
        post_source: post,
        transition_label: transition_label.to_string(),
        construction,
    })
}

/// Build every cell of one axis, in frozen ascending order.
pub fn generate_axis(axis: Axis) -> Result<Vec<ControlledCase>, String> {
    let points = axis.points();
    let labels = axis.point_labels();
    debug_assert_eq!(points.len(), labels.len());
    let mut out = Vec::with_capacity(points.len());
    for (index, (value, label)) in points.iter().zip(labels.iter()).enumerate() {
        let case = match axis {
            Axis::N => case_n(index as u32, *value, label)?,
            Axis::B => case_b(index as u32, *value, label)?,
            Axis::D => case_d(index as u32, *value, label)?,
            Axis::F => case_f(index as u32, *value, label)?,
            Axis::K => case_k(index as u32, *value, label)?,
        };
        out.push(case);
    }
    Ok(out)
}

/// Build one cell by axis + point label (used by the pilot and by
/// `mdbench-replay`).
pub fn generate_cell(axis: Axis, label: &str) -> Result<ControlledCase, String> {
    let labels = axis.point_labels();
    let points = axis.points();
    let index = labels
        .iter()
        .position(|candidate| *candidate == label)
        .ok_or_else(|| {
            format!(
                "unknown point {label:?} for axis {} (have {labels:?})",
                axis.as_str()
            )
        })?;
    let value = points[index];
    match axis {
        Axis::N => case_n(index as u32, value, label),
        Axis::B => case_b(index as u32, value, label),
        Axis::D => case_d(index as u32, value, label),
        Axis::F => case_f(index as u32, value, label),
        Axis::K => case_k(index as u32, value, label),
    }
}

/// Local-text edit: insert the fixed 8-byte run at `offset`, which must sit
/// strictly inside a paragraph line (never at a line boundary).
fn local_text_edit(offset: usize) -> CanonicalEdit {
    CanonicalEdit::new(offset, offset, LOCAL_TEXT_INSERT)
        .expect("local-text insertion is a valid canonical edit")
}

// ---------------------------------------------------------------------------
// C-N — document size crossover (task §21)
// ---------------------------------------------------------------------------

/// `N` bytes of 128-byte paragraph blocks; the target block is the block
/// nearest the document middle and the edit is a fixed 8-byte insertion at
/// the middle of that block's line.
fn case_n(index: u32, n: u64, label: &str) -> Result<ControlledCase, String> {
    let n = n as usize;
    if n % 128 != 0 || n < 512 {
        return Err(format!("C-N size {n} is not a positive multiple of 128 (>= 512)"));
    }
    let blocks = n / 128;
    let target = blocks / 2;
    let mut pre = String::with_capacity(n);
    for b in 0..blocks {
        pre.push_str(&para_block(PAD128_TEXT, b));
    }
    debug_assert_eq!(pre.len(), n);
    let target_start = target * 128;
    // Middle of the 126-byte line: byte 63 of the block.
    let offset = target_start + 63;
    let construction = vec![
        ("n_bytes".to_string(), n.to_string()),
        ("block_unit_bytes".to_string(), "128".to_string()),
        ("block_count".to_string(), blocks.to_string()),
        ("target_block_index".to_string(), target.to_string()),
        ("target_block_bytes".to_string(), "128".to_string()),
        ("target_block_offset".to_string(), target_start.to_string()),
        ("container_depth".to_string(), "0".to_string()),
        ("references".to_string(), "none".to_string()),
        ("fences".to_string(), "none".to_string()),
    ];
    finish(
        Axis::N,
        index,
        label,
        n as u64,
        pre,
        local_text_edit(offset),
        "C2-N-LOCAL-TEXT-INSERT",
        n as u64,
        construction,
    )
}

// ---------------------------------------------------------------------------
// C-B — affected block size (task §22)
// ---------------------------------------------------------------------------

/// `N` fixed at 128 KiB; one target block of `b` bytes near the middle,
/// the rest filled with 128-byte padding blocks.
fn case_b(index: u32, b: u64, label: &str) -> Result<ControlledCase, String> {
    let b = b as usize;
    let n = FIXED_N_BYTES as usize;
    if b < 128 || b > n || (n - b) % 128 != 0 {
        return Err(format!("C-B block size {b} is not usable at N = {n}"));
    }
    let pad_blocks = (n - b) / 128;
    let before = pad_blocks / 2;
    let after = pad_blocks - before;
    let mut pre = String::with_capacity(n);
    for i in 0..before {
        pre.push_str(&para_block(PAD128_TEXT, i));
    }
    let target_start = pre.len() as usize;
    pre.push_str(&content_block(b, 0x51));
    for i in 0..after {
        pre.push_str(&para_block(PAD128_TEXT, i));
    }
    debug_assert_eq!(pre.len(), n, "C-B document must be byte exact");
    // Middle of the target block's line.
    let offset = target_start + (b - 2) / 2;
    let construction = vec![
        ("n_bytes".to_string(), n.to_string()),
        ("target_block_bytes".to_string(), b.to_string()),
        ("target_block_offset".to_string(), target_start.to_string()),
        ("padding_block_bytes".to_string(), "128".to_string()),
        ("padding_blocks_before".to_string(), before.to_string()),
        ("padding_blocks_after".to_string(), after.to_string()),
        ("container_depth".to_string(), "0".to_string()),
        ("references".to_string(), "none".to_string()),
        ("fences".to_string(), "none".to_string()),
    ];
    finish(
        Axis::B,
        index,
        label,
        b as u64,
        pre,
        local_text_edit(offset),
        "C2-B-LOCAL-TEXT-INSERT",
        b as u64,
        construction,
    )
}

/// One content paragraph block of exactly `total` bytes (>= 3).
fn content_block(total: usize, phase: usize) -> String {
    assert!(total >= 3, "content block must hold at least one byte");
    para_block(total - 2, phase)
}

// ---------------------------------------------------------------------------
// C-D — fence propagation span (task §23)
// ---------------------------------------------------------------------------

/// Fixed geometry of the C-D document:
///
/// ```text
/// [0 .. PREFIX)                fixed paragraph padding (256 x 128 B)
/// [PREFIX .. PREFIX+4)         fence opener  "```\n"
/// [+4 .. +516)                 fence body    (512 B, no backtick line)
/// [+516 .. +520)               CLOSER_A      "```\n"   <-- EDIT removes 3 B
/// [+520 .. +520+D-6)           INTERIOR      paragraphs
/// [+520+D-6 .. +520+D)         CLOSER_B      "`````\n" (reconvergence)
/// [+520+D .. N)                SUFFIX        paragraphs
/// ```
///
/// `D` is the pre-edit byte distance from the edited closer to the
/// reconvergence closer. The edit is a 3-byte backtick deletion at a FIXED
/// absolute offset for every `D` (`PREFIX + 4 + 512`).
const C_D_PREFIX: usize = 32_768;
const C_D_OPENER: &str = "```\n";
const C_D_BODY_BYTES: usize = 512;
const C_D_CLOSER_A: &str = "```\n";
const C_D_CLOSER_B: &str = "`````\n";
/// Pre-edit absolute offset of `CLOSER_A` — identical for every `D`.
pub const C_D_EDIT_OFFSET: usize = C_D_PREFIX + C_D_OPENER.len() + C_D_BODY_BYTES;

fn case_d(index: u32, d: u64, label: &str) -> Result<ControlledCase, String> {
    let d = d as usize;
    let n = FIXED_N_BYTES as usize;
    let closet_b_len = C_D_CLOSER_B.len();
    if d <= closet_b_len || d >= n {
        return Err(format!("C-D span {d} must satisfy {closet_b_len} < D < {n}"));
    }
    let mut pre = String::with_capacity(n);
    pre.push_str(&para_region(C_D_PREFIX, 0x11));
    let fence_start = pre.len();
    pre.push_str(C_D_OPENER);
    pre.push_str(&content_block(C_D_BODY_BYTES, 0x22));
    let closer_a_start = pre.len();
    debug_assert_eq!(closer_a_start, C_D_EDIT_OFFSET);
    pre.push_str(C_D_CLOSER_A);
    let interior = d - closet_b_len;
    pre.push_str(&para_region(interior, 0x33));
    let closer_b_start = pre.len();
    // Frozen cell geometry (controlled-cells-v1.txt): interior = D - 6, so
    // the reconvergence closer sits A.len() + D - 6 past the edited
    // closer's start. The old `+ d` expectation contradicted the manifest.
    debug_assert_eq!(
        closer_b_start,
        closer_a_start + C_D_CLOSER_A.len() + d - C_D_CLOSER_B.len()
    );
    pre.push_str(C_D_CLOSER_B);
    let suffix = n
        .checked_sub(pre.len())
        .ok_or_else(|| format!("C-D span {d} overflows N = {n}"))?;
    pre.push_str(&para_region(suffix, 0x44));
    debug_assert_eq!(pre.len(), n, "C-D document must be byte exact");

    // EDIT: delete the three backticks of CLOSER_A, leaving its LF.
    let edit = CanonicalEdit::new(closer_a_start, closer_a_start + 3, "")
        .expect("C-D closer deletion is a valid canonical edit");
    let construction = vec![
        ("n_bytes".to_string(), n.to_string()),
        ("prefix_bytes".to_string(), C_D_PREFIX.to_string()),
        ("fence_opener_offset".to_string(), fence_start.to_string()),
        ("fence_body_bytes".to_string(), C_D_BODY_BYTES.to_string()),
        ("closer_a_offset".to_string(), closer_a_start.to_string()),
        ("closer_b_offset".to_string(), closer_b_start.to_string()),
        ("propagation_span_bytes".to_string(), d.to_string()),
        ("interior_bytes".to_string(), interior.to_string()),
        ("suffix_bytes".to_string(), suffix.to_string()),
        ("opener_backticks".to_string(), "3".to_string()),
        ("reconvergence_closer_backticks".to_string(), "5".to_string()),
    ];
    finish(
        Axis::D,
        index,
        label,
        d as u64,
        pre,
        edit,
        "C2-D-FENCE-CLOSER-REMOVE",
        d as u64,
        construction,
    )
}

// ---------------------------------------------------------------------------
// C-F — reference fanout (task §24)
// ---------------------------------------------------------------------------

/// Fixed geometry of the C-F document:
///
/// ```text
/// [0 .. HEAD)          HEAD paragraph padding
/// [HEAD .. HEAD+32)    the reference DEFINITION  "[rd]: /<24 bytes>\n"  <-- EDIT
/// [+32 .. +32+PAD)     definition-environment padding
/// [SLOTS)              64 x 64 B slots: slot i is a use site when i < F,
///                      otherwise a mechanically equivalent padding block
/// [TAIL)               trailing paragraph padding
/// ```
///
/// Everything except the number of occupied slots is identical across `F`,
/// so `F` cannot be confounded with the document's shape.
const C_F_HEAD: usize = 8_192;
const C_F_DEF_LABEL: &str = "rd";
const C_F_DEF_BYTES: usize = 32;
const C_F_ENV_PAD: usize = 4_096;
const C_F_SLOT_COUNT: usize = 64;
const C_F_SLOT_BYTES: usize = 64;
const C_F_SLOT_TEXT: usize = C_F_SLOT_BYTES - 2;

fn reference_definition() -> String {
    // "[rd]: /" (7) + 24 destination bytes + LF = 32 bytes.
    let body = format!("[{C_F_DEF_LABEL}]: /{}", filler(24, 7));
    debug_assert_eq!(body.len() + 1, C_F_DEF_BYTES);
    format!("{body}\n")
}

fn use_site_block(i: usize) -> String {
    // "[t<i:03>][rd] " is 11 bytes; 5 repetitions + 7 filler bytes = 62.
    let link = format!("[t{i:03}][{C_F_DEF_LABEL}] ");
    debug_assert_eq!(link.len(), 11);
    let line = format!("{}{}", link.repeat(5), filler(7, i));
    debug_assert_eq!(line.len(), C_F_SLOT_TEXT);
    format!("{line}\n\n")
}

fn case_f(index: u32, f: u64, label: &str) -> Result<ControlledCase, String> {
    let f = f as usize;
    let n = FIXED_N_BYTES as usize;
    if f > C_F_SLOT_COUNT {
        return Err(format!("C-F fanout {f} exceeds {} slots", C_F_SLOT_COUNT));
    }
    let slots_bytes = C_F_SLOT_COUNT * C_F_SLOT_BYTES;
    let fixed = C_F_HEAD + C_F_DEF_BYTES + C_F_ENV_PAD + slots_bytes;
    let tail = n
        .checked_sub(fixed)
        .ok_or_else(|| format!("C-F fixed region {fixed} exceeds N = {n}"))?;

    let mut pre = String::with_capacity(n);
    pre.push_str(&para_region(C_F_HEAD, 0x61));
    let def_start = pre.len();
    pre.push_str(&reference_definition());
    pre.push_str(&para_region(C_F_ENV_PAD, 0x62));
    let slots_start = pre.len();
    for i in 0..C_F_SLOT_COUNT {
        if i < f {
            pre.push_str(&use_site_block(i));
        } else {
            pre.push_str(&para_block(C_F_SLOT_TEXT, 0x63 + i));
        }
    }
    debug_assert_eq!(pre.len(), slots_start + slots_bytes);
    pre.push_str(&para_region(tail, 0x64));
    debug_assert_eq!(pre.len(), n, "C-F document must be byte exact");

    // EDIT: delete the whole definition line (32 B, LF included).
    let edit = CanonicalEdit::new(def_start, def_start + C_F_DEF_BYTES, "")
        .expect("C-F definition deletion is a valid canonical edit");
    let construction = vec![
        ("n_bytes".to_string(), n.to_string()),
        ("definition_offset".to_string(), def_start.to_string()),
        ("definition_bytes".to_string(), C_F_DEF_BYTES.to_string()),
        ("definition_label".to_string(), C_F_DEF_LABEL.to_string()),
        ("definition_environment_bytes".to_string(),
         (C_F_ENV_PAD + C_F_DEF_BYTES).to_string()),
        ("slot_count".to_string(), C_F_SLOT_COUNT.to_string()),
        ("slot_bytes".to_string(), C_F_SLOT_BYTES.to_string()),
        ("occupied_slots".to_string(), f.to_string()),
        ("use_distance_distribution".to_string(), "fixed-slot-index".to_string()),
        ("tail_bytes".to_string(), tail.to_string()),
    ];
    finish(
        Axis::F,
        index,
        label,
        f as u64,
        pre,
        edit,
        "C2-F-REFDEF-REMOVE",
        f as u64,
        construction,
    )
}

// ---------------------------------------------------------------------------
// C-K — container depth (task §25)
// ---------------------------------------------------------------------------

/// Fixed geometry of the C-K document:
///
/// ```text
/// [0 .. PREFIX)      fixed paragraph padding
/// [CONTAINER)        C_K_UNITS units, each = one depth-K content line
///                    plus one depth-K blank line, so the unit width is
///                    (4K + C_K_CONTENT + 2) and each unit is its own
///                    paragraph inside the K-deep blockquote stack
/// [SUFFIX)          trailing paragraph padding (absorbs N - K growth)
/// ```
///
/// Sibling width (`C_K_CONTENT`) and sibling count (`C_K_UNITS`) are both
/// CONSTANT across `K`; only the container prefix `"> " * K` changes.
const C_K_PREFIX: usize = 32_768;
const C_K_UNITS: usize = 256;
const C_K_CONTENT: usize = 62;

fn case_k(index: u32, k: u64, label: &str) -> Result<ControlledCase, String> {
    let k = k as usize;
    let n = FIXED_N_BYTES as usize;
    let mut pre = String::with_capacity(n);
    pre.push_str(&para_region(C_K_PREFIX, 0x71));
    let container_start = pre.len();
    let marker = "> ".repeat(k);
    for unit in 0..C_K_UNITS {
        // Content line.
        pre.push_str(&marker);
        pre.push_str(&para_line(C_K_CONTENT, 0x72 + unit));
        pre.push('\n');
        // Blank line INSIDE the container (keeps the quote open, ends the
        // paragraph, so each unit is one paragraph of the container).
        pre.push_str(&marker);
        pre.push('\n');
    }
    let container_bytes = pre.len() - container_start;
    let expected_container = C_K_UNITS * (4 * k + C_K_CONTENT + 2);
    debug_assert_eq!(container_bytes, expected_container);
    let suffix = n
        .checked_sub(pre.len())
        .ok_or_else(|| format!("C-K depth {k} overflows N = {n}"))?;
    pre.push_str(&para_region(suffix, 0x73));
    debug_assert_eq!(pre.len(), n, "C-K document must be byte exact");

    // EDIT: local-text insertion in the middle of the FIRST container
    // content line (fixed line index; absolute offset moves only by the
    // container prefix length, 2*K bytes).
    let offset = container_start + 2 * k + C_K_CONTENT / 2;
    let construction = vec![
        ("n_bytes".to_string(), n.to_string()),
        ("prefix_bytes".to_string(), C_K_PREFIX.to_string()),
        ("container_start".to_string(), container_start.to_string()),
        ("container_depth".to_string(), k.to_string()),
        ("container_marker".to_string(), "block_quote".to_string()),
        ("sibling_width_bytes".to_string(), C_K_CONTENT.to_string()),
        ("sibling_count".to_string(), C_K_UNITS.to_string()),
        ("container_bytes".to_string(), container_bytes.to_string()),
        ("suffix_bytes".to_string(), suffix.to_string()),
        ("edit_line_index".to_string(), "0".to_string()),
    ];
    finish(
        Axis::K,
        index,
        label,
        k as u64,
        pre,
        local_text_edit(offset),
        "C2-K-LOCAL-TEXT-INSERT",
        k as u64,
        construction,
    )
}

// ---------------------------------------------------------------------------
// Self-check
// ---------------------------------------------------------------------------

/// Structural self-check of one generated case: byte-exact sizes, the
/// declared edit location, and a post source that is exactly the pre
/// source with the edit applied.
pub fn verify_case(case: &ControlledCase) -> Result<(), String> {
    let pre_len = case.pre_source.len() as u64;
    if case.axis_value != 0 || true {
        // N axis declares its size in `axis_value`; the others are fixed.
        let expected = match case.axis {
            Axis::N => case.axis_value,
            _ => FIXED_N_BYTES,
        };
        if pre_len != expected {
            return Err(format!(
                "{} {}: pre source {} bytes != declared {expected}",
                case.axis.as_str(),
                case.axis_label,
                pre_len
            ));
        }
    }
    if case.edit_start != case.target_offset {
        return Err(format!(
            "{} {}: recorded target offset {} != edit start {}",
            case.axis.as_str(),
            case.axis_label,
            case.target_offset,
            case.edit_start
        ));
    }
    let pre_source = markit_mdbench_common::Source::new(
        markit_mdbench_common::SourceId(0),
        case.pre_source.clone(),
    );
    let replayed = case
        .edit
        .apply(&pre_source, markit_mdbench_common::SourceId(1))
        .map_err(|e| format!("{} {}: replay: {e:?}", case.axis.as_str(), case.axis_label))?
        .as_str()
        .to_string();
    if replayed != case.post_source {
        return Err(format!(
            "{} {}: post source is not edit.apply(pre)",
            case.axis.as_str(),
            case.axis_label
        ));
    }
    Ok(())
}

/// Generate every axis and verify it.
pub fn generate_all() -> Result<Vec<ControlledCase>, String> {
    let mut out = Vec::new();
    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        for case in generate_axis(axis)? {
            verify_case(&case)?;
            out.push(case);
        }
    }
    Ok(out)
}
