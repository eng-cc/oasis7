#!/usr/bin/env python3
"""Process-level policy adoption/read-context tests against a fake `gh` CLI."""
from __future__ import annotations

import base64
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
from collections import OrderedDict

HERE = Path(__file__).resolve().parent
REPO = "eng-cc/oasis7"
UID = "task_" + "a" * 32
ISSUE = 42
PR = 55
PROJECT_ID = "PVT_test"
PROJECT_ITEM = "PVTI_test"
PROJECT_NUMBER = 7
LOGIN = "human-owner"
PR_URL = f"https://github.com/{REPO}/pull/{PR}"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


sys.path.insert(0, str(HERE))
task_api = load_module("policy_adoption_task_api", HERE / "github-project-task.py")
sync_api = load_module("policy_adoption_sync_api", HERE / "github-project-sync.py")
policy_api = load_module("policy_adoption_policy_api", HERE / "loop_policy.py")
snapshot_api = load_module("policy_adoption_snapshot_api", HERE / "bootstrap-task-snapshot.py")


FAKE_GH = r'''#!/usr/bin/env python3
import base64, json, os, pathlib, subprocess, sys
from urllib.parse import urlsplit, parse_qs, unquote

state_path = pathlib.Path(os.environ["FAKE_GH_STATE"])
state = json.loads(state_path.read_text())
args = sys.argv[1:]
state.setdefault("calls", []).append(args)
state_path.write_text(json.dumps(state, sort_keys=True))

def save():
    state_path.write_text(json.dumps(state, sort_keys=True))

def emit(value, code=0):
    if isinstance(value, str):
        sys.stdout.write(value)
    else:
        sys.stdout.write(json.dumps(value))
    sys.stdout.write("\n")
    raise SystemExit(code)

def endpoint_value(endpoint):
    parsed = urlsplit(str(endpoint))
    path = parsed.path
    repo_prefix = "repos/" + state["repository"] + "/"
    if path == "repos/" + state["repository"]:
        return {"default_branch": "main"}
    if path == repo_prefix + "branches/main":
        return {"name": "main", "protected": True,
                "commit": {"sha": state["default_oid"]}}
    if path.startswith(repo_prefix + "contents/"):
        relative = unquote(path[len(repo_prefix + "contents/"):])
        oid = parse_qs(parsed.query).get("ref", [state["default_oid"]])[0]
        raw = subprocess.check_output(["git", "-C", state["root"], "show", f"{oid}:{relative}"])
        return {"encoding": "base64", "content": base64.b64encode(raw).decode()}
    if path == "user":
        return {"login": state["caller_login"], "type": "User"}
    prefix = repo_prefix + "collaborators/"
    if path.startswith(prefix) and path.endswith("/permission"):
        return {"permission": state.get("permissions", {}).get(path[len(prefix):-len("/permission")], "write")}
    if path == repo_prefix + f"issues/{state['issue']['number']}":
        issue = state["issue"]
        return {"id": issue["id"], "number": issue["number"], "node_id": issue["node_id"],
                "html_url": issue["url"], "url": issue["url"], "title": issue["title"],
                "body": issue["body"], "state": issue["state"].lower(), "state_reason": None,
                "user": issue["user"], "updated_at": issue.get("updated_at", "2026-10-02T00:00:00Z")}
    if state.get("pr") and path == repo_prefix + f"pulls/{state['pr']['number']}":
        return state["pr"]
    if path == repo_prefix + f"issues/{state['issue']['number']}/comments":
        comments = state["comments"]
        return [comments] if "--slurp" in args else comments
    state["fake_gh_error"] = {"endpoint": endpoint, "path": path,
                              "expected_comments": repo_prefix + f"issues/{state['issue']['number']}/comments"}
    save()
    raise SystemExit("unhandled fake gh endpoint: " + endpoint)

if not args:
    raise SystemExit("missing gh command")
if args[0] == "auth" and args[1:] == ["token"]:
    emit("fixture-token")
if args[0] == "project" and args[1] == "view":
    emit({"id": state["project"]["id"]})
if args[0] == "project" and args[1] == "field-list":
    fields = []
    for index, name in enumerate(state["project_values"], 1):
        options = []
        if name in {"Status", "Owner Role", "Module", "PM Status", "Workflow Phase", "Priority", "Test Tier Required", "Loop"}:
            value = state["project_values"][name]
            if value:
                options = [{"name": value, "id": f"OPT_{index}"}]
        fields.append({"id": f"FIELD_{index}", "name": name, "options": options})
    emit({"fields": fields})
if args[0] == "issue" and args[1] == "list":
    issue = state["issue"]
    hits = [{"number": issue["number"], "url": issue["url"], "title": issue["title"],
             "state": issue["state"]}]
    if state.get("duplicate_issue_discovery"):
        duplicate = dict(hits[0])
        duplicate["number"] = issue["number"] + 1
        duplicate["url"] = issue["url"].rsplit("/", 1)[0] + "/" + str(duplicate["number"])
        hits.append(duplicate)
    emit(hits)
if args[0] == "issue" and args[1] == "view":
    issue = state["issue"]
    requested_number = int(args[2])
    issue_url = issue["url"] if requested_number == issue["number"] else issue["url"].rsplit("/", 1)[0] + "/" + str(requested_number)
    body = issue["body"]
    if state.get("ambiguous_task_uid"):
        body += "\ntask_uid: task_" + ("b" * 32) + "\n"
    emit({"body": body, "number": requested_number, "title": issue["title"],
          "url": issue_url, "state": issue["state"], "stateReason": None,
          "updatedAt": issue.get("updated_at", "2026-10-02T00:00:00Z")})
if args[0] == "issue" and args[1] == "comment":
    state["post_attempts"] = state.get("post_attempts", 0) + 1
    body_path = args[args.index("--body-file") + 1]
    body = pathlib.Path(body_path).read_text()
    fault = state.get("post_fault")
    state["post_fault"] = None
    if fault == "before":
        save()
        emit("fixture rejected before effect", 1)
    comment_id = state.get("next_comment_id", 200)
    state["next_comment_id"] = comment_id + 1
    comment = {"id": comment_id, "body": body,
               "html_url": f"https://github.com/{state['repository']}/issues/{state['issue']['number']}#issuecomment-{comment_id}",
               "created_at": "2026-10-02T02:00:00Z", "updated_at": "2026-10-02T02:00:00Z",
               "user": {"login": state["caller_login"], "type": "User"},
               "author_association": "MEMBER"}
    state["comments"].append(comment)
    state["post_effects"] = state.get("post_effects", 0) + 1
    if fault == "edited":
        comment["updated_at"] = "2026-10-02T02:00:01Z"
    elif fault == "missing-updated-at":
        comment.pop("updated_at", None)
    elif fault == "invalid-updated-at":
        comment["updated_at"] = "not-a-server-timestamp"
    save()
    if fault == "after":
        emit("fixture lost response after effect", 1)
    emit(comment["html_url"])

if args[0] == "api":
    endpoint = next((arg for arg in args[1:] if not arg.startswith("-")), None)
    if endpoint == "graphql":
        flags = {}
        for index, arg in enumerate(args[1:-1]):
            if arg in {"-f", "-F"}:
                key, value = args[index + 2].split("=", 1)
                flags[key] = value
        query = flags.get("query", "")
        if "repository(owner:" in query:
            issue = state["issue"]
            project = {"id": state["project"]["id"], "number": state["project"]["number"],
                       "viewerCanUpdate": True, "owner": {"login": state["project"]["owner"]}}
            item = {"id": state["project"]["item_id"], "isArchived": False, "project": project}
            emit({"data": {"repository": {"issue": {
                "id": issue["node_id"], "number": issue["number"], "url": issue["url"],
                "state": issue["state"].upper(), "body": issue["body"],
                "projectItems": {"nodes": [item], "pageInfo": {"hasNextPage": False, "endCursor": None}},
            }}}})
        item_id = flags.get("item", state["project"]["item_id"])
        project = {"id": state["project"]["id"], "number": state["project"]["number"],
                   "viewerCanUpdate": True, "owner": {"login": state["project"]["owner"]}}
        values = []
        select = {"Status", "Owner Role", "Module", "PM Status", "Workflow Phase", "Priority", "Test Tier Required", "Loop"}
        for name, value in state["project_values"].items():
            field = {"name": name}
            values.append(({"name": value, "field": field} if name in select
                           else {"text": value, "field": field}))
        emit({"data": {"node": {"id": item_id, "isArchived": False, "project": project,
                                   "fieldValues": {"nodes": values,
                                                   "pageInfo": {"hasNextPage": False, "endCursor": None}}}}})
    emit(endpoint_value(endpoint))
raise SystemExit("unhandled fake gh command: " + " ".join(args))
'''


