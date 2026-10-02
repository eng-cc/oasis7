#!/usr/bin/env python3
"""Focused contract tests for linked-delivery aggregate completion receipts."""
from __future__ import annotations

import base64
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/pm/aggregate-task-completion.py"


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def comment_body(plan: dict) -> str:
    return "<!-- oasis7-aggregate-delivery-plan/v1 -->\n" + canonical_bytes(plan).decode()


def task_uid(hex_char: str) -> str:
    return "task_" + hex_char * 32


def load_test_module(path: pathlib.Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load fixture module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def copy_pm_helpers(root: pathlib.Path) -> None:
    # DeliveryFixture now stages its own isolated PM tree and API adapter.
    # Keep that complete tree instead of trying to copy over it (or replacing
    # its adapter with the production transport).
    if (root / "scripts/pm").is_dir():
        return
    (root / "scripts").mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / "scripts/pm", root / "scripts/pm")


def install_issue_pr_view_readback(fixture) -> None:
    """Extend the producer's isolated gh fixture with terminal audit CLI reads."""
    gh = fixture.bin / "gh"
    base = fixture.bin / "gh-base"
    gh.replace(base)
    gh.write_text(r'''#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
args = sys.argv[1:]
if args[:2] in (["issue", "view"], ["pr", "view"]):
    state = json.loads(pathlib.Path(os.environ["QA_GH_STATE"]).read_text())
    number = args[2]
    if args[0] == "issue":
        value = state["issue"]
        result = {"number": value["number"], "url": value.get("html_url", value.get("url")),
                  "title": value.get("title", "terminal fixture"), "state": value["state"],
                  "body": value.get("body", ""), "stateReason": value.get("state_reason", ""),
                  "updatedAt": value.get("updated_at", value.get("updatedAt", "2026-09-30T08:00:00Z"))}
    else:
        value = state["pr"]
        head, base_ref = value.get("head", {}), value.get("base", {})
        result = {"number": value["number"], "url": value.get("html_url", value.get("url")),
                  "state": "MERGED" if value.get("merged") else str(value.get("state", "")).upper(),
                  "mergedAt": value.get("merged_at"),
                  "mergeCommit": {"oid": value.get("merge_commit_sha")},
                  "headRefOid": head.get("sha"), "headRefName": head.get("ref"),
                  "baseRefName": base_ref.get("ref"), "body": value.get("body", "")}
    print(json.dumps(result))
else:
    base = pathlib.Path(__file__).with_name("gh-base")
    result = subprocess.run([str(base), *args])
    raise SystemExit(result.returncode)
''', encoding="utf-8")
    gh.chmod(0o755)


class AggregateTaskCompletionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not HELPER.is_file():
            raise AssertionError("aggregate-task-completion.py is missing")
        spec = importlib.util.spec_from_file_location("aggregate_task_completion", HELPER)
        if spec is None or spec.loader is None:
            raise AssertionError("aggregate task completion helper cannot be loaded")
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def context(self):
        uid_a, uid_b = task_uid("a"), task_uid("b")
        candidate = {
            "change_id": "change-4035",
            "integration_base_oid": "1" * 40,
            "tested_tree_oid": "2" * 40,
            "configuration_digest": "sha256:" + "3" * 64,
            "entry": "aggregate-required",
            "environment": "linux-x86_64",
            "evidence_window": {"started_at": "2026-09-25T08:00:00Z", "ended_at": "2026-09-25T09:00:00Z"},
        }
        evidence = [{"obligation_id": "delivery-a", "status": "passed", "profile": "repository_required"}]
        plan = {
            "schema": "oasis7.aggregate-delivery-plan/v1",
            "task_uid": task_uid("f"),
            "repository": "eng-cc/oasis7",
            "issue_number": 4035,
            "change_id": "change-4035",
            "required_deliveries": [
                {
                    "ordinal": 1,
                    "obligation_id": "delivery-a",
                    "task_uid": uid_a,
                    "issue_number": 4101,
                    "pr_number": 5101,
                    "pr_url": "https://github.com/eng-cc/oasis7/pull/5101",
                    "depends_on": [],
                },
                {
                    "ordinal": 2,
                    "obligation_id": "delivery-b",
                    "task_uid": uid_b,
                    "issue_number": 4102,
                    "pr_number": 5102,
                    "pr_url": "https://github.com/eng-cc/oasis7/pull/5102",
                    "depends_on": ["delivery-a"],
                },
            ],
            "candidate_selection": {
                "candidate_sha256": digest(candidate),
                "evidence_sha256": digest(evidence),
                **{key: candidate[key] for key in (
                    "integration_base_oid", "tested_tree_oid", "configuration_digest",
                    "entry", "environment", "evidence_window",
                )},
            },
        }
        body = comment_body(plan)
        body_sha = hashlib.sha256(body.encode()).hexdigest()
        coordinator = {
            "number": 4035,
            "repository": "eng-cc/oasis7",
            "state": "OPEN",
            "body": (
                "<!-- oasis7-pm-task -->\n"
                f"task_uid: {plan['task_uid']}\n"
                "completion_mode: ordered_delivery_aggregate\n"
                "aggregate_plan_comment_id: 6001\n"
                f"aggregate_plan_sha256: sha256:{body_sha}\n"
            ),
        }
        comment = {
            "id": 6001,
            "issue_number": 4035,
            "body": body,
            "author": "eng-cc",
            "permission": "admin",
        }
        child_reports = {}
        task_complete_claim = {
            "claim_type": "task_complete", "status": "verified",
            "verification_exit_code": 0,
        }
        for delivery, uid, merged_at in zip(
            plan["required_deliveries"],
            (uid_a, uid_b),
            ("2026-09-25T09:00:00Z", "2026-09-25T09:01:00Z"),
        ):
            child_reports[uid] = {
                "status": "reconciled",
                "task": {
                    "task_uid": uid,
                    "repository": "eng-cc/oasis7",
                    "issue_number": delivery["issue_number"],
                    "pr_number": delivery["pr_number"],
                    "pr_url": delivery["pr_url"],
                    "status": "done",
                    "workflow_phase": "post_merge_done",
                    "claim_verifications": [task_complete_claim],
                },
                "checks": {
                    "mapping_post_merge_done": True,
                    "terminal_receipt_chain_valid": True,
                    "finalizer_ledger_committed": True,
                    "terminal_tombstone_valid": True,
                    "issue_closed": True,
                    "project_item_bound": True,
                    "project_item_identity": True,
                    "project_field_values_complete": True,
                    "project_terminal": True,
                    "pr_merged": True,
                    "worktree_absent": True,
                    "task_branch_not_registered_elsewhere": True,
                    "local_branch_absent": True,
                    "remote_branch_absent": True,
                },
                "live": {
                    "issue": {"number": delivery["issue_number"], "state": "CLOSED"},
                    "pr": {
                        "number": delivery["pr_number"],
                        "url": delivery["pr_url"],
                        "state": "MERGED",
                        "base_ref": "main",
                        "head_oid": "4" * 40,
                        "merge_commit_oid": "5" * 40,
                        "merged_at": merged_at,
                    },
                },
                "proof": {
                    "task_complete_claim_sha256": "sha256:" + digest(task_complete_claim),
                    "merge_receipt_sha256": "sha256:" + "7" * 64,
                    "main_sync_receipt_sha256": "sha256:" + "8" * 64,
                    "terminal_receipt_sha256": "sha256:" + "9" * 64,
                    "terminal_tombstone_sha256": "sha256:" + "a" * 64,
                },
            }
        coordinator["default_branch"] = "main"
        return plan, candidate, evidence, coordinator, comment, child_reports

    def build(self, context=None):
        values = context or self.context()
        plan, candidate, evidence, coordinator, comment, children = values
        return self.helper.build_receipt(
            task_uid=plan["task_uid"],
            plan=plan,
            candidate=candidate,
            evidence=evidence,
            coordinator_issue=coordinator,
            plan_comment=comment,
            child_reports=children,
            effective_validator_commit="c" * 40,
            observed_at="2026-09-25T09:05:00Z",
        )

    def test_builds_canonical_receipt_from_live_coordinator_and_child_terminal_proofs(self):
        plan, candidate, evidence, *_ = self.context()
        receipt = self.build()
        self.assertEqual(receipt["schema"], "oasis7.aggregate-task-completion/v1")
        self.assertEqual(receipt["claim_type"], "task_complete")
        self.assertEqual(receipt["status"], "verified")
        self.assertEqual([item["ordinal"] for item in receipt["deliveries"]], [1, 2])
        self.assertEqual(receipt["candidate_sha256"], digest(candidate))
        self.assertEqual(receipt["evidence_sha256"], digest(evidence))
        unsigned = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
        self.assertEqual(receipt["receipt_sha256"], "sha256:" + digest(unsigned))

    def test_rejects_coordinator_plan_pointer_or_body_digest_drift(self):
        values = list(self.context())
        values[3] = copy.deepcopy(values[3])
        values[3]["body"] = values[3]["body"].replace("6001", "6002")
        with self.assertRaisesRegex(ValueError, "plan comment|plan digest"):
            self.build(tuple(values))

    def test_rejects_malformed_or_duplicate_coordinator_task_uid_fields(self):
        plan, _candidate, _evidence, coordinator, _comment, _children = self.context()
        expected_uid = plan["task_uid"]
        malformed_duplicate = coordinator["body"] + "task_uid: malformed\n"
        duplicate_same = coordinator["body"] + f"task_uid: {expected_uid}\n"
        duplicate_conflict = coordinator["body"] + f"task_uid: {task_uid('c')}\n"
        malformed_only = coordinator["body"].replace(
            f"task_uid: {expected_uid}\n", "task_uid: malformed\n",
        )
        for label, body in (
            ("valid plus malformed duplicate", malformed_duplicate),
            ("duplicate same UID", duplicate_same),
            ("duplicate conflicting UID", duplicate_conflict),
            ("malformed only", malformed_only),
        ):
            with self.subTest(coordinator_task_uid=label):
                values = list(self.context())
                values[3] = {**values[3], "body": body}
                with self.assertRaises(self.helper.ReceiptError):
                    self.build(tuple(values))

    def test_rejects_malformed_duplicate_coordinator_route_fields_on_open_and_closed_paths(self):
        plan, candidate, evidence, coordinator, comment, children = self.context()
        receipt = self.build((plan, candidate, evidence, coordinator, comment, children))
        malformed_fields = (
            ("completion mode", "- completion_mode : `pr_task`"),
            ("plan comment pointer", "- aggregate_plan_comment_id : `9999`"),
            ("singular PR number", "- pr_number : `9999`"),
            ("singular PR URL", "- pr_url : `https://github.com/eng-cc/oasis7/pull/9999`"),
        )
        for label, malformed_field in malformed_fields:
            body = coordinator["body"] + malformed_field + "\n"
            open_issue = {**coordinator, "body": body}
            with self.subTest(field=label, path="open-build"):
                with self.assertRaises(self.helper.ReceiptError):
                    self.build((plan, candidate, evidence, open_issue, comment, children))

            closed_issue = {**coordinator, "state": "CLOSED", "body": body}
            with self.subTest(field=label, path="closed-replay"):
                with (
                    mock.patch.object(
                        self.helper, "read_coordinator", return_value=(closed_issue, comment),
                    ),
                    mock.patch.object(
                        self.helper,
                        "read_child_report",
                        side_effect=lambda _root, delivery, _branch: children[delivery["task_uid"]],
                    ),
                ):
                    with self.assertRaises(self.helper.ReceiptError):
                        self.helper.validate_terminal_receipt(
                            ROOT, plan["task_uid"], plan, candidate, evidence, receipt,
                        )

    def test_rejects_duplicate_or_noncontiguous_delivery_ordinals(self):
        for mutate in (
            lambda plan: plan["required_deliveries"][1].update(ordinal=1),
            lambda plan: plan["required_deliveries"][1].update(ordinal=3),
        ):
            values = list(self.context())
            values[0] = copy.deepcopy(values[0])
            mutate(values[0])
            values[4] = copy.deepcopy(values[4])
            values[4]["body"] = comment_body(values[0])
            values[3] = copy.deepcopy(values[3])
            values[3]["body"] = values[3]["body"].replace(
                "sha256:" + hashlib.sha256(comment_body(self.context()[0]).encode()).hexdigest(),
                "sha256:" + hashlib.sha256(comment_body(values[0]).encode()).hexdigest(),
            )
            with self.assertRaisesRegex(ValueError, "ordinal|order"):
                self.build(tuple(values))

    def test_rejects_candidate_or_evidence_digest_mismatch(self):
        values = list(self.context())
        values[1] = copy.deepcopy(values[1])
        values[1]["tested_tree_oid"] = "d" * 40
        with self.assertRaisesRegex(ValueError, "candidate|tested_tree"):
            self.build(tuple(values))

    def test_rejects_child_without_live_terminal_chain(self):
        values = list(self.context())
        values[5] = copy.deepcopy(values[5])
        uid = values[0]["required_deliveries"][0]["task_uid"]
        values[5][uid]["checks"]["terminal_tombstone_valid"] = False
        with self.assertRaisesRegex(ValueError, "terminal|reconciled"):
            self.build(tuple(values))

    def test_rejects_child_task_or_pr_reciprocity_mismatch(self):
        values = list(self.context())
        values[5] = copy.deepcopy(values[5])
        uid = values[0]["required_deliveries"][0]["task_uid"]
        values[5][uid]["task"]["pr_number"] = 9999
        with self.assertRaisesRegex(ValueError, "child|PR|identity"):
            self.build(tuple(values))

    def test_dependency_merge_chronology_is_enforced_during_receipt_build(self):
        cases = (
            ("prerequisite strictly earlier", "2026-09-25T09:02:00Z", "2026-09-25T09:03:00Z", True),
            ("equal merge timestamps", "2026-09-25T09:03:00Z", "2026-09-25T09:03:00Z", False),
            ("dependent merged earlier", "2026-09-25T09:04:00Z", "2026-09-25T09:03:00Z", False),
        )
        for label, prerequisite_at, dependent_at, accepted in cases:
            with self.subTest(chronology=label):
                values = list(self.context())
                children = copy.deepcopy(values[5])
                deliveries = values[0]["required_deliveries"]
                children[deliveries[0]["task_uid"]]["live"]["pr"]["merged_at"] = prerequisite_at
                children[deliveries[1]["task_uid"]]["live"]["pr"]["merged_at"] = dependent_at
                values[5] = children
                if accepted:
                    receipt = self.build(tuple(values))
                    self.assertEqual(
                        [item["merged_at"] for item in receipt["deliveries"]],
                        [prerequisite_at, dependent_at],
                    )
                else:
                    with self.assertRaises(self.helper.ReceiptError):
                        self.build(tuple(values))

    def test_dependency_merge_chronology_is_enforced_by_receipt_validation(self):
        cases = (
            ("prerequisite strictly earlier", "2026-09-25T09:02:00Z", "2026-09-25T09:03:00Z", True),
            ("equal merge timestamps", "2026-09-25T09:03:00Z", "2026-09-25T09:03:00Z", False),
            ("dependent merged earlier", "2026-09-25T09:04:00Z", "2026-09-25T09:03:00Z", False),
        )
        for label, prerequisite_at, dependent_at, accepted in cases:
            with self.subTest(chronology=label):
                receipt = copy.deepcopy(self.build())
                receipt["deliveries"][0]["merged_at"] = prerequisite_at
                receipt["deliveries"][1]["merged_at"] = dependent_at
                unsigned = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
                receipt["receipt_sha256"] = self.helper.canonical_digest(unsigned, prefix=True)
                observed = self.helper._timestamp(receipt["observed_at"], "receipt observed_at")
                if accepted:
                    self.assertIs(self.helper.validate_receipt(receipt, now=observed), receipt)
                else:
                    with self.assertRaises(self.helper.ReceiptError):
                        self.helper.validate_receipt(receipt, now=observed)

    def test_live_child_issue_rejects_ambiguous_fields_and_requires_exact_project_item(self):
        plan, _candidate, _evidence, _coordinator, _comment, _reports = self.context()
        delivery = plan["required_deliveries"][0]
        uid = delivery["task_uid"]
        task_branch = "task/aggregate-child-a"
        claim = {
            "claim_type": "task_complete", "status": "verified",
            "verification_exit_code": 0,
        }
        claims = [claim]
        encoded_claims = base64.urlsafe_b64encode(
            json.dumps(claims, sort_keys=True, separators=(",", ":")).encode("utf-8")
        ).decode("ascii").rstrip("=")
        issue_fields = [
            f"task_uid: {uid}",
            "- status: `done`",
            "- workflow_phase: `post_merge_done`",
            "- completion_mode: `pr_task`",
            f"- pr_number: `{delivery['pr_number']}`",
            f"- pr_url: `{delivery['pr_url']}`",
            f"- claim_verifications_b64: `{encoded_claims}`",
        ]
        task = {
            "task_uid": uid,
            "repository": "eng-cc/oasis7",
            "issue_number": delivery["issue_number"],
            "pr_number": delivery["pr_number"],
            "pr_url": delivery["pr_url"],
            "project_item_id": "PVTI_child_a",
            "status": "done",
            "workflow_phase": "post_merge_done",
            "completion_mode": "pr_task",
            "claim_verifications": claims,
            "task_branch": task_branch,
        }
        project_mapping = {"owner": "eng-cc", "number": 1, "repo": "eng-cc/oasis7"}
        live_project_item = {
            "id": "PVTI_child_a",
            "_project_owner": "eng-cc",
            "_project_number": 1,
            "content": {
                "body": f"task_uid: {uid}",
                "number": delivery["issue_number"],
                "url": f"https://github.com/eng-cc/oasis7/issues/{delivery['issue_number']}",
            },
            "_field_values_has_next_page": False,
            "Status": "Done",
            "PM Status": "done",
            "Workflow Phase": "done",
        }
        report = {
            "status": "reconciled",
            "live": {"pr": {
                "state": "MERGED", "mergedAt": "2026-09-25T09:00:00Z",
                "headRefName": task_branch,
            }, "project_item": live_project_item},
        }
        merged_at = "2026-09-25T09:00:00Z"
        live_pr = {
            "number": delivery["pr_number"], "url": delivery["pr_url"],
            "state": "MERGED", "mergedAt": merged_at,
            "mergeCommit": {"oid": "5" * 40}, "headRefOid": "4" * 40,
            "baseRefName": "main", "body": f"Refs #{delivery['issue_number']}",
        }

        cases = (
            ("status", "done"),
            ("status", "in_progress"),
            ("workflow_phase", "post_merge_done"),
            ("workflow_phase", "task_done"),
            ("task_uid", uid),
            ("task_uid", task_uid("c")),
            ("completion_mode", "pr_task"),
            ("completion_mode", "non_pr_task"),
            ("pr_number", str(delivery["pr_number"])),
            ("pr_number", "5199"),
            ("pr_url", delivery["pr_url"]),
            ("pr_url", "https://github.com/eng-cc/oasis7/pull/5199"),
        )
        with tempfile.TemporaryDirectory() as raw_receipts:
            receipt_root = pathlib.Path(raw_receipts)
            for filename in (
                "merge-receipt.json", "main-sync-receipt.json",
                "terminal-cleanup-receipt.json", "terminal-tombstone.json",
            ):
                (receipt_root / filename).write_text("{}\n", encoding="utf-8")

            def read_child_with_body(
                body, mapped_task=task, project_item=live_project_item,
                mapped_project=project_mapping,
            ):
                live_issue = {
                    "number": delivery["issue_number"],
                    "url": f"https://github.com/eng-cc/oasis7/issues/{delivery['issue_number']}",
                    "state": "CLOSED", "body": body,
                }
                terminal_audit = mock.Mock()
                terminal_audit.audit.return_value = {
                    **report,
                    "live": {**report["live"], "project_item": project_item},
                    "receipt_root": str(receipt_root),
                }
                with (
                    mock.patch.object(
                        self.helper, "_load_mapping",
                        return_value={"project": mapped_project, "tasks": {uid: mapped_task}},
                    ),
                    mock.patch.object(self.helper, "_import_terminal_audit", return_value=terminal_audit),
                    mock.patch.object(self.helper, "_run_json", side_effect=(live_issue, live_pr)),
                ):
                    return self.helper.read_child_report(ROOT, delivery, "main")

            valid_report = read_child_with_body("\n".join(issue_fields))
            self.assertEqual(valid_report["task"]["task_uid"], uid)
            self.assertEqual(valid_report["task"]["status"], "done")
            self.assertEqual(valid_report["task"]["workflow_phase"], "post_merge_done")
            task_without_completion_mode = dict(task)
            task_without_completion_mode.pop("completion_mode")
            body_without_completion_mode = "\n".join(
                line for line in issue_fields if not line.startswith("- completion_mode:")
            )
            optional_mode_report = read_child_with_body(
                body_without_completion_mode, task_without_completion_mode,
            )
            self.assertEqual(optional_mode_report["task"]["task_uid"], uid)

            # A child delivery needs its own current, exact Project item proof.
            for label, changed_task in (
                ("missing mapped Project item", {key: value for key, value in task.items() if key != "project_item_id"}),
                ("empty mapped Project item", {**task, "project_item_id": ""}),
                ("mismatched mapped Project item", {**task, "project_item_id": "PVTI_other"}),
            ):
                with self.subTest(project_case=label):
                    with self.assertRaises(self.helper.ReceiptError):
                        read_child_with_body("\n".join(issue_fields), mapped_task=changed_task)

            with self.subTest(project_case="missing live Project item"):
                with self.assertRaises(self.helper.ReceiptError):
                    read_child_with_body("\n".join(issue_fields), project_item={})

            wrong_project_items = (
                ("live item id", {**live_project_item, "id": "PVTI_other"}),
                ("Project owner", {**live_project_item, "_project_owner": "another-owner"}),
                ("Project number", {**live_project_item, "_project_number": 2}),
                ("Issue number", {
                    **live_project_item,
                    "content": {**live_project_item["content"], "number": delivery["issue_number"] + 1},
                }),
                ("Issue URL", {
                    **live_project_item,
                    "content": {
                        **live_project_item["content"],
                        "url": f"https://github.com/eng-cc/oasis7/issues/{delivery['issue_number'] + 1}",
                    },
                }),
                ("Issue Task UID", {
                    **live_project_item,
                    "content": {**live_project_item["content"], "body": f"task_uid: {task_uid('c')}"},
                }),
                ("duplicate Issue Task UID", {
                    **live_project_item,
                    "content": {**live_project_item["content"], "body": f"task_uid: {uid}\ntask_uid: {uid}"},
                }),
                ("conflicting duplicate Issue Task UID", {
                    **live_project_item,
                    "content": {**live_project_item["content"], "body": f"task_uid: {uid}\ntask_uid: {task_uid('c')}"},
                }),
                ("incomplete Project fields", {
                    **live_project_item, "_field_values_has_next_page": True,
                }),
                ("wrong terminal Status", {**live_project_item, "Status": "In Progress"}),
                ("wrong terminal PM Status", {**live_project_item, "PM Status": "in_progress"}),
                ("wrong terminal Workflow Phase", {**live_project_item, "Workflow Phase": "post_merge_done"}),
            )
            for label, item in wrong_project_items:
                with self.subTest(project_case=label):
                    with self.assertRaises(self.helper.ReceiptError):
                        read_child_with_body("\n".join(issue_fields), project_item=item)

            for label, project in (
                ("wrong mapped Project owner", {**project_mapping, "owner": "another-owner"}),
                ("wrong mapped Project number", {**project_mapping, "number": 2}),
                ("wrong mapped Project repository", {**project_mapping, "repo": "another/repo"}),
            ):
                with self.subTest(project_case=label):
                    with self.assertRaises(self.helper.ReceiptError):
                        read_child_with_body("\n".join(issue_fields), mapped_project=project)

            for field, duplicate_value in cases:
                with self.subTest(field=field, duplicate_value=duplicate_value):
                    duplicate = (
                        f"task_uid: {duplicate_value}"
                        if field == "task_uid"
                        else f"- {field}: `{duplicate_value}`"
                    )
                    body = "\n".join((*issue_fields, duplicate))
                    with self.assertRaises(self.helper.ReceiptError):
                        read_child_with_body(body)

    def test_rejects_modified_receipt_digest(self):
        receipt = self.build()
        receipt["deliveries"][0]["merge_commit_oid"] = "e" * 40
        with self.assertRaisesRegex(ValueError, "receipt digest"):
            self.helper.validate_receipt(receipt)

    def test_rejects_malformed_child_proof_fields_with_recomputed_receipt_digest(self):
        invalid_fields = (
            ("merge_commit_oid", "not-an-oid"),
            ("head_oid", "not-an-oid"),
            ("merged_at", "yesterday"),
            ("task_complete_claim_sha256", "not-a-sha256"),
            ("merge_receipt_sha256", "not-a-sha256"),
            ("main_sync_receipt_sha256", "not-a-sha256"),
            ("terminal_receipt_sha256", "not-a-sha256"),
            ("terminal_tombstone_sha256", "not-a-sha256"),
        )
        for key, value in invalid_fields:
            with self.subTest(key=key):
                receipt = copy.deepcopy(self.build())
                receipt["deliveries"][0][key] = value
                unsigned = {name: item for name, item in receipt.items() if name != "receipt_sha256"}
                receipt["receipt_sha256"] = self.helper.canonical_digest(unsigned, prefix=True)
                observed = self.helper._timestamp(receipt["observed_at"], "receipt observed_at")
                with self.assertRaisesRegex(ValueError, "delivery|commit|timestamp|digest|SHA-256"):
                    self.helper.validate_receipt(receipt, now=observed)

    def test_build_rejects_malformed_live_child_merge_timestamp(self):
        values = list(self.context())
        children = copy.deepcopy(values[5])
        first_uid = values[0]["required_deliveries"][0]["task_uid"]
        children[first_uid]["live"]["pr"]["merged_at"] = "yesterday"
        values[5] = children
        with self.assertRaises(self.helper.ReceiptError):
            self.build(tuple(values))

    def test_terminal_receipt_revalidation_reads_every_child_against_closed_coordinator(self):
        plan, candidate, evidence, coordinator, comment, child_reports = self.context()
        receipt = self.build((plan, candidate, evidence, coordinator, comment, child_reports))
        closed_coordinator = {**coordinator, "state": "CLOSED"}
        with mock.patch.object(self.helper, "read_coordinator", return_value=(closed_coordinator, comment)), \
                mock.patch.object(
                    self.helper, "read_child_report",
                    side_effect=lambda _root, delivery, _branch: child_reports[delivery["task_uid"]],
                ) as read_child:
            result = self.helper.validate_terminal_receipt(
                ROOT, plan["task_uid"], plan, candidate, evidence, receipt,
            )

        self.assertEqual(result, receipt)
        self.assertEqual(read_child.call_count, len(plan["required_deliveries"]))
        self.assertEqual(
            [call.args[1]["task_uid"] for call in read_child.call_args_list],
            [delivery["task_uid"] for delivery in plan["required_deliveries"]],
        )

    def test_terminal_receipt_revalidation_rejects_drifted_child_terminal_chain(self):
        plan, candidate, evidence, coordinator, comment, child_reports = self.context()
        receipt = self.build((plan, candidate, evidence, coordinator, comment, child_reports))
        closed_coordinator = {**coordinator, "state": "CLOSED"}
        reports = copy.deepcopy(child_reports)
        second_uid = plan["required_deliveries"][1]["task_uid"]
        reports[second_uid]["checks"]["terminal_receipt_chain_valid"] = False
        with mock.patch.object(self.helper, "read_coordinator", return_value=(closed_coordinator, comment)), \
                mock.patch.object(
                    self.helper, "read_child_report",
                    side_effect=lambda _root, delivery, _branch: reports[delivery["task_uid"]],
                ):
            with self.assertRaisesRegex(ValueError, "terminal receipt chain is incomplete"):
                self.helper.validate_terminal_receipt(
                    ROOT, plan["task_uid"], plan, candidate, evidence, receipt,
                )

    def test_terminal_replay_enforces_dependency_merge_chronology(self):
        cases = (
            ("prerequisite strictly earlier", "2026-09-25T09:02:00Z", "2026-09-25T09:03:00Z", True),
            ("equal merge timestamps", "2026-09-25T09:03:00Z", "2026-09-25T09:03:00Z", False),
            ("dependent merged earlier", "2026-09-25T09:04:00Z", "2026-09-25T09:03:00Z", False),
        )
        for label, prerequisite_at, dependent_at, accepted in cases:
            with self.subTest(chronology=label):
                plan, candidate, evidence, coordinator, comment, child_reports = self.context()
                receipt = copy.deepcopy(self.build())
                receipts = receipt["deliveries"]
                receipts[0]["merged_at"] = prerequisite_at
                receipts[1]["merged_at"] = dependent_at
                unsigned = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
                receipt["receipt_sha256"] = self.helper.canonical_digest(unsigned, prefix=True)

                reports = copy.deepcopy(child_reports)
                reports[plan["required_deliveries"][0]["task_uid"]]["live"]["pr"]["merged_at"] = prerequisite_at
                reports[plan["required_deliveries"][1]["task_uid"]]["live"]["pr"]["merged_at"] = dependent_at
                closed_coordinator = {**coordinator, "state": "CLOSED"}
                with mock.patch.object(self.helper, "read_coordinator", return_value=(closed_coordinator, comment)), \
                        mock.patch.object(
                            self.helper, "read_child_report",
                            side_effect=lambda _root, delivery, _branch: reports[delivery["task_uid"]],
                        ):
                    if accepted:
                        self.assertEqual(
                            self.helper.validate_terminal_receipt(
                                ROOT, plan["task_uid"], plan, candidate, evidence, receipt,
                            ),
                            receipt,
                        )
                    else:
                        with self.assertRaises(self.helper.ReceiptError):
                            self.helper.validate_terminal_receipt(
                                ROOT, plan["task_uid"], plan, candidate, evidence, receipt,
                            )

    def test_real_mixed_v1_v2_child_readers_build_and_revalidate_aggregate(self):
        protocol = load_test_module(
            ROOT / "scripts/pm/terminal-delivery-protocol.test.py",
            "aggregate_actual_delivery_fixture",
        )
        legacy = load_test_module(
            ROOT / "scripts/pm/loop_terminal.test.py",
            "aggregate_actual_legacy_fixture",
        )
        claim_api = load_test_module(
            ROOT / "scripts/pm/task_complete_claim.py",
            "aggregate_actual_claim_api",
        )

        with tempfile.TemporaryDirectory(prefix="oasis7-aggregate-live-reader-") as directory:
            parent = pathlib.Path(directory)
            legacy_parent = parent / "legacy"
            delivery_parent = parent / "delivery"
            legacy_parent.mkdir()
            delivery_parent.mkdir()

            # V1 uses the immutable legacy receipt/comment fixture. V2 is
            # produced by the real finalizer against its isolated GitHub stub.
            v1_fixture = protocol.DeliveryFixture(legacy_parent)
            v1_root = v1_fixture.root
            copy_pm_helpers(v1_root)

            v2_uid = task_uid("b")
            v2_issue = 4102
            v2_pr = 5102
            v2_pr_url = f"https://github.com/fixture/repo/pull/{v2_pr}"
            v2_issue_url = f"https://github.com/fixture/repo/issues/{v2_issue}"
            with mock.patch.object(protocol, "UID", v2_uid), \
                    mock.patch.object(protocol, "ISSUE", v2_issue), \
                    mock.patch.object(protocol, "PR", v2_pr), \
                    mock.patch.object(protocol, "PR_URL", v2_pr_url), \
                    mock.patch.object(protocol, "ISSUE_URL", v2_issue_url):
                v2_fixture = protocol.DeliveryFixture(delivery_parent)
                producer = v2_fixture.bin / "gh"
                producer_source = producer.read_text(encoding="utf-8")
                comment_write = 'state["comments"].append(comment); state_path.write_text(json.dumps(state))'
                self.assertIn(comment_write, producer_source)
                # GitHub advances the parent Issue's updatedAt for comments.
                # The immutable accepted claim comment is at NOW; the producer
                # later publishes unrelated terminal evidence, which must not
                # make historical claim selection reject that accepted claim.
                producer_source = producer_source.replace(
                    comment_write,
                    'comment["created_at"] = "2026-09-30T08:00:01Z"; '
                    'comment["updated_at"] = comment["created_at"]; '
                    'state["comments"].append(comment); '
                    'state["issue"]["updated_at"] = comment["created_at"]; '
                    'state_path.write_text(json.dumps(state))',
                )
                for old, new in (
                    ("issues/11/comments", f"issues/{v2_issue}/comments"),
                    ("issues/11", f"issues/{v2_issue}"),
                    ("pulls/12", f"pulls/{v2_pr}"),
                    ("issues/11#issuecomment-", f"issues/{v2_issue}#issuecomment-"),
                ):
                    producer_source = producer_source.replace(old, new)
                producer.write_text(producer_source, encoding="utf-8")
                producer.chmod(0o755)
                produced = v2_fixture.run_producer()
                self.assertEqual(produced.returncode, 0, produced.stderr)

            v2_root = v2_fixture.root
            copy_pm_helpers(v2_root)
            install_issue_pr_view_readback(v1_fixture)
            install_issue_pr_view_readback(v2_fixture)

            # The terminal task audit and aggregate reader consume these live
            # fields. Add the completed task projection as the closeout's live
            # Issue readback while leaving the producer's receipt/comment bytes
            # untouched.
            v2_mapping = v2_fixture.mapping()
            v2_record = v2_mapping["tasks"][v2_uid]
            v2_record["completion_mode"] = "pr_task"
            v2_record["status"] = "done"
            v2_record["workflow_phase"] = "post_merge_done"
            # The producer's fake gh process persists its GitHub readbacks to
            # disk; refresh the in-memory fixture before adding the aggregate
            # task projection so the closed Issue and immutable comments remain
            # the actual producer output.
            v2_fixture.state = json.loads(v2_fixture.state_path.read_text(encoding="utf-8"))
            claim_comment = next(
                comment for comment in v2_fixture.state["comments"]
                if comment.get("id") == 801
            )
            terminal_comment = next(
                comment for comment in v2_fixture.state["comments"]
                if "<!-- oasis7-pm-evidence/v2 -->" in comment.get("body", "")
            )
            self.assertGreater(terminal_comment["created_at"], claim_comment["created_at"])
            self.assertEqual(
                v2_fixture.state["issue"]["updated_at"], terminal_comment["created_at"],
                "unrelated terminal evidence must advance the live Issue updatedAt",
            )
            v2_body = v2_fixture.state["issue"]["body"] + (
                "- status: `done`\n"
                "- workflow_phase: `post_merge_done`\n"
                "- completion_mode: `pr_task`\n"
            )
            v2_fixture.state["issue"]["body"] = v2_body
            v2_fixture.state["project_item"]["content"]["body"] = v2_body
            v2_fixture.state["pr"]["head"]["ref"] = v2_record["task_branch"]
            v2_fixture.mapping_path.write_text(
                json.dumps(v2_mapping, sort_keys=True) + "\n", encoding="utf-8",
            )
            v2_fixture._write_state()

            v1_uid = legacy.UID
            v1_issue = 11
            v1_pr = legacy.PR_NUMBER
            v1_pr_url = legacy.PR_URL
            v1_claim = copy.deepcopy(v1_fixture.record["claim_verifications"][0])
            v1_claim["task_uid"] = v1_uid
            v1_claim["frozen_source_head"] = legacy.PR["head"]["sha"]
            v1_claim["repository_head"] = legacy.PR["head"]["sha"]
            v1_claim["frozen_source_tree"] = "c" * 40
            v1_claim["verified_at"] = "2026-09-10T00:00:00Z"
            claim_history = base64.urlsafe_b64encode(
                canonical_bytes([v1_claim]),
            ).decode("ascii").rstrip("=")
            v1_body = "\n".join((
                "<!-- oasis7-pm-task -->",
                f"task_uid: {v1_uid}",
                "- status: `done`",
                "- workflow_phase: `post_merge_done`",
                "- completion_mode: `pr_task`",
                f"- pr_number: `{v1_pr}`",
                f"- pr_url: `{v1_pr_url}`",
                f"- claim_verifications_b64: `{claim_history}`",
            )) + "\n"
            claim_created_at = "2026-09-10T00:00:01Z"
            v1_claim_comment = {
                "id": 8,
                "body": claim_api._claim_comment_body(v1_uid, v1_claim),
                "created_at": claim_created_at,
                "updated_at": claim_created_at,
                "html_url": f"{legacy.URL}#issuecomment-8",
                "issue_url": f"https://api.github.com/repos/fixture/repo/issues/{v1_issue}",
            }
            legacy_comment = copy.deepcopy(legacy.COMMENT)
            legacy_comment.update({
                "id": 7,
                "issue_url": f"https://api.github.com/repos/fixture/repo/issues/{v1_issue}",
                "created_at": "2026-09-10T00:00:02Z",
                "updated_at": "2026-09-10T00:00:02Z",
            })
            v1_issue_readback = copy.deepcopy(legacy.ISSUE)
            v1_issue_readback.update({
                "body": v1_body,
                "state": "closed",
                "state_reason": "completed",
                "updated_at": claim_created_at,
            })
            v1_item = copy.deepcopy(legacy.ITEM)
            v1_item["id"] = v1_fixture.state["project_item"]["id"]
            v1_item["project"]["id"] = "PROJECT_fixture"
            v1_item["content"].update({"number": v1_issue, "url": legacy.URL, "body": v1_body})
            v1_item["fieldValues"]["pageInfo"]["hasNextPage"] = False
            v1_pr_readback = copy.deepcopy(legacy.PR)
            v1_pr_readback["head"]["ref"] = "task/fixture"
            v1_receipts = legacy.RECEIPTS
            v1_receipt_root = pathlib.Path(subprocess.check_output([
                sys.executable, str(ROOT / "scripts/pm/canonical-receipt-root.py"),
                "--default-worktree", str(v1_root), "--task-uid", v1_uid, "--create",
            ], text=True).strip())
            v1_raw = {}
            for key, filename in (
                ("merge", "merge-receipt.json"),
                ("main_sync", "main-sync-receipt.json"),
                ("terminal", "terminal-cleanup-receipt.json"),
                ("tombstone", "terminal-tombstone.json"),
            ):
                v1_raw[filename] = v1_receipts[key]["bytes"]
                (v1_receipt_root / filename).write_bytes(v1_raw[filename])
            v1_ledger = {
                "schema": "oasis7_finalizer_ledger_v1",
                "task_uid": v1_uid,
                "operations": {
                    effect: {
                        "effect": effect,
                        "operation_id": hashlib.sha256(
                            f"{v1_uid}:post_merge_done:{effect}".encode("utf-8"),
                        ).hexdigest(),
                        "intent": True,
                        "readback": True,
                        "committed": True,
                    }
                    for effect in ("issue_close", "project_update", "evidence_comment")
                },
            }
            v1_raw["finalizer-ledger.json"] = canonical_bytes(v1_ledger) + b"\n"
            (v1_receipt_root / "finalizer-ledger.json").write_bytes(v1_raw["finalizer-ledger.json"])
            v1_record = {
                "task_uid": v1_uid,
                "repository": "fixture/repo",
                "status": "done",
                "workflow_phase": "post_merge_done",
                "completion_mode": "pr_task",
                "issue_number": v1_issue,
                "issue_url": legacy.URL,
                "pr_number": v1_pr,
                "pr_url": v1_pr_url,
                "default_branch": "main",
                "canonical_worktree": "/fixture/worktree",
                "task_branch": "task/fixture",
                "project_item_id": v1_item["id"],
                "claim_verifications": [v1_claim],
                "merge_receipt": v1_receipts["merge"]["record"],
                "merge_receipt_sha256": v1_receipts["merge"]["digest"],
                "phase_receipts": {
                    "main_sync": v1_receipts["main_sync"]["record"],
                    "post_merge_done": v1_receipts["terminal"]["record"],
                },
                "phase_receipt_sha256": {
                    "main_sync": v1_receipts["main_sync"]["digest"],
                    "post_merge_done": v1_receipts["terminal"]["digest"],
                },
            }
            v1_mapping = json.loads(v1_fixture.mapping_path.read_text(encoding="utf-8"))
            v1_mapping["tasks"] = {v1_uid: v1_record}
            v1_fixture.mapping_path.write_text(
                json.dumps(v1_mapping, sort_keys=True) + "\n", encoding="utf-8",
            )
            v1_fixture.state.update({
                "issue": v1_issue_readback,
                "project_item": v1_item,
                "pr": v1_pr_readback,
                "comments": [legacy_comment, v1_claim_comment],
            })
            v1_fixture._write_state()

            values = list(self.context())
            plan, candidate, evidence, coordinator, _old_comment, _old_reports = values
            first, second = plan["required_deliveries"]
            first.update({
                "task_uid": v1_uid, "issue_number": v1_issue,
                "pr_number": v1_pr, "pr_url": v1_pr_url,
            })
            second.update({
                "task_uid": v2_uid, "issue_number": v2_issue,
                "pr_number": v2_pr, "pr_url": v2_pr_url,
            })
            plan["repository"] = "fixture/repo"
            plan_comment_body = comment_body(plan)
            comment = {
                "id": 6001, "issue_number": plan["issue_number"],
                "body": plan_comment_body, "author": "eng-cc", "permission": "admin",
            }
            plan_body_digest = hashlib.sha256(plan_comment_body.encode("utf-8")).hexdigest()
            coordinator["repository"] = "fixture/repo"
            coordinator["default_branch"] = "main"
            coordinator["body"] = (
                "<!-- oasis7-pm-task -->\n"
                f"task_uid: {plan['task_uid']}\n"
                "completion_mode: ordered_delivery_aggregate\n"
                "aggregate_plan_comment_id: 6001\n"
                f"aggregate_plan_sha256: sha256:{plan_body_digest}\n"
            )

            fixtures = {v1_uid: v1_fixture, v2_uid: v2_fixture}
            original_reader = self.helper.read_child_report
            with mock.patch.object(self.helper, "REPOSITORY", "fixture/repo"):
                reports = {}
                for delivery in plan["required_deliveries"]:
                    fixture = fixtures[delivery["task_uid"]]
                    with mock.patch.dict(os.environ, fixture.env()):
                        audit_module = self.helper._import_terminal_audit(fixture.root)
                        audit_report = audit_module.audit(fixture.root, delivery["task_uid"])
                        self.assertEqual(
                            audit_report["status"], "reconciled",
                            json.dumps(audit_report, sort_keys=True),
                        )
                        reports[delivery["task_uid"]] = original_reader(
                            fixture.root, delivery, "main",
                        )

                self.assertEqual(
                    [reports[row["task_uid"]]["proof"]["protocol_version"]
                     for row in plan["required_deliveries"]],
                    [1, 2],
                )
                self.assertEqual(reports[v1_uid]["task"]["issue_number"], v1_issue)
                self.assertEqual(reports[v1_uid]["task"]["pr_number"], v1_pr)
                self.assertEqual(reports[v2_uid]["task"]["issue_number"], v2_issue)
                self.assertEqual(reports[v2_uid]["task"]["pr_number"], v2_pr)

                observed_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
                receipt = self.helper.build_receipt(
                    task_uid=plan["task_uid"], plan=plan, candidate=candidate, evidence=evidence,
                    coordinator_issue=coordinator, plan_comment=comment, child_reports=reports,
                    effective_validator_commit="c" * 40, observed_at=observed_at,
                )
                self.assertEqual(receipt["schema"], "oasis7.aggregate-task-completion/v2")
                self.assertEqual(
                    [row["terminal_protocol_version"] for row in receipt["deliveries"]], [1, 2],
                )

                closed_coordinator = {**coordinator, "state": "CLOSED"}

                def reread_child(_root, delivery, default_branch):
                    fixture = fixtures[delivery["task_uid"]]
                    with mock.patch.dict(os.environ, fixture.env()):
                        return original_reader(fixture.root, delivery, default_branch)

                with mock.patch.object(
                    self.helper, "read_coordinator", return_value=(closed_coordinator, comment),
                ), mock.patch.object(self.helper, "read_child_report", side_effect=reread_child):
                    self.assertEqual(
                        self.helper.validate_terminal_receipt(
                            v2_root, plan["task_uid"], plan, candidate, evidence, receipt,
                        ),
                        receipt,
                    )

                v2_raw_delivery = (v2_fixture.receipt_root / "terminal-delivery-receipt.json").read_bytes()
                v2_delivery_digest = hashlib.sha256(v2_raw_delivery).hexdigest()
                self.assertEqual(
                    v2_record["phase_receipt_sha256"]["post_merge_done"], v2_delivery_digest,
                )
                self.assertEqual(
                    receipt["deliveries"][1]["selected_terminal_receipt_sha256"],
                    v2_delivery_digest,
                )
                v2_map_saved = v2_fixture.mapping_path.read_bytes()
                for label, mutate in (
                    ("unknown protocol selector", lambda row: row["phase_receipt_type"].__setitem__(
                        "post_merge_done", "oasis7_terminal_delivery_v9",
                    )),
                    ("changed raw receipt digest", lambda row: row["phase_receipt_sha256"].__setitem__(
                        "post_merge_done", "0" * 64,
                    )),
                ):
                    with self.subTest(v2_selector=label):
                        v2_mutated_map = json.loads(v2_map_saved)
                        mutate(v2_mutated_map["tasks"][v2_uid])
                        v2_fixture.mapping_path.write_text(
                            json.dumps(v2_mutated_map, sort_keys=True) + "\n", encoding="utf-8",
                        )
                        with mock.patch.dict(os.environ, v2_fixture.env()):
                            with self.assertRaisesRegex(
                                self.helper.ReceiptError, "not reconciled|proof readback",
                            ):
                                original_reader(v2_root, second, "main")
                v2_fixture.mapping_path.write_bytes(v2_map_saved)

                v1_after = {
                    name: (v1_receipt_root / name).read_bytes() for name in v1_raw
                }
                self.assertEqual(v1_after, v1_raw)
                self.assertEqual(
                    (v2_fixture.receipt_root / "terminal-delivery-receipt.json").read_bytes(),
                    v2_raw_delivery,
                )


if __name__ == "__main__":
    unittest.main()
