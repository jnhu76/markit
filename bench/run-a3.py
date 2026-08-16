#!/usr/bin/env python3
"""Phase A3 experiment driver (runs on the A1 Windows machine via WSL).

Same machine/exes/corpora as A2; output goes to results/raw/a3/<stage>/ and
results/summary/a3/<stage>/ so the A1/A2 archives stay untouched.

Stage: `before` = A2 exes as-is (A3-0 baseline revalidation);
       `after`  = A3 intervention builds (post-P1/P2/G1/G2).

Usage:
  run-a3.py <before|after> pjs-scaling <10k|100k|250k|500k|1m> [runs]
  run-a3.py <before|after> pjs-pos <corpus> <begin|q1|mid|q3|end> [runs]
  run-a3.py <before|after> pjs-vp <corpus> <inside|near|far> [runs]
  run-a3.py <before|after> gpui-smoke <10k|100k|250k|500k|1m> [runs]
  run-a3.py <before|after> gpui-a2 <corpus> <pos|vp|static> <arg> [runs]
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "results" / "raw" / "a3"
SUM = ROOT / "results" / "summary" / "a3"
PARSER_A2 = ROOT / "bench" / "parse-a2.py"
PARSER_A1 = ROOT / "bench" / "parse-trace.py"

# Same Windows workspace as A2 (exes + dist are replaced by the A3 builds).
A2 = Path("/mnt/c/markit-a2")
PJS_EXE = A2 / "mvp" / "pocketjs" / "target" / "release" / "mvp-pocketjs.exe"
GPUI_EXE = A2 / "mvp" / "gpui" / "target" / "release" / "mvp-gpui.exe"
DIST = A2 / "mvp" / "pocketjs" / "dist"


def win(p: Path) -> str:
    """/mnt/c/... -> C:\\... (WSL interop does not translate args)."""
    s = str(p)
    if s.startswith("/mnt/"):
        drive = s[5]
        return f"{drive.upper()}:\\" + s[7:].replace("/", "\\")
    return s


def corpus_lines(name: str) -> int:
    """Line count of the corpus file in the Windows workspace."""
    p = A2 / f"{name}.txt"
    text = p.read_text(encoding="ascii", errors="replace")
    # A line starts after every '\n' (file ends with '\n'); count lines as
    # the A2/GPUI convention (line_count = newline count).
    return text.count("\n")


def click_line(kind: str, total: int) -> int:
    """Fraction-based click line for position experiments (A2 used fixed
    1M line numbers; fractions reproduce the same positions generically)."""
    frac = {"begin": 0.0, "q1": 0.25, "mid": 0.5, "q3": 0.75, "end": 1.0}
    return max(0, min(total - 1, int((total - 1) * frac[kind])))


# The A1 windowed workload: 100 single-char inserts at ticks 340..439,
# backspace @450, scroll @460, auto-quit 9s.
def a1_typing():
    flags = []
    for t in range(340, 440):
        flags += ["--type", f"a@{t}"]
    flags += ["--key", "Backspace@450", "--scroll", "56@460"]
    return flags


def run(stage, name, argv, env, corpus, run_i):
    raw_dir = RAW / stage
    sum_dir = SUM / stage
    raw_dir.mkdir(parents=True, exist_ok=True)
    sum_dir.mkdir(parents=True, exist_ok=True)
    log = raw_dir / f"{name}-{corpus}-{run_i}.log"
    with open(log, "wb") as f:
        p = subprocess.run(argv, env=env, stdout=f, stderr=subprocess.STDOUT)
    if p.returncode != 0:
        print(f"  !! {log.name}: exit={p.returncode}")
    family_flag = ["--gpui"] if name.startswith(("gpui-",)) else []
    for parser in (PARSER_A2, PARSER_A1):
        with open(log) as f:
            s = subprocess.run(
                [sys.executable, str(parser), "--quiet", *family_flag],
                stdin=f, capture_output=True, text=True,
            )
        if "no trace events" in s.stdout:
            continue
        with open(sum_dir / f"{name}-{corpus}-{run_i}.summary.txt", "a") as o:
            o.write(f"=== {parser.name} {log.name} ===\n{s.stdout}\n")
    print(f"  wrote {log.name}")
    return log


def pjs_env():
    env = os.environ.copy()
    env["POCKETJS_DIST"] = win(DIST)
    return env


def pjs_cmd(corpus, extra, auto_quit="9", perf=False):
    argv = [
        str(PJS_EXE),
        "--js", win(DIST / "markit-editor.js"),
        "--pak", win(DIST / "markit-editor.pak"),
        "--file", win(A2 / f"{corpus}.txt"),
        "--width", "1000", "--height", "700",
        "--auto-quit", auto_quit,
    ]
    if perf:
        argv += ["--perf"]
    return argv + extra


def gpui_cmd(corpus, extra, a2=False):
    argv = [str(GPUI_EXE)]
    if a2:
        argv += ["--a2"]
    return argv + extra + ["--file", win(A2 / f"{corpus}.txt")]


def main():
    cmd = sys.argv[1:]
    if not cmd or cmd[0] not in ("before", "after"):
        print(__doc__)
        return 1
    stage = cmd[0]
    rest = cmd[1:]
    if not rest:
        print(__doc__)
        return 1
    kind = rest[0]
    runs = 5

    if kind == "pjs-scaling":
        corpus = rest[1]
        if len(rest) > 2:
            runs = int(rest[2])
        for i in range(runs):
            run(stage, f"pjs-scale-{corpus}", pjs_cmd(corpus, a1_typing(), perf=True),
                pjs_env(), corpus, i)

    elif kind == "pjs-pos":
        corpus, pos = rest[1], rest[2]
        if len(rest) > 3:
            runs = int(rest[3])
        y = click_line(pos, corpus_lines(corpus)) * 28
        extra = ["--click", f"100,{y}@340"]
        for t in range(342, 392):
            extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(stage, f"pjs-pos-{pos}", pjs_cmd(corpus, extra, "7", perf=True),
                pjs_env(), corpus, i)

    elif kind == "pjs-vp":
        corpus, vp = rest[1], rest[2]
        if len(rest) > 3:
            runs = int(rest[3])
        click_line_vp = {"inside": 10, "near": 30, "far": "far"}[vp]
        if click_line_vp == "far":
            click_line_vp = max(0, corpus_lines(corpus) // 2)
        extra = ["--click", f"100,{click_line_vp * 28}@340"]
        for t in range(342, 392):
            extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(stage, f"pjs-vp-{vp}", pjs_cmd(corpus, extra, "7", perf=True),
                pjs_env(), corpus, i)

    elif kind == "gpui-smoke":
        corpus = rest[1]
        if len(rest) > 2:
            runs = int(rest[2])
        for i in range(runs):
            run(stage, f"gpui-smoke-{corpus}",
                gpui_cmd(corpus, ["--smoke"], a2=True), {}, corpus, i)

    elif kind == "gpui-a2":
        corpus, mode, arg = rest[1], rest[2], rest[3]
        if len(rest) > 4:
            runs = int(rest[4])
        argv = ["--a2-mode", mode, "--a2-n", "50"]
        if mode == "pos":
            argv += ["--a2-pos", arg]
        elif mode == "vp":
            argv += ["--a2-vp", arg]
        for i in range(runs):
            run(stage, f"gpui-{mode}-{arg}", gpui_cmd(corpus, argv, a2=True),
                {}, corpus, i)

    else:
        print(f"unknown kind {kind}")
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
