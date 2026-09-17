//! Golden grammar fixture loading (r3-fixture-v1).
//!
//! The 43 hand-authored fixtures under `grammar/fixtures/` are independent
//! semantic authority (NORMALIZED-RESULT-v1 §5): their `expected_tree` is
//! TRUE BY DECLARATION. This module loads them and parses the expected
//! tree text into a [`NormalizedDocument`] so any horse can be compared
//! structurally. It contains NO Markdown parser — only the small
//! declarative tree-syntax reader (same grammar as `scripts/verify_r3.py`,
//! which owns the static freeze gate).
//!
//! Comparison against a horse's normalized result is plain structural
//! equality ([`NormalizedDocument: PartialEq`]) — never checksum-based.

use std::fmt;
use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::normalized::{Node, NodeKind, NormalizedDocument};

/// Everything that can go wrong while loading fixtures.
#[derive(Debug)]
pub enum FixtureError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    /// A structural problem in one fixture file (id reported).
    Bad {
        id: String,
        path: PathBuf,
        reason: String,
    },
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureError::Io(e) => write!(f, "fixture io error: {e}"),
            FixtureError::Toml(e) => write!(f, "fixture toml error: {e}"),
            FixtureError::Bad { id, path, reason } => {
                write!(f, "fixture {id} ({}):\n{reason}", path.display())
            }
        }
    }
}

impl std::error::Error for FixtureError {}

/// One loaded fixture.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub id: String,
    pub name: String,
    pub covers: Vec<String>,
    /// Exact source bytes (TOML multiline-literal semantics already
    /// applied by the toml crate: leading newline trimmed, content kept
    /// verbatim including the final LF).
    pub source: String,
    pub source_sha256: String,
    /// The hand-authored expected tree as a normalized document.
    pub expected: NormalizedDocument,
}

fn kind_from_name(name: &str) -> Option<NodeKind> {
    Some(match name {
        "Document" => NodeKind::Document,
        "Paragraph" => NodeKind::Paragraph,
        "Heading" => NodeKind::Heading,
        "BlockQuote" => NodeKind::BlockQuote,
        "List" => NodeKind::List,
        "ListItem" => NodeKind::ListItem,
        "FencedCode" => NodeKind::FencedCode,
        "Text" => NodeKind::Text,
        "Emphasis" => NodeKind::Emphasis,
        "CodeSpan" => NodeKind::CodeSpan,
        "Link" => NodeKind::Link,
        "ReferenceLink" => NodeKind::ReferenceLink,
        "ReferenceDefinition" => NodeKind::ReferenceDefinition,
        _ => return None,
    })
}

/// Parse one expected-tree text `(Kind start end key=value ...)` into a
/// normalized document. Nesting is defined by parentheses; indentation is
/// cosmetic (several frozen fixtures are irregularly indented).
pub fn parse_expected_tree(text: &str, id: &str) -> Result<NormalizedDocument, String> {
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let root = parse_node(bytes, &mut pos, id)?;
    skip_ws(bytes, &mut pos);
    if pos != bytes.len() {
        return Err(format!("trailing content after root node at byte {pos}"));
    }
    if root.kind != NodeKind::Document {
        return Err("root node is not a Document".to_string());
    }
    Ok(NormalizedDocument::new(root))
}

fn skip_ws(b: &[u8], pos: &mut usize) {
    while *pos < b.len() && (b[*pos] == b' ' || b[*pos] == b'\n' || b[*pos] == b'\t') {
        *pos += 1;
    }
}

