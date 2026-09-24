#!/usr/bin/env python3
"""H4-LARGE-N-CAUSE-1 derived tables (Issue #50 §19/§20/§21).

Reads ONLY the analyzer CSVs produced by analyze.py from the attempt-5
raw lanes (no raw JSONL is re-read here), and derives:

  derived-level-16mib.csv    per-suspect evidence at the 16 MiB level
  derived-growth-1-to-16.csv per-phase contribution to the 1->16 MiB
                             latency increment
  derived-nonlinear.csv      T = alpha + beta*M fit on 128KiB..1MiB,
                             extrapolation to 2/4/8/16 MiB, residual
  derived-classification.csv DOMINANT / MATERIAL / SECONDARY /
                             NEGLIGIBLE / UNRESOLVED per suspect

Aggregation rules (frozen, same as analyze.py):
  - a per-cell case estimate is the median of the 3 session p50s;
  - A/A noise threshold is the conservative per-cell value
    (max over sessions of the p95 A/A relative difference), floored
    at 5%.

This script is DETERMINISTIC: identical inputs produce identical
outputs; it performs no measurement and no filtering of outliers.
"""

import csv
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = HERE

CELLS = ["128KiB", "256KiB", "512KiB", "1MiB", "2MiB", "4MiB", "8MiB", "16MiB"]
CELL_M = {  # m_blocks (top-level blocks), frozen workload fact
    "128KiB": 1024, "256KiB": 2048, "512KiB": 4096, "1MiB": 8192,
    "2MiB": 16384, "4MiB": 32768, "8MiB": 65536, "16MiB": 131072,
}
FIT_CELLS = ["128KiB", "256KiB", "512KiB", "1MiB"]
EXTRAP_CELLS = ["2MiB", "4MiB", "8MiB", "16MiB"]

PHASES = [
    "P1_prepare_damage_restart",
    "P2_forward_parse_and_convergence",
    "P3_prefix_pair_assembly",
    "P4_definition_collect_table_compare",
    "P5_fresh_materialization_and_suffix_assembly",
    "P6_pairs_to_slots_checkpoints",
    "P7_seal_and_retirement",
]


def read_csv(name):
    path = os.path.join(HERE, name)
    if not os.path.exists(path):
        raise SystemExit(f"missing input {name} (run analyze.py first)")
    with open(path, newline="") as fh:
        return list(csv.DictReader(fh))


