#!/usr/bin/env python3
"""Count shared-client sends through the real selected Project refresh path."""
from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
from collections import OrderedDict
from typing import Any
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
PM = ROOT / "scripts" / "pm"
TASK_UID = "task_" + "a" * 32
REPOSITORY = "eng-cc/oasis7"
ISSUE_NUMBER = 8421
ISSUE_URL = f"https://github.com/{REPOSITORY}/issues/{ISSUE_NUMBER}"
PROJECT_ID = "PVT_budget"
ITEM_ID = "PVTI_budget"


def load_module(name: str, path: pathlib.Path, *, register: bool = False):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    if register:
        sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class CountedTransport:
    def __init__(self, project_node: dict[str, Any]):
        self.project_node = project_node
        self.calls: list[dict[str, Any]] = []

    def __call__(self, method, url, headers, body, timeout):
        payload = json.loads(body.decode("utf-8"))
        query = str(payload.get("query") or "")
        self.calls.append({
            "method": method,
            "url": url,
            "query": query,
            "variables": payload.get("variables") or {},
            "timeout": timeout,
        })
        if query.lstrip().lower().startswith("mutation"):
            data = {"f0": {"projectV2Item": {"id": ITEM_ID}}}
        elif "nodes(ids: $ids)" in query:
            data = {"nodes": [self.project_node]}
        else:
            raise AssertionError(f"unexpected GraphQL request in budget fixture: {query[:120]!r}")
        return {
            "status": 200,
            "headers": {
                "X-RateLimit-Remaining": "4999",
                "X-RateLimit-Used": "1",
                "X-RateLimit-Limit": "5000",
            },
            "body": {"data": data},
        }


