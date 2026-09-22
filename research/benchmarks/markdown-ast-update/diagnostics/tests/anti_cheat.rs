//! Negative anti-cheat guards for the #22 correctness closure (task §22).
//!
//! These are STATIC guards over the repository tree: they fail if the
//! horse implementations, or the dependency graph around them, start to
//! depend on benchmark identity instead of on source/edit/state. They
//! read files only; they never run a mechanism and never measure one.
//!
//! The guards exist because the closure repair touched reuse POLICY: the
//! cheapest wrong fix would have been to special-case the failing
//! payloads, the failing transitions, or the failing source paths, or to
//! obtain correctness by substituting H0's output. None of that is
//! detectable by a correctness matrix, so it is guarded structurally.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("benchmark root")
        .to_path_buf()
}

/// The five horse crates. `null-r1` is the R1 null mechanism (a fixture
/// generator, not a horse) and is deliberately out of scope.
const HORSE_CRATES: &[&str] = &[
    "mechanisms/full-rebuild",
    "mechanisms/block-local",
    "mechanisms/fragment-reuse",
    "mechanisms/old-tree-subtree-reuse",
    "mechanisms/restart-convergence",
];

/// Every `.rs` file under a crate's `src/`, as (path, text).
fn crate_sources(crate_dir: &str) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).expect("mechanism source readable");
                out.push((path, text));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join(crate_dir).join("src"), &mut out);
    assert!(!out.is_empty(), "{crate_dir}/src has no sources");
    out
}

/// Tokens that would mean production mechanism behaviour depends on
/// benchmark identity rather than on source/edit/state.
const FORBIDDEN_TOKENS: &[&str] = &[
    // Payload identity.
    "rp1:",
    "payload_id",
    "PayloadRecord",
    "expected_transition",
    // Transition identity.
    "G0-FENCE",
    "G0-REFDEF",
    "G0-LOCAL",
    "G0-PARAGRAPH",
    "G0-LIST",
    "G0-CODESPAN",
    "G0-LINK",
    "G0-ATX",
    "G0-EMPH",
    "G0-BQ",
    "transition_id",
    // Workload membership / freeze artifacts.
    "workload",
    "workload-freeze",
    "markit_mdbench_workload_freeze",
    "G0_PRIMARY",
    "freeze-receipt",
    "selected-files",
    // Known source paths / projects (the closure's failing files and the
    // acquisition universe's project names).
    "myst-parser",
    "oci-image",
    "rust-rfcs",
    "rust-book",
    "CppCoreGuidelines",
    "crafting-interpreters",
    "kubernetes-keps",
    "ethereum-eips",
    "openmlsys",
    "d2l-en",
    "swift-evolution",
    "cs231n",
    "owasp-cheatsheets",
];

#[test]
fn mechanism_sources_reference_no_benchmark_identity() {
    for crate_dir in HORSE_CRATES {
        for (path, text) in crate_sources(crate_dir) {
            for token in FORBIDDEN_TOKENS {
                assert!(
                    !text.contains(token),
                    "{}: production mechanism source contains the benchmark-identity \
                     token {token:?}; mechanism behaviour must depend only on source, \
                     edit, old state and grammar/parser state",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn mechanism_crates_do_not_depend_on_the_workload_freeze() {
    for crate_dir in HORSE_CRATES {
        let manifest = std::fs::read_to_string(root().join(crate_dir).join("Cargo.toml"))
            .expect("mechanism manifest readable");
        let dependencies = dependency_section(&manifest, "dependencies");
        assert!(
            !dependencies.contains("workload-freeze"),
            "{crate_dir}: a mechanism must not depend on the workload freeze"
        );
        assert!(
            !dependencies.contains("markit-mdbench-diagnostics"),
            "{crate_dir}: a mechanism must not depend on the diagnostic crate"
        );
        assert!(
            !dependencies.contains("markit-mdbench-semantics"),
            "{crate_dir}: a mechanism must not depend on the workload semantics crate"
        );
    }
}

#[test]
fn only_the_h0_oracle_crate_may_use_full_rebuild_in_production() {
    // H2/H3/H4 must never obtain correctness by substituting H0's output:
    // the full-rebuild crate may appear only as a TEST-side oracle.
    for crate_dir in [
        "mechanisms/block-local",
        "mechanisms/fragment-reuse",
        "mechanisms/old-tree-subtree-reuse",
        "mechanisms/restart-convergence",
    ] {
        let manifest = std::fs::read_to_string(root().join(crate_dir).join("Cargo.toml"))
            .expect("mechanism manifest readable");
        let dependencies = dependency_section(&manifest, "dependencies");
        assert!(
            !dependencies.contains("full-rebuild"),
            "{crate_dir}: H0 may appear only under [dev-dependencies]"
        );
        let dev = dependency_section(&manifest, "dev-dependencies");
        assert!(
            dev.contains("full-rebuild"),
            "{crate_dir}: the H0 oracle must remain available to tests"
        );
    }
}

/// The text of one TOML table's body (until the next top-level table).
fn dependency_section(manifest: &str, name: &str) -> String {
    let header = format!("[{name}]");
    let mut out = String::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[test]
fn the_diagnostic_crate_is_not_a_mechanism_dependency() {
    // A diagnostic crate that mechanisms could import would let workload
    // knowledge leak into parsing behaviour.
    let manifest = std::fs::read_to_string(root().join("diagnostics/Cargo.toml"))
        .expect("diagnostics manifest readable");
    assert!(manifest.contains("markit-mdbench-workload-freeze"));
    for crate_dir in HORSE_CRATES {
        let mechanism = std::fs::read_to_string(root().join(crate_dir).join("Cargo.toml"))
            .expect("mechanism manifest readable");
        assert!(
            !mechanism.contains("diagnostics"),
            "{crate_dir}: no mechanism may depend on the diagnostic crate"
        );
    }
}
