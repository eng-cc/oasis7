#!/usr/bin/env python3
"""Fixture-only cache ownership and reclamation regression tests."""
import importlib.util
import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

spec = importlib.util.spec_from_file_location("cargo_cache", Path(__file__).with_name("cargo-cache.py"))
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class CacheTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "repo"
        self.root.mkdir()
        self.git("init")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--allow-empty", "-m", "initial")
        self.common, _, self.family = cache.layout(self.root)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], stderr=subprocess.DEVNULL, text=True)

    def target(self, owner):
        return self.family / ("worktree-" + cache.digest(owner)) / "rustc-fixture-host"

    def create(self, owner, managed=True):
        target = self.target(owner)
        target.mkdir(parents=True)
        (target / "artifact").write_text("fixture")
        if managed:
            cache.write_metadata(target, owner, self.common)
            data = json.loads((target / cache.MARKER).read_text())
            data["last_used"] = time.time() - 10 * 86400
            (target / cache.MARKER).write_text(json.dumps(data))
        return target

    def test_wrapper_only_registered_target_discovered(self):
        target = self.create(self.root)
        self.assertFalse((self.root / "target").exists())
        row = cache.inventory(self.root)["targets"][0]
        self.assertEqual(row["target"], str(target))
        self.assertTrue(row["registered"])
        self.assertGreater(row["bytes"], 0)
        self.assertEqual(cache.collect(self.root)["actions"], [])

    @unittest.skipUnless(os.name == "posix", "actual GC requires Unix inherited leases")
    def test_orphan_dry_run_then_delete_and_unknown_preserved(self):
        orphan = self.create(Path(self.tmp.name) / "removed")
        unknown = self.create(Path(self.tmp.name) / "legacy", False)
        result = cache.collect(self.root)
        self.assertEqual(result["actions"][0]["action"], "would-delete")
        self.assertTrue(orphan.exists())
        cache.collect(self.root, True)
        self.assertFalse(orphan.exists())
        self.assertTrue(unknown.exists())

    def test_missing_registered_owner_preserved(self):
        worktree = Path(self.tmp.name) / "missing"
        self.git("worktree", "add", "-b", "fixture", str(worktree))
        target = self.create(worktree)
        import shutil
        shutil.rmtree(worktree)
        self.assertTrue(cache.inventory(self.root)["targets"][0]["registered"])
        self.assertEqual(cache.collect(self.root)["actions"], [])
        self.assertTrue(target.exists())

    def test_other_namespace_symlink_reference_preserved(self):
        target = self.create(Path(self.tmp.name) / "removed")
        (self.root / "target").symlink_to(target)
        self.assertEqual(cache.inventory(self.root)["targets"][0]["status"], "referenced")
        self.assertEqual(cache.collect(self.root)["actions"], [])

    @unittest.skipUnless(os.name == "posix", "actual GC requires Unix inherited leases")
    def test_busy_lock_refused(self):
        target = self.create(Path(self.tmp.name) / "removed")
        with cache.locked(target, self.family):
            self.assertEqual(cache.collect(self.root, True)["actions"][0]["action"], "busy-or-unsafe")
        self.assertTrue(target.exists())

    @unittest.skipUnless(os.name == "posix", "actual GC requires Unix inherited leases")
    def test_metadata_tamper_age_and_nested_symlink(self):
        target = self.create(Path(self.tmp.name) / "removed")
        marker = target / cache.MARKER
        original = json.loads(marker.read_text())
        tampered = dict(original, common_dir="/different")
        marker.write_text(json.dumps(tampered))
        self.assertFalse(cache.inventory(self.root)["targets"][0]["managed"])
        marker.write_text(json.dumps(dict(original, last_used=time.time())))
        self.assertEqual(cache.collect(self.root)["actions"], [])
        marker.write_text(json.dumps(original))
        (target / "external").symlink_to(self.root)
        self.assertEqual(cache.collect(self.root, True)["actions"][0]["action"], "preserved-symlink")
        self.assertTrue(target.exists())

    @unittest.skipUnless(os.name == "posix", "GC deletion window uses Unix leases")
    def test_worktree_attachment_protects_orphan_through_registration(self):
        owner = Path(self.tmp.name) / "reused"
        target = self.create(owner)
        with cache.locked(self.family / ".attachments", self.family):
            self.assertEqual(cache.collect(self.root, True)["actions"][0]["action"], "busy-or-unsafe")
            self.git("worktree", "add", "-b", "reused", str(owner))
        self.assertEqual(cache.collect(self.root, True)["actions"], [])
        self.assertTrue(target.exists())

    @unittest.skipUnless(os.name == "posix", "Unix attachment lock")
    def test_attachment_waits_for_deletion_window(self):
        ready = Path(self.tmp.name) / "attached"
        command = [sys.executable, "-c", "from pathlib import Path; Path(%r).touch()" % str(ready)]
        with cache.locked(self.family / ".attachments", self.family, exclusive=True):
            child = subprocess.Popen([sys.executable, cache.__file__, "attach", "--repo-root", str(self.root), "--", *command])
            try:
                time.sleep(0.2)
                self.assertIsNone(child.poll())
                self.assertFalse(ready.exists())
            except BaseException:
                child.kill()
                child.wait(timeout=5)
                raise
        self.assertEqual(child.wait(timeout=5), 0)
        self.assertTrue(ready.exists())

    def test_lease_enrolls_empty_only_and_propagates_exit(self):
        target = self.target(self.root)
        self.assertEqual(cache.lease(target, self.root, self.common, [sys.executable, "-c", "raise SystemExit(7)"]), 7)
        self.assertTrue(cache.read_metadata(target, self.common))
        (target / cache.MARKER).unlink()
        (target / "legacy").write_text("legacy")
        self.assertEqual(cache.lease(target, self.root, self.common, [sys.executable, "-c", "pass"]), 0)
        self.assertFalse((target / cache.MARKER).exists())

    @unittest.skipUnless(os.name == "posix", "inherited Unix lease")
    def test_child_keeps_lease_after_supervisor_is_killed(self):
        target = self.target(self.root)
        ready = Path(self.tmp.name) / "child-ready"
        child_code = "import os,time; from pathlib import Path; Path(%r).write_text(str(os.getpid())); time.sleep(30)" % str(ready)
        supervisor = subprocess.Popen([sys.executable, str(Path(cache.__file__)), "lease",
            "--target", str(target), "--repo-root", str(self.root),
            "--common-dir", str(self.common), "--", sys.executable, "-c", child_code])
        child_pid = None
        try:
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(ready.exists(), "child did not start")
            child_pid = int(ready.read_text())
            supervisor.kill()
            supervisor.wait(timeout=5)
            with self.assertRaises(BlockingIOError):
                with cache.locked(target, self.family, exclusive=True, blocking=False):
                    pass
        finally:
            if supervisor.poll() is None:
                supervisor.kill()
                supervisor.wait(timeout=5)
            if child_pid:
                os.kill(child_pid, signal.SIGTERM)

    def test_legacy_namespace_and_nonfinite_input(self):
        legacy = self.family / "rustc-legacy-host"
        legacy.mkdir(parents=True)
        (legacy / "artifact").write_text("legacy")
        self.assertEqual(cache.inventory(self.root)["targets"][0]["status"], "unknown")
        self.assertEqual(cache.collect(self.root)["actions"], [])
        for kwargs in ({"min_age_days": float("nan")}, {"budget_gib": float("inf")}):
            with self.assertRaises(ValueError):
                cache.collect(self.root, **kwargs)

    @unittest.skipUnless(os.name == "posix", "newline paths are not valid on Windows")
    def test_newline_worktree_path(self):
        worktree = Path(self.tmp.name) / "odd\nworktree"
        self.git("worktree", "add", "-b", "odd", str(worktree))
        self.create(worktree)
        self.assertTrue(cache.inventory(self.root)["targets"][0]["registered"])

    @unittest.skipUnless(os.name == "nt", "Windows deletion boundary")
    def test_windows_apply_rejected(self):
        with self.assertRaisesRegex(ValueError, "Unix inherited leases"):
            cache.collect(self.root, True)

    def test_external_override_unmanaged(self):
        target = Path(self.tmp.name) / "override"
        self.assertEqual(cache.lease(target, self.root, self.common, [sys.executable, "-c", "pass"]), 0)
        self.assertFalse(target.exists())


if __name__ == "__main__":
    unittest.main()
