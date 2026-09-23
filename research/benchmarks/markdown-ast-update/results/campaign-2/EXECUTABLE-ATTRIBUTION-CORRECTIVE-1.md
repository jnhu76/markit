# EXECUTABLE-ATTRIBUTION-CORRECTIVE-1

```text
authority        3762b7a42e1c284a4c2c2e0ebac8496e70c63431
scope            results/campaign-2/receipts/**  (274 receipt files)
triggered by     review of the receipts against the raw rows
status           CORRECTED — no raw observation row was touched
```

## 1. What was wrong

`tools/closure.py` filled the receipt field `executable_sha256` by hashing
`target/release/mdbench-campaign2` **as it existed at closure time**:

```python
executable_sha = sha256_file(os.path.join(root, "target/release/mdbench-campaign2"))
```

That is a **current-state reading**, not a record of provenance. Because the
campaign binary was rebuilt once between the formal collection and the
lifecycle attribution lane, every one of the 274 receipts ended up
asserting the same value:

```text
executable_sha256 = 1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282   x274
```

while the `run_id` carried inside the raw rows proves that 8 of the 9
observation sub-campaigns were produced by a different binary. The receipts
therefore contradicted the evidence they were supposed to certify.

This is precisely the defect class the campaign's own rules exist to catch
(a field that reports the present instead of the provenance), so it is
corrected and recorded rather than quietly reissued.

Affected: **31 of 34** observation files were mislabelled. The 3 lifecycle
attribution files happened to be correct by coincidence (they were produced
by the binary that was also present at closure time).

## 2. How the correct value is obtained

Two independent methods, both required to agree.

### M1 — execution-time lane log (primary)

`logs/run-lane.sh`, `logs/run-profiling.sh` and `logs/run-memory.sh` hash
the binary and print `executable_sha256=…` **immediately before the lane
runs**. The lane log is a record made at execution time, not a
reconstruction:

```text
logs/60-attribution-construction.log        c1c7756a1d36316d…52f5822
logs/61-attribution-resident-update.log     c1c7756a1d36316d…52f5822
logs/62-attribution-controlled-{N,B,D,F,K}  c1c7756a1d36316d…52f5822
logs/65-lifecycle-attribution-session-{0,1,2}  1bd1c09e0bc271bb…378fbf282
logs/70-memory.log                          c1c7756a1d36316d…52f5822
logs/80-profiling.log                       c1c7756a1d36316d…52f5822
                                            (replay binary 2b4ac6b877c88d6d…157c34c6)
```

### M2 — RunId derivation (independent cross-check)

The frozen derivation in `campaign2/src/identity.rs`:

```text
RunId = SHA256(StudyId || CampaignSpecId || SubCampaignSpecId
               || runner_git_commit || machine_manifest_sha256
               || rustc || target || build_profile_id || cargo_lock_sha256
               || executable_sha256)
```

Recomputing it for each candidate `(build_commit, executable)` pair must
reproduce the `run_id` found in the raw rows. This is a cryptographic
check, not a lookup:

```text
observed RunId                                                      candidate that reproduces it
88377126393279123bb450af51f2e7f56a9294319a148f892dca4a8f2839d2ba    commit 4fd86e48, exe c1c7756a
b1f1b2509eb106200a114e1d1952a9746eebe9e64017ce96cdb00535a6dbadaa    commit 4fd86e48, exe c1c7756a
4c2a016ab180df21e974ee335b6ee64b13b5c6b66cd2a85c3fc13ff75e552265    commit 4fd86e48, exe c1c7756a
e53edd02f80d21bd1852b1b289d849dc49fba64375d5ca736640521b2f536ffa    commit 4fd86e48, exe c1c7756a
953f30932878d4e8858912b461afefa82c8233b93d9b0020d65fc3fc6cdc5e84    commit 4fd86e48, exe c1c7756a
d0d68391045c9f61af3693e3a4bdd748e8365e75f89fa56ca096f4d33c68b28c    commit 4fd86e48, exe c1c7756a
6d3572cc37eba8bdc1c35e9f245583982f81f4e4efbcb6cc31b0a3dffad23e21    commit 4fd86e48, exe c1c7756a
9411446f0f5cd8ce5fd116591e7557fd5b674bcd9d09d1de65054ae2ad1d7a46    commit 4fd86e48, exe c1c7756a
142720eda03d39da4400193df2810a92bc805e01e22fc86f4e5882fc19f58b7a    commit f6262716, exe 1bd1c09e
```

```text
M1_M2_AGREEMENT = PASS   (all 34 observation files)
```

## 3. Corrected attribution

```text
mdbench-campaign2 @ c1c7756a1d36316dff2fb278e9db7926ee20a39bceaa0e8248e2619af52f5822
    build commit 4fd86e489b897c987074550b800c08e613e5185d
    31 observation files: construction, resident_update, controlled N/B/D/F/K,
    lifecycle TIMING lanes, memory
mdbench-campaign2 @ 1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282
    build commit f6262716daddf8b5151de0b2b0e2879d3e838b2b
    3 observation files: lifecycle ATTRIBUTION lanes only
mdbench-replay    @ 2b4ac6b877c88d6dd2f1408071be52ca008181630613831886423511157c34c6
    44 raw profiling artifacts (region / setuponly / .data)
196 further artifacts (perf text reports, headers, the documents in this
    directory, .md files) are tooling/analysis artifacts and have no
    campaign-binary producer.
```

Per-file: `manifests/executable-derivation-v1.json`.

## 4. What each receipt now carries

```text
producer                       mdbench-campaign2 | mdbench-replay | tooling/analysis artifact
executable_sha256              the binary that PRODUCED the file
executable_sha256_method       M1_lane_log and M2_runid_derivation agree
                               M1_lane_header profiling_binary_sha256
                               not a measured artifact
executable_sha256_evidence     the log or header the value came from
superseded_executable_sha256   the old (wrong) value, preserved
superseded_reason              EXECUTABLE-ATTRIBUTION-CORRECTIVE-1 …
run_id / sub_campaign_tag      carried through unchanged
```

Nothing was deleted: the superseded value stays in every receipt, so a
reader can see both what was claimed before and what is correct now.

## 5. What did NOT change

```text
raw observation rows       untouched (0 bytes changed)
raw file SHA256            unchanged (verified: 274/274 still match the inventory)
inventory                  unchanged
RunIds                     unchanged
StudyId / CampaignSpecId   unchanged
production horses          untouched
```

## 6. Residual disclosure

The rebuild of the campaign binary between the formal collection and the
lifecycle attribution lane was a real procedural event, not a defect: the
change was purely additive campaign tooling
(`git diff` = 160 insertions, 0 deletions over
`campaign2/src/bin/mdbench-campaign2.rs`), no mechanism, runner, oracle or
common file differs, and the two lanes are never pooled under one RunId.
It is recorded here and in `CAMPAIGN-2-CLOSURE.md` §1 so a reader can see
it rather than having to infer it.

The lesson is recorded in `EVIDENCE-GAPS.md` §13: a receipt field must be
derived from the artifact's provenance, never from the environment present
when the receipt is written.
