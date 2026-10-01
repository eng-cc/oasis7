#!/usr/bin/env python3
import importlib.util
import hashlib
import json
import subprocess
import tempfile
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


def ordinary_receipt(**changes):
    value = {
        "receipt_type": "oasis7_ci_ready_receipt",
        "issuer": "github_live_query",
        "repository": "eng-cc/oasis7",
        "task_uid": UID,
        "task_issue_number": 1,
        "pr_number": 7,
        "base_oid": SOURCE_SCOPE,
        "head_oid": HEAD,
        "base_ref": "main",
        "check_name": "required-gate",
        "check_app_id": 42,
        "check_run_id": 13,
        "planner_digest": "5" * 64,
        "planner_config_sha256": "sha256:" + "6" * 64,
        "run_rust_baseline": True,
        "conclusion": "success",
        "ci_validation_mode": "ordinary_pr",
        "live_validation": "ci-ready-receipt-live",
        "impact_projection_schema": PROJECTION_SCHEMA,
        "impact_projection_digest": PROJECTION_DIGEST,
        "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
    }
    value.update(changes)
    return value


def ordinary_plan(source, *, change_class="mechanical-doc", include_projection=True):
    paths = ["doc/change.md"]
    changed_paths_digest = "sha256:" + hashlib.sha256(
        json.dumps(paths, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    source["changed_paths_digest"] = changed_paths_digest.removeprefix("sha256:")
    plan = {
        "schema": "oasis7-review-plan/v2",
        "source_review_identity": source,
        "source_review_digest": MODULE.source_review_digest(source),
        "impact_projection_schema": PROJECTION_SCHEMA,
        "impact_projection_digest": PROJECTION_DIGEST,
        "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
        "professional_review_applicability": verified_applicability(source),
        "effective_mode": {
            "effective_policy": "loop-bound",
            "review_schema": "oasis7-review-plan/v2",
            "source_review_mode": "separated",
            "integration_validation_mode": "ordinary_pr_ci",
            "enabled_optimizations": ["source_review_integration_separation"],
            "fallback_reason": None,
        },
    }
    if include_projection:
        projection = {
            "schema": PROJECTION_SCHEMA,
            "task_uid": source["task_uid"],
            "source_head_oid": source["source_head_oid"],
            "scope_base_oid": source["source_scope_oid"],
            "changed_paths": paths,
            "changed_paths_digest": changed_paths_digest,
            "change_class": change_class,
            "review_escalated": False,
            "verification_affected": False,
            "closure_status": {"status": "complete"},
            "consumed_contracts": [],
            "public_semantics": [],
            "review_reasons": [],
        }
        projection["projection_digest"] = "sha256:" + hashlib.sha256(
            json.dumps(projection, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        plan["impact_projection"] = projection
        plan["impact_projection_digest"] = projection["projection_digest"]
    return plan


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
    def _target_relation_case(self, target_path):
        temp = tempfile.TemporaryDirectory()
        root = Path(temp.name)

        def git(*args):
            return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

        subprocess.run(["git", "init", str(root)], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.name", "Review Test"], check=True)
        (root / "README").write_text("stable\n", encoding="utf-8")
        (root / "contracts").mkdir()
        (root / "contracts" / "stable.md").write_text("v1\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", "README", "contracts/stable.md"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-m", "base"], check=True, capture_output=True)
        base = git("rev-parse", "HEAD")
        subprocess.run(["git", "-C", str(root), "checkout", "-b", "source"], check=True, capture_output=True)
        (root / "doc").mkdir()
        (root / "doc" / "change.md").write_text("source\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", "doc/change.md"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-m", "source"], check=True, capture_output=True)
        source_head = git("rev-parse", "HEAD")
        subprocess.run(["git", "-C", str(root), "checkout", "-b", "target", base], check=True, capture_output=True)
        target_file = root / target_path
        target_file.parent.mkdir(parents=True, exist_ok=True)
        target_file.write_text("target\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", target_path], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-m", "target"], check=True, capture_output=True)
        target_head = git("rev-parse", "HEAD")

        changed_paths = ["doc/change.md"]
        changed_paths_digest = "sha256:" + hashlib.sha256(
            json.dumps(changed_paths, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        source = MODULE.source_review_identity(
            task_uid=UID, bootstrap_epoch=1, repository="eng-cc/oasis7", pr_number=7,
            source_head_oid=source_head, source_scope_oid=base,
            changed_paths_digest=changed_paths_digest.removeprefix("sha256:"),
            ordered_role_ids=["qa_engineer", "repository_health_engineer"],
            role_contract_digest="1" * 64, review_policy_digest="2" * 64,
            input_contract_digest="3" * 64,
        )
        projection = {
            "schema": PROJECTION_SCHEMA, "task_uid": UID,
            "source_head_oid": source_head, "scope_base_oid": base,
            "changed_paths": changed_paths, "changed_paths_digest": changed_paths_digest,
            "change_class": "mechanical-doc", "review_escalated": False,
            "verification_affected": False,
            "closure_status": {"status": "complete", "evidence": [{"path": "README"}]},
            "consumed_contracts": [{"id": "stable-contract", "path": "contracts/stable.md"}],
            "affected_consumers": [{"path": "contracts/stable.md"}],
            "public_semantics": [], "review_reasons": [],
        }
        projection["projection_digest"] = "sha256:" + hashlib.sha256(
            json.dumps(projection, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        plan = {
            "schema": "oasis7-review-plan/v2", "source_review_identity": source,
            "source_review_digest": MODULE.source_review_digest(source),
            "impact_projection_schema": PROJECTION_SCHEMA,
            "impact_projection": projection,
            "impact_projection_digest": projection["projection_digest"],
            "impact_projection_planner_digest": PROJECTION_PLANNER_DIGEST,
            "professional_review_applicability": verified_applicability(source),
            "effective_mode": {"effective_policy": "loop-bound"},
        }
        receipt = ordinary_receipt(
            base_oid=base, head_oid=source_head,
            impact_projection_digest=projection["projection_digest"],
            impact_projection_planner_digest=PROJECTION_PLANNER_DIGEST,
        )
        return temp, root, plan, receipt, target_head

    def test_ordinary_receipt_reuses_only_low_risk_bound_projection(self):
        source = MODULE.source_review_identity(**source_fields())
        low_risk = ordinary_plan(source)
        low_receipt = ordinary_receipt(
            impact_projection_digest=low_risk["impact_projection_digest"],
            impact_projection_planner_digest=low_risk["impact_projection_planner_digest"],
        )
        self.assertTrue(MODULE.can_reuse_source_review(
            low_risk, low_receipt
        ))
        stable = ordinary_plan(source)
        stable_projection = stable["impact_projection"]
        stable_projection["consumed_contracts"] = [{"id": "stable-contract", "revision": "v1"}]
        stable_projection["affected_consumers"] = ["stable-consumer"]
        stable_projection.pop("projection_digest", None)
        stable_projection["projection_digest"] = "sha256:" + hashlib.sha256(
            json.dumps(stable_projection, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        stable["impact_projection_digest"] = stable_projection["projection_digest"]
        stable_receipt = ordinary_receipt(
            impact_projection_digest=stable["impact_projection_digest"],
            impact_projection_planner_digest=stable["impact_projection_planner_digest"],
        )
        self.assertTrue(MODULE.can_reuse_source_review(stable, stable_receipt))
        high_risk = ordinary_plan(source, change_class="workflow-doc")
        high_receipt = ordinary_receipt(
            impact_projection_digest=high_risk["impact_projection_digest"],
            impact_projection_planner_digest=high_risk["impact_projection_planner_digest"],
        )
        self.assertFalse(MODULE.can_reuse_source_review(
            high_risk, high_receipt
        ))
        semantic = ordinary_plan(source)
        semantic_projection = semantic["impact_projection"]
        semantic_projection["public_semantics"] = ["wire shape changed"]
        semantic_projection.pop("projection_digest", None)
        semantic_projection["projection_digest"] = "sha256:" + hashlib.sha256(
            json.dumps(semantic_projection, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        semantic["impact_projection_digest"] = semantic_projection["projection_digest"]
        semantic_receipt = ordinary_receipt(
            impact_projection_digest=semantic["impact_projection_digest"],
            impact_projection_planner_digest=semantic["impact_projection_planner_digest"],
        )
        self.assertFalse(MODULE.can_reuse_source_review(semantic, semantic_receipt))
        legacy = ordinary_plan(source)
        legacy["effective_mode"] = {**legacy["effective_mode"], "effective_policy": "legacy"}
        self.assertFalse(MODULE.can_reuse_source_review(legacy, low_receipt))
        tampered = dict(low_risk)
        tampered["impact_projection"] = {
            **low_risk["impact_projection"], "review_escalated": True,
        }
        self.assertFalse(MODULE.can_reuse_source_review(tampered, low_receipt))
        missing = ordinary_plan(source, include_projection=False)
        self.assertFalse(MODULE.can_reuse_source_review(
            missing, ordinary_receipt(
                impact_projection_digest=missing["impact_projection_digest"],
                impact_projection_planner_digest=missing["impact_projection_planner_digest"],
            )
        ))

    def test_ordinary_reuse_rejects_related_target_advance(self):
        temp, root, plan, receipt, target = self._target_relation_case("contracts/stable.md")
        self.addCleanup(temp.cleanup)
        self.assertFalse(MODULE.can_reuse_source_review(
            plan, receipt, current_target_oid=target, current_target_root=root,
        ))

    def test_ordinary_reuse_allows_unrelated_target_advance(self):
        temp, root, plan, receipt, target = self._target_relation_case("unrelated.md")
        self.addCleanup(temp.cleanup)
        self.assertTrue(MODULE.can_reuse_source_review(
            plan, receipt, current_target_oid=target, current_target_root=root,
        ))

    def test_ordinary_reuse_fails_closed_for_unmapped_contract_on_target_advance(self):
        temp, root, plan, receipt, target = self._target_relation_case("unrelated.md")
        self.addCleanup(temp.cleanup)
        projection = plan["impact_projection"]
        projection["consumed_contracts"] = [{"id": "stable-contract", "revision": "v1"}]
        projection["projection_digest"] = "sha256:" + hashlib.sha256(
            json.dumps(projection, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        plan["impact_projection_digest"] = projection["projection_digest"]
        receipt["impact_projection_digest"] = projection["projection_digest"]
        self.assertFalse(MODULE.can_reuse_source_review(
            plan, receipt, current_target_oid=target, current_target_root=root,
        ))

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

    def test_integration_identity_rejects_conflicting_legacy_aliases(self):
        for receipt in (
            integration_receipt(run_id=12, integration_run_id=99),
            integration_receipt(source_head_oid=HEAD, head_oid="e" * 40),
            integration_receipt(integration_base_oid=INTEGRATION_BASE, base_oid="e" * 40),
        ):
            with self.subTest(receipt=receipt), self.assertRaisesRegex(ValueError, "conflicting aliases"):
                MODULE.integration_ci_identity(receipt)

    def test_integration_identity_requires_positive_numeric_provenance_ids(self):
        for field, value in (
            ("request_id", "abc"), ("run_id", 0), ("check_app_id", -1),
            ("check_run_id", True),
        ):
            with self.subTest(field=field, value=value), self.assertRaisesRegex(ValueError, field):
                MODULE.integration_ci_identity(integration_receipt(**{field: value}))

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
