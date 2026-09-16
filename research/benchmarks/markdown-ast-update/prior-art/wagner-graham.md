# Prior-art mechanism record — Wagner & Graham incremental parsing

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md`
§2 (prior-art role) and §3 (frozen horses H0–H4); §15 lists the 1998 TOPLAS
paper as methodology reference 1.

Evidence rules applied: OBSERVED claims cite the retrieved dissertation text
(UC Berkeley tech report CSD-97-946, which reprints the TOPLAS paper as its
Chapter 6) by chapter/section/page. INFERRED claims are labeled. UNKNOWN is
stated as unknown. No timing numbers, no performance claims of our own, no
benchmark execution. Page numbers refer to the printed dissertation pages
visible in the retrieved PDF's running headers.

## 1. Identity

```text
PAPER(S):
  P1. Tim A. Wagner, Susan L. Graham. "Efficient and Flexible Incremental
      Parsing". ACM Transactions on Programming Languages and Systems
      (TOPLAS) 20(5), September 1998, pp. 980-1013.
      DOI: 10.1145/293677.293678  (confirmed via Crossref metadata)
  P2. Tim A. Wagner. "Practical Algorithms for Incremental Software
      Development Environments". PhD dissertation, UC Berkeley, March 10,
      1998. Distributed as tech report CSD-97-946. Chapter 6 = P1;
      Chapter 7 = P3; Chapter 5 = incremental lexing; Chapters 3-4 =
      versioned document model.
  P3. Tim A. Wagner, Susan L. Graham. "Incremental analysis of real
      programming languages". PLDI proceedings, 1997, pp. 31-43.
      DOI: 10.1145/258915.258920 (Crossref: "Proceedings of the ACM SIGPLAN
      1997 conference on Programming language design and implementation",
      May 1997). Metadata only; full text not retrieved.

VENUE/YEAR/DOI: as above. R0 §15 cites P1 as "TOPLAS 20(5), 1998" — the
volume/issue agree with Crossref (volume 20, issue 5, Sept 1998, pp. 980-1013).
NOTE: P3 is dated 1997 by Crossref, by the dissertation bibliography, and by
Wagner's archived homepage ("to appear in PLDI'97"); some secondary sources
label it PLDI 1996. This record follows the retrieved metadata: 1997.

AUTHORS: Tim A. Wagner, Susan L. Graham (UC Berkeley).

PRIMARY SOURCES (retrieved 2026-09-16):
  S1. Dissertation full text PDF (CSD-97-946):
      https://www2.eecs.berkeley.edu/Pubs/TechRpts/1997/Archive/CSD-97-946.pdf
      FULL TEXT read (text layer extracted and inspected: front matter, TOC,
      Chapter 4 in full, Chapter 5 in substantial part, Chapter 6 in full,
      Chapter 7 opening, bibliography entries [98]-[101]).
  S2. Dissertation abstract page:
      https://www2.eecs.berkeley.edu/Pubs/TechRpts/1997/5885.html
      FULL abstract read.
  S3. Crossref metadata for P1 DOI 10.1145/293677.293678 (title, venue,
      volume 20, issue 5, pages 980-1013, Sept 1998, authors). METADATA ONLY.

SECONDARY SOURCES:
  S4. Archived Tim A. Wagner UC Berkeley homepage, capture 2000-12-16:
      https://web.archive.org/web/20001216002100/http://www.cs.berkeley.edu/~twagner/
      Link listing read (links to `parsing.ps` = P1 draft, `glr.ps` = P3,
      `TR.ps.Z` = dissertation). The linked PostScript files themselves were
      NOT captured by the Wayback Machine; every fetch returns an archived
      Berkeley 404 page. Listed here for provenance only.
  S5. Semantic Scholar API for DOI 10.1145/293677.293678: confirms title,
      authors, year, venue, DOI; abstract elided by publisher (returned null).
  S6. dl.acm.org PDF of P1: HTTP 403 (bot-blocked). NOT retrieved.
      CiteSeerx / dblp: not attempted further (bot verification / rate limits).
      UNKNOWN whether P1's published text differs in detail from dissertation
      Chapter 6; this record treats Chapter 6 as the mechanism authority and
      labels that mapping INFERRED (the dissertation preface explicitly states
      Chapter 6 material is the ACM-copyrighted P1 material, reprinted by
      permission — OBSERVED, preface and bibliography entry [99]).

