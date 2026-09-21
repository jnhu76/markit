# The frozen real Markdown workload

Entrypoint for the **real Markdown workload** of
**#22 MARKIT-MARKDOWN-BENCHMARK-1** (Stage A, constructed under **#35**).
This document is navigation and usage only — **not** a new authority layer:
every number and rule below is owned by a file named in §2.

Authority chain: #22 → `protocol/R0-METHODOLOGY.md` →
`protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` → #35 Stage A
(CORRECTIVE-A/B/C, PRs #37/#38/#39) → the frozen artifacts.

## 1. What is the workload?

```text
37 complete real Markdown files
logical roles:
- REPRESENTATIVE_SET   18 files     - SYNTAX_COVERAGE_SET  2 files + 1 repair
- EXTREMAL_SET         12 files     - FULL_DOCUMENT_SET     7 files

FULL_READ:            37 records (22 of them strict G0 cases)
EDIT_WRITE:          449 payloads total
G0 strict H0-H4:     362 EDIT_WRITE cases
trace directions:    329 BREAK / 120 RESTORE (209 single-step traces
                     + 120 break/restore pairs)
sources represented: 12 of the 20 locked repositories
```

The selected files are complete real Markdown documents — not truncated into
arbitrary snippets. Edits are derived from syntax already present in the real
source (a fence closer that exists, a reference definition that exists, a
table header that exists), never invented for the benchmark, and each anchor
resolves to EARLY / MIDDLE / LATE so the workload cannot collapse into one
fixed file location. The primary strict comparison lane is **BENCH-GRAMMAR-v1
/ G0**; G1 Table cases are semantic-only (the horses are not G1-qualified) and
G2 Math remains deferred.

Workload construction is **complete**, and the workload must **not** be
modified in response to later performance results. It is evidence: a mechanism
that measures badly is a finding about the mechanism, never a reason to move a
file, an anchor, an edit or a payload id.

## 2. Where is the frozen workload?

| What | File / directory |
|---|---|
| candidate/source authority | `workloads/source-lock.json` + `workloads/sources/` |
| final selected files | `workloads/selections/selected-files-v1.json` + `workloads/selections/syntax-coverage-repair-v1.json` |
| transition definitions | `workloads/payloads/transition-registry-v1.json` |
| applicability | `workloads/payloads/applicability-matrix-v1.jsonl` |
| FULL_READ | `workloads/payloads/full-read-manifest-v1.jsonl` |
| EDIT_WRITE | `workloads/payloads/edit-write-manifest-v1.jsonl` |
| traces | `workloads/payloads/trace-manifest-v1.jsonl` |
| coverage | `workloads/payloads/coverage-final-v1.json` |
| workload identity | `workloads/payloads/freeze-receipt-v1.json` |

Two kinds of artifact live side by side in `workloads/payloads/`:

```text
FROZEN WORKLOAD IDENTITY            must never change silently
  source-lock.json + sources/<id>/SOURCE.json + manifests/
  selections/selected-files-v1.json + syntax-coverage-repair-v1.json
  payloads/transition-registry-v1.json + applicability-matrix-v1.jsonl
  payloads/full-read|edit-write|trace-manifest-v1.jsonl
  payloads/coverage-final-v1.json + freeze-receipt-v1.json (digest-binds the six)
DERIVED CORRECTNESS EVIDENCE        may change after a horse correctness repair
  payloads/dry-run-cases-v1.jsonl + dry-run-report-v1.json
  protocol/R5-REAL-WORKLOAD-CORRECTNESS-CLOSURE-v1.md
```

Derived correctness evidence records *what the horses did* on the frozen
workload, so a correctness repair legitimately regenerates it. Frozen workload
identity records *what the workload is*; nothing regenerates it in place, and
changing it requires a reviewed superseding artifact
(`workloads/payloads/PAYLOAD-LIFECYCLE-v1.md`).

## 3. Original source material

### Repository provenance (canonical)

```text
workloads/source-lock.json                  frozen campaign identity
workloads/sources/<source_id>/SOURCE.json   per-file provenance + hashes
workloads/manifests/                        inventories + candidate universe
```

These bind, per file: upstream repository, immutable commit SHA, upstream
path, git blob id, SHA-256 and byte length. `workloads/tools/acquire.py` can
rematerialize the exact bytes from those pins alone — snapshot bytes come from
`git cat-file blob`, never from a working tree, so line endings are preserved
by construction. The acquisition frame is 3,970 candidate files / 66,391,919
bytes of the complete 5,752-file / 92,303,767-byte inventory (71.9% coverage).

### External archive (convenience only)

```text
External archive: Google Drive / Markit_Workload
with a corresponding SHA-256 record: markit_workload_sha256.txt
URL intentionally omitted from the public repository.
```

This is an **external convenience/archive copy**, not the canonical
experiment authority: reproducibility never depends on its availability.

## 4. What is the benchmark?

```text
workloads/       frozen data / provenance / manifests  = data + authority
semantics/       grammar lanes / profiler / transition oracle / payload semantics
profile-select/  mechanism-blind profiling and file selection
workload-freeze/ generate / verify / determinism / dry-run = generator + validator
common/ oracle/ runner/   shared substrate; runner/ = execution harness
mechanisms/      H0-H4 implementations
diagnostics/     post-freeze correctness diagnosis only = correctness debugging
```

`diagnostics/` owns no workload identity and must never be imported by a
mechanism crate. The horses are H0 `FULL_REBUILD`, H1 `BLOCK_LOCAL_REPARSE`,
H2 `FRAGMENT_REUSE`, H3 `OLD_TREE_SUBTREE_REUSE`, H4 `RESTART_CONVERGENCE`;
H0's clean full parse is the correctness authority for every comparison.

## 5. How to use the workload

### Step 1 — materialize original sources

```bash
cd research/benchmarks/markdown-ast-update/workloads
python3 tools/acquire.py materialize
python3 tools/acquire.py verify --full
```

`materialize` obtains the exact pinned upstream bytes; `verify --full` checks
source-lock closure, manifests and every materialized file byte-for-byte.

### Step 2 — verify the frozen workload

```bash
cd research/benchmarks/markdown-ast-update
cargo run -q -p markit-mdbench-workload-freeze \
  --bin mdbench-corrective-c -- verify .

cargo run -q -p markit-mdbench-workload-freeze \
  --bin mdbench-corrective-c -- determinism .
```

Current output:
```text
VERIFY_OK payloads=449 full_read_records=37 receipt_artifacts=6
DETERMINISM_OK artifacts=7 byte-identical
```

`verify` re-validates every frozen payload against the transition registry and
the receipt digests; `determinism` regenerates the artifacts twice and requires
byte-identical results. Tampering fails closed
(`./scripts/corrective-c-negative-tests.sh`).

### Step 3 — correctness dry-run

```bash
cargo run -q -p markit-mdbench-workload-freeze \
  --bin mdbench-corrective-c -- dry-run .
```

At the PR #40 correctness-closure state:

```text
FULL_READ:   22/22 strict G0 cases PASS
EDIT_WRITE:  362 cases x H0-H4
             1810 / 1810 correctness dispatches PASS
             0 wrong_result, 0 execution failures
DRY_RUN_OK correctness-only; no timing was performed
```

The dry-run is **correctness-only** — no clock, no work counters, no
performance evidence. It is **not** a performance benchmark.

### Step 4 — diagnose a correctness failure

```bash
cargo run -q -p markit-mdbench-diagnostics \
  --bin mdbench-diverge -- case . <payload_id>

cargo run -q -p markit-mdbench-diagnostics \
  --bin mdbench-diverge -- inventory .
```

`case` runs A/B/C/D isolation for one frozen payload (A = H0 clean parse of
post = oracle authority, B = the horse's own clean parse of post, C = its clean
parse of pre, D = its incremental update) and reports the first normalized
divergence; `inventory` lists every wrong dispatch with its class.

## 6. Design references

Workload and mechanism design was informed by prior-art extraction under
`prior-art/` — start at `prior-art/README.md`, then `SOURCE-MAP.md`,
`MECHANISM-SOURCE-MAP.md`, `MECHANISM-MATRIX.md`, `R2-HYPOTHESES.md`.

| Reference | What it informed |
|---|---|
| MD4C / pulldown-cmark / Comrak | clean full-parse baseline behavior; full-parse and reference-ordering evidence |
| Tree-sitter core | old-tree subtree reuse, edit mapping, reuse-validity concepts |
| tree-sitter-markdown | Markdown parser hidden state: containers, fences, context-sensitive validity |
| Lezer | fragment reuse, invalidation ranges, reuse safety |
| mizchi/markdown | block-local reparsing and block-boundary reuse/fallback ideas |
| Wagner & Graham | classical incremental parsing restart/convergence framing |
| Swift incremental syntax | checkpoint/reuse validation, lookahead/interference, incremental correctness methodology |

These references informed experimental dimensions and mechanism models. They
do **not** make the upstream implementations benchmark subjects and do **not**
prove Markit's final architecture; pinned versions and fidelity boundaries
live under `prior-art/`.

## 7. Why the workload has its major dimensions

```text
real complete files   -> external validity / realistic document regimes
E1-E6 edits           -> distinct local, structural, forward-state, delimiter
                         and dependency-update behaviors
EARLY / MIDDLE / LATE -> avoid a single fixed file location
BREAK / RESTORE       -> both transition directions; update cost and
                         correctness need not be symmetric
real syntax anchors   -> no fabricated constructs the file never contained
controlled synthetic  -> later causal/scaling explanation, after the real
suite                    observations exist
```

The real-vs-synthetic boundary is fixed by
`workloads/PERFORMANCE-SURFACE-BOUNDARY-v1.md`; construction rationale is in

#35 and the CORRECTIVE-A/B/C reports; performance interpretation belongs to

#31, after the frozen workload and correctness gates.

## 8. Reviewing this workload

Verify in this order:

```text
1. Do source hashes/provenance match?
2. Does freeze-receipt validate?
3. Are selected files / payload ids unchanged?
4. Do transition-registry assertions match the payloads?
5. Does the correctness dry-run pass?
6. Is a proposed change modifying workload identity or only derived evidence?
7. Is any performance-driven change trying to move the frozen workload?
```

Do not infer design rationale only from current code. For mechanism rationale
read `prior-art/`; for workload-construction rationale read #35 and the
CORRECTIVE-A/B/C reports; for performance interpretation read #31, and only
after the frozen workload and correctness gates.
