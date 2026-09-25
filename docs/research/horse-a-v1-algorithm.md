# Horse-A v1 — Frozen retained-AST update algorithm

Status: **FROZEN DESIGN SPECIFICATION CANDIDATE FOR DURABLE MERGE**

This document consolidates the already-frozen Horse-A mechanism, implementation design, and failure-first structural-locality contract into one durable implementation-facing specification.

It does **not** create a new mechanism identity. If this document disagrees with the frozen authorities listed below, the frozen authorities win until this document is corrected.

## 0. Authority and freeze record

Frozen authority chain:

```text
H0–H4 evidence / Campaign-2
        ↓
#50 / PR #51 — H4 large-N causal diagnosis
        ↓
#53 — Horse-A evolution roadmap
        ↓
#55 — Horse-A logical mechanism design
        ↓
PR #54 — durable Horse-A design/review bundle
        ↓
#59 — concrete Rust implementation design
        ↓
#60 — failure-first structural-locality contract
```

Frozen repository/design identities:

```text
PR #54 merge commit
3b9ff484e5da73f3f4510afdbcc1f3bc3d877a48

Horse-A logical design review head
d44f32345674f1e1db88fb832adb40e291da0120

#59 reviewed/frozen body SHA-256
457bd38d43ab0c53bb9547b5315cbb7e2560e7e488f088de6d38f5e21f1a416e

#60 reviewed/frozen body SHA-256
cbfa9e937d3b31c8f375cb53a6b12d142af51a15f9c52b3342a0534093369041
```

Final focused review:

```text
P0 = 0
P1 = 0

IMPLEMENTATION_DESIGN_READY_TO_FREEZE = YES
FAILURE_FIRST_CONTRACT_READY_TO_FREEZE = YES
HORSE_A_IMPLEMENTATION_AUTHORIZED = YES

STRUCTURAL_COLLECTION_AUTHORIZED = NO
PERFORMANCE_COLLECTION_AUTHORIZED = NO
```

The implementation may now be written and correctness/conformance work may be run. The frozen #60 treatment cells must **not** be collected until a later explicit collection authorization.

Merging this PR does **not** expand any authorization boundary.

### 0.1 Authority and future-change rule

This document is the implementation-facing durable consolidation after merge.

Frozen source authorities remain:

```text
#55 / PR #54
#59
#60
```

If a contradiction is found: the source frozen authority wins until this document is corrected.

Any future change to:

```text
mechanism identity
state semantics
restart/convergence
semantic preservation
operator algorithm
counter units
static bounds
thresholds
failure adjudication
```

requires an explicit new version and review. Private Rust ergonomics (module/file names, helper names, minor ownership shape) do not.

### 0.2 Trace authority

The frozen worked/adversarial traces from #55/#59 (E01–E25) remain supporting conformance authority; this consolidation does not reproduce all of them. The most decision-bearing inline case — the E24 preceding-LF certificate-touch example — is retained in §6.

---

# 1. Research identity

Horse-A exists to test one claim, quoted faithfully from the frozen #60 contract:

> For a fixed safe local paragraph edit whose syntax replacement, restart/propagation and semantic-preservation proof remain bounded as total retained size `M` grows, Horse-A must not perform work proportional to unaffected retained state merely to locate, preserve, re-coordinate, certify, count, or retire it.

This is a scoped structural-locality claim about the frozen witness class, not a global complexity claim. It does not assert that all local edits are O(log M), and Horse-A makes no claim to be the final or fastest Markdown updater.

Frozen identity:

```text
single externally current version
mutable retained representation
one complete top-level block subtree per Owner
physical-first-line source coverage
blank/interstitial source bytes belong left
RIGHT edit affinity
conservative left guard Owner
root-only parser-derived restart certificates
first valid convergence or real EOF
ordered replacement facts before semantic materialization
facts differ/unknown -> same-target full build
owner-relative spans
byte-weighted ordered sequence
one-record-per-node mutable weighted AVL
true structural split/join/range replacement
fallible staging -> PreparedCommit frontier -> no algorithmic fallback
ownership-proportional retirement
```

Explicitly not in Horse-A v1:

```text
nested continuation checkpoints
winner index / consumer postings
stable cross-edit IDs
persistent locator map
COW / historical roots / snapshot readers
packed/chunked sequence layout
global subtree/fragment reuse index
advanced calibrated cost selector
budget-based fallback selector
```

These omissions are intentional experiment boundaries, not permanent architectural prohibitions.

---

# 2. State model

Conceptual ownership:

```text
ReadyDocument
├── source association
│   ├── SourceId
│   ├── source_len
│   └── interpretation identity
├── OwnerSeq
│   └── mutable weighted AVL
│       └── AvlNode
│           ├── left / right
│           ├── height
│           ├── Aggregate
│           └── Owner
└── RefTable
```

Logical Rust shape:

```rust
struct ReadyDocument {
    source_id: SourceId,
    source_len: usize,
    interpretation: InterpretationId,
    owners: OwnerSeq,
    refs: RefTable,
}

struct OwnerSeq {
    root: Option<Box<AvlNode>>,
}

struct AvlNode {
    left: Option<Box<AvlNode>>,
    right: Option<Box<AvlNode>>,
    height: u32,
    agg: Aggregate,
    owner: Owner,
}

struct Aggregate {
    subtree_bytes: usize,
    subtree_records: usize,
    subtree_has_safe: bool,
}

struct Owner {
    coverage_len: usize,
    payload: OwnerPayload,
    outgoing_restart: Option<RestartCertificate>,
}

enum OwnerPayload {
    TriviaOnly,
    Syntax(AstPayload),
}
```

