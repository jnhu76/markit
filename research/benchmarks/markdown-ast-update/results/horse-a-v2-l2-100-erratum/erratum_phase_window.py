#!/usr/bin/env python3
"""#100 L2 evidence erratum — warmup-window reconstruction (SENSITIVITY ONLY).

Derived-evidence tool for the 2026-09-30 L2 evidence erratum. It does NOT
re-collect anything and does NOT modify any historical artifact; it re-reads
the retained L2 `mid/` perf-script dumps, the committed attribution receipts
and the run receipts, and answers one question the original pipeline could
not: how much of each retained "in-region" sample set belongs to the WARMUP
phase versus the FORMAL region, and how do the published samples/op
quantities move under the corresponding normalizations.

Method (all three normalizations recomputed per bucket):

  A. HISTORICAL_REPORTED   retained samples / region_ops
                          (what classify.py published — mixed-phase
                          numerator, formal-only denominator)
  B. POOLED_NORMALIZED     retained samples / (region_ops + warmup_ops)
                          (valid pooled average per profiled op; assumes
                          nothing about phase distribution — every
                          retained sample IS one of those ops)
  C. WINDOW_WARMED_ONLY    samples with timestamp >= boundary, / region_ops
                          (boundary = last in-region timestamp -
                          region_wall_ns from the run receipt; the formal
                          region is the last region_wall_ns of the sampled
                          in-region window). This is an ESTIMATE whose only
                          free inference is the boundary placement; the
                          ambiguity band is quantified by shifting the
                          boundary and recounting.

Replay gate: the per-bucket counts reproduced here must equal the committed
attribution_*.json buckets EXACTLY (integer equality, all 12 profiles) —
proving this tool reads the same evidence the historical pipeline did
before any phase claim is derived from it.

Run:  python3 erratum_phase_window.py   (from this directory)
"""

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
WS = HERE.parents[1]                      # research/benchmarks/markdown-ast-update
L2 = WS / "results" / "horse-a-v2-l2-100"
L2TOOLS = WS / "horse-a-v2-l2" / "l2tools"
sys.path.insert(0, str(L2TOOLS))
import classify  # noqa: E402  (the frozen L2 classifier, reused verbatim)

# --- perf_fold.py's frozen regexes/predicates, byte-identical -------------
HEADER_RE = re.compile(r"(\S+)\s+(\d+)\s+([\d.]+):\s+\S+")
FRAME_RE = re.compile(r"^\s+([0-9a-f]+)\s+(.*?)\s+\((.*)\)\s*$")
REGION_MARKERS = ["l2_p1_build", "l2_p0_parse"]
CALLER = "l2_p1_build"
CALLEES = ["full_build", "full_parse"]


def parse_samples(path):
    """(timestamp, [(symbol, dso), ... leaf-first]) per sample, perf_fold order."""
    samples = []
    cur = None
    ts = None
    with open(path, "r", errors="replace") as fh:
        for line in fh:
            if line.startswith("#"):
                continue
            hm = HEADER_RE.match(line)
            if hm:
                if cur is not None:
                    samples.append((ts, cur))
                ts = float(hm.group(3))
                cur = []
                continue
            if cur is None:
                continue
            fm = FRAME_RE.match(line)
            if fm:
                cur.append((fm.group(2), fm.group(3)))
            elif line.strip() == "":
                samples.append((ts, cur))
                cur = None
        if cur is not None:
            samples.append((ts, cur))
    return samples


def detect_leaf_first(samples):
    """perf_fold.py's frame-order detection, verbatim logic."""
    callee_first = 0
    caller_first = 0
    for _, frames in samples:
        syms = [s for s, _ in frames]
        caller_idx = next(
            (i for i, s in enumerate(syms) if any(m in s for m in REGION_MARKERS)), None
        )
        if caller_idx is None:
            continue
        is_p1 = "l2_p1_build" in syms[caller_idx]
        callees = ["full_build"] if is_p1 else ["full_parse", "parse_full"]
        callee_idx = next(
            (i for i, s in enumerate(syms) if i != caller_idx and any(c in s for c in callees)),
            None,
        )
        if callee_idx is None:
            continue
        if callee_idx < caller_idx:
            callee_first += 1
        else:
            caller_first += 1
        if callee_first + caller_first >= 20:
            break
    if callee_first + caller_first == 0:
        return None
    return callee_first > caller_first


