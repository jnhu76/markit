#!/usr/bin/env python3
"""HORSE-A-STRUCTURAL-LOCALITY-1 mechanical derivation (task §41/§42/§43).

Pure mechanical derivation from the immutable raw rows + the frozen
#60 authority. NO performance data are read or interpreted. Produces:

  derived/raw-manifest.txt              path / sha256 / row identity / cell / rep / status
  derived/cell-verdicts.txt             per-cell determinism + verdict + overall verdict
  derived/structural-verdict-report.txt the full §43 report

Every threshold below is a mechanical transcription of the synchronized
#60 §9.5 table (corrected authority: ACCOUNTING-CORRECTION-1 / PR #72
@ 04b6496 / spec §16) — the same values frozen in the reviewed producer
(markit-mdbench-horse-a producer::contract). This script is an
independent cross-check of the producer-embedded adjudication; the
frozen Rust adjudicator remains the authority.
"""

import hashlib
import json
import sys
from pathlib import Path

STUDY_DIR = Path(__file__).resolve().parent.parent
RAW = STUDY_DIR / "raw"
DERIVED = STUDY_DIR / "derived"

STUDY_ID = "2e061da9cb6fbe57f9ce139ca02382fda4cad672fd9422e6c7d7b885f42f4e17"
CONTRACT_REV = (
    "HORSE-A-FAILURE-FIRST-1/#60@sync-20260928T021211Z"
    "/master-f7fdcdaabc5d761435f3c0e8c17611973642934b"
    "/tree-9ec58ed228972c323da5cccf0b991235a0bc0e8a"
)
EXEC_MASTER = "745098f112e559d930c5242e43b71f08bf514560"
EXEC_TREE = "bb98ce87f2cd50471460c15ab6e06d59acbdf525"
EXEC_SHA = "fc41cb49a9b869973955db569efb91b513fdfe17d91ba5120a8b976e0d9a4e3b"
STUDY_BASE = "8e40932239273cc798795a625244ffb3e12b5f0e"
AUTH_BASE = "f7fdcdaabc5d761435f3c0e8c17611973642934b"
INSERTED_SHA = "c129db8be8904b40ac21c9cf5d9f5c0e24ef455d1d7a7bbfd7049fc6dc9d2429"

# Frozen #60 §9.5 table per cell: (thresholds, exact geometry, exact values).
CELLS = {
    "H4N-128KiB": dict(n=131072, m=1024, target=512, edit=65599, hmax=14,
        restart=65408, q_old=65664, q_new=65672, lo=511, hi=513,
        pre="58b39c389bfe5d5cbcfdbbd74150d276c7996e1b25a84a467fa4deb3cec29a30",
        post="1ca6bb1f138507758230ba868c6a55bb9bc824b94fe0b9c75ff3d39c8fb777ea",
        case="22e51081397aebb06a1c4c3f85bd08a6e7041a438f4aae416970027e1ddbb24e",
        t=dict(locate=14, safe=40, f1=54, cursor=38, fact=40, split=280,
               pivot=110, join=110, f2=500, rot=248, links=1005,
               reads=5650, writes=2328, depth=14)),
    "H4N-1MiB": dict(n=1048576, m=8192, target=4096, edit=524351, hmax=18,
        restart=524160, q_old=524416, q_new=524424, lo=4095, hi=4097,
        pre="681c2c032bd1585deef29cf2ce9f9c2a3b8c4bf163ca54c8001d45309462b294",
        post="e3e0d2ddf4717cc9bbd0e60848ada7b542500d143134b39609cd2537da8c174c",
        case="337a179ec729c20c01309d98317f7ca00691d12ca10285b55ade71fb880e191c",
        t=dict(locate=18, safe=52, f1=70, cursor=46, fact=52, split=360,
               pivot=142, join=142, f2=644, rot=320, links=1293,
               reads=7274, writes=3000, depth=18)),
    "H4N-16MiB": dict(n=16777216, m=131072, target=65536, edit=8388671, hmax=24,
        restart=8388480, q_old=8388736, q_new=8388744, lo=65535, hi=65537,
        pre="0fdca64e71df4386d4407afa1dd5e72aa0faaf60d6854a1fab730d580be6c78d",
        post="c8014b5ecae8ca489cb083e6a1a1f8170b3fd2d8d115a85ffa4c58ab4bb29eb0",
        case="0c00f8138e0e622f6fc7215fc0f0d135e79bf60450e4c79d30e30624a8842117",
        t=dict(locate=24, safe=70, f1=94, cursor=58, fact=70, split=480,
               pivot=190, join=190, f2=860, rot=428, links=1725,
               reads=9710, writes=4008, depth=24)),
}