fn parse_node(b: &[u8], pos: &mut usize, id: &str) -> Result<Node, String> {
    skip_ws(b, pos);
    if *pos >= b.len() || b[*pos] != b'(' {
        return Err(format!("expected '(' at byte {pos}"));
    }
    *pos += 1;
    // kind name
    let name_start = *pos;
    while *pos < b.len() && b[*pos].is_ascii_alphanumeric() {
        *pos += 1;
    }
    let kind = kind_from_name(&String::from_utf8_lossy(&b[name_start..*pos]))
        .ok_or_else(|| format!("{id}: unknown node kind at byte {name_start}"))?;
    // span
    let start = parse_usize(b, pos, id)?;
    let end = parse_usize(b, pos, id)?;
    let mut node = Node::new(kind, start, end);
    // fields: ` key=value` tokens until ')' or a child '('
    loop {
        // look ahead: whitespace then ')' | '(' | alphabetic key
        let mut look = *pos;
        while look < b.len() && (b[look] == b' ' || b[look] == b'\n') {
            look += 1;
        }
        if look >= b.len() {
            return Err(format!("{id}: unexpected end of tree"));
        }
        match b[look] {
            b')' | b'(' => {
                *pos = look;
                break;
            }
            _ => {
                *pos = look;
                let key_start = *pos;
                while *pos < b.len() && b[*pos].is_ascii_alphanumeric() {
                    *pos += 1;
                }
                let key = String::from_utf8_lossy(&b[key_start..*pos]).into_owned();
                if *pos >= b.len() || b[*pos] != b'=' {
                    return Err(format!("{id}: expected '=' after key {key:?}"));
                }
                *pos += 1;
                match key.as_str() {
                    "level" => {
                        if node.level.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        node.level = Some(parse_usize(b, pos, id)? as u8);
                    }
                    "marker" => {
                        if node.marker.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        node.marker = Some(parse_quoted(b, pos, id, "marker")?);
                    }
                    "info" => {
                        if node.info.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        node.info = Some(parse_quoted(b, pos, id, "info")?);
                    }
                    "label" => {
                        if node.label.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        node.label = Some(parse_quoted(b, pos, id, "label")?);
                    }
                    "destination" => {
                        if node.destination.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        node.destination = Some(parse_quoted(b, pos, id, "destination")?)
                    }
                    "content" => {
                        if node.content.is_some() {
                            return Err(format!("{id}: duplicate field {key:?}"));
                        }
                        let a = parse_usize(b, pos, id)?;
                        if *pos >= b.len() || b[*pos] != b':' {
                            return Err(format!("{id}: content field must be start:end"));
                        }
                        *pos += 1;
                        let cend = parse_usize(b, pos, id)?;
                        node.content = Some((a, cend));
                    }
                    other => return Err(format!("{id}: unknown field {other:?}")),
                }
            }
        }
    }
    // children
    loop {
        skip_ws(b, pos);
        if *pos < b.len() && b[*pos] == b'(' {
            node.children.push(parse_node(b, pos, id)?);
        } else {
            break;
        }
    }
    if *pos >= b.len() || b[*pos] != b')' {
        return Err(format!("{id}: expected ')' at byte {pos}"));
    }
    *pos += 1;
    Ok(node)
}

