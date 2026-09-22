# Real Markdown workload substrate

This directory is the **frozen real-workload provenance and artifact tree**
of #22 / #35 (Stage A), starting from the #33 round-1 acquisition substrate
(campaign `MARKIT-REAL-WORKLOAD-ACQUISITION-1`).

```text
workloads/
= frozen real-workload provenance and artifact tree

acquisition            sources/, source-lock.json, manifests/, licenses/
-> profiles / selection evidence   profiles/, analysis/, selections/
-> frozen workload manifests       payloads/ (registry, applicability matrix,
                                   FULL_READ / EDIT_WRITE / trace manifests,
                                   coverage, freeze receipt)
-> derived correctness evidence    payloads/dry-run-*.json{,l}
```

No parser runs here and no benchmark executes here: `tools/acquire.py` is
acquisition and verification only, and the frozen payloads are correctness
evidence, never measurement input. Selection happens one layer up
(`profile-select/`), and the frozen artifacts are produced by
`workload-freeze/`.

For what the workload is and how to use it, see `../WORKLOAD.md` — that is the
single workload-facing entrypoint; this file documents the acquisition
substrate beneath it.

## Acquisition layer

The acquisition layer remains strict replay over a frozen lock:

```text
acquisition ONLY:
  clone/fetch -> pin SHA -> manifest + hash -> inventory -> verify -> STOP
  (snapshot bytes: materialized locally on demand, byte-verified)
```

## Corrective 1 storage model (PR #34 human adversarial review)

Candidate snapshot bytes are **not tracked in Git** during the candidate
stage. This separates candidate acquisition material from the final frozen
benchmark corpus: the eventual corpus storage decision (which files, which
policies, tracked vs materialized) is a later, separately reviewed campaign
and cannot silently bake ~66 MB of pre-selection material into permanent
main history. Reproducibility does not depend on the bytes being tracked:

```text
source-lock.json + SOURCE.json manifests + complete Markdown inventories
        ↓  python3 tools/acquire.py materialize   (fetch pinned bytes, verify)
byte-exact local snapshot trees under sources/<id>/files/   (gitignored)
        ↓  python3 tools/acquire.py verify --full
static characterization -> selection -> final frozen-corpus storage decision
```

`sources/<id>/files/` is where materialized bytes live locally; it is
gitignored and rebuildable at any time from the pinned SHAs.

## Fail-closed lock state machine

`source-lock.json` is a **frozen lock**. Once it exists, the normal
commands are strict replay:

```text
INIT / RELOCK   (explicit, distinct, reviewed operations — the only lock writers)
    init    first-time discovery: resolve upstream HEADs, construct pins,
            snapshot, inventory, manifests, create the lock
    relock  explicit lock rebuild: --keep-pins (default; pins reused from
            SOURCE.json) or --rediscover (re-resolve HEADs = new round)

REPLAY          (acquire / materialize / verify — never write the lock)
    BEFORE any mutation, preflight verifies ALL of:
      - acquisition-config.json sha256 against the lock
      - tools/acquire.py sha256 against the lock
      - campaign identity + lock schema
      - exact source-set equality (rejects extra and missing sources)
      - every configured source has an existing pinned 40-hex SHA
      - every SOURCE.json exists and its identity matches the lock
        (rejects SOURCE identity changes / manifest drift)
      - storage-policy correspondence
    Any mismatch = FAIL with a nonzero exit BEFORE a single byte is written.

verify is bound to the SAME frozen authority: every SOURCE.json must match
its lock entry (repository_url, commit_sha, storage_policy,
source_manifest_hash) and the on-disk derived manifests
(candidate-universe / exact-duplicates / inventory-summary) must match the
hashes recorded IN the lock — recomputing from current inputs is not
authority. SOURCE drift therefore fails verify even on a fresh clone with
no materialized bytes; structural closure, config correspondence, inventory
closure, and local byte checks run on top (--full additionally requires
100% materialization and inventory-vs-tree closure).
```

Ordinary `acquire`/`materialize` therefore never re-pin, never refresh, and
never regenerate lock identity; a changed config or tool is an error, and
changing the frozen frame requires an explicit `relock` whose full diff is
the human-reviewable event.

## Directory contract

