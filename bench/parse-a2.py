#!/usr/bin/env python3
"""Parse Phase A2 JSONL instrumentation streams (stdin) into summaries.

Two line families (both emitted as JSON with "perf":1):

  PocketJS host (per tick, PJS_PERF=1):
    {"perf":1,"tick":N,"ev":E,"in":I,"gf_us":..,"ct_us":..,"dl_us":..,
     "words":..,"dirty":D,"r_us":..}
    gf_us  guest JS turn (guest.frame)
    ct_us  core tick (surface.tick)
    dl_us  DrawList build + FNV hash
    words  DrawList word count
    r_us   last render pass (GPU), us

    plus one "[perf] {...}" reply line with guest work counters.

  GPUI (per render frame, GPUI_A2=1):
    {"perf":1,"prepaint_us":..,"shape_us":..,"lines_shaped":..,"glyphs":..,
     "first":..,"visible":..,"last":..,"lines_total":..,"quads":..,
     "paint_us":..[, "concat_us":..,"lines_us":..,"scan_chars":..,
     "lines_recreated":..,"doc_len":..]}
    concat_us+lines_us present when the frame followed a document edit.

Output: per-family percentile tables + the guest perf reply (raw).
Usage: mvp-... | python3 bench/parse-a2.py [--pjs|--gpui]
"""

import json
import re
import sys


def percentile(sorted_vals: list, p: float) -> float:
    if not sorted_vals:
        return float("nan")
    k = (len(sorted_vals) - 1) * p
    lo = int(k)
    hi = min(lo + 1, len(sorted_vals) - 1)
    frac = k - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def stats(name: str, vals: list) -> None:
    if not vals:
        print(f"  {name:22s} n=0")
        return
    v = sorted(vals)
    print(
        f"  {name:22s} n={len(v):5d} p50={percentile(v, 0.50):10.1f} "
        f"p95={percentile(v, 0.95):10.1f} p99={percentile(v, 0.99):10.1f} "
        f"max={v[-1]:10.1f}"
    )


def main() -> None:
    family = "--gpui" in sys.argv and "gpui" or "pjs"
    pjs_edit: dict[str, list] = {
        "gf_us": [], "ct_us": [], "dl_us": [], "words": [], "r_us": [],
    }
    pjs_idle: dict[str, list] = {"gf_us": [], "ct_us": [], "dl_us": []}
    gpui_edit: dict[str, list] = {
        "concat_us": [], "lines_us": [], "prepaint_us": [], "shape_us": [],
        "paint_us": [], "lines_shaped": [], "glyphs": [], "quads": [],
    }
    gpui_static: dict[str, list] = {
        "prepaint_us": [], "shape_us": [], "paint_us": [], "lines_shaped": [],
    }
    gpui_all: dict[str, list] = {
        "lines_shaped": [], "glyphs": [], "visible": [], "first": [], "last": [],
        "lines_total": [],
    }
    perf_reply: list[str] = []
    first_words = None
    last_words = None

    for line in sys.stdin:
        line = line.strip()
        m = re.match(r"^\[perf\] (.*)$", line)
        if m:
            perf_reply.append(m.group(1))
            continue
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("perf") != 1:
            continue
        if "gf_us" in obj:
            # PocketJS tick line
            is_edit = obj.get("ev", 0) > 0 or obj.get("in", 0) > 0
            bucket = pjs_edit if is_edit else pjs_idle
            for k in bucket:
                bucket[k].append(obj.get(k, 0))
            if first_words is None:
                first_words = obj.get("words")
            last_words = obj.get("words")
        else:
            # GPUI render line
            if "concat_us" in obj:
                bucket = gpui_edit
                for k in ("concat_us", "lines_us", "prepaint_us", "shape_us",
                          "paint_us", "lines_shaped", "glyphs", "quads"):
                    bucket[k].append(obj.get(k, 0))
            else:
                bucket = gpui_static
                for k in ("prepaint_us", "shape_us", "paint_us", "lines_shaped"):
                    bucket[k].append(obj.get(k, 0))
            for k in ("lines_shaped", "glyphs", "visible", "first", "last",
                      "lines_total"):
                gpui_all[k].append(obj.get(k, 0))

    if family == "pjs":
        print("=== PocketJS (per tick) ===")
        print("edit ticks:")
        for k in ("gf_us", "ct_us", "dl_us", "words", "r_us"):
            stats(k, pjs_edit[k])
        print("idle ticks:")
        for k in ("gf_us", "dl_us"):
            stats(k, pjs_idle[k])
        print(f"  words: first={first_words} last={last_words}")
        if perf_reply:
            print("=== guest perf reply ===")
            for r in perf_reply:
                print(f"  {r}")
    else:
        print("=== GPUI (per frame) ===")
        print("edit frames:")
        for k in ("concat_us", "lines_us", "prepaint_us", "shape_us",
                  "paint_us", "lines_shaped", "glyphs", "quads"):
            stats(k, gpui_edit[k])
        print("static frames (no edit):")
        for k in ("prepaut_us", "prepaint_us", "shape_us", "paint_us", "lines_shaped"):
            stats(k, gpui_static[k])
        print("all frames:")
        for k in ("lines_shaped", "glyphs", "visible", "first", "last",
                  "lines_total"):
            vals = gpui_all[k]
            if vals:
                v = sorted(vals)
                print(
                    f"  {k:22s} n={len(v):5d} min={v[0]:8.0f} "
                    f"p50={percentile(v, 0.50):8.0f} max={v[-1]:8.0f}"
                )
    return 0


if __name__ == "__main__":
    sys.exit(main())
