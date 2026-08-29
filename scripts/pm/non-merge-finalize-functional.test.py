#!/usr/bin/env python3
"""Functional fixture coverage for the non-merge terminal writer.

The fake ``gh`` process is deliberately small and deterministic.  It records
every call and models only the bound Issue, PR, Project item, comments, and
Project field mutations used by the helper.  No network or GitHub state is
reached by this test.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/pm/non-merge-finalize.py"
PROJECT_TASK = ROOT / "scripts/pm/github-project-task.py"
UID = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
REPO = "fixture/repo"
ISSUE = 11
ISSUE_URL = f"https://github.com/{REPO}/issues/{ISSUE}"
PR_URL = f"https://github.com/{REPO}/pulls/22"


FAKE_GH = r'''#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

def path(name):
    return Path(os.environ[name])

def read(name, default):
    p = path(name)
    if not p.exists():
        return default
    return json.loads(p.read_text())

def write(name, value):
    path(name).write_text(json.dumps(value, sort_keys=True) + "\n")

args = sys.argv[1:]
path("GH_CALLS").open("a").write(json.dumps(args) + "\n")
uid = os.environ["GH_UID"]
repo = os.environ["GH_REPO"]
issue = int(os.environ["GH_ISSUE"])
issue_url = f"https://github.com/{repo}/issues/{issue}"

if args[:2] == ["issue", "view"]:
    state = read("GH_ISSUE_STATE", {"state": "OPEN"})["state"]
    body = path("GH_ISSUE_BODY").read_text()
    if (os.environ.get("GH_FAIL_AFTER_MAPPING_COMMIT_ON_ISSUE_VIEW") == "1"
            and os.environ.get("GH_FAILURE_MARKER")
            and not path("GH_FAILURE_MARKER").exists()):
        mapping = json.loads(path("GH_MAPPING").read_text())
        record = (mapping.get("tasks") or {}).get(uid) or {}
        if record.get("workflow_phase") == "closed_without_merge":
            path("GH_FAILURE_MARKER").write_text("failed once")
            print("injected crash after mapping commit", file=sys.stderr)
            raise SystemExit(93)
    if (state == "CLOSED" and os.environ.get("GH_REQUIRE_ISSUE_DONE") == "1"
            and "- status: `done`" not in body):
        print("closed issue body was not updated to done", file=sys.stderr)
        raise SystemExit(91)
    print(json.dumps({"state": state, "body": body,
                      "number": issue, "url": issue_url}))
elif args[:2] == ["issue", "edit"]:
    body = Path(args[args.index("--body-file") + 1]).read_text()
    path("GH_ISSUE_BODY").write_text(body)
    print("edited")
elif args[:2] == ["issue", "comment"]:
    body = Path(args[args.index("--body-file") + 1]).read_text()
    comments = read("GH_COMMENTS", [])
    number = len(comments) + 1
    comments.append({"id": number,
                     "html_url": f"{issue_url}#issuecomment-{number}",
                     "body": body})
    write("GH_COMMENTS", comments)
    print(f"{issue_url}#issuecomment-{number}")
elif args[:2] == ["issue", "close"]:
    reason = args[args.index("--reason") + 1]
    if (os.environ.get("GH_REQUIRE_ISSUE_DONE") == "1"
            and "- status: `done`" not in path("GH_ISSUE_BODY").read_text()):
        print("issue body must be done before close", file=sys.stderr)
        raise SystemExit(92)
    closes = read("GH_CLOSES", [])
    closes.append(reason)
    write("GH_CLOSES", closes)
    write("GH_ISSUE_STATE", {"state": "CLOSED"})
    print("closed")
elif args[:2] == ["pr", "view"]:
    number = int(os.environ.get("GH_PR_NUMBER", "22"))
    state = os.environ.get("GH_PR_STATE", "CLOSED")
    merged_at = os.environ.get("GH_PR_MERGED_AT", "") or None
    print(json.dumps({"number": number, "url": os.environ.get("GH_PR_URL", ""),
                      "state": state, "mergedAt": merged_at,
                      "headRefOid": "head-oid", "headRefName": "task/fixture"}))
elif args[:2] == ["project", "view"]:
    print(json.dumps({"id": "P1", "number": 1, "title": "fixture"}))
elif args[:2] == ["project", "field-list"]:
    options = {
        "Status": [("Done", "STATUS_DONE"), ("In Progress", "STATUS_PROGRESS")],
        "PM Status": [("done", "PM_DONE"), ("committed", "PM_COMMITTED")],
        "Workflow Phase": [("done", "PHASE_DONE"), ("execution", "PHASE_EXECUTION")],
    }
    fields = []
    for name, values in options.items():
        fields.append({"id": "FIELD_" + name.replace(" ", "_"), "name": name,
                       "type": "ProjectV2SingleSelectField",
                       "options": [{"id": option_id, "name": value}
                                   for value, option_id in values]})
    print(json.dumps({"fields": fields}))
elif args[:2] == ["project", "item-edit"]:
    option_id = args[args.index("--single-select-option-id") + 1]
    values = read("GH_PROJECT_FIELDS", {"Status": "In Progress",
                                         "PM Status": "committed",
                                         "Workflow Phase": "execution"})
    values.update({"STATUS_DONE": {"Status": "Done"},
                   "PM_DONE": {"PM Status": "done"},
                   "PHASE_DONE": {"Workflow Phase": "done"}}.get(option_id, {}))
    write("GH_PROJECT_FIELDS", values)
    if (os.environ.get("GH_FAIL_AFTER_PROJECT_UPDATE_ON_ITEM_EDIT") == "1"
            and os.environ.get("GH_PROJECT_FAILURE_MARKER")
            and os.environ.get("GH_PROJECT_UPDATE_EDIT_COUNT")
            and not path("GH_PROJECT_FAILURE_MARKER").exists()):
        count_path = path("GH_PROJECT_UPDATE_EDIT_COUNT")
        count = int(count_path.read_text()) if count_path.exists() else 0
        count_path.write_text(str(count + 1))
        if count + 1 < 3:
            print("{}")
            raise SystemExit(0)
        path("GH_PROJECT_FAILURE_MARKER").write_text("failed after Project publication")
        print("injected crash after Project publication", file=sys.stderr)
        raise SystemExit(93)
    if (os.environ.get("GH_FAIL_AFTER_PROJECT_UPDATE_ON_ITEM_EDIT") == "1"
            and os.environ.get("GH_PROJECT_FAILURE_MARKER")
            and path("GH_PROJECT_FAILURE_MARKER").exists()):
        print("injected crash after Project publication", file=sys.stderr)
        raise SystemExit(93)
    print("{}")
elif args[:2] == ["issue", "list"] and os.environ.get("GH_REFRESH_TASK") == "1":
    print(json.dumps([{"number": issue, "state": read("GH_ISSUE_STATE", {"state": "OPEN"})["state"],
                      "title": "[PM] fixture", "url": issue_url}]))
elif args and args[0] == "api" and len(args) > 1 and args[1].startswith("repos/"):
    if os.environ.get("GH_MUTATE_MAPPING_ON_COMMENT_READBACK") == "1":
        mapping_path = path("GH_MAPPING")
        mapping = json.loads(mapping_path.read_text())
        mapping["tasks"][uid].update({
            "pr_number": 23,
            "pr_url": f"https://github.com/{repo}/pulls/23",
        })
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n")
    if os.environ.get("GH_MUTATE_PROJECT_MAPPING_ON_COMMENT_READBACK") == "1":
        mapping_path = path("GH_MAPPING")
        mapping = json.loads(mapping_path.read_text())
        mapping["project"].update({"owner": "other", "number": 2, "id": "P2"})
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n")
    print(json.dumps([read("GH_COMMENTS", [])]))
elif args[:2] == ["api", "graphql"]:
    if os.environ.get("GH_PROJECT_ITEM_MISSING") == "1":
        print(json.dumps({"data": {"nodes": []}}))
        raise SystemExit(0)
    if os.environ.get("GH_MUTATE_PROJECT_MAPPING_ON_PROJECT_READBACK") == "1":
        mapping_path = path("GH_MAPPING")
        mapping = json.loads(mapping_path.read_text())
        if mapping.get("project", {}).get("id") != "P2":
            mapping["project"].update({"owner": "other", "number": 2, "id": "P2"})
            mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n")
    values = read("GH_PROJECT_FIELDS", {"Status": "In Progress",
                                         "PM Status": "committed",
                                         "Workflow Phase": "execution"})
    if os.environ.get("GH_PROJECT_DRIFT") == "1":
        content = {"body": "task_uid: task_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
                   "number": 99, "url": f"https://github.com/{repo}/issues/99"}
    else:
        content = {"body": f"task_uid: {uid}\n", "number": issue, "url": issue_url}
    nodes = [{"name": values.get(name, ""), "field": {"name": name}}
             for name in ("Status", "PM Status", "Workflow Phase")]
    node = {"id": "ITEM1", "project": {"id": "P1", "number": 1,
            "owner": {"login": "fixture"}}, "content": content,
            "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": nodes}}
    print(json.dumps({"data": {"nodes": [node]}}))
else:
    print("unexpected gh invocation: " + " ".join(args), file=sys.stderr)
    raise SystemExit(90)
'''


class NonMergeFinalizeFunctionalTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "repo"
        self.root.mkdir()
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.root)], check=True)
        (self.root / ".pm/github-project-sync").mkdir(parents=True)
        self.bin = Path(self.tmp.name) / "bin"
        self.bin.mkdir()
        gh = self.bin / "gh"
        gh.write_text(textwrap.dedent(FAKE_GH))
        gh.chmod(0o755)
        self.env = os.environ.copy()
        self.env["PATH"] = f"{self.bin}:{self.env['PATH']}"
        self.calls_path = Path(self.tmp.name) / "calls.jsonl"
        self.comments = Path(self.tmp.name) / "comments.json"
        self.closes = Path(self.tmp.name) / "closes.json"
        self.issue_state = Path(self.tmp.name) / "issue-state.json"
        self.issue_body = Path(self.tmp.name) / "issue-body.md"
        self.project_fields = Path(self.tmp.name) / "project-fields.json"
        self.failure_marker = Path(self.tmp.name) / "failure-marker"
        self.write_state(self.comments, [])
        self.write_state(self.closes, [])
        self.write_state(self.issue_state, {"state": "OPEN"})
        self.issue_body.write_text(
            f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\n- status: `committed`\n"
        )
        self.write_state(self.project_fields, {
            "Status": "In Progress", "PM Status": "committed", "Workflow Phase": "execution"
        })
        self.env.update({
            "GH_CALLS": str(self.calls_path), "GH_COMMENTS": str(self.comments),
            "GH_CLOSES": str(self.closes), "GH_ISSUE_STATE": str(self.issue_state),
            "GH_ISSUE_BODY": str(self.issue_body),
            "GH_PROJECT_FIELDS": str(self.project_fields), "GH_UID": UID,
            "GH_REPO": REPO, "GH_ISSUE": str(ISSUE), "GH_PR_URL": PR_URL,
        })

    def tearDown(self) -> None:
        self.tmp.cleanup()

    @staticmethod
    def write_state(path: Path, value: object) -> None:
        path.write_text(json.dumps(value, sort_keys=True) + "\n")

    def mapping(self, *, status: str = "committed", phase: str = "execution",
                pr: bool = False, merge: bool = False,
                extra: dict | None = None) -> Path:
        record = {
            "task_uid": UID, "repository": REPO, "issue_number": ISSUE,
            "issue_url": ISSUE_URL, "project_item_id": "ITEM1", "status": status,
            "workflow_phase": phase, "owner_role": "repository_health_engineer",
            "module": "engineering", "priority": "P2",
        }
        if pr:
            record.update({"pr_number": 22, "pr_url": PR_URL})
        if merge:
            record["merge_receipt"] = {"state": "MERGED"}
        if extra:
            record.update(extra)
        path = self.root / ".pm/github-project-sync/tasks.json"
        path.write_text(json.dumps({"version": 1, "project": {
            "owner": "fixture", "number": 1, "id": "P1"
        }, "tasks": {UID: record}}, sort_keys=True) + "\n")
        return path

    def reset_runtime(self) -> None:
        """Reset mutable fake-remote and receipt state for another reason."""
        for path, value in (
            (self.comments, []), (self.closes, []),
            (self.issue_state, {"state": "OPEN"}),
            (self.project_fields, {"Status": "In Progress", "PM Status": "committed",
                                   "Workflow Phase": "execution"}),
        ):
            self.write_state(path, value)
        self.issue_body.write_text(
            f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\n- status: `committed`\n"
        )
        receipt_root = self.root / ".git/oasis7-workflow-receipts" / UID
        if receipt_root.exists():
            shutil.rmtree(receipt_root)

    def evidence(self, name: str = "evidence.md") -> Path:
        path = Path(self.tmp.name) / name
        path.write_text("owner decision: terminal non-merge closure\n")
        return path

    def invoke(self, reason: str, evidence: Path | None = None) -> subprocess.CompletedProcess[str]:
        evidence = evidence or self.evidence()
        return subprocess.run([
            sys.executable, str(HELPER), "--repo-root", str(self.root),
            "--task-uid", UID, "--reason", reason,
            "--evidence-file", str(evidence), "--json",
        ], cwd=ROOT, env=self.env, text=True, capture_output=True)

    def read_json(self, path: Path) -> object:
        return json.loads(path.read_text())

    def calls(self) -> list[list[str]]:
        if not self.calls_path_exists():
            return []
        return [json.loads(line) for line in self.calls_path.read_text().splitlines()]

    def calls_path_exists(self) -> bool:
        return self.calls_path.exists()

    def test_invalid_reason_fails_before_any_gh_call(self) -> None:
        self.mapping()
        result = self.invoke("invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"reason|invalid choice|allowed")
        self.assertEqual(self.calls(), [])

    def test_superseded_without_bound_pr_fails(self) -> None:
        self.mapping()
        result = self.invoke("superseded")
        self.assertNotEqual(result.returncode, 0)
        record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
        self.assertEqual(record["workflow_phase"], "execution")
        self.assertEqual(self.read_json(self.comments), [])
        self.assertEqual(self.read_json(self.closes), [])

    def test_closed_unmerged_pr_closes_with_not_planned_and_retry_is_idempotent(self) -> None:
        self.mapping(pr=True)
        first = self.invoke("superseded")
        self.assertEqual(first.returncode, 0, first.stderr)
        payload = json.loads(first.stdout[first.stdout.find("{"):])
        self.assertEqual(payload["status"], "finalized")
        mapping = self.read_json(self.root / ".pm/github-project-sync/tasks.json")
        record = mapping["tasks"][UID]
        self.assertEqual(record["status"], "done")
        self.assertEqual(record["workflow_phase"], "closed_without_merge")
        self.assertNotIn("merge_receipt", record)
        self.assertNotIn("merge_receipt_sha256", record)
        receipt = self.read_json(Path(payload["receipt"]))
        self.assertEqual(receipt["receipt_type"], "oasis7_closed_without_merge")
        self.assertNotIn("merge_receipt", receipt)
        self.assertEqual(receipt["pr_number"], 22)
        self.assertEqual(self.read_json(self.project_fields), {
            "Status": "Done", "PM Status": "done", "Workflow Phase": "done"
        })
        comments = self.read_json(self.comments)
        self.assertEqual(len(comments), 1)
        self.assertIn("Evidence Phase: closed_without_merge", comments[0]["body"])
        self.assertIn("Evidence SHA256:", comments[0]["body"])
        self.assertIn("owner decision: terminal non-merge closure", comments[0]["body"])
        for field in ("Action", "Validation Command", "Expected Result", "Actual Result"):
            self.assertRegex(comments[0]["body"], rf"(?m)^{re.escape(field)}:\s+\S")
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_closed_draft_verification_phase_is_eligible(self) -> None:
        self.mapping(pr=True, phase="verification")
        result = self.invoke("superseded")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
        self.assertEqual(record["status"], "done")
        self.assertEqual(record["workflow_phase"], "closed_without_merge")

        retry = self.invoke("superseded")
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertEqual(json.loads(retry.stdout[retry.stdout.find("{"):])["status"], "already_finalized")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_non_pr_completed_retry_recovers_after_mapping_commit_crash(self) -> None:
        self.mapping(extra={
            "status": "done", "workflow_phase": "task_done",
            "last_closed_at": "2026-08-29T00:00:00+08:00",
            "claim_verifications": [{
                "claim_type": "task_complete", "status": "verified",
                "allowed_to_claim": True, "verification_exit_code": 0,
            }],
            "completion_mode": "non_pr_task",
            "non_pr_completion_evidence": "persisted fixture truth",
        })
        self.env.update({
            "GH_MAPPING": str(self.root / ".pm/github-project-sync/tasks.json"),
            "GH_FAILURE_MARKER": str(self.failure_marker),
            "GH_FAIL_AFTER_MAPPING_COMMIT_ON_ISSUE_VIEW": "1",
        })

        crashed = self.invoke("non_pr_completed")
        self.assertNotEqual(crashed.returncode, 0)
        record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
        self.assertEqual(record["status"], "done")
        self.assertEqual(record["workflow_phase"], "closed_without_merge")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), [])

        retry = self.invoke("non_pr_completed")
        self.assertEqual(retry.returncode, 0, retry.stderr)
        payload = json.loads(retry.stdout[retry.stdout.find("{"):])
        self.assertEqual(payload["status"], "already_finalized")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["completed"])

    def test_retry_recovers_after_project_done_then_selected_refresh_without_manual_edits(self) -> None:
        canonical_worktree = Path(self.tmp.name) / "canonical-task-worktree"
        subprocess.run(["git", "-C", str(self.root), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.name", "Test"], check=True)
        (self.root / "tracked").write_text("base\n")
        subprocess.run(["git", "-C", str(self.root), "add", "tracked"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "base"], check=True)
        subprocess.run([
            "git", "-C", str(self.root), "worktree", "add", "-qb",
            "task/fixture", str(canonical_worktree),
        ], check=True, stdout=subprocess.DEVNULL)
        mapping_path = self.mapping(pr=True, extra={
            "canonical_worktree": str(canonical_worktree),
            "task_branch": "task/fixture",
            "default_branch": "main",
        })
        self.issue_body.write_text(
            f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\nTask metadata:\n"
            "- owner_role: `repository_health_engineer`\n"
            "- module: `engineering`\n- status: `committed`\n"
            "- priority: `P2`\n"
            f"- worktree_hint: `{canonical_worktree}`\n"
        )
        self.env.update({
            "GH_MAPPING": str(mapping_path),
            "GH_PROJECT_FAILURE_MARKER": str(Path(self.tmp.name) / "project-failure-marker"),
            "GH_PROJECT_UPDATE_EDIT_COUNT": str(Path(self.tmp.name) / "project-update-count"),
            "GH_FAIL_AFTER_PROJECT_UPDATE_ON_ITEM_EDIT": "1",
            "GH_REFRESH_TASK": "1",
        })

        crashed = self.invoke("duplicate")
        edit_count = Path(self.tmp.name) / "project-update-count"
        self.assertNotEqual(
            crashed.returncode,
            0,
            crashed.stdout + crashed.stderr + repr(self.calls()) + repr(edit_count.read_text() if edit_count.exists() else None),
        )
        after_crash = self.read_json(mapping_path)["tasks"][UID]
        self.assertIn("closed_without_merge_intent", after_crash)
        self.assertEqual(after_crash["workflow_phase"], "execution")
        receipt = self.root / ".git/oasis7-workflow-receipts" / UID / "closed-without-merge-receipt.json"
        self.assertTrue(receipt.exists())
        self.assertEqual(self.read_json(self.project_fields), {
            "Status": "Done", "PM Status": "done", "Workflow Phase": "done"
        })

        refreshed = subprocess.run([
            sys.executable, str(PROJECT_TASK), "refresh-task", str(self.root),
            "--repo", REPO, "--project-owner", "fixture", "--project-number", "1",
            "--task-uid", UID, "--json",
        ], cwd=ROOT, env=self.env, text=True, capture_output=True)
        self.assertEqual(refreshed.returncode, 0, refreshed.stderr)
        after_refresh = self.read_json(mapping_path)["tasks"][UID]
        self.assertEqual(after_refresh["workflow_phase"], "execution")

        self.env.pop("GH_FAIL_AFTER_PROJECT_UPDATE_ON_ITEM_EDIT")
        retry = self.invoke("duplicate")
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertEqual(
            self.read_json(mapping_path)["tasks"][UID]["workflow_phase"],
            "closed_without_merge",
        )
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_authority_drift_fails_before_issue_project_or_comment_terminal_effects(self) -> None:
        mapping_path = self.mapping(pr=True)
        self.env.update({
            "GH_MAPPING": str(mapping_path),
            "GH_MUTATE_PROJECT_MAPPING_ON_PROJECT_READBACK": "1",
        })
        result = self.invoke("duplicate")
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"drift|identity|authority|project")

        mapping = self.read_json(mapping_path)
        self.assertEqual(mapping["project"], {"owner": "other", "number": 2, "id": "P2"})
        record = mapping["tasks"][UID]
        self.assertEqual(record["workflow_phase"], "execution")
        self.assertEqual(record["status"], "committed")
        self.assertNotIn("closed_without_merge_receipt", record)
        self.assertIn("- status: `committed`", self.issue_body.read_text())
        self.assertEqual(self.read_json(self.project_fields), {
            "Status": "In Progress", "PM Status": "committed", "Workflow Phase": "execution"
        })
        self.assertEqual(self.read_json(self.comments), [])
        self.assertEqual(self.read_json(self.closes), [])

    def test_retry_rejects_receipt_embedded_evidence_drift_with_matching_digest(self) -> None:
        mapping_path = self.mapping(pr=True)
        self.env["GH_MAPPING"] = str(mapping_path)
        first = self.invoke("duplicate")
        self.assertEqual(first.returncode, 0, first.stderr)
        payload = json.loads(first.stdout[first.stdout.find("{"):])
        receipt_path = Path(payload["receipt"])
        receipt = self.read_json(receipt_path)
        original_digest = receipt["evidence_sha256"]
        receipt["evidence"] = {"text": "tampered receipt payload"}
        self.assertEqual(receipt["evidence_sha256"], original_digest)
        receipt_path.write_text(json.dumps(receipt, sort_keys=True) + "\n")

        retry = self.invoke("duplicate")
        self.assertNotEqual(retry.returncode, 0)
        self.assertRegex(retry.stderr.lower(), r"receipt|evidence|authority|disagree")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_retry_with_renamed_same_evidence_is_idempotent(self) -> None:
        mapping_path = self.mapping(pr=True)
        self.env["GH_MAPPING"] = str(mapping_path)
        original = self.evidence("original-evidence.md")
        first = self.invoke("duplicate", original)
        self.assertEqual(first.returncode, 0, first.stderr)
        renamed = Path(self.tmp.name) / "renamed-evidence.md"
        renamed.write_bytes(original.read_bytes())

        retry = self.invoke("duplicate", renamed)
        self.assertEqual(retry.returncode, 0, retry.stderr)
        payload = json.loads(retry.stdout[retry.stdout.find("{"):])
        self.assertEqual(payload["status"], "already_finalized")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_project_identity_drift_fails_closed_on_retry(self) -> None:
        self.mapping(pr=True)
        first = self.invoke("duplicate")
        self.assertEqual(first.returncode, 0, first.stderr)
        before = self.read_json(self.root / ".pm/github-project-sync/tasks.json")
        self.env["GH_PROJECT_DRIFT"] = "1"
        retry = self.invoke("duplicate")
        self.assertNotEqual(retry.returncode, 0)
        self.assertRegex(retry.stderr.lower(), r"identity|mismatch")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])
        self.assertEqual(self.read_json(self.root / ".pm/github-project-sync/tasks.json"), before)

    def test_missing_project_binding_fails_before_intent_or_receipt_and_repaired_retry_succeeds(self) -> None:
        mapping_path = self.mapping(pr=True)
        mapping = self.read_json(mapping_path)
        mapping["tasks"][UID]["project_item_id"] = "MISSING_ITEM"
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n")
        self.env.update({
            "GH_MAPPING": str(mapping_path),
            "GH_PROJECT_ITEM_MISSING": "1",
        })

        failed = self.invoke("duplicate")
        self.assertNotEqual(failed.returncode, 0)
        failed_mapping = self.read_json(mapping_path)
        self.assertNotIn("closed_without_merge_intent", failed_mapping["tasks"][UID])
        receipt = self.root / ".git/oasis7-workflow-receipts" / UID / "closed-without-merge-receipt.json"
        self.assertFalse(receipt.exists())
        self.assertIn("- status: `committed`", self.issue_body.read_text())
        self.assertEqual(self.read_json(self.comments), [])
        self.assertEqual(self.read_json(self.closes), [])

        repaired = self.read_json(mapping_path)
        repaired["tasks"][UID]["project_item_id"] = "ITEM1"
        mapping_path.write_text(json.dumps(repaired, sort_keys=True) + "\n")
        self.env.pop("GH_PROJECT_ITEM_MISSING")
        retry = self.invoke("duplicate")
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_retry_rejects_receipt_pr_identity_drift(self) -> None:
        self.mapping(pr=True)
        first = self.invoke("duplicate")
        self.assertEqual(first.returncode, 0, first.stderr)

        mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        mapping = self.read_json(mapping_path)
        mapping["tasks"][UID].update({
            "pr_number": 23,
            "pr_url": f"https://github.com/{REPO}/pulls/23",
        })
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n")
        self.env["GH_PR_NUMBER"] = "23"
        self.env["GH_PR_URL"] = f"https://github.com/{REPO}/pulls/23"

        retry = self.invoke("duplicate")
        self.assertNotEqual(retry.returncode, 0)
        self.assertRegex(retry.stderr.lower(), r"receipt|pr|identity|authority")
        self.assertEqual(len(self.read_json(self.comments)), 1)
        self.assertEqual(self.read_json(self.closes), ["not planned"])

    def test_pr_bound_reasons_reject_stale_merge_authority(self) -> None:
        for reason in ("superseded", "duplicate"):
            with self.subTest(reason=reason):
                self.reset_runtime()
                self.mapping(
                    pr=True,
                    merge=True,
                    extra={"merge_receipt_sha256": "stale-merge-receipt"},
                )
                result = self.invoke(reason)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr.lower(), r"merge|receipt|authority")
                record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
                self.assertEqual(record["workflow_phase"], "execution")
                self.assertEqual(record["status"], "committed")
                self.assertEqual(record["merge_receipt"], {"state": "MERGED"})
                self.assertEqual(record["merge_receipt_sha256"], "stale-merge-receipt")
                self.assertEqual(self.read_json(self.comments), [])
                self.assertEqual(self.read_json(self.closes), [])

    def test_duplicate_issue_status_fields_fail_closed(self) -> None:
        self.mapping(pr=True)
        self.issue_body.write_text(
            f"<!-- oasis7-pm-task -->\ntask_uid: {UID}\n"
            "- status: `committed`\n- status: `pr_watch`\n"
        )
        result = self.invoke("superseded")
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"status|canonical|exactly one")
        record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
        self.assertEqual(record["workflow_phase"], "execution")
        self.assertEqual(record["status"], "committed")
        self.assertEqual(self.read_json(self.comments), [])
        self.assertEqual(self.read_json(self.closes), [])

    def test_mismatched_preexisting_comment_is_not_reconciled(self) -> None:
        self.mapping(pr=True)
        evidence = self.evidence()
        evidence_digest = hashlib.sha256(evidence.read_bytes()).hexdigest()
        receipt_root = Path(subprocess.check_output([
            sys.executable, str(ROOT / "scripts/pm/canonical-receipt-root.py"),
            "--default-worktree", str(self.root), "--task-uid", UID, "--create",
        ], cwd=ROOT, env=self.env, text=True).strip())
        (receipt_root / "closed-without-merge-receipt.json").write_text(json.dumps({
            "receipt_type": "oasis7_closed_without_merge",
            "schema_version": 1,
            "issuer": "non-merge-finalize",
            "task_uid": UID,
            "repository": REPO,
            "issue_number": ISSUE,
            "project_item_id": "ITEM1",
            "reason": "superseded",
            "evidence_sha256": evidence_digest,
            "pr_number": 22,
            "pr_url": PR_URL,
            "pr_state": "CLOSED",
            "mergedAt": None,
        }) + "\n")
        operation_id = hashlib.sha256(
            f"{UID}:closed_without_merge:evidence_comment".encode()
        ).hexdigest()
        self.write_state(self.comments, [{
            "id": 1,
            "html_url": f"{ISSUE_URL}#issuecomment-1",
            "body": (
                "<!-- oasis7-pm-evidence -->\n"
                f"Operation-ID: {operation_id}\nTask UID: {UID}\n"
                "Evidence Phase: closed_without_merge\nRole: tpm\n"
                "Reason: duplicate\nEvidence SHA256: " + "0" * 64 + "\n"
                "Evidence File: stale.md\n"
            ),
        }])
        (receipt_root / "non-merge-finalizer-ledger.json").write_text(json.dumps({
            "schema": "oasis7_non_merge_finalizer_ledger_v1",
            "task_uid": UID,
            "operations": {
                "evidence_comment": {
                    "operation_id": operation_id,
                    "effect": "evidence_comment",
                    "action": True,
                },
            },
        }) + "\n")

        result = self.invoke("superseded", evidence)
        self.assertEqual(result.returncode, 0, result.stderr)
        comments = self.read_json(self.comments)
        self.assertEqual(len(comments), 2)
        self.assertIn("Reason: superseded", comments[1]["body"])
        self.assertIn(f"Evidence SHA256: {evidence_digest}", comments[1]["body"])
        record = self.read_json(self.root / ".pm/github-project-sync/tasks.json")["tasks"][UID]
        self.assertEqual(record["evidence_comments"], [f"{ISSUE_URL}#issuecomment-2"])

    def test_mapping_pr_authority_drift_during_remote_readback_fails_closed(self) -> None:
        mapping_path = self.mapping(pr=True)
        self.env.update({
            "GH_MAPPING": str(mapping_path),
            "GH_MUTATE_MAPPING_ON_COMMENT_READBACK": "1",
        })
        result = self.invoke("duplicate")
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"drift|identity|authority|pr")

        record = self.read_json(mapping_path)["tasks"][UID]
        self.assertEqual(record["pr_number"], 23)
        self.assertEqual(record["workflow_phase"], "execution")
        self.assertEqual(record["status"], "committed")
        self.assertNotIn("closed_without_merge_receipt", record)
        self.assertEqual(self.read_json(self.closes), [])

    def test_mapping_project_authority_drift_during_remote_readback_fails_closed(self) -> None:
        mapping_path = self.mapping(pr=True)
        self.env.update({
            "GH_MAPPING": str(mapping_path),
            "GH_MUTATE_PROJECT_MAPPING_ON_COMMENT_READBACK": "1",
        })
        result = self.invoke("duplicate")
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"drift|identity|authority|project")

        mapping = self.read_json(mapping_path)
        self.assertEqual(mapping["project"], {"owner": "other", "number": 2, "id": "P2"})
        record = mapping["tasks"][UID]
        self.assertEqual(record["workflow_phase"], "execution")
        self.assertEqual(record["status"], "committed")
        self.assertNotIn("closed_without_merge_receipt", record)
        self.assertEqual(self.read_json(self.closes), [])

    def test_non_pr_completed_requires_done_task_complete_closeout(self) -> None:
        cases = (
            {"status": "committed", "workflow_phase": "execution"},
            {"status": "done", "workflow_phase": "execution"},
            {"status": "done", "workflow_phase": "task_done"},
            {
                "status": "done", "workflow_phase": "task_done",
                "last_closed_at": "2026-08-29T00:00:00+08:00",
            },
            {
                "status": "done", "workflow_phase": "task_done",
                "last_closed_at": "2026-08-29T00:00:00+08:00",
                "claim_verifications": [{
                    "claim_type": "task_complete", "status": "pending",
                    "allowed_to_claim": True, "verification_exit_code": 0,
                }],
            },
            {
                "status": "done", "workflow_phase": "task_done",
                "last_closed_at": "2026-08-29T00:00:00+08:00",
                "claim_verifications": [{
                    "claim_type": "task_complete", "status": "verified",
                    "allowed_to_claim": False, "verification_exit_code": 0,
                }],
            },
            {
                "status": "done", "workflow_phase": "task_done",
                "last_closed_at": "2026-08-29T00:00:00+08:00",
                "claim_verifications": [{
                    "claim_type": "task_complete", "status": "verified",
                    "allowed_to_claim": True, "verification_exit_code": 0,
                }],
            },
        )
        for index, extra in enumerate(cases):
            with self.subTest(case=index):
                self.reset_runtime()
                self.mapping(extra=extra)
                result = self.invoke("non_pr_completed")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.read_json(self.comments), [])
                self.assertEqual(self.read_json(self.closes), [])

        self.reset_runtime()
        self.mapping(extra={
            "status": "done",
            "workflow_phase": "task_done",
            "last_closed_at": "2026-08-29T00:00:00+08:00",
            "claim_verifications": [{
                "claim_type": "task_complete", "status": "verified",
                "allowed_to_claim": True, "verification_exit_code": 0,
            }],
            "completion_mode": "non_pr_task",
            "non_pr_completion_evidence": "persisted fixture truth",
        })
        result = self.invoke("non_pr_completed")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_json(self.closes), ["completed"])

    def test_non_merge_and_post_merge_finalizers_share_task_scoped_lock_path(self) -> None:
        canonical_worktree = Path(self.tmp.name) / "canonical-task-worktree"
        canonical_worktree.mkdir()
        self.mapping(
            pr=True,
            merge=True,
            extra={
                "canonical_worktree": str(canonical_worktree),
                "phase_receipts": {"main_sync": {"receipt_type": "oasis7_main_sync"}},
            },
        )
        receipt_root = Path(subprocess.check_output([
            sys.executable, str(ROOT / "scripts/pm/canonical-receipt-root.py"),
            "--default-worktree", str(self.root), "--task-uid", UID, "--create",
        ], cwd=ROOT, env=self.env, text=True).strip())
        terminal = receipt_root / "terminal-cleanup-receipt.json"
        terminal.write_text(json.dumps({
            "receipt_type": "oasis7_terminal_cleanup",
            "issuer": "post-merge-cleanup",
            "task_uid": UID,
            "repository": REPO,
            "issue_number": ISSUE,
            "pr_number": 22,
        }) + "\n")
        subprocess.run([
            sys.executable, str(ROOT / "scripts/pm/post-merge-finalize.py"),
            "--repo-root", str(self.root), "--task-uid", UID,
            "--terminal-receipt", str(terminal),
        ], cwd=ROOT, env=self.env, text=True, capture_output=True)
        self.invoke("not_planned")

        mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        task_locks = sorted(mapping_path.parent.glob(f"{mapping_path.name}.{UID}*lock"))
        expected = mapping_path.with_name(f"{mapping_path.name}.{UID}.finalizer-lock")
        self.assertEqual(task_locks, [expected])

    def test_success_updates_issue_body_before_close_and_retry_reads_it_back(self) -> None:
        self.mapping(pr=True)
        self.env["GH_REQUIRE_ISSUE_DONE"] = "1"
        first = self.invoke("superseded")
        self.assertEqual(first.returncode, 0, first.stderr)
        calls = self.calls()
        close_index = next(index for index, call in enumerate(calls) if call[:2] == ["issue", "close"])
        edit_indices = [index for index, call in enumerate(calls) if call[:2] == ["issue", "edit"]]
        self.assertTrue(edit_indices)
        self.assertLess(max(edit_indices), close_index)
        self.assertIn("- status: `done`", self.issue_body.read_text())

        retry = self.invoke("superseded")
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertIn("- status: `done`", self.issue_body.read_text())

    def test_all_non_merge_reason_close_reason_mapping(self) -> None:
        for reason in ("duplicate", "not_planned", "non_pr_completed"):
            with self.subTest(reason=reason):
                self.reset_runtime()
                extra = None
                if reason == "non_pr_completed":
                    extra = {
                        "status": "done", "workflow_phase": "task_done",
                        "last_closed_at": "2026-08-29T00:00:00+08:00",
                        "claim_verifications": [{
                            "claim_type": "task_complete", "status": "verified",
                            "allowed_to_claim": True, "verification_exit_code": 0,
                        }],
                        "completion_mode": "non_pr_task",
                        "non_pr_completion_evidence": "persisted fixture truth",
                    }
                self.mapping(pr=reason == "duplicate", extra=extra)
                result = self.invoke(reason)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(
                    self.read_json(self.closes),
                    ["completed" if reason == "non_pr_completed" else "not planned"],
                )

    def test_non_pr_completed_with_pr_or_merge_authority_fails(self) -> None:
        for merge in (False, True):
            with self.subTest(merge_authority=merge):
                self.reset_runtime()
                self.mapping(pr=not merge, merge=merge)
                result = self.invoke("non_pr_completed")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.read_json(self.comments), [])
                self.assertEqual(self.read_json(self.closes), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
