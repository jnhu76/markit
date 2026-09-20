//! Generate the CORRECTIVE-A machine-readable contract artifacts from the
//! Rust models, which are the single schema authority:
//!
//! ```text
//! grammar/grammar-lanes-v1.json
//! workloads/profiles/schema/real-profile-v1.schema.json
//! workloads/payloads/schema/real-payload-v1.schema.json
//! workloads/payloads/schema/transition-oracle-v1.schema.json
//! ```
//!
//! Usage (from `research/benchmarks/markdown-ast-update`):
//!
//! ```text
//! cargo run -p markit-mdbench-semantics --bin mdbench-gen-semantic-schema
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use markit_mdbench_semantics::artifacts::schema_artifacts;

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

fn main() -> io::Result<()> {
    let root = bench_root();
    for (relative, contents) in schema_artifacts() {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
