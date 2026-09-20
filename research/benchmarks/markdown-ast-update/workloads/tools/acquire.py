#!/usr/bin/env python3
"""Deterministic real-Markdown workload acquisition tool (#33 round 1, corrective 1).

Campaign: MARKIT-REAL-WORKLOAD-ACQUISITION-1.

Acquisition only. This tool never parses, normalizes, filters, or scores
Markdown content; it clones/fetches upstream repositories, pins immutable
commit SHAs, records provenance + a COMPLETE repository Markdown inventory,
materializes snapshot bytes locally, and verifies determinism. No H0-H4
execution, no workload selection, no eligibility analysis.

Corrective 1 (human adversarial review on PR #34) state machine:

    INIT / RELOCK  (explicit, distinct operations)
        resolve or reuse upstream pins -> construct the frozen lock
        -> build COMPLETE Markdown inventory -> write provenance manifests

    REPLAY  (acquire / materialize / verify once the lock exists)
        consume the frozen lock -> exact pins only -> preflight EVERYTHING
        (config hash, tool hash, source set, per-source pin, SOURCE identity,
        storage policy) BEFORE any mutation -> mismatch = FAIL, no writes

    MATERIALIZATION  (the only snapshot-byte-producing step after init)
        fetch exact pinned bytes from upstream into the LOCAL, gitignored
        sources/<id>/files/ area; verify sha256 + git blob id + byte count
        against SOURCE.json before the file counts as acquired.

Candidate snapshot bytes are NOT tracked in Git during the candidate stage
(storage policy separates VENDORED from MATERIALIZE_ONLY; the final frozen
benchmark corpus storage decision is a later campaign). Reproducibility is
carried by: source-lock.json + per-source SOURCE.json manifests + complete
Markdown inventories + derived manifests, all hash-pinned by the lock.

Commands:
    plan               offline frozen-frame enumeration from lock + inventories
    init               FIRST-TIME discovery: resolve HEADs, pin, lock (network)
    relock             EXPLICIT lock rebuild: --keep-pins (default) or
                       --rediscover (re-resolve upstream HEADs) (network)
    acquire            STRICT REPLAY of manifests: full preflight, rebuild derived
                       manifests byte-identically, fail closed; writes nothing
    materialize        fetch exact pinned snapshot bytes locally + verify (network)
    verify             offline integrity/closure verification [--full = also
                       require 100% materialization + inventory-vs-tree closure]
    report             print the acquisition report table (markdown)
    determinism-check  materialize -> digest -> wipe cache AND materialized bytes
                       -> strict replay -> compare digests

Byte preservation: every snapshot file is written from `git cat-file blob
<commit>:<path>` stdout with no post-processing, and its integrity is proven
by recomputing the git blob object id (sha1 over "blob <size>\\0" + bytes)
against the id recorded in the pinned commit's tree. Bytes (including line
endings) therefore match the upstream commit exactly, independent of any
core.autocrlf / .gitattributes checkout behavior (we never read the working
tree).

Determinism identity: SOURCE.json records `retrieved_at` (first-acquisition
wall-clock). It is the ONLY volatile field and is excluded from
`source_manifest_hash`, which is sha256 over the canonical JSON serialization
(sorted keys, no whitespace, ensure_ascii=False) of SOURCE.json without
`retrieved_at`. The lock records those identity hashes plus sha256 of the
config, the tool, every inventory manifest, and every derived manifest, and
is itself byte-stable across replay. Replay NEVER rewrites SOURCE.json, the
lock, or any identity; only `init` / `relock` (explicit operations) do.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path

TOOL_PATH = Path(__file__).resolve()
WORKLOADS_DIR = TOOL_PATH.parent.parent
CONFIG_PATH = WORKLOADS_DIR / "acquisition-config.json"
SOURCES_DIR = WORKLOADS_DIR / "sources"
MANIFESTS_DIR = WORKLOADS_DIR / "manifests"
CACHE_DIR = WORKLOADS_DIR / "_cache" / "repos"
LOCK_PATH = WORKLOADS_DIR / "source-lock.json"
LICENSES_DIR = WORKLOADS_DIR / "licenses"
INVENTORY_DIR = MANIFESTS_DIR / "inventory"
UNIVERSE_PATH = MANIFESTS_DIR / "candidate-universe-v1.json"
DUPLICATES_PATH = MANIFESTS_DIR / "exact-duplicates-v1.json"
INVENTORY_SUMMARY_PATH = MANIFESTS_DIR / "inventory-summary-v1.json"

SCHEMA_VERSION = "2"
CANDIDATE_UNIVERSE_SCHEMA = "candidate-universe-v2"
DUPLICATES_SCHEMA = "exact-duplicates-v1"
INVENTORY_SCHEMA = "markdown-inventory-v1"
INVENTORY_SUMMARY_SCHEMA = "inventory-summary-v1"

STORAGE_POLICIES = ("VENDORED", "MATERIALIZE_ONLY")
MD_EXTENSIONS = (".md", ".markdown")

SELECTION_INCLUDED = "included"
SELECTION_EXCLUDED = "excluded"
SELECTION_UNMATCHED = "not matched by any include pattern"


class ToolExit(SystemExit):
    """Raised instead of a bare sys.exit so tests can intercept any
    tool-issued failure; behavior at the CLI is identical (exit code 1)."""


def log(msg: str) -> None:
    print(msg, flush=True)


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr, flush=True)
    raise ToolExit(1)


def run(cmd: list[str], cwd: Path | None = None) -> str:
    proc = subprocess.run(cmd, cwd=cwd, check=False,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        detail = (proc.stderr or b"").decode("utf-8", "replace").strip()
        fail(f"command failed ({proc.returncode}): {' '.join(cmd)}\n{detail}")
    return proc.stdout.decode("utf-8", "replace")


def run_with_retries(cmd: list[str], cwd: Path | None = None,
                     attempts: int = 4, sleep_s: float = 5.0) -> str:
    """Run a network-touching git command with bounded retry/backoff.

    Transient TLS/promisor-fetch failures under concurrency are the failure
    mode seen in practice; the pinned SHA and blob-id checks below still
    guarantee that a 'successful' retry is byte-correct.
    """
    last = None
    for i in range(attempts):
        proc = subprocess.run(cmd, cwd=cwd, check=False,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if proc.returncode == 0:
            return proc.stdout.decode("utf-8", "replace")
        last = (proc.returncode, (proc.stderr or b"").decode("utf-8", "replace").strip())
        if i < attempts - 1:
            time.sleep(sleep_s * (i + 1))
    fail(f"command failed after {attempts} attempts ({last[0]}): {' '.join(cmd)}\n"
         f"{last[1]}")


def git_bytes(cache: Path, args: list[str]) -> bytes:
    proc = subprocess.run(["git", "-C", str(cache)] + args, check=False,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        fail(f"git failed: {' '.join(args)}\n{proc.stderr.decode('utf-8', 'replace')}")
    return proc.stdout


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_blob_id(data: bytes) -> str:
    """Git blob object id: sha1(b'blob <size>\\0' + content)."""
    h = hashlib.sha1()
    h.update(b"blob %d\x00" % len(data))
    h.update(data)
    return h.hexdigest()


def canonical_json(obj) -> str:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def manifest_bytes(obj) -> bytes:
    """Canonical on-disk serialization of a JSON manifest."""
    return (json.dumps(obj, indent=2, ensure_ascii=False, sort_keys=True)
            + "\n").encode("utf-8")


def write_json_deterministic(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(manifest_bytes(obj))


def is_40_hex(s: str) -> bool:
    return bool(re.fullmatch(r"[0-9a-f]{40}", s))


def load_config() -> dict:
    cfg = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    validate_config(cfg)
    return cfg


def validate_config(cfg: dict) -> None:
    """Structural + policy validation of the acquisition config.

    Hard rule: a NEEDS_REVIEW license state must never permit public
    vendoring by default (storage_policy must be MATERIALIZE_ONLY).
    """
    if cfg.get("campaign_id") != "MARKIT-REAL-WORKLOAD-ACQUISITION-1":
        fail("acquisition config: unexpected campaign_id")
    ids = [s["source_id"] for s in cfg["sources"]]
    if len(ids) != len(set(ids)):
        fail("duplicate source_id in acquisition config")
    for s in cfg["sources"]:
        sid = s["source_id"]
        if not s.get("include"):
            fail(f"[{sid}] config: include rule set is empty")
        policy = s.get("storage_policy")
        if policy not in STORAGE_POLICIES:
            fail(f"[{sid}] config: storage_policy must be one of "
                 f"{list(STORAGE_POLICIES)}, got {policy!r}")
        if not (s.get("scope_rationale") or "").strip():
            fail(f"[{sid}] config: scope_rationale is required (acquisition-scope rule)")
        if not (s.get("storage_policy_rationale") or "").strip():
            fail(f"[{sid}] config: storage_policy_rationale is required")
        if s["license"]["spdx"] == "NEEDS_REVIEW" and policy != "MATERIALIZE_ONLY":
            fail(f"[{sid}] config: NEEDS_REVIEW license must never permit public "
                 "vendoring by default (storage_policy must be MATERIALIZE_ONLY)")
        if not s["repository_url"].startswith("https://"):
            fail(f"[{sid}] config: repository_url must be https")


# ---------------------------------------------------------------------------
# gitignore-style glob matching (documented subset)
#   - a pattern starting with '/' or containing '/' is root-anchored
#   - '**' spans zero or more path segments
#   - '*' matches within a single path segment
# ---------------------------------------------------------------------------

def glob_to_regex(pattern: str) -> re.Pattern:
    anchored = pattern.startswith("/") or "/" in pattern.strip("/")
    segs = pattern.strip("/").split("/")
    parts: list[str] = []
    for i, seg in enumerate(segs):
        if seg == "**":
            parts.append(r"(?:[^/]+/)*")
        else:
            rx = ""
            for ch in seg:
                if ch == "*":
                    rx += r"[^/]*"
                else:
                    rx += re.escape(ch)
            parts.append(rx if i == len(segs) - 1 else rx + "/")
    rx = "".join(parts)
    if not anchored:
        rx = r"(?:[^/]+/)*" + rx
    return re.compile("^" + rx + "$")


class GlobSet:
    def __init__(self, patterns: list[str]):
        self.patterns = list(patterns)
        self.res = [glob_to_regex(p) for p in patterns]

    def match(self, path: str) -> bool:
        return any(r.match(path) for r in self.res)

    def first_match(self, path: str) -> str | None:
        for p, r in zip(self.patterns, self.res):
            if r.match(path):
                return p
        return None


# ---------------------------------------------------------------------------
# upstream fetch helpers
# ---------------------------------------------------------------------------

def ls_remote_head(url: str) -> tuple[str, str]:
    """Return (default_branch, head_sha) observed at discovery time."""
    out = run_with_retries(["git", "ls-remote", "--symref", url, "HEAD"])
    branch, sha = None, None
    for line in out.splitlines():
        if line.startswith("ref:"):
            branch = line[4:].split("\t")[0].strip().rsplit("/", 1)[-1]
        else:
            sha = line.split("\t")[0].strip()
    if not branch or not sha:
        fail(f"cannot parse ls-remote output for {url}")
    return branch, sha


def ensure_cache(cfg_source: dict, pinned_sha: str) -> Path:
    """Fetch the PINNED SHA into the local partial clone cache.

    Replay never discovers: the pinned SHA comes from the lock/SOURCE.json
    (never a floating branch). The cache clone is partial (blob:none) with a
    sparse-checkout restricted to the configured include patterns, so
    checkout batch-prefetches exactly the blobs we will materialize;
    extraction then reads from the local object database. The working tree
    is never read for content.
    """
    url = cfg_source["repository_url"]
    sid = cfg_source["source_id"]
    cache = CACHE_DIR / sid
    cache.mkdir(parents=True, exist_ok=True)
    if not (cache / ".git").exists():
        log(f"  clone (depth=1, blob:none, no-checkout): {url}")
        run_with_retries(["git", "clone", "--depth", "1", "--filter=blob:none",
                          "--no-checkout", url, str(cache)])
    run(["git", "-C", str(cache), "sparse-checkout", "init", "--no-cone"])
    patterns = "\n".join(cfg_source["include"]) + "\n"
    proc = subprocess.run(
        ["git", "-C", str(cache), "sparse-checkout", "set", "--no-cone", "--stdin"],
        input=patterns.encode("utf-8"), check=False,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        fail("sparse-checkout set failed: "
             + proc.stderr.decode("utf-8", "replace"))
    run_with_retries(["git", "-C", str(cache), "fetch", "--depth", "1",
                      "--filter=blob:none", "origin", pinned_sha])
    actual = run(["git", "-C", str(cache), "rev-parse", pinned_sha + "^{commit}"]).strip()
    if actual != pinned_sha:
        fail(f"[{sid}] fetched object {actual} != pinned {pinned_sha}")
    # checkout batch-prefetches the blobs matching the sparse patterns so
    # per-file extraction below does not pay one network round-trip per blob
    run_with_retries(["git", "-C", str(cache), "checkout", "--force", "--detach",
                      pinned_sha])
    return cache


def discover_pin(cfg_source: dict) -> tuple[str, str]:
    """INIT/RELOCK-only: resolve the upstream default-branch tip."""
    log(f"  discovering upstream HEAD: {cfg_source['repository_url']}")
    branch, sha = ls_remote_head(cfg_source["repository_url"])
    log(f"  default branch {branch} at {sha[:12]}")
    return sha, branch


def tree_blob_ids(cache: Path, sha: str) -> dict[str, str]:
    """path -> blob sha1 for every blob in the pinned commit tree.

    Uses `ls-tree -r` WITHOUT `-l`: asking for sizes would lazily fetch every
    missing blob under a partial clone. Blob sizes are obtained explicitly
    (and only where wanted) by blob_sizes_batch().
    """
    out = run(["git", "-C", str(cache), "ls-tree", "-r", "-z", sha])
    files: dict[str, str] = {}
    for entry in out.split("\0"):
        if not entry:
            continue
        meta, path = entry.split("\t", 1)
        parts = meta.split()
        if len(parts) < 3 or parts[1] != "blob":
            continue
        files[path] = parts[2]
    return files


def commit_timestamp(cache: Path, sha: str) -> str:
    return run(["git", "-C", str(cache), "show", "-s", "--format=%cI", sha]).strip()


def read_blob(cache: Path, sha: str, path: str) -> bytes:
    """Read a blob from the pinned commit; retry transient promisor fetches."""
    last = None
    for i in range(4):
        proc = subprocess.run(
            ["git", "-C", str(cache), "cat-file", "blob", f"{sha}:{path}"],
            check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if proc.returncode == 0:
            return proc.stdout
        last = (proc.returncode, proc.stderr.decode("utf-8", "replace").strip())
        time.sleep(5.0 * (i + 1))
    fail(f"cat-file failed for {path} after retries ({last[0]})\n{last[1]}")


def blob_sizes_batch(cache: Path, sha: str, paths: list[str]) -> dict[str, int]:
    """path -> exact blob byte size via `git cat-file --batch-check`.

    Sizes come from the object database (never the working tree). Promisor
    blobs must already be local (the caller prefetches them via sparse
    checkout); anything still missing is fetched explicitly per blob by
    read_blob, whose size is the exact content length. The recorded number
    is the upstream byte size at the pinned commit either way.
    """
    if not paths:
        return {}
    stdin = "".join(f"{sha}:{p}\n" for p in paths).encode("utf-8")
    proc = subprocess.run(
        ["git", "-C", str(cache), "cat-file", "--batch-check", "--buffer"],
        input=stdin, check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        fail("cat-file --batch-check failed: "
             + proc.stderr.decode("utf-8", "replace"))
    sizes: dict[str, int] = {}
    for line in proc.stdout.decode("utf-8", "replace").splitlines():
        name, _, rest = line.rpartition(" ")
        if not name.startswith(sha + ":"):
            continue
        path = name[len(sha) + 1:]
        fields = rest.split()
        if len(fields) == 2 and fields[0] == "blob" and fields[1].isdigit():
            sizes[path] = int(fields[1])
    missing = [p for p in paths if p not in sizes]
    for p in missing:
        data = read_blob(cache, sha, p)  # triggers the promisor fetch
        sizes[p] = len(data)
    return sizes


# ---------------------------------------------------------------------------
# COMPLETE Markdown sampling-frame inventory
# ---------------------------------------------------------------------------

def is_markdown_path(path: str) -> bool:
    low = path.lower()
    return low.endswith(MD_EXTENSIONS)


def build_inventory_entries(tree: dict[str, str], include: GlobSet,
                            exclude: GlobSet) -> list[dict]:
    """Mechanical selected/unselected marking for EVERY Markdown path in the
    pinned tree. Pure function of the tree listing + glob rules."""
    entries = []
    for path in sorted(tree):
        if not is_markdown_path(path):
            continue
        inc = include.first_match(path)
        if inc is not None:
            exc = exclude.first_match(path)
            if exc is not None:
                reason = f"{SELECTION_EXCLUDED}:{exc} (include:{inc})"
                selected = False
            else:
                reason = f"{SELECTION_INCLUDED}:{inc}"
                selected = True
        else:
            reason = SELECTION_UNMATCHED
            selected = False
        entries.append({
            "upstream_path": path,
            "git_blob_sha1": tree[path],
            "selected_for_snapshot": selected,
            "selection_reason": reason,
        })
    return entries


def sparse_prefetch(cache: Path, sid: str, patterns: list[str]) -> None:
    """Extend the cache sparse-checkout and check out, batch-prefetching the
    listed paths' blobs. The working tree is never read for content; this is
    purely a bulk object-database prefetch."""
    run(["git", "-C", str(cache), "sparse-checkout", "init", "--no-cone"])
    proc = subprocess.run(
        ["git", "-C", str(cache), "sparse-checkout", "set", "--no-cone", "--stdin"],
        input=("\n".join(patterns) + "\n").encode("utf-8"), check=False,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        fail(f"[{sid}] sparse-checkout set failed: "
             + proc.stderr.decode("utf-8", "replace"))
    run_with_retries(["git", "-C", str(cache), "checkout", "--force", "--detach", "HEAD"])


def build_source_inventory(cfg_source: dict, cache: Path, sha: str) -> dict:
    """Enumerate the COMPLETE repository Markdown inventory at the pin,
    independent of the snapshot include glob. Prefetches inventory blobs
    (sizes only) via bulk sparse checkout; content is never read."""
    sid = cfg_source["source_id"]
    log(f"[{sid}] building complete Markdown inventory at {sha[:12]}")
    tree = tree_blob_ids(cache, sha)
    md_paths = sorted(p for p in tree if is_markdown_path(p))
    log(f"  prefetching sizes for {len(md_paths)} repository Markdown files")
    sparse_prefetch(cache, sid, md_paths)
    sizes = blob_sizes_batch(cache, sha, md_paths)
    include = GlobSet(cfg_source["include"])
    exclude = GlobSet(cfg_source["exclude"])
    entries = build_inventory_entries(tree, include, exclude)
    for e in entries:
        e["bytes"] = sizes[e["upstream_path"]]
    snapshot_entries = [e for e in entries if e["selected_for_snapshot"]]
    inv = {
        "schema_version": INVENTORY_SCHEMA,
        "campaign_id": cfg_source.get("campaign_id", "MARKIT-REAL-WORKLOAD-ACQUISITION-1"),
        "authority_issue": "33",
        "source_id": sid,
        "repository_url": cfg_source["repository_url"],
        "commit_sha": sha,
        "inventory_definition": (
            "every blob in the pinned commit tree whose path ends in .md or "
            ".markdown (case-insensitive), enumerated via git ls-tree -r; "
            "byte sizes via git cat-file (object database, never the working "
            "tree); selection marking is purely mechanical (include/exclude "
            "globs) with the deciding rule recorded"
        ),
        "repository_md_files": len(entries),
        "repository_md_bytes": sum(e["bytes"] for e in entries),
        "snapshot_md_files": len(snapshot_entries),
        "snapshot_md_bytes": sum(e["bytes"] for e in snapshot_entries),
        "entries": entries,
    }
    write_json_deterministic(INVENTORY_DIR / f"{sid}.json", inv)
    log(f"  inventory: {inv['repository_md_files']} repository Markdown files "
        f"({inv['repository_md_bytes']} bytes); snapshot frame "
        f"{inv['snapshot_md_files']} files ({inv['snapshot_md_bytes']} bytes)")
    return inv


def load_inventories() -> dict[str, dict]:
    invs = {}
    for p in sorted(INVENTORY_DIR.glob("*.json")):
        invs[p.stem] = json.loads(p.read_text(encoding="utf-8"))
    return invs


# ---------------------------------------------------------------------------
# SOURCE.json
# ---------------------------------------------------------------------------

def source_manifest_identity(source_obj: dict) -> str:
    """sha256 over canonical JSON of SOURCE.json minus volatile retrieved_at."""
    payload = {k: v for k, v in source_obj.items() if k != "retrieved_at"}
    return sha256_hex(canonical_json(payload).encode("utf-8"))


def license_files_in_tree(tree: dict[str, str], candidates: list[str]) -> list[dict]:
    return [{"upstream_path": p, "git_blob_sha1": tree[p]}
            for p in candidates if p in tree]


def preserve_license_artifacts(sid: str, cache: Path, sha: str,
                               candidates: list[str]) -> list[dict]:
    """Extract the upstream LICENSE/NOTICE/attribution files at the pin into
    tracked provenance artifacts (byte-exact, hashed). License texts are
    attribution material, not candidate workload bytes."""
    tree = tree_blob_ids(cache, sha)
    artifacts = []
    for name in candidates:
        if name not in tree or "/" in name:
            continue
        data = read_blob(cache, sha, name)
        if git_blob_id(data) != tree[name]:
            fail(f"[{sid}] {name}: license artifact blob id mismatch")
        rel = Path("licenses") / sid / name
        out_path = WORKLOADS_DIR / rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(data)
        artifacts.append({
            "upstream_path": name,
            "artifact_path": str(rel),
            "sha256": sha256_hex(data),
            "bytes": len(data),
            "git_blob_sha1": tree[name],
        })
    return artifacts


def build_source_obj(cfg_source: dict, sha: str, default_branch: str,
                     commit_ts: str, files: list[dict],
                     license_files: list[dict],
                     preserved_artifacts: list[dict]) -> dict:
    # SPDX comes from the frozen config, authored after reading the actual
    # license files at the pinned commit. NEEDS_REVIEW means the identity was
    # not confirmed; it does not mean no license file exists.
    spdx = cfg_source["license"]["spdx"]
    policy = cfg_source["storage_policy"]
    if policy == "VENDORED":
        redistribution_note = (
            "Storage policy VENDORED: candidate snapshot bytes are not tracked "
            "in Git during the candidate stage; the later final-corpus freeze "
            "may vendor these bytes publicly under the license recorded here, "
            "with LICENSE/NOTICE/attribution artifacts preserved under "
            "licenses/ (hashes in this manifest).")
    else:
        redistribution_note = (
            "Storage policy MATERIALIZE_ONLY: these bytes must not be publicly "
            "vendored by default. They are materialized locally at the pinned "
            "commit for verification and characterization and are excluded "
            "from public Git storage; expected sha256/bytes/blob-id are "
            "recorded here so any local materialization is verifiable.")
    return {
        "schema_version": SCHEMA_VERSION,
        "campaign_id": cfg_source.get("campaign_id", "MARKIT-REAL-WORKLOAD-ACQUISITION-1"),
        "authority_issue": "33",
        "source_id": cfg_source["source_id"],
        "repository_url": cfg_source["repository_url"],
        "commit_sha": sha,
        "commit_timestamp": commit_ts,
        "default_branch": default_branch,
        "candidate_role": cfg_source["candidate_role"],
        "role_caveat": cfg_source.get("role_caveat"),
        "storage_policy": policy,
        "storage_policy_rationale": cfg_source["storage_policy_rationale"],
        "scope_rationale": cfg_source["scope_rationale"],
        "snapshot_policy": {
            "mode": cfg_source["snapshot_mode"],
            "mode_a_selected_files": cfg_source["snapshot_mode"] == "A",
            "include": cfg_source["include"],
            "exclude": cfg_source["exclude"],
        },
        "license": {
            "spdx": spdx,
            "files": license_files,
            "preserved_artifacts": preserved_artifacts,
            "note": cfg_source["license"].get("note"),
            "redistribution_note": redistribution_note,
        },
        "extraction_method": (
            "git cat-file blob <commit_sha>:<upstream_path>, byte-exact, no eol/"
            "filter conversion; per-file integrity proven by git blob sha1 "
            "recomputation (git_blob_sha1) against the pinned commit tree"
        ),
        "file_count": len(files),
        "total_bytes": sum(f["bytes"] for f in files),
        "files": files,
    }


def check_source_closure(o: dict) -> None:
    """SOURCE.json internal closure: counts, byte sums, ordering, layout."""
    sid = o.get("source_id", "?")
    files = o["files"]
    paths = [f["upstream_path"] for f in files]
    if paths != sorted(paths):
        fail(f"[{sid}] SOURCE.json files[] is not sorted by upstream_path")
    if len(paths) != len(set(paths)):
        fail(f"[{sid}] SOURCE.json files[] contains duplicate upstream_path")
    if o["file_count"] != len(files):
        fail(f"[{sid}] SOURCE.json file_count {o['file_count']} != "
             f"len(files) {len(files)}")
    byte_sum = sum(f["bytes"] for f in files)
    if o["total_bytes"] != byte_sum:
        fail(f"[{sid}] SOURCE.json total_bytes {o['total_bytes']} != "
             f"sum(file.bytes) {byte_sum}")
    for f in files:
        if f["snapshot_path"] != "files/" + f["upstream_path"]:
            fail(f"[{sid}] {f['upstream_path']}: snapshot_path layout mismatch")
        if not is_40_hex(f["git_blob_sha1"]):
            fail(f"[{sid}] {f['upstream_path']}: malformed git_blob_sha1")
        if len(f["sha256"]) != 64:
            fail(f"[{sid}] {f['upstream_path']}: malformed sha256")


def snapshot_selected_files(cfg_source: dict, cache: Path, sha: str) -> list[dict]:
    """Materialize the selected snapshot files for one source into the local
    (gitignored) area, verifying every byte against the pinned tree."""
    sid = cfg_source["source_id"]
    source_dir = SOURCES_DIR / sid
    tree = tree_blob_ids(cache, sha)
    include = GlobSet(cfg_source["include"])
    exclude = GlobSet(cfg_source["exclude"])
    selected = sorted(p for p in tree if include.match(p) and not exclude.match(p))
    if not selected:
        fail(f"[{sid}] include globs selected no files; fix acquisition config")
    files_dir = source_dir / "files"
    files_dir.mkdir(parents=True, exist_ok=True)
    file_entries = []
    for path in selected:
        blob_sha1 = tree[path]
        data = read_blob(cache, sha, path)
        if git_blob_id(data) != blob_sha1:
            fail(f"[{sid}] {path}: git blob id mismatch — byte preservation violated")
        snapshot_rel = "files/" + path
        out_path = source_dir / snapshot_rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(data)
        file_entries.append({
            "upstream_path": path,
            "snapshot_path": snapshot_rel,
            "sha256": sha256_hex(data),
            "bytes": len(data),
            "git_blob_sha1": blob_sha1,
        })
    # remove stale locally-materialized files no longer selected (local-only
    # hygiene; the manifest, not this sweep, is the authority)
    keep = {Path(e["snapshot_path"]) for e in file_entries}
    for p in sorted(files_dir.rglob("*"), reverse=True):
        if p.is_file() and p.relative_to(source_dir) not in keep:
            p.unlink()
    return file_entries


def write_source_json(cfg_source: dict, sha: str, default_branch: str,
                      file_entries: list[dict], preserved: list[dict],
                      retrieved_at: str | None) -> dict:
    """Write SOURCE.json. retrieved_at is preserved when an existing manifest
    records an earlier first-acquisition time; it is the only volatile field."""
    cache = CACHE_DIR / cfg_source["source_id"]
    license_files = license_files_in_tree(
        tree_blob_ids(cache, sha), cfg_source["license"]["candidate_paths"])
    obj = build_source_obj(cfg_source, sha, default_branch,
                           commit_timestamp(cache, sha), file_entries,
                           license_files, preserved)
    existing_path = SOURCES_DIR / cfg_source["source_id"] / "SOURCE.json"
    if retrieved_at is None and existing_path.exists():
        retrieved_at = json.loads(existing_path.read_text(encoding="utf-8")).get("retrieved_at")
    obj["retrieved_at"] = retrieved_at or time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    check_source_closure(obj)
    write_json_deterministic(existing_path, obj)
    identity = source_manifest_identity(obj)
    log(f"  SOURCE.json written: {obj['file_count']} files, {obj['total_bytes']} "
        f"bytes, identity {identity[:12]}")
    return obj


# ---------------------------------------------------------------------------
# derived (regenerable) manifests — deterministic from pinned inputs
# ---------------------------------------------------------------------------

def render_universe_manifest(cfg: dict, objs: dict[str, dict],
                             invs: dict[str, dict]) -> dict:
    cfg_sources = {s["source_id"]: s for s in cfg["sources"]}
    entries = []
    for sid in sorted(objs):
        o, c, inv = objs[sid], cfg_sources[sid], invs[sid]
        entries.append({
            "source_id": sid,
            "category": c["category"],
            "candidate_role": c["candidate_role"],
            "role_caveat": c.get("role_caveat"),
            "counts_as_role_coverage": c.get("role_caveat") is None,
            "repository_url": c["repository_url"],
            "pinned_sha": o["commit_sha"],
            "snapshot_mode": c["snapshot_mode"],
            "storage_policy": c["storage_policy"],
            "repository_md_files": inv["repository_md_files"],
            "repository_md_bytes": inv["repository_md_bytes"],
            "snapshot_md_files": inv["snapshot_md_files"],
            "snapshot_md_bytes": inv["snapshot_md_bytes"],
            "file_coverage": inv["snapshot_md_files"] / inv["repository_md_files"],
            "byte_coverage": (inv["snapshot_md_bytes"] / inv["repository_md_bytes"]
                              if inv["repository_md_bytes"] else None),
            "notes": c["notes"],
        })
    return {
        "schema_version": CANDIDATE_UNIVERSE_SCHEMA,
        "campaign_id": cfg["campaign_id"],
        "authority_issue": "33",
        "note": (
            "Candidate membership only. Candidate != final benchmark workload: "
            "no eligibility filtering, no representativeness or extremeness "
            "selection, no performance classification. Category/role labels are "
            "descriptive acquisition metadata. counts_as_role_coverage=false "
            "means the intended workloads of that upstream project are NOT "
            "Markdown, so the snapshot must not be counted as coverage for the "
            "role during workload selection (see role_caveat). Coverage fields "
            "quantify the acquisition sampling frame against the complete "
            "pinned-repository Markdown inventory; the frame is a deliberate "
            "pre-performance scope decision recorded per source in "
            "acquisition-config.json (scope_rationale)."),
        "candidates": entries,
    }


def render_duplicates_manifest(objs: dict[str, dict]) -> dict:
    """Exact byte-identical duplicate registry, derived from SOURCE.json
    sha256 lists (no snapshot bytes needed). Detection only; no deletion."""
    by_hash: dict[str, list[str]] = {}
    for sid in sorted(objs):
        for f in objs[sid]["files"]:
            by_hash.setdefault(f["sha256"], []).append(f"{sid}/{f['snapshot_path']}")
    dups = [{"sha256": h, "paths": sorted(paths)}
            for h, paths in sorted(by_hash.items()) if len(paths) > 1]
    return {
        "schema_version": DUPLICATES_SCHEMA,
        "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
        "note": ("Exact byte-identical snapshot files across the candidate "
                 "universe, recorded for later redundancy analysis. No copy is "
                 "deleted during acquisition."),
        "duplicate_groups": dups,
    }


def render_inventory_summary(invs: dict[str, dict]) -> dict:
    rows = []
    for sid in sorted(invs):
        inv = invs[sid]
        rows.append({
            "source_id": sid,
            "commit_sha": inv["commit_sha"],
            "repository_md_files": inv["repository_md_files"],
            "snapshot_md_files": inv["snapshot_md_files"],
            "file_coverage": inv["snapshot_md_files"] / inv["repository_md_files"],
            "repository_md_bytes": inv["repository_md_bytes"],
            "snapshot_md_bytes": inv["snapshot_md_bytes"],
            "byte_coverage": (inv["snapshot_md_bytes"] / inv["repository_md_bytes"]
                              if inv["repository_md_bytes"] else None),
        })
    return {
        "schema_version": INVENTORY_SUMMARY_SCHEMA,
        "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
        "authority_issue": "33",
        "note": ("Acquisition-scope coverage: complete pinned-repository "
                 "Markdown denominator vs candidate-snapshot numerator, per "
                 "source and in total. Makes acquisition-scope bias measurable "
                 "before static characterization. No syntax inspection, no "
                 "eligibility, no performance judgment happens here."),
        "totals": {
            "repository_md_files": sum(r["repository_md_files"] for r in rows),
            "snapshot_md_files": sum(r["snapshot_md_files"] for r in rows),
            "repository_md_bytes": sum(r["repository_md_bytes"] for r in rows),
            "snapshot_md_bytes": sum(r["snapshot_md_bytes"] for r in rows),
        },
        "sources": rows,
    }


def write_derived_manifests(cfg: dict, objs: dict[str, dict],
                            invs: dict[str, dict]) -> dict[str, str]:
    """Write regenerable manifests; return their sha256s (recorded in lock)."""
    hashes = {}
    universe = render_universe_manifest(cfg, objs, invs)
    write_json_deterministic(UNIVERSE_PATH, universe)
    hashes[UNIVERSE_PATH.name] = sha256_hex(UNIVERSE_PATH.read_bytes())
    duplicates = render_duplicates_manifest(objs)
    write_json_deterministic(DUPLICATES_PATH, duplicates)
    hashes[DUPLICATES_PATH.name] = sha256_hex(DUPLICATES_PATH.read_bytes())
    summary = render_inventory_summary(invs)
    write_json_deterministic(INVENTORY_SUMMARY_PATH, summary)
    hashes[INVENTORY_SUMMARY_PATH.name] = sha256_hex(INVENTORY_SUMMARY_PATH.read_bytes())
    log(f"derived manifests written: universe ({len(universe['candidates'])} "
        f"candidates), duplicates, inventory summary")
    return hashes


# ---------------------------------------------------------------------------
# the frozen lock
# ---------------------------------------------------------------------------

def load_lock() -> dict | None:
    if not LOCK_PATH.exists():
        return None
    return json.loads(LOCK_PATH.read_text(encoding="utf-8"))


def build_lock(cfg: dict, objs: dict[str, dict], derived_hashes: dict[str, str],
               inventory_hashes: dict[str, str]) -> dict:
    lock_sources = []
    for sid in sorted(objs):
        o = objs[sid]
        lock_sources.append({
            "source_id": sid,
            "repository_url": o["repository_url"],
            "commit_sha": o["commit_sha"],
            "storage_policy": o["storage_policy"],
            "source_manifest_hash": source_manifest_identity(o),
        })
    return {
        "schema_version": SCHEMA_VERSION,
        "campaign_id": cfg["campaign_id"],
        "authority_issue": "33",
        "protocol_root": "research/benchmarks/markdown-ast-update",
        "lock_semantics": (
            "FROZEN LOCK. Once this file exists, acquire/materialize are strict "
            "replay: they preflight acquisition-config hash, acquisition-tool "
            "hash, campaign identity, exact source set, per-source pins, "
            "SOURCE.json identity, and storage policy BEFORE any mutation, and "
            "fail closed on any mismatch. Replay never regenerates or refreshes "
            "this lock; init (first discovery) and relock (explicit operation) "
            "are the only writers."),
        "acquisition_config": "acquisition-config.json",
        "acquisition_config_sha256": sha256_hex(CONFIG_PATH.read_bytes()),
        "acquisition_tool": "tools/acquire.py",
        "acquisition_tool_sha256": sha256_hex(TOOL_PATH.read_bytes()),
        "inventory_manifests": {f"{sid}.json": h
                                for sid, h in sorted(inventory_hashes.items())},
        "candidate_universe_manifest": UNIVERSE_PATH.name,
        "candidate_universe_manifest_sha256": derived_hashes[UNIVERSE_PATH.name],
        "exact_duplicates_manifest": DUPLICATES_PATH.name,
        "exact_duplicates_manifest_sha256": derived_hashes[DUPLICATES_PATH.name],
        "inventory_summary_manifest": INVENTORY_SUMMARY_PATH.name,
        "inventory_summary_manifest_sha256": derived_hashes[INVENTORY_SUMMARY_PATH.name],
        "identity_semantics": (
            "source_manifest_hash = sha256(canonical json of sources/<id>/"
            "SOURCE.json excluding volatile retrieved_at). Same source-lock "
            "-> same upstream versions -> same snapshot bytes -> same hashes. "
            "Candidate snapshot bytes are not tracked in Git during the "
            "candidate stage; 'materialize' reconstructs them locally and "
            "verifies them against these identities."),
        "sources": lock_sources,
    }


def write_lock(lock: dict) -> None:
    write_json_deterministic(LOCK_PATH, lock)
    log(f"source-lock.json written (schema {lock['schema_version']}, "
        f"{len(lock['sources'])} sources)")


# ---------------------------------------------------------------------------
# STRICT REPLAY preflight — the fail-closed gate
# ---------------------------------------------------------------------------

class PreflightError(ToolExit):
    """Preflight rejection: terminate BEFORE any mutation."""


def preflight_error(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr, flush=True)
    raise PreflightError(1)


def preflight_replay(cfg: dict, source_filter: str | None = None) -> dict:
    """Full fail-closed preflight for replay commands (acquire/materialize).

    Performs NO writes and NO network I/O. Every identity check runs before
    any caller may mutate anything. Raises PreflightError on the first
    mismatch; callers must treat it as 'terminate before any mutation'.
    """
    lock = load_lock()
    if lock is None:
        if SOURCES_DIR.exists() and any(SOURCES_DIR.glob("*/SOURCE.json")):
            preflight_error(
                "source-lock.json is missing while frozen source material "
                "already exists — refusing to mutate. Lock creation is an "
                "explicit operation: run 'relock' (human-reviewed).")
        preflight_error(
            "source-lock.json does not exist and nothing has been acquired — "
            "cold start requires the explicit 'init' command.")
    if lock.get("campaign_id") != cfg["campaign_id"]:
        preflight_error("lock campaign_id does not match acquisition config")
    if lock.get("schema_version") != SCHEMA_VERSION:
        preflight_error(f"lock schema_version {lock.get('schema_version')!r} != "
                        f"expected {SCHEMA_VERSION!r}; run 'relock' explicitly")
    cfg_hash = sha256_hex(CONFIG_PATH.read_bytes())
    if cfg_hash != lock["acquisition_config_sha256"]:
        preflight_error(
            "acquisition-config.json hash does not match the frozen lock — "
            "changed acquisition config under an existing campaign identity is "
            "rejected. Explicit 'relock' (a reviewed campaign operation) is "
            "required to change the frozen frame.")
    tool_hash = sha256_hex(TOOL_PATH.read_bytes())
    if tool_hash != lock["acquisition_tool_sha256"]:
        preflight_error(
            "tools/acquire.py hash does not match the frozen lock — a changed "
            "acquisition tool is rejected under the existing campaign identity. "
            "Explicit 'relock' is required to adopt a new tool version.")
    lock_by_id = {s["source_id"]: s for s in lock["sources"]}
    cfg_by_id = {s["source_id"]: s for s in cfg["sources"]}
    missing_in_lock = sorted(set(cfg_by_id) - set(lock_by_id))
    extra_in_lock = sorted(set(lock_by_id) - set(cfg_by_id))
    if missing_in_lock or extra_in_lock:
        preflight_error(
            "configured sources and lock sources differ — "
            f"missing from lock: {missing_in_lock or '[]'}; "
            f"extra in lock: {extra_in_lock or '[]'}. Source-set changes "
            "require explicit 'relock'.")
    for sid in sorted(lock_by_id):
        entry = lock_by_id[sid]
        if not is_40_hex(entry.get("commit_sha", "")):
            preflight_error(f"[{sid}] lock has no valid pinned commit SHA")
        if entry.get("repository_url") != cfg_by_id[sid]["repository_url"]:
            preflight_error(f"[{sid}] lock repository_url differs from config — "
                            "SOURCE identity change rejected")
        if entry.get("storage_policy") != cfg_by_id[sid]["storage_policy"]:
            preflight_error(f"[{sid}] lock storage_policy differs from config")
        source_json = SOURCES_DIR / sid / "SOURCE.json"
        if not source_json.exists():
            preflight_error(f"[{sid}] SOURCE.json is missing — replay requires "
                            "the frozen manifest for every locked source")
        o = json.loads(source_json.read_text(encoding="utf-8"))
        if o.get("source_id") != sid:
            preflight_error(f"[{sid}] SOURCE.json source_id mismatch (SOURCE "
                            "identity change rejected)")
        if o.get("commit_sha") != entry["commit_sha"]:
            preflight_error(f"[{sid}] SOURCE.json commit_sha differs from lock "
                            "pin — SOURCE identity change rejected")
        if o.get("repository_url") != entry["repository_url"]:
            preflight_error(f"[{sid}] SOURCE.json repository_url differs from "
                            "lock — SOURCE identity change rejected")
        if o.get("storage_policy") != entry["storage_policy"]:
            preflight_error(f"[{sid}] SOURCE.json storage_policy differs from lock")
        actual = source_manifest_identity(o)
        if actual != entry["source_manifest_hash"]:
            preflight_error(f"[{sid}] SOURCE.json identity {actual[:12]} != lock "
                            f"{entry['source_manifest_hash'][:12]} — a frozen "
                            "SOURCE may never be rewritten by replay")
    if source_filter is not None and source_filter not in lock_by_id:
        preflight_error(f"unknown source_id {source_filter}")
    return lock


# ---------------------------------------------------------------------------
# replay commands
# ---------------------------------------------------------------------------

def acquire(cfg: dict) -> None:
    """STRICT REPLAY of manifests. Offline; writes only regenerable manifests,
    and only if they reproduce byte-identically. Never touches the lock,
    SOURCE.json, or snapshot bytes."""
    lock = preflight_replay(cfg)
    objs = load_all_source_objs()
    for sid, o in sorted(objs.items()):
        check_source_closure(o)
        log(f"[{sid}] replay ok: pin {o['commit_sha'][:12]}, {o['file_count']} "
            f"files, {o['total_bytes']} bytes, policy {o['storage_policy']}")
    invs = load_inventories()
    if sorted(invs) != sorted(objs):
        preflight_error("inventory manifests do not cover the locked source set — "
                        "run 'verify --full' or relock explicitly")
    derived = write_derived_manifests_checked(cfg, objs, invs, lock)
    log(f"STRICT REPLAY PASS: {len(objs)} sources, manifests byte-identical to "
        "the frozen lock; no identity, snapshot byte, or lock was modified")


def write_derived_manifests_checked(cfg: dict, objs: dict[str, dict],
                                    invs: dict[str, dict], lock: dict) -> dict:
    """Render derived manifests, require byte-identity with the lock BEFORE
    writing anything, then write (an idempotent repair of identical bytes)."""
    renders = {
        UNIVERSE_PATH.name: render_universe_manifest(cfg, objs, invs),
        DUPLICATES_PATH.name: render_duplicates_manifest(objs),
        INVENTORY_SUMMARY_PATH.name: render_inventory_summary(invs),
    }
    hashes = {name: sha256_hex(manifest_bytes(obj))
              for name, obj in renders.items()}
    for name, recorded in (
            (UNIVERSE_PATH.name, lock["candidate_universe_manifest_sha256"]),
            (DUPLICATES_PATH.name, lock["exact_duplicates_manifest_sha256"]),
            (INVENTORY_SUMMARY_PATH.name, lock["inventory_summary_manifest_sha256"])):
        if hashes[name] != recorded:
            preflight_error(f"{name} does not reproduce the hash recorded in the "
                            "frozen lock — derived state diverged; refusing to "
                            "write anything (explicit relock required)")
    for name, obj in renders.items():
        write_json_deterministic(MANIFESTS_DIR / name, obj)
    log("derived manifests verified byte-identical to the frozen lock")
    return hashes


def materialize_source(cfg_source: dict, lock_entry: dict) -> None:
    """Fetch the exact pinned snapshot bytes locally and verify them against
    the frozen SOURCE.json. The ONLY post-init snapshot-byte producer."""
    sid = cfg_source["source_id"]
    o = json.loads((SOURCES_DIR / sid / "SOURCE.json").read_text(encoding="utf-8"))
    cache = ensure_cache(cfg_source, lock_entry["commit_sha"])
    file_entries = snapshot_selected_files(cfg_source, cache, o["commit_sha"])
    by_path = {f["upstream_path"]: f for f in file_entries}
    for f in o["files"]:
        got = by_path.get(f["upstream_path"])
        if got is None:
            fail(f"[{sid}] {f['upstream_path']}: not selected by frozen include "
                 "rules but present in SOURCE.json — state diverged")
        if got["sha256"] != f["sha256"]:
            fail(f"[{sid}] {f['upstream_path']}: sha256 mismatch against "
                 "SOURCE.json")
        if got["git_blob_sha1"] != f["git_blob_sha1"]:
            fail(f"[{sid}] {f['upstream_path']}: git blob id mismatch against "
                 "SOURCE.json")
        if got["bytes"] != f["bytes"]:
            fail(f"[{sid}] {f['upstream_path']}: byte count mismatch against "
                 "SOURCE.json")
    log(f"[{sid}] materialized {len(file_entries)} files "
        f"({sum(f['bytes'] for f in file_entries)} bytes), all verified against "
        f"SOURCE.json (policy {o['storage_policy']})")


def materialize(cfg: dict, source_filter: str | None) -> None:
    lock = preflight_replay(cfg, source_filter)
    lock_by_id = {s["source_id"]: s for s in lock["sources"]}
    selected = [s for s in cfg["sources"]
                if source_filter is None or s["source_id"] == source_filter]
    for cfg_source in selected:
        materialize_source(cfg_source, lock_by_id[cfg_source["source_id"]])
    log("MATERIALIZE PASS: snapshot bytes reconstructed locally at the pinned "
        "commits and verified against the frozen manifests")


# ---------------------------------------------------------------------------
# verification (offline unless --full)
# ---------------------------------------------------------------------------

def check_inventory_closure(inv: dict, o: dict) -> None:
    """Inventory↔SOURCE closure: the inventory selection set must be exactly
    the snapshot manifest set; inventory counts must close over entries."""
    sid = o["source_id"]
    if inv["commit_sha"] != o["commit_sha"]:
        fail(f"[{sid}] inventory pinned sha differs from SOURCE.json")
    if inv["source_id"] != sid:
        fail(f"[{sid}] inventory source_id mismatch")
    sel = sorted(e["upstream_path"] for e in inv["entries"]
                 if e["selected_for_snapshot"])
    snap = sorted(f["upstream_path"] for f in o["files"])
    if sel != snap:
        fail(f"[{sid}] inventory selection does not close over SOURCE.json "
             "files[] (acquisition-scope closure violated)")
    count = sum(1 for e in inv["entries"] if e["selected_for_snapshot"])
    if inv["snapshot_md_files"] != count or inv["repository_md_files"] != len(inv["entries"]):
        fail(f"[{sid}] inventory counts do not close over entries[]")
    bsum = sum(e["bytes"] for e in inv["entries"] if e["selected_for_snapshot"])
    if inv["snapshot_md_bytes"] != bsum:
        fail(f"[{sid}] inventory snapshot byte total does not close")
    rall = sum(e["bytes"] for e in inv["entries"])
    if inv["repository_md_bytes"] != rall:
        fail(f"[{sid}] inventory repository byte total does not close")
    if len({e["upstream_path"] for e in inv["entries"]}) != len(inv["entries"]):
        fail(f"[{sid}] inventory contains duplicate upstream paths")
    if [e["upstream_path"] for e in inv["entries"]] != \
            sorted(e["upstream_path"] for e in inv["entries"]):
        fail(f"[{sid}] inventory entries are not sorted by upstream_path")


def verify(cfg: dict, full: bool) -> None:
    lock = load_lock()
    if lock is None:
        fail("source-lock.json does not exist — nothing to verify")
    if sha256_hex(CONFIG_PATH.read_bytes()) != lock["acquisition_config_sha256"]:
        fail("acquisition-config.json changed since the lock was written; "
             "explicit 'relock' is required (this is a fail-closed state)")
    if sha256_hex(TOOL_PATH.read_bytes()) != lock["acquisition_tool_sha256"]:
        fail("tools/acquire.py changed since the lock was written; "
             "explicit 'relock' is required (this is a fail-closed state)")
    cfg_by_id = {s["source_id"]: s for s in cfg["sources"]}
    lock_by_id = {s["source_id"]: s for s in lock["sources"]}
    if set(cfg_by_id) != set(lock_by_id):
        fail("config sources and lock sources differ")

    objs = load_all_source_objs()
    checked_files = 0
    checked_bytes = 0
    missing_files = 0
    for sid in sorted(objs):
        o = objs[sid]
        c = cfg_by_id[sid]
        check_source_closure(o)
        source_dir = SOURCES_DIR / sid
        actual = sorted(
            str(p.relative_to(source_dir)) for p in (source_dir / "files").rglob("*")
            if p.is_file())
        expected = sorted(f["snapshot_path"] for f in o["files"])
        extra = [p for p in actual if p not in set(expected)]
        # policy/config correspondence (SOURCE must mirror the frozen config)
        if o["snapshot_policy"]["include"] != c["include"] or \
                o["snapshot_policy"]["exclude"] != c["exclude"] or \
                o["snapshot_policy"]["mode"] != c["snapshot_mode"]:
            fail(f"[{sid}] SOURCE.json snapshot_policy does not match config")
        if o["storage_policy"] != c["storage_policy"]:
            fail(f"[{sid}] SOURCE.json storage_policy does not match config")
        if o["storage_policy"] not in STORAGE_POLICIES:
            fail(f"[{sid}] invalid storage_policy in SOURCE.json")
        if o["candidate_role"] != c["candidate_role"] or \
                o["role_caveat"] != c.get("role_caveat"):
            fail(f"[{sid}] SOURCE.json role metadata does not match config")
        for f in o["files"]:
            p = source_dir / f["snapshot_path"]
            if not p.exists():
                missing_files += 1
                continue
            data = p.read_bytes()
            if len(data) != f["bytes"]:
                fail(f"[{sid}] {f['snapshot_path']}: bytes {len(data)} != {f['bytes']}")
            if sha256_hex(data) != f["sha256"]:
                fail(f"[{sid}] {f['snapshot_path']}: sha256 mismatch")
            if git_blob_id(data) != f["git_blob_sha1"]:
                fail(f"[{sid}] {f['snapshot_path']}: git blob id mismatch")
            checked_files += 1
            checked_bytes += len(data)
        if extra:
            fail(f"[{sid}] unmanifested snapshot files present locally: {extra[:3]}")
        # license provenance artifacts (tracked) must exist and hash-match
        for a in o["license"]["preserved_artifacts"]:
            ap = WORKLOADS_DIR / a["artifact_path"]
            if not ap.exists():
                fail(f"[{sid}] missing license provenance artifact {a['artifact_path']}")
            if sha256_hex(ap.read_bytes()) != a["sha256"]:
                fail(f"[{sid}] license provenance artifact hash mismatch: "
                     f"{a['artifact_path']}")
        log(f"[{sid}] ok: pin {o['commit_sha'][:12]}, {o['file_count']} files, "
            f"{o['total_bytes']} bytes, policy {o['storage_policy']}")

    # derived manifests must reproduce byte-identically from pinned inputs
    invs = load_inventories()
    if sorted(invs) != sorted(objs):
        fail(f"inventory manifests {sorted(invs)} do not cover locked sources "
             f"{sorted(objs)}")
    for sid, inv in invs.items():
        check_inventory_closure(inv, objs[sid])
    for sid, h in lock["inventory_manifests"].items():
        p = INVENTORY_DIR / sid
        if not p.exists() or sha256_hex(p.read_bytes()) != h:
            fail(f"inventory manifest {sid} missing or does not match the lock")
    expected_derived = {
        UNIVERSE_PATH.name: sha256_hex(manifest_bytes(
            render_universe_manifest(cfg, objs, invs))),
        DUPLICATES_PATH.name: sha256_hex(manifest_bytes(
            render_duplicates_manifest(objs))),
        INVENTORY_SUMMARY_PATH.name: sha256_hex(manifest_bytes(
            render_inventory_summary(invs))),
    }
    for name, expect in expected_derived.items():
        p = MANIFESTS_DIR / name
        if not p.exists() or sha256_hex(p.read_bytes()) != expect:
            fail(f"derived manifest {name} does not match lock-pinned inputs")

    if full:
        # inventory-vs-tree closure requires the pinned trees (warm cache)
        for sid in sorted(objs):
            cache = CACHE_DIR / sid
            if not (cache / ".git").exists():
                fail(f"[{sid}] --full requires the pinned tree locally; run "
                     "'materialize' first (warm cache)")
            tree = tree_blob_ids(cache, objs[sid]["commit_sha"])
            inv_paths = {e["upstream_path"]: e["git_blob_sha1"]
                         for e in invs[sid]["entries"]}
            tree_md = {p: b for p, b in tree.items() if is_markdown_path(p)}
            if inv_paths != tree_md:
                fail(f"[{sid}] inventory is not exactly the pinned-tree "
                     "Markdown set (inventory-vs-tree closure violated)")
        if missing_files:
            fail(f"--full requires 100% materialization; {missing_files} files "
                 "are not materialized locally")

    log(f"VERIFY PASS: {len(objs)} sources, {checked_files} materialized files "
        f"byte-verified ({checked_bytes} bytes), {missing_files} not materialized "
        f"locally; lock + inventories + derived manifests consistent "
        f"(full={'yes' if full else 'no'})")
    log("NO benchmark execution is performed by this tool (acquisition only).")


# ---------------------------------------------------------------------------
# plan (offline frozen-frame enumeration)
# ---------------------------------------------------------------------------

def plan(cfg: dict, source_filter: str | None) -> None:
    lock = load_lock()
    if lock is None:
        fail("no lock; run 'init' (first discovery) — plan reports the frozen frame")
    if sha256_hex(CONFIG_PATH.read_bytes()) != lock["acquisition_config_sha256"]:
        fail("acquisition-config.json changed since the lock; plan refuses to "
             "report a frame it cannot pin (explicit 'relock' required)")
    summary_path = INVENTORY_SUMMARY_PATH
    if not summary_path.exists():
        fail("inventory summary missing; the frozen frame cannot report byte "
             "totals without the complete Markdown inventory (no size is "
             "guessed or summed from unknown values)")
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    rows = {r["source_id"]: r for r in summary["sources"]}
    totals = summary["totals"]
    grand_snapshot_bytes = 0
    for s in cfg["sources"]:
        sid = s["source_id"]
        if source_filter and sid != source_filter:
            continue
        r = rows.get(sid)
        if r is None:
            fail(f"[{sid}] missing from inventory summary")
        grand_snapshot_bytes += r["snapshot_md_bytes"]
        log(f"[{sid}] pin {r['commit_sha'][:12]} policy {s['storage_policy']}")
        log(f"  REPOSITORY_MD_FILES {r['repository_md_files']}  "
            f"SNAPSHOT_MD_FILES {r['snapshot_md_files']}  "
            f"FILE_COVERAGE {r['file_coverage']:.4f}")
        log(f"  REPOSITORY_MD_BYTES {r['repository_md_bytes']}  "
            f"SNAPSHOT_MD_BYTES {r['snapshot_md_bytes']}  "
            f"BYTE_COVERAGE "
            f"{r['byte_coverage'] if r['byte_coverage'] is not None else 'n/a'}")
    log(f"TOTALS: REPOSITORY_MD_FILES {totals['repository_md_files']}  "
        f"SNAPSHOT_MD_FILES {totals['snapshot_md_files']}  "
        f"REPOSITORY_MD_BYTES {totals['repository_md_bytes']}  "
        f"SNAPSHOT_MD_BYTES {totals['snapshot_md_bytes']}  "
        f"selected-frame bytes for filter: {grand_snapshot_bytes}")


# ---------------------------------------------------------------------------
# report
# ---------------------------------------------------------------------------

def report(cfg: dict) -> str:
    objs = load_all_source_objs()
    lock = load_lock()
    if lock is None:
        fail("no lock; report requires the frozen campaign identity")
    lock_by_id = {s["source_id"]: s for s in lock["sources"]}
    invs = load_inventories()
    lines = [
        "| SOURCE_ID | STORAGE_POLICY | LICENSE | PINNED_SHA | REPO_MD_FILES | "
        "SNAPSHOT_MD_FILES | FILE_COVERAGE | SNAPSHOT_MD_BYTES | REPO_MD_BYTES | "
        "BYTE_COVERAGE |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for sid in sorted(objs):
        o = objs[sid]
        inv = invs[sid]
        fc = inv["snapshot_md_files"] / inv["repository_md_files"]
        bc = (inv["snapshot_md_bytes"] / inv["repository_md_bytes"]
              if inv["repository_md_bytes"] else None)
        lines.append(
            f"| {sid} | {o['storage_policy']} | {o['license']['spdx']} "
            f"| `{o['commit_sha']}` | {inv['repository_md_files']} "
            f"| {inv['snapshot_md_files']} | {fc:.4f} "
            f"| {inv['snapshot_md_bytes']} | {inv['repository_md_bytes']} "
            f"| {bc:.4f} |" if bc is not None else
            f"| {sid} | {o['storage_policy']} | {o['license']['spdx']} "
            f"| `{o['commit_sha']}` | {inv['repository_md_files']} "
            f"| {inv['snapshot_md_files']} | {fc:.4f} "
            f"| {inv['snapshot_md_bytes']} | {inv['repository_md_bytes']} | n/a |")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# determinism check: cold-cache strict replay of the materialization
# ---------------------------------------------------------------------------

def workload_tree_digest() -> tuple[int, int, int, str]:
    """Digest tracked workload artifacts AND locally materialized snapshot
    bytes (counted separately from tracked files)."""
    h = hashlib.sha256()
    tracked = materialized = license_files = 0
    for p in sorted(WORKLOADS_DIR.rglob("*")):
        if not p.is_file():
            continue
        rel = p.relative_to(WORKLOADS_DIR)
        if "_cache" in rel.parts:
            continue
        is_materialized = len(rel.parts) > 3 and rel.parts[0] == "sources" \
            and rel.parts[2] == "files"
        is_license = rel.parts[0] == "licenses"
        h.update(str(rel).encode("utf-8"))
        h.update(b"\0")
        h.update(sha256_hex(p.read_bytes()).encode("ascii"))
        h.update(b"\0")
        if is_materialized:
            materialized += 1
        elif is_license:
            license_files += 1
        else:
            tracked += 1
    return tracked, materialized, license_files, h.hexdigest()


def determinism_check(cfg: dict) -> None:
    log("determinism check: strict replay + materialize (pass 1)")
    acquire(cfg)
    for s in cfg["sources"]:
        materialize_source(s, {"commit_sha": _pin(cfg, s["source_id"])})
    t1, m1, l1, d1 = workload_tree_digest()
    log(f"artifacts: tracked {t1}, materialized {m1}, license {l1}, digest {d1}")

    log("clearing rebuildable state: _cache/repos AND materialized bytes "
        "sources/*/files")
    subprocess.run(["rm", "-rf", str(CACHE_DIR)], check=True)
    for files_dir in SOURCES_DIR.glob("*/files"):
        subprocess.run(["rm", "-rf", str(files_dir)], check=True)

    log("determinism check: strict replay from cold cache (pass 2, pinned SHAs)")
    acquire(cfg)
    for s in cfg["sources"]:
        materialize_source(s, {"commit_sha": _pin(cfg, s["source_id"])})
    t2, m2, l2, d2 = workload_tree_digest()
    log(f"artifacts: tracked {t2}, materialized {m2}, license {l2}, digest {d2}")

    if (t1, m1, l1, d1) != (t2, m2, l2, d2):
        fail("workload artifacts changed across a cold-cache strict replay — "
             "determinism FAILED")
    status = run(["git", "status", "--porcelain"], cwd=WORKLOADS_DIR)
    log(f"git status (repository scope) after replay:\n{status or '  (clean)'}")
    log("DETERMINISM PASS: cold-cache strict replay is byte-identical")


def _pin(cfg: dict, sid: str) -> str:
    lock = load_lock()
    if lock is None:
        fail("no lock")
    for e in lock["sources"]:
        if e["source_id"] == sid:
            return e["commit_sha"]
    fail(f"no pin for {sid}")


# ---------------------------------------------------------------------------
# init / relock — the ONLY lock writers (explicit operations)
# ---------------------------------------------------------------------------

def existing_source_objs() -> dict[str, dict]:
    objs = {}
    if SOURCES_DIR.exists():
        for d in sorted(SOURCES_DIR.iterdir()):
            p = d / "SOURCE.json"
            if d.is_dir() and p.exists():
                objs[d.name] = json.loads(p.read_text(encoding="utf-8"))
    return objs


def load_all_source_objs() -> dict[str, dict]:
    """All SOURCE.json manifests; every source directory must have one."""
    objs = {}
    if not SOURCES_DIR.exists():
        fail("no sources directory")
    for d in sorted(SOURCES_DIR.iterdir()):
        if not d.is_dir():
            continue
        p = d / "SOURCE.json"
        if not p.exists():
            fail(f"missing SOURCE.json for source dir {d.name}")
        objs[d.name] = json.loads(p.read_text(encoding="utf-8"))
    return objs


def cmd_init(cfg: dict) -> None:
    """FIRST-TIME DISCOVERY ONLY. Resolves upstream HEADs, constructs
    candidate pins, builds inventories + manifests, and creates the lock."""
    if LOCK_PATH.exists():
        fail("source-lock.json already exists — init refuses to touch a frozen "
             "campaign; use 'relock' (explicit, reviewed) to refresh")
    if existing_source_objs():
        fail("sources/<id>/SOURCE.json already exist — refusing ambiguous cold "
             "start; use 'relock --keep-pins' to rebuild the lock from pinned "
             "state, or remove the substrate deliberately")
    log("INIT: first-time discovery (explicit acquisition operation)")
    pins = {}
    branches = {}
    for s in cfg["sources"]:
        sid = s["source_id"]
        pins[sid], branches[sid] = discover_pin(s)
    _build_campaign_state(cfg, pins, branches)
    log("INIT PASS: campaign discovered, pinned, inventoried, and frozen")


def cmd_relock(cfg: dict, rediscover: bool) -> None:
    """EXPLICIT lock rebuild. Default --keep-pins: pins come from existing
    SOURCE.json (no upstream HEAD resolution); used when config/tool/schema/
    policy change under the same upstream versions. --rediscover re-resolves
    upstream HEADs: a genuinely new acquisition round."""
    if rediscover:
        log("RELOCK --rediscover: re-resolving upstream HEADs (new acquisition "
            "round, explicit operation)")
        pins, branches = {}, {}
        for s in cfg["sources"]:
            sid = s["source_id"]
            pins[sid], branches[sid] = discover_pin(s)
    else:
        objs = existing_source_objs()
        if not objs:
            fail("relock --keep-pins requires existing pinned SOURCE.json "
                 "manifests; run 'init' for first discovery")
        log("RELOCK --keep-pins: rebuilding campaign identity from pinned "
            "SOURCE.json state (explicit operation)")
        pins = {sid: o["commit_sha"] for sid, o in objs.items()}
        branches = {sid: o.get("default_branch", "unknown") for sid, o in objs.items()}
        for s in cfg["sources"]:
            sid = s["source_id"]
            if sid not in pins:
                fail(f"[{sid}] configured source has no existing pin; add it "
                     "via a reviewed config change + relock --keep-pins after "
                     "first discovery")
    _build_campaign_state(cfg, pins, branches)
    log("RELOCK PASS: campaign identity rebuilt and frozen")


def _build_campaign_state(cfg: dict, pins: dict[str, str],
                          branches: dict[str, str]) -> None:
    """Shared init/relock body: materialize + manifest + inventory + lock."""
    objs: dict[str, dict] = {}
    invs: dict[str, dict] = {}
    for s in cfg["sources"]:
        sid = s["source_id"]
        log(f"[{sid}] pinning at {pins[sid][:12]}")
        cache = ensure_cache(s, pins[sid])
        file_entries = snapshot_selected_files(s, cache, pins[sid])
        preserved = preserve_license_artifacts(sid, cache, pins[sid],
                                               s["license"]["candidate_paths"])
        log(f"  preserved {len(preserved)} license/attribution artifacts")
        obj = write_source_json(s, pins[sid], branches[sid], file_entries,
                                preserved, retrieved_at=None)
        objs[sid] = obj
        invs[sid] = build_source_inventory(s, cache, pins[sid])
    inventory_hashes = {sid: sha256_hex((INVENTORY_DIR / f"{sid}.json").read_bytes())
                        for sid in sorted(invs)}
    derived = write_derived_manifests(cfg, objs, invs)
    write_lock(build_lock(cfg, objs, derived, inventory_hashes))


# ---------------------------------------------------------------------------

def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("command",
                    choices=["plan", "init", "relock", "acquire", "materialize",
                             "verify", "report", "determinism-check"])
    ap.add_argument("--source", help="limit to one source_id (plan/materialize)")
    ap.add_argument("--full", action="store_true",
                    help="verify: require full (100 percent) materialization "
                         "and inventory-vs-tree closure")
    ap.add_argument("--keep-pins", action="store_true", default=True,
                    help="relock: reuse pins recorded in existing SOURCE.json")
    ap.add_argument("--rediscover", action="store_true",
                    help="relock: re-resolve upstream HEADs (new round)")
    args = ap.parse_args()
    cfg = load_config()

    if args.command == "plan":
        plan(cfg, args.source)
    elif args.command == "init":
        cmd_init(cfg)
    elif args.command == "relock":
        cmd_relock(cfg, args.rediscover)
    elif args.command == "acquire":
        acquire(cfg)
    elif args.command == "materialize":
        materialize(cfg, args.source)
    elif args.command == "verify":
        verify(cfg, args.full)
    elif args.command == "report":
        print(report(cfg))
    elif args.command == "determinism-check":
        determinism_check(cfg)


if __name__ == "__main__":
    main()
