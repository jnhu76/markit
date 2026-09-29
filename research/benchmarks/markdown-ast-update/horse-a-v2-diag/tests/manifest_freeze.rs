//! #98 frozen-manifest invariants the correctness oracle does not
//! directly expose: the cell manifest is deterministic (byte-identical
//! receipt on every construction) and every cell's edit is associable
//! with its pre source (the precondition the 0C gates rely on).

use markit_mdbench_common::Source;
use markit_mdbench_horse_a_v2_diag::cells::{frozen_cells, manifest_receipt};

#[test]
fn manifest_receipt_is_deterministic() {
    let a = serde_json::to_string(&manifest_receipt()).unwrap();
    let b = serde_json::to_string(&manifest_receipt()).unwrap();
    assert_eq!(a, b, "the frozen #98 manifest must construct identically");
}

#[test]
fn every_cell_edit_validates_against_its_pre_source() {
    for cell in frozen_cells() {
        let pre = Source::new(markit_mdbench_common::SourceId(0), cell.pre_source.clone());
        let edit = cell.edit();
        edit.validate_against(&pre)
            .unwrap_or_else(|e| panic!("cell {}: edit does not associate: {e}", cell.id));
        let post = edit
            .apply(&pre, markit_mdbench_common::SourceId(1))
            .unwrap_or_else(|e| panic!("cell {}: apply failed: {e}", cell.id));
        assert_eq!(
            post.len_bytes() as u64,
            pre.len_bytes() as u64 + edit.inserted_text_len_bytes() - edit.removed_len_bytes(),
            "cell {}: length arithmetic",
            cell.id
        );
    }
}

#[test]
fn cell_ids_are_unique_and_roles_are_consistent() {
    let cells = frozen_cells();
    let mut ids: Vec<&str> = cells.iter().map(|c| c.id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "cell IDs must be unique");
    for cell in &cells {
        match cell.role {
            r if r.starts_with("E6_CHALLENGE") => {
                assert_eq!(cell.arms.len(), 4, "E6 cells run A/B/C/D");
            }
            r if r.starts_with("TINY") => {
                assert_eq!(cell.arms.len(), 3, "tiny cells run A/B/C");
            }
            r if r.starts_with("SENTINEL") => {
                assert_eq!(cell.arms.len(), 2, "sentinels run A/B");
            }
            other => panic!("cell {}: unknown role prefix: {other}", cell.id),
        }
    }
}