```text
workloads/
├── README.md                     this file
├── acquisition-config.json       frozen acquisition definition (inputs):
│                                 include/exclude = sampling frame with
│                                 scope_rationale; storage_policy; licenses
├── source-lock.json              frozen campaign identity (schema 2):
│                                 config/tool/inventory/manifest hashes + pins
│
├── manifests/
│   ├── inventory/<source_id>.json  COMPLETE pinned-repository Markdown
│   │                               inventory per source (see below)
│   ├── inventory-summary-v1.json   REPOSITORY_MD_* vs SNAPSHOT_MD_* coverage
│   ├── candidate-universe-v1.json  candidate membership + provenance + coverage
│   └── exact-duplicates-v1.json    byte-identical snapshot registry
│
├── sources/
│   └── <source_id>/
│       ├── SOURCE.json           machine-readable provenance + per-file
│       │                         sha256/bytes/git_blob_sha1 (tracked)
│       └── files/...             byte-exact upstream Markdown at the pin
│                                 (LOCAL ONLY — gitignored, materialized)
│
├── licenses/<source_id>/         upstream LICENSE/NOTICE/attribution files,
│                                 byte-exact at the pin, hashed in SOURCE.json
│
├── profiles/                     CORRECTIVE-A profiler contract + schema;
│                                 CORRECTIVE-B generated distributions
│                                 (distributions-v1.json/.md) plus
│                                 universe-scale derived data
│                                 (real-profile-v1.jsonl, candidate-rows-v1.jsonl
│                                 — gitignored, hash-bound in distributions)
├── analysis/                     CORRECTIVE-B: DOMAIN-STRATA input,
│                                 materialization verification, eligibility
│                                 bias, redundancy, uncovered space
├── selections/                   CORRECTIVE-B: frozen selection contract +
│                                 config, four logical sets, selection trace,
│                                 selected files, coverage report; plus the
│                                 CORRECTIVE-C deterministic SYNTAX_COVERAGE_SET
│                                 repair overlay (syntax-coverage-repair-v1.json)
├── payloads/                     CORRECTIVE-C frozen workload artifacts
│                                 (transition registry, applicability matrix,
│                                 FULL_READ / EDIT_WRITE / trace manifests,
│                                 final coverage report, freeze receipt,
│                                 dry-run report) — digest-bound by
│                                 freeze-receipt-v1.json; correctness
│                                 evidence only, never measurement input
├── CORRECTIVE-C-REPORT-1.md      CORRECTIVE-C report + G1-G8 audit + verdicts
├── TRANSITION-REGISTRY-v1.md     human authority for the frozen registry
├── traces/                       RESERVED: later canonical edit traces.
├── cases/                        RESERVED: later frozen CaseId materialization.
│
├── tools/acquire.py              deterministic acquisition/verification tool
├── tools/test_acquire.py         offline unit suite (glob semantics, lock
│                                 state machine, closures, ordering)
├── ACQUISITION-REPORT-1.md       round-1 acquisition report (as executed)
├── ACQUISITION-REPORT-2-CORRECTIVE-1.md  corrective-1 report + evidence
└── ACQUISITION-REPORT-3-CORRECTIVE-2.md  corrective-2 report + evidence
└── _cache/repos/                 rebuildable partial clones (gitignored)
```

## The complete Markdown sampling frame (inventory)

Every pinned repository's **complete** Markdown inventory
(`.md`/`.markdown`, case-insensitive) is enumerated independently of the
snapshot include globs, from the pinned commit tree (`git ls-tree -r`), with
byte sizes from the object database (never the working tree). Each entry
records `upstream_path`, `git_blob_sha1`, `bytes`,
`selected_for_snapshot`, and the mechanical `selection_reason`
(`included:<glob>` / `excluded:<glob>` / `not matched by any include
pattern`).

This makes acquisition-scope bias **measurable before characterization**:
`inventory-summary-v1.json` reports per source and in total

```text
REPOSITORY_MD_FILES  SNAPSHOT_MD_FILES  FILE_COVERAGE
REPOSITORY_MD_BYTES  SNAPSHOT_MD_BYTES  BYTE_COVERAGE
```

Round-1 totals: 5,752 repository Markdown files / 92,303,767 bytes;
candidate snapshot frame 3,970 files / 66,391,919 bytes (71.9% byte
coverage). The frame is a deliberate pre-performance scope decision recorded
per source in `acquisition-config.json` (`scope_rationale`); no
redundancy-based or performance-based exclusion is applied at acquisition.
Corrective 1 explicitly added OpenMLSys **v2** chapters (previously scoped
v1-only by an undocumented decision; 234 of 251 files now in frame).

## Byte preservation

Snapshot bytes are extracted with `git cat-file blob <commit_sha>:<path>`
— byte-exact, bypassing every checkout filter/eol conversion — and each
file's integrity is proven by recomputing the git blob object id
(`sha1("blob <size>\0" + bytes)`) against the id in the pinned commit's
tree. **Line-ending preservation is guaranteed by construction**: the
working tree is never read; bytes come straight from the object database.
The root `.gitattributes` (`workloads/** -text`) additionally prevents any
`core.autocrlf` normalization of tracked files in this subtree.

## Why both the commit SHA and per-file SHA-256 are stored

```text
repository commit SHA  !=  file SHA-256

commit SHA   -> upstream provenance/version (which commit was snapshotted)
file SHA-256 -> exact benchmark byte identity (which bytes we benchmark)
```

The commit SHA lets anyone re-derive the same content from upstream
(`git rev-parse <pin>:<path>` and compare with the recorded
`git_blob_sha1`). The file SHA-256 lets anyone verify the exact bytes
without network or upstream access. Both are needed; neither substitutes
for the other. `SOURCE.json` records per-file `sha256`, `bytes`, and
`git_blob_sha1`; `source-lock.json` additionally records a
`source_manifest_hash` per source.

