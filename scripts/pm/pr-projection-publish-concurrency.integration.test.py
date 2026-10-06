#!/usr/bin/env python3
"""Process-level concurrency checks for the real PR publication entrypoint.

The shared fixture, fake GitHub boundary, linked worktrees, local bare remote,
and command builders are owned by pr-projection-publish-cli.integration.test.py.
This file adds only cross-process interleaving checks for its existing fixture.
"""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time
import unittest


HERE = Path(__file__).resolve().parent
BASE_HARNESS = HERE / "pr-projection-publish-cli.integration.test.py"


def load_base_harness():
    spec = importlib.util.spec_from_file_location("publisher_cli_concurrency_base", BASE_HARNESS)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


base = load_base_harness()


GATED_GH = r'''#!/usr/bin/env python3
import fcntl, os, pathlib, subprocess, sys, time

gate = pathlib.Path(os.environ["T25_GATE_DIR"])
caller = os.environ.get("T25_CALLER", "unknown")
gate.mkdir(parents=True, exist_ok=True)
(gate / ("seen-" + caller)).touch()
args = sys.argv[1:]
if args[:2] == ["project", "item-edit"]:
    entered = gate / "project-edit-entered"
    first = False
    try:
        descriptor = os.open(str(entered), os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        os.close(descriptor)
        first = True
    except FileExistsError:
        pass
    with (gate / "project-edit-callers.log").open("a", encoding="utf-8") as stream:
        fcntl.flock(stream.fileno(), fcntl.LOCK_EX)
        stream.write(caller + "\n")
        stream.flush()
        fcntl.flock(stream.fileno(), fcntl.LOCK_UN)
    if first:
        deadline = time.monotonic() + 30
        while not (gate / "release-project-edit").exists():
            if time.monotonic() >= deadline:
                sys.stderr.write("T25 project edit gate timed out\n")
                raise SystemExit(97)
            time.sleep(0.01)
base = pathlib.Path(os.environ["T25_BASE_GH"])
# Serialize each fake GitHub request as one service transaction. This lock is
# independent from Git's common-dir publication lock and prevents the fixture's
# simple JSON-backed fake service from losing concurrent call/effect updates.
with pathlib.Path(os.environ["T25_SERVICE_LOCK"]).open("a+b") as service_lock:
    fcntl.flock(service_lock.fileno(), fcntl.LOCK_EX)
    result = subprocess.run([str(base), *args], check=False)
    fcntl.flock(service_lock.fileno(), fcntl.LOCK_UN)
if (caller == "contender-linked-worktree" and args[:1] == ["api"]
        and args[1].startswith("repos/eng-cc/oasis7/pulls?")):
    (gate / "contender-pr-discovery-finished").touch()
raise SystemExit(result.returncode)
'''


def wait_for(path: Path, *, timeout: float = 30.0) -> None:
    deadline = time.monotonic() + timeout
    while not path.exists():
        if time.monotonic() >= deadline:
            raise AssertionError(f"timed out waiting for {path}")
        time.sleep(0.01)


