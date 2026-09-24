#!/usr/bin/env python3
"""Acceptance contract for the repository-owned Cargo package scope checker.

CLI contract under test:

    python3 scripts/pm/check-cargo-package-scope \
        --repo-root <checkout> --base <B> --head <H> \
        --primary-package <cargo-package-name> \
        --policy <trusted-policy-path> --json

An allowed scope exits 0 and emits JSON with ``status=allowed``.  A rejected
scope exits non-zero and emits a stable machine-readable reason (or the same
reason in stderr).  The checker must derive package identity from Cargo
metadata, not from a guessed ``crates/<name>`` path.

This file deliberately creates all fixtures at runtime so scope checks remain
self-contained and cannot accidentally depend on a production fixture.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts" / "pm" / "check-cargo-package-scope"


class CargoPackageScopeContract(unittest.TestCase):
    """Executable acceptance contract for C1 package-scope enforcement."""

    def setUp(self) -> None:
        self._temps: list[tempfile.TemporaryDirectory[str]] = []

    def tearDown(self) -> None:
        for temp in self._temps:
            temp.cleanup()

    def _git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(repo), *args],
            check=True,
            text=True,
            capture_output=True,
        )
        return result.stdout.strip()

    def _write(self, repo: Path, relative: str, content: str) -> None:
        path = repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def _fixture(self) -> tuple[Path, str]:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-scope-")
        self._temps.append(temp)
        repo = Path(temp.name)
        self._write(
            repo,
            "Cargo.toml",
            """[workspace]
members = ["crates/alpha", "crates/beta"]
resolver = "2"
""",
        )
        self._write(repo, "Cargo.lock", """version = 3

[[package]]
name = "alpha"
version = "0.1.0"

[[package]]
name = "beta"
version = "0.1.0"
""")
        self._write(
            repo,
            ".pm/cargo-package-scope-policy.json",
            json.dumps(
                {
                    "schema": "oasis7-cargo-package-scope-policy/v1",
                    "policy_version": 1,
                    "protected_paths": [".pm/cargo-package-scope-policy.json"],
                },
                indent=2,
            )
            + "\n",
        )
        for package in ("alpha", "beta"):
            self._write(
                repo,
                f"crates/{package}/Cargo.toml",
                f"""[package]
