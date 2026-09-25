#!/usr/bin/env python3
"""Contract tests for the trusted required-gate unit inventory producer."""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import ExitStack, contextmanager
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
PLANNER_EVENT_NAME = "pull_request"
PLANNER_RUN_MODE = "legacy"
PLANNER_CHANGED_PATHS = ["crates/oasis7_consensus/src/lib.rs"]


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RequiredInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.planner = load_module(
            ROOT / "scripts/plan-rust-required-scope.py", "scope_planner_for_inventory_test",
        )
        cls.inventory = load_module(
            ROOT / "scripts/pm/ci_required_inventory.py", "ci_required_inventory_test",
        )

    def copy_file(self, source: Path, destination: Path):
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)

    @contextmanager
    def observed_product_runtime(
        self, *, runner_image="ubuntu-24.04", python_implementation="CPython",
        python_version="3.12.3", mdurl_version="0.1.2",
    ):
        runner = {
            "runner_image": runner_image,
            "runner_os": "Linux" if runner_image == "ubuntu-24.04" else "macOS",
            "image_os": "ubuntu24" if runner_image == "ubuntu-24.04" else runner_image,
            "image_version": "20260907.300.1",
            "os_id": "ubuntu" if runner_image == "ubuntu-24.04" else "macos",
            "os_version_id": "24.04" if runner_image == "ubuntu-24.04" else "14",
            "matches_trusted_W": runner_image == "ubuntu-24.04",
        }
        python = {
            "implementation": python_implementation,
            "version": python_version,
            "full_version": python_version + " (fixture)",
            "cache_tag": "cpython-312" if python_version.startswith("3.12.") else "other",
        }

        def package(distribution):
            return {
                "markdown-it-py": "3.0.0",
                "mdurl": mdurl_version,
            }[distribution]

        with ExitStack() as stack:
            stack.enter_context(patch.object(self.inventory, "_observe_runner_identity", return_value=runner))
            stack.enter_context(patch.object(self.inventory, "_observe_python_identity", return_value=python))
            stack.enter_context(patch.object(self.inventory, "package_version", side_effect=package))
            yield

    def make_fixture(self, parent: Path):
        trusted = parent / "trusted-w"
        target = parent / "target-m"
        trusted.mkdir()
        target.mkdir()
        source_paths = [
            "scripts/plan-rust-required-scope.py",
            "scripts/ci-tests.sh",
            "scripts/ci-required-capability-test-inventory.tsv",
            "scripts/pm/ci_input_scope.py",
            "scripts/pm/ci_required_inventory.py",
            "scripts/product_doc_markdown.py",
            "scripts/doc-governance-requirements.txt",
            ".github/workflows/rust.yml",
        ]
        for relative in source_paths:
            self.copy_file(ROOT / relative, trusted / relative)
        self.copy_file(
            ROOT / "scripts/fixtures/ci-required-scope.versioned-test.json",
            trusted / "scripts/ci-required-scope.v2.json",
        )
        subprocess.run(["git", "init", "-q"], cwd=trusted, check=True)
        subprocess.run(["git", "config", "user.email", "trusted-w@example.invalid"], cwd=trusted, check=True)
        subprocess.run(["git", "config", "user.name", "Trusted Workflow Test"], cwd=trusted, check=True)
        subprocess.run(["git", "add", "."], cwd=trusted, check=True)
        subprocess.run(["git", "commit", "-qm", "trusted W fixture"], cwd=trusted, check=True)

        target_paths = set(self.inventory.BASELINE_CHECKER_PATHS) | {
            "scripts/ci-tests.sh",
            "scripts/pm/ci-required-inventory.test.py",
            "scripts/plan-rust-required-scope.py",
            "scripts/ci-required-scope.v2.json",
            "scripts/ci-required-capability-test-inventory.tsv",
            "scripts/product_doc_markdown.py",
            "scripts/doc-governance-requirements.txt",
            "scripts/product-doc-governance-check.py",
            "scripts/product-doc-content-check.py",
        }
        for relative in target_paths:
            source = ROOT / relative
            if source.is_file():
                self.copy_file(source, target / relative)
            else:
                self.copy_file(ROOT / "scripts/ci-tests.sh", target / relative)
        self.copy_file(
            ROOT / "scripts/fixtures/ci-required-scope.versioned-test.json",
            target / "scripts/ci-required-scope.v2.json",
        )
        (target / "doc/product").mkdir(parents=True, exist_ok=True)
        (target / "doc/product/demo.prd.md").write_text(
            "# Demo\n\nSee [details](detail.design.md#scope).\n", encoding="utf-8",
        )
        (target / "doc/product/detail.design.md").write_text("# Scope\n\nDetails.\n", encoding="utf-8")
        (target / "doc/product/unlinked.prd.md").write_text("# Unlinked\n", encoding="utf-8")
        (target / "crates/oasis7_consensus/src").mkdir(parents=True, exist_ok=True)
        (target / "crates/oasis7_consensus/Cargo.toml").write_text(
            '[package]\nname="oasis7_consensus"\nversion="0.1.0"\nedition="2021"\n',
            encoding="utf-8",
        )
        (target / "crates/oasis7_consensus/src/lib.rs").write_text("pub fn consensus() {}\n", encoding="utf-8")
        (target / "crates/oasis7_node/src").mkdir(parents=True, exist_ok=True)
        (target / "crates/oasis7_node/Cargo.toml").write_text(
            '[package]\nname="oasis7_node"\nversion="0.1.0"\nedition="2021"\n',
            encoding="utf-8",
        )
        (target / "crates/oasis7_node/src/lib.rs").write_text("pub fn node() {}\n", encoding="utf-8")
        (target / "Cargo.toml").write_text(
            '[workspace]\nmembers=["crates/oasis7_consensus","crates/oasis7_node"]\nresolver="2"\n',
            encoding="utf-8",
        )
        (target / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "oasis7_consensus"\nversion = "0.1.0"\n\n'
            '[[package]]\nname = "oasis7_node"\nversion = "0.1.0"\n',
            encoding="utf-8",
        )
        (target / "rust-toolchain.toml").write_text(
            '[toolchain]\nchannel="1.96.0"\ncomponents=["rustfmt","clippy"]\n'
            'targets=["wasm32-unknown-unknown"]\n', encoding="utf-8",
        )
        (target / "scripts").mkdir(exist_ok=True)
        (target / "scripts/ci-required-scope.v2.json").write_text(
            (trusted / "scripts/ci-required-scope.v2.json").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        subprocess.run(["git", "init", "-q"], cwd=target, check=True)
        subprocess.run(["git", "config", "user.email", "inventory-test@example.invalid"], cwd=target, check=True)
        subprocess.run(["git", "config", "user.name", "Inventory Test"], cwd=target, check=True)
        subprocess.run(["git", "add", "."], cwd=target, check=True)
        subprocess.run(["git", "commit", "-qm", "fixture"], cwd=target, check=True)
        plan = subprocess.run(
            [sys.executable, str(trusted / "scripts/plan-rust-required-scope.py"),
             "--event-name", PLANNER_EVENT_NAME, "--run-mode", PLANNER_RUN_MODE, "--config",
             str(trusted / "scripts/ci-required-scope.v2.json"),
             *[arg for path in PLANNER_CHANGED_PATHS for arg in ("--changed-path", path)]],
            cwd=trusted, check=True, capture_output=True, text=True,
        ).stdout
        return trusted, target, plan

    def build_fixture_inventory(self, trusted: Path, target: Path, plan: str, **overrides):
        target_oid = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=target, text=True).strip()
        planner_oid = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=trusted, text=True).strip()
        inherit_github_context = overrides.pop("_inherit_github_context", False)
        arguments = {
            "event_name": PLANNER_EVENT_NAME,
            "run_mode": PLANNER_RUN_MODE,
            "changed_paths": PLANNER_CHANGED_PATHS,
            "run_id": 17,
            "run_attempt": 1,
            "check_app_id": 2,
            "check_run_id": 19,
        }
        arguments.update(overrides)

        def build():
            return self.inventory.build_required_inventory(
                trusted, target, target_oid, plan,
                repository="eng-cc/oasis7",
                workflow_ref="eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                planner_authority_oid=planner_oid,
                **arguments,
            )

        if inherit_github_context:
            return build()
        return self._build_with_github_context(build, arguments)

    @staticmethod
    def _build_with_github_context(build, arguments):
        with patch.dict(os.environ, {
            "GITHUB_REPOSITORY": "eng-cc/oasis7",
            "GITHUB_EVENT_NAME": arguments["event_name"],
            "GITHUB_RUN_ID": str(arguments["run_id"]),
            "GITHUB_RUN_ATTEMPT": str(arguments["run_attempt"]),
        }):
            return build()

    def test_inventory_authority_must_match_the_trusted_w_checkout(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-w-identity-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            target_oid = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=target, text=True).strip()
            with self.assertRaisesRegex(self.inventory.InventoryError, "does not match planner authority"):
                self.inventory.build_required_inventory(
                    trusted, target, target_oid, plan,
                    repository="eng-cc/oasis7",
                    workflow_ref="eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                    planner_authority_oid="1" * 40,
                    event_name=PLANNER_EVENT_NAME,
                    run_mode=PLANNER_RUN_MODE,
                    changed_paths=PLANNER_CHANGED_PATHS,
                    run_id=17, run_attempt=1, check_app_id=2, check_run_id=19,
                )

    def test_mutated_self_consistent_plan_selection_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-plan-mutation-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            mutated = dict(self.inventory._plan_mapping(plan))
            mutated["selected_capabilities"] = "required_gate_baseline"
            mutated["required_test_units"] = "required_gate_baseline"
            with self.assertRaisesRegex(self.inventory.InventoryError, "does not match trusted W invocation"):
                self.build_fixture_inventory(trusted, target, mutated)

    def test_plan_is_bound_to_exact_event_and_changed_path_invocation(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-plan-identity-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with self.assertRaisesRegex(self.inventory.InventoryError, "does not match trusted W invocation"):
                self.build_fixture_inventory(
                    trusted, target, plan,
                    event_name="workflow_dispatch",
                )
            with self.assertRaisesRegex(self.inventory.InventoryError, "does not match trusted W invocation"):
                self.build_fixture_inventory(
                    trusted, target, plan,
                    changed_paths=["crates/oasis7_node/src/lib.rs"],
                )
            planner_oid = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=trusted, text=True,
            ).strip()
            with self.assertRaisesRegex(self.inventory.InventoryError, "exact changed paths are required"):
                self.build_fixture_inventory(
                    trusted, target, plan,
                    changed_paths=[], base_ref=planner_oid, head_ref=planner_oid,
                )

    def test_run_attempt_is_bound_without_changing_selection_fingerprints(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-attempt-binding-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            first = self.build_fixture_inventory(trusted, target, plan)
            retry = self.build_fixture_inventory(
                trusted, target, plan, run_attempt=2, check_run_id=20,
            )
            self.assertEqual(first["planner_invocation"]["digest"], retry["planner_invocation"]["digest"])
            self.assertNotEqual(
                first["planner_invocation"]["producer"],
                retry["planner_invocation"]["producer"],
            )
            self.assertEqual(
                first["input_scope"]["input_fingerprints"],
                retry["input_scope"]["input_fingerprints"],
            )

    def test_producer_run_attempt_must_match_ambient_actions_identity(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-ambient-run-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with patch.dict(os.environ, {
                "GITHUB_REPOSITORY": "eng-cc/oasis7",
                "GITHUB_WORKFLOW_REF": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                "GITHUB_EVENT_NAME": PLANNER_EVENT_NAME,
                "GITHUB_RUN_ID": "17",
                "GITHUB_RUN_ATTEMPT": "1",
            }, clear=True):
                with self.assertRaisesRegex(
                    self.inventory.InventoryError,
                    "GITHUB_RUN_ATTEMPT",
                ):
                    self.build_fixture_inventory(
                        trusted, target, plan, run_attempt=2,
                        _inherit_github_context=True,
                    )

    def test_cli_emits_inventory_for_the_exact_w_and_m_targets(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-cli-") as temp:
            root = Path(temp)
            trusted, target, plan = self.make_fixture(root)
            target_oid = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=target, text=True).strip()
            planner_oid = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=trusted, text=True).strip()
            plan_json = root / "planner-output.json"
            plan_json.write_text(
                json.dumps(dict(line.split("=", 1) for line in plan.splitlines())), encoding="utf-8",
            )
            output = root / "out/inventory.json"
            cli_environment = os.environ.copy()
            cli_environment.update({
                "GITHUB_REPOSITORY": "eng-cc/oasis7",
                "GITHUB_EVENT_NAME": PLANNER_EVENT_NAME,
                "GITHUB_RUN_ID": "17",
                "GITHUB_RUN_ATTEMPT": "1",
            })
            subprocess.run([
                sys.executable, str(ROOT / "scripts/pm/ci_required_inventory.py"),
                "--planner-root", str(trusted), "--target-repo-root", str(target),
                "--target-oid", target_oid, "--planner-output-json", str(plan_json),
                "--repository", "eng-cc/oasis7",
                "--workflow-ref", "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                "--event-name", PLANNER_EVENT_NAME,
                "--run-mode", PLANNER_RUN_MODE,
                *[arg for path in PLANNER_CHANGED_PATHS for arg in ("--changed-path", path)],
                "--planner-authority-oid", planner_oid, "--run-id", "17",
                "--run-attempt", "1", "--check-app-id", "2", "--check-run-id", "19",
                "--output", str(output),
            ], check=True, capture_output=True, text=True, env=cli_environment)
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(result["schema"], "oasis7-required-test-inventory/v1")
            self.assertEqual(result["input_scope"]["target_oid"], target_oid)
            self.assertEqual(result["planner_invocation"]["event_name"], PLANNER_EVENT_NAME)
            self.assertEqual(result["planner_invocation"]["changed_paths"], PLANNER_CHANGED_PATHS)
            self.assertEqual(
                result["planner_invocation"]["producer"],
                result["planner_inventory_issuer"]["producer"],
            )
            baseline_contract = next(
                item["unit_contract"] for item in result["unit_specs"]
                if item["unit_id"] == "required_gate_baseline"
            )
            self.assertEqual(
                baseline_contract["trusted_W"]["planner_invocation_digest"],
                result["planner_invocation"]["digest"],
            )

    def test_planner_unit_list_always_includes_baseline_and_matches_selected_capabilities(self):
        plan = {
            "execution_contract": self.planner.EXECUTION_CONTRACT,
            "selected_capabilities": "consensus;site_quality",
            "required_test_units": "consensus;required_gate_baseline;site_quality",
        }
        self.assertEqual(
            self.inventory.selected_test_units(plan, self.planner.CAPABILITIES),
            ["consensus", "required_gate_baseline", "site_quality"],
        )

    def test_planner_unit_list_rejects_omitted_or_unknown_units(self):
        base = {
            "execution_contract": self.planner.EXECUTION_CONTRACT,
            "selected_capabilities": "consensus",
            "required_test_units": "consensus",
        }
        with self.assertRaisesRegex(self.inventory.InventoryError, "always include"):
            self.inventory.selected_test_units(base, self.planner.CAPABILITIES)
        base["required_test_units"] = "consensus;required_gate_baseline;untrusted"
        with self.assertRaisesRegex(self.inventory.InventoryError, "unknown required test unit"):
            self.inventory.selected_test_units(base, self.planner.CAPABILITIES)

    def test_registry_covers_every_trusted_planner_capability_and_required_runner(self):
        registry = self.inventory.required_test_unit_registry(self.planner.CAPABILITIES)
        self.assertEqual(set(registry), set(self.planner.CAPABILITIES))
        self.assertTrue(registry["consensus"]["obligations"])
        self.assertTrue(registry["required_gate_baseline"]["obligations"])

    def test_runner_image_observation_requires_ubuntu24_and_github_image_markers(self):
        with ExitStack() as stack:
            stack.enter_context(patch.object(self.inventory.platform, "system", return_value="Linux"))
            stack.enter_context(patch.object(
                self.inventory.Path, "read_text",
                return_value='ID=ubuntu\nVERSION_ID="24.04"\n',
            ))
            stack.enter_context(patch.dict(os.environ, {
                "RUNNER_OS": "Linux", "ImageOS": "ubuntu24", "ImageVersion": "20260907.300.1",
            }, clear=True))
            accepted = self.inventory._observe_runner_identity()
        self.assertEqual(accepted["runner_image"], "ubuntu-24.04")
        self.assertTrue(accepted["matches_trusted_W"])

        with ExitStack() as stack:
            stack.enter_context(patch.object(self.inventory.platform, "system", return_value="Darwin"))
            stack.enter_context(patch.object(
                self.inventory.Path, "read_text", side_effect=OSError("no os-release on macOS"),
            ))
            stack.enter_context(patch.dict(os.environ, {
                "RUNNER_OS": "macOS", "ImageOS": "macos14", "ImageVersion": "20260907.300.1",
            }, clear=True))
            rejected = self.inventory._observe_runner_identity()
        self.assertEqual(rejected["runner_image"], "macos14")
        self.assertFalse(rejected["matches_trusted_W"])

    def test_package_dependency_closure_is_transitive_and_repo_scoped(self):
        metadata = {
            "workspace_root": "/tmp/oasis",
            "packages": [
                {"id": "a 1", "name": "a", "manifest_path": "/tmp/oasis/crates/a/Cargo.toml"},
                {"id": "b 1", "name": "b", "manifest_path": "/tmp/oasis/crates/b/Cargo.toml"},
                {"id": "c 1", "name": "c", "manifest_path": "/tmp/oasis/crates/c/Cargo.toml"},
                {"id": "ext 1", "name": "ext", "manifest_path": "/tmp/cargo/ext/Cargo.toml"},
            ],
            "resolve": {"nodes": [
                {"id": "a 1", "deps": [{"pkg": "b 1"}, {"pkg": "ext 1"}]},
                {"id": "b 1", "deps": [{"pkg": "c 1"}]},
                {"id": "c 1", "deps": []},
                {"id": "ext 1", "deps": []},
            ]},
        }
        self.assertEqual(
            self.inventory.package_dependency_roots(metadata, "/tmp/oasis", ["a"]),
            ["crates/a", "crates/b", "crates/c"],
        )

    def test_complete_scoped_package_and_product_fingerprints_survive_unrelated_package_change(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with self.observed_product_runtime():
                before = self.build_fixture_inventory(trusted, target, plan)
                self.assertEqual(before["closure_status"], "complete")
                before_fingerprints = before["input_scope"]["input_fingerprints"]
                self.assertIn("consensus", before_fingerprints)
                doc_unit = "product-document::doc/product/demo.prd.md"
                self.assertIn(doc_unit, before_fingerprints)
                product_units = [item for item in before["unit_specs"]
                                 if item["unit_id"].startswith("product-")]
                self.assertTrue(product_units)
                self.assertTrue(all(item["applicable_policy"]["reuse_eligible"] for item in product_units))
                baseline_fingerprint = before_fingerprints["required_gate_baseline"]
                self.assertFalse(
                    next(item for item in before["unit_specs"]
                         if item["unit_id"] == "required_gate_baseline")["applicable_policy"]["reuse_eligible"]
                )
                self.assertFalse(
                    next(item for item in before["unit_specs"]
                         if item["unit_id"] == "consensus")["applicable_policy"]["reuse_eligible"]
                )

                node_source = target / "crates/oasis7_node/src/lib.rs"
                node_source.write_text("pub fn node() { let _unrelated = true; }\n", encoding="utf-8")
                subprocess.run(["git", "add", "."], cwd=target, check=True)
                subprocess.run(["git", "commit", "-qm", "unrelated node package update"], cwd=target, check=True)
                after = self.build_fixture_inventory(trusted, target, plan)
                after_fingerprints = after["input_scope"]["input_fingerprints"]
                self.assertEqual(before_fingerprints["consensus"], after_fingerprints["consensus"])
                self.assertEqual(before_fingerprints[doc_unit], after_fingerprints[doc_unit])
                self.assertNotEqual(baseline_fingerprint, after_fingerprints["required_gate_baseline"])

    def test_changed_and_removed_product_documents_invalidate_their_exact_units(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-product-mutation-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            before = self.build_fixture_inventory(trusted, target, plan)
            before_fingerprints = before["input_scope"]["input_fingerprints"]
            document_unit = "product-document::doc/product/demo.prd.md"
            membership_unit = "product-corpus:membership"
            link_unit = (
                "product-link::doc/product/demo.prd.md->doc/product/detail.design.md#scope"
            )
            self.assertIn(link_unit, before_fingerprints)

            demo = target / "doc/product/demo.prd.md"
            demo.write_text(
                "# Demo\n\nChanged text; see [details](detail.design.md#scope).\n",
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=target, check=True)
            subprocess.run(["git", "commit", "-qm", "change product doc"], cwd=target, check=True)
            changed = self.build_fixture_inventory(trusted, target, plan)
            changed_fingerprints = changed["input_scope"]["input_fingerprints"]
            self.assertNotEqual(before_fingerprints[document_unit], changed_fingerprints[document_unit])
            self.assertNotEqual(before_fingerprints[link_unit], changed_fingerprints[link_unit])

            (target / "doc/product/unlinked.prd.md").unlink()
            subprocess.run(["git", "add", "-A"], cwd=target, check=True)
            subprocess.run(["git", "commit", "-qm", "remove product doc"], cwd=target, check=True)
            removed = self.build_fixture_inventory(trusted, target, plan)
            removed_fingerprints = removed["input_scope"]["input_fingerprints"]
            self.assertNotEqual(changed_fingerprints[membership_unit], removed_fingerprints[membership_unit])
            self.assertNotIn("product-document::doc/product/unlinked.prd.md", removed_fingerprints)

    def test_incomplete_product_corpus_cannot_emit_a_planner_inventory(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-product-blocked-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            (target / "doc/product/detail.design.md").unlink()
            subprocess.run(["git", "add", "-A"], cwd=target, check=True)
            subprocess.run(["git", "commit", "-qm", "remove linked product doc"], cwd=target, check=True)
            with self.assertRaisesRegex(self.inventory.InventoryError, "product-corpus closure is incomplete"):
                self.build_fixture_inventory(trusted, target, plan)

    def test_changed_target_markdown_parser_makes_product_units_ineligible(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-parser-drift-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            parser = target / "scripts/product_doc_markdown.py"
            parser.write_text(parser.read_text(encoding="utf-8") + "\n# candidate parser change\n",
                              encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=target, check=True)
            subprocess.run(["git", "commit", "-qm", "change target Markdown parser"], cwd=target, check=True)
            result = self.build_fixture_inventory(trusted, target, plan)
            product_units = [item for item in result["unit_specs"]
                             if item["unit_id"].startswith("product-")]
            self.assertTrue(product_units)
            self.assertTrue(all(not item["applicable_policy"]["reuse_eligible"] for item in product_units))

    def test_macOS_runner_is_not_product_reuse_eligible(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-macos-runner-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with self.observed_product_runtime(runner_image="macos-14"):
                result = self.build_fixture_inventory(trusted, target, plan)
            product_units = [item for item in result["unit_specs"]
                             if item["unit_id"].startswith("product-")]
            self.assertTrue(product_units)
            self.assertTrue(all(not item["applicable_policy"]["reuse_eligible"] for item in product_units))

    def test_python_runtime_drift_is_not_product_reuse_eligible(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-python-drift-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with self.observed_product_runtime(python_version="3.13.0"):
                result = self.build_fixture_inventory(trusted, target, plan)
            product_units = [item for item in result["unit_specs"]
                             if item["unit_id"].startswith("product-")]
            self.assertTrue(product_units)
            self.assertTrue(all(not item["applicable_policy"]["reuse_eligible"] for item in product_units))

    def test_mdurl_drift_is_not_product_reuse_eligible(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-mdurl-drift-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            with self.observed_product_runtime(mdurl_version="0.1.3"):
                result = self.build_fixture_inventory(trusted, target, plan)
            product_units = [item for item in result["unit_specs"]
                             if item["unit_id"].startswith("product-")]
            self.assertTrue(product_units)
            self.assertTrue(all(not item["applicable_policy"]["reuse_eligible"] for item in product_units))

    def test_missing_cargo_closure_falls_back_to_every_inventory_unit(self):
        with tempfile.TemporaryDirectory(prefix="ci-required-inventory-fallback-") as temp:
            trusted, target, plan = self.make_fixture(Path(temp))
            original = self.inventory._cargo_metadata
            self.inventory._cargo_metadata = lambda _root: (_ for _ in ()).throw(
                self.inventory.InventoryError("injected unresolved package closure"),
            )
            try:
                result = self.build_fixture_inventory(trusted, target, plan)
            finally:
                self.inventory._cargo_metadata = original
            scope = result["input_scope"]
            self.assertEqual(result["closure_status"], "unknown")
            self.assertEqual(scope["closure_status"]["status"], "unknown")
            self.assertEqual(scope["input_fingerprints"], {})
            self.assertEqual(scope["required_test_units"], result["planner_inventory_issuer"]["unit_ids"])


if __name__ == "__main__":
    unittest.main()
