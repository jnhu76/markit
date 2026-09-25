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

---

# 1. Research identity

Horse-A exists to test one claim:

> When a safe local Markdown edit has bounded parser/replacement/semantic-preservation work, preserving unaffected retained representation must not itself require work proportional to total retained size `M`.

Horse-A is therefore deliberately a **structural-locality horse**, not a claim to be the final or fastest Markdown updater.

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

If top-level starts are:

```text
c0 < c1 < ... < cM
```

then:

```text
Owner_i.coverage = [c_i, c_(i+1))
```

with the last Owner extending to EOF.

Consequences:

- blank/interstitial bytes between two semantic blocks belong to the left Owner;
- leading trivia belongs to the first Owner;
- trailing trivia belongs to the last Owner;
- all-whitespace non-empty input is represented by one `TriviaOnly` Owner;
- empty input has an empty `OwnerSeq`;
- semantic spans need not cover trivia bytes;
- source coverage and semantic span are distinct concepts.

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
absolute span after Owner is located = O(1) + owner-internal path cost
whole export = sequential Owner traversal + running base
```

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
same-target full build
```

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
       build a complete same-target Horse-A ReadyDocument candidate.

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

16. complete() is a pure/no-fail wrapper.
```

The implementation must not add a branch equivalent to:

```text
work too large -> full build
```

---

# 11. Staging and commit frontier

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

```text
split(root,k)
→ first k Owners
→ remaining Owners
```

It follows one search spine and reconstructs outputs with existing nodes and `join_with_pivot`.

The final accepted proof does **not** use the withdrawn false `[h-2,h+1]` output-height lemma.

Accepted conservative bounds per split:

```text
node visits <= 10H
rotations   <= 5H
```

The underlying proof establishes O(H) by accumulator-height telescoping.

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

# 13. Retirement

Local update:

```text
P + O + S
→ structurally transfer P/S
→ retire detached O only
```

Retirement quantities are separate:

```text
retired AVL records
payload nodes retired
retirement frames entered
maximum retirement depth
```

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

Retained P/S payload is never traversed merely to retire or count it.

A full-build commit may retire the complete old representation because that branch explicitly replaces the complete document state.

---

# 14. Structural accounting authority

Horse-A structural attribution uses explicit work events/counters, not a post-update full-tree walk.

Core units:

```text
node visit
  = one logical processing of one non-empty AVL node by the named operator;
    revisiting later counts again

link write
  = one write/reassignment of a persistent structural root/left/right link slot;
    moving a Box between local variables alone is not a link write

rotation
  single = 1
  double = 2

aggregate read/write
  = the frozen per-field persistent aggregate accounting defined by #59

certificate read
  = inspection of one persistent RestartCertificate for a mechanism decision;
    subtree_has_safe reads are aggregate reads, not certificate reads

certificate write
  = creation/installation/update of one persistent outgoing certificate
```

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

# 15. Frozen failure-first bounds

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

Numerical primary thresholds:

| counter | 128 KiB | 1 MiB | 16 MiB |
|---|---:|---:|---:|
| locate visits | 14 | 18 | 24 |
| safe predecessor visits | 40 | 52 | 70 |
| f1 | 54 | 70 | 94 |
| cursor visits | 38 | 46 | 58 |
| fact-range visits | 14 | 18 | 24 |
| split visits ×2 | 280 | 360 | 480 |
| pivot extraction visits ×2 | 110 | 142 | 190 |
| top-level join visits ×2 | 60 | 76 | 100 |
| f2 visits | 450 | 578 | 770 |
| AVL rotations | 225 | 289 | 385 |
| sequence link writes | 977 | 1249 | 1657 |
| aggregate reads | 4007 | 5123 | 6797 |
| aggregate writes | 1832 | 2344 | 3112 |
| certificate reads | 5 | 5 | 5 |
| certificate writes | 2 | 2 | 2 |
| retired AVL records | 2 | 2 | 2 |
| payload nodes retired | 4 | 4 | 4 |
| retirement frames entered | 6 | 6 | 6 |
| max retirement depth | 14 | 18 | 24 |
| old fact Owner visits | 4 | 4 | 4 |
| RefTable entries visited | 0 | 0 | 0 |

The `f4 = 2H + 4k + Q` expression is a frozen conservative allowance for the witness, not an assertion that the cursor ledger's exact primitive count equals that expression.

---

# 16. Frozen #60 witness

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
old replacement Owners = 2
new replacement Owners = 2
old replacement bytes  = 256
new replacement bytes  = 264
edit_start - restart    = 191
ordered facts           = [] == []
same-target full        = false
convergence before EOF  = true
Q                       = 2
```

Candidate walking starts strictly after the restart cut.

The first candidate is rejected because the edit/damage has not yet been fully crossed. The second candidate is the first legal convergence.

---

# 17. Failure-first adjudication

Before any economics claim, each primary cell must pass:

```text
C1: Horse-A full-build pre-source == clean H0 normalized result
C2: Horse-A primary update post-source == clean H0 normalized result
C3: delete inserted 8 bytes on the SAME returned state and recover clean pre-source result
```

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

Structural FAIL cannot be rescued by latency.

External instrumentation that adds work not executed by the treatment may invalidate that run; mechanism-required accounting that itself violates R6 is a structural/conformance FAIL, not an instrumentation escape hatch.

Structural collection remains unauthorized until a later explicit execution gate.

---

# 18. Deliberately preserved weaknesses

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
replacement ordered facts differ
→ same-target full build
```

A shadowed definition change may rebuild even when effective first-wins semantics are unchanged.

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

# 19. Implementation freedom that remains

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

# 20. Implementation-stage gates

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

# 21. Final frozen algorithm summary

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
