#!/usr/bin/env python3
"""Offline deterministic unit tests for tools/acquire.py (#33 corrective 1).

Focus: the immutable-lock state machine (fail-closed replay preflight), glob
semantics, inventory selection marking, manifest ordering, SOURCE closure, and
config validation. Every adversarial test also asserts the byte-state
invariance property: a rejected operation must leave the workspace byte-for-
byte unchanged (termination BEFORE any mutation).

Run: python3 tools/test_acquire.py   (stdlib only, no network)
"""

from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import shutil
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("acquire", TOOLS_DIR / "acquire.py")
acquire = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(acquire)


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def workspace_digest(root: Path) -> str:
    """Full byte-state digest of a workspace (every file, sorted by relpath)."""
    h = hashlib.sha256()
    for p in sorted(root.rglob("*")):
        if not p.is_file():
            continue
        h.update(str(p.relative_to(root)).encode("utf-8"))
        h.update(b"\0")
        h.update(sha256_hex(p.read_bytes()).encode("ascii"))
        h.update(b"\0")
    return h.hexdigest()


CFG_SOURCE = {
    "authority_issue": "33",
    "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
    "candidate_role": "LARGE_TECHNICAL",
    "category": "LARGE_TECHNICAL",
    "exclude": [],
    "include": ["/docs/**/*.md", "/*.md"],
    "license": {
        "candidate_paths": ["LICENSE", "LICENSE.md"],
        "note": None,
        "spdx": "MIT",
    },
    "notes": "test source",
    "repository_url": "https://github.com/example/test",
    "scope_rationale": "test scope",
    "snapshot_mode": "B",
    "source_id": "test-src",
    "storage_policy": "VENDORED",
    "storage_policy_rationale": "test policy rationale",
}

PIN = "0" * 40


class AcquireTestCase(unittest.TestCase):
    """Base: captures stderr so failure messages can be asserted."""

    def setUp(self) -> None:
        self.stderr = io.StringIO()

    def assert_fail_with(self, msg_part: str, fn, *args, **kwargs) -> None:
        """fn must fail via fail() with msg_part on stderr."""
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.ToolExit):
            fn(*args, **kwargs)
        self.assertIn(msg_part, buf.getvalue())


