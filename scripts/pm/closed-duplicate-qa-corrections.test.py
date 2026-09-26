#!/usr/bin/env python3
"""Focused regressions for QA corrective findings on retirement readers."""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PM = ROOT / "scripts/pm"
FIXTURE_TEST = PM / "retire-closed-duplicate-candidate.test.py"
TASK_TOOL = PM / "github-project-task.py"
AUDIT_TOOL = PM / "audit-pr-watch-issues.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


FIXTURE = load_module(FIXTURE_TEST, "retirement_fixture_for_qa_corrections")


class RetirementReaderCorrectionTests(unittest.TestCase):
    def test_preflight_does_not_create_lock_sidecar(self):
        with tempfile.TemporaryDirectory(prefix="retirement-preflight-no-sidecar-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            helper = load_module(PM / "retire-closed-duplicate-candidate.py", "retirement_preflight_no_sidecar")
            lock_path = helper.STORE.mapping_lock_path(fixture.mapping)
            self.assertFalse(lock_path.exists())

            result = FIXTURE.run_test_seam(fixture, "preflight")

            self.assertIn(result.get("status"), {"preflight_ok", "ready"})
            self.assertFalse(lock_path.exists(), "preflight must not create even a lock-sidecar filesystem artifact")

    def test_refresh_task_guards_closed_duplicate_alias_before_issue_fallback(self):
        with tempfile.TemporaryDirectory(prefix="retirement-refresh-guard-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            before = fixture.mapping.read_bytes()
            result = subprocess.run(
                [
                    sys.executable, str(TASK_TOOL), "refresh-task", str(fixture.mapping_root),
                    "--repo", "eng-cc/oasis7", "--project-owner", "eng-cc", "--project-number", "1",
                    "--mapping", str(fixture.mapping), "--task-uid", FIXTURE.CANDIDATE_UID, "--json",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=15,
            )
            calls = [json.loads(line) for line in fixture.gh_log.read_text(encoding="utf-8").splitlines()]
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"duplicate|reconcile")
            self.assertTrue(any(call[:2] == ["api", "graphql"] or call[:1] == ["api"] for call in calls))
            self.assertFalse(any(call[:2] == ["issue", "list"] for call in calls), "guard must precede cached identity fallback")
            self.assertEqual(before, fixture.mapping.read_bytes())

    def test_refresh_task_rejects_retired_uid_before_any_github_read(self):
        with tempfile.TemporaryDirectory(prefix="retirement-refresh-tombstone-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            FIXTURE.run_test_seam(fixture, "apply")
            fixture.gh_log.unlink(missing_ok=True)
            result = subprocess.run(
                [
                    sys.executable, str(TASK_TOOL), "refresh-task", str(fixture.mapping_root),
                    "--repo", "eng-cc/oasis7", "--project-owner", "eng-cc", "--project-number", "1",
                    "--mapping", str(fixture.mapping), "--task-uid", FIXTURE.CANDIDATE_UID, "--json",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=15,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"retir|tombstone")
            self.assertFalse(fixture.gh_log.exists(), "tombstoned UID refresh must stop before GitHub fallback")

    def test_append_execution_log_rejects_retired_uid_before_github_fallback_or_write(self):
        with tempfile.TemporaryDirectory(prefix="retirement-append-reader-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            FIXTURE.run_test_seam(fixture, "apply")
            fixture.gh_log.unlink(missing_ok=True)
            (fixture.bin / "gh").write_text(
                "#!/bin/sh\n"
                "python3 -c 'import json,os,sys; open(os.environ[\"RETIREMENT_GH_LOG\"],\"a\").write(json.dumps(sys.argv[1:])+\"\\n\")'\n"
                "echo '[]'\n",
                encoding="utf-8",
            )
            (fixture.bin / "gh").chmod(0o755)
            before = fixture.mapping.read_bytes()
            result = subprocess.run(
                [
                    sys.executable, str(TASK_TOOL), "append-execution-log", str(fixture.mapping_root),
                    "--repo", "eng-cc/oasis7", "--mapping", str(fixture.mapping), "--task-uid", FIXTURE.CANDIDATE_UID,
                    "--role", "repository_health_engineer", "--completed", "test", "--pending", "none",
                    "--action", "test", "--validation-command", "test", "--expected-result", "reject",
                    "--actual-result", "rejected", "--blocker-next-action", "none",
                ],
                cwd=ROOT, env=fixture.environment(), text=True, capture_output=True, check=False, timeout=15,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"retir|tombstone")
            self.assertFalse(fixture.gh_log.exists(), "retired UID must be rejected before fallback lookup or comment mutation")
            self.assertEqual(before, fixture.mapping.read_bytes())

    def test_audit_mapping_snapshot_cannot_resurrect_tombstoned_candidate(self):
        with tempfile.TemporaryDirectory(prefix="retirement-audit-writer-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            stale = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            FIXTURE.run_test_seam(fixture, "apply")
            before = fixture.mapping.read_bytes()
            audit = load_module(AUDIT_TOOL, "audit_pr_watch_retirement_test")

            error = None
            try:
                audit.save_mapping(fixture.mapping, stale)
            except ValueError as exc:
                error = exc

            self.assertIsNotNone(error, "audit writer must use durable tombstone-aware mapping merge")
            self.assertRegex(str(error).lower(), r"retir|reintroduce")
            self.assertEqual(before, fixture.mapping.read_bytes())

    def test_unregistered_candidate_cached_path_artifact_blocks_live_cli(self):
        with tempfile.TemporaryDirectory(prefix="retirement-unregistered-path-artifact-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            cached = Path(temporary) / "unregistered-cached-worktree"
            scratch = cached / ".pm" / "scratch" / FIXTURE.CANDIDATE_UID
            scratch.mkdir(parents=True)
            artifact = scratch / "bootstrap-task-snapshot.json"
            artifact.write_text(json.dumps({"task_uid": FIXTURE.CANDIDATE_UID, "fixture": "unregistered"}), encoding="utf-8")
            mapping = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            mapping["tasks"][FIXTURE.CANDIDATE_UID]["canonical_worktree"] = str(cached)
            mapping["tasks"][FIXTURE.CANDIDATE_UID]["task_branch"] = "codex/unregistered-candidate"
            fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            before = fixture.mapping.read_bytes()

            result = fixture.run_helper("preflight")

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"candidate-owned|snapshot|bootstrap")
            self.assertEqual(before, fixture.mapping.read_bytes())
            self.assertTrue(artifact.exists())

    def test_configured_remote_candidate_branch_blocks_live_cli(self):
        with tempfile.TemporaryDirectory(prefix="retirement-remote-candidate-branch-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            branch = "codex/remote-only-candidate"
            mapping = json.loads(fixture.mapping.read_text(encoding="utf-8"))
            mapping["tasks"][FIXTURE.CANDIDATE_UID]["canonical_worktree"] = str(Path(temporary) / "unregistered-remote-path")
            mapping["tasks"][FIXTURE.CANDIDATE_UID]["task_branch"] = branch
            fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            before = fixture.mapping.read_bytes()
            environment = fixture.environment()
            environment["RETIREMENT_REMOTE_REFS_JSON"] = json.dumps([{"branch": branch, "oid": "a" * 40}])

            result = subprocess.run(
                [sys.executable, str(FIXTURE.HELPER), "--mapping-root", str(fixture.mapping_root), "--task-uid", FIXTURE.CANDIDATE_UID,
                 "--disposition-comment-id", "555001", "--preflight"],
                cwd=ROOT, env=environment, text=True, capture_output=True, check=False, timeout=15,
            )

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"candidate-owned|remote|branch")
            self.assertEqual(before, fixture.mapping.read_bytes())
            git_calls = [json.loads(line) for line in fixture.git_log.read_text(encoding="utf-8").splitlines()]
            self.assertTrue(any("ls-remote" in call for call in git_calls), "configured remotes must be enumerated read-only")

    def test_cli_reads_heads_from_every_configured_remote(self):
        with tempfile.TemporaryDirectory(prefix="retirement-all-remotes-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub()
            subprocess.run(
                ["git", "-C", str(fixture.repository), "remote", "add", "upstream", "https://github.com/eng-cc/upstream-fixture.git"],
                check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            )
            environment = fixture.environment()
            environment["RETIREMENT_REMOTE_REFS_JSON"] = "[]"

            result = subprocess.run(
                [sys.executable, str(FIXTURE.HELPER), "--mapping-root", str(fixture.mapping_root), "--task-uid", FIXTURE.CANDIDATE_UID,
                 "--disposition-comment-id", "555001", "--preflight"],
                cwd=ROOT, env=environment, text=True, capture_output=True, check=False, timeout=15,
            )

            self.assertEqual(0, result.returncode, result.stderr)
            git_calls = [json.loads(line) for line in fixture.git_log.read_text(encoding="utf-8").splitlines()]
            scanned = {
                call[call.index("--heads") + 1]
                for call in git_calls
                if "ls-remote" in call and "--heads" in call
            }
            self.assertEqual({"origin", "upstream"}, scanned)

    def test_project_owner_interface_query_uses_concrete_type_fragments(self):
        with tempfile.TemporaryDirectory(prefix="retirement-project-owner-schema-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            fixture._write_paginated_live_gh_stub(validate_project_owner_schema=True)

            result = fixture.run_helper("preflight")

            self.assertEqual(0, result.returncode, result.stderr)
            calls = [json.loads(line) for line in fixture.gh_log.read_text(encoding="utf-8").splitlines()]
            project_queries = [" ".join(call) for call in calls if call[:2] == ["api", "graphql"] and "projectV2(number:" in " ".join(call)]
            self.assertTrue(project_queries)
            self.assertTrue(any("... on Organization" in query and "... on User" in query for query in project_queries))
            self.assertFalse(any("owner { login }" in query for query in project_queries))

    def test_global_sync_apply_rejects_retired_source_task_before_remote_mutation(self):
        with tempfile.TemporaryDirectory(prefix="retirement-global-sync-source-") as temporary:
            fixture = FIXTURE.RetirementFixture(Path(temporary))
            FIXTURE.run_test_seam(fixture, "apply")
            source_tasks = fixture.mapping_root / ".pm" / "tasks"
            source_tasks.mkdir(parents=True)
            uid = FIXTURE.CANDIDATE_UID
            (source_tasks / f"{uid}.yaml").write_text(
                "\n".join([
                    f"task_uid: {uid}", "title: stale duplicate candidate", "owner_role: repository_health_engineer",
                    "module: engineering", "worktree_hint: \"\"", "execution_log_path: .pm/tasks/execution.md",
                    "status: candidate", "priority: P2", "source_refs: []", "doc_refs: []", "related_prd: []",
                    "acceptance: []", "handoff_to: []", "updated_at: 2026-09-26T00:00:00Z", "",
                ]),
                encoding="utf-8",
            )
            call_log = Path(temporary) / "sync-gh-calls.jsonl"
            gh = fixture.bin / "gh"
            gh.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, sys\n"
                "args=sys.argv[1:]\n"
                "with open(os.environ['GH_CALL_LOG'],'a',encoding='utf-8') as f: f.write(json.dumps(args)+'\\n')\n"
                "def emit(v): print(json.dumps(v))\n"
                "if args[:2]==['api','graphql']:\n"
                "  query=' '.join(args)\n"
                "  if 'rateLimit' in query: emit({'data':{'rateLimit':{'remaining':5000,'resetAt':'2099-01-01T00:00:00Z'}}})\n"
                "  else: emit({'data':{}})\n"
                "elif args[:2]==['project','view']: emit({'id':'PVT_fixture'})\n"
                "elif args[:2]==['project','field-list']: emit({'fields':[]})\n"
                "elif args[:2]==['issue','create']: print('https://github.com/eng-cc/oasis7/issues/900099')\n"
                "elif args[:2]==['project','item-add']: emit({'id':'PVTI_created'})\n"
                "elif args[:2]==['project','item-edit']: emit({})\n"
                "else: print('unexpected stub command '+json.dumps(args),file=sys.stderr); sys.exit(9)\n",
                encoding="utf-8",
            )
            gh.chmod(0o755)
            environment = fixture.environment()
            environment["GH_CALL_LOG"] = str(call_log)
            before = fixture.mapping.read_bytes()
            result = subprocess.run(
                [
                    sys.executable, str(PM / "github-project-sync.py"), str(fixture.mapping_root),
                    "--repo", "eng-cc/oasis7", "--project-owner", "eng-cc", "--project-number", "1",
                    "--mapping", str(fixture.mapping), "--global-maintenance", "--apply", "--json",
                ],
                cwd=ROOT, env=environment, text=True, capture_output=True, check=False, timeout=20,
            )
            calls = [json.loads(line) for line in call_log.read_text(encoding="utf-8").splitlines()]
            mutating = [
                call for call in calls
                if call[:2] in (["issue", "create"], ["project", "item-add"], ["project", "item-edit"])
                or (call[:2] == ["api", "graphql"] and any(token in " ".join(call) for token in ("mutation", "addProjectV2Item", "updateProjectV2ItemFieldValue")))
            ]
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex((result.stderr + result.stdout).lower(), r"retir|tombstone")
            self.assertEqual([], mutating, f"broad sync attempted remote mutation for a retired UID: {mutating}")
            self.assertEqual(before, fixture.mapping.read_bytes())


if __name__ == "__main__":
    unittest.main(verbosity=2)
