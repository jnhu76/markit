#!/usr/bin/env python3
"""Phase A2 experiment driver (runs on the A1 Windows machine via WSL).

Invokes the Windows exes (windowed, same machine/toolchain as A1), saves
raw logs to results/raw/a2/, and appends parse-a2.py + parse-trace.py
summaries. Keeps the A1 result files untouched.

WSL interop does not translate /mnt/... arguments or env vars, so all file
paths are passed as C:\\... and instrumentation is enabled by flags
(--perf / --a2), not environment.

Usage:
  run-a2.py pjs-scaling <10k|100k|1m> [runs]
  run-a2.py pjs-pos <10k|100k|1m> <begin|q1|mid|q3|end> [runs]
  run-a2.py pjs-vp <10k|100k|1m> <inside|near|far> [runs]
  run-a2.py pjs-noop <10k|100k|1m> <empty|left> [runs]
  run-a2.py gpui-smoke <10k|100k|1m> [runs]
  run-a2.py gpui-a2 <10k|100k|1m> <pos|vp|static|scale> <arg> [runs]
  run-a2.py cal-pjs <10k|1m> [runs]
  run-a2.py cal-gpui <10k|1m> [runs]
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "results" / "raw" / "a2"
SUM = ROOT / "results" / "summary" / "a2"
PARSER_A2 = ROOT / "bench" / "parse-a2.py"
PARSER_A1 = ROOT / "bench" / "parse-trace.py"

# A2 workspace (instrumented) and A1 workspace (original baseline exes).
A2 = Path("/mnt/c/markit-a2")
A1 = Path("/mnt/c/markit-a1")
PJS_A2 = A2 / "mvp" / "pocketjs" / "target" / "release" / "mvp-pocketjs.exe"
GPUI_A2 = A2 / "mvp" / "gpui" / "target" / "release" / "mvp-gpui.exe"
PJS_A1 = A1 / "mvp-pocketjs.exe"
GPUI_A1 = A1 / "gpui" / "target" / "release" / "mvp-gpui.exe"
DIST_A2 = A2 / "mvp" / "pocketjs" / "dist"
DIST_A1 = A1 / "dist"


def win(p: Path) -> str:
    """/mnt/c/... -> C:\\... (WSL interop does not translate args)."""
    s = str(p)
    if s.startswith("/mnt/"):
        drive = s[5]
        return f"{drive.upper()}:\\" + s[7:].replace("/", "\\")
    return s


# The A1 windowed workload: 100 single-char inserts at ticks 340..439,
# backspace @450, scroll @460, auto-quit 9s.
def a1_typing():
    flags = []
    for t in range(340, 440):
        flags += ["--type", f"a@{t}"]
    flags += ["--key", "Backspace@450", "--scroll", "56@460"]
    return flags


def run(name, argv, env, corpus, run_i):
    RAW.mkdir(parents=True, exist_ok=True)
    SUM.mkdir(parents=True, exist_ok=True)
    log = RAW / f"{name}-{corpus}-{run_i}.log"
    with open(log, "wb") as f:
        p = subprocess.run(argv, env=env, stdout=f, stderr=subprocess.STDOUT)
    if p.returncode != 0:
        print(f"  !! {log.name}: exit={p.returncode}")
    # Summaries: parse-a2 for the JSONL streams, parse-trace for the A1
    # ring dump (only written when the log actually has trace events).
    for parser in (PARSER_A2, PARSER_A1):
        with open(log) as f:
            s = subprocess.run(
                [sys.executable, str(parser), "--quiet"], stdin=f, capture_output=True, text=True
            )
        if "no trace events" in s.stdout:
            continue
        with open(SUM / f"{name}-{corpus}-{run_i}.summary.txt", "a") as o:
            o.write(f"=== {parser.name} {log.name} ===\n{s.stdout}\n")
    print(f"  wrote {log.name}")
    return log


def pjs_env():
    env = os.environ.copy()
    env["POCKETJS_DIST"] = win(DIST_A2)
    return env


def pjs_cmd(exe, corpus, extra, auto_quit="9", perf=False):
    dist = DIST_A1 if exe == PJS_A1 else DIST_A2
    argv = [
        str(exe),  # execable from WSL (interop); args below are C:\ paths
        "--js", win(dist / "markit-editor.js"),
        "--pak", win(dist / "markit-editor.pak"),
        "--file", win(A2 / f"{corpus}.txt"),
        "--width", "1000", "--height", "700",
        "--auto-quit", auto_quit,
    ]
    if perf:
        argv += ["--perf"]
    return argv + extra


def gpui_cmd(corpus, extra, a2=False, workspace=A2):
    argv = [str(workspace / "mvp" / "gpui" / "target" / "release" / "mvp-gpui.exe")]
    if a2:
        argv += ["--a2"]
    return argv + extra + ["--file", win(workspace / f"{corpus}.txt")]


def main():
    cmd = sys.argv[1:]
    if not cmd:
        print(__doc__)
        return 1
    kind = cmd[0]
    runs = 5

    if kind == "pjs-scaling":
        corpus = cmd[1]
        if len(cmd) > 2:
            runs = int(cmd[2])
        for i in range(runs):
            run(f"pjs-scale-{corpus}", pjs_cmd(PJS_A2, corpus, a1_typing(), perf=True), pjs_env(), corpus, i)

    elif kind == "pjs-pos":
        corpus, pos = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        # Click places the caret (y = line*28); 50 typed chars after.
        click_line = {"begin": 0, "q1": 4913, "mid": 9826, "q3": 14739, "end": 19652}[pos]
        y = click_line * 28
        extra = ["--click", f"100,{y}@340"]
        for t in range(342, 392):
            extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(f"pjs-pos-{pos}", pjs_cmd(PJS_A2, corpus, extra, "7", perf=True), pjs_env(), corpus, i)

    elif kind == "pjs-vp":
        corpus, vp = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        click_line = {"inside": 10, "near": 30, "far": 9826}[vp]
        extra = ["--click", f"100,{click_line * 28}@340"]
        for t in range(342, 392):
            extra += ["--type", f"a@{t}"]
        for i in range(runs):
            run(f"pjs-vp-{vp}", pjs_cmd(PJS_A2, corpus, extra, "7", perf=True), pjs_env(), corpus, i)

    elif kind == "pjs-noop":
        corpus, variant = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        extra = []
        if variant == "empty":
            # Empty insert: the guest skips the mutation entirely (falsy s)
            # — pure turn overhead (poll + parse + state/caret sends).
            for t in range(340, 390):
                extra += ["--type", f"@{t}"]
        else:  # left: caret moves, document unchanged (memo stays valid)
            extra = ["--click", "100,0@340"]
            for t in range(342, 392):
                extra += ["--key", f"Left@{t}"]
        for i in range(runs):
            run(f"pjs-noop-{variant}", pjs_cmd(PJS_A2, corpus, extra, "7", perf=True), pjs_env(), corpus, i)

    elif kind == "pjs-cf":
        # Counterfactual: the dist bundle was rebuilt with perf.ts CF=<n>;
        # runs the A1 typing workload and labels the run cf<n>.
        cf, corpus = cmd[1], cmd[2]
        if len(cmd) > 3:
            runs = int(cmd[3])
        for i in range(runs):
            run(f"pjs-cf{cf}-{corpus}", pjs_cmd(PJS_A2, corpus, a1_typing(), perf=True), pjs_env(), corpus, i)

    elif kind == "gpui-cf":
        # Counterfactual: the release exe was rebuilt with the diagnostic
        # truncation; the caller restores the clean exe afterwards.
        corpus = cmd[1]
        if len(cmd) > 2:
            runs = int(cmd[2])
        for i in range(runs):
            run(f"gpui-cf-{corpus}", gpui_cmd(corpus, ["--smoke"], a2=True), {}, corpus, i)

    elif kind == "gpui-smoke":
        corpus = cmd[1]
        if len(cmd) > 2:
            runs = int(cmd[2])
        for i in range(runs):
            run(
                f"gpui-smoke-{corpus}",
                gpui_cmd(corpus, ["--smoke"], a2=True),
                {}, corpus, i,
            )

    elif kind == "gpui-a2":
        corpus, mode, arg = cmd[1], cmd[2], cmd[3]
        if len(cmd) > 4:
            runs = int(cmd[4])
        argv = ["--a2-mode", mode, "--a2-n", "50"]
        if mode == "pos":
            argv += ["--a2-pos", arg]
        elif mode == "vp":
            argv += ["--a2-vp", arg]
        for i in range(runs):
            run(f"gpui-{mode}-{arg}", gpui_cmd(corpus, argv, a2=True), {}, corpus, i)

    elif kind == "cal-pjs":
        corpus = cmd[1]
        if len(cmd) > 2:
            runs = int(cmd[2])
        for i in range(runs):
            # original exe + original bundle (A1 baseline)
            run(f"cal-pjs-orig-{corpus}", pjs_cmd(PJS_A1, corpus, a1_typing()), pjs_env(), corpus, i)
            # instrumented exe, JSONL off
            run(f"cal-pjs-off-{corpus}", pjs_cmd(PJS_A2, corpus, a1_typing()), pjs_env(), corpus, i)
            # instrumented exe, JSONL on
            run(f"cal-pjs-on-{corpus}", pjs_cmd(PJS_A2, corpus, a1_typing(), perf=True), pjs_env(), corpus, i)

    elif kind == "cal-gpui":
        corpus = cmd[1]
        if len(cmd) > 2:
            runs = int(cmd[2])
        for i in range(runs):
            run(f"cal-gpui-orig-{corpus}", gpui_cmd(corpus, ["--smoke"], a2=False, workspace=A1), {}, corpus, i)
            run(f"cal-gpui-off-{corpus}", gpui_cmd(corpus, ["--smoke"], a2=False), {}, corpus, i)
            run(f"cal-gpui-on-{corpus}", gpui_cmd(corpus, ["--smoke"], a2=True), {}, corpus, i)

    else:
        print(f"unknown kind {kind}")
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
