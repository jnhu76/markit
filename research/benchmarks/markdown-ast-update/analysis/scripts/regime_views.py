#!/usr/bin/env python3
"""MARKIT-31 primary analysis — regime views + attribution aggregates.

Reads ONLY the Stage-A CSVs produced by primary_summary.py (no raw
input) and produces the stratified regime tables and simple descriptive
aids allowed by task §31 (binned medians, Spearman rank correlations,
rank-flip tables). Deterministic; no randomness; no network.

Bins are descriptive quantiles of the FROZEN workload population,
documented in REGIME-BINS below; they are not tuned to any result.
"""

from __future__ import annotations

import csv
import math
import sys
from collections import defaultdict
from pathlib import Path

FAMILIES = [
    "E1_LOCAL_TEXT",
    "E2_PARAGRAPH_SPLIT_MERGE",
    "E3_CONTAINER_DEPTH",
    "E4_FENCE_OPEN_CLOSE",
    "E5_INLINE_DELIMITER",
    "E6_REFERENCE_DEFINITION",
    "ATX_HEADING_TOGGLE",
]
HORSES = ["H0", "H1", "H2", "H3", "H4"]

# Descriptive bins over the frozen population (bytes):
#   file_bytes quartile-ish cuts 2000 / 4000 / 8000
#   largest_block_bytes quartile-ish cuts 300 / 800 / 2000
FILE_BINS = [(0, 2000, "<2KiB"), (2000, 4000, "2-4KiB"),
             (4000, 8000, "4-8KiB"), (8000, 10**12, ">8KiB")]
BLOCK_BINS = [(0, 300, "<300B"), (300, 800, "300-800B"),
              (800, 2000, "800B-2KiB"), (2000, 10**12, ">2KiB")]
DEPTH_BINS = [(0, 0, "0"), (1, 2, "1-2"), (3, 10**9, "3+")]


def bin_of(value: int, bins) -> str:
    for lo, hi, label in bins:
        if lo <= value <= hi:
            return label
    raise ValueError(value)


def median(values):
    s = sorted(values)
    n = len(s)
    if n == 0:
        return None
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2


def spearman(xs, ys):
    """Plain Spearman rank correlation (average ranks on ties)."""
    n = len(xs)
    if n < 3:
        return None

    def ranks(v):
        order = sorted(range(n), key=lambda i: v[i])
        r = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j + 1 < n and v[order[j + 1]] == v[order[i]]:
                j += 1
            avg = (i + j) / 2 + 1
            for k in range(i, j + 1):
                r[order[k]] = avg
            i = j + 1
        return r

    rx, ry = ranks(xs), ranks(ys)
    mx = math.fsum(rx) / n
    my = math.fsum(ry) / n
    num = math.fsum((a - mx) * (b - my) for a, b in zip(rx, ry))
    dx = math.sqrt(math.fsum((a - mx) ** 2 for a in rx))
    dy = math.sqrt(math.fsum((b - my) ** 2 for b in ry))
    if dx == 0 or dy == 0:
        return None
    return num / (dx * dy)


