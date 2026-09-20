//! CORRECTIVE-A required tests — contracts, lane identity, artifact drift,
//! and the frozen synthetic identity boundary.
//!
//! Test families covered here: 1 (lane identities), 12 (existing #22 CaseId
//! semantics unchanged), plus the artifact-drift guard that keeps the
//! committed JSON contracts equal to the reviewed Rust models.

use std::fs;
use std::path::{Path, PathBuf};

use markit_mdbench_common::{to_lower_hex, CaseId, CaseKeyV1, OperationKind, PayloadShape};
use markit_mdbench_semantics::artifacts::{pilot_artifacts, schema_artifacts};
use markit_mdbench_semantics::lanes::{
    lane_registry_v1, lane_spec, ConstructStatus, HorseId, HorseStatus, SemanticStatus,
    G0_GRAMMAR_ID, G1_BASE_SPEC_VERSION, G1_EXTENSION_SPEC_VERSION, G1_GRAMMAR_ID, G2_GRAMMAR_ID,
};
use markit_mdbench_semantics::{SyntaxKind, PAYLOAD_ID_NAMESPACE, PROFILER_VERSION};

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

/// Family 1 — lane identities are stable and versioned.
#[test]
fn lane_identities_are_stable_and_versioned() {
    let registry = lane_registry_v1();
    assert_eq!(registry.registry_version, "GRAMMAR-LANES-v1");
    assert_eq!(registry.lanes.len(), 3);

    let g0 = lane_spec(G0_GRAMMAR_ID).expect("G0 lane");
    assert_eq!(g0.lane_id, "G0");
    assert_eq!(g0.lane_version, "v1");

    let g1 = lane_spec(G1_GRAMMAR_ID).expect("G1 lane");
    assert_eq!(g1.lane_id, "G1");
    // The executable identity names the exact base version and extension
    // version; "CommonMark/GFM" as a vague label would be a contract break.
    assert!(g1.grammar_id.contains(G1_BASE_SPEC_VERSION));
    assert!(g1.grammar_id.contains(G1_EXTENSION_SPEC_VERSION));
    assert_eq!(g1.extensions_enabled.len(), 1);
    assert!(g1.extensions_enabled[0].name.contains("tables"));
    assert!(g1
        .extensions_disabled
        .iter()
        .any(|extension| extension.contains("strikethrough")));
    let oracle = g1.reference_oracle.as_ref().expect("G1 oracle");
    assert_eq!(oracle.exact_crate_version.as_deref(), Some("0.13.4"));
    assert!(oracle.configuration.contains("ENABLE_TABLES"));

    let g2 = lane_spec(G2_GRAMMAR_ID).expect("G2 lane");
    assert_eq!(g2.semantic_status, SemanticStatus::Deferred);
    assert!(g2.reference_oracle.is_none());
}

/// Family 1 (continued) — semantic qualification is separate from horse
/// qualification, in both directions.
#[test]
fn semantic_and_horse_qualification_are_independent() {
    let g0 = lane_spec(G0_GRAMMAR_ID).expect("G0 lane");
    assert_eq!(g0.semantic_status, SemanticStatus::Qualified);
    for horse in HorseId::all() {
        assert_eq!(g0.horse_status(horse), HorseStatus::Qualified);
    }

    let g1 = lane_spec(G1_GRAMMAR_ID).expect("G1 lane");
    assert_eq!(g1.semantic_status, SemanticStatus::Qualified);
    for horse in HorseId::all() {
        // A frozen semantic contract does not imply any horse implements it.
        assert_eq!(g1.horse_status(horse), HorseStatus::NotImplemented);
    }

    // The G1 pilot scope is the table construct only.
    assert_eq!(
        g1.construct_status(SyntaxKind::Table),
        ConstructStatus::Frozen
    );
    assert_eq!(
        g1.construct_status(SyntaxKind::Paragraph),
        ConstructStatus::DeclaredNotQualified
    );
    assert_eq!(
        g1.construct_status(SyntaxKind::InlineMath),
        ConstructStatus::Deferred
    );
    // G0 keeps its own construct set: tables are not G0 constructs.
    assert_eq!(
        g0.construct_status(SyntaxKind::Table),
        ConstructStatus::OutOfLane
    );
}

/// The committed contract artifacts must equal the reviewed Rust models.
#[test]
fn generated_artifacts_match_the_models() {
    let root = bench_root();
    for (relative, expected) in schema_artifacts() {
        let path = root.join(relative);
        let on_disk = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            on_disk, expected,
            "{relative} drifted from the Rust model; re-run \
             `cargo run -p markit-mdbench-semantics --bin mdbench-gen-semantic-schema`"
        );
    }
}

