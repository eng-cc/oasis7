# Cross-platform test contract: setup must run on Windows PowerShell and Linux/macOS without weakening Git fallback coverage.

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
UID = "task_test"


def git_executable() -> str:
    executable = shutil.which("git")
    if executable is None and sys.platform == "win32":
        candidate = pathlib.Path("C:/Program Files/Git/cmd/git.exe")
        if candidate.is_file():
            executable = str(candidate)
    if executable is None:
        raise RuntimeError("bootstrap-task-snapshot test setup cannot find git")
    return executable


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
        return subprocess.run(
            [git_executable(), "-C", str(self.root), *args],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        ).stdout.strip()

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

    def run_helper(
        self, command: str, request: str = "request-1", env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess[str]:
        args = [
            sys.executable, str(SCRIPT), command, "--repo-root", str(self.root),
            "--task-uid", UID, "--request-identity", request,
            "--tasks-json", str(self.mapping), "--snapshot", str(self.snapshot),
        ]
        if command in {"create", "validate-or-create"}:
            args.extend(("--producer", "tpm"))
        return subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)

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

    def test_validate_or_create_creates_then_reuses_the_immutable_snapshot(self) -> None:
        created = self.run_helper("validate-or-create")
        self.assertEqual(created.returncode, 0, created.stderr)
        self.assertEqual("created", json.loads(created.stdout)["status"])
        original = self.snapshot.read_bytes()

        reused = self.run_helper("validate-or-create")
        self.assertEqual(reused.returncode, 0, reused.stderr)
        self.assertEqual("reused", json.loads(reused.stdout)["status"])
        self.assertEqual(original, self.snapshot.read_bytes())

    def test_validate_or_create_rejects_drift_while_legacy_create_remains_strict(self) -> None:
        created = self.run_helper("validate-or-create")
        self.assertEqual(created.returncode, 0, created.stderr)

        drifted = self.run_helper("validate-or-create", request="request-2")
        self.assertNotEqual(drifted.returncode, 0, drifted.stdout)
        self.assertIn("snapshot request drift", drifted.stderr)
        legacy_create = self.run_helper("create")
        self.assertNotEqual(legacy_create.returncode, 0)
        self.assertIn("refusing overwrite", legacy_create.stderr)

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

    def test_epoch_identity_allows_later_review_head_while_strict_validation_rejects_it(self) -> None:
        self.create()
        (self.root / "REVIEW-HEAD").write_text("implementation\n", encoding="utf-8")
        self.git("add", "REVIEW-HEAD")
        self.git("commit", "-m", "implementation head")

        epoch_identity = self.run_helper("validate-epoch-identity")
        self.assertEqual(0, epoch_identity.returncode, epoch_identity.stderr)
        self.assertEqual("valid_epoch_identity", json.loads(epoch_identity.stdout)["status"])
        self.assert_invalid("snapshot git drift")

    def test_epoch_identity_allows_status_transition_but_rejects_immutable_truth_drift(self) -> None:
        self.create()
        self.write_mapping(status="ready")

        epoch_identity = self.run_helper("validate-epoch-identity")
        self.assertEqual(0, epoch_identity.returncode, epoch_identity.stderr)
        self.assertEqual("valid_epoch_identity", json.loads(epoch_identity.stdout)["status"])
        self.assert_invalid("snapshot task drift")

        self.write_mapping(status="ready", acceptance=["changed acceptance"])
        drifted = self.run_helper("validate-epoch-identity")
        self.assertNotEqual(0, drifted.returncode, drifted.stdout)
        self.assertIn("snapshot request drift", drifted.stderr)

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

    @unittest.skipUnless(sys.platform == "win32", "Windows Git fallback contract")
    def test_validate_finds_windows_git_when_path_omits_git(self) -> None:
        self.create()
        environment = dict(__import__("os").environ)
        environment["PATH"] = str(self.root / "no-git-path")
        result = self.run_helper("validate", env=environment)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