The concrete implementation may refine private Rust names and ergonomics. It may not change the state semantics above.

## 2.1 READY invariant

A `ReadyDocument` is externally usable only when:

- its source/version association matches the runner-provided immutable source;
- Owner coverage exactly partitions `[0, source_len)`;
- every syntax Owner has complete eager block/inline semantic payload;
- all persisted spans/content intervals are Owner-relative;
- document-wide `RefTable` is coherent with the complete document;
- restart certificates obey the parser-derived support contract;
- AVL aggregates are coherent;
- no parsing, semantic repair, deferred index construction, or required retirement remains to make the state correct.

There is one externally current version. Staging may temporarily hold the old READY state plus fresh candidate state, but this does not create a persistent snapshot API.

## 2.2 AstPayload responsibility

`AstPayload` (the payload of `OwnerPayload::Syntax`) is not an unexplained opaque blob. Normatively:

```text
AstPayload contains the complete eager semantic subtree for one Owner,
using Owner-relative spans/content intervals and owned semantic strings/
children sufficient for normalized export and subsequent updates.
```

It introduces no additional persistent cache beyond the state model above.

## 2.3 Payload ownership across updates

Completed retained semantic payload borrows neither `Source` nor `RefTable` across updates.

It owns whatever semantic values it must retain. It does not persist absolute document coordinates. All nested spans and content intervals remain Owner-relative; document-absolute positions are derived only at access/export time from the byte-weight prefix sums.

## 2.4 RefTable relationship

```text
RefTable.entries is the document-global source-order projection of the
complete retained ReferenceDefinition facts.
```

There is one document-global RefTable owner. There is no independently mutable second truth for definitions.

The implementation must not introduce:

```text
per-Owner mutable definition index
winner index
consumer postings
partial RefTable patch API
```

Old replacement definition facts may be extracted locally from retained AST payload during an update; this is a transient local read and does NOT create a second permanent definition table. In the local facts-preserved path the old RefTable is retained (moved exactly once, §11); it is never rebuilt or patched entry-by-entry.

---

# 3. Coverage and Owner semantics

## 3.1 Owner granularity

One Owner retains one complete **root-level top-level Markdown block subtree**.

Examples that remain one Owner even when large:

```text
Paragraph
Heading
FencedCode
BlockQuote
List
ReferenceDefinition
```

A large List/Quote/Fence is intentionally not split into finer retained Owners in Horse-A v1.

This preserves W-A1 as an intentional weakness.

## 3.2 Physical-first-line coverage

Owner coverage is based on the physical first line of each top-level block, not the semantic node span.

For `k > 0` root-level Owners with physical first-line starts:

```text
p0 < p1 < ... < p_(k-1)
```

define the coverage cuts:

```text
c0 = 0
c_i = p_i          for 0 < i < k
c_k = source_len
```

then:

```text
Owner_i.coverage = [c_i, c_(i+1))
```

Consequences of this frozen definition:

- the leading trivia `[0, p0)` belongs to the first Owner;
- blank/interstitial bytes between two semantic blocks belong to the left Owner;
- trailing trivia belongs to the last Owner, up to `source_len`;
- the union of all coverage is exactly `[0, source_len)` — no gaps, no overlap;
- empty input has an empty `OwnerSeq`;
- all-whitespace non-empty input is represented by one `TriviaOnly` Owner;
- semantic spans need not cover trivia bytes;
- source coverage and semantic span are distinct concepts.

The `TriviaOnly` rule applies according to the frozen grammar's actual whitespace classification: "all-whitespace" here means the shared BENCH-GRAMMAR-v1 root blank class (`SPACES* LF` lines only). TAB/CR-only input is ordinary text under the shared grammar (an ordinary paragraph) and does not use the `TriviaOnly` special case.

Required invariant:

```text
Owner coverage partitions [0, source_len)
with no gap and no overlap.
```

## 3.3 RIGHT edit affinity

Boundary insertions use the frozen RIGHT affinity rules. Coverage damage must include the affected seam and the conservative left guard where required.

---

# 4. Relative coordinates

For an Owner:

```text
owner_base = byte-weight prefix sum before that Owner
```

For a nested semantic span:

```text
absolute_span = owner_base + relative_span
```

All retained nested spans, including `FencedCode.content`, are Owner-relative.

An insertion before an untouched suffix therefore changes only the derived suffix bases, not every retained suffix payload.

Complexity boundaries:

```text
locate Owner by document byte = O(H)
document absolute base for an Owner = byte-weight prefix sum (sequential
    traversal for whole export; no per-Owner root seek)
once an individual retained span is reached, converting its Owner-relative
    offset to an absolute document offset = O(1)
```

AST navigation cost inside a retained payload is a separate concern and is not part of the coordinate-projection claim above.

Whole export must not perform one root seek per Owner.

---

# 5. Parser observation seam

Horse-A does not implement a second Markdown parser. It exposes facts from the existing BENCH-GRAMMAR-v1 `BlockScanner`.

Required observations:

```text
TopLevelStart { physical_line_start }
RootBlankBarrier {
    line_start,
    line_lf,
    cut,
    preceding_lf,
}
```

## 5.1 Root blank issuance

