# Horse-A v2 research direction

> Status: **research direction / umbrella authority candidate**  
> Baseline: **Horse-A v1 is immutable**  
> Independent review verdict: **KEEP_WITH_RESTRUCTURE / READY_WITH_CHANGES**  
> Implementation authorization: **NOT YET GRANTED**

## 1. Purpose

Horse-A v1 is now a sealed experimental baseline. The next phase is not a
continuation of v1 implementation and not an accumulation phase where more
indexes, caches, summaries, PMU counters, threads, or state are added because
they look promising.

Horse-A v2 research asks a more basic question:

> **What work and state in Horse-A v1 can safely disappear, and what is the
> minimum mechanism that must remain or be added after that deletion?**

The goal is to produce a stronger mechanism by removing avoidable work first,
then adding only the minimum new authority required by correctness and measured
weaknesses.

A valid outcome of this research is also:

```text
NO_NEW_MECHANISM
```

The research does not have to issue a Horse-A v2 merely because the study was
performed.

---

## 2. Immutable v1 authority

The v1 baseline remains:

```text
BASELINE_MECHANISM = Horse-A v1
BASELINE_CAPSULE = markit-r0-rq8-research-record-v1.tar.gz
BASELINE_CAPSULE_SHA256 = 156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849
```

Human-readable baseline:

```text
docs/research/horse-a-v1-baseline-experimental-record.md
```

Completed study authority:

```text
#33 = CLOSED / COMPLETE
#76 = CLOSED / COMPLETE
#91 = v1 baseline / figure and reproducibility tail work
PR #92 = merged baseline experimental record
```

Horse-A v1 must not be silently modified to absorb v2 ideas. A candidate that
changes mechanism identity receives a new experimental identity.

---

## 3. Research decision after independent review

The initial v2 proposal had three axes:

```text
A. microarchitectural diagnosis
B. failure-regime / mechanism-weakness analysis
C. prior-art / new-mechanism exploration
```

The independent review retained the content but rejected the A -> B -> C
sequence.

The three items live at different layers:

```text
B = the research problem
A = a diagnostic tool
C = a source of candidate mechanisms
```

Therefore Horse-A v2 is reorganized around two research questions rather than
three sequential axes.

---

## 4. The two primary research questions

### RQ-V2.1 — What can we delete?

> Which work, state, and mechanism in Horse-A v1 can be safely removed while
> preserving correctness, eager READY completion semantics, and the measured v1
> strengths that still matter?

This question distinguishes three things that must not be collapsed:

```text
work required because output really changed
work performed conservatively because the mechanism cannot prove reuse
work repeated because the mechanism chose an unnecessarily expensive path
```

The first may be unavoidable. The second and third are the primary deletion
targets.

### RQ-V2.2 — What is the minimum mechanism left?

> After deletion, what is the smallest retained state, certification rule, or
> decision mechanism needed to avoid the remaining unnecessary work without
> creating a larger lifecycle, memory, or maintenance tax?

This is a state-versus-work question.

A proposed v2 feature must show not merely that it can save work, but that the
work saved is worth:

```text
construction
ordinary-edit maintenance
validation
retained memory
retirement / deletion
fallback complexity
future-edit obligations
```

---

## 5. Deletion-first rule

Horse-A v2 adopts a deletion-first rule inspired by the broader project lesson
that historical machinery is not an architecture obligation.

**No Horse-A v1 component has grandfather privilege.**

For every v1 component ask:

```text
1. Is the responsibility required for correctness?
2. Is the responsibility required by eager READY / lifecycle semantics?
3. Does measured evidence show that the current mechanism has net benefit?
4. Is there a smaller mechanism that preserves the same authority and obligations?
```

Correctness and READY/lifecycle necessity take precedence over isolated
performance evidence. Required responsibilities include, where applicable,
termination, failure containment, required identity/association and ownership,
and next-edit obligations.

Decision rule:

