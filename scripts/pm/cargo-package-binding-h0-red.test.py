#!/usr/bin/env python3
"""H0 RED contracts for one canonical Cargo package task/slice binding.

These tests intentionally cover the PM/helper boundary before the production
binding is implemented.  Existing Cargo scope checker tests already cover
second-package, rename, and unattributable diff rejection; this slice binds
that declared package to task truth, Issue/Project/cache projections, bounded
packets, and profile planning.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import types
import unittest
from collections import OrderedDict
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TASK_PATH = ROOT / "scripts/pm/github-project-task.py"
SYNC_PATH = ROOT / "scripts/pm/github-project-sync.py"
PACKET_PATH = ROOT / "scripts/pm/subagent-task-packet.py"
PLANNER_PATH = ROOT / "scripts/pm/cargo_package_profile_planner.py"
TASK_UID = "task_11111111111111111111111111111111"
PRIMARY = "oasis7_client_launcher"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TASK = load_module(TASK_PATH, "github_project_task_h0_binding")
SYNC = load_module(SYNC_PATH, "github_project_sync_h0_binding")
PACKET = load_module(PACKET_PATH, "subagent_task_packet_h0_binding")
PLANNER = load_module(PLANNER_PATH, "cargo_package_profile_planner_h0_binding")


def task_record(primary_package: str | None = PRIMARY) -> dict[str, object]:
    record: dict[str, object] = {
        "task_uid": TASK_UID,
        "title": "package binding contract",
        "owner_role": "repository_health_engineer",
        "module": "engineering",
        "status": "committed",
        "workflow_phase": "execution",
        "priority": "P2",
        "worktree_hint": "/tmp/oasis7-binding-worktree",
        "source_refs": ["doc/engineering/workflow/source-of-truth.md"],
        "acceptance": ["one package"],
    }
    if primary_package is not None:
        record["primary_package"] = primary_package
    return record


class TaskBindingContracts(unittest.TestCase):
    def test_task_issue_cache_round_trip_carries_one_primary_package(self) -> None:
        task = TASK.task_from_record(TASK_UID, task_record())
        self.assertEqual(PRIMARY, task.get("primary_package"), task)
        body = TASK.issue_body(task)
        self.assertIn(f"- primary_package: `{PRIMARY}`", body)
        parsed = TASK.issue_task_fields(body)
        self.assertEqual(PRIMARY, parsed.get("primary_package"), parsed)

    def test_duplicate_issue_package_identity_fails_closed(self) -> None:
        body = (
            "<!-- oasis7-pm-task -->\n"
            f"task_uid: {TASK_UID}\n"
            "\nTask metadata:\n"
            f"- primary_package: `{PRIMARY}`\n"
            "- primary_package: `oasis7_agent_api`\n"
            "Acceptance:\n"
        )
        with self.assertRaisesRegex(SystemExit, "primary package|ambiguous|duplicated"):
            TASK.issue_task_fields(body)

    def test_new_task_parser_allows_zero_business_package_tasks(self) -> None:
        argv = [
            "new-task", "/tmp/repo", "--owner-role", "repository_health_engineer",
            "--title", "package binding", "--module", "engineering",
            "--source-ref", "doc/engineering/workflow/source-of-truth.md",
        ]
        args = TASK.build_parser().parse_args(argv)
        self.assertIsNone(args.primary_package)

    def test_new_task_parser_accepts_exactly_one_primary_package(self) -> None:
        argv = [
            "new-task", "/tmp/repo", "--owner-role", "repository_health_engineer",
            "--title", "package binding", "--module", "engineering",
            "--source-ref", "doc/engineering/workflow/source-of-truth.md",
            "--primary-package", PRIMARY,
        ]
        try:
            args = TASK.build_parser().parse_args(argv)
        except SystemExit as exc:
            self.fail(f"RED: new-task parser must accept --primary-package: {exc}")
        self.assertEqual(PRIMARY, args.primary_package)

    def test_new_task_persists_primary_package_in_cache_and_issue_projection(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cargo-package-binding-task-") as raw:
            root = Path(raw)
            subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "qa@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "H0 QA"], check=True)
            (root / "README").write_text("base\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "README"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "base"], check=True)
            worktree = root / "task-worktree"
            subprocess.run(
                ["git", "-C", str(root), "worktree", "add", "-qb", "task/package-binding", str(worktree)],
                check=True,
                stdout=subprocess.DEVNULL,
            )
            args = types.SimpleNamespace(
                root=worktree,
                mapping=".pm/github-project-sync/tasks.json",
                repo="eng-cc/oasis7",
                project_owner="eng-cc",
                project_number=1,
                owner_role="repository_health_engineer",
                title="package binding",
                module="engineering",
                priority="P2",
                source_signal=None,
                source_type=None,
                severity=None,
                source_ref=["doc/engineering/workflow/source-of-truth.md"],
                doc_ref=[],
                related_prd=[],
                acceptance=["one package"],
                handoff_to=[],
                worktree_hint=str(worktree),
                loop_binding=None,
                request_key=None,
                bootstrap_base_oid=None,
                primary_package=PRIMARY,
                json=True,
            )
            with (
                mock.patch.object(TASK.uuid, "uuid4", return_value=types.SimpleNamespace(hex=TASK_UID.removeprefix("task_"))),
                mock.patch.object(TASK, "create_issue", return_value="https://github.com/eng-cc/oasis7/issues/1"),
                mock.patch.object(TASK, "add_project_item", return_value="ITEM_ID"),
                mock.patch.object(TASK, "update_project_fields", return_value=0),
                mock.patch("builtins.print"),
            ):
                self.assertEqual(0, TASK.command_new_task(args))
            mapping = TASK.load_mapping(worktree / args.mapping)
            saved = mapping["tasks"][TASK_UID]
            self.assertEqual(PRIMARY, saved.get("primary_package"), saved)
            self.assertIn(f"- primary_package: `{PRIMARY}`", TASK.issue_body(TASK.task_from_record(TASK_UID, saved)))

    def test_project_projection_carries_primary_package(self) -> None:
        values = SYNC.project_field_values(OrderedDict(task_record()))
        self.assertEqual(PRIMARY, values.get("Primary Package"), values)

    def test_legacy_task_without_package_remains_readable(self) -> None:
        legacy = TASK.task_from_record(TASK_UID, task_record(None))
        self.assertNotIn("primary_package", legacy)
        body = TASK.issue_body(legacy)
        self.assertNotIn("primary_package:", body)


def packet_create_args(*, primary_package: str | None = None) -> list[str]:
    args = [
        "create", "--task-uid", TASK_UID, "--slice-id", "qa-package-binding",
        "--role", "qa_engineer", "--slice-type", "test", "--owner-role", "repository_health_engineer",
        "--integration-owner", "tpm", "--integration-order", "1", "--packet-producer", "tpm",
        "--context-delivery-mode", "minimal_head_bound_task_packet",
        "--intended-model-configuration", "inherit current parent selection",
        "--actual-dispatched-model-reasoning", "inherited/unverified",
        "--actual-runtime-evidence-reason", "adapter inactive on this surface",
        "--role-activation", "message_assigned_adapter_inactive", "--base", "main",
        "--user-intent", "bind one package", "--work-item", "H0 package identity RED",
        "--non-goals", "no production implementation", "--acceptance-target", "focused RED tests",
        "--governance-ref", "AGENTS.md", "--governance-ref", "doc/engineering/workflow/source-of-truth.md",
        "--governance-ref", ".agents/roles/qa_engineer.md", "--scoped-ref", "scope.txt",
        "--evidence-summary", "package binding contract", "--collaboration-boundary", "tests only",
        "--write-scope", "scripts/pm/cargo-package-binding-h0-red.test.py",
        "--return-contract", "RED evidence", "--validation-command", "python3 scripts/pm/cargo-package-binding-h0-red.test.py",
        "--formal-sink", "https://github.com/eng-cc/oasis7/issues/3762",
    ]
    if primary_package is not None:
        args.extend(["--primary-package", primary_package])
    return args


class SlicePacketBindingContracts(unittest.TestCase):
    def test_slice_packet_parser_allows_zero_business_package_tasks(self) -> None:
        args = PACKET.parser().parse_args(packet_create_args())
        self.assertIsNone(args.primary_package)

    def test_slice_packet_parser_accepts_exact_primary_package(self) -> None:
        try:
            args = PACKET.parser().parse_args(packet_create_args(primary_package=PRIMARY))
        except SystemExit as exc:
            self.fail(f"RED: slice packet parser must accept --primary-package: {exc}")
        self.assertEqual(PRIMARY, args.primary_package)


class ProfileSelectionBindingContracts(unittest.TestCase):
    def _write(self, root: Path, relative: str, content: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def _fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, str, str]:
        temp = tempfile.TemporaryDirectory(prefix="cargo-package-binding-profile-")
        root = Path(temp.name)
        self._write(root, "Cargo.toml", '[workspace]\nmembers = ["crates/alpha", "crates/beta"]\nresolver = "2"\n')
        for name in ("alpha", "beta"):
            self._write(root, f"crates/{name}/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"\n')
            self._write(root, f"crates/{name}/src/lib.rs", f"pub fn {name}() -> u8 {{ 1 }}\n")
        self._write(root, ".pm/cargo-package-scope-policy.json", json.dumps({
            "schema": "oasis7-cargo-package-scope-policy/v1",
            "policy_version": 1,
            "protected_paths": [".pm/cargo-package-scope-policy.json", "scripts/pm/check-cargo-package-scope"],
        }) + "\n")
        self._write(root, "scripts/pm/check-cargo-package-scope", "#!/bin/sh\nexit 0\n")
        (root / "scripts/pm/check-cargo-package-scope").chmod(0o755)
        subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.email", "qa@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.name", "H0 QA"], check=True)
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-qm", "trusted base"], check=True)
        base = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        (root / "crates/alpha/src/lib.rs").write_text("pub fn alpha() -> u8 { 2 }\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-qm", "alpha change"], check=True)
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        return temp, root, base, head

    def test_profile_plan_binds_declared_package_and_rejects_ci_package_drift(self) -> None:
        temp, root, base, head = self._fixture()
        self.addCleanup(temp.cleanup)
        try:
            plan = PLANNER.plan_package_profiles(
                root,
                integration_base=base,
                source_head=head,
                policy_path=".pm/cargo-package-scope-policy.json",
                checker_path="scripts/pm/check-cargo-package-scope",
                profiles=["native"],
                primary_package="alpha",
            )
        except TypeError as exc:
            self.fail(f"RED: profile planner must accept primary_package binding: {exc}")
        self.assertEqual("alpha", plan.get("primary_package"), plan)
        self.assertEqual(["alpha"], plan.get("changed_packages"), plan)
        with self.assertRaisesRegex(PLANNER.PlanError, "primary package|mismatch|drift"):
            PLANNER.plan_package_profiles(
                root,
                integration_base=base,
                source_head=head,
                policy_path=".pm/cargo-package-scope-policy.json",
                checker_path="scripts/pm/check-cargo-package-scope",
                profiles=["native"],
                primary_package="beta",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
