#!/usr/bin/env python3
"""Issue #100 L2 — context attribution over folded stacks.

Consumes a folded-stacks file (root -> leaf, from perf_fold.py) and
produces the machine-readable context attribution:

- every in-region sample is attributed to ONE calling-context bucket by
  an explicit per-arm rule chain over the full calling path (the same
  leaf under different parent paths stays separate; full paths are
  retained and ranked);
- runtime frames (allocator/memmove) are stripped before the anchor so
  allocation cost lands under the responsibility that caused it;
- teardown (the per-iteration drop) is bucketed separately so the
  construction attribution stays comparable to the #98 probe semantics
  (build -> black_box -> untimed drop).

Frame names are the bare DWARF method names perf's --inline expansion
prints (e.g. `persist_interior_certificates`, `run<NoopWorkSink>`).
"""

import argparse
import json

RUNTIME_TOKENS = [
    "__rust_alloc", "__rust_realloc", "__rust_dealloc",
    "alloc::alloc::", "alloc::boxed::", "alloc::raw_vec::",
    "alloc::vec::", "alloc::slice::", "alloc::string::",
    "core::alloc::", "realloc", "malloc", "calloc",
    "cfree", "_int_", "sysmalloc", "malloc_consolidate",
    "memcpy", "memmove", "memset", "__mem", "bcmp", "strcmp",
]

DROP_TOKEN = "drop_in_place"

REGION_MARKERS = ["l2_p1_build", "l2_p0_parse"]

# Ordered rule chains (first match wins) over the entry-relative body.
P1_RULES = [
    (["interior_certificates"], "certificate"),
    (["shift_spans"], "span_coordinate"),
    (["CoveragePlan", "horse_a::coverage"], "coverage"),
    (["bulk_build", "OwnerSeq"], "avl_ownerseq"),
    (["materialize_one_with_sink"], "owner_materialize"),
    (["parse_region_observed"], "block_parse_shared"),
    (["full_build"], "full_build_other"),
]

P0_RULES = [
    (["finish_document_with_sink", "oracle::normalized", "NormalizedDocument"], "h0_document_materialize"),
    (["parse_full", "full_parse", "parse_with_attribution"], "block_parse_shared"),
    (["complete"], "h0_complete"),
]

CONSTRUCTION_TOKENS = [
    "full_build", "full_parse", "parse_full", "parse_region_observed",
    "finish_document_with_sink", "materialize_one_with_sink",
]


def is_runtime(sym):
    return any(t in sym for t in RUNTIME_TOKENS)


def classify_stack(path):
    frames = path.split(";")
    region_idx = max(i for i, f in enumerate(frames) if any(m in f for m in REGION_MARKERS))
    seg = frames[region_idx:]  # entry frame + below
    body = seg[1:]             # below the entry marker

    has_construction = any(t in f for f in body for t in CONSTRUCTION_TOKENS)

    # strip trailing runtime/drop frames to find the anchor
    anchor = len(body)
    while anchor > 0 and (is_runtime(body[anchor - 1]) or DROP_TOKEN in body[anchor - 1]):
        anchor -= 1
    body_anchor = body[:anchor]

    if not has_construction:
        if any(DROP_TOKEN in f for f in body):
            return "teardown"
        return "region_loop_other"
    if not body_anchor:
        return "region_loop_other"

    is_p1 = REGION_MARKERS[0] in seg[0]
    rules = P1_RULES if is_p1 else P0_RULES
    for tokens, bucket in rules:
        if any(t in f for f in body_anchor for t in tokens):
            return bucket
    return "h0_other" if not is_p1 else "full_build_other"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--folded", required=True)
    ap.add_argument("--cell", required=True)
    ap.add_argument("--arm", required=True)
    ap.add_argument("--reps", type=int, required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    paths = {}
    samples_total = 0
    with open(args.folded) as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line:
                continue
            path, _, cnt = line.rpartition(" ")
            cnt = int(cnt)
            samples_total += cnt
            paths[path] = paths.get(path, 0) + cnt

    buckets = {}
    in_region = 0
    for path, cnt in paths.items():
        b = classify_stack(path)
        if b is None:
            continue
        d = buckets.setdefault(b, {"samples": 0, "paths": {}})
        d["samples"] += cnt
        d["paths"][path] = d["paths"].get(path, 0) + cnt
        in_region += cnt

    out = {
        "cell": args.cell,
        "arm": args.arm,
        "reps": args.reps,
        "folded_samples_total": samples_total,
        "samples_in_region": in_region,
        "samples_per_op_in_region": round(in_region / args.reps, 4),
        "buckets": {},
        "top_paths": [],
    }
    for b, d in buckets.items():
        out["buckets"][b] = {
            "samples": d["samples"],
            "samples_per_op": round(d["samples"] / args.reps, 4),
            "share_of_region": round(d["samples"] / in_region, 6) if in_region else None,
            "top_paths": [
                {"path": p, "samples": c}
                for p, c in sorted(d["paths"].items(), key=lambda kv: -kv[1])[:5]
            ],
        }
    out["top_paths"] = [
        {"path": p, "samples": c, "samples_per_op": round(c / args.reps, 4)}
        for p, c in sorted(paths.items(), key=lambda kv: -kv[1])
    ][:80]

    with open(args.out, "w") as fh:
        json.dump(out, fh, indent=2)
    print(f"{args.cell} {args.arm}: in_region={in_region} per_op={out['samples_per_op_in_region']}")
    for b in sorted(buckets, key=lambda x: -buckets[x]["samples"]):
        print(f"  {b:26s} {buckets[b]['samples']:8d}  {buckets[b]['samples']/args.reps:9.3f}/op")


if __name__ == "__main__":
    main()
