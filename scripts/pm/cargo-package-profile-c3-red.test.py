#!/usr/bin/env python3
"""C3 RED contracts for precise integration revalidation activation.

This fixture suite deliberately targets the C2 deferred boundaries.  It does
not change production code or weaken the immutable C2 contracts:

* the trusted impact-projection helper must be available beside a relocated
  base planner;
* deleted packages may remain in dependency closure, but cannot become
  executable profile items;
* a non-Cargo change must produce an explicitly accepted no-op disposition;
* every externally supplied plan must carry a digest-bound plan_id;
* integration_revalidation must be precise while full_escalation remains
  explicit and full; and
* incomplete, stale, cancelled, or ambiguous result ledgers fail closed.

The current C2 production head is expected to fail the new assertions.  The
repository-health GREEN slice owns the corresponding production changes.
"""

from __future__ import annotations

import importlib.util
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
PLANNER = ROOT / "scripts/pm/cargo_package_profile_planner.py"
DRIVER = ROOT / "scripts/pm/cargo_package_profile_driver.py"
SCOPE_PLANNER = ROOT / "scripts/plan-rust-required-scope.py"
WORKFLOW = ROOT / ".github/workflows/rust.yml"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], text=True
    ).strip()


class C3CargoPackageProfileRED(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.planner = load_module(PLANNER, "cargo_package_profile_planner_c3")
        cls.driver = load_module(DRIVER, "cargo_package_profile_driver_c3")

    def _write(self, root: Path, relative: str, content: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def _package(self, root: Path, name: str, dependencies: str = "") -> None:
        self._write(
            root,
            f"crates/{name}/Cargo.toml",
            f'''[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
{dependencies}
''',
        )
        self._write(root, f"crates/{name}/src/lib.rs", f"pub fn {name}() -> u8 {{ 1 }}\n")

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
        git(root, "config", "user.name", "C3 QA")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "trusted base")
        return git(root, "rev-parse", "HEAD")

    def _plan(
        self,
        root: Path,
        integration_base: str,
        source_head: str,
        **kwargs: object,
    ) -> dict[str, object]:
        return self.planner.plan_package_profiles(
            root,
            integration_base=integration_base,
            source_head=source_head,
            policy_path=".pm/cargo-package-scope-policy.json",
            checker_path="scripts/pm/check-cargo-package-scope",
            profiles=("native",),
            **kwargs,
        )

    def _base_plan(self, **overrides: object) -> dict[str, object]:
        command = ["cargo", "test", "-p", "alpha"]
        plan: dict[str, object] = {
            "schema": "oasis7-cargo-package-profile-plan/v1",
            "plan_id": "not-a-digest",
            "source_scope_base": "s" * 40,
            "integration_base": "b" * 40,
            "source_head": "h" * 40,
            "tested_tree": "t" * 40,
            "trusted_authority": {
                "policy_sha256": "sha256:" + "1" * 64,
                "planner_sha256": "sha256:" + "2" * 64,
                "toolchain": "rust-toolchain.toml@sha256:" + "3" * 64,
            },
            "selected_items": ["alpha-native"],
            "items": [
                {
                    "id": "alpha-native",
                    "package": "alpha",
                    "profile": "native",
                    "target": "native",
                    "features": [],
                    "command": command,
                    "command_digest": "sha256:" + hashlib.sha256(
                        json.dumps(command, sort_keys=True, separators=(",", ":")).encode()
                    ).hexdigest(),
                }
            ],
        }
        plan.update(overrides)
        return plan

    def _result(self, item_id: str = "alpha-native", **overrides: object) -> dict[str, object]:
        unsigned_plan = self._base_plan()
        unsigned_plan.pop("plan_id", None)
        result: dict[str, object] = {
            "item_id": item_id,
            "status": "passed",
            "exit_code": 0,
            "source_head": "h" * 40,
            "integration_base": "b" * 40,
            "tested_tree": "t" * 40,
            "plan_id": "sha256:" + hashlib.sha256(
                json.dumps(unsigned_plan, sort_keys=True, separators=(",", ":")).encode()
            ).hexdigest(),
            "command_digest": "sha256:" + hashlib.sha256(
                json.dumps(["cargo", "test", "-p", "alpha"], sort_keys=True, separators=(",", ":")).encode()
            ).hexdigest(),
            "toolchain": "rust-toolchain.toml@sha256:" + "3" * 64,
            "profile": {
                "package": "alpha", "profile": "native", "target": "native", "features": []
            },
        }
        result.update(overrides)
        return result

    def test_trusted_base_projection_helper_is_copied_for_relocated_base_planner(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required_gate = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        self.assertRegex(
            required_gate,
            r'git show "\$\{OASIS7_CARGO_SCOPE_BASE\}:scripts/pm/workflow-impact-projection\.py"',
            "required-gate must materialize the trusted projection helper next to the relocated planner",
        )

    def test_new_run_mode_flag_is_only_passed_to_dispatch_planner(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required_gate = workflow.split("  required-gate:", 1)[1].split(
            "  windows-package-rollout-behavior:", 1
        )[0]
        dispatch = required_gate.index(
            'elif [[ "${GITHUB_EVENT_NAME}" == workflow_dispatch ]]'
        )
        assignment = required_gate.index(
            'run_mode_args=(--run-mode "${{ inputs.run_mode }}")'
        )
        invocation = required_gate.index('"${run_mode_args[@]}"')
        self.assertLess(dispatch, assignment)
        self.assertLess(assignment, invocation)
        self.assertNotIn('--run-mode "${{ github.event_name', required_gate)

    def test_deleted_package_is_not_an_executable_profile_item(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cargo-profile-c3-deleted-") as raw:
            root = Path(raw)
            self._write(
                root,
                "Cargo.toml",
                '[workspace]\nmembers = ["crates/alpha", "crates/consumer"]\nresolver = "2"\n',
            )
            self._package(root, "alpha")
            self._package(root, "consumer", '[dependencies]\nalpha = { path = "../alpha" }\n')
            trusted = self._authority(root)
            git(root, "switch", "-c", "source", trusted)
            subprocess.run(["git", "-C", str(root), "rm", "-q", "-r", "crates/alpha"], check=True)
            self._write(
                root,
                "crates/consumer/Cargo.toml",
                '[package]\nname = "consumer"\nversion = "0.1.0"\nedition = "2021"\n\n'
                '[lib]\npath = "src/lib.rs"\n',
            )
            self._write(
                root,
                "Cargo.toml",
                '[workspace]\nmembers = ["crates/consumer"]\nresolver = "2"\n',
            )
            git(root, "add", "-A")
            git(root, "commit", "-qm", "delete package")
            source_head = git(root, "rev-parse", "HEAD")
            plan = self._plan(root, trusted, source_head)
            self.assertNotIn("alpha-native", plan["selected_items"])
            self.assertNotIn("alpha-native", [item["id"] for item in plan["items"]])
            self.assertIn("alpha", plan["affected_packages"])

    def test_non_cargo_change_emits_validated_accepted_noop_disposition(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cargo-profile-c3-noop-") as raw:
            root = Path(raw)
            self._write(root, "Cargo.toml", '[workspace]\nmembers = []\nresolver = "2"\n')
            trusted = self._authority(root)
            git(root, "switch", "-c", "source", trusted)
            self._write(root, "README.md", "documentation-only change\n")
            git(root, "add", "-A")
            git(root, "commit", "-qm", "docs-only")
            source_head = git(root, "rev-parse", "HEAD")
            plan = self._plan(root, trusted, source_head)
            self.assertEqual([], plan["selected_items"])
            self.assertEqual("legacy_required_coverage", plan["execution_disposition"])
            self.assertIs(plan["disposition_validated"], True)

    def test_external_plan_id_must_always_be_sha256_digest_bound(self) -> None:
        plan = self._base_plan()
        with self.assertRaisesRegex(Exception, "plan.?id|digest"):
            self.driver.validate_planned_items(
                plan,
                [self._result()],
                integration_base="b" * 40,
                source_head="h" * 40,
                tested_tree="t" * 40,
            )
        plan["plan_id"] = None
        with self.assertRaisesRegex(Exception, "plan.?id|digest"):
            self.driver.validate_planned_items(
                plan,
                [self._result()],
                integration_base="b" * 40,
                source_head="h" * 40,
                tested_tree="t" * 40,
            )

    def test_cancelled_and_ambiguous_results_fail_closed(self) -> None:
        plan = self._base_plan()
        plan_without_id = dict(plan)
        unsigned = dict(plan_without_id)
        unsigned.pop("plan_id", None)
        plan_without_id["plan_id"] = "sha256:" + hashlib.sha256(
            json.dumps(unsigned, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        for result in (
            self._result(status="cancelled"),
            self._result(status="timeout"),
            self._result(exit_code=None),
        ):
            with self.subTest(result=result):
                with self.assertRaisesRegex(Exception, "pass|exit|cancel|timeout"):
                    self.driver.validate_planned_items(
                        plan_without_id,
                        [result],
                        integration_base="b" * 40,
                        source_head="h" * 40,
                        tested_tree="t" * 40,
                    )
        with self.assertRaisesRegex(Exception, "duplicate|ambiguous"):
            self.driver.validate_planned_items(
                plan_without_id,
                [self._result(), self._result()],
                integration_base="b" * 40,
                source_head="h" * 40,
                tested_tree="t" * 40,
            )

    def test_integration_revalidation_is_precise_but_full_escalation_is_explicit(self) -> None:
        def run(mode: str) -> dict[str, str]:
            command = [
                "python3",
                str(SCOPE_PLANNER),
                "--event-name",
                "workflow_dispatch",
                "--run-mode",
                mode,
                "--base-ref",
                "HEAD",
                "--head-ref",
                "HEAD",
                "--changed-path",
                "crates/oasis7_consensus/src/lib.rs",
            ]
            result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            return dict(
                line.split("=", 1)
                for line in result.stdout.splitlines()
                if "=" in line
            )

        targeted = run("integration_revalidation")
        self.assertNotEqual("full", targeted["scope"])
        self.assertEqual("true", targeted["run_consensus_tests"])
        full = run("full_escalation")
        self.assertEqual("full", full["scope"])
        self.assertEqual("true", full["run_oasis7_required_tests"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
