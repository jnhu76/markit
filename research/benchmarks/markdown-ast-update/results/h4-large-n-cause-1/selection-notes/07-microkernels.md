# Selection note — microkernels

**Decision: none built.** Issue #50 §17 allows a microkernel only when the
integrated evidence leaves a *specific causal ambiguity*, and lists
`K_damage_restart`, `K_definition_traversal`, `K_suffix_slot_clone_rebase`,
`K_checkpoint_rebuild`, `K_pairs_slots_vector_assembly`, `K_drop_retirement`.

Each of the six was already answered on the integrated path:

| Candidate | Why no microkernel was needed |
|---|---|
| `K_damage_restart` | `P1` is a mutually exclusive phase with a measured share, and its two components are separated by `restart_predicate_evaluations` (= M/2) and `damage_records_visited` (= M), both exactly linear in M |
| `K_definition_traversal` | answered by the `Adefs` single-factor ablation, whose effect (8.1-9.0 % at 16 MiB) agrees with `P4`'s phase share (8.5-8.7 %) |
| `K_suffix_slot_clone_rebase` | answered by `P5` and by the allocator lane, which shows the suffix re-attachment allocates **nothing** |
| `K_checkpoint_rebuild` | `P6` covers checkpoint construction, and `checkpoint_records_created == M` with `checkpoint_key_clones == M` |
| `K_pairs_slots_vector_assembly` | `P6` plus the `Acapacity` ablation, plus the allocator lane's exact per-vector byte counts |
| `K_drop_retirement` | answered by the `Adrop` single-factor ablation with a timed drain and an empty-drain control |

An isolated kernel time cannot be added back to reconstruct U (Issue #50
§17), so a kernel that merely restates a phase would add cost without adding
evidence. The remaining ambiguity — how the O(M) operations' cost divides
between pure instruction work and stall time — is a hardware question, and it
was answered by the PMU and `perf record` lanes instead.

`MICROKERNELS = NOT_NEEDED`.