name = "{package}"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
""",
            )
            self._write(repo, f"crates/{package}/src/lib.rs", f"pub fn {package}() {{}}\n")
        self._write(repo, "crates/beta/src/shared.rs", "pub fn shared() {}\n")
        self._write(repo, "shared/common.rs", "pub const SHARED: u8 = 1;\n")

        self._git(repo, "init", "-q", "-b", "main")
        self._git(repo, "config", "user.email", "qa@example.invalid")
        self._git(repo, "config", "user.name", "C1 QA")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "fixture base")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def _head(self, repo: Path, mutate: Callable[[Path], None], message: str) -> str:
        mutate(repo)
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", message)
        return self._git(repo, "rev-parse", "HEAD")

    def _run_checker(self, repo: Path, base: str, head: str, primary: str) -> subprocess.CompletedProcess[str]:
        if not CHECKER.is_file():
            self.fail(
                "RED: missing implementation: expected executable "
                f"{CHECKER}; add production behavior without editing this test"
            )
        return subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(repo),
                "--base",
                base,
                "--head",
                head,
                "--primary-package",
                primary,
                "--policy",
                str(repo / ".pm/cargo-package-scope-policy.json"),
                "--json",
            ],
            cwd=repo,
            text=True,
            capture_output=True,
        )

    def _assert_allowed(self, repo: Path, base: str, primary: str, mutate: Callable[[Path], None]) -> None:
        head = self._head(repo, mutate, "allowed package-scope fixture")
        result = self._run_checker(repo, base, head, primary)
        self.assertEqual(
            0,
            result.returncode,
            f"expected allowed scope; stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            self.fail(f"allowed result must be JSON: {exc}; stdout={result.stdout!r}")
        self.assertEqual("allowed", payload.get("status"), payload)

    def _assert_rejected(
        self,
        repo: Path,
        base: str,
        primary: str,
        mutate: Callable[[Path], None],
        reason: str,
    ) -> None:
        head = self._head(repo, mutate, "rejected package-scope fixture")
        result = self._run_checker(repo, base, head, primary)
        self.assertNotEqual(
            0,
            result.returncode,
            f"expected rejection {reason!r}; stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        combined = (result.stdout + "\n" + result.stderr).lower()
        self.assertIn(reason.lower(), combined, combined)

    def _assert_rejected_behavior(
        self,
        repo: Path,
        base: str,
        primary: str,
        mutate: Callable[[Path], None],
    ) -> None:
        """Require a semantic rejection without prescribing its reason label."""
        head = self._head(repo, mutate, "rejected dependency fixture")
        result = self._run_checker(repo, base, head, primary)
        combined = (result.stdout + "\n" + result.stderr).lower()
        self.assertNotEqual(
            0,
            result.returncode,
            f"expected semantic scope rejection; stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertNotIn("cargo_metadata_unavailable", combined, combined)
        self.assertNotIn("git_range_unavailable", combined, combined)

    def test_one_package_source_change_is_allowed(self) -> None:
        repo, base = self._fixture()
        self._assert_allowed(
            repo,
            base,
            "alpha",
            lambda root: (root / "crates/alpha/src/lib.rs").write_text(
                "pub fn alpha() { println!(\"changed\"); }\n", encoding="utf-8"
            ),
        )

    def test_one_package_source_change_with_unowned_root_readme_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text(
                "pub fn alpha() { println!(\"changed\"); }\n", encoding="utf-8"
            )
            (root / "README.md").write_text("Unowned root-level change.\n", encoding="utf-8")

        self._assert_rejected(
            repo, base, "alpha", mutate, "ambiguous_package_attribution"
        )

    def test_existing_normal_path_dependency_to_unchanged_target_is_allowed(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { path = "../beta" }\n',
                encoding="utf-8",
            )

        self._assert_allowed(repo, base, "alpha", mutate)

    def _add_normal_path_dependency_with_generated_lockfile(self, root: Path) -> None:
        manifest = root / "crates/alpha/Cargo.toml"
        manifest.write_text(
            manifest.read_text(encoding="utf-8")
            + '\n[dependencies]\nbeta = { path = "../beta" }\n',
            encoding="utf-8",
        )
        generated = subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", str(root / "Cargo.toml")],
            cwd=root,
            check=False,
            text=True,
            capture_output=True,
        )
        self.assertEqual(
            0,
            generated.returncode,
            f"cargo generate-lockfile fixture failed: stdout={generated.stdout!r} stderr={generated.stderr!r}",
        )

    def test_existing_normal_path_dependency_with_generated_lockfile_is_allowed(self) -> None:
        repo, base = self._fixture()
        self._assert_allowed(repo, base, "alpha", self._add_normal_path_dependency_with_generated_lockfile)

    def test_generated_lock_dependency_array_replacement_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._add_normal_path_dependency_with_generated_lockfile(root)
            lock = root / "Cargo.lock"
            content = lock.read_text(encoding="utf-8")
            self.assertIn(' "beta",', content)
            lock.write_text(content.replace(' "beta",', ' "forged",', 1), encoding="utf-8")

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_generated_lock_dependency_array_append_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._add_normal_path_dependency_with_generated_lockfile(root)
            lock = root / "Cargo.lock"
            content = lock.read_text(encoding="utf-8")
            self.assertIn(' "beta",\n]', content)
            lock.write_text(
                content.replace(' "beta",\n]', ' "beta",\n "forged",\n]', 1),
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_generated_lock_dependency_version_suffix_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._add_normal_path_dependency_with_generated_lockfile(root)
            lock = root / "Cargo.lock"
            content = lock.read_text(encoding="utf-8")
            self.assertIn(' "beta",', content)
            lock.write_text(content.replace(' "beta",', ' "beta 0.1.0",', 1), encoding="utf-8")

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_generated_lock_duplicate_dependency_entry_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._add_normal_path_dependency_with_generated_lockfile(root)
            lock = root / "Cargo.lock"
            content = lock.read_text(encoding="utf-8")
            self.assertIn(' "beta",', content)
            lock.write_text(
                content.replace(' "beta",', ' "beta",\n "beta",', 1), encoding="utf-8"
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_source_only_change_with_forged_lock_version_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text(
                "pub fn alpha() { println!(\"source changed\"); }\n", encoding="utf-8"
            )
            lock = root / "Cargo.lock"
            content = lock.read_text(encoding="utf-8")
            self.assertIn('name = "alpha"\nversion = "0.1.0"', content)
            lock.write_text(
                content.replace('name = "alpha"\nversion = "0.1.0"', 'name = "alpha"\nversion = "9.9.9"', 1),
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_existing_normal_path_dependency_with_unrelated_lock_mutation_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._add_normal_path_dependency_with_generated_lockfile(root)
            with (root / "Cargo.lock").open("a", encoding="utf-8") as handle:
                handle.write("\n# unrelated lock mutation\n")

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_two_business_packages_are_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() { 1; }\n", encoding="utf-8")
            (root / "crates/beta/src/lib.rs").write_text("pub fn beta() { 2; }\n", encoding="utf-8")

        self._assert_rejected(repo, base, "alpha", mutate, "multiple_business_packages")

    def test_cross_package_rename_counts_both_endpoints(self) -> None:
        repo, base = self._fixture()
        self._git(repo, "mv", "crates/alpha/src/lib.rs", "crates/beta/src/renamed_from_alpha.rs")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "cross-package rename fixture")
        head = self._git(repo, "rev-parse", "HEAD")
        result = self._run_checker(repo, base, head, "alpha")
        self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("cross_package_rename", (result.stdout + result.stderr).lower())

    def test_root_manifest_change_cannot_hide_behind_package_change(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "Cargo.toml").write_text(
                """[workspace]
