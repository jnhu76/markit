# PROVENANCE — markit-markdown-h4

This crate is a **product transplant** of the H4 RESTART_CONVERGENCE
mechanism from the Markit research tree, re-owned by the product line under
the product Markdown contract (`markit-markdown-api`). Per campaign rule:
research stays untouched; the product reaches the mechanism only through
this crate; nothing here claims the research question is closed or that H4
"won".

## Donor

```text
repository:        this repository (jnhu76/markit)
donor commit:      b8730cfa3923a78bf40a37101d687b84462578ff
                   (gate-b(#76): compact implementation-parity preflight
                   + six-horse campaign freeze)
donor paths:
  research/benchmarks/markdown-ast-update/shared-grammar/src/parser.rs
  research/benchmarks/markdown-ast-update/shared-grammar/src/inline.rs
  research/benchmarks/markdown-ast-update/shared-grammar/src/lib.rs
  research/benchmarks/markdown-ast-update/oracle/src/normalized.rs
  research/benchmarks/markdown-ast-update/oracle/src/validate.rs
  research/benchmarks/markdown-ast-update/mechanisms/restart-convergence/src/lib.rs
mechanism freeze:  research/benchmarks/markdown-ast-update/protocol/R5-HORSE-CORRECTNESS-PARITY.md §9
grammar freeze:    research/benchmarks/markdown-ast-update/grammar/BENCH-GRAMMAR-v1.md
result freeze:     research/benchmarks/markdown-ast-update/grammar/NORMALIZED-RESULT-v1.md
```

## Product mapping

```text
src/grammar/          <- shared-grammar (parser.rs, inline.rs), donor-faithful
src/impls/normalize.rs    <- oracle/normalized.rs (vocabulary only)
src/impls/validate.rs     <- oracle/validate.rs (TEST-ONLY conformance gate)
src/impls/restart.rs      <- mechanisms/restart-convergence (the H4 mechanism)
src/session.rs, src/plugin.rs   BUILD_NEW (product seam: H4Service /
                        markdown_plugin implementing markit-markdown-api)
```

## Adaptations (donor -> product)

1. **Harness removal.** The `Mechanism` trait, `MechanismContext`,
   `WorkSink` counters, `Source`/`CanonicalEdit` harness vocabulary,
   `Observed`/`NotApplicableSlot` gauges, the prepare/update measurement
   split (merged into `H4Document::update`), and the post-timer
   `ResultChecksum`/`NormalizeV1` exports are benchmark instrumentation
   and were NOT transplanted. The algorithm's control flow (restart
   selection, continuation-margin backoff, damage scan, definition fast
   path, convergence predicate, splice assembly, definition-environment
   re-check, restart-at-zero) is line-faithful.
2. **Source-inspection reporting stripped.** Every
   `record_source_inspection` site and the `*_reported` helper variants
   were experiment attribution plumbing; removed (plain
   `line_start_of`/`memchr_lf` remain).
3. **Observer lane removed.** The donor's observed-region protocol
   (`RegionObserver`, `TopLevelEvent`, `RootBlankEvent`,
   `ObserverControl`, `RegionOutcome`, `ObservedRegionParse`,
   `parse_region_observed`, the scanner's
   `observer`/`prev_lf`/`stop_requested` machinery) is Horse-A research
   machinery unused by H4. Removed from the transplant; the donor archive
   retains it. Donor unit tests covering it were not ported.
4. **Edition 2024 renames.** The reserved-keyword `gen` became
   `generation`.
5. **Donor grammar unit tests** were kept where they pin grammar behavior
   (splice placeholders, EOF tails, context keys, container fences) with
   the sink plumbing removed; the research-only counter/inspection tests
   were not ported. `validate.rs` (NORMALIZED-RESULT-v1 conformance) is
   compiled `#[cfg(test)]` only — the clean-parse oracle and the
   vocabulary validator are test-side, never a second production
   authority.
6. **Dead helper surface removed.** `classify_top_level_line`,
   `LineClass`, `fence_opener_at`, `RefTable::len/is_empty` and other
   donor exports unused by H4 were dropped from the transplant.
7. **Style-only rewrites.** Nested guard `if`s collapsed to edition-2024
   let-chains at clippy's request; no semantic change. `cargo fmt` applied.

## Excluded on purpose

Benchmark runner, corpus/mutation machinery, workload definitions,
preregistration/protocol documents, measurement counters, attribution
instruments, Horse-A observed-region machinery, checksum export. None of
it is required for runtime correctness of the mechanism.

## Correctness evidence

The donor's correctness gate (`H4 update result == H0 clean parse`) is
re-proven product-side by `tests/markdown_seam.rs`: after every
incremental update (beginning/middle/end edits, large paste, CJK/emoji),
the session's headings equal the clean authoritative parse of the same
grammar, computed in the test-support module only.
