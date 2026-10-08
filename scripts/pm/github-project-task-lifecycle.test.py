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
) -> tuple[object, dict[str, object], pathlib.Path, str, list[dict[str, str]]]:
    publication_module = MODULE.load_pr_projection_publication_module()
    projection_digest = publication_projection_digest or publication_module.digest({"projection": "current"})
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
        planner_config_sha256="sha256:" + "d" * 64,
        policy_digest=publication_module.digest({"policy": "test"}),
        projection_digest=projection_digest,
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

    def test_ordinary_verified_closeout_move_done_has_no_recovery_namespace(self) -> None:
        import subprocess,sys
        with tempfile.TemporaryDirectory() as directory:
            root=pathlib.Path(directory)
            record=mapping_record(status="committed",phase="execution")
            verified=subprocess.run([sys.executable,"-c","raise SystemExit(0)"],capture_output=True)
            self.assertEqual(verified.returncode,0)
            record.update(last_closed_at="2026-10-07T00:00:00Z",claim_verifications=[{
                "claim_type":"task_complete","status":"verified","verification_exit_code":verified.returncode,
                "verification_command":"python -c 'raise SystemExit(0)'"}])
            mapping_path=self.write_mapping(root,record);before=mapping_path.read_bytes()
            self.assertTrue(MODULE.has_verified_task_complete(record))
            task=MODULE.task_from_record(UID,record)
            durable={"body":MODULE.issue_body(task),"number":record["issue_number"],"title":record["title"],"url":record["issue_url"],"state":"OPEN","stateReason":None,"updatedAt":"2026-10-07T00:00:00Z"}
            effects=[];reads=[]
            def transport(command):
                reads.append(command)
                if command==["gh","issue","list","-R","eng-cc/oasis7","--state","all","--search",UID+" in:body","--json","number,url,title,state","--limit","5"]:
                    return json.dumps([{key:durable[key] for key in ("number","url","title","state")}])
                if command==["gh","issue","view",str(record["issue_number"]),"-R","eng-cc/oasis7","--json","body,number,title,url,state,stateReason,updatedAt"]:
                    return json.dumps(durable)
                if len(command)==8 and command[:6]==["gh","issue","edit",str(record["issue_number"]),"-R","eng-cc/oasis7"] and command[6]=="--body-file":
                    durable["body"]=pathlib.Path(command[7]).read_text();effects.append(command[:]);return durable["url"]
                raise AssertionError("unprovided ordinary move transport: "+repr(command))
            with mock.patch.object(MODULE,"run_text",side_effect=transport),mock.patch("builtins.print"):
                try:result=MODULE.command_move_task(args(root,"done"))
                except NameError:
                    self.assertEqual(mapping_path.read_bytes(),before)
                    self.assertEqual(effects,[])
                    raise
            self.assertEqual(result,0)
            final=json.loads(mapping_path.read_text())["tasks"][UID]
            self.assertEqual((final["status"],final["workflow_phase"]),("done","task_done"))
            self.assertEqual(final["claim_verifications"],record["claim_verifications"])
            self.assertEqual(len(effects),1)
            self.assertNotIn("oasis7-postmerge-completion",durable["body"])
            self.assertTrue(all(call[:2]==["gh","issue"] for call in reads))
            live=None
            with mock.patch.object(MODULE,"run_text",side_effect=transport):live=MODULE.github_issue_record("eng-cc/oasis7",UID)
            self.assertEqual((live["status"],live["workflow_phase"]),("done","task_done"))

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
            import subprocess
            root = pathlib.Path(directory)
            def git(*args: str) -> str:
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q');git('config', 'user.name', 'Fixture');git('config', 'user.email', 'fixture@example.invalid')
            git('commit', '-q', '--allow-empty', '-m', 'fixture base')
            base = git('rev-parse', 'HEAD')
            git('checkout', '-q', '-b', 'task/lifecycle-move-contract')
            git('commit', '-q', '--allow-empty', '-m', 'fixture head')
            head = git('rev-parse', 'HEAD')
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
                source_head_oid=head,
                source_scope_oid=base,
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

            identity = record_pr_identity(root)
            actor = {"login": "fixture-writer", "type": "User"}
            live_issue["bootstrap_epoch"] = 1
            raw_pr = record_pr_live_pr(draft=True)
            raw_pr["head"]["sha"] = head
            raw_pr.update(user=actor, created_at="2026-01-03T00:00:00Z", updated_at="2026-01-03T00:00:00Z")
            intent_comment = {
                "id": 2002, "user": actor, "author_association": "OWNER", "created_at": "2026-01-02T00:00:00Z", "updated_at": "2026-01-02T00:00:00Z",
                "body": publication_module.publication_comment(publication),
                "html_url": "https://github.com/eng-cc/oasis7/issues/2001#issuecomment-2002",
            }
            existing_comments = [intent_comment]
            written_comments: list[str] = []
            server_effects: list[str] = []
            loss = {'post': False, 'read': False}
            sync = MODULE.load_sync_module()
            project = {'id': 'PROJECT_fixture', 'number': 1, 'owner': {'login': 'eng-cc'}, 'viewerCanUpdate': True}
            project_values = {'Task UID': UID, 'Status': 'In Progress', 'PM Status': 'committed', 'Workflow Phase': 'execution', 'PR': ''}
            catalog = [{'id': 'FIELD_PHASE', 'name': 'Workflow Phase', 'type': 'ProjectV2SingleSelectField',
                        'options': [{'id': 'OPTION_VERIFY', 'name': 'verification'}]},
                       {'id': 'FIELD_PR', 'name': 'PR', 'type': 'ProjectV2Field'}]
            mapping = json.loads(mapping_path.read_text())
            mapping['project'] = {'id': project['id'], 'number': 1, 'owner': 'eng-cc', 'repo': 'eng-cc/oasis7'}
            MODULE.save_mapping(mapping_path, mapping)
            raw_issue = {'id': 'ISSUE_fixture', 'number': 2001, 'state': 'OPEN', 'user': actor,
                         'url': 'https://github.com/eng-cc/oasis7/issues/2001',
                         'body': MODULE.issue_body(MODULE.task_from_record(UID, record))}

            def comment_read(*_args: object) -> list[dict[str, object]]:
                if loss['post'] and not loss['read']:
                    loss['read'] = True
                    raise RuntimeError('simulated unavailable immediate readback after persisted comment')
                return list(existing_comments)

            def graphql_leaf(_token: str, query: str, variables: dict[str, object], **kwargs: object) -> dict[str, object]:
                page = {'hasNextPage': False, 'endCursor': None}
                if kwargs.get('operation') == 'project_sync_live_issue_memberships':
                    self.assertEqual(variables, {'owner': 'eng-cc', 'name': 'oasis7', 'number': 2001, 'after': None})
                    self.assertIn('projectItems(first: 100', query)
                    return {'repository': {'issue': {**{k: raw_issue[k] for k in ('id', 'number', 'url', 'state', 'body')},
                        'projectItems': {'nodes': [{'id': 'ITEM_ID', 'isArchived': False, 'project': project}], 'pageInfo': page}}}}
                if kwargs.get('operation') == 'project_sync_live_item_fields':
                    self.assertEqual(variables, {'item': 'ITEM_ID', 'after': None})
                    self.assertIn('fieldValues(first: 100', query)
                    return {'node': {'id': 'ITEM_ID', 'isArchived': False, 'project': project,
                        'fieldValues': {'nodes': [{'field': {'name': name}, 'name' if name in sync.SINGLE_SELECT_FIELDS else 'text': value}
                            for name, value in project_values.items()], 'pageInfo': page}}}
                raise AssertionError('unprovided GraphQL transport operation: '+repr(kwargs))

            def writer_transport(command: list[str]) -> str:
                if command == ['gh', 'api', 'user']:
                    return json.dumps(actor)
                if command == ['gh', 'api', 'repos/eng-cc/oasis7/collaborators/fixture-writer/permission']:
                    return json.dumps({'permission': 'write', 'user': actor, 'permissions': {'push': True}})
                if command[:2] == ['git', '-C'] and pathlib.Path(command[2]).resolve() == root.resolve() and command[3:] in (['rev-parse', '--verify', 'HEAD^{commit}'], ['rev-parse', '--git-common-dir']):
                    return subprocess.check_output(command, text=True).strip()
                if command == ['gh', 'api', 'repos/eng-cc/oasis7/issues/2001']:
                    return json.dumps(raw_issue)
                if command[:3] in (['gh', 'issue', 'edit'], ['gh', 'issue', 'comment']):
                    self.assertEqual(command[:6], ['gh', 'issue', command[2], '2001', '-R', 'eng-cc/oasis7'])
                    self.assertEqual(command[6], '--body-file');self.assertEqual(len(command), 8)
                    body = pathlib.Path(command[7]).read_text()
                    if command[2] == 'edit':
                        server_effects.append('issue');raw_issue['body'] = body
                        live_issue.update(MODULE.issue_task_fields(body))
                        if live_issue.get('pr_number'): live_issue['pr_number'] = int(live_issue['pr_number'])
                        return raw_issue['url']
                    ident = 2002 + len(written_comments) + 1
                    comment = {'id': ident, 'body': body, 'user': actor,
                        'html_url': raw_issue['url']+'#issuecomment-'+str(ident)}
                    written_comments.append(body);existing_comments.append(comment);server_effects.append('comment')
                    if '<!-- oasis7-ci-publication-binding/v1 -->' in body:
                        loss['post'] = True
                        raise RuntimeError('simulated lost response after reciprocal comment write')
                    return comment['html_url']
                if command[:2] == ['gh', 'api'] and len(command) == 3 and command[2].startswith('repos/eng-cc/oasis7/issues/comments/'):
                    ident = int(command[2].rsplit('/', 1)[1])
                    matches = [c for c in existing_comments if c.get('id') == ident]
                    self.assertEqual(len(matches), 1)
                    return json.dumps(matches[0])
                if command == ['gh', 'project', 'view', '1', '--owner', 'eng-cc', '--format', 'json']:
                    return json.dumps(project)
                if command == ['gh', 'project', 'field-list', '1', '--owner', 'eng-cc', '--format', 'json']:
                    return json.dumps({'fields': catalog, 'totalCount': len(catalog)})
                if command[:3] == ['gh', 'project', 'item-edit']:
                    self.assertEqual(command[:7], ['gh', 'project', 'item-edit', '--id', 'ITEM_ID', '--project-id', project['id']])
                    self.assertEqual(command[7], '--field-id');self.assertEqual(command[-2:], ['--format', 'json'])
                    field, flag, value = command[8:11]
                    self.assertEqual(len(command), 13)
                    if (field, flag, value) == ('FIELD_PHASE', '--single-select-option-id', 'OPTION_VERIFY'):
                        project_values['Workflow Phase'] = 'verification';server_effects.append('project:Workflow Phase')
                    elif (field, flag, value) == ('FIELD_PR', '--text', raw_pr['html_url']):
                        project_values['PR'] = value;server_effects.append('project:PR')
                    else:raise AssertionError('unprovided Project mutation: '+repr(command))
                    return json.dumps({'id': 'ITEM_ID'})
                raise AssertionError('unprovided raw C1 writer transport: '+repr(command))

            def sync_transport(command: list[str]) -> subprocess.CompletedProcess:
                return subprocess.CompletedProcess(command, 0, stdout=writer_transport(command), stderr='')

            with (
                mock.patch.object(MODULE, "github_issue_record", return_value=live_issue),
                mock.patch.object(MODULE, "authoritative_repository_identity", return_value=identity),
                mock.patch.object(MODULE, "github_pull_request", return_value=raw_pr),
                mock.patch.object(MODULE, "github_issue_comments", side_effect=comment_read),
                mock.patch.object(MODULE, "run_text", side_effect=writer_transport),
                mock.patch.object(MODULE, "synchronize_live_issue_traceability", return_value=[]),
                mock.patch.object(MODULE, "load_sync_module", return_value=sync),
                mock.patch.object(sync, "github_token", return_value="offline-noncredential"),
                mock.patch.object(sync, "graphql_request", side_effect=graphql_leaf),
                mock.patch.object(sync, "run_subprocess_with_retry", side_effect=sync_transport),
                mock.patch("urllib.request.urlopen", side_effect=AssertionError("unexpected real network")),
                mock.patch.object(MODULE, "load_pr_projection_publication_module", return_value=publication_module),
                mock.patch("builtins.print"),
            ):
                with self.assertRaisesRegex(MODULE._CommandExit, "publication-pending:.*complete readback unavailable"):
                    MODULE.command_record_pr(request)
                self.assertEqual("execution", json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]["workflow_phase"])
                self.assertTrue(loss['post']);self.assertTrue(loss['read'])
                journal_module = MODULE.load_pr_projection_journal_module()
                common_dir = (root / git('rev-parse', '--git-common-dir')).resolve()
                journal = journal_module.open_journal(common_dir, request.repo, identity['task_branch'], publication['publication_id'],
                    task_uid=UID, source_head_oid=head, scope_base_oid=base, projection_digest=publication['projection_digest'])
                with journal.locked(): uncertain = journal.read()
                self.assertEqual(uncertain['disposition'], 'COMMENT_READBACK_UNAVAILABLE')
                binding_action = [action for action in uncertain['actions'] if action['action_id'].endswith(':publication-binding')]
                self.assertEqual(len(binding_action), 1);self.assertEqual(binding_action[0]['state'], 'uncertain')
                effects_before_retry = list(server_effects)
                comments_before_retry = list(existing_comments)
                self.assertEqual(0, MODULE.command_record_pr(request))
                self.assertEqual(server_effects, effects_before_retry)
                self.assertEqual(existing_comments, comments_before_retry)
                with journal.locked(): final_journal = journal.read()
                self.assertEqual([action['state'] for action in final_journal['actions'] if action['action_id'].endswith(':publication-binding')], ['observed'])

            self.assertEqual(server_effects, ['issue', 'project:Workflow Phase', 'project:PR', 'comment', 'comment'])
            self.assertEqual(project_values['Workflow Phase'], 'verification')
            self.assertEqual(project_values['PR'], raw_pr['html_url'])
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
