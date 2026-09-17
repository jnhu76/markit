#!/usr/bin/env python3
"""verify_r3.py — R3 freeze-gate static validator.

Checks that the R3 artifacts are internally consistent:
  * fixtures: schema, unique ids, covers tags, sha256, tree invariants
  * manifests: TOML validity, unique ids, frozen enums, applicability
  * case expansion (content-deduped) == the frozen unique total (322)

This deliberately does NOT re-parse Markdown: the hand-authored expected
trees in the fixtures are the semantic authority; this script only checks
their internal consistency (bounds, containment, ordering, vocabulary).
It never benchmarks and never implements BENCH-GRAMMAR-v1 semantics.
"""
import hashlib
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SHAPES = (
    "plain", "many_blocks", "huge_block", "deep_container",
    "inline_dense", "fence_heavy", "reference_fanout", "mixed",
)
SIZES = {"64k": 65536, "1m": 1048576, "16m": 16777216}
FAMILIES = {
    "local_text", "block_boundary", "container_state",
    "forward_state", "inline_delimiter_state", "semantic_dependency",
}
ANCHORS = {"early", "middle", "late"}
KINDS = {
    "Document", "Paragraph", "Heading", "BlockQuote", "List", "ListItem",
    "FencedCode", "Text", "Emphasis", "CodeSpan", "Link", "ReferenceLink",
    "ReferenceDefinition",
}
COVER_TAGS = {
    "paragraph", "text", "heading", "blockquote", "list", "listitem",
    "fenced-code", "unclosed-fence", "emphasis", "code-span",
    "inline-link", "reference-link", "reference-definition",
    "reference-fanout", "mixed", "utf8", "cjk", "emoji", "deviation",
}
EXPECTED_TOTAL_CASES = 370

errors: list[str] = []


def err(msg: str) -> None:
    errors.append(msg)


def is_boundary(bs: bytes, i: int) -> bool:
    if i == 0 or i == len(bs):
        return True
    return (bs[i] & 0xC0) != 0x80


NODE_RE = re.compile(r"\((\w+) (\d+) (\d+)")


def parse_tree(s: str):
    pos = 0

    def ws():
        nonlocal pos
        while pos < len(s) and s[pos] in " \n":
            pos += 1

    def node():
        nonlocal pos
        ws()
        if pos >= len(s) or s[pos] != "(":
            raise ValueError(f"expected ( at {pos}")
        m = NODE_RE.match(s, pos)
        if not m:
            raise ValueError(f"bad node at {pos}")
        kind, a, b = m.group(1), int(m.group(2)), int(m.group(3))
        pos = m.end()
        fm = re.match(r"((?: \w+=[^)\s]+)*)(\)|\n)", s[pos:])
        if not fm:
            raise ValueError(f"bad fields at {pos}")
        fields = {}
        for tok in fm.group(1).split():
            k, v = tok.split("=", 1)
            fields[k] = v
        pos += fm.end(1)
        children = []
        ws()
        while pos < len(s) and s[pos] == "(":
            children.append(node())
            ws()
        if pos >= len(s) or s[pos] != ")":
            raise ValueError(f"expected ) at {pos}")
        pos += 1
        return (kind, a, b, fields, children)

    n = node()
    ws()
    if pos != len(s):
        raise ValueError("trailing content in tree")
    return n