class PolicyAdoptionCLITests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="policy-adoption-cli-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "repo"
        self.root.mkdir()
        pm = self.root / "scripts" / "pm"
        shutil.copytree(HERE, pm, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        source = HERE.parents[1] / "doc" / "engineering" / "workflow" / "source-of-truth.md"
        (self.root / "doc/engineering/workflow").mkdir(parents=True)
        shutil.copy2(source, self.root / "doc/engineering/workflow/source-of-truth.md")
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.git("config", "remote.origin.url", f"https://github.com/{REPO}.git")
        self.write("doc/engineering/unrelated.txt", "seed\n")
        self.git("add", "scripts/pm", "doc")
        self.git("commit", "-qm", "frozen workflow source")
        self.binding_oid = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.binding_oid)
        self.git("symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        self.task_root = self.root.parent / "task-worktree"
        self.git("worktree", "add", "-q", "-b", "task/" + UID[-8:], str(self.task_root), self.binding_oid)
        self.task_head = subprocess.check_output(["git", "-C", str(self.task_root), "rev-parse", "HEAD"], text=True).strip()
        self.write("doc/engineering/unrelated.txt", "current trusted default tip\n")
        self.git("add", "doc/engineering/unrelated.txt")
        self.git("commit", "-qm", "advance default branch without changing policy")
        self.current_oid = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.current_oid)
        self.git("config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*")
        self.git("config", "branch.main.remote", "origin")
        self.git("config", "branch.main.merge", "refs/heads/main")
        self.git("worktree", "lock", str(self.task_root))
        self.binding = self.make_binding()
        self.comments = []
        self.issue_state = "open"
        self.issue_status = "committed"
        self.issue_phase = "verification"
        self.pr_enabled = True
        self.project_values = {}
        self._write_mapping_and_snapshot()
        self._write_fake_gh()
        self._refresh_state()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def task_git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.task_root), *args], text=True).strip()

    def write(self, relative: str, value: str):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value, encoding="utf-8")

    def make_binding(self):
        raw_policy = (self.root / "scripts/pm/loop-policy.v1.json").read_bytes()
        return {
            "schema": "oasis7.loop-task/v1", "task_uid": UID,
            "change_id": "change-adoption-fixture", "loop": "code",
            "owner_role": "repository_health_engineer", "bootstrap_epoch": 1,
            "manual_request_ref": "issuecomment-99", "request_key": "adoption-cli-test",
            "write_scope": ["scripts/pm/**"], "out_of_scope": [],
            "input_contracts": [], "acceptance_refs": ["policy adoption CLI recovery"],
            "dependencies": [], "target_delivery": "single-pr",
            "policy_digest": "sha256:" + hashlib.sha256(raw_policy).hexdigest(),
            "policy_commit": self.binding_oid,
        }

    def _write_mapping_and_snapshot(self):
        mapping = {
            "version": 1,
            "project": {"id": PROJECT_ID, "owner": "eng-cc", "number": PROJECT_NUMBER,
                        "repo": REPO},
            "tasks": {UID: {"task_uid": UID, "issue_number": ISSUE,
                            "issue_url": f"https://github.com/{REPO}/issues/{ISSUE}",
                            "project_item_id": PROJECT_ITEM, "repository": REPO,
                            "canonical_worktree": str(self.task_root),
                            "task_branch": "task/" + UID[-8:], "default_branch": "main"}},
        }
        mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        mapping_path.parent.mkdir(parents=True, exist_ok=True)
        mapping_path.write_text(json.dumps(mapping, sort_keys=True), encoding="utf-8")
        snapshot = {"schema": "oasis7.bootstrap-task-snapshot/v1", "task": {
            "uid": UID, "bootstrap_epoch": 1, "loop_binding": self.binding,
        }}
        snapshot["digest"] = snapshot_api.digest(snapshot)
        snapshot_path = self.task_root / ".pm/scratch" / UID / "bootstrap-task-snapshot.json"
        snapshot_path.parent.mkdir(parents=True, exist_ok=True)
        snapshot_path.write_text(json.dumps(snapshot, sort_keys=True), encoding="utf-8")
        common = Path(self.git("rev-parse", "--git-common-dir"))
        if not common.is_absolute():
            common = self.root / common
        lineage_path = common.resolve() / "oasis7-loop-lineage" / f"{UID}.json"
        lineage_path.parent.mkdir(parents=True, exist_ok=True)
        lineage_path.write_text(json.dumps({"loop_binding": self.binding, "previous_epochs": []}), encoding="utf-8")

    def _task(self):
        task = OrderedDict({
            "task_uid": UID, "title": "Policy adoption integration fixture",
            "owner_role": "repository_health_engineer", "module": "engineering",
            "status": self.issue_status, "workflow_phase": self.issue_phase,
            "priority": "P1", "worktree_hint": str(self.task_root),
            "loop_binding": self.binding,
            "pr_number": PR if self.pr_enabled else None,
            "pr_url": PR_URL if self.pr_enabled else None,
            "acceptance": ["Adoption records preserve the immutable original binding."],
            "merge_hold": {"kind": "normal_pr_ci_watch", "active": False,
                           "requester": "tpm", "reason": "", "resume_authority": "tpm"},
            "updated_at": "2026-10-02T00:00:00Z",
        })
        return task

    def _history_comment(self):
        digest = hashlib.sha256(json.dumps(
            self.binding, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        ).encode()).hexdigest()
        marker = f"<!-- oasis7-loop-binding-history task_uid={UID} epoch=1 digest={digest} -->"
        return {"id": 98, "body": marker + "\nImmutable loop binding history; removing current metadata does not restore legacy admission.\n",
                "created_at": "2026-10-01T00:00:00Z", "updated_at": "2026-10-01T00:00:00Z",
                "user": {"login": LOGIN, "type": "User"}, "author_association": "MEMBER"}

    def _authorization_comment(self):
        identity = {
            "task_uid": UID, "bootstrap_epoch": 1,
            "binding_identity_digest": policy_api.binding_identity_digest(self.binding),
            "write_scope_digest": policy_api.write_scope_digest(self.binding),
        }
        target_digest = self.binding["policy_digest"]
        body = policy_api.policy_adoption_authorization_comment({
            **identity, "target_policy_commit": self.current_oid,
            "target_policy_digest": target_digest,
        })
        return {"id": 99, "body": body, "created_at": "2026-10-01T00:01:00Z",
                "updated_at": "2026-10-01T00:01:00Z",
                "user": {"login": LOGIN, "type": "User"}, "author_association": "MEMBER"}

    def _refresh_state(self):
        task = self._task()
        issue_body = task_api.issue_body(task)
        issue = {"id": 1042, "node_id": "I_issue", "number": ISSUE,
                 "url": f"https://github.com/{REPO}/issues/{ISSUE}",
                 "title": "[PM] Policy adoption integration fixture", "state": self.issue_state,
                 "body": issue_body, "user": {"login": LOGIN, "type": "User"},
                 "updated_at": "2026-10-02T00:00:00Z"}
        self.project_values = sync_api.project_field_values(task)
        pr = None
        if self.pr_enabled:
            pr = {"number": PR, "html_url": PR_URL, "state": "open", "merged": False,
                  "merged_at": None, "draft": True,
                  "created_at": "2026-10-01T01:00:00Z", "updated_at": "2026-10-02T00:00:00Z",
                  "body": f"Task: {UID}\nRefs #{ISSUE}\n",
                  "user": {"login": LOGIN, "type": "User"},
                  "head": {"ref": "task/" + UID[-8:], "sha": self.task_head,
                           "repo": {"full_name": REPO}},
                  "base": {"ref": "main", "sha": self.current_oid,
                           "repo": {"full_name": REPO}}}
        state = {
            "root": str(self.root), "repository": REPO, "default_oid": self.current_oid,
            "issue": issue, "pr": pr,
            "comments": [self._history_comment(), self._authorization_comment()],
            "caller_login": LOGIN, "permissions": {LOGIN: "write"},
            "project": {"id": PROJECT_ID, "number": PROJECT_NUMBER,
                        "owner": "eng-cc", "item_id": PROJECT_ITEM},
            "project_values": self.project_values, "post_attempts": 0,
            "post_effects": 0, "next_comment_id": 200, "post_fault": None,
        }
        self.state_path = Path(self.temporary.name) / "github.json"
        self.state_path.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")

    def _write_fake_gh(self):
        binary = Path(self.temporary.name) / "bin"
        binary.mkdir(parents=True)
        gh = binary / "gh"
        gh.write_text(FAKE_GH, encoding="utf-8")
        gh.chmod(0o755)
        self.env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ["PATH"],
                        FAKE_GH_STATE=str(Path(self.temporary.name) / "github.json"),
                        OASIS7_PM_FAKE_GITHUB="1", PYTHONDONTWRITEBYTECODE="1")

    def state(self):
        return json.loads(self.state_path.read_text(encoding="utf-8"))

    def save_state(self, state):
        self.state_path.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")

    def invoke(self, command: str, *, hosted=False):
        argv = [sys.executable, str(self.root / "scripts/pm/github-project-task.py"),
                command, str(self.root), "--repo", REPO, "--project-owner", "eng-cc",
                "--project-number", str(PROJECT_NUMBER), "--task-uid", UID, "--json"]
        if hosted:
            argv.insert(-1, "--hosted-read")
        return subprocess.run(argv, text=True, capture_output=True, env=self.env, cwd=self.root)

    def output_json(self, result):
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"CLI did not return JSON (exit={result.returncode}): {result.stdout}\n{result.stderr}\n{exc}")

    def test_hosted_terminal_no_pr_fallback_needs_no_project_or_worktree(self):
        self.issue_state = "closed"
        self.issue_status = "done"
        self.issue_phase = "post_merge_done"
        self.pr_enabled = False
        self._refresh_state()
        result = self.invoke("read-live-policy-context", hosted=True)
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(payload["status"], "passed", payload)
        self.assertTrue(payload["complete"])
        self.assertEqual(payload["live_task_identity"]["task_branch"], "")
        self.assertIsNone(payload["project"])
        self.assertIsNone(payload["caller"])
        self.assertIsNone(payload["pr"])
        self.assertEqual(payload["effective_policy"]["pin_source"], "immutable_binding")
        self.assertEqual(payload["effective_policy"]["policy_commit"], self.binding_oid)

    def test_duplicate_task_issue_identity_is_blocked_not_pending(self):
        for state_flag, expected in (
            ("duplicate_issue_discovery", "multiple canonical task Issues; reconcile before creation"),
            ("ambiguous_task_uid", "task Issue has ambiguous canonical UID"),
        ):
            for command, hosted in (("read-live-policy-context", True),
                                    ("adopt-workflow-policy", False)):
                with self.subTest(state_flag=state_flag, command=command):
                    state = self.state()
                    state[state_flag] = True
                    self.save_state(state)
                    result = self.invoke(command, hosted=hosted)
                    payload = self.output_json(result)
                    self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
                    self.assertEqual(payload["status"], "blocked", payload)
                    self.assertEqual(payload["complete"], False)
                    self.assertEqual(payload["blockers"], [expected])
                    self.assertEqual(self.state()["post_effects"], 0)
                    state = self.state()
                    state.pop(state_flag, None)
                    self.save_state(state)

    def test_new_adoption_without_bound_pr_is_zero_write(self):
        self.pr_enabled = False
        self.issue_phase = "execution"
        self._refresh_state()
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        state = self.state()
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr + repr(self.state().get("fake_gh_error")) + repr(self.state().get("calls")))
        self.assertEqual(payload["status"], "blocked")
        self.assertIn("already-bound open PR", payload["blockers"][0])
        self.assertEqual(state["post_attempts"], 0)
        self.assertEqual(state["post_effects"], 0)
        action_dir = self.root / ".git/oasis7-loop-recovery"
        self.assertFalse(action_dir.exists() and list(action_dir.glob("*.actions.jsonl")))

    def test_candidate_helper_change_cannot_self_admit(self):
        helper = self.root / "scripts/pm/github-project-task.py"
        helper.write_text(helper.read_text(encoding="utf-8") + "\n# unmerged candidate mutation\n", encoding="utf-8")
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertEqual(payload["status"], "blocked")
        self.assertIn("uncommitted changes", payload["blockers"][0])
        self.assertEqual(self.state()["post_effects"], 0)

    def test_untrusted_authorization_comment_is_zero_write(self):
        state = self.state()
        state["comments"][1]["author_association"] = "NONE"
        self.save_state(state)
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertEqual(payload["status"], "blocked")
        self.assertIn("trusted human Task evidence", payload["blockers"][0])
        self.assertEqual(self.state()["post_effects"], 0)

    def assert_bad_authorization_timestamp_is_zero_write(self, mutate):
        state = self.state()
        mutate(state["comments"][1])
        self.save_state(state)
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn(payload["status"], {"blocked", "pending"}, payload)
        after = self.state()
        self.assertEqual(0, after["post_attempts"])
        self.assertEqual(0, after["post_effects"])

    def test_edited_authorization_comment_is_zero_write(self):
        self.assert_bad_authorization_timestamp_is_zero_write(
            lambda comment: comment.update(updated_at="2026-10-01T00:02:00Z"),
        )

    def test_missing_authorization_updated_at_is_zero_write(self):
        self.assert_bad_authorization_timestamp_is_zero_write(
            lambda comment: comment.pop("updated_at"),
        )

    def test_invalid_authorization_updated_at_is_zero_write(self):
        self.assert_bad_authorization_timestamp_is_zero_write(
            lambda comment: comment.update(updated_at="not-a-server-timestamp"),
        )

    def test_authorization_for_superseded_tip_is_rejected(self):
        self.write("doc/engineering/unrelated.txt", "supersede the authorized target\n")
        self.git("add", "doc/engineering/unrelated.txt")
        self.git("commit", "-qm", "advance trusted default branch")
        next_oid = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", next_oid)
        state = self.state()
        state["default_oid"] = next_oid
        self.save_state(state)
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertEqual(payload["status"], "blocked")
        self.assertIn("one unique pre-existing user authorization", payload["blockers"][0])
        self.assertEqual(self.state()["post_effects"], 0)

    def test_pre_effect_post_failure_is_pending_then_retried_once(self):
        state = self.state()
        state["post_fault"] = "before"
        self.save_state(state)
        first = self.invoke("adopt-workflow-policy")
        first_payload = self.output_json(first)
        self.assertEqual(first.returncode, 2, first.stdout + first.stderr)
        self.assertEqual(first_payload["status"], "pending")
        state = self.state()
        self.assertEqual(state["post_effects"], 0)
        self.assertEqual(state["post_attempts"], 1)

        retry = self.invoke("adopt-workflow-policy")
        retry_payload = self.output_json(retry)
        self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)
        self.assertEqual(retry_payload["status"], "adopted", retry_payload)
        self.assertEqual(retry_payload["mutation_count"], 1)
        state = self.state()
        self.assertEqual(state["post_attempts"], 2)
        self.assertEqual(state["post_effects"], 1)
        self.assertEqual(sum("oasis7.workflow-policy-adoption/v1" in item["body"] for item in state["comments"]), 1)

        again = self.invoke("adopt-workflow-policy")
        again_payload = self.output_json(again)
        self.assertEqual(again.returncode, 0, again.stdout + again.stderr)
        self.assertEqual(again_payload["status"], "unchanged", again_payload)
        self.assertEqual(again_payload["mutation_count"], 0)
        self.assertEqual(self.state()["post_effects"], 1)

    def test_post_effect_lost_response_reconciles_without_duplicate(self):
        state = self.state()
        state["post_fault"] = "after"
        self.save_state(state)
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(payload["status"], "adopted", payload)
        self.assertEqual(payload["mutation_count"], 1)
        state = self.state()
        self.assertEqual(state["post_attempts"], 1)
        self.assertEqual(state["post_effects"], 1)
        again = self.invoke("adopt-workflow-policy")
        again_payload = self.output_json(again)
        self.assertEqual(again.returncode, 0, again.stdout + again.stderr)
        self.assertEqual(again_payload["status"], "unchanged", again_payload)
        self.assertEqual(self.state()["post_effects"], 1)

    def test_edited_adoption_post_readback_stays_pending_without_duplicate(self):
        state = self.state()
        state["post_fault"] = "edited"
        self.save_state(state)
        first = self.invoke("adopt-workflow-policy")
        first_payload = self.output_json(first)
        self.assertNotEqual(0, first.returncode, first.stdout + first.stderr)
        self.assertIn(first_payload["status"], {"blocked", "pending"}, first_payload)
        state = self.state()
        adoption_comments = [comment for comment in state["comments"]
                             if "oasis7.workflow-policy-adoption/v1" in comment["body"]]
        self.assertEqual(1, len(adoption_comments))
        self.assertNotEqual(adoption_comments[0]["created_at"], adoption_comments[0]["updated_at"])
        self.assertEqual(1, state["post_attempts"])
        self.assertEqual(1, state["post_effects"])

        retry = self.invoke("adopt-workflow-policy")
        retry_payload = self.output_json(retry)
        self.assertNotEqual(0, retry.returncode, retry.stdout + retry.stderr)
        self.assertIn(retry_payload["status"], {"blocked", "pending"}, retry_payload)
        state = self.state()
        adoption_comments = [comment for comment in state["comments"]
                             if "oasis7.workflow-policy-adoption/v1" in comment["body"]]
        self.assertEqual(1, len(adoption_comments), "an edited adoption record must not be appended again")
        self.assertEqual(1, state["post_attempts"])
        self.assertEqual(1, state["post_effects"])

    def test_adopted_pin_survives_unrelated_default_branch_advance(self):
        result = self.invoke("adopt-workflow-policy")
        payload = self.output_json(result)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        adopted_oid = payload["policy_commit"]
        self.assertNotEqual(adopted_oid, self.binding_oid)

        self.write("doc/engineering/unrelated.txt", "advance again\n")
        self.git("add", "doc/engineering/unrelated.txt")
        self.git("commit", "-qm", "unrelated default branch advance")
        next_oid = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", next_oid)
        state = self.state()
        state["default_oid"] = next_oid
        self.save_state(state)

        hosted = self.invoke("read-live-policy-context", hosted=True)
        hosted_payload = self.output_json(hosted)
        self.assertEqual(hosted.returncode, 0, hosted.stdout + hosted.stderr)
        self.assertEqual(hosted_payload["effective_policy"]["policy_commit"], adopted_oid)
        self.assertEqual(hosted_payload["effective_policy"]["pin_source"], "task_issue_adoption_chain")
        self.assertEqual(hosted_payload["binding"]["policy_commit"], self.binding_oid)
        self.assertEqual(self.state()["post_effects"], 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
