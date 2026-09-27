# HORSE-A-ACCOUNTING-CORRECTION-1 — mechanically correct structural attribution bounds before collection

```text
STATUS                     = DERIVED, AWAITING INDEPENDENT ACCOUNTING REVIEW
CORRECTION_TYPE            = PRE-COLLECTION_MECHANICAL_ACCOUNTING_CORRECTION
IMPLEMENTATION_BASELINE    = PR #70 @ 653d5a4f10605fe17ded6f6d703656f9950c97c4
IMPLEMENTATION_BASE_BASE   = master @ 9dc365ed878ba9f0fda877dc6daf6d300c292d73

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

### 6.2 remove_max link writes (8B)

Exact event ledger from the baseline code (sequence.rs:442–474):

| Step | Persistent Link slot mutated? | Count | In old ≤ 6d − 5? |
|---|---|---:|---|
| right-spine descent (d nodes) | no (reads/visits only) | 0 | n/a |
| pivot detach (parent right / root slot takes p.left) | yes | 1 | yes |
| unwind relink `node.right = rest` per ancestor | yes — installing a Box into a persistent child slot is a link write under §7.1, and the installed subtree differs (max removed, possibly re-rooted) | d − 1 | **NO — omitted** |
| rotation slot writes (single = 3, double = 6; ≤ 2 units per unwind level) | yes (flat convention) | ≤ 6(d − 1) | yes |

```text
remove_max link writes  ≤ 1 + (d − 1) + 6(d − 1) = 7d − 6     (was 6d − 5)
```

The same mechanical cause (by-value recursion take/restore) applies to
`join_with_pivot` (pass write + unwind relink per spine level) and its
§21.1 formula additionally omits rotation-participant visits (which its
own §20 ledger row requires) and double-rotation units:

```text
join_with_pivot visits   ≤ 4δ + 3      (was 2δ + 1;  = descent δ−1 + pivot 1
                                         + unwind δ−1 + participants ≤ 2(δ−1),
                                         frozen-slack form)
join_with_pivot rotations ≤ 2δ + 2     (was δ + 1;   a double = 2 units and
                                         ≤ 1 rebalance action per level)
join_with_pivot links    ≤ 8δ − 7      (was 6δ + 9;  attach 3 + ≤ 2 per
                                         non-terminal spine level (δ−2)
                                         + ≤ 6(δ−1) rotation writes)
join_with_pivot recomputes ≤ 5δ + 6    (was 3δ + 4;  unwind 1 + ≤ 4 rotation
                                         recomputes per level — a double
                                         rotation recomputes 4 nodes)