def in_region(syms_root_first):
    return any(m in s for s in syms_root_first for m in REGION_MARKERS)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    profiles = []
    for rep in ("rep1", "rep2"):
        for cell in ("E6-1", "E6-5", "E6-6"):
            for arm in ("P1", "P0"):
                profiles.append(f"{cell}_{arm}_{rep}")

    out = {"meta": {
        "purpose": "SENSITIVITY-ONLY warmup/phase reconstruction for the L2 evidence erratum",
        "authoritative": False,
        "retained_evidence_only": True,
        "recollected": False,
        "classifier": "horse-a-v2-l2/l2tools/classify.py (frozen, imported verbatim)",
        "in_region_predicate": "identical to perf_fold.py REGION_MARKERS containment",
    }, "profiles": {}, "replay_gate": {}, "differential": {}}

    per_profile = {}
    for name in profiles:
        mid = L2 / "campaign" / "mid" / f"perf_{name}.txt"
        samples = parse_samples(mid)
        leaf_first = detect_leaf_first(samples)

        rows = []  # (ts, root->leaf syms, bucket)
        for ts, frames in samples:
            syms = [s for s, _ in frames]
            if leaf_first:
                syms = list(reversed(syms))
            if not in_region(syms):
                continue
            bucket = classify.classify_stack(";".join(syms))
            rows.append((ts, syms, bucket))

        receipt = json.loads((L2 / "campaign" / "receipts" / f"run_{name}.json").read_text())
        reps, warmup = receipt["region_ops"], receipt["warmup_ops"]
        wall_s = receipt["region_wall_ns"] / 1e9

        # --- replay gate vs committed attribution ------------------------
        att = json.loads((L2 / "campaign" / "receipts" / f"attribution_{name}.json").read_text())
        hist_buckets = {b: d["samples"] for b, d in att["buckets"].items()}
        mine = {}
        for _, _, b in rows:
            mine[b] = mine.get(b, 0) + 1
        gate_ok = mine == hist_buckets and sum(mine.values()) == att["samples_in_region"]
        out["replay_gate"][name] = {
            "buckets_identical": gate_ok,
            "reproduced_in_region": sum(mine.values()),
            "committed_in_region": att["samples_in_region"],
        }
        if not gate_ok:
            print(f"REPLAY GATE FAILED for {name}: {mine} vs {hist_buckets}", file=sys.stderr)
            sys.exit(1)

        # --- phase-window reconstruction ---------------------------------
        ts_all = [ts for ts, _, _ in rows]
        last_ts, first_ts = max(ts_all), min(ts_all)
        boundary = last_ts - wall_s
        # boundary sensitivity: shift by these amounts and recount formal
        shifts = [0.0, 0.0005, 0.001, 0.002, -0.0005, -0.001, -0.002]

        def split_at(b):
            f = {"_total": 0}
            w = {"_total": 0}
            for ts, _, bkt in rows:
                tgt = f if ts >= b else w
                tgt["_total"] += 1
                tgt[bkt] = tgt.get(bkt, 0) + 1
            return f, w

        f_nom, w_nom = split_at(boundary)
        sens = {}
        for s in shifts:
            f_s, w_s = split_at(boundary + s)
            sens[f"{s:+.4f}s"] = {"formal_total": f_s["_total"], "warmup_total": w_s["_total"]}

        total_n = len(rows)
        a = {b: n / reps for b, n in mine.items()}
        b_pool = {b: n / (reps + warmup) for b, n in mine.items()}
        c = {b: f_nom.get(b, 0) / reps for b in mine}
        c_warm = {b: w_nom.get(b, 0) / warmup for b in mine}

        per_profile[name] = {
            "receipt": {"reps": reps, "warmup": warmup, "region_wall_ns": receipt["region_wall_ns"]},
            "samples_total_perf_script": len(samples),
            "in_region_total": total_n,
            "in_region_window_s": round(last_ts - first_ts, 6),
            "warmup_phase_duration_s": round((last_ts - first_ts) - wall_s, 6),
            "boundary_ts": boundary,
            "formal_total": f_nom["_total"],
            "warmup_total": w_nom["_total"],
            "formal_per_op_total": round(f_nom["_total"] / reps, 4),
            "warmup_per_op_total": round(w_nom["_total"] / warmup, 4) if warmup else None,
            "pooled_per_op_total_B": round(total_n / (reps + warmup), 4),
            "historical_per_op_total_A": round(total_n / reps, 4),
            "boundary_sensitivity": sens,
            "buckets": {
                b: {
                    "samples_total": mine[b],
                    "A_historical_per_op": round(a[b], 4),
                    "B_pooled_per_op": round(b_pool[b], 4),
                    "C_window_formal_per_op": round(c[b], 4),
                    "C_window_warmup_per_op": round(c_warm[b], 4),
                }
                for b in sorted(mine, key=lambda x: -mine[x])
            },
        }
        out["profiles"][name] = per_profile[name]

    # --- differential under the three normalizations ---------------------
    for cell in ("E6-1", "E6-5", "E6-6"):
        cell_out = {}
        for rep in ("rep1", "rep2"):
            p1 = per_profile[f"{cell}_P1_{rep}"]
            p0 = per_profile[f"{cell}_P0_{rep}"]
            reps = p1["receipt"]["reps"]
            norm = {
                "A_historical": ("A_historical_per_op", reps),
                "B_pooled": ("B_pooled_per_op", reps + p1["receipt"]["warmup"]),
                "C_window": ("C_window_formal_per_op", reps),
            }
            rep_out = {}
            for label, (key, _) in norm.items():
                buckets = set(p1["buckets"]) | set(p0["buckets"])
                excess, cert = 0.0, 0.0
                rows_ = {}
                for b in buckets:
                    v1 = p1["buckets"].get(b, {}).get(key, 0.0)
                    v0 = p0["buckets"].get(b, {}).get(key, 0.0)
                    d = v1 - v0
                    rows_[b] = round(d, 4)
                    excess += d
                    if b == "certificate":
                        cert = d
                rep_out[label] = {
                    "excess_total_per_op": round(excess, 4),
                    "certificate_excess_per_op": round(cert, 4),
                    "certificate_share_of_excess": round(cert / excess, 4) if excess > 0 else None,
                    "excess_rows": rows_,
                }
            cell_out[rep] = rep_out
        out["differential"][cell] = cell_out

    (HERE / "phase_window.json").write_text(json.dumps(out, indent=1) + "\n")
    print("replay gate: 12/12 OK")
    for cell in ("E6-1", "E6-5"):
        for rep in ("rep1", "rep2"):
            d = out["differential"][cell][rep]
            a = d["A_historical"]["certificate_share_of_excess"]
            b = d["B_pooled"]["certificate_share_of_excess"]
            c = d["C_window"]["certificate_share_of_excess"]
            print(f"{cell} {rep}: cert share  A={a:.4f}  B={b:.4f}  C={c:.4f}")


if __name__ == "__main__":
    main()
