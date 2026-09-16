//! Versioned case identity: `CaseKeyV1` -> canonical bytes -> SHA256.
//!
//! Guarantees required by the R1 contract:
//!
//! - identity is logical (payload + content digests + operation + edit
//!   facts), never runtime identity (no HashMap order, no `DefaultHasher`,
//!   no UUID/timestamp/PID/pointer/iteration index);
//! - identity contains NO mechanism id and NO lane, so the same logical
//!   case keeps the same `CaseId` across horses and lanes;
//! - encoding is explicit, little-endian, length-prefixed and versioned —
//!   not a serde memory representation.
//!
//! `CASE_ID_ALGORITHM_ID = "sha256-of-casekey-v1"` names the derivation.

use sha2::{Digest, Sha256};

use crate::edit::OperationKind;
use crate::payload::PayloadShape;
use crate::source::to_lower_hex;

/// Version of the CaseKey logical form implemented by this crate.
pub const CASE_KEY_VERSION_V1: u16 = 1;

/// Run/case-order seed. Pure identity for deterministic shuffling; never
/// part of a [`CaseId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Seed(pub u64);

/// Identifier of the CaseId derivation algorithm.
pub const CASE_ID_ALGORITHM_ID: &str = "sha256-of-casekey-v1";

const DOMAIN: &[u8] = b"MKMDB-CASEKEY-V1";

// Field tags for the canonical encoding. Fixed order; never renumber.
mod tag {
    pub const PAYLOAD_ID: u8 = 1;
    pub const PAYLOAD_SHAPE: u8 = 2;
    pub const PAYLOAD_SIZE: u8 = 3;
    pub const OLD_SOURCE_SHA256: u8 = 4;
    pub const OPERATION: u8 = 5;
    pub const EDIT_START: u8 = 6;
    pub const EDIT_END: u8 = 7;
    pub const INSERTED_SHA256: u8 = 8;
    pub const GENERATOR_ID: u8 = 9;
    pub const GENERATOR_SEED: u8 = 10;
}

/// Everything needed to identify the exact logical input of a case,
/// independent of mechanism, lane, run order and runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaseKeyV1 {
    pub payload_id: String,
    pub payload_shape: PayloadShape,
    pub payload_size_bytes: u64,
    /// SHA256 of the exact old-source bytes.
    pub old_source_sha256: [u8; 32],
    pub operation: OperationKind,
    pub edit_start_byte: Option<u64>,
    pub edit_end_byte: Option<u64>,
    /// SHA256 of the exact inserted UTF-8 bytes (`Some` whenever the
    /// operation carries inserted text, including empty insertions).
    pub inserted_text_sha256: Option<[u8; 32]>,
    /// Generator identity where applicable (e.g. `"R1_SMOKE_ONLY"`).
    pub generator_id: Option<String>,
    /// Generator seed where applicable.
    pub generator_seed: Option<u64>,
}

/// Case-key consistency problems (wrong option pattern for the operation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseKeyError {
    /// Operation carries no edit but an edit field is present.
    UnexpectedEdit,
    /// Operation carries an edit but an edit field is missing.
    MissingEdit,
    /// Operation carries inserted text but no inserted-text digest.
    MissingInsertedDigest,
    /// Operation carries no inserted text but a digest is present.
    UnexpectedInsertedDigest,
    /// `edit_start > edit_end`.
    StartAfterEnd,
    /// Required identity string is empty.
    EmptyIdentity,
}

impl core::fmt::Display for CaseKeyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CaseKeyError::UnexpectedEdit => {
                f.write_str("operation carries no edit, but edit fields present")
            }
            CaseKeyError::MissingEdit => {
                f.write_str("operation carries an edit, but edit fields missing")
            }
            CaseKeyError::MissingInsertedDigest => {
                f.write_str("inserted-text digest required for this operation")
            }
            CaseKeyError::UnexpectedInsertedDigest => {
                f.write_str("inserted-text digest present for an operation without insertion")
            }
            CaseKeyError::StartAfterEnd => f.write_str("edit start > end"),
            CaseKeyError::EmptyIdentity => f.write_str("payload_id must be non-empty"),
        }
    }
}

impl std::error::Error for CaseKeyError {}

impl CaseKeyV1 {
    /// Consume a literally-constructed key, validating the option pattern
    /// against the operation. The inserted-text digest is supplied by the
    /// caller (usually computed from the exact inserted bytes) and lives
    /// with the key, so key equality is complete.
    pub fn validated(self) -> Result<Self, CaseKeyError> {
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), CaseKeyError> {
        if self.payload_id.is_empty() {
            return Err(CaseKeyError::EmptyIdentity);
        }
        let expect_edit = self.operation.has_edit();
        match (expect_edit, self.edit_start_byte, self.edit_end_byte) {
            (false, None, None) => {}
            (false, _, _) => return Err(CaseKeyError::UnexpectedEdit),
            (true, None, _) | (true, _, None) => return Err(CaseKeyError::MissingEdit),
            (true, Some(s), Some(e)) if s <= e => {}
            (true, Some(_), Some(_)) => return Err(CaseKeyError::StartAfterEnd),
        }
        match (
            self.operation.has_inserted_text(),
            self.inserted_text_sha256,
        ) {
            (true, Some(_)) | (false, None) => {}
            (true, None) => return Err(CaseKeyError::MissingInsertedDigest),
            (false, Some(_)) => return Err(CaseKeyError::UnexpectedInsertedDigest),
        }
        Ok(())
    }

