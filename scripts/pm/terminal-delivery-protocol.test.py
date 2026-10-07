#!/usr/bin/env python3
"""End-to-end regressions for the separate v2 terminal delivery protocol."""
from __future__ import annotations

import base64
import copy
import hashlib
import importlib.util
import json
import os
import pathlib
import shlex
import shutil
import subprocess
import sys
import tempfile
import uuid
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/pm"))
import task_complete_claim
import terminal_proof

UID = "task_" + "a" * 32
REPOSITORY = "fixture/repo"
ISSUE = 11
PR = 12
PR_URL = f"https://github.com/{REPOSITORY}/pull/{PR}"
ISSUE_URL = f"https://github.com/{REPOSITORY}/issues/{ISSUE}"
NOW = "2026-09-30T08:00:00Z"
TICK = chr(96)


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


class DeliveryFixture:
    """Disposable Git repository plus deterministic live-GitHub readbacks."""

    def __init__(self, parent: pathlib.Path):
        parent = parent.resolve()
        self.root = parent / "repo"
        self.task = parent / "task-worktree"
        self.pm_tools = self.root / "scripts/pm"
        self.bin = parent / "bin"
        self.state_path = parent / "github-state.json"
        self.remote_state_path = parent / "remote-branch-state.json"
        self.log_path = parent / "gh-log.jsonl"
        self.lost_marker = parent / "lost-response-once"
        self.lost_close_marker = parent / "lost-close-response-once"
        self.process_probe_mode: str | None = None
        self.remote = parent / "fixture-origin.git"
        self.bin.mkdir(parents=True)
        self._git("init", "-q", "-b", "main", str(self.root))
        self._git("config", "user.name", "QA Fixture", cwd=self.root)
        self._git("config", "user.email", "qa@example.invalid", cwd=self.root)
        canonical_origin = f"https://github.com/{REPOSITORY}.git"
        self._git("config", "remote.origin.url", canonical_origin, cwd=self.root)
        (self.root / "README.md").write_text("base\n", encoding="utf-8")
        self._git("add", "README.md", cwd=self.root)
        self._git("commit", "-qm", "base", cwd=self.root)
        self._git("worktree", "add", "-qb", "task/protocol-fixture", str(self.task), cwd=self.root)
        (self.task / "README.md").write_text("base\nmerged task change\n", encoding="utf-8")
        self._git("add", "README.md", cwd=self.task)
        self._git("commit", "-qm", "task change", cwd=self.task)
        shutil.copytree(ROOT / "scripts/pm", self.pm_tools)
        shutil.copy2(ROOT / "scripts/pm/fixtures/github_api_test_adapter.py",
                     self.pm_tools / "github_api.py")
        self.head_oid = self._git("rev-parse", "HEAD", cwd=self.task)
        self.head_tree = self._git("rev-parse", "HEAD^{tree}", cwd=self.task)
        self.worktree_common_dir = self._git(
            "rev-parse", "--path-format=absolute", "--git-common-dir", cwd=self.task)
        self.worktree_admin_dir = self._git(
            "rev-parse", "--absolute-git-dir", cwd=self.task)
        self.worktree_instance_id = str(uuid.uuid4())
        (pathlib.Path(self.worktree_admin_dir) / "oasis7-instance-id").write_text(
            self.worktree_instance_id + "\n", encoding="ascii")
        self._git("merge", "--no-ff", "-qm", "merge task", "task/protocol-fixture", cwd=self.root)
        self.merge_oid = self._git("rev-parse", "HEAD", cwd=self.root)

        fingerprint = "1" * 64
        self.claim = {
            "claim_type": "task_complete", "verify_command": "true", "verified_at": NOW,
            "verification_exit_code": 0, "status": "verified", "allowed_to_claim": True,
            "claim_message": "fixture verifies task completion", "blocked_phrase": "BLOCKED",
            "success_phrase": "PASS", "task_uid": UID,
            "repository_fingerprint_before": fingerprint,
            "repository_fingerprint_after": fingerprint,
            "verification_epoch_stable": True, "verification_mode": "detached_frozen_tree",
            "frozen_source_head": self.head_oid, "frozen_source_tree": self.head_tree,
            "comparison_ref": "refs/heads/main", "verification_profile": "repository_required",
            "repository_head": self.head_oid, "repository_index_sha256": "2" * 64,
        }
        history = base64.urlsafe_b64encode(canonical([self.claim])).decode().rstrip("=")
        self.issue_body = (
            f"task_uid: {UID}\n- pr_number: {TICK}{PR}{TICK}\n- pr_url: {TICK}{PR_URL}{TICK}\n"
            f"- claim_verifications_b64: {TICK}{history}{TICK}\n"
        )
        claim_body = task_complete_claim._claim_comment_body(UID, self.claim)
        self.issue = {
            "number": ISSUE, "html_url": ISSUE_URL,
            "url": f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE}",
            "body": self.issue_body, "state": "OPEN", "state_reason": "",
        }
        self.pr = {
            "number": PR, "html_url": PR_URL, "body": f"Task: {UID}\nRefs #{ISSUE}\n",
            "state": "CLOSED", "merged": True, "merged_at": NOW,
            "merge_commit_sha": self.merge_oid,
            "base": {"ref": "main", "repo": {"full_name": REPOSITORY}},
            "head": {"sha": self.head_oid, "repo": {"full_name": REPOSITORY}},
        }
        self.comments = [{
            "id": 801, "body": claim_body,
            "user": {"login": REPOSITORY.split("/", 1)[0]},
            "html_url": f"{ISSUE_URL}#issuecomment-801",
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE}",
            "created_at": NOW, "updated_at": NOW,
        }]
        self.project_item = {
            "id": "PVTI_fixture", "project": {"id": "PROJECT_fixture", "number": 1,
                "owner": {"login": "fixture"}},
            "content": {"number": ISSUE, "url": ISSUE_URL, "body": self.issue_body},
            "fieldValues": {"pageInfo": {"hasNextPage": False}, "nodes": [
                {"__typename": "ProjectV2ItemFieldSingleSelectValue", "name": "Done", "field": {"name": "Status"}},
                {"__typename": "ProjectV2ItemFieldSingleSelectValue", "name": "done", "field": {"name": "PM Status"}},
                {"__typename": "ProjectV2ItemFieldSingleSelectValue", "name": "done", "field": {"name": "Workflow Phase"}},
                {"__typename": "ProjectV2ItemFieldTextValue", "text": UID, "field": {"name": "Task UID"}},
                {"__typename": "ProjectV2ItemFieldRepositoryValue", "field": {"name": "Repository"},
                 "repository": {"nameWithOwner": REPOSITORY}},
            ]},
        }
        merge_receipt = {
            "receipt_type": "oasis7_pr_merge", "issuer": "github_live_query",
            "evidence_mode": "production", "repository": REPOSITORY,
            "default_branch": "main", "pr_number": PR, "pr_url": PR_URL,
            "state": "MERGED", "merged_at": NOW, "observed_at": NOW,
            "head_oid": self.head_oid, "merge_commit_oid": self.merge_oid, "base_ref": "main",
        }
        merge_raw = canonical(merge_receipt) + b"\n"
        merge_digest = sha(merge_raw)
        self.mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        self.mapping_path.parent.mkdir(parents=True)
        self.record = {
            "task_uid": UID, "repository": REPOSITORY, "status": "done",
            "workflow_phase": "task_done", "completion_mode": "single_pr",
            "issue_number": ISSUE, "issue_url": ISSUE_URL, "pr_number": PR,
            "pr_url": PR_URL, "canonical_worktree": str(self.task),
            "task_branch": "task/protocol-fixture", "default_branch": "main",
            "owner_role": "repository_health_engineer", "project_item_id": "PVTI_fixture",
            "claim_verifications": [self.claim], "merge_receipt": merge_receipt,
            "merge_receipt_sha256": merge_digest,
            "worktree_registration": {
                "common_dir": self.worktree_common_dir,
                "admin_dir": self.worktree_admin_dir,
                "instance_id": self.worktree_instance_id,
            },
        }
        # Use the original producer's pre-finalization projection, not a
        # terminal Status supplied before the finalizer has performed its effect.
        sync_spec = importlib.util.spec_from_file_location(
            "fixture_initial_project_sync", self.pm_tools / "github-project-sync.py")
        sync = importlib.util.module_from_spec(sync_spec)
        sync_spec.loader.exec_module(sync)
        initial_fields = sync.project_field_values(self.record)
        for node in self.project_item["fieldValues"]["nodes"]:
            field = node["field"]["name"]
            if field in {"Status", "PM Status", "Workflow Phase"}:
                node["name"] = initial_fields[field]
        mapping = {"version": 1, "project": {"owner": "fixture", "number": 1,
                    "id": "PROJECT_fixture", "repo": REPOSITORY}, "tasks": {UID: self.record}}
        self.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        self.state = {"issue": self.issue, "pr": self.pr, "comments": self.comments,
                      "project_item": self.project_item, "merge_oid": self.merge_oid,
                      "target_oid": self.merge_oid}
        self._write_state()
        self.remote_state_path.write_text(
            json.dumps({"refs/heads/task/protocol-fixture": self.head_oid}), encoding="utf-8")
        helper = self.pm_tools / "canonical-receipt-root.py"
        raw = subprocess.check_output([
            sys.executable, str(helper), "--default-worktree", str(self.root),
            "--task-uid", UID, "--create",
        ], text=True)
        self.receipt_root = pathlib.Path(raw.strip())
        (self.receipt_root / "merge-receipt.json").write_bytes(merge_raw)
        self._install_gh_stub()
        self._install_git_network_stub()

    def prepare_native_v2_readiness(self):
        """Explicit positive-v2 setup; default/v1 constructors stay unprepared.

        Build source T before accepted H and merge M. Encode independent native
        artifacts; the real production validator alone creates the proof.
        """
        spec = importlib.util.spec_from_file_location(
            "fixture_readiness_transport", self.pm_tools / "readiness_transport.py")
        readiness = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(readiness)
        base = self._git("rev-parse", "HEAD^1", cwd=self.root)
        self._git("reset", "--hard", base, cwd=self.root)
        self._git("add", "scripts/pm/pr-lifecycle-gate.py", "scripts/pm/claim-ready.sh", cwd=self.root)
        self._git("commit", "-qm", "trusted native-v2 tool entries", cwd=self.root)
        source = readiness.source_identity(self.root)
        self._git("reset", "--hard", source["oid"], cwd=self.task)
        (self.task / "README.md").write_text("base\nmerged task change\n", encoding="utf-8")
        self._git("add", "README.md", cwd=self.task)
        self._git("commit", "-qm", "accepted v2 task change", cwd=self.task)
        self.head_oid = self._git("rev-parse", "HEAD", cwd=self.task)
        self.head_tree = self._git("rev-parse", "HEAD^{tree}", cwd=self.task)
        self._git("merge", "--no-ff", "-qm", "merge accepted v2 task",
                  "task/protocol-fixture", cwd=self.root)
        self.merge_oid = self._git("rev-parse", "HEAD", cwd=self.root)
        self.claim.update(frozen_source_head=self.head_oid, frozen_source_tree=self.head_tree,
                          repository_head=self.head_oid)
        history = base64.urlsafe_b64encode(canonical([self.claim])).decode().rstrip("=")
        self.issue_body = (
            f"task_uid: {UID}\n- pr_number: {TICK}{PR}{TICK}\n- pr_url: {TICK}{PR_URL}{TICK}\n"
            f"- claim_verifications_b64: {TICK}{history}{TICK}\n")
        self.issue["body"] = self.issue_body
        self.project_item["content"]["body"] = self.issue_body
        self.pr["head"]["sha"] = self.head_oid
        self.pr["base"]["sha"] = source["oid"]
        self.pr["merge_commit_sha"] = self.merge_oid
        self.comments[0]["body"] = task_complete_claim._claim_comment_body(UID, self.claim)
        merge_receipt = dict(self.record["merge_receipt"], head_oid=self.head_oid,
                             merge_commit_oid=self.merge_oid)
        merge_raw = canonical(merge_receipt) + b"\n"
        self.record.update(claim_verifications=[self.claim], merge_receipt=merge_receipt,
                           merge_receipt_sha256=sha(merge_raw))
        mapping = self.mapping()
        mapping["tasks"][UID] = self.record
        self.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        (self.receipt_root / "merge-receipt.json").write_bytes(merge_raw)
        self.state.update(issue=self.issue, pr=self.pr, project_item=self.project_item,
                          merge_oid=self.merge_oid, target_oid=self.merge_oid)
        self.remote_state_path.write_text(
            json.dumps({"refs/heads/task/protocol-fixture": self.head_oid}), encoding="utf-8")

        verified_at = "2026-09-30T15:59:40+08:00"
        policy = {"status": "resolved", "label": "审查", "ci_digest": "sha256:" + "a" * 64}
        preimage = {"repository": REPOSITORY, "pr_number": PR, "head_oid": self.head_oid,
                    "blockers": [], "policy": policy, "hold": None}
        epoch = sha(json.dumps(preimage, sort_keys=True, separators=(",", ":")).encode())
        gate = {"evidence_mode": "production", "status": "ready", "ready_for_merge": True,
                "blockers": [], "pr_number": PR, "pr_url": PR_URL,
                "policy_discovery": policy, "merge_hold": None,
                "readiness_receipt": {"receipt_type": "oasis7_pr_lifecycle_ready",
                    "issuer": "oasis7_pr_lifecycle_gate/v1", "repository": REPOSITORY,
                    "pr_number": PR, "head_oid": self.head_oid,
                    "observed_at": "2026-09-30T07:59:35Z", "gate_epoch": epoch}}
        gate_raw = json.dumps(gate, ensure_ascii=False, indent=2).encode() + b"\n"
        binding = {"schema": "oasis7-native-readiness/v2", "task_uid": UID,
            "repository": REPOSITORY, "issue_number": ISSUE, "pr_number": PR,
            "pr_url": PR_URL, "head_oid": self.head_oid, "claim_type": "ready_for_merge",
            "status": "verified", "exit_code": 0, "verified_at": verified_at,
            "gate_raw_sha256": sha(gate_raw), "gate_epoch": epoch,
            "gate_observed_at": gate["readiness_receipt"]["observed_at"], "source": source}
        body = "<!-- oasis7-native-readiness/v2 -->\n" + canonical(binding).decode()
        native_comment = {"id": 802, "body": body, "user": {"login": "fixture"},
            "created_at": "2026-09-30T07:59:41Z", "updated_at": "2026-09-30T07:59:41Z",
            "html_url": f"{ISSUE_URL}#issuecomment-802",
            "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE}"}
        capture = {"id": 802, "body_b64": base64.b64encode(body.encode()).decode(),
            "body_sha256": sha(body.encode()), "author": "fixture",
            "created_at": native_comment["created_at"], "updated_at": native_comment["updated_at"]}
        native_result = dict(self.claim, claim_type="ready_for_merge", verified_at=verified_at,
            claim_message="accepted native readiness", frozen_source_head=None, frozen_source_tree=None,
            comparison_ref=None, verification_mode="live_nonfinal",
            readiness_binding=binding, readiness_comment=capture)
        self.state["readiness_comments"] = [native_comment]
        self._write_state()
        native_root = self.receipt_root / "native-readiness" / sha(canonical(binding))
        native_root.mkdir(parents=True)
        (native_root / "gate.stdout").write_bytes(gate_raw)
        (native_root / "result.stdout").write_bytes(
            json.dumps(native_result, ensure_ascii=False).encode() + b"\n")
        (native_root / "comment.json").write_bytes(canonical(capture))
        created = subprocess.run([sys.executable, str(self.pm_tools / "readiness_transport.py"),
            "--repo-root", str(self.root), "--task-uid", UID, "--create"],
            text=True, capture_output=True, env=self.env())
        if created.returncode:
            raise AssertionError("positive native readiness validation failed: " + created.stdout + created.stderr)

    @staticmethod
    def _git(*args: str, cwd: pathlib.Path | None = None) -> str:
        proc = subprocess.run(["git", *args], cwd=cwd, text=True, capture_output=True)
        if proc.returncode:
            raise AssertionError(f"git {' '.join(args)} failed: {proc.stderr}")
        return proc.stdout.strip()

    def _write_state(self):
        self.state_path.write_text(json.dumps(self.state), encoding="utf-8")

    def _install_gh_stub(self):
        script = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