```text
if correctness_required == YES or ready_required == YES:
    PRESERVE THE RESPONSIBILITY
    CHALLENGE THE CURRENT IMPLEMENTATION
    REPLACE IT ONLY WITH A SMALLER MECHANISM THAT CARRIES THE SAME OBLIGATIONS

if correctness_required == NO
and ready_required == NO
and measured_net_benefit == NO:
    DELETE

if measured_net_benefit == UNKNOWN:
    UNKNOWN IS NOT NO
    MEASURE ONLY WHEN THE UNRESOLVED PERFORMANCE VALUE AFFECTS THE CURRENT CHOICE
```

A required authority therefore does not need an isolated speedup result to
justify its responsibility, but its current representation still has no
grandfather privilege.

Likewise, deletion is not automatically simplification. Removing one shared
authority can move checks, state, or invariants elsewhere and increase total
mechanism complexity. Evaluate deletion together with every displaced
responsibility introduced by its replacement.

The default question is therefore not:

```text
How can this v1 subsystem be optimized?
```

It is:

```text
Why must this subsystem still exist in its current form?
```

Deletion is preferred over optimization when the complete replacement preserves
the contract and does not merely duplicate the removed authority elsewhere.

---

## 6. Component deletion audit

The first structural output of v2 research should be a component-by-component
declaration similar to the following:

| Component | Correctness-required? | READY/lifecycle-required? | Measured benefit? | Measured cost? | v2 decision |
|---|---|---|---|---|---|
| source/edit validation | TBD | TBD | TBD | TBD | KEEP / SHRINK / DELETE |
| damage location | TBD | TBD | TBD | TBD | KEEP / SHRINK / DELETE |
| Owner abstraction | TBD | TBD | v1 locality strength suggests value | TBD | CHALLENGE, not assume |
| restart selection | TBD | TBD | local/regime evidence suggests value | TBD | KEEP / REFINE / SHRINK |
| coordinate mapping | TBD | TBD | TBD | tiny-tax hypothesis | DECOMPOSE |
| prepare/stage machinery | mixed responsibilities | mixed | TBD | cannot be treated as one fixed tax | DECOMPOSE |
| convergence machinery | TBD | TBD | TBD | propagation hypothesis | DECOMPOSE |
| facts extraction/comparison | correctness-sensitive | likely | TBD | E6 hypothesis | DECOMPOSE |
| semantic certification | correctness-sensitive | likely | TBD | E6 hypothesis | CHALLENGE |
| retained AVL structure | TBD | TBD | W-A3 not confirmed as primary weakness | TBD | RE-EARN PLACE |
| payload reuse | TBD | TBD | strong v1 local results | TBD | PRESERVE UNLESS FALSIFIED |
| full-build fallback | safety role | READY role | TBD | E6 repeated-work hypothesis | REFRAME / EARLIER? |
| retirement | lifecycle-sensitive | yes | N/A | TBD | MINIMIZE, do not hide |

This table is intentionally incomplete at the start. Unknown fields are research
questions, not permission to keep the mechanism unchanged and not evidence that
the mechanism has no value. Responsibilities already justified by correctness
or lifecycle remain in force until an equivalent replacement demonstrably
carries them. Only components implicated in the current mechanism choice need
new performance measurement.

### Important implementation boundary

Current Horse-A `prepare_update` includes substantial `stage()` work such as
forward parsing, convergence, facts comparison, and either local materialization
or complete construction. Therefore `T_prepare` must not be interpreted as one
fixed administrative overhead bucket.

The v2 diagnosis should use coarse responsibility regions first, then split only
the largest unexplained contributor.

---

## 7. Failure causes: use causes and locations, not one exclusive taxonomy

A single update can suffer from multiple costs. Do not force every failure into
one mutually exclusive label.

Use four causal labels:

```text
PATH_SELECTION_OVERHEAD
    unnecessary incremental attempt or wrong path choice before rebuild

IMPACT_RANGE_EXPANSION
    restart / convergence / Owner granularity causes excess syntax work

REUSE_CERTIFICATION_INSUFFICIENCY
    output is actually reusable but current authority cannot prove it

STATE_MAINTENANCE_OVERHEAD
    construction / mapping / retained-tree / validation / retirement costs
    exceed the work they save
```