class PublisherConcurrencyTests(unittest.TestCase):
    maxDiff = None

    def setUp(self):
        # Reuse the exact C-owned real-process fixture; do not create a second
        # publisher abstraction or bypass its canonical worktree checks.
        self.fixture = base.PublisherProcessTests(
            "test_real_publisher_and_record_pr_complete_then_noop_retry",
        )
        self.fixture.setUp()
        self.addCleanup(self.fixture.tearDown)
        self.contender_root = self.fixture.tmp / "contender-linked-worktree"
        subprocess.run([
            "git", "-C", str(self.fixture.repo), "worktree", "add", "--detach",
            str(self.contender_root), self.fixture.source_head,
        ], check=True, text=True, capture_output=True)
        self.gate = self.fixture.tmp / "t25-gate"
        self.gate.mkdir()
        self.service_lock = self.gate / "fake-service.lock"
        self.service_lock.touch()
        self.base_gh = self.fixture.bin / "gh"
        self.real_gh = self.fixture.bin / "gh-real"
        self.base_gh.rename(self.real_gh)
        self.base_gh.write_text(GATED_GH, encoding="utf-8")
        self.base_gh.chmod(0o755)
        self.env = self.fixture.env.copy()
        self.env.update({"T25_GATE_DIR": str(self.gate), "T25_BASE_GH": str(self.real_gh),
                         "T25_SERVICE_LOCK": str(self.service_lock)})

    def _start(self, caller: str, cwd: Path) -> subprocess.Popen[str]:
        env = self.env.copy()
        env["T25_CALLER"] = caller
        return subprocess.Popen(
            self.fixture.publisher_command(), cwd=cwd, env=env,
            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )

    def _finish(self, process: subprocess.Popen[str]) -> dict[str, object]:
        stdout, stderr = process.communicate(timeout=90)
        return {"returncode": process.returncode, "stdout": stdout, "stderr": stderr}

    def _release_gate(self) -> None:
        (self.gate / "release-project-edit").touch()

    def _load_state(self) -> dict[str, object]:
        import fcntl
        with self.service_lock.open("a+b") as lock:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
            return self.fixture._load_state()

    def _update_state(self, update) -> dict[str, object]:
        import fcntl
        with self.service_lock.open("a+b") as lock:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
            state = self.fixture._load_state()
            update(state)
            self.fixture._save_state()
            return state

    def _effects(self, state: dict[str, object], kind: str) -> list[dict[str, object]]:
        return [item for item in state["mutations"]
                if item["kind"] == kind and item["effect"]]

    def _evidence(self, label: str, **values: object) -> None:
        print("T25_EVIDENCE " + json.dumps({"case": label, **values}, sort_keys=True))

    def test_two_linked_worktree_callers_share_publication_lock_and_reconcile_once(self):
        """A second real CLI waits across record-pr's lock release window."""
        first = self._start("task-worktree", self.fixture.task_root)
        second = None
        first_result = second_result = None
        try:
            wait_for(self.gate / "project-edit-entered")
            before = self._load_state()
            self.assertEqual(1, len(self._effects(before, "issue:body")),
                             "the child writer must have completed the Task Issue step first")
            self.assertEqual("execution", before["project_values"]["Workflow Phase"],
                             "the gate must fall inside the Issue-complete/Project-old prefix")

            # A second, real detached linked worktree shares the same Git
            # common-dir branch lock. It invokes the same guarded task CLI and
            # continues to pass the canonical task root as --worktree.
            def common_dir(root: Path) -> Path:
                reported = Path(base.git(root, "rev-parse", "--git-common-dir"))
                return (reported if reported.is_absolute() else root / reported).resolve()

            task_common = common_dir(self.fixture.task_root)
            contender_common = common_dir(self.contender_root)
            self.assertEqual(task_common, contender_common)
            second = self._start("contender-linked-worktree", self.contender_root)
            wait_for(self.gate / "seen-contender-linked-worktree")
            wait_for(self.gate / "contender-pr-discovery-finished")
            time.sleep(0.25)
            self.assertIsNone(first.poll(), "the first caller should still hold the child-writer gate")
            self.assertIsNone(second.poll(), "the second caller should be waiting for the shared journal lock")
            callers = (self.gate / "project-edit-callers.log").read_text(encoding="utf-8").splitlines()
            self.assertEqual(["task-worktree"], callers,
                             "the lock must prevent the competing caller from entering the Project write")
            during = self._load_state()
            self.assertEqual(1, len(self._effects(during, "issue:body")))
            self.assertEqual([], self._effects(during, "project:Workflow Phase"))

            self._release_gate()
            first_result = self._finish(first)
            second_result = self._finish(second)
        finally:
            self._release_gate()
            for process in (first, second):
                if process is not None and process.poll() is None:
                    process.kill()
                    process.communicate(timeout=10)

        self.assertEqual(0, first_result["returncode"], first_result)
        self.assertEqual(0, second_result["returncode"], second_result)
        state = self._load_state()
        self.fixture._assert_complete(state)
        self.assertEqual(1, len(self._effects(state, "pr:create")))
        self.assertEqual(1, len(self._effects(state, "issue:body")))
        self.assertEqual(1, len(self._effects(state, "project:Workflow Phase")))
        self.assertEqual(1, len(self._effects(state, "project:PR")))
        self.assertEqual({"C1": 1, "binding": 1, "evidence": 1}, state["post_attempts"])
        self._evidence("two-linked-worktree-callers", first=first_result,
                       second=second_result, remote_effects={
                           kind: len(self._effects(state, kind))
                           for kind in ("pr:create", "issue:body", "project:Workflow Phase", "project:PR")
                       }, post_attempts=state["post_attempts"],
                       project_edit_callers=(self.gate / "project-edit-callers.log").read_text(
                           encoding="utf-8").splitlines())

    def _run_foreign_pr_drift(self, field: str) -> None:
        first = self._start("task-worktree", self.fixture.task_root)
        result = None
        foreign_value: object
        try:
            wait_for(self.gate / "project-edit-entered")
            state = self._load_state()
            self.assertIsNotNone(state["pr"], "the PR must exist before child record-pr starts")
            self.assertEqual(1, len([item for item in state["comments"]
                                     if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]))
            if field == "head":
                foreign_value = "f" * 40
                self._update_state(lambda live: live["pr"]["head"].__setitem__("sha", foreign_value))
            elif field == "body":
                foreign_value = state["pr"]["body"] + "\nForeign concurrent body edit.\n"
                self._update_state(lambda live: live["pr"].__setitem__("body", foreign_value))
            elif field == "draft":
                foreign_value = False
                self._update_state(lambda live: live["pr"].__setitem__("draft", foreign_value))
            else:
                raise AssertionError(f"unsupported drift field: {field}")
            self._release_gate()
            result = self._finish(first)
        finally:
            self._release_gate()
            if first.poll() is None:
                first.kill()
                first.communicate(timeout=10)

        self.assertNotEqual(0, result["returncode"],
                            f"foreign {field} drift must never produce publication success: {result}")
        final = self._load_state()
        self.assertEqual(foreign_value, self._read_pr_field(final, field),
                         "the publisher must preserve the foreign PR value and never roll it back")
        self.assertEqual(1, len(self._effects(final, "pr:create")))
        self.assertEqual(1, len([item for item in final["comments"]
                                 if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]))
        self.assertEqual(1, len([item for item in final["comments"]
                                 if "<!-- oasis7-ci-publication-binding/v1 -->" in item["body"]]),
                         "record-pr may write its reciprocal evidence before its final PR readback; this comment alone is not success")
        self.assertLessEqual(final["post_attempts"].get("binding", 0), 1,
                             "uncertain/failed child completion must not duplicate its write-once binding")
        self._evidence("record-pr-release-window-foreign-pr-drift", field=field,
                       result=result, foreign_value=foreign_value,
                       project_effects={
                           kind: len(self._effects(final, kind))
                           for kind in ("issue:body", "project:Workflow Phase", "project:PR")
                       }, final_pr={"number": final["pr"]["number"], "draft": final["pr"]["draft"],
                                    "head_sha": final["pr"]["head"]["sha"],
                                    "body": final["pr"]["body"]})

    @staticmethod
    def _read_pr_field(state: dict[str, object], field: str) -> object:
        if field == "head":
            return state["pr"]["head"]["sha"]
        return state["pr"][field]

    def test_parent_child_release_window_rejects_foreign_head_drift_without_rollback(self):
        self._run_foreign_pr_drift("head")

    def test_parent_child_release_window_rejects_foreign_body_drift_without_rollback(self):
        self._run_foreign_pr_drift("body")

    def test_parent_child_release_window_rejects_foreign_draft_drift_without_rollback(self):
        self._run_foreign_pr_drift("draft")


if __name__ == "__main__":
    unittest.main(verbosity=2)
