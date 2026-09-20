# ACQUISITION REPORT 2 — CORRECTIVE 1 (MARKIT-REAL-WORKLOAD-ACQUISITION-1)

Corrective round addressing the fresh-context human adversarial review of
PR #34 (verdict `CORRECTIVE_REQUIRED / DO NOT MERGE YET`). This campaign
remains **acquisition only**: no H0-H4, no eligibility scanning, no static
characterization, no workload selection. PR #34 stays Draft; nothing is
merged.

```text
VERDICT: CORRECTIVE_PASS   (acquisition substrate correction only)

ALL 4 BLOCKING FINDINGS FIXED:        PASS (see mapping below)
ALL 4 ADDITIONAL CORRECTIONS FIXED:   PASS (see mapping below)
NO H0-H4 EXECUTED:                    PASS (nothing measured)
NO NEW CANDIDATES FOR BOOK ROLES:     PASS (role truth recorded, no swaps)
MERGE:                                BLOCKED BY CONTRACT (PR stays Draft)
```

## Finding-by-finding mapping

### 1. MAJOR — frozen lock writable/destructive under normal acquire → FIXED

The lock is now a fail-closed frozen identity (schema 2). Normal commands
are strict replay and **the only lock writers are the explicit operations
`init` and `relock`**:

- `acquire` / `materialize` run `preflight_replay()` BEFORE any mutation:
  acquisition-config sha256, tools/acquire.py sha256, campaign identity,
  lock schema, exact source-set equality (both directions), a valid pinned
  40-hex SHA for every configured source, SOURCE.json existence + identity
  (`source_manifest_hash`) for every locked source, and storage-policy
  correspondence. Any mismatch terminates with exit 1 before a single byte
  is written or deleted.
- Replay never re-pins (pinned SHAs only, never a floating branch), never
  rewrites SOURCE.json, never regenerates or refreshes lock identity.
  Derived manifests are rebuilt in memory and must reproduce the hashes
  recorded in the lock byte-identically or the run aborts without writing.
- First-time discovery (`init`) and lock refresh (`relock --keep-pins` /
  `relock --rediscover`) are separate, loud, explicit operations whose git
  diff is the reviewable event. `init` refuses when a lock or any
  SOURCE.json already exists; `manifests` (the old implicit lock rewriter)
  no longer exists as a command.
- `materialize` — the only post-init snapshot-byte producer — runs the
  same preflight, then fetches the exact pinned bytes locally and verifies
  sha256 + git blob id + byte count against SOURCE.json.

**Adversarial unit evidence** (`tools/test_acquire.py`, 45 tests, offline,
all PASS). Every rejection test also asserts the byte-state invariance
property (full workspace digest identical before/after the rejected call —
termination BEFORE any mutation):

| adversarial mutation | gate that fires |
|---|---|
| changed acquisition-config.json | config sha256 vs lock |
| changed include glob (subset of above, tested separately) | config sha256 vs lock |
| changed tools/acquire.py | tool sha256 vs lock |
| missing lock with existing source material | "run relock" refusal |
| missing lock with no sources | "run init" cold-start guidance |
| missing source pin / malformed pin in lock | pinned-SHA gate |
| source missing from lock / extra source in lock | source-set equality |
| changed SOURCE identity (re-pin, manifest drift) | SOURCE identity gate |
| missing SOURCE.json | SOURCE existence gate |
| changed storage policy (config vs lock) | config hash, then policy gate |
| changed repository identity | repository gate |
| lock schema downgrade | schema gate |
| unknown --source filter | filter gate |

**Live negative-test evidence (this machine, post-relock):** the node
include rule was narrowed to `/doc/api/fs.md` in `acquisition-config.json`;
`acquire`, `materialize --source node`, and `verify` all exited 1 with
`acquisition-config.json hash does not match the frozen lock — changed
acquisition config under an existing campaign identity is rejected`. Full
workspace digest (every file, sha256-chained) before vs after the rejected
operations differed ONLY by the deliberate config write; after reverting
the tamper the digest matched the pre-test value exactly
(`b7d7c044…20da57b`). **No source byte changed at any point.**

### 2. MAJOR — acquisition globs formed an unmeasured sampling frame → FIXED

