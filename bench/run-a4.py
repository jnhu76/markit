#!/usr/bin/env python3
"""Phase A4-R1 experiment driver (runs on the A1 Windows machine via WSL).

Same machine/exes/corpora conventions as A3; output goes to
results/raw/a4/ and results/summary/a4/.

Cells:

  r1-scale <corpus> <bundle> [runs]     A1 typing trace (100 single-char
                                        inserts + backspace + scroll) on
                                        the given bundle. Bundles:
                                          base       Solid editor (markit-editor)
                                          cf         imperative counterfactual
                                          cf-notext  counterfactual, no text
  r1-pos   <corpus> <pos> <bundle> [runs]  click at begin/q1/mid/q3/end,
                                        50 single-char inserts (position
                                        scaling of the suffix-shift term).
  r1-ops   <corpus> <bundle> [runs]     op-count run (WRAP_OPS bundles:
                                        main-ops / cf-ops; slow, work
                                        counts only).

  r2-case  <corpus> <m1|m2|m3|m4|m5|m6> [runs]
                                        one Markdown L1 edit at the manifest
                                        position (workloads/corpus-md/
                                        markdown-<corpus>.positions.json):
                                        scroll the target line into view,
                                        click at its start, Right×char,
                                        type one char (M6 additionally
                                        scrolls back out of view before the
                                        edit). Reports the edit's
                                        blocks_scanned / blocks_reparsed /
                                        inline_parsed / runs_rendered plus
                                        the host-side gf_us.

Each run prints a per-tick summary (gf_us/ct_us/dl_us/r_us/words/dirty
p50/p95/max over edit ticks) plus the guest perf reply, and writes the raw
log + summary file.
"""

import json
import os
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "results" / "raw" / "a4"
SUM = ROOT / "results" / "summary" / "a4"

# Same Windows workspace as A2/A3 (exes + dist are replaced by A4 builds).
A2 = Path("/mnt/c/markit-a2")
PJS_EXE = A2 / "mvp" / "pocketjs" / "target" / "release" / "mvp-pocketjs.exe"
DIST = A2 / "mvp" / "pocketjs" / "dist"

BUNDLES = {
    "base": ("markit-editor", "markit-editor"),
    "cf": ("cf", "cf"),
    "cf-notext": ("cf-notext", "cf-notext"),
    "main-ops": ("main-ops", "main-ops"),
    "cf-ops": ("cf-ops", "cf-ops"),
    "md": ("md", "md"),
}


def win(p: Path) -> str:
    s = str(p)
    if s.startswith("/mnt/"):
        drive = s[5]
        return f"{drive.upper()}:\\" + s[7:].replace("/", "\\")
    return s


def corpus_lines(name: str) -> int:
    p = A2 / f"{name}.txt"
    return p.read_text(encoding="ascii", errors="replace").count("\n")


def click_line(kind: str, total: int) -> int:
    frac = {"begin": 0.0, "q1": 0.25, "mid": 0.5, "q3": 0.75, "end": 1.0}
    return max(0, min(total - 1, int((total - 1) * frac[kind])))


def a1_typing():
    flags = []
    for t in range(340, 440):
        flags += ["--type", f"a@{t}"]
    flags += ["--key", "Backspace@450", "--scroll", "56@460"]
    return flags


def pjs_cmd(corpus, bundle, extra, auto_quit="9", file_name=None):
    js, pak = BUNDLES[bundle]
    return [
        str(PJS_EXE),
        "--js", win(DIST / f"{js}.js"),
        "--pak", win(DIST / f"{pak}.pak"),
        "--file", win(A2 / (file_name or f"{corpus}.txt")),
        "--width", "1000", "--height", "700",
        "--auto-quit", auto_quit,
        "--perf",
    ] + extra


def pjs_env():
    env = os.environ.copy()
    env["POCKETJS_DIST"] = win(DIST)
    return env


