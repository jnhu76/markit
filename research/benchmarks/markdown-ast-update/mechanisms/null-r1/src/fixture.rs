//! `R1_SMOKE_ONLY` fixture — the smallest deterministic harness-validation
//! input. NOT a benchmark workload; deliberately tiny; includes CJK and
//! emoji bytes so Unicode correctness of the substrate is exercised.
//! Corpus generation (shapes/sizes) belongs to R3+.

use markit_mdbench_common::CanonicalEdit;
use markit_mdbench_common::OperationKind;
use markit_mdbench_common::PayloadId;
use markit_mdbench_common::PayloadShape;
use markit_mdbench_common::Source;
use markit_mdbench_common::SourceId;

/// Payload identity prefix that marks every R1 fixture artifact.
pub const R1_SMOKE_ONLY_PAYLOAD_ID: &str = "R1_SMOKE_ONLY/fixture-1";

/// Generator identity recorded in case keys for the fixture.
pub const R1_SMOKE_ONLY_GENERATOR_ID: &str = "R1_SMOKE_ONLY";

/// The one canonical edit of the smoke fixture (INSERT).
pub const SMOKE_EDIT_OPERATION: OperationKind = OperationKind::Insert;

pub const OLD_SOURCE_ID: SourceId = SourceId(9001);
pub const POST_SOURCE_ID: SourceId = SourceId(9002);

const OLD_TEXT: &str = "\
# R1 smoke fixture

This paragraph carries *emphasis*, `code`, and gets edited below.

- list item one
- list item two

> a short quote

```text
fenced block
```

CJK: 中文段落。 Emoji: 🐉✅. Tail line.
";

/// Byte offset just after `"carries "` in [`OLD_TEXT`] — an ASCII char
/// boundary inside the first paragraph (never inside a multi-byte char).
const EDIT_START: usize = 43;

const INSERTED_TEXT: &str = "🛠️ edited ";

/// `(old_source, canonical_edit, post_source)`. The post source is
/// materialized host-side by [`CanonicalEdit::apply`], mirroring the
/// runner flow where post-edit materialization happens outside timers.
pub fn smoke_fixture() -> (Source, CanonicalEdit, Source) {
    let old = Source::new(OLD_SOURCE_ID, OLD_TEXT);
    let edit = CanonicalEdit::new(EDIT_START, EDIT_START, INSERTED_TEXT)
        .and_then(|e| {
            e.validate_against(&old)?;
            Ok(e)
        })
        .expect("R1_SMOKE_ONLY fixture must be valid by construction");
    let post = edit
        .apply(&old, POST_SOURCE_ID)
        .expect("R1_SMOKE_ONLY fixture edit must apply");
    (old, edit, post)
}

/// Fixture payload metadata for case keys / result rows.
pub fn smoke_payload_id() -> PayloadId {
    PayloadId(R1_SMOKE_ONLY_PAYLOAD_ID.to_string())
}

pub fn smoke_payload_shape() -> PayloadShape {
    // Honest tag: the fixture is a mixed blob, and it is NOT a benchmark
    // shape surface — real shape/surface work belongs to R3.
    PayloadShape::Mixed
}

pub fn smoke_payload_size_bytes() -> u64 {
    // Size of the old source at fixture-authoring time; asserted below.
    OLD_TEXT.len() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_small_utf8_and_edit_is_char_aligned() {
        let (old, _edit, post) = smoke_fixture();
        assert!(old.len_bytes() < 512, "fixture must stay tiny");
        assert_eq!(old.len_bytes() as u64, smoke_payload_size_bytes());
        assert!(post.len_bytes() > old.len_bytes());
        // Unicode sanity: CJK and emoji survive the splice intact.
        assert!(post.as_str().contains("中文段落。"));
        assert!(post.as_str().contains("🐉✅"));
        assert!(post.as_str().contains("🛠️ edited "));
    }

    #[test]
    fn edit_inserts_at_declared_offset() {
        let (_, edit, post) = smoke_fixture();
        assert_eq!(edit.start_byte(), EDIT_START as u64);
        assert_eq!(edit.end_byte(), EDIT_START as u64);
        assert_eq!(
            &post.as_str()[EDIT_START..EDIT_START + INSERTED_TEXT.len()],
            INSERTED_TEXT
        );
    }
}
