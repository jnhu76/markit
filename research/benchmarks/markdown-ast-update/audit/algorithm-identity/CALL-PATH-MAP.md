# CALL-PATH-MAP

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.
All paths relative to `research/benchmarks/markdown-ast-update/`.

Question: who can reach the parser / inline scanner, through which
seam, and can any horse reach a FULL parse outside the frozen paths?

## The only parser entries

`shared-grammar/src/parser.rs` exposes exactly three parse entries
(CODE_INSPECTION_SUPPORT — module `pub use` surface, parser.rs L27–L45
and L277–L298):

1. `parse_full(src, sink)` — full parse, region [0, len), no hook.
2. `parse_region(src, base, end, sink)` — region parse, no hook.
3. `parse_region_with_hook(src, base, end, sink, hook)` — region parse
   with the horse-supplied splice hook.

There is no other `pub fn` that constructs a `BlockScanner`
(`BlockScanner::new` / `new_region` are private; L300–L323).
`MECHANICALLY_PROVEN_BY_INVARIANT` (visibility).

## Per-horse call paths (who calls what, from where)

| Horse | full_parse path | update path | Scanner entries used |
|---|---|---|---|
| H0 | `full_parse` (full-rebuild L223) -> shared full region parse | `update` (L249) -> full region parse of post | `parse_region` only |
| H1 | `parse_into_pending` -> `parse_region` | `update` (L278) -> `parse_region` on the damaged region; fallback via `total_fallback` (L223) -> `parse_region` on [0, len) | `parse_region` only; NO hook |
| H2 | `parse_into_pending` -> `parse_region` | `update` (L337) -> `parse_region_with_hook(post, 0, post.len())` (L409–L414) | hook = `Cursor::consult` (L642) |
| H3 | `parse_into_pending` -> `parse_region` | `update` (L340) -> `parse_region_with_hook(post, 0, post.len())` (L381) | hook = `Cursor::consult` (L782) |
| H4 | `parse_into_pending` -> `parse_region` (L230) | `update` (L432) -> `parse_region_with_hook(post, r, post.len())` (L481); restart-at-zero -> `parse_region(post, 0, post.len())` (L274) | hook = `Cursor::consult` (L739) |

The inline scanner (`inline.rs` `scan_region_with_sink` L263,
`materialize_one_with_sink` L130) is reached only from
(a) the block parser's inline pass and (b) the horses' retained-payload
rematerialization (`H2` rematerialize L1079; `H3` rematerialize L1140).
`CODE_INSPECTION_SUPPORT`.

## Hidden-full-rebuild analysis

A hidden full rebuild would require a horse update path to parse [0,
post.len()) with no take. Trace at the audit SHA:

- H0: by definition (its identity IS full rebuild) — not hidden.
- H1: update parses only the mapped damaged region; the whole-document
  parse exists ONLY behind `total_fallback` (L223), which is COUNTED
  (`record_fallback_to_full`, L229). Pinned: fallback Known(1),
  exactly once, on the probe test. `TEST_SUPPORT`.
- H2: `parse_region_with_hook(post, 0, post.len())` spans the whole
  document BY DESIGN (the fragment table is consulted per line start),
  but every taken line is skipped by the splice (parser.rs L334–L339)
  and only untaken lines are scanned/reported — pinned by the
  interior-unscanned test ([20,150) clean). A degenerate run (zero
  reuse) makes it LOOK like a full rebuild in scanned bytes, but then
  `nodes_reused = Known(0)` exposes it. `TEST_SUPPORT` +
  `PARSE-RANGE-TRACE.md`.
- H3: same shape as H2; the fence-closer test pins reused=0 with
  blocks=1 (the fence absorbs the tail — parse is NOT a clean full
  rebuild: a clean one would produce the same single fence, which is
  exactly why the counter profile + damage-map observability, not the
  tree, is the discriminator). `TEST_SUPPORT`.
- H4: forward parse starts at `r`, not 0 — a restart-at-zero is an
  EXPLICIT, gauged restart (`restart_distance = Known(es)`,
  `convergence_distance = Known(post.len())`), never silent.
  `TEST_SUPPORT`.

Conclusion: `HIDDEN_FULL_REBUILD = NONE_FOUND`. The one structural
caveat — H2/H3's hook-driven region always spans [0, post.len()) — is
visible in the counter profile by construction (unique_post /
nodes_reused), not hidden. `MECHANICALLY_PROVEN_BY_INVARIANT` (the
splice cannot both skip a range and report it).

## Oracle-reachability (contamination) paths

The correctness oracle lives in `mechanisms/full-rebuild` + 
`markit-mdbench-oracle`. Lib-side reachability from the four
incremental horses: NONE (`diagnostics/tests/anti_cheat.rs` L140–L156
enforces full-rebuild only under `[dev-dependencies]`; the workspace
Cargo graph agrees). The rebuilt reference tables (`rebuilt_table` H2
L1219 / H3 L1269; H4 defs L530–L547) are computed from each mechanism's
OWN assembled structure. `MECHANICALLY_PROVEN_BY_INVARIANT` +
`TEST_SUPPORT` (def-environment repair tests).