def check_fixture_tree(src: bytes, root, fid: str) -> None:
    def walk(n, lo, hi, depth):
        kind, a, b, fields, children = n
        if kind not in KINDS:
            err(f"{fid}: unknown node kind {kind}")
        if not (lo <= a <= b <= hi):
            err(f"{fid}: {kind} span [{a},{b}) escapes parent [{lo},{hi})")
        if depth > 0 and kind != "Document" and a == b:
            err(f"{fid}: zero-length node {kind}")
        if not is_boundary(src, a):
            err(f"{fid}: {kind} start {a} not a UTF-8 char boundary")
        if not is_boundary(src, b):
            err(f"{fid}: {kind} end {b} not a UTF-8 char boundary")
        if kind == "Document" and (a, b) != (0, len(src)):
            err(f"{fid}: Document span {(a, b)} != (0, {len(src)})")
        if kind == "Heading" and "level" not in fields:
            err(f"{fid}: Heading missing level")
        if kind == "ListItem" and "marker" not in fields:
            err(f"{fid}: ListItem missing marker")
        if kind == "FencedCode" and ("info" not in fields or "content" not in fields):
            err(f"{fid}: FencedCode missing info/content")
        if kind in ("ReferenceLink", "ReferenceDefinition") and (
            "label" not in fields or "destination" not in fields
        ):
            err(f"{fid}: {kind} missing label/destination")
        if kind == "Link" and "destination" not in fields:
            err(f"{fid}: Link missing destination")
        prev_end = a
        for c in children:
            if c[1] < prev_end:
                err(f"{fid}: child {c[0]} starts before previous sibling ends")
            walk(c, a, b, depth + 1)
            prev_end = c[2]

    walk(root, 0, len(src), 0)


def check_fixtures() -> int:
    fixtures = sorted((ROOT / "grammar" / "fixtures").glob("*.toml"))
    if not 20 <= len(fixtures) <= 50:
        err(f"fixture count {len(fixtures)} outside the frozen 20-50 band")
    seen_ids = set()
    for path in fixtures:
        with open(path, "rb") as f:
            d = tomllib.load(f)
        fid = d.get("id", path.stem)
        if d.get("schema") != "r3-fixture-v1":
            err(f"{fid}: bad schema literal")
        if fid in seen_ids:
            err(f"{fid}: duplicate fixture id")
        seen_ids.add(fid)
        if not re.fullmatch(r"F\d{3}", fid):
            err(f"{fid}: id not F###")
        covers = d.get("covers", [])
        if not covers or not set(covers) <= COVER_TAGS:
            err(f"{fid}: bad covers {covers}")
        source = d.get("source", "")
        src = source.encode("utf-8")
        sha = hashlib.sha256(src).hexdigest()
        if d.get("source_sha256") != sha:
            err(f"{fid}: source_sha256 mismatch")
        if not source.endswith("\n"):
            err(f"{fid}: source does not end with LF")
        try:
            tree = parse_tree(d.get("expected_tree", "").strip())
        except ValueError as e:
            err(f"{fid}: expected_tree unparseable: {e}")
            continue
        check_fixture_tree(src, tree, fid)
    return len(fixtures)


def load_toml(rel: str):
    with open(ROOT / rel, "rb") as f:
        return tomllib.load(f)


