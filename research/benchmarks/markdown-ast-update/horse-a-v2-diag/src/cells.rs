//! The frozen #98 diagnostic cell manifest (L0.6).
//!
//! Frozen BEFORE treatment collection: the stable IDs, roles, document
//! construction, and canonical edits below are the diagnostic manifest.
//! Nothing here was tuned after seeing comparator timing.
//!
//! # Lineage (what "existing" means for these cells)
//!
//! - Padding comes from the frozen campaign2 controlled-construction
//!   generator `campaign2::generators::para_region` (byte-exact
//!   paragraph regions over the frozen `'a'..'z'` filler alphabet) —
//!   the same generator h4diag (#50) reused for its cells.
//! - The E6 document geometry is the frozen campaign2 C-F
//!   reference-fanout skeleton (`C-F` task §24): 128 KiB total, an
//!   8 KiB head, a 32-byte reference-definition slot at offset 8192, a
//!   4 KiB definition-environment pad, then 64 fixed 64-byte slots
//!   (each use-site slot holds five `[t<i>][rd] ` full-reference
//!   links), then a paragraph tail. #98's E6 cells adapt this skeleton
//!   (duplicate definitions, value edits, negative-dependency inserts,
//!   a fence variant, a late-definition variant) — each adaptation is
//!   stated in the cell's `role` string.
//! - Tiny cells sit at frozen campaign2 Axis::D size points (64 B,
//!   1 KiB, 4 KiB) and use the frozen G0 transition semantics
//!   (E1 same-length text replace, E2 paragraph split, E5 inline
//!   delimiter break).
//! - Sentinels use the frozen G0 container/inline transition semantics
//!   (list-item indent, blockquote nest, emphasis delimiter break) on
//!   128 KiB documents.
//!
//! # Roles and comparator arms
//!
//! `E6_CHALLENGE` cells run arms A/B/C/D (H2 included: it answers the
//! distinct same-contract semantic-regime question). `TINY` cells run
//! A/B/C. `SENTINEL` cells run A/B.

use markit_mdbench_common::CanonicalEdit;

/// Comparator arm identifiers (L0.2). `D` (H2) exists for E6 only.
pub const ARM_A: &str = "HORSE_A_V1_NORMAL";
pub const ARM_B: &str = "HORSE_A_V1_DIRECT_READY_REBUILD";
pub const ARM_C: &str = "H0_REFERENCE";
pub const ARM_D: &str = "H2_REFERENCE";

/// One frozen diagnostic cell.
pub struct DiagCell {
    /// Stable frozen ID (never reused after treatment collection).
    pub id: &'static str,
    /// Headline family the cell belongs to.
    pub family: &'static str,
    /// Role in the #98 diagnostic (E6_CHALLENGE / TINY / SENTINEL) plus
    /// the semantic statement of the challenge.
    pub role: &'static str,
    /// Comparator arms for this role (L0.2).
    pub arms: &'static [&'static str],
    /// The pre-edit source (deterministic, byte-exact construction).
    pub pre_source: String,
    /// The canonical edit: (start_byte, end_byte, inserted_text).
    pub edit: (usize, usize, &'static str),
    /// Expected # of `[rd]`-style consumer links affected by the cell's
    /// semantic statement (the frozen challenge description's count;
    /// the effect ledger re-derives the truth from H0).
    pub declared_consumers: usize,
}

impl DiagCell {
    pub fn edit(&self) -> CanonicalEdit {
        CanonicalEdit::new(self.edit.0, self.edit.1, self.edit.2)
            .unwrap_or_else(|e| panic!("cell {} has an invalid canonical edit: {e}", self.id))
    }
}

// ---------------------------------------------------------------------------
// Shared frozen construction helpers (campaign2 lineage)
// ---------------------------------------------------------------------------

/// C-F frozen geometry constants (campaign2 task §24), reused verbatim.
const CF_HEAD: usize = 8_192;
const CF_DEF_BYTES: usize = 32;
const CF_ENV_PAD: usize = 4_096;
const CF_SLOT_COUNT: usize = 64;
const CF_SLOT_BYTES: usize = 64;
const CF_SLOT_TEXT: usize = CF_SLOT_BYTES - 2;
const CF_N: usize = 131_072;

use markit_mdbench_campaign2::generators::{para_block, para_region};
use markit_mdbench_common::{Source, SourceId};
use markit_mdbench_corpusgen::filler;