/// The committed pilot artifacts must equal what the driver produces now.
///
/// Without this guard a model change could leave the reviewed pilot outputs
/// stale: the pilot binary writes them, but nothing would notice a forgotten
/// `--write`.
#[test]
fn committed_pilot_artifacts_match_the_driver() {
    let root = bench_root();
    let artifacts = pilot_artifacts(&root).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(artifacts.len(), 2, "manifest and results are both published");
    for (relative, expected) in artifacts {
        let path = root.join(relative);
        let on_disk = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            on_disk, expected,
            "{relative} is stale; re-run \
             `cargo run -p markit-mdbench-semantics --bin mdbench-semantic-pilot -- --write`"
        );
    }
}

/// Family 12 — the frozen #22 synthetic identity must not move.
///
/// The golden values are copied from `common/tests/golden_vectors.rs`
/// (R1-CORRECTIVE-1, rustc 1.97.1). If this test fails, the synthetic
/// identity space was perturbed by CORRECTIVE-A work, which is a stop
/// condition, not a value to regenerate.
#[test]
fn synthetic_case_identity_is_unchanged() {
    const GOLDEN_CANONICAL_HEX: &str = "4d4b4d44422d434153454b45592d56310100011700000052315f534d4f4b455f4f4e4c592f666978747572652d3102050000006d6978656403080000000001000000000000042000000007070707070707070707070707070707070707070707070707070707070707070506000000696e736572740609000000010a000000000000000709000000010a0000000000000008200000000909090909090909090909090909090909090909090909090909090909090909090d00000052315f534d4f4b455f4f4e4c590a0100000000";
    const GOLDEN_CASE_ID_HEX: &str =
        "a4f8dec99a35da3d588e2035859eb0b5a91517e993fcbbb1d81e6d708526613c";

    let key = CaseKeyV1 {
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
    };
    let encoded: String = key
        .canonical_encode()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(encoded, GOLDEN_CANONICAL_HEX);
    assert_eq!(CaseId::from_key(&key).hex(), GOLDEN_CASE_ID_HEX);
    assert_eq!(to_lower_hex(&[0xab, 0xcd]), "abcd");

    // Real payload identities live in their own namespace and are not
    // 64-hex CaseIds.
    assert_eq!(PAYLOAD_ID_NAMESPACE, "rp1:");
    assert!(PAYLOAD_ID_NAMESPACE.len() + 32 != 64);
}

/// The profiler identity is versioned and frozen.
#[test]
fn profiler_identity_is_versioned() {
    assert_eq!(PROFILER_VERSION, "REAL-MARKDOWN-PROFILER-v1");
    let profile =
        markit_mdbench_semantics::profile("x\n", &[G1_GRAMMAR_ID, G0_GRAMMAR_ID]);
    assert_eq!(profile.profiler_version, PROFILER_VERSION);
    assert_eq!(profile.schema, "real-profile-v1");
    assert_eq!(profile.source.source_sha256.len(), 64);
    assert_eq!(profile.source.file_bytes, 2);
    for lane in &profile.lanes {
        assert_eq!(lane.lane_version, "v1");
    }
    // Every lane record carries its frozen oracle configuration.
    assert!(profile.lanes[0].configuration.contains("ENABLE_TABLES"));
}

/// The semantics crate must stay measurement-free: no mechanism crate may
/// appear in its dependency list, or it could observe horse behavior.
#[test]
fn semantics_crate_has_no_mechanism_dependency() {
    let manifest = fs::read_to_string(bench_root().join("semantics/Cargo.toml")).expect("manifest");
    for forbidden in [
        "markit-mdbench-full-rebuild",
        "markit-mdbench-block-local",
        "markit-mdbench-fragment-reuse",
        "markit-mdbench-old-tree-subtree-reuse",
        "markit-mdbench-restart-convergence",
        "markit-mdbench-instrumentation",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "semantics crate must not depend on {forbidden}"
        );
    }
}

/// The published artifacts are properties of the bytes and the contract,
/// never of the machine that produced them. An absolute path in a committed
/// artifact would also make the drift guard above pass only in the author's
/// checkout.
#[test]
fn published_artifacts_carry_no_machine_paths() {
    let root = bench_root();
    let artifacts = pilot_artifacts(&root).unwrap_or_else(|error| panic!("{error}"));
    for (relative, text) in artifacts {
        for needle in [
            root.to_string_lossy().as_ref(),
            "/home/",
            "/Users/",
            "/tmp/",
        ] {
            assert!(
                !text.contains(needle),
                "{relative} contains a machine path ({needle})"
            );
        }
    }
}
