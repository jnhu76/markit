# ACQUISITION REPORT 3 — CORRECTIVE 2 (MARKIT-REAL-WORKLOAD-ACQUISITION-1)

Corrective round addressing the second fresh-context human adversarial
review of PR #34 (review ID 5259622652, verdict
`CORRECTIVE-2 REQUIRED / KEEP DRAFT`). The review confirmed corrective 1
closed all four blocking findings; one substantive MAJOR remained, plus one
truthfulness item. This campaign remains **acquisition only**: no H0-H4,
no eligibility scanning, no static characterization, no workload selection,
no storage-architecture change. PR #34 stays Draft; nothing is merged.

```text
VERDICT: CORRECTIVE_PASS   (verify lock authority + PR body truthfulness)

VERIFY BOUND TO FROZEN LOCK:          PASS (identity + derived hashes vs lock)
VERIFY BYPASS NEGATIVE TESTS:         PASS (4 new tests; suite 50/50 OK)
ACQUIRE AUTHORITY == VERIFY AUTHORITY: PASS (same frozen lock governs both)
PR BODY TRUTHFULNESS:                 PASS (corrective-1 facts live, read back)
NO WORKLOAD/STORAGE CHANGES:          PASS (pins, bytes, frame untouched)
```

## MAJOR — verify bypassed frozen lock authority → FIXED

**The gap (as reviewed):** `acquire` / `materialize` run the fail-closed
`preflight_replay()` gate, but `verify` checked only structural closure,
SOURCE↔config correspondence, inventory closure, and local bytes. It did
NOT enforce `source_manifest_identity(current SOURCE) ==
source-lock.sources[*].source_manifest_hash`, and it compared derived
manifests only against a recomputation from current inputs — not against
the hashes recorded in the lock. On a fresh clone without materialized
bytes, a SOURCE.json whose per-file `sha256` was swapped for another valid
64-hex value would pass `verify` while being (correctly) rejected by
`acquire`/`materialize`: `acquire authority ≠ verify authority`.

**The fix (tools/acquire.py, `verify()`):** verify is now bound to the same
frozen authority. Per source it additionally fails unless:

```text
SOURCE.repository_url  == source-lock.sources[sid].repository_url
SOURCE.commit_sha      == source-lock.sources[sid].commit_sha
SOURCE.storage_policy  == source-lock.sources[sid].storage_policy
source_manifest_identity(SOURCE)
                       == source-lock.sources[sid].source_manifest_hash
```

and for derived state the **actual on-disk file hashes** must equal the
hashes recorded IN the lock (not merely recompute from current inputs):

```text
sha256(manifests/candidate-universe-v1.json)  == lock.candidate_universe_manifest_sha256
sha256(manifests/exact-duplicates-v1.json)    == lock.exact_duplicates_manifest_sha256
sha256(manifests/inventory-summary-v1.json)   == lock.inventory_summary_manifest_sha256
```

(The inventory manifests were already checked against the lock and remain
so. The recompute-consistency check is retained as a second, weaker
self-consistency layer on top of the lock comparison.)

Consequence: any SOURCE.json drift — including a structurally-clean
per-file sha256 swap — and any derived-manifest drift now fails `verify`
with the same fail-closed semantics as `acquire`/`materialize`. The only
legitimate path for authority change remains an explicit, git-reviewable
`relock`.

## New negative tests (tools/test_acquire.py, 45 → 50 tests, all OK)

| test | scenario | outcome |
|---|---|---|
| `test_source_sha256_drift_fails_verify_without_local_bytes` | the exact review bypass: fresh-clone state (no materialized bytes), structurally-consistent SOURCE with swapped per-file sha256 | `verify` FAILS on frozen-lock identity; `preflight_replay` rejects the same state |
| `test_source_sha256_drift_fails_verify_with_local_bytes` | same drift with bytes materialized | `verify` FAILS — lock-identity gate fires before byte checks |
| `test_source_and_derived_coordinated_drift_fails_verify` | attacker regenerates ALL derived manifests from the tampered SOURCE (coordinated drift; every self-consistency check passes) | `verify` STILL FAILS — the lock-pinned SOURCE identity is the authority; derived regeneration cannot help |
| `test_derived_manifest_only_drift_fails_verify` | candidate-universe manifest tampered on disk | `verify` FAILS — actual file hash != hash recorded in the lock |
| `test_verify_passes_untouched_and_full_requires_bytes` | positive control: untouched state passes plain `verify` even with zero materialized bytes; `--full` still demands the bytes | PASS / FAIL (`--full`) as designed |

