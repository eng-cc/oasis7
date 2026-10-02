#!/usr/bin/env python3
"""Process-level publisher/recovery checks against a stateful fake `gh` CLI.

These tests run the real pr_projection_publish.py and github-project-task.py
entrypoints in linked Git worktrees. The fake executable models live Issue,
Project, PR, comment, permission, and pagination reads; Git push still targets
a real local bare remote.
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
REPOSITORY = "eng-cc/oasis7"
UID = "task_" + "a" * 32
ISSUE = 42
PR = 55
PROJECT_ID = "PVT_fixture"
PROJECT_ITEM = "PVTI_fixture"
PROJECT_NUMBER = 1
LOGIN = "task-owner"
PR_URL = f"https://github.com/{REPOSITORY}/pull/{PR}"
OLD_BASE = "745a98d41f936cbc35f7a612262b42aa2997130d"


def load_module(name: str, path: Path):
    sys.path.insert(0, str(path.parent))
    try:
        spec = importlib.util.spec_from_file_location(name, path)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.pop(0)


task_api = load_module("publisher_test_task_api", HERE / "github-project-task.py")
sync_api = load_module("publisher_test_sync_api", HERE / "github-project-sync.py")


FAKE_GH = r'''#!/usr/bin/env python3
import json, os, pathlib, signal, sys

state_path = pathlib.Path(os.environ["FAKE_GH_STATE"])
state = json.loads(state_path.read_text())
args = sys.argv[1:]
state.setdefault("calls", []).append(args)

def save():
    state_path.write_text(json.dumps(state, sort_keys=True))

def emit(value, code=0):
    if isinstance(value, str):
        sys.stdout.write(value)
    else:
        sys.stdout.write(json.dumps(value))
    sys.stdout.write("\n")
    save()
    raise SystemExit(code)

def option_values(name):
    return state["project_fields"][name].get("options", {})

def project_field_values():
    return state["project_values"]

def graphql_flags():
    result = {}
    for index, arg in enumerate(args[:-1]):
        if arg in {"-f", "-F"}:
            key, sep, value = args[index + 1].partition("=")
            if sep:
                result[key] = value
    return result

def field_nodes():
    result = []
    for name, value in project_field_values().items():
        node = {"field": {"name": name}}
        if name in state["single_select_fields"]:
            node["name"] = value
        else:
            node["text"] = value
        result.append(node)
    return result

def issue_payload():
    issue = state["issue"]
    return {
        "id": "I_fixture", "node_id": "I_fixture", "number": issue["number"],
        "url": issue["url"], "html_url": issue["url"], "title": issue["title"],
        "body": issue["body"], "state": issue["state"].upper(), "state_reason": None,
        "stateReason": None, "user": issue["user"],
        "updated_at": issue["updated_at"], "updatedAt": issue["updated_at"],
    }

def append_comment(body):
    next_id = state.get("next_comment_id", 100)
    state["next_comment_id"] = next_id + 1
    created = "2026-10-02T02:00:%02dZ" % (next_id - 100)
    comment = {
        "id": next_id, "body": body,
        "html_url": f"{state['issue']['url']}#issuecomment-{next_id}",
        "url": f"{state['issue']['url']}#issuecomment-{next_id}",
        "created_at": created, "updated_at": created,
        "user": {"login": state["login"], "type": "User"},
        "author_association": "MEMBER",
    }
    state["comments"].append(comment)
    kind = ("C1" if "<!-- oasis7-ci-publication/v1 -->" in body else
            "binding" if "<!-- oasis7-ci-publication-binding/v1 -->" in body else "evidence")
    timestamp_fault = state.setdefault("faults", {}).pop("comment:" + kind + ":timestamp", None)
    if kind == "C1" and timestamp_fault == "edited":
        comment["updated_at"] = "2026-10-02T02:00:01Z"
    elif kind == "C1" and timestamp_fault == "missing":
        comment.pop("updated_at", None)
    elif kind == "C1" and timestamp_fault == "invalid":
        comment["updated_at"] = "not-a-server-timestamp"
    state.setdefault("mutations", []).append({"kind": "comment:" + kind, "effect": True})
    return comment

def pull_payload():
    pr = state.get("pr")
    if pr is None:
        return None
    return pr

if not args:
    raise SystemExit("missing gh command")

if args[0] == "auth" and args[1:] == ["token"]:
    emit("fixture-token")

if args[:2] == ["project", "view"]:
    emit({"id": state["project_id"]})

if args[:2] == ["project", "field-list"]:
    fields = []
    for name, definition in state["project_fields"].items():
        fields.append({"id": definition["id"], "name": name,
                       "options": [{"name": label, "id": option_id}
                                   for label, option_id in definition.get("options", {}).items()]})
    emit({"fields": fields})

if args[:2] == ["project", "item-edit"]:
    field_id = args[args.index("--field-id") + 1]
    field_name = state["field_names_by_id"].get(field_id)
    if not field_name:
        raise SystemExit("unknown Project field id: " + field_id)
    key = "project:" + field_name
    fault = state.setdefault("faults", {}).pop(key, None)
    mutation = {"kind": key, "effect": False}
    state.setdefault("mutations", []).append(mutation)
    if fault == "before":
        save()
        raise SystemExit("injected pre-effect Project write failure")
    if fault == "interrupt-before":
        save()
        os.kill(os.getppid(), signal.SIGKILL)
        raise SystemExit("unreachable after interrupted Project write")
    if "--single-select-option-id" in args:
        option_id = args[args.index("--single-select-option-id") + 1]
        value = state["option_values_by_id"].get(option_id)
        if value is None:
            raise SystemExit("unknown single-select option: " + option_id)
    else:
        value = args[args.index("--text") + 1]
    state["project_values"][field_name] = value
    mutation["effect"] = True
    save()
    if fault == "interrupt-after":
        os.kill(os.getppid(), signal.SIGKILL)
        raise SystemExit("unreachable after interrupted Project write")
    if fault == "after":
        raise SystemExit("injected lost Project write response")
    emit({"id": "PVTI_fixture"})

if args[0] == "issue" and args[1] == "list":
    issue = state["issue"]
    emit([{"number": issue["number"], "url": issue["url"],
           "title": issue["title"], "state": issue["state"]}])

if args[0] == "issue" and args[1] == "view":
    emit(issue_payload())

if args[0] == "issue" and args[1] == "edit":
    body_path = args[args.index("--body-file") + 1]
    body = pathlib.Path(body_path).read_text(encoding="utf-8")
    fault = state.setdefault("faults", {}).pop("issue:body", None)
    mutation = {"kind": "issue:body", "effect": False}
    state.setdefault("mutations", []).append(mutation)
    if fault == "before":
        save()
        raise SystemExit("injected pre-effect Issue write failure")
    state["issue"]["body"] = body
    state["issue"]["updated_at"] = "2026-10-02T03:00:00Z"
    mutation["effect"] = True
    save()
    if fault == "after":
        raise SystemExit("injected lost Issue write response")
    emit("edited")

if args[0] == "issue" and args[1] == "comment":
    body_path = args[args.index("--body-file") + 1]
    body = pathlib.Path(body_path).read_text(encoding="utf-8")
    kind = ("C1" if "<!-- oasis7-ci-publication/v1 -->" in body else
            "binding" if "<!-- oasis7-ci-publication-binding/v1 -->" in body else "evidence")
    fault = state.setdefault("faults", {}).pop("comment:" + kind, None)
    state.setdefault("post_attempts", {}).setdefault(kind, 0)
    state["post_attempts"][kind] += 1
    if fault == "before":
        state.setdefault("mutations", []).append({"kind": "comment:" + kind, "effect": False})
        save()
        raise SystemExit("injected pre-effect Issue comment failure")
    comment = append_comment(body)
    save()
    if fault == "after":
        raise SystemExit("injected lost Issue comment response")
    emit(comment["html_url"])

if args[0] == "pr" and args[1] == "create":
    body_path = args[args.index("--body-file") + 1]
    body = pathlib.Path(body_path).read_text(encoding="utf-8")
    fault = state.setdefault("faults", {}).pop("pr:create", None)
    state.setdefault("mutations", []).append({"kind": "pr:create", "effect": False})
    if state.get("pr") is None:
        state["pr"] = {
            "number": state["pr_number"], "html_url": state["pr_url"],
            "state": "open", "merged_at": None, "draft": True,
            "created_at": "2026-10-02T02:01:00Z", "updated_at": "2026-10-02T02:01:00Z",
            "body": body, "user": {"login": state["login"], "type": "User"},
            "head": {"repo": {"full_name": state["repository"]},
                     "ref": state["source_ref"], "sha": state["source_head"]},
            "base": {"repo": {"full_name": state["repository"]}, "ref": state["target_ref"]},
        }
        state["mutations"][-1]["effect"] = True
    save()
    if fault == "after":
        raise SystemExit("injected lost PR create response")
    emit(state["pr_url"])

if args[0] == "api":
    endpoint = args[1] if len(args) > 1 else ""
    if endpoint == "user":
        emit({"login": state["login"], "type": "User"})
    if endpoint == "graphql":
        flags = graphql_flags()
        query = flags.get("query", "")
        if "repository(owner:" in query:
            issue = state["issue"]
            project = {"id": state["project_id"], "number": state["project_number"],
                       "viewerCanUpdate": True, "owner": {"login": state["project_owner"]}}
            item = {"id": state["project_item"], "isArchived": False, "project": project}
            payload = {"data": {"repository": {"issue": {
                "id": "I_fixture", "number": issue["number"], "url": issue["url"],
                "state": issue["state"].upper(), "body": issue["body"],
                "projectItems": {"nodes": [item],
                                 "pageInfo": {"hasNextPage": False, "endCursor": None}},
            }}}}
            emit(payload)
        item_id = flags.get("item", state["project_item"])
        project = {"id": state["project_id"], "number": state["project_number"],
                   "viewerCanUpdate": state.get("project_viewer_can_update", True),
                   "owner": {"login": state["project_owner"]}}
        page_info = {"hasNextPage": bool(state.get("faults", {}).get("project-fields-incomplete")),
                     "endCursor": None}
        item = {"id": item_id, "isArchived": False, "project": project,
                "fieldValues": {"nodes": field_nodes(),
                                "pageInfo": page_info}}
        emit({"data": {"node": item}})
    if endpoint == "repos/" + state["repository"]:
        emit({"id": state["repository_id"], "default_branch": state["target_ref"]})
    if endpoint.startswith("repos/" + state["repository"] + "/collaborators/") and endpoint.endswith("/permission"):
        emit({"permission": "write", "permissions": {"push": True}})
    if endpoint.startswith("repos/" + state["repository"] + "/issues/comments/"):
        comment_id = int(endpoint.rsplit("/", 1)[-1])
        found = [item for item in state["comments"] if item["id"] == comment_id]
        if len(found) != 1:
            raise SystemExit("Issue comment not found: " + str(comment_id))
        emit(found[0])
    if endpoint == "repos/" + state["repository"] + f"/issues/{state['issue']['number']}":
        emit(issue_payload())
    if endpoint.startswith("repos/" + state["repository"] + f"/issues/{state['issue']['number']}/comments"):
        if state.get("faults", {}).get("issue-comments-read"):
            save()
            raise SystemExit("403 injected incomplete Issue comment pagination")
        comments = state["comments"]
        if (state.get("edit_c1_on_third_locked_comments_read") is True
                and state.get("pr") is not None
                and not any(item.get("kind") == "issue:body" and item.get("effect")
                            for item in state.get("mutations", []))):
            state["record_pr_c1_reads"] = state.get("record_pr_c1_reads", 0) + 1
            if state["record_pr_c1_reads"] == 3:
                for comment in comments:
                    if "<!-- oasis7-ci-publication/v1 -->" in comment.get("body", ""):
                        comment["updated_at"] = "2026-10-02T02:00:59Z"
        emit([comments] if "--slurp" in args else comments)
    if endpoint.startswith("repos/" + state["repository"] + "/pulls?"):
        if (state.get("pr") is not None
                and state.get("faults", {}).get("pr-list-after-create")):
            save()
            raise SystemExit("injected unconfirmable PR list readback")
        pr = pull_payload()
        emit([[pr]] if pr is not None else [[]])
    if endpoint.startswith("repos/" + state["repository"] + "/pulls/"):
        pr = pull_payload()
        if pr is None:
            raise SystemExit("PR not found")
        emit(pr)
    raise SystemExit("unhandled fake gh API endpoint: " + endpoint)

raise SystemExit("unhandled fake gh command: " + " ".join(args))
'''


def git(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], text=True,
                            capture_output=True, check=False)
    if check and result.returncode:
        raise AssertionError(f"git {args!r} failed: {result.stderr}")
    if result.returncode and not check:
        return str(result.returncode)
    return result.stdout.strip()


class PublisherProcessTests(unittest.TestCase):
    maxDiff = None

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="pr-publisher-cli-")
        self.tmp = Path(self.temp.name).resolve()
        self.repo = self.tmp / "repo"
        self.task_root = self.tmp / "task-worktree"
        self.remote = self.tmp / "origin.git"
        self.bin = self.tmp / "bin"
        self.state_path = self.tmp / "fake-gh-state.json"
        self.projection_input = self.tmp / "projection-input.json"
        self.projection_path = self.tmp / "projection.json"
        self.body_path = self.tmp / "pr-body.md"
        self.repo.mkdir()
        self.bin.mkdir()
        (self.bin / "gh").write_text(FAKE_GH, encoding="utf-8")
        (self.bin / "gh").chmod(0o755)
        self._seed_repository()
        self.env = os.environ.copy()
        self.env.update({
            "PATH": str(self.bin) + os.pathsep + self.env.get("PATH", ""),
            "FAKE_GH_STATE": str(self.state_path),
            "OASIS7_PM_FAKE_GITHUB": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "GIT_AUTHOR_NAME": LOGIN,
            "GIT_AUTHOR_EMAIL": "task-owner@example.test",
            "GIT_COMMITTER_NAME": LOGIN,
            "GIT_COMMITTER_EMAIL": "task-owner@example.test",
        })
        self.state = self._initial_state()
        self._save_state()
        self._build_projection()

    def tearDown(self):
        self.temp.cleanup()

    def _seed_repository(self):
        subprocess.run(["git", "init", "--bare", "--initial-branch=main", str(self.remote)],
                       check=True, capture_output=True)
        subprocess.run(["git", "init", "--initial-branch=main", str(self.repo)],
                       check=True, capture_output=True)
        git(self.repo, "config", "user.name", LOGIN)
        git(self.repo, "config", "user.email", "task-owner@example.test")
        git(self.repo, "remote", "add", "origin", str(self.remote))

        shutil.copytree(HERE, self.repo / "scripts/pm",
                        ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        (self.repo / "scripts").mkdir(exist_ok=True)
        for relative in ("scripts/plan-rust-required-scope.py",
                         "scripts/ci-required-scope.v2.json", "scripts/ci-tests.sh"):
            destination = self.repo / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO_ROOT / relative, destination)
        roles = self.repo / ".agents/roles"
        roles.mkdir(parents=True)
        shutil.copy2(REPO_ROOT / ".agents/roles/repository_health_engineer.md",
                     roles / "repository_health_engineer.md")
        (self.repo / ".gitignore").write_text(
            ".pm/\n__pycache__/\n*.pyc\n",
            encoding="utf-8",
        )
        (self.repo / "doc").mkdir()
        (self.repo / "doc/fixture.md").write_text("fixture baseline\n", encoding="utf-8")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-m", "trusted fixture base")
        self.target_oid = git(self.repo, "rev-parse", "HEAD")
        git(self.repo, "push", "origin", "main")
        git(self.repo, "fetch", "origin", "main")
        git(self.repo, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        subprocess.run(["git", "-C", str(self.repo), "worktree", "add", "-b", "task/publication-fixture",
                        str(self.task_root), "main"], check=True, capture_output=True)

        record = {
            "task_uid": UID, "repository": REPOSITORY, "issue_number": ISSUE,
            "issue_url": f"https://github.com/{REPOSITORY}/issues/{ISSUE}",
            "project_item_id": PROJECT_ITEM, "title": "Publisher subprocess fixture",
            "owner_role": "repository_health_engineer", "module": "engineering",
            "priority": "P2", "status": "committed", "workflow_phase": "execution",
            "worktree_hint": str(self.task_root), "canonical_worktree": str(self.task_root),
            "task_branch": "task/publication-fixture", "default_branch": "main",
            "bootstrap_epoch": None, "updated_at": "2026-10-02T00:00:00Z",
            "merge_hold": {"active": False},
        }
        mapping = {"version": 1,
                   "project": {"id": PROJECT_ID, "owner": "eng-cc", "number": PROJECT_NUMBER},
                   "tasks": {UID: record}}
        mapping_path = self.task_root / ".pm/github-project-sync/tasks.json"
        mapping_path.parent.mkdir(parents=True)
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        git(self.task_root, "remote", "set-head", "origin", "main")
        (self.task_root / "doc/fixture.md").write_text("fixture change\n", encoding="utf-8")
        git(self.task_root, "add", "doc/fixture.md")
        git(self.task_root, "commit", "-m", "task source fixture")
        self.source_head = git(self.task_root, "rev-parse", "HEAD")
        self.changed_paths = git(self.task_root, "diff", "--name-only", f"{self.target_oid}...{self.source_head}").splitlines()
        self.issue_body = task_api.issue_body(task_api.task_from_record(UID, record))
        self.body_path.write_text(f"Task: {UID}\nRefs #{ISSUE}\n\nPublisher fixture.\n", encoding="utf-8")

    def _initial_state(self):
        base_task = task_api.task_from_record(UID, {
            "task_uid": UID, "title": "Publisher subprocess fixture",
            "owner_role": "repository_health_engineer", "module": "engineering",
            "priority": "P2", "status": "committed", "workflow_phase": "execution",
            "worktree_hint": str(self.task_root), "merge_hold": {"active": False},
        })
        project_values = sync_api.project_field_values(base_task)
        options = {
            "Status": {"Todo": "OPT_STATUS_TODO", "In Progress": "OPT_STATUS_PROGRESS",
                       "Blocked": "OPT_STATUS_BLOCKED", "Ready / PR": "OPT_STATUS_READY",
                       "PR Watch": "OPT_STATUS_WATCH", "Done": "OPT_STATUS_DONE"},
            "Owner Role": {"repository_health_engineer": "OPT_OWNER_RH"},
            "Module": {"engineering": "OPT_MODULE_ENGINEERING"},
            "PM Status": {"committed": "OPT_PM_COMMITTED"},
            "Workflow Phase": {"execution": "OPT_PHASE_EXECUTION",
                                "verification": "OPT_PHASE_VERIFICATION"},
            "Priority": {"P2": "OPT_PRIORITY_P2"},
            "Test Tier Required": {"n/a": "OPT_TEST_NA"},
        }
        fields = {}
        for index, name in enumerate(project_values, 1):
            fields[name] = {"id": f"FIELD_{index}", "options": options.get(name, {})}
        option_values_by_id = {option_id: value for name, entries in options.items()
                               for value, option_id in entries.items()}
        return {
            "repository": REPOSITORY, "repository_id": 701,
            "issue": {"number": ISSUE, "url": f"https://github.com/{REPOSITORY}/issues/{ISSUE}",
                      "title": "[PM] Publisher subprocess fixture", "body": self.issue_body,
                      "state": "open", "updated_at": "2026-10-02T00:00:00Z",
                      "user": {"login": LOGIN, "type": "User"}},
            "comments": [], "next_comment_id": 100,
            "pr": None, "pr_number": PR, "pr_url": PR_URL,
            "source_head": self.source_head, "source_ref": "task/publication-fixture",
            "target_ref": "main", "login": LOGIN,
            "project_id": PROJECT_ID, "project_item": PROJECT_ITEM,
            "project_number": PROJECT_NUMBER, "project_owner": "eng-cc",
            "project_values": project_values, "project_fields": fields,
            "field_names_by_id": {value["id"]: key for key, value in fields.items()},
            "single_select_fields": list(options),
            "option_values_by_id": option_values_by_id,
            "mutations": [], "faults": {}, "post_attempts": {}, "calls": [],
        }

    def _save_state(self):
        self.state_path.write_text(json.dumps(self.state, sort_keys=True), encoding="utf-8")

    def _load_state(self):
        self.state = json.loads(self.state_path.read_text(encoding="utf-8"))
        return self.state

    def _build_projection(self):
        evidence_bytes = (self.task_root / "doc/fixture.md").read_bytes()
        payload = {
            "task_uid": UID,
            "source_head_oid": self.source_head,
            "scope_base_oid": self.target_oid,
            "changed_paths": self.changed_paths,
            "change_class": "mechanical-doc",
            "manual_roles": [],
            "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["scripts/pm/pr_projection_publication.test.py"],
            "consumed_contracts": ["FS-R04"],
            "public_semantics": [],
            "affected_consumers": [],
            "closure_status": {"status": "complete", "reason": "fixture closure",
                               "evidence": [{"path": "doc/fixture.md",
                                             "sha256": "sha256:" + hashlib.sha256(evidence_bytes).hexdigest()}]},
            "verification_affected": False,
        }
        self.projection_input.write_text(json.dumps(payload), encoding="utf-8")
        result = subprocess.run([
            sys.executable, str(self.task_root / "scripts/pm/workflow-impact-projection.py"),
            "--root", str(self.task_root), "--input", str(self.projection_input),
            "--out", str(self.projection_path), "--planner-authority-oid", self.target_oid,
        ], text=True, capture_output=True, env=self.env if hasattr(self, "env") else os.environ.copy())
        if result.returncode:
            raise AssertionError(f"projection fixture failed: {result.stderr}\n{result.stdout}")

    def publisher_command(self, helper: Path | None = None) -> list[str]:
        return [
            sys.executable, str(self.task_root / "scripts/pm/pr_projection_publish.py"),
            "--worktree", str(self.task_root), "--repo", REPOSITORY,
            "--issue-number", str(ISSUE), "--task-uid", UID, "--remote", "origin",
            "--source-ref", "task/publication-fixture", "--target-ref", "main",
            "--source-head", self.source_head, "--target-oid", self.target_oid,
            "--projection", str(self.projection_path), "--body-file", str(self.body_path),
            "--task-helper", str(helper or self.task_root / "scripts/pm/github-project-task.py"),
            "--json",
        ]

    def run_publisher(self, *, helper: Path | None = None):
        return subprocess.run(self.publisher_command(helper), text=True,
                              capture_output=True, env=self.env, timeout=90)

    def old_745_helper(self) -> Path:
        result = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "show",
             f"{OLD_BASE}:scripts/pm/github-project-task.py"],
            text=True, capture_output=True, check=False,
        )
        if result.returncode:
            raise AssertionError(f"cannot read frozen 745 helper: {result.stderr}")
        helper_root = self.tmp / "old-745-scripts"
        helper_root.mkdir()
        for sibling in (self.task_root / "scripts/pm").iterdir():
            if sibling.is_file():
                (helper_root / sibling.name).symlink_to(sibling.resolve())
        helper = helper_root / "github-project-task.py"
        helper.unlink()
        helper.write_text(result.stdout, encoding="utf-8")
        return helper

    def _task_state(self, state=None):
        state = state or self._load_state()
        body = state["issue"]["body"]
        def one(pattern):
            import re
            match = re.findall(pattern, body, re.MULTILINE)
            return match[-1] if match else None
        pr_number = one(r"^- pr_number: `([0-9]+)`$")
        pr_url = one(r"^- pr_url: `([^`]+)`$")
        return {"status": one(r"^- status: `([^`]+)`$"),
                "workflow_phase": one(r"^- workflow_phase: `([^`]+)`$"),
                "pr_number": int(pr_number) if pr_number else None,
                "pr_url": pr_url}

    def _effects(self, kind: str, state=None) -> list[dict]:
        state = state or self._load_state()
        return [item for item in state["mutations"] if item["kind"] == kind and item["effect"]]

    def _assert_complete(self, state=None):
        state = state or self._load_state()
        self.assertEqual({"status": "committed", "workflow_phase": "verification",
                          "pr_number": PR, "pr_url": PR_URL}, self._task_state(state))
        self.assertEqual("verification", state["project_values"]["Workflow Phase"])
        self.assertEqual(PR_URL, state["project_values"]["PR"])
        self.assertIsNotNone(state["pr"])
        self.assertIs(state["pr"]["draft"], True)
        c1 = [item for item in state["comments"] if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        reciprocal = [item for item in state["comments"] if "<!-- oasis7-ci-publication-binding/v1 -->" in item["body"]]
        self.assertEqual(1, len(c1), "one immutable C1 intent must exist")
        self.assertEqual(1, len(reciprocal), "one reciprocal C1 binding must exist")

    def _journal(self):
        journals = list((self.repo / ".git/oasis7/pr-publication").glob("*/*/journal.json"))
        self.assertEqual(1, len(journals), f"expected one exact publication journal, found {journals}")
        return json.loads(journals[0].read_text(encoding="utf-8"))

    def test_real_publisher_and_record_pr_complete_then_noop_retry(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        self.assertEqual("published", json.loads(first.stdout)["status"])
        state = self._load_state()
        self._assert_complete(state)
        self.assertEqual(1, len(self._effects("pr:create", state)))
        self.assertEqual(1, len(self._effects("issue:body", state)))
        self.assertEqual(1, len(self._effects("project:Workflow Phase", state)))
        self.assertEqual(1, len(self._effects("project:PR", state)))
        first_mutations = list(state["mutations"])
        self.assertEqual("", git(self.task_root, "status", "--porcelain=v1"))

        second = self.run_publisher()
        self.assertEqual(0, second.returncode, second.stderr)
        self._assert_complete(self._load_state())
        self.assertEqual(first_mutations, self._load_state()["mutations"],
                         "completed replay must be a live-read no-op")
        self.assertEqual(1, len(self._effects("pr:create")))
        self.assertEqual(1, len(self._effects("issue:body")))

    def test_old_745_helper_happy_control_exposes_completed_replay_write(self):
        helper = self.old_745_helper()
        self.assertEqual("", git(self.task_root, "status", "--porcelain=v1"),
                         "old-helper fixture setup dirtied the source worktree")
        first = self.run_publisher(helper=helper)
        self.assertEqual(0, first.returncode, first.stderr)
        self._assert_complete(self._load_state())
        first_mutations = list(self._load_state()["mutations"])

        replay = self.run_publisher(helper=helper)
        self.assertEqual(0, replay.returncode, replay.stderr)
        state = self._load_state()
        self._assert_complete(state)
        self.assertEqual(1, len(self._effects("pr:create", state)))
        self.assertEqual(2, len(self._effects("issue:body", state)),
                         "the 745 helper is a happy control but rewrites completed Issue state")
        self.assertGreater(len(state["mutations"]), len(first_mutations),
                           "old completed replay should provide the behavioral RED witness")

    def test_pr_create_without_confirmable_readback_stays_pending_without_repost(self):
        self.state["faults"]["pr-list-after-create"] = "always"
        self._save_state()
        first = self.run_publisher()
        self.assertNotEqual(0, first.returncode, first.stderr)
        first_state = self._load_state()
        self.assertEqual(1, len(self._effects("pr:create", first_state)))
        self.assertEqual(1, first_state["post_attempts"]["C1"])
        self.assertIsNotNone(first_state["pr"])

        retry = self.run_publisher()
        self.assertNotEqual(0, retry.returncode, retry.stderr)
        final = self._load_state()
        self.assertEqual(first_state["mutations"], final["mutations"],
                         "unconfirmable retry must not blindly repeat any write")
        self.assertEqual(1, len(self._effects("pr:create", final)))
        self.assertEqual(1, final["post_attempts"]["C1"])

    def test_project_phase_effect_interruption_retries_only_pr_field(self):
        self.state["faults"]["project:Workflow Phase"] = "interrupt-after"
        self._save_state()
        interrupted = self.run_publisher()
        self.assertNotEqual(0, interrupted.returncode, interrupted.stderr)
        partial = self._load_state()
        self.assertEqual({"status": "committed", "workflow_phase": "verification",
                          "pr_number": PR, "pr_url": PR_URL}, self._task_state(partial))
        self.assertEqual("verification", partial["project_values"]["Workflow Phase"])
        self.assertEqual("", partial["project_values"]["PR"])
        self.assertEqual(1, len(self._effects("issue:body", partial)))
        self.assertEqual(1, len(self._effects("project:Workflow Phase", partial)))
        self.assertEqual([], self._effects("project:PR", partial))

        retried = self.run_publisher()
        self.assertEqual(0, retried.returncode, retried.stderr)
        final = self._load_state()
        self._assert_complete(final)
        self.assertEqual(1, len(self._effects("issue:body", final)))
        self.assertEqual(1, len(self._effects("project:Workflow Phase", final)))
        self.assertEqual(1, len(self._effects("project:PR", final)))

    def test_completed_remote_vector_rebuilds_missing_cache_without_writes(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        before = self._load_state()["mutations"]
        mapping_path = self.task_root / ".pm/github-project-sync/tasks.json"
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        mapping["tasks"].pop(UID)
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")

        rebuilt = subprocess.run([
            sys.executable, str(self.task_root / "scripts/pm/github-project-task.py"),
            "refresh-task", str(self.task_root), "--repo", REPOSITORY,
            "--project-owner", "eng-cc", "--project-number", str(PROJECT_NUMBER),
            "--task-uid", UID, "--json",
        ], text=True, capture_output=True, env=self.env, timeout=60)

        self.assertEqual(0, rebuilt.returncode, rebuilt.stderr)
        self.assertEqual(before, self._load_state()["mutations"],
                         "cache rebuild must not mutate Issue, Project, or PR")
        recovered = json.loads(mapping_path.read_text(encoding="utf-8"))["tasks"][UID]
        self.assertEqual("committed", recovered["status"])
        self.assertEqual("verification", recovered["workflow_phase"])
        self.assertEqual(PR, recovered["pr_number"])
        self.assertEqual(PR_URL, recovered["pr_url"])

    def test_active_hold_and_task_uid_conflicts_block_before_publication_writes(self):
        self.state["issue"]["body"] = self.state["issue"]["body"].replace(
            "- merge_hold_active: `false`", "- merge_hold_active: `true`",
        )
        self._save_state()
        held = self.run_publisher()
        self.assertNotEqual(0, held.returncode, "active hold must block publication")
        self.assertEqual([], self._load_state()["mutations"])
        self.assertEqual([], self._load_state()["comments"])
        self.assertIsNone(self._load_state()["pr"])

    def test_bound_pr_draft_and_repository_conflicts_block_before_writes(self):
        def bind_issue(number, url):
            self.state["issue"]["body"] += (
                f"- pr_url: `{url}`\n- pr_number: `{number}`\n"
            )

        bind_issue(99, f"https://github.com/{REPOSITORY}/pull/99")
        self._save_state()
        other_pr = self.run_publisher()
        self.assertNotEqual(0, other_pr.returncode)
        self.assertEqual([], self._load_state()["mutations"])
        self.assertEqual([], self._load_state()["comments"])

        self.state = self._initial_state()
        self.state["pr"] = {
            "number": PR, "html_url": PR_URL, "state": "open", "merged_at": None,
            "draft": False, "created_at": "2026-10-02T02:01:00Z",
            "updated_at": "2026-10-02T02:01:00Z", "body": "",
            "user": {"login": LOGIN, "type": "User"},
            "head": {"repo": {"full_name": REPOSITORY},
                     "ref": "task/publication-fixture", "sha": self.source_head},
            "base": {"repo": {"full_name": REPOSITORY}, "ref": "main"},
        }
        bind_issue(PR, PR_URL)
        self._save_state()
        wrong_draft = self.run_publisher()
        self.assertNotEqual(0, wrong_draft.returncode)
        self.assertEqual([], self._load_state()["mutations"])
        self.assertEqual([], self._load_state()["comments"])

        self.state = self._initial_state()
        self.state["pr"] = {
            "number": PR, "html_url": PR_URL, "state": "open", "merged_at": None,
            "draft": True, "created_at": "2026-10-02T02:01:00Z",
            "updated_at": "2026-10-02T02:01:00Z", "body": "",
            "user": {"login": LOGIN, "type": "User"},
            "head": {"repo": {"full_name": "fork/other"},
                     "ref": "task/publication-fixture", "sha": self.source_head},
            "base": {"repo": {"full_name": REPOSITORY}, "ref": "main"},
        }
        bind_issue(PR, PR_URL)
        self._save_state()
        wrong_repository = self.run_publisher()
        self.assertNotEqual(0, wrong_repository.returncode)
        self.assertEqual([], self._load_state()["mutations"])
        self.assertEqual([], self._load_state()["comments"])

        self.state = self._initial_state()
        self.state["issue"]["body"] = self.state["issue"]["body"].replace(UID, "task_" + "b" * 32)
        self._save_state()
        wrong_uid = self.run_publisher()
        self.assertNotEqual(0, wrong_uid.returncode, "mismatched live UID must block publication")
        self.assertEqual([], self._load_state()["mutations"])
        self.assertEqual([], self._load_state()["comments"])
        self.assertIsNone(self._load_state()["pr"])

    def test_incomplete_project_readback_stays_pending_without_task_or_project_write(self):
        self.state["faults"]["project-fields-incomplete"] = True
        self._save_state()
        blocked = self.run_publisher()
        self.assertNotEqual(0, blocked.returncode, blocked.stderr)
        self.assertIn("cursor", blocked.stderr.lower())
        partial = self._load_state()
        self.assertEqual(1, len(self._effects("pr:create", partial)))
        self.assertEqual(1, partial["post_attempts"]["C1"])
        self.assertEqual("execution", self._task_state(partial)["workflow_phase"])
        self.assertEqual("execution", partial["project_values"]["Workflow Phase"])
        self.assertEqual("", partial["project_values"]["PR"])
        self.assertEqual([], self._effects("issue:body", partial))
        self.assertEqual([], self._effects("project:Workflow Phase", partial))
        self.assertEqual([], self._effects("project:PR", partial))

        self.state["faults"].pop("project-fields-incomplete")
        self._save_state()
        resumed = self.run_publisher()
        self.assertEqual(0, resumed.returncode, resumed.stderr)
        final = self._load_state()
        self._assert_complete(final)
        self.assertEqual(1, len(self._effects("pr:create", final)))
        self.assertEqual(1, len(self._effects("issue:body", final)))

    def test_issue_comment_permission_failure_blocks_then_resumes_without_guessing(self):
        self.state["faults"]["issue-comments-read"] = True
        self._save_state()
        blocked = self.run_publisher()
        self.assertNotEqual(0, blocked.returncode, blocked.stderr)
        self.assertIn("403", blocked.stderr)
        self.assertEqual([], self._load_state()["mutations"],
                         "unreadable C1 history must block before comment, push, or PR writes")
        self.assertEqual([], self._load_state()["comments"])
        self.assertIsNone(self._load_state()["pr"])

        self.state["faults"].pop("issue-comments-read")
        self._save_state()
        resumed = self.run_publisher()
        self.assertEqual(0, resumed.returncode, resumed.stderr)
        self._assert_complete(self._load_state())

    def test_project_permission_unavailable_stays_pending_before_binding_writes(self):
        self.state["project_viewer_can_update"] = False
        self._save_state()
        blocked = self.run_publisher()
        self.assertNotEqual(0, blocked.returncode, blocked.stderr)
        self.assertIn("permission", blocked.stderr.lower())
        partial = self._load_state()
        self.assertEqual(1, len(self._effects("pr:create", partial)))
        self.assertEqual(1, partial["post_attempts"]["C1"])
        self.assertEqual("execution", self._task_state(partial)["workflow_phase"])
        self.assertEqual("execution", partial["project_values"]["Workflow Phase"])
        self.assertEqual("", partial["project_values"]["PR"])
        self.assertEqual([], self._effects("issue:body", partial))
        self.assertEqual([], self._effects("project:Workflow Phase", partial))
        self.assertEqual([], self._effects("project:PR", partial))

        self.state["project_viewer_can_update"] = True
        self._save_state()
        resumed = self.run_publisher()
        self.assertEqual(0, resumed.returncode, resumed.stderr)
        final = self._load_state()
        self._assert_complete(final)
        self.assertEqual(1, len(self._effects("pr:create", final)))

    def test_t21_issue_complete_project_old_interruption_retries_missing_steps_only(self):
        self.state["faults"]["project:Workflow Phase"] = "interrupt-before"
        self._save_state()
        interrupted = self.run_publisher()
        self.assertNotEqual(0, interrupted.returncode,
                            "injected write boundary should remain non-success\n"
                            + interrupted.stdout + interrupted.stderr
                            + json.dumps(self._load_state(), sort_keys=True))
        self.assertIn("record-pr", interrupted.stderr)
        state = self._load_state()
        self.assertEqual({"status": "committed", "workflow_phase": "verification",
                          "pr_number": PR, "pr_url": PR_URL}, self._task_state(state))
        self.assertEqual("execution", state["project_values"]["Workflow Phase"])
        self.assertEqual("", state["project_values"]["PR"])
        self.assertEqual(1, len(self._effects("issue:body", state)))
        self.assertEqual([], self._effects("project:Workflow Phase", state))
        journal = self._journal()
        parent_action = next(action for action in journal["actions"]
                             if action["action_id"].startswith("record-pr:"))
        self.assertEqual("uncertain", parent_action["state"])
        issue_step = next(action for action in journal["actions"]
                          if action["action_id"].endswith(":issue"))
        self.assertEqual("observed", issue_step["state"])

        retried = self.run_publisher()
        self.assertEqual(0, retried.returncode, retried.stderr)
        final = self._load_state()
        self._assert_complete(final)
        self.assertEqual(1, len(self._effects("pr:create", final)))
        self.assertEqual(1, len(self._effects("issue:body", final)),
                         "recovery must observe the completed Issue step")
        self.assertEqual(1, len(self._effects("project:Workflow Phase", final)))
        self.assertEqual(1, len(self._effects("project:PR", final)))

    def test_issue_write_lost_response_is_confirmed_by_live_readback(self):
        self.state["faults"]["issue:body"] = "after"
        self._save_state()
        result = self.run_publisher()
        self.assertEqual(0, result.returncode, result.stderr)
        state = self._load_state()
        self._assert_complete(state)
        self.assertEqual(1, len(self._effects("issue:body", state)))
        self.assertEqual(1, len(self._effects("project:Workflow Phase", state)))
        self.assertEqual(1, len(self._effects("project:PR", state)))

    def test_crossed_issue_project_vector_is_rejected_before_owned_writes(self):
        self.state["faults"]["project:Workflow Phase"] = "interrupt-before"
        self._save_state()
        first = self.run_publisher()
        self.assertNotEqual(0, first.returncode)
        self.state = self._load_state()
        # This vector combines a later Project PR field with its old phase. It
        # is not one of the fixed Workflow Phase -> PR reachable prefixes.
        self.state["project_values"]["Workflow Phase"] = "execution"
        self.state["project_values"]["PR"] = PR_URL
        self._save_state()
        before = list(self.state["mutations"])
        retry = self.run_publisher()
        self.assertNotEqual(0, retry.returncode)
        self.assertIn("conflict", retry.stderr.lower())
        after = self._load_state()
        self.assertEqual(before, after["mutations"], "crossed vector must authorize zero new remote writes")

    def test_uncertain_c1_post_is_reconciled_without_duplicate_comment_or_pr(self):
        self.state["faults"]["comment:C1"] = "after"
        self._save_state()
        first = self.run_publisher()
        self.assertNotEqual(0, first.returncode)
        state = self._load_state()
        self.assertEqual(1, len([item for item in state["comments"]
                                 if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]))
        self.assertIsNone(state["pr"])

        second = self.run_publisher()
        self.assertEqual(0, second.returncode, second.stderr)
        state = self._load_state()
        self._assert_complete(state)
        self.assertEqual(1, state["post_attempts"]["C1"])
        self.assertEqual(1, len(self._effects("pr:create", state)))

    def test_edited_c1_readback_timestamp_stays_pending_without_pr_or_repost(self):
        self.state["faults"]["comment:C1:timestamp"] = "edited"
        self._save_state()
        first = self.run_publisher()
        self.assertNotEqual(0, first.returncode, first.stderr)
        state = self._load_state()
        c1 = [item for item in state["comments"]
              if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        self.assertEqual(1, len(c1))
        self.assertNotEqual(c1[0]["created_at"], c1[0]["updated_at"])
        self.assertIsNone(state["pr"])
        self.assertEqual(1, state["post_attempts"]["C1"])
        self.assertEqual([], self._effects("pr:create", state))

        retry = self.run_publisher()
        self.assertNotEqual(0, retry.returncode, retry.stderr)
        state = self._load_state()
        c1 = [item for item in state["comments"]
              if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        self.assertEqual(1, len(c1), "an edited immutable publication must not be reposted")
        self.assertEqual(1, state["post_attempts"]["C1"])
        self.assertIsNone(state["pr"])

    def test_locked_record_pr_reread_rejects_c1_edited_after_initial_validation(self):
        self.state["edit_c1_on_third_locked_comments_read"] = True
        self._save_state()

        first = self.run_publisher()
        state = self._load_state()
        c1 = [item for item in state["comments"]
              if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        protected_effects = [item for item in state["mutations"]
                             if item.get("effect") and
                             (item["kind"] == "issue:body"
                              or item["kind"].startswith("project:"))]
        first_evidence = {
            "returncode": first.returncode,
            "record_pr_c1_reads": state.get("record_pr_c1_reads"),
            "c1_count": len(c1),
            "c1_timestamps": [
                {key: item.get(key) for key in ("created_at", "updated_at")}
                for item in c1
            ],
            "pr_create_effects": len(self._effects("pr:create", state)),
            "protected_effects": protected_effects,
            "stdout": first.stdout,
            "stderr": first.stderr,
        }
        self.assertTrue(
            first.returncode != 0
            and state.get("record_pr_c1_reads") == 3
            and len(c1) == 1
            and c1[0].get("created_at") != c1[0].get("updated_at")
            and len(self._effects("pr:create", state)) == 1
            and protected_effects == [],
            "record-pr must revalidate exact C1 server timestamps on its locked "
            "fresh read before any Issue/Project binding write; observed "
            + json.dumps(first_evidence, sort_keys=True),
        )
        self.assertEqual(1, state["post_attempts"]["C1"], first_evidence)

        mutations_before_retry = list(state["mutations"])
        retry = self.run_publisher()
        retried = self._load_state()
        retry_evidence = {
            "returncode": retry.returncode,
            "c1_count": len([item for item in retried["comments"]
                             if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]),
            "post_attempts": retried["post_attempts"],
            "new_mutations": retried["mutations"][len(mutations_before_retry):],
            "stdout": retry.stdout,
            "stderr": retry.stderr,
        }
        self.assertTrue(
            retry.returncode != 0
            and retry_evidence["c1_count"] == 1
            and retried["post_attempts"]["C1"] == 1
            and retried["mutations"] == mutations_before_retry,
            "an edited immutable C1 must remain pending on replay without another "
            "comment or binding write; observed "
            + json.dumps(retry_evidence, sort_keys=True),
        )

    def test_uncertain_pr_create_is_reconciled_to_the_unique_live_draft(self):
        self.state["faults"]["pr:create"] = "after"
        self._save_state()
        result = self.run_publisher()
        # The publisher can resolve this immediately through its stable two
        # live PR reads, or leave it pending for a same-candidate retry.
        if result.returncode:
            result = self.run_publisher()
        self.assertEqual(0, result.returncode, result.stderr)
        state = self._load_state()
        self._assert_complete(state)
        self.assertEqual(1, len(self._effects("pr:create", state)))
        self.assertEqual(1, state["post_attempts"]["C1"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