RELEVANT SECTIONS (dissertation page numbers):
  Ch. 4 (pp. 25-31): editing model, Reference/Previous/Current versions,
    damage detection via versioning, node-reuse role.
  Ch. 5 (pp. 37-54): incremental lexing, token restart, bottom-up reuse.
  Ch. 6 (pp. 55-76): P1. §6.3 sentential-form parsing + subtree reuse;
    §6.4 optimal algorithm + Correctness (Thm 6.4.1.1) + Optimality;
    §6.5 ambiguity/"fragile" nodes; §6.6 sequences/balancing/performance
    model; §6.7 node reuse (reuse paths, bottom-up/top-down); §6.8 conclusion.
  Ch. 7 (pp. 77+): incremental GLR, parse dags (contrast mechanism; state
    matching; states stored in nodes).
```

## 2. Problem actually solved

OBSERVED (Ch. 6 opening, p. 55): batch parsers re-derive the whole structure;
"Incremental parsers retain the document's structure, in the form of its parse
tree, and use this data structure to update the parse after changes have been
made". The specific problem of P1: after an arbitrary number of mixed textual
and structural edits (unrestricted editing model, Ch. 4 §4.3), update a
persistent LR-family parse tree in time driven by the edits, not the file, and
without storing extra parser state in tree nodes. Stated bound (p. 55,
OBSERVED quote): "Our incremental parsing algorithm runs in O(t + s lg N) time
for t new terminal symbols and s modification sites in a tree containing N
nodes" — with the balancing preconditions of §6.6.

Orientation-hint correction (recorded to prevent transfer error): the task
hints described a "toplevel-scan + yield mechanism" with input partitioned
into small scan units ("yields"). In the retrieved text, "yield" is a grammar
notion — the sequence of terminal symbols derived by a subtree — not a scan
unit, and no toplevel-scan mechanism appears in Chapter 6. The actual reuse
unit is the retained subtree, not a fixed scan chunk. INFERRED: the hint
probably conflates W&G with a different prior-art line.

## 3. Retained representation

- Persistent parse tree of production-labeled nodes; terminals are tokens.
  Each node knows its production; parent/child links are versioned fields
  (Ch. 3/4; OBSERVED: version queries like `parent(previous_version)`,
  `child(i, previous_version)`, `right_sibling(previous_version)` used by the
  parser, Fig. 6.5, p. 62).
- Explicitly NOT retained (OBSERVED, p. 55): "No state information, parse
  stack links, or terminal symbol links are recorded in tree nodes. A
  transient stack is required during the application of the parsing algorithm,
  but it is not part of the persistent data structure."
- The self-versioning document (Ch. 3) retains per-field history; the parser
  reads `has_changes(reference_version)` and local/nested change marks
  (Fig. 6.4/6.6 captions, pp. 61/63, OBSERVED). Footnote 2 (p. 55, OBSERVED):
  without the history services, "two bits per node are needed to track changes
  made between applications of the parser, and the old value of each
  structural link must remain accessible until the completion of parsing."
- Contrasts (Ch. 7, p. 78, OBSERVED): the GLR variant of P3 DOES store states
  in nodes ("Lookahead information is dynamically tracked and encoded in
  parsing states stored in the nodes") — a different mechanism from P1.

## 4. Reuse unit

OBSERVED: the reuse unit is the subtree (a nonterminal node with its entire
descendant structure), plus reused tokens below it (Ch. 5) and interior nodes
on the spine (§6.7 top-down reuse). Reused subtrees are shifted whole by the
parser when "shiftable"; otherwise they are broken down one level at a time.
Not a fixed-size scan unit, not a line/block; granularity is grammar-driven.
§6.3.1 (p. 58, OBSERVED): "the fact that a subtree representing a nonterminal
is shiftable in the current parse state means that the entire subtree except
for its right-hand edge (the portion affected by lookahead outside the
subtree) can be immediately reused."

## 5. Damage detection / invalidation

OBSERVED (Ch. 4 §4.4, p. 29): three versions — Reference (last consistent
state), Previous (state at re-analysis start), Current (being written).
"The difference between the reference and previous versions determines the
document components that are potentially reusable." Damage is recorded as
local and nested change marks on nodes by the history mechanism; the parser
queries `has_changes` (Fig. 6.4/6.6). Modification sites are "either interior
nodes with structural changes or terminal nodes with textual changes" (§6.3.1,
p. 58). Textual edits are localized to the token containing the affected
characters (§4.3, p. 28 footnote 4). Splitting: "we conceptually 'split' the
tree in a series of locations determined by the modifications since the
previous parse ... the split points are based on the (fixed) number of
lookahead items used when constructing the parse table" (§6.3.1, p. 58).

## 6. Restart rule

There is no backward search for a restart point and no checkpoint table.
OBSERVED (§6.3.1-6.3.2): parsing resumes over the whole document, but at two
speeds. The tree is split at modification sites; left of the first split, the
root-path structure is loaded as the initial parse stack and unmodified
subtrees are shifted whole (Fig. 6.1 caption, p. 58: a spelling change "will
be 'sewn up' along the path of nested changes; the parser will not need to
create any new nodes"). Right of the damage, the old suffix is consumed as the
parser's input stream: "The input stream to the parser will consist of both
new material (in the form of tokens provided by the incremental lexer) and
reused subtrees; the latter are conceptually on a stack, but are actually
produced by a directed traversal over the previous version of the tree"
(§6.3.1, p. 58). Restart cost is therefore proportional to damage plus the
number of subtree boundaries crossed, under the balancing assumption — the
"restart distance" is zero-cost in work terms except for O(lg N) boundary
accesses per reused subtree (§6.3.2, p. 61). Backward movement exists only as
bounded backtracking within validation (see §7) and as decomposition
(`left_breakdown`) of lookahead subtrees. Boundedness: INFERRED from
footnote 10 (p. 63, OBSERVED quote): "If the grammar contains V nonterminals,
the amount of backtracking is limited to O(kV)."

## 7. Convergence rule (the exact comparison)

Most important section. The paper's convergence authority is NOT a comparison
of newly derived tree against old tree. It is equivalence with a batch parser
using the same tables, established at well-defined matchpoints.

OBSERVED (Theorem 6.4.1.1, p. 65): the incremental parser's configuration
matches "that of a batch parser using the same parse table" in four cases:
(1) at the beginning of the parse; (2) at the end of the parse (accept);
(3) when an error is detected (recover invoked); (4) "Immediately prior to a
shift of any non-epsilon-subtree by the incremental parser." Corollary
6.4.1.2 (p. 65): "The incremental parsing algorithm of Figure 6.6 produces the
same parse tree constructed by a batch parser reading the same terminal
yield."

Operational convergence mechanism (optimistic validation, §6.4, pp. 62-63,
OBSERVED): after shifting a reused subtree, the parser does NOT eagerly undo
the subtree's right-edge reductions (the conservative variant of Fig. 6.4
calls `right_breakdown` after every nonterminal shift; §6.3.2, p. 61, admits
"These reductions are often valid, in which case the discarded structure will
be immediately re-constructed"). Instead it parses optimistically — performs
reductions even on insufficient (subtree) lookahead — and treats a subsequent
legal shift of a non-epsilon-subtree as proof: "If one or more actions were
incorrect, the problem will be discovered before k terminal symbols past the
point of the invalid action have been shifted" (p. 62). Invalid speculative
actions are undone by a delayed `right_breakdown` (backtracking; in the k=1
case, "backtracking is merely a delayed invocation of right_breakdown",
p. 63). Correctness conditions (OBSERVED): the parse table's nonterminal
transitions must be "complete and correct" (Fig. 6.6 caption, p. 63), the
parser class must retain the viable prefix property (footnote 13, p. 65), and
GOTO lossy compression is disallowed (§6.3.2, p. 61).

So the exact stopping comparison is: speculative suffix reductions are kept
iff the parser can subsequently shift a non-epsilon symbol (canonical LR:
also validate via reduce actions on non-epsilon lookaheads, boxed code,
p. 63); equality to the batch result is a theorem at matchpoints, not a
runtime tree comparison. There is no "old tree vs new tree at unit boundary"
criterion anywhere in the retrieved text; the only old-vs-new structural
comparison is the isomorphism check in the top-down reuse post-pass
(Fig. 6.15, p. 75: reuse when `current_child` is new, the previous counterpart
was discarded, and both have the same type).

## 8. Reconstruction

OBSERVED: after the last modification, the remaining old suffix is consumed as
input: each suffix subtree is tested by the exact nonterminal-shift test
("both necessary and sufficient", §6.4.2, p. 65); shiftable subtrees are
spliced in whole (with their right-edge reductions re-validated as in §7);
non-shiftable ones are decomposed and their tokens reparsed. Reductions are
re-created only where structure must change. Node identity preservation is a
first-class goal (§6.1, p. 56; §6.7). In the common spelling-change case the
parser "will not need to create any new nodes" beyond the modified token
(Fig. 6.1 caption, p. 58; §6.7.4, p. 76: "this simple method results in only
one changed node in the entire tree: the modified token").

## 9. Position / range maintenance

OBSERVED: offsets are maintained as `<token, offset>` pairs within tokens
(Ch. 5, lexical analysis); node extents/structural navigation go through the
versioned query interface (`child(i, version)`, `parent(version)`,
`right_sibling(version)`), and Appendix A maintains character offsets for
versioned objects ("Compute offset of leftmost character ..."). Yield counts
per subtree are needed for lookahead validation and may be approximated
conservatively (footnote 18, p. 69). Line maps: UNKNOWN — no line-number
maintenance is described in the retrieved portions. The reuse machinery
needs: production id per node, changed-flags (or versioned fields), and
old-version structural access until parsing completes (footnote 2, p. 55).

## 10. Fallback

- No reference version exists (first analysis of a new document) → batch
  parse: "the initial analysis of a newly entered program has no reference
  version, since it represents a batch scenario" (Fig. 4.2 caption, p. 28,
  OBSERVED).
- Ambiguous/"fragile" regions (conflict-resolved grammars): the affected
  regions are simply re-created — "regions of the parse tree described by
  ambiguous portions of the grammar must be re-created whenever any
  modification occurs that might affect their structure" (§6.5.2, p. 68,
  OBSERVED). Frequency claim (same page): "Fragile nodes constitute a
  negligible portion of the tree across a variety of programs and languages
  studied (C, Java, Fortran, Modula-2)". No general full-reparse fallback for
  unambiguous input is described in Chapter 6. Parse errors are NOT a
  fallback trigger: error handling is entered "in exactly the same
  configuration where a batch parser would discover the error" (§6.4, p. 64);
  recovery itself is Chapter 8 (not examined in depth here).

## 11. Correctness authority

OBSERVED: the safety argument is Theorem 6.3.1.2 (p. 59: after shifting a
subtree and running `right_breakdown`, the incremental configuration is
"identical" to the batch parser's configuration) plus Theorem 6.4.1.1 /
Corollary 6.4.1.2 (§7 above): incremental result == batch parse tree on the
same terminal yield with the same tables. Optimality is argued separately
against a general shift/reduce model with subtree shifting (§6.4.2, pp. 65-66):
the exact shift test is "both necessary and sufficient", hence no algorithm
can shift less; reduction counts are asymptotically optimal under the stated
cost model. Paper-stated rules vs implementation heuristics:
- Rule (paper-stated): batch-equivalence at matchpoints; exact nonterminal
  shift test; validation by subsequent shift of non-epsilon symbols; GOTO
  completeness.
- Heuristic (implementation, labeled as such by the authors): fragile-node
  re-creation policy for ambiguous grammars (§6.5.2); first-come/first-served
  resolution of competing ambiguous bottom-up reuse sites ("The policy we
  adopt in our implementation is first-come/first-served", §6.7.2, p. 73);
  preferred practical configuration "ambiguous bottom-up reuse without the
  top-down pass" (§6.7.4, p. 76); bottom-up token reuse "simple heuristic"
  (Fig. 5.17, p. 53).

## 12. Known limitations

All OBSERVED unless noted:
- Grammar class: deterministic LR-family (LR(1)/LALR(1)/SLR(1)); ambiguity
  requires fragile-region reconstruction (§6.5) or the separate GLR machinery
  of Chapter 7.
- Balancing precondition: with conventional left/right-recursive sequences
  "parse 'trees' are really linked lists in practice ... any incremental
  algorithms degenerate to at best linear behavior, providing no asymptotic
  advantage over their batch counterparts" (§6.6, pp. 68-69). Fix requires
  grammar-level sequence notation and commit-time re-balancing (footnote 20,
  p. 70) — a representation change the paper reports cost "less than 1%" of
  grammar text for ported grammars (p. 70).
- Context-dependent yields defeat locality: the 'bad grammar' example needs
  "O(|sentence|) recomputation ... each time the leading symbol is toggled"
  (§6.6.1, p. 71).
- Table quality: LALR/SLR lossy compression and erroneous reductions can
  perform invalid reductions before an error is detected, requiring the
  `verifying` flag to avoid cycling (footnote 12 and Fig. 6.6 boxed code,
  pp. 63-65); non-canonical tables lose the stronger reduction-based
  validation (footnote 13, p. 65).
- Conservative variant (Fig. 6.4) pays spurious `right_breakdown`
  reconstruction (§6.3.2, p. 61); the optimal bound O(t + s lg N) holds for
  the optimistic variant and degrades to O(t + s (lg N)^2) without it
  (§6.3.2/§6.4, pp. 61-62).
- Criticism of prior art (for contrast, p. 57): Larcheveque's state-matching
  algorithm "exhibits linear (batch) performance in many cases. (For example,
  replacing the opening bracket of a function definition requires reparsing
  the entire function body from scratch.)" — W&G's description of Larcheveque,
  not of their own algorithm.
- Environment prerequisites: O(1) parent/child/production access; a
  versioning/history system is assumed (§6.1, p. 55; Ch. 3-4). Persistent
  space cost of parsing itself: "no persistent space cost attributable solely
  to the incremental parsing algorithm, since the syntax tree is required by
  the environment" (§6.3.2, p. 61).
- UNKNOWN: yield-granularity/per-edit overhead tradeoffs as measured numbers
  (the dissertation's empirical chapters were not the target of this
  extraction; no timing numbers are recorded here per R2 rules).

## 13. Adversarial hypotheses

All items are HYPOTHESIS for R3/R8 unless marked OBSERVED (with cite) or
UNKNOWN.

- Q1 old info surviving: OBSERVED as a design goal — versioning preserves
  annotations on reused nodes; information preservation is a stated objective
  (preface; §6.1, p. 56). HYPOTHESIS: stale annotation attachment when
  ambiguous-reuse competition (FCFS) re-attaches a node to a different site.
- Q2 who vouches: OBSERVED — batch-equivalence theorem at matchpoints
  (Thm 6.4.1.1 / Cor 6.4.1.2) under table-completeness preconditions.
- Q3 maximally invalidating edit: UNKNOWN for W&G's own algorithm (no
  experiment retrieved). OBSERVED contrast: W&G fault state-matching parsers
  for whole-body reparse on a bracket replacement (p. 57, about Larcheveque).
  HYPOTHESIS: a context-dependent-yield edit (their 'bad grammar' analogue)
  is maximally invalidating for their own bound.
- Q4 tiny edit -> O(N): OBSERVED conditional — linear only when sequences are
  represented as recursive lists (§6.6, pp. 68-69); with balanced sequences
  the bound is edit-driven and "the location of the changes does not affect
  the running time" (§6.1, p. 55). For Markdown with unbalanced container
  chains this is the direct analogue risk. HYPOTHESIS for our H4 model.
- Q5 far-forward propagation: OBSERVED mechanism — forward propagation is
  bounded by validation within k terminals past speculative actions plus the
  suffix subtree stream; far-forward structure is reused, not reparsed.
  HYPOTHESIS: only viability-prefix violations can push convergence forward.
- Q6 parse saved vs reconstruction paid: OBSERVED tradeoff — conservative
  variant pays spurious right-edge reconstruction; optimistic variant pays
  only when validation fails (§6.3.2/§6.4). Also OBSERVED (p. 73): matching-
  condition bookkeeping can cost more than "a simple parsing algorithm
  followed by a direct reuse computation".
- Q7 position maintenance: OBSERVED — versioned structural queries + token
  offsets + (approximated) yield counts; line maps UNKNOWN (absent in
  retrieved text).
- Q8 memory ~ N: OBSERVED — tree is inherently O(N) and retained anyway;
  parsing adds no persistent space (p. 61); history services add versioned
  storage (differential storage options, Ch. 3 — details not quantified in
  retrieved portions: UNKNOWN).
- Q9 hidden parser state: OBSERVED for P1 — none in nodes (p. 55); transient
  stack only; two change bits per node needed if history services are absent
  (footnote 2). OBSERVED contrast: the Ch. 7 GLR variant stores states and
  dynamic lookahead info in nodes (p. 78); §6.5 fragility adds a
  `dyn_fragility` boolean (p. 69).
- Q10 false convergence: OBSERVED risk in degraded configurations — LALR/SLR
  erroneous reductions performed before error detection (pp. 63-65), and
  ambiguous grammars can yield a wrong tree if fragile handling is omitted
  (Fig. 6.8, pp. 67-68). For unambiguous grammars with complete tables:
  claimed impossible (theorem). HYPOTHESIS: porting this to Markdown
  (no LR tables) replaces the theorem with weaker authorities.
- Q11 fallback frequency: UNKNOWN — no measurements retrieved; only the
  qualitative "negligible portion" claim for fragile nodes (p. 68) and the
  no-reference-version batch case (p. 28).
- Q12 mechanism-intrinsic vs implementation-specific: OBSERVED splits —
  intrinsic: sentential-form shift test (necessary+sufficient), matchpoint
  equivalence, validation-by-subsequent-shift, balanced-sequence requirement;
  implementation-specific: FCFS ambiguous reuse, preferred no-top-down-pass
  configuration, fragile-region re-creation choice, yield-count
  approximation.

## 14. Relevance assessment

For #22 this record is mechanism-provenance, not a subject to benchmark: W&G
is cited by R0 §3 as a prior-art anchor for H4 RESTART_CONVERGENCE ("classical
incremental parsing") and sits in §2's primary source list.

Mapping (INFERRED, hypothesis-level):
- H4 (restart + convergence + stable suffix): W&G is the strongest classical
  source for the *shape* — resume forward from before the damage, validate
  speculatively retained structure by bounded forward progress, and splice
  the stable suffix. But the fidelity boundary is sharp: W&G's convergence is
  equivalence-to-batch-parser at theorem matchpoints, operationalized by the
  exact nonterminal-shift test + k-terminal validation; it is NOT
  "reparse then compare new tree vs old tree at boundaries". Any Rust H4
  model that compares trees at unit boundaries is NOT Wagner-Graham-faithful
  and must say so.
- H3 (old-tree subtree reuse): W&G simultaneously supplies unchanged-subtree
  reuse (shift test), bottom-up reuse at reduction time, and a top-down
  isomorphic-replacement post-pass — i.e., much of the H3 concept space
  exists in the same mechanism as H4. See HORSE_BOUNDARY_AMBIGUITY below.
- H1/H2 overlap: top-down reuse restricted to modified regions resembles
  bounded local repair; reuse paths resemble fragment composition. Weakest
  mapping; not claimed as anchor.

Fidelity boundary for implementation models:
- We would NOT reproduce: LALR/LR table generation, the GLR/parse-dag
  machinery of Chapter 7, the self-versioning storage subsystem, or C++
  object identity semantics. A mechanism model borrowing only
  restart/validate/converge shape must be documented as
  "Wagner-Graham-inspired" per R0 §3's naming rule.
- Markdown has no LR grammar and no parser-state notion in the W&G sense;
  the transferable invariants are: exact (necessary+sufficient) reuse test
  ideal, bounded speculative validation with a defined rollback, and
  suffix-as-input rather than suffix-as-reparse. Whether a
  Markdown-equivalent of "nonterminal shiftable in current state" exists at
  all is an open design question for R3+, not something this paper answers.

## 15. Manifest entry

```toml
[[prior_art]]
id = "wagner-graham-1998-toplas"
name = "Wagner & Graham — Efficient and Flexible Incremental Parsing (TOPLAS 20(5):980-1013)"
source_type = "paper"
paper_url = "https://dl.acm.org/doi/10.1145/293677.293678"
doi = "10.1145/293677.293678"
retrieved_date = "2026-09-16"
relevant_sections = ["1 Introduction", "3 Sentential forms/subtree reuse", "4 Optimal parsing/Correctness/Optimality", "5 Ambiguity", "6 Sequences/performance model", "7 Node reuse"]
notes = "Full text NOT retrievable (dl.acm.org 403; Semantic Scholar abstract elided). Mechanism content read via dissertation Chapter 6 (CSD-97-946), which reprints this paper per its preface and bibliography [99]. Bibliographic metadata (vol 20, iss 5, pp 980-1013, Sept 1998) verified via Crossref DOI negotiation. Readability: full text via dissertation; metadata only via Crossref."

