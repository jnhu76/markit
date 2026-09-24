#!/usr/bin/env python3
"""H4-LARGE-N-CAUSE-1 analysis: raw JSONL -> the issue's CSV deliverables.

Reads only `raw/*.jsonl` produced by the four diagnostic executables and
writes:

  u-plain.csv            per-observation U_PLAIN rows
  u-plain-summary.csv    per (session, cell, label) p50/p95 + case estimate
  u-phase.csv            per-observation phase rows + closure
  u-phase-summary.csv    per-session phase p50s, share of U_PHASE, and
                         U_PHASE / U_PLAIN per (session, cell)
  work-counters.csv      direct operation counts per cell
  ablations.csv          per-observation ablation rows
  ablations-summary.csv  per-session variant/A0 ratios
  allocator.csv          per-observation allocator windows
  allocator-summary.csv  per-session allocator p50s

No statistic is ever taken across pooled sessions: p50/p95 are per session,
and the case estimate is the median of the three session p50s.
"""
import csv
import json
import math
import os
import statistics
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "raw")
OUT = HERE

CELL_ORDER = ["128KiB", "256KiB", "512KiB", "1MiB", "2MiB", "4MiB", "8MiB", "16MiB"]
PHASES = [
    "P1_prepare_damage_restart",
    "P2_forward_parse_and_convergence",
    "P3_prefix_pair_assembly",
    "P4_definition_collect_table_compare",
    "P5_fresh_materialization_and_suffix_assembly",
    "P6_pairs_to_slots_checkpoints",
    "P7_seal_and_retirement",
]


def load(name):
    path = os.path.join(RAW, name)
    if not os.path.exists(path):
        return []
    rows = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def observations(rows):
    return [r for r in rows if r.get("record") == "observation"]


def nearest_rank(values, p):
    if not values:
        return None
    v = sorted(values)
    n = len(v)
    rank = math.ceil(p * n)
    idx = min(max(rank, 1), n) - 1
    return v[idx]


def p50_p95(values):
    return nearest_rank(values, 0.50), nearest_rank(values, 0.95)


def median3(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2]


