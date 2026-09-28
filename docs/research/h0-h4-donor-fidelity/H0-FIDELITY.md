# H0 fidelity contract — FULL_REBUILD

Status: **FROZEN GATE-A FIDELITY CONTRACT (#76)**

```text
HORSE = H0 — FULL_REBUILD (BENCH-GRAMMAR-v1 zero-reuse full-rebuild reference)

LOCAL_MECHANISM_AUTHORITY =
  research/benchmarks/markdown-ast-update/mechanisms/full-rebuild/
  (crate markit-mdbench-full-rebuild)
  protocol/R0-METHODOLOGY.md §2–§3 (frozen horse definition)

DONOR_SOURCES =
  lineage anchors only (NOT reproduction targets):
    md4c           release-0.5.3  472c417005c2c71b8617de4f7b8d6b30411d78f4
    pulldown-cmark v0.13.4        38e4d08f14ec4bd9783270e9623db7681ebed968
    comrak         v0.55.0        6fbe87fafde3953a9f3bc582804318593d703805
  (see DONOR-SOURCES.md; evidence classes in the R1 review record)

BORROWED_MECHANISM =
  donor family property, not any single implementation:
    - clean whole-buffer parse per call; zero retained state across calls
    - block/line analysis completes before inline resolution
    - document-global reference-definition environment built in the block
      phase from the current buffer, consumed by the inline phase
    - result handed to caller; emit-and-forget coordinates
```

## D1–D12 (donor vs local; classification)

Full donor-side and local-side detail with file:line citations:
`../reviews/gate-a-r1-h0-h1-2026-09-28.md` (H0 half).

| Dim | Donor (family) | Local (H0) | Classification |
|---|---|---|---|
| D1 retained representation | nothing across parses | `H0State` = result product only; dropped before reparse, never consulted | MATCH |
| D2 edit/damage propagation | N/A (no edit input) | ignored by construction | MATCH (N/A) |
| D3 candidate discovery | N/A | none | MATCH (N/A) |
| D4 eligibility/context | N/A | none | MATCH (N/A) |
| D5 reuse/reparse granularity | full document, blocks-then-inlines | full-document `parse_full`, same two-phase order | MATCH |
| D6 candidate rejection | N/A | none | MATCH (N/A) |
| D7 fallback/progress | full parse is the only mode | same; fallback counter honestly NotApplicable | MATCH |
| D8 coordinate/edit mapping | per-call emit-and-forget offsets | spans recomputed per parse; UTF-8 bytes (R0 §6) | MATCH (mechanism level) |
| D9 semantic/global dependency | ref-def table built block-phase, consumed inline-phase | `RefTable` rebuilt from new source, first-wins, consumed by inline pass | MATCH |
| D10 retained-state maintenance | nothing to maintain | nothing | MATCH |
| D11 indexing vs enumeration | N/A (fresh transient structures) | N/A | MATCH (N/A) |
| D12 lifetime/retirement | all transient state dies at call end | `drop(old_state)` disposal boundary; per-call state ends with call | MATCH |

Grammar-scope differences (BENCH-GRAMMAR-v1 subset, Rust normalized
vocabulary, arena/callback ABI, SIMD, offset widths) are the declared
NON_MATERIAL_ADAPTATIONS of the benchmark substrate — they carry no
update-mechanism semantics.

```text
EXPERIMENT_SPECIFIC_ADAPTATION =
  BENCH-GRAMMAR-v1; common Source/Edit contract; common normalized result
  contract; Rust benchmark substrate; R0 §6 UTF-8 byte coordinates.

INTENTIONALLY_OMITTED =
  donor allocation strategies (arena/CowStr/InlineStr), SIMD/jetscii
  scanning, callback ABI shapes, 32-bit offsets, ref-def output budgets,
  upstream event ordering, spec-suite corpora.

PERFORMANCE_RELEVANT_DEVIATIONS =
  none.

CHALLENGE_CASES =
  H0-F1 old parse state cannot affect new parse result/work path —
    PASS-MECHANISM (audit_identity.rs history-independence test)
  H0-F2 global definition/reference environment rebuilt from new source —
    PASS-MECHANISM (parse_full two-phase order + golden fixtures)
  H0-F3 incremental path reuses no old syntax nodes —
    PASS-MECHANISM (nodes_reused = Known(0) on update and full_parse)

UPSTREAM_QUALITATIVE_CROSS_CHECK =
  COMPLETE (source-level; the mechanism identity rests on negative
  evidence — the donors have no incremental API — which is inherently
  source-level). All three pins SHA-verified on the clean-room clones.

CLAIM_BOUNDARY =
  "H0 is the benchmark's own BENCH-GRAMMAR-v1 zero-reuse full-rebuild
  reference and correctness authority; md4c/pulldown-cmark/comrak-INSPIRED
  at mechanism level. It is not a port of, and produces no performance
  claims about, any of the three donors." No H0 == donor equality claim
  exists anywhere in the repository.

FIDELITY_VERDICT =
  FAITHFUL_MECHANISM_MODEL
```

Open hygiene item (P3, non-blocking): `mechanisms/full-rebuild/README.md`
is stale ("R1 puts NO algorithm code here") — tracked in the Gate-A
findings; fix belongs to routine documentation cleanup, not to a mechanism
change.
