# Selection note — `Acapacity`

**Trigger observation (from the superseded attempt-3 run).** The W_COUNTERS
lane shows the `pairs` vector is the only assembly vector that is not
pre-sized: `pairs_vec_reallocations` is 9 at 128 KiB and 16 at 16 MiB, i.e.
`log2(M) + 1`, and `pairs_vec_pushes == M`. The allocator lane attributes
~2 x the final vector size in requested bytes to that growth chain. The other
two vectors (`slots`, `checkpoints`) are created with
`Vec::with_capacity(pairs.len())` and show no reallocation at all.

**Two explanations to separate.**

1. The reallocation traffic is a *material* part of the assembly cost.
2. The element construction and copy work dominates and the growth policy is
   *secondary*.

**Variant chosen.** `Acapacity` (Issue #50 §11): give `pairs` an exact
capacity computed from lengths the algorithm already has after the forward
parse (`restart_slot + fresh blocks + suffix length`), leaving representation
semantics untouched. Nothing is read from the generator; the capacity is
derived from `rp.blocks`, `take` and `old_state.blocks.len()`.

**What would support each.** A large, consistent effect supports explanation
1 and makes capacity policy a removable cost. A small effect supports
explanation 2 and downgrades the growth policy to secondary — while still
reporting the reallocation *events*, which are facts regardless of their
timing share. Note that the issue warns not to re-"optimize" a vector that is
already exact-capacity, which is why `slots`/`checkpoints` are untouched.
