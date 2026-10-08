#!/usr/bin/env python3
"""Regression coverage for trusted risk-triggered integration selection."""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
GATE_PATH = Path(__file__).with_name("pr-lifecycle-gate.py").resolve()
GATE_SPEC = importlib.util.spec_from_file_location("pr_lifecycle_gate_under_test", GATE_PATH)
assert GATE_SPEC is not None and GATE_SPEC.loader is not None
GATE = importlib.util.module_from_spec(GATE_SPEC)
GATE_SPEC.loader.exec_module(GATE)
PROJECTION_PATH = Path(__file__).with_name("workflow-impact-projection.py").resolve()
PROJECTION_SPEC = importlib.util.spec_from_file_location(
    "workflow_impact_projection_under_test", PROJECTION_PATH
)
assert PROJECTION_SPEC is not None and PROJECTION_SPEC.loader is not None
PROJECTION = importlib.util.module_from_spec(PROJECTION_SPEC)
PROJECTION_SPEC.loader.exec_module(PROJECTION)

TASK = "task_" + "1" * 32


class StrictIntegrationRoutingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Strict routing test")
        (self.root / "README").write_text("fixture\n", encoding="utf-8")
        (self.root / "contracts").mkdir()
        (self.root / "contracts" / "stable.md").write_text("stable contract\n", encoding="utf-8")
        self.git("add", "README", "contracts/stable.md")
        self.git("commit", "-m", "base")
        self.base = self.git("rev-parse", "HEAD")
        self.effective = ROOT
        self.policy_commit = subprocess.run(
            ["git", "-C", str(self.effective), "rev-parse", "HEAD"],
            check=True, text=True, stdout=subprocess.PIPE,
        ).stdout.strip()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", "-C", str(self.root), *args], check=True, text=True,
            stdout=subprocess.PIPE,
        ).stdout.strip()

    def source_projection(
        self, *, change_class: str = "mechanical-doc", stable_contract: bool = False,
        mapped_contract: bool = True, target_path: str = "target.txt",
    ) -> tuple[dict, str, str]:
        self.git("checkout", "-b", "source")
        target = self.root / "doc" / "change.md"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("change\n", encoding="utf-8")
        self.git("add", "doc/change.md")
        self.git("commit", "-m", "source")
        source = self.git("rev-parse", "HEAD")
        projection = PROJECTION.build_projection(self.root, {
            "task_uid": TASK,
            "source_head_oid": source,
            "scope_base_oid": self.base,
            "changed_paths": ["doc/change.md"],
            "change_class": change_class,
            "manual_roles": [],
            "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["required_gate_baseline"],
            "consumed_contracts": ([{
                "id": "stable-contract", "revision": "v1",
                **({"path": "contracts/stable.md"} if mapped_contract else {}),
            }]
                                    if stable_contract else []),
            "public_semantics": [],
            "affected_consumers": ([{"path": "contracts/stable.md"}] if stable_contract else []),
            "closure_status": {"status": "complete", "reason": "fixture", "evidence": [{
                "path": "README",
                "sha256": "sha256:" + hashlib.sha256(
                    (self.root / "README").read_bytes()
                ).hexdigest(),
            }]},
        })
        self.projection = projection
        self.git("checkout", "-b", "target", self.base)
        target_file = self.root / target_path
        target_file.parent.mkdir(parents=True, exist_ok=True)
        target_file.write_text("target change\n", encoding="utf-8")
        self.git("add", target_path)
        self.git("commit", "-m", "target")
        current_base = self.git("rev-parse", "HEAD")
        return projection, source, current_base

    def data_for(self, projection: dict, source: str, current_base: str) -> dict:
        encoded = base64.b64encode(json.dumps(projection).encode()).decode()
        return {
            "body": f"<!-- oasis7-impact-projection-b64: {encoded} -->",
            "headRefOid": source,
            "baseRefOid": current_base,
        }

    def route(self, projection: dict, source: str, current_base: str) -> dict:
        return GATE.trusted_strict_integration_decision(
            self.data_for(projection, source, current_base),
            self.root, self.effective, TASK, self.policy_commit,
        )

    def test_projection_risk_metadata_and_target_drift_stay_ordinary(self) -> None:
        projection, source, current_base = self.source_projection(
            change_class="workflow-doc", stable_contract=True,
            target_path="contracts/stable.md",
        )
        self.assertEqual(self.route(projection, source, current_base)["status"], "not_required")

    def test_review_markers_paths_and_keywords_do_not_create_strict_route(self) -> None:
        projection, source, current_base = self.source_projection()
        for fields in (
            {"public_semantics": ["wire shape changed"]},
            {"review_escalated": True},
            {"verification_affected": True},
            {"change_class": "mixed"},
            {"changed_paths": ["scripts/pm/pr-lifecycle-gate.py"]},
            {"review_reasons": ["security contract workflow"]},
        ):
            with self.subTest(fields=fields):
                candidate = {**projection, **fields}
                candidate.pop("projection_digest", None)
                candidate["projection_digest"] = PROJECTION.canonical_digest(candidate)
                # Projection validation is still an ordinary foundation check;
                # only the digest-bound changed-path list must match the source.
                if "changed_paths" in fields:
                    candidate["changed_paths"] = projection["changed_paths"]
                    candidate.pop("projection_digest", None)
                    candidate["projection_digest"] = PROJECTION.canonical_digest(candidate)
                self.assertEqual(self.route(candidate, source, current_base)["status"], "not_required")

    def historical_target_admission(self, target_path):
        projection, source, target = self.source_projection(stable_contract=True, target_path=target_path)
        data = {**self.data_for(projection, source, self.base), 'repository':'owner/repo',
                'number':12, 'state':'OPEN', 'isDraft':False, 'baseRefName':'main',
                'headRefName':'codex/source'}
        observed = []
        def admission(actual, *args, **kwargs):
            # Exercise classification with the target actually propagated by the
            # production entrypoint; absent propagation retains historical B.
            assessed = kwargs.get('assessed_target_oid')
            classifier_args = {'assessed_target_oid':assessed} if assessed is not None else {}
            route = GATE.trusted_strict_integration_decision(
                actual, self.root, self.effective, TASK, self.policy_commit, **classifier_args)
            observed.append(route['status'])
            if route['status'] == 'blocked':raise ValueError('strict integration route blocked')
            return None
        with patch.object(GATE, 'decision', return_value={'ready_for_merge':True,'blockers':[]}), \
             patch.object(GATE, 'live_target_oid', return_value=target), \
             patch.object(GATE, 'local_loop_admission', return_value={'status':'passed'}), \
             patch.object(GATE, 'live_integration_admission', side_effect=admission), \
             patch.object(GATE, 'read_pr_identity', return_value=data):
            result = GATE.production_decision(data, False, self.root, TASK, None)
        self.assertEqual(data['baseRefOid'], self.base)
        return observed, result

    def test_related_main_advance_does_not_create_strict_request(self):
        observed, result = self.historical_target_admission('contracts/stable.md')
        self.assertEqual(observed, ['not_required'])
        self.assertTrue(result['ready_for_merge'])

    def test_unrelated_main_advance_keeps_ordinary(self):
        observed, result = self.historical_target_admission('unrelated.md')
        self.assertEqual(observed, ['not_required'])
        self.assertTrue(result['ready_for_merge'])

    def test_missing_or_malformed_projection_fails_closed(self) -> None:
        for body in ("", "<!-- oasis7-impact-projection-b64: !!! -->"):
            with self.subTest(body=body):
                with self.assertRaisesRegex(ValueError, "projection"):
                    GATE.trusted_requires_strict_integration(
                        {"body": body, "headRefOid": self.base, "baseRefOid": self.base},
                        self.root, self.effective, TASK, self.policy_commit,
                    )

    def test_missing_context_is_blocked_instead_of_implicitly_strict(self) -> None:
        identity = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_routing_test", Path(__file__).with_name("ci_ready_receipt_identity.py"))
        assert identity and identity.loader
        module = importlib.util.module_from_spec(identity)
        identity.loader.exec_module(module)
        self.assertEqual(module.evaluate_strict_integration_requirement()["status"], "blocked")
        with self.assertRaisesRegex(ValueError, "trusted projection context"):
            GATE.requires_strict_integration({"high_risk": False, "risk_class": "ordinary"})

    def test_caller_exception_claim_is_blocked_without_protected_producer(self) -> None:
        projection, _, _ = self.source_projection()
        identity = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_claim_test", Path(__file__).with_name("ci_ready_receipt_identity.py"))
        assert identity and identity.loader
        module = importlib.util.module_from_spec(identity)
        identity.loader.exec_module(module)
        decision = module.evaluate_strict_integration_requirement(
            trusted_projection=projection,
            expected_task_uid=TASK,
            exception_rule="trusted_executor_isolation",
            ordinary_limitation="claimed by caller",
            constraint_evidence={"self_reported": True},
            required_check_scope={"check": "required-gate"},
            strict_capability={"claim": "strict supports it"},
        )
        self.assertEqual(decision["status"], "blocked")
        self.assertEqual(decision["reason"], "no_protected_strict_exception_fact_producer")

    def test_projection_schema_and_task_binding_are_part_of_ordinary_foundation(self) -> None:
        projection, _, _ = self.source_projection()
        spec = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_projection_binding_test",
            Path(__file__).with_name("ci_ready_receipt_identity.py"),
        )
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        for field, value in (("schema", "wrong/v1"), ("task_uid", "task_" + "2" * 32)):
            candidate = {**projection, field: value}
            candidate.pop("projection_digest", None)
            candidate["projection_digest"] = PROJECTION.canonical_digest(candidate)
            with self.subTest(field=field):
                decision = module.evaluate_strict_integration_requirement(
                    trusted_projection=candidate, expected_task_uid=TASK,
                )
                self.assertEqual(decision["status"], "blocked")
        self.assertEqual(
            module.evaluate_strict_integration_requirement(
                trusted_projection=projection, expected_task_uid="task_" + "2" * 32,
            )["status"],
            "blocked",
        )

    def test_related_target_advance_refreshes_ordinary_ci_without_strict_dispatch(self) -> None:
        projection, source, current_base = self.source_projection(
            stable_contract=True, target_path="contracts/stable.md",
        )
        identity_spec = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_target_freshness_test",
            Path(__file__).with_name("ci_ready_receipt_identity.py"),
        )
        assert identity_spec and identity_spec.loader
        identity = importlib.util.module_from_spec(identity_spec)
        identity_spec.loader.exec_module(identity)
        self.assertEqual(self.route(projection, source, current_base)["status"], "not_required")
        self.assertTrue(identity.ordinary_target_advance_requires_refresh(
            projection, root=self.root, source_scope_oid=self.base,
            source_head_oid=source, current_target_oid=current_base,
        ))

    def test_unmapped_relation_does_not_add_target_proof_requirement(self) -> None:
        projection, source, current_base = self.source_projection(
            stable_contract=True, mapped_contract=False, target_path="unrelated.md",
        )
        identity_spec = importlib.util.spec_from_file_location(
            "ci_ready_receipt_identity_unmapped_target_test",
            Path(__file__).with_name("ci_ready_receipt_identity.py"),
        )
        assert identity_spec and identity_spec.loader
        identity = importlib.util.module_from_spec(identity_spec)
        identity_spec.loader.exec_module(identity)
        self.assertEqual(self.route(projection, source, current_base)["status"], "not_required")
        self.assertFalse(identity.ordinary_target_advance_requires_refresh(
            projection, root=self.root, source_scope_oid=self.base,
            source_head_oid=source, current_target_oid=current_base,
        ))

    def test_strict_gate_rejects_an_ordinary_fallback_without_dispatch(self) -> None:
        with self.assertRaisesRegex(ValueError, "strict integration"):
            GATE._validate_live_integration_proof(
                {
                    "ci_validation_mode": "ordinary_pr",
                    "integration_base_oid": "c" * 40,
                    "base_ref": "main",
                    "head_oid": "a" * 40,
                },
                {"baseRefOid": "b" * 40, "baseRefName": "main", "headRefOid": "a" * 40},
                strict=True,
            )


if __name__ == "__main__":
    unittest.main()
