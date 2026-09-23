# PARSE-RANGE-TRACE

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

Question: can a horse run a FULL parse (or rescan reused bytes) without
it being visible? The audit answers this structurally and dynamically.

## Structural argument

Every byte the SUBSTRATE reads on behalf of a horse is routed through
`WorkSink::record_source_inspection` (`common/src/work.rs` L260):

- per scanned line: `parser.rs` L343–L347 (AFTER the hook check —
  taken lines are skipped, L331–L339);
- the splice's carried tail byte: `splice_to` L385–L389;
- backward line-start scans: `line_start_of_reported_in` L1024–L1033;
- forward LF scans: `memchr_lf_reported_in` L1044–L1057;
- inline segment scans: `inline.rs` `scan_region_with_sink` L271–L275.

Every MECHANISM-side read uses the same sink (R5-CORRECTIVE-2 closure):
H3's patch margins (`patch_tree` L555–L559, L574–L578) and consult
margins (`Cursor::consult` L795–L805, buffered then reported by
`update` L391–L394); H4's boundary margin (L367–L371, L378–L383) and
convergence blank checks (buffered, reported L486–L489); H2's consult
margins and H1's guard scans (same pattern). The splice hook cannot
read at all — it receives positions and a key, and returns an
end (`SpliceHook`, parser.rs L205–L212).
`MECHANICALLY_PROVEN_BY_INVARIANT` for the substrate; the horse-side
read inventory is `CODE_INSPECTION_SUPPORT` (complete read of all five
crates at the audit SHA found no source access outside these paths and
the `Source` handed to the Mechanism trait methods).

## Dynamic argument

The audit suites assert exact union sets, which subsume range tracing:

- H0: POST coverage == the whole document, one interval
  (`h0_complete_post_coverage`).
- H1: safe local edit POST union == [9,22) exactly on DOC3; probe +
  fallback path makes `source_bytes_inspected_total > unique_post`
  (cumulative honesty).
- H2: interior of the taken run ([20,150)) ABSENT from the event log;
  margin reads confined to the run's tail line; minGap 127/128 flips
  the fragment existence with the corresponding coverage change.
- H3: full event-log-derived pins — old union [8,15); post union
  [0,31) WITH the mechanism identified per event (7 parse lines, 6
  consult margins, probe, p2' segment + line offset, 2 splice carries,
  patch margins). The per-event derivation lives in the test's doc
  comment and was cross-checked against a raw event dump during the
  audit (scratch harness, removed before commit).
- H4: post union pins per case (e.g. [9,20) + carried [30,31) = 12 on
  the canonical edit) — each event attributed to its producer (probe,
  line scan, blank check, materialize, line offset).

A hidden rescan of reused bytes would necessarily add events to the
log (the substrate reports what it reads); a hidden full parse would
either add line events for taken ranges (impossible: the splice skips
them and the scanner is the only line-event producer) or bypass the
substrate (no second parser exists in the workspace reachable from the
mechanisms — see `CALL-PATH-MAP.md`).

Verdict: `HIDDEN_FULL_REBUILD = NONE_FOUND`; undeclared reads:
`NONE_FOUND` (F-4 in `FINDINGS.md` records the H3 full-POST-coverage
profile — reads all declared, aggregate surprising).
