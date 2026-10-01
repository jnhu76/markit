#!/usr/bin/env python3
"""Product boundary gate for the Markit production workspace.

Enforces the Markit internal Cargo topology: every dependency edge
between markit-* production crates — normal, dev, build, optional, and
target-specific alike — must be explicitly admitted by the rules below.
There is no denylist: an unadmitted edge fails closed. A new internal
crate must also be admitted explicitly (universe-level allowlist), so
sneaking a crate into the workspace fails too (the root workspace uses
`members = ["crates/*"]`, so dropping a crate under crates/ is enough
to make it a member — the universe rule is what catches it).

Two repository boundaries the layout document promises
(docs/product/repository-layout.md) are enforced here as well:

  - production -> research: a production crate must not depend (any
    dependency kind) on a crate resolved under research/. Research
    trees are self-contained workspaces outside the product
    architecture; workspace exclusion must never be circumvented by a
    path dependency.
  - exclusion is never architectural exclusion: a markit-* crate hidden
    in the root [workspace] exclude list is still part of the
    production universe and must be admitted explicitly
    (WORKSPACE_EXCLUDED_PRODUCTION).

A narrow donor-contamination scan covers production sources
(crates/*/src/**, apps/*/src/**) for high-signal donor markers only.
Provenance records (crate PROVENANCE.md files) may legitimately name
the donor; production implementation and its normative comments must
not depend on it. This is a contamination detector, not a vocabulary
constitution — deliberately no broad deny lists.

research/** is intentionally not part of the production universe.

Usage:
  python3 tools/check_product_boundaries.py                     # gate
  python3 tools/check_product_boundaries.py --negative-controls # prove non-vacuity

Exit codes: 0 PASS, 1 FAIL, 2 TOOLING-FAIL (refuses to mutate a dirty tree).
"""

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RESEARCH_DIR = ROOT / "research"

# ---------------------------------------------------------------------------
# Universe: production crates this gate knows about.

# No markit-* crate is excluded from the workspace today. The table stays
# so an excluded crate can only ever leave the gate's sight on purpose.
WORKSPACE_EXCLUDED_PRODUCTION = []

INTERNAL_PREFIX = "markit-"

# ---------------------------------------------------------------------------
# Admitted dependency edges (allowlist-first).
# Every markit-* → markit-* edge must appear here, per dependency kind.

NORMAL_EDGES = {
    "markit-composition": set(),  # K0 depends on no product crate
    "markit-app": {"markit-composition"},  # thin admission layer over K0
    # markit-document publishes the stable `document` composition role as a
    # ComponentSpec (foundation-rules constructor convention); the document
    # hot path itself stays inside the crate.
    "markit-document": {"markit-composition"},
    # markit-markdown-api is the backend-neutral Markdown seam: contract
    # types over document identity, capability publication over K0.
    "markit-markdown-api": {"markit-composition", "markit-document"},
    # markit-markdown-h4 implements the Markdown contract; it may depend on
    # the seam, document identity, and K0 role publication — never on
    # research (the donor mechanism was transplanted, see its PROVENANCE.md).
    "markit-markdown-h4": {
        "markit-composition",
        "markit-document",
        "markit-markdown-api",
    },
}

DEV_EDGES = {}  # no internal dev edges are admitted for any crate

BUILD_EDGES = {}  # no internal build edges are admitted for any crate

# ---------------------------------------------------------------------------
# Donor-contamination markers (high-signal exact spellings only). These
# name the donor's playback/decode/backend crates and its product-authority
# document identifiers; none may appear in production sources. Bare donor
# mentions that carry no authority (e.g. a provenance pointer in a comment)
# are not scanned for — the crate PROVENANCE.md files are the provenance
# authority, and this is not a vocabulary gate.

DONOR_MARKERS = [
    "qianqian_playback",
    "qianqian_audio_api",
    "qianqian_output_wasapi",
    "qianqian_decode_songcore",
    "qianqian_songcore_sys",
    "ADR-PBK",
    "PBK-",
    "F6-AUTHORITY-PROMOTION",
    "SongCore",
    "PcmSink",
    "PlaybackSession",
]

