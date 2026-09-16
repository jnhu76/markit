# R2 HYPOTHESES — questions R3+ must test

Status: **R2 DRAFT FOR ADVERSARIAL REVIEW**

This file contains only hypotheses produced by prior-art extraction. No
performance numbers, no rankings, no winners. Each hypothesis is a question
a later stage (R4–R10) should be able to confirm or refute under the #22
protocol. "OBSERVED BASIS" cites the extraction record; anything not
directly proven by pinned source is a hypothesis and remains labeled as
such. Predicted weaknesses are hypotheses, not established facts about the
prior art's quality.

---

```text
HYPOTHESIS-ID:          R2-H01
SOURCE:                 mizchi-markdown.md §10 (fallback), §14
MECHANISM:              H1-shaped block-local reparse (M2)
OBSERVED BASIS:         OBSERVED — shipped code performs a TOTAL full parse
                        whenever link reference definitions exist in the old
                        or new document (incremental.mbt); no partial
                        reference handling exists.
PREDICTED STRENGTH:     strong on definition-free payloads with local edits.
PREDICTED WEAKNESS:     collapses to full rebuild on any definition-bearing
                        payload — deterministic, not probabilistic.
MINIMAL COUNTEREXAMPLE  one reference definition + one link anywhere; any
SHAPE:                  edit anywhere else.
WHICH LATER STAGE       R8 (REFERENCE_FANOUT / semantic-dependency family)
TESTS IT:               + fallback-to-full counters.
```

```text
HYPOTHESIS-ID:          R2-H02
SOURCE:                 mizchi-markdown.md §10 (INFERRED divergence classes)
MECHANISM:              H1-shaped block-local reparse (M2)
OBSERVED BASIS:         INFERRED from code shape — blank-line runs are
                        blocks; deleting the blank line between two
                        paragraphs maps the edit into one region while the
                        merged paragraph's block boundary crosses the region
                        edge; no boundary check exists.
PREDICTED STRENGTH:     as R2-H01.
PREDICTED WEAKNESS:     silent divergence (WRONG_RESULT under the #22
                        oracle) on paragraph merges; reuse of the right
                        neighbor block is unsound.
MINIMAL COUNTEREXAMPLE  "A\n\nB" → delete the blank line ("A\nB"); check
SHAPE:                  right-neighbor reuse against H0 clean parse.
WHICH LATER STAGE       R8 (block-boundary family); correctness oracle.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H03
SOURCE:                 mizchi-markdown.md §10 (INFERRED divergence classes)
MECHANISM:              H1-shaped block-local reparse (M2)
OBSERVED BASIS:         INFERRED — an unclosed fence swallows subsequent
                        blocks; the region's right edge is computed from OLD
                        spans, so reusing suffix blocks after a newly
                        unclosed fence assumes fence state cannot propagate.
PREDICTED STRENGTH:     as R2-H01.
PREDICTED WEAKNESS:     silent divergence whenever fence open/close state
                        changes across the region boundary (forward-state
                        propagation).
MINIMAL COUNTEREXAMPLE  insert "```" (unclosed) before blocks; reused
SHAPE:                  suffix must become CodeText under H0 but is reused
                        as its old kinds.
WHICH LATER STAGE       R8 (fence / forward-state family); oracle.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H04
SOURCE:                 lezer.md §4/§7 (contextHash, gap extension)
MECHANISM:              H2 fragment reuse with state-anchored absorption (M3)
OBSERVED BASIS:         OBSERVED — node absorption is gated by live goto
                        acceptance; contextHash equality applies when a
                        strict ContextTracker is configured (@lezer/lr),
                        and unconditionally via the composite-block rolling
                        hash in @lezer/markdown (M4), so container-context
                        changes mismatch.
PREDICTED STRENGTH:     strong for edits whose enclosing context is
                        unchanged (local text, same-context blocks).
PREDICTED WEAKNESS:     container edits (list depth, blockquote entry) void
                        vouching for the entire affected region; effective
                        reuse may degrade well beyond the edited bytes.
