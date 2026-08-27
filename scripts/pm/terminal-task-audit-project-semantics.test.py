#!/usr/bin/env python3
"""Regression matrix for bound Project-item terminal semantics."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
UID = "task_22222222222222222222222222222222"


class TerminalTaskAuditProjectSemantics(unittest.TestCase):
    def run_audit(
        self,
        *,
        item: dict,
        issue_project_items: list[dict] | None = None,
        project_repo: str = "fixture/repo",
    ) -> dict:
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            repo = scratch / "repo"
            repo.mkdir()
            subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
            origin = scratch / "origin.git"
            subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True)
            subprocess.run(["git", "-C", str(repo), "remote", "add", "origin", str(origin)], check=True)
            pm = repo / "scripts/pm"
            pm.mkdir(parents=True)
            for name in (
                "terminal-task-audit.py",
                "canonical-receipt-root.py",
                "github-project-workflow.py",
                "portable_file_lock.py",
            ):
                shutil.copy2(ROOT / "scripts/pm" / name, pm / name)
            mapping_path = repo / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            mapping_path.write_text(json.dumps({
                # This is the canonical cache shape: Project id is not
                # persisted, while owner/number/repo are authoritative.
                "project": {"owner": "fixture-owner", "number": 7, "repo": project_repo},
                "tasks": {UID: {
                    "task_uid": UID,
                    "status": "done",
                    "workflow_phase": "post_merge_done",
                    "repository": "fixture/repo",
                    "issue_number": 11,
                    "pr_number": 22,
                    "project_item_id": "ITEM1",
                    "canonical_worktree": str(scratch / "retired-task"),
                    "task_branch": "task/retired",
                }},
            }) + "\n", encoding="utf-8")
            subprocess.run([
                "python3", str(ROOT / "scripts/pm/canonical-receipt-root.py"),
                "--default-worktree", str(repo), "--task-uid", UID, "--create",
            ], check=True, text=True, capture_output=True)
            payload = {"data": {"nodes": [{
                "id": item.get("id", "ITEM1"),
                "project": {
                    "id": item.get("_project_id", "PVT_actual"),
                    "number": item.get("_project_number", 7),
                    "owner": {"login": item.get("_project_owner", "fixture-owner")},
                },
                "content": {
                    "body": item.get("_body", f"task_uid: {UID}"),
                    "number": item.get("_issue_number", 11),
                    "url": item.get("_issue_url", "https://github.com/fixture/repo/issues/11"),
                },
                "fieldValues": {
                    **({} if item.get("_omit_field_values_page_info") else {
                        "pageInfo": {"hasNextPage": item.get("_field_values_has_next_page", False)},
                    }),
                    "nodes": [
                        {"name": item.get("Status", "Done"), "field": {"name": "Status"}},
                        {"name": item.get("PM Status", "done"), "field": {"name": "PM Status"}},
                        {"name": item.get("Workflow Phase", "done"), "field": {"name": "Workflow Phase"}},
                    ],
                },
            }]}}
            bindir = scratch / "bin"
            bindir.mkdir()
            gh = bindir / "gh"
            gh.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, sys\n"
                "args = sys.argv[1:]\n"
                "if args[:2] == ['issue', 'view']:\n"
                " print(json.dumps({'state':'CLOSED','projectItems':json.loads(os.environ['ISSUE_PROJECT_ITEMS'])}))\n"
                "elif args[:2] == ['pr', 'view']:\n"
                " print(json.dumps({'state':'MERGED','mergedAt':'2026-08-27T00:00:00Z','headRefName':'task/retired'}))\n"
                "elif args[:2] == ['api', 'graphql']:\n"
                " print(os.environ['PROJECT_PAYLOAD'])\n"
                "else:\n"
                " print('{}')\n",
                encoding="utf-8",
            )
            gh.chmod(0o755)
            environment = {
                **os.environ,
                "PATH": f"{bindir}:{os.environ.get('PATH', '')}",
                "PROJECT_PAYLOAD": json.dumps(payload),
                "ISSUE_PROJECT_ITEMS": json.dumps(issue_project_items or []),
            }
            result = subprocess.run([
                "python3", str(pm / "terminal-task-audit.py"), "--repo-root", str(repo),
                "--task-uid", UID, "--json",
            ], env=environment, text=True, capture_output=True)
            self.assertIn(result.returncode, (0, 1), result.stderr)
            return json.loads(result.stdout)

    def test_bound_item_wins_over_unrelated_done_item(self) -> None:
        result = self.run_audit(
            item={"Status": "Done", "PM Status": "done", "Workflow Phase": "done"},
            issue_project_items=[{"id": "UNRELATED", "status": {"name": "Done"}}],
        )
        self.assertTrue(result["checks"]["project_terminal"], result)
        self.assertTrue(result["checks"]["project_item_bound"], result)
        self.assertTrue(result["checks"]["project_item_identity"], result)
        self.assertTrue(result["checks"]["project_field_values_complete"], result)

        result = self.run_audit(
            item={"Status": "In Progress", "PM Status": "pr_watch", "Workflow Phase": "pr_watch"},
            issue_project_items=[{"id": "UNRELATED", "status": {"name": "Done"}}],
        )
        self.assertFalse(result["checks"]["project_terminal"], result)

    def test_bound_item_rejects_field_identity_and_pagination_drift(self) -> None:
        cases = (
            ({"Status": "In Progress"}, None),
            ({"PM Status": "pr_watch"}, None),
            ({"Workflow Phase": "main_sync"}, None),
            ({"id": "ITEM2"}, "project_item_bound"),
            ({"_project_number": 8}, "project_item_identity"),
            ({"_issue_number": 12}, "project_item_identity"),
            ({"_issue_url": "https://github.com/other/repo/issues/11"}, "project_item_identity"),
            ({"_body": "task_uid: task_ffffffffffffffffffffffffffffffff"}, "project_item_identity"),
            ({"_project_owner": "foreign-owner"}, "project_item_identity"),
            ({"_project_owner": None}, "project_item_identity"),
            ({"_field_values_has_next_page": True}, "project_field_values_complete"),
            ({"_omit_field_values_page_info": True}, "project_field_values_complete"),
            ({"_field_values_has_next_page": "false"}, "project_field_values_complete"),
        )
        for item, failed_check in cases:
            with self.subTest(item=item):
                result = self.run_audit(item=item)
                self.assertFalse(result["checks"]["project_terminal"], result)
                if failed_check:
                    self.assertFalse(result["checks"][failed_check], result)

        result = self.run_audit(item={}, project_repo="other/repo")
        self.assertFalse(result["checks"]["project_item_identity"], result)


if __name__ == "__main__":
    unittest.main(verbosity=2)