A reusable interior root certificate is issued only after the actual parser has consumed a real physical `SPACES* LF` root blank and completed the corresponding grammar actions:

```text
consume line
→ strip_prefixes
→ classify via the real grammar
→ flush paragraph if required
→ close required containers
→ emit completed blocks
→ inspect live state
```

The event may be issued only when:

```text
frames empty
para none
fence none
all previous output already sealed/emitted
```

It is a **pre-EOF** observation. `finish()` may not manufacture an ordinary restart certificate.

`ContextKey::default()` alone is not a certificate.

Blank-looking text inside a fence is fence body and does not certify. Quote/list/internal blank behavior follows the actual shared grammar transition, not a second scanner rule.

## 5.2 Live-root local stop

When a valid `RootBlankBarrier` is accepted as convergence, the observed scanner may return at that already-sealed cut before dispatching the next physical line.

It does not call artificial region-EOF closure to manufacture convergence.

BOF is a distinguished restart authority. Real EOF is an independent legal completion path.

## 5.3 Observer non-interference

The observation seam is inert. This is an exact conformance requirement, not an informal "the observer should not affect semantics" remark:

```text
With a no-op observer — one that never stops and ignores all observations:

RegionParse output
    == the same parse_region run without an observer

and the sink/source-inspection event stream
    == the same unobserved parse_region run.
```

Byte-identical in both cases.

Horse-A's local parsing installs no SpliceHook. The observed entry point is a sibling of the existing `parse_region` / `parse_region_with_hook` seam; the splice-hook path is untouched.

H0–H4 do not install this observer, so adding the seam does not alter their parsing behavior or treatment identity.

## 5.4 TopLevelStart provenance

`TopLevelStart { physical_line_start }` is generated exactly when the actual root-level dispatch begins a new top-level block — at the dispatch that opens its first physical line at root level. Conceptually, the frozen cases are:

```text
root fence opener            (entry frames empty)
root heading                 (pushed to doc)
root quote                   (quote frame pushed onto empty frames)
root list                    (list+item frames pushed onto empty frames)
root reference definition    (pushed to doc)
root paragraph first line    (entry frames empty)
```

`TopLevelStart` must NOT be derived from:

```text
semantic span.start                 (span start excludes leading
                                     spaces/prefixes; physical provenance
                                     is the physical line start)
nested/descendant blocks            (descendants stay inside the Owner
                                     opened by the root start)
paragraph continuation lines        (the paragraph is already open)
every list item                     (items are descendants of the root
                                     list start)
```

A root Quote or root List gets exactly one `TopLevelStart`, at its outermost physical first line; all of its descendants remain inside that Owner.

This event defines Owner physical-first-line provenance (the `p_i` of §3.2). No new parser state is invented for it; it is observed from the existing shared parser's root-level dispatch.

---

# 6. RestartCertificate

Logical support:

```rust
struct RestartSupport {
    preceding_lf: Option<usize>,
    blank_line: Range<usize>,
}
```

Support includes:

```text
the LF terminating the physical line before the blank barrier
+
the complete blank physical line including its LF
```

For an edit `[start,end)` the certificate is touched iff:

```text
[start,end) intersects support
OR
(start == end && start lies in support)
```

Example authority:

```text
source = "ab\n\n"
bytes       0 1 2 3
            a b LF LF
support = {2,3}

insert at byte 2
→ touches support
→ adjacent certificate is not reusable
```

Unchanged suffix-internal certificates survive by structural retention after valid convergence; they are not rewritten Owner-by-Owner.

---

# 7. Restart and convergence

## 7.1 Restart selection

The update locates damage using weighted sequence navigation, then chooses the nearest eligible certified predecessor using aggregate-pruned search.

Frozen safe-predecessor behavior:

- initial weighted descent;
- ancestor backtracking only through the correct abandoned-side predecessor regions;
- ancestor's own eligible certificate is considered where appropriate;
- position eligibility is tested before persistent certificate inspection;
- `subtree_has_safe` guides the final descent;
- no linear Owner scan toward BOF.

Conservative bound:

```text
safe_predecessor_node_visits <= 3H - 2
```

## 7.2 Conservative left guard

The replacement begins far enough left to include one guard Owner under the frozen coverage contract. This additional parsing is an intentional Horse-A cost.

## 7.3 Convergence

A normal interior convergence requires all of:

```text
real old Owner boundary
valid old restart certificate
edit/damage fully crossed
exact old↔new mapping
new live parser at equivalent empty-root continuation
coverage legally splittable at the boundary
mapped suffix unchanged
```

The restart cut itself is **not** a convergence candidate.

Binding clarification from the final freeze review:

> Candidate walking begins strictly after the restart cut. The restart boundary is only the parse starting authority; it is never itself offered as a convergence candidate.

Horse-A chooses the first valid convergence. If none occurs before real EOF, EOF is a legal replacement end.

There is no work-budget selector that converts a long local attempt into full build.

---

# 8. Semantic preservation

Let:

```text
old = P · O · S
new = P · N · S
```

and ordered document facts be:

```text
Defs(old) = Defs(P) ++ Defs(O) ++ Defs(S)
Defs(new) = Defs(P) ++ Defs(N) ++ Defs(S)
```

After syntax retention of P/S has been established, Horse-A compares the exact ordered normalized replacement facts:

```text
Defs(O) == Defs(N)
```

If equal:

```text
global first-wins RefTable remains semantically identical
→ retain old RefTable
→ eager-materialize fresh N under that retained environment
```

If facts differ or preservation is unknown:

```text
facts differ OR preservation unknown
    → same-target full build
```

The full-build target is the complete same-target Horse-A ReadyDocument defined in §13 (Same-target full builder) — never a weaker state.

Inequality means only "Horse-A did not prove semantic preservation". It does not claim effective winners necessarily changed.

This deliberately preserves W-A2: a shadowed-definition edit may cause a conservative full rebuild.

---

# 9. Facts-before-materialization order

The local update order is frozen:

```text
parse complete replacement block structure
→ extract complete ordered replacement definition facts
→ compare old/new facts
→ decide semantic environment
→ only then eager materialize fresh payload
```

Reference-sensitive payload may not enter READY before its environment is known.

No deferred semantic repair is allowed after READY.

---

# 10. High-level UPDATE algorithm

```text
INPUT:
  old ReadyDocument
  old source
  post source
  canonical edit

1. Validate source/version/interpretation/edit association.

2. Weighted-locate the affected coverage and seam.

3. Find the nearest eligible certified restart predecessor.

4. Apply the frozen left-guard expansion.

5. Keep old ReadyDocument coherent; create staging state.

6. Forward-parse from restart using the observed shared parser.

7. Evaluate sealed certified candidates strictly after restart.
   Stop at the first valid convergence, otherwise continue to real EOF.

8. The complete old/new replacement intervals are now known.

9. Extract ordered old/new replacement definition facts.

10a. If facts equal:
       retain old RefTable;
       eager-materialize complete fresh replacement Owners;

10b. If facts differ/unknown:
       build a complete same-target Horse-A ReadyDocument candidate (§13).

11. Prepare every ordinary fallible resource required by commit.
    Drop parser/cursor borrows.

12. Form PreparedCommit.

13. CROSS COMMIT FRONTIER.

14a. Local plan:
       split old OwnerSeq at [lo,hi)
       structurally transfer P/S
       splice fresh N
       install new source association
       move old RefTable exactly once
       retire detached O only

14b. Full plan:
       install fully built new ReadyDocument
       retire complete old representation

15. Return already-READY pending state.

16. complete() is a sealing/value-move wrapper. Horse-A's frozen realization
    keeps all attributable commit/retirement work inside update(), so
    complete() performs no real mechanism work. At the generic experimental
    phase-contract level, primary structural work still includes complete()
    attribution whenever a realization's complete() performs real work.
```

The implementation must not add a branch equivalent to:

```text
work too large -> full build
```

---

# 11. Staging and commit frontier

## 11.1 UpdateStaging responsibility boundary

`UpdateStaging` is the pre-frontier staging state. Conceptually it owns or references:

```text
the old READY document, while it remains logically coherent
the prepared scalar/rank/edit mapping
the fresh local replacement candidate OR the full candidate
the semantic-preservation decision (facts equal / differ / unknown)
all pre-frontier fallible resources
the structural work attribution accumulated before the frontier
```

The old RefTable is not duplicated into staging; in the local path it stays owned by the old document and is moved exactly once at commit (below). Exact private Rust field names remain implementation freedom.

## 11.2 PreparedCommit definition

```text
PreparedCommit
=
a state in which every ordinary recoverable/fallible operation required
for commit has completed, and only non-fallible structural ownership
operations remain.
```

Before `PreparedCommit`:

```text
old READY remains coherent
all parsing occurs
all semantic decisions occur
all ordinary fallible allocation/resource preparation occurs
all validation occurs
fresh payload/full candidate is complete
```

After the frontier:

```text
old ownership may be consumed
split/join/rotation/relink only
aggregate repair only
scalar/fixed structural counters only
retirement only
state installation only
```

No post-frontier:

```text
parsing
source-inspection event growth
semantic lookup
fallback
validation returning to staging
Vec/String/log growth required by the mechanism
ordinary recoverable allocation
```

Process OOM, panic, stack overflow, or invariant bugs are not algorithmic fallback paths.

The local plan records `ReuseOldRefTable`; it does not clone the full table. After the frontier `old.refs` is moved exactly once into the new READY state.

---

# 12. Weighted AVL realization

Horse-A v1 uses one mutable AVL node per Owner.

Height convention:

```text
h(empty) = 0
h(leaf)  = 1
```

Required operators:

```text
locate_by_byte
safe_predecessor
split(root,k)
join_with_pivot(left,pivot,right)
join(left,right)
replace_range(root,lo,hi,middle)
bulk_build(ordered Owners)
sequential cursor
```

## 12.1 join_with_pivot

Height-aware AVL join:

- compatible heights: attach directly under pivot;
- otherwise descend the inner spine of the taller tree;
- recursively join at compatible height;
- relink/recompute/rebalance while unwinding;
- only existing nodes are relinked.

## 12.2 join

Frozen deterministic policy:

```text
if one side empty -> return the other
otherwise remove_max(left) exactly once to obtain pivot
then join_with_pivot(remaining_left,pivot,right)
```

No fresh pivot allocation.

## 12.3 split

Precondition and semantics:

```text
0 <= k <= subtree_records(root)

split(root,k) = (first k Owners, remaining Owners)
```

Conceptual rank cases (no production Rust is frozen here; functional identity is):

