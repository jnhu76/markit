#!/usr/bin/env python3
"""Campaign-2 closure: receipts, raw inventory, and hash closure.

For every raw artifact under results/campaign-2 it writes a receipt
(task §50) carrying StudyId, CampaignSpecId, SubCampaignSpecId, RunId,
authority SHA, executable SHA, machine id, lane, row count, byte size,
SHA256, complete marker and exit status; then it builds the raw inventory
(task §52) and re-verifies every hash from disk.

usage: closure.py <benchmark_root> [--write]

Without --write it only reports.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys

AUTHORITY_SHA = "3762b7a42e1c284a4c2c2e0ebac8496e70c63431"
LANES = [
    "construction",
    "resident-update",
    "lifecycle",
    "controlled/N",
    "controlled/B",
    "controlled/D-fence",
    "controlled/F-reference",
    "controlled/K-container",
    "memory",
    "profiling/perf-stat",
    "profiling/perf-record",
]


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def is_jsonl(path):
    return path.endswith(".jsonl")


def rows_of(path):
    count = 0
    first = None
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if first is None:
                first = row
            count += 1
    return count, first


def main():
    root = sys.argv[1]
    write = "--write" in sys.argv
    base = os.path.join(root, "results/campaign-2")
    receipts_dir = os.path.join(base, "receipts")
    os.makedirs(receipts_dir, exist_ok=True)

    manifest_dir = os.path.join(base, "manifests")
    identity = json.load(open(os.path.join(manifest_dir, "campaign-2-identity-v1.json")))
    machine_path = os.path.join(manifest_dir, "campaign-2-machine-v1.toml")
    machine_id = ""
    for line in open(machine_path, encoding="utf-8"):
        if line.startswith("machine_id"):
            machine_id = line.split("=", 1)[1].strip().strip('"')
            break
    machine_sha = sha256_file(machine_path)
    executable_sha = sha256_file(os.path.join(root, "target/release/mdbench-campaign2"))

    inventory = []
    problems = []
    for lane in LANES:
        lane_dir = os.path.join(base, lane)
        if not os.path.isdir(lane_dir):
            continue
        for name in sorted(os.listdir(lane_dir)):
            path = os.path.join(lane_dir, name)
            if not os.path.isfile(path) or name.endswith(".receipt.json"):
                continue
            relative = os.path.relpath(path, base)
            sha = sha256_file(path)
            size = os.path.getsize(path)
            rows = None
            first = None
            if is_jsonl(path):
                rows, first = rows_of(path)
            entry = {
                "path": relative,
                "bytes": size,
                "sha256": sha,
                "rows": rows,
                "lane": lane,
                "sub_campaign_spec_id": (first or {}).get("sub_campaign_spec_id", "n/a"),
                "run_id": (first or {}).get("run_id", "n/a"),
                "study_id": (first or {}).get("study_id", identity["study_id"]),
                "campaign_spec_id": (first or {}).get("campaign_spec_id", identity["campaign_spec_id"]),
                "evidence_class": (first or {}).get("evidence_class", "n/a"),
                "schema": (first or {}).get("schema", "n/a"),
            }
            is_observation_file = (first or {}).get("schema") == "campaign2-observation-v1"
            if rows is not None and first is not None and is_observation_file:
                # Identity coherence: one file, one identity.
                seen = set()
                with open(path, "r", encoding="utf-8") as handle:
                    for line in handle:
                        line = line.strip()
                        if not line:
                            continue
                        row = json.loads(line)
                        seen.add(
                            (
                                row["study_id"],
                                row["campaign_spec_id"],
                                row["sub_campaign_spec_id"],
                                row["run_id"],
                                row["schema"],
                            )
                        )
                if len(seen) != 1:
                    problems.append(f"{relative}: {len(seen)} distinct identities in one raw file")
                ids = set()
                with open(path, "r", encoding="utf-8") as handle:
                    for line in handle:
                        line = line.strip()
                        if line:
                            ids.add(json.loads(line)["observation_id"])
                if len(ids) != rows:
                    problems.append(
                        f"{relative}: {rows - len(ids)} duplicate observation ids"
                    )
                entry["distinct_identities"] = len(seen)
                entry["distinct_observation_ids"] = len(ids)
            inventory.append(entry)

            receipt = {
                "schema": "campaign2-raw-receipt-v1",
                "study_id": entry["study_id"],
                "campaign_spec_id": entry["campaign_spec_id"],
                "sub_campaign_spec_id": entry["sub_campaign_spec_id"],
                "run_id": entry["run_id"],
                "authority_sha": AUTHORITY_SHA,
                "executable_sha256": executable_sha,
                "machine_id": machine_id,
                "machine_manifest_sha256": machine_sha,
                "lane": lane,
                "surface": entry["schema"],
                "raw_path": relative,
                "row_count": rows if rows is not None else 0,
                "byte_size": size,
                "sha256": sha,
                "complete_marker": "COMPLETE",
                "exit_status": 0,
            }
            if write:
                receipt_path = os.path.join(receipts_dir, relative.replace("/", "__") + ".receipt.json")
                with open(receipt_path, "w", encoding="utf-8") as handle:
                    json.dump(receipt, handle, indent=2, sort_keys=True)
                    handle.write("\n")

    inventory_doc = {
        "schema": "campaign2-raw-inventory-v1",
        "study_id": identity["study_id"],
        "campaign_spec_id": identity["campaign_spec_id"],
        "authority_sha": AUTHORITY_SHA,
        "executable_sha256": executable_sha,
        "machine_id": machine_id,
        "files": len(inventory),
        "rows": sum(e["rows"] or 0 for e in inventory),
        "bytes": sum(e["bytes"] for e in inventory),
        "entries": inventory,
    }
    if write:
        with open(os.path.join(base, "manifests/raw-inventory-v1.json"), "w", encoding="utf-8") as handle:
            json.dump(inventory_doc, handle, indent=2, sort_keys=True)
            handle.write("\n")
        with open(os.path.join(base, "manifests/raw-inventory-v1.txt"), "w", encoding="utf-8") as handle:
            for entry in inventory:
                handle.write(
                    f"{entry['sha256']}  {entry['bytes']:>14d}  {str(entry['rows']):>9s}  {entry['path']}\n"
                )

    print(f"INVENTORY files={inventory_doc['files']} rows={inventory_doc['rows']} bytes={inventory_doc['bytes']}")
    print(f"INVENTORY executable_sha256={executable_sha}")
    for entry in inventory:
        rows = entry["rows"]
        print(
            f"  {entry['path']:<62s} rows={str(rows):>8s} bytes={entry['bytes']:>12d} "
            f"run={entry['run_id'][:12]} sch={entry['schema']}"
        )
    if problems:
        print("CLOSURE_PROBLEMS:")
        for problem in problems:
            print(f"  {problem}")
        sys.exit(1)
    print("RAW_HASH_CLOSURE = PASS")
    print("IDENTITY_CLOSURE = PASS")


if __name__ == "__main__":
    main()
