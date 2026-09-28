//! The immutable raw-row writer (task §13; readiness record §8:
//! append-only raw evidence).
//!
//! A row already written for an identity may NOT be overwritten. The
//! writer creates the destination exclusively (`O_EXCL` semantics via
//! `create_new`): an existing path is a hard refusal — never truncate,
//! never replace, never rewrite. A failed or inconvenient row is
//! recorded, never deleted.

use std::fs;
use std::io;
use std::path::Path;

use super::schema::RawRowV1;

/// Why a row could not be written.
#[derive(Debug)]
pub enum WriteError {
    /// The destination already exists: duplicate-run refusal (fail
    /// closed).
    OutputExists(String),
    /// The parent directory is missing.
    NoParentDir(String),
    /// Serialization or I/O failure.
    Io(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::OutputExists(p) => write!(
                f,
                "raw-row output already exists (duplicate identity refusal): {p}"
            ),
            WriteError::NoParentDir(p) => write!(f, "raw-row output parent directory missing: {p}"),
            WriteError::Io(detail) => write!(f, "raw-row write failed: {detail}"),
        }
    }
}

impl std::error::Error for WriteError {}

/// Check output-path legality BEFORE treatment (`--validate-only`
/// obligation): the destination must not exist and its parent directory
/// must.
pub fn check_output_path(path: &Path) -> Result<(), WriteError> {
    if path.exists() {
        return Err(WriteError::OutputExists(path.display().to_string()));
    }
    match path.parent() {
        Some(parent) if parent.as_os_str().is_empty() => Ok(()),
        Some(parent) if parent.is_dir() => Ok(()),
        Some(parent) => Err(WriteError::NoParentDir(parent.display().to_string())),
        None => Ok(()),
    }
}

/// Write exactly one immutable raw row. The file is created exclusively;
/// fsync makes the archived evidence durable before the process reports
/// success.
pub fn write_row(path: &Path, row: &RawRowV1) -> Result<(), WriteError> {
    check_output_path(path)?;
    let mut file = fs::File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                WriteError::OutputExists(path.display().to_string())
            } else {
                WriteError::Io(format!("{}: {e}", path.display()))
            }
        })?;
    serde_json::to_writer_pretty(&mut file, row)
        .map_err(|e| WriteError::Io(format!("serialize row: {e}")))?;
    file.sync_data()
        .map_err(|e| WriteError::Io(format!("fsync {}: {e}", path.display())))?;
    Ok(())
}