def fnum(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def write_csv(path: Path, header, rows):
    with path.open("w", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(header)
        w.writerows(rows)


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    est = list(csv.DictReader(open(out / "case-estimates.csv")))
    ew = [r for r in est if r["surface"] == "edit_write"]
    sp = {(r["case_id"], r["horse"]): r
          for r in csv.DictReader(open(out / "h0-relative-speedup.csv"))
          if r["surface"] == "edit_write"}
    attr = list(csv.DictReader(open(out / "attribution-joined.csv")))
    attr_by = {(r["case_id"], r["horse"]): r for r in attr
               if r["surface"] == "edit_write"}

    def one_per_case(rows):
        """case-estimates has one row per case x horse; keep as-is."""
        return rows

    # ---------------- regime-observations.csv (family x horse) -----------
    rows = []
    for family in FAMILIES:
        for horse in HORSES:
            subset = [r for r in ew if r["edit_family"] == family
                      and r["horse"] == horse]
            if not subset:
                continue
            p50s = [int(r["total_p50_ns"]) for r in subset]
            speeds = [float(sp[(r["case_id"], horse)]["speedup_geomean"])
                      for r in subset]
            pas = [fnum(attr_by[(r["case_id"], horse)]["parse_amplification"])
                   for r in subset]
            pas = [p for p in pas if p is not None]
            cov = [int(attr_by[(r["case_id"], horse)]["unique_source_bytes"])
                   for r in subset
                   if attr_by[(r["case_id"], horse)]["unique_source_bytes"] != ""]
            insp = [int(attr_by[(r["case_id"], horse)]["source_bytes_inspected_total"])
                    for r in subset
                    if attr_by[(r["case_id"], horse)]["source_bytes_inspected_total"] != ""]
            rows.append([family, horse, len(subset),
                         int(median(p50s)), f"{median(speeds):.6f}",
                         f"{median(pas):.6f}" if pas else "",
                         int(median(cov)) if cov else "",
                         int(median(insp)) if insp else ""])
    write_csv(out / "regime-observations.csv",
              ["edit_family", "horse", "n_cases", "median_case_total_p50_ns",
               "median_case_speedup", "median_parse_amplification",
               "median_unique_source_bytes", "median_inspected_bytes"], rows)

    # ---------------- file-size / block-size / depth bins -----------------
    def binned(bin_spec, field, name, path):
        rows = []
        for lo, hi, label in bin_spec:
            for horse in HORSES:
                subset = [r for r in ew if r["horse"] == horse
                          and lo <= int(r[field]) <= hi]
                if not subset:
                    continue
                p50s = [int(r["total_p50_ns"]) for r in subset]
                speeds = [float(sp[(r["case_id"], horse)]["speedup_geomean"])
                          for r in subset]
                rows.append([name, label, horse, len(subset),
                             int(median(p50s)), f"{median(speeds):.6f}"])
        write_csv(out / path,
                  ["binning", "bin", "horse", "n_cases",
                   "median_case_total_p50_ns", "median_case_speedup"], rows)

    binned(FILE_BINS, "file_bytes", "file_bytes", "regime-filesize.csv")
    binned(BLOCK_BINS, "largest_block_bytes", "largest_block_bytes",
           "regime-blocksize.csv")

    # container depth uses the per-file structural depth
    rows = []
    for lo, hi, label in DEPTH_BINS:
        for horse in HORSES:
            subset = [r for r in ew if r["horse"] == horse
                      and lo <= int(r["max_container_depth"]) <= hi]
            if not subset:
                continue
            p50s = [int(r["total_p50_ns"]) for r in subset]
            speeds = [float(sp[(r["case_id"], horse)]["speedup_geomean"])
                      for r in subset]
            rows.append(["max_container_depth", label, horse, len(subset),
                         int(median(p50s)), f"{median(speeds):.6f}"])
    write_csv(out / "regime-container-depth.csv",
              ["binning", "bin", "horse", "n_cases",
               "median_case_total_p50_ns", "median_case_speedup"], rows)

    # ---------------- per-file macro (§13 E) ------------------------------
    file_horse: dict[tuple[str, str], list[float]] = defaultdict(list)
    for (case, horse), r in sp.items():
        file_horse[(r["file"], horse)].append(float(r["speedup_geomean"]))
    rows = []
    for (file, horse) in sorted(file_horse):
        vals = file_horse[(file, horse)]
        files_meta = [r for r in ew if r["file"] == file]
        fb = files_meta[0]["file_bytes"]
        fams = sorted({r["edit_family"] for r in files_meta})
        rows.append([file, horse, len(vals), fb, "|".join(fams),
                     f"{math.exp(math.fsum(math.log(v) for v in vals) / len(vals)):.6f}"])
    write_csv(out / "file-macro.csv",
              ["file", "horse", "n_cases", "file_bytes",
               "edit_families", "file_macro_speedup"], rows)

    # ---------------- E4 fence: H4 convergence + H1 fallback --------------
    rows = []
    for r in ew:
        if r["edit_family"] != "E4_FENCE_OPEN_CLOSE":
            continue
        horse = r["horse"]
        a = attr_by[(r["case_id"], horse)]
        rows.append([r["case_id"], horse, r["phase"], int(r["total_p50_ns"]),
                     float(sp[(r["case_id"], horse)]["speedup_geomean"]),
                     a["restart_distance"], a["convergence_distance"],
                     a["fallback_to_full_count"], a["blocks_reparsed"],
                     a["source_bytes_inspected_total"]])
    write_csv(out / "regime-e4-convergence.csv",
              ["case_id", "horse", "phase", "case_total_p50_ns",
               "speedup_geomean", "restart_distance", "convergence_distance",
               "fallback_to_full_count", "blocks_reparsed",
               "source_bytes_inspected_total"], rows)
    # Spearman: H4 E4 latency vs convergence distance
    h4 = [r for r in ew if r["horse"] == "H4"
          and r["edit_family"] == "E4_FENCE_OPEN_CLOSE"]
    conv = [int(attr_by[(r["case_id"], "H4")]["convergence_distance"])
            for r in h4
            if attr_by[(r["case_id"], "H4")]["convergence_distance"] != ""]
    lat = [int(r["total_p50_ns"]) for r in h4
           if attr_by[(r["case_id"], "H4")]["convergence_distance"] != ""]
    rho = spearman(conv, lat)
    with (out / "regime-e4-convergence.csv").open("a", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow([])
        w.writerow(["# H4 E4: n=", len(conv), "spearman(lat,convergence)=",
                    f"{rho:.6f}" if rho is not None else "UNDEFINED"])

    # ---------------- E6 reference detail ---------------------------------
    rows = []
    for family_case in {r["case_id"] for r in ew
                        if r["edit_family"] == "E6_REFERENCE_DEFINITION"}:
        subset = sorted((r for r in ew if r["case_id"] == family_case),
                        key=lambda r: r["horse"])
        info = subset[0]
        row = [family_case, info["file"], info["file_bytes"],
               info["reference_definition_count"], info["reference_use_count"],
               info["phase"], info["logical_edited_bytes"]]
        by_horse = {r["horse"]: r for r in subset}
        for horse in HORSES:
            r = by_horse.get(horse)
            row.append(int(r["total_p50_ns"]) if r else "")
        for horse in ("H1", "H2", "H3", "H4"):
            r = by_horse.get(horse)
            row.append(f"{float(sp[(family_case, horse)]['speedup_geomean']):.6f}"
                       if r else "")
        for horse in HORSES:
            a = attr_by.get((family_case, horse))
            row.append(a["nodes_reused"] if a and a["nodes_reused"] != "" else "")
        rows.append(row)
    rows.sort(key=lambda r: r[0])
    write_csv(out / "regime-e6-reference.csv",
              ["case_id", "file", "file_bytes", "reference_definition_count",
               "reference_use_count", "phase", "logical_edited_bytes"]
              + [f"total_p50_{h}" for h in HORSES]
              + [f"speedup_{h}" for h in ("H1", "H2", "H3", "H4")]
              + [f"nodes_reused_{h}" for h in HORSES], rows)

    # ---------------- attribution-by-horse-family (MQ3/MQ5) ---------------
    rows = []
    for horse in HORSES:
        for family in FAMILIES + ["ALL"]:
            subset = [r for r in attr if r["surface"] == "edit_write"
                      and r["horse"] == horse
                      and (family == "ALL" or r["edit_family"] == family)]
            if not subset:
                continue

            def med(field, cast=float):
                vals = [cast(r[field]) for r in subset if r[field] != ""]
                return median(vals) if vals else None

            fb = [r for r in subset if r["fallback_to_full_count"] != ""]
            fb_sum = sum(int(r["fallback_to_full_count"]) for r in fb)
            rows.append([horse, family, len(subset),
                         med("unique_source_bytes", int) or "",
                         med("source_bytes_inspected_total", int) or "",
                         med("parse_amplification") if med("parse_amplification") is not None else "",
                         med("inspection_effort_amplification") if med("inspection_effort_amplification") is not None else "",
                         med("blocks_reparsed", int) if med("blocks_reparsed", int) is not None else "",
                         med("nodes_rebuilt", int) if med("nodes_rebuilt", int) is not None else "",
                         med("nodes_reused", int) if med("nodes_reused", int) is not None else "",
                         med("metadata_records_touched", int) if med("metadata_records_touched", int) is not None else "",
                         (f"{fb_sum / len(fb):.6f}" if fb else ""),
                         (int(med("restart_distance", int)) if med("restart_distance", int) is not None else ""),
                         (int(med("convergence_distance", int)) if med("convergence_distance", int) is not None else "")])
    write_csv(out / "attribution-by-horse-family.csv",
              ["horse", "edit_family", "n_cases",
               "median_unique_source_bytes", "median_inspected_bytes",
               "median_parse_amplification",
               "median_inspection_effort_amplification",
               "median_blocks_reparsed", "median_nodes_rebuilt",
               "median_nodes_reused", "median_metadata_records_touched",
               "fallback_mean_per_case", "median_restart_distance",
               "median_convergence_distance"], rows)

    # ---------------- DQ6 small-file view ---------------------------------
    rows = []
    for lo, hi, label in FILE_BINS:
        for horse in ("H1", "H2", "H3", "H4"):
            subset = [r for r in ew if r["horse"] == horse
                      and lo <= int(r["file_bytes"]) <= hi]
            speeds = [float(sp[(r["case_id"], horse)]["speedup_geomean"])
                      for r in subset]
            if not speeds:
                continue
            frac_slower = sum(1 for v in speeds if v < 1.0) / len(speeds)
            rows.append([label, horse, len(subset), f"{median(speeds):.6f}",
                         f"{frac_slower:.6f}"])
    write_csv(out / "regime-smallfile.csv",
              ["file_bin", "horse", "n_cases", "median_case_speedup",
               "frac_cases_slower_than_H0"], rows)

    print("regime views written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