class WorkspaceTest(AcquireTestCase):
    """Base: build a consistent minimal locked workspace in a temp dir."""

    def setUp(self) -> None:
        super().setUp()
        self.tmp = Path(tempfile.mkdtemp(prefix="acquire-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.cfg = self.build_config()
        self.build_workspace()

    def build_config(self) -> dict:
        return {
            "authority_issue": "33",
            "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
            "notes": "test",
            "round": 1,
            "schema_version": "2",
            "sources": [json.loads(json.dumps(CFG_SOURCE))],
        }

    def build_workspace(self, lock: bool = True) -> None:
        acquire.WORKLOADS_DIR = self.tmp
        acquire.CONFIG_PATH = self.tmp / "acquisition-config.json"
        acquire.SOURCES_DIR = self.tmp / "sources"
        acquire.MANIFESTS_DIR = self.tmp / "manifests"
        acquire.CACHE_DIR = self.tmp / "_cache" / "repos"
        acquire.LOCK_PATH = self.tmp / "source-lock.json"
        acquire.LICENSES_DIR = self.tmp / "licenses"
        acquire.INVENTORY_DIR = acquire.MANIFESTS_DIR / "inventory"
        acquire.UNIVERSE_PATH = acquire.MANIFESTS_DIR / "candidate-universe-v1.json"
        acquire.DUPLICATES_PATH = acquire.MANIFESTS_DIR / "exact-duplicates-v1.json"
        acquire.INVENTORY_SUMMARY_PATH = acquire.MANIFESTS_DIR / "inventory-summary-v1.json"
        acquire.write_json_deterministic(acquire.CONFIG_PATH, self.cfg)
        # frozen SOURCE.json + snapshot byte layout
        source_obj = {
            "schema_version": "2",
            "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
            "authority_issue": "33",
            "source_id": "test-src",
            "repository_url": "https://github.com/example/test",
            "commit_sha": PIN,
            "commit_timestamp": "2026-01-01T00:00:00Z",
            "default_branch": "main",
            "candidate_role": "LARGE_TECHNICAL",
            "role_caveat": None,
            "storage_policy": "VENDORED",
            "storage_policy_rationale": "test policy rationale",
            "scope_rationale": "test scope",
            "snapshot_policy": {
                "mode": "B",
                "mode_a_selected_files": False,
                "include": ["/docs/**/*.md", "/*.md"],
                "exclude": [],
            },
            "license": {
                "spdx": "MIT",
                "files": [{"upstream_path": "LICENSE", "git_blob_sha1": "a" * 40}],
                "preserved_artifacts": [],
                "note": None,
                "redistribution_note": "n/a",
            },
            "extraction_method": "test",
            "file_count": 2,
            "total_bytes": 14,
            "files": [
                {"upstream_path": "README.md", "snapshot_path": "files/README.md",
                 "sha256": sha256_hex(b"# hi\n"), "bytes": 6,
                 "git_blob_sha1": "b" * 40},
                {"upstream_path": "docs/a.md", "snapshot_path": "files/docs/a.md",
                 "sha256": sha256_hex(b"hello md"), "bytes": 8,
                 "git_blob_sha1": "c" * 40},
            ],
            "retrieved_at": "2026-01-01T00:00:00Z",
        }
        sdir = acquire.SOURCES_DIR / "test-src"
        acquire.write_json_deterministic(sdir / "SOURCE.json", source_obj)
        (sdir / "files").mkdir(parents=True, exist_ok=True)
        (sdir / "files" / "README.md").write_bytes(b"# hi\n")
        (sdir / "files" / "docs").mkdir(parents=True, exist_ok=True)
        (sdir / "files" / "docs" / "a.md").write_bytes(b"hello md")
        inv = {
            "schema_version": "markdown-inventory-v1",
            "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
            "authority_issue": "33",
            "source_id": "test-src",
            "repository_url": "https://github.com/example/test",
            "commit_sha": PIN,
            "inventory_definition": "test",
            "repository_md_files": 3,
            "repository_md_bytes": 19,
            "snapshot_md_files": 2,
            "snapshot_md_bytes": 14,
            "entries": [
                {"upstream_path": "README.md", "git_blob_sha1": "b" * 40,
                 "bytes": 6, "selected_for_snapshot": True,
                 "selection_reason": "included:/*.md"},
                {"upstream_path": "guides/contributing.md", "git_blob_sha1": "d" * 40,
                 "bytes": 5, "selected_for_snapshot": False,
                 "selection_reason": "not matched by any include pattern"},
                {"upstream_path": "docs/a.md", "git_blob_sha1": "c" * 40,
                 "bytes": 8, "selected_for_snapshot": True,
                 "selection_reason": "included:/docs/**/*.md"},
            ],
        }
        acquire.write_json_deterministic(acquire.INVENTORY_DIR / "test-src.json", inv)
        objs = {"test-src": source_obj}
        invs = {"test-src": inv}
        if lock:
            acquire.write_json_deterministic(
                acquire.UNIVERSE_PATH,
                acquire.render_universe_manifest(self.cfg, objs, invs))
            acquire.write_json_deterministic(
                acquire.DUPLICATES_PATH, acquire.render_duplicates_manifest(objs))
            acquire.write_json_deterministic(
                acquire.INVENTORY_SUMMARY_PATH,
                acquire.render_inventory_summary(invs))
            lock_obj = {
                "schema_version": "2",
                "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
                "authority_issue": "33",
                "protocol_root": "test",
                "lock_semantics": "test",
                "acquisition_config": "acquisition-config.json",
                "acquisition_config_sha256": sha256_hex(
                    acquire.CONFIG_PATH.read_bytes()),
                "acquisition_tool": "tools/acquire.py",
                "acquisition_tool_sha256": sha256_hex(acquire.TOOL_PATH.read_bytes()),
                "inventory_manifests": {"test-src.json": sha256_hex(
                    (acquire.INVENTORY_DIR / "test-src.json").read_bytes())},
                "candidate_universe_manifest": acquire.UNIVERSE_PATH.name,
                "candidate_universe_manifest_sha256": sha256_hex(
                    acquire.UNIVERSE_PATH.read_bytes()),
                "exact_duplicates_manifest": acquire.DUPLICATES_PATH.name,
                "exact_duplicates_manifest_sha256": sha256_hex(
                    acquire.DUPLICATES_PATH.read_bytes()),
                "inventory_summary_manifest": acquire.INVENTORY_SUMMARY_PATH.name,
                "inventory_summary_manifest_sha256": sha256_hex(
                    acquire.INVENTORY_SUMMARY_PATH.read_bytes()),
                "identity_semantics": "test",
                "sources": [{
                    "source_id": "test-src",
                    "repository_url": "https://github.com/example/test",
                    "commit_sha": PIN,
                    "storage_policy": "VENDORED",
                    "source_manifest_hash":
                        acquire.source_manifest_identity(source_obj),
                }],
            }
            acquire.write_json_deterministic(acquire.LOCK_PATH, lock_obj)

    def assert_preflight_rejects_with(self, msg_part: str) -> None:
        """The strict-replay preflight must reject with msg_part on stderr AND
        leave the workspace byte-for-byte unchanged (termination BEFORE any
        mutation). The digest is captured at call time: any deliberate test
        tampering already happened by then."""
        before = workspace_digest(self.tmp)
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.PreflightError):
            acquire.preflight_replay(self.cfg)
        self.assertIn(msg_part, buf.getvalue())
        self.assertEqual(workspace_digest(self.tmp), before,
                         "rejected operation mutated the workspace")


class GlobSemantics(AcquireTestCase):
    def test_root_anchored(self):
        g = acquire.GlobSet(["/*.md"])
        self.assertTrue(g.match("README.md"))
        self.assertFalse(g.match("docs/a.md"))

    def test_double_star_spans(self):
        g = acquire.GlobSet(["/docs/**/*.md"])
        self.assertTrue(g.match("docs/a.md"))
        self.assertTrue(g.match("docs/sub/deep/a.md"))
        self.assertFalse(g.match("other/a.md"))
        self.assertFalse(g.match("docs/a.md.bak"))

    def test_unanchored_pattern_matches_any_depth(self):
        g = acquire.GlobSet(["*.md"])
        self.assertTrue(g.match("README.md"))
        self.assertTrue(g.match("a/b/README.md"))

    def test_single_star_within_segment(self):
        g = acquire.GlobSet(["/chapter_*/index.md"])
        self.assertTrue(g.match("chapter_1/index.md"))
        self.assertFalse(g.match("chapter_1/sub/index.md"))

    def test_literal_chars_escaped(self):
        g = acquire.GlobSet(["/EIPS/eip-1.md"])
        self.assertTrue(g.match("EIPS/eip-1.md"))
        self.assertFalse(g.match("EIPS/eip-11.md"))

    def test_first_match_reports_deciding_rule(self):
        g = acquire.GlobSet(["/docs/**/*.md", "/*.md"])
        self.assertEqual(g.first_match("docs/a.md"), "/docs/**/*.md")
        self.assertEqual(g.first_match("README.md"), "/*.md")
        self.assertIsNone(g.first_match("code/x.py"))


class SelectionMarking(AcquireTestCase):
    def test_complete_marking_with_reasons(self):
        tree = {
            "README.md": "b" * 40,
            "docs/a.md": "c" * 40,
            "docs/deep/b.md": "d" * 40,
            "guides/contributing.md": "e" * 40,
            "code/main.py": "f" * 40,
            "LICENSE": "0" * 40,
            "notes.markdown": "1" * 40,
        }
        entries = acquire.build_inventory_entries(
            tree, acquire.GlobSet(["/docs/**/*.md", "/*.md"]), acquire.GlobSet([]))
        by_path = {e["upstream_path"]: e for e in entries}
        # complete: every repository Markdown file appears exactly once
        self.assertEqual(sorted(by_path),
                         ["README.md", "docs/a.md", "docs/deep/b.md",
                          "guides/contributing.md", "notes.markdown"])
        self.assertTrue(by_path["README.md"]["selected_for_snapshot"])
        self.assertEqual(by_path["README.md"]["selection_reason"], "included:/*.md")
        self.assertTrue(by_path["docs/deep/b.md"]["selected_for_snapshot"])
        self.assertEqual(by_path["docs/deep/b.md"]["selection_reason"],
                         "included:/docs/**/*.md")
        self.assertFalse(by_path["guides/contributing.md"]["selected_for_snapshot"])
        self.assertEqual(by_path["guides/contributing.md"]["selection_reason"],
                         "not matched by any include pattern")
        # non-Markdown never enters the inventory
        self.assertNotIn("code/main.py", by_path)
        self.assertNotIn("LICENSE", by_path)

    def test_exclude_overrides_include_with_reason(self):
        tree = {"spec.md": "a" * 40, "LICENSE.md": "b" * 40}
        entries = acquire.build_inventory_entries(
            tree, acquire.GlobSet(["/*.md"]),
            acquire.GlobSet(["/LICENSE.md"]))
        by_path = {e["upstream_path"]: e for e in entries}
        self.assertTrue(by_path["spec.md"]["selected_for_snapshot"])
        self.assertFalse(by_path["LICENSE.md"]["selected_for_snapshot"])
        self.assertEqual(by_path["LICENSE.md"]["selection_reason"],
                         "excluded:/LICENSE.md (include:/*.md)")

    def test_sorted_output(self):
        tree = {"z.md": "a" * 40, "a.md": "b" * 40, "m/x.md": "c" * 40}
        entries = acquire.build_inventory_entries(
            tree, acquire.GlobSet(["**/*.md"]), acquire.GlobSet([]))
        self.assertEqual([e["upstream_path"] for e in entries],
                         ["a.md", "m/x.md", "z.md"])


class SourceClosure(AcquireTestCase):
    def make_obj(self, **overrides):
        obj = {
            "source_id": "s",
            "file_count": 2,
            "total_bytes": 14,
            "files": [
                {"upstream_path": "a.md", "snapshot_path": "files/a.md",
                 "sha256": "1" * 64, "bytes": 6, "git_blob_sha1": "a" * 40},
                {"upstream_path": "b.md", "snapshot_path": "files/b.md",
                 "sha256": "2" * 64, "bytes": 8, "git_blob_sha1": "b" * 40},
            ],
        }
        obj.update(overrides)
        return obj

    def test_consistent_obj_passes(self):
        acquire.check_source_closure(self.make_obj())

    def test_file_count_mismatch_fails(self):
        self.assert_fail_with("file_count",
                              acquire.check_source_closure,
                              self.make_obj(file_count=3))

    def test_total_bytes_mismatch_fails(self):
        self.assert_fail_with("total_bytes",
                              acquire.check_source_closure,
                              self.make_obj(total_bytes=999))

    def test_unsorted_files_fail(self):
        obj = self.make_obj()
        obj["files"] = list(reversed(obj["files"]))
        self.assert_fail_with("sorted", acquire.check_source_closure, obj)

    def test_snapshot_layout_mismatch_fails(self):
        obj = self.make_obj()
        obj["files"][0]["snapshot_path"] = "files/wrong.md"
        self.assert_fail_with("snapshot_path",
                              acquire.check_source_closure, obj)


class InventoryClosure(AcquireTestCase):
    def make_inv(self):
        return {
            "source_id": "s",
            "commit_sha": PIN,
            "repository_md_files": 3,
            "repository_md_bytes": 30,
            "snapshot_md_files": 2,
            "snapshot_md_bytes": 14,
            "entries": [
                {"upstream_path": "README.md", "git_blob_sha1": "1" * 40,
                 "bytes": 6, "selected_for_snapshot": True, "selection_reason": "r"},
                {"upstream_path": "docs/a.md", "git_blob_sha1": "2" * 40,
                 "bytes": 8, "selected_for_snapshot": True, "selection_reason": "r"},
                {"upstream_path": "other.md", "git_blob_sha1": "3" * 40,
                 "bytes": 16, "selected_for_snapshot": False, "selection_reason": "r"},
            ],
        }

    def make_src(self):
        return {
            "source_id": "s",
            "commit_sha": PIN,
            "files": [
                {"upstream_path": "README.md", "snapshot_path": "files/README.md",
                 "sha256": "1" * 64, "bytes": 6, "git_blob_sha1": "1" * 40},
                {"upstream_path": "docs/a.md", "snapshot_path": "files/docs/a.md",
                 "sha256": "2" * 64, "bytes": 8, "git_blob_sha1": "2" * 40},
            ],
        }

    def test_closes(self):
        acquire.check_inventory_closure(self.make_inv(), self.make_src())

    def test_selection_gap_fails(self):
        inv = self.make_inv()
        inv["entries"][2]["selected_for_snapshot"] = True
        self.assert_fail_with("closure",
                              acquire.check_inventory_closure, inv, self.make_src())

    def test_extra_snapshot_file_fails(self):
        src = self.make_src()
        src["files"].append({"upstream_path": "zz.md",
                             "snapshot_path": "files/zz.md",
                             "sha256": "9" * 64, "bytes": 1,
                             "git_blob_sha1": "9" * 40})
        self.assert_fail_with("closure",
                              acquire.check_inventory_closure, self.make_inv(), src)

    def test_byte_total_gap_fails(self):
        inv = self.make_inv()
        inv["snapshot_md_bytes"] = 99
        self.assert_fail_with("byte total",
                              acquire.check_inventory_closure, inv, self.make_src())

    def test_repository_byte_total_gap_fails(self):
        inv = self.make_inv()
        inv["repository_md_bytes"] = 99
        self.assert_fail_with("byte total",
                              acquire.check_inventory_closure, inv, self.make_src())


class ManifestOrdering(WorkspaceTest):
    def load_fixture(self):
        inv = json.loads((acquire.INVENTORY_DIR / "test-src.json").read_text())
        objs = {"test-src": json.loads(
            (acquire.SOURCES_DIR / "test-src" / "SOURCE.json").read_text())}
        return inv, objs

    def test_universe_candidates_sorted_and_complete(self):
        inv, objs = self.load_fixture()
        u = acquire.render_universe_manifest(self.cfg, objs, {"test-src": inv})
        ids = [c["source_id"] for c in u["candidates"]]
        self.assertEqual(ids, sorted(ids))
        c = u["candidates"][0]
        self.assertEqual(c["counts_as_role_coverage"], True)
        self.assertIn("file_coverage", c)
        self.assertIn("byte_coverage", c)

    def test_role_caveat_disables_role_coverage(self):
        cfg = self.build_config()
        cfg["sources"][0]["role_caveat"] = "INTENDED_BOOK_SOURCE_IS_HTML: test"
        inv, objs = self.load_fixture()
        u = acquire.render_universe_manifest(cfg, objs, {"test-src": inv})
        self.assertFalse(u["candidates"][0]["counts_as_role_coverage"])
        self.assertIsNotNone(u["candidates"][0]["role_caveat"])

    def test_duplicate_groups_sorted(self):
        objs = {
            "a-src": {"files": [
                {"sha256": "b" * 64, "snapshot_path": "files/x.md"},
                {"sha256": "a" * 64, "snapshot_path": "files/y.md"},
            ]},
            "b-src": {"files": [
                {"sha256": "b" * 64, "snapshot_path": "files/copy.md"},
                {"sha256": "a" * 64, "snapshot_path": "files/other.md"},
            ]},
        }
        d = acquire.render_duplicates_manifest(objs)
        groups = d["duplicate_groups"]
        self.assertEqual([g["sha256"][0] for g in groups], ["a", "b"])
        self.assertEqual(groups[0]["paths"],
                         ["a-src/files/y.md", "b-src/files/other.md"])
        self.assertEqual(groups[1]["paths"],
                         ["a-src/files/x.md", "b-src/files/copy.md"])

    def test_summary_rows_sorted_and_close(self):
        inv, _ = self.load_fixture()
        s = acquire.render_inventory_summary({"test-src": inv})
        self.assertEqual([r["source_id"] for r in s["sources"]], ["test-src"])
        self.assertEqual(s["totals"]["repository_md_files"], 3)
        self.assertEqual(s["totals"]["snapshot_md_bytes"], 14)


class ConfigValidation(AcquireTestCase):
    def base_cfg(self):
        return {
            "campaign_id": "MARKIT-REAL-WORKLOAD-ACQUISITION-1",
            "sources": [json.loads(json.dumps(CFG_SOURCE))],
        }

    def test_valid_config_passes(self):
        acquire.validate_config(self.base_cfg())

    def test_needs_review_must_not_vendor(self):
        cfg = self.base_cfg()
        cfg["sources"][0]["license"]["spdx"] = "NEEDS_REVIEW"
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.ToolExit):
            acquire.validate_config(cfg)
        self.assertIn("NEEDS_REVIEW", buf.getvalue())
        cfg["sources"][0]["storage_policy"] = "MATERIALIZE_ONLY"
        acquire.validate_config(cfg)

    def test_missing_storage_policy_fails(self):
        cfg = self.base_cfg()
        del cfg["sources"][0]["storage_policy"]
        self.assert_fail_with("storage_policy", acquire.validate_config, cfg)

    def test_missing_scope_rationale_fails(self):
        cfg = self.base_cfg()
        del cfg["sources"][0]["scope_rationale"]
        self.assert_fail_with("scope_rationale", acquire.validate_config, cfg)

    def test_duplicate_ids_fail(self):
        cfg = self.base_cfg()
        cfg["sources"].append(json.loads(json.dumps(cfg["sources"][0])))
        self.assert_fail_with("duplicate", acquire.validate_config, cfg)


