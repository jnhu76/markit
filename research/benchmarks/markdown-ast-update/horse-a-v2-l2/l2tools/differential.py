#!/usr/bin/env python3
"""Issue #100 L2 — paired differential attribution + cross-cell synthesis.

Reads the six attribution JSONs (classify.py outputs) plus the quality
JSONs and emits:

  differential.json   per cell: bucket -> {p1_samples_per_op, p0_samples_per_op,
                      excess_samples_per_op} (sign preserved, common frozen
                      sampling period => samples/op is cycle-proportional)
  differential.md     the per-cell markdown tables and the cross-cell
                      bucket x cell matrix

samples/op is the ONLY normalization used: counts / frozen region ops,
under one frozen cycles:u period, one pinned core, one repetition rule.
Percentage-of-flamegraph comparisons are deliberately NOT computed.
"""

import argparse
import json
import os

CONSTRUCTION_BUCKETS = [
    "block_parse_shared",
    "owner_materialize",
    "h0_document_materialize",
    "span_coordinate",
    "coverage",
    "certificate",
    "avl_ownerseq",
    "full_build_other",
    "h0_other",
    "h0_complete",
    "region_loop_other",
]
NON_CONSTRUCTION = ["teardown"]


def load_profile(dirpath, cell, arm, tag):
    path = os.path.join(dirpath, f"attribution_{cell}_{arm}_{tag}.json")
    with open(path) as fh:
        return json.load(fh)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--evidence", required=True, help="evidence dir with attribution_*.json")
    ap.add_argument("--cells", nargs="+", required=True)
    ap.add_argument("--tag", required=True)
    ap.add_argument("--out-json", required=True)
    ap.add_argument("--out-md", required=True)
    args = ap.parse_args()

    profiles = {}
    for cell in args.cells:
        for arm in ("P1", "P0"):
            profiles[(cell, arm)] = load_profile(args.evidence, cell, arm, args.tag)

    diff = {}
    for cell in args.cells:
        p1 = profiles[(cell, "P1")]
        p0 = profiles[(cell, "P0")]
        rows = {}
        all_buckets = set(p1["buckets"]) | set(p0["buckets"])
        for b in all_buckets:
            a = p1["buckets"].get(b, {}).get("samples_per_op", 0.0)
            c = p0["buckets"].get(b, {}).get("samples_per_op", 0.0)
            rows[b] = {
                "p1_samples_per_op": a,
                "p0_samples_per_op": c,
                "excess_samples_per_op": round(a - c, 4),
            }
        diff[cell] = {
            "p1_samples_per_op_total": p1["samples_per_op_in_region"],
            "p0_samples_per_op_total": p0["samples_per_op_in_region"],
            "excess_samples_per_op_total": round(
                p1["samples_per_op_in_region"] - p0["samples_per_op_in_region"], 4),
            "buckets": rows,
        }

    with open(args.out_json, "w") as fh:
        json.dump(diff, fh, indent=2)

    lines = []
    for cell in args.cells:
        d = diff[cell]
        lines.append(f"### {cell}\n")
        lines.append(
            f"P1 total {d['p1_samples_per_op_total']:.1f} samples/op; "
            f"P0 total {d['p0_samples_per_op_total']:.1f}; "
            f"excess {d['excess_samples_per_op_total']:.1f}\n")
        lines.append("| Context/responsibility | P1 samples/op | P0 samples/op | Excess |")
        lines.append("|---|---:|---:|---:|")
        for b in sorted(d["buckets"], key=lambda k: -d["buckets"][k]["excess_samples_per_op"]):
            r = d["buckets"][b]
            lines.append(
                f"| {b} | {r['p1_samples_per_op']:.2f} | {r['p0_samples_per_op']:.2f} "
                f"| {r['excess_samples_per_op']:+.2f} |")
        lines.append("")
    lines.append("## Cross-cell bucket x cell (excess samples/op)\n")
    lines.append("| Bucket | " + " | ".join(args.cells) + " |")
    lines.append("|---|" + "---:|" * len(args.cells))
    buckets = sorted(
        {b for c in args.cells for b in diff[c]["buckets"]},
        key=lambda b: -max(diff[c]["buckets"].get(b, {}).get("excess_samples_per_op", 0)
                           for c in args.cells))
    for b in buckets:
        lines.append(
            f"| {b} | " + " | ".join(
                f"{diff[c]['buckets'].get(b, {}).get('excess_samples_per_op', 0.0):+.2f}"
                for c in args.cells) + " |")
    lines.append("")
    with open(args.out_md, "w") as fh:
        fh.write("\n".join(lines))
    print("\n".join(lines))


if __name__ == "__main__":
    main()
