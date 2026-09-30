#!/usr/bin/env python3
"""#100 L2 evidence erratum — mechanical warmup-inclusion audit (AUDIT ONLY).

Emits warmup_audit.json: the mechanically checkable facts behind erratum
item E1 (warmup inclusion / denominator defect), each tied to a source file
and SHA-256 so a reviewer can verify every claim against the exact bytes:

  1. the profiling harness runs warmup through the SAME marker functions
     the formal region uses (l2_p1_build / l2_p0_parse);
  2. perf record launches the process, so the sampling window covers
     warmup from process start (first samples sit in ld.so);
  3. perf_fold.py retains samples by marker-frame containment only —
     the #[inline(never)] profile_region frame appears in ZERO retained
     samples (all 12 mid + 12 folded files), so no call-path phase
     separation exists in the retained evidence;
  4. classify.py divides retained samples by region_ops (formal-only),
     so the published samples/op numerator may contain warmup samples
     while the denominator counts only formal operations.

Run:  python3 make_warmup_audit.py   (from this directory)
"""

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
WS = HERE.parents[1]
L2 = WS / "results" / "horse-a-v2-l2-100"
L2CRATE = WS / "horse-a-v2-l2"


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def count_marker(path, needle):
    n = 0
    with open(path, "r", errors="replace") as fh:
        for line in fh:
            n += line.count(needle)
    return n


def main():
    audit = {
        "artifact": "warmup_audit.json",
        "purpose": "mechanical evidence for erratum E1 (warmup inclusion / denominator defect)",
        "authority": "SENSITIVITY / AUDIT ONLY — corrects interpretation; rewrites nothing",
        "historical_evidence_modified": False,
        "source_authorities": {
            "harness": {
                "path": "horse-a-v2-l2/src/bin/mdbench-horse-a-v2-l2.rs",
                "sha256": sha256(L2CRATE / "src/bin/mdbench-horse-a-v2-l2.rs"),
                "warmup_calls_marker_fns": "lines 101-104: match arm { P1 => l2_p1_build(.., warmup), P0 => l2_p0_parse(.., warmup) } run from main BEFORE profile_region",
                "formal_calls_marker_fns": "lines 123-129: #[inline(never)] profile_region -> l2_p1_build/l2_p0_parse(reps)",
                "design_intent_comment": "harness doc claims 'the folding stage detects frame order and filters setup samples by these names' — the implemented filter cannot do this (see perf_fold)",
            },
            "collect_profile": {
                "path": "horse-a-v2-l2/l2tools/collect_profile.sh",
                "sha256": sha256(L2CRATE / "l2tools/collect_profile.sh"),
                "perf_record_launches_process": "line 26: taskset -c 2 perf record ... -- \"$BIN\" --cell ... --warmup \"$WARMUP\" — sampling starts at exec, covering startup AND warmup",
            },
            "perf_fold": {
                "path": "horse-a-v2-l2/l2tools/perf_fold.py",
                "sha256": sha256(L2CRATE / "l2tools/perf_fold.py"),
                "region_markers": "REGION_MARKERS = ['l2_p1_build', 'l2_p0_parse'] (line 32)",
                "retention_predicate": "line 121: any(m in s for s in syms for m in REGION_MARKERS) — call-path containment of the SAME functions warmup uses; no profile_region requirement, no time/phase filter",
            },
            "classify": {
                "path": "horse-a-v2-l2/l2tools/classify.py",
                "sha256": sha256(L2CRATE / "l2tools/classify.py"),
                "denominator": "samples_per_op = samples / args.reps (lines 131, 138); args.reps = region_ops only — warmup ops never enter the denominator",
            },
        },
        "profile_region_frame_occurrences": {},
        "profiles": {},
        "operation_counts": {},
    }

    profiles = [f"{c}_{a}_{r}" for r in ("rep1", "rep2") for c in ("E6-1", "E6-5", "E6-6") for a in ("P1", "P0")]

    for name in profiles:
        mid = L2 / "campaign" / "mid" / f"perf_{name}.txt"
        folded = L2 / "campaign" / "folded" / f"perf_{name}.folded"
        att = L2 / "campaign" / "receipts" / f"attribution_{name}.json"
        run = L2 / "campaign" / "receipts" / f"run_{name}.json"
        quality = L2 / "campaign" / "receipts" / f"quality_{name}.json"
        audit["profile_region_frame_occurrences"][f"mid/perf_{name}.txt"] = count_marker(mid, "profile_region")
        audit["profile_region_frame_occurrences"][f"folded/perf_{name}.folded"] = count_marker(folded, "profile_region")
        a = json.loads(att.read_text())
        r = json.loads(run.read_text())
        q = json.loads(quality.read_text())
        audit["profiles"][name] = {
            "mid_sha256": sha256(mid),
            "attribution_sha256": sha256(att),
            "run_sha256": sha256(run),
            "quality_sha256": sha256(quality),
            "run_receipt": {"warmup_ops": r["warmup_ops"], "region_ops": r["region_ops"], "region_wall_ns": r["region_wall_ns"]},
            "attribution_in_region": a["samples_in_region"],
            "attribution_reps_denominator": a["reps"],
            "quality_samples_total": q["samples_total"],
        }

    audit["operation_counts"] = {
        "E6-1": "20 warmup + 100 formal (identical within the P1/P0 pair)",
        "E6-5": "20 warmup + 100 formal (identical within the P1/P0 pair)",
        "E6-6": "30 warmup + 1500 formal (identical within the P1/P0 pair)",
    }
    audit["conclusion"] = {
        "warmup_inside_perf_window": True,
        "fold_filter_distinguishes_phase": False,
        "historical_denominator": "region_ops (formal only)",
        "historical_numerator": "all retained marker-containing samples (warmup + formal, not separable by call path in retained evidence)",
        "true_warmed_only_strictly_recoverable": "NO — no recorded phase boundary; the timestamp+wall-duration reconstruction in phase_window.json is an ESTIMATE (sensitivity-only)",
    }

    (HERE / "warmup_audit.json").write_text(json.dumps(audit, indent=1) + "\n")
    nonzero = {k: v for k, v in audit["profile_region_frame_occurrences"].items() if v}
    print(f"profile_region occurrences nonzero entries: {nonzero if nonzero else 'NONE (24/24 files zero)'}")
    print("warmup_audit.json written")


if __name__ == "__main__":
    main()
