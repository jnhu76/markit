# RQ8 optimization-sensitivity freeze (#33)

This directory freezes the #33 RQ8 replication authority
(`MARKIT-33-RQ8-OPTIMIZATION-SENSITIVITY-v1`), consumed by
`mdbench-campaign` subcommands `sensitivity-*` and
`run-sensitivity-session`. Frozen 2026-09-29, BEFORE any sensitivity
measurement.

## Artifacts

| Artifact | Role |
|---|---|
| `rq8-sensitivity-manifest-v1.toml` | frozen execution manifest: pinned primary authorities, K1–K6 case/horse matrix, session/seed policy, cardinalities |
| `rq8-sensitivity-schedule-v1.jsonl` | frozen projected schedule (1,152 rows = 3 sessions × (362 edit + 22 clean) case rows); SHA256 `b5e7ad76e85549e219ae48ae896c4d6cc0c06ad34c298ed83e405fb2e326d130`; sensitivity `CampaignSpecId` `7686e6e149cd9ea11f4582f0bb0ad737890ade42a9b9210b2abcbca8d7394d2f` |
| `primary-receipt-supersession-v1.json` | the reviewed record that lets the byte-frozen primary campaign receipt accept exactly the two authorized workspace evolutions (`Cargo.toml`, `manifest/environment.toml`) caused by adding the second frozen profile; every other drift fails closed |

## Governance facts

- The replication runs the frozen K1–K6 shortlist ONLY (R8 FINAL
  SYNTHESIS v1, `OPTIMIZATION-SENSITIVITY-SHORTLIST-v1.md`, sha
  `03cfeae5…`): 362 edit cases × {H0, H4, HorseA, H1} + the 55-case
  K2/K4 union × {H2, H3} + 22 clean-state cases × 6 — 1,690 cells per
  session, 202,800 timing rows total (73% of the primary's row count;
  the full 2,304-cell matrix is NOT rerun).
- Timing lane only. Attribution, memory, and PMU lanes are NOT rerun;
  their sealed evidence is compiler-profile-independent for the
  conclusions under test and stays authoritative.
- The profile is the single-factor `release-sensitivity-lto-off-v1`
  (`lto = false`; everything else inherited from `release`). Full
  binding: `manifest/rq8-sensitivity-profile-v1.toml`; policy:
  `protocol/implementation-parity.md` "Second frozen profile".
- The schedule is a pure projection of the frozen primary schedule
  (`primary-schedule-projection-v1`): same sessions, same case order
  ordinals, same per-case horse order restricted to the matrix. No new
  randomness; the seed is inherited from the primary campaign
  (`0xd7b11f1df0cc7cb5`), so per-case seeds are identical.
- Raw sensitivity evidence lands under `results/sensitivity/raw/`
  (isolated from the primary `results/raw/` tree; append/create-only).
- Primary evidence, horses, workload, and measurement boundary are
  UNCHANGED. Issue #76 stays CLOSED.
