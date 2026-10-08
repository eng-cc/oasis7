#!/usr/bin/env python3
"""Exact legacy comment binding is repeatable; migration is create-once."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    "legacy_comment_validator", ROOT / "scripts/pm/readiness_transport.py")
readiness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(readiness)
LEGACY = "<!-- oasis7-pm-claim-verification -->"
REPOSITORY = "fixture/repo"
ISSUE = 11


def server_comment(identifier, body):
    return {"id": identifier, "body": body, "user": {"login": "fixture"},
            "created_at": "2026-09-30T07:59:41Z", "updated_at": "2026-09-30T07:59:41Z",
            "html_url": f"https://github.com/{REPOSITORY}/issues/{ISSUE}#issuecomment-{identifier}"}


class LegacyCommentRepeatabilityTests(unittest.TestCase):
    def validate(self, current, discovered, marker):
        # Only the external GitHub readback is isolated. The real closed parser,
        # raw binding, complete discovery, exact ID and author/time checks run.
        with patch.object(readiness, "query", return_value=current) as query:
            result = readiness.validate_comment(REPOSITORY, ISSUE,
                readiness.comment_capture(current), discovered,
                current["body"].encode(), marker)
            query.assert_called_once_with(REPOSITORY,
                f"repos/{REPOSITORY}/issues/comments/{current['id']}")
            return result

    def test_distinct_older_legacy_ready_body_allows_exact_current_readback(self):
        prefix = LEGACY + "\nTask UID: task_11111111111111111111111111111111\nClaim Type: ready_for_merge\n"
        old = server_comment(901, prefix + "Verified At: 2026-09-30T15:58:40+08:00\n")
        current = server_comment(902, prefix + "Verified At: 2026-09-30T15:59:40+08:00\n")
        self.assertEqual(self.validate(current, [old, current], LEGACY), current)

    def test_duplicate_exact_legacy_body_at_another_id_is_rejected(self):
        body = LEGACY + "\nClaim Type: ready_for_merge\nVerified At: 2026-09-30T15:59:40+08:00\n"
        current = server_comment(902, body)
        with self.assertRaises(ValueError):
            self.validate(current, [server_comment(901, body), current], LEGACY)

    def test_distinct_duplicate_migration_markers_are_rejected(self):
        current = server_comment(902, readiness.MIGRATION + "\ncurrent transport")
        older = server_comment(901, readiness.MIGRATION + "\nolder transport")
        with self.assertRaises(ValueError):
            self.validate(current, [older, current], readiness.MIGRATION)


if __name__ == "__main__":
    unittest.main(verbosity=2)
