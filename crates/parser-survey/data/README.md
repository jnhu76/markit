# Pinned research corpora

## commonmark-0.31.2-spec.json

ORACLE-B (DIALECT SEMANTICS) corpus for issue #19 (plan §7).

- spec version: **CommonMark 0.31.2** (frozen; never re-download "latest")
- source URL: `https://spec.commonmark.org/0.31.2/spec.json`
- sha256: `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`
- retrieval date: 2026-09-16
- contents: 652 examples, 26 sections, keys
  `markdown / html / example / start_line / end_line / section`

Usage: `cargo run --release -p parser-survey -- --commonmark \
crates/parser-survey/data/commonmark-0.31.2-spec.json`

The oracle compares only semantics every implementation can honestly map
(normalized block-kind sequence, heading levels); constructs outside the
measured implementation's vocabulary are `UNSUPPORTED`, never `FAIL`.
