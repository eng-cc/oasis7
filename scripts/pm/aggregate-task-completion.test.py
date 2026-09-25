#!/usr/bin/env python3
"""Focused contract tests for linked-delivery aggregate completion receipts."""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import pathlib
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
        for delivery, uid in zip(plan["required_deliveries"], (uid_a, uid_b)):
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
                        "merged_at": "2026-09-25T09:00:00Z",
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


if __name__ == "__main__":
    unittest.main()
