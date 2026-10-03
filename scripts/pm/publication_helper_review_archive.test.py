#!/usr/bin/env python3
"""Focused public-CLI RED for durable helper-review archive recovery."""
from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[1]
TASK_HELPER = ROOT / "github-project-task.py"
PUBLISHER = ROOT / "pr_projection_publish.py"


class PublicationHelperReviewArchiveCLITests(unittest.TestCase):
    def test_canonical_archive_command_is_uid_only(self) -> None:
        result = subprocess.run(
            [sys.executable, str(TASK_HELPER), "archive-publication-helper-review", "--help"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=10,
        )

        self.assertEqual(
            0,
            result.returncode,
            "canonical helper-review archive producer is unavailable:\n"
            + result.stdout
            + result.stderr,
        )
        self.assertIn("--task-uid", result.stdout)
        for forbidden_locator in ("--root", "--path", "--receipt-root", "--pr-number"):
            self.assertNotIn(forbidden_locator, result.stdout)

    def test_post_cleanup_recovery_uses_existing_public_resume_route(self) -> None:
        result = subprocess.run(
            [sys.executable, str(PUBLISHER), "--help"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=10,
        )

        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("--resume-action-id", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
