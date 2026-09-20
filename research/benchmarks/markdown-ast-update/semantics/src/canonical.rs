//! Canonical JSON serialization + digest helpers.
//!
//! CORRECTIVE-A requires that profiler/payload artifacts be reproducible
//! byte-for-byte "where practical, or canonically equivalent where
//! serialization order differs". Every artifact this crate writes goes
//! through [`canonical_json`]: object keys sorted, no insignificant
//! whitespace, arrays in declaration order, floats via `serde_json`'s
//! deterministic formatting. Two runs over the same inputs therefore
//! produce byte-identical files.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of arbitrary bytes (the repository digest form).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Canonical JSON text of any serializable value (sorted keys, no spaces).
pub fn canonical_json<T: Serialize>(value: &T) -> String {
    let value = serde_json::to_value(value).expect("value is serializable");
    let mut out = String::new();
    write_canonical(&value, &mut out);
    out
}

/// Canonical JSON text followed by one trailing newline (text-file form).
pub fn canonical_json_line<T: Serialize>(value: &T) -> String {
    let mut out = canonical_json(value);
    out.push('\n');
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(&json_string(s)),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(key));
                out.push(':');
                write_canonical(&map[*key], out);
            }
            out.push('}');
        }
    }
}

fn json_string(s: &str) -> String {
    serde_json::to_string(s).expect("string is serializable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_keys_and_drops_whitespace() {
        let value = serde_json::json!({"b": 1, "a": [1, 2, {"d": null, "c": "x"}]});
        assert_eq!(
            canonical_json(&value),
            r#"{"a":[1,2,{"c":"x","d":null}],"b":1}"#
        );
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
