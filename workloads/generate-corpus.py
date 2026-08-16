#!/usr/bin/env python3
"""Generate the Markit Phase A1 shared corpus (workloads/corpus/).

Deterministic, fixed-seed ASCII corpus family for the GPUI/PocketJS MVP
comparison: 10 KB, 100 KB, 1 MB. One generator, one seed — both MVPs read
the exact same files, so cross-framework measurement stays apples-to-apples.

Line profile mirrors the MVP seed style: short prose-ish ASCII lines with
wraps at word boundaries, occasional blank lines, no CJK (the Phase A1
first-round workload is ASCII/Latin only; CJK is a capability item).

Run: python3 workloads/generate-corpus.py
"""

import random
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "corpus"
SEED = 0xA1C0FFEE  # fixed: same bytes on every machine, every run

WORDS = (
    "the quick brown fox jumps over a lazy dog near the fence while the "
    "editor measures frame latency on a modern windows desktop with a "
    "fixed step clock and a demand rendered draw list where one character "
    "edit invalidates exactly the visible region and nothing else "
    "benchmark workloads keep the same bytes on both sides so the "
    "comparison table reports one truth and never a guess"
).split()

def sentence(rng: random.Random) -> str:
    n = rng.randint(6, 14)
    words = [rng.choice(WORDS) for _ in range(n)]
    s = " ".join(words)
    return s[0].upper() + s[1:] + "."

def document(rng: random.Random, target: int) -> str:
    lines = []
    total = 0
    while total < target:
        if rng.random() < 0.08:
            lines.append("")
            total += 1
        else:
            ln = sentence(rng)
            lines.append(ln)
            total += len(ln) + 1
    return "\n".join(lines) + "\n"

def main() -> None:
    OUT.mkdir(exist_ok=True)
    # Phase A3: 250K/500K added for the intervention scaling cells; the
    # 10K/100K/1M bytes are unchanged (same seed, same generation order).
    for name, kb in (("10k.txt", 10), ("100k.txt", 100), ("250k.txt", 250),
                     ("500k.txt", 500), ("1m.txt", 1024)):
        rng = random.Random(SEED)
        body = document(rng, kb * 1024)
        (OUT / name).write_text(body, encoding="ascii")
        print(f"corpus: {name} {len(body)} bytes, {body.count(chr(10))} lines")

if __name__ == "__main__":
    main()
