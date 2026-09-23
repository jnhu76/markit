#!/usr/bin/env python3
"""Campaign-2 mechanical summarizer (evidence class DERIVED_MECHANICAL).

Deterministic tables ONLY. It reads raw JSONL and emits:

  - per (cell, horse, session) nearest-rank p50/p95
  - the case estimate (median of the three session p50s)
  - the H0-relative per-session ratio and its cross-session geometric mean
  - the stability diagnostic  max(session p50)/min(session p50) > 1.5
  - lifecycle per-step p50 tables and a DERIVED cumulative trajectory
  - attribution counter joins keyed by the same stable identities
  - controlled per-axis tables

It writes NO conclusion, ranking, or winner. Sessions are never pooled
into n=90. Raw observations are never modified.

usage: summarize.py <benchmark_root> <out_dir>
"""

from __future__ import annotations

import csv
import json
import math
import os
import sys
from collections import defaultdict

WARMUP = "warmup"
MEASURED = "measured"
P50_RANK = 15  # 1-indexed nearest rank over 30 measured values
P95_RANK = 29
STABILITY_RATIO = 1.5


def observed_number(value):
    """The frozen Observed<u64> JSON form: number | 'UNKNOWN' | 'NOT_APPLICABLE'."""
    if isinstance(value, (int, float)):
        return int(value)
    return None


def marker(value):
    if isinstance(value, (int, float)):
        return "KNOWN"
    return str(value)


def nearest_rank(sorted_values, rank):
    if not sorted_values or rank < 1 or rank > len(sorted_values):
        return None
    return sorted_values[rank - 1]


