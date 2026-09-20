# ACQUISITION REPORT — MARKIT-REAL-WORKLOAD-ACQUISITION-1

Campaign: real Markdown workload acquisition substrate, round 1 (#33
Stage A1 candidate material).

> **STATUS (CORRECTIVE-1, PR #34 human adversarial review):** this report
> records round 1 **as originally executed**. Four of its statements are
> superseded by `ACQUISITION-REPORT-2-CORRECTIVE-1.md`: (a) candidate
> snapshot bytes are no longer tracked in Git (materialized + verified
> locally instead); (b) the source lock is now a fail-closed frozen lock
> (schema 2) with strict-replay semantics; (c) the sampling frame is now
> fully measured by complete per-repository Markdown inventories;
> (d) `cpp-core-guidelines` (and the non-commercial CC sources) are now
> `MATERIALIZE_ONLY`, with their bytes excluded from public Git storage.
> Round-1 pins, hashes, and byte-preservation evidence remain valid and
> unchanged.

```text
VERDICT: ACQUISITION_PASS   (acquisition only; awaiting human review)

NO H0-H4 EXECUTED:                    PASS (nothing measured)
NO FINAL WORKLOAD SELECTION:          PASS (no REPRESENTATIVE_SET / EXTREMAL_SET)
NO ELIGIBILITY FILTERING:             PASS (syntax preserved as source truth)
NO BYTE MODIFICATION / NORMALIZATION: PASS (extraction + blob-id proof)
ALL ROUND-1 SOURCES RESOLVED:         PASS (20 / 20 pinned + snapshotted)
MANIFESTS + SOURCE-LOCK COMPLETE:     PASS (verify PASS, offline)
SECOND ACQUISITION DETERMINISTIC:     PASS (cold-cache byte-identical)
CLEAN WORKTREE EXCEPT INTENT:         PASS (see git status in the PR)
```

## Per-source acquisition table

| SOURCE_ID | REPOSITORY | PINNED_SHA | MARKDOWN_FILE_COUNT | SNAPSHOT_BYTES | LICENSE_STATUS | SOURCE_MANIFEST_HASH | ACQUISITION_STATUS |
|---|---|---|---|---|---|---|---|
| cpp-core-guidelines | https://github.com/isocpp/CppCoreGuidelines | `33bcd015997f0d8e0fa0202eb66254a16f59ad8f` | 1 | 841419 | NEEDS_REVIEW | `35991496a7f7d2e5ecd258e754cb17c22151b4fd37bfa27b500a007ac0a789d8` | ACQUIRED |
| crafting-interpreters | https://github.com/munificent/craftinginterpreters | `4a840f70f69c6ddd17cfef4f6964f8e1bcd8c3d4` | 50 | 181127 | CC-BY-NC-ND-4.0 | `fdee1b8efb183e9098e9fca446e826a21b04e4792b22dc37d35326350d8bb241` | ACQUIRED |
| cs231n | https://github.com/cs231n/cs231n.github.io | `8ec57ccfc32379ae4b37c45de949af359673f92f` | 30 | 606126 | MIT | `9656bbc07550e7f6288c28a422c4e7cc39ea76f9b7c690486cc552bb30d5b35d` | ACQUIRED |
| d2l-en | https://github.com/d2l-ai/d2l-en | `23d7a5aecceee57d1292c56e90cce307f183bb0a` | 191 | 3058724 | CC-BY-SA-4.0 | `ce26dbe5587013b526526f66e872d15e4395bcc60d4dc78cd74f087afa3a49c8` | ACQUIRED |
| ethereum-eips | https://github.com/ethereum/EIPs | `dbc6d457cbf90d5f9d55552f3fd91d58b88cc898` | 956 | 6578099 | CC0-1.0 | `4b38d692a1fa8e84c96636adb678cd47c7c3f23940959e723b74c376b1b172bf` | ACQUIRED |
| graphql-spec | https://github.com/graphql/graphql-spec | `59bc70ac974b0d5d1e00869f41c06bb3bfc756da` | 15 | 336747 | OWFa-1.0 | `9771fd80281b3c3353f58e039af3b25775a4d0894bfda5a6481f5debdec01f3a` | ACQUIRED |
| kubernetes-keps | https://github.com/kubernetes/enhancements | `0469acb1d9444635135faf5a6c6711486b1a9040` | 678 | 20907633 | Apache-2.0 | `c7be65c0741ba46e1c64fe9e16a04cffd50c66401e27909526d7ea55e8f0cb22` | ACQUIRED |
| myst-parser | https://github.com/executablebooks/MyST-Parser | `723cffcf84213f0cb58695b27eec9ad72052b53a` | 27 | 139298 | MIT | `834d0561fd4f2c83ca8028efaf473e1587b3f03fb44d7819e02a425cdc29a767` | ACQUIRED |
| mystmd | https://github.com/jupyter-book/mystmd | `d09d3e59391c3163e73aa88ce2b97092e2e5612e` | 89 | 621868 | MIT | `46ca79743de515f7e0c8b74198220a455af2ee83e26260c72829c679a351dda8` | ACQUIRED |
| node | https://github.com/nodejs/node | `97af3d7da562406b75d1baeecb79ef8105d4b92a` | 70 | 4695458 | MIT | `05ae643aa0dee2fd5bf1967cba1c1226b60fef72e5d297cbb5d76574097408e0` | ACQUIRED |
| oci-image | https://github.com/opencontainers/image-spec | `ca68a05fad732be329ef67e713c3af60fb7d5f76` | 18 | 129249 | Apache-2.0 | `3f08fd49b6735e7397e4fad0a9044949ac6ae203de26d7c41357518cf6e22ba7` | ACQUIRED |
| oci-runtime | https://github.com/opencontainers/runtime-spec | `6999a89a76a0329f440d5740497bedb9dd431297` | 21 | 209935 | Apache-2.0 | `c189368944d26c801c24d3e2abe0186463f6539da5a1d2131a5a2630c96dafec` | ACQUIRED |
| openapi | https://github.com/OAI/OpenAPI-Specification | `c9f8f040e825a827bb011955bd41b7e2d899688f` | 23 | 1983802 | Apache-2.0 | `f157c7a4d88404d019f9fa1a14f4d2bbf5252480c8ddc9b30fcdaafdb53b85fd` | ACQUIRED |
| openmlsys | https://github.com/openmlsys/openmlsys | `9c289782ccbb165ac8ad7c960ecffc12942a5560` | 210 | 1630531 | CC-BY-NC-SA-4.0 | `42284132765da6a648577fc35fd544c89f35d8149ac6b5fa15a8b0e212b3f015` | ACQUIRED |
| opentelemetry-spec | https://github.com/open-telemetry/opentelemetry-specification | `148f27606cf0352c11a314e7bf9eefa6bf88db86` | 99 | 1324328 | Apache-2.0 | `8dbba839f13d366f6bca41e38c3af9f1164c5522cc65b0384aa9694651a9933e` | ACQUIRED |
| owasp-cheatsheets | https://github.com/OWASP/CheatSheetSeries | `ffa997b0ba0604176cba2bdba1a9563ae258329a` | 122 | 2538601 | CC-BY-SA-4.0 | `e80c41d3808d3698ef7e52305934fd9ac91d8419018e9a62c7e69aa821f7df3c` | ACQUIRED |
| progit | https://github.com/progit/progit2 | `a013e3230a1207cfa5ae94d28ba7d2021063c337` | 3 | 9290 | CC-BY-NC-SA-3.0 | `bb40b18e0b4f61c38ffd59606f8732a5c717b8e9fe2b5c8e99d09291c09b4f52` | ACQUIRED |
| rust-book | https://github.com/rust-lang/book | `1500248d8f230566e4ec9f27fcbb8fe9e2898ab1` | 112 | 1221077 | MIT OR Apache-2.0 | `b60cdc6cd1c902c63c7e1a617cb5e41300a0b8ef40215a759163b9268da8a9ea` | ACQUIRED |
| rust-rfcs | https://github.com/rust-lang/rfcs | `51783df9a76c355de7ceebeae101cba47f8ca463` | 651 | 9267975 | MIT OR Apache-2.0 | `e031661ea3664ca748821d148785931a4632795733dbf7e825e469d31057fa48` | ACQUIRED |
| swift-evolution | https://github.com/swiftlang/swift-evolution | `5d88903d843f6f2babaf6d5e76b29dd58d9fcae8` | 580 | 10104895 | Apache-2.0 | `b52fb5b0dc5d0ac160441ef048333bb04595fb7ea3645c358b4748275021b928` | ACQUIRED |

## Corpus size (Git size discipline)

```text
repositories (sources):        20
markdown files:                3946
total markdown bytes:          66386182  (66.4 MB, ~63.3 MiB)
largest source file:           cpp-core-guidelines/CppCoreGuidelines.md = 841419 bytes
largest project snapshot:      kubernetes-keps = 20907633 bytes (678 files)
```

Decision: the measured corpus is text-only, ~66 MB across ~3.9k files.
This is comfortably within normal Git tracking; no Git LFS, no external
blob store, no storage-architecture switch.

Exact byte-identical duplicates detected (recorded in
`manifests/exact-duplicates-v1.json`; **not** deleted — deduplication is a
later selection concern):

```text
openapi: 3 groups of byte-identical versions/*-editors.md files
```

## Byte preservation and line endings

Every snapshot file was written from `git cat-file blob
<commit_sha>:<upstream_path>` stdout with zero post-processing. The
upstream working tree was never read, so no core.autocrlf / .gitattributes
smudge/eol conversion can affect stored bytes. For every file,
`SOURCE.json` records the upstream blob id, and integrity is proven by
recomputing `sha1("blob <size>\0" + bytes)` over the snapshot and
comparing it with the pinned commit tree (plus the independent sha256).

Independent spot check (2026-09-20, this campaign): for 94 randomly
sampled files across all 20 sources,
`git rev-parse <pin>:<path>` == `SOURCE.json.git_blob_sha1` ==
`git hash-object --no-filters <snapshot file>`.

Repository-side guarantee: this campaign also added a root
`.gitattributes` rule marking the whole `workloads/` subtree `-text`, so
no machine-local `core.autocrlf` setting can normalize bytes at checkin
or checkout. With the rule in place, a full audit compared the staged
git blob of every snapshot file against the recorded upstream blob id:

```text
STAGED-BLOB AUDIT: 3946 files checked, 0 mismatches
```

## Reproducibility / determinism evidence

Executed with `python3 tools/acquire.py determinism-check`:

```text
acquire (pass 1)                       -> tracked artifacts: 3976 files
                                          digest fb2b34bbbeef54014eab32a3c8c3652409c3be7d381fcad84d00a919203bbb27
clear _cache/repos (rebuildable cache) -> removed entirely
acquire (pass 2, cold, pinned SHAs)    -> tracked artifacts: 3976 files
                                          digest fb2b34bbbeef54014eab32a3c8c3652409c3be7d381fcad84d00a919203bbb27
compare                                -> DETERMINISM PASS (byte-identical)
```

Notes:

- 40 per-source `SOURCE.json identity unchanged; existing bytes preserved`
  records (20 sources x 2 passes): `retrieved_at` (first-acquisition
  timestamp) is preserved and is excluded from `source_manifest_hash`;
  identity is timestamp-free.
- Pass 2 fetched each upstream by **pinned SHA**, not by branch, so
  upstream default-branch movement cannot change the snapshot.
- `verify` runs fully offline from `sources/` + manifests with
  `_cache/repos` absent: benchmark source material does not depend on
  upstream HEAD or on network availability.

## Source isolation

`source-lock.json` pins, per source: repository URL, immutable commit SHA,
and the timestamp-free `source_manifest_hash`; plus campaign-level sha256
of `acquisition-config.json`, `tools/acquire.py`, and
`manifests/candidate-universe-v1.json`. Re-acquiring the lock means
re-fetching exactly those commits and reproducing exactly these bytes.
Upstream HEAD changes cannot affect the frozen workload.

## License status summary

SPDX identifiers were recorded only after reading the actual license files
at the pinned commits; per-source evidence (license file paths + blob ids)
is in each `SOURCE.json`.

```text
Apache-2.0            kubernetes-keps, oci-image, oci-runtime, openapi,
                      opentelemetry-spec, swift-evolution
MIT                   cs231n, myst-parser, mystmd, node (Node.js MIT grant;
                      LICENSE also carries bundled third-party notices)
MIT OR Apache-2.0     rust-book, rust-rfcs
CC0-1.0               ethereum-eips
CC-BY-SA-4.0          d2l-en, owasp-cheatsheets
CC-BY-NC-SA-4.0       openmlsys (stated in README; no dedicated LICENSE file)
CC-BY-NC-SA-3.0       progit (LICENSE.asc)
CC-BY-NC-ND-4.0       crafting-interpreters (.md files; code is MIT)
OWFa-1.0              graphql-spec (specification deliverables; code MIT)
NEEDS_REVIEW          cpp-core-guidelines (custom Standard C++ Foundation
                      license, personal/internal business use; no SPDX id)
```

## Acquisition audit notes

- `progit`: book content is AsciiDoc (`book/*/*.asc`); NOT converted per
  contract. Only genuine root-level repository Markdown is snapshotted
  (3 files). Fact recorded in provenance.
- `crafting-interpreters`: book chapters are HTML (`book/`); NOT converted.
  Snapshotted material is the Markdown under `note/` (50 files).
- `node`: full `doc/api` Markdown tree snapshotted (70 files) so later
  static characterization can inspect beyond the six initially named API
  files without a new acquisition round; the initial named subset is not
  assumed to be the final Node workload.
- `d2l-en`: D2L-specific fence info strings (e.g. ```` ```{.python .input} ````)
  preserved verbatim. `openmlsys`: Chinese and English Markdown both
  preserved; no translation deduplication. `cs231n`: delimiter dialects
  outside BENCH-GRAMMAR-v1 recorded as source truth only.
- Include/exclude rules are mechanical glob rules frozen in
  `acquisition-config.json`. The only content-independent exclusions are
  license/notice file names for root-glob sources (license identity is
  recorded in each `SOURCE.json` license block instead). No file was
  excluded or rewritten for syntax, size, or expected parser behavior.

## Explicit non-goals (unchanged)

No parser features, no math/Mermaid parsing, no H0-H4, no latency
measurement, no representative/extremal set, no clustering, no PCA, no
workload weights, no edit generation, no CaseId materialization, no
eligibility analysis, no BENCH-GRAMMAR-v1 changes, no horse or parser
implementation changes.

Next stages (later campaigns, not this one): eligibility scan ->
mechanism-neutral static characterization -> diversity/redundancy/eligibility
bias analysis -> representative + extremal selection -> corpus freeze ->
edit-trace freeze -> and only then measurement.