Also record lifecycle location:

```text
BUILD / OPEN
EDIT
COMPLETION / RETIREMENT
```

Compiler profile, cache warmth, and working-set transition are explanatory
conditions, not peer failure categories.

---

## 8. First diagnostic target: E6 and tiny under the same READY contract

The highest diagnostic priority is E6/reference-definition behavior, but the
research must not pre-decide that the answer is a semantic dependency index.

The observed E6 failure can arise from different causes:

```text
A. incremental work is attempted and then discarded before full build
B. constructing an equivalent Horse-A READY state is itself expensive
C. many eager semantic outputs genuinely must change
D. outputs mostly stay the same, but v1 lacks enough authority to prove reuse
```

These lead to different designs.

### 8.1 Required comparison paths

For a small diagnostic panel compare:

```text
A. Horse-A v1 normal update

B. direct same-target rebuild
   - same source/edit validation boundary
   - constructs the same required Horse-A READY post-state
   - performs correct old-state retirement
   - remains ready for the next edit

C. H0
   - reference cost, not assumed to be a legal Horse-A fallback cost

D. H2 for E6
   - same-contract comparison in a regime where H2 remained near/full-rebuild parity

H4 may remain as a strong local/restart comparison where useful.
```

The direct READY rebuild path is essential. H0 cannot substitute for it because
H0 and Horse-A do not necessarily construct the same next-edit-ready retained
state.

### 8.2 Initial diagnostic units

Start with a small panel, roughly 8–12 units rather than a new broad campaign.
Include at least:

```text
E6:
- losing duplicate changes but effective binding remains unchanged
- winning binding/value changes with low fanout
- definition creation/removal changes reference existence
- high-fanout effective change

Tiny:
- representative tiny inputs
- threshold-near cases if a direct READY rebuild advantage exists

Sentinels:
- a few local/container cases that protect known Horse-A strengths
```

The count is a practical start, not a statistical sufficiency guarantee.

### 8.3 Cost ledger

Record enough attribution to separate:

| Cost / effect | v1 update | direct READY rebuild | H0 | H2 where relevant |
|---|---:|---:|---:|---:|
| validation / locating / restart work | | | | |
| forward parse / repeated parse bytes | | | | |
| convergence attempts / rejection reasons | | | | |
| facts extraction / comparison | | | | |
| work already spent before full-build branch | | | | |
| full-build parse / materialization | | | | |
| retained-state construction / maintenance | | | | |
| retirement | | | | |
| allocations / allocated bytes | | | | |
| instructions/update | | | | |
| cycles/update | | | | |
| wall latency | | | | |

Also record semantic/output facts:

```text
facts changed?
effective winner identity changed?
effective lookup value changed?
reference existence changed?
affected Owners / result nodes?
number of outputs that actually must change?
```

The offline oracle may describe actual output change but must not become a free
online decision oracle for a candidate mechanism.

---

## 9. Semantic correctness challenges before candidate design

Any candidate that attempts to reduce E6 work must handle at least these
counterexamples:

### Losing duplicate changes

Facts and local definition nodes may change while the effective first-wins
binding used elsewhere does not.

### Winning definition deletion

A later duplicate may become the new winner. Deletion cannot be represented as
only "label removed".

### Negative dependency

A label that previously had no definition may cause old Text to become a
ReferenceLink after a definition is introduced. Tracking only already-resolved
ReferenceLink users is incomplete.

### Topology-changing semantic result

Definition existence/value changes may change inline node topology, not merely
replace a destination string in place.

Owner boundaries are not semantic lexical scopes. A per-Owner summary may be a
useful implementation granularity, but it must not redefine the document-global
reference semantics.

Horse-A v2 retains eager READY semantics unless a separate research decision
explicitly changes that contract. Lazy repair may be used internally only if all
required results are correct by completion.

