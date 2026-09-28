//! Static provenance capture (readiness record §8): everything a raw row
//! must carry about the executable, the toolchain and the host, captured
//! at process start before any treatment work.
//!
//! Capture is deliberately fail-closed: a missing commit/tree,
//! executable hash, rustc/cargo version, kernel or CPU model is a
//! provenance capture failure, and a row without complete provenance is
//! INVALID — it is never written as decision-bearing evidence.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::contract;
use super::schema::HostBlock;

/// The frozen build profile recorded in every row (readiness record §8:
/// `release-primary-v1` — the workspace `[profile.release]`; the debug
/// conformance builds record `debug`).
pub fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release-primary-v1"
    }
}

/// The frozen feature set (readiness record §8: FEATURES = none).
pub const FEATURES: &str = "none";

/// The captured static provenance of one producer process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub repository_commit: String,
    pub repository_tree: String,
    pub executable_path: String,
    pub executable_sha256: String,
    pub rustc: String,
    pub cargo: String,
    pub toolchain_channel: String,
    pub build_profile: String,
    pub features: String,
    pub host: HostBlock,
}

/// Why provenance capture failed (fail-closed; never an excuse to omit
/// fields from a decision-bearing row).
#[derive(Debug)]
pub struct ProvenanceError(pub String);

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "provenance capture failure: {}", self.0)
    }
}

impl std::error::Error for ProvenanceError {}

fn run_trimmed(
    program: &str,
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> Result<String, ProvenanceError> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| ProvenanceError(format!("running `{program} {}`: {e}", args.join(" "))))?;
    if !output.status.success() {
        return Err(ProvenanceError(format!(
            "`{program} {}` exited with {:?}",
            args.join(" "),
            output.status.code()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| ProvenanceError(format!("`{program}` output is not UTF-8: {e}")))
}

/// `git rev-parse HEAD` + `HEAD^{tree}` in the current directory, plus
/// the repository toplevel.
fn git_identity() -> Result<(String, String, PathBuf), ProvenanceError> {
    let commit = run_trimmed("git", &["rev-parse", "HEAD"], None)?;
    let tree = run_trimmed("git", &["rev-parse", "HEAD^{tree}"], None)?;
    let toplevel = run_trimmed("git", &["rev-parse", "--show-toplevel"], None)?;
    Ok((commit, tree, PathBuf::from(toplevel)))
}

/// The toolchain channel pinned by the workspace `rust-toolchain.toml`
/// (readiness record §8: pin 1.97.1).
fn toolchain_channel(toplevel: &std::path::Path) -> Result<String, ProvenanceError> {
    let path = toplevel.join("research/benchmarks/markdown-ast-update/rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| ProvenanceError(format!("reading {}: {e}", path.display())))?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("channel") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Ok(value.to_string());
                }
            }
        }
    }
    Err(ProvenanceError(format!(
        "no channel pin found in {}",
        path.display()
    )))
}

/// The kernel release (Linux: /proc/sys/kernel/osrelease).
fn kernel_release() -> Result<String, ProvenanceError> {
    if let Ok(text) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    run_trimmed("uname", &["-r"], None)
}

/// The CPU model (Linux: first `model name` in /proc/cpuinfo).
fn cpu_model() -> Result<String, ProvenanceError> {
    if let Ok(text) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("model name") {
                if let Some(value) = rest.split_once(':').map(|(_, v)| v.trim()) {
                    if !value.is_empty() {
                        return Ok(value.to_string());
                    }
                }
            }
        }
    }
    Err(ProvenanceError(
        "CPU model not found (/proc/cpuinfo has no model name)".to_string(),
    ))
}

/// sha256 of the currently running executable.
fn executable_sha256() -> Result<(String, String), ProvenanceError> {
    let path = std::env::current_exe()
        .map_err(|e| ProvenanceError(format!("locating the running executable: {e}")))?;
    let bytes = std::fs::read(&path)
        .map_err(|e| ProvenanceError(format!("reading {}: {e}", path.display())))?;
    Ok((path.display().to_string(), contract::sha256_hex(&bytes)))
}

/// Capture the complete static provenance (fail-closed).
pub fn capture() -> Result<Provenance, ProvenanceError> {
    let (commit, tree, toplevel) = git_identity()?;
    if commit.len() != 40 || tree.len() != 40 {
        return Err(ProvenanceError(format!(
            "git identity is not a 40-hex SHA pair: {commit} / {tree}"
        )));
    }
    let (executable_path, executable_sha) = executable_sha256()?;
    let rustc = run_trimmed("rustc", &["--version"], None)?;
    let cargo = run_trimmed("cargo", &["--version"], None)?;
    let channel = toolchain_channel(&toplevel)?;
    Ok(Provenance {
        repository_commit: commit,
        repository_tree: tree,
        executable_path,
        executable_sha256: executable_sha,
        rustc,
        cargo,
        toolchain_channel: channel,
        build_profile: build_profile().to_string(),
        features: FEATURES.to_string(),
        host: HostBlock {
            os: std::env::consts::OS.to_string(),
            kernel: kernel_release()?,
            arch: std::env::consts::ARCH.to_string(),
            cpu_model: cpu_model()?,
        },
    })
}
