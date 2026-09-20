//! Generated, committed contract artifacts (CORRECTIVE-A §O).
//!
//! The Rust models are the single schema authority. This module renders
//! them; the `mdbench-gen-semantic-schema` binary writes them, and a test
//! re-renders them and compares byte-for-byte with the committed files so a
//! model change cannot silently diverge from the reviewed contract.

use std::path::Path;

use crate::canonical::canonical_json_line;
use crate::lanes::lane_registry_v1;
use crate::pilot::{
    load_fixtures, real_source_spec, run_pilot, PilotManifest, PILOT_MANIFEST_SCHEMA, PILOT_SET_ID,
};
use crate::payload::PayloadRecord;
use crate::profile::Profile;
use crate::transition::TransitionReport;

/// `(path relative to the benchmark root, file contents)` pairs.
pub fn schema_artifacts() -> Vec<(&'static str, String)> {
    vec![
        (
            "grammar/grammar-lanes-v1.json",
            canonical_json_line(&lane_registry_v1()),
        ),
        (
            "workloads/profiles/schema/real-profile-v1.schema.json",
            schema_text::<Profile>(),
        ),
        (
            "workloads/payloads/schema/real-payload-v1.schema.json",
            schema_text::<PayloadRecord>(),
        ),
        (
            "workloads/payloads/schema/transition-oracle-v1.schema.json",
            schema_text::<TransitionReport>(),
        ),
    ]
}

/// The two SEMANTIC_PILOT_SET artifacts, rendered from the same models the
/// driver uses, so a stale committed pilot output cannot go unnoticed.
pub fn pilot_artifacts(bench_root: &Path) -> Result<Vec<(&'static str, String)>, String> {
    let fixtures = load_fixtures(bench_root)?;
    let real_source = real_source_spec(bench_root);
    let results = run_pilot(bench_root, &fixtures, real_source.as_ref());
    let manifest = PilotManifest {
        schema: PILOT_MANIFEST_SCHEMA.to_string(),
        pilot_set_id: PILOT_SET_ID.to_string(),
        profiler_version: crate::facts::PROFILER_VERSION.to_string(),
        lane_registry_version: crate::lanes::LANE_REGISTRY_VERSION.to_string(),
        transition_oracle_version: crate::transition::TRANSITION_ORACLE_VERSION.to_string(),
        payload_lifecycle_version: crate::payload::PAYLOAD_LIFECYCLE_VERSION.to_string(),
        notice: PILOT_NOTICE.to_string(),
        fixtures,
        real_source,
    };
    Ok(vec![
        (
            "workloads/pilots/semantic-pilot-manifest-v1.json",
            canonical_json_line(&manifest),
        ),
        (
            "workloads/pilots/semantic-pilot-results-v1.json",
            canonical_json_line(&results),
        ),
    ])
}

/// The notice carried by every pilot artifact.
pub const PILOT_NOTICE: &str =
    "SEMANTIC_PILOT_SET is a contract-validation pilot, not a workload: it is not \
     the representative set, the extremal set, the syntax-coverage set or the \
     full-document set, and no representativeness claim may be derived from it.";

fn schema_text<T: schemars::JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    let mut text = serde_json::to_string_pretty(&schema).expect("schema serializes");
    text.push('\n');
    text
}
