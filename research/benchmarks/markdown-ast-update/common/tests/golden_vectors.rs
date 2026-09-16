//! Golden reproducibility vectors for case identity
//! (R1-CORRECTIVE-1, IMPORTANT-2).
//!
//! These constants pin the exact `CaseKeyV1` canonical byte encoding and
//! the exact `CaseId` derivation. If an implementation change alters any
//! of them, that is a **case-identity break**: `CASE_KEY_VERSION_V1` and
//! `CASE_ID_ALGORITHM_ID` must change explicitly in the same commit.
//! Never silently regenerate these values.
//!
//! Vectors computed at R1-CORRECTIVE-1 (rustc 1.97.1, 2026-09-16).

use markit_mdbench_common::{to_lower_hex, CaseId, CaseKeyV1, OperationKind, PayloadShape};

/// The pinned sample key (intentionally NOT derived from the smoke
/// fixture, so fixture changes can never mask an encoding break).
fn pinned_key() -> CaseKeyV1 {
    CaseKeyV1 {
        payload_id: "R1_SMOKE_ONLY/fixture-1".to_string(),
        payload_shape: PayloadShape::Mixed,
        payload_size_bytes: 256,
        old_source_sha256: [7u8; 32],
        operation: OperationKind::Insert,
        edit_start_byte: Some(10),
        edit_end_byte: Some(10),
        inserted_text_sha256: Some([9u8; 32]),
        generator_id: Some("R1_SMOKE_ONLY".to_string()),
        generator_seed: None,
    }
}

/// GOLDEN: canonical encoding of `pinned_key()`.
const GOLDEN_CANONICAL_HEX: &str = "4d4b4d44422d434153454b45592d56310100011700000052315f534d4f4b455f4f4e4c592f666978747572652d3102050000006d6978656403080000000001000000000000042000000007070707070707070707070707070707070707070707070707070707070707070506000000696e736572740609000000010a000000000000000709000000010a0000000000000008200000000909090909090909090909090909090909090909090909090909090909090909090d00000052315f534d4f4b455f4f4e4c590a0100000000";

/// GOLDEN: `SHA256(GOLDEN_CANONICAL_HEX bytes)`, the CaseId of the key.
const GOLDEN_CASE_ID_HEX: &str = "a4f8dec99a35da3d588e2035859eb0b5a91517e993fcbbb1d81e6d708526613c";

#[test]
fn canonical_encoding_is_pinned_byte_for_byte() {
    let encoded = pinned_key().canonical_encode();
    let hex: String = encoded.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, GOLDEN_CANONICAL_HEX,
        "CaseKeyV1 canonical encoding drifted: a version identifier must \
         change explicitly with it"
    );
}

#[test]
fn case_id_derivation_is_pinned() {
    let id = CaseId::from_key(&pinned_key());
    assert_eq!(
        id.hex(),
        GOLDEN_CASE_ID_HEX,
        "CaseId derivation drifted: CASE_ID_ALGORITHM_ID must change \
         explicitly with it"
    );
    // Cross-check the derivation is exactly SHA256 of the pinned bytes.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let bytes: Vec<u8> = (0..GOLDEN_CANONICAL_HEX.len() / 2)
        .map(|i| u8::from_str_radix(&GOLDEN_CANONICAL_HEX[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    hasher.update(&bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    assert_eq!(id.hex(), to_lower_hex(&digest));
}

#[test]
fn changing_any_identity_field_changes_the_case_id() {
    let base = CaseId::from_key(&pinned_key());
    let mut key = pinned_key();
    key.payload_id = "R1_SMOKE_ONLY/fixture-2".to_string();
    assert_ne!(CaseId::from_key(&key), base);
    let mut key = pinned_key();
    key.generator_seed = Some(1);
    assert_ne!(CaseId::from_key(&key), base);
}
