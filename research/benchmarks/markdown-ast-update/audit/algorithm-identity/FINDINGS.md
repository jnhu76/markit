# FINDINGS

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

No mechanism defect was found: every H0–H4 implementation realizes its
frozen model and every counter measures a derivable, pinned quantity.
What follows are OBSERVATIONS — properties worth recording for the
Weakness Map, the counter schema, and any future algorithm work.
None was fixed in the audit branch (production changes forbidden);
each carries its severity and the evidence to re-derive it.

## Taxonomy used

```text
MECHANISM_DEFECT      wrong algorithm / wrong tree on some input
COUNTER_NAMING        counter name vs measured unit mismatch
COUNTER_APPLICABILITY phase/horse slot convention break (Unknown vs N/A)
ATTRIBUTION_PROFILE   aggregate coverage surprising though every read
                      is declared
DOC_CLAIM_DRIFT       record/witness wording overtaken by corrective
ORACLE_CONTAMINATION  correctness oracle reachable from mechanism code
SCHEMA_HYGIENE        coordinate/version underspecification
```

Severity: P0 correctness of results, P1 systematic measurement bias,
P2 misleading aggregate, P3 hygiene/consistency.

---

## F-1 — `blocks_reparsed` counts skeleton NODES, not blocks or bytes

- Class: COUNTER_NAMING. Severity: P3.
- Evidence: H4 container join yields `blocks_reparsed = Known(3)` for
  ONE fresh list — List + Item + Para are three skeleton units
  (`skel_count`, restart-convergence lib.rs L855–L865; items carry
  their paragraphs as children). H1 counts the DISCARDED fallback
  region's skeletons too (`total_fallback` L223–L260, pinned).
  `TEST_SUPPORT` (every suite pins exact values).
- Impact: any reader interpreting the counter as "number of blocks
  reparsed" or "bytes reparsed" will mis-size work. The unit is
  frozen and consistent; only the name is coarse.
- Recommended follow-up: a one-line unit note in the counter schema
  doc, or a rename at the next schema revision (R7+).

## F-2 — H1 `full_parse` leaves `fallback_to_full_count` Unknown

- Class: COUNTER_APPLICABILITY. Severity: P3.
- Evidence: `declare_gauges_not_applicable` (block-local L950–L954)
  sets only the two gauges; H1 `full_parse` (L249–L259) never touches
  the fallback slot -> Unknown. All four siblings declare
  `NotApplicableSlot::FallbackToFullCount` on `full_parse` (H0
  `report_attribution` L158–L160; H2 L513; H3 L472; H4 L708–L711).
  On the UPDATE path H1 is exact (Known(0) via `add_fallback_to_full(0)`
  L453; Known(1) via `record_fallback_to_full` L229, pinned exactly
  once).
- Impact: consumers comparing `full_parse` rows across horses see
  Unknown vs NotApplicable for the same meaningless slot.
- Recommended follow-up: set the slot N/A in H1's full_parse (one
  line) in the next corrective — NOT in this audit branch.

## F-3 — Inspection events carry no explicit coordinate-system statement

- Class: SCHEMA_HYGIENE. Severity: P3.
- Evidence: `record_source_inspection(version, start, end)` with
  `SourceVersion::Old | Post`; the byte-offset coordinate system and
  the old->post mapping (the edit) are implicit conventions
  (`common/src/work.rs` L260, `SourceVersion` enum). AGENTS.md §8
  requires any persistent position design to state its coordinate
  system — the attribution lane is experiment-internal, so this is
  hygiene, not a violation. `CODE_INSPECTION_SUPPORT`.
- Impact: none today; a future consumer joining Old and Post intervals
  must know the mapping is the edit itself.

## F-4 — H3 attribution profile: full POST coverage via margin closure

- Class: ATTRIBUTION_PROFILE. Severity: P3.
- Evidence: H3's consult-time live-side paragraph margin reports
  `[prev_line_start-1, pos-1)` at EVERY line start, pass or fail
  (`Cursor::consult` L795–L805, reported at `update` L391–L394). On
  the canonical local edit the blank-line consults read the taken
  neighbors' lines: `unique_post_source_bytes = 31/31` while the takes
  themselves add ZERO parser reads. Pinned with full derivation in
  `h3_exact_reuse_and_change_flags`. `TEST_SUPPORT`.
