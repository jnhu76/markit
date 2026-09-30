# horse-a-v2-l2-100-erratum — derived/audit evidence namespace

Append-only derived-evidence namespace for
[docs/research/horse-a-v2-l2-evidence-erratum-2026-09-30.md](../../../../../docs/research/horse-a-v2-l2-evidence-erratum-2026-09-30.md)
(issue #100 CLOSED; erratum only — no reopening, no recollection, no
mechanism work).

## Contents

| file | kind | authority |
|---|---|---|
| `erratum.json` | machine-readable erratum summary (E1–E6, status block) | summary of the erratum document |
| `warmup_audit.json` | mechanical E1 audit facts, each tied to source file + SHA-256 | AUDIT ONLY |
| `phase_window.json` | phase-window reconstruction: replay gate + A/B/C normalizations, all 12 profiles | SENSITIVITY ONLY |
| `sensitivity.md` | human-readable sensitivity analysis | SENSITIVITY ONLY |
| `erratum_phase_window.py` | replayable derivation for `phase_window.json` | tool |
| `make_warmup_audit.py` | replayable generator for `warmup_audit.json` | tool |
| `SHA256SUMS` | checksums of this namespace | integrity |

## Derivation provenance

```text
source historical artifacts (NOT modified by this namespace):
  research/benchmarks/markdown-ast-update/results/horse-a-v2-l2-100/
      campaign/mid/perf_{cell}_{arm}_{rep}.txt        (12)
      campaign/receipts/attribution_{...}.json        (12)
      campaign/receipts/run_{...}.json                (12)
      campaign/receipts/quality_{...}.json            (12)
  per-file SHA-256 recorded inside warmup_audit.json
  namespace identity of the historical evidence:
      git tree 0d6f0d28400d3a801a2c132fc372d717b9afbf9e @ fb359de

calculation:
  phase_window: perf_fold-identical parse of mid files ->
    classify.py (frozen, imported verbatim) per-sample bucketing ->
    (A) historical /region_ops, (B) /(region_ops+warmup_ops),
    (C) timestamp window [last_in_region - region_wall_ns, end] / region_ops
  denominators: A = region_ops (100 / 100 / 1500 per cell, formal only)
                B = region_ops + warmup_ops (120 / 120 / 1530, all profiled ops)
                C = region_ops (formal), numerator restricted to the window
  assumptions: C assumes the formal region occupies the final region_wall_ns
    of the sampled in-region window (boundary inferred from the run
    receipt; no phase marker was recorded at collection time)

validation:
  replay gate — reproduced per-bucket counts equal the committed
  attribution buckets integer-exact on 12/12 profiles before any phase
  claim is derived

status: SENSITIVITY / AUDIT ONLY — nothing here replaces the historical
  campaign, reverses an L2 verdict, or authorizes a mechanism
```

Reproduce with `python3 erratum_phase_window.py && python3 make_warmup_audit.py`
(reads only the retained L2 evidence; writes only inside this directory).
