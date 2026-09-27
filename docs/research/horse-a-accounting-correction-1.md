# HORSE-A-ACCOUNTING-CORRECTION-1 — mechanically correct structural attribution bounds before collection

```text
STATUS                     = REVIEW-FIX COMPLETE, AWAITING FOCUSED INDEPENDENT
                            ACCOUNTING RE-REVIEW
CORRECTION_TYPE            = PRE-COLLECTION_MECHANICAL_ACCOUNTING_CORRECTION
IMPLEMENTATION_BASELINE    = PR #70 @ 653d5a4f10605fe17ded6f6d703656f9950c97c4
IMPLEMENTATION_BASE_BASE   = master @ 9dc365ed878ba9f0fda877dc6daf6d300c292d73
FIRST_CANDIDATE_HEAD       = 39de28e075477f99d241f170f771ac344b1b5993
REVIEW_FIX                 = YES (independent review P0=0 / P1=2 / P2=2 / P3=1;
                            every finding closed by this pass — §0)

MECHANISM_IDENTITY_CHANGED = NO
W_A1_CHANGED               = NO
W_A2_CHANGED               = NO
W_A3_CHANGED               = NO
SCHEMA_VERSION_CHANGE      = NO   (HORSE-A-STRUCTURAL-COUNTERS-v1 unchanged)

STRUCTURAL_COLLECTION_RUN   = NO
PERFORMANCE_COLLECTION_RUN  = NO
STRUCTURAL_COLLECTION_AUTHORIZED = NO
PERFORMANCE_COLLECTION_AUTHORIZED = NO

PR_70_CODE_MODIFIED        = NO   (authority first; implementation follows in a
                                   separate task after independent review)
METHODOLOGY_FAIL           = NO   (every bound below is derived from the frozen
                                   algorithm + symbolic geometry only; the
                                   observed 19/25/33 diagnostic is used ONLY as
                                   a post-derivation consistency check)
```

## 0. REVIEW FIX (this pass)