# Human-facing rule prose for violation output.
EDGE_RULE_PROSE = {
    "markit-composition": "K0 depends on no product crate",
    "markit-app": "the composition root may depend on composition only",
    "markit-document": "document code may depend on composition only (role publication)",
    "markit-markdown-api": "the Markdown seam may depend on composition + document identity only",
    "markit-markdown-h4": "the H4 provider may depend on the seam, document identity, and composition only",
}


def fail(message):
    print(f"PRODUCT_BOUNDARY_VIOLATION\n{message}")
    sys.exit(1)


def tooling_fail(message):
    print(f"TOOLING-FAIL: {message}")
    sys.exit(2)


def cargo_metadata(manifest=None):
    cmd = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
    if manifest:
        cmd += ["--manifest-path", str(ROOT / manifest)]
    try:
        out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    except FileNotFoundError:
        tooling_fail("cargo not found")
    if out.returncode != 0:
        tooling_fail(f"cargo metadata failed for {manifest or 'workspace'}:\n{out.stderr.strip()}")
    try:
        return json.loads(out.stdout)
    except json.JSONDecodeError as error:
        tooling_fail(
            f"cargo metadata returned invalid JSON for {manifest or 'workspace'}: {error}"
        )


def internal_deps(package):
    """Yield (target, kind, flags) for every markit-* dependency."""
    for dep in package.get("dependencies", []):
        name = dep["name"]
        if not name.startswith(INTERNAL_PREFIX):
            continue  # external crates are outside this architecture policy
        kind = dep.get("kind") or "normal"
        flags = []
        if dep.get("optional"):
            flags.append("optional")
        if dep.get("target"):
            flags.append(f"target:{dep['target']}")
        yield name, kind, flags


def research_dep_violations(package):
    """Yield (target, kind, resolved) for every dependency whose path
    resolves under research/. Only path dependencies can point inside
    this repository, so registry/git dependencies are out of scope."""
    manifest_dir = Path(package["manifest_path"]).parent
    for dep in package.get("dependencies", []):
        rel = dep.get("path")
        if not rel:
            continue
        resolved = (manifest_dir / rel).resolve()
        if resolved == RESEARCH_DIR or RESEARCH_DIR in resolved.parents:
            yield dep["name"], dep.get("kind") or "normal", resolved


def production_source_files():
    for pattern in ("crates/*/src/**/*.rs", "apps/*/src/**/*.rs"):
        yield from sorted(ROOT.glob(pattern))


