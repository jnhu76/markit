# FOLLOWUP-CANDIDATES — MARKIT-31 primary analysis

Candidates for the next experimental gate (task §49). This list is INPUT
to `CONTROLLED-FOLLOWUP-PLAN-v1.md`; it is not itself authority to run
anything. Ordered by (materiality x unexplainedness).

```text
F1  H1 fallback boundary (R1)          which byte/block structure forces
                                       block-local fallback? wins on owasp/
                                       crafting vs losses on rust-rfcs/
                                       rust-book are the single largest
                                       unexplained ranking split. [P1,P2]
F2  E6 definition-environment cost (R3) environment rebuild vs node
                                       rematerialization vs consultation:
                                       branch-miss doubling + near-full
                                       inspection with structure reuse.
                                       Strongest DQ5 residual. [P4,P5]
F3  H4 convergence scaling (R5)        latency tracks convergence distance
                                       (Spearman 0.667 E4; E6 median 3225B);
                                       no controlled propagation-length
                                       sweep exists. [P8]
F4  CLEAN_STATE premium (R4)           all incremental horses 0.68-0.86x on
                                       clean build; premium composition
                                       unknown beyond "parse + aux state +
                                       churn". Bounds all small-file
                                       crossovers. [P6,P7]
F5  H1 small-file crossover placement  H1 loses below 8KiB but wins 3.2x
                                       above; is the knee block-count- or
                                       byte-driven? [O5/O6]
F6  allocator sensitivity              replay regime shows ~25-30% of
                                       cycles in malloc/free family
                                       (PROFILE_ONLY). Whether allocation
                                       shapes PRIMARY rankings is unknown;
                                       primary allocator policy is frozen.
F7  deep-container flip (R5)           depth>=3 flips H2/H3/H4 (n=45);
                                       confirm with controlled container
                                       depth before it enters the Weakness
                                       Map as a law. [P8]
```

Explicitly NOT candidates (already answered at available evidence level):
E1/DQ1 ranking, MQ2 avoided-work linkage, MQ5 locality bookkeeping,
MQ6 verification-cost boundary, MQ7 qualification set.