def write_csv(name, header, rows):
    path = os.path.join(OUT, name)
    with open(path, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(header)
        w.writerows(rows)
    print(f"wrote {name}: {len(rows)} rows")


def cell_key(cell):
    return CELL_ORDER.index(cell) if cell in CELL_ORDER else 99


# ---------------------------------------------------------------------------
# U_PLAIN
# ---------------------------------------------------------------------------

def analyze_plain():
    rows = []
    for s in range(3):
        rows += observations(load(f"u-plain-session-{s}.jsonl"))
    if not rows:
        print("u-plain: no rows")
        return

    write_csv(
        "u-plain.csv",
        ["session", "round", "ordinal", "label", "cell", "n_bytes", "m_blocks",
         "case_id_hex", "sample_kind", "measured_ordinal",
         "prepare_ns", "native_ns", "total_ns",
         "execution_status", "correctness_status", "result_checksum"],
        [[r["session"], r["round"], r["ordinal"], r["label"], r["cell"], r["n_bytes"],
          r["m_blocks"], r["case_id_hex"], r["sample_kind"], r["measured_ordinal"],
          r["prepare_ns"], r["native_ns"], r["total_ns"],
          r["execution_status"], r["correctness_status"], r["result_checksum"]]
         for r in rows],
    )

    # per (session, cell, label) p50/p95 over the measured samples
    by = defaultdict(list)
    for r in rows:
        if r["sample_kind"] == "measured":
            by[(r["session"], r["cell"], r["label"])].append(r["total_ns"])
    summary = []
    for (session, cell, label), vals in sorted(
        by.items(), key=lambda kv: (kv[0][0], cell_key(kv[0][1]), kv[0][2])
    ):
        p50, p95 = p50_p95(vals)
        summary.append([session, cell, label, len(vals), p50, p95])
    write_csv("u-plain-summary.csv",
              ["session", "cell", "label", "n", "p50_ns", "p95_ns"], summary)

    # case estimate: median of the three session p50s, per (cell, label)
    by2 = defaultdict(dict)
    for session, cell, label, n, p50, p95 in summary:
        by2[(cell, label)][session] = p50
    est = []
    for (cell, label), per_session in sorted(by2.items(), key=lambda kv: (cell_key(kv[0][0]), kv[0][1])):
        if len(per_session) == 3:
            med = median3([per_session[s] for s in range(3)])
        else:
            med = None
        lo, hi = min(per_session.values()), max(per_session.values())
        spread = (hi / lo) if lo else None
        est.append([cell, label] + [per_session.get(s) for s in range(3)] + [med, spread])
    write_csv("u-plain-case-estimate.csv",
              ["cell", "label", "session0_p50_ns", "session1_p50_ns", "session2_p50_ns",
               "case_estimate_ns", "session_p50_max_over_min"], est)

    # A/A noise: p95 of the per-round absolute relative difference between
    # the two identical control labels, per session.
    rounds = defaultdict(dict)
    for r in rows:
        if r["sample_kind"] == "measured":
            rounds[(r["session"], r["cell"], r["measured_ordinal"])][r["label"]] = r["total_ns"]
    # Issue #50 §14: the empirical fluctuation threshold is defined PER N
    # AND PER SESSION -- the p95 of the per-round absolute relative
    # difference between the two identical A0 control slots. Pooling rounds
    # across N would report the spread of the whole curve, not the
    # resolution at a point. The most conservative of the three sessions is
    # used per cell.
    per_session_cell_pairs = defaultdict(list)
    for (session, cell, ordinal), labels in rounds.items():
        if "A0" in labels and "A0-dup" in labels:
            per_session_cell_pairs[(session, cell)].append((labels["A0"], labels["A0-dup"]))
    noise_rows = []
    per_cell = defaultdict(dict)
    for (session, cell), pairs in sorted(per_session_cell_pairs.items(),
                                         key=lambda kv: (kv[0][0], cell_key(kv[0][1]))):
        rel = sorted(abs(a - b) / max(a, b) for a, b in pairs if a and b)
        p95 = rel[math.ceil(0.95 * len(rel)) - 1] if rel else None
        per_cell[cell][session] = p95
        noise_rows.append([session, cell, len(rel), p95, 0.05, max(0.05, p95 or 0.0)])
    write_csv("u-plain-aa-noise.csv",
              ["session", "cell", "n_rounds", "aa_noise_p95_relative", "floor",
               "delta_threshold"], noise_rows)
    conservative = []
    for cell in sorted(per_cell, key=cell_key):
        vals = [v for v in per_cell[cell].values() if v is not None]
        noise = max(vals) if vals else None
        conservative.append([cell, per_cell[cell].get(0), per_cell[cell].get(1),
                             per_cell[cell].get(2), noise, 0.05,
                             max(0.05, noise or 0.0)])
    write_csv("u-plain-aa-noise-conservative.csv",
              ["cell", "session0_noise", "session1_noise", "session2_noise",
               "conservative_noise", "floor", "delta_threshold"], conservative)
    print("  conservative A/A noise per cell:",
          {r[0]: r[4] for r in conservative})
    print("  case estimates (ns):",
          {c: m for c, l, *_ , m, _s in [(e[0], e[1], e[2], e[3], e[4], e[5], e[6]) for e in est] if l == "A0"})


# ---------------------------------------------------------------------------
# U_PHASE
# ---------------------------------------------------------------------------

def analyze_phase():
    rows = []
    for s in range(3):
        rows += observations(load(f"u-phase-session-{s}.jsonl"))
    if not rows:
        print("u-phase: no rows")
        return

    header = (["session", "round", "ordinal", "label", "cell", "n_bytes", "m_blocks",
               "sample_kind", "measured_ordinal", "total_ns"]
              + PHASES + ["phase_disjoint_sum_ns", "phase_residual_ns",
                          "hook_inclusive_ns", "phase_clock_reads",
                          "execution_status", "correctness_status"])
    write_csv("u-phase.csv", header,
              [[r["session"], r["round"], r["ordinal"], r["label"], r["cell"], r["n_bytes"],
                r["m_blocks"], r["sample_kind"], r["measured_ordinal"], r["total_ns"]]
               + [r[p] for p in PHASES]
               + [r["phase_disjoint_sum_ns"], r["phase_residual_ns"],
                  r["hook_inclusive_ns"], r["phase_clock_reads"],
                  r["execution_status"], r["correctness_status"]]
               for r in rows])

    # per (session, cell) p50 of every phase and of U_PHASE
    by = defaultdict(list)
    for r in rows:
        if r["sample_kind"] == "measured":
            by[(r["session"], r["cell"])].append(r)
    summary = []
    for (session, cell), rs in sorted(by.items(), key=lambda kv: (kv[0][0], cell_key(kv[0][1]))):
        total_p50, _ = p50_p95([r["total_ns"] for r in rs])
        entry = [session, cell, rs[0]["n_bytes"], rs[0]["m_blocks"], len(rs), total_p50]
        for p in PHASES:
            entry.append(nearest_rank([r[p] for r in rs], 0.50))
        entry.append(nearest_rank([r["phase_disjoint_sum_ns"] for r in rs], 0.50))
        entry.append(nearest_rank([r["phase_residual_ns"] for r in rs], 0.50))
        entry.append(nearest_rank([r["hook_inclusive_ns"] for r in rs], 0.50))
        # shares of U_PHASE (per-observation p50 of the phase / p50 of total)
        for p in PHASES:
            v = nearest_rank([r[p] for r in rs], 0.50)
            entry.append(round(v / total_p50, 6) if total_p50 else None)
        summary.append(entry)
    write_csv("u-phase-summary.csv",
              ["session", "cell", "n_bytes", "m_blocks", "n", "u_phase_p50_ns"]
              + [p + "_p50_ns" for p in PHASES]
              + ["disjoint_sum_p50_ns", "residual_p50_ns", "hook_p50_ns"]
              + [p + "_share" for p in PHASES],
              summary)


def analyze_phase_vs_plain():
    """U_PHASE / U_PLAIN per (session, cell), both from their own p50s."""
    def p50_by_session_cell(files, label_filter=None):
        out = defaultdict(list)
        for s in range(3):
            for r in observations(load(files.format(s))):
                if r["sample_kind"] == "measured":
                    out[(r["session"], r["cell"])].append(r["total_ns"])
        return {k: nearest_rank(v, 0.50) for k, v in out.items()}

    plain = p50_by_session_cell("u-plain-session-{}.jsonl")
    phase = p50_by_session_cell("u-phase-session-{}.jsonl")
    rows = []
    for key in sorted(set(plain) & set(phase), key=lambda k: (k[0], cell_key(k[1]))):
        a, b = plain[key], phase[key]
        rows.append([key[0], key[1], a, b, round(b / a, 6) if a else None])
    if rows:
        write_csv("u-phase-vs-plain.csv",
                  ["session", "cell", "u_plain_p50_ns", "u_phase_p50_ns", "ratio"], rows)


# ---------------------------------------------------------------------------
# W_COUNTERS
# ---------------------------------------------------------------------------

def analyze_counters():
    rows = observations(load("work-counters.jsonl"))
    if not rows:
        print("work-counters: no rows")
        return
    rows.sort(key=lambda r: cell_key(r["cell"]))
    diag_cols = list(rows[0]["diag_counters"].keys())
    frozen_cols = list(rows[0]["frozen_work_counters"].keys())
    header = ["cell", "n_bytes", "m_blocks", "case_id_hex",
              "correctness_status"] + frozen_cols + diag_cols
    out = []
    for r in rows:
        out.append([r["cell"], r["n_bytes"], r["m_blocks"], r["case_id_hex"],
                    r["correctness_status"]]
                   + [r["frozen_work_counters"][c] for c in frozen_cols]
                   + [r["diag_counters"][c] for c in diag_cols])
    write_csv("work-counters.csv", header, out)

    # per-element normalisation: counters divided by M, to show which
    # quantities are O(1) in M and which are O(M)
    norm_header = ["cell", "m_blocks"] + [c + "_per_M" for c in diag_cols]
    norm = []
    for r in rows:
        m = r["m_blocks"]
        norm.append([r["cell"], m] + [round(r["diag_counters"][c] / m, 6) for c in diag_cols])
    write_csv("work-counters-per-block.csv", norm_header, norm)


# ---------------------------------------------------------------------------
# Ablations
# ---------------------------------------------------------------------------

def analyze_ablations():
    rows = []
    for s in range(3):
        rows += observations(load(f"ablations-session-{s}.jsonl"))
    if not rows:
        print("ablations: no rows")
        return
    write_csv("ablations.csv",
              ["session", "round", "ordinal", "label", "variant", "cell", "n_bytes",
               "m_blocks", "case_id_hex", "sample_kind", "measured_ordinal",
               "prepare_ns", "native_ns", "total_ns", "drain_ns",
               "drain_states", "drain_payloads_destroyed",
               "inventory_old_block_slots", "inventory_handles_kept_shared",
               "inventory_handles_last_reference",
               "execution_status", "correctness_status", "result_checksum"],
              [[r["session"], r["round"], r["ordinal"], r["label"], r["variant"], r["cell"],
                r["n_bytes"], r["m_blocks"], r["case_id_hex"], r["sample_kind"],
                r["measured_ordinal"], r["prepare_ns"], r["native_ns"], r["total_ns"],
                r["drain_ns"], r["drain"]["drained_states"],
                r["drain"]["payloads_destroyed"],
                r["retirement_inventory"]["old_block_slots"],
                r["retirement_inventory"]["handles_kept_shared"],
                r["retirement_inventory"]["handles_last_reference"],
                r["execution_status"], r["correctness_status"], r["result_checksum"]]
               for r in rows])

    # per (session, cell, label) p50, then per-session ratio vs A0
    by = defaultdict(list)
    for r in rows:
        if r["sample_kind"] == "measured":
            by[(r["session"], r["cell"], r["label"])].append(r["total_ns"])
    p50 = {k: nearest_rank(v, 0.50) for k, v in by.items()}
    p95 = {k: nearest_rank(v, 0.95) for k, v in by.items()}
    labels = ["A0", "A0-dup", "Adefs", "Adrop", "Acapacity"]
    out = []
    for (session, cell) in sorted({(s, c) for s, c, _ in p50},
                                  key=lambda k: (k[0], cell_key(k[1]))):
        base = p50.get((session, cell, "A0"))
        row = [session, cell]
        # All p50s first, then all p95s: the column order must match the
        # header exactly.
        for lab in labels:
            row.append(p50.get((session, cell, lab)))
        for lab in labels:
            row.append(p95.get((session, cell, lab)))
        for lab in labels:
            v = p50.get((session, cell, lab))
            row.append(round(v / base, 6) if (v and base) else None)
        out.append(row)
    write_csv("ablations-summary.csv",
              ["session", "cell"]
              + [f"{l}_p50_ns" for l in labels]
              + [f"{l}_p95_ns" for l in labels]
              + [f"{l}_ratio_vs_A0" for l in labels],
              out)

    # drain quantities per session/cell
    drain = defaultdict(list)
    for r in rows:
        if r["sample_kind"] == "measured":
            drain[(r["session"], r["cell"], r["label"])].append(r["drain_ns"])
    dout = []
    for k in sorted(drain, key=lambda k: (k[0], cell_key(k[1]), k[2])):
        dout.append([k[0], k[1], k[2], nearest_rank(drain[k], 0.50), nearest_rank(drain[k], 0.95)])
    write_csv("ablations-drain.csv",
              ["session", "cell", "label", "drain_p50_ns", "drain_p95_ns"], dout)


# ---------------------------------------------------------------------------
# Allocator
# ---------------------------------------------------------------------------

def analyze_allocator():
    rows = []
    for s in range(3):
        rows += observations(load(f"allocator-session-{s}.jsonl"))
    if not rows:
        print("allocator: no rows")
        return
    first = rows[0]["allocator_row"]
    cols = ["alloc_calls", "realloc_calls", "dealloc_calls", "alloc_requested_bytes",
            "realloc_old_requested_bytes", "realloc_new_requested_bytes",
            "freed_requested_bytes", "total_requested_bytes",
            "live_start_requested_bytes", "live_end_requested_bytes",
            "peak_live_requested_bytes", "peak_growth_requested_bytes",
            "net_live_change_requested_bytes"]
    assert len(cols) == len(first), (cols, first)
    cats = sorted(rows[0]["allocator"]["by_category"].keys())
    write_csv("allocator.csv",
              ["session", "round", "ordinal", "cell", "n_bytes", "m_blocks", "sample_kind",
               "measured_ordinal", "alloc_lane_wall_ns"] + cols
              + [f"cat_{c}_calls" for c in cats] + [f"cat_{c}_bytes" for c in cats],
              [[r["session"], r["round"], r["ordinal"], r["cell"], r["n_bytes"], r["m_blocks"],
                r["sample_kind"], r["measured_ordinal"], r["alloc_lane_wall_ns"]]
               + r["allocator_row"]
               + [r["allocator"]["by_category"][c]["calls"] for c in cats]
               + [r["allocator"]["by_category"][c]["requested_bytes"] for c in cats]
               for r in rows])

    by = defaultdict(list)
    for r in rows:
        if r["sample_kind"] == "measured":
            by[(r["session"], r["cell"])].append(r)
    out = []
    for (session, cell), rs in sorted(by.items(), key=lambda kv: (kv[0][0], cell_key(kv[0][1]))):
        idx = {c: i for i, c in enumerate(cols)}
        row = [session, cell, rs[0]["n_bytes"], rs[0]["m_blocks"], len(rs)]
        for c in cols:
            row.append(nearest_rank([r["allocator_row"][idx[c]] for r in rs], 0.50))
        for c in cats:
            row.append(nearest_rank([r["allocator"]["by_category"][c]["calls"] for r in rs], 0.50))
            row.append(nearest_rank([r["allocator"]["by_category"][c]["requested_bytes"] for r in rs], 0.50))
        out.append(row)
    write_csv("allocator-summary.csv",
              ["session", "cell", "n_bytes", "m_blocks", "n"] + [c + "_p50" for c in cols]
              + [f"cat_{c}_{k}_p50" for c in cats for k in ("calls", "bytes")],
              out)




def analyze_ablation_effects():
    """Intervention effect vs the Issue #50 §14 practical threshold.

    The threshold is `delta_N = max(5%, noise_N)` where `noise_N` is the
    conservative (largest of the three sessions) p95 of the per-round
    absolute relative difference between the two identical A0 control
    slots in the SAME lane. An intervention counts as reaching the
    threshold only when its direction is consistent across all three
    sessions AND its magnitude exceeds delta_N in every session.
    """
    import os
    noise_path = os.path.join(OUT, "u-plain-aa-noise-conservative.csv")
    if not os.path.exists(noise_path):
        print("ablation effects: run the plain analysis first")
        return
    delta = {}
    with open(noise_path) as fh:
        for row in csv.DictReader(fh):
            delta[row["cell"]] = float(row["delta_threshold"])

    rows = []
    for s in range(3):
        rows += observations(load(f"ablations-session-{s}.jsonl"))
    rows = [r for r in rows if r["sample_kind"] == "measured"]
    by = defaultdict(list)
    for r in rows:
        by[(r["session"], r["cell"], r["label"])].append(r["total_ns"])
    p50 = {k: nearest_rank(v, 0.50) for k, v in by.items()}

    out = []
    for cell in sorted({c for _, c, _ in p50}, key=cell_key):
        for lab in ["A0-dup", "Adefs", "Adrop", "Acapacity"]:
            ratios = []
            for s in range(3):
                a = p50.get((s, cell, "A0"))
                b = p50.get((s, cell, lab))
                ratios.append(b / a if (a and b) else None)
            if any(r is None for r in ratios):
                continue
            effect = [1.0 - r for r in ratios]          # positive = faster than A0
            thr = delta.get(cell)
            same_direction = all(e > 0 for e in effect) or all(e < 0 for e in effect)
            exceeds = all(abs(e) > thr for e in effect) if thr else None
            out.append([cell, lab, thr]
                       + [round(r, 6) for r in ratios]
                       + [round(e, 6) for e in effect]
                       + [same_direction, exceeds,
                          "REACHES_THRESHOLD" if (same_direction and exceeds)
                          else ("CONSISTENT_BELOW_THRESHOLD" if same_direction
                                else "INCONSISTENT")])
    write_csv("ablations-effects.csv",
              ["cell", "label", "delta_threshold"]
              + [f"ratio_session{s}" for s in range(3)]
              + [f"effect_session{s}" for s in range(3)]
              + ["direction_consistent", "exceeds_threshold_in_all_sessions", "verdict"],
              out)


def main():
    which = sys.argv[1:] or ["plain", "phase", "counters", "ablations", "allocator"]
    if "plain" in which:
        analyze_plain()
    if "phase" in which:
        analyze_phase()
        analyze_phase_vs_plain()
    if "counters" in which:
        analyze_counters()
    if "ablations" in which:
        analyze_ablations()
        analyze_ablation_effects()
    if "allocator" in which:
        analyze_allocator()


if __name__ == "__main__":
    main()