/// One 32-byte definition line `"[rd]: /<24 bytes>\n"` (C-F shape).
fn def_line(destination_24: &str) -> String {
    let body = format!("[rd]: /{destination_24}");
    assert_eq!(body.len() + 1, CF_DEF_BYTES, "definition line must be 32 bytes");
    format!("{body}\n")
}

/// A use-site slot: five `[t<i>][rd] ` full-reference links + filler
/// (C-F shape, 64 bytes).
fn use_site_block(i: usize) -> String {
    let link = format!("[t{i:03}][rd] ");
    let line = format!("{}{}", link.repeat(5), filler(7, i));
    assert_eq!(line.len(), CF_SLOT_TEXT);
    format!("{line}\n\n")
}

/// A padding slot byte-identical in size to a use site.
fn pad_slot(i: usize) -> String {
    para_block(CF_SLOT_TEXT, 0x63 + i)
}

/// The five E6 documents' shared post-def geometry builder.
struct E6Doc {
    pre: String,
    def_start: usize,
}

/// C-F-shaped document with ONE definition at the C-F offset and `f`
/// occupied use-site slots (the frozen E6-4/E6-5 base shape).
fn e6_doc_single_def(f_slots: usize, destination_24: &str) -> E6Doc {
    let mut pre = String::with_capacity(CF_N);
    pre.push_str(&para_region(CF_HEAD, 0x61));
    let def_start = pre.len();
    pre.push_str(&def_line(destination_24));
    pre.push_str(&para_region(CF_ENV_PAD, 0x62));
    let slots_start = pre.len();
    for i in 0..CF_SLOT_COUNT {
        if i < f_slots {
            pre.push_str(&use_site_block(i));
        } else {
            pre.push_str(&pad_slot(i));
        }
    }
    debug_assert_eq!(pre.len(), slots_start + CF_SLOT_COUNT * CF_SLOT_BYTES);
    pre.push_str(&para_region(CF_N - pre.len(), 0x64));
    assert_eq!(pre.len(), CF_N, "E6 document must be byte exact");
    E6Doc { pre, def_start }
}

/// C-F-shaped document with a WINNING definition at the C-F offset, a
/// LOSING duplicate placed mid-environment (offset CF_ENV_PAD/2 into the
/// env pad), and `f` occupied use-site slots (the E6-1/E6-2 base shape).
fn e6_doc_duplicate_defs(f_slots: usize, win_24: &str, lose_24: &str) -> (String, usize, usize) {
    let mut pre = String::with_capacity(CF_N);
    pre.push_str(&para_region(CF_HEAD, 0x61));
    let win_start = pre.len();
    pre.push_str(&def_line(win_24));
    // split the environment pad: half, loser def, half
    pre.push_str(&para_region(CF_ENV_PAD / 2, 0x62));
    let lose_start = pre.len();
    pre.push_str(&def_line(lose_24));
    pre.push_str(&para_region(CF_ENV_PAD - CF_ENV_PAD / 2 - CF_DEF_BYTES, 0x62));
    let slots_start = pre.len();
    for i in 0..CF_SLOT_COUNT {
        if i < f_slots {
            pre.push_str(&use_site_block(i));
        } else {
            pre.push_str(&pad_slot(i));
        }
    }
    debug_assert_eq!(pre.len(), slots_start + CF_SLOT_COUNT * CF_SLOT_BYTES);
    pre.push_str(&para_region(CF_N - pre.len(), 0x64));
    assert_eq!(pre.len(), CF_N, "E6 duplicate document must be byte exact");
    (pre, win_start, lose_start)
}

/// C-F-shaped document with NO definition (unresolved `[rd]` consumers)
/// and an empty 32-byte definition SLOT at the C-F offset so the
/// negative-dependency insert lands at the canonical position.
fn e6_doc_no_def(f_slots: usize) -> E6Doc {
    let mut pre = String::with_capacity(CF_N);
    pre.push_str(&para_region(CF_HEAD, 0x61));
    let def_start = pre.len();
    // hold the definition territory with plain paragraph bytes (the
    // insert will replace this 32-byte window with the definition line)
    let hold = para_region(CF_DEF_BYTES, 0x65);
    pre.push_str(&hold);
    pre.push_str(&para_region(CF_ENV_PAD, 0x62));
    let slots_start = pre.len();
    for i in 0..CF_SLOT_COUNT {
        if i < f_slots {
            pre.push_str(&use_site_block(i));
        } else {
            pre.push_str(&pad_slot(i));
        }
    }
    debug_assert_eq!(pre.len(), slots_start + CF_SLOT_COUNT * CF_SLOT_BYTES);
    pre.push_str(&para_region(CF_N - pre.len(), 0x64));
    assert_eq!(pre.len(), CF_N);
    E6Doc { pre, def_start }
}

