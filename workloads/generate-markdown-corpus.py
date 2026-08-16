#!/usr/bin/env python3
"""Generate the A4-R2 Markdown L1 corpus family (workloads/corpus-md/).

Deterministic fixed-seed Markdown documents for the Phase A4-R2 invalidation
radius experiment: 10K / 100K / 1M bytes of L1-subset Markdown (heading,
paragraph, unordered/ordered list, blockquote, fenced code block, inline
bold/emphasis/code/link — see the A4 phase spec §8). ASCII only (U0), like
the A1-A3 plain corpora.

Each document is a sequence of chapters, each chapter = heading + paragraph
(with inline formatting) + list + quote + fence. The generator also writes a
manifest (markdown-<size>.positions.json) naming target lines/chars for the
M1-M6 edit cases:

  m1  paragraph edit        — mid-document paragraph line, mid-line char
  m2  inline emphasis edit  — a line containing a **bold** span, char inside
  m3  heading edit          — a heading line, char after the "# " marker
  m4  list edit             — a list item line, char after the "- " marker
  m5  fence boundary edit   — a fence-open line, char 0 (typing there breaks
                              the fence: structural invalidation)
  m6  edit outside viewport — a paragraph line far below the initial
                              viewport, mid-line char

Run: python3 workloads/generate-markdown-corpus.py
"""

import json
import pathlib
import random

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "corpus-md"
SEED = 0xA4B0C0DE  # fixed: same bytes on every machine, every run

WORDS = (
    "the quick brown fox jumps over a lazy dog near the fence while the "
    "editor measures frame latency on a modern windows desktop with a "
    "fixed step clock and a demand rendered draw list where one character "
    "edit invalidates exactly the visible region and nothing else "
    "benchmark workloads keep the same bytes on both sides so the "
    "comparison table reports one truth and never a guess about the "
    "markdown heading paragraph list quote and fenced code block structure"
).split()

TITLES = (
    "Introduction", "Methods", "Results", "Discussion", "Conclusion",
    "Appendix", "Related Work", "Evaluation", "Future Directions",
    "Background",
)


def sentence(rng: random.Random) -> str:
    n = rng.randint(6, 14)
    words = [rng.choice(WORDS) for _ in range(n)]
    s = " ".join(words)
    return s[0].upper() + s[1:] + "."


def inline(rng: random.Random, base: str) -> str:
    """Sprinkle L1 inline formatting into a sentence (deterministic)."""
    r = rng.random()
    if r < 0.25:
        return f"**{base}**"
    if r < 0.4:
        return f"*{base}*"
    if r < 0.55:
        return f"`{base}`"
    if r < 0.65:
        return f"[{base}](https://example.com/{rng.randint(1, 99)})"
    return base


def chapter(rng: random.Random, n: int) -> list[str]:
    lines = []
    lines.append(f"## {rng.choice(TITLES)} {n}")
    lines.append("")
    for _ in range(rng.randint(2, 4)):
        lines.append(inline(rng, sentence(rng)))
    lines.append("")
    items = rng.randint(3, 5)
    for _ in range(items):
        lines.append(f"- {inline(rng, sentence(rng))}")
    lines.append("")
    for _ in range(rng.randint(2, 3)):
        lines.append(f"{rng.randint(1, 9)}. {inline(rng, sentence(rng))}")
    lines.append("")
    for _ in range(rng.randint(2, 3)):
        lines.append(f"> {sentence(rng)}")
    lines.append("")
    lines.append("```")
    for _ in range(rng.randint(4, 7)):
        lines.append(f"let x{rng.randint(0, 99)} = {rng.randint(1, 999)};")
    lines.append("```")
    lines.append("")
    return lines


def document(rng: random.Random, target: int) -> str:
    lines: list[str] = []
    n = 0
    while sum(len(l) + 1 for l in lines) < target:
        n += 1
        lines.extend(chapter(rng, n))
    return "\n".join(lines) + "\n"


def manifest(lines: list[str], size: str) -> dict:
    """Target (line, char) for each M1-M6 case (0-based line index)."""
    total = len(lines)
    m = {}

    def find(pattern, lo_frac=0.0, hi_frac=1.0, predicate=lambda s: True):
        lo = int(total * lo_frac)
        hi = int(total * hi_frac)
        for i in range(lo, hi):
            if predicate(lines[i]) and pattern in lines[i]:
                return i
        raise RuntimeError(f"no line matching {pattern!r} in {size}")

    def is_plain_para(s: str) -> bool:
        if not s:
            return False
        if s.startswith(("#", "-", ">", "```", "*", "+")):
            return False
        if len(s) >= 2 and s[0].isdigit() and s[1] == ".":
            return False
        return "**" not in s and "*" not in s and "`" not in s and "[" not in s

    def find_para(lo_frac: float, hi_frac: float) -> int:
        """First plain paragraph line in the fraction range, fence-aware."""
        lo = int(total * lo_frac)
        hi = int(total * hi_frac)
        # Fence state at the range start: computed from the document head
        # (the range may begin inside a fence opened before it).
        in_fence = sum(1 for l in lines[:lo] if l == "```") % 2 == 1
        for i in range(lo, hi):
            l = lines[i]
            if l == "```":
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            if is_plain_para(l):
                return i
        raise RuntimeError(f"no paragraph line in {size}")

    # m1: a plain paragraph line (no inline markers) around 30% depth.
    m1_line = find_para(0.25, 0.5)
    m["m1"] = {"line": m1_line, "char": len(lines[m1_line]) // 2}

    # m2: a line with a **bold** span around 40% depth, char inside the span.
    m2_line = find("**", 0.3, 0.6)
    m["m2"] = {"line": m2_line, "char": lines[m2_line].index("**") + 2}

    # m3: the first heading line, char right after "## ".
    m3_line = next(i for i, l in enumerate(lines) if l.startswith("## "))
    m["m3"] = {"line": m3_line, "char": min(3, len(lines[m3_line]))}

    # m4: the first list item, char after "- ".
    m4_line = next(i for i, l in enumerate(lines) if l.startswith("- "))
    m["m4"] = {"line": m4_line, "char": 2}

    # m5: the first fence-open line, char 0 (typing 'a' breaks the fence).
    m5_line = next(i for i, l in enumerate(lines) if l == "```")
    m["m5"] = {"line": m5_line, "char": 0}

    # m6: a paragraph line at ~80% depth (far below the initial viewport).
    m6_line = find_para(0.7, 0.95)
    m["m6"] = {"line": m6_line, "char": len(lines[m6_line]) // 2}

    return m


def main() -> None:
    OUT.mkdir(exist_ok=True)
    for name, kb in (("markdown-10k.md", 10), ("markdown-100k.md", 100),
                     ("markdown-1m.md", 1024)):
        rng = random.Random(SEED)
        body = document(rng, kb * 1024)
        lines = body.split("\n")
        (OUT / name).write_text(body, encoding="ascii")
        m = manifest(lines, name)
        (OUT / name.replace(".md", ".positions.json")).write_text(
            json.dumps(m, indent=2) + "\n", encoding="ascii")
        print(f"corpus: {name} {len(body)} bytes, {body.count(chr(10))} lines, "
              f"m5 line {m['m5']['line']}, m6 line {m['m6']['line']}")


if __name__ == "__main__":
    main()
