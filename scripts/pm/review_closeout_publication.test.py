#!/usr/bin/env python3
"""Focused C1 identity tests for the review closeout journal adapter."""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import json
import os
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


class LockedC1SnapshotTests(unittest.TestCase):
    """Exercise the real closeout writer against a changing full Issue snapshot."""

    def _git(self, root: Path, *args: str) -> str:
        result = subprocess.run(["git", "-C", str(root), *args],
                                text=True, capture_output=True, check=False)
        if result.returncode:
            raise AssertionError(result.stderr or result.stdout)
        return result.stdout.strip()

    def _run_with_locked_snapshot(self, mutation: str) -> dict[str, object]:
        fixture_path = ROOT / "scripts" / "pm" / "pr_projection_publication.test.py"
        spec = importlib.util.spec_from_file_location("closeout_c1_writer_fixture", fixture_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        fixture = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixture)
        resolver_fixture = object.__new__(fixture.PublicationMatrixTests)
        publication_value, c1, complete_read, pr_binding = resolver_fixture.c1_resolution_inputs(8801)
        initial_resolution = fixture.publication_module.resolve_task_publication(
            complete_read,
            {key: publication_value[key]
             for key in fixture.publication_module._TASK_PUBLICATION_FIELDS},
            live_task_author={"login": "task-author", "type": "User"},
            pr_binding=pr_binding,
        )
        self.assertEqual("passed", initial_resolution.get("status"))

        complete = closeout.load_module(
            ROOT, "closeout_writer_test_complete", "review_closeout_complete.py",
        )
        journal_module = closeout.load_module(
            ROOT, "closeout_writer_test_journal", "pr_projection_journal.py",
        )
        with tempfile.TemporaryDirectory(prefix="closeout-locked-c1-test-") as temp:
            base = Path(temp)
            repo = base / "repo"
            repo.mkdir()
            self._git(repo, "init", "-q", "-b", publication_value["source_ref"])
            self._git(repo, "config", "user.name", "Closeout Fixture")
            self._git(repo, "config", "user.email", "closeout@example.invalid")
            (repo / "seed.txt").write_text("seed\n", encoding="utf-8")
            self._git(repo, "add", "seed.txt")
            self._git(repo, "commit", "-qm", "closeout fixture")
            head = self._git(repo, "rev-parse", "HEAD^{commit}")
            (repo / "scripts").mkdir()
            (repo / "scripts" / "pm").symlink_to(ROOT / "scripts" / "pm", target_is_directory=True)
            common_dir = Path(self._git(repo, "rev-parse", "--git-common-dir"))
            if not common_dir.is_absolute():
                common_dir = (repo / common_dir).resolve()
            journal = journal_module.open_journal(
                common_dir, publication_value["repository"], publication_value["source_ref"],
                publication_value["publication_id"],
                task_uid=publication_value["task_uid"],
                source_head_oid=publication_value["source_head_oid"],
                scope_base_oid=publication_value["source_scope_oid"],
                projection_digest=publication_value["projection_digest"],
            )

            state_path = base / "fake-gh-state.json"
            state_path.write_text(json.dumps({
                "issue_url": f"https://api.github.com/repos/{publication_value['repository']}/issues/123",
                "comments": [c1], "reads": 0, "posts": 0,
            }), encoding="utf-8")
            bin_dir = base / "bin"
            bin_dir.mkdir()
            gh = bin_dir / "gh"
            gh.write_text(r'''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
path=Path(os.environ["CLOSEOUT_GH_STATE"])
state=json.loads(path.read_text())
args=sys.argv[1:]
def save(): path.write_text(json.dumps(state, sort_keys=True))
def emit(value): print(json.dumps(value)); save(); raise SystemExit(0)
if args[:2] == ["api", "user"]:
    emit({"login":"repo-admin"})
if args[:2] == ["api", "repos/eng-cc/oasis7/collaborators/repo-admin/permission"]:
    emit({"permission":"admin"})
if len(args) > 1 and args[0] == "api" and args[1].startswith("repos/eng-cc/oasis7/issues/123/comments") and "--method" not in args:
    state["reads"] += 1
    if state["reads"] == 1:
        if os.environ["CLOSEOUT_C1_MUTATION"] == "edited":
            state["comments"][0]["updated_at"] = "2026-10-02T00:01:00Z"
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "missing":
            state["comments"] = []
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "body-edited":
            state["comments"][0]["body"] += "\n"
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "id-replaced":
            state["comments"][0]["id"] += 1
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "created-at-edited":
            state["comments"][0]["created_at"] = "2026-09-30T11:59:00Z"
            state["comments"][0]["updated_at"] = state["comments"][0]["created_at"]
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "duplicate":
            duplicate = dict(state["comments"][0])
            duplicate["id"] += 1
            state["comments"].append(duplicate)
        elif os.environ["CLOSEOUT_C1_MUTATION"] == "unreadable":
            print("synthetic incomplete Task Issue comments read", file=sys.stderr)
            raise SystemExit(1)
        state["locked_snapshot"] = list(state["comments"])
    emit([state["comments"]] if "--slurp" in args else state["comments"])
if len(args) > 1 and args[0] == "api" and args[1] == "repos/eng-cc/oasis7/issues/123/comments" and "--method" in args:
    body=next(arg.split("=",1)[1] for arg in args if arg.startswith("body="))
    comment={"id":9900,"body":body,"created_at":"2026-10-02T00:02:00Z",
             "updated_at":"2026-10-02T00:02:00Z","user":{"login":"repo-admin","type":"User"},
             "html_url":"https://github.com/eng-cc/oasis7/issues/123#issuecomment-9900",
             "issue_url":state["issue_url"]}
    state["comments"].append(comment); state["posts"] += 1
    emit(comment)
if len(args) > 1 and args[0] == "api" and args[1] == "repos/eng-cc/oasis7/issues/comments/9900":
    emit(state["comments"][-1])
raise SystemExit("unexpected fake gh invocation: " + " ".join(args))
''', encoding="utf-8")
            gh.chmod(0o755)
            body = (
                f"## Review Packet\n- Task UID: {publication_value['task_uid']}\n"
                f"- Source Head: {publication_value['source_head_oid']}\n"
                "- Review Plan: review-plan.json\n- Pre-PR Local Role Review: passed\n"
            )
            context = {
                "journal": journal, "issue_number": 123, "root": repo, "head": head,
                "publication": publication_value, "task_uid": publication_value["task_uid"],
                "c1_validation": closeout._validated_c1_context(
                    [c1],
                    {key: publication_value[key]
                     for key in fixture.publication_module._TASK_PUBLICATION_FIELDS},
                    {"login": "task-author", "type": "User"},
                    pr_binding, initial_resolution,
                ),
            }
            failure: Exception | None = None
            result: dict[str, object] | None = None
            with patch.dict(os.environ, {
                "PATH": str(bin_dir) + os.pathsep + os.environ.get("PATH", ""),
                "CLOSEOUT_GH_STATE": str(state_path),
                "CLOSEOUT_C1_MUTATION": mutation,
            }):
                try:
                    result = closeout.publish_comment(
                        context, action_id="locked-c1-review-packet",
                        kind="publish_review_packet",
                        expected={"task_uid": publication_value["task_uid"]}, body=body,
                        find_matches=lambda comments: [
                            comment for comment in comments if comment.get("body") == body
                        ],
                        verify=lambda comment: complete.verify_comment(
                            comment, 123, body,
                            {"Task UID": publication_value["task_uid"],
                             "Source Head": publication_value["source_head_oid"],
                             "Review Plan": "review-plan.json"},
                        ),
                    )
                except Exception as exc:
                    failure = exc

            final_state = json.loads(state_path.read_text(encoding="utf-8"))
            with journal.locked():
                actions = journal.read()["actions"]
            return {
                "mutation": mutation,
                "initial_c1_status": initial_resolution["status"],
                "locked_snapshot": final_state.get("locked_snapshot"),
                "result_status": result.get("status") if result else None,
                "failure_type": type(failure).__name__ if failure else None,
                "failure": str(failure) if failure else None,
                "post_count": final_state["posts"],
                "observed_actions": [x for x in actions if x.get("state") == "observed"],
            }

    def _assert_locked_c1_change_blocks_before_append(self, mutation: str) -> None:
        evidence = self._run_with_locked_snapshot(mutation)
        self.assertEqual(
            0, evidence["post_count"],
            f"locked {mutation} C1 snapshot must block every closeout append: {evidence}",
        )
        self.assertEqual(
            [], evidence["observed_actions"],
            f"locked {mutation} C1 snapshot must not become journal success: {evidence}",
        )
        self.assertEqual(
            "CloseoutPublicationError", evidence["failure_type"],
            f"locked {mutation} C1 snapshot must fail closed: {evidence}",
        )

    def test_edited_c1_on_locked_complete_read_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("edited")

    def test_missing_c1_on_locked_complete_read_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("missing")

    def test_duplicate_c1_on_locked_complete_read_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("duplicate")

    def test_unreadable_locked_complete_read_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("unreadable")

    def test_c1_body_change_with_same_semantics_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("body-edited")

    def test_replaced_c1_comment_id_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("id-replaced")

    def test_changed_unedited_c1_created_at_blocks_packet_append(self) -> None:
        self._assert_locked_c1_change_blocks_before_append("created-at-edited")


if __name__ == "__main__":
    unittest.main(verbosity=2)
