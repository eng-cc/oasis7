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
import shutil
import subprocess
import sys
import tempfile
import uuid
import unittest

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
        self.bin = parent / "bin"
        self.state_path = parent / "github-state.json"
        self.remote_state_path = parent / "remote-branch-state.json"
        self.log_path = parent / "gh-log.jsonl"
        self.lost_marker = parent / "lost-response-once"
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
                {"name": "Done", "field": {"name": "Status"}},
                {"name": "done", "field": {"name": "PM Status"}},
                {"name": "done", "field": {"name": "Workflow Phase"}},
                {"text": UID, "field": {"name": "Task UID"}},
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
        mapping = {"version": 1, "project": {"owner": "fixture", "number": 1,
                    "id": "PROJECT_fixture", "repo": REPOSITORY}, "tasks": {UID: self.record}}
        self.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        self.state = {"issue": self.issue, "pr": self.pr, "comments": self.comments,
                      "project_item": self.project_item, "merge_oid": self.merge_oid,
                      "target_oid": self.merge_oid}
        self._write_state()
        self.remote_state_path.write_text(
            json.dumps({"refs/heads/task/protocol-fixture": self.head_oid}), encoding="utf-8")
        helper = ROOT / "scripts/pm/canonical-receipt-root.py"
        raw = subprocess.check_output([
            sys.executable, str(helper), "--default-worktree", str(self.root),
            "--task-uid", UID, "--create",
        ], text=True)
        self.receipt_root = pathlib.Path(raw.strip())
        (self.receipt_root / "merge-receipt.json").write_bytes(merge_raw)
        self._install_gh_stub()
        self._install_git_network_stub()

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
    out({"fields":[]})
elif args[:1] == ["api"] and len(args) > 1 and args[1] == "graphql":
    query = next((x.split("=",1)[1] for x in args if x.startswith("query=")), "")
    item = state["project_item"]
    if "nodes(ids" in query:
        out({"data":{"nodes":[item]}})
    else:
        out({"data":{"repository":{"issue":{"projectItems":{"pageInfo":{"hasNextPage":False},"nodes":[item]}}}}})
elif args[:1] == ["api"]:
    endpoint = next((x for x in args[1:] if x.startswith("repos/")), "")
    if endpoint == "repos/fixture/repo/issues/11": out(state["issue"])
    elif endpoint == "repos/fixture/repo/pulls/12": out(state["pr"])
    elif endpoint == "repos/fixture/repo": out({"full_name":"fixture/repo","default_branch":"main"})
    elif endpoint == "repos/fixture/repo/git/ref/heads/main":
        out({"ref":"refs/heads/main","object":{"sha":state["target_oid"]}})
    elif endpoint == "repos/fixture/repo/compare/" + state["merge_oid"] + "..." + state["target_oid"]:
        out({"status":"identical","base_commit":{"sha":state["merge_oid"]},"head_commit":{"sha":state["target_oid"]}})
    elif endpoint == "repos/fixture/repo/issues/11/comments": out([state["comments"]])
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

    def env(self, *, lose_response: bool = False, comment_author: str | None = None,
            comment_no_user: bool = False) -> dict[str, str]:
        env = dict(os.environ)
        env["PATH"] = str(self.bin) + os.pathsep + env.get("PATH", "")
        env["QA_GH_STATE"] = str(self.state_path)
        env["QA_GH_LOG"] = str(self.log_path)
        env["QA_LOST_MARKER"] = str(self.lost_marker)
        env["QA_REAL_GIT"] = self.real_git
        env["QA_REMOTE_BRANCH_OID"] = self.head_oid
        env["QA_REMOTE_STATE"] = str(self.remote_state_path)
        if lose_response:
            env["QA_LOSE_COMMENT_RESPONSE"] = "1"
        else:
            env.pop("QA_LOSE_COMMENT_RESPONSE", None)
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
                     comment_author: str | None = None,
                     comment_no_user: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            sys.executable, str(ROOT / "scripts/pm/post-merge-finalize.py"),
            "--repo-root", str(self.root), "--task-uid", UID, "--delivery", *extra,
            "--json",
        ], text=True, capture_output=True,
            env=self.env(lose_response=lose_response, comment_author=comment_author,
                         comment_no_user=comment_no_user))

    def run_finalizer(self, *extra: str) -> subprocess.CompletedProcess[str]:
        helper_dir = self.root / "scripts/pm"
        if not helper_dir.exists():
            helper_dir.parent.mkdir(parents=True, exist_ok=True)
            helper_dir.symlink_to(ROOT / "scripts/pm", target_is_directory=True)
        return subprocess.run([
            "bash", str(ROOT / "scripts/pm/finalize-task.sh"),
            "--repo-root", str(self.root), "--task-uid", UID, "--pr", str(PR),
            "--resume", *extra, "--json",
        ], cwd=self.root, text=True, capture_output=True, env=self.env())

    def run_cleanup(self, *extra: str) -> subprocess.CompletedProcess[str]:
        helper_dir = self.root / "scripts/pm"
        if not helper_dir.exists():
            helper_dir.parent.mkdir(parents=True, exist_ok=True)
            helper_dir.symlink_to(ROOT / "scripts/pm", target_is_directory=True)
        return subprocess.run([
            "bash", str(ROOT / "scripts/pm/post-merge-cleanup.sh"),
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
            "merge_compare": {"status": "identical", "base_commit": {"sha": state["merge_oid"]},
                              "head_commit": {"sha": state["target_oid"]}},
            "observed_target_compare": None,
        }
        return record, state["issue"], state["project_item"], state["pr"], live_repo, state["comments"]


class TerminalDeliveryProtocolTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="oasis7-delivery-protocol-")
        self.fixture = DeliveryFixture(pathlib.Path(self.temp.name))

    def tearDown(self):
        self.temp.cleanup()

    def read_proof(self):
        record, issue, item, pr, live_repo, comments = self.fixture.live_inputs()
        return terminal_proof.read_terminal_proof(
            self.fixture.root, UID, record, live_issue=issue,
            live_project_item=item, live_pr=pr, live_repository=live_repo,
            comments=comments,
        )

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

    def test_shared_reader_rejects_selector_digest_comment_claim_and_tombstone_drift(self):
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

    def test_public_cleanup_preflight_consumes_the_same_strict_delivery_proof(self):
        produced = self.fixture.run_producer()
        self.assertEqual(produced.returncode, 0, produced.stderr)
        cleanup = self.fixture.run_cleanup("--preflight")
        self.assertEqual(cleanup.returncode, 0, cleanup.stdout + cleanup.stderr)
        payload = json.loads(cleanup.stdout)
        self.assertEqual(payload["status"], "ready", payload)
        self.assertEqual(payload["delivery"]["state"], "complete", payload)
        self.assertEqual({row["state"] for row in payload["resources"]}, {"ready"}, payload)

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
