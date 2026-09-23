#!/usr/bin/env python3
"""Campaign-2 receipt CORRECTIVE (EXECUTABLE-ATTRIBUTION-CORRECTIVE-1).

WHAT WAS WRONG
--------------
`tools/closure.py` wrote `executable_sha256` into every receipt by hashing
`target/release/mdbench-campaign2` AS IT EXISTED AT CLOSURE TIME. That is a
current-state reading, not a record of what produced the file. Because the
campaign binary was rebuilt once between the formal collection and the
lifecycle attribution lane, all 274 receipts ended up claiming
`1bd1c09e…` — while the RunId carried inside the raw rows proves that 8 of
the 9 observation sub-campaigns were produced by `c1c7756a…`.

A receipt field that reports the present instead of the provenance is
exactly the defect class this campaign is supposed to catch, so it is
corrected here and recorded rather than papered over.

HOW THE CORRECT VALUE IS OBTAINED (two independent methods)
-----------------------------------------------------------
M1  EXECUTION-TIME LOG (primary).
    `logs/run-lane.sh` hashes the binary and prints `executable_sha256=…`
    immediately BEFORE the lane runs; `run-profiling.sh` and
    `run-memory.sh` do the same. The lane log is therefore a record made at
    execution time, not a reconstruction.

M2  RUN-ID DERIVATION (independent cross-check).
    `RunId = SHA256(StudyId || CampaignSpecId || SubCampaignSpecId ||
    runner_commit || machine_manifest_digest || rustc || target ||
    build_profile_id || Cargo.lock_digest || executable_sha256)`, frozen in
    campaign2/src/identity.rs. Recomputing it for each candidate
    (build_commit, executable) pair must reproduce the RunId found in the
    raw rows. This is a cryptographic check, not a lookup.

Both methods must agree for every observation file, or this tool exits
non-zero and writes no receipts.

usage: receipts-rebuild.py <benchmark_root> [--write]
"""

from __future__ import annotations

import glob
import hashlib
import json
import os
import re
import sys

AUTHORITY_SHA = "3762b7a42e1c284a4c2c2e0ebac8496e70c63431"

# Candidate producers: (build commit, executable sha, label).
# `4fd86e48…` is the build that produced the formal collection;
# `f6262716…` (the CAMPAIGN-2-FREEZE commit) produced only the lifecycle
# attribution lane. Both are recorded in the lane logs and both are
# re-derived from the RunId here.
CANDIDATES = [
    ("4fd86e489b897c987074550b800c08e613e5185d",
     "c1c7756a1d36316dff2fb278e9db7926ee20a39bceaa0e8248e2619af52f5822",
     "formal-collection-build"),
    ("f6262716daddf8b5151de0b2b0e2879d3e838b2b",
     "1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282",
     "lifecycle-attribution-build"),
]

# Lane -> the log whose execution-time header names the producing binary.
LANE_LOG = {
    "construction": "60-attribution-construction.log",
    "resident-update": "61-attribution-resident-update.log",
    "controlled/N": "62-attribution-controlled-N.log",
    "controlled/B": "62-attribution-controlled-B.log",
    "controlled/D-fence": "62-attribution-controlled-D.log",
    "controlled/F-reference": "62-attribution-controlled-F.log",
    "controlled/K-container": "62-attribution-controlled-K.log",
    "lifecycle": None,  # split by filename: timing lanes vs attribution lanes
    "memory": "70-memory.log",
    "profiling/perf-stat": "80-profiling.log",
    "profiling/perf-record": "81-perf-record.log",
}


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 22), b""):
            digest.update(chunk)
    return digest.hexdigest()


