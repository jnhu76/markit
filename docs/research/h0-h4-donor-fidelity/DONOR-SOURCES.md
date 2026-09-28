# Gate-A donor source manifest — H0–H4 donor fidelity (#76)

Status: **FROZEN GATE-A PROVENANCE RECORD**

- Pinned: 2026-09-28
- Pinned by: Gate-A orchestrator (issue #76), from the repository's
  R2 prior-art authority (`../../research/benchmarks/markdown-ast-update/prior-art/`,
  which pins every source to a tag or commit) plus clean-room verification.
- Clean-room verification host: `jnhu@192.168.31.75`
  (`~/research/markit-gate76/donors/`), git 2.55.0, Fedora kernel
  7.2.5-200.fc44.x86_64. Every SHA below was resolved by `git rev-parse HEAD`
  on a fresh `--filter=blob:none` clone checked out at the pin. Access was
  password-authenticated once via a transient OpenSSH askpass (no credential
  stored in any repository file, script, or log).
- Selection rule: pins come from repository authority (R2 extraction records).
  No pin was chosen, moved, or re-resolved based on performance behavior.
  This file is provenance only; it confers no implementation authority.

## Donor identity table

| Donor | Role (horses) | Repo | Pin | Resolved commit | Retrieval |
|---|---|---|---|---|---|
| MD4C | H0 lineage (clean full-parse anchor) | github.com/mity/md4c | `release-0.5.3` | `472c417005c2c71b8617de4f7b8d6b30411d78f4` | 2026-09-28 |
| pulldown-cmark | H0 lineage | github.com/pulldown-cmark/pulldown-cmark | `v0.13.4` | `38e4d08f14ec4bd9783270e9623db7681ebed968` | 2026-09-28 |
| Comrak | H0 lineage | github.com/kivikakk/comrak | `v0.55.0` | `6fbe87fafde3953a9f3bc582804318593d703805` | 2026-09-28 |
| mizchi/markdown (`markdown.mbt`) | H1 primary donor | github.com/mizchi/markdown.mbt | master `ffe7dc00` | `ffe7dc00e60e6778e6407fb0c9bbfe9f202fea94` | 2026-09-28 |
| tree-sitter-markdown | H1/H3/H4 entry-context-state evidence | github.com/MDeiml/tree-sitter-markdown | `v0.5.3` | `f969cd3ae3f9fbd4e43205431d0ae286014c05b5` | 2026-09-28 |
| @lezer/common | H2 primary donor | github.com/lezer-parser/common | `1.5.2` | `de5f96276a2954c249de1475e8b03f79c20d9ce4` | 2026-09-28 |
| @lezer/lr | H2 primary donor | github.com/lezer-parser/lr | `1.4.8` | `f81d6a25c3482aa7fc12434e9adea9d75c56ad08` | 2026-09-28 |
| @lezer/markdown | H2 primary donor; H4 Markdown convergence guards | github.com/lezer-parser/markdown | `1.6.3` | `9942d7ce41d734d743cbdb48177dddc1975fdc5c` | 2026-09-28 |
| Tree-sitter core | H3 primary donor; H4 state-agreement gating | github.com/tree-sitter/tree-sitter | `v0.27.0` | `6070dbfefd326bd735e5683eb128cc1b57dad0c0` | 2026-09-28 |
| swift-syntax | H3 overlap variant; H4 modern restart/checkpoint anchor | github.com/swiftlang/swift-syntax | `604.0.0` | `050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf` | 2026-09-28 |
| Wagner & Graham | H4 classical anchor; H3 concept ancestor | (paper; no repo) | TOPLAS 20(5) 1998, DOI 10.1145/293677.293678; dissertation CMU-CSD-97-946 Ch. 6 | n/a | repo record (R2) + PDF identity check 2026-09-28 |

## Load-bearing source paths (per donor, at the pinned commit)

- md4c: `src/md4c.c`, `src/md4c.h`
- pulldown-cmark: `pulldown-cmark/src/{lib,parse,firstpass,tree}.rs`
- comrak: `src/parser/{mod,inlines}.rs`, `src/lib.rs`
- mizchi/markdown.mbt: `src/incremental.mbt` (the entire incremental
  mechanism), `src/types.mbt`, `src/block_parser.mbt`, `src/api/exports.mbt`,
  `src/incremental_test.mbt`
- tree-sitter-markdown: `src/scanner.c` (external-scanner serialize state)
- @lezer/common: `src/parse.ts` (TreeFragment machinery)
- @lezer/lr: `src/parse.ts`, `src/stack.ts` (fragment-gated parse,
  state-anchored absorption)
- @lezer/markdown: `src/markdown.ts` (FragmentCursor / takeNodes / NotLast
  convergence guards)
- tree-sitter core: `lib/src/{parser.c,reusable_node.h,subtree.c,subtree.h,tree.c,get_changed_ranges.c}`,
  `lib/include/tree_sitter/api.h`
- swift-syntax: `Sources/SwiftParser/IncrementalParseTransition.swift`,
  `Sources/SwiftParser/TopLevel.swift`, `Sources/SwiftParser/Parser.swift`
  (LookaheadRanges)

## Mechanism dimensions supported per horse

| Horse | Primary donors | Supporting evidence donors |
|---|---|---|
| H0 FULL_REBUILD | md4c, pulldown-cmark, comrak (family: zero-state whole-buffer rebuild) | — (H0 is the benchmark's own reference; donors are lineage, not reproduction targets) |
| H1 BLOCK_LOCAL_REPARSE | mizchi/markdown.mbt | tree-sitter-markdown (candidate entry-context dimensions, non-authoritative); md4c/pulldown-cmark (cross-line-state catalogs) |
| H2 FRAGMENT_REUSE | @lezer/common + @lezer/lr + @lezer/markdown | — |
| H3 OLD_TREE_SUBTREE_REUSE | tree-sitter core | tree-sitter-markdown (scanner entry state); Wagner & Graham (concept ancestor); swift-syntax (overlap variant) |
| H4 RESTART_CONVERGENCE | Wagner & Graham + swift-syntax (explicitly composite) | tree-sitter core (state-agreement gating); @lezer/markdown (Markdown convergence guards) |

## Provenance augmentation notes (documentation-only, no pin changes)

1. **Lezer repository org.** The R2 records cite the codemirror org
   (`codemirror/lezer*`). The npm `repository.url` metadata of the pinned
   package versions points at the `lezer-parser` org
   (`lezer-parser/{common,lr,markdown}`), and the repos' tags at exactly
   `1.5.2` / `1.4.8` / `1.6.3` resolve to the commits above — the org was
   renamed upstream; the lineage is the same projects the R2 records
   extracted. Recorded here so the URL difference does not read as a donor
   substitution.
2. **swift-syntax tag identity.** Tag `604.0.0` resolves (verified via the
   GitHub git-ref API and on the clone) to `050f1a34…`. `git describe
   --exact-match` prefers a different tag on the same commit
   (`swift-6.4.x-DEVELOPMENT-SNAPSHOT-…`); the pin is the `604.0.0` tag
   object → commit mapping, which is unambiguous.
3. **mizchi pin.** R2 pins "master `ffe7dc00`" (a master-branch commit, no
   release tag). Resolved to the full SHA above; no exact tag exists, which
   is expected and documented.
4. **Wagner & Graham.** No runnable repository exists; the donor is the
   paper/dissertation. The dissertation PDF's identity, preface, and Ch. 6
   section structure were re-verified against the primary document on
   2026-09-28 (web reader; the archived PDF itself is not vendored).

## License notes

All donors are open-source (MIT/BSD/Apache-2.0-family per their
repositories); no donor source is vendored into Markit. This manifest and
the fidelity contracts reference donors immutably and summarize mechanism
behavior only — per the Gate-A rule, no large upstream source passages are
copied into this repository. The preserved reviewer harness/probe scripts
under `challenge-evidence/` are Markit-authored code that LINKS against or
INSTALLS donors at the pinned versions; they embed no donor source.
