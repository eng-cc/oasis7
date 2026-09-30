#!/usr/bin/env python3
"""Focused cross-consumer and exact-M QA proof for pre-activation CI reuse."""
from __future__ import annotations

import contextlib
import importlib.util
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
GIT = "git"


def load_from_root(root: Path, name: str):
    path = root / "scripts" / "pm" / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load trusted helper: {name}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path.pop(0)
    return module


def load_fixture(name: str, filename: str):
    path = ROOT / "scripts" / "pm" / filename
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load test fixture: {filename}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path.pop(0)
    return module


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(
        [GIT, "-C", str(root), *args], stderr=subprocess.PIPE, text=True,
    ).strip()


class ReuseAcceptanceQATests(unittest.TestCase):
    def test_disabled_policy_rejects_candidate_enabled_capability(self):
        policy = load_from_root(ROOT, "ci_reuse_policy")
        self.assertEqual([], policy.default_effective_policy()["enabled_capabilities"])
        candidate = policy.default_effective_policy()
        candidate["enabled_capabilities"] = ["input-scope-reuse/v1"]
        with self.assertRaisesRegex(ValueError, "W-owned disabled default"):
            policy.effective_policy_identity(candidate)

    def test_validation_only_dispatch_is_not_selected_as_integration_evidence(self):
        integration = load_from_root(ROOT, "integration_ci")
        uid = "task_" + "a" * 32
        base, head = "1" * 40, "2" * 40
        run = {
            "id": 701,
            "event": "workflow_dispatch",
            "path": integration.WORKFLOW,
            "repository": {"full_name": "eng-cc/oasis7"},
            "head_sha": base,
            "head_branch": "main",
            "display_title": (
                f"oasis7-ci|workflow_dispatch|v1_reuse_validation_only|{uid}|143|"
                f"{base}|{head}|{'a' * 64}"
            ),
        }
        with patch.object(integration, "gh", return_value={"workflow_runs": [run]}):
            selected = integration.current_request(
                "eng-cc/oasis7", uid, 143, base, head, "main",
            )
        self.assertIsNone(selected)

    def test_reusable_fixture_revocation_and_legacy_manifest_stay_conservative(self):
        lifecycle = load_fixture(
            "ci_reuse_acceptance_lifecycle_fixtures", "pr-lifecycle-loop.test.py",
        )
        gate = lifecycle.gate
        fixtures = lifecycle._APPLICABILITY_FIXTURES
        proof, inventory, applicability, live_pr = (
            lifecycle.KeyedQApplicabilityTests().evaluator_inputs()
        )
        accepted = gate.evaluate_keyed_q_applicability(
            proof, inventory, applicability, live_pr,
        )
        self.assertEqual("reusable", accepted["test_evidence"])
        self.assertEqual([], accepted["decision"]["required_test_units"])

        # Revoke the trusted policy at the fresh target, keeping its observation
        # internally consistent. A stale source B must not fall back to the
        # previously reusable result.
        policy_module = load_from_root(ROOT, "ci_reuse_policy")
        revoked_policy = policy_module.default_effective_policy()
        target_context = {
            "repository": inventory["repository"],
            "task_uid": inventory["task_uid"],
            "pr_number": inventory["pr_number"],
            "source_head_oid": inventory["source_head_oid"],
            "source_scope_oid": inventory["source_scope_oid"],
            "target_oid": inventory["assessed_target_oid"],
            "input_scope_commit_oid": inventory["input_scope_commit_oid"],
            "input_scope_tree_oid": inventory["input_scope_tree_oid"],
            "unit_specs": inventory["unit_specs"],
            "product_corpus": inventory["product_corpus"],
        }
        observation = fixtures.trusted_target_observation(target_context, revoked_policy)
        inventory["effective_policy"] = revoked_policy
        inventory["effective_policy_identity"] = observation["effective_policy_identity"]
        inventory["target_observation"] = observation
        inventory["input_scope"]["target_observation"] = observation
        revoked_decision = applicability.evaluate_evidence_applicability(
            fixtures.source_plan(), {"reviews": [], "tests": []},
            fixtures.target_snapshot(), revoked_policy,
        )
        self.assertEqual("disabled", revoked_decision.test_evidence)
        self.assertEqual((), revoked_decision.reused_units)
        with self.assertRaisesRegex(ValueError, "keyed target Q C0 decision"):
            gate.evaluate_keyed_q_applicability(
                proof, inventory, applicability, live_pr,
            )

        # A legacy plan with no V2 capability manifest remains on the legacy
        # reader path; C0 does not infer reusable evidence from it.
        legacy_plan = {"schema": "oasis7-required-plan-v1"}
        self.assertIsNone(applicability.read_required_plan_capabilities(legacy_plan))
        legacy_decision = applicability.evaluate_evidence_applicability(
            legacy_plan, {"reviews": [], "tests": []}, fixtures.target_snapshot(),
            revoked_policy,
        )
        self.assertEqual("disabled", legacy_decision.test_evidence)

    def test_exact_m_runs_selected_local_required_executor(self):
        """Exercise W's real required runner against the exact local M worktree."""
        head = git(ROOT, "rev-parse", "HEAD")
        base = os.environ.get("OASIS7_CI_REUSE_LOCAL_BASE_OID") or git(
            ROOT, "rev-parse", f"{head}^",
        )
        if not re.fullmatch(r"[0-9a-f]{40,64}", base):
            self.fail("exact-M fixture base must be a full lowercase commit OID")
        self.assertEqual(base, git(ROOT, "rev-parse", "--verify", "--end-of-options", f"{base}^{{commit}}"))
        subprocess.run(
            [GIT, "-C", str(ROOT), "merge-base", "--is-ancestor", base, head],
            check=True, capture_output=True, text=True,
        )
        with tempfile.TemporaryDirectory(prefix="oasis7-ci-reuse-exact-m-") as temporary:
            temp = Path(temporary)
            trusted_w = temp / "W"
            target_m = temp / "M"
            log_path = temp / "required-tier.log"
            subprocess.run(
                [GIT, "-C", str(ROOT), "worktree", "add", "--quiet", "--detach",
                 str(trusted_w), base], check=True, capture_output=True, text=True,
            )
            try:
                self.assertEqual(base, git(trusted_w, "rev-parse", "HEAD"))
                integration = load_from_root(trusted_w, "integration_ci")
                composed = integration.compose(trusted_w, base, head, worktree_path=target_m)
                self.assertEqual(base, composed["base_oid"])
                self.assertEqual(head, composed["head_oid"])
                self.assertEqual(composed["tested_commit_oid"], git(target_m, "rev-parse", "HEAD"))
                self.assertEqual(composed["tested_tree_oid"], git(target_m, "rev-parse", "HEAD^{tree}"))

                validation = load_from_root(trusted_w, "ci-reuse-validation")
                inventory = load_from_root(trusted_w, "ci_required_inventory")
                selected_units = ["doc_checker_contracts", "required_gate_baseline"]
                environment = validation.build_runner_environment(
                    selected_units, inventory.CAPABILITY_RUNNERS, inventory.SELECTOR_ENV,
                )
                self.assertEqual("true", environment["OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS"])
                self.assertEqual("false", environment["OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS"])
                authority = types.SimpleNamespace(request={
                    "source_scope_oid": base,
                    "head_oid": head,
                    "integration_base_oid": base,
                })
                evidence_log = os.environ.get("OASIS7_CI_REUSE_EVIDENCE_LOG")
                log_path = Path(evidence_log) if evidence_log else temp / "required-tier.log"
                log_path.parent.mkdir(parents=True, exist_ok=True)
                with log_path.open("wb") as output:
                    stream = types.SimpleNamespace(buffer=output)
                    with contextlib.redirect_stdout(stream):
                        digest = validation._stream_required_tier(
                            trusted_w, target_m, authority,
                            composed["tested_commit_oid"], composed["tested_tree_oid"],
                            environment,
                        )
                self.assertRegex(digest, r"sha256:[0-9a-f]{64}")
                self.assertEqual(composed["tested_commit_oid"], git(target_m, "rev-parse", "HEAD"))
                self.assertEqual(composed["tested_tree_oid"], git(target_m, "rev-parse", "HEAD^{tree}"))
                self.assertTrue(log_path.stat().st_size > 0)
                print(
                    "exact-M local executor passed: "
                    f"W={head} B={base} H={head} M={composed['tested_commit_oid']} "
                    f"T={composed['tested_tree_oid']} "
                    f"selected={','.join(selected_units)} output_digest={digest} "
                    "claim=local-only"
                )
            except Exception as exc:
                detail = ""
                if log_path.is_file():
                    lines = log_path.read_text(encoding="utf-8", errors="replace").splitlines()
                    detail = "\n" + "\n".join(lines[-60:])
                self.fail(f"exact-M selected required executor failed: {exc}{detail}")
            finally:
                if target_m.exists():
                    subprocess.run(
                        [GIT, "-C", str(trusted_w), "worktree", "remove", "--force", str(target_m)],
                        check=False, capture_output=True, text=True,
                    )
                subprocess.run(
                    [GIT, "-C", str(ROOT), "worktree", "remove", "--force", str(trusted_w)],
                    check=False, capture_output=True, text=True,
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
