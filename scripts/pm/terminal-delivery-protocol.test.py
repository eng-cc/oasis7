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
import subprocess
import sys
import tempfile
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
        self.root = parent / "repo"
        self.task = parent / "task-worktree"
        self.bin = parent / "bin"
        self.state_path = parent / "github-state.json"
        self.log_path = parent / "gh-log.jsonl"
        self.lost_marker = parent / "lost-response-once"
        self.bin.mkdir(parents=True)
        self._git("init", "-q", "-b", "main", str(self.root))
        self._git("config", "user.name", "QA Fixture", cwd=self.root)
        self._git("config", "user.email", "qa@example.invalid", cwd=self.root)
        (self.root / "README.md").write_text("base\n", encoding="utf-8")
        self._git("add", "README.md", cwd=self.root)
        self._git("commit", "-qm", "base", cwd=self.root)
        self._git("worktree", "add", "-qb", "task/protocol-fixture", str(self.task), cwd=self.root)
        (self.task / "README.md").write_text("base\nmerged task change\n", encoding="utf-8")
        self._git("add", "README.md", cwd=self.task)
        self._git("commit", "-qm", "task change", cwd=self.task)
        self.head_oid = self._git("rev-parse", "HEAD", cwd=self.task)
        self.head_tree = self._git("rev-parse", "HEAD^{tree}", cwd=self.task)
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
        }
        mapping = {"version": 1, "project": {"owner": "fixture", "number": 1,
                    "id": "PROJECT_fixture", "repo": REPOSITORY}, "tasks": {UID: self.record}}
        self.mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        self.state = {"issue": self.issue, "pr": self.pr, "comments": self.comments,
                      "project_item": self.project_item, "merge_oid": self.merge_oid,
                      "target_oid": self.merge_oid}
        self._write_state()
        helper = ROOT / "scripts/pm/canonical-receipt-root.py"
        raw = subprocess.check_output([
            sys.executable, str(helper), "--default-worktree", str(self.root),
            "--task-uid", UID, "--create",
        ], text=True)
        self.receipt_root = pathlib.Path(raw.strip())
        (self.receipt_root / "merge-receipt.json").write_bytes(merge_raw)
        self._install_gh_stub()

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
    out({"id":"PROJECT_fixture"})
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

    def env(self, *, lose_response: bool = False) -> dict[str, str]:
        env = dict(os.environ)
        env["PATH"] = str(self.bin) + os.pathsep + env.get("PATH", "")
        env["QA_GH_STATE"] = str(self.state_path)
        env["QA_GH_LOG"] = str(self.log_path)
        env["QA_LOST_MARKER"] = str(self.lost_marker)
        if lose_response:
            env["QA_LOSE_COMMENT_RESPONSE"] = "1"
        else:
            env.pop("QA_LOSE_COMMENT_RESPONSE", None)
        return env

    def run_producer(self, *extra: str, lose_response: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            sys.executable, str(ROOT / "scripts/pm/post-merge-finalize.py"),
            "--repo-root", str(self.root), "--task-uid", UID, "--delivery", *extra,
            "--json",
        ], text=True, capture_output=True, env=self.env(lose_response=lose_response))

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