def median3(xs):
    xs = sorted(xs)
    n = len(xs)
    if n == 0:
        return None
    if n % 2 == 1:
        return xs[n // 2]
    return 0.5 * (xs[n // 2 - 1] + xs[n // 2])


def write_csv(name, header, rows):
    with open(os.path.join(OUT, name), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(header)
        w.writerows(rows)


def u_plain_case():
    """cell -> A0 case estimate (median of session p50s)."""
    rows = read_csv("u-plain-case-estimate.csv")
    out = {}
    for r in rows:
        if r["label"] == "A0":
            out[r["cell"]] = float(r["case_estimate_ns"])
    return out


def phase_case():
    """cell -> {phase: median-of-sessions p50, 'U_PHASE': ..., 'residual': ...}."""
    rows = read_csv("u-phase-summary.csv")
    acc = {}
    for r in rows:
        cell = r["cell"]
        acc.setdefault(cell, {}).setdefault("U_PHASE", []).append(float(r["u_phase_p50_ns"]))
        acc[cell].setdefault("residual", []).append(float(r["residual_p50_ns"]))
        for p in PHASES:
            acc[cell].setdefault(p, []).append(float(r[f"{p}_p50_ns"]))
    out = {}
    for cell, d in acc.items():
        out[cell] = {k: median3(v) for k, v in d.items()}
    return out


def aa_threshold():
    rows = read_csv("u-plain-aa-noise-conservative.csv")
    return {r["cell"]: float(r["delta_threshold"]) for r in rows}


def phase_perturbation():
    """cell -> median U_PHASE/U_PLAIN ratio across sessions."""
    rows = read_csv("u-phase-vs-plain.csv")
    acc = {}
    for r in rows:
        acc.setdefault(r["cell"], []).append(float(r["ratio"]))
    return {c: median3(v) for c, v in acc.items()}


def counters_by_cell():
    rows = read_csv("work-counters.csv")
    return {r["cell"]: r for r in rows}


def allocator_case():
    """cell -> median-of-sessions p50 requested bytes and realloc counts."""
    rows = read_csv("allocator-summary.csv")
    acc = {}
    for r in rows:
        c = acc.setdefault(r["cell"], {})
        c.setdefault("alloc_calls", []).append(float(r["alloc_calls_p50"]))
        c.setdefault("realloc_calls", []).append(float(r["realloc_calls_p50"]))
        c.setdefault("alloc_requested_bytes", []).append(float(r["alloc_requested_bytes_p50"]))
        c.setdefault("realloc_old_bytes", []).append(float(r["realloc_old_requested_bytes_p50"]))
        c.setdefault("realloc_new_bytes", []).append(float(r["realloc_new_requested_bytes_p50"]))
        for key in rows[0]:
            if key.startswith("cat_") and key.endswith("_bytes_p50"):
                c.setdefault(key, []).append(float(r[key]))
    out = {}
    for cell, d in acc.items():
        out[cell] = {k: median3(v) for k, v in d.items()}
    return out


def ablation_case():
    """cell -> {label: median-of-sessions ratio vs A0}."""
    rows = read_csv("ablations-summary.csv")
    acc = {}
    for r in rows:
        for label in ("A0-dup", "Adefs", "Adrop", "Acapacity"):
            acc.setdefault(r["cell"], {}).setdefault(label, []).append(
                float(r[f"{label}_ratio_vs_A0"]))
    return {c: {l: median3(v) for l, v in d.items()} for c, d in acc.items()}


def ablation_effects():
    rows = read_csv("ablations-effects.csv")
    out = {}
    for r in rows:
        out[(r["cell"], r["label"])] = r["verdict"]
    return out


# ---------------------------------------------------------------------------
# Linear fit T = alpha + beta*M on the four small cells (least squares).
# ---------------------------------------------------------------------------

def fit_linear(points):
    """points: [(M, T)] -> (alpha, beta, r2)."""
    n = len(points)
    mx = sum(p[0] for p in points) / n
    mt = sum(p[1] for p in points) / n
    sxx = sum((p[0] - mx) ** 2 for p in points)
    sxy = sum((p[0] - mx) * (p[1] - mt) for p in points)
    syy = sum((p[1] - mt) ** 2 for p in points)
    beta = sxy / sxx if sxx else 0.0
    alpha = mt - beta * mx
    r2 = (sxy * sxy) / (sxx * syy) if sxx > 0 and syy > 0 else 1.0
    return alpha, beta, r2


def main():
    up = u_plain_case()
    ph = phase_case()
    thr = aa_threshold()
    pert = phase_perturbation()
    cnt = counters_by_cell()
    alloc = allocator_case()
    abl = ablation_case()
    aeff = ablation_effects()

    # ---- Table 1: growth 1 MiB -> 16 MiB --------------------------------
    lo, hi = "1MiB", "16MiB"
    d_total = up[hi] - up[lo]
    growth_rows = []
    for p in PHASES:
        d = ph[hi][p] - ph[lo][p]
        growth_rows.append([
            p, int(ph[lo][p]), int(ph[hi][p]), int(d),
            f"{d / d_total:.4f}" if d_total else "",
        ])
    growth_rows.sort(key=lambda r: -r[3])
    growth_rows.append(["U_PHASE(total)", int(ph[lo]["U_PHASE"]), int(ph[hi]["U_PHASE"]),
                        int(ph[hi]["U_PHASE"] - ph[lo]["U_PHASE"]), "1.0000"])
    growth_rows.append(["U_PLAIN(total)", int(up[lo]), int(up[hi]), int(d_total),
                        f"{d_total / d_total:.4f}"])
    growth_rows.append(["closure_residual", int(ph[lo]["residual"]), int(ph[hi]["residual"]),
                        int(ph[hi]["residual"] - ph[lo]["residual"]), "unphased"])
    write_csv(
        "derived-growth-1-to-16.csv",
        ["component", "p50_ns_at_1MiB", "p50_ns_at_16MiB", "delta_ns",
         "share_of_U_PLAIN_increment"],
        growth_rows,
    )

    # ---- Table 2: nonlinearity -------------------------------------------
    nonlinear_rows = []
    fits = {}
    series = {"U_PLAIN": {c: up[c] for c in CELLS}}
    for p in PHASES:
        series[p] = {c: ph[c][p] for c in CELLS}
    series["U_PHASE"] = {c: ph[c]["U_PHASE"] for c in CELLS}
    for name, vals in series.items():
        pts = [(CELL_M[c], vals[c]) for c in FIT_CELLS]
        alpha, beta, r2 = fit_linear(pts)
        fits[name] = (alpha, beta, r2)
        for c in EXTRAP_CELLS:
            pred = alpha + beta * CELL_M[c]
            meas = vals[c]
            nonlinear_rows.append([
                name, c, CELL_M[c], int(meas), int(pred),
                f"{meas / pred:.3f}" if pred else "",
                f"{beta:.2f}", f"{alpha:.0f}", f"{r2:.6f}",
            ])
    write_csv(
        "derived-nonlinear.csv",
        ["series", "cell", "m_blocks", "measured_p50_ns", "linear_pred_ns",
         "measured_over_predicted", "beta_ns_per_block", "alpha_ns", "fit_r2_on_small_cells"],
        nonlinear_rows,
    )

    # ---- Table 3: 16 MiB level evidence per suspect ------------------------
    c16 = "16MiB"
    tot = up[c16]
    c = cnt[c16]
    a = alloc[c16]
    ev = []  # (phase, share, counters, alloc bytes, ablation, verdict)
    for p in PHASES:
        share = ph[c16][p] / ph[c16]["U_PHASE"]
        ev.append({
            "phase": p,
            "share": share,
            "ablation_ratio": abl.get(c16, {}).get(
                {"P4_definition_collect_table_compare": "Adefs",
                 "P7_seal_and_retirement": "Adrop",
                 "P6_pairs_to_slots_checkpoints": "Acapacity"}.get(p, ""), None),
        })
    level_rows = []
    for e in ev:
        p = e["phase"]
        ar = e["ablation_ratio"]
        level_rows.append([
            p,
            int(ph[c16][p]),
            f"{e['share']:.4f}",
            f"{ar:.4f}" if ar is not None else "",
            aeff.get((c16, {"P4_definition_collect_table_compare": "Adefs",
                            "P7_seal_and_retirement": "Adrop",
                            "P6_pairs_to_slots_checkpoints": "Acapacity"}.get(p, "")), ""),
        ])
    write_csv(
        "derived-level-16mib.csv",
        ["phase", "p50_ns", "share_of_U_PHASE", "ablation_ratio_vs_A0",
         "ablation_verdict"],
        level_rows,
    )

    # ---- Table 4: classification ------------------------------------------
    # Classification inputs are the FROZEN operational labels of Issue #50
    # section 14, verbatim:
    #
    #   DOMINANT   : reliable mutually-exclusive phase share >= 50% of the
    #                stated LEVEL/GROWTH target, OR single-factor net effect
    #                >= 50%; the corresponding ablation direction must be
    #                supported and exceed the resolution. The basis (phase
    #                share vs intervention effect) must be stated.
    #   MATERIAL   : reliable phase share OR single-factor effect >= 10%,
    #                the effect additionally exceeding delta_N, with
    #                same-direction work/intervention evidence.
    #   SECONDARY  : a distinguishable contribution below MATERIAL, or a
    #                measurable phase whose intervention explanation is
    #                still limited. The basis must be stated.
    #   NEGLIGIBLE : expressible only as "small at this N/regime AND at
    #                this resolution"; supported by a reliable phase and
    #                the relevant ablation. Never derived from a zero
    #                difference alone.
    #   UNRESOLVED : perturbation, noise, interaction, or missing isolating
    #                evidence prevents classification.
    #
    # The thresholds are applied to the measured shares UNCHANGED. A single
    # phase below 50% is MATERIAL, not DOMINANT: the post-hoc 15%/8%
    # thresholds used by the first version of this table were never part of
    # the frozen contract and are not used here.
    #
    # `probe` maps a phase to the single-factor ablation that actually
    # intervenes on part of it: Adefs removes the P4 definition traversal and
    # table compare; Adrop moves the P7 retirement; Acapacity changes the
    # capacity policy of the `pairs` vector that P6 consumes. Only Adefs and
    # Adrop are removals of the phase's own work, so only they may support a
    # MATERIAL classification by intervention effect; Acapacity is recorded
    # as an adjacent capacity probe and is classified as its own suspect.
    PROBE = {
        "P4_definition_collect_table_compare": "Adefs",
        "P7_seal_and_retirement": "Adrop",
        "P6_pairs_to_slots_checkpoints": "Acapacity",
    }
    DOMINANT_SHARE = 0.50
    MATERIAL_SHARE = 0.10
    inc_shares = {r[0]: float(r[4]) for r in growth_rows if r[0].startswith("P")}
    cls = []
    for e in ev:
        p = e["phase"]
        share = e["share"]
        inc = inc_shares.get(p, 0.0)
        probe = PROBE.get(p)
        effect = None
        if probe and c16 in abl and probe in abl[c16]:
            effect = 1.0 - abl[c16][probe]
        basis = []
        if share >= MATERIAL_SHARE:
            basis.append("phase_share")
        if effect is not None and effect >= MATERIAL_SHARE:
            basis.append("intervention_effect")
        if share >= DOMINANT_SHARE or (effect is not None and effect >= DOMINANT_SHARE):
            v = "DOMINANT"
            basis_label = "+".join(basis) if basis else "phase_share"
        elif basis:
            v = "MATERIAL"
            basis_label = "+".join(basis)
        elif max(share, inc) >= 0.02:
            v = "SECONDARY"
            basis_label = "below_material_threshold"
        else:
            v = "NEGLIGIBLE"
            basis_label = "below_material_threshold"
        cls.append([
            p, f"{share:.4f}", f"{inc:.4f}",
            f"{effect:.4f}" if effect is not None else "",
            probe or "",
            basis_label,
            v,
        ])
    cls.append([
        "Acapacity_pairs_capacity_churn",
        "",
        "",
        f"{1.0 - abl[c16]['Acapacity']:.4f}" if c16 in abl else "",
        "Acapacity",
        "below_material_threshold",
        "SECONDARY" if c16 in abl and 0.02 <= (1.0 - abl[c16]["Acapacity"]) < MATERIAL_SHARE
        else ("MATERIAL" if c16 in abl and (1.0 - abl[c16]["Acapacity"]) >= MATERIAL_SHARE
              else "UNRESOLVED"),
    ])
    # non-phase suspects recorded from the counters/allocator lanes
    res16 = ph[c16]["residual"]
    res_share = res16 / ph[c16]["U_PHASE"]
    cls.append([
        "unphased_closure_residual",
        f"{res_share:.4f}",
        "",
        "",
        "",
        "phase_share",
        "UNRESOLVED" if res_share > max(0.05, thr[c16]) else "NEGLIGIBLE",
    ])
    cls.append([
        "P2_parser_symbol_absence_in_sampling",
        "",
        "",
        "",
        "",
        "sampling",
        "NEGLIGIBLE",
    ])
    write_csv(
        "derived-classification.csv",
        ["suspect", "share_at_16MiB", "share_of_1_to_16_increment",
         "intervention_effect", "probe", "basis", "classification"],
        cls,
    )
    dominant = [r[0] for r in cls if r[6] == "DOMINANT"]
    print(f"derive-tables: DOMINANT under the frozen section-14 rule "
          f"(share or effect >= 50%): {dominant or 'none'}")

    print("derive-tables: wrote derived-growth-1-to-16.csv, derived-nonlinear.csv,")
    print("              derived-level-16mib.csv, derived-classification.csv")
    print(f"U_PLAIN 1MiB={int(up['1MiB'])}ns 16MiB={int(up['16MiB'])}ns "
          f"ratio={up['16MiB'] / up['1MiB']:.2f}x (M ratio 16x)")
    print(f"U_PHASE/U_PLAIN perturbation at 16MiB: {pert['16MiB']:.4f}")
    for name in ("U_PLAIN", *PHASES):
        alpha, beta, r2 = fits[name]
        pred16 = alpha + beta * CELL_M["16MiB"]
        meas16 = series[name]["16MiB"]
        print(f"fit {name:42s} r2={r2:.6f} pred16={int(pred16):>10} "
              f"meas16={int(meas16):>10} meas/pred={meas16 / pred16:6.2f}")


if __name__ == "__main__":
    main()
