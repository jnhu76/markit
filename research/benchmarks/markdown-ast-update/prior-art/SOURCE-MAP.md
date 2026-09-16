# R2 SOURCE-MAP — prior-art source inventory

Retrieval date for all entries: **2026-09-16**. Machine-readable pins are
consolidated in `../manifest/prior-art.toml`. Source classes follow the R2
source hierarchy (original paper > official repository source > official
design documentation > official issue / maintainer explanation > author
talks > secondary explanations).

## Extracted sources

| id | subject | class | pin | license | record |
|---|---|---|---|---|---|
| `md4c` | MD4C (`mity/md4c`) | official repository source | tag `release-0.5.3` = `472c417005c2c71b8617de4f7b8d6b30411d78f4` | MIT | [md4c.md](./md4c.md) |
| `pulldown-cmark` | pulldown-cmark (`raphlinus/pulldown-cmark`) | official repository source | tag `v0.13.4` = `38e4d08f14ec4bd9783270e9623db7681ebed968` | MIT | [pulldown-cmark.md](./pulldown-cmark.md) |
| `comrak` | Comrak (`kivikakk/comrak`) | official repository source | tag `v0.55.0` = `6fbe87fafde3953a9f3bc582804318593d703805` | BSD-2-Clause | [comrak.md](./comrak.md) |
| `tree-sitter-core-incremental` | Tree-sitter engine (`tree-sitter/tree-sitter`) | official repository source (+ Context7 official docs as secondary, labeled) | tag `v0.27.0` = `6070dbfefd326bd735e5683eb128cc1b57dad0c0` | MIT | [tree-sitter.md](./tree-sitter.md) |
| `tree-sitter-markdown-grammar` | tree-sitter-markdown grammars (`tree-sitter-grammars/tree-sitter-markdown`) | official repository source + official issues (#92/#134/#243 et al., cited in record) | tag `v0.5.3` = `f969cd3ae3f9fbd4e43205431d0ae286014c05b5` | MIT | [tree-sitter-markdown.md](./tree-sitter-markdown.md) |
| `lezer-common` / `lezer-lr` / `lezer-markdown` | Lezer stack (`lezer-parser/common`, `lezer-parser/lr`, `lezer-parser/markdown`) | official repository source (+ Context7 official docs as secondary, labeled) | `1.5.2` = `de5f9627…`; `1.4.8` = `f81d6a25…`; `1.6.3` = `9942d7ce…` | MIT | [lezer.md](./lezer.md) |
| `mizchi-markdown` | mizchi/markdown (`mizchi/markdown.mbt`) | official repository source | master `ffe7dc00e60e6778e6407fb0c9bbfe9f202fea94` (includes "Release 0.8.3"); newest tag `v0.8.1` = `a9679fad…` | MIT | [mizchi-markdown.md](./mizchi-markdown.md) |
| `wagner-graham-1998-toplas` | "Efficient and Flexible Incremental Parsing", TOPLAS 20(5):980–1013 | paper (ACM full text NOT retrievable, HTTP 403); mechanism read via dissertation Ch. 6 reprint (labeled mapping) | DOI 10.1145/293677.293678 (Crossref metadata verified) | — | [wagner-graham.md](./wagner-graham.md) |
| `wagner-1998-thesis-csd-97-946` | T. Wagner PhD dissertation, UC Berkeley CSD-97-946 | paper / technical report — **primary mechanism authority for W&G** (full text read) | https://www2.eecs.berkeley.edu/Pubs/TechRpts/1997/Archive/CSD-97-946.pdf | — | [wagner-graham.md](./wagner-graham.md) |
| `wagner-graham-1997-pldi` | "Incremental Analysis of Real Programming Languages", PLDI (pp. 31–43) | paper (metadata only via Crossref; NOT read — year is **1997**, commonly mislabeled 1996) | DOI 10.1145/258915.258920 | — | [wagner-graham.md](./wagner-graham.md) |
| `swift-incremental-syntax` | swift-syntax (`swiftlang/swift-syntax`) | official repository source | tag `604.0.0` = `050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf` | Apache-2.0 | [swift-incremental-syntax.md](./swift-incremental-syntax.md) |
| `swift-incremental-syntax-thread` | A. Hoppen, "Incremental syntax parsing", Swift Forums (2018) | official maintainer design explanation (design intent; **diverges from pinned code**, recorded in record) | https://forums.swift.org/t/incremental-syntax-parsing/12368 | forum post | [swift-incremental-syntax.md](./swift-incremental-syntax.md) |

## Naming / identification corrections recorded by R2

1. "mizchi/markdown" resolves to `github.com/mizchi/markdown.mbt`
   (npm `@mizchi/markdown`); the core is **MoonBit**, not Rust. `mizchi/md`
   and `mizchi/markdown` GitHub paths do not resolve. Recorded in
   [mizchi-markdown.md](./mizchi-markdown.md) §1.
2. The W&G PLDI paper is **1997** (proceedings pp. 31–43), not 1996 as some
   secondary sources label it. Dissertation bibliography [101] and Crossref
   agree. Recorded in [wagner-graham.md](./wagner-graham.md) §1.
3. `lezer-parser/lezer` is now a forwarding stub (no tags); the LR engine
   source lives in `lezer-parser/lr`. Recorded in [lezer.md](./lezer.md) §1.
4. The 2018 Swift forum design (re-lex from change, lexer-state settling)
   does not exist at the pinned `604.0.0` code; the pinned code is the
   mechanism truth, the thread is design intent. Recorded in
   [swift-incremental-syntax.md](./swift-incremental-syntax.md) §1/§7.

## Considered and not added (boundary decision)

R2 may add further prior art only when it materially fills a missing
mechanism family. After extraction of the minimum set, the following were
considered and **not added**, because no missing mechanism family remained:

- **Roslyn incremental syntax trees (red/green trees)** — persistent-tree
  reuse and representation separation are already evidenced by the
  tree-sitter / Lezer / W&G records; the red/green split is a representation
  design, not an additional update-mechanism family.
- **Salsa-style incremental computation** — memoized-computation reuse on
  query graphs; no mechanism family in H0–H4 requires it, and R0 §2 fixes
  the subject set.
- **Incremental GLR/LR literature beyond W&G** (e.g., the PLDI 1997
  companion, declarative/IDE parser recovery) — W&G Ch. 7 (read via the
  dissertation) already anchors the incremental-LR/GLR family; adding more
  members of the same family would turn R2 into a literature survey without
  a mechanism boundary (R2 scope rule).
- **Rope / piece-tree text-buffer designs** — explicitly outside R0 §6
  (host text mutation is outside parser timing; the benchmark studies syntax
  update, not text-buffer design).

None of the nine minimum-set targets was dropped as NOT_RELEVANT; all nine
produced at least negative evidence or a mechanism family anchor.