```text
k <  left_records       descend left, split there
k == left_records       left output = existing left subtree;
                        right output = join_with_pivot over the pivot
k == left_records + 1   pivot becomes the last node of the left output
k >  left_records + 1   descend right with rank reduced
```

It follows one search spine and reconstructs outputs with existing nodes and `join_with_pivot`, using two monotone output accumulators.

### 12.3.1 Accepted split-proof core

For `split(T,k) -> (A,B)`:

```text
h(A) <= h(T)
h(B) <= h(T)
```

There is no common useful lower-bound window on the output heights. Along the one split search spine, reconstruction uses the two monotone output accumulators. If `δ_i` is the height difference seen by each reconstruction `join_with_pivot`, then, because accumulator heights never decrease:

```text
Σ δ_i <= h(A) + h(B) <= 2H
```

Therefore, with at most H search-spine nodes and at most H reconstruction joins, each join costing `<= 2δ_i + 1` visits and `<= δ_i + 1` rotation units:

```text
V_split <= H + 2Σδ_i + #joins <= 6H        (proved)

R_split <= Σδ_i + #joins      <= 3H        (proved)
```

The study intentionally retains the more conservative frozen thresholds:

```text
split node visits <= 10H    (per split)
split rotations   <= 5H     (per split)
```

Explicitly INVALID/WITHDRAWN — do not restore either statement:

```text
every split output lies in [h-2, h+1]     (false output-height lemma)
all internal δ <= 3
```

Per-split structural derivatives retained from #59: link writes <= 22H; aggregate field reads <= 86H; aggregate field writes <= 40H.

## 12.4 replace_range

```text
(A,BC) = split(root,lo)
(B,C)  = split(BC,hi-lo)

new_root = join(join(A,middle),C)
removed  = B
```

Retained P/S are never reinserted record-by-record.

## 12.5 bulk build

Full/fresh ordered Owners are converted to a balanced AVL in O(M) by consuming the ordered sequence once. No repeated O(log M) insertion loop.

This deliberately preserves W-A3: one-record-per-node allocation/pointer/cache costs remain visible.

---

# 13. Same-target full builder

Whenever the semantic-preservation rule requires it (`facts differ OR preservation unknown`, §8), or a full candidate is otherwise selected by the frozen branches, Horse-A constructs the complete same-target ReadyDocument with the dedicated full builder. This section freezes its contract; it is not a weaker "re-parse fallback".

## 13.1 Frozen pipeline

```text
shared full block parse over the exact source
→ obtain physical root-level starts / parser observations
  (TopLevelStart provenance, §5.4)
→ collect complete ordered document definition facts
→ construct the complete document RefTable
→ eagerly materialize all semantic payload under that FINAL RefTable
→ convert payload coordinates to Owner-relative form
→ construct canonical physical-first-line Owner coverage (§3.2)
→ attach only parser-derived real pre-EOF restart certificates
  (RootBlankBarrier evidence, §5.1; finish() manufactures nothing)
→ build the balanced weighted OwnerSeq / AVL in O(M) (bulk build, §12.5)
→ construct ReadyDocument
→ READY
```

Physical-line starts and certificate live evidence are collected online during the block pass; the order above does not permit reconstructing certificates after the parser is destroyed from an empty `ContextKey`.

## 13.2 Target equivalence

The full builder returns the SAME LOGICAL READY TARGET CLASS as successful local completion. It must NOT return a weaker state such as:

```text
normalized tree only
tree + RefTable but no OwnerSeq
state without restart certificates
state not immediately eligible for another update
```

For a given source, local completion and full construction must agree on:

```text
logical Owners
coverage partition
semantic payload/result
RefTable
restart-certificate set permitted by the frozen parser evidence
query/export result
subsequent-update eligibility
```

The following may differ (non-logical construction detail):

```text
AVL shape
heap addresses
short-lived construction identities
```

Empty documents, TriviaOnly documents, synthetic Document roots, spans, the RefTable, queries, and subsequent-update capability are identical between the two paths. H0's existing resident state is not substituted for this target.

The full path is a normal correct route, not a free one: the abandoned incremental attempt (if any), the fresh full parse/RefTable/payload/sequence construction, the temporary coexistence of old and new state, and the complete retirement of the old representation are all real recorded work of that branch.

---

# 14. Retirement

Local update:

```text
P + O + S
→ structurally transfer P/S
→ retire detached O only
```

Retirement quantities are separate named counters and are never merged:

```text
retire_node_visits         = retired AVL records traversed by the drop walk
payload_nodes_retired      = semantic payload nodes destroyed
retirement_frames_entered  = one recursion frame per retired AVL record
                             and per destroyed payload node
max_retirement_depth       = resource depth
```

Depth quantities (`max_retirement_depth`) are never combined with operation counts (`retire_node_visits`, `payload_nodes_retired`, `retirement_frames_entered`) into one scalar; the historical combined `retirement_workspace_ops` measure is withdrawn.

Retained P/S payload is never traversed merely for retirement or for attribution.

For the frozen failure-first witness:

```text
Δ_old = 2
P_removed = 4
D_payload = 2
```

Resource depth must account for both detached AVL depth and payload-tree depth:

```text
O(H_detached + D_payload)
```

A full-build commit may retire the complete old representation because that branch explicitly replaces the complete document state.

---

# 15. Structural accounting authority

Horse-A structural attribution uses explicit work events/counters, not a post-update full-tree walk. This section is self-contained: it defines every unit needed to reproduce the frozen thresholds of §16.

