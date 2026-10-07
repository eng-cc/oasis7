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
publication_api = load_module("publisher_test_publication_api", HERE / "pr_projection_publication.py")
journal_api = load_module("publisher_test_journal_api", HERE / "pr_projection_journal.py")
packet_api = load_module("publisher_test_packet_api", HERE / "subagent-task-packet.py")


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
        if isinstance(value, dict) and value.get("type") == "repository":
            node["__typename"] = "ProjectV2ItemFieldRepositoryValue"
            node["repository"] = {"id": value["id"], "nameWithOwner": value["name_with_owner"]}
        elif name in state["single_select_fields"]:
            node["__typename"] = "ProjectV2ItemFieldSingleSelectValue"
            node["name"] = value
        else:
            node["__typename"] = "ProjectV2ItemFieldTextValue"
            node["text"] = value
        overrides = state.get("project_field_node_overrides", {})
        if isinstance(overrides, dict) and isinstance(overrides.get(name), dict):
            node.update(overrides[name])
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
        "issue_url": f"https://api.github.com/repos/{state['repository']}/issues/{state['issue']['number']}",
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
            "base": {"repo": {"full_name": state["repository"]}, "ref": state["target_ref"], "sha": state["target_oid"]},
        }
        state["mutations"][-1]["effect"] = True
    save()
    if fault == "after":
        raise SystemExit("injected lost PR create response")
    emit(state["pr_url"])