def check_manifests() -> int:
    corpus = load_toml("corpus/manifest.toml")
    if corpus.get("schema") != "corpus-manifest-v1":
        err("corpus manifest: bad schema")
    if corpus.get("generator_version") != "corpus-gen-v1":
        err("corpus manifest: bad generator_version")
    ids = [c["corpus_id"] for c in corpus.get("corpus", [])]
    if len(ids) != len(set(ids)):
        err("corpus manifest: duplicate corpus_id")
    if len(ids) != 24:
        err(f"corpus manifest: expected 24 corpora, found {len(ids)}")
    for c in corpus.get("corpus", []):
        if c["shape"] not in SHAPES:
            err(f"corpus {c['corpus_id']}: unknown shape")
        if c["size_label"] not in SIZES:
            err(f"corpus {c['corpus_id']}: unknown size label")
        elif c["target_bytes"] != SIZES[c["size_label"]]:
            err(f"corpus {c['corpus_id']}: target_bytes != size label")
        if c["corpus_id"] != f"{c['shape']}-{c['size_label']}":
            err(f"corpus {c['corpus_id']}: id not shape-size")

    muts = load_toml("mutations/manifest.toml")
    if muts.get("schema") != "mutation-manifest-v1":
        err("mutation manifest: bad schema")
    mids = [m["mutation_id"] for m in muts.get("mutation", [])]
    if len(mids) != len(set(mids)):
        err("mutation manifest: duplicate mutation_id")
    for m in muts.get("mutation", []):
        if m["family"] not in FAMILIES:
            err(f"{m['mutation_id']}: unknown family")
        if m["op_label"] != "structural_edit":
            err(f"{m['mutation_id']}: op_label must be structural_edit")
        if not set(m["anchors"]) <= ANCHORS:
            err(f"{m['mutation_id']}: bad anchors")
        if not set(m["applicable_shapes"]) <= set(SHAPES):
            err(f"{m['mutation_id']}: bad applicable_shapes")
    mut_by_id = {m["mutation_id"]: m for m in muts.get("mutation", [])}
    if set(muts.get("families", [])) != FAMILIES:
        err("mutation manifest: families enum drift")

    cases = load_toml("cases/case-manifest-v1.toml")
    if cases.get("schema") != "case-manifest-v1":
        err("case manifest: bad schema")
    blocks = cases.get("block", [])
    if len(blocks) != 3:
        err("case manifest: expected 3 generic blocks (core/grid/scaling)")
    structural = cases.get("structural", [])
    sids = [s["mutation_id"] for s in structural]
    if len(sids) != len(set(sids)):
        err("case manifest: duplicate structural mutation")
    if set(sids) != set(mut_by_id):
        err("case manifest: structural rows != mutation manifest set")

    # Case identity is CONTENT-addressed (MUTATION-v1 §7): the frozen
    # total counts the UNIQUE case set; blocks may overlap on identical
    # content (e.g. the core TINY-INSERT also exists in the grid).
    unique: set = set()

    def add(shape, size, op, esize, pos, mut):
        unique.add((shape, size, op, esize, pos, mut))

    for b in blocks:
        kind = b.get("kind")
        if kind == "core":
            for c in b.get("cases", []):
                for shape in SHAPES:
                    for size in SIZES:
                        add(shape, size, c["op"], c.get("edit_size"), c.get("position"), None)
        elif kind == "generic_grid":
            for shape in b.get("shapes", []):
                for op in b.get("operations", []):
                    for es in b.get("edit_sizes", []):
                        for pos in b.get("positions", []):
                            add(shape, b["size"], op, es, pos, None)
        elif kind == "scaling_slice":
            for shape in b.get("shapes", []):
                for size in b.get("sizes", []):
                    for c in b.get("cases", []):
                        add(shape, size, c["op"], c.get("edit_size"), c.get("position"), None)
        else:
            err(f"case manifest: unknown block kind {b.get('kind')}")

    for s in structural:
        sid = s["mutation_id"]
        m = mut_by_id[sid]
        if sorted(s.get("anchors", [])) != sorted(m["anchors"]):
            err(f"{sid}: anchors drift vs mutation manifest")
        if not set(s["applicable_shapes"]) <= set(m["applicable_shapes"]):
            err(f"{sid}: case shapes exceed mutation applicability")
        for shape in s["applicable_shapes"]:
            for size in SIZES:
                for anchor in s["anchors"]:
                    add(shape, size, "structural_edit", None, anchor, sid)

    # apply frozen NOT_APPLICABLE exceptions
    for s in structural:
        sid = s["mutation_id"]
        for e in s.get("exceptions", []):
            if e.get("applicability") != "NOT_APPLICABLE":
                err(f"{sid}: unknown exception applicability {e.get('applicability')}")
                continue
            for size in e.get("sizes", []):
                for anchor in mut_by_id[sid]["anchors"]:
                    unique.discard((e["shape"], size, "structural_edit", None, anchor, sid))

    total = len(unique)
    if total != EXPECTED_TOTAL_CASES:
        err(f"unique case expansion {total} != frozen total {EXPECTED_TOTAL_CASES}")
    return total


def main() -> int:
    n_fix = check_fixtures()
    n_cases = check_manifests()
    print(f"fixtures checked:        {n_fix}")
    print(f"unique cases expanded:   {n_cases} (frozen total {EXPECTED_TOTAL_CASES})")
    if errors:
        print(f"R3 FREEZE GATE: FAIL ({len(errors)} problems)")
        for e in errors:
            print("  -", e)
        return 1
    print("R3 FREEZE GATE: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