def summarize(log_path: Path) -> str:
    """Per-tick stats over edit ticks + the guest perf reply + the last
    edit's R2 work counters (blocks_scanned/reparsed, inline, runs)."""
    out = []
    edits = []
    ticks = 0
    last_ring = None
    for line in log_path.open(errors="replace"):
        if line.startswith('{"perf"'):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            ticks += 1
            if d.get("ev", 0) > 0:
                edits.append(d)
        elif line.startswith("[perf] "):
            payload = line[len("[perf] "):].strip()
            out.append("guest-perf " + payload)
            try:
                d = json.loads(payload)
                ring = json.loads(d.get("editsRing", "[]"))
                if ring:
                    last_ring = ring[-1]
            except ValueError:
                pass
        elif line.startswith("[state]"):
            out.append("state " + line.strip())
    if last_ring:
        for key in ("blocksScanned", "blocksReparsed", "inlineParsed", "runsRendered", "adjusts", "modelMs", "indexMs", "solidMs"):
            if key in last_ring:
                out.insert(0, f"last-edit {key}={last_ring[key]}")
    if not edits:
        return "no edit ticks\n" + "\n".join(out)

    def stats(key):
        xs = sorted(e[key] for e in edits)
        return (statistics.median(xs), xs[int(len(xs) * 0.95)], max(xs))

    for key, unit in [("gf_us", "us"), ("ct_us", "us"), ("dl_us", "us"), ("r_us", "us"), ("words", "w"), ("dirty", "d")]:
        p50, p95, mx = stats(key)
        out.insert(0, f"{key} p50={p50} p95={p95} max={mx} ({unit})")
    out.insert(0, f"edit_ticks={len(edits)} total_ticks={ticks}")
    return "\n".join(out)


def run(cell, argv, env, run_i):
    raw_dir = RAW / "r1"
    sum_dir = SUM / "r1"
    raw_dir.mkdir(parents=True, exist_ok=True)
    sum_dir.mkdir(parents=True, exist_ok=True)
    log = raw_dir / f"{cell}-{run_i}.log"
    with open(log, "wb") as f:
        p = subprocess.run(argv, env=env, stdout=f, stderr=subprocess.STDOUT)
    if p.returncode != 0:
        print(f"  !! {log.name}: exit={p.returncode}")
    with open(sum_dir / f"{cell}-{run_i}.summary.txt", "w") as o:
        o.write(summarize(log))
    print(f"  wrote {log.name}")
    return log


def main():
    cmd = sys.argv[1:]
    if not cmd:
        print(__doc__)
        return 1
    kind = cmd[0]
    runs = 3

    if kind == "r1-scale":
        corpus, bundle = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        for i in range(runs):
            run(f"r1-scale-{corpus}-{bundle}", pjs_cmd(corpus, bundle, a1_typing()), pjs_env(), i)

    elif kind == "r1-pos":
        corpus, pos, bundle = cmd[1], cmd[2], cmd[3]
        if len(cmd) > 4:
            runs = int(cmd[4])
        y = click_line(pos, corpus_lines(corpus)) * 28
        extra = ["--click", f"100,{y}@340"]
        for t in range(342, 392):
            extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(f"r1-pos-{pos}-{corpus}-{bundle}", pjs_cmd(corpus, bundle, extra, "7"), pjs_env(), i)

    elif kind == "r2-case":
        corpus, case = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        manifest = json.loads(
            (ROOT / "workloads" / "corpus-md" / f"markdown-{corpus}.positions.json").read_text())
        target = manifest[case]
        line, char = target["line"], target["char"]
        t = 340
        extra = ["--scroll", f"{line * 28}@340", "--click", f"0,14@341"]
        t = 342
        for _ in range(char):
            extra += ["--key", f"Right@{t}"]
            t += 1
        if case == "m6":
            # Edit outside the viewport: scroll back out of view first.
            extra += ["--scroll", f"{-line * 28}@{t}"]
            t += 1
        extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(f"r2-case-{case}-{corpus}",
                pjs_cmd(corpus, "md", extra, "7", file_name=f"markdown-{corpus}.md"),
                pjs_env(), i)

    elif kind == "r1-ops":
        corpus, bundle = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        for i in range(runs):
            run(f"r1-ops-{corpus}-{bundle}", pjs_cmd(corpus, bundle, a1_typing()), pjs_env(), i)

    else:
        print(f"unknown kind {kind}")
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
