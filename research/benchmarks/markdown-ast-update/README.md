# #22 — Standardized Markdown AST/CST update benchmark (RESERVED)

Status: **RESERVED AREA — the benchmark itself is implemented under issue #22
(MARKIT-MARKDOWN-BENCHMARK-1), not by the repository reset (#23).**

This directory is the future home of the standardized benchmark that runs
existing Markdown parsers / AST-CST update designs on the same payloads, the
same edits, the same correctness gates, and the same measurement model:

```text
MD4C                       full-rebuild control (SAX/callback)
pulldown-cmark             full-rebuild control (Rust events)
Comrak or cmark-gfm        full-rebuild control (retained AST; one of them)
tree-sitter-markdown       incremental CST subject
@lezer/markdown            incremental tree-fragment subject
mizchi/markdown            lossless/incremental CST subject
```

Planned layout (created by #22 as needed — nothing here is implemented yet):

```text
corpus/       pinned payloads + mutation definitions
manifest/     baseline versions, commits, runtimes, capability matrix
results/      raw/ (gitignored, local) + summary/ (curated evidence)
```

Rules (see `AGENTS.md`):

- a fast wrong parser fails: `incremental result == clean authoritative parse`
  is a hard gate wherever the comparison is defined;
- wall-clock alone is insufficient — record work amplification
  (bytes rescanned, nodes rebuilt/reused, allocations, position/index
  maintenance, propagation distance);
- every performance claim records toolchain, commit SHA, hardware/OS, corpus
  version, and benchmark mode;
- this benchmark does NOT pick a "winning parser"; its output is a
  performance surface and a Weakness Map that future Markit algorithm work
  must earn from evidence.

Historical predecessor: Experiment 0
(`research/experiments/experiment-0-parser-survey/`) — reconnaissance only,
not a normalized measurement and not architecture authority.