- Impact: H3's PA numerator is margin-dominated; comparing horses on
  raw PA conflates "work to vouch" with "work to parse". The stage
  record's W1 wording ("sub-full inspection on safe local edits",
  R5-HORSES-STAGE-RECORD.md L248–L250) holds for H1/H2/H4 and is
  overtaken for H3 by the CORRECTIVE-2 closure (DOC_CLAIM_DRIFT
  aspect, same finding).
- Recommended follow-up: report H3 margins as a SEPARATE derived slot
  (vouch reads vs parse reads) in a future schema revision, so reuse
  comparisons separate vouching cost from parsing cost. Candidate
  Weakness Map entry: vouching cost is part of reuse's price.

## F-5 — H2/H3 hook regions span the whole document by design

- Class: ATTRIBUTION_PROFILE (informational). Severity: P3.
- Evidence: `parse_region_with_hook(post, 0, post.len())` in H2
  (L409–L414) and H3 (L381). Taken ranges are skipped by the splice,
  so scanned bytes track reuse; but a ZERO-reuse update scans the
  whole document — visible as `nodes_reused = Known(0)` +
  full coverage, never hidden. H4 avoids the shape (forward parse
  starts at `r`). `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT`.
- Impact: none for correctness; noted so Campaign-2 sampling treats
  H2/H3 zero-reuse cases as full-scan by design.

## F-6 — H4 convergence gauge semantics: EOF counts as convergence

- Class: DOC_CLAIM_DRIFT (wording). Severity: P3.
- Evidence: `convergence_pos = take.map_or(post.len(), |(_, p)| p)`
  (L665): with NO take, `convergence_distance` = post.len() - r (the
  forward pass ran to EOF). The gauge is thus "forward pass length",
  which EQUALS "convergence distance" only when a take happened.
  Every audit test pins which case it is. `CODE_INSPECTION_SUPPORT`.
- Impact: a reader must check `nodes_reused` to distinguish a true
  convergence from a no-take pass; the pair (gauge, reused) is
  unambiguous together.

## F-7 — Environmental: frozen matrix needs gitignored corpus

- Class: (not a code finding — reproducibility note). Severity: P3.
- Evidence: `workloads/.gitignore` L10 ignores `sources/*/files/`;
  `campaign` preflight tests panic on missing materialized sources in
  a fresh worktree. The audit worktree mirrors them from the main
  checkout (adds nothing to the commit). `CODE_INSPECTION_SUPPORT`.

---

## Explicit non-findings (checked, clean)

- ORACLE_CONTAMINATION: `NONE_FOUND` — dependency isolation enforced
  (anti_cheat L140–L156) + dynamic def-repair tests.
- HIDDEN_FULL_REBUILD: `NONE_FOUND` — `PARSE-RANGE-TRACE.md`.
- FRESH_STATE_ISOLATION: `PASS` — no statics/thread-locals/lazy init
  in any mechanism crate (full read); interleaving test (H1) and
  per-case fresh states everywhere. `CODE_INSPECTION_SUPPORT` +
  `TEST_SUPPORT`.
- COMPLETE_BOUNDARY: `PASS` — `complete()` seals native state only in
  all five horses (H0 L269, H1, H2, H3 L452–L459, H4 L688–L695);
  projection/checksum are post-timer exports via `NormalizeV1` /
  `ResultChecksum` impls; purity pinned in all five suites.
  `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT`.
- Timing contamination: mechanisms receive no clock
  (`MechanismContext` = sink only, mechanism.rs L65–L66); lanes are
  structurally separate payloads (lanes.rs L234–L249).

## Verdict block

```text
ORACLE_CONTAMINATION   = NONE_FOUND
HIDDEN_FULL_REBUILD    = NONE_FOUND
FRESH_STATE_ISOLATION  = PASS
COMPLETE_BOUNDARY      = PASS
MECHANISM_DEFECTS      = NONE (all observations P3)
PRODUCTION_CODE_CHANGED= NO
```