def run_git(repo: pathlib.Path, *args: str) -> None:
    subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def make_registered_repository(repo: pathlib.Path) -> None:
    subprocess.run(
        ["git", "init", "--quiet", "--initial-branch=task/project-budget", str(repo)],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    run_git(repo, "config", "user.name", "Project budget fixture")
    run_git(repo, "config", "user.email", "project-budget-fixture@example.invalid")
    run_git(repo, "commit", "--quiet", "--allow-empty", "-m", "fixture root")
    run_git(repo, "remote", "add", "origin", f"https://github.com/{REPOSITORY}.git")
    run_git(repo, "update-ref", "refs/remotes/origin/main", "HEAD")
    run_git(repo, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")


def make_issue_cli(directory: pathlib.Path, issue_json: pathlib.Path, calls_file: pathlib.Path) -> None:
    directory.mkdir(parents=True)
    executable = directory / "gh"
    executable.write_text(
        "#!/usr/bin/env python3\n"
        "import json, os, sys\n"
        "args=sys.argv[1:]\n"
        "with open(os.environ['GH_BUDGET_CALLS'], 'a', encoding='utf-8') as out:\n"
        "    out.write(json.dumps(args, separators=(',', ':')) + '\\n')\n"
        "if args[:2] == ['issue', 'list']:\n"
        f"    print(json.dumps([{{'number': {ISSUE_NUMBER}, 'url': {ISSUE_URL!r}, 'title': '[PM] Project budget fixture', 'state': 'OPEN'}}]))\n"
        "elif args[:2] == ['issue', 'view']:\n"
        "    print(open(os.environ['GH_BUDGET_ISSUE'], encoding='utf-8').read())\n"
        "else:\n"
        "    raise SystemExit('unexpected legacy gh invocation: ' + repr(args))\n",
        encoding="utf-8",
    )
    executable.chmod(0o755)
    calls_file.write_text("", encoding="utf-8")


def project_node() -> dict[str, Any]:
    fields = [
        {"name": "In Progress", "field": {"name": "Status"}},
        {"text": TASK_UID, "field": {"name": "Task UID"}},
        {"name": "repository_health_engineer", "field": {"name": "Owner Role"}},
        {"name": "engineering", "field": {"name": "Module"}},
        {"name": "committed", "field": {"name": "PM Status"}},
        {"name": "execution", "field": {"name": "Workflow Phase"}},
        {"name": "P1", "field": {"name": "Priority"}},
        {"text": "fixture-worktree", "field": {"name": "Canonical Worktree"}},
        {"name": "n/a", "field": {"name": "Test Tier Required"}},
    ]
    return {
        "id": ITEM_ID,
        "project": {"id": PROJECT_ID, "number": 1, "owner": {"login": "eng-cc"}},
        "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": fields},
    }


def main() -> int:
    sys.path.insert(0, str(PM))
    api_path = (PM / "github_api.py").resolve()
    api_name = "_oasis7_github_api_" + hashlib.sha256(str(api_path).encode("utf-8")).hexdigest()[:16]
    api_module = load_module(api_name, api_path, register=True)
    task_module = load_module("project_budget_task_under_test", PM / "github-project-task.py")
    sync_module = load_module("project_budget_sync_under_test", PM / "github-project-sync.py")

    with tempfile.TemporaryDirectory(prefix="oasis7-project-api-budget-") as temp:
        temp_root = pathlib.Path(temp)
        repo = temp_root / "task-worktree"
        repo.mkdir()
        repo = repo.resolve()
        make_registered_repository(repo)
        cache = repo / ".pm" / "github-project-sync" / "tasks.json"
        cache.parent.mkdir(parents=True)

        body = "\n".join((
            f"task_uid: {TASK_UID}",
            "- owner_role: `repository_health_engineer`",
            "- module: `engineering`",
            "- status: `committed`",
            "- workflow_phase: `execution`",
            "- priority: `P1`",
            f"- worktree_hint: `{repo}`",
            "",
        ))
        issue = {
            "body": body,
            "number": ISSUE_NUMBER,
            "title": "[PM] Project budget fixture",
            "url": ISSUE_URL,
            "state": "OPEN",
            "stateReason": None,
            "updatedAt": "2026-10-02T00:00:00Z",
        }
        issue_file = temp_root / "issue.json"
        issue_file.write_text(json.dumps(issue), encoding="utf-8")
        calls_file = temp_root / "gh-calls.jsonl"
        fake_bin = temp_root / "bin"
        make_issue_cli(fake_bin, issue_file, calls_file)

        record = {
            "task_uid": TASK_UID,
            "title": "Project budget fixture",
            "owner_role": "repository_health_engineer",
            "module": "engineering",
            "status": "committed",
            "workflow_phase": "execution",
            "priority": "P1",
            "worktree_hint": str(repo),
            "repository": REPOSITORY,
            "canonical_worktree": str(repo),
            "task_branch": "task/project-budget",
            "default_branch": "main",
            "issue_number": ISSUE_NUMBER,
            "issue_url": ISSUE_URL,
            "project_item_id": ITEM_ID,
        }
        mapping = {
            "version": 1,
            "project": {"owner": "eng-cc", "number": 1, "id": PROJECT_ID, "repo": REPOSITORY},
            "tasks": {TASK_UID: record},
        }
        cache.write_text(json.dumps(mapping) + "\n", encoding="utf-8")

        transport = CountedTransport(project_node())
        client = api_module.GitHubAPIClient(
            "injected-budget-fixture-token",
            transport=transport,
            state_root=temp_root / "github-api-state",
            clock=lambda: 1_790_000_000.0,
            sleeper=lambda _seconds: None,
            random_value=lambda: 0.0,
        )
        operations: list[str] = []
        original_log_attempt = client._log_attempt

        def record_operation(operation, *args, **kwargs):
            operations.append(operation)
            return original_log_attempt(operation, *args, **kwargs)

        client._log_attempt = record_operation
        api_module.GitHubAPIClient.from_gh = classmethod(lambda _cls, **_kwargs: client)

        env = {
            "PATH": str(fake_bin) + os.pathsep + os.environ.get("PATH", ""),
            "GH_BUDGET_CALLS": str(calls_file),
            "GH_BUDGET_ISSUE": str(issue_file),
        }
        args = task_module.argparse.Namespace(
            root=repo,
            mapping=str(cache.relative_to(repo)),
            task_uid=TASK_UID,
            repo=REPOSITORY,
            project_owner="eng-cc",
            project_number=1,
            json=True,
        )
        output = io.StringIO()
        errors = io.StringIO()
        try:
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors), mock.patch.dict(os.environ, env):
                result = task_module.command_refresh_task(args)
        except BaseException as exc:
            raise AssertionError(
                f"command_refresh_task raised {exc!r}: stdout={output.getvalue()!r}; stderr={errors.getvalue()!r}"
            ) from exc

        if result != 0:
            raise AssertionError(f"command_refresh_task returned {result}: {output.getvalue()}")
        refresh_call_count = len(transport.calls)
        refresh_operations = list(operations)
        cli_calls = [json.loads(line) for line in calls_file.read_text(encoding="utf-8").splitlines() if line]
        if refresh_call_count != 1:
            raise AssertionError(f"known-item refresh expected one GraphQL send, got {refresh_call_count}")
        if refresh_operations != ["project_task_refresh_bound_item"]:
            raise AssertionError(f"unexpected refresh operations: {refresh_operations}")
        if transport.calls[0]["method"] != "POST" or not transport.calls[0]["url"].endswith("/graphql"):
            raise AssertionError(f"refresh did not use GraphQL transport: {transport.calls[0]}")
        if transport.calls[0]["variables"] != {"ids": [ITEM_ID]}:
            raise AssertionError(f"refresh was not bound-item query: {transport.calls[0]['variables']}")
        if len(cli_calls) != 2 or [call[:2] for call in cli_calls] != [["issue", "list"], ["issue", "view"]]:
            raise AssertionError(f"Issue CLI cold calls were not counted separately: {cli_calls}")

        # Exercise the actual direct Project field updater with the same real
        # client. A proven unchanged value sends nothing; changed and unknown
        # values both reach the injected HTTP boundary once.
        update_task = OrderedDict((("task_uid", TASK_UID), ("status", "committed")))
        field_map = {"Task UID": {"id": "PVTF_uid"}}
        controls = {}
        for name, current in (
            ("unchanged", {"Task UID": TASK_UID}),
            ("changed", {"Task UID": "task_" + "b" * 32}),
            ("unknown", {}),
        ):
            transport.calls.clear()
            operations.clear()
            updated, skipped = sync_module.update_fields_direct(
                client,
                PROJECT_ID,
                ITEM_ID,
                update_task,
                field_map,
                only_fields={"Task UID"},
                current_values=current,
            )
            calls = len(transport.calls)
            expected = 0 if name == "unchanged" else 1
            if calls != expected:
                raise AssertionError(f"{name} field-control expected {expected} sends, got {calls}")
            if calls and operations != ["project_sync_update_fields"]:
                raise AssertionError(f"{name} field-control operation mismatch: {operations}")
            if name == "unchanged" and (updated != 0 or skipped != ["Task UID:unchanged"]):
                raise AssertionError(f"unchanged control failed: updated={updated}, skipped={skipped}")
            if name != "unchanged" and updated != 1:
                raise AssertionError(f"{name} field-control did not update: {updated}, {skipped}")
            controls[name] = {
                "graphql_attempts": calls,
                "operations": list(operations),
                "updated_fields": updated,
                "skipped": skipped,
            }

        print(json.dumps({
            "status": "passed",
            "refresh": {
                "graphql_attempts": refresh_call_count,
                "operations": refresh_operations,
                "legacy_issue_cli_calls": len(cli_calls),
                "legacy_issue_cli_operations": [call[0] + " " + call[1] for call in cli_calls],
            },
            "field_update_controls": controls,
            "result": json.loads(output.getvalue()),
        }, sort_keys=True))
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
