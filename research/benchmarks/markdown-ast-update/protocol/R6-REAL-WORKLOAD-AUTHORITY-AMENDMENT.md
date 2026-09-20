# R6 Real-Workload Authority Amendment

Status: **CORRECTIVE-A CONTRACT AMENDMENT / PRE-MEASUREMENT**

Authority chain:

```text
#22 / R0-R5
  -> H0-H4 mechanism identities
  -> BENCH-GRAMMAR-v1 / normalized-result authority
  -> canonical edit / CaseId / runner / timer / controlled protocol

PR #34
  -> acquired candidate-source authority
  -> source-lock / SOURCE / inventory / materialization / verification

#35 + this amendment
  -> Stage A real-workload construction authority
  -> grammar-lane eligibility
  -> profiler contract
  -> selection / syntax coverage / payload lifecycle
  -> workload freeze gates

#31
  -> post-freeze performance / work / resource / attribution execution

#33
  -> paper-level synthesis, regime map, Weakness Map, Design Decision Matrix
```

This amendment resolves execution ambiguities between the original R6 project-performance amendment and the later real-workload authority in #35. It does not reopen R0-R5 timer, correctness, parity, or controlled-protocol contracts.

---

## 1. Supersession map

For Stage A workload construction, the current effective authority is:

1. #35 body,
2. #35 Corrective-1,
3. #35 Workload Construction Algorithm v1,
4. this amendment where repository protocol wording conflicts with those later clarifications.

The following older requirements are narrowed or deferred for the first frozen real-file campaign:

### Render-ready

`T_render_ready` is **not** required to grant the first AST-ready workload freeze.

The first strict performance campaign may be:

```text
measurement_boundary = AST_READY
headline_timer        = T_total = T_prepare + T_native
```

A render-ready lane requires a separately frozen renderer/output/completion contract. `T_total` must never be renamed or interpreted as `T_render_ready`.

### Whole-project results

The first real-file mechanism campaign may use the frozen selected-file workload. It must not call selected-file results whole-project throughput.

A later `REAL_PROJECT` campaign requires an explicit project manifest with all included files/bytes and denominator rules.

### Chained traces / long sessions

`SINGLE_RESET` is sufficient for the first strict real-file mechanism campaign.

`LOCAL_BURST` / `DOCUMENT_SESSION` remain required later for longitudinal/state-history questions, but do not block the first AST-ready workload freeze unless the campaign claims long-session behavior.

### Resource conclusions

Memory / retained-state results are required before any final memory-latency Pareto or Design Decision Matrix claim that depends on them. They do not need to block a narrower timing-only first campaign if the corresponding resource conclusions are explicitly deferred.

---

## 2. Gate mapping

The former R6 gate:

```text
PROJECT_CORPUS_FREEZE_PASS
```

is superseded as a Stage A execution gate by the following #35 gates:

```text
G1 SOURCE_AUTHORITY_PASS
G2 GRAMMAR_ELIGIBILITY_PASS
G3 PROFILER_CONTRACT_PASS
G4 SELECTION_VALIDITY_PASS
G5 SYNTAX_TRANSITION_COVERAGE_PASS
G6 PAYLOAD_LIFECYCLE_PASS
G7 HARNESS_ADAPTER_PASS
G8 CLAIM_BOUNDARY_PASS
```

Full Stage A closure:

```text
G1..G8 PASS
=> REAL_WORKLOAD_FREEZE_PASS
```

A narrower core-only freeze is permitted only when all eight gates are satisfied for the explicitly named core lane:

```text
CORE_REAL_WORKLOAD_FREEZE_PASS
```

The freeze record must state:

```text
grammar_lane
measurement_boundary
included workload sets
included trace forms
included result/claim classes
deferred lanes / claims
```

A partial freeze never implies that unfinished GFM/CommonMark extensions, math, render-ready, whole-project, long-session, or resource claims are complete.

---

## 3. Grammar lane contract: semantic qualification != implementation qualification

Every strict comparison lane must satisfy two independent gates.

### 3.1 Semantic qualification

Freeze:

```text
grammar_id + version
base dialect
extensions and precedence
normalized node/result vocabulary
reference oracle
file/edit pre-state eligibility
file/edit post-state eligibility
```

A parser merely accepting input is not semantic qualification.

For current planning, names such as `COMMONMARK_GFM` are placeholders until an exact base specification/version and extension configuration are frozen.

### 3.2 Horse implementation qualification

For every horse participating in that grammar lane, separately prove:

```text
correctness against the lane oracle
implementation parity under the lane semantics
eager completion at the frozen AST-ready boundary
failure behavior / unsupported cases retained honestly
```

A grammar lane may exist for profiling/realism before all H0-H4 are implementation-qualified. Such cases must not enter strict H0-H4 performance comparison until both qualification layers pass.

---

## 4. Transition truth is mandatory for active syntax coverage

An edit payload is not accepted merely because:

```text
bytes changed
post hash reproduced
```

Every active syntax-coverage payload must have frozen transition assertions:

```text
expected_pre
expected_post
```

and validation must prove them under the declared grammar/reference oracle.

Conceptual sequence:

```text
pre source
-> reference parse/oracle
-> expected_pre holds

apply CanonicalEdit
-> post source
-> reference parse/oracle
-> expected_post holds
```

