#!/usr/bin/env python3
"""Contract tests for archived review artifact resolution.

The archive mapping is supplied only by the canonical recovery consumer. These
unit tests exercise the shared validator boundary; they do not claim to prove
the consumer's manifest-to-origin mapping or recovery authority.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "pm" / "validate-review-provenance.py"
SPEC = importlib.util.spec_from_file_location("validate_review_provenance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)
TASK_UID = "task_33f5586aac054487998a3584ac1d8143"
ROLE = "repository_health_engineer"
HEAD = "a" * 40
SLICE_ID = "11111111-1111-4111-8111-111111111111"


def _ledger_entry(artifact_path: Path, artifact_bytes: bytes, *, head: str = HEAD) -> dict:
    return {
        "task_uid": TASK_UID,
        "role": ROLE,
        "status": "completed",
        "head": head,
        "slice_id": SLICE_ID,
        "activation": "message-assigned",
        "context_delivery": "full-history",
        "actual_runtime": "inherit current parent selection",
        "artifact_digest": hashlib.sha256(artifact_bytes).hexdigest(),
        "scope_verdict": "approved",
        "risk_verdict": "approved",
        "findings": "no_findings",
        "residual_risk": "fixture only",
        "artifacts": [str(artifact_path)],
    }


class ArchivedReviewProvenanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="archived-review-provenance-")
        self.base = Path(self.temp.name)
        self.archive_root = self.base / "archive"
        self.archive_root.mkdir()
        self.ledger_rel = Path(".pm/scratch") / TASK_UID / "slice-ledger.jsonl"
        ledger_path = self.archive_root / self.ledger_rel
        ledger_path.parent.mkdir(parents=True)
        self.origin_root = self.base / "removed-worktree"
        self.origin_artifact = self.origin_root / ".pm/scratch" / TASK_UID / "role-return.md"
        self.payload = b"review artifact bytes\n"
        self.archive_artifact = self.archive_root / "artifacts" / ".pm/scratch" / TASK_UID / "role-return.md"
        self.archive_artifact.parent.mkdir(parents=True)
        self.archive_artifact.write_bytes(self.payload)
        self.write_ledger()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_ledger(self, *, artifact: Path | None = None, payload: bytes | None = None, head: str = HEAD) -> None:
        entry = _ledger_entry(artifact or self.origin_artifact, payload if payload is not None else self.payload, head=head)
        ledger_path = self.archive_root / self.ledger_rel
        ledger_path.write_text(json.dumps(entry, sort_keys=True) + "\n", encoding="utf-8")

    def args(self, *, roles: str = ROLE, source_head: str = HEAD) -> argparse.Namespace:
        return argparse.Namespace(
            root=self.archive_root,
            task_uid=TASK_UID,
            ledger=str(self.ledger_rel),
            roles=roles,
            source_head=source_head,
            mode="human-operated",
        )

    def call_validator(self, args: argparse.Namespace, resolver=None) -> int:
        validate_ledger = getattr(VALIDATOR, "validate_ledger", None)
        self.assertTrue(
            callable(validate_ledger),
            "RED: validate-review-provenance.py must expose validate_ledger(args, parser, *, archived_artifact_resolver=None)",
        )
        parser = argparse.ArgumentParser(prog="validate-review-provenance-test")
        return validate_ledger(args, parser, archived_artifact_resolver=resolver)

    def expect_rejected(self, resolver, *, args: argparse.Namespace | None = None) -> None:
        with self.assertRaises((SystemExit, ValueError, OSError)) as caught:
            self.call_validator(args or self.args(), resolver)
        if isinstance(caught.exception, SystemExit):
            self.assertNotEqual(caught.exception.code, 0)

    def test_live_cli_baseline_remains_valid_without_archive_options(self) -> None:
        repo = self.base / "live-repo"
        artifact = repo / ".pm/scratch" / TASK_UID / "role-return.md"
        artifact.parent.mkdir(parents=True)
        artifact.write_bytes(self.payload)
        ledger = repo / ".pm/scratch" / TASK_UID / "slice-ledger.jsonl"
        ledger.parent.mkdir(parents=True, exist_ok=True)
        ledger.write_text(json.dumps(_ledger_entry(artifact.relative_to(repo), self.payload), sort_keys=True) + "\n", encoding="utf-8")
        run = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(repo), "--task-uid", TASK_UID,
             "--ledger", str(ledger.relative_to(repo)), "--roles", ROLE, "--source-head", HEAD],
            text=True, capture_output=True, check=False,
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn('"status": "passed"', run.stdout)
        self.assertNotIn("archive", run.stdout.lower())

    def test_live_cli_has_no_caller_selectable_archive_authority(self) -> None:
        run = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.archive_root), "--task-uid", TASK_UID,
             "--ledger", str(self.ledger_rel), "--roles", ROLE, "--source-head", HEAD,
             "--archive-root", str(self.archive_root)],
            text=True, capture_output=True, check=False,
        )
        self.assertNotEqual(run.returncode, 0)
        self.assertIn("unrecognized arguments", run.stderr)

    def test_removed_checkout_artifact_resolves_from_archive_and_keeps_digest(self) -> None:
        self.assertFalse(self.origin_root.exists(), "fixture must model a removed source checkout")
        calls: list[tuple[str, Path]] = []
        def resolver(raw: str, archive_root: Path) -> Path:
            calls.append((raw, archive_root))
            self.assertEqual(raw, str(self.origin_artifact))
            return archive_root / "artifacts" / ".pm/scratch" / TASK_UID / "role-return.md"
        self.assertEqual(self.call_validator(self.args(), resolver), 0)
        self.assertEqual(calls, [(str(self.origin_artifact), self.archive_root)])

    def test_rejects_foreign_origin_when_consumer_resolver_rejects_it(self) -> None:
        foreign = self.base / "another-worktree" / ".pm/scratch" / TASK_UID / "role-return.md"
        self.write_ledger(artifact=foreign)
        def resolver(raw: str, archive_root: Path) -> Path:
            raise ValueError("artifact path is outside authenticated bootstrap origin")
        self.expect_rejected(resolver)

    def test_rejects_resolver_target_outside_archive_root(self) -> None:
        outside = self.base / "outside.md"
        outside.write_bytes(self.payload)
        self.expect_rejected(lambda raw, archive_root: outside)

    def test_rejects_traversal_resolver_target(self) -> None:
        outside = self.base / "outside-traversal.md"
        outside.write_bytes(self.payload)
        traversal = self.archive_root / "artifacts" / ".." / ".." / "outside-traversal.md"
        self.expect_rejected(lambda raw, archive_root: traversal)

    def test_rejects_symlink_archive_member_even_when_target_is_inside(self) -> None:
        link = self.archive_root / "artifacts" / "linked-role-return.md"
        link.symlink_to(self.archive_artifact)
        self.expect_rejected(lambda raw, archive_root: link)

    def test_rejects_missing_archive_member(self) -> None:
        missing = self.archive_root / "artifacts" / "not-present.md"
        self.expect_rejected(lambda raw, archive_root: missing)

    def test_surfaces_unlisted_or_duplicate_mapping_rejection_from_consumer(self) -> None:
        # The callback is the recovery consumer's authority boundary. This only
        # verifies the shared validator does not suppress its fail-closed error;
        # public consumer tests must prove the manifest mapping itself.
        for diagnostic in ("unlisted archive member", "duplicate origin mapping"):
            with self.subTest(diagnostic=diagnostic):
                def resolver(raw: str, archive_root: Path) -> Path:
                    raise ValueError(diagnostic)
                self.expect_rejected(resolver)

    def test_rejects_archived_bytes_that_do_not_match_ledger_digest(self) -> None:
        self.archive_artifact.write_bytes(b"tampered archive bytes\n")
        self.expect_rejected(lambda raw, archive_root: self.archive_artifact)

    def test_mandatory_role_and_source_head_checks_remain_active(self) -> None:
        resolver = lambda raw, archive_root: self.archive_artifact
        self.expect_rejected(resolver, args=self.args(roles=f"{ROLE},qa_engineer"))
        self.expect_rejected(resolver, args=self.args(source_head="b" * 40))


if __name__ == "__main__":
    unittest.main(verbosity=2)
