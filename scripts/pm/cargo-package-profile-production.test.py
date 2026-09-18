#!/usr/bin/env python3
"""Production boundary contracts beyond the immutable C2 RED fixtures."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "scripts/pm/cargo_package_profile_driver.py"
PLANNER = ROOT / "scripts/pm/cargo_package_profile_planner.py"


class CargoPackageProfileProductionContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        spec = importlib.util.spec_from_file_location("profile_driver", DRIVER)
        assert spec and spec.loader
        cls.driver = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.driver)

    def _plan(self, **extra: object) -> dict[str, object]:
        plan: dict[str, object] = {
            "schema": "oasis7-cargo-package-profile-plan/v1",
            "plan_id": "empty-plan",
            "integration_base": "b" * 40,
            "source_head": "h" * 40,
            "tested_tree": "t" * 40,
            "selected_items": [],
            "items": [],
        }
        plan.update(extra)
        return plan

    def _validate(self, plan: dict[str, object]) -> dict[str, object]:
        return self.driver.validate_planned_items(
            plan,
            [],
            integration_base="b" * 40,
            source_head="h" * 40,
            tested_tree="t" * 40,
        )

    def test_empty_plan_without_disposition_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "empty"):
            self._validate(self._plan())

    def test_empty_plan_requires_validated_legacy_or_full_disposition(self) -> None:
        with self.assertRaisesRegex(Exception, "validated"):
            self._validate(self._plan(execution_disposition="legacy_required_coverage"))
        receipt = self._validate(
            self._plan(
                execution_disposition="full_escalation",
                disposition_validated=True,
            )
        )
        self.assertEqual("full_escalation", receipt["execution_disposition"])

    def test_opt_in_cli_path_produces_and_validates_real_receipt(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cargo-profile-production-") as raw:
            repo = Path(raw)
            (repo / "crates/alpha/src").mkdir(parents=True)
            (repo / ".pm").mkdir()
            (repo / "scripts/pm").mkdir(parents=True)
            (repo / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n', encoding="utf-8"
            )
            (repo / "crates/alpha/Cargo.toml").write_text(
                '[package]\nname="alpha"\nversion="0.1.0"\nedition="2021"\n', encoding="utf-8"
            )
            source = repo / "crates/alpha/src/lib.rs"
            source.write_text("pub fn value() -> u8 { 1 }\n", encoding="utf-8")
            (repo / ".pm/cargo-package-scope-policy.json").write_text(
                json.dumps(
                    {
                        "schema": "oasis7-cargo-package-scope-policy/v1",
                        "policy_version": 1,
                        "protected_paths": [".pm/cargo-package-scope-policy.json"],
                    }
                ),
                encoding="utf-8",
            )
            checker = repo / "scripts/pm/check-cargo-package-scope"
            checker.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            checker.chmod(0o755)
            subprocess.run(["git", "-C", str(repo), "init", "-q", "-b", "main"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.email", "qa@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "C2 production"], check=True)
            subprocess.run(["git", "-C", str(repo), "add", "-A"], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-qm", "base"], check=True)
            base = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
            source.write_text("pub fn value() -> u8 { 2 }\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "-A"], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-qm", "source"], check=True)
            head = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
            plan_path, results_path = repo / "plan.json", repo / "results.json"
            subprocess.run(
                [
                    "python3", str(PLANNER), "--repo-root", str(repo),
                    "--integration-base", base, "--source-head", head,
                    "--policy", ".pm/cargo-package-scope-policy.json",
                    "--checker", "scripts/pm/check-cargo-package-scope",
                    "--profile", "native", "--output", str(plan_path),
                ],
                check=True,
            )
            plan = json.loads(plan_path.read_text(encoding="utf-8"))
            results_path.write_text(
                json.dumps(
                    [
                        {
                            "item_id": item_id,
                            "status": "passed",
                            "exit_code": 0,
                            "integration_base": base,
                            "source_head": head,
                            "tested_tree": plan["tested_tree"],
                        }
                        for item_id in plan["selected_items"]
                    ]
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [
                    "python3", str(DRIVER), "--plan", str(plan_path),
                    "--results", str(results_path), "--integration-base", base,
                    "--source-head", head, "--tested-tree", plan["tested_tree"],
                ],
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual("passed", json.loads(completed.stdout)["status"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