MINIMAL COUNTEREXAMPLE  indent one list item by one space (depth change)
SHAPE:                  inside a long list; measure where reuse is voided.
WHICH LATER STAGE       R8 (container family) + unique-bytes-inspected PA.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H05
SOURCE:                 lezer.md §7 (NotLast guards, fenced code identity)
MECHANISM:              Markdown-specific convergence guards (M4)
OBSERVED BASIS:         OBSERVED — reuse runs may not end inside CodeBlock
                        or list-item blocks (continuable across blank
                        lines); fenced code is reused whole (content is
                        raw CodeText).
PREDICTED STRENGTH:     soundness for continuation-ambiguity at negligible
                        predicate cost.
PREDICTED WEAKNESS:     convergence distance is block-kind-dependent: edits
                        inside fences/lists exclude reuse farther than text
                        edits of equal size; locality becomes shape-dependent.
MINIMAL COUNTEREXAMPLE  same-size one-byte edit, once inside a paragraph,
SHAPE:                  once inside a long fenced block / long list item.
WHICH LATER STAGE       R7 (edit size × position × shape) / R8.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H06
SOURCE:                 tree-sitter.md §10 (reuse shutdown), §13 Q11
MECHANISM:              H3 old-tree subtree reuse (M5)
OBSERVED BASIS:         OBSERVED — allow_node_reuse = (version_count == 1):
                        any GLR ambiguity/error split disables reuse
                        wholesale for the remainder of the parse.
PREDICTED STRENGTH:     strong on well-formed, unambiguous sources.
PREDICTED WEAKNESS:     on error-dense or ambiguous sources, the mechanism
                        silently degrades toward full-parse work while still
                        paying old-tree retention and edit mapping; fallback
                        frequency, not edit locality, dominates behavior.
MINIMAL COUNTEREXAMPLE  same local edit in two documents: one clean, one
SHAPE:                  containing an earlier unresolved construct that
                        triggers a version split.
WHICH LATER STAGE       R7/R8 + fallback-to-full / reuse counters.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H07
SOURCE:                 tree-sitter-markdown.md §3/§5/§7 (serialized state)
MECHANISM:              hidden-state gating for Markdown reuse (M6)
OBSERVED BASIS:         OBSERVED — reuse eligibility requires byte-equality
                        of the serialized scanner state: container stack
                        (depth + per-item content indent), fence delimiter
                        length, phase flags, partial-line indentation,
                        tab-stop column.
PREDICTED STRENGTH:     byte-identical subtrees in matching context reuse
                        safely; the state set is small and enumerable.
PREDICTED WEAKNESS:     a tiny container/fence edit at document start can
                        propagate invalidation forward until the serialized
                        state re-converges (or EOF) — convergence distance
                        is governed by state, not by edit size.
MINIMAL COUNTEREXAMPLE  add one ">" at the first line (enter blockquote);
SHAPE:                  every subsequent line's container state differs
                        until a matching close/re-entry.
WHICH LATER STAGE       R8 (container family) + restart/convergence
TESTS IT:               distance counters.
```

```text
HYPOTHESIS-ID:          R2-H08
SOURCE:                 tree-sitter.md §13 Q8; swift-incremental-syntax.md
                        §13 Q8; lezer.md §13 Q8; ROADMAP R6
MECHANISM:              old-tree/fragment retention (M3, M5, M7)
OBSERVED BASIS:         OBSERVED — all three families retain structure
                        proportional to document size (old tree, fragment
                        table, lookahead table) across edits.
PREDICTED STRENGTH:     update latency can approach O(edit) for suitable
                        edits.
PREDICTED WEAKNESS:     retained-state construction cost and memory are
                        paid up front and per session; for large documents
                        or infrequent edits the retained-state fixed cost
                        can exceed repeated clean parses — a crossover, not
                        a universal win.
MINIMAL COUNTEREXAMPLE  full-parse + state construction vs K clean parses
SHAPE:                  for K small; large N.
WHICH LATER STAGE       R6 (state construction surface) + R9 (retained
TESTS IT:               bytes).
```

```text
HYPOTHESIS-ID:          R2-H09
SOURCE:                 MECHANISM-MATRIX.md (position strategy classes);
                        md4c/pulldown/comrak + M2/M3/M5/M7 records §9
MECHANISM:              position/range maintenance across all mechanisms
OBSERVED BASIS:         OBSERVED — three classes extracted: emit-and-forget
                        (M1), delta-shift of retained coordinates (M2/M3),
                        patch-on-edit vs never-store/derive (M5 vs M7).