    /// Explicit, versioned canonical byte encoding.
    ///
    /// Format:
    /// `MKMDB-CASEKEY-V1` || u16le version || fields in fixed tag order,
    /// each field = tag byte || u32le length || payload. Optional scalars
    /// encode presence as a leading 0x00/0x01 byte.
    pub fn canonical_encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(DOMAIN);
        out.extend_from_slice(&CASE_KEY_VERSION_V1.to_le_bytes());

        put_field(&mut out, tag::PAYLOAD_ID, self.payload_id.as_bytes());
        put_field(
            &mut out,
            tag::PAYLOAD_SHAPE,
            self.payload_shape.canonical_name().as_bytes(),
        );
        put_field(
            &mut out,
            tag::PAYLOAD_SIZE,
            &self.payload_size_bytes.to_le_bytes(),
        );
        put_field(&mut out, tag::OLD_SOURCE_SHA256, &self.old_source_sha256);
        put_field(
            &mut out,
            tag::OPERATION,
            self.operation.canonical_name().as_bytes(),
        );
        put_opt_u64(&mut out, tag::EDIT_START, self.edit_start_byte);
        put_opt_u64(&mut out, tag::EDIT_END, self.edit_end_byte);
        match self.inserted_text_sha256 {
            Some(d) => put_field(&mut out, tag::INSERTED_SHA256, &d),
            None => put_field(&mut out, tag::INSERTED_SHA256, &[]),
        }
        match &self.generator_id {
            Some(g) => put_field(&mut out, tag::GENERATOR_ID, g.as_bytes()),
            None => put_field(&mut out, tag::GENERATOR_ID, &[]),
        }
        put_opt_u64(&mut out, tag::GENERATOR_SEED, self.generator_seed);
        out
    }
}

fn put_field(out: &mut Vec<u8>, field_tag: u8, payload: &[u8]) {
    out.push(field_tag);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

fn put_opt_u64(out: &mut Vec<u8>, field_tag: u8, value: Option<u64>) {
    match value {
        Some(v) => {
            let mut buf = [0u8; 9];
            buf[0] = 0x01;
            buf[1..].copy_from_slice(&v.to_le_bytes());
            put_field(out, field_tag, &buf);
        }
        None => put_field(out, field_tag, &[0x00]),
    }
}

/// Case identity: `SHA256(canonical_encode(CaseKeyV1))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaseId([u8; 32]);

impl CaseId {
    pub fn from_key(key: &CaseKeyV1) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key.canonical_encode());
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn hex(&self) -> String {
        to_lower_hex(&self.0)
    }
}

impl core::fmt::Display for CaseId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key() -> CaseKeyV1 {
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

    #[test]
    fn validation_enforces_option_patterns() {
        assert_eq!(sample_key().validate(), Ok(()));

        let mut k = sample_key();
        k.operation = OperationKind::FullParse;
        assert_eq!(k.validate(), Err(CaseKeyError::UnexpectedEdit));

        let mut k = sample_key();
        k.operation = OperationKind::Delete;
        assert_eq!(k.validate(), Err(CaseKeyError::UnexpectedInsertedDigest));

        let mut k = sample_key();
        k.edit_start_byte = None;
        assert_eq!(k.validate(), Err(CaseKeyError::MissingEdit));

        let mut k = sample_key();
        k.edit_start_byte = Some(10);
        k.edit_end_byte = Some(9);
        assert_eq!(k.validate(), Err(CaseKeyError::StartAfterEnd));

        let mut k = sample_key();
        k.payload_id = String::new();
        assert_eq!(k.validate(), Err(CaseKeyError::EmptyIdentity));
    }

    #[test]
    fn encoding_is_stable_and_explicit() {
        let key = sample_key();
        let a = key.canonical_encode();
        let b = key.canonical_encode();
        assert_eq!(a, b);
        assert!(a.starts_with(DOMAIN));
        // Length prefix protects against ambiguity between fields.
        assert_eq!(&a[16..18], &CASE_KEY_VERSION_V1.to_le_bytes());
    }

    #[test]
    fn case_id_is_content_identity_not_runtime_identity() {
        let key = sample_key();
        let id1 = CaseId::from_key(&key);
        let id2 = CaseId::from_key(&key);
        assert_eq!(id1, id2);
        assert_eq!(id1.hex().len(), 64);

        let mut other = key.clone();
        other.payload_id = "other".to_string();
        assert_ne!(CaseId::from_key(&other), id1);

        // Identity ignores mechanism/lane by construction: those are not
        // fields of CaseKeyV1 at all, and the canonical encoding never
        // mentions them.
        let encoded = key.canonical_encode();
        assert!(!encoded.windows(9).any(|w| w == b"mechanism"));
        assert!(!encoded.windows(4).any(|w| w == b"lane"));
    }

    #[test]
    fn full_parse_key_has_no_edit_fields() {
        let key = CaseKeyV1 {
            payload_id: "R1_SMOKE_ONLY/fixture-1".to_string(),
            payload_shape: PayloadShape::Mixed,
            payload_size_bytes: 256,
            old_source_sha256: [7u8; 32],
            operation: OperationKind::FullParse,
            edit_start_byte: None,
            edit_end_byte: None,
            inserted_text_sha256: None,
            generator_id: Some("R1_SMOKE_ONLY".to_string()),
            generator_seed: None,
        }
        .validated()
        .expect("valid");
        let encoded = key.canonical_encode();
        // EDIT_START tag present with absent marker, no u64 payload.
        let pos = encoded
            .iter()
            .position(|&b| b == tag::EDIT_START)
            .expect("tag");
        assert_eq!(encoded[pos + 1..pos + 5], 1u32.to_le_bytes());
        assert_eq!(encoded[pos + 5], 0x00);
    }
}