fn parse_usize(b: &[u8], pos: &mut usize, id: &str) -> Result<usize, String> {
    while *pos < b.len() && b[*pos] == b' ' {
        *pos += 1;
    }
    let start = *pos;
    while *pos < b.len() && b[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if start == *pos {
        return Err(format!("{id}: expected number at byte {start}"));
    }
    String::from_utf8_lossy(&b[start..*pos])
        .parse::<usize>()
        .map_err(|e| format!("{id}: bad number: {e}"))
}

fn parse_quoted(b: &[u8], pos: &mut usize, id: &str, field: &str) -> Result<String, String> {
    while *pos < b.len() && b[*pos] == b' ' {
        *pos += 1;
    }
    if *pos >= b.len() || b[*pos] != b'"' {
        return Err(format!("{id}: field {field} must be a quoted string"));
    }
    *pos += 1;
    let start = *pos;
    while *pos < b.len() && b[*pos] != b'"' {
        *pos += 1;
    }
    if *pos >= b.len() {
        return Err(format!("{id}: unterminated string in field {field}"));
    }
    let s = String::from_utf8_lossy(&b[start..*pos]).into_owned();
    *pos += 1;
    Ok(s)
}

/// Load every `*.toml` fixture from a directory, sorted by file name,
/// and verify the source sha256 recorded in each file.
pub fn load_fixtures(dir: &Path) -> Result<Vec<Fixture>, FixtureError> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(FixtureError::Io)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let raw = std::fs::read_to_string(&path).map_err(FixtureError::Io)?;
        let value: toml::Value = toml::from_str(&raw).map_err(FixtureError::Toml)?;
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| path.file_stem().unwrap().to_str().unwrap())
            .to_string();
        let bad = |reason: String| FixtureError::Bad {
            id: id.clone(),
            path: path.clone(),
            reason,
        };
        if value.get("schema").and_then(|v| v.as_str()) != Some("r3-fixture-v1") {
            return Err(bad("schema is not r3-fixture-v1".into()));
        }
        let source = value
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| bad("missing source".into()))?
            .to_string();
        let sha = value
            .get("source_sha256")
            .and_then(|v| v.as_str())
            .ok_or_else(|| bad("missing source_sha256".into()))?
            .to_string();
        let actual = sha2::Sha256::digest(source.as_bytes());
        let actual_hex: String = actual.iter().map(|b| format!("{b:02x}")).collect();
        if actual_hex != sha {
            return Err(bad(format!(
                "source_sha256 mismatch: expected {sha}, actual {actual_hex}"
            )));
        }
        let tree_text = value
            .get("expected_tree")
            .and_then(|v| v.as_str())
            .ok_or_else(|| bad("missing expected_tree".into()))?;
        let expected = parse_expected_tree(tree_text.trim(), &id).map_err(bad)?;
        // R4-H0-REFERENCE-CORRECTIVE-1: an expected tree is semantic
        // authority only if it satisfies the frozen NORMALIZED-RESULT-v1
        // conformance gate — exact field-kind legality, the zero-length
        // rule, the FencedCode.content interval rule, char boundaries.
        crate::validate_normalized(&expected, Some(source.as_bytes())).map_err(bad)?;
        out.push(Fixture {
            id,
            name: value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            covers: value
                .get("covers")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            source,
            source_sha256: sha,
            expected,
        });
    }
    Ok(out)
}

/// Path of the frozen fixture directory inside this repository.
pub fn repo_fixture_dir() -> PathBuf {
    // `env!("CARGO_MANIFEST_DIR")` of the oracle crate is
    // `<benchmark>/oracle`; fixtures are `<benchmark>/grammar/fixtures`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("grammar/fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_frozen_fixtures() {
        let fixtures = load_fixtures(&repo_fixture_dir()).expect("fixtures load");
        assert_eq!(fixtures.len(), 43, "the frozen fixture count is 43");
        for f in &fixtures {
            assert_eq!(f.expected.root.kind, NodeKind::Document);
            assert_eq!(f.expected.root.start, 0);
            assert_eq!(f.expected.root.end, f.source.len());
        }
    }

    #[test]
    fn parses_documentation_example() {
        let doc = parse_expected_tree("(Document 0 12\n  (Paragraph 0 11\n    (Text 0 11)))", "T")
            .unwrap();
        assert_eq!(doc.root.children.len(), 1);
        assert_eq!(doc.root.children[0].kind, NodeKind::Paragraph);
    }

    #[test]
    fn load_gate_rejects_the_corrected_codespan_field() {
        // R4-CORRECTIVE-1 regression: a tree carrying the removed
        // CodeSpan `content` field parses as SYNTAX but must fail the
        // conformance gate at load time.
        let doc = parse_expected_tree("(Document 0 5\n  (CodeSpan 0 5 content=1:4))", "T")
            .expect("tree syntax still parses");
        let err = crate::validate_normalized(&doc, Some("a`b`c".as_bytes()))
            .expect_err("CodeSpan content= must never load again");
        assert!(err.contains("forbidden field(s) content"), "{err}");
    }
}
