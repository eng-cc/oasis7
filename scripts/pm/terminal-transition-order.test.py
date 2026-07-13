#!/usr/bin/env python3
"""RED contract for crash-safe, single-authority terminal transitions."""
from __future__ import annotations

import re
import os
import importlib.util
import inspect
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TASK = ROOT / "scripts/pm/github-project-task.py"
CLOSEOUT = ROOT / "scripts/pm/task-closeout.sh"
MAIN_SYNC = ROOT / "scripts/pm/post-merge-main-sync.sh"
CLEANUP = ROOT / "scripts/pm/post-merge-cleanup.sh"
FINALIZE = ROOT / "scripts/pm/post-merge-finalize.py"
PR_WATCH_AUDIT = ROOT / "scripts/pm/audit-pr-watch-issues.py"
SOURCE = ROOT / "doc/engineering/workflow/source-of-truth.md"
SYNC = ROOT / "scripts/pm/github-project-sync.py"


def function(text: str, name: str) -> str:
    match = re.search(rf"(?ms)^def {re.escape(name)}\b.*?(?=^def |\Z)", text)
    if not match:
        raise AssertionError(f"missing function {name}")
    return match.group(0)


class TerminalTransitionOrder(unittest.TestCase):
    def test_done_closeout_is_intermediate_and_does_not_close_issue(self) -> None:
        closeout = function(TASK.read_text(encoding="utf-8"), "command_closeout_task")
        self.assertIn('"Workflow Phase": "task_done"', closeout)
        self.assertNotIn('"Workflow Phase": "post_merge_done"', closeout)
        self.assertNotRegex(closeout, r"gh[\"'],\s*[\"']issue[\"'],\s*[\"']close")
        self.assertNotIn('run_text(["gh", "issue", "close"', closeout)

    def test_main_sync_persists_intermediate_main_sync_phase(self) -> None:
        text = MAIN_SYNC.read_text(encoding="utf-8")
        self.assertIn('"workflow_phase":"main_sync"', re.sub(r"\s+", "", text).lower())
        self.assertRegex(text, r"github-project-task\.py[^\n]+(?:advance|transition|set-phase)")
        self.assertLess(text.index("oasis7_main_sync"), text.index("main_sync"))

    def test_cleanup_receipt_precedes_the_only_terminal_finalizer(self) -> None:
        cleanup = CLEANUP.read_text(encoding="utf-8")
        self.assertIn("--terminal-receipt-output", cleanup)
        self.assertIn("oasis7_terminal_cleanup", cleanup)
        self.assertIn("post-merge-finalize.py", cleanup)
        self.assertLess(cleanup.index("oasis7_terminal_cleanup"), cleanup.index("post-merge-finalize.py"))
        all_writers = []
        for path in (TASK, MAIN_SYNC, CLEANUP, PR_WATCH_AUDIT, FINALIZE):
            if path.exists():
                text = path.read_text(encoding="utf-8")
                if "post_merge_done" in text or '"issue", "close"' in text:
                    all_writers.append(path.name)
        self.assertEqual(["post-merge-finalize.py"], all_writers)

    def test_finalizer_is_receipt_bound_and_idempotent_after_crash(self) -> None:
        self.assertTrue(FINALIZE.is_file(), "missing terminal finalizer")
        text = FINALIZE.read_text(encoding="utf-8")
        for marker in (
            "--terminal-receipt", "post_merge_done", "already_finalized",
            "task_uid", "repository", "issue_number", "pr_number",
        ):
            self.assertIn(marker, text)
        self.assertRegex(text, r"(?is)already_finalized.{0,500}(return 0|SystemExit\(0\))")
        self.assertRegex(text, r"(?is)(mismatch|disagrees).{0,500}(fail|SystemExit|raise)")

    def test_terminal_store_cannot_be_imported_as_caller_authority(self) -> None:
        """A caller must not be able to mint terminal state by importing a helper."""
        store = ROOT / "scripts/pm/terminal-finalization-store.py"
        self.assertFalse(
            store.exists(),
            "terminal commit authority must remain inside the validated finalizer, not an importable module",
        )

    def test_import_attempt_cannot_forge_terminal_mapping_in_safe_fixture(self) -> None:
        store = ROOT / "scripts/pm/terminal-finalization-store.py"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mapping_path = root / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            uid = "task_11111111111111111111111111111111"
            mapping_path.write_text(
                '{"tasks":{"%s":{"task_uid":"%s","workflow_phase":"main_sync",'
                '"repository":"fixture/repo","issue_number":1}}}\n' % (uid, uid),
                encoding="utf-8",
            )
            before = mapping_path.read_bytes()
            bin_dir = root / "bin"
            bin_dir.mkdir()
            gh = bin_dir / "gh"
            gh.write_text("#!/bin/sh\nprintf '%s\\n' https://example.invalid/comment/1\n", encoding="utf-8")
            gh.chmod(0o755)
            exploit = """
import importlib.util, json, pathlib, sys
path=pathlib.Path(sys.argv[1]); root=pathlib.Path(sys.argv[2]); uid=sys.argv[3]
spec=importlib.util.spec_from_file_location('caller_terminal_store',path)
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
module.commit_terminal(root,uid,{'receipt_type':'forged'},'0'*64)
"""
            result = subprocess.run(
                [sys.executable, "-c", exploit, str(store), str(root), uid],
                text=True,
                capture_output=True,
                env={**os.environ, "PATH": f"{bin_dir}:{os.environ.get('PATH', '')}"},
            )
            self.assertNotEqual(0, result.returncode, "direct import/call forged terminal state")
            self.assertEqual(before, mapping_path.read_bytes(), "failed import attempt mutated task truth")

    def test_imported_finalizer_write_helpers_revalidate_authority_inputs(self) -> None:
        """Private naming is not authority: every writer must revalidate from durable inputs."""
        spec = importlib.util.spec_from_file_location("caller_imported_finalizer", FINALIZE)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        writers = [
            (name, candidate)
            for name, candidate in inspect.getmembers(module, inspect.isfunction)
            if candidate.__module__ == module.__name__ and name == "_write_terminal"
        ]
        self.assertTrue(writers, "finalizer must expose an identifiable terminal write boundary")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mapping_path = root / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            uid = "task_11111111111111111111111111111111"
            mapping_path.write_text(
                '{"tasks":{"%s":{"task_uid":"%s","workflow_phase":"main_sync",'
                '"repository":"fixture/repo","issue_number":1}}}\n' % (uid, uid),
                encoding="utf-8",
            )
            forged_path = root / "forged-terminal.json"
            forged_path.write_text(
                '{"receipt_type":"oasis7_terminal_cleanup","issuer":"caller",'
                '"task_uid":"%s","repository":"fixture/repo","issue_number":1}\n' % uid,
                encoding="utf-8",
            )
            bin_dir = root / "bin"
            bin_dir.mkdir()
            gh = bin_dir / "gh"
            gh.write_text("#!/bin/sh\nprintf '%s\\n' https://example.invalid/comment/1\n", encoding="utf-8")
            gh.chmod(0o755)
            previous_path = os.environ.get("PATH", "")
            os.environ["PATH"] = f"{bin_dir}:{previous_path}"
            try:
                for name, writer in writers:
                    with self.subTest(writer=name):
                        params = inspect.signature(writer).parameters
                        before = mapping_path.read_bytes()
                        rejected = False
                        try:
                            if "terminal_receipt_path" in params:
                                writer(root, uid, forged_path)
                            else:
                                writer(root, uid, {"receipt_type": "forged"}, "0" * 64)
                        except BaseException:
                            rejected = True
                        self.assertEqual(before, mapping_path.read_bytes(), "forged helper call mutated mapping")
                        self.assertTrue(rejected, "forged helper call was accepted")
                        self.assertIn("root", params)
                        self.assertIn("task_uid", params)
                        self.assertIn(
                            "terminal_receipt_path", params,
                            "terminal writer must reload and validate the durable receipt path itself",
                        )
                        self.assertNotIn("digest", params)
                        self.assertNotIn("receipt", params)
            finally:
                os.environ["PATH"] = previous_path

    def test_full_terminal_validation_precedes_already_finalized_and_issue_effects(self) -> None:
        text = FINALIZE.read_text(encoding="utf-8")
        validation_markers = (
            'terminal.get("receipt_type")',
            'terminal.get("issuer")',
            'terminal.get("merge_receipt_sha256")',
            'terminal.get("main_sync_receipt_sha256")',
        )
        already = text.index("already_finalized=")
        issue_effect = text.index('["gh","issue","close"')
        for marker in validation_markers:
            with self.subTest(marker=marker):
                self.assertLess(text.index(marker), already)
                self.assertLess(text.index(marker), issue_effect)

    def test_terminal_validation_and_write_share_one_exclusive_lock_snapshot(self) -> None:
        """A mapping drift at lock acquisition must not inherit pre-lock validation."""
        spec = importlib.util.spec_from_file_location("race_imported_finalizer", FINALIZE)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        writer = module._write_terminal
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
            mapping_path = root / ".pm/github-project-sync/tasks.json"
            mapping_path.parent.mkdir(parents=True)
            canonical_worktree = root / "task-worktree"
            canonical_worktree.mkdir()
            uid = "task_11111111111111111111111111111111"
            original = {
                "tasks": {uid: {
                    "task_uid": uid,
                    "workflow_phase": "main_sync",
                    "repository": "fixture/repo",
                    "issue_number": 11,
                    "pr_number": 22,
                    "canonical_worktree": str(canonical_worktree),
                    "merge_receipt": {"state": "MERGED"},
                    "phase_receipts": {"main_sync": {"receipt_type": "oasis7_main_sync"}},
                }}
            }
            mapping_path.write_text(json.dumps(original) + "\n", encoding="utf-8")
            receipt_root = subprocess.check_output([
                sys.executable, str(ROOT / "scripts/pm/canonical-receipt-root.py"),
                "--default-worktree", str(root), "--task-uid", uid, "--create",
            ], text=True).strip()
            receipt_path = Path(receipt_root) / "terminal-cleanup-receipt.json"
            receipt_path.write_text(json.dumps({
                "receipt_type": "oasis7_terminal_cleanup",
                "issuer": "post-merge-cleanup",
                "task_uid": uid,
                "repository": "fixture/repo",
                "issue_number": 11,
                "pr_number": 22,
            }) + "\n", encoding="utf-8")

            drifted = json.loads(json.dumps(original))
            drifted_record = drifted["tasks"][uid]
            drifted_record["repository"] = "fixture/drifted"
            drifted_record["issue_number"] = 12
            drifted_record["pr_number"] = 23
            drifted_record["merge_receipt"] = {"state": "MERGED", "epoch": "changed"}
            drift_bytes = (json.dumps(drifted, sort_keys=True) + "\n").encode()
            bin_dir = Path(directory) / "bin"
            bin_dir.mkdir()
            gh_log = Path(directory) / "gh.log"
            gh = bin_dir / "gh"
            gh.write_text(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GH_LOG\"\nprintf '%s\\n' '{}'\n",
                encoding="utf-8",
            )
            gh.chmod(0o755)

            real_flock = module.fcntl.flock
            swapped = False
            def swap_then_lock(handle, operation):
                nonlocal swapped
                if not swapped:
                    mapping_path.write_bytes(drift_bytes)
                    swapped = True
                return real_flock(handle, operation)
            module.fcntl.flock = swap_then_lock
            old_path = os.environ.get("PATH", "")
            old_log = os.environ.get("GH_LOG")
            os.environ["PATH"] = f"{bin_dir}:{old_path}"
            os.environ["GH_LOG"] = str(gh_log)
            rejected = False
            try:
                try:
                    writer(root, uid, receipt_path)
                except BaseException:
                    rejected = True
            finally:
                module.fcntl.flock = real_flock
                os.environ["PATH"] = old_path
                if old_log is None:
                    os.environ.pop("GH_LOG", None)
                else:
                    os.environ["GH_LOG"] = old_log
            self.assertTrue(swapped, "race hook did not run at lock acquisition")
            self.assertEqual(drift_bytes, mapping_path.read_bytes(), "drifted snapshot received terminal mutation")
            self.assertTrue(rejected, "writer accepted identity/digest drift across lock acquisition")
            self.assertFalse(gh_log.exists() and gh_log.read_text(encoding="utf-8").strip(), "issue effect escaped rejected snapshot")

    def test_terminal_runbook_names_task_and_default_worktree_roots(self) -> None:
        text = SOURCE.read_text(encoding="utf-8")
        runbook = re.search(r"(?ms)^### Terminal runbook\s*$\n(.*?)(?=^### |^## |\Z)", text)
        self.assertIsNotNone(runbook)
        body = runbook.group(1)
        self.assertIn("<canonical-task-worktree>", body)
        self.assertIn("<canonical-default-worktree>", body)
        self.assertRegex(body, r"cd <canonical-task-worktree>[\s\S]{0,500}task-closeout\.sh")
        self.assertRegex(body, r"cd <canonical-default-worktree>[\s\S]{0,500}post-merge-main-sync\.sh")
        self.assertIn("--terminal-receipt-output", body)

    def test_missing_merge_receipt_guidance_names_only_receipt_flag(self) -> None:
        text = CLOSEOUT.read_text(encoding="utf-8")
        receipt_errors = [
            line for line in text.splitlines()
            if "receipt" in line.lower() and ("missing" in line.lower() or "requires" in line.lower())
        ]
        self.assertTrue(receipt_errors)
        joined = "\n".join(receipt_errors)
        self.assertIn("--pr-receipt", joined)
        for forbidden in ("--pr-state", "--merged", "PR_STATE=MERGED"):
            self.assertNotIn(forbidden, joined)

    def test_canonical_phase_enum_and_done_default_are_nonterminal(self) -> None:
        source = SOURCE.read_text(encoding="utf-8")
        phase_row = next(line for line in source.splitlines() if line.startswith("| `Workflow Phase`"))
        for phase in ("blocked", "task_done", "main_sync", "post_merge_done"):
            self.assertIn(f"`{phase}`", phase_row)
        sync = SYNC.read_text(encoding="utf-8")
        phase_function = function(sync, "workflow_phase_for")
        self.assertRegex(phase_function, r"done[^\n]{0,120}task_done|task_done[^\n]{0,120}done")
        self.assertNotRegex(phase_function, r"done[^\n]{0,120}post_merge_done")

    def test_generic_set_phase_cannot_mint_terminal_authority(self) -> None:
        task = TASK.read_text(encoding="utf-8")
        set_phase = function(task, "command_set_phase")
        self.assertNotIn('args.phase == "post_merge_done"', set_phase)
        self.assertRegex(set_phase, r"ALLOWED_PHASE_TRANSITIONS|allowed_transition")
        self.assertRegex(set_phase, r"RECEIPT_SCHEMAS|receipt_schema")
        parser = function(task, "build_parser")
        self.assertRegex(parser, r"--phase[^\n]+choices=")
        self.assertNotRegex(parser, r"choices=.*post_merge_done")

    def test_public_task_cli_cannot_bypass_the_receipt_bound_finalizer(self) -> None:
        task = TASK.read_text(encoding="utf-8")
        finalizer = FINALIZE.read_text(encoding="utf-8")
        self.assertNotIn("finalize-phase", task)
        self.assertNotRegex(task, r"(?m)^def command_finalize_phase\b")
        self.assertNotIn('"_".join(("post", "merge", "done"))', task)
        self.assertNotIn("finalize-phase", finalizer)

    def test_finalizer_verifies_complete_digest_chain(self) -> None:
        text = FINALIZE.read_text(encoding="utf-8")
        for marker in ("merge_receipt_sha256", "main_sync_receipt_sha256", "hashlib.sha256"):
            self.assertIn(marker, text)
        self.assertRegex(text, r"(?is)merge_receipt_sha256.{0,800}(mismatch|disagrees)")
        self.assertRegex(text, r"(?is)main_sync_receipt_sha256.{0,800}(mismatch|disagrees)")

    def test_cleanup_journals_intent_before_effects_and_retries_after_crash(self) -> None:
        text = CLEANUP.read_text(encoding="utf-8")
        for marker in (
            "oasis7_cleanup_intent", "worktree_removed", "branch_deleted",
            "terminal_receipt_committed",
        ):
            self.assertIn(marker, text)
        self.assertLess(text.index("oasis7_cleanup_intent"), text.index('worktree remove "$WORKTREE"'))
        self.assertRegex(text, r"(?is)worktree_removed.{0,1000}(already|missing|retry)")
        self.assertRegex(text, r"(?is)branch_deleted.{0,1000}(already|missing|retry)")

    def test_production_cleanup_has_no_fixture_or_fault_environment_channel(self) -> None:
        text = CLEANUP.read_text(encoding="utf-8")
        self.assertNotRegex(text, r"TPM_CLEANUP_(?:FIXTURE|FAULT)")
        self.assertNotRegex(text, r"(?i)(fixture-only kill|fault injection)")

    def test_runbook_uses_one_absolute_durable_receipt_root_across_worktrees(self) -> None:
        text = SOURCE.read_text(encoding="utf-8")
        runbook = re.search(r"(?ms)^### Terminal runbook\s*$\n(.*?)(?=^### |^## |\Z)", text)
        self.assertIsNotNone(runbook)
        body = runbook.group(1)
        self.assertIn("canonical-receipt-root.py", body)
        self.assertNotRegex(body, r"(?m)(?:>|--pr-receipt|--receipt-output|--main-sync-receipt)\s+\.pm/scratch/")
        self.assertRegex(
            body,
            r"(?is)cd <canonical-default-worktree>.{0,500}refresh-task-cache\.sh.{0,500}post-merge-main-sync\.sh",
        )
        self.assertRegex(body, r'(?m)^RECEIPT_ROOT="\$\(python3 scripts/pm/canonical-receipt-root\.py \\')
        self.assertRegex(body, r'--default-worktree <canonical-default-worktree>.*\n\s*--task-uid <TASK-UID> --create\)"')
        helpers = re.findall(
            r"(?:python3\s+)?(?:\./)?scripts/pm/(pr-merge-receipt\.py|task-closeout\.sh|"
            r"refresh-task-cache\.sh|post-merge-main-sync\.sh|post-merge-cleanup\.sh|"
            r"post-merge-finalize\.py)", body)
        self.assertEqual(6, len(helpers), helpers)
        self.assertRegex(body, r"(?i)all six (?:commands|transitions|steps)")

    def test_terminal_runbook_numbers_six_actions_with_readback_and_resume(self) -> None:
        text = SOURCE.read_text(encoding="utf-8")
        match = re.search(r"(?ms)^### Terminal runbook\s*$\n(.*?)(?=^### |^## |\Z)", text)
        self.assertIsNotNone(match)
        body = match.group(1)
        numbered = re.findall(r"(?m)^([1-6])[.)]\s+", body)
        self.assertEqual(["1", "2", "3", "4", "5", "6"], numbered)
        for number in numbered:
            action = re.search(rf"(?ms)^{number}[.)]\s+.*?(?=^[1-6][.)]\s+|\Z)", body).group(0)
            self.assertRegex(action, r"(?i)readback")
            self.assertRegex(action, r"(?i)resume|retry")

    def test_only_finalizer_has_terminal_semantics_in_current_source(self) -> None:
        text = SOURCE.read_text(encoding="utf-8").split("## 7. Change Log", 1)[0]
        paragraphs = [re.sub(r"\s+", " ", item).strip() for item in re.split(r"\n\s*\n", text)]
        conflicting = [
            paragraph for paragraph in paragraphs
            if "task-closeout" in paragraph.lower()
            and re.search(
                r"(?i)(post_merge_done|close(?:s|d|ing)? (?:the )?(?:task )?issue|issue close)",
                paragraph,
            )
        ]
        self.assertEqual([], conflicting)
        self.assertRegex(text, r"(?is)post-merge-finalize\.py.{0,300}(only|唯一).{0,300}(post_merge_done|close)")

    def test_terminal_receipt_paths_are_absolute_and_outside_task_worktree(self) -> None:
        surfaces = {
            MAIN_SYNC: ("RECEIPT_OUTPUT", "canonical task worktree"),
            CLEANUP: ("TERMINAL_RECEIPT_OUTPUT", "canonical task worktree"),
            FINALIZE: ("terminal_path", "canonical task worktree"),
        }
        for path, (variable, boundary) in surfaces.items():
            text = path.read_text(encoding="utf-8")
            with self.subTest(path=path):
                self.assertRegex(text, rf"(?is){variable}.{{0,800}}is_absolute")
                self.assertRegex(text, rf"(?is){variable}.{{0,1200}}(relative_to|commonpath).{{0,500}}(canonical_worktree|worktree)")
                self.assertIn(boundary, text.lower())

    def test_path_validation_precedes_every_terminal_effect(self) -> None:
        sync = MAIN_SYNC.read_text(encoding="utf-8")
        cleanup = CLEANUP.read_text(encoding="utf-8")
        finalizer = FINALIZE.read_text(encoding="utf-8")
        self.assertIn("is_absolute", sync)
        self.assertIn("is_absolute", cleanup)
        self.assertIn("is_absolute", finalizer)
        self.assertLess(sync.index("is_absolute"), sync.index('fetch --quiet'))
        self.assertLess(cleanup.index("is_absolute"), cleanup.index('worktree remove "$WORKTREE"'))
        self.assertLess(finalizer.index("is_absolute"), finalizer.index('"issue","close"'))


if __name__ == "__main__":
    unittest.main(verbosity=2)