members = ["crates/alpha", "crates/beta"]
resolver = "2"

[workspace.metadata.unrelated_policy_change]
enabled = true
""",
                encoding="utf-8",
            )
            (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() { 3; }\n", encoding="utf-8")

        self._assert_rejected(repo, base, "alpha", mutate, "root_manifest_scope")

    def test_new_package_with_workspace_metadata_membership_policy_is_rejected(self) -> None:
        repo, base = self._fixture()
        workspace = repo / "Cargo.toml"
        workspace.write_text(
            workspace.read_text(encoding="utf-8")
            + '\n[workspace.metadata]\nmembership_policy = "legacy"\n',
            encoding="utf-8",
        )
        self._git(repo, "add", "Cargo.toml")
        self._git(repo, "commit", "-qm", "prepare existing workspace metadata")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            workspace = root / "Cargo.toml"
            workspace.write_text(
                workspace.read_text(encoding="utf-8")
                .replace(
                    'members = ["crates/alpha", "crates/beta"]',
                    'members = ["crates/alpha", "crates/beta", "crates/gamma"]',
                )
                .replace('membership_policy = "legacy"', 'membership_policy = "strict"'),
                encoding="utf-8",
            )
            (root / "crates/gamma/Cargo.toml").parent.mkdir(parents=True, exist_ok=True)
            (root / "crates/gamma/Cargo.toml").write_text(
                """[package]
name = "gamma"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
""",
                encoding="utf-8",
            )
            self._write(root, "crates/gamma/src/lib.rs", "pub fn gamma() {}\n")
            with (root / "Cargo.lock").open("a", encoding="utf-8") as handle:
                handle.write('\n[[package]]\nname = "gamma"\nversion = "0.1.0"\n')

        self._assert_rejected(repo, base, "gamma", mutate, "root_manifest_scope")

    def test_unattributable_root_lock_change_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            with (root / "Cargo.lock").open("a", encoding="utf-8") as handle:
                handle.write("\n# unrelated lockfile mutation\n")

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text(
                'include!("../../beta/src/shared.rs");\npub fn alpha() {}\n', encoding="utf-8"
            )

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_include")

    def test_cross_package_path_attribute_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text(
                '#[path = "../../beta/src/shared.rs"]\nmod imported;\npub fn alpha() {}\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_path")

    def test_changed_symlink_into_another_package_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/linked.rs").symlink_to("../../beta/src/shared.rs")

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_symlink")

    def test_unchanged_other_package_path_into_changed_source_is_rejected(self) -> None:
        repo, _ = self._fixture()
        (repo / "crates/beta/src/lib.rs").write_text(
            '#[path = "../../alpha/src/shared.rs"]\nmod imported;\npub fn beta() {}\n',
            encoding="utf-8",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with reverse path edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/shared.rs", "pub fn shared() {}\n"),
            "cross_package_path",
        )

    def test_unchanged_other_package_include_into_changed_source_is_rejected(self) -> None:
        repo, _ = self._fixture()
        (repo / "crates/beta/src/lib.rs").write_text(
            'include!("../../alpha/src/shared.rs");\npub fn beta() {}\n',
            encoding="utf-8",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with reverse include edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/shared.rs", "pub fn shared() {}\n"),
            "cross_package_include",
        )

    def test_cross_package_dev_dependency_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8")
                + '\n[dev-dependencies]\nbeta = { path = "../beta" }\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_dependency")

    def test_cross_package_build_dependency_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8")
                + '\n[build-dependencies]\nbeta = { path = "../beta" }\n',
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_cross_package_registry_dependency_disguise_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { version = "0.1.0" }\n',
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_cross_package_git_dependency_disguise_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { git = "https://example.invalid/beta.git", rev = "0" }\n',
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_cross_package_reverse_dependency_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            alpha = root / "crates/alpha/Cargo.toml"
            beta = root / "crates/beta/Cargo.toml"
            alpha.write_text(
                alpha.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { path = "../beta" }\n',
                encoding="utf-8",
            )
            beta.write_text(
                beta.read_text(encoding="utf-8")
                + '\n[dependencies]\nalpha = { path = "../alpha" }\n',
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_normal_path_dependency_with_changed_target_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            alpha = root / "crates/alpha/Cargo.toml"
            alpha.write_text(
                alpha.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { path = "../beta" }\n',
                encoding="utf-8",
            )
            (root / "crates/beta/src/lib.rs").write_text(
                "pub fn beta() { println!(\"changed target\"); }\n", encoding="utf-8"
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_normal_path_dependency_with_new_target_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            workspace = root / "Cargo.toml"
            workspace.write_text(
                workspace.read_text(encoding="utf-8").replace(
                    'members = ["crates/alpha", "crates/beta"]',
                    'members = ["crates/alpha", "crates/beta", "crates/gamma"]',
                ),
                encoding="utf-8",
            )
            self._write(
                root,
                "crates/gamma/Cargo.toml",
                """[package]
