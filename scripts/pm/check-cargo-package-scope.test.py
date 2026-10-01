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

    def _auxiliary_base(
        self,
        entries: object,
        files: dict[str, str] | None = None,
        protected_paths: list[str] | None = None,
    ) -> tuple[Path, str]:
        repo, _ = self._fixture()
        policy_path = ".pm/cargo-package-scope-policy.json"
        if protected_paths is not None:
            policy = json.loads((repo / policy_path).read_text(encoding="utf-8"))
            policy["protected_paths"] = protected_paths
            self._write(repo, policy_path, json.dumps(policy, indent=2) + "\n")
        self._write(
            repo,
            ".pm/cargo-package-auxiliary-files.json",
            json.dumps(
                {
                    "schema": "oasis7-cargo-package-auxiliary-files/v1",
                    "auxiliary_files": entries,
                },
                indent=2,
            )
            + "\n",
        )
        for path, content in (files or {}).items():
            self._write(repo, path, content)
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "register trusted auxiliary file paths")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def _assert_invalid_auxiliary_base(
        self,
        entries: object,
        protected_paths: list[str] | None = None,
    ) -> None:
        repo, base = self._auxiliary_base(entries, protected_paths=protected_paths)
        result = self._run_checker(repo, base, base, "auto")
        combined = (result.stdout + "\n" + result.stderr).lower()
        self.assertNotEqual(
            0,
            result.returncode,
            f"expected invalid auxiliary registry rejection; stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertNotIn("cargo_metadata_unavailable", combined, combined)
        self.assertNotIn("git_range_unavailable", combined, combined)

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

    def _assert_invalid_auxiliary_json(self, registry_text: str) -> None:
        repo, _ = self._fixture()
        registry_path = ".pm/cargo-package-auxiliary-files.json"
        self._write(repo, registry_path, registry_text)
        self._git(repo, "add", registry_path)
        self._git(repo, "commit", "-qm", "write duplicate-key auxiliary registry fixture")
        base = self._git(repo, "rev-parse", "HEAD")
        result = self._run_checker(repo, base, base, "auto")
        combined = (result.stdout + "\n" + result.stderr).lower()
        self.assertNotEqual(
            0,
            result.returncode,
            f"expected duplicate-key policy rejection; stdout={result.stdout!r} stderr={result.stderr!r}",
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

    def test_new_normal_dependency_executes_target_build_script_into_source_package(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/beta/build.rs",
            'fn main() { std::fs::write("../alpha/src/generated.rs", "pub fn generated() {}\\n").unwrap(); }\n')
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with beta build script")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(manifest.read_text(encoding="utf-8")
                + '\n[dependencies]\nbeta = { path = "../beta" }\n', encoding="utf-8")

        head = self._head(repo, mutate, "alpha depends on beta")
        compiled = subprocess.run(["cargo", "check", "--offline", "-p", "alpha"],
            cwd=repo, check=False, text=True, capture_output=True)
        self.assertEqual(0, compiled.returncode, compiled.stderr)
        self.assertIn("generated", (repo / "crates/alpha/src/generated.rs").read_text(encoding="utf-8"))
        result = self._run_checker(repo, base, head, "alpha")
        self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("cross_package_generated_output", result.stdout + result.stderr)

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

    def _standalone_fixture(self, *, depends_on_primary: bool = True) -> tuple[Path, str]:
        repo, _ = self._fixture()
        primary_dependency = (
            '\n[dependencies]\nalpha = { path = "../../crates/alpha" }\n'
            if depends_on_primary
            else ""
        )
        self._write(
            repo,
            "tools/runner/Cargo.toml",
            """[package]
name = "runner"
version = "0.1.0"
edition = "2021"

[workspace]
""" + primary_dependency,
        )
        self._write(repo, "tools/runner/src/main.rs", "fn main() {}\n")
        generated = subprocess.run(
            [
                "cargo",
                "generate-lockfile",
                "--offline",
                "--manifest-path",
                str(repo / "tools/runner/Cargo.toml"),
            ],
            cwd=repo,
            check=False,
            text=True,
            capture_output=True,
        )
        self.assertEqual(0, generated.returncode, generated.stderr)
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone workspace base")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def _standalone_transitive_fixture(self) -> tuple[Path, str]:
        repo, _ = self._standalone_fixture()
        workspace = repo / "Cargo.toml"
        workspace.write_text(
            workspace.read_text(encoding="utf-8").replace(
                'members = ["crates/alpha", "crates/beta"]',
                'members = ["crates/alpha", "crates/beta", "crates/gamma"]',
            ),
            encoding="utf-8",
        )
        self._write(
            repo,
            "crates/gamma/Cargo.toml",
            """[package]
name = "gamma"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
""",
        )
        self._write(repo, "crates/gamma/src/lib.rs", "pub fn gamma() {}\n")
        beta_manifest = repo / "crates/beta/Cargo.toml"
        beta_manifest.write_text(
            beta_manifest.read_text(encoding="utf-8")
            + '\n[dependencies]\ngamma = { path = "../gamma" }\n',
            encoding="utf-8",
        )
        generated = subprocess.run(
            [
                "cargo",
                "generate-lockfile",
                "--offline",
                "--manifest-path",
                str(repo / "Cargo.toml"),
            ],
            cwd=repo,
            check=False,
            text=True,
            capture_output=True,
        )
        self.assertEqual(
            0,
            generated.returncode,
            f"cargo workspace lock fixture failed: stdout={generated.stdout!r} stderr={generated.stderr!r}",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone transitive dependency base")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def _standalone_edge_substitution_fixture(self) -> tuple[Path, str]:
        repo, _ = self._standalone_transitive_fixture()
        workspace = repo / "Cargo.toml"
        workspace_text = workspace.read_text(encoding="utf-8")
        members = 'members = ["crates/alpha", "crates/beta", "crates/gamma"]'
        self.assertEqual(1, workspace_text.count(members))
        workspace.write_text(
            workspace_text.replace(
                members,
                'members = ["crates/alpha", "crates/beta", "crates/gamma", '
                '"crates/delta", "crates/epsilon"]',
                1,
            ),
            encoding="utf-8",
        )
        for package in ("delta", "epsilon"):
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

        beta_manifest = repo / "crates/beta/Cargo.toml"
        beta_manifest.write_text(
            beta_manifest.read_text(encoding="utf-8")
            + 'delta = { path = "../delta" }\nepsilon = { path = "../epsilon" }\n',
            encoding="utf-8",
        )
        gamma_manifest = repo / "crates/gamma/Cargo.toml"
        gamma_manifest.write_text(
            gamma_manifest.read_text(encoding="utf-8")
            + '\n[dependencies]\ndelta = { path = "../delta" }\n',
            encoding="utf-8",
        )
        generated = subprocess.run(
            [
                "cargo",
                "generate-lockfile",
                "--offline",
                "--manifest-path",
                str(repo / "Cargo.toml"),
            ],
            cwd=repo,
            check=False,
            text=True,
            capture_output=True,
        )
        self.assertEqual(
            0,
            generated.returncode,
            f"cargo workspace lock fixture failed: stdout={generated.stdout!r} stderr={generated.stderr!r}",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone edge substitution base")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def _add_standalone_lock_for_normal_path_edge(self, root: Path) -> None:
        manifest = root / "crates/alpha/Cargo.toml"
        manifest.write_text(
            manifest.read_text(encoding="utf-8")
            + '\n[dependencies]\nbeta = { path = "../beta" }\n',
            encoding="utf-8",
        )
        generated = subprocess.run(
            [
                "cargo",
                "generate-lockfile",
                "--offline",
                "--manifest-path",
                str(root / "tools/runner/Cargo.toml"),
            ],
            cwd=root,
            check=False,
            text=True,
            capture_output=True,
        )
        if generated.returncode != 0:
            raise AssertionError(generated.stderr)

    def test_standalone_cargo_generated_lock_for_admitted_path_edge_is_allowed(self) -> None:
        repo, base = self._standalone_fixture()
        self._assert_allowed(repo, base, "alpha", self._add_standalone_lock_for_normal_path_edge)

    def test_registered_auxiliary_change_with_valid_standalone_lock_change_is_allowed(self) -> None:
        repo, _ = self._standalone_fixture()
        self._write(
            repo,
            ".pm/cargo-package-auxiliary-files.json",
            json.dumps(
                {
                    "schema": "oasis7-cargo-package-auxiliary-files/v1",
                    "auxiliary_files": [
                        {"path": "scripts/local-signer/helper.py", "package": "alpha"}
                    ],
                },
                indent=2,
            )
            + "\n",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "register trusted signer auxiliary path")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            self._write(root, "scripts/local-signer/helper.py", "print('registered')\n")

        self._assert_allowed(repo, base, "alpha", mutate)

    def test_registered_auxiliary_change_does_not_allow_unrelated_standalone_lock_change(self) -> None:
        repo, _ = self._standalone_fixture()
        self._write(
            repo,
            ".pm/cargo-package-auxiliary-files.json",
            json.dumps(
                {
                    "schema": "oasis7-cargo-package-auxiliary-files/v1",
                    "auxiliary_files": [
                        {"path": "scripts/local-signer/helper.py", "package": "alpha"}
                    ],
                },
                indent=2,
            )
            + "\n",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "register trusted signer auxiliary path")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            self._write(root, "scripts/local-signer/helper.py", "print('registered')\n")
            lock = root / "tools/runner/Cargo.lock"
            lock.write_text(
                lock.read_text(encoding="utf-8")
                + '\n[[package]]\nname = "unrelated"\nversion = "9.9.9"\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_standalone_lock_reachable_transitive_identity_mismatch_is_rejected(self) -> None:
        repo, base = self._standalone_transitive_fixture()
        self._assert_allowed(
            repo, base, "alpha", self._add_standalone_lock_for_normal_path_edge
        )
        valid_migration_head = self._git(repo, "rev-parse", "HEAD")

        def forge_gamma_lock_identity(root: Path) -> None:
            lock = root / "tools/runner/Cargo.lock"
            text = lock.read_text(encoding="utf-8")
            original = 'name = "gamma"\nversion = "0.1.0"'
            self.assertEqual(1, text.count(original), "expected one Cargo-generated gamma lock record")
            lock.write_text(
                text.replace(original, 'name = "gamma"\nversion = "99.0.0"', 1),
                encoding="utf-8",
            )

        forged_head = self._head(
            repo, forge_gamma_lock_identity, "forge reachable gamma lock identity"
        )
        result = self._run_checker(repo, base, forged_head, "alpha")
        combined = (result.stdout + "\n" + result.stderr).lower()
        self.assertNotEqual(
            0,
            result.returncode,
            "expected semantic scope rejection for forged reachable gamma identity; "
            f"fixture_base={base} valid_migration_head={valid_migration_head} "
            f"forged_head={forged_head}; stdout={result.stdout!r} stderr={result.stderr!r}",
        )
        self.assertNotIn("cargo_metadata_unavailable", combined, combined)
        self.assertNotIn("git_range_unavailable", combined, combined)
        self.assertIn("unattributable_lock_change", combined, combined)

    def test_standalone_lock_transitive_edge_substitution_is_rejected(self) -> None:
        repo, base = self._standalone_edge_substitution_fixture()
        self._assert_allowed(
            repo, base, "alpha", self._add_standalone_lock_for_normal_path_edge
        )

        def substitute_gamma_lock_edge(root: Path) -> None:
            lock = root / "tools/runner/Cargo.lock"
            text = lock.read_text(encoding="utf-8")
            for package in ("delta", "epsilon"):
                identity = f'[[package]]\nname = "{package}"\nversion = "0.1.0"'
                self.assertEqual(1, text.count(identity), f"expected one valid {package} lock identity")
            original = (
                '[[package]]\nname = "gamma"\nversion = "0.1.0"\n'
                'dependencies = [\n "delta",\n]'
            )
            replacement = original.replace(' "delta",', ' "epsilon",', 1)
            self.assertEqual(1, text.count(original), "expected Cargo-generated gamma -> delta lock edge")
            lock.write_text(text.replace(original, replacement, 1), encoding="utf-8")

        self._assert_rejected(
            repo,
            base,
            "alpha",
            substitute_gamma_lock_edge,
            "unattributable_lock_change",
        )

    def test_standalone_lock_unrelated_record_change_is_rejected(self) -> None:
        repo, base = self._standalone_fixture()

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            lock = root / "tools/runner/Cargo.lock"
            lock.write_text(
                lock.read_text(encoding="utf-8")
                + '\n[[package]]\nname = "unrelated"\nversion = "9.9.9"\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_standalone_lock_metadata_change_is_rejected(self) -> None:
        repo, base = self._standalone_fixture()

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            lock = root / "tools/runner/Cargo.lock"
            text = lock.read_text(encoding="utf-8")
            version_start = text.index("version = ")
            version_end = text.index("\n", version_start)
            lock.write_text(
                text[:version_start] + "version = 999" + text[version_end:],
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_standalone_lock_unrelated_text_change_is_rejected(self) -> None:
        repo, base = self._standalone_fixture()

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            lock = root / "tools/runner/Cargo.lock"
            lock.write_text(
                lock.read_text(encoding="utf-8").replace(
                    'name = "beta"\nversion = "0.1.0"',
                    'name = "beta"\nversion = "0.1.0"\nmanual = "drift"',
                ),
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def test_standalone_lock_with_changed_manifest_is_rejected(self) -> None:
        repo, base = self._standalone_fixture()

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            manifest = root / "tools/runner/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    'version = "0.1.0"', 'version = "0.1.1"'
                ),
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "ambiguous_package_attribution")

    def test_second_standalone_lock_change_is_rejected(self) -> None:
        repo, base = self._standalone_fixture()

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            self._write(
                root,
                "tools/other/Cargo.lock",
                (root / "tools/runner/Cargo.lock").read_text(encoding="utf-8"),
            )

        self._assert_rejected(repo, base, "alpha", mutate, "ambiguous_package_attribution")

    def test_standalone_lock_missing_base_primary_record_is_rejected(self) -> None:
        repo, _ = self._standalone_fixture()
        lock = repo / "tools/runner/Cargo.lock"
        lock.write_text(
            "version = 3\n\n[[package]]\nname = \"runner\"\nversion = \"0.1.0\"\n",
            encoding="utf-8",
        )
        self._git(repo, "add", "tools/runner/Cargo.lock")
        self._git(repo, "commit", "-qm", "standalone lock missing primary record")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha", self._add_standalone_lock_for_normal_path_edge,
            "unattributable_lock_change",
        )

    def test_standalone_lock_missing_base_manifest_dependency_is_rejected(self) -> None:
        repo, base = self._standalone_fixture(depends_on_primary=False)

        def mutate(root: Path) -> None:
            self._add_standalone_lock_for_normal_path_edge(root)
            lock = root / "tools/runner/Cargo.lock"
            lock.write_text(
                lock.read_text(encoding="utf-8")
                + '\n[[package]]\nname = "alpha"\nversion = "0.1.0"\n\n'
                + '[[package]]\nname = "beta"\nversion = "0.1.0"\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "unattributable_lock_change")

    def _standalone_consumer_with_reference(self, reference: str) -> tuple[Path, str]:
        repo, _ = self._standalone_fixture()
        self._write(repo, "crates/alpha/src/shared.rs", "pub fn shared() {}\n")
        self._write(repo, "tools/runner/src/main.rs", reference)
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone consumer references primary source")
        return repo, self._git(repo, "rev-parse", "HEAD")

    def test_unchanged_standalone_consumer_include_into_changed_source_is_rejected(self) -> None:
        repo, base = self._standalone_consumer_with_reference(
            'include!("../../../crates/alpha/src/shared.rs");\nfn main() {}\n'
        )
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_include",
        )

    def test_unchanged_standalone_consumer_include_bytes_into_changed_source_is_rejected(self) -> None:
        repo, base = self._standalone_consumer_with_reference(
            'include_bytes!("../../../crates/alpha/src/shared.rs");\nfn main() {}\n'
        )
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_include",
        )

    def test_unchanged_standalone_consumer_include_str_into_changed_source_is_rejected(self) -> None:
        repo, base = self._standalone_consumer_with_reference(
            'include_str!("../../../crates/alpha/src/shared.rs");\nfn main() {}\n'
        )
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_include",
        )

    def test_unchanged_standalone_consumer_path_into_changed_source_is_rejected(self) -> None:
        repo, base = self._standalone_consumer_with_reference(
            '#[path = "../../../crates/alpha/src/shared.rs"]\nmod imported;\nfn main() {}\n'
        )
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_path",
        )

    def test_unchanged_standalone_consumer_absolute_include_fails_closed(self) -> None:
        repo, _ = self._standalone_fixture()
        self._write(repo, "crates/alpha/src/shared.rs", "pub fn shared() {}\n")
        absolute_source = (repo / "crates/alpha/src/shared.rs").as_posix()
        self._write(repo, "tools/runner/src/main.rs", f'include!("{absolute_source}");\nfn main() {{}}\n')
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone consumer uses absolute source path")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "unresolved_rust_source_reference",
        )

    def test_unchanged_standalone_consumer_raw_backslash_path_fails_closed(self) -> None:
        repo, base = self._standalone_consumer_with_reference(
            'include!(r"..\\..\\..\\crates\\alpha\\src\\shared.rs");\nfn main() {}\n'
        )
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "unresolved_rust_source_reference",
        )

    def test_unchanged_standalone_consumer_symlink_into_changed_source_is_rejected(self) -> None:
        repo, _ = self._standalone_fixture()
        self._write(repo, "crates/alpha/src/shared.rs", "pub fn shared() {}\n")
        link = repo / "tools/runner/src/linked.rs"
        link.symlink_to("../../../crates/alpha/src/shared.rs")
        self._write(repo, "tools/runner/src/main.rs", "mod linked;\nfn main() {}\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone consumer has reverse symlink")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_symlink",
        )

    def test_unchanged_standalone_consumer_directory_symlink_into_changed_source_is_rejected(self) -> None:
        repo, _ = self._standalone_fixture()
        self._write(repo, "crates/alpha/src/generated/shared.rs", "pub fn shared() {}\n")
        link = repo / "tools/runner/src/alias"
        link.symlink_to("../../../crates/alpha/src/generated", target_is_directory=True)
        self._write(repo, "tools/runner/src/main.rs", "pub mod alias;\nfn main() {}\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "standalone consumer has reverse directory symlink")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/generated/shared.rs", "pub fn changed() {}\n"
            ),
            "cross_package_symlink",
        )

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

    def test_windows_drive_relative_include_fails_closed(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo,
            base,
            "alpha",
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                'include!("C:../../beta/src/shared.rs");\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_cross_package_include_bytes_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_bytes!("../../beta/src/shared.rs");\npub fn alpha() {}\n',
            ),
            "cross_package_include",
        )

    def test_cross_package_include_str_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_str!("../../beta/src/shared.rs");\npub fn alpha() {}\n',
            ),
            "cross_package_include",
        )

    def test_computed_cross_package_include_bytes_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_bytes!(concat!("../../beta/src/", "shared.rs"));\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_computed_cross_package_include_str_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_str!(concat!("../../beta/src/", "shared.rs"));\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_comment_separated_raw_cross_package_include_bytes_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_bytes /* macro gap */ ! /* paren gap */ ( /* literal gap */ '
                'r##"../../beta/src/shared.rs"## );\npub fn alpha() {}\n',
            ),
            "cross_package_include",
        )

    def test_comment_separated_raw_cross_package_include_str_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include_str /* macro gap */ ! /* paren gap */ ( /* literal gap */ '
                'r##"../../beta/src/shared.rs"## );\npub fn alpha() {}\n',
            ),
            "cross_package_include",
        )

    def test_same_package_include_bytes_durable_asset_is_allowed(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/assets/payload.bin", "durable alpha payload\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with durable alpha asset")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_allowed(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'const PAYLOAD: &[u8] = include_bytes!("../assets/payload.bin");\n'
                'pub fn alpha() {}\n',
            ),
        )

    def test_same_package_include_str_durable_asset_is_allowed(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/assets/message.txt", "durable alpha message\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with durable alpha asset")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_allowed(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'const MESSAGE: &str = include_str!("../assets/message.txt");\n'
                'pub fn alpha() {}\n',
            ),
        )

    def test_unaffected_package_manifest_rooted_include_is_allowed(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/beta/src/lib.rs",
            'const MESSAGE: &str = include_str!(concat!(\n'
            '    env!("CARGO_MANIFEST_DIR"),\n'
            '    "/src/shared.rs"\n'
            '));\n'
            'pub fn beta() {}\n',
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with manifest-rooted same-package include")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_allowed(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs", "pub fn changed_alpha() {}\n"),
        )

    def test_manifest_rooted_include_into_changed_package_is_rejected(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/shared.rs", "pub fn shared() {}\n")
        self._write(repo, "crates/beta/src/lib.rs",
            'include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../alpha/src/shared.rs"));\n'
            'pub fn beta() {}\n',
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with manifest-rooted cross-package include")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"),
            "cross_package_include",
        )

    def test_manifest_relative_include_same_package_is_allowed_with_unrelated_fixed_consumer(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/assets/message.txt", "durable alpha message\n")
        self._write(
            repo,
            "crates/beta/src/lib.rs",
            'const MESSAGE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shared.rs"));\n'
            "pub fn beta() {}\n",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with fixed manifest-relative same-package includes")
        base = self._git(repo, "rev-parse", "HEAD")

        def mutate(root: Path) -> None:
            self._write(
                root,
                "crates/alpha/src/lib.rs",
                'const MESSAGE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/message.txt"));\n'
                'pub fn alpha() { println!("changed"); }\n',
            )

        self._assert_allowed(repo, base, "alpha", mutate)

    def test_manifest_relative_include_cross_package_suffix_is_rejected(self) -> None:
        repo, base = self._fixture()
        head = self._head(
            repo,
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                'include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../beta/src/shared.rs"));\n'
                "pub fn alpha() {}\n",
            ),
            "add manifest-relative cross-package include",
        )
        result = self._run_checker(repo, base, head, "alpha")
        self.assertNotEqual(0, result.returncode, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual("cross_package_include", payload.get("reason"), payload)

    def test_manifest_relative_include_suffix_cannot_escape_repository(self) -> None:
        repo, base = self._fixture()
        head = self._head(
            repo,
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                'include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../outside.txt"));\n'
                "pub fn alpha() {}\n",
            ),
            "add escaping manifest-relative include",
        )
        result = self._run_checker(repo, base, head, "alpha")
        self.assertNotEqual(0, result.returncode, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual("unresolved_rust_source_reference", payload.get("reason"), payload)
        self.assertIn("out-of-repository", payload.get("detail", ""), payload)

    def test_cross_package_path_attribute_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/lib.rs").write_text(
                '#[path = "../../beta/src/shared.rs"]\nmod imported;\npub fn alpha() {}\n',
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_path")

    def test_windows_drive_relative_path_attribute_fails_closed(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo,
            base,
            "alpha",
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                '#[path = "C:../../beta/src/shared.rs"]\nmod imported;\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_windows_drive_absolute_path_attribute_fails_closed(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo,
            base,
            "alpha",
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                '#[path = "C:/crates/beta/src/shared.rs"]\nmod imported;\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_same_package_path_attribute_is_allowed(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/sibling.rs", "pub fn sibling() {}\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with alpha sibling source")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_allowed(
            repo,
            base,
            "alpha",
            lambda root: self._write(
                root,
                "crates/alpha/src/lib.rs",
                '#[path = "sibling.rs"]\nmod sibling;\npub fn alpha() {}\n',
            ),
        )

    def test_comment_separated_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs",
                'include/* gap */!/* gap */(/* gap */"../../beta/src/shared.rs");\n'),
            "cross_package_include",
        )

    def test_comment_separated_cross_package_path_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs",
                '#/* gap */[path/* gap */=/* gap */"../../beta/src/shared.rs"]\nmod linked;\n'),
            "cross_package_path",
        )

    def test_nested_comment_separated_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs",
                'include/* outer /* inner */ outer */!/* gap */("../../beta/src/shared.rs");\n'),
            "cross_package_include",
        )

    def test_nested_comment_separated_cross_package_path_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs",
                '#/* outer /* inner */ outer */[path = "../../beta/src/shared.rs"]\nmod linked;\n'),
            "cross_package_path",
        )

    def test_computed_build_output_into_another_package_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/build.rs",
                'fn main() { std::fs::write(concat!("../beta/src/", "generated.rs"), "").unwrap(); }\n'),
            "unresolved_generated_output",
        )

    def test_build_copy_into_another_package_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/build.rs",
                'fn main() { std::fs::copy("src/input.rs", "../beta/src/shared.rs").unwrap(); }\n'),
            "cross_package_generated_output",
        )

    def test_comment_separated_build_copy_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/build.rs",
                'fn main() { std::fs /*gap*/ :: copy("src/input.rs", "../beta/src/shared.rs").unwrap(); }\n'),
            "unresolved_generated_output",
        )

    def test_open_options_build_write_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/build.rs",
                'use std::io::Write;\nfn main() { std::fs::OpenOptions::new().write(true).open("../beta/src/shared.rs").unwrap().write_all(b"x").unwrap(); }\n'),
            "unresolved_generated_output",
        )

    def test_inert_build_script_is_allowed(self) -> None:
        repo, base = self._fixture()
        self._assert_allowed(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/build.rs", "fn main() {}\n"),
        )

    def test_custom_build_target_writing_other_package_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    'edition = "2021"', 'edition = "2021"\nbuild = "scripts/generator.rs"'
                ), encoding="utf-8",
            )
            self._write(root, "crates/alpha/scripts/generator.rs",
                'fn main() { std::fs::write("../beta/src/shared.rs", "pub fn overwritten() {}\\n").unwrap(); }\n')

        head = self._head(repo, mutate, "custom build target fixture")
        compiled = subprocess.run(
            ["cargo", "check", "--offline", "-p", "alpha"], cwd=repo,
            check=False, text=True, capture_output=True,
        )
        self.assertEqual(0, compiled.returncode, compiled.stderr)
        self.assertIn("overwritten", (repo / "crates/beta/src/shared.rs").read_text(encoding="utf-8"))
        result = self._run_checker(repo, base, head, "alpha")
        self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("cross_package_generated_output", result.stdout + result.stderr)

    def test_inert_custom_build_target_is_allowed(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(manifest.read_text(encoding="utf-8").replace(
                'edition = "2021"', 'edition = "2021"\nbuild = "scripts/generator.rs"'),
                encoding="utf-8")
            self._write(root, "crates/alpha/scripts/generator.rs", "fn main() {}\n")

        self._assert_allowed(repo, base, "alpha", mutate)

    def test_custom_build_target_symlink_into_other_package_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            manifest = root / "crates/alpha/Cargo.toml"
            manifest.write_text(manifest.read_text(encoding="utf-8").replace(
                'edition = "2021"', 'edition = "2021"\nbuild = "scripts/generator.rs"'),
                encoding="utf-8")
            self._write(root, "crates/beta/src/generator.rs", "fn main() {}\n")
            link = root / "crates/alpha/scripts/generator.rs"
            link.parent.mkdir(parents=True, exist_ok=True)
            link.symlink_to("../../beta/src/generator.rs")

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_symlink")

    def test_manifest_only_activates_existing_custom_build_target(self) -> None:
        repo, _ = self._fixture()
        manifest = repo / "crates/alpha/Cargo.toml"
        manifest.write_text(manifest.read_text(encoding="utf-8").replace(
            'edition = "2021"', 'edition = "2021"\nbuild = false'), encoding="utf-8")
        self._write(repo, "crates/alpha/scripts/generator.rs",
            'fn main() { std::fs::write("../beta/src/shared.rs", "pub fn overwritten() {}\\n").unwrap(); }\n')
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "dormant custom build target")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/Cargo.toml",
                (root / "crates/alpha/Cargo.toml").read_text(encoding="utf-8")
                .replace("build = false", 'build = "scripts/generator.rs"')),
            "cross_package_generated_output")

    def test_manifest_activates_unchanged_cross_package_path_is_rejected(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/lib.rs",
            '#[cfg(feature = "import_beta")]\n#[path = "../../beta/src/shared.rs"]\nmod linked;\n')
        self._write(repo, "crates/alpha/Cargo.toml",
            (repo / "crates/alpha/Cargo.toml").read_text(encoding="utf-8") + "\n[features]\nimport_beta = []\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with dormant path edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/Cargo.toml",
                (root / "crates/alpha/Cargo.toml").read_text(encoding="utf-8")
                .replace("[features]\n", '[features]\ndefault = ["import_beta"]\n')),
            "cross_package_path")

    def test_manifest_activates_unchanged_computed_include_is_rejected(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/lib.rs",
            '#[cfg(feature = "import_beta")]\ninclude!(concat!("../../beta/src/", "shared.rs"));\n')
        self._write(repo, "crates/alpha/Cargo.toml",
            (repo / "crates/alpha/Cargo.toml").read_text(encoding="utf-8") + "\n[features]\nimport_beta = []\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with dormant computed edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/Cargo.toml",
                (root / "crates/alpha/Cargo.toml").read_text(encoding="utf-8")
                .replace("[features]\n", '[features]\ndefault = ["import_beta"]\n')),
            "unresolved_rust_source_reference")

    def test_escaped_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include!("\\x2e\\x2e/\\x2e\\x2e/beta/src/shared.rs");\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_escaped_cross_package_path_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                '#[path = "\\x2e\\x2e/\\x2e\\x2e/beta/src/shared.rs"]\nmod imported;\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_changed_symlink_into_another_package_is_rejected(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            (root / "crates/alpha/src/linked.rs").symlink_to("../../beta/src/shared.rs")

        self._assert_rejected(repo, base, "alpha", mutate, "cross_package_symlink")

    def test_changed_module_activates_unchanged_cross_package_file_symlink(self) -> None:
        repo, _ = self._fixture()
        (repo / "crates/alpha/src/linked.rs").symlink_to("../../beta/src/shared.rs")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with inactive file symlink")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs", "mod linked;\npub fn alpha() {}\n"),
            "cross_package_symlink",
        )

    def test_changed_module_activates_unchanged_cross_package_directory_symlink(self) -> None:
        repo, _ = self._fixture()
        (repo / "crates/alpha/src/alias").symlink_to("../../beta/src", target_is_directory=True)
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with inactive directory symlink")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/lib.rs", "pub mod alias;\npub fn alpha() {}\n"),
            "cross_package_symlink",
        )

    def test_manifest_feature_activates_unchanged_cross_package_symlink(self) -> None:
        repo, _ = self._fixture()
        (repo / "crates/alpha/src/linked.rs").symlink_to("../../beta/src/shared.rs")
        self._write(repo, "crates/alpha/src/lib.rs", "#[cfg(feature = \"use_link\")]\nmod linked;\n")
        self._write(
            repo, "crates/alpha/Cargo.toml",
            (repo / "crates/alpha/Cargo.toml").read_text(encoding="utf-8")
            + "\n[features]\nuse_link = []\n",
        )
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with dormant feature symlink")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/Cargo.toml",
                (root / "crates/alpha/Cargo.toml").read_text(encoding="utf-8")
                .replace("[features]\n", '[features]\ndefault = ["use_link"]\n'),
            ),
            "cross_package_symlink",
        )

    def test_unchanged_other_package_symlink_into_changed_source_is_rejected(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/shared.rs", "pub fn shared() {}\n")
        (repo / "crates/beta/src/linked.rs").symlink_to("../../alpha/src/shared.rs")
        self._write(repo, "crates/beta/src/lib.rs", "mod linked;\npub fn beta() {}\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with reverse symlink edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/shared.rs", "pub fn changed() {}\n"),
            "cross_package_symlink",
        )

    def test_unchanged_other_package_directory_symlink_into_changed_descendant_is_rejected(self) -> None:
        repo, _ = self._fixture()
        self._write(repo, "crates/alpha/src/mod.rs", "pub fn shared() {}\n")
        (repo / "crates/beta/src/alias").symlink_to("../../alpha/src", target_is_directory=True)
        self._write(repo, "crates/beta/src/lib.rs", "pub mod alias;\npub fn beta() {}\n")
        self._git(repo, "add", "-A")
        self._git(repo, "commit", "-qm", "base with reverse directory symlink edge")
        base = self._git(repo, "rev-parse", "HEAD")
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(root, "crates/alpha/src/mod.rs", "pub fn changed() {}\n"),
            "cross_package_symlink",
        )

    def test_computed_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include!(concat!("../../beta/src/", "shared.rs"));\npub fn alpha() {}\n',
            ),
            "unresolved_rust_source_reference",
        )

    def test_raw_string_cross_package_include_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                'include!(r#"../../beta/src/shared.rs"#);\npub fn alpha() {}\n',
            ),
            "cross_package_include",
        )

    def test_raw_string_cross_package_path_is_rejected(self) -> None:
        repo, base = self._fixture()
        self._assert_rejected(
            repo, base, "alpha",
            lambda root: self._write(
                root, "crates/alpha/src/lib.rs",
                '#[path = r##"../../beta/src/shared.rs"##]\nmod imported;\n',
            ),
            "cross_package_path",
        )

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

    def test_registered_auxiliary_files_cover_add_modify_delete_and_rename(self) -> None:
        cases = [
            (
                "add",
                [{"path": "scripts/local-signer/add.py", "package": "alpha"}],
                {},
                lambda root: self._write(root, "scripts/local-signer/add.py", "print('added')\n"),
            ),
            (
                "modify",
                [{"path": "doc/local-signer/operations.md", "package": "alpha"}],
                {"doc/local-signer/operations.md": "before\n"},
                lambda root: self._write(root, "doc/local-signer/operations.md", "after\n"),
            ),
            (
                "delete",
                [{"path": "scripts/local-signer/obsolete.py", "package": "alpha"}],
                {"scripts/local-signer/obsolete.py": "print('obsolete')\n"},
                lambda root: (root / "scripts/local-signer/obsolete.py").unlink(),
            ),
            (
                "rename",
                [
                    {"path": "scripts/local-signer/old-name.py", "package": "alpha"},
                    {"path": "scripts/local-signer/new-name.py", "package": "alpha"},
                ],
                {"scripts/local-signer/old-name.py": "print('same bytes')\n"},
                lambda root: (root / "scripts/local-signer/old-name.py").rename(
                    root / "scripts/local-signer/new-name.py"
                ),
            ),
        ]
        for operation, entries, files, change_auxiliary in cases:
            with self.subTest(operation=operation):
                repo, base = self._auxiliary_base(entries, files)

                def mutate(root: Path, change_auxiliary=change_auxiliary) -> None:
                    self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
                    change_auxiliary(root)

                self._assert_allowed(repo, base, "alpha", mutate)

    def test_unregistered_auxiliary_sibling_is_not_covered_by_an_exact_entry(self) -> None:
        repo, base = self._auxiliary_base(
            [{"path": "scripts/local-signer/package-release.py", "package": "alpha"}]
        )

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            self._write(root, "scripts/local-signer/package-release.py", "print('registered')\n")
            self._write(root, "scripts/local-signer/unregistered-sibling.py", "print('unregistered')\n")

        self._assert_rejected(repo, base, "alpha", mutate, "ambiguous_package_attribution")

    def test_head_only_auxiliary_registration_cannot_self_authorize(self) -> None:
        repo, base = self._fixture()

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            self._write(root, "scripts/local-signer/installer.py", "print('installer')\n")
            registry_path = root / ".pm/cargo-package-auxiliary-files.json"
            registry_path.write_text(
                json.dumps(
                    {
                        "schema": "oasis7-cargo-package-auxiliary-files/v1",
                        "auxiliary_files": [
                            {"path": "scripts/local-signer/installer.py", "package": "alpha"}
                        ],
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

        self._assert_rejected(repo, base, "alpha", mutate, "policy_self_modification")

    def test_malformed_conflicting_unknown_and_reserved_auxiliary_registrations_fail_closed(self) -> None:
        cases = [
            ("registry must be a list", {"path": "scripts/setup.py", "package": "alpha"}),
            ("entry must have exact fields", [{"path": "scripts/setup.py"}]),
            ("entry cannot add authority fields", [{"path": "scripts/setup.py", "package": "alpha", "owner": "beta"}]),
            ("parent traversal", [{"path": "../outside.py", "package": "alpha"}]),
            ("absolute path", [{"path": "/outside.py", "package": "alpha"}]),
            ("non-normalized path", [{"path": "scripts/./setup.py", "package": "alpha"}]),
            ("glob path", [{"path": "scripts/*.py", "package": "alpha"}]),
            ("unknown package", [{"path": "scripts/setup.py", "package": "missing"}]),
            (
                "duplicate path",
                [
                    {"path": "scripts/setup.py", "package": "alpha"},
                    {"path": "scripts/setup.py", "package": "alpha"},
                ],
            ),
            (
                "conflicting path owners",
                [
                    {"path": "scripts/setup.py", "package": "alpha"},
                    {"path": "scripts/setup.py", "package": "beta"},
                ],
            ),
            ("Cargo-owned path cannot be remapped", [{"path": "crates/beta/src/lib.rs", "package": "alpha"}]),
            ("scope policy path", [{"path": ".pm/cargo-package-scope-policy.json", "package": "alpha"}]),
            ("auxiliary registry path", [{"path": ".pm/cargo-package-auxiliary-files.json", "package": "alpha"}]),
            ("GitHub workflow path", [{"path": ".github/workflows/rust.yml", "package": "alpha"}]),
            ("agent role path", [{"path": ".agents/roles/tpm.md", "package": "alpha"}]),
            ("Codex adapter path", [{"path": ".codex/agents/tpm.toml", "package": "alpha"}]),
            ("PM evidence path", [{"path": ".pm/tasks/example.execution.md", "package": "alpha"}]),
            ("scope checker path", [{"path": "scripts/pm/check-cargo-package-scope", "package": "alpha"}]),
            ("required CI gate entrypoint", [{"path": "scripts/ci-tests.sh", "package": "alpha"}]),
            ("required scope planner config", [{"path": "scripts/ci-required-scope.v2.json", "package": "alpha"}]),
            ("required scope planner Python entrypoint", [{"path": "scripts/plan-rust-required-scope.py", "package": "alpha"}]),
            ("required scope planner shell entrypoint", [{"path": "scripts/plan-rust-required-scope.sh", "package": "alpha"}]),
            ("required scope planner config path", [{"path": "scripts/plan-rust-required-scope.json", "package": "alpha"}]),
            ("PR preparation path", [{"path": "scripts/prepare-task-pr.sh", "package": "alpha"}]),
            (
                "canonical workflow source path",
                [{"path": "doc/engineering/workflow/source-of-truth.md", "package": "alpha"}],
            ),
        ]
        for label, entries in cases:
            with self.subTest(case=label):
                self._assert_invalid_auxiliary_base(entries)

    def test_registered_auxiliary_path_cannot_remap_unowned_rust_source(self) -> None:
        repo, base = self._auxiliary_base(
            [{"path": "shared/common.rs", "package": "alpha"}]
        )

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            self._write(root, "shared/common.rs", "pub const SHARED: u8 = 2;\n")

        self._assert_rejected(repo, base, "alpha", mutate, "ambiguous_package_attribution")

    def test_protected_path_rejection_precedes_registered_auxiliary_attribution(self) -> None:
        repo, base = self._auxiliary_base(
            [{"path": "doc/local-signer/release.md", "package": "alpha"}],
            {
                "doc/local-signer/release.md": "before\n",
                "ops/protected.md": "before\n",
            },
            protected_paths=[".pm/cargo-package-scope-policy.json", "ops/protected.md"],
        )

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            self._write(root, "doc/local-signer/release.md", "after\n")
            self._write(root, "ops/protected.md", "after\n")

        self._assert_rejected(repo, base, "alpha", mutate, "policy_self_modification")

    def test_auxiliary_attribution_does_not_allow_a_second_business_package(self) -> None:
        repo, base = self._auxiliary_base(
            [{"path": "scripts/local-signer/helper.py", "package": "alpha"}]
        )

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            self._write(root, "crates/beta/src/lib.rs", "pub fn beta() { println!(\"changed\"); }\n")
            self._write(root, "scripts/local-signer/helper.py", "print('helper')\n")

        self._assert_rejected(repo, base, "alpha", mutate, "multiple_business_packages")

    def test_auxiliary_rename_endpoints_must_resolve_to_the_same_package(self) -> None:
        repo, base = self._auxiliary_base(
            [
                {"path": "scripts/local-signer/old-name.py", "package": "alpha"},
                {"path": "scripts/local-signer/new-name.py", "package": "beta"},
            ],
            {"scripts/local-signer/old-name.py": "print('same bytes')\n"},
        )

        def mutate(root: Path) -> None:
            self._write(root, "crates/alpha/src/lib.rs", "pub fn alpha() { println!(\"changed\"); }\n")
            (root / "scripts/local-signer/old-name.py").rename(
                root / "scripts/local-signer/new-name.py"
            )

        self._assert_rejected_behavior(repo, base, "alpha", mutate)

    def test_auxiliary_registration_cannot_claim_a_protected_path(self) -> None:
        self._assert_invalid_auxiliary_base(
            [{"path": "ops/protected.md", "package": "alpha"}],
            protected_paths=[".pm/cargo-package-scope-policy.json", "ops/protected.md"],
        )

    def test_duplicate_json_auxiliary_registry_or_member_keys_fail_closed(self) -> None:
        policies = [
            (
                "duplicate registry key",
                '{"schema":"oasis7-cargo-package-auxiliary-files/v1",'
                '"auxiliary_files":[],"auxiliary_files":[]}\n',
            ),
            (
                "duplicate member key",
                '{"schema":"oasis7-cargo-package-auxiliary-files/v1",'
                '"auxiliary_files":[{"path":"scripts/first.py","path":"scripts/second.py",'
                '"package":"alpha"}]}\n',
            ),
        ]
        for label, policy_text in policies:
            with self.subTest(case=label):
                self._assert_invalid_auxiliary_json(policy_text)

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
