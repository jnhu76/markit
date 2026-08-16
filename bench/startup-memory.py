#!/usr/bin/env python3
"""Phase A3-M — startup, memory, and idle baseline runner (Windows via WSL).

Per run, for one MVP:
  launch the exe windowed with a corpus file
  wait for MARKIT_FIRST_USABLE_FRAME <ms> on stdout (process-internal
    delta: launch -> first usable frame, labeled frame-ready in the report)
  sample WorkingSet / Private Bytes / CPU at marker+1s and marker+4s
    (Get-Process by exe name; newest instance)
  kill the process after the idle window

Outputs per-run records to results/raw/a3/<mvp>-startup-<corpus>-<i>.log and
a summary block to results/summary/a3/<mvp>-startup-<corpus>-<i>.summary.txt.

Usage:
  startup-memory.py pjs <10k|100k|1m> [runs]
  startup-memory.py gpui <10k|100k|1m> [runs]
"""

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "results" / "raw" / "a3"
SUM = ROOT / "results" / "summary" / "a3"

A2 = Path("/mnt/c/markit-a2")
PJS_EXE = A2 / "mvp" / "pocketjs" / "target" / "release" / "mvp-pocketjs.exe"
GPUI_EXE = A2 / "mvp" / "gpui" / "target" / "release" / "mvp-gpui.exe"
DIST = A2 / "mvp" / "pocketjs" / "dist"

MARKER = re.compile(r"MARKIT_FIRST_USABLE_FRAME (\d+)")


def win(p: Path) -> str:
    s = str(p)
    if s.startswith("/mnt/"):
        drive = s[5]
        return f"{drive.upper()}:\\" + s[7:].replace("/", "\\")
    return s


def powershell(script: str) -> str:
    p = subprocess.run(
        ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script],
        capture_output=True, text=True, errors="replace", timeout=60,
    )
    return p.stdout.strip()


def sample_process(name: str) -> dict:
    """WorkingSet / PrivateBytes / CPU-seconds of the newest instance."""
    script = (
        f"$p = Get-Process {name} -ErrorAction SilentlyContinue | "
        "Sort-Object StartTime -Descending | Select-Object -First 1; "
        "if ($p) { $p | Select-Object Id,WorkingSet64,PrivateMemorySize64,CPU | "
        "ConvertTo-Json -Compress } else { 'null' }"
    )
    out = powershell(script)
    if not out or out == "null":
        return {}
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        return {}


def kill_process(name: str) -> None:
    subprocess.run(["taskkill.exe", "/IM", f"{name}.exe", "/F"],
                   capture_output=True, text=True, errors="replace", timeout=30)


def argv_for(kind: str, corpus: str) -> list:
    if kind == "pjs":
        return [
            str(PJS_EXE),
            "--js", win(DIST / "markit-editor.js"),
            "--pak", win(DIST / "markit-editor.pak"),
            "--file", win(A2 / f"{corpus}.txt"),
            "--width", "1000", "--height", "700",
            "--auto-quit", "30",
        ]
    return [str(GPUI_EXE), "--a2", "--file", win(A2 / f"{corpus}.txt")]


def run(kind: str, corpus: str, run_i: int) -> dict:
    exe_name = "mvp-pocketjs" if kind == "pjs" else "mvp-gpui"
    # Clean slate: no stale instances of this exe.
    kill_process(exe_name)
    time.sleep(0.5)

    log = RAW / f"{kind}-startup-{corpus}-{run_i}.log"
    with open(log, "wb") as f:
        p = subprocess.Popen(argv_for(kind, corpus), stdout=f, stderr=subprocess.STDOUT)

    marker_ms = None
    deadline = time.time() + 60
    while time.time() < deadline:
        if p.poll() is not None:
            break
        text = log.read_text(errors="replace") if log.exists() else ""
        m = MARKER.search(text)
        if m:
            marker_ms = int(m.group(1))
            break
        time.sleep(0.1)
    if marker_ms is None:
        print(f"  !! {kind} run {run_i}: no MARKIT_FIRST_USABLE_FRAME (exit={p.poll()})")

    # Idle sampling: memory + CPU at marker+1s and marker+4s (3s apart).
    time.sleep(1.2)
    s1 = sample_process(exe_name)
    time.sleep(3.0)
    s2 = sample_process(exe_name)

    # PocketJS prints its frames/tick summary on clean exit; gpui keeps the
    # a2 JSONL per frame (idle => ~1 line, the initial frame).
    kill_process(exe_name)
    try:
        p.wait(timeout=10)
    except subprocess.TimeoutExpired:
        p.kill()

    frames = None
    ticks = None
    text = log.read_text(errors="replace") if log.exists() else ""
    m = re.search(r"pocket-widget: (\d+) ticks, (\d+) frames rendered", text)
    if m:
        ticks, frames = int(m.group(1)), int(m.group(2))
    gpui_frames = len(re.findall(r'"prepaint_us"', text))

    rec = {
        "mvp": kind,
        "corpus": corpus,
        "run": run_i,
        "first_usable_frame_ms": marker_ms,
        "ws_bytes_1s": s1.get("WorkingSet64"),
        "priv_bytes_1s": s1.get("PrivateMemorySize64"),
        "cpu_s_1s": s1.get("CPU"),
        "ws_bytes_4s": s2.get("WorkingSet64"),
        "priv_bytes_4s": s2.get("PrivateMemorySize64"),
        "cpu_s_4s": s2.get("CPU"),
        "ticks": ticks,
        "frames_rendered": frames,
        "gpui_a2_frames": gpui_frames,
    }
    with open(SUM / f"{kind}-startup-{corpus}-{run_i}.summary.txt", "w") as o:
        o.write(f"=== startup-memory {kind} {corpus} run {run_i} ===\n")
        for k, v in rec.items():
            o.write(f"  {k}: {v}\n")
    print(f"  {kind} {corpus} run {run_i}: startup={marker_ms}ms ws={rec['ws_bytes_4s']} priv={rec['priv_bytes_4s']}")
    return rec


def main() -> int:
    cmd = sys.argv[1:]
    if len(cmd) < 2 or cmd[0] not in ("pjs", "gpui"):
        print(__doc__)
        return 1
    kind, corpus = cmd[0], cmd[1]
    runs = int(cmd[2]) if len(cmd) > 2 else 5
    RAW.mkdir(parents=True, exist_ok=True)
    SUM.mkdir(parents=True, exist_ok=True)
    print(f"startup-memory: {kind} {corpus} x{runs} (1 warmup + {runs - 1} measured)")
    recs = [run(kind, corpus, i) for i in range(runs)]
    key = lambda r, k: [x[k] for x in recs if x.get(k) is not None]
    for k, scale in (("first_usable_frame_ms", 1), ("ws_bytes_4s", 1 << 20), ("priv_bytes_4s", 1 << 20)):
        vals = sorted(key(recs, k))
        if not vals:
            continue
        n = len(vals)
        med = vals[n // 2] if n % 2 else (vals[n // 2 - 1] + vals[n // 2]) / 2
        unit = "ms" if scale == 1 else "MiB"
        print(f"  {k}: median={med / scale:.1f} {unit} min={vals[0] / scale:.1f} max={vals[-1] / scale:.1f} (n={n})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