[[prior_art]]
id = "wagner-1998-thesis-csd-97-946"
name = "Tim A. Wagner — Practical Algorithms for Incremental Software Development Environments (PhD diss., UC Berkeley, CSD-97-946)"
source_type = "paper"
paper_url = "https://www2.eecs.berkeley.edu/Pubs/TechRpts/1997/Archive/CSD-97-946.pdf"
doi = ""
retrieved_date = "2026-09-16"
relevant_sections = ["Ch.4 Editing/analysis model + versions", "Ch.5 Incremental lexing", "Ch.6 = TOPLAS paper", "Ch.7 GLR contrast", "App.A offsets"]
notes = "PRIMARY mechanism authority for this record. Full text PDF retrieved and read (Ch.4, Ch.5, Ch.6 complete, Ch.7 opening, bibliography). Abstract page: https://www2.eecs.berkeley.edu/Pubs/TechRpts/1997/5885.html . Report number CSD-97-946; title page dated March 10, 1998. No DOI."

[[prior_art]]
id = "wagner-graham-1997-pldi"
name = "Wagner & Graham — Incremental Analysis of Real Programming Languages (PLDI)"
source_type = "paper"
paper_url = "https://dl.acm.org/doi/10.1145/258915.258920"
doi = "10.1145/258915.258920"
retrieved_date = "2026-09-16"
relevant_sections = ["(not read; metadata only)"]
notes = "Crossref: SIGPLAN 1997 PLDI proceedings, pp 31-43, May 1997 (dissertation bibliography [101] says '97 'to appear'; archived homepage says 'to appear in PLDI'97'; some secondary sources mislabel as 1996). Corresponds to dissertation Chapter 7 (incremental GLR, state matching, parse dags). Full text not retrieved; mechanism claims in this record attributed to Ch.7 are from the dissertation, not the PLDI PDF."
```
