#!/usr/bin/env python3
"""Focused C1 identity tests for the review closeout journal adapter."""

from __future__ import annotations

import base64
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "pm"))
import review_closeout_publication as closeout  # noqa: E402


TASK_UID = "task_11111111111111111111111111111111"
ISSUE_PREFIX = f"<!-- oasis7-pm-task -->\ntask_uid: {TASK_UID}\n"


def issue_body(binding: dict[str, object] | None = None) -> str:
    if binding is None:
        return ISSUE_PREFIX
    encoded = base64.urlsafe_b64encode(
        json.dumps(binding, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).decode("ascii").rstrip("=")
    return ISSUE_PREFIX + f"- loop_binding_b64: `{encoded}`\n"


def valid_binding(epoch: int) -> dict[str, object]:
    return {
        "schema": "oasis7.loop-task/v1",
        "task_uid": TASK_UID,
        "change_id": "closeout-test",
        "loop": "code",
        "owner_role": "repository_health_engineer",
        "bootstrap_epoch": epoch,
        "manual_request_ref": "test-request",
        "request_key": "closeout-test-key",
        "write_scope": ["scripts/pm/**"],
        "out_of_scope": [],
        "input_contracts": [],
        "acceptance_refs": ["FS-R06"],
        "dependencies": [],
        "target_delivery": "single-pr",
        "policy_digest": "sha256:" + "a" * 64,
        "policy_commit": "b" * 40,
    }


class C1EpochTests(unittest.TestCase):
    def test_legacy_non_loop_publication_keeps_nullable_c1_epoch(self) -> None:
        # The v2 review epoch can be snapshot-derived; it is not a substitute
        # for the nullable C1 field produced from the Task record.
        self.assertIsNone(closeout.c1_bootstrap_epoch(ROOT, TASK_UID, {}, ISSUE_PREFIX))

    def test_non_loop_publication_uses_task_record_epoch(self) -> None:
        self.assertEqual(
            4,
            closeout.c1_bootstrap_epoch(ROOT, TASK_UID, {"bootstrap_epoch": 4}, ISSUE_PREFIX),
        )

    def test_loop_publication_uses_matching_live_issue_binding_epoch(self) -> None:
        binding = valid_binding(3)
        self.assertEqual(
            3,
            closeout.c1_bootstrap_epoch(
                ROOT, TASK_UID, {"loop_binding": binding}, issue_body(binding),
            ),
        )

    def test_issue_binding_drift_blocks_before_c1_resolution(self) -> None:
        with self.assertRaisesRegex(closeout.CloseoutPublicationError, "differs"):
            closeout.c1_bootstrap_epoch(
                ROOT, TASK_UID, {}, issue_body({"bootstrap_epoch": 2}),
            )

    def test_invalid_epoch_and_binding_uid_fail_closed(self) -> None:
        with self.assertRaisesRegex(closeout.CloseoutPublicationError, "epoch"):
            closeout.c1_bootstrap_epoch(ROOT, TASK_UID, {"bootstrap_epoch": True}, ISSUE_PREFIX)
        binding = valid_binding(3)
        binding["task_uid"] = "task_22222222222222222222222222222222"
        with self.assertRaisesRegex(closeout.CloseoutPublicationError, "another Task UID"):
            closeout.c1_bootstrap_epoch(
                ROOT, TASK_UID, {"loop_binding": binding}, issue_body(binding),
            )


class C1AuthorityLocatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="closeout-authority-test-")
        self.repo = Path(self.temp.name)
        self._git("init", "-q", "-b", "main")
        self._git("config", "user.email", "test@example.invalid")
        self._git("config", "user.name", "Fixture")
        config_path = self.repo / "scripts" / "ci-required-scope.v2.json"
        config_path.parent.mkdir(parents=True)
        config_path.write_text('{"schema":"test-config"}\n', encoding="utf-8")
        self._git("add", "scripts/ci-required-scope.v2.json")
        self._git("commit", "-qm", "common base")
        self.scope = self._git("rev-parse", "HEAD")
        self._git("switch", "-qc", "task/test", self.scope)
        (self.repo / "task.txt").write_text("task\n", encoding="utf-8")
        self._git("add", "task.txt")
        self._git("commit", "-qm", "task head")
        self.head = self._git("rev-parse", "HEAD")
        self._git("switch", "-q", "main")
        (self.repo / "main.txt").write_text("main update\n", encoding="utf-8")
        self._git("add", "main.txt")
        self._git("commit", "-qm", "default tip after fork")
        self.authority = self._git("rev-parse", "HEAD")
        self.config_digest = "sha256:" + hashlib.sha256(config_path.read_bytes()).hexdigest()
        self.publication_module = closeout.load_module(
            ROOT, "test_c1_locator_publication", "pr_projection_publication.py",
        )
        self.publication = self._publication(self.authority)
        self.expected = {key: value for key, value in self.publication.items()
                         if key not in {"publication_id", "schema", "projection_required"}}

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _git(self, *args: str) -> str:
        result = subprocess.run(["git", "-C", str(self.repo), *args],
                                text=True, capture_output=True, check=False)
        if result.returncode:
            raise AssertionError(result.stderr or result.stdout)
        return result.stdout.strip()

    def _publication(self, authority: str) -> dict[str, object]:
        return self.publication_module.build_task_publication(
            repository=closeout.REPOSITORY, repository_id=7, task_uid=TASK_UID,
            bootstrap_epoch=None, source_repository_id=7, source_ref="task/test",
            target_ref="main", source_head_oid=self.head, source_scope_oid=self.scope,
            planner_authority_oid=authority, planner_config_sha256=self.config_digest,
            policy_digest="sha256:" + "a" * 64,
            projection_digest="sha256:" + "b" * 64,
        )

    def _comment(self, publication: dict[str, object] | None = None,
                 *, updated_at: str = "2026-09-06T09:00:00Z") -> dict[str, object]:
        value = publication or self.publication
        return {
            "id": 10,
            "body": self.publication_module.publication_comment(value),
            "created_at": "2026-09-06T09:00:00Z", "updated_at": updated_at,
        }

    def test_unique_unedited_c1_supplies_only_authority_locator(self) -> None:
        independent = {key: value for key, value in self.publication.items()
                       if key not in {"planner_authority_oid", "publication_id", "schema", "projection_required"}}
        authority, comment = closeout.c1_authority_locator(
            [self._comment()], independent, self.publication_module,
        )
        self.assertEqual(self.authority, authority)
        self.assertEqual("2026-09-06T09:00:00Z", comment["created_at"])

    def test_duplicate_matching_c1_candidates_are_rejected_before_selection(self) -> None:
        other_target = self._git("rev-parse", self.scope)
        second = self._publication(other_target)
        independent = {key: value for key, value in self.publication.items()
                       if key not in {"planner_authority_oid", "publication_id", "schema", "projection_required"}}
        with self.assertRaisesRegex(closeout.CloseoutPublicationError, "ambiguous"):
            closeout.c1_authority_locator(
                [self._comment(), self._comment(second)], independent, self.publication_module,
            )

    def test_edited_c1_timestamps_are_rejected(self) -> None:
        independent = {key: value for key, value in self.publication.items()
                       if key not in {"planner_authority_oid", "publication_id", "schema", "projection_required"}}
        with self.assertRaisesRegex(closeout.CloseoutPublicationError, "edited"):
            closeout.c1_authority_locator(
                [self._comment(updated_at="2026-09-06T09:01:00Z")],
                independent, self.publication_module,
            )

    def test_diverged_target_is_accepted_only_with_independent_scope_and_config(self) -> None:
        with patch.object(closeout, "gh_json", return_value={
            "name": "main", "commit": {"sha": self.authority},
        }):
            self.assertEqual(
                self.authority,
                closeout.resolve_c1_planner_authority(
                    self.repo, [self._comment()], self.expected, self.scope,
                    "main", self.head, self.scope, self.config_digest,
                    self.publication_module,
                ),
            )
            with self.assertRaisesRegex(closeout.CloseoutPublicationError, "config differs"):
                closeout.resolve_c1_planner_authority(
                    self.repo, [self._comment()], self.expected, self.scope,
                    "main", self.head, self.scope, "sha256:" + "c" * 64,
                    self.publication_module,
                )
            with self.assertRaisesRegex(closeout.CloseoutPublicationError, "merge-base"):
                closeout.resolve_c1_planner_authority(
                    self.repo, [self._comment()], self.expected, self.scope,
                    "main", self.head, self.authority, self.config_digest,
                    self.publication_module,
                )

    def test_authority_must_be_in_live_canonical_default_ancestry(self) -> None:
        self._git("switch", "-q", "-c", "other-policy")
        (self.repo / "alternate.txt").write_text("unrelated\n", encoding="utf-8")
        self._git("add", "alternate.txt")
        self._git("commit", "-qm", "unrelated target")
        unrelated = self._git("rev-parse", "HEAD")
        with patch.object(closeout, "gh_json", return_value={
            "name": "main", "commit": {"sha": self.authority},
        }):
            bad_publication = self._publication(unrelated)
            with self.assertRaisesRegex(closeout.CloseoutPublicationError, "ambiguous"):
                # Two authority values with the same other twelve fields are
                # ambiguous even when one would happen to validate.
                closeout.resolve_c1_planner_authority(
                    self.repo, [self._comment(), self._comment(bad_publication)],
                    self.expected, self.scope, "main", self.head, self.scope,
                    self.config_digest, self.publication_module,
                )
            with self.assertRaisesRegex(closeout.CloseoutPublicationError, "independently verify"):
                closeout.resolve_c1_planner_authority(
                    self.repo, [self._comment(bad_publication)], self.expected,
                    self.scope, "main", self.head, self.scope, self.config_digest,
                    self.publication_module,
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
