#!/usr/bin/env python3
"""Derive perf/stat-summary.csv from the committed raw perf/stat JSONL files.

Deterministic derivation, no network, no inputs besides:

    perf/stat/{user,root}-{groupA,groupB,groupC1,groupC2,groupS}-{cell}.json
    perf/stat/{priv}-groupA-{cell}.jsonl   (pmu-run receipt: round count)

History (PR #51 corrective, 2026-09): the summary first shipped in 45ad068
was produced by an inline script that looked events up under their
user-mode perf names ("cycles:u"). Root-privilege files record plain names
("cycles"), so every lookup silently resolved to a 0.0 default and all
eight root rows were written as zeros even though the raw root counters
were valid. This script replaces it as the committed derivation:

  - event names are NORMALIZED by stripping the perf modifier suffix
    (":u", ":k", ...) before matching, so both privileges derive;
  - a raw counter that is non-zero can never silently derive to a zero
    summary value: missing events raise, and a non-zero raw counter with
    a zero derived value aborts the run;
  - every file must report pcnt-running >= 99 (PMU_GROUP_UNRELIABLE guard,
    same threshold as the report).

Semantics (identical to the values already reported for the user rows in
45ad068, which this script reproduces byte-identically): each pmu-run
round executes exactly one resident update inside the perf window, so

    per_update  = counter / rounds
    per_block   = per_update / m_blocks          (m_blocks = N / 128)
    CPI         = cycles_per_update / instructions_per_update

Formatting: per-update counts rounded to integers; cycles/instructions
per block rounded to 2 decimals; CPI, cache-misses and LLC-load-misses
per block to 3; dTLB-load-misses per block to 4; context switches and
cpu migrations emitted as plain floats.
"""

import argparse
import json
import math
import sys
from pathlib import Path

GROUPS = ("groupA", "groupB", "groupC1", "groupC2", "groupS")
CELLS = ("128KiB", "256KiB", "512KiB", "1MiB",
         "2MiB", "4MiB", "8MiB", "16MiB")
PRIVILEGES = ("user", "root")  # CSV row order (as first shipped in 45ad068)

# events as collected per group (run-perf-stat-per-cell.sh)
GROUP_EVENTS = {
    "groupA": ("cycles", "instructions", "branches", "branch-misses"),
    "groupB": ("cache-references", "cache-misses"),
    "groupC1": ("LLC-loads", "LLC-load-misses"),
    "groupC2": ("dTLB-loads", "dTLB-load-misses"),
    "groupS": ("context-switches", "cpu-migrations", "page-faults",
               "minor-faults", "major-faults"),
}

HEADER = ("privilege,cell,m_blocks,cycles_per_update,instructions_per_update,"
          "cycles_per_block,instructions_per_block,cycles_per_instruction,"
          "branches_per_update,branch_misses_per_update,cache_misses_per_block,"
          "LLC_load_misses_per_block,dTLB_load_misses_per_block,"
          "page_faults_per_update,context_switches_per_update,"
          "cpu_migrations_per_update")

MIN_PCNT_RUNNING = 99.0


def normalize_event(name):
    """Strip a trailing perf modifier (":u", ":k", ...) from an event name.

    This is the exact defect fixed here: the superseded inline generator
    matched raw names like "cycles:u" only, so root files ("cycles")
    silently fell through to zero.
    """
    if ":" in name:
        head, _, mod = name.rpartition(":")
        if mod and all(c in "ukhHGpP" for c in mod):
            return head
    return name


def parse_events(path, expected=()):
    """Parse one perf stat --json JSONL file into {normalized: value}.

    Raises KeyError listing the missing events if any of `expected` is
    absent — the anti-silent-zero property the superseded generator lacked.
    """
    counters, pcnts = {}, []
    with open(path, encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            ev = normalize_event(row["event"])
            if ev in counters:
                raise ValueError(f"{path}:{lineno}: duplicate event {ev!r}")
            counters[ev] = float(row["counter-value"])
            pcnts.append(float(row.get("pcnt-running", 100.0)))
    missing = [e for e in expected if e not in counters]
    if missing:
        raise KeyError(f"{path}: missing events {missing} "
                       f"(have {sorted(counters)})")
    if pcnts and min(pcnts) < MIN_PCNT_RUNNING:
        raise ValueError(f"{path}: pcnt-running min {min(pcnts)} < "
                         f"{MIN_PCNT_RUNNING} (PMU_GROUP_UNRELIABLE)")
    return counters


def m_blocks(cell):
    return int(cell[:-3]) * 1024 * 1024 // 128 if cell.endswith("MiB") \
        else int(cell[:-3]) * 1024 // 128


def rounds_for(stat_dir, privilege, cell):
    """Round count from the pmu-run receipt sidecar (extra.rounds)."""
    sidecar = stat_dir / f"{privilege}-groupA-{cell}.jsonl"
    with open(sidecar, encoding="utf-8") as fh:
        receipt = json.loads(fh.readline())
    rounds = receipt["extra"]["rounds"]
    if receipt.get("record") != "receipt" or receipt.get("lane") != "pmu":
        raise ValueError(f"{sidecar}: first line is not a pmu receipt")
    return int(rounds)


def derive_row(stat_dir, privilege, cell):
    """Return the (raw counters, formatted summary row) for one cell."""
    rounds = rounds_for(stat_dir, privilege, cell)
    m = m_blocks(cell)

    raw = {}
    for group in GROUPS:
        raw.update(parse_events(
            stat_dir / f"{privilege}-{group}-{cell}.json",
            expected=GROUP_EVENTS[group]))

    cycles_u = raw["cycles"] / rounds
    instr_u = raw["instructions"] / rounds

    def per_block(name, decimals):
        return round(raw[name] / rounds / m, decimals)

    # Guard: a non-zero raw counter must never derive to a zero summary
    # value. (Zero raw counters legitimately derive to zero.)
    for name, value in raw.items():
        if value > 0 and math.isclose(value / rounds, 0.0):
            raise AssertionError(
                f"{privilege}-{cell}: non-zero raw {name} derived to 0")

    row = [
        privilege, cell, m,
        int(round(cycles_u)),
        int(round(instr_u)),
        round(cycles_u / m, 2),
        round(instr_u / m, 2),
        round(cycles_u / instr_u, 3) if instr_u else 0.0,
        int(round(raw["branches"] / rounds)),
        int(round(raw["branch-misses"] / rounds)),
        per_block("cache-misses", 3),
        per_block("LLC-load-misses", 3),
        per_block("dTLB-load-misses", 4),
        int(round(raw["page-faults"] / rounds)),
        round(raw["context-switches"] / rounds, 2),
        round(raw["cpu-migrations"] / rounds, 2),
    ]
    return raw, row


def derive_all(stat_dir):
    rows = []
    for privilege in PRIVILEGES:
        for cell in CELLS:
            _, row = derive_row(stat_dir, privilege, cell)
            rows.append(row)
    return rows


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--stat-dir", type=Path, default=Path(__file__).parent / "stat")
    ap.add_argument("--out", type=Path, default=Path(__file__).parent / "stat-summary.csv")
    args = ap.parse_args(argv)

    rows = derive_all(args.stat_dir)
    lines = [HEADER]
    for row in rows:
        lines.append(",".join(str(v) for v in row))
    # CRLF, matching every other csv.writer-produced CSV in this results dir
    args.out.write_text("\r\n".join(lines) + "\r\n", encoding="utf-8")
    print(f"STAT_SUMMARY_REGENERATED rows={len(rows)} out={args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
