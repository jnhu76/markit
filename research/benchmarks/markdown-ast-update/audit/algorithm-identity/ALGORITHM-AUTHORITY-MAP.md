# ALGORITHM-AUTHORITY-MAP

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.

This map answers: for each mechanism behavior, WHICH frozen document is
the authority, and whether the code's authority chain is clean.

## Authority chain

```text
R5-HORSE-CORRECTNESS-PARITY.md      (the freeze: mechanism decisions,
    §2 ContextKey, §3 counter applicability, §4 eager completion,      frozen
    §6 H1, §7 H2, §8 H3, §9 H4, §11 corrections, §12 ref environment)  pre-coding
        |
        +-- R5-CORRECTIVE-1  (ownership pass-through, inline inspection,
        |                     retained representation, completed-state
        |                     QUERY, counting rule §11.6)
        +-- R5-CORRECTIVE-2  (source-inspection closure: every mechanism
        |                     source read is reported)
        +-- R5-CORRECTIVE-3  (reference-environment clause: rebuilt-table
                              comparison, not byte comparison)
        |
        v
    mechanisms/*/src/lib.rs   (the audited implementations)
        |
        v
    Campaign-1 measurements   (the artifact this audit re-baselines)
```

`R5-HORSES-STAGE-RECORD.md` records the stage evidence (parity table,
gate history, identity witnesses W1–W3); it is a RECORD, not an
authority for mechanism semantics.

## Per-mechanism authority -> implementation binding

| Mechanism | Frozen section | Implementation (audit SHA) |
|---|---|---|
| H0 FULL_REBUILD | R5 §5 (control) | `mechanisms/full-rebuild/src/lib.rs` `FullRebuildMechanism` (L126), `update` (L249) |
| H1 BLOCK_LOCAL_REPARSE | R5 §6 | `mechanisms/block-local/src/lib.rs` `BlockLocalMechanism` (L174), `prepare_update` (L261), `update` (L278), `total_fallback` (L223), `guards_fire` (L585) |
| H2 FRAGMENT_REUSE | R5 §7 (+CORRECTIVE-3) | `mechanisms/fragment-reuse/src/lib.rs` `FragmentReuseMechanism` (L238), `MIN_GAP` (L59), `consult` (L642), `rematerialize` (L1079), env clause (L444) |
| H3 OLD_TREE_SUBTREE_REUSE | R5 §8 (+CORRECTIVE-2) | `mechanisms/old-tree-subtree-reuse/src/lib.rs` `OldTreeSubtreeReuseMechanism` (L238), `patch_tree` (L496), `Cursor::consult` (L782) |
| H4 RESTART_CONVERGENCE | R5 §9 (+CORRECTIVE-1 §6) | `mechanisms/restart-convergence/src/lib.rs` `RestartConvergenceMechanism` (L216), `prepare_update` (L320), `update` (L432), `Cursor::consult` (L739), `restart_at_zero` (L268) |

All paths relative to `research/benchmarks/markdown-ast-update/`.
`CODE_INSPECTION_SUPPORT`.

## PR #45 status

PR #45 is under independent review and was NOT used as algorithm
authority here. Nothing in this pack cites it. Where this audit's
reading of the freeze differs from a PR #45 interpretation, the freeze
document at the audit SHA is the tiebreaker.

## Authority cleanliness checks

- The Mechanism trait boundary
  (`common/src/mechanism.rs` L113–L167: `full_parse` / `prepare_update`
  / `update` / `complete`; `MechanismContext` hands out a `WorkSink`
  only, L65–L66) is the ONLY way a horse touches the scanner.
  `CODE_INSPECTION_SUPPORT`.
- Horses never call H0 in lib code: enforced statically by
  `diagnostics/tests/anti_cheat.rs` L140–L156 (full-rebuild allowed
  only under `[dev-dependencies]`) and by the workspace dependency
  graph. `MECHANICALLY_PROVEN_BY_INVARIANT`.
- Correctness oracles are test-side only: every `audit_identity.rs`
  imports `markit_mdbench_full_rebuild::parse_document` as a dev
  dependency; no mechanism crate does.
  `MECHANICALLY_PROVEN_BY_INVARIANT`.
