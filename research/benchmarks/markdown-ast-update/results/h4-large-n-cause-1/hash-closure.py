#!/usr/bin/env python3
"""Deterministic RAW-HASH-CLOSURE.txt generator (H4-LARGE-N-CAUSE-1).

Writes and verifies the evidence closure of this directory:

    <sha256>  ./<path relative to this directory>

  usage:  hash-closure.py            report only, verify the existing file
          hash-closure.py --write    regenerate, then verify the result

Why this script exists
----------------------
The closure shipped in `45ad068` had two reproducibility defects that this
generator removes:

  1. It was produced ad hoc, so a later documentation-only edit to one of
     the covered files (`receipts/PRODUCER-RECEIPTS.csv`) left a stale hash
     behind and the closure stopped verifying. Regeneration is now a single
     committed command.
  2. Its line order was filesystem readdir order, which is not reproducible
     across machines or checkouts. The order here is the byte order
     (LC_ALL=C) of the "./"-prefixed relative path, so identical inputs
     produce a byte-identical file.

Coverage rule (explicit, and the reason it is this and not "all files")
----------------------------------------------------------------------
Every regular file under this directory is covered EXCEPT:

  - `RAW-HASH-CLOSURE.txt`   the artifact itself; it cannot carry its own
                             hash, and the prompt's closure scheme does not
                             include it in its own closure;
  - `hash-closure.py`        this generator: a tool, not a measurement or
                             a derived measurement artifact;
  - `__pycache__/`, `*.pyc`  interpreter bytecode caches, not evidence.

Everything else is covered, including the attempt-1..4 failure logs and the
archived `attempt-*-superseded/` trees, because Issue #50 section 17 and the
manifest's collection-attempt history cite them as evidence. Those `.log`
files are matched by the repository's `*.log` ignore rule, so this generator
prints a warning listing any covered path that git does not track: a closure
over files that a fresh clone cannot contain is not verifiable evidence.
"""

from __future__ import annotations

import hashlib
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CLOSURE_NAME = "RAW-HASH-CLOSURE.txt"
SELF_NAME = "hash-closure.py"
EXCLUDED_DIRS = {"__pycache__"}
EXCLUDED_SUFFIXES = (".pyc",)
EXCLUDED_NAMES = {CLOSURE_NAME, SELF_NAME}


def sha256_file(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def covered_paths() -> list[str]:
    """Relative "./"-prefixed paths of every covered file, in byte order."""
    found = []
    for root, dirs, files in os.walk(HERE):
        dirs[:] = sorted(d for d in dirs if d not in EXCLUDED_DIRS)
        for name in files:
            if name in EXCLUDED_NAMES or name.endswith(EXCLUDED_SUFFIXES):
                continue
            absolute = os.path.join(root, name)
            if not os.path.isfile(absolute) or os.path.islink(absolute):
                continue
            relative = os.path.relpath(absolute, HERE)
            found.append("./" + relative.replace(os.sep, "/"))
    return sorted(found)


def render(paths: list[str]) -> str:
    return "".join(f"{sha256_file(os.path.join(HERE, p[2:]))}  {p}\n" for p in paths)


def untracked(paths: list[str]) -> list[str]:
    """Covered paths that `git ls-files` does not track (warning only)."""
    try:
        top = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=HERE, capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return []
    rel_dir = os.path.relpath(HERE, top)
    try:
        tracked = set(subprocess.run(
            ["git", "ls-files", "--", rel_dir],
            cwd=top, capture_output=True, text=True, check=True,
        ).stdout.split())
    except (OSError, subprocess.CalledProcessError):
        return []
    return [p for p in paths if os.path.join(rel_dir, p[2:]) not in tracked]


def verify(paths: list[str]) -> int:
    expected = render(paths)
    closure_path = os.path.join(HERE, CLOSURE_NAME)
    if not os.path.exists(closure_path):
        print(f"VERIFY: {CLOSURE_NAME} does not exist")
        return 1
    with open(closure_path, "r", encoding="utf-8") as handle:
        actual = handle.read()
    if actual == expected:
        print(f"RAW_HASH_CLOSURE = PASS  ({len(paths)}/{len(paths)} files)")
        return 0
    actual_lines = actual.splitlines()
    expected_lines = expected.splitlines()
    missing = [ln for ln in expected_lines if ln not in set(actual_lines)]
    extra = [ln for ln in actual_lines if ln not in set(expected_lines)]
    print(f"RAW_HASH_CLOSURE = FAIL  ({len(actual_lines) - len(missing)}/{len(expected_lines)} match)")
    for line in missing[:20]:
        print(f"  STALE/MISSING  {line[:80]}...")
    for line in extra[:20]:
        print(f"  UNEXPECTED     {line[:80]}...")
    return 1


def main() -> int:
    write = "--write" in sys.argv[1:]
    paths = covered_paths()

    if write:
        with open(os.path.join(HERE, CLOSURE_NAME), "w", encoding="utf-8") as handle:
            handle.write(render(paths))
        print(f"wrote {CLOSURE_NAME}: {len(paths)} files")

    status = verify(paths)

    tracked_gap = untracked(paths)
    if tracked_gap:
        print(f"WARNING: {len(tracked_gap)} covered file(s) are NOT tracked by git; "
              f"the closure cannot be verified from a fresh clone:")
        for path in tracked_gap[:10]:
            print(f"  {path}")
        if len(tracked_gap) > 10:
            print(f"  ... and {len(tracked_gap) - 10} more")
    else:
        print("TRACKED_CLOSURE = PASS  (every covered file is tracked by git)")

    return status


if __name__ == "__main__":
    sys.exit(main())
