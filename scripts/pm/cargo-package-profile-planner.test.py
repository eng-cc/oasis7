#!/usr/bin/env python3
"""RED contract for the C2 Cargo package/profile planner.

The implementation under test is intentionally not present on the C2 RED
head.  The GREEN implementation must provide
``scripts/pm/cargo_package_profile_planner.py`` with a public
``plan_package_profiles`` function.  The API is deliberately small so the
same planner can be called by local preflight and the trusted CI driver.

The planner contract is:

* source scope is ``merge-base(integration_base, source_head)..source_head``;
  ``integration_base`` remains a separate frozen identity;
* checker and policy authority are read from the trusted source-scope base,
  never from candidate files;
* package closure uses the union of base/head dependency graphs, including
  reverse consumers when a dependency is deleted;
* package/profile selection is deterministic and does not silently select
  the legacy workspace_support aggregate;
* native, WASM, feature, target, and unknown-impact boundaries are explicit.
* consumer closure reaches a fixed point over the source and tested trees,
  including root packages and configured independent tool workspaces.
* bounded production-style independent roots remain discoverable even when
  candidate workspace metadata omits their configuration.
* unknown/full escalation carries an explicit validated disposition instead of
  pretending that a partial item list is safe.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
IMPLEMENTATION = ROOT / "scripts" / "pm" / "cargo_package_profile_planner.py"


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        text=True,
        capture_output=True,
    )
    return result.stdout.strip()


class CargoPackageProfilePlannerContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not IMPLEMENTATION.is_file():
            raise AssertionError(
                "RED: missing implementation: expected public planner at "
                f"{IMPLEMENTATION}; add production behavior without editing this test"
            )
        spec = importlib.util.spec_from_file_location("cargo_package_profile_planner", IMPLEMENTATION)
        if spec is None or spec.loader is None:
            raise AssertionError(f"RED: unable to load planner module {IMPLEMENTATION}")
        cls.api = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.api)
        if not hasattr(cls.api, "plan_package_profiles"):
            raise AssertionError("RED: planner must export plan_package_profiles")

    def _write(self, root: Path, relative: str, content: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def _package(self, root: Path, name: str, dependencies: str = "") -> None:
        self._write(
            root,
            f"crates/{name}/Cargo.toml",
            f"""[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
{dependencies}
""",
        )
        self._write(root, f"crates/{name}/src/lib.rs", f"pub fn {name}() -> u8 {{ 1 }}\n")

    def _fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, str, str, str]:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-planner-")
        root = Path(temp.name)
        self._write(
            root,
            "Cargo.toml",
            """[workspace]
members = ["crates/alpha", "crates/beta", "crates/gamma"]
resolver = "2"
""",
        )
        self._package(root, "alpha", '[dependencies]\nbeta = { path = "../beta" }\n')
        self._package(root, "beta")
        self._package(root, "gamma", '[dependencies]\nbeta = { path = "../beta" }\n')
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
                },
                indent=2,
            )
            + "\n",
        )
        self._write(root, "scripts/pm/check-cargo-package-scope", "#!/bin/sh\necho trusted-checker\n")
        (root / "scripts/pm/check-cargo-package-scope").chmod(0o755)

        git(root, "init", "-q", "-b", "main")
        git(root, "config", "user.email", "qa@example.invalid")
        git(root, "config", "user.name", "C2 QA")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "trusted base")
        trusted_base = git(root, "rev-parse", "HEAD")

        # The integration target advances independently with an unrelated
        # package change.  The source branch remains based on trusted_base.
        git(root, "switch", "-c", "integration")
        (root / "crates/gamma/src/lib.rs").write_text("pub fn gamma() -> u8 { 2 }\n", encoding="utf-8")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "integration-only change")
        integration_base = git(root, "rev-parse", "HEAD")

        git(root, "switch", "-c", "source", trusted_base)
        # Delete alpha's dependency.  A head-only graph would lose beta from
        # the affected closure entirely; base/head union must retain the
        # removed dependency and its reverse consumer.
        alpha_manifest = root / "crates/alpha/Cargo.toml"
        alpha_manifest.write_text(
            alpha_manifest.read_text(encoding="utf-8").replace(
                "[dependencies]\nbeta = { path = \"../beta\" }\n", ""
            ),
            encoding="utf-8",
        )
        git(root, "add", "-A")
        git(root, "commit", "-qm", "delete dependency")
        source_head = git(root, "rev-parse", "HEAD")
        return temp, root, trusted_base, integration_base, source_head

    def _init_authority(self, root: Path) -> str:
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
                },
                indent=2,
            )
            + "\n",
        )
        self._write(root, "scripts/pm/check-cargo-package-scope", "#!/bin/sh\necho trusted-checker\n")
        (root / "scripts/pm/check-cargo-package-scope").chmod(0o755)
        git(root, "init", "-q", "-b", "main")
        git(root, "config", "user.email", "qa@example.invalid")
        git(root, "config", "user.name", "C2 QA")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "trusted base")
        return git(root, "rev-parse", "HEAD")

    def _commit(self, root: Path, message: str) -> str:
        git(root, "add", "-A")
        git(root, "commit", "-qm", message)
        return git(root, "rev-parse", "HEAD")

    def _plan(self, root: Path, integration_base: str, source_head: str, **kwargs):
        return self.api.plan_package_profiles(
            root,
            integration_base=integration_base,
            source_head=source_head,
            policy_path=".pm/cargo-package-scope-policy.json",
            checker_path="scripts/pm/check-cargo-package-scope",
            **kwargs,
        )

    def test_advanced_integration_base_preserves_source_scope_and_identity(self) -> None:
        temp, root, trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        plan = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertEqual(trusted_base, plan["source_scope_base"])
        self.assertEqual(integration_base, plan["integration_base"])
        self.assertEqual(source_head, plan["source_head"])
        self.assertEqual(
            git(root, "merge-tree", "--write-tree", integration_base, source_head),
            plan["tested_tree"],
        )

    def test_trusted_authority_rejects_candidate_checker_or_policy_self_modification(self) -> None:
        temp, root, trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        git(root, "switch", "source")
        (root / ".pm/cargo-package-scope-policy.json").write_text(
            '{"schema":"candidate-self-approval"}\n', encoding="utf-8"
        )
        (root / "scripts/pm/check-cargo-package-scope").write_text(
            "#!/bin/sh\necho candidate-checker\n", encoding="utf-8"
        )
        git(root, "add", "-A")
        git(root, "commit", "-qm", "candidate authority mutation")
        candidate = git(root, "rev-parse", "HEAD")
        with self.assertRaisesRegex(Exception, "trusted|authority|self"):
            self._plan(root, integration_base, candidate, profiles=("native",))
        self.assertEqual(trusted_base, git(root, "merge-base", integration_base, candidate))

    def test_base_head_dependency_union_retains_reverse_consumer_after_deletion(self) -> None:
        temp, root, _trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        plan = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertEqual(["alpha"], plan["changed_packages"])
        self.assertIn("alpha", plan["affected_packages"])
        self.assertIn("beta", plan["affected_packages"])
        self.assertIn("base", plan["dependency_graph_sources"])
        self.assertIn("head", plan["dependency_graph_sources"])

    def test_deterministic_plan_avoids_unrelated_workspace_support_aggregate(self) -> None:
        temp, root, _trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        first = self._plan(root, integration_base, source_head, profiles=("native",))
        second = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertEqual(first, second)
        self.assertEqual(["alpha", "beta"], first["affected_packages"])
        self.assertNotIn("workspace_support", first["selected_items"])
        self.assertNotIn("oasis7_wasm_router", first["affected_packages"])

    def test_supported_native_wasm_feature_target_profiles_are_explicit(self) -> None:
        temp, root, _trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        plan = self._plan(
            root,
            integration_base,
            source_head,
            profiles=(
                {"id": "native", "target": "native", "features": []},
                {
                    "id": "wasm",
                    "target": "wasm32-unknown-unknown",
                    "features": ["wasm"],
                },
            ),
        )
        profiles = {item["id"]: item for item in plan["profiles"]}
        self.assertEqual("native", profiles["native"]["target"])
        self.assertEqual([], profiles["native"]["features"])
        self.assertEqual("wasm32-unknown-unknown", profiles["wasm"]["target"])
        self.assertEqual(["wasm"], profiles["wasm"]["features"])

    def test_unknown_profile_or_target_escalates_conservatively(self) -> None:
        temp, root, _trusted_base, integration_base, source_head = self._fixture()
        self.addCleanup(temp.cleanup)
        plan = self._plan(
            root,
            integration_base,
            source_head,
            profiles=({"id": "unknown", "target": "mips-unknown-none", "features": ["experimental"]},),
        )
        self.assertEqual("full", plan["scope"])
        self.assertIn("unknown", plan["escalation_reasons"])
        self.assertEqual([], plan["selected_items"])
        self.assertEqual([], plan["items"])
        self.assertEqual("full_escalation", plan["execution_disposition"])
        self.assertIs(plan["disposition_validated"], True)

    def test_fixed_point_transitive_consumers_include_all_reverse_dependents(self) -> None:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-transitive-")
        root = Path(temp.name)
        self.addCleanup(temp.cleanup)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/alpha", "crates/beta", "crates/gamma"]\nresolver = "2"\n',
        )
        self._package(root, "alpha")
        self._package(root, "beta", '[dependencies]\nalpha = { path = "../alpha" }\n')
        self._package(root, "gamma", '[dependencies]\nbeta = { path = "../beta" }\n')
        trusted_base = self._init_authority(root)

        git(root, "switch", "-c", "integration")
        (root / "crates/gamma/src/lib.rs").write_text("pub fn gamma() -> u8 { 2 }\n", encoding="utf-8")
        integration_base = self._commit(root, "integration-only change")

        git(root, "switch", "-c", "source", trusted_base)
        (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() -> u8 { 2 }\n", encoding="utf-8")
        source_head = self._commit(root, "change dependency root")
        plan = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertEqual(["alpha", "beta", "gamma"], plan["affected_packages"])

    def test_tested_tree_only_consumer_edge_is_selected(self) -> None:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-tested-tree-")
        root = Path(temp.name)
        self.addCleanup(temp.cleanup)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n',
        )
        self._package(root, "alpha")
        trusted_base = self._init_authority(root)

        git(root, "switch", "-c", "integration")
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/alpha", "crates/integration_consumer"]\nresolver = "2"\n',
        )
        self._package(
            root,
            "integration_consumer",
            '[dependencies]\nalpha = { path = "../alpha" }\n',
        )
        integration_base = self._commit(root, "add integration consumer")

        git(root, "switch", "-c", "source", trusted_base)
        (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() -> u8 { 2 }\n", encoding="utf-8")
        source_head = self._commit(root, "change source package")
        plan = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertIn("integration_consumer", plan["affected_packages"])

    def test_root_package_owns_root_manifest_and_sources(self) -> None:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-root-")
        root = Path(temp.name)
        self.addCleanup(temp.cleanup)
        self._write(
            root,
            "Cargo.toml",
            '[package]\nname = "root_app"\nversion = "0.1.0"\nedition = "2021"\n\n'
            '[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n',
        )
        self._write(root, "src/lib.rs", "pub fn root_app() -> u8 { 1 }\n")
        self._package(root, "alpha")
        trusted_base = self._init_authority(root)

        git(root, "switch", "-c", "integration")
        (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() -> u8 { 2 }\n", encoding="utf-8")
        integration_base = self._commit(root, "integration-only change")

        git(root, "switch", "-c", "source", trusted_base)
        (root / "src/lib.rs").write_text("pub fn root_app() -> u8 { 2 }\n", encoding="utf-8")
        source_head = self._commit(root, "change root package")
        plan = self._plan(root, integration_base, source_head, profiles=("native",))
        self.assertIn("root_app", plan["changed_packages"])
        self.assertIn("root_app", plan["affected_packages"])

    def test_configured_independent_wasm_and_tool_workspaces_are_consumers(self) -> None:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-independent-")
        root = Path(temp.name)
        self.addCleanup(temp.cleanup)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/core"]\nresolver = "2"\n\n'
            '[workspace.metadata.oasis7]\n'
            'independent_profile_workspaces = ["tools/builtin_modules", '
            '"tools/wasm_build_suite", "tools/wasm_module_observe"]\n',
        )
        self._package(root, "core")
        for path, name in (
            ("tools/builtin_modules", "builtin_modules"),
            ("tools/wasm_build_suite", "wasm_build_suite"),
            ("tools/wasm_module_observe", "wasm_module_observe"),
        ):
            self._write(
                root,
                f"{path}/Cargo.toml",
                f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n'
                '[dependencies]\ncore = { path = "../../crates/core" }\n\n[workspace]\n',
            )
            self._write(root, f"{path}/src/lib.rs", f"pub fn {name}() -> u8 {{ 1 }}\n")
        trusted_base = self._init_authority(root)

        git(root, "switch", "-c", "integration")
        (root / "tools/wasm_build_suite/src/lib.rs").write_text(
            "pub fn wasm_build_suite() -> u8 { 2 }\n", encoding="utf-8"
        )
        integration_base = self._commit(root, "integration-only tool change")

        git(root, "switch", "-c", "source", trusted_base)
        (root / "crates/core/src/lib.rs").write_text("pub fn core() -> u8 { 2 }\n", encoding="utf-8")
        source_head = self._commit(root, "change core package")
        plan = self._plan(
            root,
            integration_base,
            source_head,
            profiles=(
                {"id": "native", "target": "native", "features": []},
                {"id": "wasm", "target": "wasm32-unknown-unknown", "features": ["wasm"]},
            ),
        )
        for package in ("builtin_modules", "wasm_build_suite", "wasm_module_observe"):
            self.assertIn(package, plan["affected_packages"])

    def test_deterministic_independent_roots_survive_candidate_metadata_omission(self) -> None:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-root-discovery-")
        root = Path(temp.name)
        self.addCleanup(temp.cleanup)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/core"]\nresolver = "2"\n\n'
            '[workspace.metadata.oasis7]\n'
            'independent_profile_workspaces = ["tools/wasm_build_suite"]\n',
        )
        self._package(root, "core")
        self._write(
            root,
            "tools/wasm_build_suite/Cargo.toml",
            '[package]\nname = "wasm_build_suite"\nversion = "0.1.0"\nedition = "2021"\n\n'
            '[dependencies]\ncore = { path = "../../crates/core" }\n\n[workspace]\n',
        )
        self._write(root, "tools/wasm_build_suite/src/lib.rs", "pub fn wasm_build_suite() -> u8 { 1 }\n")
        trusted_base = self._init_authority(root)

        git(root, "switch", "-c", "source", trusted_base)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/core"]\nresolver = "2"\n',
        )
        for path, name in (
            ("crates/oasis7_builtin_wasm_modules/m1_demo", "builtin_demo"),
            ("tools/wasm_module_observe", "wasm_module_observe"),
        ):
            self._write(
                root,
                f"{path}/Cargo.toml",
                f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n'
                '[dependencies]\ncore = { path = "../../core" }\n\n[workspace]\n',
            )
            self._write(root, f"{path}/src/lib.rs", f"pub fn {name}() -> u8 {{ 1 }}\n")
        (root / "crates/core/src/lib.rs").write_text("pub fn core() -> u8 { 2 }\n", encoding="utf-8")
        source_head = self._commit(root, "omit metadata but change core")
        plan = self._plan(root, trusted_base, source_head, profiles=("native",))
        for package in ("builtin_demo", "wasm_build_suite", "wasm_module_observe"):
            self.assertIn(package, plan["affected_packages"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