## Tool change adopted via the designed path (explicit relock)

Editing `tools/acquire.py` changes the tool sha256, so every replay command
fail-closed as designed until an explicit `relock --keep-pins` adopted the
new tool version. The resulting `source-lock.json` diff is **exactly one
line** — `acquisition_tool_sha256` — proving pins, config hash, SOURCE
identities, inventory hashes, and derived-manifest hashes all survived the
relock unchanged (SOURCE.json ×20 and all 20 inventories were rewritten
byte-identically by the relock itself; git shows no diff for them).

## Validation evidence (all commands re-run for this report)

```text
$ python3 tools/test_acquire.py            → Ran 50 tests ... OK
$ python3 tools/acquire.py acquire         → STRICT REPLAY PASS (20 sources)
$ python3 tools/acquire.py materialize     → MATERIALIZE PASS (3,970 files,
                                             66,391,919 bytes, all verified)
$ python3 tools/acquire.py verify --full   → VERIFY PASS: 3,970 files
                                             byte-verified, 0 missing; lock +
                                             inventories + derived manifests
                                             consistent (now against lock
                                             records directly)
$ python3 tools/acquire.py determinism-check
    DETERMINISM PASS: cold-cache strict replay byte-identical
    digest 042c98acbaf4ec68b7932ddc38ee47afa00a154ce2d9b6e340bcf403aa29f3ae
    (55 tracked artifacts + 3,970 materialized + 21 license artifacts,
     two passes with _cache/repos AND sources/*/files wiped between)
$ git status                               → only source-lock.json (tool hash),
                                             tools/acquire.py, tools/test_acquire.py
```

## PR body truthfulness — fixed

The live PR body on GitHub still showed the round-1 text (3,946 files,
tracked `files/`, old determinism digest) because the corrective-1 body
update silently failed to persist. The body has been re-set with the
corrective-1+2 facts (3,970 candidate files / 66,391,919 bytes; `files/`
gitignored + local materialization; lock + SOURCE + inventory as authority;
determinism digest `042c98ac…`) and **read back from the GitHub API** to
confirm persistence this time.

## Cloud-drive copy: role clarified, no repackaging needed

The uploaded archive
(`markit-33-candidate-corpus-round1-corrective1.zip`,
sha256 `36f4dc6b1fe05ee9a7661617f385651d608f0e5ddcb7f853e3dbd9075a87f322`)
is a **backup / convenience copy only — not benchmark authority**. The
authority remains source-lock.json + SOURCE.json + inventories + per-file
hashes + upstream pinned SHAs; the archive's 3,970 files are unchanged by
this corrective (verify --full re-proves every byte against the manifests),
so the existing archive remains valid and no repackaging is required.
Companion values recorded alongside it on the drive:

```text
archive sha256          36f4dc6b1fe05ee9a7661617f385651d608f0e5ddcb7f853e3dbd9075a87f322
source-lock.json sha256 7884c61b2919420df52413326f2d163bca38d7d52590d0b7978dda9a4eab24a1
                        (schema-2 lock as of corrective 2; repo file is authoritative)
restore + re-verify     unpack into workloads/, then:
                        python3 tools/acquire.py materialize && python3 tools/acquire.py verify --full
```

## Explicit non-goals (unchanged)

No H0-H4 execution, no latency/profiling, no parser behavior runs, no
eligibility scanning, no static characterization, no representative/extremal
selection, no workload changes, no storage-architecture change, no release,
no source modification, no merge. PR #34 remains Draft.