if args[0] == "api":
    endpoint = next((value for value in args[1:] if value == "user" or value == "graphql" or value.startswith("repos/")), "")
    if endpoint == "user":
        emit({"login": state["login"], "type": "User"})
    if endpoint == "graphql":
        flags = graphql_flags()
        query = flags.get("query", "")
        if "fields(first:" in query:
            definitions = []
            for name, definition in state['project_fields'].items():
                datatype = 'SINGLE_SELECT' if name in state['single_select_fields'] else 'TEXT'
                if name == 'Repository': datatype = 'REPOSITORY'
                definitions.append({'id': definition['id'], 'name': name, 'dataType': datatype,
                    'options': [{'name': label, 'id': oid} for label, oid in definition.get('options', {}).items()]})
            emit({'data': {'node': {'id': state['project_id'], 'fields': {'nodes': definitions,
                'pageInfo': {'hasNextPage': bool(state.get('faults', {}).get('project-fields-incomplete')),
                             'endCursor': None}}}}})
        if "nodes(ids:" in query:
            issue = state["issue"]
            project = {"id": state["project_id"], "number": state["project_number"],
                       "viewerCanUpdate": state.get("project_viewer_can_update", True),
                       "owner": {"login": state["project_owner"]}}
            item = {"id": state["project_item"], "isArchived": False, "project": project,
                    "content": {"__typename": "Issue", "number": issue["number"],
                                "url": issue["url"],
                                "repository": {"nameWithOwner": state["repository"]}},
                    "fieldValues": {"nodes": field_nodes(),
                                    "pageInfo": {"hasNextPage": False, "endCursor": None}}}
            payload = {"data": {
                "repository": {"issue": {"number": issue["number"], "url": issue["url"],
                                         "body": issue["body"], "state": issue["state"].upper(),
                                         "viewerCanUpdate": True}},
                "nodes": [item],
            }}
            emit(payload)
        if "repository(owner:" in query:
            issue = state["issue"]
            project = {"id": state["project_id"], "number": state["project_number"],
                       "viewerCanUpdate": state.get("project_viewer_can_update", True),
                       "owner": {"login": state["project_owner"]}}
            item = {"id": state["project_item"], "isArchived": False, "project": project}
            payload = {"data": {"repository": {"issue": {
                "id": "I_fixture", "number": issue["number"], "url": issue["url"],
                "state": issue["state"].upper(), "body": issue["body"],
                "projectItems": {"nodes": state.get("issue_project_items", [item]),
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
    if endpoint.startswith("repos/" + state['repository'] + '/commits/'):
        requested_ref = endpoint.rsplit('/', 1)[-1]
        if requested_ref != state['target_ref']:
            raise SystemExit('unknown protected target ref: ' + requested_ref)
        emit({'sha': state['target_oid']})
    if endpoint.startswith("repos/" + state["repository"] + "/collaborators/") and endpoint.endswith("/permission"):
        emit({"permission": state.get("collaborator_permission", "write"),
              "permissions": {"push": True, "admin": state.get("collaborator_permission") == "admin"},
              "user": {"login": endpoint.split("/collaborators/", 1)[1].split("/", 1)[0]}})
    if endpoint.startswith("repos/" + state["repository"] + "/issues/comments/"):
        comment_id = int(endpoint.rsplit("/", 1)[-1])
        found = [item for item in state["comments"] if item["id"] == comment_id]
        if len(found) != 1:
            raise SystemExit("Issue comment not found: " + str(comment_id))
        payload = json.loads(json.dumps(found[0]))
        variation = state.get("human_comment_readback_variation")
        if variation and ("<!-- oasis7-ci-publication-binding/v1 -->" in payload["body"]
                          or "<!-- oasis7-pm-evidence -->" in payload["body"]):
            payload["user"].update(id=42, node_id="U_fixture", avatar_url="https://avatars.example/u/42?v=" + ("GET" if variation == "avatar" else "LIST"))
            if variation == "wrong_actor": payload["user"]["login"] = "other-writer"
            if variation == "wrong_body": payload["body"] += "\nchanged server body"
        emit(payload)
    if endpoint == "repos/" + state["repository"] + f"/issues/{state['issue']['number']}":
        payload = issue_payload()
        payload["state"] = state["issue"]["state"]
        emit(payload)
    if endpoint.startswith("repos/" + state["repository"] + f"/issues/{state['issue']['number']}/comments"):
        if state.get("faults", {}).get("issue-comments-read"):
            save()
            raise SystemExit("403 injected incomplete Issue comment pagination")
        comments = state["comments"]
        if state.get("human_comment_readback_variation"):
            comments = json.loads(json.dumps(comments))
            for comment in comments:
                if ("<!-- oasis7-ci-publication-binding/v1 -->" in comment["body"]
                        or "<!-- oasis7-pm-evidence -->" in comment["body"]):
                    comment["user"].update(id=42, node_id="U_fixture", avatar_url="https://avatars.example/u/42?v=LIST")
            if state["human_comment_readback_variation"] == "duplicate":
                matching = [c for c in comments if "<!-- oasis7-ci-publication-binding/v1 -->" in c["body"]]
                if matching:
                    duplicate = json.loads(json.dumps(matching[0]))
                    duplicate["id"] += 10000
                    comments.append(duplicate)
        if ((state.get("edit_c1_on_locked_comments_read") is True
                or state.get("edit_c1_on_third_locked_comments_read") is True)
                and state.get("pr") is not None
                and not any(item.get("kind") == "issue:body" and item.get("effect")
                            for item in state.get("mutations", []))):
            state["record_pr_c1_reads"] = state.get("record_pr_c1_reads", 0) + 1
            target_read = (3 if state.get("edit_c1_on_third_locked_comments_read") is True else 4)
            if state["record_pr_c1_reads"] == target_read:
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
        shutil.copy2(REPO_ROOT / "scripts/pm/fixtures/github_api_test_adapter.py",
                     self.repo / "scripts/pm/github_api.py")
        (self.repo / "scripts").mkdir(exist_ok=True)
        for relative in ("scripts/plan-rust-required-scope.py",
                         "scripts/ci-required-scope.v2.json", "scripts/ci-tests.sh"):
            destination = self.repo / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO_ROOT / relative, destination)
        roles = self.repo / ".agents/roles"
        roles.mkdir(parents=True)
        for role in ("runtime_engineer", "repository_health_engineer", "qa_engineer"):
            shutil.copy2(REPO_ROOT / f".agents/roles/{role}.md", roles / f"{role}.md")
        shutil.copy2(REPO_ROOT / "AGENTS.md", self.repo / "AGENTS.md")
        source_of_truth = self.repo / "doc/engineering/workflow/source-of-truth.md"
        source_of_truth.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO_ROOT / "doc/engineering/workflow/source-of-truth.md", source_of_truth)
        (self.repo / ".gitignore").write_text(
            ".pm/\n__pycache__/\n*.pyc\n",
            encoding="utf-8",
        )
        (self.repo / "doc").mkdir(exist_ok=True)
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
            "repository": REPOSITORY, "repository_id": 701, "target_oid": self.target_oid,
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

    def publisher_command(self, helper: Path | None = None,
                          resume_action_id: str | None = None) -> list[str]:
        command = [
            sys.executable, str(self.task_root / "scripts/pm/pr_projection_publish.py"),
            "--worktree", str(self.task_root), "--repo", REPOSITORY,
            "--issue-number", str(ISSUE), "--task-uid", UID, "--remote", "origin",
            "--source-ref", "task/publication-fixture", "--target-ref", "main",
            "--source-head", self.source_head, "--target-oid", self.target_oid,
            "--projection", str(self.projection_path), "--body-file", str(self.body_path),
            "--task-helper", str(helper or self.task_root / "scripts/pm/github-project-task.py"),
            "--json",
        ]
        if resume_action_id is not None:
            command.extend(["--resume-action-id", resume_action_id])
        return command

    def run_publisher(self, *, helper: Path | None = None,
                      resume_action_id: str | None = None, human_reconcile=False):
        command = self.publisher_command(helper, resume_action_id)
        if human_reconcile:
            command.extend(["--human-reconcile", "--maintenance-authority-comment-id", "9002"])
        return subprocess.run(command, text=True,
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
        c1 = [publication_api.parse_publication_comment(item["body"])
              for item in state["comments"]
              if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        current_c1 = [item for item in c1 if item["source_head_oid"] == self.source_head]
        reciprocal = [item for item in state["comments"] if "<!-- oasis7-ci-publication-binding/v1 -->" in item["body"]]
        self.assertEqual(1, len(current_c1), "one immutable current C1 intent must exist")
        self.assertEqual(1, len(reciprocal), "one reciprocal C1 binding must exist")

    def _journal(self):
        journals = list((self.repo / ".git/oasis7/pr-publication").glob("*/*/journal.json"))
        self.assertEqual(1, len(journals), f"expected one exact publication journal, found {journals}")
        return json.loads(journals[0].read_text(encoding="utf-8"))

    def _pending_resume_action_id(self) -> str:
        return "task-intent:" + self._journal()["identity"]["publication_id"]

    @staticmethod
    def _canonical(value):
        return json.dumps(value, ensure_ascii=False, sort_keys=True,
                          separators=(",", ":")).encode("utf-8")

    @classmethod
    def _sha(cls, value) -> str:
        raw = value if isinstance(value, bytes) else cls._canonical(value)
        return hashlib.sha256(raw).hexdigest()

    def _common_git_dir(self) -> Path:
        return Path(git(self.task_root, "rev-parse", "--path-format=absolute",
                        "--git-common-dir")).resolve()

    def _publication_journal_path(self, publication: dict) -> Path:
        path, _ = journal_api.publication_paths(
            self._common_git_dir(), publication["repository"],
            publication["source_ref"], publication["publication_id"],
        )
        return path

    def _append_comment(self, comment_id: int, body: str) -> None:
        self.state.setdefault("comments", []).append({
            "id": comment_id,
            "body": body,
            "html_url": f"{self.state['issue']['url']}#issuecomment-{comment_id}",
            "url": f"{self.state['issue']['url']}#issuecomment-{comment_id}",
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE}",
            "created_at": f"2026-10-02T01:{comment_id % 60:02d}:00Z",
            "updated_at": f"2026-10-02T01:{comment_id % 60:02d}:00Z",
            "user": {"login": LOGIN, "type": "User"},
            "author_association": "MEMBER",
        })

    def _task_record(self) -> dict:
        return json.loads((self.task_root / ".pm/github-project-sync/tasks.json").read_text(
            encoding="utf-8"))["tasks"][UID]

    def _build_predecessor_publication(self) -> tuple[dict, dict]:
        projection = json.loads(self.projection_path.read_text(encoding="utf-8"))
        predecessor = publication_api.build_task_publication(
            repository=REPOSITORY,
            repository_id=self.state["repository_id"],
            task_uid=UID,
            bootstrap_epoch=None,
            source_repository_id=self.state["repository_id"],
            source_ref="task/publication-fixture",
            target_ref="main",
            source_head_oid=self.target_oid,
            source_scope_oid=self.target_oid,
            planner_authority_oid=self.target_oid,
            planner_config_sha256=projection["planner_config_sha256"],
            policy_digest=projection["planner_digest"],
            projection_digest=projection["projection_digest"],
        )
        identity = {
            "repository": REPOSITORY,
            "branch": predecessor["source_ref"],
            "publication_id": predecessor["publication_id"],
            "task_uid": UID,
            "source_head_oid": predecessor["source_head_oid"],
            "scope_base_oid": predecessor["source_scope_oid"],
            "projection_digest": predecessor["projection_digest"],
        }
        path, lock = journal_api.publication_paths(
            self._common_git_dir(), REPOSITORY, predecessor["source_ref"],
            predecessor["publication_id"],
        )
        local = journal_api.PublicationJournal(
            path, lock, identity, common_dir=self._common_git_dir(),
            canonical_worktree=self.task_root,
        )
        with local.locked():
            action_id = "record-pr:" + predecessor["publication_id"]
            local.intent(
                action_id,
                "record_pr",
                {"publication_id": predecessor["publication_id"],
                 "task_uid": UID, "pr_number": PR},
            )
            local.uncertain(
                "record-pr:" + predecessor["publication_id"],
                "NETWORK_UNCERTAIN",
            )
        self._append_comment(1001, publication_api.publication_comment(predecessor))
        return predecessor, {"publication": predecessor, "journal_path": path}

    def _publication_action(self, publication: dict, journal_path: Path,
                            comment_id: int) -> dict:
        raw = journal_path.read_bytes()
        return {
            "publication_id": publication["publication_id"],
            "action_id": "record-pr:" + publication["publication_id"],
            "journal_sha256": self._sha(raw),
            "H": publication["source_head_oid"],
            "B": publication["planner_authority_oid"],
            "S": publication["source_scope_oid"],
            "D": publication["projection_digest"],
            "intent_comment_id": comment_id,
            "intent_body_sha256": self._sha(
                publication_api.publication_comment(publication).encode("utf-8")),
        }

    def _helper_manifest(self) -> tuple[list[dict], str]:
        rows = []
        for line in git(self.task_root, "ls-tree", "-r", "HEAD", "--", "scripts/pm").splitlines():
            metadata, path = line.split("\t", 1)
            mode, kind, oid = metadata.split()
            self.assertEqual("blob", kind)
            rows.append({"path": path, "mode": mode, "blob_oid": oid})
        rows.sort(key=lambda item: item["path"].encode("ascii"))
        return rows, self._sha(rows)

    def _seed_recovery_reviews(self, closure_sha: str) -> tuple[Path, list[dict]]:
        review = self.task_root / ".pm/scratch" / UID / "recovery-role-review"
        packet_dir = self.task_root / ".pm/scratch" / UID / "slice-packets"
        review.mkdir(parents=True, exist_ok=True)
        packet_dir.mkdir(parents=True, exist_ok=True)
        task = self._task_record()
        roles = ("runtime_engineer", "repository_health_engineer", "qa_engineer")
        ledger = []
        returns = []
        for index, role in enumerate(roles, 1):
            slice_id = f"00000000-0000-4000-8000-{index:012d}"
            packet = {
                "schema": "oasis7-subagent-task-packet/v1",
                "identity": {
                    "task_uid": UID,
                    "repository": REPOSITORY,
                    "project_item_id": PROJECT_ITEM,
                    "task_status": "committed",
                    "issue_url": task["issue_url"],
                    "worktree": str(self.task_root),
                    "branch": "task/publication-fixture",
                    "head": self.source_head,
                    "base_ref": self.target_oid,
                    "base_sha": self.target_oid,
                    "base_binding": "immutable_oid",
                    "packet_producer": "tpm",
                },
                "slice": {
                    "slice_id": slice_id,
                    "role": role,
                    "slice_type": "professional_review",
                    "owner_role": task["owner_role"],
                    "integration_owner": "tpm",
                    "integration_order": "REC bounded review then exact action binding",
                    "context_delivery_mode": "minimal_head_bound_task_packet",
                    "intended_model_configuration": "inherit current parent selection",
                    "actual_dispatched_model_reasoning": "fixture unobserved",
                    "actual_runtime_evidence_reason": "fixture adapter inactive",
                    "role_activation": "message_assigned_adapter_inactive",
                    "write_scope": "scratch-only bounded helper review",
                    "return_contract": "exact immutable helper closure verdict",
                    "validation_command": "human-operated local provenance",
                    "formal_sink": task["issue_url"],
                    "full_history_escalation_reason": "",
                },
                "context": {
                    "user_intent": "existing approved same-PR metadata recovery",
                    "work_item": "REC bounded helper review",
                    "non_goals": "no CI/review/Ready authority",
                    "acceptance_target": "exact immutable helper closure",
                    "evidence_summary": "isolated authentic Git/helper/role artifact fixture",
                    "collaboration_boundary": "fixture scratch only",
                    "governance_refs": [
                        "AGENTS.md",
                        "doc/engineering/workflow/source-of-truth.md",
                        f".agents/roles/{role}.md",
                    ],
                    "scoped_refs": [
                        "scripts/pm/github-project-task.py",
                        "scripts/pm/pr_projection_publish.py",
                    ],
                },
            }
            packet["packet_digest"] = packet_api.canonical_digest(packet)
            packet_path = packet_dir / f"{slice_id}.json"
            packet_path.write_bytes(self._canonical(packet) + b"\n")
            returned = {
                "task_uid": UID,
                "role": role,
                "slice_id": slice_id,
                "head": self.source_head,
                "status": "completed",
                "scope_verdict": "approved",
                "risk_verdict": "approved",
                "findings": "no_findings",
                "residual_risk": "isolated fixture",
                "helper_source_oid": self.source_head,
                "helper_closure_sha256": closure_sha,
                "activation": "message-assigned",
                "context_delivery": "minimal_head_bound_task_packet",
                "actual_runtime": "fixture inherited unobserved",
            }
            return_path = review / f"{role}.return.json"
            return_path.write_bytes(self._canonical(returned) + b"\n")
            return_sha = self._sha(return_path.read_bytes())
            ledger.append(dict(returned, artifact_digest=return_sha,
                               artifacts=[str(return_path.relative_to(self.task_root))]))
            returns.append({
                "role": role,
                "slice_id": slice_id,
                "packet_sha256": self._sha(packet_path.read_bytes()),
                "source_head_oid": self.source_head,
                "return_path": str(return_path.relative_to(self.task_root)),
                "return_sha256": return_sha,
                "verdict": "approved",
            })
        ledger_path = review / "ledger.jsonl"
        ledger_path.write_bytes(b"".join(self._canonical(row) + b"\n" for row in ledger))
        return ledger_path, returns

    def _seed_recovery_admission(self, *, current_journal_before: dict | None = None) -> str:
        self.state = self._load_state()
        current_c1 = [item for item in self.state["comments"]
                      if "<!-- oasis7-ci-publication/v1 -->" in item["body"]]
        self.assertEqual(1, len(current_c1))
        current_publication = publication_api.parse_publication_comment(current_c1[0]["body"])
        current_journal = self._publication_journal_path(current_publication)
        if current_journal_before is not None:
            self.assertEqual(current_journal_before, json.loads(current_journal.read_text(
                encoding="utf-8")))
        _, predecessor = self._build_predecessor_publication()
        manifest, closure_sha = self._helper_manifest()
        ledger_path, returns = self._seed_recovery_reviews(closure_sha)
        frozen_red_digest = self._sha(b"synthetic isolated frozen RED patch")
        fixture_identity = f"Task UID {UID}; Issue {self.state['issue']['url']}; same PR {PR_URL}"

        def plan_row(step, acceptance, dependencies, command, evidence, writes,
                     exclusions, role_slices):
            return {
                "step_id": step,
                "acceptance_refs": acceptance,
                "dependencies": dependencies,
                "verification_command": command,
                "verification_evidence": evidence,
                "write_scope": writes,
                "out_of_scope": exclusions,
                "required_role_slices": role_slices,
            }

        rows = [
            plan_row("REC-SPEC", "fixture Task original acceptance plus same-PR publication recovery",
                     "existing user approval; same fixture PR; immutable historical journal",
                     "workflow documentation contracts and runtime/QA API closure",
                     "source-only contract tests and bounded runtime/QA returns",
                     "doc/engineering/workflow/source-of-truth.md only by repository_health_engineer",
                     "gate waiver; activation; raw task/cache/Project patches; another PR",
                     "repository_health_engineer author; runtime_engineer and qa_engineer API review"),
            plan_row("REC-RED", "partial Issue/Project write recovery exact journal/UID/H/PR and complete poststate",
                     "REC-SPEC closure", "focused genuine github-project-task/publication RED tests",
                     "immutable test patch digest and actual exit logs; independent QA",
                     "scripts/pm/github-project-task.test.sh and scripts/pm/pr_projection_publication.test.py by runtime_engineer",
                     "production helpers before admitted GREEN; loosened assertions",
                     "runtime_engineer test author; qa_engineer independent RED"),
            plan_row("REC-GREEN", "authentic journal-constrained partial-poststate reconciliation; reject drift and missing readback",
                     "actual RED confirmed plus TPM GREEN admission",
                     "focused REC cases and ordinary task/publication regressions",
                     "runtime and independent QA GREEN logs/digests; immutable reviewed helper closure",
                     "scripts/pm/github-project-task.py; scripts/pm/pr_projection_publish.py only if required for journal transport/recovery",
                     "other helpers; activation; forged authority; historical journal mutation",
                     "runtime_engineer implementation; qa_engineer independent; repository_health_engineer conformance"),
            plan_row("REC-RESTORE", "fresh current-H unique fixture PR reciprocal binding through canonical execution authority",
                     "REC-SPEC/RED/GREEN closure and executable-route confirmation",
                     "canonical selected-task refresh/audit/publication resume and live four-surface readback",
                     "complete exact poststate; unique reciprocal binding; current action observed only after final readback",
                     "canonical tasktruth/journal through approved helpers only",
                     "raw cache/body/Project patches; rollback; CI before binding; second PR",
                     "repository_health_engineer authority review; runtime_engineer route; TPM execution"),
            plan_row("REC-DELIVER", "fixture original acceptance and bounded recovery delivered in same Task/same PR",
                     "REC-RESTORE complete",
                     "fresh freeze, involved-role review, required CI and canonical merge/finalization",
                     "current source/base/scope/projection roles, CI, receipts and cleanup",
                     "sameTask samePR only; original two CLI paths retained, total seven approved paths max",
                     "second PR; historical receipt reuse; activation; unrelated delivery completion",
                     "runtime_engineer/repository_health_engineer/qa_engineer formal review; TPM integration"),
        ]
        scope_bodies = {
            7101: "WP2 bounded dormant CLI artifact output prerequisite\n" + fixture_identity
                  + "\nPlan-Gap entries beforewrites:\n"
                  + "CLI-RED acceptance_refs original artifact output acceptance; dependencies actual focused CLI RED; "
                  + "verification_command focused main-output unittest; verification_evidence actual exit/log/test patch digest; "
                  + "write_scope scripts/pm/ci-reuse-validation.test.py ONLY; out_of_scope production helpers/schema/activation; "
                  + "required_role_slices runtime_engineer author and independent qa_engineer.\n"
                  + "CLI-GREEN acceptance_refs verified payload mode to artifact output while disabled; dependencies immutable RED plus QA; "
                  + "verification_command same focused cases and producer/contract/readback negatives; verification_evidence actual GREEN logs/diff; "
                  + "write_scope scripts/pm/ci-reuse-validation.py ONLY; out_of_scope authority/schema/activation; "
                  + "required_role_slices runtime_engineer implementation and independent qa_engineer.",
            7102: "CLI-RED completed and independently confirmed. Frozen synthetic test digest "
                  + frozen_red_digest + ". CLI-GREEN admission: runtime_engineer may modify ONLY "
                  + "scripts/pm/ci-reuse-validation.py; original tests remain immutable. No other source, policy, "
                  + "schema, commits, dispatch or activation. " + fixture_identity,
            7103: "User authority intake: explicit same-PR source-first recovery; existing original acceptance retained; "
                  + "original two CLI paths from synthetic history retained; total seven approved paths max. No gate exceptions.\n"
                  + fixture_identity + "\nPlan-Gap Evidence:\n" + json.dumps(rows, indent=2),
            7104: "REC-SPEC COMPLETE with bounded runtime_engineer and independent qa_engineer API closure. "
                  + "REC-RED admission: ONLY scripts/pm/github-project-task.test.sh and "
                  + "scripts/pm/pr_projection_publication.test.py. Frozen synthetic RED digest "
                  + frozen_red_digest + "; no production helpers until a new TPM GREEN admission. "
                  + "Preserve exact final readback, pending uncertainty and immutable historical journal; no activation. "
                  + fixture_identity,
            7105: "REC-RED COMPLETE / GREEN ADMITTED: frozen synthetic test patch " + frozen_red_digest
                  + "; actual focused RED and independent QA confirmed. Runtime may implement ONLY "
                  + "scripts/pm/github-project-task.py and scripts/pm/pr_projection_publish.py "
                  + "(publisher only when journal transport/recovery requires it). Frozen tests immutable; exact independent "
                  + "final Issue/Project/PR/unique binding readback before current action observed; retain pending state on "
                  + "uncertainty; no historical journal mutation, activation or other helper scope. " + fixture_identity,
        }
        scope = []
        for comment_id, body in scope_bodies.items():
            self._append_comment(comment_id, body)
            scope.append({
                "comment_id": comment_id,
                "comment_url": f"{self.state['issue']['url']}#issuecomment-{comment_id}",
                "body_sha256": self._sha(body.encode("utf-8")),
                "author_login": LOGIN,
            })
        issue_task = self._task_state(self.state)
        unrelated_issue = {
            "task_uid": UID,
            "owner_role": self._task_record()["owner_role"],
            "module": "engineering",
            "priority": "P2",
            "worktree_hint": str(self.task_root),
            "primary_package": None,
        }
        unrelated_project = {key: value for key, value in self.state["project_values"].items()
                             if key not in {"Status", "PM Status", "Workflow Phase", "PR"}}
        admission = {
            "schema": "oasis7-publication-recovery-admission/v1",
            "operation": "record_pr_publication_recovery",
            "identity": {
                "repository": REPOSITORY,
                "task_uid": UID,
                "issue_number": ISSUE,
                "issue_url": self.state["issue"]["url"],
                "pr_number": PR,
                "pr_url": PR_URL,
                "project_id": PROJECT_ID,
                "project_item_id": PROJECT_ITEM,
                "canonical_worktree": str(self.task_root),
                "source_ref": "task/publication-fixture",
                "target_ref": "main",
            },
            "scope_evidence": scope,
            "current_action": self._publication_action(current_publication, current_journal, current_c1[0]["id"]),
            "predecessor": self._publication_action(predecessor["publication"], predecessor["journal_path"], 1001),
            "helper_review": {
                "helper_source_oid": self.source_head,
                "helper_closure_sha256": closure_sha,
                "closure_manifest": manifest,
                "ledger_path": str(ledger_path.relative_to(self.task_root)),
                "ledger_sha256": self._sha(ledger_path.read_bytes()),
                "role_returns": returns,
            },
            "unrelated_snapshot": {
                "issue_sha256": self._sha(self._canonical(unrelated_issue)),
                "project_sha256": self._sha(self._canonical(unrelated_project)),
            },
        }
        self.assertEqual("verification", issue_task["workflow_phase"])
        body = ("<!-- oasis7-publication-recovery-admission/v1 -->\n```json\n"
                + self._canonical(admission).decode("utf-8") + "\n```\n")
        self._append_comment(9001, body)
        self._save_state()
        return "task-intent:" + current_publication["publication_id"]

    def test_actual_publisher_transports_full_projection_to_workflow_materializer(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stdout + first.stderr)
        state = self._load_state()
        workflow = (REPO_ROOT / '.github/workflows/rust.yml').read_text()
        start = workflow.index('      - id: impact')
        run = workflow.index('        run: |\n', start) + len('        run: |\n')
        end = workflow.index('      - id: integration', run)
        script = '\n'.join(line[10:] if line.startswith('          ') else line
                           for line in workflow[run:end].splitlines())
        runner = self.tmp / 'workflow-runner'
        runner.mkdir()
        output = runner / 'output'
        materialized = subprocess.run(['bash', '-euo', 'pipefail', '-c', script],
            cwd=self.task_root, env={**self.env, 'PR_BODY': state['pr']['body'],
                'RUNNER_TEMP': str(runner), 'GITHUB_OUTPUT': str(output)},
            text=True, capture_output=True)
        self.assertEqual(0, materialized.returncode, materialized.stdout + materialized.stderr)
        self.assertIn('enabled=true', output.read_text(),
                      'actual publisher C1 body must transport verified full projection DATA')
        self.assertEqual(json.loads(self.projection_path.read_text()),
                         json.loads((runner / 'impact-projection.json').read_text()))

    def seed_human_maintenance_authority(self):
        maintenance = load_module('fixture_maintenance', HERE / 'workflow_maintenance.py')
        self.state = self._load_state()
        self.state['collaborator_permission'] = 'admin'
        scope = dict(repository=REPOSITORY, task_uid=UID, issue_number=ISSUE, pr_number=PR,
                     purpose='candidate-tool-verification',
                     allowed_write_paths=['scripts/pm/pr_projection_publication.py',
                         'scripts/pm/pr_projection_publish.py', 'scripts/pm/github-project-task.py'] + self.changed_paths,
                     allowed_tool_paths=list(dict.fromkeys(list(maintenance.TOOL_PATHS) + [
                         'scripts/pm/github-project-task.py', 'scripts/pm/pr_projection_publish.py',
                         'scripts/pm/pr_projection_record_pr.py', 'scripts/pm/github-project-sync.py',
                         'scripts/pm/github_api.py', 'scripts/pm/task_complete_claim.py',
                         'scripts/pm/workflow-durable-store.py', 'scripts/pm/pr_projection_transition.py'])))
        self._append_comment(9002, maintenance.MARKER + '\n```json\n' + json.dumps(scope) + '\n```')
        self._save_state()

    def test_human_completed_vector_replay_needs_no_fabricated_recovery_admission(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        interrupted = self.run_publisher()
        self.assertNotEqual(0, interrupted.returncode)
        live = self._load_state()
        self.assertEqual(PR, self._task_state(live)['pr_number'])
        self.assertEqual('verification', live['project_values']['Workflow Phase'])
        self.assertEqual('', live['project_values']['PR'])
        # Human completed the exact missing owned field; fresh reads still verify it.
        live['project_values']['PR'] = PR_URL
        self.state = live
        self._save_state()
        self.seed_human_maintenance_authority()
        mutations = list(live['mutations'])
        journal_path = next((self.repo / ".git/oasis7/pr-publication").glob("*/*/journal.json"))
        old_journal = journal_path.read_bytes()
        retried = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, retried.returncode, retried.stdout + retried.stderr)
        self.assertEqual(old_journal, journal_path.read_bytes(), "human observation preserves original uncertain journal")
        final = self._load_state()
        self.assertEqual(mutations, [event for event in final['mutations']
                         if event['kind'] not in ('comment:binding', 'comment:evidence')],
                         'completed remote metadata must not be rewritten')
        self.assertEqual(1, len(self._effects('comment:binding', final)))
        self.assertEqual(1, len(self._effects('comment:evidence', final)))
        self.assertEqual(PR_URL, final['project_values']['PR'])
        self.assertEqual(PR, self._task_state(final)['pr_number'])
        again = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, again.returncode, again.stdout + again.stderr)
        self.assertEqual(final['mutations'], self._load_state()['mutations'])

    def test_human_reconcile_accepts_same_actor_with_changed_avatar_readback(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        self.assertNotEqual(0, self.run_publisher().returncode)
        self.seed_human_maintenance_authority()
        self.state = self._load_state()
        self.state['project_values']['PR'] = PR_URL
        self.state['human_comment_readback_variation'] = 'avatar'
        self._save_state()
        journal_path = next((self.repo / '.git/oasis7/pr-publication').glob('*/*/journal.json'))
        old_journal = journal_path.read_bytes()
        result = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        final = self._load_state()
        self.assertEqual(1, len(self._effects('comment:binding', final)))
        self.assertEqual(1, len(self._effects('comment:evidence', final)))
        self.assertEqual(old_journal, journal_path.read_bytes())
        again = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, again.returncode, again.stdout + again.stderr)
        self.assertEqual(final['mutations'], self._load_state()['mutations'])

    def test_human_reconcile_rejects_wrong_actor_body_and_duplicate_readbacks(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        self.assertNotEqual(0, self.run_publisher().returncode)
        self.seed_human_maintenance_authority()
        baseline = self._load_state()
        baseline['project_values']['PR'] = PR_URL
        journal_path = next((self.repo / '.git/oasis7/pr-publication').glob('*/*/journal.json'))
        old_journal = journal_path.read_bytes()
        for case in ('wrong_actor', 'wrong_body', 'duplicate'):
            self.state = json.loads(json.dumps(baseline))
            self.state['human_comment_readback_variation'] = case
            self._save_state()
            result = self.run_publisher(human_reconcile=True)
            with self.subTest(case=case):
                self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
                expected = ('human exact comment readback is pending' if case == 'duplicate'
                            else 'human exact server comment differs')
                self.assertIn(expected, result.stderr, 'publisher must preserve actual child terminal cause')
                final = self._load_state()
                self.assertEqual(1, len(self._effects('comment:binding', final)))
                self.assertEqual(0, len(self._effects('comment:evidence', final)))
                self.assertEqual(old_journal, journal_path.read_bytes())
                retry = self.run_publisher(human_reconcile=True)
                self.assertNotEqual(0, retry.returncode)
                self.assertEqual(final['mutations'], self._load_state()['mutations'])

    def test_human_reconcile_writes_only_missing_project_pr_once(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        interrupted = self.run_publisher()
        self.assertNotEqual(0, interrupted.returncode)
        self.seed_human_maintenance_authority()
        before = self._load_state()
        journal_path = next((self.repo / '.git/oasis7/pr-publication').glob('*/*/journal.json'))
        old_journal = journal_path.read_bytes()
        recovered = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, recovered.returncode, recovered.stdout + recovered.stderr)
        final = self._load_state()
        self.assertEqual(PR_URL, final['project_values']['PR'])
        self.assertEqual(len(self._effects('issue:body', before)), len(self._effects('issue:body', final)))
        self.assertEqual(len(self._effects('project:Workflow Phase', before)),
                         len(self._effects('project:Workflow Phase', final)))
        self.assertEqual(1, len(self._effects('project:PR', final)))
        self.assertEqual(old_journal, journal_path.read_bytes())
        again = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, again.returncode, again.stdout + again.stderr)
        self.assertEqual(final['mutations'], self._load_state()['mutations'])

    def test_human_reconcile_rejects_permission_hold_and_conflicting_vector_without_writes(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        self.assertNotEqual(0, self.run_publisher().returncode)
        self.seed_human_maintenance_authority()
        baseline = self._load_state()
        mutations = list(baseline['mutations'])
        # Restore the unchanged server baseline per case; rejected calls may add read logs only.
        for case in ('permission', 'hold', 'conflicting_pr', 'incomplete_readback'):
            self.state = json.loads(json.dumps(baseline))
            if case == 'permission': self.state['collaborator_permission'] = 'write'
            elif case == 'hold':
                self.state['issue']['body'] += '\n- merge_hold_active: `true`\n'
            elif case == 'conflicting_pr': self.state['project_values']['PR'] = PR_URL + '0'
            else: self.state['faults']['project-fields-incomplete'] = True
            self._save_state()
            result = self.run_publisher(human_reconcile=True)
            with self.subTest(case=case):
                self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
                self.assertEqual(mutations, self._load_state()['mutations'])

    def test_human_reconcile_accepts_live_target_advance_preserving_historical_c1_authority(self):
        self.state['faults']['project:Workflow Phase'] = 'interrupt-after'
        self._save_state()
        self.assertNotEqual(0, self.run_publisher().returncode)
        self.seed_human_maintenance_authority()
        before = self._load_state()
        c1_before = [c['body'] for c in before['comments']
                     if '<!-- oasis7-ci-publication/v1 -->' in c['body']]
        self.assertEqual(1, len(c1_before))
        subject = publication_api.parse_publication_comment(c1_before[0])
        self.assertEqual(self.target_oid, subject['planner_authority_oid'])
        self.assertEqual(self.source_head, subject['source_head_oid'])
        historical_config = git(self.repo, 'show',
            self.target_oid + ':scripts/ci-required-scope.v2.json')
        # Actual protected main advances independently of the exact source subject.
        (self.repo / 'doc/main-advance.md').write_text('independent protected target change\n')
        git(self.repo, 'add', 'doc/main-advance.md')
        git(self.repo, 'commit', '-m', 'advance protected target independently')
        advanced_target = git(self.repo, 'rev-parse', 'HEAD')
        self.assertNotEqual(self.target_oid, advanced_target)
        self.assertEqual(historical_config, git(self.repo, 'show',
            advanced_target + ':scripts/ci-required-scope.v2.json'))
        before['target_oid'] = advanced_target
        before['pr']['base']['sha'] = advanced_target
        self.state = before
        self._save_state()
        journal_path = next((self.repo / '.git/oasis7/pr-publication').glob('*/*/journal.json'))
        journal_before = journal_path.read_bytes()
        result = self.run_publisher(human_reconcile=True)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        final = self._load_state()
        self.assertEqual(c1_before, [c['body'] for c in final['comments']
                                   if '<!-- oasis7-ci-publication/v1 -->' in c['body']])
        self.assertEqual(self.source_head, final['pr']['head']['sha'])
        self.assertEqual(advanced_target, final['pr']['base']['sha'])
        self.assertEqual(PR_URL, final['project_values']['PR'])
        self.assertEqual(journal_before, journal_path.read_bytes())
        self.assertEqual(1, len(self._effects('project:PR', final)))

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

    def test_completed_replay_requires_fresh_live_project_vector(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        baseline_state = self._load_state()
        self._assert_complete(baseline_state)
        baseline_mutations = list(baseline_state["mutations"])
        baseline_comments = list(baseline_state["comments"])

        mapping_path = self.task_root / ".pm/github-project-sync/tasks.json"
        mapping_bytes = mapping_path.read_bytes()
        journal_paths = list((self.repo / ".git/oasis7/pr-publication").glob("*/*/journal.json"))
        self.assertEqual(1, len(journal_paths), f"expected one publication journal, found {journal_paths}")
        journal_bytes = journal_paths[0].read_bytes()

        # The local Task cache and Issue still claim verification. Only the
        # authoritative live Project Workflow Phase is changed, so a completed
        # replay must not accept the cache as a substitute for Project truth.
        self.state["project_values"]["Workflow Phase"] = "execution"
        self._save_state()
        result = self.run_publisher()
        after = self._load_state()

        self.assertNotEqual(
            0, result.returncode,
            "completed replay must reject a fresh live Project vector that differs from Task/PR state; "
            f"stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertEqual(baseline_mutations, after["mutations"],
                         "Project drift must not authorize publication writes")
        self.assertEqual(baseline_comments, after["comments"],
                         "Project drift must not create comments or another C1 record")
        self.assertEqual("execution", after["project_values"]["Workflow Phase"],
                         "the test drift must remain visible after the rejected replay")
        self.assertEqual(mapping_bytes, mapping_path.read_bytes(),
                         "a rejected completed replay must not rewrite the Task cache")
        self.assertEqual(journal_bytes, journal_paths[0].read_bytes(),
                         "a rejected completed replay must not rewrite publication history")

    def test_completed_replay_rejects_live_status_drift(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        baseline = self._load_state()
        self._assert_complete(baseline)
        mutations = list(baseline["mutations"])
        self.state["project_values"]["Status"] = "Done"
        self._save_state()
        result = self.run_publisher()
        after = self._load_state()
        self.assertNotEqual(
            0, result.returncode,
            "completed replay must reject Status drift in the live Project vector; "
            f"stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertEqual(mutations, after["mutations"], "Status drift must not authorize writes")
        self.assertEqual("Done", after["project_values"]["Status"], "test drift must remain visible")

    def test_completed_replay_rejects_detached_live_issue_project_item(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        baseline = self._load_state()
        self._assert_complete(baseline)
        mutations = list(baseline["mutations"])
        self.state["issue_project_items"] = []
        self._save_state()
        result = self.run_publisher()
        after = self._load_state()
        self.assertNotEqual(
            0, result.returncode,
            "completed replay must reject a cached Project item no longer linked from the live Task Issue; "
            f"stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertEqual(mutations, after["mutations"], "detached item must not authorize writes")
        self.assertEqual([], after["issue_project_items"], "test detachment must remain visible")


    def test_completed_replay_accepts_readable_project_without_update_permission(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        baseline = self._load_state()
        self._assert_complete(baseline)
        mutations = list(baseline["mutations"])
        self.state["project_viewer_can_update"] = False
        self._save_state()
        result = self.run_publisher()
        after = self._load_state()
        self.assertEqual(0, result.returncode, result.stderr)
        self._assert_complete(after)
        self.assertEqual(mutations, after["mutations"], "read-only replay must remain a no-write operation")

    def _completed_replay_no_write_snapshot(self):
        first = self.run_publisher()
        self.assertEqual(0, first.returncode, first.stderr)
        state = self._load_state()
        self._assert_complete(state)
        journal_paths = list((self.repo / ".git/oasis7/pr-publication").glob("*/*/journal.json"))
        self.assertEqual(1, len(journal_paths), f"expected one publication journal, found {journal_paths}")
        mapping_path = self.task_root / ".pm/github-project-sync/tasks.json"
        return {
            "mutations": list(state["mutations"]),
            "comments": list(state["comments"]),
            "mapping_path": mapping_path,
            "mapping": mapping_path.read_bytes(),
            "journal_path": journal_paths[0],
            "journal": journal_paths[0].read_bytes(),
        }

    def _assert_completed_replay_rejected_without_writes(self, result, snapshot, expected_error):
        self.assertNotEqual(0, result.returncode, "invalid completed replay must fail closed")
        self.assertIn(expected_error, result.stderr)
        after = self._load_state()
        self.assertEqual(snapshot["mutations"], after["mutations"],
                         "rejected completed replay must not write Issue, Project, or PR state")
        self.assertEqual(snapshot["comments"], after["comments"],
                         "rejected completed replay must not append publication comments")
        self.assertEqual(snapshot["mapping"], snapshot["mapping_path"].read_bytes(),
                         "rejected completed replay must not rewrite Task cache")
        self.assertEqual(snapshot["journal"], snapshot["journal_path"].read_bytes(),
                         "rejected completed replay must not rewrite publication history")

    def test_completed_replay_rejects_malformed_repository_union_value(self):
        snapshot = self._completed_replay_no_write_snapshot()
        self.state = self._load_state()
        self.state["project_values"]["Repository"] = {
            "type": "repository", "id": "", "name_with_owner": REPOSITORY,
        }
        self._save_state()
        result = self.run_publisher()
        self._assert_completed_replay_rejected_without_writes(
            result, snapshot, "completed replay Project Repository field is malformed",
        )

    def test_completed_replay_rejects_unsupported_repository_union_type(self):
        snapshot = self._completed_replay_no_write_snapshot()
        self.state = self._load_state()
        self.state["project_values"]["Repository"] = {
            "type": "repository", "id": "R_fixture_oasis7", "name_with_owner": REPOSITORY,
        }
        self.state["project_field_node_overrides"] = {
            "Repository": {"__typename": "ProjectV2ItemFieldUserValue"},
        }
        self._save_state()
        result = self.run_publisher()
        self._assert_completed_replay_rejected_without_writes(
            result, snapshot, "completed replay Project field type is unsupported",
        )

    def test_completed_replay_rejects_missing_required_task_project_field(self):
        snapshot = self._completed_replay_no_write_snapshot()
        self.state = self._load_state()
        self.state["project_values"].pop("Workflow Phase")
        self._save_state()
        result = self.run_publisher()
        self._assert_completed_replay_rejected_without_writes(
            result, snapshot, "completed replay live Project fields differ from the completed Task vector",
        )
    def test_record_pr_rejects_c1_timestamp_edit_on_locked_reread_before_writes(self):
        self.state["edit_c1_on_locked_comments_read"] = True
        self._save_state()

        result = self.run_publisher()
        state = self._load_state()
        self.assertEqual(4, state.get("record_pr_c1_reads"))
        c1 = [item for item in state["comments"]
              if "<!-- oasis7-ci-publication/v1 -->" in item.get("body", "")]
        self.assertEqual(1, len(c1))
        self.assertEqual("2026-10-02T02:00:00Z", c1[0].get("created_at"))
        self.assertEqual("2026-10-02T02:00:59Z", c1[0].get("updated_at"))
        self.assertEqual(1, len(self._effects("pr:create", state)))
        self.assertEqual([], self._effects("issue:body", state))
        self.assertEqual([], [item for item in state["mutations"]
                              if item.get("effect") and item.get("kind", "").startswith("project:")])
        self.assertNotEqual(
            0, result.returncode,
            "record-pr must reject a C1 server timestamp edit on the locked reread; "
            f"stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertIn("TASK_IDENTITY_CONFLICT: record-pr rejected the live Issue/Project vector",
                      result.stderr)

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
        self.assertEqual(1, len(self._effects("issue:body", state)),
                         "completed replay must stop before the old helper can rewrite Issue state")
        self.assertEqual(first_mutations, state["mutations"],
                         "completed replay must be a live-read no-op even with the old helper")

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

        resume_action_id = self._seed_recovery_admission(current_journal_before=self._journal())
        retried = self.run_publisher(resume_action_id=resume_action_id)
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
        self.assertNotEqual(0, resumed.returncode, resumed.stderr)
        self.assertIn("recovery admission", resumed.stderr)
        final = self._load_state()
        self.assertEqual(1, len(self._effects("pr:create", final)))
        self.assertEqual([], self._effects("issue:body", final))
        self.assertEqual([], self._effects("project:Workflow Phase", final))
        self.assertEqual([], self._effects("project:PR", final))

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
        resumed = self.run_publisher(resume_action_id=self._pending_resume_action_id())
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
        self.assertNotEqual(0, resumed.returncode, resumed.stderr)
        self.assertIn("recovery admission", resumed.stderr)
        final = self._load_state()
        self.assertEqual(1, len(self._effects("pr:create", final)))
        self.assertEqual([], self._effects("issue:body", final))
        self.assertEqual([], self._effects("project:Workflow Phase", final))
        self.assertEqual([], self._effects("project:PR", final))

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

        resume_action_id = self._seed_recovery_admission(current_journal_before=journal)
        retried = self.run_publisher(resume_action_id=resume_action_id)
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
        resume_action_id = self._seed_recovery_admission()
        before = list(self.state["mutations"])
        retry = self.run_publisher(resume_action_id=resume_action_id)
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

        second = self.run_publisher(resume_action_id=self._pending_resume_action_id())
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