class LockReplayStateMachine(WorkspaceTest):
    """The fail-closed lock state machine: every adversarial mutation must be
    rejected BEFORE any mutation, leaving the workspace byte-identical."""

    def test_valid_workspace_preflights(self):
        lock = acquire.preflight_replay(self.cfg)
        self.assertEqual(lock["schema_version"], "2")

    def test_changed_config_rejected(self):
        acquire.write_json_deterministic(
            acquire.CONFIG_PATH, {**self.cfg, "notes": "tampered"})
        self.assert_preflight_rejects_with("acquisition-config.json")

    def test_changed_include_glob_rejected(self):
        cfg = json.loads(json.dumps(self.cfg))
        cfg["sources"][0]["include"] = ["/*.md"]  # narrowed frame
        acquire.write_json_deterministic(acquire.CONFIG_PATH, cfg)
        self.assert_preflight_rejects_with("acquisition-config.json")

    def test_changed_tool_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["acquisition_tool_sha256"] = "f" * 64
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("acquire.py")

    def test_missing_lock_with_existing_sources_rejected(self):
        acquire.LOCK_PATH.unlink()
        self.assert_preflight_rejects_with("source-lock.json is missing")

    def test_missing_lock_message_distinguishes_cold_start(self):
        acquire.LOCK_PATH.unlink()
        self.assert_preflight_rejects_with("relock")
        shutil.rmtree(acquire.SOURCES_DIR)
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.PreflightError):
            acquire.preflight_replay(self.cfg)
        self.assertIn("init", buf.getvalue())

    def test_missing_source_pin_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["sources"][0]["commit_sha"] = "nothex"
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("pinned commit SHA")

    def test_missing_source_entry_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["sources"] = []
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("differ")

    def test_extra_source_entry_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["sources"].append({
            "source_id": "ghost-src",
            "repository_url": "https://github.com/example/ghost",
            "commit_sha": PIN,
            "storage_policy": "VENDORED",
            "source_manifest_hash": "1" * 64,
        })
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("differ")

    def test_changed_source_identity_rejected(self):
        sj = acquire.SOURCES_DIR / "test-src" / "SOURCE.json"
        obj = json.loads(sj.read_text())
        obj["commit_sha"] = "1" * 40  # re-pinned under the same lock
        acquire.write_json_deterministic(sj, obj)
        self.assert_preflight_rejects_with("SOURCE identity change rejected")

    def test_mutated_source_manifest_rejected(self):
        sj = acquire.SOURCES_DIR / "test-src" / "SOURCE.json"
        obj = json.loads(sj.read_text())
        obj["total_bytes"] += 1  # any manifest drift
        acquire.write_json_deterministic(sj, obj)
        self.assert_preflight_rejects_with("may never be rewritten")

    def test_missing_source_manifest_rejected(self):
        (acquire.SOURCES_DIR / "test-src" / "SOURCE.json").unlink()
        self.assert_preflight_rejects_with("SOURCE.json is missing")

    def test_changed_storage_policy_rejected(self):
        cfg = json.loads(json.dumps(self.cfg))
        cfg["sources"][0]["storage_policy"] = "MATERIALIZE_ONLY"
        acquire.write_json_deterministic(acquire.CONFIG_PATH, cfg)
        self.assert_preflight_rejects_with("acquisition-config.json")

        # even with a re-signed config hash in the lock, policy divergence fails
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["acquisition_config_sha256"] = sha256_hex(
            acquire.CONFIG_PATH.read_bytes())
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        before = workspace_digest(self.tmp)
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.PreflightError):
            acquire.preflight_replay(cfg)
        self.assertIn("storage_policy", buf.getvalue())
        self.assertEqual(workspace_digest(self.tmp), before)
        # the rejected preflight mutated no source state
        self.assertEqual(
            json.loads(
                (acquire.SOURCES_DIR / "test-src" / "SOURCE.json").read_text()
            )["storage_policy"],
            "VENDORED")

    def test_changed_repository_identity_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["sources"][0]["repository_url"] = "https://github.com/example/other"
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("repository_url differs from config")

    def test_schema_downgrade_rejected(self):
        lock = json.loads(acquire.LOCK_PATH.read_text())
        lock["schema_version"] = "1"
        acquire.write_json_deterministic(acquire.LOCK_PATH, lock)
        self.assert_preflight_rejects_with("schema_version")

    def test_unknown_source_filter_rejected(self):
        buf = io.StringIO()
        with redirect_stderr(buf), self.assertRaises(acquire.PreflightError):
            acquire.preflight_replay(self.cfg, "not-a-source")
        self.assertIn("unknown source_id", buf.getvalue())

    def test_acquire_strict_replay_is_byte_stable(self):
        before = workspace_digest(self.tmp)
        acquire.acquire(self.cfg)
        self.assertEqual(workspace_digest(self.tmp), before,
                         "strict replay mutated derived manifests or anything else")


if __name__ == "__main__":
    unittest.main(verbosity=2)
