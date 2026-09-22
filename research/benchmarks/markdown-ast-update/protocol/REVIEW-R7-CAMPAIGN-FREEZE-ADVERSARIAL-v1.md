# Adversarial review — R7 campaign freeze (fresh context, v1)

Task: MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1 (#31).
Reviewer: independent context that did not implement the change; reviewed
the branch `research/22-primary-performance-campaign-freeze-1` against
base `911788a2ad5fcb5b939d8bc3576d3aea77f7df62`.

## Method

The reviewer read the frozen authority documents and the implementation
(`campaign/src/**`, `runner/src/case_order.rs`, `scripts/verify-r7.sh`,
the manifests/schema/receipt), then independently re-derived, in a
separate tooling (Python), from the frozen artifacts only:

- the campaign seed (`0x5346ab392350f03d`) and the spec id
  (`ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2`);
- all 12 receipt hashes and the CaseKeyV1 canonical encoding for all
  384 primary CaseIds (22 + 362), including cross-surface disjointness;
- all 1,152 schedule rows (case order per surface × session, payload/
  source/trace identity, horse rotation and position balance);
- targeted adversarial probes (e.g. the smoke output-path guard, a
  payload-drift schedule mutation, attribution counter repeatability).

## Verdicts (Q1-Q30)

All thirty required questions were answered PASS with file/line
evidence; the reviewer additionally probed defect classes (measured
values influencing identity, smoke provenance/placement, schedule
verification gaps, TOML/serde behavior, runner API exposure,
preflight scope, FULL_READ CaseId derivation, nondeterminism sources).
Full question list: task §60 / R7 §21-§24 context.

## Findings and remediation

| # | severity | finding | remediation |
|---|---|---|---|
| 1 | MAJOR | The smoke output guard failed OPEN for a non-existent relative `--out` path: `canonicalize()` fails for fresh paths, so a relative `results/raw/...` target passed the CLI guard and the smoke wrote there (rows tagged NON_RESEARCH, but inside the primary raw tree). | Fixed in `campaign/src/smoke.rs`: lexical normalization (no filesystem dependency) plus component-wise `results/raw` detection, enforced BOTH in the library (`run_smoke`) and in the CLI; unit test covers relative, absolute, `..`-escaping, and near-miss paths. Probes now fail closed (exit 1). |
| 2 | MINOR | Machine preflight compared only the selected CPU; physical cores, SMT, NUMA nodes/cpu map/selected node, cargo, LLVM, and allocator policy were recorded but never matched. | `match_current_host` now compares every frozen stable field. |
| 3 | MINOR | The frozen "3 fresh worker processes per surface" rule was documented but not auditable. | Run receipts now record producer PID, kernel start ticks, and the executable SHA-256, so distinct processes are verifiable after the fact. |
| 4 | MINOR | `stats::summarize` invalidated on failed measured samples but ignored failed warmups (unreachable through the executor, but a defense-in-depth gap). | Warmup failures now invalidate too; new unit test. |
| 5 | MINOR | The schedule verification blocker message printed only the case id for both sides even when payload/source/trace drifted (check was correct, diagnostic misleading). | Message now prints every differing identity field. |
| 6 | MINOR | The receipt bound `Cargo.lock` but not the files that DEFINE the frozen release profile, so a post-freeze `[profile.release]` edit changed no bound hash. | `Cargo.toml` and `rust-toolchain.toml` added to the bound set (14 artifacts). `CampaignSpecId` verified unchanged. |
| 7 | NIT | `ExecutionIdentity.non_research` was written but never read. | Provenance coherence guard: NON_RESEARCH tag and flag must agree or the session fails closed. |
| 8 | NIT | `expected_frozen_schedule_rows` was dead code. | Now used by the preflight cardinality guard. |
| 9 | NIT | `seed.base_authority_sha` was not cross-checked against the top-level field. | Cross-check added to `CampaignManifest::verify`. |
| 10 | NIT | `SurfacesMap` did not deny unknown fields. | `deny_unknown_fields` added. |
| 11 | NIT | A test comment overclaimed a deserialize-time schema-version check. | Comment corrected; explicit v2 assertions remain in the smoke validator. |
| 12 | NIT | Attribution rows were not checked for all-Unknown counters. | A completed attribution run whose every slot is Unknown now fails closed (task §45). |
| 13 | NIT | `verify-r7.sh` ran the campaign CLI in debug, so the enforced preflight could never pass (observed as a blocker). | The script now builds and uses the frozen release profile, and documents that it must run on the primary machine. |

No finding affected the validity of the primary measurement contract
itself; finding 1 was the only gate-blocking issue and is fixed with
tests.

## Outcome

`PASS` after remediation. The freeze verdict stands:

```text
PRIMARY_PERFORMANCE_CAMPAIGN_FREEZE_PASS
PRIMARY_TIMING = NOT_STARTED
PRIMARY_PERFORMANCE_CAMPAIGN_READY = CANDIDATE_FOR_HUMAN_REVIEW
```