## Storage policy (redistribution gate)

`acquisition-config.json` assigns each source a `storage_policy`:

- `VENDORED` — permissive/attribution license recorded from the pinned
  license material; the FINAL frozen corpus may vendor these bytes publicly
  with the preserved `licenses/<source_id>/` attribution artifacts.
- `MATERIALIZE_ONLY` — bytes must not be publicly vendored by default.
  They are materialized locally and verified against the expected
  sha256/bytes/blob-id recorded in SOURCE.json.

Hard rule (enforced by `validate_config` and by the lock preflight): a
`NEEDS_REVIEW` license state must never permit public vendoring by default.
`cpp-core-guidelines` is therefore `MATERIALIZE_ONLY`: its snapshot bytes
are not tracked in this repository; SOURCE.json retains the repository URL,
pinned commit, upstream path, git blob id, expected SHA-256, and expected
bytes so any local materialization is verifiable. Non-commercial CC
variants (`progit`, `openmlsys`, `crafting-interpreters`) also fail closed.
No new legal claims are made anywhere in this substrate; uncertainty is
recorded explicitly in `storage_policy_rationale` fields.

## Reproducing acquisition / verification

Requires: git (partial-clone + fetch-by-SHA capable), Python 3 (stdlib
only). Network is needed by `init`/`relock`/`materialize` only.

```bash
cd research/benchmarks/markdown-ast-update/workloads

python3 tools/test_acquire.py          # offline unit suite (50 tests)
python3 tools/acquire.py plan          # offline frozen-frame enumeration
python3 tools/acquire.py acquire       # strict replay: preflight + manifest closure
python3 tools/acquire.py materialize   # fetch pinned bytes locally + verify
python3 tools/acquire.py verify --full # closure + 100% byte verification
python3 tools/acquire.py report        # markdown coverage/policy table

# one-shot cold-cache evidence: materialize -> digest -> wipe cache AND
# materialized bytes -> strict replay -> digests must be identical:
python3 tools/acquire.py determinism-check
```

A fresh clone contains no snapshot bytes: `verify` (without `--full`)
checks all manifest closure offline and reports materialization coverage;
`--full` demands the bytes and verifies every one.

### Frozen workload artifacts (CORRECTIVE-C, PR #39)

After acquisition is verified, the frozen workload is regenerated and
checked from the benchmark root (never timed; correctness only):

```bash
cd research/benchmarks/markdown-ast-update

cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- generate .      # rebuild all frozen artifacts
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- verify .        # re-validate payloads + digests
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- determinism .   # two runs, byte-identical
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- dry-run .       # A8 correctness-only dispatch
./scripts/corrective-c-negative-tests.sh                                                     # tamper paths fail closed
```

`generate` applies the committed SYNTAX_COVERAGE_SET repair overlay
fail-closed; a changed base selection, drifted universe hash, or tampered
receipt is a hard error, not a warning.

### Determinism / timestamp semantics

`source_manifest_hash` = sha256 over the canonical JSON serialization
(sorted keys, no whitespace) of `sources/<id>/SOURCE.json` **excluding the
single volatile field `retrieved_at`**. The lock stores those identity
hashes plus sha256 of the config, the tool, every inventory manifest, and
every derived manifest, and is itself byte-stable. Strict replay reproduces
all derived manifests byte-identically or fails. Same lock therefore means:
same upstream versions, same snapshot bytes, same hashes — without the
bytes ever needing to be stored.

## Why candidate != final benchmark

The candidate universe is a broad, real, unpinned-in-scope input pool:

```text
candidate universe -> eligibility scan -> mechanism-neutral static
characterization -> diversity/redundancy analysis -> REPRESENTATIVE_SET +
EXTREMAL_SET freeze -> edit-trace freeze -> and only then H0-H4
measurement   (#33 Stage A authority)
```

Nothing here is eligible/representative/extreme by acquisition. Files are
not excluded for containing unsupported syntax, tables, HTML, MyST
directives, or non-BENCH-GRAMMAR math (eligibility is a later formal
phase). The only exclusions are the mechanical include/exclude sampling
frame (recorded per file in the inventories with scope rationales in the
config) and license files (preserved as provenance artifacts, not
workload).

Role truthfulness: `progit` and `crafting-interpreters` carry
`role_caveat` fields — their intended book sources are AsciiDoc and HTML
respectively, not Markdown, so their snapshots (administrative Markdown /
exercise notes) **must not be counted as BOOK_TUTORIAL Markdown workload
coverage** during the later selection stage
(`counts_as_role_coverage: false` in the candidate-universe manifest). No
replacement candidates were added in corrective 1.

No performance selection occurs here because **no performance exists here
yet**: this substrate must not (and does not) run H0-H4, measure latency or
profile parsers. Selection, set freezing and edit generation have since
happened in the sibling layers above (`profiles/`, `analysis/`,
`selections/`, `payloads/`) — all of them blind to horse performance, which is
the #33 freeze-discipline requirement this substrate is built to keep.