---

## 10. Candidate mechanisms: make them compete, do not assume an upgrade ladder

Do not define a mandatory progression such as:

```text
semantic dirty bit -> dependency summary -> reverse dependency index
```

Instead compare candidate mechanism families directly against the diagnosed
avoidable work.

### Candidate A — earlier conservative rebuild / duplicate-work elimination

```text
Permanent new dependency state: near zero
Question: can v1 stop an unprofitable incremental attempt earlier?
```

This candidate should be tested before adding permanent dependency state if
repeated work is a major loss source.

### Candidate B — effective binding/value certification

Maintain or derive only enough information to determine whether the effective
first-wins binding/value seen by consumers actually changed.

This is different from simply comparing the entire definition-facts sequence.

### Candidate C — coarse semantic candidate summary + bounded scan

Retain small per-label or per-region candidate information sufficient to rule
out most unaffected work, including negative-dependency cases.

"Bounded" must be defined concretely: by candidate set, affected Owners, work
budget, or another measurable bound.

### Candidate D — reverse dependency index / dynamic dependency graph

This candidate is not pre-authorized.

It may enter only if:

```text
actual affected set is sparse
and finding that set via simpler scan/summary is itself materially expensive
and expected saved work can repay build/edit/retirement/memory cost
```

A dependency index can reduce the cost of finding affected outputs; it cannot
eliminate work for outputs that genuinely must change under eager semantics.

---

## 11. Tiny-input research: prove that a fallback opportunity exists first

The first tiny question is not how to predict incremental-versus-rebuild cost.
It is:

> Is a legal direct same-READY rebuild path actually cheaper than normal v1
> incremental update on the target tiny cases?

If the answer is NO, stop selector research.

If the answer is YES, test the smallest selector first.

Preferred order:

```text
1. one already-free feature, e.g. source length, as a static null hypothesis
2. add one already-free structural feature only if the simple rule fails
3. multi-feature / adaptive policy only if simpler policies fail materially
```

Do not traverse the AST just to acquire selector features for tiny inputs.
Do not select using benchmark family labels or an offline oracle.

Evaluate the decision rule using paired legal-path costs, including selector
cost and asymmetric mistakes, rather than classification accuracy alone.

---

## 12. Propagation/restart research: explain why the window expands

Propagation-window length is useful but not by itself a causal explanation.
The research must explain why the restart starts too early, why convergence
fails, or why work is repeated inside the window.

Minimum useful observations include:

```text
left excess distance from restart to edit
forward bytes/nodes inspected or reparsed
repeated inspection
number of convergence candidates
candidate rejection reason
affected Owner count
materialized result nodes
semantic fallback occurrence
```

Prefer a few paired interventions over a large factorial matrix:

```text
same affected prefix/window + irrelevant suffix growth
similar total size + move first legal convergence boundary
similar window + vary candidate/boundary density
similar window + vary node/inline density
```

Use counters to verify what the intervention actually changed.

---

## 13. Microarchitecture is a diagnostic tool, not a research axis

Existing v1 evidence already shows that many important differences are
algorithmic work-volume differences. Therefore more PMU collection is justified
only when it can change a mechanism decision.

Default order:

```text
algorithmic work differs materially
    -> explain and reduce the work first

algorithmic work is comparable but cycles/latency still differ
    -> use PMU to explain the residual
```

For comparable-work cases prefer per-update absolute quantities:

```text
instructions/update
cycles/update
misses/update
branches/update
```

MPKI/IPC remain supporting ratios; they are not sufficient causal evidence
because denominators and overlap behavior differ across mechanisms.

Useful conditional tools include:

```text
existing cycles/instructions and branch groups
existing dTLB/iTLB groups
perf record hotspot sampling
LBR for a concrete branch problem
PEBS/load-latency only after a concrete memory-latency residual exists
working-set sweeps only when a candidate adds substantial retained state
```

Do not restart a general hardware-counter survey. The repository already has
PMU groups and host constraints; reuse them when a specific question warrants
it.

