#!/usr/bin/env python3
import importlib.util
import multiprocessing
from pathlib import Path
import tempfile
import time
import unittest

PM = Path(__file__).parent
ROOT = PM.parent.parent


def _finalizer_lock_worker(repo_root, task_uid, entered, release):
    spec = importlib.util.spec_from_file_location(
        "post_merge_finalize_under_test", PM / "post-merge-finalize.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    def critical_section(root, uid, receipt):
        entered.set()
        release.wait(10)
        return 0

    module._write_terminal_locked = critical_section
    module._write_terminal(Path(repo_root), task_uid, Path(repo_root) / "receipt.json")

class LifecycleOptimizationContractTests(unittest.TestCase):
    def test_closeout_has_task_scoped_postcondition_readback_and_single_refresh_budget(self):
        source=(PM/"task-closeout.sh").read_text()
        self.assertIn("postcondition", source.lower())
        self.assertIn("task_uid", source.lower())
        self.assertIn("refresh-same-identity", source)
        self.assertEqual(1, source.count("refresh-same-identity"), "closeout may integrate at most one CI freshness refresh")

    def test_finalizer_uses_persistent_task_lock_in_git_common_state(self):
        source=(PM/"post-merge-finalize.py").read_text()
        self.assertIn('.pm/github-project-sync/tasks.json', source)
        self.assertIn('finalizer-lock', source)
        self.assertNotRegex(
            source,
            r"finalizer_lock\.unlink|unlink\([^\n]*finalizer_lock",
            "unlinking a flock path permits old- and new-inode holders to overlap",
        )

    def test_finalizer_three_process_contention_never_splits_lock_inode(self):
        ctx = multiprocessing.get_context("fork")
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary) / ".pm/github-project-sync"
            state.mkdir(parents=True)
            (state / "tasks.json").write_text("{}", encoding="utf-8")
            task_uid = "task_" + "1" * 32
            entered = [ctx.Event() for _ in range(3)]
            release = [ctx.Event() for _ in range(3)]
            processes = [
                ctx.Process(
                    target=_finalizer_lock_worker,
                    args=(temporary, task_uid, entered[index], release[index]),
                )
                for index in range(3)
            ]
            try:
                processes[0].start()
                self.assertTrue(entered[0].wait(3), "first finalizer never acquired lock")
                processes[1].start()
                time.sleep(0.25)  # second process has opened the first lock inode and is waiting
                release[0].set()
                self.assertTrue(entered[1].wait(3), "second finalizer never acquired lock")
                processes[2].start()
                self.assertFalse(
                    entered[2].wait(0.75),
                    "third finalizer entered while second held the old unlinked lock inode",
                )
            finally:
                for event in release:
                    event.set()
                for process in processes:
                    if process.pid is not None:
                        process.join(3)
                        if process.is_alive():
                            process.terminate()
                            process.join(1)

    def test_cleanup_has_distinct_canonical_worktree_diagnostics(self):
        source=(PM/"post-merge-cleanup.sh").read_text().lower()
        for signature in ("worktree path is absent", "not a git worktree", "not registered", "common-dir mismatch"):
            with self.subTest(signature=signature): self.assertIn(signature, source)

    def test_invalid_docs_module_is_pre_mutation_and_suggests_engineering(self):
        source=(ROOT/"scripts/new-task-worktree.sh").read_text().lower()
        validation=source.find("unsupported pm module")
        mutation=min(pos for pos in (source.find("git worktree add"), source.find("new-task.sh")) if pos >= 0)
        self.assertGreaterEqual(validation, 0)
        self.assertLess(validation, mutation)
        self.assertRegex(source[validation:validation+800], r"docs.*engineering|engineering.*docs")

if __name__ == "__main__": unittest.main()