SENTINELS = [f"forbidden_{k}" for k in [
    "prefix_sequential_enumeration", "suffix_sequential_enumeration",
    "unaffected_payload_inspections", "unaffected_coordinate_writes",
    "unaffected_certificate_writes", "global_fact_recollection",
    "unaffected_old_retirement", "attribution_tree_walk"]]

COUNTER_FIELDS = [
    "m_old", "m_new", "h_old", "h_new", "restart_old", "convergence_old",
    "convergence_new", "replace_lo", "replace_hi", "full_build_selected",
    "full_build_reason", "locate_node_visits",
    "safe_predecessor_node_visits", "cursor_node_visits",
    "fact_range_node_visits", "split_node_visits",
    "pivot_extract_node_visits", "join_node_visits", "bulk_build_node_visits",
    "retire_node_visits", "sequence_link_writes", "avl_rotations",
    "aggregate_reads", "aggregate_writes", "certificate_reads",
    "certificate_writes", "candidate_checks", "cursor_advances",
    "owners_created", "owners_removed", "fresh_payload_nodes_final",
    "fresh_payload_nodes_temporary", "payload_nodes_retired",
    "old_fact_owner_visits", "old_facts_extracted", "new_facts_extracted",
    "facts_compared", "fact_compare_bytes", "reftable_entries_visited",
    "retirement_frames_entered", "max_retirement_depth"] + SENTINELS


def known(row, name):
    v = row["counters"].get(name)
    if v is None or v == "Unknown":
        return None
    assert v.startswith("Known(") and v.endswith(")"), f"malformed {name}: {v}"
    return int(v[6:-1])