### Do not optimize yet

Keep these closed until direct evidence reopens them:

```text
AVL packing / compact nodes
branchless rewrites
custom allocator
huge pages / TLB tuning
software prefetch
SIMD
NUMA tuning
lock-free structures
aggressive multithreading / speculation
full reverse dependency graph
online adaptive predictor
new rope / green-tree replacement
```

A high counter value by itself is not the reopening criterion.

---

## 14. Prior art is a candidate generator, not an independent completion gate

Read mechanisms only when they can change a candidate choice.

High-value initial families include:

```text
Salsa red-green / backdating
    -> distinguish possible invalidation from actual result change

rustc incremental queries / fingerprints
    -> change certification and projection barriers, including certification cost

Build Systems à la Carte
    -> separate rebuild decision from scheduling / dependency representation

incremental reference attribute grammar / dynamic dependency work
    -> compare dependency granularity and maintenance tax

self-adjusting computation / Adapton
    -> construction tax, dependency maintenance, and observation/completion boundaries

Lezer / Tree-sitter / rust-analyzer incremental reparsing
    -> restart/reuse certificates and cheap safe fallback conditions
```

Do not import another system's architecture wholesale. Extract one mechanism,
state its retained-state and work cost, and design a cheap falsification test
against a measured Horse-A problem.

---

## 15. Parallelism is frozen for now

Current decision:

```text
PARALLELISM_NOT_YET_JUSTIFIED = YES
```

Do not create Horse-A-P yet.

The amount and shape of useful parallel work can change substantially after the
serial algorithm removes repeated or unnecessary work. Computing Amdahl-style
parallel fractions on the current inefficient work graph can therefore be
misleading.

Reopen parallelism only if all three hold:

```text
1. after serial algorithmic repair, an important latency/tail problem remains
2. the remaining dependency DAG exposes coarse independent tasks with a short span
3. measured executor/synchronization/cache cost leaves material net speedup headroom
```

The most plausible future case is a large set of genuinely independent eager
semantic repairs after the reference environment is known. This remains a
hypothesis, not a v2 requirement.

---

## 16. Anti-overfitting discipline

Existing E6/tiny/E4/Q3/Q4 cases may be used for diagnosis and candidate
development, but not as the sole confirmation surface.

Use three evidence roles:

```text
DEVELOPMENT / DIAGNOSTIC
    existing v1 cases and new explicitly diagnostic variants

PROJECT HOLDOUT
    fixed before candidate timing is revealed; not used for tuning

FALSIFICATION TRACES
    small multi-step traces targeting semantic edge cases and lifecycle/state churn
```

Minimum falsification traces should cover examples such as:

```text
local edit
-> definition introduced
-> reference becomes active
-> definition changed/deleted
-> local edit

winning/losing duplicate alternation + inverse/undo-like edit

tiny cases oscillating near any proposed fallback boundary

local edit alternating with fence/propagation edits
```

A stateful candidate must be evaluated across construction, repeated edits, and
retirement. Correctness-only holdout does not expose maintenance/state growth.

---

## 17. Preserve-v1 guardrails

Horse-A v2 is not accepted by improving one pathology at arbitrary cost.

Before timing a candidate, declare:

```text
target regime
protected regimes
latency guardrail
allocation guardrail
retained-state guardrail
build/lifecycle guardrail
tail/cliff guardrail
compiler-profile confirmation surface
```

At minimum protect the known Horse-A v1 strengths:

```text
E1 / E2 / E5 local behavior
container behavior
Q3 / Q4 large-document behavior
low edit-path allocation profile
structural locality
correctness
eager READY deterministic completion
```

A candidate that fixes E6 but materially harms these surfaces is not
implicitly better.

The exact numeric acceptance thresholds must be frozen before holdout timing is
revealed. They are not v1 experimental facts and should not be retroactively
chosen to fit the candidate.

---

## 18. Minimum path to a Horse-A v2 freeze

Do not recreate the R0–RQ8 governance stack.

