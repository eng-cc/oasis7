#!/usr/bin/env python3
import importlib.util
import unittest
from pathlib import Path


HERE = Path(__file__).parent
SPEC = importlib.util.spec_from_file_location("ci_ready_receipt_identity", HERE / "ci_ready_receipt_identity.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

UID = "task_12345678901234567890123456789012"
HEAD = "a" * 40
SOURCE_SCOPE = "b" * 40
INTEGRATION_BASE = "c" * 40
TREE = "d" * 40


def source_fields():
    return {
        "task_uid": UID,
        "bootstrap_epoch": 1,
        "repository": "eng-cc/oasis7",
        "pr_number": 7,
        "source_head_oid": HEAD,
        "source_scope_oid": SOURCE_SCOPE,
        "changed_paths_digest": "f" * 64,
        "ordered_role_ids": ["qa_engineer", "repository_health_engineer"],
        "role_contract_digest": "1" * 64,
        "review_policy_digest": "2" * 64,
        "input_contract_digest": "3" * 64,
    }


def integration_receipt(**changes):
    value = {
        "repository": "eng-cc/oasis7",
        "task_uid": UID,
        "pr_number": 7,
        "source_head_oid": HEAD,
        "integration_base_oid": INTEGRATION_BASE,
        "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
        "workflow_sha": "4" * 40,
        "request_id": 11,
        "request_created_at": "2026-09-11T00:00:00Z",
        "run_id": 12,
        "run_attempt": 1,
        "check_app_id": 42,
        "check_run_id": 13,
        "planner_digest": "5" * 64,
        "tested_tree_oid": TREE,
        "conclusion": "success",
        "receipt_type": "oasis7_ci_ready_receipt",
        "issuer": "github_live_query",
        "live_validation": "ci-ready-receipt-live",
        "trusted_integration_artifact": True,
    }
    value.update(changes)
    return value


class ReviewIdentityV2Test(unittest.TestCase):
    def test_bootstrap_epoch_uses_positive_integer_snapshot_identity(self):
        identity = MODULE.source_review_identity(**source_fields())
        self.assertEqual(identity["bootstrap_epoch"], 1)
        for invalid in (True, 0, "1"):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                MODULE.source_review_identity(**dict(source_fields(), bootstrap_epoch=invalid))

    def test_source_identity_has_fixed_scope_and_digest(self):
        identity = MODULE.source_review_identity(**source_fields())
        self.assertEqual(identity["source_scope_oid"], SOURCE_SCOPE)
        self.assertNotIn("integration_base_oid", identity)
        digest = MODULE.source_review_digest(identity)
        self.assertEqual(digest, MODULE.source_review_digest(dict(identity)))
        identity["source_scope_oid"] = INTEGRATION_BASE
        self.assertNotEqual(digest, MODULE.source_review_digest(identity))

    def test_latest_integration_may_change_run_identity_when_tree_and_authority_match(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "integration_ci_identity": accepted,
            "integration_ci_digest": MODULE.integration_ci_digest(accepted),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
        }
        latest = integration_receipt(request_id=14, request_created_at="2026-09-11T01:00:00Z",
                                     run_id=15, run_attempt=1, check_run_id=16,
                                     integration_base_oid="8" * 40)
        self.assertTrue(MODULE.can_reuse_source_review(plan, latest))

    def test_changed_tree_or_ci_authority_requires_new_review(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "integration_ci_identity": accepted,
            "integration_ci_digest": MODULE.integration_ci_digest(accepted),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
        }
        self.assertFalse(MODULE.can_reuse_source_review(plan, integration_receipt(tested_tree_oid="6" * 40)))
        self.assertFalse(MODULE.can_reuse_source_review(plan, integration_receipt(workflow_sha="7" * 40)))
        self.assertFalse(MODULE.can_reuse_source_review(plan, integration_receipt(conclusion="failure")))
        plan["integration_ci_identity"]["conclusion"] = "failure"
        plan["integration_ci_digest"] = MODULE.integration_ci_digest(plan["integration_ci_identity"])
        self.assertFalse(MODULE.can_reuse_source_review(plan, integration_receipt()))

    def test_shape_valid_unattested_receipt_cannot_prove_tree_reuse(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "integration_ci_identity": accepted,
            "integration_ci_digest": MODULE.integration_ci_digest(accepted),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
        }
        forged = integration_receipt(live_validation=None, trusted_integration_artifact=False)
        self.assertFalse(MODULE.can_reuse_source_review(plan, forged))

    def test_v1_identity_remains_available(self):
        receipt = {
            "receipt_type": "oasis7_ci_ready_receipt", "issuer": "github_live_query",
            "repository": "eng-cc/oasis7", "task_uid": UID, "task_issue_number": 1,
            "pr_number": 7, "base_oid": INTEGRATION_BASE, "head_oid": HEAD,
            "check_name": "required-gate", "check_app_id": 42, "check_run_id": 13,
            "planner_digest": "5" * 64, "planner_config_sha256": "sha256:" + "6" * 64,
            "run_rust_baseline": True, "conclusion": "success",
        }
        self.assertEqual(len(MODULE.review_evidence_digest(receipt)), 64)


if __name__ == "__main__":
    unittest.main()
