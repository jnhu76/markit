#!/usr/bin/env python3
"""verify_r3.py — R3 freeze-gate static validator.

Checks that the R3 artifacts are internally consistent:
  * fixtures: schema, unique ids, covers tags, sha256, tree invariants
  * manifests: TOML validity, unique ids, frozen enums, applicability
  * case expansion: recipe-slot expansion == `expected_unique_cases`
    (single machine-readable authority in cases/case-manifest-v1.toml),
    cross-checked against the total stated in CASE-MATRIX-v1.md
  * corrective-1 regressions (recipe arithmetic on small in-memory
    units — NOT corpus generation, NOT a Markdown parser):
      - every declared unit variant has its declared byte size
        (fence_heavy ASCII AND CJK units == 512, all eight shapes)
      - deep_container max list depth == 16 and max quote depth == 16
        by recipe (grammar §7 indent arithmetic / §6 marker chain)
      - M-FS-FENCE-CLOSE precondition satisfiable on fence_heavy and
        mixed; its post-edit unit releases the last body line from raw
        fence content with the old closer gone
      - generic anchors (raw + 7) are never on unit/mountain/tile
        boundaries; PLAIN anchors land in paragraph content; FENCE_HEAVY
        anchors land in fence body content
      - totals agree everywhere (no stale duplicated counts)

The hand-authored expected trees in the fixtures remain the semantic
authority; this script checks internal consistency and the frozen
recipe arithmetic. It never benchmarks and never implements
BENCH-GRAMMAR-v1 semantics.
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

# R4-H0-REFERENCE-CORRECTIVE-1: the frozen field-kind legality table
# (NORMALIZED-RESULT-v1 §1). For every node: the listed fields are the
# ONLY fields the kind may carry (anything else is FORBIDDEN) and every
# listed field is REQUIRED. Must stay identical to `allowed_fields` in
# oracle/src/validate.rs — the Rust twin enforces the same table on
# runtime results.
FIELD_TABLE = {
    "Document": set(),
    "Paragraph": set(),
    "Heading": {"level"},
    "BlockQuote": set(),
    "List": set(),
    "ListItem": {"marker"},
    "FencedCode": {"info", "content"},
    "Text": set(),
    "Emphasis": set(),
    "CodeSpan": set(),
    "Link": {"destination"},
    "ReferenceLink": {"label", "destination"},
    "ReferenceDefinition": {"label", "destination"},
}

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
            if k in fields:
                raise ValueError(f"duplicate field key {k!r}")
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
        # R4-H0-REFERENCE-CORRECTIVE-1 §3: a normalized node span is
        # NEVER zero-length (nor inverted) — for EVERY kind, root
        # included. The only zero-length interval in the vocabulary is
        # the FencedCode `content` FIELD, validated separately below.
        if a >= b:
            err(f"{fid}: zero-length or inverted node span {kind} [{a},{b})")
        if not is_boundary(src, a):
            err(f"{fid}: {kind} start {a} not a UTF-8 char boundary")
        if not is_boundary(src, b):
            err(f"{fid}: {kind} end {b} not a UTF-8 char boundary")
        if kind == "Document" and (a, b) != (0, len(src)):
            err(f"{fid}: Document span {(a, b)} != (0, {len(src)})")
        # exact field-kind legality (R4-H0-REFERENCE-CORRECTIVE-1 §2):
        # required fields exist, forbidden fields are absent
        allowed = FIELD_TABLE.get(kind, set())
        missing = allowed - fields.keys()
        if missing:
            err(f"{fid}: {kind} missing required field(s) {sorted(missing)}")
        forbidden = fields.keys() - allowed
        if forbidden:
            err(
                f"{fid}: {kind} carries forbidden field(s) {sorted(forbidden)} "
                f"(kind allows only {sorted(allowed)})"
            )
        # the FencedCode `content` FIELD: containment + order inside the
        # node span, UTF-8 boundaries; EMPTY (cs == ce) is legal
        if kind == "FencedCode" and "content" in fields:
            m = re.fullmatch(r"(\d+):(\d+)", fields["content"])
            if not m:
                err(f"{fid}: FencedCode content {fields['content']!r} is not start:end")
            else:
                cs, ce = int(m.group(1)), int(m.group(2))
                if not (a <= cs <= ce <= b):
                    err(
                        f"{fid}: FencedCode content [{cs},{ce}) not inside/ordered "
                        f"vs node span [{a},{b})"
                    )
                elif not (is_boundary(src, cs) and is_boundary(src, ce)):
                    err(f"{fid}: FencedCode content bound not a UTF-8 char boundary")
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


def _gate_errors(tree_text: str, src: bytes) -> list:
    """Run the fixture gate on one synthetic tree, collecting its errors
    without touching the global artifact-error list."""
    global errors
    tree = parse_tree(tree_text)
    saved = errors
    errors = []
    try:
        check_fixture_tree(src, tree, "SELFTEST")
        return list(errors)
    finally:
        errors = saved


def check_gate_self_test() -> tuple:
    """R4-H0-REFERENCE-CORRECTIVE-1 §2/§3 regressions, run against the
    gate itself on synthetic trees (independent of the 43 artifacts):
    forbidden/missing fields and zero-length node spans must FAIL; the
    corrected CodeSpan shape and an empty FencedCode content interval
    must PASS."""
    src = b"a`b`cde\n"  # 8 bytes

    must_fail = [
        ("CodeSpan content= (the corrected MAJOR must never return)",
         "(Document 0 8\n  (Paragraph 0 7\n    (CodeSpan 0 5 content=1:4)))"),
        ("Text destination=",
         '(Document 0 8\n  (Paragraph 0 7\n    (Text 0 7 destination="/p")))'),
        ("Paragraph level=",
         "(Document 0 8\n  (Paragraph 0 7 level=2\n    (Text 0 7)))"),
        ("FencedCode missing content",
         '(Document 0 8\n  (FencedCode 0 8 info="x"))'),
        ("ReferenceLink missing destination",
         '(Document 0 8\n  (Paragraph 0 7\n    (ReferenceLink 0 7 label="r")))'),
        ("zero-length FencedCode NODE",
         '(Document 0 8\n  (FencedCode 4 4 info="" content=4:4))'),
        ("zero-length Text",
         "(Document 0 8\n  (Paragraph 0 7\n    (Text 3 3)))"),
    ]
    for why, tree in must_fail:
        if not _gate_errors(tree, src):
            err(f"self-test: gate FAILED to reject {why}")

    must_pass = [
        ("CodeSpan with no fields",
         "(Document 0 8\n  (Paragraph 0 7\n    (CodeSpan 0 5)))"),
        ("FencedCode with info + content",
         '(Document 0 8\n  (FencedCode 0 8 info="x" content=2:6))'),
        ("non-empty FencedCode node with an EMPTY content interval",
         '(Document 0 8\n  (FencedCode 0 8 info="" content=4:4))'),
        ("ReferenceLink with label + destination",
         '(Document 0 8\n  (Paragraph 0 7\n    (ReferenceLink 0 7 label="r" destination="/p")))'),
    ]
    for why, tree in must_pass:
        problems = _gate_errors(tree, src)
        if problems:
            err(f"self-test: gate rejected legal shape ({why}): {problems}")

    # duplicate field keys must not survive the tree reader either
    try:
        parse_tree('(Document 0 8\n  (ListItem 0 7 marker="-" marker="-"))')
        err("self-test: duplicate field key was accepted")
    except ValueError as e:
        if "duplicate field key" not in str(e):
            err(f"self-test: duplicate key rejected for the wrong reason: {e}")
    return len(must_fail), len(must_pass)


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

    # Recipe-slot expansion (MUTATION-v1 §7 / CASE-MATRIX §6): the frozen
    # total counts UNIQUE recipe slots; blocks may overlap when they
    # declare the same (corpus, operation, edit bytes) by construction
    # (e.g. the core INSERT-TINY also exists in the grid). Content-level
    # identity (offsets + inserted_sha256) exists only at instantiation.
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
    expected = cases.get("expected_unique_cases")
    if not isinstance(expected, int):
        err("case manifest: missing machine-readable `expected_unique_cases`")
        expected = -1
    if total != expected:
        err(f"recipe-slot expansion {total} != expected_unique_cases {expected}")

    # cross-check the total stated in the CASE-MATRIX narrative (the
    # manifest field is the single machine authority; the doc must echo it)
    matrix = (ROOT / "cases" / "CASE-MATRIX-v1.md").read_text(encoding="utf-8")
    m = re.search(r"unique case total:\s+(\d+)", matrix)
    if not m:
        err("CASE-MATRIX-v1.md: no 'unique case total:' line")
    elif int(m.group(1)) != expected:
        err(
            f"CASE-MATRIX-v1.md total {m.group(1)} != expected_unique_cases {expected}"
        )
    # stale duplicated totals must not survive in any artifact
    for stale_doc, stale in [
        ("cases/CASE-MATRIX-v1.md", "386"),
        ("cases/case-manifest-v1.toml", "386"),
        ("cases/case-manifest-v1.toml", "167"),
    ]:
        if stale in (ROOT / stale_doc).read_text(encoding="utf-8"):
            err(f"{stale_doc}: stale total {stale} still present")
    return total, expected


# ---------------------------------------------------------------------------
# Corrective-1 regressions: recipe arithmetic on small in-memory units.
#
# These mirror the recipes of corpus/CORPUS-v1.md (authority) and prove
# the four MAJOR repair surfaces. They build ONE unit / mountain / tile
# in memory — they do NOT generate corpora (no 16 MiB outputs, no
# receipts) — and they do line-level / byte-level assertions only. They
# do NOT parse Markdown: the hand-authored fixtures stay the semantic
# authority.
# ---------------------------------------------------------------------------

def filler(n: int, off: int = 0) -> str:
    return "".join(chr(ord("a") + (i + off) % 26) for i in range(n))


def _doc(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def build_units() -> dict:
    """One unit per (shape, variant) exactly per CORPUS-v1 §4."""
    units = {}
    cjk_line = "中文"
    units["plain.ascii"] = ("\n" + filler(62) + "\n").encode()
    units["plain.cjk"] = ("\n" + cjk_line + filler(56) + "\n").encode()
    units["many_blocks.ascii"] = (filler(14) + "\n\n").encode()
    units["many_blocks.cjk"] = (cjk_line + filler(8) + "\n\n").encode()
    units["huge_block.base"] = (cjk_line + filler(65528) + "\n\n").encode()
    depths = list(range(1, 17)) + list(range(15, 0, -1)) + [1]
    list_m, quote_m = [], []
    for d in depths:
        k_list = 61 - 2 * (d - 1)
        k_quote = 63 - 2 * d
        list_m.append(("  " * (d - 1) + "- " + filler(k_list) + "\n").encode())
        quote_m.append(("> " * d + filler(k_quote) + "\n").encode())
    units["deep.list_mountain"] = b"".join(list_m)
    units["deep.quote_mountain"] = b"".join(quote_m)
    units["deep.depths"] = depths  # type: ignore[assignment]
    units["inline_dense.ascii"] = (
        "\n" + "a *bb* `c` [x](/p) " * 3 + "a *b*" + "\n"
    ).encode()
    units["inline_dense.cjk"] = (
        "\n"
        + "中 *bb* `c` [x](/p) 中 *em* `cd` [y](/q) 文 *f* b [z](/r) c"
        + "\n"
    ).encode()
    units["fence_heavy.ascii"] = (
        "```x\n" + filler(250) + "\n" + filler(250) + "\n```\n" + "\n"
    ).encode()
    units["fence_heavy.cjk"] = (
        "```x\n" + "中" * 82 + filler(4) + "\n"
        + "中" * 82 + filler(4) + "\n```\n" + "\n"
    ).encode()
    units["reference_fanout.header"] = b"".join(
        (f"[r{j:02d}]: /" + filler(23, off=j) + "\n").encode() for j in range(16)
    )
    units["reference_fanout.ascii"] = (
        "\n" + "[t000][r00] " * 5 + "xx" + "\n"
    ).encode()
    units["reference_fanout.cjk"] = (
        "\n" + cjk_line + "[t000][r00] " * 4 + filler(8) + "\n"
    ).encode()
    tile = (
        "## section\n"
        + "\n"
        + "".join(f"[rx{j}]: /" + filler(23, off=j) + "\n" for j in range(4))
        + "\n"
        + "> quoted aaaa\n> quoted bbbb\n"
        + "\n"
        + "- item alpha\n- item beta\n  - nested gamma\n"
        + "\n"
        + units["fence_heavy.ascii"].decode()
        + "a *bb* `c` [x](/p) " * 3
        + "a *b*\n"
        + "\n"
        + "".join(
            f"[t{n:03d}][rx{j}] " * 5 + "xx\n"
            for n, j in zip(range(8), [0, 1, 2, 3, 0, 1, 2, 3])
        )
        + "\n"
        + "x" * 48
        + "\n\n"
    )
    units["mixed.tile"] = tile.encode()
    units["mixed.tile_full"] = tile.encode() + units["plain.ascii"] * 43
    return units


EXPECTED_UNIT_SIZES = {
    "plain.ascii": 64,
    "plain.cjk": 64,
    "many_blocks.ascii": 16,
    "many_blocks.cjk": 16,
    "huge_block.base": 65536,
    "deep.list_mountain": 2048,
    "deep.quote_mountain": 2048,
    "inline_dense.ascii": 64,
    "inline_dense.cjk": 64,
    "fence_heavy.ascii": 512,
    "fence_heavy.cjk": 512,
    "reference_fanout.header": 512,
    "reference_fanout.ascii": 64,
    "reference_fanout.cjk": 64,
    "mixed.tile": 1344,
    "mixed.tile_full": 4096,
}


def check_unit_variant_sizes(units: dict) -> int:
    """MAJOR-1: EVERY declared variant has its declared byte size."""
    n = 0
    for key, want in EXPECTED_UNIT_SIZES.items():
        got = len(units[key])
        if got != want:
            err(f"unit variant {key}: {got} bytes != declared {want}")
        n += 1
    # the doc's own CJK body arithmetic must satisfy 3*a + b == 250
    corpus_md = _doc("corpus/CORPUS-v1.md")
    m = re.search(r'"中"×(\d+) \+ filler\((\d+)\)', corpus_md)
    if not m:
        err("CORPUS-v1.md: fence CJK body formula not found")
    elif 3 * int(m.group(1)) + int(m.group(2)) != 250:
        err(
            f"CORPUS-v1.md: fence CJK body {m.group(1)}×3+{m.group(2)} != 250 B"
        )
    # fence unit internal layout (both variants)
    for key in ("fence_heavy.ascii", "fence_heavy.cjk"):
        u = units[key]
        if u[0:5] != b"```x\n" or u[507:511] != b"```\n" or u[511:512] != b"\n":
            err(f"{key}: fence unit layout drift (opener/body/closer)")
        if u[256:257] == b"\n" or u[5:6] == b"\n":
            err(f"{key}: body lines collapsed")
    # deep lines are all exactly 64 B and the quote line formula is the
    # corrected marker-chain form
    for key in ("deep.list_mountain", "deep.quote_mountain"):
        lines = units[key].split(b"\n")[:-1]
        if len(lines) != 32 or any(len(l) + 1 != 64 for l in lines):
            err(f"{key}: mountain is not 32 lines x 64 B")
    if not re.search(
        r'quote line\(d\)\s*=\s*"> "×d \+ content\(63.2d\)', corpus_md
    ):
        err("CORPUS-v1.md: corrected quote-line formula not found")
    if '"  "×(d−1) + "> "' in corpus_md:
        err("CORPUS-v1.md: old (broken) quote-line formula still present")
    return n


def check_deep_depths(units: dict) -> None:
    """MAJOR-2: max list depth 16 and max quote depth 16 BY RECIPE."""
    depths = units["deep.depths"]

    # quote depth = number of leading "> " markers (grammar §6 chain)
    qd = []
    for line in units["deep.quote_mountain"].split(b"\n")[:-1]:
        m = re.match(rb"^(?:\x3e\x20)+", line)
        qd.append(len(m.group(0)) // 2 if m else 0)
    if max(qd) != 16:
        err(f"deep_container: max quote depth {max(qd)} != 16")
    if qd != depths:
        err("deep_container: quote depth ladder drift")

    # list depth by §7 indent arithmetic (stack of (marker_indent,
    # item_content_indent); sibling rule continues the same list)
    stack: list[tuple[int, int]] = []
    ld = []
    for line in units["deep.list_mountain"].split(b"\n")[:-1]:
        stripped = line.decode()
        indent = len(stripped) - len(stripped.lstrip(" "))
        placed = False
        while stack and indent < stack[-1][1]:
            cand = stack.pop()
            if indent == cand[0]:  # §7 sibling rule: same list, new item
                stack.append((cand[0], indent + 2))
                placed = True
                break
        if not placed:
            if not stack:
                stack.append((indent, indent + 2))
            else:
                c = stack[-1][1]
                if not (0 <= indent - c <= 3):
                    err(
                        "deep_container: list ladder violates §7 nesting "
                        f"(indent {indent}, content indent {c})"
                    )
                    return
                stack.append((indent, indent + 2))
        ld.append(len(stack))
    if max(ld) != 16:
        err(f"deep_container: max list depth {max(ld)} != 16")
    if ld != depths:
        err("deep_container: list depth ladder drift")


def check_fence_close(units: dict) -> None:
    """MAJOR-3: M-FS-FENCE-CLOSE executable on fence_heavy and mixed."""
    unit = units["fence_heavy.ascii"]
    lines = unit.decode().split("\n")
    # precondition: opener + >= 2 body lines + closer
    body = [l for l in lines if l and not set(l) <= {"`"} and l != "```x"]
    if len(body) != 2:
        err(f"fence_heavy: precondition unsatisfiable ({len(body)} body lines)")
        return
    # the frozen canonical edit: [body2_start, old_closer_end) -> closer + body2
    b2s, cls = 256, 507
    removed = unit[b2s:cls + 4]
    inserted = unit[cls:cls + 4] + unit[b2s:cls]
    if len(removed) != 255 or len(inserted) != 255:
        err("M-FS-FENCE-CLOSE: removed != inserted == 255 (REPLACE_EQ)")
        return
    post = unit[:b2s] + inserted + unit[cls + 4:]
    pl = post.decode().split("\n")
    if not (
        pl[0] == "```x"
        and pl[1] == filler(250)
        and pl[2] == "```"
        and pl[3] == filler(250)
        and pl[4] == ""
        and pl[5] == ""
    ):
        err("M-FS-FENCE-CLOSE: post unit is not opener|body1|closer|body2|blank")
        return
    # the released line: exactly one bare closer remains, and body2 now
    # sits AFTER it (ordinary document structure, raw bytes released)
    closers = [i for i, l in enumerate(pl) if l == "```"]
    if closers != [2]:
        err(f"M-FS-FENCE-CLOSE: post unit closer lines {closers} != [2]")
    # old closer must not survive at its old position: no backtick run
    # may start at or after the released body line
    if "```" in "\n".join(pl[3:]):
        err("M-FS-FENCE-CLOSE: stray backtick run after the released line")
    # mixed: exactly one fence unit per tile -> precondition holds there too
    tile = units["mixed.tile"]
    if tile.count(b"```x\n") != 1:
        err("mixed: tile does not carry exactly one fence unit")
    i = tile.index(b"```x\n")
    fu = units["fence_heavy.ascii"]
    if tile[i:i + 512] != fu:
        err("mixed: tile fence unit drifts from the FENCE_HEAVY recipe")


def check_generic_anchors(units: dict) -> int:
    """MAJOR-4: dephased anchors land where the contract says."""
    n = 0
    fence = units["fence_heavy.ascii"]
    plain_cjk_line = units["plain.cjk"][1:63]  # line content between LFs
    deep_l0 = ("- " + "中文" + filler(55) + "\n").encode()  # CJK list line 0
    for size in (65536, 1048576, 16777216):
        for label, raw in (
            ("early", size // 4),
            ("middle", size // 2),
            ("late", (3 * size) // 4),
        ):
            generic = raw + 7
            # never on any unit / mountain / tile / super-unit boundary
            for u in (16, 64, 512, 2048, 4096, 65536):
                if generic % u == 0:
                    err(f"anchor {label}@{size}: on boundary mod {u}")
            # PLAIN: anchored unit index is a variant unit (≡0 mod 16);
            # intra-unit byte inside line content (bytes 1..62), ASCII
            if (raw // 64) % 16 != 0:
                err(f"plain anchor {label}@{size}: not on a variant unit")
            intra = generic % 64
            if not (1 <= intra <= 62):
                err(f"plain anchor {label}@{size}: intra {intra} not in line")
            elif not chr(plain_cjk_line[intra - 1]).isascii():
                err(f"plain anchor {label}@{size}: mid-char landing")
            # FENCE_HEAVY: anchored unit index is even (ASCII body);
            # intra-unit byte inside body content, not opener/closer/LF
            if (raw // 512) % 2 != 0:
                err(f"fence anchor {label}@{size}: not on an ASCII unit")
            fi = generic % 512
            if not (5 <= fi <= 506) or fence[fi] == 0x0A:
                err(f"fence anchor {label}@{size}: intra {fi} not in body")
            # DEEP: even mountains are list mountains; line 0 content
            # (CJK) — byte 7 falls inside the second CJK scalar; the
            # down-snap must land on its first byte, still in content
            if (raw // 2048) % 2 != 0:
                err(f"deep anchor {label}@{size}: not on a list mountain")
            j = 7
            while j < len(deep_l0) and (deep_l0[j] & 0xC0) == 0x80:
                j -= 1
            if not (3 <= j <= 62):
                err(f"deep anchor {label}@{size}: snap lands at {j}")
            # HUGE_BLOCK: anchor interior to the giant line in every size
            if size == 65536 and generic >= 65534:
                err("huge_block-64k anchor not interior")
            n += 1
    # the frozen rule must be stated in the mutation contract
    mut_md = _doc("mutations/MUTATION-v1.md")
    if "generic_anchor = raw_anchor + 7" not in mut_md:
        err("MUTATION-v1.md: frozen generic-anchor rule not found")
    # tie-breaking default must be frozen (IMPORTANT-4)
    for phrase in (
        "minimum distance wins",
        "the lower byte offset wins",
        "source-order first",
    ):
        if phrase not in mut_md:
            err(f"MUTATION-v1.md: tie rule missing {phrase!r}")
    # QUERY ordered batch must be frozen (IMPORTANT-3)
    norm_md = _doc("grammar/NORMALIZED-RESULT-v1.md")
    for phrase in ("ONE ORDERED BATCH", "ordered tuple", "NODE_PATH_AT(EARLY_generic)"):
        if phrase not in norm_md:
            err(f"NORMALIZED-RESULT-v1.md: query batch missing {phrase!r}")
    return n


def main() -> int:
    n_fix = check_fixtures()
    n_must_fail, n_must_pass = check_gate_self_test()
    n_cases, expected = check_manifests()
    units = build_units()
    n_units = check_unit_variant_sizes(units)
    check_deep_depths(units)
    check_fence_close(units)
    n_anchors = check_generic_anchors(units)
    print(f"fixtures checked:        {n_fix}")
    print(f"gate self-tests:         {n_must_fail} must-fail, {n_must_pass} must-pass, duplicate keys rejected")
    print(f"unique cases expanded:   {n_cases} (expected_unique_cases {expected})")
    print(
        f"corrective regressions:  {n_units} unit variants, deep depths, "
        f"fence-close edit, {n_anchors} generic anchors"
    )
    if errors:
        print(f"R3 FREEZE GATE: FAIL ({len(errors)} problems)")
        for e in errors:
            print("  -", e)
        return 1
    print("R3 FREEZE GATE: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
