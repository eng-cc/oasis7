#!/usr/bin/env python3
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location("incremental_clean", Path(__file__).with_name("cargo-incremental-clean.py"))
cleaner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleaner)

@unittest.skipUnless(os.name == "posix", "Unix cleanup")
class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "repo"
        self.root.mkdir()
        import subprocess
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.common, _, self.family = cleaner.cache.layout(self.root)
        self.target = self.family / ("worktree-" + cleaner.cache.digest(self.root)) / "rustc-fixture"
        for name in ("incremental", "deps", "build"):
            p = self.target / "debug" / name
            p.mkdir(parents=True)
            (p / "artifact").write_text("keep" if name != "incremental" else "rebuild")
        cleaner.cache.write_metadata(self.target, self.root, self.common)
        self.usage = mock.patch.object(cleaner.resources, "inspect_process_use", return_value={"state": "clear_observed"})
        self.usage.start()
        self.addCleanup(self.usage.stop)

    def test_preview_and_delete_preserve_dependencies(self):
        before = sorted(str(p.relative_to(self.target)) for p in self.target.rglob("*"))
        report = cleaner.clean(self.root, self.target)
        self.assertEqual(before, sorted(str(p.relative_to(self.target)) for p in self.target.rglob("*")))
        self.assertEqual(report["actions"][0]["action"], "would-delete")
        self.assertTrue((self.target / "debug/incremental").exists())
        cleaner.clean(self.root, self.target, True)
        self.assertFalse((self.target / "debug/incremental").exists())
        self.assertEqual((self.target / "debug/deps/artifact").read_text(), "keep")
        self.assertEqual((self.target / "debug/build/artifact").read_text(), "keep")
        self.assertTrue((self.target / cleaner.cache.MARKER).exists())

    def test_unknown_and_busy_preserved(self):
        with mock.patch.object(cleaner.resources, "inspect_process_use", return_value={"state": "busy", "reason": "fixture busy"}):
            with self.assertRaises(ValueError):
                cleaner.clean(self.root, self.target, True)
        (self.target / cleaner.cache.MARKER).unlink()
        with self.assertRaises(ValueError):
            cleaner.clean(self.root, self.target, True)
        self.assertTrue((self.target / "debug/incremental").exists())

    def test_wrapper_and_raw_cargo_locks_preserved(self):
        import fcntl
        with cleaner.cache.locked(self.target, self.family):
            with self.assertRaises(OSError):
                cleaner.clean(self.root, self.target, True)
        with open(self.target / "debug/.cargo-lock", "w") as f:
            fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaises(OSError):
                cleaner.clean(self.root, self.target, True)
        self.assertTrue((self.target / "debug/incremental").exists())

    def test_symlinks_preserved_without_partial_delete(self):
        release = self.target / "release"
        release.mkdir()
        (release / "incremental").symlink_to(self.target / "debug/incremental", target_is_directory=True)
        with self.assertRaises(ValueError):
            cleaner.clean(self.root, self.target, True)
        self.assertTrue((self.target / "debug/incremental").exists())

if __name__ == "__main__":
    unittest.main()
