# parser-survey-1 — run 1: influence taxonomy + hidden-O(N) gate on the P0-02 block index

Issue: #19 (MARKIT-INCREMENTAL-MARKDOWN-PARSER-SURVEY-1)
Branch: `exp/19-parser-survey-1`
Harness: `crates/parser-survey` (research-only, zero deps beyond markit-core)
Raw data: `results/raw/parser-survey/run1/` (local; `cases.csv` is the
authoritative row set). Curated summary:
`results/summary/parser-survey-1-run1.md`.

Status: **experiment started, first evidence drop.** This is run 1 of the
survey campaign; external baselines (Tree-sitter, Lezer, MD4C-class full
parse, rowan/green-tree M1 prototype) are not yet wired. No verdicts below
are final until those exist.

---

## Question

Issue #19's fundamental question, measured against the current product
implementation (`MarkdownState`, the P0-02 incremental block index):

> What is the real parse radius, metadata movement, and downstream
> invalidation of ordinary Markdown edits — and does the current
> implementation's locality survive the hidden-O(N) gate?

## Setup

- rustc 1.97.1, release profile, WSL2 x64 (see `meta.txt` for SHA/args).
- `MarkdownState::build` = M0 full-parse control; `MarkdownState::update`
  = incremental path. Structural counters come from the implementation's
  own `MarkdownWork` (honest by design, review round 1).
- Projection invalidation computed by the harness (`FakeProjectionConsumer`,
  issue #19 §12): prefix merge on identical absolute ranges, suffix merge
  on equal distance-to-EOF **and equal source bytes** (shift-tolerant;
  detail/inline IR embed absolute offsets and would misreport shifted
  survivors as content-invalidated).
- Oracle: incremental state == clean full rebuild over the public
  observable surface (kind, source range, line span, detail, inline IR).
- Medians over 5–9 measured iterations (2 warm-up), counting allocator
  around both paths.

## Workload

- Corpus B (controlled synthetic): kitchen-sink header (heading, bold-led
  paragraph, reference definition, list with lazy continuation, quote with
  lazy line, closed rust fence, mermaid fence) + repeated 2-line
  paragraphs at 1 KB / 10 KB / 100 KB / 1 MB; CJK and CRLF variants at
  10 KB.
- Corpus C (adversarial): unclosed fence, 200-deep quote, lazy
  continuation, 100 KB single-line paragraph, 1000 reference definitions,
  duplicate refs, mixed CRLF/LF, half-written Markdown, fence at BOF.
- 34 mutation cases across the issue's families (content, delimiter,
  block-boundary, container, state-propagating, semantic-global, stress);
  sweep cases run at BOF/25/50/75/EOF. 267 scenarios measured.

## Results (run 1)

### Correctness gate

**Oracle passes on all 267 scenarios**, including adversarial corpora and
every I4 cascade. The island rewind/reparse/convergence algorithm (restart
at block start with `state_before == Ground`, converge on Ground + shifted
suffix alignment) held under every mutation thrown at it.

### R — reparse amplification is flat for I0–I3, unbounded only for I4

At 1 MB (mid-position), R = bytes_scanned / changed_bytes:

| class | cases | R at 1 MB | blocks reparsed | conv distance |
|---|---|---|---:|---|
| I0/I1 content & delimiter | para/heading/fence-body/mermaid/emphasis/codespan | 24–154 | 1–2 | 1–3 lines |
| I2 boundary | blank/split/setext/heading-from-para | 39–306 | 1–3 | 2–4 lines |
| I3 container (L1 flat blocks) | list/quote marker+indent | 29–47 | 1–2 | 3–5 lines |
| I4 state-propagating | fence opener/closer/len | 95 295–**1 048 257** | 1–3 | ≈20 469 lines (EOF) |
| I5 reference def/user | ref_* | 1.1–30 | 1–3 | 2–4 lines |
| I6 mermaid body | mermaid_body_char | 35 | 1 | 3 lines |

R is **independent of document size** for I0–I3 (identical within noise
from 10 KB to 1 MB). The only forward-propagating construct in L1 is the
fence opener/closer family — supporting H2's "limited, enumerable set of
propagation mechanisms".

Two notable zero-work convergence cases: deleting a blank line whose
neighbors cannot merge (e.g. heading | paragraph) converges immediately —
**zero bytes scanned, zero blocks reparsed**, only the blank record is
removed and survivors shift. Deleting a 210 KB middle region (`large_delete_mid`)
scans only the seam (R ≈ 0.1) — 1 block reparsed.

### Hidden-O(N) gate: PARSE-INV-08 violated for every length-changing edit

 survivor range rewrites (`survivor_blocks_shifted`) and Vec tail
relocations (`block_records_moved`) scale with document size, exactly
proportional to the survivor count after the edit point:

| edit | pos | survivors shifted @1 MB | inc_us | survivors @100 KB | inc_us |
|---|---|---:|---:|---:|---:|
| para insert 1 B | BOF | 13 656 | 218.9 | 1 360 | 13.6 |
| para insert 1 B | 25% | 10 213 | 252.0 | 1 011 | 8.7 |
| para insert 1 B | mid | 6 809 | 69.6 | 673 | 6.6 |
| para insert 1 B | 75% | 3 405 | 31.4 | 337 | 4.0 |
| para insert 1 B | EOF | 1 | 4.2 | 1 | 1.6 |
| para substitute 1 B (equal length) | mid | **0** | **4.3** | **0** | ~1 |

Equal-length edits shift nothing at any size — the implementation's
"count it honestly" split is real. But any byte-count change pays O(N):
attribution at 1 MB mid: 69.6 µs (insert) vs 4.3 µs (substitute) —
**~94% of a local length-changing edit's cost is absolute-offset metadata
rewriting, not parsing.** `LOCAL_PARSE_BUT_GLOBAL_METADATA_WORK` confirmed
empirically on the current implementation.

Vec relocation adds a second O(N) term when an island changes block count
(`heading_from_para`, `para_split`, `blank_delete`: 6 809 records moved at
1 MB mid).

### Downstream (projection) invalidation

With offset-shifts excluded, D_content mirrors R for all classes: local
edits invalidate 1–3 blocks; I4 cascades invalidate everything the fence
swallows. Reference-definition edits (`ref_def_edit`, R=30) do **not**
fan out at the syntax layer — expected, since L1 has no reference index
yet; the semantic fanout half of I5 remains unmeasured (open).

### Adversarial findings — locality is block-granular, and blocks can be huge

| corpus | edit | R | note |
|---|---|---:|---|
| 200-deep quote | quote content | 42 093 | L1 flattens the whole quote run into ONE block |
| 1000 ref defs | edit one def | 16 788 | 1000 contiguous def lines = ONE paragraph block |
| 100 KB single-line paragraph | insert 1 B | 100 003 | block == 100 KB; no sub-block locality |
| unclosed fence (54 lines) | body char | 571 | honest propagation to EOF |

L1's block granularity bounds locality from below by block size. Any
"locality" claim for L1 must therefore be read as "locality up to the
smallest enclosing block".

### Incremental vs full parse crossover (H6)

At 1 MB, full parse ≈ 5.9 ms (≈5.6 ns/B). Incremental wins 85–1400× for
I0–I3. For I4 global reinterpretations (`fence_len_grow`: doc collapses to
17 fence-blocks), incremental (≈1.9–2.1 ms) is **slower** than the fresh
full parse of the collapsed document (≈1.2 ms). Architecture should allow
the crossover, per H6.

## Taxonomy verdicts (provisional, run 1)

| class | verdict | evidence |
|---|---|---|
| I0 | CONFIRMED | all content edits 1–2 blocks, size-independent |
| I1 | CONFIRMED at block granularity | delimiter edits reparse their block; upper bound = block, as predicted |
| I2 | CONFIRMED + SPLIT | merge-class (1–3 blocks) vs zero-work convergence class (blank delete between non-mergeable neighbors) |
| I3 | REFINED | container scope = whole flattened L1 block; deep-quote R=42k shows the cost of flat containers |
| I4 | CONFIRMED | fence family is the only forward propagator; EOF propagation honest; crossover vs full parse |
| I5 | SYNTAX HALF CONFIRMED | syntax radius tiny; semantic dependency fanout unmeasured (no reference index exists) |
| I6 | CONFIRMED | mermaid body == fence body for the Markdown layer |

## Hypothesis scoreboard (H1–H6)

- H1 (local convergence) — **supported** for I0–I3 on this corpus.
- H2 (limited propagation set) — **supported**: fences only, in L1.
- H3 (syntax/semantic separation) — **not yet testable** (no semantic layer).
- H4 (position-free representation avoids O(N)) — **motivated**: the O(N)
  offset-rewrite cost is now measured, not assumed.
- H5 (markdown-specific boundaries beat generic reuse) — **untested**
  (M1 prototype + Tree-sitter/Lezer baselines pending).
- H6 (full-parse crossover) — **supported** for I4 reinterpretations at
  1 MB.

## Limitations

- Single machine (WSL2), single implementation; wall-clock is attribution
  support, structural counters are the primary signal.
- Corpus B paragraphs are small and regular; large-paste and huge-paragraph
  cases cover but do not saturate the L-dependent regime.
- No M0 external baseline yet (MD4C-class); `MarkdownState::build` is the
  only full-parse reference.
- I5 semantic fanout, FakeProjectionConsumer downstream cost, and the
  Influence-Tree per-mutation records (§4) are harness TODOs.
- Wall-clock numbers include allocator counting overhead; medians damp but
  do not eliminate it.

## Next steps

1. M1 generic subtree-reuse prototype + rowan/green-tree representation
   study (Q7).
2. External baselines: tree-sitter-markdown, Lezer Markdown (via node),
   MD4C-class full parse; same corpus + oracle where applicable.
3. ReferenceIndex sketch to measure I5 semantic fanout separately from
   syntax radius (H3).
4. Corpus A: CommonMark spec examples as an oracle-expansion battery.
5. Influence-tree recording: per-mutation ancestor/suffix-convergence
   profiles for §14's validation table.