def scan():
    violations = []

    # Workspace exclusion is never architectural exclusion: every markit-*
    # crate listed in [workspace] exclude must be explicitly known to this
    # gate. Non-markit excluded paths (research/) are outside the
    # production universe.
    root_manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    admitted_excluded = {Path(p).name for p in WORKSPACE_EXCLUDED_PRODUCTION}
    for entry in root_manifest.get("workspace", {}).get("exclude", []):
        manifest = ROOT / entry / "Cargo.toml"
        if not manifest.is_file():
            continue
        name = tomllib.loads(manifest.read_text(encoding="utf-8")).get("package", {}).get("name", "")
        if name.startswith(INTERNAL_PREFIX) and name not in admitted_excluded:
            violations.append(
                f"source: <workspace exclude>\ntarget: {name}\nkind: workspace-exclude\n"
                f"rule: a markit-* crate excluded from the workspace must be explicitly "
                f"admitted to the architecture universe (update WORKSPACE_EXCLUDED_PRODUCTION on purpose)\n"
                f"authority: docs/product/repository-layout.md — workspace exclusion never means architectural exclusion"
            )

    universe = {}
    root = cargo_metadata()
    for pkg in root["packages"]:
        universe[pkg["name"]] = pkg
    # An admitted excluded crate is governed like a member: loaded from
    # its own manifest into the universe, so the edge, research, and
    # stale-rule scans below see it. Exclusion must never remove a
    # markit-* crate from this gate's sight.
    for manifest in WORKSPACE_EXCLUDED_PRODUCTION:
        meta = cargo_metadata(f"{manifest}/Cargo.toml")
        for pkg in meta["packages"]:
            if pkg["name"] not in universe:
                universe[pkg["name"]] = pkg

    # Universe-level allowlist: every discovered internal crate must have
    # an explicit rule entry, and every rule entry must exist in reality.
    for name in sorted(universe):
        if name not in NORMAL_EDGES:
            violations.append(
                f"source: <workspace/universe>\ntarget: {name}\nkind: crate\n"
                f"rule: every internal crate must be explicitly admitted to the architecture\n"
                f"authority: allowlist-first topology (docs/product/foundation-rules.md)"
            )
    for name in sorted(NORMAL_EDGES):
        if name not in universe:
            violations.append(
                f"source: <rules>\ntarget: {name}\nkind: crate\n"
                f"rule: an admission rule names a crate that does not exist (stale rule)\n"
                f"authority: rules must match repository reality"
            )

    for name, pkg in sorted(universe.items()):
        for target, kind, flags in internal_deps(pkg):
            table = {"normal": NORMAL_EDGES, "dev": DEV_EDGES, "build": BUILD_EDGES}[kind]
            allowed = table.get(name)
            if allowed is None:
                violations.append(
                    f"source: {name}\ntarget: {target}\nkind: {kind}\n"
                    f"rule: no admitted {kind} edges for this crate (fail-closed)\n"
                    f"authority: allowlist-first topology (docs/product/foundation-rules.md)"
                )
            elif target not in allowed:
                violations.append(
                    f"source: {name}\ntarget: {target}\nkind: {kind}"
                    + (f" ({', '.join(flags)})" if flags else "")
                    + "\nrule: "
                    + EDGE_RULE_PROSE.get(name, "edge not explicitly admitted")
                    + f"\nauthority: docs/product/foundation-rules.md"
                )
        for target, kind, resolved in research_dep_violations(pkg):
            violations.append(
                f"source: {name}\ntarget: {target}\nkind: research-dependency ({kind})\n"
                f"path: {resolved}\n"
                f"rule: production crates must not depend on research\n"
                f"authority: docs/product/repository-layout.md — research stays outside "
                f"the product workspace and the product architecture"
            )

    # Donor-contamination scan (production sources only; see DONOR_MARKERS).
    for rs in production_source_files():
        text = rs.read_text(encoding="utf-8")
        for marker in DONOR_MARKERS:
            if marker in text:
                violations.append(
                    f"source: {rs.relative_to(ROOT)}\ntarget: {marker}\nkind: donor-marker\n"
                    f"rule: donor playback/product mechanism must not appear in production sources\n"
                    f"authority: docs/product/foundation-rules.md — domain semantics belong to "
                    f"future domain crates, and donor authority never adjudicates Markit code"
                )

    return violations


def run_gate():
    violations = scan()
    if violations:
        for v in violations:
            print("PRODUCT_BOUNDARY_VIOLATION")
            print(v)
            print()
        print(f"SUITE: FAILED ({len(violations)} violation(s))")
        sys.exit(1)
    print("PRODUCT_BOUNDARY_GATE: PASS")
    print("SUITE: PASS (topology allowlist + research boundary + donor scan clean)")


# ---------------------------------------------------------------------------
# Negative controls: prove the gate is not vacuous. Each control mutates
# the real tree, expects a specific failure, restores byte-exactly, and
# verifies the baseline is green again. Refuses to run on a dirty tree.

MUTABLE_FILES = [
    "Cargo.toml",
    "crates/markit-app/Cargo.toml",
    "crates/markit-app/src/lib.rs",
]


def git_dirty(path):
    out = subprocess.run(
        ["git", "status", "--porcelain", "--", str(ROOT / path)],
        cwd=ROOT, capture_output=True, text=True,
    )
    if out.returncode != 0:
        # fail-closed: a failed git status is NOT evidence of a clean tree
        tooling_fail(f"git status failed for {path}:\n{out.stderr.strip()}")
    return bool(out.stdout.strip())


