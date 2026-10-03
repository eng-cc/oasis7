#!/usr/bin/env python3
"""RED contract for gate-owned and terminal move-task transitions."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from argparse import Namespace
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/pm/github-project-task.py"
SPEC = importlib.util.spec_from_file_location("github_project_task_lifecycle", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


UID = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"


def mapping_record(*, status: str, phase: str) -> dict[str, object]:
    return {
        "task_uid": UID,
        "title": "Lifecycle move contract",
        "owner_role": "tpm",
        "module": "engineering",
        "status": status,
        "workflow_phase": phase,
        "priority": "P2",
        "worktree_hint": "/tmp/lifecycle-move-worktree",
        "issue_url": "https://github.com/eng-cc/oasis7/issues/2001",
        "issue_number": 2001,
        "project_item_id": "ITEM_ID",
    }


def args(root: pathlib.Path, target: str) -> Namespace:
    return Namespace(
        root=root,
        mapping=".pm/github-project-sync/tasks.json",
        repo="eng-cc/oasis7",
        project_owner="eng-cc",
        project_number=1,
        task_uid=UID,
        to_status=target,
        json=True,
    )


def record_pr_args(root: pathlib.Path, *, existing_ready_update: bool = False,
                   publication_binding_json: pathlib.Path | None = None) -> Namespace:
    request = Namespace(
        root=root,
        mapping=".pm/github-project-sync/tasks.json",
        repo="eng-cc/oasis7",
        project_owner="eng-cc",
        project_number=1,
        task_uid=UID,
        pr_url="https://github.com/eng-cc/oasis7/pull/2001",
        role="tpm",
        validation_command="record-pr lifecycle contract",
        draft_candidate=False,
        existing_ready_update=existing_ready_update,
        json=True,
    )
    if publication_binding_json is not None:
        request.publication_binding_json = str(publication_binding_json)
    return request


def record_pr_identity(root: pathlib.Path) -> dict[str, object]:
    return {
        "repository": "eng-cc/oasis7",
        "canonical_worktree": str(root.resolve()),
        "worktree_hint": str(root.resolve()),
        "task_branch": "task/lifecycle-move-contract",
        "default_branch": "main",
    }


def record_pr_live_issue(record: dict[str, object]) -> dict[str, object]:
    issue = {
        "task_uid": UID,
        "issue_number": 2001,
        "issue_url": "https://github.com/eng-cc/oasis7/issues/2001",
        "issue_state": "OPEN",
        "owner_role": record["owner_role"],
        "module": record["module"],
        "priority": record["priority"],
        "status": record["status"],
        "workflow_phase": record["workflow_phase"],
        "worktree_hint": record["worktree_hint"],
    }
    if record.get("pr_url"):
        issue["pr_url"] = record["pr_url"]
    if record.get("pr_number"):
        issue["pr_number"] = record["pr_number"]
    return issue


def record_pr_live_pr(**overrides: object) -> dict[str, object]:
    pr: dict[str, object] = {
        "number": 2001,
        "html_url": "https://github.com/eng-cc/oasis7/pull/2001",
        "state": "open",
        "merged_at": None,
        "draft": False,
        "head": {
            "ref": "task/lifecycle-move-contract",
            "sha": "a" * 40,
            "repo": {"full_name": "eng-cc/oasis7"},
        },
        "base": {
            "ref": "main",
            "repo": {"full_name": "eng-cc/oasis7"},
        },
    }
    pr.update(overrides)
    return pr


def existing_ready_publication_fixture(
    root: pathlib.Path,
    *,
    publication_repository: str = "eng-cc/oasis7",
    publication_head: str = "a" * 40,
    publication_ref: str = "task/lifecycle-move-contract",
    publication_scope: str = "b" * 40,
    publication_projection_digest: str | None = None,
    marker_head: str | None = None,
    marker_scope: str | None = None,
    marker_projection_digest: str | None = None,
    task_refs_body: str | None = None,
    full_projection: bool = False,
) -> tuple[object, dict[str, object], pathlib.Path, str, list[dict[str, str]]]:
    publication_module = MODULE.load_pr_projection_publication_module()
    planner_config_digest = "sha256:" + "d" * 64
    policy_digest = publication_module.digest({"policy": "test"})
    leaf = None
    if full_projection:
        planner = {
            "schema": "oasis7-required-plan-v1",
            "planner_config_sha256": planner_config_digest,
            "scope": "full",
            "selected_capabilities": [],
            "test_profile": "required",
            "declared_tests": ["required-baseline"],
        }
        leaf = {
            "schema": "oasis7-workflow-impact-projection/v2",
            "task_uid": UID,
            "source_head_oid": publication_head,
            "scope_base_oid": publication_scope,
            "changed_paths": [],
            "changed_paths_digest": publication_module.digest([]),
            "change_class": "unknown",
            "manual_roles": [],
            "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["required-baseline"],
            "consumed_contracts": [],
            "public_semantics": [],
            "affected_consumers": [],
            "closure_status": {"status": "incomplete", "reason": None, "evidence": []},
            "ci_scope": "full",
            "ci_capabilities": [],
            "ci_reasons": [],
            "review_roles": ["qa_engineer"],
            "ordered_role_ids": ["qa_engineer"],
            "review_scope": {},
            "review_escalated": False,
            "review_reasons": [],
            "planner_config_sha256": planner_config_digest,
            "planner_identity": planner,
            "planner_digest": publication_module.digest(planner),
            "verification_affected": True,
        }
        leaf["projection_digest"] = publication_module.digest(leaf)
        policy_digest = leaf["planner_digest"]
    projection_digest = (publication_projection_digest
                         or (leaf["projection_digest"] if leaf is not None
                             else publication_module.digest({"projection": "current"})))
    publication = publication_module.build_task_publication(
        repository=publication_repository,
        repository_id=7,
        task_uid=UID,
        bootstrap_epoch=1,
        source_repository_id=7,
        source_ref=publication_ref,
        target_ref="main",
        source_head_oid=publication_head,
        source_scope_oid=publication_scope,
        planner_authority_oid="c" * 40,
        planner_config_sha256=planner_config_digest,
        policy_digest=policy_digest,
        projection_digest=projection_digest,
        **({"workflow_impact_projection": leaf} if leaf is not None else {}),
    )
    binding = publication_module.build_publication_binding(
        publication, 2001, f"https://github.com/{publication_repository}/pull/2001",
    )
    binding_path = root / "publication-binding.json"
    binding_path.write_text(json.dumps(binding), encoding="utf-8")

    body_head = marker_head or publication_head
    body_scope = marker_scope or publication_scope
    body_projection = marker_projection_digest or projection_digest
    _contract, marker = publication_module.prepare(
        task_uid=UID,
        source_head_oid=body_head,
        scope_base_oid=body_scope,
        projection_digest=body_projection,
    )
    body_prefix = task_refs_body if task_refs_body is not None else f"Task: {UID}\nRefs #2001\n"
    pr_body = publication_module.replace_projection_marker(body_prefix, marker)
    comments = [{"body": publication_module.publication_comment(publication)}]
    return publication_module, publication, binding_path, pr_body, comments


class MoveTaskLifecycleContract(unittest.TestCase):
    def write_mapping(self, root: pathlib.Path, record: dict[str, object]) -> pathlib.Path:
        path = root / ".pm/github-project-sync/tasks.json"
        path.parent.mkdir(parents=True)
        MODULE.save_mapping(path, {"version": 1, "tasks": {UID: record}})
        return path

    @staticmethod
    def digest(path: pathlib.Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def invoke_existing_ready_writer(
        self,
        request: Namespace,
        record: dict[str, object],
        live_pr: dict[str, object],
        comments: list[dict[str, str]],
    ) -> tuple[str | None, dict[str, object]]:
        with (
            mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
            mock.patch.object(MODULE, "github_issue_comments", return_value=comments),
            mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(request.root)),
            mock.patch.object(MODULE, "github_pull_request", return_value=live_pr, create=True),
            mock.patch.object(MODULE, "run_text", return_value="a" * 40),
            mock.patch.object(MODULE, "load_pr_projection_publication_module",
                              return_value=MODULE.load_pr_projection_publication_module()),
            mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
            mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
            mock.patch.object(MODULE, "update_issue_body") as update_issue,
            mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
            mock.patch.object(MODULE, "verified_issue_comment", return_value="verified-comment-url") as verified_comment,
            mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            mock.patch("builtins.print"),
        ):
            writers = {
                "project": update_project,
                "issue": update_issue,
                "comment": comment,
                "verified_comment": verified_comment,
                "mapping": merge_mapping,
            }
            try:
                MODULE.command_record_pr(request)
            except MODULE._CommandExit as exc:
                error = str(exc)
            else:
                error = None
            counts = {name: writer.call_count for name, writer in writers.items()}
            details: dict[str, object] = {
                "counts": counts,
                "record": merge_mapping.call_args.args[2] if merge_mapping.called else None,
                "project_task": update_project.call_args.args[1] if update_project.called else None,
                "issue_task": update_issue.call_args.args[2] if update_issue.called else None,
            }
        return error, details

    def test_gate_owned_statuses_require_canonical_lifecycle_writer(self) -> None:
        for target in ("ready", "pr_watch"):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                mapping_path = self.write_mapping(root, mapping_record(status="committed", phase="execution"))
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(
                        MODULE._CommandExit,
                        rf"move-task: {target} is owned by the canonical",
                    ):
                        MODULE.command_move_task(args(root, target))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()

    def test_terminal_idempotent_done_preserves_fine_phase(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="done", phase=phase)
                record.update(
                    {
                        "last_closed_at": "2026-08-30T12:00:00+08:00",
                        "claim_verifications": [
                            {
                                "claim_type": "task_complete",
                                "status": "verified",
                                "allowed_to_claim": True,
                                "verification_exit_code": 0,
                            }
                        ],
                    }
                )
                mapping_path = self.write_mapping(root, record)
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(
                        MODULE,
                        "github_issue_record",
                        return_value={
                            "task_uid": UID,
                            "issue_number": record["issue_number"],
                            "status": record["status"],
                            "workflow_phase": record["workflow_phase"],
                        },
                    ),
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                    mock.patch("builtins.print"),
                ):
                    self.assertEqual(0, MODULE.command_move_task(args(root, "done")))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()
                persisted = json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]
                self.assertEqual(phase, persisted["workflow_phase"])

    def test_terminal_task_cannot_be_reclassified(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="done", phase=phase)
                record.update({"last_closed_at": "2026-08-30T12:00:00+08:00"})
                mapping_path = self.write_mapping(root, record)
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(MODULE._CommandExit, "terminal task cannot be reclassified"):
                        MODULE.command_move_task(args(root, "deferred"))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                update_project.assert_not_called()

    def test_record_pr_requires_ready_pre_pr_ready_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            mapping_path = self.write_mapping(root, mapping_record(status="committed", phase="execution"))
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
            ):
                with self.assertRaisesRegex(
                    MODULE._CommandExit,
                    "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready",
                ):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            merge_mapping.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_second_pr_without_mutating_task_truth(self) -> None:
        for existing_field in ("pr_url", "pull_request_url", "pr_number"):
          with self.subTest(existing_field=existing_field), tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record[existing_field] = 1999 if existing_field == "pr_number" else "https://github.com/eng-cc/oasis7/pull/1999"
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment") as comment,
                mock.patch.object(MODULE, "update_project_fields") as update_project,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "different PR .*already bound"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_foreign_repository_url_with_matching_number(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record["pr_number"] = 2001
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            request = record_pr_args(root)
            request.pr_url = "https://github.com/other/repo/pull/2001"
            with (
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment") as comment,
                mock.patch.object(MODULE, "update_project_fields") as update_project,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "PR URL repository mismatch"):
                    MODULE.command_record_pr(request)
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_rejects_replaced_canonical_branch_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value={
                    **record_pr_identity(root),
                    "task_branch": "task/lifecycle-move-contract-repaired",
                }),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True) as live_pr,
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "canonical task branch identity mismatch"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            live_pr.assert_not_called()
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_rejects_stale_live_task_truth_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            live_issue = record_pr_live_issue(record)
            live_issue["workflow_phase"] = "execution"
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True) as live_pr,
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live task Issue workflow_phase differs from cached task truth"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            live_pr.assert_not_called()

    def test_record_pr_reconciles_exact_publication_poststate_after_partial_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="committed", phase="execution")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            publication_module = MODULE.load_pr_projection_publication_module()
            publication = publication_module.build_task_publication(
                repository="eng-cc/oasis7",
                repository_id=7,
                task_uid=UID,
                bootstrap_epoch=1,
                source_repository_id=7,
                source_ref="task/lifecycle-move-contract",
                target_ref="main",
                source_head_oid="a" * 40,
                source_scope_oid="b" * 40,
                planner_authority_oid="c" * 40,
                planner_config_sha256="sha256:" + "d" * 64,
                policy_digest=publication_module.digest({"policy": "test"}),
                projection_digest=publication_module.digest({"projection": "test"}),
            )
            binding = publication_module.build_publication_binding(
                publication, 2001, "https://github.com/eng-cc/oasis7/pull/2001",
            )
            binding_path = root / "publication-binding.json"
            binding_path.write_text(json.dumps(binding), encoding="utf-8")
            request = record_pr_args(root)
            request.draft_candidate = True
            request.publication_binding_json = str(binding_path)
            live_issue = record_pr_live_issue(record)
            live_issue.update(
                status="committed",
                workflow_phase="verification",
                pr_url="https://github.com/eng-cc/oasis7/pull/2001",
                pr_number="2001",
            )
            identity = record_pr_identity(root)
            intent_comment = {
                "body": publication_module.publication_comment(publication),
                "html_url": "https://github.com/eng-cc/oasis7/issues/2001#issuecomment-2002",
            }
            existing_comments = [intent_comment]
            written_comments: list[str] = []

            def verified_comment(_repo: str, _issue: int, body: str) -> str:
                written_comments.append(body)
                comment_url = f"https://github.com/eng-cc/oasis7/issues/2001#issuecomment-{2002 + len(written_comments)}"
                existing_comments.append({"body": body, "html_url": comment_url})
                if "<!-- oasis7-ci-publication-binding/v1 -->" in body:
                    raise RuntimeError("simulated lost response after reciprocal comment write")
                return comment_url

            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=identity),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(draft=True)),
                mock.patch.object(MODULE, "github_issue_comments", side_effect=lambda *_: list(existing_comments)),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
                mock.patch.object(MODULE, "update_project_fields", return_value=7) as update_project,
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "verified_issue_comment", side_effect=verified_comment),
                mock.patch.object(MODULE, "load_pr_projection_publication_module", return_value=publication_module),
                mock.patch("builtins.print"),
            ):
                with self.assertRaisesRegex(RuntimeError, "lost response"):
                    MODULE.command_record_pr(request)
                self.assertEqual("execution", json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]["workflow_phase"])
                self.assertEqual(0, MODULE.command_record_pr(request))

            self.assertEqual(2, update_project.call_count)
            self.assertEqual(2, update_issue.call_count)
            self.assertEqual(2, len(written_comments), "evidence and reciprocal binding are restored")
            self.assertEqual(3, len(existing_comments), "recovery reuses both comments written before the lost response")
            persisted = json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]
            self.assertEqual("committed", persisted["status"])
            self.assertEqual("verification", persisted["workflow_phase"])
            self.assertEqual("https://github.com/eng-cc/oasis7/pull/2001", persisted["pr_url"])
            self.assertEqual(2001, persisted["pr_number"])
            self.assertEqual(2, len(persisted["evidence_comments"]))

    def test_record_pr_recovery_rejects_malformed_or_foreign_pr_number(self) -> None:
        for value in ("02001", 2002):
            with self.subTest(pr_number=value), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="committed", phase="execution")
                record.update(record_pr_identity(root))
                request = record_pr_args(root)
                request.draft_candidate = True
                live_issue = record_pr_live_issue(record)
                live_issue.update(
                    status="committed",
                    workflow_phase="verification",
                    pr_url="https://github.com/eng-cc/oasis7/pull/2001",
                    pr_number=value,
                )
                with mock.patch.object(MODULE, "github_issue_record", return_value=live_issue):
                    with self.assertRaisesRegex(
                        MODULE._CommandExit,
                        "neither cached truth nor the exact publication poststate",
                    ):
                        MODULE.validate_record_pr_live_identity(
                            request,
                            record,
                            2001,
                            allow_exact_publication_poststate=True,
                        )

    def test_record_pr_rejects_mixed_partial_publication_poststate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="committed", phase="execution")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            publication_module = MODULE.load_pr_projection_publication_module()
            publication = publication_module.build_task_publication(
                repository="eng-cc/oasis7", repository_id=7, task_uid=UID,
                bootstrap_epoch=1, source_repository_id=7,
                source_ref="task/lifecycle-move-contract", target_ref="main",
                source_head_oid="a" * 40, source_scope_oid="b" * 40,
                planner_authority_oid="c" * 40, planner_config_sha256="sha256:" + "d" * 64,
                policy_digest=publication_module.digest({"policy": "test"}),
                projection_digest=publication_module.digest({"projection": "test"}),
            )
            binding_path = root / "publication-binding.json"
            binding_path.write_text(json.dumps(publication_module.build_publication_binding(
                publication, 2001, "https://github.com/eng-cc/oasis7/pull/2001",
            )), encoding="utf-8")
            request = record_pr_args(root)
            request.draft_candidate = True
            request.publication_binding_json = str(binding_path)
            live_issue = record_pr_live_issue(record)
            live_issue.update(
                status="committed", workflow_phase="execution",
                pr_url="https://github.com/eng-cc/oasis7/pull/2001", pr_number=2001,
            )
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "github_issue_comments", return_value=[{
                    "body": publication_module.publication_comment(publication),
                    "html_url": "https://github.com/eng-cc/oasis7/issues/2001#issuecomment-2002",
                }]),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "update_project_fields") as update_project,
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "neither cached truth nor the exact publication poststate"):
                    MODULE.command_record_pr(request)
            self.assertEqual(before, self.digest(mapping_path))
            update_project.assert_not_called()
            update_issue.assert_not_called()

    def test_record_pr_rejects_closed_live_issue_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            live_issue = record_pr_live_issue(record)
            live_issue["issue_state"] = "CLOSED"
            failure = None
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True) as live_pr,
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]) as sync_traceability,
            ):
                try:
                    MODULE.command_record_pr(record_pr_args(root))
                except MODULE._CommandExit as exc:
                    failure = exc
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()
            sync_traceability.assert_not_called()
            live_pr.assert_not_called()
            self.assertIsNotNone(failure, "closed Issue must be rejected before record-pr writers")
            self.assertIn("live task Issue is not OPEN", str(failure))

    def test_record_pr_rejects_stale_live_pr_head_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(
                    head={
                        "ref": "task/lifecycle-move-contract",
                        "sha": "b" * 40,
                        "repo": {"full_name": "eng-cc/oasis7"},
                    },
                ), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live PR head does not match canonical task HEAD"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_rejects_foreign_live_pr_branch_before_any_task_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(
                    head={
                        "ref": "task/lifecycle-move-contract",
                        "sha": "a" * 40,
                        "repo": {"full_name": "outside/fork"},
                    },
                ), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "live PR head repository does not match task repository"):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            update_project.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_cannot_rewrite_terminal_task(self) -> None:
        for phase in ("post_merge_done", "closed_without_merge"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                mapping_path = self.write_mapping(root, mapping_record(status="done", phase=phase))
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                    mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                    mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
                ):
                    with self.assertRaisesRegex(
                        MODULE._CommandExit,
                        "record-pr: terminal task cannot be reclassified",
                    ):
                        MODULE.command_record_pr(record_pr_args(root))
                self.assertEqual(before, self.digest(mapping_path))
                update_issue.assert_not_called()
                comment.assert_not_called()
                merge_mapping.assert_not_called()
                update_project.assert_not_called()

    def test_record_pr_replay_after_pr_watch_is_rejected_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            mapping_path = self.write_mapping(root, mapping_record(status="pr_watch", phase="pr_watch"))
            before = self.digest(mapping_path)
            with (
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url") as comment,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0) as update_project,
            ):
                with self.assertRaisesRegex(
                    MODULE._CommandExit,
                    "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready",
                ):
                    MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(before, self.digest(mapping_path))
            update_issue.assert_not_called()
            comment.assert_not_called()
            merge_mapping.assert_not_called()
            update_project.assert_not_called()

    def test_record_pr_cli_accepts_explicit_existing_ready_update_mode(self) -> None:
        parsed = MODULE.build_parser().parse_args([
            "record-pr", "/tmp/existing-ready-update-fixture",
            "--task-uid", UID,
            "--pr-url", "https://github.com/eng-cc/oasis7/pull/2001",
            "--existing-ready-update",
        ])
        self.assertTrue(parsed.existing_ready_update)
        self.assertFalse(parsed.draft_candidate)

    def test_existing_ready_record_pr_preserves_pr_watch_status_and_phase(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="pr_watch", phase="pr_watch")
            record.update(record_pr_identity(root))
            record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
            record["pr_number"] = 2001
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            _publication_module, _publication, binding_path, body, comments = existing_ready_publication_fixture(root)
            request = record_pr_args(
                root, existing_ready_update=True, publication_binding_json=binding_path,
            )
            error, writer_details = self.invoke_existing_ready_writer(
                request, record, record_pr_live_pr(body=body), comments,
            )

            self.assertIsNone(error, f"valid exact C1 admission was rejected: {error}")
            writer_counts = writer_details["counts"]
            written_record = writer_details["record"]
            projected_task = writer_details["project_task"]
            issue_task = writer_details["issue_task"]
            self.assertIsInstance(written_record, dict)
            self.assertIsInstance(projected_task, dict)
            self.assertIsInstance(issue_task, dict)
            self.assertEqual("pr_watch", written_record["status"])
            self.assertEqual("pr_watch", written_record["workflow_phase"])
            self.assertEqual("pr_watch", projected_task["status"])
            self.assertEqual("pr_watch", projected_task["workflow_phase"])
            self.assertEqual("pr_watch", issue_task["status"])
            self.assertEqual("pr_watch", issue_task["workflow_phase"])
            self.assertGreater(writer_counts["project"], 0)
            self.assertGreater(writer_counts["issue"], 0)
            self.assertGreater(writer_counts["verified_comment"], 0)
            self.assertGreater(writer_counts["mapping"], 0)
            self.assertEqual(before, self.digest(mapping_path), "the mocked persistence seam must leave fixture bytes unchanged")

    def test_existing_ready_record_pr_preserves_ready_status_and_phase(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
            record["pr_number"] = 2001
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            _publication_module, _publication, binding_path, body, comments = existing_ready_publication_fixture(root)
            request = record_pr_args(
                root, existing_ready_update=True, publication_binding_json=binding_path,
            )
            error, writer_details = self.invoke_existing_ready_writer(
                request, record, record_pr_live_pr(body=body), comments,
            )

            self.assertIsNone(error, f"valid exact C1 admission was rejected: {error}")
            writer_counts = writer_details["counts"]
            written_record = writer_details["record"]
            projected_task = writer_details["project_task"]
            issue_task = writer_details["issue_task"]
            self.assertIsInstance(written_record, dict)
            self.assertIsInstance(projected_task, dict)
            self.assertIsInstance(issue_task, dict)
            self.assertEqual("ready", written_record["status"])
            self.assertEqual("pre_pr_ready", written_record["workflow_phase"])
            self.assertEqual("ready", projected_task["status"])
            self.assertEqual("pre_pr_ready", projected_task["workflow_phase"])
            self.assertEqual("ready", issue_task["status"])
            self.assertEqual("pre_pr_ready", issue_task["workflow_phase"])
            self.assertGreater(writer_counts["project"], 0)
            self.assertGreater(writer_counts["issue"], 0)
            self.assertGreater(writer_counts["verified_comment"], 0)
            self.assertGreater(writer_counts["mapping"], 0)
            self.assertEqual(before, self.digest(mapping_path), "the mocked persistence seam must leave fixture bytes unchanged")

    def test_existing_ready_record_pr_accepts_full_leaf_v2_task_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
            record["pr_number"] = 2001
            self.write_mapping(root, record)
            publication_module, publication, binding_path, body, comments = existing_ready_publication_fixture(
                root, full_projection=True,
            )
            self.assertEqual("oasis7-ci-publication/v2", publication["schema"])
            self.assertEqual(
                publication["projection_digest"],
                publication["workflow_impact_projection"]["projection_digest"],
            )
            self.assertEqual(publication, publication_module.parse_publication_comment(comments[0]["body"]))
            request = record_pr_args(
                root, existing_ready_update=True, publication_binding_json=binding_path,
            )
            error, writer_details = self.invoke_existing_ready_writer(
                request, record, record_pr_live_pr(body=body), comments,
            )

            self.assertIsNone(error, f"valid full-leaf v2 C1 admission was rejected: {error}")
            writer_counts = writer_details["counts"]
            self.assertGreater(writer_counts["project"], 0)
            self.assertGreater(writer_counts["issue"], 0)
            self.assertGreater(writer_counts["verified_comment"], 0)
            self.assertGreater(writer_counts["mapping"], 0)

    def test_existing_ready_record_pr_rejects_missing_publication_binding_before_writers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="pr_watch", phase="pr_watch")
            record.update(record_pr_identity(root))
            record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
            record["pr_number"] = 2001
            self.write_mapping(root, record)
            request = record_pr_args(root, existing_ready_update=True)

            error, writer_details = self.invoke_existing_ready_writer(
                request, record, record_pr_live_pr(body=f"Task: {UID}\nRefs #2001\n"), [],
            )

            writer_counts = writer_details["counts"]
            self.assertIsNotNone(
                error,
                f"existing-ready record-pr accepted missing C1 binding; downstream writer calls={writer_counts}",
            )
            self.assertEqual(
                {"project": 0, "issue": 0, "comment": 0, "verified_comment": 0, "mapping": 0},
                writer_counts,
                f"missing C1 binding must fail before every Project/Issue/comment/mapping writer: {writer_counts}",
            )

    def test_existing_ready_record_pr_rejects_stale_or_mismatched_c1_identity_before_writers(self) -> None:
        scenarios = (
            ("stale_intent_head", {"publication_head": "f" * 40}, {}, f"Task: {UID}\nRefs #2001\n", None),
            ("wrong_intent_ref", {"publication_ref": "task/lifecycle-move-other"}, {}, f"Task: {UID}\nRefs #2001\n", None),
            ("body_head_mismatch", {}, {"marker_head": "f" * 40}, f"Task: {UID}\nRefs #2001\n", None),
            ("body_scope_mismatch", {}, {"marker_scope": "e" * 40}, f"Task: {UID}\nRefs #2001\n", None),
            ("body_projection_mismatch", {}, {"marker_projection_digest": "sha256:" + "e" * 64}, f"Task: {UID}\nRefs #2001\n", None),
            ("wrong_live_task_uid", {}, {}, f"Task: task_{'b' * 32}\nRefs #2001\n", None),
            ("task_uid_suffix", {}, {}, f"Task: {UID}-suffix\nRefs #2001\n", None),
            ("wrong_live_issue_ref", {}, {}, f"Task: {UID}\nRefs #2002\n", None),
            ("refs_number_suffix", {}, {}, f"Task: {UID}\nRefs #20010\n", None),
            ("duplicate_live_task_lines", {}, {}, f"Task: {UID}\nTask: {UID}\nRefs #2001\n", None),
            ("conflicting_live_refs_lines", {}, {}, f"Task: {UID}\nRefs #2001\nRefs #2002\n", None),
            ("wrong_repository", {"publication_repository": "outside/oasis7"}, {}, f"Task: {UID}\nRefs #2001\n", None),
            ("missing_intent", {}, {}, f"Task: {UID}\nRefs #2001\n", "missing_intent"),
            ("duplicate_intent", {}, {}, f"Task: {UID}\nRefs #2001\n", "duplicate_intent"),
            ("malformed_intent", {}, {}, f"Task: {UID}\nRefs #2001\n", "malformed_intent"),
            ("malformed_binding", {}, {}, f"Task: {UID}\nRefs #2001\n", "malformed_binding"),
            ("intent_binding_mismatch", {}, {}, f"Task: {UID}\nRefs #2001\n", "intent_binding_mismatch"),
            ("binding_pr_mismatch", {}, {}, f"Task: {UID}\nRefs #2001\n", "binding_pr_mismatch"),
            ("ambiguous_reciprocal_binding", {}, {}, f"Task: {UID}\nRefs #2001\n", "ambiguous_binding"),
        )

        for name, publication_options, marker_options, body_prefix, admission_mutation in scenarios:
            with self.subTest(case=name), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="ready", phase="pre_pr_ready")
                record.update(record_pr_identity(root))
                record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
                record["pr_number"] = 2001
                self.write_mapping(root, record)
                _publication_module, publication, binding_path, body, comments = existing_ready_publication_fixture(
                    root, task_refs_body=body_prefix, **publication_options, **marker_options,
                )
                if admission_mutation == "missing_intent":
                    comments = []
                elif admission_mutation == "duplicate_intent":
                    comments = comments + [dict(comments[0])]
                elif admission_mutation == "malformed_intent":
                    comments = [{"body": "<!-- oasis7-ci-publication/v1 -->\n{"}]
                elif admission_mutation == "malformed_binding":
                    binding_path.write_text("{not-json", encoding="utf-8")
                elif admission_mutation == "intent_binding_mismatch":
                    binding = json.loads(binding_path.read_text(encoding="utf-8"))
                    binding["publication_id"] = "f" * 64
                    binding["binding_digest"] = MODULE.load_pr_projection_publication_module().digest(
                        {key: value for key, value in binding.items() if key != "binding_digest"},
                    )
                    binding_path.write_text(json.dumps(binding), encoding="utf-8")
                elif admission_mutation == "binding_pr_mismatch":
                    binding = _publication_module.build_publication_binding(
                        publication, 2002, "https://github.com/eng-cc/oasis7/pull/2002",
                    )
                    binding_path.write_text(json.dumps(binding), encoding="utf-8")
                elif admission_mutation == "ambiguous_binding":
                    binding = json.loads(binding_path.read_text(encoding="utf-8"))
                    comments = comments + [
                        {"body": _publication_module.publication_binding_comment(binding)},
                        {"body": _publication_module.publication_binding_comment(binding)},
                    ]
                request = record_pr_args(
                    root, existing_ready_update=True, publication_binding_json=binding_path,
                )
                # Keep canonical local and live identity intact. The candidate publication and PR body are
                # independently varied so the writer must bind both to the same current ready task.
                live_pr = record_pr_live_pr(body=body)
                error, writer_details = self.invoke_existing_ready_writer(
                    request, record, live_pr, comments,
                )
                writer_counts = writer_details["counts"]

                self.assertIsNotNone(
                    error,
                    f"C1 identity case {name} was accepted (publication={publication!r}, body={body_prefix!r}); "
                    f"downstream writer calls={writer_counts}",
                )
                self.assertEqual(
                    {"project": 0, "issue": 0, "comment": 0, "verified_comment": 0, "mapping": 0},
                    writer_counts,
                    f"C1 identity case {name} must fail before all downstream writers: {writer_counts}",
                )

    def test_existing_ready_record_pr_rejects_live_identity_drift_before_writers(self) -> None:
        cases = (
            ("draft", {"draft": True}, "live PR draft state does not match requested task transition"),
            ("head", {"head": {"ref": "task/other", "sha": "a" * 40,
                                "repo": {"full_name": "eng-cc/oasis7"}}},
             "live PR branch does not match canonical task branch"),
            ("repository", {"head": {"ref": "task/lifecycle-move-contract", "sha": "a" * 40,
                                      "repo": {"full_name": "outside/oasis7"}}},
             "live PR head repository does not match task repository"),
        )
        for name, pr_overrides, expected in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                record = mapping_record(status="pr_watch", phase="pr_watch")
                record.update(record_pr_identity(root))
                record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
                record["pr_number"] = 2001
                mapping_path = self.write_mapping(root, record)
                before = self.digest(mapping_path)
                with (
                    mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                    mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                    mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(**pr_overrides), create=True),
                    mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                    mock.patch.object(MODULE, "update_project_fields") as update_project,
                    mock.patch.object(MODULE, "update_issue_body") as update_issue,
                    mock.patch.object(MODULE, "issue_comment") as comment,
                    mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                ):
                    with self.assertRaisesRegex(MODULE._CommandExit, expected):
                        MODULE.command_record_pr(record_pr_args(root, existing_ready_update=True))
                self.assertEqual(before, self.digest(mapping_path))
                update_project.assert_not_called()
                update_issue.assert_not_called()
                comment.assert_not_called()
                merge_mapping.assert_not_called()

    def test_existing_ready_mode_cannot_be_combined_with_draft_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="pr_watch", phase="pr_watch")
            record.update(record_pr_identity(root))
            record["pr_url"] = "https://github.com/eng-cc/oasis7/pull/2001"
            record["pr_number"] = 2001
            mapping_path = self.write_mapping(root, record)
            before = self.digest(mapping_path)
            args = record_pr_args(root, existing_ready_update=True)
            args.draft_candidate = True
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_project_fields") as update_project,
                mock.patch.object(MODULE, "update_issue_body") as update_issue,
                mock.patch.object(MODULE, "issue_comment") as comment,
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "mutually exclusive"):
                    MODULE.command_record_pr(args)
            self.assertEqual(before, self.digest(mapping_path))
            update_project.assert_not_called()
            update_issue.assert_not_called()
            comment.assert_not_called()
            merge_mapping.assert_not_called()

    def test_record_pr_preserves_authoritative_ready_writer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            record = mapping_record(status="ready", phase="pre_pr_ready")
            record.update(record_pr_identity(root))
            mapping_path = self.write_mapping(root, record)
            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=record_pr_live_issue(record)),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=record_pr_identity(root)),
                mock.patch.object(MODULE, "github_pull_request", return_value=record_pr_live_pr(), create=True),
                mock.patch.object(MODULE, "run_text", return_value="a" * 40),
                mock.patch.object(MODULE, "update_issue_body"),
                mock.patch.object(MODULE, "issue_comment", return_value="comment-url"),
                mock.patch.object(MODULE, "merge_task_mapping") as merge_mapping,
                mock.patch.object(MODULE, "update_project_fields", return_value=0),
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
            ):
                result = MODULE.command_record_pr(record_pr_args(root))
            self.assertEqual(0, result)
            merge_mapping.assert_called_once()


if __name__ == "__main__":
    unittest.main(verbosity=2)
