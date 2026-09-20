# BLOCK-REGION-EFFICIENCY-CONTRACT-v1 — B vs N separation (NOT EXECUTED)

Status: **DOCUMENTATION ONLY — DEFERRED CONTROLLED LANE**
Authority: issue #35 Corrective-1 §7 (add Block Update Efficiency as a
controlled research dimension) + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`
§9 (separate B from N).

```text
CORRECTIVE-A EXECUTES NO SWEEP IN THIS FILE.
No B-sweep, no N-sweep, no timing, no ranking. This document freezes how the
dimension must be defined when it is executed, so a later campaign cannot
infer a block-size law from a confounded measurement.
```

---

## 0. The two quantities

```text
B = target affected block/region size
N = total document size
```

The rule the contract exists to enforce:

> Do not infer a block-size law from a sweep in which B and N grow together
> without qualification.

A synthetic document that gets longer by getting a bigger block confounds
local region processing cost with whole-document navigation, metadata, and
retained-state cost. Both are interesting; they are different experiments.

## 1. Complementary slices

```text
B-sweep   hold N approximately fixed
          hold edit operation + expected transition type fixed
          vary B

N-sweep   hold B approximately fixed
          hold edit operation + expected transition type fixed
          vary N
```

"Hold fixed" means held as closely as the construction allows, and the
residual movement is reported rather than asserted away:

```text
record unavoidable covariates
  block count
  node count
  container depth
  actual observed semantic impact
```

The purpose is to distinguish:

```text
local region processing cost
        from
whole-document navigation / metadata / retained-state cost
```

## 2. Candidate controlled families

To be chosen **after** real observations, not before:

```text
paragraph
fenced block
table
list / container region
```

The families are candidates. Selecting one is a later, separately reviewed
decision, and the choice must be justified by something observed in the real
workload — not by convenience.

## 3. What this lane is not

```text
- it is an explanatory controlled lane: it does not replace real files;
- it must NOT be used to select the primary corpus (#35 §4/§7): real-file
  selection is coverage-oriented and mechanism-blind, and no controlled
  sweep result may influence membership;
- it is not a substitute for the controlled synthetic suite of #22
  (`CONTROLLED_PROTOCOL_SET`), which keeps its own frozen protocol;
- it produces no result in CORRECTIVE-A, and its execution is not part of
  the G1-G8 freeze gates of #35 §10.
```

## 4. Prerequisites before execution

```text
1. workload freeze for the real-file campaign (or an explicit statement that
   the sweep is a standalone controlled lane);
2. the frozen lane + oracle + predicate contracts of CORRECTIVE-A
   (grammar/GRAMMAR-LANES-v1.md, grammar/TRANSITION-ORACLE-v1.md);
3. a frozen definition of the affected region for each family — this needs
   the correspondence rule that
   workloads/attribution/SEMANTIC-INTERFERENCE-CONTRACT-v1.md has not yet
   frozen;
4. the #22 timer contract unchanged (T_total = T_prepare + T_native, AST-ready
   boundary); no render-ready or whole-project claim may be attached.
```

Prerequisite 3 is the reason this lane cannot start in CORRECTIVE-A: "B"
cannot be measured as *affected* region size until the pre/post
correspondence rule exists, and measuring it as *authored* region size
instead would silently redefine the quantity.