def expect_fail(label, expected_fragment, edits=None, creates=None):
    """Apply reversible mutations, run the scan, expect exit 1 with a
    signature. `edits` transforms existing files (snapshotted and
    restored byte-exactly); `creates` writes new files (deleted after).
    """
    edits = edits or {}
    creates = creates or {}
    for rel in creates:
        if (ROOT / rel).exists():
            tooling_fail(f"{rel} already exists; refusing to run negative controls")
    snapshots = {}
    for rel, transform in edits.items():
        p = ROOT / rel
        snapshots[rel] = p.read_text(encoding="utf-8")
        p.write_text(transform(snapshots[rel]), encoding="utf-8")
    for rel, content in creates.items():
        (ROOT / rel).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / rel).write_text(content, encoding="utf-8")
    try:
        violations = scan()
        captured = "\n".join(violations)
    finally:
        for rel, original in snapshots.items():
            (ROOT / rel).write_text(original, encoding="utf-8")
        for rel in creates:
            (ROOT / rel).unlink(missing_ok=True)
        # prune directories created for the probe files, deepest first
        dirs = sorted({(ROOT / rel).parent for rel in creates},
                      key=lambda d: len(d.parts), reverse=True)
        for d in dirs:
            try:
                d.rmdir()
            except OSError:
                pass
    for rel, original in snapshots.items():
        if (ROOT / rel).read_text(encoding="utf-8") != original:
            tooling_fail(f"{label}: {rel} did not restore byte-exactly")
    if not violations:
        print(f"RESULT {label} TOOLING-FAIL (mutation was NOT caught)")
        sys.exit(1)
    if expected_fragment not in captured:
        print(f"RESULT {label} TOOLING-FAIL (failed, but not with the expected signature)")
        sys.exit(1)
    print(f"RESULT {label} RED (caught: {expected_fragment.splitlines()[0]})")


def append_dependency(text, section, line):
    marker = f"[{section}]"
    idx = text.index(marker) + len(marker)
    return text[:idx] + "\n" + line + text[idx:]


def run_negative_controls():
    for rel in MUTABLE_FILES:
        if git_dirty(rel):
            tooling_fail(f"{rel} is dirty; refusing to run negative controls")

    # A — an unadmitted internal dev edge must RED (the dev table is live,
    # not an accident of the normal table).
    expect_fail(
        "A markit-app unadmitted dev edge",
        "kind: dev",
        {
            "crates/markit-app/Cargo.toml": lambda t: (
                t
                + '\n[dev-dependencies]\nmarkit-composition = { path = "../markit-composition" }\n'
            )
        },
    )

    # B — an unknown markit-* crate must RED (universe allowlist; the root
    # workspace glob makes dropping a crate under crates/ a membership).
    expect_fail(
        "B unknown markit-* crate",
        "target: markit-boundary-probe",
        creates={
            "crates/markit-boundary-probe/Cargo.toml": (
                '[package]\nname = "markit-boundary-probe"\nversion = "0.1.0"\nedition = "2021"\n'
            ),
            "crates/markit-boundary-probe/src/lib.rs": "",
        },
    )

    # C — a production dependency pointing into research/ must RED even
    # though research is workspace-excluded.
    expect_fail(
        "C production dependency into research",
        "kind: research-dependency",
        {
            "crates/markit-app/Cargo.toml": lambda t: append_dependency(
                t,
                "dependencies",
                'markit-mdbench-common = { path = "../../research/benchmarks/markdown-ast-update/common" }',
            )
        },
    )

    # D — hiding a markit-* crate in the workspace exclude list must RED
    # (exclusion is never architectural exclusion).
    expect_fail(
        "D workspace-exclude bypass",
        "kind: workspace-exclude",
        edits={
            "Cargo.toml": lambda t: t.replace(
                'exclude = ["research"]',
                'exclude = ["research", "crates/markit-boundary-probe"]',
            ),
        },
        creates={
            "crates/markit-boundary-probe/Cargo.toml": (
                '[package]\nname = "markit-boundary-probe"\nversion = "0.1.0"\nedition = "2021"\n'
            ),
            "crates/markit-boundary-probe/src/lib.rs": "",
        },
    )

    # E — a donor mechanism marker in a production source must RED (the
    # contamination scan compiles in the production src paths).
    expect_fail(
        "E donor marker in production source",
        "target: qianqian_playback",
        {
            "crates/markit-app/src/lib.rs": lambda t: (
                "use qianqian_playback::episode;\n" + t
            )
        },
    )

    violations = scan()
    if violations:
        tooling_fail("baseline is not green after controls")
    print("SUITE: NEGATIVE-CONTROLS-PASS (all mutations caught; baseline green)")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--negative-controls",
        action="store_true",
        help="reversibly mutate the tree to prove the gate turns red (refuses a dirty tree)",
    )
    args = parser.parse_args()
    if args.negative_controls:
        run_negative_controls()
    else:
        run_gate()


if __name__ == "__main__":
    main()