def median3(values):
    values = sorted(values)
    return values[len(values) // 2]


def geometric_mean(values):
    values = [v for v in values if v and v > 0]
    if not values:
        return None
    return math.exp(sum(math.log(v) for v in values) / len(values))


def stability_flag(session_p50s):
    known = [v for v in session_p50s.values() if v]
    if len(known) < 2:
        return ""
    lo, hi = min(known), max(known)
    if lo == 0:
        return ""
    ratio = hi / lo
    return f"{ratio:.4f}" if ratio > STABILITY_RATIO else ""


def rows(path):
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def write_csv(path, header, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(data)
    print(f"WROTE {path} rows={len(data)}")


# ---------------------------------------------------------------------------
# TIMING: construction / resident_update / controlled
# ---------------------------------------------------------------------------


def timing_table(paths_by_session, key_fn, label_header):
    """(key, horse, session) -> measured total_ns samples."""
    samples = defaultdict(list)
    for session, path in sorted(paths_by_session.items()):
        if not os.path.exists(path):
            continue
        for row in rows(path):
            if row["sample_kind"] != MEASURED:
                continue
            metrics = row["result_row_v2"]["measurement"]
            if metrics.get("lane") != "timing":
                continue
            total = observed_number(metrics["metrics"].get("total_ns"))
            if total is None:
                continue
            samples[(key_fn(row), row["horse_id"], session)].append(total)
    return samples


def per_session_p50(samples):
    """(key, horse) -> {session: p50}"""
    per_cell = defaultdict(dict)
    for (key, horse, session), values in samples.items():
        per_cell[(key, horse)][session] = nearest_rank(sorted(values), P50_RANK)
    return per_cell


def emit_timing_summary(out_path, label_header, samples, extra_columns=None):
    per_cell = per_session_p50(samples)
    extra_columns = extra_columns or {}

    by_key = defaultdict(dict)
    for (key, horse), sessions in per_cell.items():
        by_key[key][horse] = sessions

    header = [label_header, "horse", "sessions"]
    for s in (0, 1, 2):
        header += [f"session{s}_p50_ns", f"session{s}_p95_ns"]
    header += [
        "case_estimate_p50_ns",
        "h0_relative_speedup_geomean",
        "session_p50_max_over_min",
    ]
    header += extra_columns.get("columns", [])

    data = []
    for key in sorted(by_key):
        horse_map = by_key[key]
        h0_sessions = horse_map.get("H0", {})
        for horse in sorted(horse_map):
            sessions = horse_map[horse]
            row = [key, horse, ",".join(f"s{s}" for s in sorted(sessions))]
            p50s = {}
            for s in (0, 1, 2):
                values = sorted(samples.get((key, horse, s), []))
                p50 = nearest_rank(values, P50_RANK)
                p95 = nearest_rank(values, P95_RANK)
                if p50 is not None:
                    p50s[s] = p50
                row += [
                    "" if p50 is None else p50,
                    "" if p95 is None else p95,
                ]
            row.append(median3(list(p50s.values())) if p50s else "")
            ratios = []
            for s, p50 in p50s.items():
                base = h0_sessions.get(s)
                if base and p50:
                    ratios.append(base / p50)
            row.append(f"{geometric_mean(ratios):.6f}" if ratios else "")
            row.append(stability_flag(p50s))
            for column in extra_columns.get("values", []):
                row.append(extra_columns["lookup"](key, horse, column))
            data.append(row)
    write_csv(out_path, header, data)


# ---------------------------------------------------------------------------
# ATTRIBUTION
# ---------------------------------------------------------------------------

COUNTER_KEYS = [
    "blocks_reparsed",
    "nodes_rebuilt",
    "nodes_reused",
    "metadata_records_touched",
    "unique_old_source_bytes",
    "unique_post_source_bytes",
    "source_bytes_inspected_total",
    "parse_amplification_num",
    "parse_amplification_den",
    "h1_update_fallback_count",
    "h1_full_parse_fallback",
    "h4_restart_distance_bytes",
    "h4_convergence_distance_bytes",
]


def emit_attribution(out_path, label_header, paths, key_fn):
    collected = defaultdict(dict)
    for path in paths:
        if not os.path.exists(path):
            continue
        for row in rows(path):
            metrics = row["result_row_v2"]["measurement"]
            if metrics.get("lane") != "attribution":
                continue
            counters = metrics["metrics"]
            key = (key_fn(row), row["horse_id"])
            collected[key] = {k: marker(counters.get(k)) for k in counters}
    header = [label_header, "horse"] + sorted(
        {k for c in collected.values() for k in c}
    )
    data = []
    for key in sorted(collected):
        entry = collected[key]
        data.append(list(key) + [entry.get(c, "") for c in header[2:]])
    write_csv(out_path, header, data)


# ---------------------------------------------------------------------------
# LIFECYCLE
# ---------------------------------------------------------------------------


def emit_lifecycle(root, out_dir):
    step_samples = defaultdict(list)
    checkpoint_rows = defaultdict(lambda: defaultdict(dict))
    paths = [
        os.path.join(root, "results/campaign-2/lifecycle", f"session-{s}-lifecycle.jsonl")
        for s in (0, 1, 2)
    ]
    for session, path in enumerate(paths):
        if not os.path.exists(path):
            continue
        for row in rows(path):
            metrics = row["result_row_v2"]["measurement"]
            if metrics.get("lane") != "timing":
                continue
            total = observed_number(metrics["metrics"].get("total_ns"))
            if total is None:
                continue
            life = row.get("lifecycle")
            if not life:
                continue
            key = (life["trace_id"], life["family"], row["horse_id"])
            step_samples[(key, life["step"], session)].append(total)
            if life.get("checkpoint"):
                checkpoint_rows[(key, life["checkpoint"])][session] = life["step"]

    header = ["trace_id", "family", "horse", "step", "sessions", "session_p50_ns"]
    data = []
    for (key, step, session) in sorted(step_samples, key=lambda t: (t[0], t[1], t[2])):
        values = sorted(step_samples[(key, step, session)])
        data.append(
            [key[0], key[1], key[2], step, f"s{session}", nearest_rank(values, P50_RANK)]
        )
    write_csv(os.path.join(out_dir, "lifecycle-step-p50.csv"), header, data)

    # DERIVED cumulative trajectory: cumulative sum of the per-step session
    # p50s. Labelled DERIVED — never a directly measured wall-clock trace.
    cumulative = defaultdict(lambda: defaultdict(int))
    steps_seen = defaultdict(set)
    p50_at = {}
    for (key, step, session) in step_samples:
        values = sorted(step_samples[(key, step, session)])
        p50 = nearest_rank(values, P50_RANK)
        if p50 is None:
            continue
        p50_at[(key, step, session)] = p50
        steps_seen[key].add(step)
    for (key, step, session), p50 in p50_at.items():
        cumulative[(key, session)] = cumulative[(key, session)]
    out_rows = []
    for key in sorted(steps_seen):
        for session in (0, 1, 2):
            running = 0
            for step in sorted(steps_seen[key]):
                p50 = p50_at.get((key, step, session))
                if p50 is None:
                    continue
                running += p50
                out_rows.append([key[0], key[1], key[2], step, f"s{session}", running])
    write_csv(
        os.path.join(out_dir, "lifecycle-cumulative-derived-p50.csv"),
        ["trace_id", "family", "horse", "step", "session", "cumulative_derived_p50_ns"],
        out_rows,
    )

    # Checkpoint table with the cumulative derived value at K.
    checkpoint_out = []
    for key in sorted(steps_seen):
        for k in (1, 2, 4, 8, 16, 32, 64, 128):
            row = [key[0], key[1], key[2], k]
            for session in (0, 1, 2):
                running = 0
                seen = 0
                for step in sorted(steps_seen[key]):
                    if step + 1 > k:
                        break
                    p50 = p50_at.get((key, step, session))
                    if p50 is None:
                        continue
                    running += p50
                    seen += 1
                row.append(running if seen == k else "")
            checkpoint_out.append(row)
    write_csv(
        os.path.join(out_dir, "lifecycle-checkpoint-derived-p50.csv"),
        ["trace_id", "family", "horse", "K", "s0_cumulative_ns", "s1_cumulative_ns", "s2_cumulative_ns"],
        checkpoint_out,
    )


# ---------------------------------------------------------------------------


def main():
    root = sys.argv[1]
    out_dir = sys.argv[2]
    os.makedirs(out_dir, exist_ok=True)

    # ---- Surface A: construction -------------------------------------
    construction_paths = {
        s: os.path.join(root, "results/campaign-2/construction", f"session-{s}-construction.jsonl")
        for s in (0, 1, 2)
    }
    samples = timing_table(construction_paths, lambda r: r["result_row_v2"]["case_id"], "case_id")
    payload_of = {}
    for path in construction_paths.values():
        if not os.path.exists(path):
            continue
        for row in rows(path):
            payload_of[row["result_row_v2"]["case_id"]] = row["result_row_v2"]["payload"]["payload_id"]
    emit_timing_summary(
        os.path.join(out_dir, "construction-case-horse.csv"),
        "case_id",
        samples,
        {
            "columns": ["payload_id"],
            "values": ["payload_id"],
            "lookup": lambda key, horse, column: payload_of.get(key, ""),
        },
    )

    # ---- Surface B: resident_update ----------------------------------
    resident_paths = {
        s: os.path.join(root, "results/campaign-2/resident-update", f"session-{s}-resident-update.jsonl")
        for s in (0, 1, 2)
    }
    samples = timing_table(resident_paths, lambda r: r["result_row_v2"]["case_id"], "case_id")
    emit_timing_summary(
        os.path.join(out_dir, "resident-update-case-horse.csv"), "case_id", samples
    )

    # ---- Surface C: lifecycle ----------------------------------------
    emit_lifecycle(root, out_dir)

    # ---- Surface D: controlled ---------------------------------------
    axis_dirs = {
        "N": "N",
        "B": "B",
        "D": "D-fence",
        "F": "F-reference",
        "K": "K-container",
    }
    for axis, sub in axis_dirs.items():
        low = axis.lower()
        paths = {
            s: os.path.join(
                root,
                "results/campaign-2/controlled",
                sub,
                f"session-{s}-{low}-controlled.jsonl",
            )
            for s in (0, 1, 2)
        }

        def cell_key(row, axis=axis):
            cell = row.get("cell") or {}
            return f"{cell.get('cell_id', '?')}"

        samples = timing_table(paths, cell_key, "cell_id")
        extras = {}
        first = None
        for path in paths.values():
            if os.path.exists(path) and first is None:
                for row in rows(path):
                    first = row
                    break
        if first and first.get("cell"):
            cell = first["cell"]
            extras = {
                "columns": ["axis_value", "generator_version"],
                "values": ["axis_value", "generator_version"],
                "lookup": lambda key, horse, column, cell=cell: (
                    cell.get("axis_value", "") if column == "axis_value" else cell.get("generator_version", "")
                ),
            }
        emit_timing_summary(
            os.path.join(out_dir, f"controlled-{axis}-cell-horse.csv"),
            "cell_id",
            samples,
            extras,
        )

    # ---- Attribution joins -------------------------------------------
    emit_attribution(
        os.path.join(out_dir, "attribution-construction.csv"),
        "case_id",
        [os.path.join(root, "results/campaign-2/construction/attribution-construction.jsonl")],
        lambda r: r["result_row_v2"]["case_id"],
    )
    emit_attribution(
        os.path.join(out_dir, "attribution-resident-update.csv"),
        "case_id",
        [os.path.join(root, "results/campaign-2/resident-update/attribution-resident-update.jsonl")],
        lambda r: r["result_row_v2"]["case_id"],
    )
    emit_attribution(
        os.path.join(out_dir, "attribution-lifecycle.csv"),
        "trace_step",
        [
            os.path.join(
                root, "results/campaign-2/lifecycle", f"session-{s}-lifecycle-attribution.jsonl"
            )
            for s in (0, 1, 2)
        ],
        lambda r: f"{(r.get('lifecycle') or {}).get('trace_id', '?')}"
        f"#{(r.get('lifecycle') or {}).get('step', -1)}",
    )
    for axis, sub in axis_dirs.items():
        emit_attribution(
            os.path.join(out_dir, f"attribution-controlled-{axis}.csv"),
            "cell_id",
            [os.path.join(root, "results/campaign-2/controlled", sub, f"attribution-{axis}.jsonl")],
            lambda r: (r.get("cell") or {}).get("cell_id", "?"),
        )

    print("SUMMARIZE_COMPLETE")


if __name__ == "__main__":
    main()
