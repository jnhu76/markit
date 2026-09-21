//! Generate `protocol/result-schema-v2.json` from the Rust `ResultRowV1`
//! model, which is the single schema authority (R1 task §12).
//!
//! Usage: `cargo run -p markit-mdbench-runner --bin mdbench-gen-schema --
//! protocol/result-schema-v2.json`

use std::fs;
use std::io;

use markit_mdbench_runner::ResultRowV1;

fn main() -> io::Result<()> {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "protocol/result-schema-v2.json".to_string());
    let schema = schemars::schema_for!(ResultRowV1);
    let mut text = serde_json::to_string_pretty(&schema)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    text.push('\n');
    fs::write(&out_path, text)?;
    println!("wrote {out_path}");
    Ok(())
}
