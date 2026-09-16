//! Captures the R0 §4.1 build identity at compile time: runner commit,
//! rustc version, target triple, build profile id, Cargo.lock digest.
//! All values are best-effort with explicit `"unknown"` fallbacks and can
//! be overridden through `MDBENCH_*` environment variables for
//! replicated builds.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../Cargo.lock");

    // CARGO_MANIFEST_DIR is the runner crate; the workspace root is its
    // parent directory.
    let workspace_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");

    let commit = env::var("MDBENCH_RUNNER_GIT_COMMIT")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| git_head(&workspace_dir))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=MDBENCH_RUNNER_GIT_COMMIT={commit}");

    let rustc_version = env::var("MDBENCH_RUSTC_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(rustc_version)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=MDBENCH_RUSTC_VERSION={rustc_version}");

    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=MDBENCH_TARGET={target}");

    // release -> the frozen primary profile; debug builds are harness
    // development/testing builds and must never be labeled as research
    // measurements.
    let profile_id = match env::var("PROFILE").as_deref() {
        Ok("release") => "release-primary-v1",
        _ => "debug-non-research",
    };
    println!("cargo:rustc-env=MDBENCH_BUILD_PROFILE_ID={profile_id}");

    let lock_digest = fs::read(workspace_dir.join("Cargo.lock"))
        .map(|bytes| {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            to_hex(&hasher.finalize())
        })
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=MDBENCH_CARGO_LOCK_SHA256={lock_digest}");
}

fn git_head(workspace_dir: &std::path::Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn rustc_version() -> Option<String> {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
