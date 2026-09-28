//! The frozen #60 contract surface for the structural-collection producer
//! (P2-1/P2-2/P2-4/P2-6 of the #60 authorization record).
//!
//! Every constant here is a mechanical transcription of already-frozen
//! pre-treatment authority — nothing is derived from any treatment
//! observation, and nothing here may be retuned after the first
//! decision-bearing raw row:
//!
//! - StudyId / workload identities / geometry: readiness record §3/§4/§5
//!   (`docs/research/horse-a-structural-collection-readiness-1.md`),
//!   synchronizing #60 §3/§4;
//! - thresholds: the synchronized #60 §9.5 table as corrected by merged
//!   ACCOUNTING-CORRECTION-1 (#71 / PR #72 @ 04b6496) and reflected in
//!   the durable spec §16 (`docs/research/horse-a-v1-algorithm.md`);
//! - baselines: the StudyId mechanism revision (8e409322…), the
//!   authorization master (f7fdcda…), and the frozen mechanism identity
//!   (#55 / PR #54).

use markit_mdbench_common::{CanonicalEdit, Source, SourceId};
use markit_mdbench_corpusgen::filler;
use sha2::{Digest, Sha256};

/// The frozen study identity (readiness record §3; binds the reviewed
/// mechanism revision, counter schema, threshold authority, raw schema,
/// execution policy and the three canonical workload identities).
pub const STUDY_ID: &str = "2e061da9cb6fbe57f9ce139ca02382fda4cad672fd9422e6c7d7b885f42f4e17";

/// The protocol identity recorded in every raw row (readiness record §9).
pub const PROTOCOL: &str = "HORSE-A-FAILURE-FIRST-1 / #60";

/// The frozen synchronized-contract revision (P2-4). Identifies the #60
/// authority-synchronized contract revision distinctly from the StudyId,
/// from the PR #72 accounting authority, and from the repository
/// execution commit. Frozen pre-treatment in the readiness record §9;
/// never derived from results and never changed after the first row.
pub const FROZEN_CONTRACT_REVISION: &str = concat!(
    "HORSE-A-FAILURE-FIRST-1/#60@sync-20260928T021211Z",
    "/master-f7fdcdaabc5d761435f3c0e8c17611973642934b",
    "/tree-9ec58ed228972c323da5cccf0b991235a0bc0e8a"
);

/// The StudyId-bound reviewed mechanism revision (P2-2): the master the
/// StudyId preimage names. The authorization master differs from it by
/// the readiness record only; the execution revision differs from both
/// by the producer's own merged commit.
pub const STUDY_MECHANISM_BASELINE: &str = "8e40932239273cc798795a625244ffb3e12b5f0e";

/// The authorization master reviewed by the #60 authorization record
/// (P2-2): readiness-record-only delta over the study baseline.
pub const AUTHORIZATION_BASELINE: &str = "f7fdcdaabc5d761435f3c0e8c17611973642934b";

/// The frozen mechanism identity (#55 / merged PR #54; readiness §2).
pub const MECHANISM: &str = "HORSE-A_v1";
/// The frozen mechanism design head (#55; readiness record §2).
pub const MECHANISM_DESIGN_HEAD: &str = "d44f32345674f1e1db88fb832adb40e291da0120";
/// The frozen mechanism merge commit (#55 / PR #54; readiness record §2).
pub const MECHANISM_MERGE: &str = "3b9ff484e5da73f3f4510afdbcc1f3bc3d877a48";

/// The frozen counter schema identity (readiness record §7).
pub const COUNTER_SCHEMA: &str = "HORSE-A-STRUCTURAL-COUNTERS-v1";

/// The frozen raw-row schema identity (readiness record §9).
pub const RAW_SCHEMA: &str = "HORSE-A-STRUCTURAL-RAW-v1";

/// The frozen 8-byte inserted run (#60 §2/§3).
pub const INSERTED_TEXT: &str = "zzzzzzzz";
/// sha256("zzzzzzzz") — the frozen inserted-text identity of every cell.
pub const INSERTED_TEXT_SHA256: &str =
    "c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429";