PREDICTED STRENGTH:     derived/lazy offsets (M7) shift cost to reads;
                        delta-shift amortizes over reused suffix; patch
                        front-loads into ts_tree_edit.
PREDICTED WEAKNESS:     a naive patch-everything H3 offset strategy is O(N)
                        per edit; the tree-sitter anchor patches only the
                        edited path (edit-proportional by construction,
                        subtree.c:645-815) and Swift derives offsets at
                        read time — so position cost erodes reuse mainly
                        through reconstruction and read-time derivation,
                        not through edit patching; R7/R9 must separate
                        these classes or reuse benefits get misattributed.
MINIMAL COUNTEREXAMPLE  subtree-heavy document, tiny edit; compare reuse
SHAPE:                  counters vs position-maintenance counters.
WHICH LATER STAGE       R7 (metadata/range-records-touched counters) + R9.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H10
SOURCE:                 swift-incremental-syntax.md §7/§13 Q10;
                        MECHANISM-SOURCE-MAP H4 (convergence authorities)
MECHANISM:              byte-local convergence predicates for H4 (M7 shape)
OBSERVED BASIS:         OBSERVED — Swift's predicate is byte-local
                        (position + kind + untouched lookahead range); no
                        parser-state equality exists. INFERRED — Markdown
                        block-boundary state (container depth, fence state)
                        is not byte-local.
PREDICTED STRENGTH:     very cheap per-checkpoint validation; tight bounds
                        via lookahead ranges.
PREDICTED WEAKNESS:     identical bytes under changed block context can
                        pass a byte-local predicate (false convergence /
                        WRONG_RESULT) — e.g., content that was inside a
                        fence is re-contexted as list/paragraph text.
MINIMAL COUNTEREXAMPLE  edit that re-contexts unchanged bytes (insert fence
SHAPE:                  opener/closer, change container marker before
                        identical text).
WHICH LATER STAGE       R8 (forward-state + container families); oracle.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H11
SOURCE:                 mizchi-markdown.md §8 (recursive suffix shift);
                        lezer.md §8 (buildTree spine); tree-sitter.md §8
MECHANISM:              reconstruction after reuse (all incremental families)
OBSERVED BASIS:         OBSERVED — every extracted mechanism still rebuilds
                        or re-coordinates a result whose size can be
                        proportional to the reused region, not to the edit
                        (suffix span shifts, new wrapper spines, new parents
                        around reused subtrees).
PREDICTED STRENGTH:     parse work genuinely saved; result representation
                        partially shared.
PREDICTED WEAKNESS:     "reuse" without qualification overstates savings:
                        reconstruction + re-coordination can approach the
                        cost of rebuilding; only nodes-reused vs
                        nodes-rebuilt counters expose the true trade.
MINIMAL COUNTEREXAMPLE  tiny edit early in a large document under each
SHAPE:                  mechanism; compare parse-avoidance vs reconstruction
                        counters.
WHICH LATER STAGE       R7 (nodes rebuilt/reused) + PA + R11 attribution.
TESTS IT:
```

```text
HYPOTHESIS-ID:          R2-H12
SOURCE:                 pulldown-cmark.md §8 (eager blocks / lazy inlines);
                        R1 contract §4 (completion boundary)
MECHANISM:              completion semantics of "update" (measurement model)
OBSERVED BASIS:         OBSERVED — pulldown-style architectures separate
                        eager block-tree construction from lazy inline/event
                        resolution; "when is the parse done" is a design
                        choice, not a fact.
PREDICTED STRENGTH:     lazy resolution is legitimate for streaming
                        consumers.
PREDICTED WEAKNESS:     without a pinned eager-materialization boundary,
                        "update cost" is ill-defined and lazy work can hide
                        outside measurement — the benchmark would compare
                        incomparable completion states.
MINIMAL COUNTEREXAMPLE  (protocol-level) two horses reporting "done" at
SHAPE:                  different materialization depths.
WHICH LATER STAGE       R4/R5 via EAGER_COMPLETION_VALIDATION_PASS
TESTS IT:               (already a hard gate; this hypothesis says why it
                        must stay one).
```