def sha256_file(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def reverify(cell, row):
    """Independent cross-check of one row against the frozen table."""
    c, t, problems = cell, cell["t"], []
    for name, want in [
        ("restart_old", c["restart"]), ("convergence_old", c["q_old"]),
        ("convergence_new", c["q_new"]), ("replace_lo", c["lo"]),
        ("replace_hi", c["hi"]), ("full_build_selected", 0),
        ("full_build_reason", 0), ("candidate_checks", 2),
        ("cursor_advances", 2), ("owners_created", 2), ("owners_removed", 2),
        ("bulk_build_node_visits", 2), ("fresh_payload_nodes_final", 4),
        ("payload_nodes_retired", 4), ("old_facts_extracted", 0),
        ("new_facts_extracted", 0), ("facts_compared", 0),
        ("fact_compare_bytes", 0), ("reftable_entries_visited", 0),
        ("m_old", c["m"]), ("m_new", c["m"]),
    ]:
        if known(row, name) != want:
            problems.append(f"exact:{name}")
    for name, bound in [
        ("locate_node_visits", t["locate"]),
        ("safe_predecessor_node_visits", t["safe"]),
        ("cursor_node_visits", t["cursor"]),
        ("fact_range_node_visits", t["fact"]),
        ("split_node_visits", t["split"]),
        ("pivot_extract_node_visits", t["pivot"]),
        ("join_node_visits", t["join"]),
        ("retire_node_visits", 2), ("sequence_link_writes", t["links"]),
        ("avl_rotations", t["rot"]), ("aggregate_reads", t["reads"]),
        ("aggregate_writes", t["writes"]), ("certificate_reads", 5),
        ("certificate_writes", 2), ("old_fact_owner_visits", 4),
        ("fresh_payload_nodes_temporary", 4),
        ("retirement_frames_entered", 6), ("max_retirement_depth", t["depth"]),
        ("h_old", c["hmax"]), ("h_new", c["hmax"]),
    ]:
        v = known(row, name)
        if v is None or v > bound:
            problems.append(f"threshold:{name}")
    for name in SENTINELS:
        v = known(row, name)
        if v is None or v != 0:
            problems.append(f"sentinel:{name}")
    for name in COUNTER_FIELDS:
        if row["counters"].get(name) is None:
            problems.append(f"missing:{name}")
    if known(row, "locate_node_visits") is not None and known(row, "safe_predecessor_node_visits") is not None:
        if known(row, "locate_node_visits") + known(row, "safe_predecessor_node_visits") > t["f1"]:
            problems.append("composite:f1")
    if all(known(row, n) is not None for n in ("split_node_visits", "pivot_extract_node_visits", "join_node_visits")):
        if known(row, "split_node_visits") + known(row, "pivot_extract_node_visits") + known(row, "join_node_visits") > t["f2"]:
            problems.append("composite:f2")
    if row["correctness"] != {"c1": "PASS", "c2": "PASS", "c3": "PASS",
                              "normalized_structural_equality": True}:
        problems.append("correctness")
    if row["route"] != "local" or row["full_build_reason"] != "none":
        problems.append("route")
    return problems


def main():
    lines_manifest, lines_verdicts, lines_report = [], [], []
    failures = []
    cells_out = {}

    lines_report.append("HORSE-A #60 FIRST STRUCTURAL COLLECTION — MECHANICAL "
                        "DERIVATION (from immutable raw rows + frozen #60 table)")
    lines_report.append("")

    for cell_id, cell in CELLS.items():
        rep_rows, rep_status = [], []
        for rep in (0, 1, 2):
            p = RAW / f"{cell_id}-rep{rep}.json"
            if not p.exists():
                failures.append(f"missing row {p}")
                rep_status.append("MISSING")
                rep_rows.append(None)
                continue
            row = json.loads(p.read_text())
            rep_rows.append(row)
            # §41 identity verification.
            checks = {
                "schema": row["schema"] == "HORSE-A-STRUCTURAL-RAW-v1",
                "study_id": row["study_id"] == STUDY_ID,
                "contract_rev": row["frozen_contract_revision"] == CONTRACT_REV,
                "study_base": row["study_mechanism_baseline"] == STUDY_BASE,
                "auth_base": row["authorization_baseline"] == AUTH_BASE,
                "commit": row["repository_commit"] == EXEC_MASTER,
                "tree": row["repository_tree"] == EXEC_TREE,
                "executable": row["executable_sha256"] == EXEC_SHA,
                "cell": row["cell_id"] == cell_id
                        and row["case_id_hex"] == cell["case"]
                        and row["n_bytes"] == cell["n"]
                        and row["m"] == cell["m"] and row["target"] == cell["target"]
                        and row["edit_start"] == cell["edit"]
                        and row["edit_end"] == cell["edit"],
                "hashes": row["pre_sha256"] == cell["pre"]
                          and row["post_sha256"] == cell["post"]
                          and row["inserted_text_sha256"] == INSERTED_SHA,
                "repetition": row["repetition"] == rep,
                "row_identity": row["row_identity"] == hashlib.sha256(
                    f"{STUDY_ID}\n{cell_id}\n{rep}\n{EXEC_SHA}".encode()).hexdigest(),
                "counter_schema": row["counter_schema"] == "HORSE-A-STRUCTURAL-COUNTERS-v1",
            }
            bad = [k for k, ok in checks.items() if not ok]
            status = row["adjudication"]["status"]
            problems = reverify(cell, row)
            if problems:
                bad.append("reverify:" + ",".join(problems))
            if bad:
                failures.append(f"{cell_id} rep{rep}: {bad}")
            if status == "PASS" and bad:
                status = "FAIL(reverify)"
            rep_status.append(status)
            lines_manifest.append(
                f"{p.name}  sha256={sha256_file(p)}  row_identity={row['row_identity']}  "
                f"cell={cell_id}  repetition={rep}  status={status}")

        # §42 determinism: exact equality of counters + route/reason.
        valid = [r for r, s in zip(rep_rows, rep_status) if r is not None and s != "INVALID"]
        det = (len(valid) == 3
               and all(r["counters"] == valid[0]["counters"] for r in valid)
               and all(r["route"] == valid[0]["route"] for r in valid)
               and all(r["full_build_reason"] == valid[0]["full_build_reason"] for r in valid))
        cell_pass = all(s == "PASS" for s in rep_status) and det
        cells_out[cell_id] = cell_pass

        c0 = valid[0] if valid else None
        key = lambda r: None if r is None else {  # headline counters for the report
            "locate": r["counters"]["locate_node_visits"],
            "safe": r["counters"]["safe_predecessor_node_visits"],
            "cursor": r["counters"]["cursor_node_visits"],
            "fact_range": r["counters"]["fact_range_node_visits"],
            "split": r["counters"]["split_node_visits"],
            "pivot": r["counters"]["pivot_extract_node_visits"],
            "join": r["counters"]["join_node_visits"],
            "rotations": r["counters"]["avl_rotations"],
            "links": r["counters"]["sequence_link_writes"],
            "reads": r["counters"]["aggregate_reads"],
            "writes": r["counters"]["aggregate_writes"],
            "h": r["counters"]["h_old"] + "/" + r["counters"]["h_new"],
        }
        lines_verdicts.append(
            f"{cell_id}: rep0={rep_status[0]} rep1={rep_status[1]} rep2={rep_status[2]}  "
            f"DETERMINISTIC={'YES' if det else 'NO'}  CELL_STRUCTURAL_VERDICT="
            f"{'PASS' if cell_pass else 'FAIL'}")
        lines_report.append(f"CELL = {cell_id}")
        lines_report.append(f"  REP1 = {rep_status[0]}   REP2 = {rep_status[1]}   REP3 = {rep_status[2]}")
        lines_report.append(f"  COUNTERS_DETERMINISTIC = {'YES' if det else 'NO'}")
        if c0:
            for k, v in key(c0).items():
                lines_report.append(f"  {k} = {v}")
        lines_report.append("  CORRECTNESS = C1/C2/C3 PASS + normalized equality (all reps)"
                            if all(r and r["correctness"]["c1"] == "PASS" and
                                   r["correctness"]["c2"] == "PASS" and
                                   r["correctness"]["c3"] == "PASS" for r in valid)
                            else "  CORRECTNESS = FAILURE")
        lines_report.append(f"  CELL_STRUCTURAL_VERDICT = {'PASS' if cell_pass else 'FAIL'}")
        lines_report.append("")

    overall = "STRUCTURAL_PASS" if all(cells_out.values()) else "STRUCTURAL_FAIL"
    lines_verdicts.append(f"OVERALL_STRUCTURAL_VERDICT = {overall}")
    lines_report.append(f"OVERALL_STRUCTURAL_VERDICT = {overall}")
    lines_report.append("(all three cells PASS and deterministic; no weighting, "
                        "no averaging, no latency rescue)")
    lines_report.append("")
    if failures:
        lines_report.append("REVERIFICATION FAILURES:")
        lines_report.extend("  " + f for f in failures)

    DERIVED.mkdir(exist_ok=True)
    (DERIVED / "raw-manifest.txt").write_text("\n".join(lines_manifest) + "\n")
    (DERIVED / "cell-verdicts.txt").write_text("\n".join(lines_verdicts) + "\n")
    (DERIVED / "structural-verdict-report.txt").write_text("\n".join(lines_report) + "\n")
    print("\n".join(lines_verdicts))
    if failures:
        print("\nFAILURES:", *failures, sep="\n  ")
        sys.exit(1)


if __name__ == "__main__":
    main()
