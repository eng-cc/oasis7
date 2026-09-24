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


class TrustedRiskClassifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Risk classifier test")
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

    def test_ordinary_projection_allows_unrelated_target_advance(self) -> None:
        projection, source, current_base = self.source_projection()
        self.assertFalse(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_workflow_projection_escalates_to_strict_integration(self) -> None:
        projection, source, current_base = self.source_projection(change_class="workflow-doc")
        self.assertTrue(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_stable_consumed_contract_with_unrelated_target_advance_stays_ordinary(self) -> None:
        projection, source, current_base = self.source_projection(stable_contract=True)
        self.assertFalse(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_target_change_at_consumed_contract_path_escalates(self) -> None:
        projection, source, current_base = self.source_projection(
            stable_contract=True, target_path="contracts/stable.md",
        )
        self.assertTrue(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_unmapped_consumed_contract_with_target_advance_fails_closed(self) -> None:
        projection, source, current_base = self.source_projection(
            stable_contract=True, mapped_contract=False,
        )
        self.assertTrue(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_unmapped_consumer_identifier_with_target_advance_fails_closed(self) -> None:
        projection, source, current_base = self.source_projection(stable_contract=True)
        projection = json.loads(json.dumps(projection))
        projection["affected_consumers"] = ["stable-consumer-id"]
        projection.pop("projection_digest", None)
        projection["projection_digest"] = PROJECTION.canonical_digest(projection)
        self.assertTrue(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_explicit_public_semantics_escalates_without_keyword_heuristic(self) -> None:
        projection, source, current_base = self.source_projection()
        projection = json.loads(json.dumps(projection))
        projection["public_semantics"] = ["wire shape changed"]
        projection.pop("projection_digest", None)
        projection["projection_digest"] = PROJECTION.canonical_digest(projection)
        self.assertTrue(
            GATE.trusted_requires_strict_integration(
                self.data_for(projection, source, current_base),
                self.root, self.effective, TASK, self.policy_commit,
            )
        )

    def test_missing_or_malformed_projection_fails_closed(self) -> None:
        for body in ("", "<!-- oasis7-impact-projection-b64: !!! -->"):
            with self.subTest(body=body):
                with self.assertRaisesRegex(ValueError, "projection"):
                    GATE.trusted_requires_strict_integration(
                        {"body": body, "headRefOid": self.base, "baseRefOid": self.base},
                        self.root, self.effective, TASK, self.policy_commit,
                    )

    def test_untrusted_ad_hoc_risk_keys_cannot_select_ordinary_path(self) -> None:
        self.assertTrue(GATE.requires_strict_integration({"high_risk": False, "risk_class": "ordinary"}))

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
