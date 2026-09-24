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
import base64
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


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

    def _plan_with_live_authority(
        self,
        root: Path,
        integration_base: str,
        source_head: str,
        live_receipt: dict[str, object],
        live_task: dict[str, object],
        **kwargs,
    ):
        with patch.object(
            self.api,
            "_read_approved_normative_source_from_github",
            return_value=live_receipt,
        ), patch.object(
            self.api,
            "_read_current_planner_task_from_github",
            return_value=live_task,
        ):
            return self._plan(root, integration_base, source_head, **kwargs)

    def _authority_fixture(
        self,
    ) -> tuple[tempfile.TemporaryDirectory[str], Path, str, str, str, str]:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-profile-authority-")
        root = Path(temp.name)
        self._write(
            root,
            "Cargo.toml",
            '[workspace]\nmembers = ["crates/alpha"]\nresolver = "2"\n',
        )
        self._package(root, "alpha")
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
        self._write(root, "scripts/pm/check-cargo-package-scope", "#!/bin/sh\necho trusted-checker\n")
        (root / "scripts/pm/check-cargo-package-scope").chmod(0o755)
        self._write(
            root,
            self.api.AUTHORITY_PATH,
            f'<a id="{self.api.AUTHORITY_FRAGMENT}"></a>predecessor package scope\n'
            f'<a id="{self.api.AUTHORITY_CONTRACT_FRAGMENT}"></a>predecessor authority contract\n',
        )
        git(root, "init", "-q", "-b", "main")
        git(root, "config", "user.email", "qa@example.invalid")
        git(root, "config", "user.name", "C2 QA")
        git(root, "remote", "add", "origin", "git@github.com:eng-cc/oasis7.git")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "predecessor normative source")
        predecessor_scope = git(root, "rev-parse", "HEAD")
        git(root, "update-ref", "refs/remotes/origin/main", predecessor_scope)
        git(root, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")

        git(root, "switch", "-c", "normative", predecessor_scope)
        self._write(
            root,
            self.api.AUTHORITY_PATH,
            f'<a id="{self.api.AUTHORITY_FRAGMENT}"></a>trusted standalone lock authority\n'
            f'<a id="{self.api.AUTHORITY_CONTRACT_FRAGMENT}"></a>trusted staged authority contract\n',
        )
        predecessor_head = self._commit(root, "normative source candidate")

        git(root, "switch", "main")
        self._write(
            root,
            self.api.AUTHORITY_PATH,
            f'<a id="{self.api.AUTHORITY_FRAGMENT}"></a>trusted standalone lock authority\n'
            f'<a id="{self.api.AUTHORITY_CONTRACT_FRAGMENT}"></a>trusted staged authority contract\n',
        )
        merged_commit = self._commit(root, "merged normative source")
        git(root, "update-ref", "refs/remotes/origin/main", merged_commit)

        git(root, "switch", "-c", "source", merged_commit)
        for path in self.api.PLANNER_WRITE_SCOPE:
            self._write(root, path, "# planner authority stage fixture\n")
        source_head = self._commit(root, "planner authority stage")
        return temp, root, predecessor_scope, predecessor_head, merged_commit, source_head

    def _authority_receipt(
        self,
        root: Path,
        predecessor_scope: str,
        predecessor_head: str,
        merged_commit: str,
        source_head: str,
    ) -> dict[str, object]:
        path = self.api.AUTHORITY_PATH
        merged_bytes = subprocess.check_output(
            ["git", "-C", str(root), "show", f"{merged_commit}:{path}"]
        )
        predecessor_bytes = subprocess.check_output(
            ["git", "-C", str(root), "show", f"{predecessor_scope}:{path}"]
        )
        fragment = self.api._fragment_bytes(merged_bytes, self.api.AUTHORITY_FRAGMENT)
        contract_fragment = self.api._fragment_bytes(merged_bytes, self.api.AUTHORITY_CONTRACT_FRAGMENT)
        return {
            "repository": "eng-cc/oasis7",
            "default_branch": "main",
            "stage": "normative_source",
            "task_uid": self.api.APPROVED_NORMATIVE_TASK_UID,
            "pr_number": self.api.APPROVED_NORMATIVE_PR,
            "source_head": predecessor_head,
            "source_scope_base": predecessor_scope,
            "merged_commit": merged_commit,
            "merged_tree": git(root, "show", "-s", "--format=%T", merged_commit),
            "authority_path": path,
            "authority_blob": git(root, "rev-parse", f"{merged_commit}:{path}"),
            "authority_bytes_sha256": "sha256:" + hashlib.sha256(merged_bytes).hexdigest(),
            "authority_size": len(merged_bytes),
            "stable_fragment": self.api.AUTHORITY_FRAGMENT,
            "stable_fragment_sha256": "sha256:" + hashlib.sha256(fragment).hexdigest(),
            "authority_contract_fragment": self.api.AUTHORITY_CONTRACT_FRAGMENT,
            "authority_contract_fragment_sha256": "sha256:" + hashlib.sha256(contract_fragment).hexdigest(),
            "predecessor_file_sha256": "sha256:" + hashlib.sha256(predecessor_bytes).hexdigest(),
            "candidate_source_head": source_head,
        }

    def _authority_binding(
        self, merged_commit: str, source_head: str, receipt: dict[str, object]
    ) -> dict[str, object]:
        return {
            "repository": "eng-cc/oasis7",
            "default_branch": "main",
            "stage": "planner_authority",
            "task_uid": self.api.CURRENT_PLANNER_TASK_UID,
            "pr_number": 4921,
            "integration_base": merged_commit,
            "source_head": source_head,
            "source_scope_base": merged_commit,
            "predecessor_task_uid": receipt["task_uid"],
            "predecessor_pr_number": receipt["pr_number"],
            "predecessor_source_head": receipt["source_head"],
            "predecessor_source_scope_base": receipt["source_scope_base"],
            "predecessor_file_sha256": receipt["predecessor_file_sha256"],
            "predecessor_authority_commit": receipt["merged_commit"],
            "predecessor_authority_tree": receipt["merged_tree"],
            "predecessor_authority_path": receipt["authority_path"],
            "predecessor_authority_blob": receipt["authority_blob"],
            "predecessor_authority_digest": receipt["authority_bytes_sha256"],
            "predecessor_authority_fragment": receipt["stable_fragment"],
            "predecessor_authority_fragment_sha256": receipt["stable_fragment_sha256"],
            "predecessor_authority_contract_fragment": receipt["authority_contract_fragment"],
            "predecessor_authority_contract_fragment_sha256": receipt["authority_contract_fragment_sha256"],
            "write_scope": list(self.api.PLANNER_WRITE_SCOPE),
        }

    def _current_task_identity(
        self,
        root: Path,
        merged_commit: str,
        source_head: str,
        binding: dict[str, object],
        *,
        integration_base: str | None = None,
        initial_base: str | None = None,
    ) -> dict[str, object]:
        integration_base = integration_base or merged_commit
        initial_base = initial_base or merged_commit
        return {
            "repository": "eng-cc/oasis7",
            "issue_number": 3999,
            "task_uid": binding["task_uid"],
            "initial_base": initial_base,
            "integration_base": integration_base,
            "branch": git(root, "branch", "--show-current"),
            "pr_number": binding["pr_number"],
            "head_sha": source_head,
            "head_repository": "eng-cc/oasis7",
            "head_ref": git(root, "branch", "--show-current"),
            "base_sha": integration_base,
            "base_repository": "eng-cc/oasis7",
            "base_ref": "main",
        }

    def test_approved_normative_source_is_read_back_and_bound(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(
            root, predecessor_scope, predecessor_head, merged_commit, source_head
        )
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)
        plan = self._plan_with_live_authority(
            root,
            merged_commit,
            source_head,
            receipt,
            live_task,
            profiles=("native",),
            approved_normative_source=receipt,
            authority_binding=binding,
            expected_task_uid=binding["task_uid"],
            expected_pr_number=binding["pr_number"],
        )
        authority = plan["trusted_authority"]["approved_normative_source"]
        self.assertEqual(merged_commit, authority["merged_commit"])
        self.assertEqual("planner_authority", authority["planner_stage"])
        self.assertEqual(sorted(binding["write_scope"]), authority["planner_write_scope"])

        bound_only = self._plan_with_live_authority(
            root,
            merged_commit,
            source_head,
            receipt,
            live_task,
            profiles=("native",),
            authority_binding=binding,
            expected_task_uid=binding["task_uid"],
            expected_pr_number=binding["pr_number"],
        )
        self.assertEqual(authority["merged_commit"], bound_only["trusted_authority"]["approved_normative_source"]["merged_commit"])

    def _advance_default_branch(self, root: Path, relative: str, content: str) -> str:
        git(root, "switch", "main")
        self._write(root, relative, content)
        tip = self._commit(root, "advance default branch")
        git(root, "update-ref", "refs/remotes/origin/main", tip)
        git(root, "switch", "source")
        return tip

    def _rebase_source_onto_default(self, root: Path) -> str:
        git(root, "switch", "source")
        git(root, "rebase", "main")
        return git(root, "rev-parse", "HEAD")

    def test_advanced_default_branch_with_unchanged_authority_is_allowed(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(root, predecessor_scope, predecessor_head, merged_commit, source_head)
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)
        live_tip = self._advance_default_branch(root, "README.md", "unrelated default-branch change\n")
        plan = self._plan_with_live_authority(
            root,
            merged_commit,
            source_head,
            receipt,
            live_task,
            profiles=("native",),
            authority_binding=binding,
            expected_task_uid=binding["task_uid"],
            expected_pr_number=binding["pr_number"],
        )
        self.assertNotEqual(merged_commit, live_tip)
        self.assertEqual(merged_commit, plan["trusted_authority"]["approved_normative_source"]["merged_commit"])

    def test_rebased_source_scope_uses_live_base_not_approved_commit(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        live_base = self._advance_default_branch(root, "README.md", "unrelated rebased base change\n")
        rebased_source_head = self._rebase_source_onto_default(root)
        receipt = self._authority_receipt(
            root, predecessor_scope, predecessor_head, merged_commit, rebased_source_head
        )
        binding = self._authority_binding(live_base, rebased_source_head, receipt)
        live_task = self._current_task_identity(
            root,
            merged_commit,
            rebased_source_head,
            binding,
            integration_base=live_base,
        )
        plan = self._plan_with_live_authority(
            root,
            live_base,
            rebased_source_head,
            receipt,
            live_task,
            profiles=("native",),
            authority_binding=binding,
            expected_task_uid=binding["task_uid"],
            expected_pr_number=binding["pr_number"],
        )
        self.assertEqual(live_base, plan["source_scope_base"])
        self.assertEqual(merged_commit, plan["trusted_authority"]["approved_normative_source"]["merged_commit"])

    def test_rebased_source_scope_authority_tamper_is_rejected(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        live_base = self._advance_default_branch(
            root,
            self.api.AUTHORITY_PATH,
            '<a id="cargo-checker-authority-upgrade"></a>tampered rebased authority\n',
        )
        rebased_source_head = self._rebase_source_onto_default(root)
        receipt = self._authority_receipt(
            root, predecessor_scope, predecessor_head, merged_commit, rebased_source_head
        )
        binding = self._authority_binding(live_base, rebased_source_head, receipt)
        live_task = self._current_task_identity(
            root,
            merged_commit,
            rebased_source_head,
            binding,
            integration_base=live_base,
        )
        with self.assertRaisesRegex(Exception, "source scope authority path|fragment"):
            self._plan_with_live_authority(
                root,
                live_base,
                rebased_source_head,
                receipt,
                live_task,
                profiles=("native",),
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
            )

    def test_default_branch_authority_tamper_is_rejected(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(root, predecessor_scope, predecessor_head, merged_commit, source_head)
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)
        self._advance_default_branch(
            root,
            self.api.AUTHORITY_PATH,
            '<a id="cargo-checker-authority-upgrade"></a>tampered authority\n',
        )
        with self.assertRaisesRegex(Exception, "authority path|fragment"):
            self._plan_with_live_authority(
                root,
                merged_commit,
                source_head,
                receipt,
                live_task,
                profiles=("native",),
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
            )

    def test_default_branch_divergence_is_rejected(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(root, predecessor_scope, predecessor_head, merged_commit, source_head)
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)
        git(root, "switch", "-c", "diverged", predecessor_scope)
        self._write(root, "README.md", "diverged default branch\n")
        divergent_tip = self._commit(root, "diverge default branch")
        git(root, "update-ref", "refs/remotes/origin/main", divergent_tip)
        git(root, "switch", "source")
        with self.assertRaisesRegex(Exception, "ancestor|authority"):
            self._plan_with_live_authority(
                root,
                merged_commit,
                source_head,
                receipt,
                live_task,
                profiles=("native",),
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
            )

    def test_planner_authority_requires_live_readback(self) -> None:
        temp, root, _predecessor_scope, _predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(root, _predecessor_scope, _predecessor_head, merged_commit, source_head)
        binding = self._authority_binding(merged_commit, source_head, receipt)
        with patch.object(
            self.api,
            "_read_approved_normative_source_from_github",
            side_effect=self.api.PlanError("live readback unavailable"),
        ):
            with self.assertRaisesRegex(Exception, "live readback"):
                self._plan(
                    root,
                    merged_commit,
                    source_head,
                    profiles=("native",),
                    approved_normative_source=receipt,
                    authority_binding=binding,
                    expected_task_uid=binding["task_uid"],
                    expected_pr_number=binding["pr_number"],
                )

    def test_live_predecessor_pr_binds_head_and_base_identity(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self.api.APPROVED_NORMATIVE_EXPECTED
        body = (
            "stage=normative_source; immutable, not candidate authority; "
            f"repository={receipt['repository']}; default_branch={receipt['default_branch']}; "
            f"task_uid={receipt['task_uid']}; PR={receipt['pr_number']}; "
            f"source_head={receipt['source_head']}; "
            f"trusted_predecessor/source_scope_base={receipt['source_scope_base']}; "
            f"predecessor authority path={receipt['authority_path']}; "
            f"predecessor file sha256={receipt['predecessor_file_sha256'][7:]}. "
            f"GitHub live PR readback reports MERGED into commit={receipt['merged_commit']}; "
            f"live git/commits API tree={receipt['merged_tree']}; "
            f"live contents API path={receipt['authority_path']} blob={receipt['authority_blob']}, "
            f"size={receipt['authority_size']}, decoded bytes sha256={receipt['authority_bytes_sha256'][7:]}. "
            f"Stable fragment anchor={receipt['stable_fragment']}, "
            f"sha256 including final LF={receipt['stable_fragment_sha256'][7:]}. "
            f"Adjacent staged-authority fragment anchor={receipt['authority_contract_fragment']}, "
            f"sha256 including final LF={receipt['authority_contract_fragment_sha256'][7:]}"
        )
        comment = {
            "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/3996",
            "html_url": "https://github.com/eng-cc/oasis7/issues/3996#issuecomment-5822424913",
            "body": body,
        }
        pr = {
            "number": 3997,
            "state": "closed",
            "merged": True,
            "merge_commit_sha": receipt["merged_commit"],
            "head": {"sha": receipt["source_head"], "repo": {"full_name": "eng-cc/oasis7"}},
            "base": {
                "sha": receipt["source_scope_base"],
                "ref": "main",
                "repo": {"full_name": "eng-cc/oasis7"},
            },
        }
        source_bytes = subprocess.check_output(
            ["git", "-C", str(ROOT), "show", f"{receipt['merged_commit']}:{receipt['authority_path']}"]
        )
        repository = {"full_name": "eng-cc/oasis7", "default_branch": "main"}
        commit = {
            "sha": receipt["merged_commit"],
            "tree": {"sha": receipt["merged_tree"]},
        }
        content = {
            "path": receipt["authority_path"],
            "sha": receipt["authority_blob"],
            "size": receipt["authority_size"],
            "encoding": "base64",
            "content": base64.b64encode(source_bytes).decode("ascii"),
        }
        payloads = {
            f"repos/{self.api.APPROVED_NORMATIVE_REPOSITORY}": repository,
            f"repos/{self.api.APPROVED_NORMATIVE_REPOSITORY}/issues/comments/{self.api.APPROVED_NORMATIVE_COMMENT}": comment,
            f"repos/{self.api.APPROVED_NORMATIVE_REPOSITORY}/pulls/{self.api.APPROVED_NORMATIVE_PR}": pr,
            f"repos/{self.api.APPROVED_NORMATIVE_REPOSITORY}/git/commits/{receipt['merged_commit']}": commit,
            f"repos/{self.api.APPROVED_NORMATIVE_REPOSITORY}/contents/{receipt['authority_path']}?ref={receipt['merged_commit']}": content,
            "repository": repository,
            "comment": comment,
            "pr": pr,
            "commit": commit,
            "content": content,
        }
        real_run = self.api.subprocess.run

        def fake_gh_run(command, **kwargs):
            if command[0] != "gh":
                return real_run(command, **kwargs)
            payload = payloads.get(command[2])
            if payload is None:
                return subprocess.CompletedProcess(command, 1, "", "unexpected GitHub endpoint")
            return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

        cases = (
            ("wrong PR head", "pr", lambda value: value["head"].__setitem__("sha", "0" * 40), "head"),
            ("wrong PR head repo", "pr", lambda value: value["head"].__setitem__("repo", {"full_name": "other/repository"}), "head"),
            ("wrong PR base SHA", "pr", lambda value: value["base"].__setitem__("sha", "0" * 40), "base"),
            ("wrong PR base branch", "pr", lambda value: value["base"].__setitem__("ref", "develop"), "base"),
            ("wrong PR base repo", "pr", lambda value: value["base"].__setitem__("repo", {"full_name": "other/repository"}), "base"),
            ("wrong merged commit", "pr", lambda value: value.__setitem__("merge_commit_sha", "0" * 40), "commit"),
            ("wrong repository identity", "repository", lambda value: value.__setitem__("full_name", "other/repository"), "identity"),
            ("wrong server default branch", "repository", lambda value: value.__setitem__("default_branch", "develop"), "branch"),
            ("wrong comment issue", "comment", lambda value: value.__setitem__("issue_url", "https://api.github.com/repos/other/repository/issues/3996"), "identity"),
            ("wrong commit identity", "commit", lambda value: value.__setitem__("sha", "0" * 40), "commit"),
            ("wrong commit tree", "commit", lambda value: value["tree"].__setitem__("sha", "0" * 40), "tree"),
            ("wrong contents path", "content", lambda value: value.__setitem__("path", "doc/other.md"), "path"),
            ("wrong contents blob", "content", lambda value: value.__setitem__("sha", "0" * 40), "blob"),
            ("wrong contents size", "content", lambda value: value.__setitem__("size", 0), "size"),
            ("wrong contents bytes", "content", lambda value: value.__setitem__("content", base64.b64encode(b"x" * len(source_bytes)).decode("ascii")), "digest"),
        )
        for name, key, mutation, error in cases:
            with self.subTest(case=name):
                bad_payloads = deepcopy(payloads)
                mutation(bad_payloads[key])

                def bad_gh_run(command, **kwargs):
                    if command[0] != "gh":
                        return real_run(command, **kwargs)
                    payload = bad_payloads.get(command[2])
                    if payload is None:
                        return subprocess.CompletedProcess(command, 1, "", "unexpected GitHub endpoint")
                    return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

                with patch.object(self.api, "_canonical_repository", return_value="eng-cc/oasis7"), patch.object(
                    self.api.subprocess, "run", side_effect=bad_gh_run
                ):
                    with self.assertRaisesRegex(Exception, error):
                        self.api._read_approved_normative_source_from_github(root)

        with patch.object(self.api, "_canonical_repository", return_value="eng-cc/oasis7"), patch.object(
            self.api.subprocess, "run", side_effect=fake_gh_run
        ):
            parsed = self.api._read_approved_normative_source_from_github(root)
        self.assertEqual(receipt["source_head"], parsed["source_head"])
        self.assertEqual(self.api.AUTHORITY_FRAGMENT, parsed["stable_fragment"])
        self.assertEqual(self.api.AUTHORITY_CONTRACT_FRAGMENT, parsed["authority_contract_fragment"])

        for old_or_wrong in (
            body.replace("task_uid=task_a14e1a1d51a44519ab0aa94632d90df4", "task_uid=task_wrong"),
            body.replace("PR=3997", "PR=3815"),
            body.replace("MERGED into commit=4f97540c34aefca41d8bf790cbf09b3176f07de3", "MERGED into commit=" + "0" * 40),
            body.replace("Stable fragment anchor=cargo-package-scope-and-impact-scoped-verification", "Stable fragment anchor=cargo-checker-authority-upgrade"),
            body.replace("Adjacent staged-authority fragment anchor=cargo-checker-authority-upgrade", "Adjacent staged-authority fragment anchor=other"),
            body.replace("stage=normative_source", "stage=approved_planner_authority"),
            body + "; PR=3997",
        ):
            with self.subTest(receipt_body=old_or_wrong[-80:]):
                with self.assertRaisesRegex(Exception, "mismatch|identity|immutable"):
                    self.api._parse_approved_normative_comment(old_or_wrong)

    def test_live_current_task_reads_reciprocal_pr_and_exact_refs(self) -> None:
        temp, root, _predecessor_scope, _predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        task_uid = self.api.CURRENT_PLANNER_TASK_UID
        issue = {
            "number": 3999,
            "body": (
                f"task_uid: {task_uid}\n"
                "PR: https://github.com/eng-cc/oasis7/pull/4921\n"
            ),
        }
        evidence = {
            "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/3999",
            "html_url": "https://github.com/eng-cc/oasis7/issues/3999#issuecomment-5822600447",
            "body": (
                f"identity authority=UID {task_uid}, canonical worktree {root}, "
                f"branch source, trusted base {merged_commit};"
            ),
        }
        pr = {
            "number": 4921,
            "state": "open",
            "head": {
                "sha": source_head,
                "ref": "source",
                "repo": {"full_name": "eng-cc/oasis7"},
            },
            "base": {
                "sha": merged_commit,
                "ref": "main",
                "repo": {"full_name": "eng-cc/oasis7"},
            },
        }
        real_run = self.api.subprocess.run

        def fake_gh_run(command, **kwargs):
            if command[0] != "gh":
                return real_run(command, **kwargs)
            if "/issues/comments/" in command[2]:
                payload = evidence
            elif "/pulls/" in command[2]:
                payload = pr
            else:
                payload = issue
            return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

        with patch.object(self.api, "_canonical_repository", return_value="eng-cc/oasis7"), patch.object(
            self.api.subprocess, "run", side_effect=fake_gh_run
        ):
            parsed = self.api._read_current_planner_task_from_github(root)
        self.assertEqual(4921, parsed["pr_number"])
        self.assertEqual(source_head, parsed["head_sha"])
        self.assertEqual(merged_commit, parsed["base_sha"])

        for mutation in (
            lambda value: value["head"].__setitem__("sha", "0" * 40),
            lambda value: value["head"].__setitem__("repo", {"full_name": "other/repository"}),
            lambda value: value["base"].__setitem__("sha", "0" * 40),
            lambda value: value["base"].__setitem__("repo", {"full_name": "other/repository"}),
        ):
            with self.subTest(mutation=mutation):
                bad_pr = deepcopy(pr)
                mutation(bad_pr)

                def bad_gh_run(command, **kwargs):
                    if command[0] != "gh":
                        return real_run(command, **kwargs)
                    if "/issues/comments/" in command[2]:
                        payload = evidence
                    elif "/pulls/" in command[2]:
                        payload = bad_pr
                    else:
                        payload = issue
                    return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

                with patch.object(self.api, "_canonical_repository", return_value="eng-cc/oasis7"), patch.object(
                    self.api.subprocess, "run", side_effect=bad_gh_run
                ):
                    with self.assertRaisesRegex(Exception, "head|base|mismatch"):
                        self.api._read_current_planner_task_from_github(root)

        no_pr_issue = deepcopy(issue)
        no_pr_issue["body"] = f"task_uid: {task_uid}\n"

        def no_pr_gh_run(command, **kwargs):
            if command[0] != "gh":
                return real_run(command, **kwargs)
            payload = evidence if "/issues/comments/" in command[2] else no_pr_issue
            return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

        with patch.object(self.api, "_canonical_repository", return_value="eng-cc/oasis7"), patch.object(
            self.api.subprocess, "run", side_effect=no_pr_gh_run
        ):
            with self.assertRaisesRegex(Exception, "reciprocal PR"):
                self.api._read_current_planner_task_from_github(root)

    def test_current_task_identity_rejects_fabricated_match_and_wrong_head_base(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(root, predecessor_scope, predecessor_head, merged_commit, source_head)
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)

        for field, value in (("task_uid", "task_arbitrary_12345678901234567890123456789012"), ("pr_number", 9999)):
            with self.subTest(field=field):
                bad_binding = deepcopy(binding)
                bad_binding[field] = value
                with self.assertRaisesRegex(Exception, "live task truth|identity"):
                    self._plan_with_live_authority(
                        root,
                        merged_commit,
                        source_head,
                        receipt,
                        live_task,
                        profiles=("native",),
                        authority_binding=bad_binding,
                        expected_task_uid=bad_binding["task_uid"],
                        expected_pr_number=bad_binding["pr_number"],
                    )

        bad_task = deepcopy(live_task)
        bad_task["integration_base"] = predecessor_scope
        with self.assertRaisesRegex(Exception, "base|binding|authority"):
            self._plan_with_live_authority(
                root,
                merged_commit,
                source_head,
                receipt,
                bad_task,
                profiles=("native",),
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
            )

        with self.assertRaisesRegex(Exception, "head|scope|authority"):
            self._plan_with_live_authority(
                root,
                merged_commit,
                predecessor_head,
                receipt,
                live_task,
                profiles=("native",),
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
            )

    def test_approved_normative_source_rejects_each_binding_or_readback_mismatch(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(
            root, predecessor_scope, predecessor_head, merged_commit, source_head
        )
        binding = self._authority_binding(merged_commit, source_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, source_head, binding)
        digest = "sha256:" + "0" * 64
        oid = "0" * 40
        receipt_mutations = {
            "repository": "other/repository",
            "default_branch": "develop",
            "authority_path": "doc/other.md",
            "stable_fragment": "other-fragment",
            "task_uid": "task_wrong_12345678901234567890123456789012",
            "pr_number": 3816,
            "source_head": oid,
            "source_scope_base": oid,
            "merged_commit": oid,
            "merged_tree": oid,
            "authority_blob": oid,
            "authority_bytes_sha256": digest,
            "authority_size": 0,
            "stable_fragment_sha256": digest,
            "authority_contract_fragment": "wrong-authority-contract",
            "authority_contract_fragment_sha256": digest,
            "predecessor_file_sha256": digest,
            "stage": "checker",
        }
        binding_mutations = {
            "repository": "other/repository",
            "default_branch": "develop",
            "task_uid": "task_wrong_12345678901234567890123456789012",
            "pr_number": 3817,
            "integration_base": oid,
            "source_head": oid,
            "source_scope_base": oid,
            "predecessor_task_uid": "task_wrong_12345678901234567890123456789012",
            "predecessor_pr_number": 3816,
            "predecessor_file_sha256": digest,
            "predecessor_authority_digest": digest,
            "predecessor_authority_commit": oid,
            "predecessor_authority_tree": oid,
            "predecessor_authority_path": "doc/other.md",
            "predecessor_authority_blob": oid,
            "predecessor_authority_fragment": "wrong-fragment",
            "predecessor_authority_fragment_sha256": digest,
            "predecessor_authority_contract_fragment": "wrong-contract-fragment",
            "predecessor_authority_contract_fragment_sha256": digest,
            "stage": "checker",
            "write_scope": list(self.api.PLANNER_WRITE_SCOPE) + ["scripts/pm/check-cargo-package-scope"],
        }
        def assert_rejected(
            field: str,
            value: object,
            *,
            receipt_field: bool,
        ) -> None:
            with self.subTest(field=("receipt." if receipt_field else "binding.") + field):
                bad_receipt = deepcopy(receipt)
                bad_binding = deepcopy(binding)
                if receipt_field:
                    bad_receipt[field] = value
                else:
                    bad_binding[field] = value
                with self.assertRaisesRegex(Exception, "trusted|authority|binding|mismatch|candidate"):
                    self._plan_with_live_authority(
                        root,
                        merged_commit,
                        source_head,
                        receipt,
                        live_task,
                        profiles=("native",),
                        approved_normative_source=bad_receipt,
                        authority_binding=bad_binding,
                        expected_task_uid=binding["task_uid"],
                        expected_pr_number=binding["pr_number"],
                    )

        for field, value in receipt_mutations.items():
            assert_rejected(field, value, receipt_field=True)
        for field, value in binding_mutations.items():
            assert_rejected(field, value, receipt_field=False)

    def test_candidate_local_normative_source_change_is_rejected(self) -> None:
        temp, root, predecessor_scope, predecessor_head, merged_commit, source_head = self._authority_fixture()
        self.addCleanup(temp.cleanup)
        receipt = self._authority_receipt(
            root, predecessor_scope, predecessor_head, merged_commit, source_head
        )
        self._write(
            root,
            self.api.AUTHORITY_PATH,
            '<a id="cargo-checker-authority-upgrade"></a>candidate local authority\n',
        )
        candidate_head = self._commit(root, "candidate attempts normative self-modification")
        binding = self._authority_binding(merged_commit, candidate_head, receipt)
        live_task = self._current_task_identity(root, merged_commit, candidate_head, binding)
        with self.assertRaisesRegex(Exception, "scope|authority|candidate"):
            self._plan_with_live_authority(
                root,
                merged_commit,
                candidate_head,
                receipt,
                live_task,
                profiles=("native",),
                approved_normative_source=receipt,
                authority_binding=binding,
                expected_task_uid=binding["task_uid"],
                expected_pr_number=binding["pr_number"],
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