// ---------------------------------------------------------------------------
// Tiny cell documents (Axis::D size points; G0 transition semantics)
// ---------------------------------------------------------------------------

/// 64 B: one `para_region(64)` block (62 content bytes + LF + blank LF).
fn tiny_64b() -> String {
    let s = para_region(64, 0x61);
    assert_eq!(s.len(), 64);
    s
}

/// 1 KiB: 24 paragraphs of the fixed 42-byte template
/// `"split para alpha NN words here\n\n"` = 1024 bytes.
fn tiny_1k() -> String {
    let mut s = String::with_capacity(1024);
    let mut i = 0;
    while s.len() < 1024 - 44 {
        let para = format!("split para alpha {:02} words here\n\n", i % 100);
        s.push_str(&para);
        i += 1;
    }
    let rem = 1024 - s.len();
    assert!(rem >= 3 && rem % 2 == 0, "tiny 1k remainder {rem} must be a paragraph unit");
    s.push_str(&para_region(rem, 0x62));
    assert_eq!(s.len(), 1024);
    s
}

/// 4 KiB: paragraphs carrying `*emph NN*` spans
/// `"emph para beta NN *focus words* tail\n\n"`.
fn tiny_4k() -> String {
    let mut s = String::with_capacity(4096);
    let mut i = 0;
    while s.len() < 4096 - 46 {
        let para = format!("emph para beta {:02} *focus words {}* tail\n\n", i % 100, i % 10);
        s.push_str(&para);
        i += 1;
    }
    let rem = 4096 - s.len();
    s.push_str(&para_region(rem, 0x63));
    assert_eq!(s.len(), 4096);
    s
}

// ---------------------------------------------------------------------------
// Sentinel documents (128 KiB; G0 container/inline transition semantics)
// ---------------------------------------------------------------------------

/// LIST sentinel: middle region is a contiguous 600-item list; the
/// edit inserts two spaces at the line start of item 300 (indent
/// deepens it into a nested list).
fn sentinel_list() -> (String, usize) {
    let head = para_region(40_960, 0x61);
    let tail = para_region(49_152, 0x64);
    let mut items = String::new();
    for i in 0..600 {
        items.push_str(&format!("- list item {i:03} {}\n", filler(7, i)));
    }
    const LINE: usize = 24; // "- list item NNN xxxxxxx\n"
    assert_eq!(items.len(), 600 * LINE, "list lines must be uniform");
    let items_len = items.len() + 1; // + the blank line after the list
    let mid = CF_N - head.len() - tail.len() - items_len;
    assert!(mid >= 64);
    let mid_pad = para_region(mid, 0x62);
    let items_base = head.len() + mid_pad.len();
    let mut s = String::new();
    s.push_str(&head);
    s.push_str(&mid_pad);
    s.push_str(&items);
    s.push('\n');
    s.push_str(&tail);
    assert_eq!(s.len(), CF_N);
    // item 300's line start (a '-' marker byte)
    let list_start = items_base + 300 * LINE;
    assert_eq!(s.as_bytes()[list_start], b'-');
    (s, list_start)
}

/// BLOCKQUOTE sentinel: middle region is 2400 quote lines; the edit
/// inserts '>' after the '>' of line 1200 (nest deepen).
fn sentinel_bq() -> (String, usize) {
    let head = para_region(40_960, 0x61);
    let tail = para_region(40_960, 0x64);
    let mut lines = String::new();
    for i in 0..1200 {
        lines.push_str(&format!("> quote line {i:04} {}\n", filler(7, i)));
    }
    const LINE: usize = 26; // "> quote line NNNN xxxxxxx\n"
    assert_eq!(lines.len(), 1200 * LINE, "quote lines must be uniform");
    lines.push('\n');
    let lines_len = lines.len();
    let mid = CF_N - head.len() - tail.len() - lines_len;
    assert!(mid >= 64);
    let mid_pad = para_region(mid, 0x62);
    let lines_base = head.len() + mid_pad.len();
    let mut s = String::new();
    s.push_str(&head);
    s.push_str(&mid_pad);
    s.push_str(&lines);
    s.push_str(&tail);
    assert_eq!(s.len(), CF_N);
    let nest_at = lines_base + 600 * LINE; // edit inserts '>' at nest_at+1
    assert_eq!(s.as_bytes()[nest_at], b'>');
    (s, nest_at)
}

