#!/usr/bin/env python3
"""Summarize Phase A1 benchmark logs into the comparison table.

Reads the per-run logs produced by bench/run-bench.cmd on the Windows
host (results/<mvp>-<corpus>-<run>.log with an appended parse-trace
section) and prints, per corpus and MVP, the across-run median of each
latency metric (p50/p95/p99/max), plus render/frame counts.

Usage: python3 bench/summarize.py <results-dir>
"""

import glob
import os
import re
import statistics
import sys

SPAN_RE = re.compile(
    r"^\s+(\S+)\s+n=\s*(\d+)\s+p50=\s*([\d.]+)\s+p95=\s*([\d.]+)\s+p99=\s*([\d.]+)\s+max=\s*([\d.]+)$"
)
COUNT_RE = re.compile(r"^\s+(\w+):\s+(\d+)$")


def parse_log(path: str) -> dict | None:
    out: dict[str, dict] = {}
    section = None
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip()  # keep leading indent for SPAN_RE
        m = SPAN_RE.match(line)
        if m:
            name, n, p50, p95, p99, mx = m.groups()
            out.setdefault(name, {})["n"] = int(n)
            out.setdefault(name, {})["p50"] = float(p50)
            out.setdefault(name, {})["p95"] = float(p95)
            out.setdefault(name, {})["p99"] = float(p99)
            out.setdefault(name, {})["max"] = float(mx)
        elif "Trace dump" in line:
            section = "trace"
        elif section == "trace" and COUNT_RE.match(line):
            stage, n = COUNT_RE.match(line).groups()
            out.setdefault("counts", {})[stage] = int(n)
    return out or None


def median_of(field: str, logs: list[dict]) -> float:
    vals = [l.get(field, {}).get("p50") for l in logs if l and field in l]
    vals = [v for v in vals if v is not None]
    return statistics.median(vals) if vals else float("nan")


def main() -> None:
    results_dir = sys.argv[1] if len(sys.argv) > 1 else "/mnt/c/markit-a1/results"
    corpora = ["10k", "100k", "1m"]
    mvps = ["pjs", "gpui"]
    spans = ["input->edit", "edit->layout", "layout", "render"]

    print(f"results dir: {results_dir}")
    print(f"{'corpus':>6} {'mvp':>4} | " + " | ".join(f"{s:>14}" for s in spans) + " | render/frame counts")
    print("-" * 120)
    for c in corpora:
        for m in mvps:
            logs = []
            for f in sorted(glob.glob(os.path.join(results_dir, f"{m}-{c}-*.log"))):
                p = parse_log(f)
                if p:
                    logs.append(p)
            if not logs:
                print(f"{c:>6} {m:>4} | no logs")
                continue
            row = f"{c:>6} {m:>4} | "
            for s in spans:
                p50 = median_of(s, logs)
                p99 = statistics.median(
                    [l[s]["p99"] for l in logs if s in l and "p99" in l[s]]
                )
                row += f"{p50:6.1f}/{p99:6.1f} | "
            counts = [l.get("counts", {}) for l in logs if l]
            def med_count(stage):
                vals = [c.get(stage, 0) for c in counts]
                return int(statistics.median(vals)) if vals else 0
            row += f"render={med_count('render_begin')} layout={med_count('layout_begin')} edit={med_count('edit_applied')}"
            print(row)
    print()
    print("format: p50/p99 in us. gpui frame_submit unavailable (0); pjs headless also 0.")
    print("corpus sizes: 10k=10KB 100k=100KB 1m=1MB (fixed-seed ASCII)")


if __name__ == "__main__":
    main()
