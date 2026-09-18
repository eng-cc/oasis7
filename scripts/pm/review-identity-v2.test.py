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
PROJECTION_SCHEMA = "oasis7-workflow-impact-projection/v2"
PROJECTION_DIGEST = "sha256:" + "9" * 64
PROJECTION_PLANNER_DIGEST = "sha256:" + "8" * 64


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
        "impact_projection_schema": PROJECTION_SCHEMA,
        "impact_projection_digest": PROJECTION_DIGEST,
        "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
        "conclusion": "success",
        "receipt_type": "oasis7_ci_ready_receipt",
        "issuer": "github_live_query",
        "live_validation": "ci-ready-receipt-live",
        "trusted_integration_artifact": True,
    }
    value.update(changes)
    return value


def applicability_identity(source):
    return MODULE.review_applicability_identity(source)


def applicability_digest(identity):
    return MODULE.review_applicability_digest(identity)


def verified_applicability(source, *, verified=True):
    identity = applicability_identity(source)
    return {
        "identity": identity,
        "identity_digest": applicability_digest(identity),
        "verified": verified,
    }


def v2_plan(source, accepted, applicability=None):
    applicability = applicability or verified_applicability(source)
    return {
        "schema": "oasis7-review-plan/v2",
        "source_review_identity": source,
        "source_review_digest": MODULE.source_review_digest(source),
        "impact_projection_schema": PROJECTION_SCHEMA,
        "impact_projection_digest": PROJECTION_DIGEST,
        "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
        "integration_ci_identity": accepted,
        "integration_ci_digest": MODULE.integration_ci_digest(accepted),
        "integration_ci_provenance": {
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
        },
        "professional_review_applicability": {
            **applicability,
        },
    }


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
            "impact_projection_schema": PROJECTION_SCHEMA,
            "impact_projection_digest": PROJECTION_DIGEST,
            "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
            "integration_ci_identity": accepted,
            "integration_ci_digest": MODULE.integration_ci_digest(accepted),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
            "professional_review_applicability": verified_applicability(source),
        }
        latest = integration_receipt(request_id=14, request_created_at="2026-09-11T01:00:00Z",
                                     run_id=15, run_attempt=1, check_run_id=16,
                                     integration_base_oid="8" * 40)
        self.assertTrue(MODULE.can_reuse_source_review(plan, latest))

    def test_authoritative_reuse_allows_target_workflow_and_tree_change_when_applicability_is_unchanged(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        latest = integration_receipt(
            request_id=14, request_created_at="2026-09-11T01:00:00Z",
            run_id=15, run_attempt=1, check_run_id=16,
            integration_base_oid="8" * 40, workflow_sha="7" * 40,
            tested_tree_oid="6" * 40, planner_digest="6" * 64,
        )
        self.assertTrue(MODULE.can_reuse_source_review(plan, latest))

    def test_authoritative_reuse_rejects_ordinary_rerun(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        self.assertFalse(MODULE.can_reuse_source_review(
            plan, integration_receipt(run_attempt=2, check_run_id=99)
        ))

    def test_source_only_plan_accepts_first_trusted_integration_join(self):
        source = MODULE.source_review_identity(**source_fields())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "impact_projection_schema": PROJECTION_SCHEMA,
            "impact_projection_digest": PROJECTION_DIGEST,
            "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
            "professional_review_applicability": verified_applicability(source),
        }
        self.assertTrue(MODULE.can_reuse_source_review(plan, integration_receipt()))

    def test_source_only_join_requires_receipt_projection_binding(self):
        source = MODULE.source_review_identity(**source_fields())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "impact_projection_schema": PROJECTION_SCHEMA,
            "impact_projection_digest": PROJECTION_DIGEST,
            "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
            "professional_review_applicability": verified_applicability(source),
        }
        for field in (
            "impact_projection_schema", "impact_projection_digest",
            "impact_projection_planner_digest",
        ):
            with self.subTest(missing_receipt_field=field):
                receipt = integration_receipt()
                del receipt[field]
                self.assertFalse(MODULE.can_reuse_source_review(plan, receipt))
        for field in (
            "impact_projection_schema", "impact_projection_digest",
            "impact_projection_planner_digest",
        ):
            with self.subTest(mismatched_receipt_field=field):
                receipt = integration_receipt()
                receipt[field] = (
                    "oasis7-workflow-impact-projection/other"
                    if field == "impact_projection_schema"
                    else "sha256:" + "a" * 64
                )
                self.assertFalse(MODULE.can_reuse_source_review(plan, receipt))
        for field in (
            "impact_projection_schema", "impact_projection_digest",
            "impact_projection_planner_digest",
        ):
            with self.subTest(missing_plan_field=field):
                incomplete_plan = dict(plan)
                del incomplete_plan[field]
                self.assertFalse(MODULE.can_reuse_source_review(
                    incomplete_plan, integration_receipt()
                ))

    def test_authoritative_reuse_fails_closed_without_verified_applicability(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        del plan["professional_review_applicability"]
        latest = integration_receipt(
            request_id=14, request_created_at="2026-09-11T01:00:00Z",
            run_id=15, run_attempt=1, check_run_id=16,
        )
        self.assertFalse(MODULE.can_reuse_source_review(plan, latest))

    def test_execution_identity_drift_does_not_invalidate_source_review(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = {
            "schema": "oasis7-review-plan/v2",
            "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "impact_projection_schema": PROJECTION_SCHEMA,
            "impact_projection_digest": PROJECTION_DIGEST,
            "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
            "integration_ci_identity": accepted,
            "integration_ci_digest": MODULE.integration_ci_digest(accepted),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
            "professional_review_applicability": verified_applicability(source),
        }
        self.assertTrue(MODULE.can_reuse_source_review(
            plan, integration_receipt(
                request_id=14, request_created_at="2026-09-11T01:00:00Z",
                run_id=15, check_run_id=16, integration_base_oid="8" * 40,
                workflow_sha="7" * 40, tested_tree_oid="6" * 40,
                planner_digest="6" * 64,
            )
        ))
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
            "professional_review_applicability": verified_applicability(source),
        }
        forged = integration_receipt(live_validation=None, trusted_integration_artifact=False)
        self.assertFalse(MODULE.can_reuse_source_review(plan, forged))

    def test_shadow_reports_complete_provenance_and_reusable_review_for_target_only_advance(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        latest = integration_receipt(
            request_id=14, request_created_at="2026-09-11T01:00:00Z",
            run_id=15, run_attempt=1, check_run_id=16,
            integration_base_oid="8" * 40,
        )

        decision = MODULE.shadow_source_review_applicability(
            plan, latest, verified_applicability(source)
        )

        self.assertFalse(decision["authoritative"])
        self.assertEqual(decision["integration_provenance"], "complete")
        self.assertEqual(decision["professional_review_applicability"], "unchanged")
        self.assertEqual(decision["decision"], "reusable_source_review")
        self.assertEqual(decision["audit_identity"]["workflow_sha"], "4" * 40)
        self.assertEqual(decision["audit_identity"]["tested_tree_oid"], TREE)

    def test_shadow_keeps_execution_drift_in_audit_without_invalidating_applicability(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        for changes in (
            {"workflow_sha": "7" * 40},
            {"tested_tree_oid": "6" * 40},
        ):
            with self.subTest(changes=changes):
                decision = MODULE.shadow_source_review_applicability(
                    plan,
                    integration_receipt(
                        request_id=14, run_id=15, check_run_id=16,
                        integration_base_oid="8" * 40, **changes
                    ),
                    verified_applicability(source),
                )
                self.assertEqual(decision["integration_provenance"], "complete")
                self.assertEqual(decision["professional_review_applicability"], "unchanged")
                self.assertEqual(decision["decision"], "reusable_source_review")
                self.assertEqual(decision["reason"], "target_base_only_advance")
                for field, value in changes.items():
                    self.assertEqual(decision["audit_identity"][field], value)

    def test_shadow_requires_full_review_when_verified_applicability_changes(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        for field in ("review_policy_digest", "input_contract_digest"):
            with self.subTest(field=field):
                changed = verified_applicability(dict(source, **{field: "9" * 64}))
                decision = MODULE.shadow_source_review_applicability(
                    plan,
                    integration_receipt(
                        request_id=14, run_id=15, check_run_id=16,
                        integration_base_oid="8" * 40,
                    ),
                    changed,
                )

                self.assertEqual(decision["integration_provenance"], "complete")
                self.assertEqual(decision["professional_review_applicability"], "requires_full_review")
                self.assertEqual(decision["decision"], "requires_full_review")
                self.assertEqual(decision["reason"], "applicability_identity_changed")

    def test_shadow_is_fail_closed_for_unverified_or_unknown_applicability_and_ci(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        latest = integration_receipt(
            request_id=14, run_id=15, check_run_id=16,
            integration_base_oid="8" * 40,
        )
        cases = (
            (None, latest, "applicability_identity_unknown"),
            (verified_applicability(source, verified=False), latest, "applicability_identity_unverified"),
            (verified_applicability(source), integration_receipt(live_validation=None), "integration_provenance_untrusted"),
            (verified_applicability(source), {**latest, "tested_tree_oid": None}, "integration_provenance_incomplete"),
        )
        for current, receipt, reason in cases:
            with self.subTest(reason=reason):
                decision = MODULE.shadow_source_review_applicability(plan, receipt, current)
                self.assertEqual(decision["decision"], "requires_full_review")
                self.assertEqual(decision["professional_review_applicability"], "requires_full_review")
                self.assertEqual(decision["reason"], reason)

    def test_shadow_does_not_change_authoritative_reuse_predicate(self):
        source = MODULE.source_review_identity(**source_fields())
        accepted = MODULE.integration_ci_identity(integration_receipt())
        plan = v2_plan(source, accepted)
        latest = integration_receipt(
            request_id=14, run_id=15, check_run_id=16,
            integration_base_oid="8" * 40,
        )
        self.assertTrue(MODULE.can_reuse_source_review(plan, latest))

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
