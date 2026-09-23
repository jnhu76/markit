#!/usr/bin/env python3
"""MARKIT-31 primary analysis — Stage A deterministic summary tool.

Consumes ONLY the frozen primary raw evidence (copied snapshot) and the
frozen workload metadata. Implements the frozen statistics contract of
`campaign/src/stats.rs` exactly:

- nearest-rank p50 (rank 15) / p95 (rank 29) over exactly 30 measured
  samples per case x horse x session; no interpolation;
- no pooling of the 3 sessions; case point estimate = median of the
  3 session values;
- H0-relative speedup is paired per session, then geometric mean of the
  3 session ratios (never mean(H0)/mean(H));
- session instability = max/min of the 3 session total p50s > 1.5
  (diagnostic only; never deletes data);
- PROJECT_MACRO: case -> trace -> file -> project -> equal-weight
  projects, geometric mean at every level;
- FAMILY_MACRO: equal weight across families of their project-macros.

Fails closed on identity drift, wrong row counts, duplicate
observations, missing cells, or unknown case ids. Deterministic output
order; no network; no randomness.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

# ---- frozen identities -------------------------------------------------
CAMPAIGN_SPEC_ID = "ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2"
RUN_ID = "905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d"

# ---- frozen statistics contract (campaign/src/stats.rs) ----------------
P50_RANK = 15
P95_RANK = 29
SAMPLES_PER_CELL = 30
SESSIONS = (0, 1, 2)
SESSION_UNSTABLE_RATIO = 1.5

HORSE_LABEL = {
    "h0-full-rebuild": "H0",
    "block-local-reparse-h1": "H1",
    "fragment-reuse-h2": "H2",
    "old-tree-subtree-reuse-h3": "H3",
    "restart-convergence-h4": "H4",
}
HORSES = ("H0", "H1", "H2", "H3", "H4")

EXPECTED_CLEAN_STATE_CASES = 22
EXPECTED_EDIT_WRITE_CASES = 362

NA = ""  # CSV rendering of NOT_APPLICABLE / unavailable


class Fail(Exception):
    pass


# ---- frozen primitive re-implementations (mirror stats.rs) -------------
def cell_quantiles(values: list[int]) -> tuple[int, int]:
    """Nearest-rank p50/p95 over exactly 30 ascending-sorted samples."""
    if len(values) != SAMPLES_PER_CELL:
        raise Fail(f"cell has {len(values)} measured samples != {SAMPLES_PER_CELL}")
    s = sorted(values)
    return s[P50_RANK - 1], s[P95_RANK - 1]


def median_of_three(a, b, c):
    return sorted((a, b, c))[1]


def geometric_mean(values: list[float]) -> float:
    if not values:
        raise Fail("geometric mean of empty set")
    logs = []
    for v in values:
        if v <= 0.0 or not math.isfinite(v):
            raise Fail(f"non-positive or non-finite ratio {v}")
        logs.append(math.log(v))
    return math.exp(math.fsum(logs) / len(logs))


def unstable_ratio(p50s: list[int]) -> tuple[bool, float]:
    lo, hi = min(p50s), max(p50s)
    ratio = hi / lo
    return ratio > SESSION_UNSTABLE_RATIO, ratio


# ---- frozen metadata loading -------------------------------------------
def read_jsonl(path: Path) -> list[dict]:
    rows = []
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def check_identity(obj: dict, where: str) -> None:
    spec = obj.get("campaign_spec_id")
    run = obj.get("run_id")
    if spec is not None and spec != CAMPAIGN_SPEC_ID:
        raise Fail(f"{where}: campaign_spec_id {spec} != frozen")
    if run is not None and run != RUN_ID:
        raise Fail(f"{where}: run_id {run} != frozen")


def load_metadata(root: Path) -> dict:
    meta: dict = {}

    sched_path = root / "results/manifests/primary-schedule-v1.jsonl"
    sched_rows = read_jsonl(sched_path)
    sched: dict[tuple[str, str], dict] = {}
    for row in sched_rows:
        check_identity(row, "schedule")
        key = (row["surface"], row["case_id"])
        entry = {
            "payload_id": row["payload_id"],
            "source_key": row["source_key"],
            "trace_id": row.get("trace_id", ""),
        }
        if key in sched:
            if sched[key] != entry:
                raise Fail(f"schedule disagrees across sessions for {key}")
        else:
            sched[key] = entry
    meta["schedule"] = sched

    # EDIT_WRITE payloads (362 G0_PRIMARY)
    ew: dict[str, dict] = {}
    ew_rows = read_jsonl(root / "workloads/payloads/edit-write-manifest-v1.jsonl")
    for row in ew_rows:
        if "G0_PRIMARY" not in row["memberships"]:
            continue
        inserted = row["edit"].get("inserted_text", "") or ""
        inserted_bytes = len(inserted.encode("utf-8"))
        span = row["edit"]["edit_end"] - row["edit"]["edit_start"]
        pos = row.get("position") or {}  # RESTORE steps carry no position
        requested = pos.get("requested_positions") or []
        ew[row["payload_id"]] = {
            "payload_id": row["payload_id"],
            "edit_family": row["edit_family"],
            "operation_variant": row["operation_variant"],
            "source_path": row["source_path"],
            "source_id": row["source_id"],
            "trace_id": row["trace_id"],
            "step": row["step"],
            "position_requested": "|".join(requested),
            "position_actual_fraction": pos.get("actual_relative_position"),
            "logical_edited_bytes": max(span, inserted_bytes),
            "edit_start": row["edit"]["edit_start"],
            "edit_end": row["edit"]["edit_end"],
            "inserted_bytes": inserted_bytes,
            "syntax_target": row.get("syntax_target", ""),
        }
    if len(ew) != EXPECTED_EDIT_WRITE_CASES:
        raise Fail(f"G0_PRIMARY payloads {len(ew)} != {EXPECTED_EDIT_WRITE_CASES}")
    meta["edit_write"] = ew

    # CLEAN_STATE files (22 G0_STRICT_FULL_READ)
    fr: dict[str, dict] = {}
    fr_rows = read_jsonl(root / "workloads/payloads/full-read-manifest-v1.jsonl")
    for row in fr_rows:
        strict = any(lane.get("case_class") == "G0_STRICT_FULL_READ"
                     for lane in row.get("lanes", []))
        if not strict:
            continue
        fr[row["source_key"]] = {
            "payload_id": f"full-read:{row['source_key']}",
            "source_path": row["source_key"],
            "source_id": row["source_id"],
            "file_bytes": row["file_bytes"],
        }
    if len(fr) != EXPECTED_CLEAN_STATE_CASES:
        raise Fail(f"G0_STRICT_FULL_READ files {len(fr)} != {EXPECTED_CLEAN_STATE_CASES}")
    meta["clean_state"] = fr

    # Structural profile (file-level, 22 rows)
    prof: dict[str, dict] = {}
    for row in read_jsonl(root / "workloads/profiles/strict-surface-profile-v1.jsonl"):
        st = row["structural"]
        prof[row["source_key"]] = {
            "file_bytes": row["file_bytes"],
            "block_count": st["block_count"],
            "largest_block_bytes": st["largest_block_bytes"],
            "max_container_depth": st["max_container_depth"],
            "reference_definition_count": st["reference_definition_count"],
            "reference_use_count": st["reference_use_count"],
            "fenced_code_content_bytes": st["fenced_code_content_bytes"],
            "code_occupancy": st["code_occupancy"],
            "fence_density_per_kib": st["fence_density_per_kib"],
            "reference_density_per_kib": st["reference_density_per_kib"],
        }
    meta["profile"] = prof

    # Traces: chain_kind + ordered payload steps
    traces: dict[str, dict] = {}
    for row in read_jsonl(root / "workloads/payloads/trace-manifest-v1.jsonl"):
        steps = sorted(row["steps"], key=lambda s: s["step"])
        traces[row["trace_id"]] = {
            "chain_kind": row["chain_kind"],
            "payload_ids": [s["payload_id"] for s in steps],
        }
    meta["traces"] = traces
    return meta


def phase_label(meta: dict, payload_id: str) -> str:
    rec = meta["edit_write"][payload_id]
    trace = meta["traces"].get(rec["trace_id"])
    if trace is None:
        raise Fail(f"payload {payload_id} trace missing")
    if trace["chain_kind"] == "break_restore_pair":
        if len(trace["payload_ids"]) != 2:
            raise Fail(f"break_restore_pair trace without 2 steps: {rec['trace_id']}")
        return "BREAK" if rec["step"] == 0 else "RESTORE"
    return "SINGLE"


def case_meta(meta: dict, surface: str, case_id: str) -> dict:
    """Frozen metadata join for one case (schedule -> payload/profile)."""
    entry = meta["schedule"].get((surface, case_id))
    if entry is None:
        raise Fail(f"case {case_id} missing from schedule for surface {surface}")
    if surface == "clean_state":
        fr = meta["clean_state"].get(entry["source_key"])
        if fr is None:
            raise Fail(f"clean_state source {entry['source_key']} missing")
        prof = meta["profile"].get(entry["source_key"], {})
        out = {
            "payload_id": entry["payload_id"],
            "project": fr["source_id"],
            "file": fr["source_path"],
            "trace_id": "",
            "trace_kind": "",
            "step": "",
            "phase": "",
            "edit_family": "FULL_READ",
            "position_requested": "",
            "position_actual_fraction": "",
            "operation_variant": "full_parse",
            "pre_bytes": fr["file_bytes"],
            "logical_edited_bytes": NA,
            "edit_start": NA,
            "edit_end": NA,
            "inserted_bytes": NA,
        }
    else:
        rec = meta["edit_write"].get(entry["payload_id"])
        if rec is None:
            raise Fail(f"payload {entry['payload_id']} missing from edit-write manifest")
        trace = meta["traces"].get(rec["trace_id"], {})
        prof = meta["profile"].get(rec["source_path"], {})
        out = {
            "payload_id": rec["payload_id"],
            "project": rec["source_id"],
            "file": rec["source_path"],
            "trace_id": rec["trace_id"],
            "trace_kind": trace.get("chain_kind", ""),
            "step": rec["step"],
            "phase": phase_label(meta, rec["payload_id"]),
            "edit_family": rec["edit_family"],
            "position_requested": rec["position_requested"],
            "position_actual_fraction": rec["position_actual_fraction"],
            "operation_variant": rec["operation_variant"],
            "pre_bytes": NA,  # filled from raw payload.size_bytes by caller
            "logical_edited_bytes": rec["logical_edited_bytes"],
            "edit_start": rec["edit_start"],
            "edit_end": rec["edit_end"],
            "inserted_bytes": rec["inserted_bytes"],
        }
    for key in ("file_bytes", "block_count", "largest_block_bytes",
                "max_container_depth", "reference_definition_count",
                "reference_use_count", "fenced_code_content_bytes",
                "code_occupancy", "fence_density_per_kib",
                "reference_density_per_kib"):
        out[key] = prof.get(key, NA)
    return out


# ---- raw timing ingestion ----------------------------------------------
def ingest_timing(run_dir: Path) -> dict:
    """cells[(surface,case,mech,session)] = {'prepare': [...], 'native': [...],
    'total': [...]}; pre_bytes[(surface,case)] = payload size."""
    cells: dict[tuple, dict[str, list[int]]] = defaultdict(
        lambda: {"prepare": [], "native": [], "total": []})
    pre_bytes: dict[tuple[str, str], int] = {}
    seen_obs: dict[str, int] = {}
    timing_files = sorted((run_dir / "timing").glob("*.jsonl"))
    if len(timing_files) != 6:
        raise Fail(f"expected 6 timing files, found {len(timing_files)}")
    for path in timing_files:
        with path.open() as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                obs = json.loads(line)
                check_identity(obs, path.name)
                oid = obs["observation_id"]
                seen_obs[oid] = seen_obs.get(oid, 0) + 1
                if obs["sample_kind"] != "measured":
                    continue
                row = obs["result_row_v2"]
                m = row["measurement"]["metrics"]
                key = (obs["surface"], row["case_id"], row["mechanism_id"],
                       obs["session_ordinal"])
                prep = m["prepare_ns"]
                if not isinstance(prep, int):
                    pass  # whole-cell N/A (asserted below)
                else:
                    cells[key]["prepare"].append(prep)
                cells[key]["native"].append(m["native_ns"])
                cells[key]["total"].append(m["total_ns"])
                pre_bytes[(obs["surface"], row["case_id"])] = row["payload"]["size_bytes"]
    dups = {k: v for k, v in seen_obs.items() if v > 1}
    if dups:
        raise Fail(f"duplicate observation ids in timing: {list(dups)[:3]}")
    # exactly 30 per metric per cell; prepare is either 30 ints or 0 (N/A)
    for key, parts in cells.items():
        if len(parts["native"]) != SAMPLES_PER_CELL or len(parts["total"]) != SAMPLES_PER_CELL:
            raise Fail(f"cell {key}: {len(parts['total'])} measured != 30")
        if len(parts["prepare"]) not in (0, SAMPLES_PER_CELL):
            raise Fail(f"cell {key}: partial prepare coverage {len(parts['prepare'])}")
    if len(cells) != (22 + 362) * 5 * 3:
        raise Fail(f"timing cell count {len(cells)} != 5760")
    return {"cells": cells, "pre_bytes": pre_bytes}


def summarize_cells(timing: dict) -> tuple[dict, dict]:
    """cell_summary[(surface,case,mech,session)] and
    case_est[(surface,case,mech)] with medians of 3 + unstable."""
    cell_summary = {}
    for key, parts in timing["cells"].items():
        prepare = cell_quantiles(parts["prepare"]) if parts["prepare"] else (NA, NA)
        native = cell_quantiles(parts["native"])
        total = cell_quantiles(parts["total"])
        cell_summary[key] = {
            "prepare_p50": prepare[0], "prepare_p95": prepare[1],
            "native_p50": native[0], "native_p95": native[1],
            "total_p50": total[0], "total_p95": total[1],
        }
    by_case_horse: dict[tuple, dict[int, dict]] = defaultdict(dict)
    for (surface, case, mech, session), summ in cell_summary.items():
        by_case_horse[(surface, case, mech)][session] = summ
    case_est = {}
    for key, sessions in by_case_horse.items():
        if sorted(sessions) != [0, 1, 2]:
            raise Fail(f"case/horse {key} lacks all 3 sessions")
        s0, s1, s2 = sessions[0], sessions[1], sessions[2]

        def med(field):
            return median_of_three(s0[field], s1[field], s2[field])

        unstable, _ = unstable_ratio([s0["total_p50"], s1["total_p50"], s2["total_p50"]])
        case_est[key] = {
            "prepare_p50": med("prepare_p50"), "prepare_p95": med("prepare_p95"),
            "native_p50": med("native_p50"), "native_p95": med("native_p95"),
            "total_p50": med("total_p50"), "total_p95": med("total_p95"),
            "unstable": unstable,
        }
    return cell_summary, case_est


def h0_speedups(cell_summary: dict) -> dict:
    """speedup[(surface,case,mech)] = {'ratios': [r0,r1,r2], 'geomean': float}
    H0 entries are exactly 1.0 by definition."""
    speedups = {}
    by_surface_case: dict[tuple, dict[int, int]] = defaultdict(dict)
    for (surface, case, mech, session), summ in cell_summary.items():
        by_surface_case[(surface, case, mech)][session] = summ["total_p50"]
    for (surface, case, mech), sessions in by_surface_case.items():
        if sorted(sessions) != [0, 1, 2]:
            raise Fail(f"speedup base missing sessions for {(surface, case, mech)}")
        if mech == "h0-full-rebuild":
            speedups[(surface, case, mech)] = {
                "ratios": [1.0, 1.0, 1.0], "geomean": 1.0}
            continue
        h0 = by_surface_case[(surface, case, "h0-full-rebuild")]
        ratios = []
        for s in (0, 1, 2):
            denom = sessions[s]
            if denom == 0:
                raise Fail(f"zero session p50 denominator {(surface, case, mech, s)}")
            ratios.append(h0[s] / denom)
        speedups[(surface, case, mech)] = {
            "ratios": ratios, "geomean": geometric_mean(ratios)}
    return speedups


# ---- hierarchical aggregation (mirror stats.rs project_macro) ----------
def project_macro(per_case: dict[str, float], hierarchy: dict[str, dict]) -> tuple[float, dict]:
    """Returns (macro, per_project). hierarchy[case] = {'trace','file','project'}."""
    trace_vals: dict[str, list[float]] = defaultdict(list)
    trace_file: dict[str, str] = {}
    for case in sorted(per_case):
        h = hierarchy.get(case)
        if h is None:
            raise Fail(f"case {case} missing from hierarchy index")
        trace_file[h["trace"]] = h["file"]
        trace_vals[h["trace"]].append(per_case[case])
    file_vals: dict[str, list[float]] = defaultdict(list)
    for trace in sorted(trace_vals):
        file_vals[trace_file[trace]].append(geometric_mean(trace_vals[trace]))
    proj_vals: dict[str, list[float]] = defaultdict(list)
    file_project: dict[str, str] = {}
    for case, h in hierarchy.items():
        file_project[h["file"]] = h["project"]
    for file in sorted(file_vals):
        proj_vals[file_project[file]].append(geometric_mean(file_vals[file]))
    per_project = {p: geometric_mean(vals) for p, vals in proj_vals.items()}
    macro = geometric_mean([per_project[p] for p in sorted(per_project)])
    return macro, per_project


# ---- CSV helpers --------------------------------------------------------
def write_csv(path: Path, header: list[str], rows: list[list]) -> None:
    with path.open("w", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(header)
        for row in rows:
            w.writerow(row)


def fmt(v) -> str:
    if v is None:
        return NA
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        if v == int(v) and abs(v) < 1e15:
            return str(int(v))
        return f"{v:.6f}"
    return str(v)


META_COLUMNS = [
    "payload_id", "project", "file", "trace_id", "trace_kind", "step",
    "phase", "edit_family", "position_requested", "position_actual_fraction",
    "operation_variant", "pre_bytes", "logical_edited_bytes", "edit_start",
    "edit_end", "inserted_bytes", "file_bytes", "block_count",
    "largest_block_bytes", "max_container_depth",
    "reference_definition_count", "reference_use_count",
    "fenced_code_content_bytes", "code_occupancy", "fence_density_per_kib",
    "reference_density_per_kib",
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--input-dir", required=True, type=Path,
                    help="copied raw RunId directory (analysis input snapshot)")
    ap.add_argument("--benchmark-root", required=True, type=Path)
    ap.add_argument("--out-dir", required=True, type=Path)
    args = ap.parse_args()
    run_dir = args.input_dir
    if run_dir.name != RUN_ID:
        raise Fail(f"input dir {run_dir.name} is not the frozen RunId snapshot")
    out = args.out_dir
    out.mkdir(parents=True, exist_ok=True)

    meta = load_metadata(args.benchmark_root)
    timing = ingest_timing(run_dir)
    cell_summary, case_est = summarize_cells(timing)
    speedups = h0_speedups(cell_summary)

    # per-case metadata, once per (surface, case)
    case_info: dict[tuple[str, str], dict] = {}
    for (surface, case, _mech) in {(k[0], k[1], k[2]) for k in case_est}:
        info = case_meta(meta, surface, case)
        if surface == "edit_write":
            info["pre_bytes"] = timing["pre_bytes"].get((surface, case), NA)
        case_info[(surface, case)] = info

    hierarchies: dict[str, dict[str, dict]] = {"clean_state": {}, "edit_write": {}}
    for (surface, case), info in case_info.items():
        trace_key = info["trace_id"] or f"case:{case}"
        hierarchies[surface][case] = {
            "trace": trace_key, "file": info["file"], "project": info["project"]}

    # ---------------- timing-cell-summary.csv ---------------------------
    rows = []
    for key in sorted(cell_summary):
        surface, case, mech, session = key
        s = cell_summary[key]
        rows.append([surface, case, HORSE_LABEL[mech], session,
                     fmt(s["prepare_p50"]), fmt(s["prepare_p95"]),
                     s["native_p50"], s["native_p95"],
                     s["total_p50"], s["total_p95"]])
    write_csv(out / "timing-cell-summary.csv",
              ["surface", "case_id", "horse", "session", "prepare_p50_ns",
               "prepare_p95_ns", "native_p50_ns", "native_p95_ns",
               "total_p50_ns", "total_p95_ns"], rows)

    # ---------------- case-estimates.csv --------------------------------
    rows = []
    for key in sorted(case_est):
        surface, case, mech = key
        e = case_est[key]
        info = case_info[(surface, case)]
        rows.append([surface, case, HORSE_LABEL[mech],
                     fmt(e["prepare_p50"]), fmt(e["prepare_p95"]),
                     e["native_p50"], e["native_p95"],
                     e["total_p50"], e["total_p95"],
                     fmt(e["unstable"])] + [fmt(info[c]) for c in META_COLUMNS])
    write_csv(out / "case-estimates.csv",
              ["surface", "case_id", "horse", "prepare_p50_ns", "prepare_p95_ns",
               "native_p50_ns", "native_p95_ns", "total_p50_ns", "total_p95_ns",
               "unstable"] + META_COLUMNS, rows)

    # ---------------- session-stability.csv ------------------------------
    rows = []
    for (surface, case, mech), sessions in sorted(
            {k: v for k, v in
             {(sk[0], sk[1], sk[2]): None for sk in cell_summary}.items()}.items()):
        p50s = [cell_summary[(surface, case, mech, s)]["total_p50"] for s in SESSIONS]
        unstable, ratio = unstable_ratio(p50s)
        rows.append([surface, case, HORSE_LABEL[mech], *p50s,
                     f"{ratio:.6f}", fmt(unstable)])
    write_csv(out / "session-stability.csv",
              ["surface", "case_id", "horse", "session0_p50", "session1_p50",
               "session2_p50", "max_min_ratio", "unstable"], rows)

    # ---------------- h0-relative-speedup.csv ----------------------------
    rows = []
    for key in sorted(speedups):
        surface, case, mech = key
        sp = speedups[key]
        info = case_info[(surface, case)]
        est = case_est[key]
        rows.append([surface, case, HORSE_LABEL[mech],
                     *[f"{r:.6f}" for r in sp["ratios"]],
                     f"{sp['geomean']:.6f}", fmt(est["unstable"]),
                     est["total_p50"], est["total_p95"]]
                    + [fmt(info[c]) for c in META_COLUMNS])
    write_csv(out / "h0-relative-speedup.csv",
              ["surface", "case_id", "horse", "ratio_s0", "ratio_s1", "ratio_s2",
               "speedup_geomean", "unstable", "case_total_p50_ns",
               "case_total_p95_ns"] + META_COLUMNS, rows)

    # ---------------- surface summaries ----------------------------------
    for surface, name in (("clean_state", "clean-state-summary.csv"),
                          ("edit_write", "edit-write-summary.csv")):
        rows = []
        for key in sorted(k for k in case_est if k[0] == surface):
            _, case, mech = key
            e = case_est[key]
            sp = speedups[key]
            info = case_info[(surface, case)]
            rows.append([case, HORSE_LABEL[mech],
                         fmt(e["prepare_p50"]), fmt(e["prepare_p95"]),
                         e["native_p50"], e["native_p95"],
                         e["total_p50"], e["total_p95"],
                         f"{sp['geomean']:.6f}", fmt(e["unstable"])]
                        + [fmt(info[c]) for c in META_COLUMNS])
        write_csv(out / name,
                  ["case_id", "horse", "prepare_p50_ns", "prepare_p95_ns",
                   "native_p50_ns", "native_p95_ns", "total_p50_ns",
                   "total_p95_ns", "h0_relative_speedup", "unstable"]
                  + META_COLUMNS, rows)

    # ---------------- PROJECT_MACRO / FAMILY_MACRO ------------------------
    pm_rows = []
    fm_rows = []
    fam_of_case = {(s, c): info["edit_family"]
                   for (s, c), info in case_info.items()}
    families = sorted({f for f in fam_of_case.values()})
    for surface in ("edit_write", "clean_state"):
        for horse in HORSES:
            mech = [m for m, l in HORSE_LABEL.items() if l == horse][0]
            per_case = {case: speedups[(surface, case, mech)]["geomean"]
                        for (_, case, m) in case_est
                        if _ == surface and m == mech}
            if len(per_case) == 0:
                continue
            macro, per_project = project_macro(per_case, hierarchies[surface])
            for project in sorted(per_project):
                proj_cases = [c for c in per_case
                              if hierarchies[surface][c]["project"] == project]
                pm_rows.append([surface, horse, project, len(proj_cases),
                                f"{per_project[project]:.6f}"])
            pm_rows.append([surface, horse, "PROJECT_MACRO", len(per_case),
                            f"{macro:.6f}"])
            if surface == "edit_write":
                # FAMILY_MACRO: project-macro within each family, then
                # equal family weight.
                fam_macros = {}
                for family in families:
                    sub = {c: v for c, v in per_case.items()
                           if fam_of_case[(surface, c)] == family}
                    if not sub:
                        continue
                    fmacro, _ = project_macro(sub, hierarchies[surface])
                    fam_macros[family] = fmacro
                    fm_rows.append([family, horse, len(sub), f"{fmacro:.6f}"])
                fm_rows.append(["FAMILY_MACRO", horse, len(per_case),
                                f"{geometric_mean(list(fam_macros.values())):.6f}"])
    write_csv(out / "project-macro.csv",
              ["surface", "horse", "project", "n_cases", "project_macro_speedup"],
              pm_rows)
    write_csv(out / "family-macro.csv",
              ["edit_family", "horse", "n_cases", "family_macro_speedup"], fm_rows)

    # ---------------- CASE_WEIGHTED (descriptive) -------------------------
    rows = []
    for surface in ("edit_write", "clean_state"):
        for horse in HORSES:
            mech = [m for m, l in HORSE_LABEL.items() if l == horse][0]
            ests = [case_est[(surface, c, mech)]
                    for (_, c, m) in case_est if _ == surface and m == mech]
            if not ests:
                continue
            sps = [speedups[(surface, c, mech)]["geomean"]
                   for (_, c, m) in case_est if _ == surface and m == mech]
            p50s = sorted(e["total_p50"] for e in ests)
            n = len(p50s)
            med = p50s[n // 2] if n % 2 else (p50s[n // 2 - 1] + p50s[n // 2]) / 2
            mean = math.fsum(p50s) / n
            geo = geometric_mean(sps)
            med_sp = sorted(sps)[n // 2] if n % 2 else \
                (sorted(sps)[n // 2 - 1] + sorted(sps)[n // 2]) / 2
            rows.append([surface, horse, n, med, f"{mean:.1f}",
                         f"{geo:.6f}", f"{med_sp:.6f}",
                         f"{sum(1 for v in sps if v > 1.0) / n:.6f}",
                         f"{sum(1 for v in sps if v < 1.0) / n:.6f}",
                         sum(1 for e in ests if e["unstable"])])
    write_csv(out / "case-weighted.csv",
              ["surface", "horse", "n_cases", "median_case_total_p50_ns",
               "mean_case_total_p50_ns", "geomean_case_speedup",
               "median_case_speedup", "frac_cases_faster_than_H0",
               "frac_cases_slower_than_H0", "unstable_cases"], rows)

    # ---------------- attribution join ------------------------------------
    attr_rows = []
    attr_meta_rows = []
    seen_attr: set[tuple] = set()
    for path in sorted((run_dir / "attribution").glob("*.jsonl")):
        with path.open() as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                obs = json.loads(line)
                check_identity(obs, path.name)
                row = obs["result_row_v2"]
                m = row["measurement"]["metrics"]
                surface = obs["surface"]
                case = row["case_id"]
                mech = row["mechanism_id"]
                key = (surface, case, mech)
                if key in seen_attr:
                    raise Fail(f"duplicate attribution row {key}")
                seen_attr.add(key)
                info = case_info[(surface, case)]
                edited = info["logical_edited_bytes"]
                unique_bytes = m.get("unique_source_bytes")
                inspected = m.get("source_bytes_inspected_total")
                pa = NA
                iea = NA
                if isinstance(edited, int) and edited > 0:
                    if isinstance(unique_bytes, int):
                        pa = f"{unique_bytes / edited:.6f}"
                    if isinstance(inspected, int):
                        iea = f"{inspected / edited:.6f}"

                def g(field):
                    v = m.get(field, NA)
                    return v if isinstance(v, int) else NA

                attr_rows.append([surface, case, HORSE_LABEL[mech],
                                  g("unique_old_source_intervals"),
                                  g("unique_old_source_bytes"),
                                  g("unique_post_source_intervals"),
                                  g("unique_post_source_bytes"),
                                  g("unique_source_intervals"),
                                  unique_bytes,
                                  inspected,
                                  g("blocks_reparsed"), g("nodes_rebuilt"),
                                  g("nodes_reused"),
                                  g("metadata_records_touched"),
                                  g("fallback_to_full_count"),
                                  g("restart_distance"),
                                  g("convergence_distance"),
                                  pa, iea]
                                 + [fmt(info[c]) for c in META_COLUMNS[:13]])
    write_csv(out / "attribution-joined.csv",
              ["surface", "case_id", "horse",
               "unique_old_source_intervals", "unique_old_source_bytes",
               "unique_post_source_intervals", "unique_post_source_bytes",
               "unique_source_intervals", "unique_source_bytes",
               "source_bytes_inspected_total", "blocks_reparsed",
               "nodes_rebuilt", "nodes_reused", "metadata_records_touched",
               "fallback_to_full_count", "restart_distance",
               "convergence_distance", "parse_amplification",
               "inspection_effort_amplification"] + META_COLUMNS[:13], attr_rows)

    print(f"artifacts written to {out}")
    print(f"cases: clean_state={EXPECTED_CLEAN_STATE_CASES} "
          f"edit_write={EXPECTED_EDIT_WRITE_CASES}; cells={len(cell_summary)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