/// E5 inline sentinel: middle region is paragraphs with emphasis; the
/// edit deletes the CLOSING '*' of the emphasis span in paragraph 450.
fn sentinel_emph() -> (String, usize) {
    let head = para_region(40_960, 0x61);
    let tail = para_region(40_960, 0x64);
    let mut paras = String::new();
    for i in 0..900 {
        paras.push_str(&format!("sentinel gamma {i:03} *strong words* tail\n\n"));
    }
    const PARA: usize = 40; // "sentinel gamma NNN *strong words* tail\n\n"
    assert_eq!(paras.len(), 900 * PARA, "emphasis paragraphs must be uniform");
    let paras_len = paras.len();
    let mid = CF_N - head.len() - tail.len() - paras_len;
    let mid_pad = para_region(mid, 0x62);
    let paras_base = head.len() + mid_pad.len();
    let mut s = String::new();
    s.push_str(&head);
    s.push_str(&mid_pad);
    s.push_str(&paras);
    s.push_str(&tail);
    assert_eq!(s.len(), CF_N);
    // paragraph 450; closing '*' sits at local offset 32 of the para
    let break_at = paras_base + 450 * PARA + 32;
    assert_eq!(s.as_bytes()[break_at], b'*');
    (s, break_at)
}

// ---------------------------------------------------------------------------
// The frozen manifest
// ---------------------------------------------------------------------------

