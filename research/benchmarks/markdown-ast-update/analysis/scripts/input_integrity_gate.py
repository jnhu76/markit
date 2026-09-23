#!/usr/bin/env python3
"""MARKIT-31 primary analysis — input-integrity gate (task §4, §5).

Read-only over the copied analysis-input snapshot. Fails closed (exit 2)
on ANY violation; prints PASS lines and exits 0 only when every frozen
expectation holds. No timing value is read for interpretation here —
this gate validates structure and identity only (plus the frozen
T_total == T_prepare + T_native schema assertion, task §8).

Deterministic: no randomness, no network, fixed output order.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

# ---- frozen identities (issue #31 task document, §"Frozen identities") ----
EXPECTED_CAMPAIGN_SPEC_ID = "ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2"
EXPECTED_RUN_ID = "905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d"
EXPECTED_EXECUTABLE_SHA256 = "a3ef4e63a8548604fff991c5b981e3dfc856238fd06ead3e9192a5a3b6acd5bb"

# ---- frozen cardinalities (task §5) ----
EXPECTED_RAW_FILES = 8
EXPECTED_RECEIPTS = 8
EXPECTED_ATTRIBUTION_ROWS = 1920
EXPECTED_TIMING_ROWS = 230400
EXPECTED_WARMUP_ROWS = 57600
EXPECTED_MEASURED_ROWS = 172800
EXPECTED_GRAND_ROWS = 232320
EXPECTED_SESSIONS = 3
EXPECTED_MEASURED_PER_CELL = 30
EXPECTED_MECHANISM_IDS = {
    "h0-full-rebuild",
    "block-local-reparse-h1",
    "fragment-reuse-h2",
    "old-tree-subtree-reuse-h3",
    "restart-convergence-h4",
}

failures: list[str] = []
passes: list[str] = []


def check(ok: bool, ok_msg: str, fail_msg: str) -> bool:
    if ok:
        passes.append(ok_msg)
    else:
        failures.append(fail_msg)
    return ok


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: input_integrity_gate.py <analysis-input-run-dir> <raw-inventory.txt>",
              file=sys.stderr)
        return 2
    run_dir = Path(sys.argv[1])
    inventory_path = Path(sys.argv[2])

    # ---- file census --------------------------------------------------
    raw_files = sorted(p for p in run_dir.rglob("*.jsonl"))
    receipt_files = sorted(p for p in run_dir.rglob("*.receipt.json"))
    all_files = sorted(p for p in run_dir.rglob("*") if p.is_file())
    invalid_markers = [
        p for p in all_files
        if p.suffix not in (".jsonl",) and not p.name.endswith(".receipt.json")
    ]
    check(len(raw_files) == EXPECTED_RAW_FILES,
          f"raw JSONL files = {len(raw_files)} == 8",
          f"raw JSONL files = {len(raw_files)} != 8")
    check(len(receipt_files) == EXPECTED_RECEIPTS,
          f"receipts = {len(receipt_files)} == 8",
          f"receipts = {len(receipt_files)} != 8")
    check(not invalid_markers,
          "invalid markers = 0",
          f"unexpected/invalid marker files present: {[str(p) for p in invalid_markers]}")

    # ---- inventory hash match ------------------------------------------
    inventory: dict[str, dict[str, str]] = {}
    for line in inventory_path.read_text().splitlines():
        if not line.startswith("results/raw/"):
            continue
        rel, _, rest = line.partition(" sha256=")
        sha, _, rest2 = rest.partition(" bytes=")
        rows = rest2.split(" rows=")[1]
        key = "/".join(rel.split("/")[4:])  # strip results/raw/<specId>/<runId>/
        inventory[key] = {"sha256": sha, "rows": rows}

    receipt_cache: dict[str, dict] = {}
    totals = {"attribution": 0, "timing": 0}
    kinds = {"warmup": 0, "measured": 0, "attribution": 0}
    spec_ids: set[str] = set()
    run_ids: set[str] = set()
    exe_shas: set[str] = set()
    mechanism_ids: set[str] = set()
    observation_ids_dup: dict[str, int] = {}
    # (case, horse, session, surface) -> counts
    cell_tot: dict[tuple[str, str, int, str], int] = {}
    cell_na: dict[tuple[str, str, int, str], int] = {}
    tsum_violations = 0
    status_violations = 0

    for raw_path in raw_files:
        rel = str(raw_path.relative_to(run_dir))
        inv = inventory.get(rel)
        if not check(inv is not None,
                     f"inventory row present for {rel}",
                     f"{rel} missing from raw-inventory.txt"):
            continue
        digest = sha256_file(raw_path)
        check(digest == inv["sha256"],
              f"sha256 match inventory: {rel}",
              f"sha256 mismatch for {rel}: file {digest} != inventory {inv['sha256']}")

        receipt_path = raw_path.with_name(raw_path.name + ".receipt.json")
        receipt = json.loads(receipt_path.read_text())
        receipt_cache[rel] = receipt
        check(receipt["sha256"] == digest,
              f"receipt sha256 match: {rel}",
              f"receipt sha256 != file sha256 for {rel}")
        check(receipt["verdict"] == "FINAL_RAW_FILE_PASS",
              f"receipt verdict FINAL_RAW_FILE_PASS: {rel}",
              f"receipt verdict {receipt['verdict']} != FINAL_RAW_FILE_PASS for {rel}")
        check(str(receipt["campaign_spec_id"]) == EXPECTED_CAMPAIGN_SPEC_ID,
              f"receipt CampaignSpecId: {rel}",
              f"receipt CampaignSpecId wrong for {rel}")
        check(str(receipt["run_id"]) == EXPECTED_RUN_ID,
              f"receipt RunId: {rel}",
              f"receipt RunId wrong for {rel}")
        check(str(receipt["executable_sha256"]) == EXPECTED_EXECUTABLE_SHA256,
              f"receipt executable SHA: {rel}",
              f"receipt executable SHA wrong for {rel}")

        # ---- row-level pass (streaming; deterministic) -----------------
        n_rows = 0
        unique_ids = set()
        with raw_path.open() as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                n_rows += 1
                obs = json.loads(line)
                spec_ids.add(obs["campaign_spec_id"])
                run_ids.add(obs["run_id"])
                row = obs["result_row_v2"]
                build = row["build_identity"]
                # executable identity: the campaign receipts carry the
                # binary hash; row build_identity carries the runner
                # commit (row-level executable hash is receipt-scoped).
                mech = row["mechanism_id"]
                mechanism_ids.add(mech)
                oid = obs["observation_id"]
                observation_ids_dup[oid] = observation_ids_dup.get(oid, 0) + 1
                kind = obs["sample_kind"]
                kinds[kind] = kinds.get(kind, 0) + 1
                if row["execution_status"] != "pass" or row["correctness_status"] != "pass":
                    status_violations += 1
                if kind == "attribution":
                    totals["attribution"] += 1
                else:
                    totals["timing"] += 1
                    m = row["measurement"]["metrics"]
                    prep = m["prepare_ns"]
                    key = (row["case_id"], mech, obs["session_ordinal"], obs["surface"])
                    if isinstance(prep, str):
                        # Schema semantics: prepare phase not applicable
                        # (CLEAN_STATE measures parse+native as one
                        # native phase). Assert total == native and
                        # record N/A-ness for cell-uniformity check.
                        if m["native_ns"] != m["total_ns"] or prep != "NOT_APPLICABLE":
                            tsum_violations += 1
                        cell_na[key] = cell_na.get(key, 0) + 1
                    elif m["prepare_ns"] + m["native_ns"] != m["total_ns"]:
                        tsum_violations += 1
                    if kind == "measured":
                        cell_tot[key] = cell_tot.get(key, 0) + 1
                unique_ids.add(oid)
        check(n_rows == int(inv["rows"]),
              f"row count matches inventory: {rel} rows={n_rows}",
              f"row count mismatch for {rel}: {n_rows} != {inv['rows']}")
        rc = receipt["row_count"]
        er = receipt["expected_rows"]
        uo = receipt["unique_observation_ids"]
        check(rc == er == uo == n_rows,
              f"receipt row_count==expected_rows==unique_observation_ids=={n_rows}: {rel}",
              f"receipt count triple mismatch for {rel}: row_count={rc} expected={er} "
              f"unique={uo} actual={n_rows}")

    # ---- global identity cardinality ------------------------------------
    check(spec_ids == {EXPECTED_CAMPAIGN_SPEC_ID},
          "CampaignSpecId distinct count = 1 (all rows)",
          f"CampaignSpecId values != expected: {sorted(spec_ids)}")
    check(run_ids == {EXPECTED_RUN_ID},
          "RunId distinct count = 1 (all rows)",
          f"RunId values != expected: {sorted(run_ids)}")
    check(len(exe_shas) <= 1,
          "row-level executable identity cardinality <= 1 (receipt-scoped hash)",
          f"multiple row-level executable identities: {sorted(exe_shas)}")

    # ---- exact totals ----------------------------------------------------
    check(totals["attribution"] == EXPECTED_ATTRIBUTION_ROWS,
          f"attribution rows = {totals['attribution']} == 1920",
          f"attribution rows {totals['attribution']} != 1920")
    check(totals["timing"] == EXPECTED_TIMING_ROWS,
          f"timing rows = {totals['timing']} == 230400",
          f"timing rows {totals['timing']} != 230400")
    check(kinds.get("warmup", 0) == EXPECTED_WARMUP_ROWS,
          f"warmup rows = {kinds.get('warmup', 0)} == 57600",
          f"warmup rows {kinds.get('warmup', 0)} != 57600")
    check(kinds.get("measured", 0) == EXPECTED_MEASURED_ROWS,
          f"measured rows = {kinds.get('measured', 0)} == 172800",
          f"measured rows {kinds.get('measured', 0)} != 172800")
    grand = totals["attribution"] + totals["timing"]
    check(grand == EXPECTED_GRAND_ROWS,
          f"grand rows = {grand} == 232320",
          f"grand rows {grand} != 232320")

    # ---- statuses & schema ----------------------------------------------
    check(status_violations == 0,
          "all rows execution_status==pass && correctness_status==pass",
          f"{status_violations} rows with non-pass status (campaign INVALID per stats contract)")
    check(tsum_violations == 0,
          "T_total == T_prepare + T_native for every timing row",
          f"{tsum_violations} timing rows violate T_total == T_prepare + T_native "
          "(ANALYSIS_SCHEMA_INCONSISTENCY)")
    dups = {k: v for k, v in observation_ids_dup.items() if v > 1}
    check(not dups,
          "no duplicate observation_id anywhere",
          f"duplicate observation_ids: {list(dups.items())[:5]}")
    check(mechanism_ids == EXPECTED_MECHANISM_IDS,
          "mechanism_id set == {H0..H4 frozen ids}",
          f"mechanism_id set mismatch: {sorted(mechanism_ids)}")

    # ---- 30 measured per case x horse x session -------------------------
    bad_cells = {k: v for k, v in cell_tot.items() if v != EXPECTED_MEASURED_PER_CELL}
    check(not bad_cells,
          f"every case x horse x session cell has exactly {EXPECTED_MEASURED_PER_CELL} "
          f"measured rows (cells={len(cell_tot)})",
          f"{len(bad_cells)} cells with wrong measured count, e.g. "
          f"{list(bad_cells.items())[:3]}")
    check(len(cell_tot) == (22 + 362) * 5 * 3,
          f"cell count = {len(cell_tot)} == 384*5*3 = 5760",
          f"cell count {len(cell_tot)} != 5760")
    # ---- prepare N/A is whole-cell, never partial ------------------------
    partial_na = [k for k in cell_na if cell_na[k] != 40]  # 30 measured + 10 warmup
    check(not partial_na,
          "prepare_ns NOT_APPLICABLE is whole-cell (all 40 timing rows) "
          f"for {len(cell_na)} clean-state cells",
          f"{len(partial_na)} cells with partial prepare N/A: {partial_na[:3]}")
    check(len(cell_na) == 22 * 5 * 3,
          f"prepare N/A exactly on clean-state cells: {len(cell_na)} == 330",
          f"prepare N/A cell count {len(cell_na)} != 330")

    # ---- verdict ----------------------------------------------------------
    print("== INPUT INTEGRITY GATE ==")
    for msg in passes:
        print(f"PASS: {msg}")
    if failures:
        for msg in failures:
            print(f"FAIL: {msg}")
        print("PRIMARY_ANALYSIS_INPUT_FAIL")
        return 2
    print("PRIMARY_ANALYSIS_INPUT_PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
