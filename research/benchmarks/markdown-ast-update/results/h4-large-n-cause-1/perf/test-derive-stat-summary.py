#!/usr/bin/env python3
"""Regression tests for derive-stat-summary.py (PR #51 corrective).

The superseded inline generator matched perf event names with the
user-mode modifier attached ("cycles:u"), so every root-privilege file
("cycles") silently derived to all-zero summary rows even though the raw
root PMU counters were valid and committed. These tests pin the fixed
behaviour:

  1. non-zero RAW ROOT counters (unsuffixed event names) must derive to
     non-zero ROOT summary counters;
  2. event-name normalization: the same raw counters recorded under
     ":u"-suffixed names (user privilege) must derive to identical values;
  3. a missing required event must raise, never silently zero;
  4. zero raw counters legitimately derive to zero (guard must not
     false-positive);
  5. known raw -> summary numeric case from the committed root raw file
     root-groupA-16MiB.json (cycles 1365569519 / rounds 15 -> 91037968).

Run:  python3 perf/test-derive-stat-summary.py
"""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location(
    "derive_stat_summary", HERE / "derive-stat-summary.py")
dss = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(dss)

CELL = "1MiB"
ROUNDS = 15
M = dss.m_blocks(CELL)  # 8192

# Synthetic raw counters (large enough that every per-block column also
# survives its rounding to a non-zero value).
SYNTH = {
    "groupA": {"cycles": 1500000, "instructions": 1200000,
               "branches": 300000, "branch-misses": 6000},
    "groupB": {"cache-references": 450000, "cache-misses": 75000},
    "groupC1": {"LLC-loads": 400000, "LLC-load-misses": 1000},
    "groupC2": {"dTLB-loads": 3000000, "dTLB-load-misses": 30000},
    "groupS": {"context-switches": 300, "cpu-migrations": 0,
               "page-faults": 1500, "minor-faults": 1500,
               "major-faults": 0},
}


def write_stat_dir(root, privilege, suffix, counters=None):
    """Create a synthetic perf/stat-style directory for one privilege."""
    stat = root / "stat"
    stat.mkdir(parents=True, exist_ok=True)
    counters = counters if counters is not None else SYNTH
    for group, events in counters.items():
        with open(stat / f"{privilege}-{group}-{CELL}.json", "w") as fh:
            for name, value in events.items():
                fh.write(json.dumps({
                    "counter-value": f"{float(value):.6f}", "unit": "",
                    "event": f"{name}{suffix}",
                    "event-runtime": 1000000, "pcnt-running": 100.00,
                }) + "\n")
    with open(stat / f"{privilege}-groupA-{CELL}.jsonl", "w") as fh:
        fh.write(json.dumps({
            "record": "receipt", "lane": "pmu",
            "extra": {"rounds": ROUNDS},
        }) + "\n")
    return stat


class NormalizeEventTest(unittest.TestCase):
    def test_strips_perf_modifiers(self):
        for raw, want in [("cycles:u", "cycles"), ("cycles", "cycles"),
                          ("branches:u", "branches"),
                          ("LLC-load-misses", "LLC-load-misses"),
                          ("cache-misses:u", "cache-misses")]:
            self.assertEqual(dss.normalize_event(raw), want)


class RootNonZeroRegressionTest(unittest.TestCase):
    """THE regression: raw root counters non-zero => summary non-zero."""

    def test_root_unsuffixed_events_derive_nonzero(self):
        with tempfile.TemporaryDirectory() as td:
            stat = write_stat_dir(Path(td), "root", "")
            _, row = dss.derive_row(stat, "root", CELL)
            by_col = dict(zip(dss.HEADER.split(","), row))
            for col in ("cycles_per_update", "instructions_per_update",
                        "branches_per_update", "branch_misses_per_update",
                        "page_faults_per_update"):
                self.assertGreater(by_col[col], 0, f"{col} derived to 0")
            for col in ("cache_misses_per_block", "LLC_load_misses_per_block",
                        "dTLB_load_misses_per_block"):
                self.assertGreater(by_col[col], 0.0, f"{col} derived to 0")

    def test_suffixed_and_unsuffixed_derive_identically(self):
        with tempfile.TemporaryDirectory() as td:
            root = write_stat_dir(Path(td), "root", "")
            user = write_stat_dir(Path(td), "user", ":u")
            _, r_row = dss.derive_row(root, "root", CELL)
            _, u_row = dss.derive_row(user, "user", CELL)
            # identical counters, only the privilege label differs
            self.assertEqual(r_row[1:], u_row[1:])

    def test_zero_raw_counters_legitimately_stay_zero(self):
        zero = {g: {k: 0 for k in evs} for g, evs in SYNTH.items()}
        with tempfile.TemporaryDirectory() as td:
            stat = write_stat_dir(Path(td), "root", "", counters=zero)
            _, row = dss.derive_row(stat, "root", CELL)
            self.assertEqual(row[3], 0)   # cycles_per_update
            self.assertEqual(row[4], 0)   # instructions_per_update

    def test_missing_event_raises_instead_of_silent_zero(self):
        with tempfile.TemporaryDirectory() as td:
            stat = write_stat_dir(Path(td), "root", "")
            # drop the cycles event, exactly the miss the old generator made
            path = stat / f"root-groupA-{CELL}.json"
            lines = [l for l in path.read_text().splitlines()
                     if '"cycles"' not in l]
            path.write_text("\n".join(lines) + "\n")
            with self.assertRaises(KeyError):
                dss.derive_row(stat, "root", CELL)


class KnownRawToSummaryCaseTest(unittest.TestCase):
    """Known numeric case from the committed root raw evidence."""

    REAL = HERE / "stat" / "root-groupA-16MiB.json"

    def test_root_16mib_groupA_numbers(self):
        if not self.REAL.exists():
            self.skipTest("committed raw perf/stat not present")
        raw = dss.parse_events(self.REAL)
        rounds = dss.rounds_for(HERE / "stat", "root", "16MiB")
        self.assertEqual(rounds, 15)
        cycles_u = raw["cycles"] / rounds
        instr_u = raw["instructions"] / rounds
        self.assertEqual(int(round(cycles_u)), 91037968)     # 1365569519/15
        self.assertEqual(int(round(instr_u)), 35296466)      # 529446989/15
        self.assertEqual(round(cycles_u / instr_u, 3), 2.579)
        # and the same case through the full row derivation
        _, row = dss.derive_row(HERE / "stat", "root", "16MiB")
        by_col = dict(zip(dss.HEADER.split(","), row))
        self.assertEqual(by_col["cycles_per_update"], 91037968)
        self.assertEqual(by_col["instructions_per_update"], 35296466)
        self.assertEqual(by_col["cycles_per_instruction"], 2.579)


class FullRegenerationTest(unittest.TestCase):
    def test_real_stat_dir_derives_all_16_rows(self):
        if not (HERE / "stat").exists():
            self.skipTest("committed raw perf/stat not present")
        rows = dss.derive_all(HERE / "stat")
        self.assertEqual(len(rows), 16)
        root = [r for r in rows if r[0] == "root"]
        user = [r for r in rows if r[0] == "user"]
        self.assertEqual(len(root), 8)
        self.assertEqual(len(user), 8)
        for r in root + user:
            self.assertGreater(r[3], 0, f"zero cycles row: {r}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