/// The frozen #98 cell manifest (L0.6). Deterministic; construction is
/// asserted byte-exact at freeze time in `manifest_receipt`.
pub fn frozen_cells() -> Vec<DiagCell> {
    let mut cells = Vec::new();

    // ---- E6-1: losing duplicate changes; effective binding unchanged --
    let win24 = filler(24, 1);
    let lose24 = filler(24, 2);
    let (pre_e61, _win_start, lose_start) = e6_doc_duplicate_defs(64, &win24, &lose24);
    // edit: replace the loser's 24 destination bytes with 24 different ones
    let lose_dest_start = lose_start + 7; // "[rd]: /" is 7 bytes; dest spans +7..+31
    let lose24b = filler(24, 42);
    cells.push(DiagCell {
        id: "E6-1-LOSE-DUP-CHANGE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE losing-duplicate destination changes while the \
               effective first-wins binding is unchanged; \
               64 occupied slots x 5 links = 320 consumers unaffected",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: pre_e61.clone(),
        edit: (lose_dest_start, lose_dest_start + 24, leak_str(lose24b)),
        declared_consumers: 320,
    });

    // ---- E6-2: winning definition deleted; successor takes over -------
    // same pre-document; delete the winner's whole 32-byte line
    let win_line_start = _win_start;
    cells.push(DiagCell {
        id: "E6-2-WINNER-DELETE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE the winning definition line is deleted so the \
               losing duplicate becomes the effective winner; all 320 \
               consumers' resolved destination changes",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: pre_e61,
        edit: (win_line_start, win_line_start + CF_DEF_BYTES, ""),
        declared_consumers: 320,
    });

    // ---- E6-3A: negative dependency, definition appears ----------------
    let no_def = e6_doc_no_def(64);
    let now24 = filler(24, 3);
    let insert_line = def_line(&now24);
    cells.push(DiagCell {
        id: "E6-3A-DEF-CREATE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE negative dependency, absence -> existence: with \
               no [rd] definition, all 320 consumer links render as literal \
               text; inserting the definition at the canonical slot makes \
               them ReferenceLinks (topology change, old Text must change)",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: no_def.pre.clone(),
        edit: (no_def.def_start, no_def.def_start + CF_DEF_BYTES, leak_str(insert_line.clone())),
        declared_consumers: 320,
    });

    // ---- E6-3B: inverse direction, definition removed ------------------
    // pre = E6-3A's post state; delete the definition again
    let edit3a = CanonicalEdit::new(
        no_def.def_start,
        no_def.def_start + CF_DEF_BYTES,
        insert_line.as_str(),
    )
    .unwrap();
    let pre3a_source = Source::new(SourceId(0), no_def.pre.clone());
    let edit3a = CanonicalEdit::new(
        no_def.def_start,
        no_def.def_start + CF_DEF_BYTES,
        insert_line.as_str(),
    )
    .unwrap();
    let post3a_source = edit3a
        .apply(&pre3a_source, SourceId(1))
        .expect("E6-3A post derivation");
    let post3a = post3a_source.as_str().to_owned();
    assert_eq!(post3a.len(), CF_N, "E6-3B pre must stay byte exact");
    cells.push(DiagCell {
        id: "E6-3B-DEF-DELETE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE negative dependency, existence -> absence (the \
               inverse leg of E6-3A in the same short trace): deleting the \
               definition makes all 320 consumers literal text again",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: post3a,
        edit: (no_def.def_start, no_def.def_start + CF_DEF_BYTES, ""),
        declared_consumers: 320,
    });

    // ---- E6-4: effective value change, LOW fanout ----------------------
    let v1 = filler(24, 4);
    let doc4 = e6_doc_single_def(2, &v1); // 2 slots x 5 links = 10 consumers
    let v2 = filler(24, 44);
    cells.push(DiagCell {
        id: "E6-4-LOW-FANOUT-VALUE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE effective winner destination changes with a \
               small affected consumer set (2 occupied slots x 5 links = 10 \
               consumers must change)",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: doc4.pre,
        edit: (doc4.def_start + 7, doc4.def_start + 7 + 24, leak_str(v2.clone())),
        declared_consumers: 10,
    });

    // ---- E6-5: effective value change, HIGH fanout ---------------------
    let doc5 = e6_doc_single_def(64, &v1); // 64 slots x 5 links = 320
    cells.push(DiagCell {
        id: "E6-5-HIGH-FANOUT-VALUE",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE effective winner destination changes with many \
               affected consumers (64 occupied slots x 5 links = 320 \
               consumers must change)",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: doc5.pre,
        edit: (doc5.def_start + 7, doc5.def_start + 7 + 24, leak_str(v2)),
        declared_consumers: 320,
    });

    // ---- E6-6: syntax edit hides an unchanged definition ----------------
    // Definition placed LATE (tail); inserting a fence opener right
    // before it swallows it (and the remaining tail) inside a fenced
    // block; the 320 earlier consumers lose resolution.
    let tail_def = def_line(&v1);
    let head = para_region(CF_HEAD, 0x61);
    let mut s = String::new();
    s.push_str(&head);
    s.push_str(&para_region(CF_ENV_PAD, 0x62));
    let slots_start = s.len();
    for i in 0..CF_SLOT_COUNT {
        if i < 64 {
            s.push_str(&use_site_block(i));
        } else {
            s.push_str(&pad_slot(i));
        }
    }
    debug_assert_eq!(s.len(), slots_start + CF_SLOT_COUNT * CF_SLOT_BYTES);
    let before_tail_def = s.len();
    s.push_str(&tail_def);
    s.push_str(&para_region(CF_N - s.len(), 0x64));
    assert_eq!(s.len(), CF_N);
    cells.push(DiagCell {
        id: "E6-6-FENCE-HIDE-DEF",
        family: "E6_REFERENCE_DEFINITION",
        role: "E6_CHALLENGE syntax-induced semantic change: an unchanged \
               late definition is hidden by inserting a fence-opener line \
               immediately before it (no definition text is edited); all \
               320 earlier consumers lose resolution",
        arms: &[ARM_A, ARM_B, ARM_C, ARM_D],
        pre_source: s,
        edit: (before_tail_def, before_tail_def, "```\n"),
        declared_consumers: 320,
    });

    // ---- Tiny cells (A/B/C) ---------------------------------------------
    let t64 = tiny_64b();
    // E1: same-length replace of one content byte at offset 16
    let old_byte = t64.as_bytes()[16];
    let new_byte = if old_byte == b'q' { b'r' } else { b'q' };
    cells.push(DiagCell {
        id: "TINY-64B-E1",
        family: "E1_LOCAL_TEXT",
        role: "TINY Axis::D 64 B point, G0-LOCAL-TEXT-REPLACE-EQ semantics \
               (same-length single-byte content replace)",
        arms: &[ARM_A, ARM_B, ARM_C],
        pre_source: t64,
        edit: (16, 17, leak_str((new_byte as char).to_string())),
        declared_consumers: 0,
    });

    let t1k = tiny_1k();
    // E2: split the first paragraph at its 3rd space (offset 15)
    let split_at = t1k.find(' ').map(|p| t1k[p + 1..].find(' ').map(|q| p + 1 + q)).flatten();
    let split_at = split_at.expect("tiny 1k template has interior spaces");
    cells.push(DiagCell {
        id: "TINY-1K-E2",
        family: "E2_PARAGRAPH_SPLIT_MERGE",
        role: "TINY Axis::D 1 KiB point, G0-PARAGRAPH-SPLIT semantics \
               (interior space -> blank line)",
        arms: &[ARM_A, ARM_B, ARM_C],
        pre_source: t1k,
        edit: (split_at, split_at + 1, "\n\n"),
        declared_consumers: 0,
    });

    let t4k = tiny_4k();
    // E5: delete one '*' (delimiter break) in the first emphasis span
    let close_at = t4k.find('*').and_then(|open| t4k[open + 1..].find('*').map(|d| open + 1 + d));
    let close_at = close_at.expect("tiny 4k template has emphasis spans");
    cells.push(DiagCell {
        id: "TINY-4K-E5",
        family: "E5_INLINE_DELIMITER",
        role: "TINY Axis::D 4 KiB point, G0-EMPH-DELIM-BREAK semantics \
               (delete the closing '*' of the first emphasis span)",
        arms: &[ARM_A, ARM_B, ARM_C],
        pre_source: t4k,
        edit: (close_at, close_at + 1, ""),
        declared_consumers: 0,
    });

    // ---- Sentinels (A/B) --------------------------------------------------
    let (list_doc, list_item_at) = sentinel_list();
    cells.push(DiagCell {
        id: "SENT-LIST-INDENT",
        family: "E3_CONTAINER_DEPTH",
        role: "SENTINEL Horse-A container strength guard (G0-LIST-ITEM-INDENT \
               semantics: 2-space indent deepens a middle item into a nest)",
        arms: &[ARM_A, ARM_B],
        pre_source: list_doc,
        edit: (list_item_at, list_item_at, "  "),
        declared_consumers: 0,
    });

    let (bq_doc, nest_at) = sentinel_bq();
    cells.push(DiagCell {
        id: "SENT-BQ-NEST",
        family: "E3_CONTAINER_DEPTH",
        role: "SENTINEL Horse-A container strength guard (G0-BQ-NEST-LINE \
               semantics: a middle quote line deepens one level)",
        arms: &[ARM_A, ARM_B],
        pre_source: bq_doc,
        edit: (nest_at + 1, nest_at + 1, ">"),
        declared_consumers: 0,
    });

    let (emph_doc, emph_at) = sentinel_emph();
    // verify the target byte is the closing '*' of an emphasis span
    assert_eq!(emph_doc.as_bytes()[emph_at], b'*', "sentinel E5 target must be '*'");
    cells.push(DiagCell {
        id: "SENT-E5-EMPH",
        family: "E5_INLINE_DELIMITER",
        role: "SENTINEL Horse-A inline/local strength guard \
               (G0-EMPH-DELIM-BREAK semantics on a 128 KiB document)",
        arms: &[ARM_A, ARM_B],
        pre_source: emph_doc,
        edit: (emph_at, emph_at + 1, ""),
        declared_consumers: 0,
    });

    cells
}

