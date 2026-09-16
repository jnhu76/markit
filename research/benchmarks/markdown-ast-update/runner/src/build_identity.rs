//! Build identity recorded in every result row (R0 §4.1, §13).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifier of the frozen primary release profile. The profile itself
/// lives in the workspace `Cargo.toml` and is documented in
/// `manifest/environment.toml` and `protocol/implementation-parity.md`.
pub const RELEASE_PRIMARY_PROFILE_ID: &str = "release-primary-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BuildIdentityV1 {
    pub runner_git_commit: String,
    pub rustc: String,
    pub target: String,
    pub build_profile_id: String,
    pub cargo_lock_sha256: String,
}

/// Capture the build identity baked in by the crate build script.
pub fn current_build_identity() -> BuildIdentityV1 {
    BuildIdentityV1 {
        runner_git_commit: env!("MDBENCH_RUNNER_GIT_COMMIT").to_string(),
        rustc: env!("MDBENCH_RUSTC_VERSION").to_string(),
        target: env!("MDBENCH_TARGET").to_string(),
        build_profile_id: env!("MDBENCH_BUILD_PROFILE_ID").to_string(),
        cargo_lock_sha256: env!("MDBENCH_CARGO_LOCK_SHA256").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_present_and_recorded() {
        let id = current_build_identity();
        assert!(!id.rustc.is_empty());
        assert!(!id.target.is_empty());
        // debug/test builds must never masquerade as the research profile.
        assert!(
            id.build_profile_id == RELEASE_PRIMARY_PROFILE_ID
                || id.build_profile_id == "debug-non-research"
        );
        assert_ne!(
            id.cargo_lock_sha256, "unknown",
            "Cargo.lock digest must be captured"
        );
        assert_eq!(id.cargo_lock_sha256.len(), 64);
        let json = serde_json::to_value(&id).unwrap();
        assert!(json["runner_git_commit"].is_string());
    }
}