/// The frozen local-witness geometry shared by all three cells (#60 §4,
/// §9.1): Δ_old = Δ_new = 2, Q = 2, k_crossed = 2, P_removed = 4,
/// D_payload = 2, no definitions/references/fences, ordered facts [] / [].
pub const DELTA_OLD: u64 = 2;
/// Δ_new — new replacement Owners (#60 §4).
pub const DELTA_NEW: u64 = 2;
/// Q — candidate boundaries actually evaluated (#60 §4, exact).
pub const CANDIDATE_CHECKS: u64 = 2;
/// k_crossed — certified crossed boundaries (#60 §4, exact).
pub const CURSOR_ADVANCES: u64 = 2;
/// P_removed — semantic payload nodes actually removed (#60 §9.1, exact).
pub const P_REMOVED: u64 = 4;
/// D_payload — max removed payload depth (#60 §9.1).
pub const D_PAYLOAD: u64 = 2;
/// Frozen payload nodes surviving into the returned state (#60 §9.5 exact).
pub const FRESH_PAYLOAD_NODES_FINAL: u64 = 4;
/// Upper bound on all fresh payload nodes created by the update (#60 §9.5).
pub const FRESH_PAYLOAD_NODES_TEMPORARY_MAX: u64 = 4;
/// Certificate reads bound (#60 §9.3.1).
pub const CERTIFICATE_READS_MAX: u64 = 5;
/// Certificate writes bound (#60 §9.3.1).
pub const CERTIFICATE_WRITES_MAX: u64 = 2;
/// Retirement frames bound: Δ_old + P_removed (#60 §9.5).
pub const RETIREMENT_FRAMES_MAX: u64 = DELTA_OLD + P_REMOVED;
/// Old-fact owner visit bound: Δ_old + Δ_new (#60 §9.5).
pub const OLD_FACT_OWNER_VISITS_MAX: u64 = DELTA_OLD + DELTA_NEW;
/// Exact RefTable entries visited on this witness (#60 §9.5: exact 0).
pub const REFTABLE_ENTRIES_VISITED: u64 = 0;

/// One frozen primary cell: complete identity, exact geometry and the
/// synchronized #60 §9.5 threshold instantiation (H = H_max(M)). Every
/// value is transcribed from readiness record §4/§5/§6 — never computed
/// here, never retuned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrozenCell {
    /// The canonical cell id (`H4N-128KiB | H4N-1MiB | H4N-16MiB`).
    pub cell_id: &'static str,
    /// The frozen catalog case identity (readiness record §4).
    pub case_id_hex: &'static str,
    /// N — pre-source bytes (a multiple of 128).
    pub n_bytes: u64,
    /// M = N / 128 — retained Owner records.
    pub m: u64,
    /// target = floor(M / 2).
    pub target: u64,
    /// edit_start = edit_end = target * 128 + 63.
    pub edit_start: u64,
    /// H_max(M) — the legal AVL height bound (#60 §9.4; thresholds are
    /// derived from H_max, never from a measured height).
    pub h_max: u64,
    /// The frozen restart cut r (#60 §4).
    pub restart: u64,
    /// q_old — the frozen old-side convergence cut (#60 §4).
    pub convergence_old: u64,
    /// q_new — the frozen new-side convergence cut (#60 §4).
    pub convergence_new: u64,
    /// The frozen old replacement interval, lower Owner rank (#60 §4).
    pub replace_lo: u64,
    /// The frozen old replacement interval, exclusive upper Owner rank.
    pub replace_hi: u64,
    /// Frozen `pre_sha256` byte-identity gate.
    pub pre_sha256: &'static str,
    /// Frozen `post_sha256` byte-identity gate.
    pub post_sha256: &'static str,
    // ---- synchronized #60 §9.5 thresholds (H = H_max) ----
    /// locate_node_visits ≤ H.
    pub locate_visits_max: u64,
    /// safe_predecessor_node_visits ≤ 3H − 2.
    pub safe_predecessor_visits_max: u64,
    /// f1 = locate + safe_predecessor ≤ 4H − 2.
    pub f1_visits_max: u64,
    /// cursor_node_visits ≤ 2H + 4k + Q (k = 2, Q = 2).
    pub cursor_visits_max: u64,
    /// fact_range_node_visits ≤ H + Δ_old(H−1) = 3H − 2.
    pub fact_range_visits_max: u64,
    /// split_node_visits (×2) ≤ 20H.
    pub split_visits_max: u64,
    /// pivot_extract_node_visits (×2) ≤ 8H − 2.
    pub pivot_visits_max: u64,
    /// join_node_visits (×2) ≤ 8H − 2.
    pub join_visits_max: u64,
    /// f2 visits (replace_range total) ≤ 36H − 4.
    pub f2_visits_max: u64,
    /// avl_rotations ≤ 18H − 4.
    pub rotations_max: u64,
    /// sequence_link_writes ≤ 72H − 3.
    pub link_writes_max: u64,
    /// aggregate_reads ≤ 406H − 34.
    pub aggregate_reads_max: u64,
    /// aggregate_writes ≤ 168H − 24.
    pub aggregate_writes_max: u64,
    /// max_retirement_depth ≤ H.
    pub max_retirement_depth_max: u64,
}

