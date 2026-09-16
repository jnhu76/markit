//! JSONL emission for raw result rows.

use std::io;
use std::io::Write;

use super::result::ResultRowV1;

/// Write one row as a single JSON line + `\n`.
pub fn write_row<W: Write>(writer: &mut W, row: &ResultRowV1) -> io::Result<()> {
    let line =
        serde_json::to_string(row).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::{BuildIdentitySlot, EditMetaV1, MeasurementV1, PayloadMetaV1, ResultRowV1};
    use markit_mdbench_common::CorrectnessStatus;
    use markit_mdbench_common::ExecutionStatus;
    use markit_mdbench_common::OperationKind;
    use markit_mdbench_common::PayloadShape;

    fn sample_row() -> ResultRowV1 {
        ResultRowV1 {
            schema_version: crate::RESULT_SCHEMA_VERSION_V1,
            protocol_version: crate::PROTOCOL_VERSION.to_string(),
            build_identity: BuildIdentitySlot {
                runner_git_commit: "test".to_string(),
                rustc: "test".to_string(),
                target: "test".to_string(),
                build_profile_id: "test".to_string(),
                cargo_lock_sha256: "test".to_string(),
            },
            case_id: "ab".repeat(32),
            seed: 1,
            mechanism_id: "__r1_null__".to_string(),
            operation: OperationKind::Insert,
            payload: PayloadMetaV1 {
                payload_id: "R1_SMOKE_ONLY/fixture-1".to_string(),
                shape: PayloadShape::Mixed,
                size_bytes: 10,
            },
            edit: EditMetaV1 {
                start_byte: Some(1),
                end_byte: Some(1),
                inserted_sha256: None,
            },
            execution_status: ExecutionStatus::Pass,
            correctness_status: CorrectnessStatus::Pass,
            result_checksum: Some("deadbeef".to_string()),
            environment_ref: "manifest/environment.toml#test".to_string(),
            provenance_ref: "R1_SMOKE_ONLY/NON_RESEARCH_RESULT".to_string(),
            measurement: MeasurementV1::Attribution(
                markit_mdbench_common::WorkCounters::all_unknown(),
            ),
        }
    }

    #[test]
    fn row_is_one_json_line() {
        let mut buf: Vec<u8> = Vec::new();
        write_row(&mut buf, &sample_row()).expect("write");
        write_row(&mut buf, &sample_row()).expect("write");
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let parsed: ResultRowV1 = serde_json::from_str(line).expect("valid json line");
            assert_eq!(parsed, sample_row());
        }
    }
}