## 15.1 node visit

```text
node visit
  = one logical processing of one non-empty AVL node by the named operator;
    revisiting the same node later counts again.
```

"Unique nodes touched" is never used where repeated work matters. The same logical processing is never counted in two visit counters.

## 15.2 link write

```text
sequence_link_write
  = one mutation/reassignment of a persistent AVL root/left/right
    structural Link slot that changes which node/subtree the slot owns.
```

Moving a `Box` between local variables alone = 0 link writes; installing it into a persistent slot = 1 link write.

Frozen rotation slot-write convention:

```text
single rotation = 3 structural link writes
double rotation = 6 structural link writes
```

This convention is part of the frozen link-write thresholds of §16.

## 15.3 rotation

```text
single rotation = 1
double rotation = 2
```

A rebalance decision that performs no rotation is not itself a rotation.

## 15.4 aggregate field read/write

The persistent aggregate fields are exactly:

```text
height
subtree_bytes
subtree_records
subtree_has_safe
```

(The §2 `Aggregate` shape groups three of these; `height` is stored on the node beside them. Whether `height` lives inside the aggregate struct or beside it is private Rust layout; the accounting treats all four as persistent per-node aggregate fields, per #59.)

(`subtree_payload_nodes` is not a persistent aggregate; fresh/retired payload nodes are counted at creation/retirement.)

```text
aggregate_field_read
  = one read of one persistent aggregate field from one non-empty AVL node.
    An empty child has no node and contributes no reads.

aggregate_field_write
  = one write of one persistent aggregate field during node metadata
    recomputation.

recompute(node)
  = one completed local metadata recomputation for one node:
    reads the four aggregate fields from each non-empty child
      (up to 4 reads per child / 8 total)
    and writes the four persistent aggregate fields of the node
      (= 4 writes).
    Node-local Owner fields are O(1) locals, not aggregates.
```

Owner-local scalar reads are not automatically aggregate-field reads unless the frozen ledger (§16) says so.

## 15.5 certificate read/write

```text
certificate_read
  = inspection of one boundary/Owner's persistent RestartCertificate,
    OR of its presence/absence, sufficient for one mechanism decision.
```

Reading `subtree_has_safe` is an aggregate read, never a certificate read; certificate presence counts as a certificate read when it drives a mechanism decision.

```text
certificate_write
  = creation/installation/update of one persistent outgoing
    RestartCertificate for one new/replacement Owner boundary.
```

Destruction of a detached Owner's certificate is not a write. Transient parser `RootBlankBarrier` observations are not by themselves persistent certificate writes.

## 15.6 candidate check / cursor advance

```text
candidate_check
  = one sealed old-certified boundary for which the full convergence
    eligibility predicate is evaluated (mapped position + support-touch +
    coverage closure).

cursor_advance
  = one monotone movement from the cursor's current certified-boundary
    position to the next offered candidate boundary under the frozen
    cursor algorithm.
```

Each full predicate evaluation is one `candidate_check` and one `certificate_read`, charged to those separate counters, never to `*_node_visits`.

The restart cut itself is NOT a candidate and therefore generates no candidate check. Candidate walking begins strictly after restart.

## 15.7 Decision integrity

No decision-bearing counter may treat Unknown/missing as zero.

Forbidden sentinel work includes:

```text
sequential enumeration of retained prefix
sequential enumeration of retained suffix
unrelated retained payload inspection
per-Owner retained suffix coordinate rewrite
per-Owner retained suffix certificate rewrite
global definition recollection
retirement of unaffected old records
extra retained-tree traversal solely for attribution
```

Legitimate O(H) boundary-path access through P/S is not a forbidden sentinel.

---

# 16. Frozen failure-first bounds

For the #60 witness, legal AVL heights are frozen from:

```text
N(0)=0
N(1)=1
N(h)=1+N(h-1)+N(h-2)
```

Primary cells:

```text
128 KiB  -> Hmax = 14
1 MiB    -> Hmax = 18
16 MiB   -> Hmax = 24
```

Frozen symbolic upper bounds:

```text
safe_predecessor <= 3H - 2
f1                <= 4H - 2

f2 visits         <= 32H + 2
f2 rotations      <= 16H + 1
f2 link writes    <= 68H + 25

f3 aggregate reads  <= 279H + 101
f3 aggregate writes <= 128H + 40

f4 cursor allowance <= 2H + 10

f5 is separated:
  retire_node_visits        <= Δ_old
  payload_nodes_retired      = P_removed
  retirement_frames_entered <= Δ_old + P_removed
  max_retirement_depth      <= max(H_detached,D_payload)
```

Numerical primary thresholds. Each row carries its frozen formula; every formula is a #59 §21 authority formula (or an exact witness-fixed value), and there are no unexplained constants:

| counter | frozen formula | 128 KiB | 1 MiB | 16 MiB |
|---|---|---:|---:|---:|
| locate visits | ≤ H | 14 | 18 | 24 |
| safe predecessor visits | ≤ 3H − 2 | 40 | 52 | 70 |
| f1 (locate + safe predecessor) | ≤ 4H − 2 | 54 | 70 | 94 |
| cursor visits | ≤ 2H + 4k + Q, k = 2, Q = 2 | 38 | 46 | 58 |
| fact-range visits | ≤ H | 14 | 18 | 24 |
| split visits ×2 | ≤ 2 × 10H (retained threshold) = 20H | 280 | 360 | 480 |
| pivot extraction visits ×2 | ≤ (4H−3) + (4(H+1)−3) = 8H − 2 | 110 | 142 | 190 |
| top-level join visits ×2 | ≤ (2H+1) + (2(H+1)+1) = 4H + 4 | 60 | 76 | 100 |
| f2 visits (replace_range total) | ≤ 32H + 2 | 450 | 578 | 770 |
| bulk_build_node_visits | = Δ_new | 2 | 2 | 2 |
| AVL rotations | ≤ 16H + 1 | 225 | 289 | 385 |
| sequence link writes | ≤ 68H + 25 | 977 | 1249 | 1657 |
| aggregate reads | ≤ 279H + 101 | 4007 | 5123 | 6797 |
| aggregate writes | ≤ 128H + 40 | 1832 | 2344 | 3112 |
| certificate reads | ≤ 5 (witness derivation below) | 5 | 5 | 5 |
| certificate writes | ≤ 2 (witness derivation below) | 2 | 2 | 2 |
| candidate_check | = Q | 2 | 2 | 2 |
| retire_node_visits | ≤ Δ_old | 2 | 2 | 2 |
| payload_nodes_retired | = P_removed | 4 | 4 | 4 |
| retirement_frames_entered | ≤ Δ_old + P_removed | 6 | 6 | 6 |
| max_retirement_depth | ≤ max(H_detached, D_payload) ≤ H | 14 | 18 | 24 |
| old fact Owner visits | ≤ Δ_old + Δ_new | 4 | 4 | 4 |
| RefTable entries visited | exact | 0 | 0 | 0 |

Witness certificate derivation (frozen in #60 §9.3.1):

```text
certificate_reads <= 5:
  1  safe_predecessor final selection inspection
     (the selected boundary's own certificate)
  1  edit support-touch validation against the selected certificate
  2  candidate predicate evaluations (#1 rejected, #2 convergent —
     each full predicate evaluation inspects the candidate certificate once)
  1  replacement upper-boundary certificate inspection
     (closing coverage at q_old/q_new)

certificate_writes <= 2:
  the local parse seals exactly two root-blank barriers before convergence
  (the boundary between the two replacement Owners, and the convergence
  barrier at q_old/q_new); each seal installs one persistent outgoing
  certificate. Certificates on detached old Owners are dropped with O,
  not written.
```

Thresholds are derived from `Hmax`, not from measured/expected H: an unexpectedly tall tree is an implementation invariant failure, not a larger permitted budget.

The `f4 = 2H + 4k + Q` expression is a frozen conservative allowance for the witness, not an assertion that the cursor ledger's exact primitive count equals that expression.

---

# 17. Frozen #60 witness

## 17.1 Workload identity and provenance

Primary cells are fixed by the existing #50 / Campaign-2 authority. Durable provenance anchor (no raw evidence is duplicated here):

```text
research/benchmarks/markdown-ast-update/results/h4-large-n-cause-1/cells.jsonl

git blob:
98d04bc9eb6f4f11c7da8fc92aa9c62568d78c65
```

The three primary cases use their frozen per-cell identity fields, written
here in this consolidation's terminology with the anchor's concrete JSON
keys in parentheses:

```text
case/cell identity
    (cell_id, case_id_hex)

pre_sha256
    (pre_sha256)

post_sha256
    (post_sha256)

start/end
    (edit_start, edit_end; equal for this insertion)

inserted_text_sha256
    (edit_sha256; inserted_text = "zzzzzzzz")
```

Terminology: this consolidation always names the inserted-bytes hash
`inserted_text_sha256`; the frozen anchor key `edit_sha256` denotes that
same inserted-bytes hash.

Inserted bytes alone do not identify an edit; the canonical edit identity is:

```text
(case/cell identity,
 pre_sha256,
 post_sha256,
 start,
 end,
 inserted_text_sha256)
```

with `start == end == edit_start` for this zero-length insertion witness. Do not regenerate treatment observations; no prose-equivalent generator is acceptable without byte-identity proof.

## 17.2 Frozen geometry

Source unit:

```text
128 bytes = 126 content bytes + LF + LF
```

Primary cells:

```text
128 KiB
1 MiB
16 MiB
```

For each:

```text
M = N / 128
target = floor(M/2)
edit = insert "zzzzzzzz"
edit_start = target*128 + 63
no definitions
no references
no fences
depth = 0
```

Static replacement predictions:

```text
unit                    = 128 bytes
Δ_old (old replacement Owners)  = 2
Δ_new (new replacement Owners)  = 2
old replacement bytes   = 256
new replacement bytes   = 264
edit_start - restart    = 191
ordered facts           = [] == []
same-target full        = false
convergence before EOF  = true
Q                       = 2
P_removed               = 4
D_payload               = 2
```

Candidate walking starts strictly after the restart cut.

The first candidate is rejected because the edit/damage has not yet been fully crossed — not because a paragraph is "still open". The second candidate is the first legal convergence.

---

# 18. Phase coverage

This section freezes which work the decision-bearing primary structural verdict covers.

```text
primary structural mechanism work =
    prepare_update attribution
  + update/native attribution
  + complete attribution, if complete performs real mechanism work
```

```text
prepare_update addressing work IS included.
```

(Horse-A's frozen realization keeps `complete()` as a sealing/value-move wrapper with all attributable commit/retirement work inside `update()`; the phase contract above still covers `complete()` generically, should a realization ever make it perform real mechanism work.)

Excluded from the PRIMARY UPDATE structural verdict:

```text
whole-document fresh READY-state construction used to establish
    the independent initial state
oracle normalization / export
C3 restore probe
final destruction of the finished document after observation
```

Explicitly INCLUDED in primary update structural work:

```text
fresh replacement Owners / Δ_new produced as part of the primary update
all local fresh semantic materialization
all structural splice work
retirement required before the returned state is READY
```

Frozen wording erratum, resolved normatively: the exclusion phrase "fresh ready-state construction" means whole-document fresh READY construction **outside** the primary update (establishing the independent initial state). It does NOT mean fresh replacement Owners created by the update, which remain inside primary update structural accounting.

---

# 19. Failure-first adjudication

Before any economics claim, each primary cell must pass:

```text
C1: Horse-A full-build pre-source == clean H0 normalized result
C2: Horse-A primary update post-source == clean H0 normalized result
C3: delete inserted 8 bytes on the SAME returned state and recover clean pre-source result
```

Full normalized structural equality is the correctness oracle. Checksums/hashes recorded with runs are identity/provenance/sanity fields; they are not by themselves the correctness oracle.

Decision-bearing structural runs require:

```text
3 independent processes
fresh READY state per primary run
exact deterministic result agreement
exact deterministic structural counter agreement
local path selected
facts-preserved path selected
convergence before EOF
same-target full not selected
replacement geometry equals frozen derivation
all decision counters Known
all legal counters within frozen bounds
all forbidden sentinels Known(0)
local retirement only
no post-hoc retained-tree attribution walk
```

Structural FAIL cannot be rescued by latency, PMU, allocator timing, or measured speedup.

```text
Horse-A algorithm or Horse-A-required accounting performs forbidden
O(M) retained-state work
    → structural/conformance FAIL

external recording/debug/serialization layer adds work the mechanism
itself does not execute
    → instrumentation INVALID for that run
```

External instrumentation that adds work not executed by the treatment may invalidate that run; mechanism-required accounting that itself violates V1 requirement R6 is a structural/conformance FAIL, not an instrumentation escape hatch. V1 requirement R6 implementation work is never reclassified as instrumentation INVALID.

Structural collection remains unauthorized until a later explicit execution gate.

---

# 20. Deliberately preserved weaknesses

## W-A1 — restart / retention granularity

Horse-A v1 keeps:

```text
root-only restart
whole top-level container as one Owner
conservative left guard
```

A huge container may therefore require long parser/fresh-payload work.

Possible future line: A-R.

## W-A2 — conservative semantic certification

Horse-A v1 keeps:

```text
replacement facts differ OR preservation unknown
→ same-target full build
```

A shadowed definition change may rebuild even when effective first-wins semantics are unchanged — and any unprovable preservation case falls back to the same complete same-target build (§13).

Possible future line: Horse-B1/B2.

## W-A3 — binary layout

Horse-A v1 keeps:

```text
one Owner per AVL node
```

Allocation, pointer chasing, cache locality, construction cost, and retained memory may remain poor even if structural work locality passes.

Possible future line: A-P.

These three weaknesses must not be optimized away during Horse-A v1 implementation.

---

# 21. Implementation freedom that remains

The following may be chosen during coding if they do not change frozen semantics/accounting:

```text
module/file names
private helper names
minor Rust ownership ergonomics
exact enum/struct names
test organization
debug formatting
allocator details that do not change the frozen algorithm or frontier
```

Implementation is not authorized to change:

```text
Owner granularity
coverage rule
certificate semantics
candidate offering rule
semantic preservation rule
AVL operator algorithm identity
static structural units/bounds
failure-first workload identity
W-A1/W-A2/W-A3
```

---

# 22. Implementation-stage gates

Authorized now:

```text
Horse-A implementation
unit tests
AVL invariant tests
parser-observer tests
coverage/certificate tests
semantic correctness tests
C1/C2/C3 correctness work
implementation-conformance review
```

Still unauthorized:

```text
#60 structural treatment collection
latency campaign
allocator/PMU collection
H0/H4 recollection
A-R / Horse-B / A-P treatment changes
```

Required next sequence:

```text
implement frozen Horse-A
    ↓
local/unit/invariant correctness
    ↓
C1/C2/C3 correctness
    ↓
implementation-conformance review
    ↓
explicit structural-collection authorization
    ↓
execute frozen #60 failure-first cells
```

---

# 23. Final frozen algorithm summary

```text
edit
 ↓
weighted locate
 ↓
nearest certified restart predecessor
 ↓
left guard expansion
 ↓
forward shared-parser scan
 ↓
first eligible convergence strictly after restart
(or real EOF)
 ↓
complete O/N ordered definition facts
 ↓
       facts equal? ──────────────── no/unknown ──→ same-target full build
            │
           yes
            ↓
retain old RefTable
            ↓
eager materialize fresh replacement Owners
            ↓
prepare all ordinary fallible resources
            ↓
PreparedCommit
            ↓
consume old weighted AVL
            ↓
2 structural splits + 2 joins
            ↓
transfer untouched P/S without enumeration
            ↓
retire detached O only
            ↓
READY new state
```

The first Horse-A experiment is successful only if this mechanism is correct **and** the frozen local witness stays inside the pre-registered structural bounds without forbidden global retained-state work.