join_with_pivot reads    ≤ 46δ + 54    (was 25δ + 35)
join_with_pivot writes   ≤ 20δ + 24    (was 12δ + 16)
remove_max recomputes    ≤ 5(d − 1)    (was 3(d − 1))
remove_max reads         ≤ 44(d − 1)   (was 24(d − 1))
remove_max writes        ≤ 20(d − 1)   (was 12(d − 1))
split reads              ≤ 147H + 1    (was 86H;  spine H+1 + Σ(46δi + 54),
                                         Σδi ≤ 2H, #joins ≤ H)
split writes             ≤ 64H         (was 40H;  Σ(20δi + 24))
```

Unchanged after re-derivation (still containing the corrected
compositions and the implementation's legal charges):

```text
locate / safe_predecessor / cursor / fact-range VISITS as frozen except
fact-range (§5); f1 ≤ 4H − 2; cursor ≤ 2H + 4k + Q
remove_max visits ≤ 4d − 3;  remove_max rotations ≤ 2(d − 1)
split visits ≤ 6H proved (10H retained: corrected internal composition
              H + Σ(4δi − 3) ≤ 9H − 3 ≤ 10H)
split rotations ≤ 3H proved (5H retained: ≤ 4H − 2 charged)
split link writes ≤ 22H (corrected ≤ 2H + Σ(8δi − 7) ≤ 18H − 7)
bulk_build / retirement / final root installation rows unchanged
```

### 6.3 Regenerated compositions (#59 §21.3 / #60 §9.2 style)

With Δ_old = Δ_new = 2, d₁ ≤ H, d₂ ≤ H + 1, δ₁ ≤ H, δ₂ ≤ H + 1,
Σδi ≤ 2H, #joins ≤ H per split:

```text
f2 visits      ≤ 20H (split ×2, retained) + (8H − 2) (extractions ×2)
                 + (8H − 2) (top-level join_with_pivot ×2)  = 36H − 4
f2 rotations   ≤ 10H (split ×2, retained) + (4H − 2) (extractions)
                 + (4H + 6) (top-level join_with_pivot ×2)  = 18H + 4
f2 link writes ≤ 44H (split ×2, retained) + (14H − 5) (extractions)
                 + (16H − 6) (top-level join_with_pivot ×2)
                 + 4 (bulk attach) + 2 (root take + install) = 74H − 5
f3 reads       ≤ (2H + 1) locate + 6H safe_pred + (6H − 3) fact-range
                 + (4H + 16) cursor + (294H + 2) split ×2
                 + (88H − 44) extraction recomputes + decision reads
                 + (92H + 154) join_with_pivot ×2 + 16 bulk
                                                             = 492H + 142
f3 writes      ≤ 128H split ×2 + (40H − 20) extractions
                 + (40H + 68) join_with_pivot ×2 + 8 bulk    = 208H + 56
```

### 6.4 Regenerated per-cell thresholds (#60 §9.5 rows that change)

| counter | corrected formula | 128 KiB | 1 MiB | 16 MiB | (old) |
|---|---|---:|---:|---:|---|
| fact_range_node_visits | ≤ H + Δ_old(H−1) = 3H − 2 | 40 | 52 | 70 | 14/18/24 |
| join_node_visits (×2) | ≤ (4H−3) + (4(H+1)−3) = 8H − 2 | 110 | 142 | 190 | 60/76/100 |
| f2 visits | ≤ 36H − 4 | 500 | 644 | 860 | 450/578/770 |
| avl_rotations | ≤ 18H + 4 | 256 | 328 | 436 | 225/289/385 |
| sequence_link_writes | ≤ 74H − 5 | 1,031 | 1,327 | 1,771 | 977/1,249/1,657 |
| aggregate_reads | ≤ 492H + 142 | 7,030 | 8,998 | 11,950 | 4,007/5,123/6,797 |
| aggregate_writes | ≤ 208H + 56 | 2,968 | 3,800 | 5,048 | 1,832/2,344/3,112 |

All other §9.5 rows are unchanged and were re-verified against the
baseline code (locate/safe_predecessor/f1/cursor visits, split visits,
pivot extraction visits, bulk_build, retire/payload/frames/depth,
certificate reads/writes, old_fact_owner_visits, reftable, exact Known
values).

## 7. Certificate accounting (8C) — audited, bounds unchanged

`certificate_reads ≤ 5` / `certificate_writes ≤ 2` remain conservative
and correct for the frozen witness: the baseline's witness path charges
1 (safe_predecessor final selection — the deepest examined
right-descent ancestor carries the frozen nearest certificate) + 2
(candidate predicate evaluations) = 3 ≤ 5 reads, and 2 ≤ 2 writes.
`CERTIFICATE_ACCOUNTING_CORRECTION_REQUIRED = NO`.

`CERTIFICATE_WORDING_CLARIFICATION_REQUIRED = YES`:

1. The #59 §7.1 unit "inspection … (or its presence) sufficient for one
   mechanism decision" must be read together with #59 §8: the candidate
   walk's **certification presence filtering** (crossing non-certified
   Owners while seeking the next candidate) is cursor mechanics charged
   as the frozen `k` term in `cursor_node_visits` — it is NOT a
   `certificate_read` (baseline candidate.rs:250–257 follows this).
2. The #60 §9.3.1 itemization lists two inspections that have no
   separate sites in the audited implementation: the support-touch
   validation is fused into the candidate predicate (clause 3,
   candidate.rs:196–209), and the "replacement upper-boundary
   inspection" IS the accepted candidate's predicate inspection. The
   ≤ 5 bound still contains the actual count; the itemization text is
   clarified, not the bound. (Flagged for the implementation review,
   not decided here: the baseline never separately validates the
   selected restart certificate's support against the edit — #59 §4.2
   is enforced at candidates. This is a mechanism-review question, not
   an accounting correction.)

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
This debt can be resolved separately before #62 closure.

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
5. optionally realize #59 §9.1 explicit stacks (see §9) which would
   remove the recursion-only link charges and justify a tightening
   record.

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
                      retained above)
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

## 13. Provenance

```text
audited implementation   PR #70 @ 653d5a4f10605fe17ded6f6d703656f9950c97c4
authority base           master @ 9dc365ed878ba9f0fda877dc6daf6d300c292d73
frozen authorities       #55 / #59 (§7, §9, §10, §20, §21) / #60 (§9, §13,
                         §14) / docs/research/horse-a-v1-algorithm.md
diagnostic (non-decision-bearing) fact_range_node_visits 19/25/33 vs ≤H
superseded formulas      retained verbatim in §2 and in the frozen issues'
                         history; correction comments appended to #59/#60
```