name = "gamma"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
""",
            )
            self._write(root, "crates/gamma/src/lib.rs", "pub fn gamma() {}\n")
            alpha = root / "crates/alpha/Cargo.toml"
            alpha.write_text(
                alpha.read_text(encoding="utf-8")
                + '\n[dependencies]\ngamma = { path = "../gamma" }\n',
                encoding="utf-8",
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_generated_output_into_another_package_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/build.rs").write_text(
                """fn main() {
    std::fs::write("../beta/src/generated.rs", "pub fn generated() {}\\n").unwrap();
}
""",
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_generated_output")

    def test_unowned_change_is_ambiguous_and_fails_closed(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "shared/common.rs").write_text("pub const SHARED: u8 = 2;\n", encoding="utf-8")

        self._assert_rejected(repo, base, "alpha", mutate, "ambiguous_package_attribution")

    def test_trusted_policy_self_modification_cannot_self_approve(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            policy = root / ".pm/cargo-package-scope-policy.json"
            payload = json.loads(policy.read_text(encoding="utf-8"))
            payload["protected_paths"] = []
            policy.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

        self._assert_rejected(repo, base, "alpha", mutate, "policy_self_modification")

    def test_new_package_with_mechanical_member_and_lock_registration_is_allowed(self) -> None:
        repo, base = self._fixture()
        workspace = repo / "Cargo.toml"
        workspace.write_text(
            workspace.read_text(encoding="utf-8").replace(
                'members = ["crates/alpha", "crates/beta"]',
                'members = [\n    "crates/alpha",\n    "crates/beta",\n]',
            ),
            encoding="utf-8",
        )
        self._git(repo, "add", "Cargo.toml")
        self._git(repo, "commit", "-qm", "prepare multiline workspace members")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            workspace = root / "Cargo.toml"
            workspace.write_text(
                workspace.read_text(encoding="utf-8").replace(
                    '    "crates/beta",\n]',
                    '    "crates/beta",\n    "crates/gamma",\n]',
                ),
                encoding="utf-8",
            )
            (root / "crates/gamma/Cargo.toml").parent.mkdir(parents=True, exist_ok=True)
            (root / "crates/gamma/Cargo.toml").write_text(
                """[package]
name = "gamma"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
""",
                encoding="utf-8",
            )
            self._write(root, "crates/gamma/src/lib.rs", "pub fn gamma() {}\n")
            with (root / "Cargo.lock").open("a", encoding="utf-8") as handle:
                handle.write("\n[[package]]\nname = \"gamma\"\nversion = \"0.1.0\"\n")

        self._assert_allowed(repo, base, "gamma", mutate)

    def test_package_local_manifest_and_matching_lock_update_are_allowed(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace('version = "0.1.0"', 'version = "0.1.1"'),
                encoding="utf-8",
            )
            lock = root / "Cargo.lock"
            lock.write_text(
                lock.read_text(encoding="utf-8").replace(
                    'name = "alpha"\nversion = "0.1.0"',
                    'name = "alpha"\nversion = "0.1.1"',
                ),
                encoding="utf-8",
            )

        self._assert_allowed(repo, base, "alpha", mutate)


if __name__ == "__main__":
    unittest.main(verbosity=2)