The first correction candidate (PR #72 @ `39de28e…`) underwent independent
review. Findings and their closure in this pass:

```text
P1-01  the join_with_pivot operator formulas were internally inconsistent
       with the split/f2 compositions that used them (operator visits
       4δ+3 vs compositions in 4δ−3; operator rotations 2δ+2 vs
       compositions in 2(δ−1); links 8δ−7 built from a "2 per
       non-terminal spine level" ledger that contradicts the 1-per-level
       unwind-relink ledger the same record used for remove_max).
       CLOSED: §6.2 now derives ONE piecewise operator lemma
       (compatible δ ≤ 1 / recursive δ ≥ 2 with spine depth t ≤ δ−1)
       under ONE uniform link-write ledger; §6.3/§6.4 and the durable
       spec (horse-a-v1-algorithm.md §12.1.1/§12.3.1/§16) recompose
       split / remove_max / f2 / f3 / the #60 rows from that lemma alone.

P1-02  the durable spec simultaneously carried the old active split proof
       (2δ_i+1 / δ_i+1 / 6H / 3H / 86H / 40H) and the corrected §16
       formulas.
       CLOSED: horse-a-v1-algorithm.md now contains exactly ONE active
       split proof; the superseded values remain only under explicit
       SUPERSEDED markers.

P2-01  the #59 §9.1 explicit-stack workspace obligation was described as
       optional follow-up.
       CLOSED: §9 records it as MUST RESOLVE before #62
       implementation-conformance closure and structural collection
       authorization (resolution A or B; not chosen here).

P2-02  the certificate_read unit definition and the candidate
       presence-filter clarification contradicted each other textually.
       CLOSED: §7 carries one self-contained normative rule.

P3-01  selected-restart support safety was left as an unresolved
       mechanism question.
       CLOSED: §7.1 proves it by position from the frozen geometry
       (STRUCTURALLY_PROVEN_BY_POSITION; no dynamic support-touch check
       required).
```

Mechanism identity, restart/convergence/coverage/facts semantics,
retirement semantics, W-A1/W-A2/W-A3, counter schema (v1), and the witness
geometry are unchanged by this pass. PR #70 production Rust is untouched;
no structural or performance collection has been run or authorized.

## 1. TRIGGER

Independent I5 review of PR #70 observed, in a correctness-only conformance
probe against the already-frozen #60 geometry, that
`fact_range_node_visits` measured **19 / 25 / 33** on the
128 KiB / 1 MiB / 16 MiB cells while the frozen #59 §21.1 / #60 §9.5
threshold is **≤ H = 14 / 18 / 24**. Those observations are diagnostic
evidence of a contract inconsistency only; no bound in this record is
derived from them, and they are not treatment results.

## 2. OLD AUTHORITY (verbatim rules being corrected)

From #59 §20 (structural event ledger, frozen):

> fact-range lookup
>     charges: 1 visit per weighted-descent path node (same rule as locate).

From #59 §21.1 (operator formulas, frozen):

```text
locate (weighted descent)     visits ≤ H            reads ≤ H
safe_predecessor              visits ≤ 3H − 2       reads ≤ 3H − 2
cursor (init + advance)       visits ≤ 2H + 4k + Q  reads ≤ 2(2H + 4k)
fact-range lookup             visits ≤ H            reads ≤ H
split(T)                      … reads ≤ 86H;  writes ≤ 40H
remove_max(T)                 … link writes ≤ 6d − 5
                              recomputes ≤ 3(d−1): reads ≤ 24(d−1),
                                                   writes ≤ 12(d−1)
join_with_pivot(δ)            visits ≤ 2δ + 1;  rotations ≤ δ + 1
                              link writes ≤ 6δ + 9
                              reads ≤ 25δ + 35; writes ≤ 12δ + 16
```

And the #59 §21.3 / #60 §9.2 compositions derived from them:

```text
f2 visits      ≤ 32H + 2        f2 rotations ≤ 16H + 1
f2 link writes ≤ 68H + 25
f3 reads       ≤ 279H + 101     f3 writes    ≤ 128H + 40
```

Superseded formulas are retained here and in the frozen issues' history;
they are not deleted.

## 3. CONCRETE CONTRADICTION (code level, baseline 653d5a4)

`OrderedFacts::of_old_replacement` (mechanisms/horse-a/src/facts.rs:37–66)
positions once at `replace_lo` and then consumes **exactly Δ_old**
replacement Owners:

```rust
let mut cursor = owners.cursor_at_rank(ranks.start, StructuralOp::FactRange, sink);
for _ in ranks {                                   // exactly Δ_old iterations
    let Some(item) = cursor.next(StructuralOp::FactRange, sink) else { … };
    sink.old_fact_owner_visit();
    … collect_definition_facts(&payload.root, …)
}
```

`OwnerCursor::next` (mechanisms/horse-a/src/cursor.rs:153–184) charges
**one visit per successor-walk node entering the cursor stack** (the left
spine of the yielded node's right subtree) to the supplied operator —
`FactRange`. The frozen §20 rule, however, defines successor-walk charging
only under *cursor advancement* (the convergence candidate cursor), and
defines *fact-range lookup* as descent-only. The implementation's own
`StructuralOp::FactRange` doc ("…and its sequential advance") is a
post-hoc redefinition that the frozen authority does not contain; PR #70
comments cannot redefine frozen counter semantics (#62 authority order).

Consuming exactly Δ_old Owners from a positioned cursor **requires**
Δ_old successor advances in any realization of the frozen algorithm
(there is no other way to reach Owners at consecutive ranks); therefore
the frozen `fact_range_node_visits ≤ H` omits mechanism-required work,
while no other frozen counter represents it:

- `cursor_node_visits ≤ 2H + 4k + Q` was derived for the **convergence
  candidate cursor** (#59 §8), whose "at most once per update
  transaction" successor-stack amortization belongs to that single walk;
  a second, unrelated fact cursor must not silently consume that budget
  (counter authority follows mechanism responsibility, not Rust type
  names).
- `old_fact_owner_visits` counts **Owner payload inspections** (one per
  yielded replacement Owner, facts.rs:53) — a different unit from AVL
  path/stack navigation; it cannot absorb successor-walk visits without
  becoming ambiguous.

## 4. WHY THIS IS PRE-COLLECTION

No structural or performance collection has been authorized or run
(#62: STRUCTURAL COLLECTION = NOT AUTHORIZED). The only execution
evidence is the correctness-only conformance probe described in §1. This
record corrects accounting **authority** before any treatment cell is
collected, so that the frozen #60 threshold table never has to be
regenerated after observing Horse-A results (which would be a new
experiment version).

## 5. CORRECTED DERIVATION — fact range

Symbols: H = AVL height bound of the tree the operator runs on
(thresholds instantiate H_max per #60 §9.4); Δ_old = removed old Owners;
d = right-spine descent length; δ = join height gap; k, Q as frozen.

### 5.1 Resolution choice

**Option A — broaden `fact_range_node_visits`** to
`initial seek visits + fact-range successor traversal visits`, with the
corrected §20 charging rule:

```text
fact-range lookup
    charges: 1 visit per weighted-descent path node (same rule as locate)
    + 1 visit per successor-walk node entering the fact cursor stack
      during the exactly Δ_old sequential advances (the §20 cursor
      advancement rule applied to the fact-range cursor instance).
```

Option B (route to `cursor_node_visits`) is illegal (§3). Option C (new
counter) is unnecessary: v1 represents this work truthfully and
unambiguously as the fact-range operator's own navigation, with no
double charge (each cursor instance charges exactly its own operator;
F3). Deleting the successor charges to satisfy `≤ H` is forbidden and
would hide real AVL navigation work.

### 5.2 Lemmas

- **L1 (seek).** The weighted rank descent visits exactly the
  root-to-target path; AVL heights decrease ≥ 1 per level ⇒ ≤ H visits.
  (Same rule and proof as locate.)
- **L2 (per-advance spine).** One `next()` pops one frame and pushes the
  left spine of the yielded node's right subtree. The right subtree of
  any node has height ≤ H − 1, and a left spine has length ≤ its
  subtree's height ⇒ ≤ H − 1 visits per advance.
- **L3 (advance count).** Consuming exactly Δ_old Owners from a cursor
  positioned at `replace_lo` performs exactly Δ_old advances (the
  positioned Owner is yielded by the first advance; the final advance's
  spine push is the frozen cursor substrate's look-ahead and is bounded
  by L2). The walk terminates after exactly Δ_old iterations — never an
  enumeration of the retained suffix (F2).

**Composition (retained conservative bound):**

```text
fact_range_node_visits ≤ H + Δ_old·(H − 1)
fact-range reads       ≤ 1 + 2H + 2·Δ_old·(H − 1)
```

(The read formula: 1 root `subtree_records` + `subtree_bytes` and
`subtree_records` of each descent node's left child (the frozen cursor
substrate carries byte base AND rank per frame) + the same two fields
per pushed successor node. Amortized refinement — each node enters the
successor stack at most once per walk, so the walk also fits
H + |interval closure| ≤ 3H + Δ_old — is noted but not frozen; the L2
composition is conservative and mechanical.)

**Mechanical instantiation for the frozen witness (Δ_old = 2, H_max =
14 / 18 / 24):**

```text
fact_range_node_visits ≤ 3H − 2      → 40 / 52 / 70
```

### 5.3 Post-derivation consistency check (not an input)

The frozen witness builds its old state with the deterministic
middle-split bulk build, so the seek/spline counts are exactly
derivable from the geometry (replace_lo = M/2 − 1 sits one left of the
root on the witness cells):

```text
128 KiB : seek 10 + advance spines 0 + 9  = 19
1 MiB   : seek 13 + advance spines 0 + 12 = 25
16 MiB  : seek 17 + advance spines 0 + 16 = 33
```

The independently derived bound (40/52/70) contains the observed values,
and the observed values are fully explained by the frozen geometry —
no residual inconsistency. (Check performed after derivation; F5.)

## 6. CORRECTED DERIVATION — same-pass mechanical consistency audit

The fact-range defect exposed systematic omissions in the neighboring
frozen formulas. Each correction below is derived from the frozen
algorithm's own event structure; none is fitted to any measurement.

### 6.1 locate / safe_predecessor / fact-range aggregate reads (8A)

The frozen read formulas assumed ≤ 1 aggregate field read per visit.
The frozen **algorithm itself** requires more: every weighted descent
that returns rank + base (frozen result contract) must read BOTH
`subtree_bytes` and `subtree_records` of each descent node's left
child, and the safe-predecessor phases additionally read
`subtree_has_safe` steering fields.

```text
locate reads            ≤ 2H + 1     (was ≤ H)
safe_predecessor reads  ≤ 6H         (was ≤ 3H − 2)
   = 2 (root: subtree_bytes bound + subtree_has_safe prune)
   + 2H (phase-1 descent: bytes+records per node)
   + (H−1) (phase-2 has_safe_of(left) per examined ancestor)
   + 2 (base/rank of the selected boundary)
   + 3(H−1) (guided descent: has_safe_of(right) + bytes + records)
fact-range reads        ≤ 1 + 2H + 2Δ_old(H−1)   (was ≤ H)
```

Implementation over-reads (PR #70 `child_meta` reads height +
subtree_has_safe even where only bytes/records steer — sequence.rs
locate_by_byte / safe_predecessor phase 1–2 / rightmost_certified):
classified `IMPLEMENTATION_CONFORMANCE_DEFECT`, to be narrowed in the
implementation follow-up (§10). The authority is corrected to the
mechanism-required reads, not to the implementation's convenience
reads.

### 6.2 Operator lemmas under ONE uniform event ledger (8B + P1-01 repair)

The first candidate corrected the old #59 §21.1 operator formulas but left
two inconsistencies (review P1-01): it charged the remove_max unwind relink
as 1 link write per level while building join_with_pivot's link bound from
"≤ 2 per non-terminal spine level", and it published operator rotations
≤ 2δ+2 while its own split/f2 compositions substituted 2(δ−1). This
section re-derives both operators under ONE uniform ledger and publishes
ONE piecewise lemma; every composition in §6.3 and every durable-spec
formula substitutes that lemma only.

**Uniform link-write ledger rule** (a mechanical reading of the frozen
#59 §7.1 / spec §15.2 unit "one mutation/reassignment of a persistent Link
slot that changes which subtree the slot owns; installing into a persistent
slot IS a link write"):

```text
L1  take(S) followed by a re-install into S within the same operator
    frame (the by-value recursive descent/unwind shape) is that slot's
    single reassignment                        → 1 link write total.
L2  take(S) whose slot is handed to another operator or left detached
    (pivot isolation, child handoff, final detach)
                                                → 1 link write.
L3  install into S not covered by L1             → 1 link write.
R   rotation events: single = 3 slot writes, double = 6 (frozen §15.2
    convention; code-audited: rotate_left = node.right install
    + pivot.left take/reinstall pair + slot install = 3).
```

#### 6.2.1 remove_max — full audit (confirmed)

`d` = number of non-empty nodes on the rightmost descent path, including
the extracted maximum (`d ≤ h(T)`, `d ≥ 1`). Event table from the baseline
code (sequence.rs `remove_max`):

| event | count | visits | links | recomputes | reads | writes |
|---|---:|---:|---:|---:|---:|---:|
| right-spine descent (null-check branch, slot reads only) | d | d | 0 | 0 | 0 | 0 |
| pivot detach (`pivot.left.take()`, rule L2) | 1 | 0 | 1 | 0 | 0 | 0 |
| unwind relink (`node.right = rest`, rule L1) | d−1 | 0 | d−1 | 0 | 0 | 0 |
| unwind recompute | d−1 | d−1 | 0 | d−1 | ≤8(d−1) | 4(d−1) |
| rebalance decisions (bf + child bf) | d−1 | 0 | 0 | 0 | ≤4(d−1) | 0 |
| rotation events (≤1 action/level; participants 1/2, recomputes 2/4, writes 3/6) | ≤1/level | ≤2(d−1) | ≤6(d−1) | ≤4(d−1) | ≤32(d−1) | ≤16(d−1) |

```text
remove_max visits      ≤ d + (d−1) + 2(d−1) = 4d − 3        (valid ∀ d ≥ 1)
remove_max rotations   ≤ 2(d − 1)
remove_max link writes ≤ 1 + (d−1) + 6(d−1) = 7d − 6        (was 6d − 5)
remove_max recomputes  ≤ (d−1) + 4(d−1)   = 5(d − 1)
remove_max reads       ≤ 8(d−1) + 4(d−1) + 32(d−1) = 44(d − 1)
remove_max writes      ≤ 4·5(d−1)         = 20(d − 1)
```

The first candidate's `7d − 6` survives this independent re-audit; its
`4d − 3 / 2(d−1) / 5(d−1) / 44(d−1) / 20(d−1)` are confirmed unchanged.

#### 6.2.2 join_with_pivot — piecewise lemma (P1-01 repair)

`δ = |h(left) − h(right)|` (frozen #59 §7.2 symbol). The baseline code
(sequence.rs `join_with_pivot` / `join_right` / `join_left`, mirrors)
has two structurally different cases:

**Compatible case δ ≤ 1** — attach both sides directly under the pivot:

```text
visits ≤ 1 (the pivot-attach processing; #59 §20 row (b))
rotations = 0
link writes ≤ 2 (pivot.left / pivot.right installs, rule L3)
recomputes = 1
reads ≤ 2 (prologue heights) + 8 (recompute) = 10
writes = 4
```

**Recursive case δ ≥ 2** — descend the inner spine of the taller tree.
Let `t` = number of spine calls (terminal included), `t ≥ 1`.

Spine-depth bound: AVL heights decrease by ≥ 1 per spine step
(`h(s_{i+1}) ≤ h(s_i) − 1` because `h(s_i) = 1 + max(h(l), h(r))`), every
non-terminal level satisfies `h(s_i.right) > h(short) + 1` hence
`h(s_i) ≥ h(short) + 3`, and the terminal level satisfies
`h ≤ h(short) + 3` (AVL: `h(l) ≤ h(r) + 1`). If `t ≥ 2`, applying the
descent to `s_{t−2}`: `h(short) + 3 ≤ h(s_{t−2}) ≤ h(s_0) − (t−2)`,
and `h(s_0) = h(short) + δ`:

```text
t ≤ δ − 1          (and t = 1 is legal only for δ ∈ {2, 3})
```

Event table (per #59 §20 ledger row: descent visit + pivot-attach visit +
unwind visit per level + rotation-participant visits, single = 1 / double
= 2 participants):

| event | count | visits | links | recomputes | reads | writes |
|---|---:|---:|---:|---:|---:|---:|
| prologue heights (`height_of` ×2) | 1 | 0 | 0 | 0 | ≤2 | 0 |
| spine descent decisions (`right_height` + `node.right` height per call) | t | t | 0 | 0 | ≤2t | 0 |
| terminal pivot attach (`mid.left`/`mid.right` installs L3 + `node.right` pair L1) | 1 | 1 | 3 | 1 (mid) | ≤8 | 4 |
| unwind recompute (`recompute(&mut node)`) | t | t | 0 | t | ≤8t | 4t |
| unwind relink (`node.right = Some(joined)`, rule L1) | t−1 | 0 | t−1 | 0 | 0 | 0 |
| rebalance decisions (bf + child bf per level) | t | 0 | 0 | 0 | ≤4t | 0 |
| rotation events (≤1 action/level) | ≤1/level | ≤2t | ≤6t | ≤4t | ≤32t | ≤16t |

```text
join visits      ≤ t + 1 + t + 2t = 4t + 1 ≤ 4δ − 3
join rotations   ≤ 2t             ≤ 2δ − 2
join links       ≤ 3 + (t−1) + 6t = 7t + 2 ≤ 7δ − 5
join recomputes  ≤ 1 + t + 4t     = 5t + 1 ≤ 5δ − 4
join reads       ≤ 2 + 2t + 8 + 8t + 4t + 32t = 46t + 10 ≤ 46δ − 36
join writes      ≤ 4(5t + 1)      = 20t + 4 ≤ 20δ − 16
```

**Continuity/monotone-envelope lemma.** Every δ ≥ 2 closed form evaluates
exactly to the compatible-case value at δ = 1:

```text
4·1−3 = 1   2·1−2 = 0   7·1−5 = 2   5·1−4 = 1   46·1−36 = 10   20·1−16 = 4
```

so the piecewise lemma is continuous at δ = 1 and the nondecreasing
envelope `max(compatible value, closed form)` is dominated by the closed
form for every `δ ≤ H`, `H ≥ 1`. Compositions (§6.3, split, durable §16)
substitute the closed forms directly; `δ = 0` always uses the compatible
row (the closed forms are negative or sub-direct there — the defect the
review flagged in the first candidate's universal `8δ − 7`).

Superseded intermediate candidate formulas — `4δ+3 / 2δ+2 / 8δ−7 / 5δ+6 /
46δ+54 / 20δ+24` (and the pre-correction `2δ+1 / δ+1 / 6δ+9 / 3δ+4 /
25δ+35 / 12δ+16`) — are retained HERE as labeled history only; they are
not active authority anywhere. Provenance of `8δ − 7`: it mirrors the
baseline instrumentation's own inconsistent charging — PR #70's
`join_right`/`join_left` charge `sink.link_writes(1)` at the
non-terminal child take AND `sink.link_writes(1)` in the common unwind
tail (2 per non-terminal level, actual charges 3 + 2(t−1) + ≤6t =
8t + 1), while the same file's `remove_max` charges the structurally
identical take/reinstall pair once per level and the terminal join
branch leaves its take uncharged. The unit-faithful §6.2 ledger charges
every such same-frame pair exactly once; the instrumentation
overcharges are recorded as implementation conformance defects (§10.6),
not legalized by broadening the authority.

#### 6.2.3 split — recomposed from the piecewise lemma

Frozen structural facts (unchanged): one search spine ≤ H; reconstruction
joins `J ≤ H`; `Σ δ_i ≤ 2H` (accumulator telescoping over the two monotone
outputs, #59 §7.4 step 3 — the height-window lemma stays withdrawn).
Split-local events from the baseline code (sequence.rs `split`): ≤ 2 link
writes per non-terminal spine node (the two child-slot takes handed to the
recursive split / the reconstruction join, rule L2 — the join's own
installs into the pivot's slots are charged inside the join lemma), ≤ 2
aggregate reads per spine node (`subtree_records` of the node and of its
left child), 0 recomputes of its own.

Using the universal envelopes `max(direct, closed) ≤ coefficient·δ +
direct-constant` (each row of §6.2.2: `V ≤ 4δ+1`, `R ≤ 2δ`, `L ≤ 7δ+2`,
`C ≤ 5δ+1`, `reads ≤ 46δ+10`, `writes ≤ 20δ+4` — equality at δ = 0 by
construction):

```text
split visits      ≤ H + Σ(4δ_i + 1)  ≤ H + 8H + H  = 10H   (retained threshold PROVED)
split rotations   ≤ Σ 2δ_i           ≤ 4H          ≤ 5H   (retained threshold PROVED)
split links       ≤ 2H + Σ(7δ_i + 2) ≤ 2H + 14H + 2H = 18H ≤ 22H (retained PROVED)
split recomputes  ≤ Σ(5δ_i + 1)      ≤ 10H + H     = 11H
split reads       ≤ 2H + Σ(46δ_i + 10) ≤ 2H + 92H + 10H = 104H   (was 147H + 1)
split writes      ≤ Σ(20δ_i + 4)     ≤ 40H + 4H    = 44H   (was 64H)
```

(The first candidate's `9H − 3` internal visit sum silently assumed every
reconstruction join has δ_i ≥ 1; a `join_with_pivot(None, node, None)`
inside split is legal and has δ = 0. The `4δ+1` envelope closes that gap
mechanically and still proves the retained 10H.)

Unchanged after re-derivation:

```text
locate / safe_predecessor / cursor / fact-range VISITS as frozen except
fact-range (§5); f1 ≤ 4H − 2; cursor ≤ 2H + 4k + Q
bulk_build / retirement / final root installation (1 link write, §20)
rows unchanged
```

### 6.3 Regenerated compositions (#59 §21.3 / #60 §9.2 style)

Witness instance bounds (frozen #59 §21.3, unchanged): `h(A) ≤ H`,
`h(BC) ≤ H`, `h(C) ≤ H`, `h(join(A, fresh)) ≤ H + 1`, `h(fresh) = 2`
(Δ_new = 2); extraction depths `d₁ ≤ H` (on A), `d₂ ≤ H + 1` (on
join(A, fresh)); join gaps `δ₁ ≤ H`, `δ₂ ≤ H + 1`; exactly 2 splits /
2 extractions / 2 top-level join_with_pivot (frozen §21.2 call counts);
Δ_old = 2. Per the §6.2.2 envelope lemma the closed forms are substituted
directly (they dominate the piecewise lemma for every δ ≤ H, H ≥ 1; every
substitution below is monotone).

```text
f2 visits
  ≤ 2·10H                        (splits, retained thresholds, §6.2.3)
  + (4d₁−3) + (4d₂−3)  ≤ (4H−3) + (4(H+1)−3) = 8H − 2   (extractions)
  + V(δ₁) + V(δ₂)      ≤ (4H−3) + (4(H+1)−3) = 8H − 2   (top joins)
  = 20H + 8H − 2 + 8H − 2 = 36H − 4

f2 rotations
  ≤ 2·5H                         (splits, retained)
  + 2(d₁−1) + 2(d₂−1) ≤ 2(H−1) + 2H = 4H − 2              (extractions)
  + R(δ₁) + R(δ₂)      ≤ (2H−2) + 2(H+1)−2 = 4H − 2       (top joins)
  = 10H + 4H − 2 + 4H − 2 = 18H − 4

f2 link writes
  ≤ 2·22H                        (splits, retained)
  + (7d₁−6) + (7d₂−6)  ≤ (7H−6) + (7(H+1)−6) = 14H − 5   (extractions)
  + L(δ₁) + L(δ₂)      ≤ (7H−5) + (7(H+1)−5) = 14H − 3   (top joins)
  + 4                               (bulk attach, ≤ 2·Δ_new)
  + 1                               (final root installation, §20 ledger)
  = 44H + 14H − 5 + 14H − 3 + 5 = 72H − 3

f3 aggregate reads
  ≤ (2H + 1)                       (locate, §6.1)
  + 6H                             (safe_predecessor, §6.1)
  + 1 + 2H + 2Δ_old(H−1)           (fact-range, §6.1)
  + 4H + 16                        (cursor, ≤ 2 fields/visit, k = 2)
  + 2·104H                         (splits, §6.2.3)
  + 44(d₁−1) + 44(d₂−1) ≤ 88H − 44 (extraction reads)
  + (46δ₁−36) + (46δ₂−36) ≤ 92H − 26                        (top joins)
  + 16                             (bulk build, 8·Δ_new recompute reads)
  + 1                              (replace_range precondition records())
  = 402H + 2Δ_old(H−1) − 35
  = 402H + 4(H−1) − 35             (Δ_old = 2)
  = 406H − 39

f3 aggregate writes
  ≤ 2·44H                          (splits, §6.2.3)
  + 20(d₁−1) + 20(d₂−1) ≤ 40H − 20 (extraction writes)
  + (20δ₁−16) + (20δ₂−16) ≤ 40H − 12                       (top joins)
  + 8                              (bulk build, 4·Δ_new)
  = 168H − 24
  (locate / safe_predecessor / fact-range / cursor / retirement write
   nothing; retirement performs no recomputes)
```

Constant accounting (no unexplained slack): f3 reads constants
`+1 (locate) +1 (fact-range) +16 (cursor) −44 (extractions) −26 (joins)
+16 (bulk) +1 (replace_range) = −35`; f2 links constants
`−5 −3 +4 +1 = −3` relative to the 72H coefficient. The first candidate's
`18H+4 / 74H−5 / 492H+142 / 208H+56` are superseded by this pass (they
carried the inconsistent `2δ+2` / `8δ−7` / `46δ+54` / `20δ+24` operator
forms and a `+2` root charge that contradicts the frozen §20
"final root installation = 1 link write").

### 6.4 Regenerated per-cell thresholds (#60 §9.5 rows that change)

| counter | corrected formula (review-fix pass) | 128 KiB | 1 MiB | 16 MiB | (first candidate) | (pre-correction) |
|---|---|---:|---:|---:|---|---|
| fact_range_node_visits | ≤ H + Δ_old(H−1) = 3H − 2 | 40 | 52 | 70 | 40/52/70 | 14/18/24 |
| join_node_visits (×2) | ≤ (4H−3) + (4(H+1)−3) = 8H − 2 | 110 | 142 | 190 | 110/142/190 | 60/76/100 |
| f2 visits | ≤ 36H − 4 | 500 | 644 | 860 | 500/644/860 | 450/578/770 |
| avl_rotations | ≤ 18H − 4 | 248 | 320 | 428 | 256/328/436 | 225/289/385 |
| sequence_link_writes | ≤ 72H − 3 | 1,005 | 1,293 | 1,725 | 1,031/1,327/1,771 | 977/1,249/1,657 |
| aggregate_reads | ≤ 406H − 39 | 5,645 | 7,269 | 9,705 | 7,030/8,998/11,950 | 4,007/5,123/6,797 |
| aggregate_writes | ≤ 168H − 24 | 2,328 | 3,000 | 4,008 | 2,968/3,800/5,048 | 1,832/2,344/3,112 |

All other §9.5 rows are unchanged and were re-verified against the
baseline code (locate/safe_predecessor/f1/cursor visits, split visits,
pivot extraction visits, bulk_build, retire/payload/frames/depth,
certificate reads/writes, old_fact_owner_visits, reftable, exact Known
values).

### 6.5 Operator consistency table and edge sanity checks

Consistency table (the single authority every composition in §6.3 and the
durable §16 substitutes; "n/a" = the operator performs none):

| Operator | Domain | Visits | Rotations | Link writes | Recomputes | Agg reads | Agg writes |
|---|---|---|---|---|---|---|---|
| locate | all | ≤ H | n/a | 0 | 0 | ≤ 2H + 1 | 0 |
| safe_predecessor | all | ≤ 3H − 2 | n/a | 0 | 0 | ≤ 6H | 0 |
| fact-range | all | ≤ H + Δ_old(H−1) | n/a | 0 | 0 | ≤ 1 + 2H + 2Δ_old(H−1) | 0 |
| remove_max | d ≥ 1 | ≤ 4d − 3 | ≤ 2(d−1) | ≤ 7d − 6 | ≤ 5(d−1) | ≤ 44(d−1) | ≤ 20(d−1) |
| join_with_pivot | δ ≤ 1 | ≤ 1 | 0 | ≤ 2 | 1 | ≤ 10 | 4 |
| join_with_pivot | δ ≥ 2 | ≤ 4δ − 3 | ≤ 2δ − 2 | ≤ 7δ − 5 | ≤ 5δ − 4 | ≤ 46δ − 36 | ≤ 20δ − 16 |
| split | H | ≤ 10H | ≤ 5H | ≤ 22H (18H proved) | ≤ 11H | ≤ 104H | ≤ 44H |

```text
f1 = 4H − 2
f2 = visits 36H − 4 / rotations 18H − 4 / links 72H − 3
f3 = reads 406H − 39 / writes 168H − 24
f4 = 2H + 4k + Q
f5 = unchanged (retire ≤ Δ_old; payload = P_removed;
     frames ≤ Δ_old + P_removed; depth ≤ max(H_detached, D_payload))
```

Edge sanity checks (mechanical substitutions; no legal count may produce a
negative bound or a bound below the direct-case work):

```text
δ = 0 : compatible row (1, 0, 2, 1, 10, 4) — the closed forms are NOT
        applied (4·0−3 < 1); this is exactly the defect the review found
        in the first candidate's universal 8δ − 7.
δ = 1 : closed forms give (1, 0, 2, 1, 10, 4) — identical to the
        compatible row (continuity; §6.2.2).
δ = 2 : (5, 2, 9, 6, 56, 24) with t = 1 — all ≥ direct-case events
        (1 descent + 1 pivot + 1 unwind + ≤2 participants; 3 attach
        links + ≤6 rotation links; 2 + ≤4 recomputes).
d = 1 : remove_max (1, 0, 1, 0, 0, 0) — the lone detach.
d = 2 : remove_max (5, 2, 8, 5, 44, 20) — 1 detach + 1 relink + ≤6
        rotation links; 1 + ≤4 recomputes.
H = 1 : split actuals (spine 1 visit, terminals only, 0 joins, ≤2 reads)
        ⊆ (10, 5, 22, 11, 104, 44); f2/f3 forms evaluate positive
        (32 / 14 / 69 / 367 / 144) and dominate every term monotonic in H.
H = 2 : same domination (every §6.3 substitution is monotone in H and in
        its own δ/d operand bounds).
```

## 7. Certificate accounting (8C + P2-02 + P3-01) — audited, bounds unchanged

`certificate_reads ≤ 5` / `certificate_writes ≤ 2` remain conservative
and correct for the frozen witness: the baseline's witness path charges
1 (safe_predecessor final selection — the deepest examined
right-descent ancestor carries the frozen nearest certificate) + 2
(candidate predicate evaluations) = 3 ≤ 5 reads, and 2 ≤ 2 writes.
`CERTIFICATE_ACCOUNTING_CORRECTION_REQUIRED = NO`.

`CERTIFICATE_UNIT_CLARIFIED = YES` (P2-02). One self-contained normative
rule (mirrored into durable spec §15.5; it supersedes the previously
contradictory §15.6.2-style wording):

```text
certificate_read
  = inspection of persistent certificate presence/content when that
    inspection participates in restart/candidate eligibility, selection,
    support, or convergence-predicate semantics.

EXCEPTION: the coarse certified/non-certified filtering performed while
the candidate cursor advances through crossed Owners is cursor traversal
mechanics — represented by the frozen k / cursor ledger
(cursor_node_visits); it is not separately charged as certificate_read.
```

The #60 §9.3.1 itemization lists two inspections that have no separate
sites in the audited implementation: the support-touch validation is
fused into the candidate predicate (clause 3, candidate.rs:196–209), and
the "replacement upper-boundary inspection" IS the accepted candidate's
predicate inspection. The ≤ 5 bound still contains the actual count; the
itemization text is clarified, not the bound.

### 7.1 Selected-restart support safety — positional proof (P3-01)

The first candidate flagged as an open mechanism-review question that the
baseline never separately validates the selected restart certificate's
support against the edit (#59 §4.2 is enforced at candidates). The frozen
geometry closes that question without any dynamic check; the facts are
audited against PR #70 @ 653d5a4 (update.rs `locate_damage` /
`select_restart`, sequence.rs `safe_predecessor`) and the frozen spec:

1. **RIGHT-affinity damage locate** (spec §3.3/§10 step 2): damage_base
   `t` = base of the Owner whose coverage contains `edit_start` (the
   RIGHT Owner at a boundary; `L_old` at EOF) ⇒ `t ≤ edit_start`.
2. **Strictly-before predecessor semantics** (spec §7.1): the
   safe_predecessor boundary cut is strictly below its exclusive bound,
   and `select_restart` queries it with `before = t` ⇒ restart cut
   `c_r ≤ t − 1 < t` (BOF restart: no certificate, nothing to prove).
3. **Support geometry** (spec §6 + §3.2): a reusable interior root
   certificate's support = `{preceding_lf}` ∪ the complete blank
   physical line including its LF. Blank/interstitial bytes belong LEFT
   (§3.2), so the blank line lies inside the certified Owner's coverage
   and every support byte ≤ blank_LF < base + coverage_len = `c_r`;
   `preceding_lf < blank start` only extends the support LEFTWARD.

Composition:

```text
every support byte ≤ c_r − 1 ≤ t − 2 < t ≤ edit_start = start
```

- non-empty edit `[start, end)`: `start` exceeds every support byte ⇒
  no intersection;
- zero-length insertion (`start == end`, the frozen witness): touched
  iff `start` lies in support; `start ≥ t > t − 2 ≥ max(support)` ⇒
  untouched.

Edge cases verified: zero-length insertion (above); preceding_lf support
(fact 3: leftward extension cannot reach an edit starting strictly
right); RIGHT affinity (fact 1: a boundary position yields `t =
edit_start`, and the chain stays strict through `c_r ≤ t − 1`);
strictly-before predecessor semantics (fact 2); EOF locate (`t = L_old ≥
edit_start`, chain unchanged); BOF restart (no certificate selected).

```text
SELECTED_RESTART_SUPPORT_SAFETY = STRUCTURALLY_PROVEN_BY_POSITION
NO_EXTRA_DYNAMIC_SUPPORT_TOUCH_CHECK_REQUIRED
```

#60 §9.3.1's historical support-touch-validation certificate-read
allowance therefore remains a conservative upper bound (an inspection
the geometry proves unnecessary for the selected restart); candidates
keep their own predicate-time support-touch clause (#59 §4.2), which is
unchanged mechanism behavior.

## 8. Retirement accounting (8D) — audited, unchanged

`retirement.rs` conforms exactly to #59 §11.1 / #60 f5:
`retire_node_visits` = Δ_old, `payload_nodes_retired` = P_removed,
`retirement_frames_entered` = Δ_old + P_removed, and
`max_retirement_depth` is a max-merge gauge over two separate recursion
spaces (≤ max(H_detached, D_payload)) while the combined call-stack
resource bound remains O(H_detached + D_payload).
`RETIREMENT_ACCOUNTING_CORRECTION_REQUIRED = NO`.

## 9. Resource-realization debt (recorded separately, NOT mixed in)

#59 §9.1 prescribes pre-reserved explicit-stack workspaces for
split / remove_max / join_with_pivot (and §11 retirement recursion is
frozen as recursion Option B). The baseline realizes the AVL operators
with by-value recursion:

```text
mechanisms/horse-a/src/sequence.rs:442  remove_max (recursion)
mechanisms/horse-a/src/sequence.rs:532  join_right (recursion)
mechanisms/horse-a/src/sequence.rs:568  join_left (recursion)
mechanisms/horse-a/src/sequence.rs:639  split (recursion)
vs #59 §9.1 resource table rows: split / pivot extraction /
join_with_pivot = "explicit stack ≤ H_max+1, pre-reserved"
```

`POST_FRONTIER_WORKSPACE_REALIZATION_DEBT = YES`. The corrections in
§6.2 are derived to contain this realization's legal charges (the
unwind relinks are genuine persistent-slot installs under §7.1);
realizing the §9.1 explicit-stack form later would allow re-tightening
the affected bounds — via a NEW correction record, never silently.

This debt is NOT optional follow-up (review P2-01). It is a mandatory
obligation:

```text
MUST RESOLVE BEFORE:
    #62 implementation-conformance closure
    and
    structural collection authorization

Resolution must be one of:

A. change PR #70 realization to the frozen #59 §9.1 explicit-stack
   design (split / remove_max / join_with_pivot pre-reserved stacks ≤
   H_max + 1);

OR

B. create a separate independently reviewed authority correction
   proving recursion is an admissible equivalent resource realization
   of the frozen #59 §9.1 workspace contract.
```

The accounting correction does not choose A or B and does not modify
PR #70; it only records the obligation.

## 10. IMPLEMENTATION FOLLOW-UP (separate task, after this correction passes independent review)

PR #70 must later (no production Rust is changed by this record):

1. keep the fact-range successor charges on `StructuralOp::FactRange`
   (they become conformant under the corrected §20 rule) and update the
   `StructuralOp::FactRange` doc comment to cite the corrected rule
   instead of redefining it;
2. narrow `child_meta` over-reads in the locate / safe_predecessor hot
   paths to the mechanism-required fields (bytes/records per descent
   node; has_safe only at steering points) so implementation charges
   fit the corrected read authority;
3. drop the per-spine-node `total` read in `split` beyond the terminal
   case (≤ H − 1 extra reads);
4. re-pin the I5 conformance/counter fixtures
   (`i5_tests::counters`, `i5_tests::conformance` T13) to the corrected
   authority values — expected values are hand-derived, never read back
   from the implementation;
5. resolve the #59 §9.1 explicit-stack realization debt under one of
   the two mandatory resolution paths recorded in §9 (A: realize the
   frozen explicit stacks, or B: an independently reviewed admissibility
   correction). This is a precondition for #62 implementation-conformance
   closure and for structural collection authorization, not an optional
   tightening opportunity; realizing A would remove the recursion-only
   link charges and justify a tightening record.
6. remove the link-write double charges found by the review-fix audit
   of the baseline instrumentation (653d5a4 sequence.rs):
   (a) `join_right`/`join_left` non-terminal levels charge the child
       take AND the common-tail unwind relink for the same slot
       reassignment (2/level; actual charges reach 8t+1 vs the §6.2.2
       authority 7t+2), while the terminal branch and `remove_max`
       charge the identical pair once;
   (b) `replace_range` charges the root-slot take (1) in addition to
       the final root installation (1) that #59 §20 freezes as the
       single root-slot write.
   Both must be narrowed so implementation charges fit the §6.2/§16
   link authority before structural collection; the authority is not
   broadened to legalize them (same classification as the `child_meta`
   over-reads, item 2).

## 11. Mechanism / schema / #60 impact

```text
MECHANISM IMPACT   = NONE (no operator algorithm, restart/convergence/
                      coverage/facts/retirement semantics changed; only
                      attribution scope and bounds)
SCHEMA IMPACT      = NONE (HORSE-A-STRUCTURAL-COUNTERS-v1 suffices; Option A)
#60 IMPACT         = §9.2 f2/f3 and seven §9.5 rows regenerated (§6.4);
                      §9.3.1 wording clarified (§7); all other rows
                      re-verified unchanged
#59 IMPACT         = §20 fact-range row broadened; §21.1 read-formula and
                      remove_max/join_with_pivot formula block corrected;
                      §21.3 compositions regenerated (superseded formulas
                      retained above); §9.1 workspace obligation recorded
                      as mandatory (§9)
REVIEW-FIX IMPACT  = durable spec §12.1.1/§12.3.1/§15.2/§15.5/§16 now
                      carry exactly one active formula set derived from
                      the §6.2 piecewise lemma (P1-02 closed); §6.2/§6.3
                      internal consistency restored (P1-01); certificate
                      unit self-contained (P2-02); selected-restart
                      support safety proven by position (P3-01, §7.1)
```

## 12. Falsification checks

- **F1 (hidden O(M) scan)** — PASS: the corrected bound depends only on
  H and Δ_old; the walk terminates after exactly Δ_old iterations.
- **F2 (retained suffix counted as Δ_old work)** — PASS: the loop is
  `for _ in ranks` (exactly the replacement range); suffix enumeration
  remains a forbidden sentinel with its own counter.
- **F3 (double charge fact_range/cursor)** — PASS: each cursor instance
  charges exclusively its own operator; the corrected rule adds
  successor-walk visits to `FactRange` only, never to `Cursor`.
- **F4 (old_fact_owner_visits redundant/ambiguous)** — PASS: it counts
  Owner payload inspections (one per yielded replacement Owner), a
  different unit from AVL-node navigation; on the witness it is exactly
  Δ_old = 2 ≤ Δ_old + Δ_new = 4.
- **F5 (threshold fitted from 19/25/33)** — PASS: the bound
  H + Δ_old(H−1) is derived in §5.2 before the §5.3 consistency check,
  and the observed values are independently reproduced from the frozen
  bulk-build geometry.
- **F6 (mechanism/weakness changes)** — PASS: restart, convergence,
  coverage, facts equality, semantic branch, AVL algorithm, retirement,
  W-A1/W-A2/W-A3 are untouched.
- **F7 (single active formula set)** — PASS: exactly one active
  join_with_pivot lemma exists (§6.2.2 piecewise); split/f2/f3 and the
  #60 rows substitute it only; superseded formulas appear solely as
  labeled history (§2, §6.2.2 tail, durable §12.3.1 SUPERSEDED block).
- **F8 (edge-value validity)** — PASS: §6.5 checks δ ∈ {0,1,2},
  d ∈ {1,2}, H ∈ {1,2}; no legal count produces a negative bound or a
  bound below direct-case work; the universal-at-δ=0 defect of the
  first candidate's `8δ − 7` is explicitly excluded by the piecewise
  lemma.

## 13. Provenance

```text
audited implementation   PR #70 @ 653d5a4f10605fe17ded6f6d703656f9950c97c4
authority base           master @ 9dc365ed878ba9f0fda877dc6daf6d300c292d73
first candidate          PR #72 @ 39de28e075477f99d241f170f771ac344b1b5993
                         (independently reviewed: P0=0 / P1=2 / P2=2 / P3=1;
                         closed by the review-fix pass in §0/§6.2/§7/§9)
review-fix pass          this commit (operator/composition internal
                         consistency restored; durable spec single active
                         formula set; no code, no collection)
frozen authorities       #55 / #59 (§7, §9, §10, §20, §21) / #60 (§9, §13,
                         §14) / docs/research/horse-a-v1-algorithm.md
diagnostic (non-decision-bearing) fact_range_node_visits 19/25/33 vs ≤H
superseded formulas      retained verbatim in §2 and in the frozen issues'
                         history; correction comments appended to #59/#60
```
