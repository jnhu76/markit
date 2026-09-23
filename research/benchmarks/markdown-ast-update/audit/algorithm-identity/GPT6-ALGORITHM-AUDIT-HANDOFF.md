# GPT6-ALGORITHM-AUDIT-HANDOFF

Evidence-first handoff for the independent algorithm reviewer ("GPT6").
No persuasive narrative; only anchors, derivations, and decision
points. Audit authority:
`0bf678cc1505098e6afe26cb8ec54cda24115831` (branch
`research/31-algorithm-identity-audit-1`).

## 1. What to review, and the only decisions that matter

1. Do you agree each horse's `update` path executes the algorithm its
   frozen section claims? (Verdicts to confirm or overturn:
   `H0..H4_ALGORITHM_IDENTITY`.)
2. Do you agree each counter measures a pinned, derivable quantity?
   (Verdicts: the 11 counter lines in `COUNTER-SEMANTICS-MATRIX.md`.)
3. Do you find any mechanism defect the audit missed? If yes: record,
   minimal reproducer, STOP — the audit contract forbids fixing in
   this branch.

## 2. Primary evidence (re-derive from these, not from the summaries)

Code (all at the audit SHA, paths relative to
`research/benchmarks/markdown-ast-update/`):

- Substrate: `shared-grammar/src/parser.rs` (scanner loop L325–L358;
  splice L373–L400; reported scans L1024–L1057),
  `shared-grammar/src/inline.rs` (L130, L263–L275),
  `common/src/work.rs` (WorkCounters L106+, CounterSink L274,
  finalize_derived L313), `common/src/mechanism.rs` (L65–L167),
  `runner/src/orchestrate.rs` (L347–L390),
  `instrumentation/src/lanes.rs`.
- H0: `mechanisms/full-rebuild/src/lib.rs` (287 lines; read whole).
- H1: `mechanisms/block-local/src/lib.rs` (953 lines; key: tiling
  state L60–L170, `total_fallback` L223–L260, `full_parse` L249,
  prepare L261+, update L278+, guards L585–L749, shifts L750–L810,
  counting L912+, gauges helper L950).
- H2: `mechanisms/fragment-reuse/src/lib.rs` (1537 lines; key: MIN_GAP
  L59, FNode L105, Fragment L178, prepare L301, update L337, windows
  L530/L587, consult L642, N/A helper L511, env clause L444,
  accounting L469, mentions_reference L1054, rematerialize L1079,
  rebuilt_table L1219).
- H3: `mechanisms/old-tree-subtree-reuse/src/lib.rs` (1591 lines; key:
  TNode/TEntry/TTree L80–L198, prepared L223, full_parse L280,
  prepare L290, update L340, N/A L471, patch_tree L496, patch_node
  L689, mark_changed L724, consult L782, find_run L850, search_level
  L861, mentions_reference L1114, rematerialize L1140,
  rebuilt_table L1269, assemble L1300–L1392).
- H4: `mechanisms/restart-convergence/src/lib.rs` (989 lines; read
  whole; anchors in `IMPLEMENTATION-MAP.md`).
- Frozen contract: `protocol/R5-HORSE-CORRECTNESS-PARITY.md` (whole),
  `protocol/R5-HORSES-STAGE-RECORD.md` (witnesses at L244–L256,
  L508, L548), correctives inside those documents.

Tests (added by this audit; all green):

- `mechanisms/{full-rebuild,block-local,fragment-reuse,old-tree-subtree-reuse,restart-convergence}/tests/audit_identity.rs`
  — 40 tests, hand-derived exact values. THE derivation comments are
  part of the evidence: each test's doc comment contains the full
  arithmetic (spans LF-exclusive; counting rule R5 §11.6).

## 3. Known subtleties that cost the audit time (save yourself the work)

1. **Single-sink contract.** `finalize_derived` recomputes derived
   slots from ITS OWN event log only. A test helper with per-phase
   sinks silently drops prepare-phase events (H3/H4 margins). The
   runner is correct; test helpers must copy it. (The audit's own
   helpers got this wrong first — see `IMPLEMENTATION-MAP.md` §
   test-side seam.)
2. **Spans exclude the terminating LF.** Para/Heading/etc. end at the
   LF position (`flush_para`, parser.rs L830–L840: `end: p.last_end`).
   Every hand-derived interval depends on this.
3. **Post-take silence.** After a splice the scanner jumps; taken
   lines are never scanned or reported; only the carried tail byte is
   (`splice_to` L384–L392). Counter derivations must NOT include line
   events inside takes (the audit over-counted twice before catching
   this — e.g. H4 canonical edit: no consult at line 30, no line
   event in [20,31)).
4. **H3 new state carries no change flags.** Flagged nodes are never
   taken (search_level L875), so the assembled tree's changed_count
   is always 0; the damage map is observable ONLY on `H3Prepared.tree`
   (cloned before update consumes it).
5. **skel_count recursion.** List items carry their paragraphs
   (Item{children:[Para]}), so a fresh one-item list = 3 skeleton
   units. H4/H2/H3 `nodes_*` pins depend on this.
6. **H4 consult order**: (a) exact q, (b) key, (c) gen, (d) >=
   damaged_end, (e) blank margin (read reported even on failure) —
   and consults fire at EVERY line start (including blank lines),
   which is where the (e)-margin coverage comes from.
7. **H4 probe short-circuit**: when `damaged_has_def` is true, the
   `]: ` probe does NOT run and reports nothing (Rust `||`), pinned by
   the definition-damage test.
8. **CJK byte math**: "中文第一段" = 15 bytes; `"第二段"` starts at
   byte 23 in that doc. Gauges are BYTES (15/20 in the CJK test).

## 4. Findings open for disagreement

`FINDINGS.md` F-1..F-7 (all P3, none a defect). The two most worth an
independent look:

- F-4: is H3's full-POST-coverage profile (margin closure reading
  taken neighbors' lines) acceptable as an attribution profile, or
  does the stage record's W1 wording constitute a claim Campaign data
  relied on? (Audit position: wording drift only; reads all declared;
  PA numerators are per-horse anyway.)
- F-2: H1 full_parse fallback slot Unknown vs siblings' N/A — schema
  hygiene; should be a one-line corrective eventually.

## 5. Reproduction commands

See `README.md` § Reproduction. Environmental note (F-7): mirror the
gitignored `workloads/sources/*/files/` corpus before running the
frozen matrix or campaign preflight.

## 6. Outputs to confirm (exact strings)

```text
H0_ALGORITHM_IDENTITY = PASS
H1_ALGORITHM_IDENTITY = PASS
H2_ALGORITHM_IDENTITY = PASS
H3_ALGORITHM_IDENTITY = PASS
H4_ALGORITHM_IDENTITY = PASS
ORACLE_CONTAMINATION  = NONE_FOUND
HIDDEN_FULL_REBUILD   = NONE_FOUND
FRESH_STATE_ISOLATION = PASS
COMPLETE_BOUNDARY     = PASS
COUNTER_SEMANTICS     = 11/11 PASS (F-1, F-2 observations P3)
FULL_EVIDENCE_CAMPAIGN_2_STARTED = NO
```