/// The three frozen primary cells, in frozen schedule order (readiness
/// record §4/§6; #60 §3/§9.5).
pub const FROZEN_CELLS: [FrozenCell; 3] = [
    FrozenCell {
        cell_id: "H4N-128KiB",
        case_id_hex: "22e51081397aebb06a1c4c3f85bd08a6e7041a438f4aae416970027e1ddbb24e",
        n_bytes: 131_072,
        m: 1_024,
        target: 512,
        edit_start: 65_599,
        h_max: 14,
        restart: 65_408,
        convergence_old: 65_664,
        convergence_new: 65_672,
        replace_lo: 511,
        replace_hi: 513,
        pre_sha256: "58b39c389bfe5d5cbcfdbbd74150d276c7996e1b25a84a467fa4deb3cec29a30",
        post_sha256: "1ca6bb1f138507758230ba868c6a55bb9bc824b94fe0b9c75ff3d39c8fb777ea",
        locate_visits_max: 14,
        safe_predecessor_visits_max: 40,
        f1_visits_max: 54,
        cursor_visits_max: 38,
        fact_range_visits_max: 40,
        split_visits_max: 280,
        pivot_visits_max: 110,
        join_visits_max: 110,
        f2_visits_max: 500,
        rotations_max: 248,
        link_writes_max: 1_005,
        aggregate_reads_max: 5_650,
        aggregate_writes_max: 2_328,
        max_retirement_depth_max: 14,
    },
    FrozenCell {
        cell_id: "H4N-1MiB",
        case_id_hex: "337a179ec729c20c01309d98317f7ca00691d12ca10285b55ade71fb880e191c",
        n_bytes: 1_048_576,
        m: 8_192,
        target: 4_096,
        edit_start: 524_351,
        h_max: 18,
        restart: 524_160,
        convergence_old: 524_416,
        convergence_new: 524_424,
        replace_lo: 4_095,
        replace_hi: 4_097,
        pre_sha256: "681c2c032bd1585deef29cf2ce9f9c2a3b8c4bf163ca54c8001d45309462b294",
        post_sha256: "e3e0d2ddf4717cc9bbd0e60848ada7b542500d143134b39609cd2537da8c174c",
        locate_visits_max: 18,
        safe_predecessor_visits_max: 52,
        f1_visits_max: 70,
        cursor_visits_max: 46,
        fact_range_visits_max: 52,
        split_visits_max: 360,
        pivot_visits_max: 142,
        join_visits_max: 142,
        f2_visits_max: 644,
        rotations_max: 320,
        link_writes_max: 1_293,
        aggregate_reads_max: 7_274,
        aggregate_writes_max: 3_000,
        max_retirement_depth_max: 18,
    },
    FrozenCell {
        cell_id: "H4N-16MiB",
        case_id_hex: "0c00f8138e0e622f6fc7215fc0f0d135e79bf60450e4c79d30e30624a8842117",
        n_bytes: 16_777_216,
        m: 131_072,
        target: 65_536,
        edit_start: 8_388_671,
        h_max: 24,
        restart: 8_388_480,
        convergence_old: 8_388_736,
        convergence_new: 8_388_744,
        replace_lo: 65_535,
        replace_hi: 65_537,
        pre_sha256: "0fdca64e71df4386d4407afa1dd5e72aa0faaf60d6854a1fab730d580be6c78d",
        post_sha256: "c8014b5ecae8ca489cb083e6a1a1f8170b3fd2d8d115a85ffa4c58ab4bb29eb0",
        locate_visits_max: 24,
        safe_predecessor_visits_max: 70,
        f1_visits_max: 94,
        cursor_visits_max: 58,
        fact_range_visits_max: 70,
        split_visits_max: 480,
        pivot_visits_max: 190,
        join_visits_max: 190,
        f2_visits_max: 860,
        rotations_max: 428,
        link_writes_max: 1_725,
        aggregate_reads_max: 9_710,
        aggregate_writes_max: 4_008,
        max_retirement_depth_max: 24,
    },
];

