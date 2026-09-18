#!/usr/bin/env python3
"""C3-S2b RED contracts for trusted package/profile execution evidence.

The first C3 repair makes planning precise, but keeps package/profile
execution guarded.  These tests define the remaining activation boundary
without running the whole workspace:

* each planned item carries a deterministic repo-owned command and complete
  profile tuple;
* plan authority records policy/planner/toolchain identity;
* execution results bind the planned command/profile evidence;
* integration revalidation enables the path only after trusted authority is
  materialized and publishes exact B/H/T-bound plan/results/receipt artifacts;
* a full-escalation disposition cannot pass without a passing full-tier
  receipt.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
PLANNER = ROOT / "scripts/pm/cargo_package_profile_planner.py"
DRIVER = ROOT / "scripts/pm/cargo_package_profile_driver.py"
WORKFLOW = ROOT / ".github/workflows/rust.yml"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def digest(value: object) -> str:
    return "sha256:" + hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


class C3S2bExecutionEvidenceRED(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.planner = load_module(PLANNER, "cargo_package_profile_planner_c3_s2b")
        cls.driver = load_module(DRIVER, "cargo_package_profile_driver_c3_s2b")

    def _write(self, root: Path, relative: str, content: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def _authority(self, root: Path) -> str:
        self._write(
            root,
            ".pm/cargo-package-scope-policy.json",
            json.dumps(
                {
                    "schema": "oasis7-cargo-package-scope-policy/v1",
                    "policy_version": 1,
                    "protected_paths": [
                        ".pm/cargo-package-scope-policy.json",
                        "scripts/pm/check-cargo-package-scope",
                    ],
                }
            )
            + "\n",
        )
        self._write(root, "scripts/pm/check-cargo-package-scope", "#!/bin/sh\nexit 0\n")
        (root / "scripts/pm/check-cargo-package-scope").chmod(0o755)
        git(root, "init", "-q", "-b", "main")
        git(root, "config", "user.email", "qa@example.invalid")
        git(root, "config", "user.name", "C3 S2b QA")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "trusted base")
        return git(root, "rev-parse", "HEAD")

    def _planner_fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, str, str]:
        temp = tempfile.TemporaryDirectory(prefix="cargo-profile-c3-s2b-")
        root = Path(temp.name)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n',
        )
        self._write(
            root,
            "crates/alpha/Cargo.toml",
            '[package]\nname = "alpha"\nversion = "0.1.0"\nedition = "2021"\n\n'
            '[lib]\npath = "src/lib.rs"\n',
        )
        self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() -> u8 { 1 }\n")
        trusted = self._authority(root)
        git(root, "switch", "-c", "source", trusted)
        self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() -> u8 { 2 }\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "change alpha")
        source_head = git(root, "rev-parse", "HEAD")
        return temp, root, trusted, source_head

    def _plan(self, root: Path, trusted: str, source_head: str) -> dict[str, object]:
        return self.planner.plan_package_profiles(
            root,
            integration_base=trusted,
            source_head=source_head,
            policy_path=".pm/cargo-package-scope-policy.json",
            checker_path="scripts/pm/check-cargo-package-scope",
            profiles=("native",),
        )

    def _manual_plan(self, *, full_escalation: bool = False) -> dict[str, object]:
        command = ["cargo", "test", "-p", "alpha"]
        plan: dict[str, object] = {
            "schema": "oasis7-cargo-package-profile-plan/v1",
            "source_scope_base": "s" * 40,
            "integration_base": "b" * 40,
            "source_head": "h" * 40,
            "tested_tree": "t" * 40,
            "trusted_authority": {
                "policy_sha256": "sha256:" + "1" * 64,
                "planner_sha256": "sha256:" + "2" * 64,
                "toolchain": "rust-toolchain.toml@sha256:" + "3" * 64,
            },
            "selected_items": [],
            "items": [],
            "execution_disposition": "full_escalation"
            if full_escalation
            else "planned_items",
            "disposition_validated": full_escalation,
        }
        if not full_escalation:
            plan.update(
                selected_items=["alpha-native"],
                items=[
                    {
                        "id": "alpha-native",
                        "package": "alpha",
                        "profile": "native",
                        "target": "native",
                        "features": [],
                        "command": command,
                        "command_digest": digest(command),
                    }
                ],
            )
        plan["plan_id"] = digest({key: value for key, value in plan.items()})
        return plan

    def _result(self, **overrides: object) -> dict[str, object]:
        result: dict[str, object] = {
            "item_id": "alpha-native",
            "status": "passed",
            "exit_code": 0,
            "source_head": "h" * 40,
            "integration_base": "b" * 40,
            "tested_tree": "t" * 40,
        }
        result.update(overrides)
        return result

    def test_planner_items_bind_command_profile_and_authority_identity(self) -> None:
        temp, root, trusted, source_head = self._planner_fixture()
        self.addCleanup(temp.cleanup)
        plan = self._plan(root, trusted, source_head)
        authority = plan.get("trusted_authority")
        self.assertIsInstance(authority, dict)
        assert isinstance(authority, dict)
        for field in ("policy_sha256", "planner_sha256", "toolchain"):
            self.assertTrue(authority.get(field), f"missing trusted authority identity: {field}")
        for item in plan["items"]:
            self.assertTrue(item.get("command"), "planned item lacks deterministic command")
            self.assertRegex(
                str(item["command"]),
                r"cargo",
                "planned command must be a repo-owned Cargo command",
            )
            for field in ("package", "profile", "target", "features"):
                self.assertIn(field, item, f"planned profile tuple omits {field}")
            self.assertRegex(
                str(item.get("command_digest")),
                r"^sha256:[0-9a-f]{64}$",
                "planned command must have a digest-bound identity",
            )

    def test_driver_rejects_results_without_plan_command_and_toolchain_evidence(self) -> None:
        plan = self._manual_plan()
        for result in (
            self._result(),
            self._result(plan_id=plan["plan_id"], command_digest="sha256:" + "9" * 64),
        ):
            with self.subTest(result=result):
                with self.assertRaisesRegex(Exception, "command|profile|toolchain|evidence"):
                    self.driver.validate_planned_items(
                        plan,
                        [result],
                        integration_base="b" * 40,
                        source_head="h" * 40,
                        tested_tree="t" * 40,
                    )

    def test_full_escalation_empty_plan_requires_passing_full_tier_receipt(self) -> None:
        plan = self._manual_plan(full_escalation=True)
        with self.assertRaisesRegex(Exception, "full|receipt|success"):
            self.driver.validate_planned_items(
                plan,
                [],
                integration_base="b" * 40,
                source_head="h" * 40,
                tested_tree="t" * 40,
            )

    def test_integration_workflow_activates_only_after_authority_and_publishes_exact_candidate_artifacts(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        authority_marker = 'git show "${OASIS7_CARGO_SCOPE_BASE}:scripts/pm/cargo_package_profile_planner.py"'
        self.assertIn(authority_marker, required)
        activation_marker = "OASIS7_CARGO_PROFILE_OPT_IN=true"
        self.assertIn(activation_marker, required)
        self.assertGreater(required.index(activation_marker), required.index(authority_marker))
        for identity in (
            "OASIS7_CARGO_PROFILE_INTEGRATION_BASE",
            "OASIS7_CARGO_PROFILE_SOURCE_HEAD",
            "OASIS7_CARGO_PROFILE_TESTED_TREE",
        ):
            self.assertIn(identity, required)
        for artifact in (
            "cargo-package-profile-plan",
            "cargo-package-profile-results",
            "cargo-package-profile-receipt",
        ):
            self.assertIn(artifact, required)
        self.assertRegex(required, r"integration_base.{0,500}source_head.{0,500}tested_tree")


if __name__ == "__main__":
    unittest.main(verbosity=2)