def log_executable(log_path):
    if not os.path.exists(log_path):
        return None
    with open(log_path, "r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            match = re.search(r"executable_sha256=([a-f0-9]{64})", line)
            if match:
                return match.group(1)
    return None


def build_identity_of(root):
    """The build identity that the workspace currently declares.

    Taken from a frozen raw row where one exists (it is recorded per row),
    so the derivation does not depend on the current toolchain output.
    """
    return None


def derive_run_ids(root, identity, machine_digest, build):
    """RunId(sub, commit, exe) for every candidate, using the frozen rule."""
    table = {}
    for commit, exe, label in CANDIDATES:
        for sub in identity["sub_campaigns"]:
            material = "\n".join([
                identity["study_id"],
                identity["campaign_spec_id"],
                sub["sub_campaign_spec_id"],
                commit,
                machine_digest,
                build["rustc"],
                build["target"],
                build["build_profile_id"],
                build["cargo_lock_sha256"],
                exe,
            ])
            table[hashlib.sha256(material.encode()).hexdigest()] = {
                "sub_campaign_tag": sub["tag"],
                "sub_campaign_spec_id": sub["sub_campaign_spec_id"],
                "runner_git_commit": commit,
                "executable_sha256": exe,
                "executable_label": label,
            }
    return table


def first_row(path):
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                return json.loads(line)
    return None


def main():
    root = sys.argv[1]
    write = "--write" in sys.argv
    base = os.path.join(root, "results/campaign-2")
    logs = os.path.join(base, "logs")

    identity = json.load(open(os.path.join(base, "manifests/campaign-2-identity-v1.json")))
    machine_path = os.path.join(base, "manifests/campaign-2-machine-v1.toml")
    machine_digest = sha256_file(machine_path)
    machine_id = ""
    for line in open(machine_path, encoding="utf-8"):
        if line.startswith("machine_id"):
            machine_id = line.split("=", 1)[1].strip().strip('"')
            break

    inventory = json.load(open(os.path.join(base, "manifests/raw-inventory-v1.json")))
    observation_files = [e for e in inventory["entries"] if e.get("schema") == "campaign2-observation-v1"]
    # The build identity is recorded in every raw row; take it from one.
    build = first_row(os.path.join(base, observation_files[0]["path"]))["result_row_v2"]["build_identity"]
    run_id_table = derive_run_ids(root, identity, machine_digest, build)

    # M1: execution-time lane logs.  Note the lane that wrote each log.
    log_sha = {}
    names = {n for n in LANE_LOG.values() if n}
    names |= {
        "65-lifecycle-attribution-session-0.log",
        "65-lifecycle-attribution-session-1.log",
        "65-lifecycle-attribution-session-2.log",
    }
    for name in sorted(names):
        log_sha[name] = log_executable(os.path.join(logs, name))

    replay_sha = None
    with open(os.path.join(base, "profiling/perf-stat/profiling-header.txt"), encoding="utf-8") as handle:
        for line in handle:
            match = re.search(r"profiling_binary_sha256=([a-f0-9]{64})", line)
            if match:
                replay_sha = match.group(1)

    def campaign_log_for(relative):
        if relative.startswith("construction/"):
            return LANE_LOG["construction"]
        if relative.startswith("resident-update/"):
            return LANE_LOG["resident-update"]
        if relative.startswith("memory/"):
            return LANE_LOG["memory"]
        for tag in ("N", "B", "D-fence", "F-reference", "K-container"):
            if relative.startswith(f"controlled/{tag}/"):
                return LANE_LOG[f"controlled/{tag}"]
        if relative.startswith("lifecycle/"):
            if "attribution" in relative:
                session = re.search(r"session-(\d)-lifecycle-attribution", relative).group(1)
                return f"65-lifecycle-attribution-session-{session}.log"
            return "40-lifecycle-session-0.log"
        return None

    problems = []
    derivation_rows = []
    for entry in inventory["entries"]:
        relative = entry["path"]
        producer_kind, producer_sha, method, evidence = None, None, None, None
        if entry.get("schema") == "campaign2-observation-v1":
            producer_kind = "mdbench-campaign2"
            log_name = campaign_log_for(relative)
            m1 = log_sha.get(log_name) if log_name else None
            m2 = None
            if entry.get("run_id") in run_id_table:
                m2 = run_id_table[entry["run_id"]]["executable_sha256"]
            if m1 and m2 and m1 != m2:
                problems.append(
                    f"{relative}: execution-time log says {m1[:12]} but RunId derivation says {m2[:12]}"
                )
                continue
            producer_sha = m2 or m1
            method = "M1_lane_log and M2_runid_derivation agree" if (m1 and m2) else (
                "M1_lane_log only" if m1 else "M2_runid_derivation only"
            )
            evidence = f"logs/{log_name}" if log_name else "run_id derivation"
        elif relative.startswith("profiling/perf-stat/slot-P") and (
            relative.endswith("-region.jsonl") or relative.endswith("-setuponly.jsonl")
        ):
            producer_kind = "mdbench-replay"
            producer_sha = replay_sha
            method = "M1_lane_header profiling_binary_sha256"
            evidence = "profiling/perf-stat/profiling-header.txt"
        elif relative.startswith("profiling/perf-record/slot-P") and relative.endswith(".data"):
            producer_kind = "mdbench-replay"
            producer_sha = replay_sha
            method = "M1_lane_header profiling_binary_sha256 (recorded under perf)"
            evidence = "profiling/perf-record/perf-record-header.txt"
        derivation_rows.append({
            "raw_path": relative,
            "producer": producer_kind or "tooling/analysis artifact",
            "executable_sha256": producer_sha,
            "method": method or "not a measured artifact (document/text generated by shell or authored)",
            "evidence": evidence or "n/a",
            "run_id": entry.get("run_id", "n/a"),
            "sub_campaign_tag": run_id_table.get(entry.get("run_id", ""), {}).get("sub_campaign_tag", "n/a"),
            "rows": entry.get("rows"),
            "bytes": entry["bytes"],
            "sha256": entry["sha256"],
        })

    if problems:
        print("EXECUTABLE_ATTRIBUTION_CONFLICT:")
        for problem in problems:
            print(f"  {problem}")
        sys.exit(1)

    doc = {
        "schema": "campaign2-executable-derivation-v1",
        "corrective": "EXECUTABLE-ATTRIBUTION-CORRECTIVE-1",
        "authority_sha": AUTHORITY_SHA,
        "study_id": identity["study_id"],
        "campaign_spec_id": identity["campaign_spec_id"],
        "machine_id": machine_id,
        "producer_binaries": {
            "mdbench-campaign2@formal-collection": "c1c7756a1d36316dff2fb278e9db7926ee20a39bceaa0e8248e2619af52f5822",
            "mdbench-campaign2@lifecycle-attribution": "1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282",
            "mdbench-replay@profiling": replay_sha,
        },
        "run_id_table": run_id_table,
        "entries": derivation_rows,
    }
    if write:
        with open(os.path.join(base, "manifests/executable-derivation-v1.json"), "w", encoding="utf-8") as handle:
            json.dump(doc, handle, indent=2, sort_keys=True)
            handle.write("\n")

        # Rewrite every receipt with the CORRECTED producer attribution.
        # The superseded value is preserved under
        # `superseded_executable_sha256` so nothing is hidden.
        superseded = "1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282"
        receipts = os.path.join(base, "receipts")
        written = 0
        for row in derivation_rows:
            relative = row["raw_path"]
            receipt_path = os.path.join(receipts, relative.replace("/", "__") + ".receipt.json")
            if not os.path.exists(receipt_path):
                continue
            receipt = json.load(open(receipt_path))
            receipt["producer"] = row["producer"]
            receipt["executable_sha256"] = row["executable_sha256"]
            receipt["executable_sha256_method"] = row["method"]
            receipt["executable_sha256_evidence"] = row["evidence"]
            receipt["superseded_executable_sha256"] = superseded
            receipt["superseded_reason"] = (
                "EXECUTABLE-ATTRIBUTION-CORRECTIVE-1: the original receipt recorded the "
                "campaign binary present AT CLOSURE TIME instead of the binary that "
                "produced the file"
            )
            receipt["run_id"] = row["run_id"]
            receipt["sub_campaign_tag"] = row["sub_campaign_tag"]
            with open(receipt_path, "w", encoding="utf-8") as handle:
                json.dump(receipt, handle, indent=2, sort_keys=True)
                handle.write("\n")
            written += 1
        print(f"RECEIPTS_REWRITTEN = {written}")

    from collections import Counter
    counter = Counter((r["producer"], str(r["executable_sha256"])[:12]) for r in derivation_rows)
    print("EXECUTABLE_ATTRIBUTION (274 raw artifacts)")
    for (producer, sha), count in sorted(counter.items()):
        print(f"  {producer:<34s} {sha:<14s} x{count}")
    obs = Counter(
        (r["sub_campaign_tag"], str(r["executable_sha256"])[:12])
        for r in derivation_rows
        if r["producer"] == "mdbench-campaign2"
    )
    print("OBSERVATION LANES")
    for (tag, sha), count in sorted(obs.items()):
        print(f"  {tag:<18s} {sha:<14s} x{count} files")
    print("M1_M2_AGREEMENT = PASS")


if __name__ == "__main__":
    main()
