#!/usr/bin/env python3
"""MARKIT-31 primary analysis — figures from frozen derived CSVs (task §30).

Deterministic; matplotlib Agg; no randomness (fixed seed not needed —
no sampling). Log axes are always labeled. Every plot keeps raw case /
project structure visible (scatters or per-bin bars, never macro-only).
"""

from __future__ import annotations

import csv
import math
import sys
from collections import defaultdict
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

HORSES = ["H0", "H1", "H2", "H3", "H4"]
COLORS = {"H0": "#444444", "H1": "#1f77b4", "H2": "#2ca02c",
          "H3": "#ff7f0e", "H4": "#d62728"}
PROJECTS = ["crafting-interpreters", "d2l-en", "kubernetes-keps",
            "myst-parser", "oci-image", "openmlsys", "opentelemetry-spec",
            "owasp-cheatsheets", "rust-book", "rust-rfcs"]
PMARK = {p: m for p, m in zip(PROJECTS, "ovDspPX*h8d")}


def read(out: Path, name: str):
    return list(csv.DictReader(open(out / name)))


def logx(ax, label):
    ax.set_xscale("log")
    ax.set_xlabel(label + " (log scale)")


def main() -> int:
    out = Path(sys.argv[1])
    figs = out / "figures"
    figs.mkdir(exist_ok=True)

    ew = read(out, "edit-write-summary.csv")
    sp = {(r["case_id"], r["horse"]): r for r in read(out, "h0-relative-speedup.csv")
          if r["surface"] == "edit_write"}
    attr = {(r["case_id"], r["horse"]): r for r in read(out, "attribution-joined.csv")
            if r["surface"] == "edit_write"}
    case_info = {r["case_id"]: r for r in ew}

    # 1 — PROJECT_MACRO + FAMILY_MACRO bars -------------------------------
    pm = [r for r in read(out, "project-macro.csv") if r["project"] == "PROJECT_MACRO"
          and r["surface"] == "edit_write"]
    fm = [r for r in read(out, "family-macro.csv") if r["edit_family"] == "FAMILY_MACRO"]
    fig, axes = plt.subplots(1, 2, figsize=(11, 4))
    for ax, rows, title in ((axes[0], pm, "PROJECT_MACRO (edit_write, equal-weight projects)"),
                            (axes[1], fm, "FAMILY_MACRO (edit_write, equal-weight families)")):
        vals = [float(next(r["project_macro_speedup"] if "project_macro_speedup" in r
                           else r["family_macro_speedup"] for r in rows if r["horse"] == h))
                for h in HORSES]
        ax.bar(HORSES, vals, color=[COLORS[h] for h in HORSES])
        ax.axhline(1.0, color="k", lw=1)
        ax.set_title(title, fontsize=10)
        ax.set_ylabel("H0-relative speedup (>1 = faster than H0)")
        for i, v in enumerate(vals):
            ax.text(i, v, f"{v:.2f}", ha="center", va="bottom", fontsize=9)
    fig.tight_layout()
    fig.savefig(figs / "fig-macro-overview.png", dpi=150)
    plt.close(fig)

    # 2 — family x horse grouped bars --------------------------------------
    fam_rows = read(out, "family-macro.csv")
    fams = [r["edit_family"] for r in fam_rows
            if r["horse"] == "H1" and r["edit_family"] != "FAMILY_MACRO"]
    fig, ax = plt.subplots(figsize=(11, 4.2))
    width = 0.16
    for k, h in enumerate(HORSES):
        vals = [float(next(r["family_macro_speedup"] for r in fam_rows
                           if r["horse"] == h and r["edit_family"] == f))
                for f in fams]
        xs = [i + (k - 2) * width for i in range(len(fams))]
        ax.bar(xs, vals, width=width, label=h, color=COLORS[h])
    ax.axhline(1.0, color="k", lw=1)
    ax.set_xticks(range(len(fams)))
    ax.set_xticklabels([f.replace("_", "\n") for f in fams], fontsize=8)
    ax.set_ylabel("family-macro H0-relative speedup")
    ax.set_title("EDIT_WRITE speedup by edit family (equal-weight projects within family)")
    ax.legend(ncol=5)
    fig.tight_layout()
    fig.savefig(figs / "fig-family-macro.png", dpi=150)
    plt.close(fig)

    # 3 — latency vs file bytes scatter (per project markers) --------------
    fig, ax = plt.subplots(figsize=(8, 5.5))
    for h in HORSES:
        xs = [int(r["file_bytes"]) for r in ew if r["horse"] == h]
        ys = [int(r["total_p50_ns"]) for r in ew if r["horse"] == h]
        ps = [r["project"] for r in ew if r["horse"] == h]
        for x, y, p in zip(xs, ys, ps):
            ax.scatter(x, y, s=14, color=COLORS[h], marker=PMARK[p],
                       alpha=0.65, linewidths=0.3)
    logx(ax, "file bytes")
    ax.set_yscale("log")
    ax.set_ylabel("case T_total p50 (ns, log scale)")
    ax.set_title("EDIT_WRITE latency vs file size (markers = project)")
    handles = [plt.Line2D([], [], color=COLORS[h], marker="o", ls="",
                          label=h) for h in HORSES]
    ax.legend(handles=handles, ncol=5)
    fig.tight_layout()
    fig.savefig(figs / "fig-latency-vs-filebytes.png", dpi=150)
    plt.close(fig)

    # 4 — speedup vs file-size bins ----------------------------------------
    fs = read(out, "regime-filesize.csv")
    bins = ["<2KiB", "2-4KiB", "4-8KiB", ">8KiB"]
    fig, ax = plt.subplots(figsize=(8, 4.5))
    for h in HORSES:
        if h == "H0":
            continue
        vals = [float(next(r["median_case_speedup"] for r in fs
                           if r["horse"] == h and r["bin"] == b)) for b in bins]
        ax.plot(bins, vals, marker="o", label=h, color=COLORS[h])
    ax.axhline(1.0, color="k", lw=1)
    ax.set_ylabel("median case speedup vs H0")
    ax.set_title("Speedup vs file-size bin (descriptive quartile bins)")
    ax.legend(ncol=4)
    fig.tight_layout()
    fig.savefig(figs / "fig-speedup-vs-filesize.png", dpi=150)
    plt.close(fig)

    # 5 — H4 latency vs convergence distance (E4 + E6 + others) ------------
    fig, ax = plt.subplots(figsize=(8, 5.5))
    fam_color = {"E4_FENCE_OPEN_CLOSE": "#d62728",
                 "E6_REFERENCE_DEFINITION": "#9467bd"}
    xs, ys, cs = [], [], []
    for r in ew:
        if r["horse"] != "H4":
            continue
        a = attr[(r["case_id"], "H4")]
        if a["convergence_distance"] == "":
            continue
        x = int(a["convergence_distance"])
        y = int(r["total_p50_ns"])
        f = r["edit_family"]
        c = fam_color.get(f, "#7f7f7f")
        ax.scatter(x, y, s=16, color=c, alpha=0.7, linewidths=0.3)
    logx(ax, "H4 convergence distance (bytes, log scale)")
    ax.set_yscale("log")
    ax.set_ylabel("case T_total p50 (ns, log scale)")
    ax.set_title("H4: latency vs convergence distance\n(red=E4 fence, purple=E6 reference, grey=others)")
    fig.tight_layout()
    fig.savefig(figs / "fig-h4-convergence.png", dpi=150)
    plt.close(fig)

    # 6 — container depth bins ---------------------------------------------
    cd = read(out, "regime-container-depth.csv")
    bins = ["0", "1-2", "3+"]
    fig, ax = plt.subplots(figsize=(8, 4.5))
    for h in HORSES:
        if h == "H0":
            continue
        vals = [float(next(r["median_case_speedup"] for r in cd
                           if r["horse"] == h and r["bin"] == b)) for b in bins]
        ax.plot(bins, vals, marker="o", label=h, color=COLORS[h])
    ax.axhline(1.0, color="k", lw=1)
    ax.set_ylabel("median case speedup vs H0")
    ax.set_xlabel("max container depth bin")
    ax.set_title("Deep-container edits flip H2/H3/H4 below H0 (n=45 cases at depth 3+)")
    ax.legend(ncol=4)
    fig.tight_layout()
    fig.savefig(figs / "fig-container-depth.png", dpi=150)
    plt.close(fig)

    # 7 — clean state: horse cost vs file bytes ----------------------------
    cs = [r for r in read(out, "case-estimates.csv") if r["surface"] == "clean_state"]
    fig, ax = plt.subplots(figsize=(8, 5))
    for h in HORSES:
        xs = [int(r["file_bytes"]) for r in cs if r["horse"] == h]
        ys = [int(r["total_p50_ns"]) for r in cs if r["horse"] == h]
        ax.scatter(xs, ys, s=26, color=COLORS[h], label=h, alpha=0.8)
    logx(ax, "file bytes")
    ax.set_yscale("log")
    ax.set_ylabel("T_total p50 (ns, log scale) = clean parse + native-state build")
    ax.set_title("CLEAN_STATE: all incremental horses cost MORE than H0 full parse")
    ax.legend(ncol=5)
    fig.tight_layout()
    fig.savefig(figs / "fig-clean-state.png", dpi=150)
    plt.close(fig)

    # 8 — H1 fallback rate + speedup by family -----------------------------
    ah = read(out, "attribution-by-horse-family.csv")
    fams = [r["edit_family"] for r in ah if r["horse"] == "H1"
            and r["edit_family"] != "ALL"]
    fb = [float(next(r["fallback_mean_per_case"] for r in ah
                     if r["horse"] == "H1" and r["edit_family"] == f))
          for f in fams]
    fm_rows = read(out, "family-macro.csv")
    h1sp = [float(next(r["family_macro_speedup"] for r in fm_rows
                       if r["horse"] == "H1" and r["edit_family"] == f))
            for f in fams]
    fig, ax1 = plt.subplots(figsize=(10, 4.2))
    xs = range(len(fams))
    ax1.bar([x - 0.2 for x in xs], fb, width=0.4, color="#1f77b4",
            label="H1 fallback events per case")
    ax1.set_xticks(list(xs))
    ax1.set_xticklabels([f.replace("_", "\n") for f in fams], fontsize=8)
    ax1.set_ylabel("H1 fallback / case")
    ax2 = ax1.twinx()
    ax2.bar([x + 0.2 for x in xs], h1sp, width=0.4, color="#999999",
            label="H1 family-macro speedup")
    ax2.axhline(1.0, color="k", lw=1)
    ax2.set_ylabel("H1 speedup (grey)")
    ax1.set_title("H1: fallback frequency vs speedup by family")
    fig.tight_layout()
    fig.savefig(figs / "fig-h1-fallback.png", dpi=150)
    plt.close(fig)

    # 9 — E6 per-case speedups by horse ------------------------------------
    e6 = read(out, "regime-e6-reference.csv")
    fig, ax = plt.subplots(figsize=(8, 4.5))
    for k, h in enumerate(("H1", "H2", "H3", "H4")):
        vals = [float(r[f"speedup_{h}"]) for r in e6]
        xs = [i + (k - 1.5) * 0.18 for i in range(len(vals))]
        ax.scatter(xs, vals, s=30, color=COLORS[h], label=h, alpha=0.8)
    med = [sorted(float(r[f"speedup_{h}"]) for r in e6)[len(e6) // 2]
           for h in ("H1", "H2", "H3", "H4")]
    for k, m in enumerate(med):
        ax.hlines(m, k - 1.5 - 0.3, k - 1.5 + 0.3, color=COLORS[
            ["H1", "H2", "H3", "H4"][k]], lw=2)
    ax.axhline(1.0, color="k", lw=1)
    ax.set_xticks(range(len(e6)))
    ax.set_xticklabels([r["file"].split("/")[0][:12] for r in e6], fontsize=7,
                       rotation=30)
    ax.set_ylabel("case speedup vs H0 (bars = median)")
    ax.set_title(f"E6 REFERENCE_DEFINITION: every incremental horse loses to H0 (n={len(e6)} cases)")
    ax.legend(ncol=4)
    fig.tight_layout()
    fig.savefig(figs / "fig-e6-reference.png", dpi=150)
    plt.close(fig)

    # 10 — PA vs latency ----------------------------------------------------
    fig, ax = plt.subplots(figsize=(8, 5.5))
    for h in HORSES:
        xs, ys = [], []
        for r in ew:
            if r["horse"] != h:
                continue
            pa = attr[(r["case_id"], h)]["parse_amplification"]
            if pa == "":
                continue
            xs.append(float(pa))
            ys.append(int(r["total_p50_ns"]))
        ax.scatter(xs, ys, s=14, color=COLORS[h], alpha=0.6, label=h,
                   linewidths=0.3)
    logx(ax, "Parse Amplification (unique bytes inspected / edited bytes, log)")
    ax.set_yscale("log")
    ax.set_ylabel("case T_total p50 (ns, log)")
    ax.set_title("Work-inspection amplification vs latency (per case)")
    ax.legend(ncol=5)
    fig.tight_layout()
    fig.savefig(figs / "fig-pa-vs-latency.png", dpi=150)
    plt.close(fig)

    print(f"figures written to {figs}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
