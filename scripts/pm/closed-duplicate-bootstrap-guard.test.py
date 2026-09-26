#!/usr/bin/env python3
"""Supplemental admission regressions for closed-duplicate candidate aliases.

The frozen retirement acceptance suite covers the explicit retirement path.
These tests protect the earlier admission edge: a stale candidate cache row
must not be bootstrapped as an active task before that retirement is run.
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PM = ROOT / "scripts/pm"
BOOTSTRAP = PM / "bootstrap-task-snapshot.py"
WORKFLOW_NEXT = PM / "workflow-next.py"
FIXTURE_TEST = PM / "retire-closed-duplicate-candidate.test.py"


def load_fixture_module():
    spec = importlib.util.spec_from_file_location("retirement_fixture_for_bootstrap_guard", FIXTURE_TEST)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load isolated retirement fixture from {FIXTURE_TEST}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


FIXTURE = load_fixture_module()


class ClosedDuplicateBootstrapGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="closed-duplicate-bootstrap-guard-")
        self.fixture = FIXTURE.RetirementFixture(Path(self.temp.name))
        self._prepare_active_looking_candidate()

    def tearDown(self):
        self.temp.cleanup()

    def _prepare_active_looking_candidate(self):
        """Make the cached candidate look sufficient for ordinary bootstrap."""
        subprocess.run(
            ["git", "-C", str(self.fixture.foreign_worktree), "checkout", FIXTURE.FOREIGN_BRANCH],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        for uid in (FIXTURE.CANDIDATE_UID, FIXTURE.FOREIGN_UID):
            record = mapping["tasks"][uid]
            record.update({
                "title": "fixture task title",
                "acceptance": ["snapshot must remain absent on rejection"],
                "default_branch": "main",
            })
        self.fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _remove_foreign_owner_mapping(self):
        """Leave the candidate's cached path/branch unowned so alias-only guards miss it."""
        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        del mapping["tasks"][FIXTURE.FOREIGN_UID]
        self.fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _write_unavailable_gh_stub(self):
        self.fixture.bin.mkdir(parents=True, exist_ok=True)
        (self.fixture.bin / "gh").write_text(
            "#!/bin/sh\n"
            "echo 'fixture: live GitHub authority unavailable' >&2\n"
            "exit 73\n",
            encoding="utf-8",
        )
        (self.fixture.bin / "gh").chmod(0o755)

    def _run_bootstrap(self, uid=FIXTURE.CANDIDATE_UID):
        return subprocess.run(
            [
                sys.executable,
                str(BOOTSTRAP),
                "validate-or-create",
                "--repo-root", str(self.fixture.foreign_worktree),
                "--tasks-json", str(self.fixture.mapping),
                "--task-uid", uid,
                "--producer", "closed-duplicate-bootstrap-guard-test",
            ],
            cwd=ROOT,
            env=self.fixture.environment(),
            text=True,
            capture_output=True,
            check=False,
            timeout=15,
        )

    def _run_workflow_next(self):
        return subprocess.run(
            [
                sys.executable,
                str(WORKFLOW_NEXT),
                "--repo-root", str(self.fixture.foreign_worktree),
                "--mapping", str(self.fixture.mapping),
                "--task-uid", FIXTURE.CANDIDATE_UID,
                "--json",
            ],
            cwd=ROOT,
            env=self.fixture.environment(),
            text=True,
            capture_output=True,
            check=False,
            timeout=15,
        )

    def _snapshot_path(self):
        return self.fixture.foreign_worktree / ".pm/scratch" / FIXTURE.CANDIDATE_UID / "bootstrap-task-snapshot.json"

    def test_bootstrap_refuses_live_closed_duplicate_alias_before_any_write(self):
        self.fixture._write_paginated_live_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate", (result.stderr + result.stdout).lower())
        self.assertIn("retire-closed-duplicate-candidate.py", (result.stderr + result.stdout))
        self.assertEqual(self.fixture.mapping.read_bytes(), before)
        self.assertFalse(self._snapshot_path().exists(), "rejected candidate bootstrap must not create a snapshot")

    def test_workflow_next_returns_reconcile_blocker_instead_of_bootstrap_command(self):
        self.fixture._write_paginated_live_gh_stub()

        result = self._run_workflow_next()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["next_action"], "blocked")
        self.assertEqual(payload["next_command"], [])
        self.assertTrue(any("duplicate" in blocker.lower() for blocker in payload["blockers"]))
        self.assertIn("retire-closed-duplicate-candidate.py", json.dumps(payload))

    def test_bootstrap_refuses_unaliased_live_closed_duplicate_before_snapshot(self):
        self._remove_foreign_owner_mapping()
        self.fixture._write_paginated_live_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate", (result.stderr + result.stdout).lower())
        self.assertIn("retire-closed-duplicate-candidate.py", result.stderr + result.stdout)
        self.assertEqual(self.fixture.mapping.read_bytes(), before)
        self.assertFalse(self._snapshot_path().exists(), "unaliased closed duplicate must not create a bootstrap snapshot")
        calls = [json.loads(line) for line in self.fixture.gh_log.read_text(encoding="utf-8").splitlines()]
        self.assertTrue(any(call[:1] == ["api"] for call in calls), "unaliased candidate must be checked against live Issue truth")

    def test_workflow_next_blocks_unaliased_live_closed_duplicate(self):
        self._remove_foreign_owner_mapping()
        self.fixture._write_paginated_live_gh_stub()

        result = self._run_workflow_next()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["next_action"], "blocked")
        self.assertEqual(payload["next_command"], [])
        self.assertTrue(any("duplicate" in blocker.lower() for blocker in payload["blockers"]))
        self.assertIn("retire-closed-duplicate-candidate.py", json.dumps(payload))

    def test_bootstrap_fails_closed_when_live_issue_authority_is_unavailable(self):
        self._write_unavailable_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(self._snapshot_path().exists(), "unavailable live Issue proof must not create a snapshot")
        self.assertEqual(self.fixture.mapping.read_bytes(), before)

    def test_bootstrap_fails_closed_when_live_issue_uid_is_ambiguous(self):
        self.fixture._write_paginated_live_gh_stub()
        live = json.loads(self.fixture.gh_fixture.read_text(encoding="utf-8"))
        candidate = next(item for item in live["issues"] if item["number"] == FIXTURE.CANDIDATE_ISSUE)
        candidate["body"] += f"\ntask_uid: {FIXTURE.CANDIDATE_UID}\n"
        self.fixture.gh_fixture.write_text(json.dumps(live, sort_keys=True), encoding="utf-8")
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(self._snapshot_path().exists(), "ambiguous live Issue identity must not create a snapshot")
        self.assertEqual(self.fixture.mapping.read_bytes(), before)

    def test_existing_active_task_bootstrap_remains_compatible_without_candidate_guard(self):
        self._write_unavailable_gh_stub()
        if self.fixture.foreign_snapshot.exists():
            self.fixture.foreign_snapshot.unlink()

        result = self._run_bootstrap(FIXTURE.FOREIGN_UID)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.fixture.foreign_snapshot.is_file())
        self.assertFalse(self.fixture.gh_log.exists(), "active-task bootstrap must not enter candidate-only Issue guard")


if __name__ == "__main__":
    unittest.main(verbosity=2)
