#!/usr/bin/env python3
"""Parse a Markit MVP trace dump (stdout) into latency distributions.

Both MVPs print the same trace format (`+<ns> ns <stage>` lines), so one
parser serves GPUI and PocketJS. It computes per-run distributions:

  input->edit      input_received .. edit_applied   (host-side, us)
  edit->layout     edit_applied .. layout_end       (us)
  layout duration  layout_begin .. layout_end       (us)
  render duration  render_begin .. render_end       (us)
  submit delta     layout_end .. frame_submit       (us, windowed only)

Output: p50 / p95 / p99 / max / count per span, plus long-frame count
(any layout+render span > 16.7 ms).

Usage: mvp-... --smoke ... | python3 bench/parse-trace.py [--quiet]
"""

import re
import sys

STAGE_RE = re.compile(r"^\s*\+\s*(\d+) ns\s+(\w+)$")

STAGES = {
    "input_received": 0,
    "edit_applied": 1,
    "layout_begin": 2,
    "layout_end": 3,
    "render_begin": 4,
    "render_end": 5,
    "frame_submit": 6,
}


def percentile(sorted_vals: list, p: float) -> float:
    if not sorted_vals:
        return float("nan")
    k = (len(sorted_vals) - 1) * p
    lo = int(k)
    hi = min(lo + 1, len(sorted_vals) - 1)
    frac = k - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def main() -> None:
    quiet = "--quiet" in sys.argv
    events: list[tuple[int, str]] = []
    for line in sys.stdin:
        m = STAGE_RE.match(line.strip())
        if m:
            events.append((int(m.group(1)), m.group(2)))
    if not events:
        print("parse-trace: no trace events found on stdin")
        return 1

    spans: dict[str, list[float]] = {
        "input->edit": [],
        "edit->layout": [],
        "layout": [],
        "render": [],
        "layout->submit": [],
    }
    pairs: dict[str, tuple[str, str]] = {
        "input->edit": ("input_received", "edit_applied"),
        "edit->layout": ("edit_applied", "layout_end"),
        "layout": ("layout_begin", "layout_end"),
        "render": ("render_begin", "render_end"),
        "layout->submit": ("layout_end", "frame_submit"),
    }
    # last occurrence per stage (trace is ordered); each span's "a" is
    # consumed on its first "b" after it (one edit pairs with the layout
    # it caused, not with every later frame)
    last: dict[str, int] = {}
    for t, stage in events:
        for name, (a, b) in pairs.items():
            if stage == a:
                last[a] = t
            elif stage == b and a in last:
                spans[name].append((t - last[a]) / 1000.0)  # ns -> us
                last.pop(a, None)

    long_frames = sum(1 for d in spans["layout"] if d > 16666.7)  # >16.7ms
    # print per-span stats
    print("=== latency distribution (us) ===")
    for name in ("input->edit", "edit->layout", "layout", "render", "layout->submit"):
        vals = sorted(spans[name])
        if not vals:
            print(f"  {name:14s} no samples")
            continue
        print(
            f"  {name:14s} n={len(vals):5d} p50={percentile(vals, 0.50):9.1f} "
            f"p95={percentile(vals, 0.95):9.1f} p99={percentile(vals, 0.99):9.1f} "
            f"max={vals[-1]:9.1f}"
        )
    print(f"  long frames (>16.7ms layout): {long_frames}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