### Step 1 — one comparison contract + one diagnostic table

Inherit v1 semantic authority, oracle, workload identities, capsule, and
measurement boundaries.

Freeze only:

```text
current diagnostic question
target and protected regimes
same-READY comparison rules
resource budgets
holdout rule
stop conditions
```

Run the E6 + tiny same-READY cost diagnosis first.

### Step 2 — at most two challengers

Select:

```text
one minimum repair
+
one candidate that meaningfully challenges it
```

A challenger is not required merely to fill a slot. Fail candidates early on
correctness, lifecycle cost, or targeted falsification.

Each candidate must state:

```text
mechanism identity
retained state
reuse/certification rule
fallback/termination rule
construction + maintenance + retirement ownership
one decisive counterexample / falsification case
```

### Step 3 — one confirmation

For the surviving candidate:

```text
unblind project holdout
check the key primary and LTO-off profiles
run short and sustained traces when the candidate retains new state
apply predeclared target/guardrail criteria
```

Return one of:

```text
V2_MECHANISM_FREEZE
INCONCLUSIVE / MORE_EVIDENCE_REQUIRED
REJECT
NO_NEW_MECHANISM
```

A full new 555k-row campaign, complete formal model, or broad PMU campaign is
not automatically required before mechanism freeze. They are earned only by the
claims and risks that remain after the targeted research.

---

## 19. Immediate decision tree after the first diagnosis

```text
E6 / tiny same-READY diagnosis
        |
        +-- repeated incremental work dominates
        |       -> test earlier rebuild / remove duplicate work
        |
        +-- effective binding often unchanged
        |       -> test exact effective-change certification
        |
        +-- affected set is sparse but expensive to find
        |       -> compare coarse scan/summary
        |          -> reverse index only if simpler discovery is the bottleneck
        |
        +-- many eager outputs genuinely must change
        |       -> accept necessary linear work / prefer coarser rebuild
        |
        +-- direct READY rebuild does not improve tiny
                -> stop tiny selector research
```

This decision tree is intentionally capable of deleting whole research
branches.

---

## 20. Non-goals

This phase does not authorize:

```text
mutating Horse-A v1
production Markdown editor implementation
adding a dependency graph because E6 is slow
full hardware-counter enumeration
parallel Horse-A / Horse-A-P
packing the AVL without new evidence
custom allocators / huge pages / prefetch / SIMD by default
ML or multi-feature fallback prediction before a simple rule fails
large new benchmark campaigns before targeted falsification
changing eager READY semantics implicitly
```

---

## 21. Expected research outputs

Keep the outputs small and decision-oriented:

### O1 — deletion / necessity map

```text
Horse-A v1 component
-> correctness/READY obligation
-> measured benefit
-> measured cost
-> KEEP / SHRINK / DELETE / REPLACE / UNKNOWN
```

### O2 — avoidable-work diagnosis

```text
case
-> actual output work
-> conservative certification work
-> repeated/path-selection work
-> state/lifecycle cost
```

### O3 — candidate state/work comparison

```text
candidate
-> work removed
-> new retained state
-> ordinary-edit maintenance
-> build/retirement cost
-> correctness boundary
-> protected-regime risk
```

### O4 — v2 decision

```text
V2_MECHANISM_FREEZE
or
NO_NEW_MECHANISM / INCONCLUSIVE
```

---

## 22. Final research principle

Horse-A v2 is not an accumulation phase.

```text
DELETE FIRST
    -> explain the work that remains
    -> add only the minimum new authority
    -> prove that new state pays for itself
    -> preserve the v1 strengths that still matter
```

History is evidence, not obligation.

The first task is therefore **not** to design a semantic dependency system,
collect a larger PMU matrix, or parallelize Horse-A.

The first task is:

> **E6 + tiny same-READY cost decomposition: separate repeated incremental
> work, READY-state construction/lifecycle tax, and truly unavoidable semantic
> output work.**

That result decides what Horse-A v2, if any, should become.