/// `&'static str` leak for edit payloads built at freeze time (the
/// manifest is constructed once per process; the leaked strings are the
/// frozen cell identities).
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// The frozen manifest receipt: stable IDs, SHA-256 of each pre source
/// and of the canonical edit triple, sizes, arms. Deterministic.
pub fn manifest_receipt() -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let mut rows = Vec::new();
    for cell in frozen_cells() {
        let edit = cell.edit();
        let mut h = Sha256::new();
        h.update(cell.pre_source.as_bytes());
        let pre_sha = format!("{:x}", h.finalize());
        let mut h = Sha256::new();
        h.update(cell.edit.0.to_le_bytes());
        h.update(cell.edit.1.to_le_bytes());
        h.update(cell.edit.2.as_bytes());
        let edit_sha = format!("{:x}", h.finalize());
        rows.push(serde_json::json!({
            "cell_id": cell.id,
            "family": cell.family,
            "role": cell.role,
            "arms": cell.arms,
            "pre_len_bytes": cell.pre_source.len(),
            "pre_sha256": pre_sha,
            "edit_start": cell.edit.0,
            "edit_end": cell.edit.1,
            "edit_inserted_len": cell.edit.2.len(),
            "edit_sha256": edit_sha,
            "declared_consumers": cell.declared_consumers,
        }));
    }
    serde_json::json!({
        "schema": "HORSE-A-V2-DIAG-98-CELL-MANIFEST-v1",
        "cells": rows,
    })
}
