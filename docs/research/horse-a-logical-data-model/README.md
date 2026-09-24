# Horse-A logical data-model candidate — fixed review bundle

Status: **DESIGN CANDIDATE / FRESH INDEPENDENT REVIEW REQUIRED / NOT FROZEN / NOT IMPLEMENTATION AUTHORITY**

This directory is the durable review bundle corresponding to the revised body of issue #55 after the author-side A-01—A-05 closure pass.

Review order:

1. `01-core-contracts.md` — research boundary, conceptual model, coverage, ReadyDocument, Owner/ASTPayload, coordinates, OwnerSeq/aggregates.
2. `02-restart-semantics-update.md` — restart certificate, convergence, RefTable substitution proof, facts-before-materialization, staging/commit frontier, full builder, scoped R1–R6.
3. `03-schemas-and-traces-01-15.md` — explanatory Rust-like logical schemas and semantic traces E01–E15.
4. `04-traces-16-25-complexity-verdict.md` — traces E16–E25, complexity/cost boundaries, remaining gates, author-side verdict.

The live #55 body contains the same logical candidate as one continuous document. It explicitly remains:

```text
AUTHOR ASSESSMENT = A-01..A-05 CLOSED BY PROPOSED CONTRACTS
FRESH INDEPENDENT REVIEW = REQUIRED / NOT YET RECORDED
MECHANISM IDENTITY FROZEN = NO
FALSIFICATION CONTRACT FROZEN = NO
HORSE-A IMPLEMENTATION AUTHORIZED = NO
HORSE-A PERFORMANCE COLLECTION AUTHORIZED = NO
```

Author-side verdict in this candidate:

```text
COVERAGE_CONTRACT = PASS
READYDOCUMENT_CONTRACT = PASS
OWNERSEQ_CONTRACT = PASS
OWNER_CONTRACT = PASS
ASTPAYLOAD_CONTRACT = PASS
REFTABLE_RELATION = PASS
RESTART_CERTIFICATE = PASS
CONVERGENCE_CONTRACT = PASS
STAGING_COMMIT_MODEL = PASS
FULL_BUILD_EQUIVALENCE = PASS
R1_R6_MAPPING = PASS

P0 = 0
P1 = 0
P2 = 0
P3 = 0

MECHANISM_IDENTITY_READY_TO_FREEZE = YES
READY_TO_FREEZE_FALSIFICATION_CONTRACT = YES
READY_FOR_IMPLEMENTATION = NO
```

These YES values mean only that the candidate is concrete enough to submit to a fresh independent freeze review. They are **not** the freeze decision itself.

The three deliberately preserved weaknesses remain part of Horse-A:

```text
W-A1 coarse root restart / atomic top-level Owner
W-A2 conservative ordered-definition-facts certificate
W-A3 one-record-per-node mutable AVL layout
```

Deferred mechanisms remain deferred: nested checkpoints, winner/consumer indices, stable cross-edit IDs/locators, COW/snapshots, packed/chunked layout, global reuse index, and calibrated selectors.

Local source snapshot used to update #55 had SHA-256:

```text
80949aec373df3ad7618959acf55952a02d8b1921412cbc4cf214896709cc45d
```

Per-part local SHA-256 values at bundle creation:

```text
01 fa972571f8111bd0d7adf776bcad68b32d3442f47643e13b4a004f21c9f5e551
02 5427f3a932cf05b8bb179e8d84efb69eafc6e48f73c21e88da0bf118c100c983
03 4fa86cc3b07b3abdf0a38e424494aabc2d1cc938d9cba64dda22ab2e677056db
04 6c153d7dc95d3de32376ba23e2e9d104be5ec7dfc12bec6e27e6af1fe9b67e28
```

Authority relationship:

```text
#53  = evolution roadmap / evidence triggers for later axes
#55  = current Horse-A logical mechanism candidate
#56  = prior-art reading / interpretation map, non-authoritative
PR #54 branch = durable provenance + review/design artifacts
canonical synthesis = evidence/lessons/provenance; do not promote Horse-A to frozen architecture before fresh review
```

Next gate:

```text
fresh independent review of this fixed candidate
    ↓ if PASS
explicit mechanism-freeze decision
    ↓
freeze failure-first falsification contract
    ↓
implementation authorization only after that contract is frozen
```