Examples:

```text
TABLE_DELIMITER_BREAK:
  Table -> non-Table interpretation

FENCE_CLOSE_BREAK:
  closed fence -> unclosed/forward-propagating fence state

ATX_HEADING_TOGGLE:
  Heading -> Paragraph (or reverse)
```

If the frozen result vocabulary/oracle cannot express the claimed transition, the payload is not active coverage for that syntax lane.

---

## 5. Semantic interference: attribution model, not naive AST diff

The research chain remains:

```text
EDIT
-> semantic effect under frozen grammar
-> mechanism work
-> latency / memory
-> causal attribution
```

However, semantic-interference metrics are **not** defined by an ordinary raw AST diff.

Before any quantitative interference metric is promoted, its contract must define at least:

```text
pre/post coordinate mapping through the canonical edit
node/content correspondence rule
content change vs structural change vs reference-resolution change
pure span/position shift handling
ancestor double-count policy
non-contiguous affected-range representation
union size vs enclosing span
```

Required distinctions:

```text
CONTENT_CHANGE
STRUCTURE_CHANGE
DEPENDENCY_RESOLUTION_CHANGE
POSITION_SHIFT_ONLY
```

A source insertion near the document start must not turn every shifted span into semantic change merely because absolute offsets moved.

A same-length text replacement must not disappear merely because node kind/topology/spans remain stable.

A distant reference-definition edit may affect a non-contiguous set of consumers; its union size and enclosing span must not be conflated.

Any resulting quantity must be described as:

> observed semantic impact under the frozen grammar and correspondence rule

not as the parser's mathematically minimal required work.

Internal parser-state propagation is a separate concept. If studied, it requires an independently frozen reference-state definition and must not be conflated with H4 restart/convergence distance.

### Corrective-A scope

A full semantic-interference metric implementation is **not required** for CORRECTIVE-A.

CORRECTIVE-A must first freeze:

```text
grammar lanes / eligibility
reference oracle
expected pre/post transition predicates
profiler semantics
```

Interference quantification may be added later in the attribution contract once those semantic foundations exist.

---

## 6. Profiler authority

Profiler output must distinguish three fact classes:

```text
SOURCE_FACT
SYNTAX_FACT(grammar_id, span, status)
STRUCTURAL_FACT(grammar_id, definition)
```

Profiler contract must freeze:

```text
profiler id/version
base dialect / extension config
precedence rules
block/node counting rules
largest-block definition
container-depth definition
span representation
UNKNOWN / AMBIGUOUS policy
validation / spot-check policy
```

A deterministic misclassifier is not sufficient evidence. Syntax spans must remain inspectable against source bytes/reference parsing.

---

## 7. Selection authority

The selected real-file set is coverage-oriented relative to the frozen candidate population. It must not be described as a frequency-weighted sample of all Markdown use.

Before selection freeze publish candidate -> eligible -> selected comparisons for:

```text
project/domain contribution
file/byte distribution
key structural features
syntax coverage
eligibility bias
known near-duplicate / translation / version relationships where detectable
```

Selection rules must freeze:

```text
feature bins / transforms
zero-heavy handling
project cap
domain cap
tie-break
stop rule
set-cover target units
```

No H0-H4 timing/work/resource fact may influence selection.

---

## 8. Payload lifecycle

Every payload/trace step must retain enough information to reconstruct its history:

```text
grammar_id
base_source_sha256
pre_source_sha256
post_source_sha256
trace_id
trace_form
step
syntax_target
expected_transition
canonical edit bytes/content reference
requested position
actual anchor / actual relative position
```

Rules:

- EARLY/MIDDLE/LATE candidates that resolve to the same anchor are deduplicated; they are not three-position coverage.
- RESTORE starts from the frozen broken pre-source, not silently from the original clean source.
- In chained traces, each step's coordinates belong to that step's pre-source.
- Two histories reaching byte-identical source may remain distinct trace identities because retained mechanism state can differ.
- A digest alone is not sufficient to reconstruct inserted content.

---

## 9. Block / region update efficiency: separate B from N

For controlled scaling, define:

```text
B = target affected block/region size
N = total document size
```

Do not infer a block-size law from a sweep in which B and N grow together without qualification.

Use complementary slices when feasible:

### B-sweep

```text
hold N approximately fixed
hold edit operation + expected transition type fixed
vary B
```

### N-sweep

```text
hold B approximately fixed
hold edit operation + expected transition type fixed
vary surrounding document scale N
```

Record unavoidable covariates such as block count, node count, depth, and actual observed semantic impact.

The purpose is to distinguish:

```text
local region processing cost
from
whole-document navigation / metadata / retained-state cost
```

---

## 10. Measurement-start rule

No formal H0-H4 performance result may enter #31 until the relevant workload/grammar lane is frozen under this amendment.

Allowed before freeze:

```text
source verification
static profiling
reference parsing/oracle validation
transition validation
CaseId/load/dry-run correctness checks
```

Forbidden for workload selection/freeze decisions:

```text
H0-H4 latency
horse work counters
horse allocations/memory
restart/convergence/reuse behavior
PMU/perf data
```

After workload freeze, #31 owns timing/work/resource execution. #33 owns promoted research conclusions and final design synthesis.