impl FrozenCell {
    /// Look up a frozen cell by its canonical id.
    pub fn by_id(cell_id: &str) -> Option<&'static FrozenCell> {
        FROZEN_CELLS.iter().find(|c| c.cell_id == cell_id)
    }

    /// Self-consistency of the transcribed threshold table against the
    /// frozen formulas evaluated at H = H_max (#60 §9.5). Used by
    /// `--validate-only`: a mismatch here means this transcription is
    /// corrupt, and no treatment row may be produced from it.
    pub fn threshold_table_is_consistent(&self) -> bool {
        let h = self.h_max;
        let check = |ok: bool| ok;
        // Geometry derivations (#60 §4).
        check(self.m == self.n_bytes / 128)
            && check(self.target == self.m / 2)
            && check(self.edit_start == self.target * 128 + 63)
            && check(self.restart == (self.target - 1) * 128)
            && check(self.convergence_old == (self.target + 1) * 128)
            && check(self.convergence_new == self.convergence_old + 8)
            && check(self.replace_lo == self.target - 1)
            && check(self.replace_hi == self.target + 1)
            // Synchronized #60 §9.5 rows (corrected authority).
            && check(self.locate_visits_max == h)
            && check(self.safe_predecessor_visits_max == 3 * h - 2)
            && check(self.f1_visits_max == 4 * h - 2)
            && check(self.cursor_visits_max == 2 * h + 4 * CURSOR_ADVANCES + CANDIDATE_CHECKS)
            && check(self.fact_range_visits_max == h + DELTA_OLD * (h - 1))
            && check(self.split_visits_max == 20 * h)
            && check(self.pivot_visits_max == 8 * h - 2)
            && check(self.join_visits_max == 8 * h - 2)
            && check(self.f2_visits_max == 36 * h - 4)
            && check(self.rotations_max == 18 * h - 4)
            && check(self.link_writes_max == 72 * h - 3)
            && check(self.aggregate_reads_max == 406 * h - 34)
            && check(self.aggregate_writes_max == 168 * h - 24)
            && check(self.max_retirement_depth_max == h)
            // Shared constants.
            && check(RETIREMENT_FRAMES_MAX == 6)
            && check(OLD_FACT_OWNER_VISITS_MAX == 4)
            && check(FRESH_PAYLOAD_NODES_FINAL == 4)
    }
}

/// Construct the exact frozen pre source for a cell (readiness record
/// §4): `filler(126, b) + "\n\n"` per 128-byte block, in-process, no
/// external materialization.
pub fn frozen_pre_source(cell: &FrozenCell) -> String {
    let blocks = cell.n_bytes / 128;
    let mut pre = String::with_capacity(cell.n_bytes as usize);
    for b in 0..blocks {
        pre.push_str(&filler(126, b as usize));
        pre.push_str("\n\n");
    }
    pre
}

/// The frozen primary edit: a zero-length insertion of `"zzzzzzzz"` at
/// `target * 128 + 63` (readiness record §4).
pub fn frozen_edit(cell: &FrozenCell) -> CanonicalEdit {
    let start = cell.edit_start as usize;
    CanonicalEdit::new(start, start, INSERTED_TEXT).expect("frozen edit geometry")
}

/// sha256 of arbitrary bytes, lowercase hex (the row/provenance hashing
/// convention used throughout this producer).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_lower(&hasher.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// One constructed cell: the exact pre/post sources and edit, with their
/// observed byte identities (verified by the caller against the frozen
/// gates before any treatment work).
pub struct ConstructedCell {
    /// The pre source (`SourceId(1)`, matching the frozen conformance
    /// lane's identity assignment).
    pub pre: Source,
    /// The post source (`SourceId(2)`), exactly `edit.apply(pre)`.
    pub post: Source,
    /// The frozen primary edit.
    pub edit: CanonicalEdit,
}

/// Construct the cell and verify EVERY byte identity against the frozen
/// gates (#60 §3.1 canonical edit identity). Any mismatch is
/// `CELL_IDENTITY_FAILURE` — the caller must not run a decision-bearing
/// structural observation from it.
pub fn construct_and_verify(cell: &FrozenCell) -> Result<ConstructedCell, String> {
    let pre_source = frozen_pre_source(cell);
    if pre_source.len() as u64 != cell.n_bytes {
        return Err(format!(
            "{}: constructed pre length {} != frozen n_bytes {}",
            cell.cell_id,
            pre_source.len(),
            cell.n_bytes
        ));
    }
    let pre = Source::new(SourceId(1), pre_source);
    let pre_sha = pre.sha256_hex();
    if pre_sha != cell.pre_sha256 {
        return Err(format!(
            "{}: pre sha256 {pre_sha} != frozen {}",
            cell.cell_id, cell.pre_sha256
        ));
    }
    let inserted_sha = sha256_hex(INSERTED_TEXT.as_bytes());
    if inserted_sha != INSERTED_TEXT_SHA256 {
        return Err(format!(
            "{}: inserted-text sha256 {inserted_sha} != frozen {INSERTED_TEXT_SHA256}",
            cell.cell_id
        ));
    }
    let edit = frozen_edit(cell);
    let post = edit
        .apply(&pre, SourceId(2))
        .map_err(|e| format!("{}: frozen edit applies: {e}", cell.cell_id))?;
    let post_sha = post.sha256_hex();
    if post_sha != cell.post_sha256 {
        return Err(format!(
            "{}: post sha256 {post_sha} != frozen {}",
            cell.cell_id, cell.post_sha256
        ));
    }
    if (post.as_bytes().len() as u64) != cell.n_bytes + INSERTED_TEXT.len() as u64 {
        return Err(format!("{}: post length is not pre + 8", cell.cell_id));
    }
    Ok(ConstructedCell { pre, post, edit })
}
