#!/usr/bin/env python3
"""Issue #100 L2 — perf script -> folded stacks + call-chain quality gate.

Reads `perf script` text (with --inline expansion), folds call chains
into `frame;frame;count` lines ordered ROOT -> LEAF, and emits a
quality-gate JSON:

  samples_total            every parsed sample
  samples_in_region        stacks rooted at the l2_p1_build/l2_p0_parse
                           marker frames (the sampled frozen operation)
  resolved_fraction        in-region samples with no [unknown] frame
  unknown_fraction         in-region samples containing any [unknown] frame
  leaf_unknown_fraction    in-region samples whose LEAF is [unknown]
  kernel_frames_in_region  in-region frames resolved into the kernel
  frame_order              detected order (leaf_first/root_first)

Frame order is detected per file from the frozen static nesting
l2_p1_build|l2_p0_parse -> full_build|full_parse, then applied uniformly.
"""

import argparse
import json
import re
import sys

HEADER_RE = re.compile(r"\S+\s+\d+\s+[\d.]+:\s+\S+")
FRAME_RE = re.compile(r"^\s+([0-9a-f]+)\s+(.*?)\s+\((.*)\)\s*$")

# The frozen marker frames of the profiling harness (mdbench-horse-a-v2-l2).
# profile_region may inline away; l2_p1_build/l2_p0_parse are inline(never)
# and always present as the in-region root.
REGION_MARKERS = ["l2_p1_build", "l2_p0_parse"]
CALLER = "l2_p1_build"
CALLEES = ["full_build", "full_parse"]

UNKNOWN = "[unknown]"


def parse_samples(path):
    samples = []
    cur = None
    with open(path, "r", errors="replace") as fh:
        for line in fh:
            if line.startswith("#"):
                continue
            if HEADER_RE.match(line):
                if cur is not None:
                    samples.append(cur)
                cur = []
                continue
            if cur is None:
                continue
            fm = FRAME_RE.match(line)
            if fm:
                cur.append((fm.group(2), fm.group(3)))
            elif line.strip() == "":
                samples.append(cur)
                cur = None
        if cur is not None:
            samples.append(cur)
    return samples


def detect_leaf_first(samples):
    """True if perf printed the leaf (innermost) frame first."""
    callee_first = 0
    caller_first = 0
    for frames in samples:
        syms = [s for s, _ in frames]
        caller_idx = next(
            (i for i, s in enumerate(syms) if any(m in s for m in REGION_MARKERS)), None
        )
        if caller_idx is None:
            continue
        is_p1 = "l2_p1_build" in syms[caller_idx]
        callees = ["full_build"] if is_p1 else ["full_parse", "parse_full"]
        callee_idx = next(
            (i for i, s in enumerate(syms) if i != caller_idx and any(c in s for c in callees)),
            None,
        )
        if callee_idx is None:
            continue
        if callee_idx < caller_idx:
            callee_first += 1
        else:
            caller_first += 1
        if callee_first + caller_first >= 20:
            break
    if callee_first + caller_first == 0:
        return None
    return callee_first > caller_first


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="inp", required=True)
    ap.add_argument("--out", dest="out", required=True)
    ap.add_argument("--quality", dest="quality", required=True)
    args = ap.parse_args()

    samples = parse_samples(args.inp)
    if not samples:
        sys.exit("no samples parsed from " + args.inp)
    leaf_first = detect_leaf_first(samples)
    if leaf_first is None:
        sys.exit("could not determine frame order from the static marker nesting")

    total = len(samples)
    in_region = 0
    unknown_in_region = 0
    leaf_unknown_in_region = 0
    kernel_frames_in_region = 0
    folded = {}

    for frames in samples:
        syms = [s for s, _ in frames]
        dsos = [d for _, d in frames]
        if leaf_first:
            syms.reverse()
            dsos.reverse()
        if not any(m in s for s in syms for m in REGION_MARKERS):
            continue
        in_region += 1
        if any(UNKNOWN in s for s in syms):
            unknown_in_region += 1
        if UNKNOWN in syms[-1]:
            leaf_unknown_in_region += 1
        kernel_frames_in_region += sum(
            1 for s, d in zip(syms, dsos) if UNKNOWN not in s and "kernel" in d
        )
        key = ";".join(syms)
        folded[key] = folded.get(key, 0) + 1

    with open(args.out, "w") as fh:
        for key, cnt in sorted(folded.items(), key=lambda kv: -kv[1]):
            fh.write(f"{key} {cnt}\n")

    quality = {
        "samples_total": total,
        "samples_in_region": in_region,
        "region_marker_fraction": round(in_region / total, 6),
        "resolved_fraction_in_region": round(1 - unknown_in_region / in_region, 6),
        "unknown_fraction_in_region": round(unknown_in_region / in_region, 6),
        "leaf_unknown_fraction_in_region": round(leaf_unknown_in_region / in_region, 6),
        "kernel_frames_in_region": kernel_frames_in_region,
        "frame_order": "leaf_first" if leaf_first else "root_first",
        "unique_paths_in_region": len(folded),
    }
    with open(args.quality, "w") as fh:
        json.dump(quality, fh, indent=2)
    print(json.dumps(quality, indent=2))


if __name__ == "__main__":
    main()
