#!/usr/bin/env python3
"""Positive, negative, and wiring tests for the workflow process identity guard."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "workflow_process_identity_check", ROOT / "scripts" / "workflow-process-identity-check.py",
)
assert SPEC is not None and SPEC.loader is not None
guard = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = guard
SPEC.loader.exec_module(guard)


class WorkflowProcessIdentityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="workflow-process-identity-")
        self.root = Path(self.temp.name)
        for relative in guard.SURFACE_DIRS:
            (self.root / relative).mkdir(parents=True, exist_ok=True)

    def tearDown(self):
        self.temp.cleanup()

    def write(self, relative: str, content: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_schema_and_canonical_project_configuration_are_not_delivery_instances(self):
        self.write(
            ".codex/config.toml",
            'schema_version = 1\nproject_number = 1\nrepository = "eng-cc/oasis7"\n'
            'github_actions_app_id = 15368\n',
        )
        self.write(
            "scripts/pm/protocol.py",
            'REQUEST_SCHEMA = "oasis7-validation-request/v1"\n'
            'GITHUB_REPOSITORY = "eng-cc/oasis7"\n',
        )
        self.assertEqual([], guard.scan_repository(self.root))

    def test_concrete_process_ids_urls_uids_and_authority_hashes_are_rejected(self):
        uid = "task_" + "b" * 32
        issue = "41" + "52"
        pr = "41" + "53"
        comment = "51" + "24"
        run = "61" + "13"
        digest = "abcdef0123456789" * 2 + "abcdef01"
        self.write(
            "doc/engineering/workflow/archive.md",
            f"Historical workflow issue #{issue} and Task UID {uid}.\n"
            f"https://github.com/eng-cc/oasis7/pull/{pr}#issuecomment-{comment}\n"
            f"workflow_run_id: {run}\n"
            f"source_head_oid: {digest}\n",
        )
        self.write(
            "scripts/pm/evidence.json",
            '{"issue_number": ' + issue + ', "merge_oid": "' + ("d" * 40) + '"}\n',
        )
        self.write(
            "doc/engineering/governance/issue-comment-link.md",
            f"See https://github.com/eng-cc/oasis7/issues/{issue}#issuecomment-{comment}.\n"
            f'Escaped JSON URL: "https:\\/\\/github.com\\/eng-cc\\/oasis7\\/issues\\/{issue}#issuecomment-{comment}".\n',
        )
        self.write(
            "scripts/pm/dict-evidence.json",
            '{"issue_number": ' + issue + ', "run_id": ' + run + ', "merge_oid": "' + ("d" * 40) + '"}\n',
        )
        self.write(
            "scripts/pm/commit-authority.md",
            f"source_commit_oid: {digest}\n"
            f"merge_sha: {('e' * 40)}\n"
            f"https://github.com/eng-cc/oasis7/blob/{('f' * 40)}/scripts/pm/resolver.py\n",
        )
        self.write(
            "scripts/full-escalation-receipt.py",
            f'workflow_run_id = {run}\n'
            f'head_oid = "{("a" * 40)}"\n',
        )
        self.write(
            "scripts/pm/artifact-metadata.json",
            '{"artifact_sha256": "' + ("1a2b" * 16) + '"}\n',
        )
        findings = guard.scan_repository(self.root)
        kinds = {item.kind for item in findings}
        self.assertIn("numbered_process_reference", kinds)
        self.assertIn("github_process_url", kinds)
        self.assertIn("task_uid", kinds)
        self.assertIn("process_id_field", kinds)
        self.assertIn("execution_commit_hash", kinds)
        self.assertIn("github_commit_or_blob_ref", kinds)
        self.assertTrue(any(item.path.endswith("evidence.json") for item in findings))
        self.assertTrue(any(item.path.endswith("dict-evidence.json") for item in findings))
        self.assertTrue(any(item.path == "scripts/full-escalation-receipt.py" for item in findings))
        self.assertFalse(any(item.path.endswith("artifact-metadata.json") for item in findings))

    def test_values_are_allowed_in_structurally_isolated_test_and_fixture_surfaces(self):
        task_uid = "task_" + "a1b2c3d4" * 4
        issue_number = 71
        pr_number = 72
        run_id = 73
        fake_oid = "c1d2e3f4" * 5
        self.write(
            "scripts/pm/workflow_synthetic_fixture.test.py",
            "SYNTHETIC_FIXTURE = {\n"
            f"  'task_uid': '{task_uid}',\n"
            f"  'issue_number': {issue_number},\n"
            f"  'pr_number': {pr_number},\n"
            f"  'run_id': {run_id},\n"
            f"  'head_oid': '{fake_oid}',\n"
            "}\n",
        )
        self.write(
            "scripts/pm/fixtures/task.json",
            "{\"task_uid\": \"" + task_uid + "\", \"pr_number\": 72}\n",
        )
        self.write(
            "scripts/pm/ci_reuse_validation_test_support.py",
            "TASK_UID = \"" + task_uid + "\"\n",
        )
        self.assertEqual([], guard.scan_repository(self.root))

        self.write(
            "scripts/pm/production.py",
            f"task_uid = '{task_uid}'\npr_number = {pr_number}\n",
        )
        findings = guard.scan_repository(self.root)
        self.assertGreaterEqual(len(findings), 2)
        self.assertTrue(any(item.path.endswith("production.py") for item in findings))

    def test_pm_guard_scans_tracked_static_surfaces_and_rejects_tracked_runtime_state(self):
        task_uid = "task_" + "b7c8d9e0" * 4
        self.write(".pm/.gitignore", "issue_number: 3144\n")
        self.write(".pm/README.md", "Historical issue #3149 must stay in GitHub.\n")
        self.write(
            ".pm/cargo-package-scope-policy.json",
            '{"schema": "v1", "issue_number": 3145}\n',
        )
        self.write(".pm/registry/roles.yaml", "issue_number: 3148\n")
        self.write(".pm/templates/task.yaml", f"task_uid: {task_uid}\n")
        self.write(
            ".pm/roles/repository_health_engineer/memory/active.yaml",
            "run_id: 6142\n",
        )
        self.write(".pm/github-project-sync/task-archive.jsonl", "{}\n")
        self.write(".pm/scratch/synthetic/evidence.yaml", f"task_uid: {task_uid}\n")
        self.write(".pm/working_memory/task_synthetic.yaml", f"task_uid: {task_uid}\n")
        self.write(".pm/stage/current.yaml", "issue_number: 3147\n")
        self.write(".pm/roles/tpm/backlog/candidate.yaml", "pr_number: 3146\n")
        self.write(".pm/registry/codex-sessions.yaml", "run_id: 6141\n")
        self.write(".pm/cache/local-runtime.yaml", f"task_uid: {task_uid}\n")
        subprocess.run(["git", "-C", str(self.root), "init", "--quiet"], check=True)
        tracked_pm = [
            ".pm/.gitignore", ".pm/README.md", ".pm/cargo-package-scope-policy.json",
            ".pm/registry/roles.yaml", ".pm/templates/task.yaml",
            ".pm/roles/repository_health_engineer/memory/active.yaml",
            ".pm/github-project-sync/task-archive.jsonl",
            ".pm/scratch/synthetic/evidence.yaml",
            ".pm/working_memory/task_synthetic.yaml", ".pm/stage/current.yaml",
            ".pm/roles/tpm/backlog/candidate.yaml",
            ".pm/registry/codex-sessions.yaml",
        ]
        subprocess.run(
            ["git", "-C", str(self.root), "add", "-f", "--", *tracked_pm],
            check=True,
        )

        findings = guard.scan_repository(self.root)
        paths = {item.path for item in findings}
        kinds_by_path = {}
        for item in findings:
            kinds_by_path.setdefault(item.path, set()).add(item.kind)
        self.assertIn(".pm/README.md", paths)
        self.assertIn(".pm/.gitignore", paths)
        self.assertIn(".pm/cargo-package-scope-policy.json", paths)
        self.assertIn(".pm/registry/roles.yaml", paths)
        self.assertIn(".pm/templates/task.yaml", paths)
        self.assertIn(".pm/roles/repository_health_engineer/memory/active.yaml", paths)
        self.assertIn("tracked_pm_process_state", kinds_by_path[".pm/scratch/synthetic/evidence.yaml"])
        self.assertIn("tracked_pm_process_state", kinds_by_path[".pm/working_memory/task_synthetic.yaml"])
        self.assertNotIn("task_uid", kinds_by_path[".pm/scratch/synthetic/evidence.yaml"])
        self.assertNotIn(".pm/cache/local-runtime.yaml", paths)

    def test_production_pm_modules_cannot_import_test_or_fixture_modules(self):
        self.write(
            "scripts/pm/fixtures/canonical.py",
            "TASK_UID = 'task_" + "a" * 32 + "'\n",
        )
        self.write(
            "scripts/pm/production.py",
            "from ci_reuse_validation_test_support import TASK_UID\n"
            "import fixtures.canonical\n",
        )
        findings = guard.scan_repository(self.root)
        imports = [item for item in findings if item.kind == "test_only_import"]
        self.assertEqual(2, len(imports))
        self.assertEqual({"ci_reuse_validation_test_support", "fixtures.canonical"}, {item.value for item in imports})

    def test_current_history_and_active_workflow_docs_have_no_example_exception(self):
        issue = "31" + "49"
        self.write(
            "doc/engineering/workflow/old-notes.md",
            f"old record Issue #{issue}\n"
            "local link [identity](#41-split-source-review-and-integration-identity)\n",
        )
        finding = guard.scan_repository(self.root)
        self.assertTrue(any("3149" in item.value for item in finding))
        self.assertFalse(any("#41-split-source-review" in item.value for item in finding))

    def test_local_and_required_ci_paths_execute_guard_and_test(self):
        local_entry = (ROOT / "scripts" / "doc-governance-check.sh").read_text(encoding="utf-8")
        ci_entry = (ROOT / "scripts" / "ci-tests.sh").read_text(encoding="utf-8")
        inventory = (ROOT / "scripts" / "ci-required-capability-test-inventory.tsv").read_text(
            encoding="utf-8",
        )
        self.assertIn("workflow-process-identity-check.py", local_entry)
        self.assertIn("workflow-process-identity-check.test.py", ci_entry)
        self.assertIn("workflow-process-identity-check.test.py", inventory)


if __name__ == "__main__":
    unittest.main()
