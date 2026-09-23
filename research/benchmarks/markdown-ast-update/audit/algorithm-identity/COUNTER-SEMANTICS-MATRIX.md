# COUNTER-SEMANTICS-MATRIX

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.
Question: does each counter measure the event its name claims, for each
horse and each phase?

Substrate: `common/src/work.rs` — `WorkCounters` (L106+),
`WorkSink::record_source_inspection` (L260), `CounterSink` (L274) with
the raw event log (`inspections`, L276/L293) and `finalize_derived`
(L313) which recomputes the derived slots FROM THIS SINK'S EVENT LOG
ONLY (per-version union). `Observed<T>` = Known / Unknown /
NotApplicable (`common/src/observed.rs` L26).

## S1 SOURCE_INTERVAL_SEMANTICS — PASS

`record_source_inspection(version, start, end)` events are half-open
UTF-8 byte intervals in one of two coordinate systems, tagged:
`Old` = the retained old source, `Post` = the source being parsed
(parser.rs `line_start_of_reported_in` L1024–L1033 with explicit
version for the Old-tagging variant). Both endpoints are byte offsets
in that version's coordinates. UTF-8/CJK: all five audit suites pin
byte-exact expectations on CJK docs (e.g. H4 restart/convergence
distances of 15/20 BYTES around 3-byte characters).
`CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT`.

Caveat recorded as FINDING F-3 (P3): `InspectionEvent` pairs carry no
explicit coordinate-space/version pairing beyond the `SourceVersion`
tag, and the tag is `Old | Post` — the mapping between versions is the
edit itself. Adequate for attribution; any future persistent index
design must restate coordinates (AGENTS.md §8).

## S2 UNIQUE_INSPECTION — PASS (derived, single-sink contract)

`unique_old_source_bytes` / `unique_post_source_bytes` = byte size of
the per-version UNION of inspection events, computed by
`finalize_derived` (work.rs L313+) from the raw event log.
Consequence (pinned by this audit): the events must all land in ONE
sink across prepare+update — exactly the runner's A-LANE
(`orchestrate.rs` L366–L383). A per-phase sink silently drops the
prepare events (finalize overwrites from its own log). The audit test
helpers reproduce the runner shape; H1/H3/H4 prepare phases DO emit
events (H3's prev margin, H4's boundary margin) and their exact values
are pinned (H3: old=7; H4: old=2/9). `TEST_SUPPORT`.

## S3 CUMULATIVE_INSPECTION — PASS

`source_bytes_inspected_total` = SUM of all event ranges (repeat
inclusive) — the effort measure, deliberately diverging from the union
when the same bytes are inspected twice (e.g. H1 probe+fallback path
inspects the same bytes twice; the divergence is pinned by
`h1_probe_plus_fallback...`: `total > unique`). `TEST_SUPPORT`.

## S4 PARSE_AMPLIFICATION — PASS (formula frozen; not re-measured)

Per the R7 freeze (protocol `R7-*` §10): 
`PA = unique_source_bytes / logical_edited_bytes` with
`unique_source_bytes = unique_old + unique_post` and
`logical_edited_bytes = max(ee - es, inserted_bytes)`; the effort
variant divides `source_bytes_inspected_total`. This audit did NOT run
any amplification measurement (forbidden); it verified the FORMULA
inputs are exactly the S2/S3 slots above. 
`CODE_INSPECTION_SUPPORT`.

## S5 BLOCKS_REPARSED — PASS with naming observation F-1 (P3)

Measures FRESH BLOCK-STRUCTURE UNITS (skeleton nodes incl. container
children), NOT bytes and NOT top-level blocks:

| Horse | Semantics | Anchor |
|---|---|---|
| H0 | every block unit of the full parse | `report_attribution` (full-rebuild L153+) |
| H1 | fresh region blocks (skeleton-only on the discarded fallback region too) | `total_fallback` L223–L260 |
| H2 | `built.fnodes` = fresh block skeletons | update L337+ (`add_blocks_reparsed(built.fnodes)`) |
| H3 | same as H2 | L432 |
| H4 | fresh region `skel_count` (restart adds its own full-parse count) | L666, L284 |

The name says "reparsed" but the unit is skeleton nodes: a 5-paragraph
region yields 5; a fence yields 1. Documented in every audit suite via
exact hand-derived values (e.g. H4 container join = 3: List + Item +
Para — items carry their paragraphs as skeleton children).
`TEST_SUPPORT`. Not a measurement lie — a naming coarseness (F-1).

## S6 NODES_REBUILT — PASS

Native nodes CONSTRUCTED fresh: skeleton units + every freshly
materialized inline syntax node (recursive). Verified against
hand-derived H0-tree-derived inline counts (e.g. H2 rematerialization:
rebuilt includes the rematerialized member's payload, derived
independently from the H0 tree). Anchors: H2 L463, H3 L433, H4 L667
(`built.nodes`), H1 `entry_native_nodes` L912.
`TEST_SUPPORT`.

## S7 NODES_REUSED — PASS

Native nodes entering the new state by IDENTITY (shared `Arc`), zero
parser source reads:

| Horse | What is counted | Proof |
|---|---|---|
| H0 | measured constant 0 | `report_attribution` |
| H1 | pass-through prefix/suffix ENTRIES (reconstructed by delta shift — counted as reused; their nodes are rebuilt coordinate-shifts, not re-parses) | `shift_entry_owned` L750; exact pins |
| H2 | taken run members − rematerialized members + rematerialized kept descendants | L469; split pinned exactly |
| H3 | taken members − rematerialized + kept | L438; def-env repair pinned |
| H4 | retained prefix + converged suffix blocks (`retained_block_nodes` = skel + inline) | L668; `Arc::ptr_eq` proven |

H1's inclusion of delta-shifted suffix RECONSTRUCTION under "reused"
is the frozen convention (the entries are not re-parsed; the R5
corrective counting rule counts them as reused with rebuilt shift
accounting separate). `CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT`.

## S8 METADATA_RECORDS_TOUCHED — PASS

"Structural bookkeeping records touched", horse-specific by frozen
design (R5 §3: the slot is horse-owned):

- H0: NotApplicable on update (no bookkeeping records exist).
  `report_attribution` L158–L159.
- H1: scanned entries + (fallback bookkeeping). Exact pins in suite.
- H2: consultations + take slots (+1 when the reference environment
  changed). 
- H3: prepare scanned + patched; update consultations + slots (+1 env
  changed). Canonical pin: 3+1+7+2 = 13.
- H4: prepare checkpoints-scanned + entries-scanned + backed;
  update consults + slots + assembled-checkpoint registrations +
  rebased suffix entries (+1 env changed). Canonical pins: 15/16/17/18
  by case.

All pinned exactly per test. `TEST_SUPPORT`.

## S9 H1_FALLBACK_COUNT — PASS with F-2 (P3 applicability gap)

`fallback_to_full_count`: H1-owned. Fallback path: `record_fallback_to_full()` exactly once (`total_fallback` L229) — pinned Known(1),
exactly once, on the probe path. Non-fallback update path:
`add_fallback_to_full(0)` (L453) — a measured zero. 

**F-2**: on `full_parse` (L249–L259), H1 calls
`declare_gauges_not_applicable` (L950) which sets ONLY the two gauges —
the fallback slot stays `Unknown`, while H0 (`report_attribution`
L158–L160), H2 (`declare_not_applicable` L513), H3 (L472), and H4
(L708–L711) all declare `NotApplicableSlot::FallbackToFullCount` (and
H4's same-named helper DOES set it). Unknown is honest but breaks the
sibling convention; a consumer must treat Unknown as
"not-applicable-by-phase" for H1 full_parse rows. Severity P3
(COUNTER_APPLICABILITY_MISMATCH). NOT fixed in the audit branch
(production change forbidden).

## S10 H4_RESTART_DISTANCE — PASS

`set_restart_distance(Known(es - r))` on every successful update
(L673); restart-at-zero: `Known(es)` (L289) — where r = 0, so this is
the same formula. full_parse: NotApplicable (L710). Semantics: bytes
of POST document reparsed before the retained prefix — NOT a byte
count of damage. Pinned: 0 (es=0), 5, 6 (inside fence), 11 (EOF
edit), 12 (backed-up restart), 15 (CJK bytes). `TEST_SUPPORT`.

## S11 H4_CONVERGENCE_DISTANCE — PASS

`set_convergence_distance(Known(convergence_pos - r))` (L674–L675)
where `convergence_pos` = take position, or `post.len()` when NO take
happened (parsing to EOF is "convergence with zero reuse" — the gauge
measures how far the forward pass ran, every update). restart-at-zero:
`Known(post.len())` (L290–L291). full_parse: NotApplicable. Pinned:
10 (canonical take), 18 ((e)-refusal pushes the take from 9 to 18),
23/26 (fence BREAK/RESTORE, no take), 15 (EOF continuation, no take),
20 (CJK), 32 (restart). `TEST_SUPPORT`.

## Cross-cutting verdicts

```text
SOURCE_INTERVAL_SEMANTICS = PASS
UNIQUE_INSPECTION         = PASS
CUMULATIVE_INSPECTION     = PASS
PARSE_AMPLIFICATION       = PASS (formula only; no measurement run)
BLOCKS_REPARSED           = PASS  (F-1 naming observation, P3)
NODES_REBUILT             = PASS
NODES_REUSED              = PASS
METADATA_TOUCHED          = PASS
H1_FALLBACK_COUNT         = PASS  (F-2 applicability gap, P3)
H4_RESTART_DISTANCE       = PASS
H4_CONVERGENCE_DISTANCE   = PASS
```

## H3 full-POST-coverage observation (F-4, P3, attribution profile)

H3's consult-time live-side paragraph margin reports
`[prev_line_start - 1, pos - 1)` at EVERY line start (pass or fail,
`Cursor::consult` L795–L805). On the canonical local edit this covers
taken content (consult(8) reads [0,7) — p1's own line): 
`unique_post_source_bytes = 31/31 = FULL`, although the takes
themselves add zero parser reads. The stage record's W1 witness ("W1
witnesses assert sub-full inspection on safe local edits",
R5-HORSES-STAGE-RECORD.md L248–L250) holds for H1/H2/H4 but H3's
post-coverage is full BY THE CLOSURE RULE (every read reported),
not because H3 rescan content. Downstream PA denominators are
unaffected; the numerator for H3 is simply dominated by margin reads.
`TEST_SUPPORT` (`h3_exact_reuse_and_change_flags` pins 31/31 with the
derivation).