For every pinned repository, the **complete** Markdown inventory
(`.md`/`.markdown`, case-insensitive) is now enumerated from the pinned
commit tree (`git ls-tree -r`, blob ids) with byte sizes from the object
database (`git cat-file`, never the working tree), independent of the
snapshot include globs. Per-source artifacts
`manifests/inventory/<source_id>.json` record for EVERY repository Markdown
file: `upstream_path`, `git_blob_sha1`, `bytes`, `selected_for_snapshot`,
and the mechanical `selection_reason` (`included:<glob>` /
`excluded:<glob>` (include:<glob>)` / `not matched by any include
pattern`).

`manifests/inventory-summary-v1.json` reports the measured frame:

```text
REPOSITORY_MD_FILES  5,752        SNAPSHOT_MD_FILES  3,970
REPOSITORY_MD_BYTES  92,303,767   SNAPSHOT_MD_BYTES  66,391,919
FILE_COVERAGE        0.6902       BYTE_COVERAGE      0.7193
```

Per-source coverage (source_id, repo_md/snap_md files, byte coverage):

```text
cpp-core-guidelines    7/1     0.9703     openapi           60/23   0.9297
crafting-interpreters 91/50    0.1299     openmlsys        251/234  0.9768
cs231n                66/30    0.7494     opentelemetry    195/99   0.5012
d2l-en               209/191   0.9637     owasp            157/122  0.8903
ethereum-eips       1010/956   0.8370     progit             4/3    0.9114
graphql-spec          21/15    0.5144     rust-book        478/112  0.4420
kubernetes-keps      692/678   0.9979     rust-rfcs        657/651  0.9968
myst-parser           82/27    0.4410     swift-evo        602/580  0.9610
mystmd               446/89    0.5120     node             683/70   0.2056
oci-image             19/18    0.9955     oci-runtime      22/21    0.9921
```

**OpenMLSys v2 is no longer silently absent**: the v2 chapter globs were
added to the frozen frame (v2 was never excluded for redundancy or
performance — neither was ever measured — but by an undocumented v1-only
scope decision). OpenMLSys is now 234/251 files (0.9768 byte coverage); the
remaining unselected repository Markdown (root/CONTRIBUTING) is recorded in
the inventory. Every include rule is now an explicit acquisition-scope rule
with a `scope_rationale` in `acquisition-config.json` (d2l-en, openmlsys,
node, openapi, OCI included), and everything outside a frame is enumerated
with counts and bytes so the scope decision is measurable before static
characterization. No syntax was characterized; H0-H4 untouched.

### 3. MAJOR — public vendoring needs a redistribution gate → FIXED

`storage_policy` (`VENDORED` | `MATERIALIZE_ONLY`) is now a required,
validated field per source, with a recorded `storage_policy_rationale`.
Hard rule, enforced at config validation AND by the replay preflight:
**`NEEDS_REVIEW` must never permit public vendoring by default.**

- `cpp-core-guidelines` = `MATERIALIZE_ONLY` (custom Standard C++
  Foundation license; NEEDS_REVIEW). Its Markdown snapshot is **not tracked
  in this repository**. `sources/cpp-core-guidelines/SOURCE.json` retains
  repository URL, pinned commit `33bcd015997f…`, upstream path
  `CppCoreGuidelines.md`, git blob id `34addb87fb77…`, expected SHA-256
  `27b53e7de839…`, and expected 841,419 bytes; `materialize` reconstructs
  the exact pinned bytes locally and verifies all three.
- Non-commercial CC sources (`progit` CC-BY-NC-SA-3.0, `openmlsys`
  CC-BY-NC-SA-4.0, `crafting-interpreters` CC-BY-NC-ND-4.0) also fail
  closed as `MATERIALIZE_ONLY`. 16 sources with permissive/attribution
  licenses (MIT, Apache-2.0, MIT OR Apache-2.0, CC0-1.0, OWFa-1.0,
  CC-BY-SA-4.0) are `VENDORED`; the final vendoring decision itself remains
  a later, separately reviewed freeze decision.
- For all 20 sources, upstream LICENSE/NOTICE/attribution files at the
  pinned commits are preserved byte-exactly as tracked provenance artifacts
  under `licenses/<source_id>/`, with per-artifact sha256/bytes/blob-id
  recorded in SOURCE.json (`verify` checks them).
- No new legal claims are made; uncertainty is recorded explicitly in the
  rationale fields.

### 4. MAJOR — 66.4 MB pre-selection corpus in permanent Git history → FIXED

Candidate acquisition material is separated from the final frozen benchmark
corpus. Candidate snapshot bytes are **removed from the tracked tree and
from this PR's history entirely** (the branch is rewritten onto master; the
blobs are not merely deleted in a later commit). Reproducibility is carried
by the tracked, hash-pinned small artifacts:

```text
source-lock.json (pins + config/tool/inventory/manifest hashes)
  + sources/*/SOURCE.json (per-file sha256/bytes/blob ids)
  + manifests/inventory/* (complete sampling frame)
  + derived manifests (universe, duplicates, summary)
      ↓ python3 tools/acquire.py materialize   (verified local materialization)
      ↓ static characterization                 (later campaign)
      ↓ representative/extremal selection       (later campaign)
      ↓ final frozen source storage decision    (later, separately reviewed)
```

No Git LFS and no external storage service was introduced. The final
tracked-storage decision for the frozen corpus is explicitly deferred.

### Additional correction — plan byte reporting → FIXED

`plan` no longer sums `ls-tree` sizes (the `-1` unknown sentinel is gone
entirely; `ls-tree` without `-l` never reports sizes). `plan` is now an
offline enumeration of the frozen frame from the lock + inventories and
reports REPOSITORY_MD_FILES / SNAPSHOT_MD_FILES / FILE_COVERAGE /
REPOSITORY_MD_BYTES / SNAPSHOT_MD_BYTES / BYTE_COVERAGE per source and in
totals, refusing to report (rather than guessing) if the inventory is
missing.

### Additional correction — verify closure assertions → FIXED

`verify` now asserts, per source: `SOURCE.file_count == len(SOURCE.files)`
and `SOURCE.total_bytes == sum(file.bytes for file in SOURCE.files)`
(`check_source_closure`), plus files[] ordering, snapshot-path layout, and
full SOURCE↔config correspondence (snapshot_policy include/exclude/mode,
storage_policy, candidate_role, role_caveat). Inventory closure
(`check_inventory_closure`) asserts selection-set == snapshot-set, count
closure, and byte-total closure; `verify --full` additionally asserts the
inventory equals the pinned-tree Markdown set exactly.

### Additional correction — focused deterministic tests → FIXED

`tools/test_acquire.py`: 45 offline unit tests (stdlib `unittest`, temp
workspaces, zero network) covering glob semantics (anchoring, `**`,
per-segment `*`, literal escaping, first-match rule), inventory selection
marking with reasons, manifest ordering (universe/duplicates/summary),
SOURCE count/byte closure, inventory closure, config validation
(NEEDS_REVIEW gate), and the full immutable-lock state machine listed
under Finding 1 — including the byte-state-invariance property for every
rejection.

### Additional correction — workload-role truthfulness → FIXED

`progit` and `crafting-interpreters` carry explicit `role_caveat` fields
(`INTENDED_BOOK_SOURCE_IS_ASCIIDOC` / `INTENDED_BOOK_SOURCE_IS_HTML`) and
`counts_as_role_coverage: false` in the candidate-universe manifest: their
snapshots must NOT be counted as BOOK_TUTORIAL Markdown workload coverage
during the later selection stage. The provenance fact is preserved; no
replacement candidates were introduced in this corrective.

## Validation evidence (all commands re-run for this report)

```text
$ python3 tools/test_acquire.py            → Ran 45 tests ... OK
$ python3 tools/acquire.py acquire         → STRICT REPLAY PASS (20 sources;
                                             derived manifests byte-identical)
$ python3 tools/acquire.py materialize     → MATERIALIZE PASS (3,970 files,
                                             66,391,919 bytes, all verified)
$ python3 tools/acquire.py verify --full   → VERIFY PASS: 3,970 materialized
                                             files byte-verified (66,391,919
                                             bytes), 0 missing; lock +
                                             inventories + derived manifests
                                             consistent; inventory-vs-tree
                                             closure checked
$ python3 tools/acquire.py plan            → frozen-frame totals (above)
$ python3 tools/acquire.py determinism-check
    materialize → digest → wipe _cache/repos AND sources/*/files →
    strict replay + re-materialize →
    pass 1 digest b7d7c0443a61f699cf49371c67b2896bc9933ffa5a9af440fa861254320da57b
    pass 2 digest b7d7c0443a61f699cf49371c67b2896bc9933ffa5a9af440fa861254320da57b
    DETERMINISM PASS: cold-cache strict replay is byte-identical
    (53 tracked artifacts + 3,970 materialized + 21 license artifacts)
$ git status                               → clean after commit; snapshot byte
                                             dirs ignored
```

## Explicit non-goals (unchanged)

No H0-H4 execution, no latency/profiling, no parser behavior runs, no
eligibility scanning, no static characterization, no representative/extremal
selection, no clustering, no weights, no source modification, no new legal
claims, no merge, no replacement candidates. PR #34 remains Draft.
