#!/usr/bin/env python3
"""Step 2R V-only noise calibration analysis (issue #104).

Reads the 5 V-only calibration tlane.json runs, extracts ARM_A medians per
frozen cell, computes the V-only spread and the frozen materiality
threshold per cell. V data only — never touches R numbers.
"""
import json
import statistics
import sys
from pathlib import Path

FROZEN = [
    "E6-1-LOSE-DUP-CHANGE",
    "E6-4-LOW-FANOUT-VALUE",
    "E6-5-HIGH-FANOUT-VALUE",
    "E6-6-FENCE-HIDE-DEF",
    "TINY-64B-E1",
    "TINY-1K-E2",
    "SENT-E5-EMPH",
    "SENT-LIST-INDENT",
    "SENT-BQ-NEST",
]
ARM = "HORSE_A_V1_NORMAL"


def arm_a_medians(path: Path) -> dict:
    doc = json.loads(path.read_text())
    out = {}
    for cell in doc["cells"]:
        for arm in cell["arms"]:
            if arm["arm"] == ARM:
                out[cell["cell_id"]] = arm["median_ns"]
    return out


def main() -> None:
    cal_dir = Path(sys.argv[1])
    runs = sorted(cal_dir.glob("run*/tlane.json"))
    assert len(runs) == 5, f"expected 5 V calibration runs, found {len(runs)}"
    per_run = [arm_a_medians(p) for p in runs]
    rows = []
    for cell in FROZEN:
        ms = [r[cell] for r in per_run]
        med = statistics.median(ms)
        spread = (max(ms) - min(ms)) / med
        threshold = max(2.0 * spread, 0.02)
        rows.append(
            {
                "cell": cell,
                "v_run_medians_ns": ms,
                "v_median_ns": med,
                "v_spread_rel": round(spread, 5),
                "threshold_rel": round(threshold, 5),
            }
        )
    result = {
        "schema": "HORSE-A-V2-STEP2R-104-V-ONLY-CAL-v1",
        "runs": [str(p) for p in runs],
        "rows": rows,
    }
    out = cal_dir / "v_only_calibration.json"
    out.write_text(json.dumps(result, indent=2) + "\n")
    for row in rows:
        print(
            f"{row['cell']:28s} median={row['v_median_ns']:>9} "
            f"spread={row['v_spread_rel']*100:6.2f}%  threshold={row['threshold_rel']*100:6.2f}%"
        )
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