state_path = pathlib.Path(os.environ["QA_GH_STATE"])
log_path = pathlib.Path(os.environ["QA_GH_LOG"])
state = json.loads(state_path.read_text())
with log_path.open("a", encoding="utf-8") as log:
    log.write(json.dumps(args) + "\n")
def out(value): print(json.dumps(value))
if args[:1] == ["project"] and len(args) > 1 and args[1] == "view":
    out({"id":state["project_item"]["project"]["id"]})
elif args[:1] == ["project"] and len(args) > 1 and args[1] == "field-list":
    out({"fields":[{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField", "options":[{"id":"STATUS_DONE","name":"Done"},{"id":"STATUS_PROGRESS","name":"In Progress"}]}]})
elif args[:2] == ["project", "item-edit"]:
    if (len(args) != 12 or args[2::2] != ["--id","--project-id","--field-id","--single-select-option-id","--format"]
            or args[3] != state["project_item"]["id"] or args[5] != state["project_item"]["project"]["id"]
            or args[7] != "FIELD_STATUS" or args[9] not in {"STATUS_DONE","STATUS_PROGRESS"} or args[11] != "json"):
        raise SystemExit("unsupported fixture Project mutation")
    for node in state["project_item"]["fieldValues"]["nodes"]:
        if node["field"]["name"] == "Status": node["name"] = {"STATUS_DONE":"Done","STATUS_PROGRESS":"In Progress"}[args[9]]
    state_path.write_text(json.dumps(state))
    if os.environ.get("QA_LOSE_PROJECT_RESPONSE") == "1":
        marker = state_path.with_name("lost-project-response")
        if not marker.exists(): marker.touch(); raise SystemExit(74)
    out(state["project_item"])
elif args[:1] == ["api"] and len(args) > 1 and args[1] == "graphql":
    query = next((x.split("=",1)[1] for x in args if x.startswith("query=")), "")
    item = state["project_item"]
    if "nodes(ids" in query:
        out({"data":{"nodes":[item]}})
    else:
        out({"data":{"repository":{"issue":{"projectItems":{"pageInfo":{"hasNextPage":False},"nodes":[item]}}}}})
elif args[:1] == ["api"]:
    endpoint = next((x for x in args[1:] if x.startswith("repos/")), "")
    if endpoint == f"repos/fixture/repo/issues/{state['issue']['number']}": out(state["issue"])
    elif endpoint == f"repos/fixture/repo/pulls/{state['pr']['number']}": out(state["pr"])
    elif endpoint == "repos/fixture/repo": out({"full_name":"fixture/repo","default_branch":"main"})
    elif endpoint == "repos/fixture/repo/git/ref/heads/main":
        out({"ref":"refs/heads/main","object":{"sha":state["target_oid"]}})
    elif endpoint == "repos/fixture/repo/compare/" + state["merge_oid"] + "..." + state["target_oid"]:
        advanced = state["merge_oid"] != state["target_oid"]
        out({"url":"https://api.github.com/"+endpoint,
             "status":"ahead" if advanced else "identical",
             "base_commit":{"sha":state["merge_oid"]},
             "merge_base_commit":{"sha":state["merge_oid"]},
             "ahead_by":1 if advanced else 0,"behind_by":0,"total_commits":1 if advanced else 0,
             "commits":[{"sha":state["target_oid"]}] if advanced else []})
    elif endpoint == f"repos/fixture/repo/issues/{state['issue']['number']}/comments": out([state.get("readiness_comments", []) + state["comments"]])
    elif endpoint.startswith("repos/fixture/repo/issues/comments/"):
        identifier = int(endpoint.rsplit("/", 1)[1])
        matches = [c for c in state.get("readiness_comments", []) + state["comments"] if c.get("id") == identifier]
        if len(matches) != 1: raise SystemExit("missing or ambiguous fixture comment")
        out(matches[0])
    elif endpoint == "repos/fixture/repo/collaborators/fixture/permission":
        out({"permission":"admin","user":{"login":"fixture"}})
    else: raise SystemExit("unsupported fixture gh api: " + endpoint)
elif args[:2] == ["issue", "comment"]:
    body = pathlib.Path(args[args.index("--body-file") + 1]).read_text()
    comment_id = 900 + sum("<!-- oasis7-pm-evidence/v2 -->" in c["body"] for c in state["comments"])
    comment = {"id":comment_id,"body":body,
        "html_url":f"https://github.com/fixture/repo/issues/11#issuecomment-{comment_id}",
        "issue_url":"https://api.github.com/repos/fixture/repo/issues/11"}
    if os.environ.get("QA_COMMENT_NO_USER") != "1":
        comment["user"] = {"login":os.environ.get("QA_COMMENT_AUTHOR", "fixture")}
    state["comments"].append(comment); state_path.write_text(json.dumps(state))
    if os.environ.get("QA_LOSE_COMMENT_RESPONSE") == "1":
        marker = pathlib.Path(os.environ["QA_LOST_MARKER"])
        if not marker.exists(): marker.touch(); raise SystemExit(74)
    print(comment["html_url"])
elif args[:2] == ["issue", "close"]:
    state["issue"]["state"] = "CLOSED"
    state["issue"]["state_reason"] = "completed"
    state_path.write_text(json.dumps(state))
    if os.environ.get("QA_LOSE_ISSUE_CLOSE_RESPONSE") == "1":
        marker = pathlib.Path(os.environ["QA_LOST_CLOSE_MARKER"])
        if not marker.exists():
            marker.touch()
            raise SystemExit(74)
elif args[:2] == ["issue", "view"]:
    out({"state":state["issue"]["state"],"state_reason":state["issue"].get("state_reason","")})
else:
    raise SystemExit("unsupported fixture gh command: " + " ".join(args))
'''
        gh = self.bin / "gh"
        gh.write_text(script, encoding="utf-8")
        gh.chmod(0o755)

    def _install_git_network_stub(self):
        real_git = shutil.which("git")
        if not real_git:
            raise AssertionError("test requires Git")
        stub = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
remote_path = pathlib.Path(os.environ["QA_REMOTE_STATE"])
remote = json.loads(remote_path.read_text())
if "ls-remote" in args and args[-1].startswith("refs/heads/"):
    oid = remote.get(args[-1])
    if oid:
        print(oid + "\t" + args[-1])
    raise SystemExit(0)
if "push" in args:
    lease = next((x for x in args if x.startswith("--force-with-lease=")), "")
    if not lease or not args[-1].startswith(":refs/heads/"):
        raise SystemExit("fixture rejected unexpected remote push")
    ref, expected = lease.removeprefix("--force-with-lease=").split(":", 1)
    if args[-1] != ":" + ref or remote.get(ref) != expected:
        raise SystemExit("fixture remote lease mismatch")
    remote.pop(ref)
    remote_path.write_text(json.dumps(remote))
    raise SystemExit(0)
os.execv(os.environ["QA_REAL_GIT"], [os.environ["QA_REAL_GIT"], *args])
'''
        git = self.bin / "git"
        git.write_text(stub, encoding="utf-8")
        git.chmod(0o755)
        self.real_git = real_git

    def install_process_probe(self, mode: str) -> None:
        """Control only ps/lsof readback while leaving producer and cleanup real."""
        if mode not in {"complete-idle", "unavailable"}:
            raise ValueError(f"unsupported process probe fixture mode: {mode}")
        self.process_probe_mode = mode
        ps = self.bin / "ps"
        ps.write_text(r'''#!/usr/bin/env python3
import os, sys
if sys.argv[1:] != ["-axo", "pid=,ppid=,uid=,command="]:
    raise SystemExit("unexpected fixture ps invocation: " + repr(sys.argv[1:]))
try:
    uid = os.getuid() + 1
except AttributeError:
    uid = 1001
print(f"424242 1 {uid} fixture-process-with-path-free-argv")
''', encoding="utf-8")
        ps.chmod(0o755)
        lsof = self.bin / "lsof"
        lsof.write_text(r'''#!/usr/bin/env python3
import os, sys
args = sys.argv[1:]
if args[:3] != ["-n", "-F", "pfnt"] or "-p" not in args:
    raise SystemExit("unexpected fixture lsof invocation: " + repr(args))
if os.environ["QA_PROCESS_PROBE_MODE"] == "unavailable":
    print("fixture denies process readback", file=sys.stderr)
    raise SystemExit(1)
for pid in args[args.index("-p") + 1].split(","):
    print("p" + pid)
    print("fcwd")
    print("tDIR")
    print("n/tmp")
    print("f0")
    print("tCHR")
    print("n/dev/null")
''', encoding="utf-8")
        lsof.chmod(0o755)

    def env(self, *, lose_response: bool = False, lose_close_response: bool = False,
            comment_author: str | None = None,
            comment_no_user: bool = False) -> dict[str, str]:
        env = dict(os.environ)
        for name in ("GH_TOKEN", "GITHUB_TOKEN", "GITHUB_ENTERPRISE_TOKEN"):
            env.pop(name, None)
        env["PATH"] = str(self.bin) + os.pathsep + env.get("PATH", "")
        env["QA_GH_STATE"] = str(self.state_path)
        env["QA_GH_LOG"] = str(self.log_path)
        env["QA_LOST_MARKER"] = str(self.lost_marker)
        env["QA_LOST_CLOSE_MARKER"] = str(self.lost_close_marker)
        env["QA_REAL_GIT"] = self.real_git
        env["QA_REMOTE_BRANCH_OID"] = self.head_oid
        env["QA_REMOTE_STATE"] = str(self.remote_state_path)
        if self.process_probe_mode is None:
            env.pop("QA_PROCESS_PROBE_MODE", None)
        else:
            env["QA_PROCESS_PROBE_MODE"] = self.process_probe_mode
        if lose_response:
            env["QA_LOSE_COMMENT_RESPONSE"] = "1"
        else:
            env.pop("QA_LOSE_COMMENT_RESPONSE", None)
        if lose_close_response:
            env["QA_LOSE_ISSUE_CLOSE_RESPONSE"] = "1"
        else:
            env.pop("QA_LOSE_ISSUE_CLOSE_RESPONSE", None)
        if comment_author is None:
            env.pop("QA_COMMENT_AUTHOR", None)
        else:
            env["QA_COMMENT_AUTHOR"] = comment_author
        if comment_no_user:
            env["QA_COMMENT_NO_USER"] = "1"
        else:
            env.pop("QA_COMMENT_NO_USER", None)
        return env

    def run_producer(self, *extra: str, lose_response: bool = False,
                     lose_close_response: bool = False,
                     comment_author: str | None = None,
                     comment_no_user: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            sys.executable, str(self.pm_tools / "post-merge-finalize.py"),
            "--repo-root", str(self.root), "--task-uid", UID, "--delivery", *extra,
            "--json",
        ], text=True, capture_output=True,
            env=self.env(lose_response=lose_response, lose_close_response=lose_close_response,
                         comment_author=comment_author,
                         comment_no_user=comment_no_user))

    def run_finalizer(self, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            "bash", str(self.pm_tools / "finalize-task.sh"),
            "--repo-root", str(self.root), "--task-uid", UID, "--pr", str(PR),
            "--resume", *extra, "--json",
        ], cwd=self.root, text=True, capture_output=True, env=self.env())

    def run_cleanup(self, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            "bash", str(self.pm_tools / "post-merge-cleanup.sh"),
            "--repo-root", str(self.root), "--task-uid", UID,
            "--delivery", *extra, "--json",
        ], cwd=self.root, text=True, capture_output=True, env=self.env())

    def mapping(self) -> dict:
        return json.loads(self.mapping_path.read_text(encoding="utf-8"))

    def live_inputs(self):
        record = self.mapping()["tasks"][UID]
        state = json.loads(self.state_path.read_text(encoding="utf-8"))
        live_repo = {
            "repository": {"full_name": REPOSITORY, "default_branch": "main"},
            "ref": {"ref": "refs/heads/main", "object": {"sha": state["target_oid"]}},
            "merge_compare": LiveRepositoryCompareTests().compare(state["merge_oid"], state["target_oid"]),
            "observed_target_compare": None,
        }
        return record, state["issue"], state["project_item"], state["pr"], live_repo, state.get("readiness_comments", []) + state["comments"]


class LiveRepositoryCompareTests(unittest.TestCase):
    """Real REST compare shapes, without a fabricated head_commit field."""

    merge = "a" * 40
    target = "b" * 40
    observed = "c" * 40

    def compare(self, base, target):
        advanced = base != target
        return {
            "url": f"https://api.github.com/repos/{REPOSITORY}/compare/{base}...{target}",
            "status": "ahead" if advanced else "identical",
            "base_commit": {"sha": base}, "merge_base_commit": {"sha": base},
            "ahead_by": 1 if advanced else 0, "behind_by": 0,
            "total_commits": 1 if advanced else 0,
            "commits": [{"sha": target}] if advanced else [],
        }

    def bundle(self, target, observed=None):
        return {
            "repository": {"full_name": REPOSITORY, "default_branch": "main"},
            "ref": {"ref": "refs/heads/main", "object": {"sha": target}},
            "merge_compare": self.compare(self.merge, target),
            "observed_target_compare": self.compare(observed, target) if observed else None,
        }

    def validate(self, bundle, observed=None):
        return terminal_proof.validate_live_repository(
            bundle, REPOSITORY, self.merge, default_branch="main",
            observed_target_oid=observed or bundle["ref"]["object"]["sha"],
        )

    def test_identical_real_response_without_head_commit(self):
        self.assertEqual(self.merge, self.validate(self.bundle(self.merge)))

    def test_ahead_real_response_without_head_commit(self):
        self.assertEqual(self.target, self.validate(self.bundle(self.target)))

    def test_observed_target_ahead_real_response_without_head_commit(self):
        self.assertEqual(self.target, self.validate(self.bundle(self.target, self.observed), self.observed))

    def test_observed_target_identical_to_merge_real_response(self):
        self.assertEqual(self.merge, self.validate(self.bundle(self.merge, self.observed), self.observed))

    def test_both_comparisons_reject_mismatched_or_incomplete_readbacks(self):
        mutations = [
            ("url", "https://api.github.com/repos/other/repo/compare/" + self.merge + "..." + self.target),
            ("url", f"https://api.github.com/repos/{REPOSITORY}/compare/{self.merge}...{'d' * 40}"),
            ("url", f"https://api.github.com/repos/{REPOSITORY}/compare/{'d' * 40}...{self.target}"),
            ("url", None), ("url", f"https://api.github.com/repos/{REPOSITORY}/compare/main...main"),
            ("base_commit", {"sha": "d" * 40}), ("base_commit", []),
            ("merge_base_commit", {"sha": "d" * 40}), ("merge_base_commit", None),
            ("status", "behind"), ("status", "diverged"), ("status", "identical"),
            ("status", None), ("ahead_by", 0), ("ahead_by", True), ("ahead_by", "1"),
            ("ahead_by", -1), ("behind_by", 1), ("behind_by", None),
            ("total_commits", 2), ("total_commits", None),
        ]
        for branch in ("merge_compare", "observed_target_compare"):
            for field, value in mutations:
                bundle = self.bundle(self.target, self.observed)
                bundle[branch][field] = value
                with self.subTest(branch=branch, field=field, value=value), self.assertRaises(ValueError):
                    self.validate(bundle, self.observed)
            for value in (None, [], {}, "malformed"):
                bundle = self.bundle(self.target, self.observed)
                bundle[branch] = value
                with self.subTest(branch=branch, value=value), self.assertRaises(ValueError):
                    self.validate(bundle, self.observed)

    def test_identical_rejects_inconsistent_status_and_counts(self):
        for field, value in (("status", "ahead"), ("ahead_by", 1), ("total_commits", 1), ("behind_by", 1)):
            bundle = self.bundle(self.merge)
            bundle["merge_compare"][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(bundle)

    def test_truncated_commits_do_not_supply_target_identity(self):
        bundle = self.bundle(self.target)
        bundle["merge_compare"].update(ahead_by=300, total_commits=300, commits=[{"sha": "d" * 40}])
        self.assertEqual(self.target, self.validate(bundle))

    def test_repository_ref_and_live_target_mismatch_remain_rejected(self):
        mutations = (
            ("repository", {"full_name": "other/repo", "default_branch": "main"}),
            ("repository", {"full_name": REPOSITORY, "default_branch": "other"}),
            ("ref", {"ref": "refs/heads/other", "object": {"sha": self.target}}),
            ("ref", {"ref": "refs/heads/main", "object": {"sha": "d" * 40}}),
            ("ref", {"ref": "refs/heads/main", "object": {"sha": "not-an-oid"}}),
        )
        for field, value in mutations:
            bundle = self.bundle(self.target)
            bundle[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.validate(bundle)

    def test_uncertain_live_readback_remains_rejected(self):
        with mock.patch.object(terminal_proof.subprocess, "check_output", side_effect=OSError("unreadable")):
            with self.assertRaisesRegex(ValueError, "readback failed"):
                terminal_proof.read_live_repository(REPOSITORY, self.merge)

    def test_live_reader_requests_exact_resolved_repository_base_target(self):
        responses = [
            {"full_name": REPOSITORY, "default_branch": "main"},
            {"ref": "refs/heads/main", "object": {"sha": self.target}},
            self.compare(self.merge, self.target), self.compare(self.observed, self.target),
        ]
        with mock.patch.object(terminal_proof.subprocess, "check_output", side_effect=[json.dumps(x) for x in responses]) as api:
            bundle = terminal_proof.read_live_repository(REPOSITORY, self.merge, observed_target_oid=self.observed)
        self.assertEqual(self.target, self.validate(bundle, self.observed))
        self.assertEqual([
            f"repos/{REPOSITORY}", f"repos/{REPOSITORY}/git/ref/heads/main",
            f"repos/{REPOSITORY}/compare/{self.merge}...{self.target}",
            f"repos/{REPOSITORY}/compare/{self.observed}...{self.target}",
        ], [call.args[0][2] for call in api.call_args_list])


class TerminalDeliveryProtocolTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="oasis7-delivery-protocol-")
        self.fixture = DeliveryFixture(pathlib.Path(self.temp.name))
        if "v1" not in self._testMethodName:
            self.fixture.prepare_native_v2_readiness()

    def tearDown(self):
        self.temp.cleanup()

    def read_proof(self):
        record, issue, item, pr, live_repo, comments = self.fixture.live_inputs()
        with mock.patch.dict(os.environ, self.fixture.env(), clear=True):
            return terminal_proof.read_terminal_proof(
                self.fixture.root, UID, record, live_issue=issue,
                live_project_item=item, live_pr=pr, live_repository=live_repo,
                comments=comments,
            )

    def _prepare_existing_delivery_receipt(self, mutate_receipt):
        state_before = copy.deepcopy(json.loads(self.fixture.state_path.read_text(encoding="utf-8")))
        mapping_before = self.fixture.mapping_path.read_bytes()
        receipt_files_before = {
            path.name: path.read_bytes() for path in self.fixture.receipt_root.iterdir()
            if path.is_file()
        }

        created = self.fixture.run_producer()
        self.assertEqual(created.returncode, 0, created.stdout + created.stderr)
        receipt_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        receipt = json.loads(receipt_path.read_bytes())
        mutate_receipt(receipt)
        receipt_before = canonical(receipt) + b"\n"
        receipt_path.write_bytes(receipt_before)

        # Model a crash immediately after the immutable receipt was written:
        # restore every other sink to its exact pre-finalization state.
        self.fixture.state = state_before
        self.fixture._write_state()
        self.fixture.mapping_path.write_bytes(mapping_before)
        for path in self.fixture.receipt_root.iterdir():
            if (path.is_file() and path.name not in receipt_files_before
                    and path.name != receipt_path.name):
                path.unlink()
        for name, raw in receipt_files_before.items():
            (self.fixture.receipt_root / name).write_bytes(raw)
        self.fixture.log_path.write_text("", encoding="utf-8")
        self.fixture.lost_marker.unlink(missing_ok=True)
        self.fixture.lost_close_marker.unlink(missing_ok=True)

        ledger_path = self.fixture.receipt_root / "finalizer-ledger.json"
        tombstone_path = self.fixture.receipt_root / "terminal-tombstone.json"
        snapshot = {
            "issue": state_before["issue"],
            "project": state_before["project_item"],
            "comments": state_before["comments"],
            "mapping": mapping_before,
            "ledger": ledger_path.read_bytes() if ledger_path.exists() else None,
            "tombstone": tombstone_path.read_bytes() if tombstone_path.exists() else None,
            "receipt": receipt_before,
        }
        return snapshot, ledger_path, tombstone_path

    def _assert_no_terminal_effects(self, snapshot, ledger_path, tombstone_path, producer_output):
        state_after = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        values = {
            "Project": (state_after["project_item"], snapshot["project"]),
            "Issue comment": (state_after["comments"], snapshot["comments"]),
            "mapping": (self.fixture.mapping_path.read_bytes(), snapshot["mapping"]),
            "Issue": (state_after["issue"], snapshot["issue"]),
            "ledger": (ledger_path.read_bytes() if ledger_path.exists() else None, snapshot["ledger"]),
            "tombstone": (
                tombstone_path.read_bytes() if tombstone_path.exists() else None,
                snapshot["tombstone"],
            ),
        }
        changed = [name for name, (after, before) in values.items() if after != before]
        receipt_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        receipt_unchanged = receipt_path.read_bytes() == snapshot["receipt"]

        calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        writes = [call for call in calls if (
            call[:2] in (["issue", "comment"], ["issue", "close"])
            or (call[:2] == ["api", "graphql"]
                and any("mutation" in str(argument).lower() for argument in call))
        )]
        self.assertEqual(
            (changed, receipt_unchanged, writes),
            ([], True, []),
            f"terminal side-effect audit failed; changed sinks={changed}; "
            f"receipt_unchanged={receipt_unchanged}; write_calls={writes}; "
            f"producer_output={producer_output.strip()}",
        )

    def test_existing_v2_receipt_extra_key_is_rejected_before_any_terminal_effect(self):
        snapshot, ledger_path, tombstone_path = self._prepare_existing_delivery_receipt(
            lambda receipt: receipt.update({"unexpected_audit_note": "must be rejected"}),
        )

        result = self.fixture.run_producer()
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("terminal delivery receipt closed schema mismatch", output)
        self._assert_no_terminal_effects(snapshot, ledger_path, tombstone_path, output)

    def test_existing_v2_receipt_invalid_observed_at_is_rejected_before_any_terminal_effect(self):
        snapshot, ledger_path, tombstone_path = self._prepare_existing_delivery_receipt(
            lambda receipt: receipt.update({"observed_at": "not-a-timestamp"}),
        )

        result = self.fixture.run_producer()
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        self.assertIn("terminal delivery receipt observation time is invalid", output)
        self._assert_no_terminal_effects(snapshot, ledger_path, tombstone_path, output)

    def _f7_set_project_field(self, field, value):
        state = json.loads(self.fixture.state_path.read_text())
        for node in state["project_item"]["fieldValues"]["nodes"]:
            if node["field"]["name"] == field:
                node["name"] = value
        self.fixture.state_path.write_text(json.dumps(state))

    def _f7_assert_preflight_no_effects(self, expected_success):
        state_before = self.fixture.state_path.read_bytes()
        mapping_before = self.fixture.mapping_path.read_bytes()
        receipts_before = {p.name: p.read_bytes() for p in self.fixture.receipt_root.iterdir() if p.is_file()}
        result = self.fixture.run_producer("--preflight")
        self.assertEqual(self.fixture.state_path.read_bytes(), state_before, result.stderr)
        self.assertEqual(self.fixture.mapping_path.read_bytes(), mapping_before, result.stderr)
        self.assertEqual({p.name: p.read_bytes() for p in self.fixture.receipt_root.iterdir() if p.is_file()}, receipts_before)
        calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertFalse(any(call[:2] in (["issue", "comment"], ["issue", "close"], ["project", "item-edit"]) for call in calls))
        if expected_success:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(json.loads(result.stdout)["status"], "ready")
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def test_f7_real_prephase_projection_preflight_is_readonly(self):
        state = json.loads(self.fixture.state_path.read_text())
        fields = {node["field"]["name"]: node.get("name") for node in state["project_item"]["fieldValues"]["nodes"]}
        self.assertEqual([fields[k] for k in ("Status", "PM Status", "Workflow Phase")], ["In Progress", "done", "done"])
        self._f7_assert_preflight_no_effects(True)

    def test_f7_unjournaled_done_is_not_prephase_authority(self):
        self._f7_set_project_field("Status", "Done")
        self.assertFalse((self.fixture.receipt_root / "finalizer-ledger.json").exists())
        self._f7_assert_preflight_no_effects(False)

    def test_f7_real_project_action_lost_response_resumes_exact_operation(self):
        with mock.patch.dict(os.environ, {"QA_LOSE_PROJECT_RESPONSE": "1"}):
            lost = self.fixture.run_producer()
        self.assertNotEqual(lost.returncode, 0, lost.stdout + lost.stderr)
        self.assertTrue(self.fixture.state_path.with_name("lost-project-response").exists(), lost.stderr)
        ledger = json.loads((self.fixture.receipt_root / "finalizer-ledger.json").read_text())
        entry = ledger["operations"]["project_update"]
        self.assertEqual(ledger["task_uid"], UID)
        self.assertEqual(entry["operation_id"], sha(f"{UID}:post_merge_done:project_update".encode()))
        self.assertEqual(entry["effect"], "project_update")
        self.assertTrue(entry["intent"])
        self.assertTrue(entry["action"])
        self.assertFalse(entry.get("readback", False))
        self.assertEqual(entry["result"]["fields"], ["Status"])
        retry = self.fixture.run_producer()
        self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)
        self.assertEqual(self.read_proof()["protocol_version"], 2)
        calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertEqual(sum(call[:2] == ["project", "item-edit"] for call in calls), 1)

    def test_f7_wrong_prephase_pm_status_refuses_before_effects(self):
        self._f7_set_project_field("PM Status", "committed")
        self._f7_assert_preflight_no_effects(False)

    def test_f7_wrong_prephase_workflow_phase_refuses_before_effects(self):
        self._f7_set_project_field("Workflow Phase", "verification")
        self._f7_assert_preflight_no_effects(False)

    def test_preflight_and_lost_comment_response_recover_one_delivery(self):
        preflight = self.fixture.run_producer("--preflight")
        self.assertEqual(preflight.returncode, 0, preflight.stderr)
        self.assertEqual(json.loads(preflight.stdout)["status"], "ready")
        self.assertFalse((self.fixture.receipt_root / "terminal-delivery-receipt.json").exists())
        self.assertFalse((self.fixture.receipt_root / "finalizer-ledger.json").exists())
        self.assertEqual(len(json.loads(self.fixture.state_path.read_text())["comments"]), 1)

        lost = self.fixture.run_producer(lose_response=True)
        self.assertNotEqual(lost.returncode, 0, "fixture must simulate the lost client response")
        state = json.loads(self.fixture.state_path.read_text())
        self.assertEqual(sum("<!-- oasis7-pm-evidence/v2 -->" in c["body"] for c in state["comments"]), 1)
        mapping = self.fixture.mapping()["tasks"][UID]
        self.assertNotIn("post_merge_done", mapping.get("phase_receipt_type", {}))
        ledger = json.loads((self.fixture.receipt_root / "finalizer-ledger.json").read_text())
        self.assertTrue(ledger["operations"]["evidence_comment"].get("action"))
        self.assertFalse(ledger["operations"]["evidence_comment"].get("committed"))

        retry = self.fixture.run_producer()
        self.assertEqual(retry.returncode, 0, retry.stderr)
        result = json.loads(retry.stdout)
        self.assertEqual((result["status"], result["protocol_version"]), ("finalized", 2))
        self.assertEqual(self.read_proof()["protocol_version"], 2)
        state = json.loads(self.fixture.state_path.read_text())
        self.assertEqual(sum("<!-- oasis7-pm-evidence/v2 -->" in c["body"] for c in state["comments"]), 1)
        self.assertFalse((self.fixture.receipt_root / "main-sync-receipt.json").exists())
        self.assertFalse((self.fixture.receipt_root / "terminal-cleanup-receipt.json").exists())
        self.assertFalse((self.fixture.receipt_root / "resource-cleanup.json").exists())

        again = self.fixture.run_producer()
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual(json.loads(again.stdout)["status"], "already_finalized")
        calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertEqual(sum(call[:2] == ["issue", "comment"] for call in calls), 1)

    def test_lost_issue_close_response_recovers_same_delivery_without_duplicates(self):
        mapping_before_preflight = self.fixture.mapping_path.read_bytes()
        state_before_preflight = self.fixture.state_path.read_bytes()
        receipt_files_before_preflight = {
            path.name: path.read_bytes() for path in self.fixture.receipt_root.iterdir()
            if path.is_file()
        }

        preflight = self.fixture.run_producer("--preflight")
        self.assertEqual(preflight.returncode, 0, preflight.stderr)
        self.assertEqual(json.loads(preflight.stdout)["status"], "ready")
        self.assertEqual(self.fixture.mapping_path.read_bytes(), mapping_before_preflight)
        self.assertEqual(self.fixture.state_path.read_bytes(), state_before_preflight)
        self.assertEqual(
            {path.name: path.read_bytes() for path in self.fixture.receipt_root.iterdir()
             if path.is_file()},
            receipt_files_before_preflight,
        )
        preflight_calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertFalse(any(call[:2] in (["issue", "close"], ["issue", "comment"],
                                          ["project", "item-edit"])
                             for call in preflight_calls), preflight_calls)

        lost = self.fixture.run_producer(lose_close_response=True)
        self.assertNotEqual(lost.returncode, 0, lost.stdout + lost.stderr)
        self.assertTrue(self.fixture.lost_close_marker.exists())

        after_lost = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        self.assertEqual(after_lost["issue"]["state"], "CLOSED")
        self.assertEqual(after_lost["issue"]["state_reason"], "completed")
        terminal_comments = [comment for comment in after_lost["comments"]
                             if "<!-- oasis7-pm-evidence/v2 -->" in comment["body"]]
        self.assertEqual(len(terminal_comments), 1, terminal_comments)
        comment_after_lost = copy.deepcopy(terminal_comments[0])

        record_after_lost = self.fixture.mapping()["tasks"][UID]
        self.assertEqual(record_after_lost["workflow_phase"], "post_merge_done")
        receipt_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        receipt_bytes_after_lost = receipt_path.read_bytes()
        receipt_sha_after_lost = sha(receipt_bytes_after_lost)
        self.assertEqual(record_after_lost["phase_receipt_type"]["post_merge_done"],
                         "oasis7_terminal_delivery")
        self.assertEqual(record_after_lost["phase_receipt_sha256"]["post_merge_done"],
                         receipt_sha_after_lost)
        self.assertFalse((self.fixture.receipt_root / "terminal-tombstone.json").exists())

        ledger_path = self.fixture.receipt_root / "finalizer-ledger.json"
        ledger_after_lost = json.loads(ledger_path.read_text(encoding="utf-8"))
        close_after_lost = ledger_after_lost["operations"]["issue_close"]
        expected_close_id = hashlib.sha256(f"{UID}:post_merge_done:issue_close".encode()).hexdigest()
        self.assertEqual(close_after_lost["operation_id"], expected_close_id)
        self.assertEqual(close_after_lost["effect"], "issue_close")
        self.assertTrue(close_after_lost.get("intent"), close_after_lost)
        self.assertTrue(close_after_lost.get("action"), close_after_lost)
        self.assertFalse(close_after_lost.get("readback"), close_after_lost)
        self.assertFalse(close_after_lost.get("committed"), close_after_lost)
        calls_after_lost = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertEqual(sum(call[:2] == ["issue", "close"] for call in calls_after_lost), 1)
        self.assertEqual(sum(call[:2] == ["issue", "comment"] for call in calls_after_lost), 1)

        retry = self.fixture.run_producer()
        self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)
        self.assertEqual(json.loads(retry.stdout)["status"], "finalized")
        self.assertEqual(self.read_proof()["protocol_version"], 2)

        recovered_state = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        self.assertEqual(recovered_state["issue"]["state"], "CLOSED")
        self.assertEqual(recovered_state["issue"]["state_reason"], "completed")
        recovered_terminal_comments = [comment for comment in recovered_state["comments"]
                                       if "<!-- oasis7-pm-evidence/v2 -->" in comment["body"]]
        self.assertEqual(recovered_terminal_comments, [comment_after_lost])
        self.assertEqual(receipt_path.read_bytes(), receipt_bytes_after_lost)
        recovered_record = self.fixture.mapping()["tasks"][UID]
        self.assertEqual(recovered_record["phase_receipt_sha256"]["post_merge_done"],
                         receipt_sha_after_lost)

        tombstone_path = self.fixture.receipt_root / "terminal-tombstone.json"
        tombstone = json.loads(tombstone_path.read_text(encoding="utf-8"))
        self.assertEqual(tombstone["terminal_receipt_sha256"], receipt_sha_after_lost)
        recovered_ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        recovered_close = recovered_ledger["operations"]["issue_close"]
        self.assertEqual(recovered_close["operation_id"], expected_close_id)
        self.assertTrue(recovered_close.get("readback"), recovered_close)
        self.assertTrue(recovered_close.get("committed"), recovered_close)
        self.assertEqual(recovered_close.get("result", {}).get("state"), "CLOSED")
        self.assertEqual(recovered_close.get("result", {}).get("state_reason"), "completed")

        already_finalized = self.fixture.run_producer()
        self.assertEqual(already_finalized.returncode, 0,
                         already_finalized.stdout + already_finalized.stderr)
        self.assertEqual(json.loads(already_finalized.stdout)["status"], "already_finalized")
        final_state = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        final_terminal_comments = [comment for comment in final_state["comments"]
                                  if "<!-- oasis7-pm-evidence/v2 -->" in comment["body"]]
        self.assertEqual(final_terminal_comments, [comment_after_lost])
        final_calls = [json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
        self.assertEqual(sum(call[:2] == ["issue", "close"] for call in final_calls), 1)
        self.assertEqual(sum(call[:2] == ["issue", "comment"] for call in final_calls), 1)

    def test_shared_reader_rejects_selector_digest_comment_claim_and_tombstone_drift(self):
        environment = mock.patch.dict(os.environ, self.fixture.env(), clear=True)
        environment.start()
        self.addCleanup(environment.stop)
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        self.assertEqual(self.read_proof()["status"], "passed")
        inputs = self.fixture.live_inputs()
        record = inputs[0]

        unknown = copy.deepcopy(record)
        unknown["phase_receipt_type"]["post_merge_done"] = "oasis7_terminal_delivery_v9"
        with self.assertRaisesRegex(ValueError, "terminal delivery protocol selector mismatch"):
            terminal_proof.read_terminal_proof(self.fixture.root, UID, unknown,
                live_issue=inputs[1], live_project_item=inputs[2], live_pr=inputs[3],
                live_repository=inputs[4], comments=inputs[5])

        stale = copy.deepcopy(record)
        stale["phase_receipt_sha256"]["post_merge_done"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "terminal delivery receipt digest mismatch"):
            terminal_proof.read_terminal_proof(self.fixture.root, UID, stale,
                live_issue=inputs[1], live_project_item=inputs[2], live_pr=inputs[3],
                live_repository=inputs[4], comments=inputs[5])

        delivery_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        original = delivery_path.read_bytes()
        mutated_receipt = json.loads(original)
        mutated_receipt["cleanup_state"] = "released"
        redigested = canonical(mutated_receipt) + b"\n"
        delivery_path.write_bytes(redigested)
        redigested_record = copy.deepcopy(record)
        redigested_record["phase_receipt_sha256"]["post_merge_done"] = sha(redigested)
        redigested_record["phase_receipts"]["post_merge_done"] = mutated_receipt
        with self.assertRaisesRegex(ValueError, "terminal delivery receipt closed schema mismatch"):
            terminal_proof.read_terminal_proof(self.fixture.root, UID, redigested_record,
                live_issue=inputs[1], live_project_item=inputs[2], live_pr=inputs[3],
                live_repository=inputs[4], comments=inputs[5])
        delivery_path.write_bytes(original)

        with self.assertRaisesRegex(ValueError, "terminal delivery comment readback mismatch"):
            terminal_proof.read_terminal_proof(self.fixture.root, UID, record,
                live_issue=inputs[1], live_project_item=inputs[2], live_pr=inputs[3],
                live_repository=inputs[4], comments=inputs[5][:-1])

        no_claim = copy.deepcopy(record)
        no_claim["claim_verifications"] = []
        no_claim_issue = copy.deepcopy(inputs[1])
        history_marker = "claim_verifications_b64: " + TICK
        old_history = no_claim_issue["body"].split(history_marker, 1)[1].split(TICK, 1)[0]
        empty_history = base64.urlsafe_b64encode(canonical([])).decode().rstrip("=")
        no_claim_issue["body"] = no_claim_issue["body"].replace(old_history, empty_history)
        with self.assertRaisesRegex(ValueError,
                "terminal delivery accepted task_complete claim is missing, ambiguous, or invalid"):
            terminal_proof.read_terminal_proof(self.fixture.root, UID, no_claim,
                live_issue=no_claim_issue, live_project_item=inputs[2], live_pr=inputs[3],
                live_repository=inputs[4], comments=inputs[5])

        tombstone = self.fixture.receipt_root / "terminal-tombstone.json"
        saved = tombstone.read_bytes()
        try:
            value = json.loads(saved)
            value["checkout_recreation_forbidden"] = False
            tombstone.write_bytes(canonical(value) + b"\n")
            with self.assertRaisesRegex(ValueError, "terminal finalizer ledger or tombstone mismatch"):
                self.read_proof()
        finally:
            tombstone.write_bytes(saved)

    def test_shared_reader_requires_repository_owner_as_v2_comment_author(self):
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        state = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        matching = [comment for comment in state["comments"]
                    if "<!-- oasis7-pm-evidence/v2 -->" in comment.get("body", "")]
        self.assertEqual(len(matching), 1, state["comments"])
        self.assertEqual(matching[0].get("user", {}).get("login"), REPOSITORY.split("/", 1)[0])

        for label, author in (("wrong", {"login": "untrusted-user"}), ("missing", None)):
            with self.subTest(author=label):
                invalid = copy.deepcopy(state)
                comment = next(c for c in invalid["comments"]
                               if "<!-- oasis7-pm-evidence/v2 -->" in c.get("body", ""))
                if author is None:
                    comment.pop("user", None)
                else:
                    comment["user"] = author
                self.fixture.state_path.write_text(json.dumps(invalid), encoding="utf-8")
                try:
                    with self.assertRaisesRegex(ValueError, "author"):
                        self.read_proof()
                finally:
                    self.fixture.state_path.write_text(json.dumps(state), encoding="utf-8")

    def test_producer_rejects_wrong_or_missing_terminal_comment_author(self):
        for label, options in (
            ("wrong", {"comment_author": "untrusted-user"}),
            ("missing", {"comment_no_user": True}),
        ):
            with self.subTest(author=label):
                with tempfile.TemporaryDirectory(prefix="oasis7-delivery-author-") as scratch:
                    fixture = DeliveryFixture(pathlib.Path(scratch))
                    fixture.prepare_native_v2_readiness()
                    result = fixture.run_producer(**options)
                    self.assertNotEqual(result.returncode, 0,
                                        "producer accepted a terminal comment without repository-owner provenance")
                    mapping = fixture.mapping()["tasks"][UID]
                    state = json.loads(fixture.state_path.read_text(encoding="utf-8"))
                    self.assertEqual(
                        ("post_merge_done" in mapping.get("phase_receipt_type", {}), state["issue"]["state"]),
                        (False, "OPEN"),
                        "author mismatch was detected only after terminal state mutation; "
                        f"producer stdout/stderr: {result.stdout}{result.stderr}",
                    )
                    self.assertEqual(sum("<!-- oasis7-pm-evidence/v2 -->" in c.get("body", "")
                                         for c in state["comments"]), 1)

    def test_producer_result_and_public_wrapper_preflight_expose_only_read_back_delivery(self):
        pending = self.fixture.run_finalizer("--preflight")
        self.assertEqual(pending.returncode, 0, pending.stdout + pending.stderr)
        pending_json = json.loads(pending.stdout)
        self.assertEqual(pending_json["status"], "ready")
        self.assertNotEqual((pending_json.get("delivery") or {}).get("state"), "complete",
                            "task_done alone must not be reported as terminal delivery")

        producer_pending = self.fixture.run_producer("--preflight")
        self.assertEqual(producer_pending.returncode, 0,
                         producer_pending.stdout + producer_pending.stderr)
        producer_pending_json = json.loads(producer_pending.stdout)
        self.assertEqual(
            ((producer_pending_json.get("delivery") or {}).get("state"),
             (producer_pending_json.get("delivery") or {}).get("protocol_version")),
            ("pending", 2),
            producer_pending_json)
        self.assertFalse((self.fixture.receipt_root / "terminal-delivery-receipt.json").exists())
        self.assertNotIn("post_merge_done",
                         self.fixture.mapping()["tasks"][UID].get("phase_receipt_type", {}))

        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        producer_json = json.loads(produced.stdout)
        wrapped_delivery = producer_json.get("delivery") or {}
        self.assertEqual((wrapped_delivery.get("state"), wrapped_delivery.get("protocol_version")),
                         ("complete", 2), producer_json)

        ready = self.fixture.run_finalizer("--preflight")
        self.assertEqual(ready.returncode, 0, ready.stderr)
        ready_json = json.loads(ready.stdout)
        self.assertEqual(ready_json["status"], "ready", ready_json)
        self.assertNotEqual((ready_json.get("delivery") or {}).get("state"), "complete",
                            "preflight must not independently overstate delivery state")

    def test_public_wrapper_defer_and_cleanup_only_resume_without_redelivery(self):
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        delivery_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        delivery_raw = delivery_path.read_bytes()
        mapping_raw = self.fixture.mapping_path.read_bytes()
        before_defer = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        evidence = [comment for comment in before_defer["comments"]
                    if "<!-- oasis7-pm-evidence/v2 -->" in comment.get("body", "")]
        self.assertEqual(len(evidence), 1)

        deferred = self.fixture.run_finalizer("--cleanup=defer")
        self.assertEqual(deferred.returncode, 0, deferred.stdout + deferred.stderr)
        deferred_json = json.loads(deferred.stdout)
        self.assertEqual(deferred_json["delivery"], {"state": "complete", "protocol_version": 2})
        self.assertEqual(deferred_json["cleanup_state"], "cleanup_deferred")
        self.assertFalse((self.fixture.receipt_root / "resource-cleanup.json").exists())

        before_cleanup = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        self.fixture.install_process_probe("complete-idle")
        cleanup_only = self.fixture.run_finalizer("--cleanup-only")
        self.assertEqual(cleanup_only.returncode, 0, cleanup_only.stdout + cleanup_only.stderr)
        cleanup_json = json.loads(cleanup_only.stdout)
        self.assertEqual(cleanup_json["delivery"], {"state": "complete", "protocol_version": 2})
        self.assertEqual(cleanup_json["cleanup_state"], "cleanup_complete", cleanup_json)
        self.assertEqual(self.fixture.mapping_path.read_bytes(), mapping_raw)
        self.assertEqual(delivery_path.read_bytes(), delivery_raw)
        after_cleanup = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        self.assertEqual(after_cleanup["issue"], before_cleanup["issue"])
        self.assertEqual(after_cleanup["comments"], before_cleanup["comments"])
        self.assertFalse(self.fixture.task.exists())
        branch = subprocess.run([self.fixture.real_git, "-C", str(self.fixture.root),
                                 "rev-parse", "--verify", "refs/heads/task/protocol-fixture"],
                                text=True, capture_output=True)
        self.assertNotEqual(branch.returncode, 0, branch.stdout + branch.stderr)
        self.assertEqual(json.loads(self.fixture.remote_state_path.read_text(encoding="utf-8")), {})

    def test_public_wrapper_resumes_task_done_before_selector_without_recompletion(self):
        # This is the durable checkpoint after closeout and cache refresh, but
        # before the delivery producer has selected its terminal protocol.
        # A forbidden-closeout sentinel fails rather than pretending repeated
        # completion succeeded; native readiness and delivery stay real.
        fixture = self.fixture
        original_claims = copy.deepcopy(fixture.mapping()["tasks"][UID]["claim_verifications"])
        self.assertEqual(len(original_claims), 1)
        self.assertEqual(original_claims[0]["frozen_source_head"], fixture.head_oid)
        receipt_path = fixture.receipt_root / "merge-receipt.json"
        original_receipt = receipt_path.read_bytes()
        self.assertFalse((fixture.receipt_root / "terminal-delivery-receipt.json").exists())
        self.assertEqual(fixture.mapping()["tasks"][UID]["workflow_phase"], "task_done")

        sentinel = pathlib.Path(self.temp.name) / "forbidden-closeout"
        closeout = fixture.pm_tools / "task-closeout.sh"
        closeout.write_text(
            "#!/usr/bin/env bash\nset -euo pipefail\n"
            + "printf '%s\\n' 'completion was already accepted at task_done' > "
            + shlex.quote(str(sentinel)) + "\n"
            + "echo 'forbidden task_done re-completion' >&2\nexit 86\n", encoding="utf-8")
        closeout.chmod(0o755)

        # Supply only metadata for the real receipt helper if the faulty
        # wrapper calls it. Do not synthesize or normalize its resulting bytes.
        original_gh = fixture.bin / "gh-native"
        (fixture.bin / "gh").rename(original_gh)
        (fixture.bin / "gh").write_text(r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
state = json.loads(pathlib.Path(os.environ["QA_GH_STATE"]).read_text())
if args[:2] == ["pr", "view"]:
    pr = state["pr"]
    print(json.dumps({"number":pr["number"],"url":pr["html_url"],"state":"MERGED",
        "mergedAt":pr["merged_at"],"headRefOid":pr["head"]["sha"],"baseRefName":pr["base"]["ref"]}))
elif args[:2] == ["repo", "view"]:
    print(json.dumps({"nameWithOwner":"fixture/repo","defaultBranchRef":{"name":"main"}}))
else:
    original = pathlib.Path(__file__).with_name("gh-native")
    os.execv(str(original), [str(original), *args])
''', encoding="utf-8")
        (fixture.bin / "gh").chmod(0o755)

        # Finish fixture source setup before invoking the real public wrapper.
        # The accepted H/T objects remain unchanged and available to readers.
        (fixture.root / ".gitignore").write_text(".pm/\n__pycache__/\n", encoding="utf-8")
        fixture._git("add", ".", cwd=fixture.root)
        fixture._git("commit", "-qm", "complete task_done recovery fixture tools", cwd=fixture.root)
        self.assertEqual(fixture._git("status", "--porcelain", cwd=fixture.root), "")

        for attempt in (1, 2):
            resumed = fixture.run_finalizer("--cleanup=defer")
            with self.subTest(attempt=attempt, invariant="accepted merge receipt"):
                self.assertEqual(receipt_path.read_bytes(), original_receipt,
                    "public resume replaced accepted merge receipt before delivery; " + resumed.stderr)
            with self.subTest(attempt=attempt, invariant="accepted completion"):
                self.assertEqual(fixture.mapping()["tasks"][UID]["claim_verifications"], original_claims)
                self.assertFalse(sentinel.exists(), "public resume invoked forbidden task_done re-completion")
            self.assertEqual(resumed.returncode, 0, resumed.stdout + resumed.stderr)
            self.assertEqual(json.loads(resumed.stdout)["delivery"],
                             {"state":"complete", "protocol_version":2})
            self.assertEqual(self.read_proof()["status"], "passed")

    def test_public_cleanup_preflight_consumes_the_same_strict_delivery_proof(self):
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        self.fixture.install_process_probe("complete-idle")
        cleanup = self.fixture.run_cleanup("--preflight")
        self.assertEqual(cleanup.returncode, 0, cleanup.stdout + cleanup.stderr)
        payload = json.loads(cleanup.stdout)
        self.assertEqual(payload["status"], "ready", payload)
        self.assertEqual(payload["delivery"]["state"], "complete", payload)
        self.assertEqual({row["state"] for row in payload["resources"]}, {"ready"}, payload)

    def test_unknown_process_coverage_defers_cleanup_without_revoking_delivery(self):
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        delivery_path = self.fixture.receipt_root / "terminal-delivery-receipt.json"
        delivery_raw = delivery_path.read_bytes()
        proof_before = self.read_proof()
        self.assertEqual(proof_before["status"], "passed", proof_before)

        deferred = self.fixture.run_finalizer("--cleanup=defer")
        self.assertEqual(deferred.returncode, 0, deferred.stdout + deferred.stderr)
        self.assertEqual(json.loads(deferred.stdout)["delivery"],
                         {"state": "complete", "protocol_version": 2})

        self.fixture.install_process_probe("unavailable")
        cleanup_only = self.fixture.run_finalizer("--cleanup-only")
        self.assertEqual(cleanup_only.returncode, 3, cleanup_only.stdout + cleanup_only.stderr)
        payload = json.loads(cleanup_only.stdout)
        self.assertEqual(payload["delivery"], {"state": "complete", "protocol_version": 2}, payload)
        self.assertEqual(payload["cleanup_state"], "cleanup_deferred", payload)
        self.assertTrue(self.fixture.task.is_dir())
        self.assertEqual(delivery_path.read_bytes(), delivery_raw)
        proof_after = self.read_proof()
        self.assertEqual(proof_after["status"], "passed", proof_after)
        self.assertEqual(proof_after, proof_before)

    def install_selected_v1_fixture(self):
        """Install the accepted legacy proof chain in the disposable fixture."""
        legacy_path = ROOT / "scripts/pm/loop_terminal.test.py"
        spec = importlib.util.spec_from_file_location("legacy_wrapper_fixture", legacy_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        names = {
            "merge": "merge-receipt.json", "main_sync": "main-sync-receipt.json",
            "terminal": "terminal-cleanup-receipt.json", "ledger": "finalizer-ledger.json",
            "tombstone": "terminal-tombstone.json",
        }
        receipts = copy.deepcopy(module.RECEIPTS)
        ledger = receipts["ledger"]["record"]
        ledger["operations"]["evidence_comment"]["result"] = module.COMMENT["html_url"]
        receipts["ledger"] = module._receipt(ledger)
        terminal = receipts["terminal"]["record"]
        terminal["worktree"] = str(self.fixture.task)
        terminal["branch"] = self.fixture.record["task_branch"]
        receipts["terminal"] = module._receipt(terminal)
        tombstone = receipts["tombstone"]["record"]
        tombstone["terminal_receipt_sha256"] = receipts["terminal"]["digest"]
        tombstone["canonical_worktree"] = str(self.fixture.task)
        tombstone["task_branch"] = self.fixture.record["task_branch"]
        receipts["tombstone"] = module._receipt(tombstone)
        before = {filename: receipts[key]["bytes"] for key, filename in names.items()}
        for filename, raw in before.items():
            (self.fixture.receipt_root / filename).write_bytes(raw)
        record = copy.deepcopy(self.fixture.record)
        record["workflow_phase"] = "post_merge_done"
        record["phase_receipts"] = {"post_merge_done": receipts["terminal"]["record"]}
        record.setdefault("phase_receipt_sha256", {})["post_merge_done"] = receipts["terminal"]["digest"]
        mapping = self.fixture.mapping()
        record["merge_receipt"] = receipts["merge"]["record"]
        record["merge_receipt_sha256"] = receipts["merge"]["digest"]
        record["phase_receipts"] = {
            "main_sync": receipts["main_sync"]["record"],
            "post_merge_done": receipts["terminal"]["record"],
        }
        record["phase_receipt_sha256"] = {
            "main_sync": receipts["main_sync"]["digest"],
            "post_merge_done": receipts["terminal"]["digest"],
        }
        record["project_item_id"] = module.ITEM["id"]
        mapping["tasks"][UID] = record
        mapping["project"].update(id=module.ITEM["project"]["id"])
        self.fixture.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        state = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        from terminal_proof import receipt_chain_digest
        issue = copy.deepcopy(module.ISSUE)
        pr = copy.deepcopy(module.PR)
        comment = copy.deepcopy(module.COMMENT)
        chain_before = receipt_chain_digest(UID, REPOSITORY, ISSUE, PR, PR_URL,
            module.RECEIPTS["merge"]["digest"], module.RECEIPTS["main_sync"]["digest"],
            module.RECEIPTS["terminal"]["digest"])
        chain_after = receipt_chain_digest(UID, REPOSITORY, ISSUE, PR, PR_URL,
            receipts["merge"]["digest"], receipts["main_sync"]["digest"], receipts["terminal"]["digest"])
        comment["body"] = comment["body"].replace(chain_before, chain_after).replace(
            module.RECEIPTS["terminal"]["digest"], receipts["terminal"]["digest"])
        comment["id"] = 7
        state.update(issue=issue, pr=pr, project_item=module.ITEM, comments=[comment])
        state["merge_oid"] = module.PR["merge_commit_sha"]
        state["target_oid"] = state["merge_oid"]
        self.fixture.state_path.write_text(json.dumps(state), encoding="utf-8")
        return module, receipts, before, record, issue, pr, comment

    def test_selected_v1_wrapper_resume_preserves_all_historical_receipt_and_comment_bytes(self):
        module, receipts, before, record, issue, pr, comment = self.install_selected_v1_fixture()
        names = {
            "merge": "merge-receipt.json", "main_sync": "main-sync-receipt.json",
            "terminal": "terminal-cleanup-receipt.json", "ledger": "finalizer-ledger.json",
            "tombstone": "terminal-tombstone.json",
        }

        # Bind the exact existing v1 tombstone bytes into a real aggregate-v1
        # child projection before exercising the public resume path.
        v1_record = self.fixture.mapping()["tasks"][UID]
        v1_proof = terminal_proof.read_terminal_proof(
            self.fixture.root, UID, v1_record, live_issue=issue,
            live_project_item=module.ITEM, live_pr=pr, live_repository={}, comments=[comment],
        )
        self.assertEqual((v1_proof["status"], v1_proof["protocol_version"]), ("passed", 1))
        aggregate_tests = importlib.util.spec_from_file_location(
            "aggregate_v1_resume_fixture", ROOT / "scripts/pm/aggregate-task-completion.test.py",
        )
        aggregate_module = importlib.util.module_from_spec(aggregate_tests)
        aggregate_tests.loader.exec_module(aggregate_module)
        aggregate_module.AggregateTaskCompletionTests.setUpClass()
        aggregate_fixture = aggregate_module.AggregateTaskCompletionTests()
        aggregate_values = list(aggregate_fixture.context())
        plan, candidate, evidence, coordinator, plan_comment, reports = aggregate_values
        plan["repository"] = REPOSITORY
        first_delivery, second_delivery = plan["required_deliveries"]
        first_delivery.update(issue_number=ISSUE, pr_number=PR, pr_url=PR_URL)
        second_delivery["pr_url"] = f"https://github.com/{REPOSITORY}/pull/{second_delivery['pr_number']}"
        plan_comment_body = aggregate_module.comment_body(plan)
        plan_comment.update(body=plan_comment_body)
        coordinator.update(
            repository=REPOSITORY,
            body=(
                "<!-- oasis7-pm-task -->\n"
                f"task_uid: {plan['task_uid']}\n"
                "completion_mode: ordered_delivery_aggregate\n"
                "aggregate_plan_comment_id: 6001\n"
                f"aggregate_plan_sha256: sha256:{sha(plan_comment_body.encode())}\n"
            ),
        )
        first_report = reports[UID]
        first_report["task"].update(
            repository=REPOSITORY, issue_number=ISSUE, pr_number=PR, pr_url=PR_URL,
            claim_verifications=record["claim_verifications"],
        )
        first_report["live"].update(
            issue={"number": ISSUE, "state": "CLOSED"},
            pr={
                "number": PR, "url": PR_URL, "state": "MERGED", "base_ref": "main",
                "head_oid": module.PR["head"]["sha"],
                "merge_commit_oid": module.PR["merge_commit_sha"],
                "merged_at": module.PR["merged_at"],
            },
        )
        first_report["checks"].update({
            "completion_route_identity": True,
            "terminal_delivery_proof_valid": True,
        })
        first_report["proof"] = {
            "status": "passed", "protocol_version": 1,
            "task_complete_claim_sha256": "sha256:" + aggregate_module.digest(record["claim_verifications"][-1]),
            "merge_receipt_sha256": v1_proof["merge_receipt_sha256"],
            "main_sync_receipt_sha256": sha(receipts["main_sync"]["bytes"]),
            "terminal_receipt_sha256": v1_proof["terminal_receipt_sha256"],
            "terminal_comment_sha256": v1_proof["comment_sha256"],
            "finalizer_ledger_sha256": v1_proof["finalizer_ledger_sha256"],
            "terminal_tombstone_sha256": v1_proof["tombstone_sha256"],
        }
        second_report = reports[second_delivery["task_uid"]]
        second_report["task"]["repository"] = REPOSITORY
        second_report["task"]["pr_url"] = second_delivery["pr_url"]
        second_report["live"]["pr"]["url"] = second_delivery["pr_url"]
        previous_repository = aggregate_module.AggregateTaskCompletionTests.helper.REPOSITORY
        aggregate_module.AggregateTaskCompletionTests.helper.REPOSITORY = REPOSITORY
        try:
            aggregate_receipt = aggregate_fixture.build(tuple(aggregate_values))
        finally:
            aggregate_module.AggregateTaskCompletionTests.helper.REPOSITORY = previous_repository
        self.assertEqual(aggregate_receipt["schema"], "oasis7.aggregate-task-completion/v1")
        projected_tombstone = aggregate_receipt["deliveries"][0]["terminal_tombstone_sha256"]
        self.assertEqual(projected_tombstone, v1_proof["tombstone_sha256"])

        first = self.fixture.run_finalizer()
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        self.assertEqual(json.loads(first.stdout)["delivery"], {"state": "complete", "protocol_version": 1})
        second = self.fixture.run_finalizer()
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        self.assertEqual(json.loads(second.stdout)["delivery"], {"state": "complete", "protocol_version": 1})
        final_state = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
        legacy = [comment for comment in final_state["comments"]
                  if "<!-- oasis7-pm-evidence -->" in comment.get("body", "")]
        self.assertEqual(len(legacy), 1, legacy)
        self.assertEqual(legacy[0]["body"], comment["body"])
        self.assertFalse((self.fixture.receipt_root / "terminal-delivery-receipt.json").exists())
        final_record = self.fixture.mapping()["tasks"][UID]
        self.assertEqual(final_record["phase_receipt_sha256"]["post_merge_done"],
                         receipts["terminal"]["digest"])
        for filename, raw in before.items():
            observed = (self.fixture.receipt_root / filename).read_bytes()
            if filename == "finalizer-ledger.json":
                before_ledger, after_ledger = json.loads(raw), json.loads(observed)
                for key in ("schema", "task_uid", "repository"):
                    if key in before_ledger:
                        self.assertEqual(after_ledger.get(key), before_ledger[key], key)
                before_ops = before_ledger["operations"]
                after_ops = after_ledger["operations"]
                self.assertTrue(set(before_ops).issubset(after_ops))
                for operation, prior in before_ops.items():
                    current = after_ops[operation]
                    for key in ("effect", "operation_id", "committed"):
                        if key in prior:
                            self.assertEqual(current.get(key), prior[key], f"{operation}.{key}")
                self.assertEqual(after_ops["evidence_comment"]["result"], comment["html_url"])
                self.assertTrue(after_ops["issue_close"]["intent"])
                self.assertTrue(after_ops["issue_close"]["readback"])
                self.assertEqual(after_ops["issue_close"]["result"], {
                    "state": "closed", "state_reason": "completed",
                })
                self.assertTrue(after_ops["project_update"]["intent"])
                self.assertTrue(after_ops["project_update"]["readback"])
                self.assertEqual(after_ops["project_update"]["result"], {
                    "PM Status": "done", "Status": "Done", "Workflow Phase": "done",
                })
            else:
                self.assertEqual(observed, raw, filename)
        self.assertEqual(hashlib.sha256(
            (self.fixture.receipt_root / "terminal-tombstone.json").read_bytes(),
        ).hexdigest(), projected_tombstone)

    def test_selected_v1_resume_rejects_malformed_or_conflicting_tombstone_before_effects(self):
        original_fixture = self.fixture
        try:
            for case, diagnostic in (
                ("malformed", "terminal tombstone is malformed"),
                ("conflicting", "terminal tombstone identity or receipt link mismatch"),
            ):
                with self.subTest(case=case), tempfile.TemporaryDirectory(
                    prefix=f"oasis7-v1-tombstone-{case}-",
                ) as temp:
                    self.fixture = DeliveryFixture(pathlib.Path(temp))
                    self.install_selected_v1_fixture()
                    tombstone_path = self.fixture.receipt_root / "terminal-tombstone.json"
                    if case == "malformed":
                        corrupted = b'{"schema": "oasis7_terminal_tombstone_v1",\n'
                    else:
                        tombstone = json.loads(tombstone_path.read_text(encoding="utf-8"))
                        tombstone["task_uid"] = "task_" + "b" * 32
                        corrupted = json.dumps(tombstone, sort_keys=True).encode("utf-8") + b"\n"
                    tombstone_path.write_bytes(corrupted)

                    mapping_before = self.fixture.mapping_path.read_bytes()
                    state_before = self.fixture.state_path.read_bytes()
                    remote_before = self.fixture.remote_state_path.read_bytes()
                    receipt_names = (
                        "merge-receipt.json", "main-sync-receipt.json",
                        "terminal-cleanup-receipt.json", "finalizer-ledger.json",
                        "terminal-tombstone.json",
                    )
                    receipts_before = {
                        name: (self.fixture.receipt_root / name).read_bytes()
                        for name in receipt_names
                    }
                    result = self.fixture.run_finalizer()
                    self.assertNotEqual(result.returncode, 0, result.stdout)
                    self.assertIn(diagnostic, result.stderr + result.stdout)
                    self.assertEqual(self.fixture.mapping_path.read_bytes(), mapping_before)
                    self.assertEqual(self.fixture.state_path.read_bytes(), state_before)
                    self.assertEqual(self.fixture.remote_state_path.read_bytes(), remote_before)
                    for name, raw in receipts_before.items():
                        self.assertEqual((self.fixture.receipt_root / name).read_bytes(), raw, name)
                    self.assertFalse((self.fixture.receipt_root / "terminal-delivery-receipt.json").exists())
                    self.assertTrue(self.fixture.task.exists(), "guard failure must not clean the task worktree")
                    calls = ([json.loads(line) for line in self.fixture.log_path.read_text().splitlines()]
                             if self.fixture.log_path.exists() else [])
                    self.assertFalse(any(call[:2] in (["issue", "close"], ["issue", "comment"])
                                         for call in calls), calls)
        finally:
            self.fixture = original_fixture

    def test_selected_v1_resume_canonically_creates_only_a_missing_tombstone(self):
        original_fixture = self.fixture
        try:
            with tempfile.TemporaryDirectory(prefix="oasis7-v1-tombstone-missing-") as temp:
                self.fixture = DeliveryFixture(pathlib.Path(temp))
                _module, receipts, _before, record, _issue, _pr, comment = self.install_selected_v1_fixture()
                tombstone_path = self.fixture.receipt_root / "terminal-tombstone.json"
                tombstone_path.unlink()
                mapping_before = self.fixture.mapping_path.read_bytes()
                comment_before = comment["body"]
                immutable_names = (
                    "merge-receipt.json", "main-sync-receipt.json", "terminal-cleanup-receipt.json",
                )
                immutable_before = {
                    name: (self.fixture.receipt_root / name).read_bytes()
                    for name in immutable_names
                }

                first = self.fixture.run_finalizer()
                self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                self.assertEqual(json.loads(first.stdout)["delivery"], {
                    "state": "complete", "protocol_version": 1,
                })
                expected = {
                    "schema": "oasis7_terminal_tombstone_v1",
                    "task_uid": UID,
                    "repository": REPOSITORY,
                    "issue_number": ISSUE,
                    "pr_number": PR,
                    "canonical_worktree": str(self.fixture.task),
                    "task_branch": self.fixture.record["task_branch"],
                    "workflow_phase": "post_merge_done",
                    "terminal_receipt_sha256": receipts["terminal"]["digest"],
                    "checkout_recreation_forbidden": True,
                }
                first_bytes = tombstone_path.read_bytes()
                self.assertEqual(json.loads(first_bytes), expected)
                self.assertEqual(first_bytes, (json.dumps(
                    expected, indent=2, sort_keys=True, ensure_ascii=False,
                ) + "\n").encode("utf-8"))
                self.assertEqual(self.fixture.mapping_path.read_bytes(), mapping_before)
                for name, raw in immutable_before.items():
                    self.assertEqual((self.fixture.receipt_root / name).read_bytes(), raw, name)
                state_after_first = json.loads(self.fixture.state_path.read_text(encoding="utf-8"))
                legacy_comments = [row for row in state_after_first["comments"]
                                   if "<!-- oasis7-pm-evidence -->" in row.get("body", "")]
                self.assertEqual(len(legacy_comments), 1)
                self.assertEqual(legacy_comments[0]["body"], comment_before)
                self.assertFalse((self.fixture.receipt_root / "terminal-delivery-receipt.json").exists())

                second = self.fixture.run_finalizer()
                self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
                self.assertEqual(tombstone_path.read_bytes(), first_bytes)
        finally:
            self.fixture = original_fixture

    def test_legacy_v1_chain_and_comment_are_read_without_rewriting_bytes(self):
        test_path = ROOT / "scripts/pm/loop_terminal.test.py"
        spec = importlib.util.spec_from_file_location("legacy_terminal_fixture", test_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        receipts = module.RECEIPTS
        names = {
            "merge": "merge-receipt.json", "main_sync": "main-sync-receipt.json",
            "terminal": "terminal-cleanup-receipt.json", "ledger": "finalizer-ledger.json",
            "tombstone": "terminal-tombstone.json",
        }
        before = {}
        for key, name in names.items():
            before[name] = receipts[key]["bytes"]
            (self.fixture.receipt_root / name).write_bytes(before[name])
        record = copy.deepcopy(self.fixture.record)
        record["workflow_phase"] = "post_merge_done"
        record["phase_receipts"] = {"post_merge_done": receipts["terminal"]["record"]}
        record["phase_receipt_sha256"] = {"post_merge_done": receipts["terminal"]["digest"]}
        legacy_comment = copy.deepcopy(module.COMMENT)
        legacy_comment["id"] = 7
        issue = copy.deepcopy(module.ISSUE)
        result = terminal_proof.read_terminal_proof(
            self.fixture.root, UID, record, live_issue=issue,
            live_project_item=module.ITEM, live_pr=module.PR,
            live_repository={}, comments=[legacy_comment],
        )
        self.assertEqual((result["status"], result["protocol_version"]), ("passed", 1))
        for name, raw in before.items():
            self.assertEqual((self.fixture.receipt_root / name).read_bytes(), raw)

    def test_aggregate_accepts_mixed_child_versions_and_rejects_mixed_markers_in_one_row(self):
        test_path = ROOT / "scripts/pm/aggregate-task-completion.test.py"
        spec = importlib.util.spec_from_file_location("aggregate_fixture_tests", test_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.AggregateTaskCompletionTests.setUpClass()
        aggregate_fixture = module.AggregateTaskCompletionTests()
        values = list(aggregate_fixture.context())
        plan, _candidate, _evidence, _coordinator, _comment, reports = values
        reports = copy.deepcopy(reports)

        # Build the per-version proof material through the real shared reader.
        # The aggregate child-report boundary projects the v2 delivery digest
        # to its versioned terminal_delivery_receipt_sha256 field and adds the
        # v1 main-sync digest from its already validated receipt chain.
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        shared_v2 = self.read_proof()
        legacy = importlib.util.spec_from_file_location(
            "legacy_terminal_aggregate_fixture", ROOT / "scripts/pm/loop_terminal.test.py",
        )
        legacy_module = importlib.util.module_from_spec(legacy)
        legacy.loader.exec_module(legacy_module)
        legacy_receipts = legacy_module.RECEIPTS
        for key, filename in {
            "merge": "merge-receipt.json", "main_sync": "main-sync-receipt.json",
            "terminal": "terminal-cleanup-receipt.json", "ledger": "finalizer-ledger.json",
            "tombstone": "terminal-tombstone.json",
        }.items():
            (self.fixture.receipt_root / filename).write_bytes(legacy_receipts[key]["bytes"])
        legacy_record = copy.deepcopy(self.fixture.record)
        legacy_record["workflow_phase"] = "post_merge_done"
        legacy_record["phase_receipts"] = {"post_merge_done": legacy_receipts["terminal"]["record"]}
        legacy_record["phase_receipt_sha256"] = {"post_merge_done": legacy_receipts["terminal"]["digest"]}
        legacy_issue = copy.deepcopy(legacy_module.ISSUE)
        legacy_comment = copy.deepcopy(legacy_module.COMMENT)
        legacy_comment["id"] = 7
        legacy_comment["html_url"] = f"{ISSUE_URL}#issuecomment-7"
        shared_v1 = terminal_proof.read_terminal_proof(
            self.fixture.root, UID, legacy_record, live_issue=legacy_issue,
            live_project_item=legacy_module.ITEM, live_pr=legacy_module.PR,
            live_repository={}, comments=[legacy_comment],
        )

        for index, delivery in enumerate(plan["required_deliveries"]):
            report = reports[delivery["task_uid"]]
            claim = report["task"]["claim_verifications"][-1]
            if index == 0:
                proof = {
                    "status": "passed", "protocol_version": 1,
                    "task_complete_claim_sha256": "sha256:" + module.digest(claim),
                    "merge_receipt_sha256": shared_v1["merge_receipt_sha256"],
                    "terminal_receipt_sha256": shared_v1["terminal_receipt_sha256"],
                    "main_sync_receipt_sha256": sha(legacy_receipts["main_sync"]["bytes"]),
                    "terminal_comment_sha256": shared_v1["comment_sha256"],
                    "finalizer_ledger_sha256": shared_v1["finalizer_ledger_sha256"],
                    "terminal_tombstone_sha256": shared_v1["tombstone_sha256"],
                }
            else:
                proof = {
                    "status": "passed", "protocol_version": 2,
                    "task_complete_claim_sha256": "sha256:" + module.digest(claim),
                    "merge_receipt_sha256": shared_v2["merge_receipt_sha256"],
                    "terminal_delivery_receipt_sha256": shared_v2["delivery_receipt_sha256"],
                    "terminal_comment_sha256": shared_v2["comment_sha256"],
                    "finalizer_ledger_sha256": shared_v2["finalizer_ledger_sha256"],
                    "terminal_tombstone_sha256": shared_v2["tombstone_sha256"],
                }
            report["proof"] = proof
            report["checks"].update({
                "completion_route_identity": True,
                "terminal_delivery_proof_valid": True,
            })
        values[5] = reports
        receipt = aggregate_fixture.build(tuple(values))
        self.assertEqual(receipt["schema"], "oasis7.aggregate-task-completion/v2")
        self.assertEqual([row["terminal_protocol_version"] for row in receipt["deliveries"]], [1, 2])

        mixed = copy.deepcopy(values)
        mixed[5][plan["required_deliveries"][1]["task_uid"]]["proof"]["main_sync_receipt_sha256"] = "d" * 64
        with self.assertRaises(module.AggregateTaskCompletionTests.helper.ReceiptError):
            aggregate_fixture.build(tuple(mixed))


if __name__ == "__main__":
    unittest.main()
