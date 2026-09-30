#!/usr/bin/env python3
"""Step 2R paired V/R screening analysis (issue #104).

Reads the 5 V + 5 R paired-campaign tlane.json runs plus the frozen
V-only thresholds, extracts ARM_A medians, and emits the frozen 9-cell
decision table. Run only after ALL ten runs are complete.
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
GUARDRAILS = {
    "E6-6-FENCE-HIDE-DEF",
    "TINY-64B-E1",
    "TINY-1K-E2",
    "SENT-E5-EMPH",
    "SENT-LIST-INDENT",
    "SENT-BQ-NEST",
}
CONTEXT_ONLY = ["E6-2-WINNER-DELETE", "E6-3A-DEF-CREATE", "E6-3B-DEF-DELETE", "TINY-4K-E5"]
ARM = "HORSE_A_V1_NORMAL"


def medians(path: Path, cells=None) -> dict:
    doc = json.loads(path.read_text())
    out = {}
    for cell in doc["cells"]:
        for arm in cell["arms"]:
            if arm["arm"] == ARM:
                out[cell["cell_id"]] = arm
    return out


def stats(runs, cell):
    ms = [r[cell]["median_ns"] for r in runs]
    return {
        "run_medians_ns": ms,
        "median_ns": statistics.median(ms),
        "min_ns": min(ms),
        "max_ns": max(ms),
    }


def main() -> None:
    camp = Path(sys.argv[1])
    cal = json.loads((camp.parent / "v-only-cal" / "v_only_calibration.json").read_text())
    thresholds = {row["cell"]: row["threshold_rel"] for row in cal["rows"]}

    v_runs = [medians(camp / f"v{i}" / "tlane.json") for i in range(1, 6)]
    r_runs = [medians(camp / f"r{i}" / "tlane.json") for i in range(1, 6)]

    rows = []
    for cell in FROZEN:
        vs, rs = stats(v_runs, cell), stats(r_runs, cell)
        rel = rs["median_ns"] / vs["median_ns"] - 1.0
        thr = thresholds[cell]
        material = abs(rel) >= thr
        rows.append(
            {
                "cell": cell,
                "role": "guardrail" if cell in GUARDRAILS else "challenge",
                "v": vs,
                "r": rs,
                "relative_effect": round(rel, 5),
                "frozen_threshold": thr,
                "material": material,
                "direction": "faster" if rel < 0 else "slower",
            }
        )
        print(
            f"{cell:28s} V={vs['median_ns']:>9} R={rs['median_ns']:>9} "
            f"rel={rel*100:+7.2f}%  thr={thr*100:6.2f}%  "
            f"{'MATERIAL' if material else 'within'}  {'FASTER' if rel < 0 else 'slower'}"
        )

    context = []
    for cell in CONTEXT_ONLY:
        if cell in v_runs[0] and cell in r_runs[0]:
            vs, rs = stats(v_runs, cell), stats(r_runs, cell)
            context.append(
                {
                    "cell": cell,
                    "v_median_ns": vs["median_ns"],
                    "r_median_ns": rs["median_ns"],
                    "relative_effect": round(rs["median_ns"] / vs["median_ns"] - 1.0, 5),
                }
            )

    result = {
        "schema": "HORSE-A-V2-STEP2R-104-PAIR-CAMPAIGN-v1",
        "frozen_cells": rows,
        "context_only_cells": context,
    }
    out = camp / "pair_campaign_result.json"
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
