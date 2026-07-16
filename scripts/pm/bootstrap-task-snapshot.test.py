#!/usr/bin/env python3

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
UID = "task_test"


class BootstrapTaskSnapshotTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name) / "repo"
        self.root.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        (self.root / "README").write_text("fixture\n", encoding="utf-8")
        self.git("add", "README")
        self.git("commit", "-m", "base")
        self.git("switch", "-q", "-c", "task/test")
        self.mapping = self.root / "tasks.json"
        self.snapshot = self.root / "snapshot.json"
        self.write_mapping()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(["git", "-C", str(self.root), *args], check=True, text=True, stdout=subprocess.PIPE).stdout.strip()

    def write_mapping(self, **changes: object) -> None:
        task = {
            "task_uid": UID,
            "issue_number": 2300,
            "issue_url": "https://example.invalid/issues/2300",
            "project_item_id": "PVTI_test",
            "status": "committed",
            "owner_role": "repository_health_engineer",
            "repository": "eng-cc/oasis7",
            "canonical_worktree": str(self.root.resolve()),
            "task_branch": "task/test",
            "default_branch": "main",
            "acceptance": ["snapshot binds canonical truth"],
        }
        task.update(changes)
        self.mapping.write_text(json.dumps({"version": 1, "project": {"owner": "eng-cc", "number": 1}, "tasks": {UID: task}}), encoding="utf-8")

    def run_helper(self, command: str, request: str = "request-1") -> subprocess.CompletedProcess[str]:
        args = [
            sys.executable, str(SCRIPT), command, "--repo-root", str(self.root),
            "--task-uid", UID, "--request-identity", request,
            "--tasks-json", str(self.mapping), "--snapshot", str(self.snapshot),
        ]
        if command == "create":
            args.extend(("--producer", "tpm"))
        return subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def create(self) -> None:
        result = self.run_helper("create")
        self.assertEqual(result.returncode, 0, result.stderr)

    def assert_invalid(self, needle: str, request: str = "request-1") -> None:
        result = self.run_helper("validate", request)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(needle, result.stderr)

    def test_valid_snapshot_and_overwrite_rejected(self) -> None:
        self.create()
        result = self.run_helper("validate")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "valid")
        overwrite = self.run_helper("create")
        self.assertNotEqual(overwrite.returncode, 0)
        self.assertIn("refusing overwrite", overwrite.stderr)

    def test_missing_required_truth_rejected(self) -> None:
        self.write_mapping(issue_url="")
        result = self.run_helper("create")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing required bootstrap truth", result.stderr)

    def test_wrong_worktree_rejected(self) -> None:
        self.write_mapping(canonical_worktree=str(self.root.parent / "other"))
        result = self.run_helper("create")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("worktree drift", result.stderr)

    def test_head_drift_rejected(self) -> None:
        self.create()
        (self.root / "HEAD-DRIFT").write_text("drift\n", encoding="utf-8")
        self.git("add", "HEAD-DRIFT")
        self.git("commit", "-m", "head drift")
        self.assert_invalid("snapshot git drift")

    def test_request_drift_rejected(self) -> None:
        self.create()
        self.assert_invalid("snapshot request drift", request="request-2")

    def test_acceptance_drift_rejected(self) -> None:
        self.create()
        self.write_mapping(acceptance=["changed acceptance"])
        self.assert_invalid("snapshot task drift")

    def test_digest_tampering_rejected(self) -> None:
        self.create()
        payload = json.loads(self.snapshot.read_text(encoding="utf-8"))
        payload["producer"] = "tampered"
        self.snapshot.write_text(json.dumps(payload), encoding="utf-8")
        self.assert_invalid("snapshot digest mismatch")


if __name__ == "__main__":
    unittest.main()
