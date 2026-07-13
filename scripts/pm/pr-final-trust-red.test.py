#!/usr/bin/env python3
"""RED contracts for the final PR-policy and GitHub-evidence trust boundary."""

from __future__ import annotations

import importlib.util
import inspect
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("pr_gate", ROOT / "scripts/pm/pr-lifecycle-gate.py")
assert SPEC and SPEC.loader
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


def clean_pr() -> dict:
    return {
        "number": 9,
        "repository": "eng-cc/oasis7",
        "url": "https://github.com/eng-cc/oasis7/pull/9",
        "state": "OPEN",
        "headRefOid": "a" * 40,
        "baseRefName": "main",
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "merge_hold": {
            "kind": "normal_pr_ci_watch",
            "active": False,
            "requester": "workflow",
            "reason": "normal",
            "resume_authority": "workflow",
        },
        "comments": [],
        "reviews": [],
        "threads": [],
        "required_status_checks": [],
        "statusCheckRollup": [],
    }


class FinalTrustRed(unittest.TestCase):
    def test_live_gate_rejects_caller_merge_hold_override(self) -> None:
        """Only task-issue readback may clear an active production hold."""
        source = (ROOT / "scripts/pm/pr-lifecycle-gate.py").read_text(encoding="utf-8")
        live_branch = source[source.index("if not args.fixture:"):source.index("result = decision")]
        self.assertNotRegex(
            live_branch,
            r"if args\.merge_hold:[\s\S]*data\[\"merge_hold\"\]",
            "--merge-hold must be fixture-only and must not overwrite live GitHub task truth",
        )
        self.assertRegex(source, r"(?s)if args\.fixture:.*args\.merge_hold|args\.merge_hold.*if args\.fixture")

    def test_fixture_gate_never_issues_production_readiness_receipt(self) -> None:
        """Fixture decisions are useful tests, but are never merge authority."""
        source = (ROOT / "scripts/pm/pr-lifecycle-gate.py").read_text(encoding="utf-8")
        self.assertIn("evidence_mode", source)
        self.assertRegex(source, r"(?s)args\.fixture.*evidence_mode.*fixture")
        self.assertRegex(source, r"(?s)readiness_receipt.*production|production.*readiness_receipt")

    def test_stable_gate_epoch_excludes_observation_time(self) -> None:
        first = gate.decision(clean_pr(), False)
        second = gate.decision(clean_pr(), False)
        self.assertNotEqual(first["readiness_receipt"]["observed_at"], second["readiness_receipt"]["observed_at"])
        self.assertEqual(first["readiness_receipt"]["gate_epoch"], second["readiness_receipt"]["gate_epoch"])

    def test_forged_full_disposition_receipt_requires_live_comment_roundtrip(self) -> None:
        data = clean_pr()
        receipt = {
            "source": "github_task_issue_comment",
            "runtime_verified": True,
            "task_uid": "task_" + "1" * 32,
            "repository": data["repository"],
            "issue_number": 2198,
            "pr_number": data["number"],
            "head_oid": data["headRefOid"],
            "github_node_id": "IC_forged",
            "url": "https://github.com/eng-cc/oasis7/issues/2198#issuecomment-forged",
            "author": "trusted-bot",
            "observed_at": "2026-07-11T01:00:00Z",
            "digest": "b" * 64,
        }
        with mock.patch.object(gate.subprocess, "check_output", side_effect=subprocess.CalledProcessError(1, ["gh"])):
            self.assertFalse(gate.verified_evidence(receipt, data, data["headRefOid"]))

    def test_disposition_writer_reads_back_and_binds_complete_identity(self) -> None:
        source = (ROOT / "scripts/pm/record-pr-disposition.sh").read_text(encoding="utf-8")
        self.assertIn("gh api", source, "writer must fetch the created comment back from GitHub")
        for field in ("task_uid", "repository", "issue_number", "pr_number", "head_oid", "node_id", "author", "observed_at", "digest"):
            self.assertIn(f"'{field}'", source, f"writer receipt must bind {field}")

    def test_hold_clear_requires_roundtrip_not_cache_shape(self) -> None:
        data = clean_pr()
        forged_clear = dict(data["merge_hold"])
        forged_clear["evidence_receipt"] = {
            "source": "github_task_issue_comment",
            "runtime_verified": True,
            "repository": data["repository"],
            "pr_number": data["number"],
            "head_oid": data["headRefOid"],
            "github_node_id": "IC_cache_only",
            "url": "https://github.com/eng-cc/oasis7/issues/2198#issuecomment-cache",
            "digest": "c" * 64,
        }
        with mock.patch.object(gate.subprocess, "check_output", side_effect=subprocess.CalledProcessError(1, ["gh"])):
            self.assertFalse(gate.verified_evidence(forged_clear["evidence_receipt"], data, data["headRefOid"]))

    def test_classic_and_applicable_rulesets_are_unioned(self) -> None:
        classic = {"required_status_checks": {"checks": [{"context": "classic", "app_id": 1}]}}
        rulesets = [[{
            "id": 7,
            "target": "branch",
            "enforcement": "active",
            "conditions": {"ref_name": {"include": ["refs/heads/main"]}},
            "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "ruleset", "integration_id": 2}]}}],
        }]]
        with mock.patch.object(gate, "_run_json", side_effect=[classic, rulesets]):
            result = gate.discover_required_policy("eng-cc/oasis7", "main")
        self.assertEqual({("classic", 1), ("ruleset", 2)}, {(x["context"], x["app_id"]) for x in result["required_status_checks"]})

    def test_ruleset_ref_semantics_default_pattern_include_exclude(self) -> None:
        missing = subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 404")
        rulesets = [[
            {"id": 1, "target": "branch", "enforcement": "active", "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"]}}, "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "default", "integration_id": 1}]}}]},
            {"id": 2, "target": "branch", "enforcement": "active", "conditions": {"ref_name": {"include": ["refs/heads/release/*"]}}, "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "pattern", "integration_id": 2}]}}]},
            {"id": 3, "target": "branch", "enforcement": "active", "conditions": {"ref_name": {"include": ["~ALL"], "exclude": ["refs/heads/release/private*"]}}, "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "all", "integration_id": 3}]}}]},
        ]]
        with mock.patch.object(gate, "_run_json", side_effect=[missing, rulesets, {"default_branch":"main"}]):
            result = gate.discover_required_policy("eng-cc/oasis7", "release/v1")
        identities = {(x["context"], x["app_id"]) for x in result["required_status_checks"]}
        self.assertNotIn(("default", 1), identities, "~DEFAULT_BRANCH must use actual default-branch identity")
        self.assertIn(("pattern", 2), identities)
        self.assertIn(("all", 3), identities)

    def test_ruleset_pull_request_approval_is_normalized_and_unknown_constraints_fail_closed(self) -> None:
        missing = subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 404")
        base = {
            "id": 1, "target": "branch", "enforcement": "active",
            "conditions": {"ref_name": {"include": ["refs/heads/main"]}},
            "rules": [{"type": "pull_request", "parameters": {
                "required_approving_review_count": 1,
                "required_review_thread_resolution": True,
                "allowed_merge_methods": ["squash"],
            }}],
        }
        with mock.patch.object(gate, "_run_json", side_effect=[missing, [[base]]]):
            result = gate.discover_required_policy("eng-cc/oasis7", "main")
        self.assertEqual(
            {"required_pull_request_reviews", "required_conversation_resolution"},
            set(result["active_rule_types"]),
        )

        constrained = json.loads(json.dumps(base))
        constrained["rules"][0]["parameters"]["unknown_future_constraint"] = True
        with mock.patch.object(gate, "_run_json", side_effect=[missing, [[constrained]]]):
            result = gate.discover_required_policy("eng-cc/oasis7", "main")
        self.assertIn("unsupported_pull_request_policy", result["active_rule_types"])

    def test_admin_path_uses_head_bound_human_authority_without_runtime_producer(self) -> None:
        data = clean_pr()
        data["mergeStateStatus"] = "BLOCKED"
        data["reviewDecision"] = "REVIEW_REQUIRED"
        data["policy_discovery"] = {
            "status": "resolved",
            "active_rule_types": ["required_pull_request_reviews"],
            "required_status_checks": [],
        }
        without_authority = gate.decision(data, True, evidence_mode="fixture")
        self.assertEqual("blocked", without_authority["status"])
        self.assertFalse(without_authority["use_admin_merge"])

        data["admin_merge_authority"] = {
            "requester": "user",
            "scope": "review_approval_only",
            "reason": "explicit fixture authorization",
            "disposition": "authorized",
        }
        result = gate.decision(data, True, evidence_mode="fixture")
        self.assertEqual("ready", result["status"])
        self.assertTrue(result["ready_for_merge"])
        self.assertTrue(result["use_admin_merge"])

    def test_canonical_comment_identity_rejects_unrelated_stale_comment(self) -> None:
        """A live comment is not evidence unless its canonical body is for this task/action."""
        data = clean_pr()
        unrelated_body = """<!-- oasis7-pr-disposition -->
- task_uid: `task_wrong`
- repository: `eng-cc/oasis7`
- issue_number: `999`
- pr_number: `9`
- head_oid: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`
- node_id: `R-wrong`
- kind: `review`
- disposition: `addressed`
"""
        receipt = {
            "source": "github_task_issue_comment", "runtime_verified": True,
            "task_uid": "task_wrong", "issue_number": 999,
            "repository": data["repository"], "pr_number": data["number"],
            "head_oid": data["headRefOid"], "node_id": "R-current",
            "kind": "review", "disposition": "addressed",
            "github_node_id": "123", "url": "https://github.com/eng-cc/oasis7/issues/999#issuecomment-123",
            "author": "alice", "observed_at": "2000-01-01T00:00:00Z",
            "digest": __import__("hashlib").sha256(unrelated_body.encode()).hexdigest(),
        }
        comment = {"body": unrelated_body, "user": {"login": "alice"}, "html_url": receipt["url"]}
        parameters = inspect.signature(gate.verified_evidence).parameters
        for required in ("task_uid", "issue_number", "node_id", "kind", "disposition"):
            self.assertIn(required, parameters, f"verifier must bind current {required}")
        with mock.patch.object(gate.subprocess, "check_output", return_value=json.dumps(comment)):
            self.assertFalse(gate.verified_evidence(
                receipt, data, data["headRefOid"],
                task_uid="task_" + "1" * 32, issue_number=2198,
                node_id="R-current", kind="review", disposition="addressed",
            ))

    def test_writer_to_gate_has_supported_persistence_or_issue_rebuild(self) -> None:
        writer = (ROOT / "scripts/pm/record-pr-disposition.sh").read_text(encoding="utf-8")
        gate_source = (ROOT / "scripts/pm/pr-lifecycle-gate.py").read_text(encoding="utf-8")
        persists_via_adapter = "github-project-task.py" in writer and "record-pr-disposition" in writer
        rebuilds_from_issue = "oasis7-pr-disposition" in gate_source and "issueComments" in gate_source
        self.assertTrue(persists_via_adapter or rebuilds_from_issue,
                        "writer output must survive the process boundary and be consumable by the live gate")

    def test_merge_hold_active_and_clear_use_canonical_readback_receipts(self) -> None:
        source = (ROOT / "scripts/pm/github-project-task.py").read_text(encoding="utf-8")
        hold_section = source[source.index("def command_set_merge_hold"):source.index("def add_common", source.index("def command_set_merge_hold"))]
        self.assertIn("read", hold_section.lower(), "active and clear comments must be read back")
        self.assertIn("evidence_receipt", hold_section,
                      "canonical active/clear receipt must be persisted; cache shape alone is not authority")
        self.assertIn("task_uid", hold_section)
        self.assertIn("issue_number", hold_section)

    def test_live_rebuild_preserves_default_inactive_normal_watch_hold(self) -> None:
        task_uid = "task_" + "1" * 32
        live = clean_pr()
        live.update({"number": 2198, "reviewDecision": "APPROVED",
                     "statusCheckRollup": [], "required_status_checks": [],
                     "policy_discovery": {"status": "resolved", "required_status_checks": []}})
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            mapping = root / ".pm/github-project-sync/tasks.json"
            mapping.parent.mkdir(parents=True)
            mapping.write_text(json.dumps({"tasks": {task_uid: {
                "issue_number": 2198,
                "pr_number": 2198,
                "merge_hold": {"kind": "normal_pr_ci_watch", "active": False,
                               "requester": "workflow", "reason": "normal",
                               "resume_authority": "workflow"},
            }}}))
            argv = ["pr-lifecycle-gate.py", "2198", "--root", str(root),
                    "--task-uid", task_uid, "--json"]
            with mock.patch.object(gate, "load_live", return_value=live), \
                 mock.patch.object(gate, "rebuild_issue_evidence", return_value={
                     "comment_dispositions": [], "review_dispositions": []}), \
                 mock.patch.object(sys, "argv", argv):
                self.assertEqual(0, gate.main(),
                                 "an absent override comment must not erase the canonical default hold")

    def test_default_hold_requires_task_truth_bound_to_the_live_pr(self) -> None:
        task_uid = "task_" + "1" * 32
        live = clean_pr()
        live.update({"number": 2198, "reviewDecision": "APPROVED",
                     "statusCheckRollup": [], "required_status_checks": [],
                     "policy_discovery": {"status": "resolved", "required_status_checks": []}})
        for label, recorded_pr in (("missing", None), ("mismatched", 9999)):
            with self.subTest(label=label), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                mapping = root / ".pm/github-project-sync/tasks.json"
                mapping.parent.mkdir(parents=True)
                record = {
                    "issue_number": 2198,
                    "merge_hold": {"kind": "normal_pr_ci_watch", "active": False,
                                   "requester": "workflow", "reason": "normal",
                                   "resume_authority": "workflow"},
                }
                if recorded_pr is not None:
                    record["pr_number"] = recorded_pr
                mapping.write_text(json.dumps({"tasks": {task_uid: record}}))
                argv = ["pr-lifecycle-gate.py", "2198", "--root", str(root),
                        "--task-uid", task_uid, "--json"]
                with mock.patch.object(gate, "load_live", return_value=live), \
                     mock.patch.object(gate, "rebuild_issue_evidence", return_value={
                         "comment_dispositions": [], "review_dispositions": []}), \
                     mock.patch.object(sys, "argv", argv):
                    self.assertEqual(3, gate.main(),
                                     "unbound or mismatched task truth must not authorize the default hold")

    def test_merge_hold_writer_binds_comment_to_live_pr_head(self) -> None:
        source = (ROOT / "scripts/pm/github-project-task.py").read_text(encoding="utf-8")
        hold_section = source[source.index("def command_set_merge_hold"):
                              source.index("def add_common", source.index("def command_set_merge_hold"))]
        self.assertIn("headRefOid", hold_section,
                      "manual/user hold comments must bind the current live PR head")
        self.assertRegex(hold_section, r"gh[^\n]+pr[^\n]+view",
                         "the hold writer must read the PR head from GitHub, not stale task cache")

    def test_default_branch_is_queried_for_default_branch_ruleset(self) -> None:
        missing = subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 404")
        rulesets = [[{
            "id": 41, "target": "branch", "enforcement": "active",
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"]}},
            "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "trunk-gate", "integration_id": 4}]}}],
        }]]
        calls: list[list[str]] = []
        def fake(cmd: list[str]):
            calls.append(cmd)
            endpoint = cmd[2] if len(cmd) > 2 else ""
            if "/branches/" in endpoint: raise missing
            if endpoint.endswith("/rulesets"): return rulesets
            if endpoint == "repos/eng-cc/oasis7": return {"default_branch": "trunk"}
            raise AssertionError(cmd)
        with mock.patch.object(gate, "_run_json", side_effect=fake):
            result = gate.discover_required_policy("eng-cc/oasis7", "trunk")
        self.assertTrue(any(len(c) > 2 and c[2] == "repos/eng-cc/oasis7" for c in calls),
                        "~DEFAULT_BRANCH requires querying the actual repository default branch")
        self.assertIn(("trunk-gate", 4), {(x["context"], x["app_id"]) for x in result["required_status_checks"]})

    def test_classic_403_is_terminal_policy_read_error_without_ruleset_fallback(self) -> None:
        denied = subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 403 Resource not accessible")
        runner = mock.Mock(side_effect=denied)
        with mock.patch.object(gate, "_run_json", runner):
            result = gate.discover_required_policy("eng-cc/oasis7", "main")
        self.assertEqual(1, runner.call_count, "only classic 404 may fall back to rulesets")
        self.assertEqual("capability_blocked", result["status"])
        self.assertEqual("policy_read_error", result["reason"])

    def test_writer_review_kind_and_readback_timestamp_roundtrip(self) -> None:
        writer_path = ROOT / "scripts/pm/record-pr-disposition.sh"
        help_text = subprocess.check_output([str(writer_path), "--help"], text=True)
        source = writer_path.read_text(encoding="utf-8")
        self.assertIn("--kind", help_text, "writer must distinguish comment from top-level review")
        self.assertRegex(help_text, r"comment\|review")
        self.assertIn("created_at", source, "receipt observed_at must come from GitHub readback")
        self.assertNotIn("datetime.datetime.now", source,
                         "local clock cannot stand in for the readback comment timestamp")

        data = clean_pr()
        data["reviews"] = [{
            "id": "R1", "state": "COMMENTED", "body": "please add a regression test",
            "url": "https://github.com/eng-cc/oasis7/pull/9#pullrequestreview-1",
            "author": {"login": "reviewer"}, "submittedAt": "2026-07-11T01:00:00Z",
        }]
        review_body = """<!-- oasis7-pr-disposition -->
- task_uid: `task_11111111111111111111111111111111`
- repository: `eng-cc/oasis7`
- issue_number: `2198`
- pr_number: `9`
- head_oid: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`
- node_id: `R1`
- kind: `review`
- disposition: `addressed`
"""
        comment_body = review_body.replace("node_id: `R1`", "node_id: `C1`").replace("kind: `review`", "kind: `comment`")
        issue_comments = [[
            {"id": 101, "body": review_body, "html_url": "https://github.com/eng-cc/oasis7/issues/2198#issuecomment-101", "created_at": "2026-07-11T01:01:00Z", "user": {"login": "writer"}},
            {"id": 102, "body": comment_body, "html_url": "https://github.com/eng-cc/oasis7/issues/2198#issuecomment-102", "created_at": "2026-07-11T01:02:00Z", "user": {"login": "writer"}},
        ]]
        with mock.patch.object(gate, "_run_json", return_value=issue_comments):
            rebuilt = gate.rebuild_issue_evidence(data["repository"], 2198, "task_" + "1" * 32, data)
        self.assertEqual("R1", rebuilt["review_dispositions"][0]["node_id"])
        self.assertEqual("C1", rebuilt["comment_dispositions"][0]["node_id"])
        data.update(rebuilt)
        with mock.patch.object(gate.subprocess, "check_output", side_effect=[json.dumps(issue_comments[0][1]), json.dumps(issue_comments[0][0])]):
            result = gate.decision(data, False)
        self.assertTrue(result["ready_for_merge"], result)

    def test_default_branch_api_failure_is_structured_capability_blocker(self) -> None:
        missing = subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 404")
        rulesets = [[{
            "id": 51, "target": "branch", "enforcement": "active",
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"]}},
            "rules": [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "default-only", "integration_id": 5}]}}],
        }]]
        calls = iter([missing, rulesets, subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 503")])
        def fake(_cmd: list[str]):
            value = next(calls)
            if isinstance(value, Exception): raise value
            return value
        with mock.patch.object(gate, "_run_json", side_effect=fake):
            result = gate.discover_required_policy("eng-cc/oasis7", "main")
        self.assertEqual("capability_blocked", result["status"])
        self.assertEqual("default_branch_read_error", result["reason"])
        self.assertEqual([], result["required_status_checks"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
