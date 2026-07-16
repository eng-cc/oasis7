#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("review-batch-epoch.py")
TASK = "task_" + "1" * 32
HEAD = "a" * 40
EVIDENCE = "b" * 64


class ReviewBatchEpochTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.batch = self.root / "batch.json"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_script(self, *args: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), *args], text=True, capture_output=True
        )
        if ok and result.returncode != 0:
            self.fail(f"command failed: {result.stderr}")
        if not ok and result.returncode == 0:
            self.fail(f"command unexpectedly passed: {result.stdout}")
        return result

    def create(self) -> dict[str, object]:
        result = self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", EVIDENCE, "--slice", "qa_engineer=slice-qa",
            "--slice", "repository_health_engineer=slice-health", "--out", str(self.batch),
        )
        return json.loads(result.stdout)

    def ledger(self, epoch: str, *, omit_health: bool = False, duplicate: bool = False,
               wrong_head: bool = False, wrong_epoch: bool = False, bad_digest: bool = False) -> Path:
        ledger = self.root / "slice-ledger.jsonl"
        rows = []
        roles = [("qa_engineer", "slice-qa")]
        if not omit_health:
            roles.append(("repository_health_engineer", "slice-health"))
        for role, slice_id in roles:
            artifact = self.root / f"{slice_id}.md"
            artifact.write_text(f"return from {role}\n", encoding="utf-8")
            digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
            rows.append({
                "task_uid": TASK, "role": role, "slice_id": slice_id,
                "status": "completed", "head": "c" * 40 if wrong_head else HEAD,
                "epoch": "d" * 64 if wrong_epoch else epoch,
                "artifact_digest": "e" * 64 if bad_digest else digest,
                "artifacts": [str(artifact)],
            })
        if duplicate:
            rows.append(dict(rows[0]))
        ledger.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
        return ledger

    def test_create_is_deterministic_and_immutable(self) -> None:
        first = self.create()
        self.assertEqual(first["epoch"], json.loads(self.batch.read_text())["epoch"])
        failure = self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", EVIDENCE, "--slice", "qa_engineer=slice-qa",
            "--slice", "repository_health_engineer=slice-health", "--out", str(self.batch), ok=False,
        )
        self.assertIn("immutable", failure.stderr)

    def test_complete_collection_is_idempotent_only_for_same_ledger(self) -> None:
        epoch = str(self.create()["epoch"])
        ledger = self.ledger(epoch)
        first = json.loads(self.run_script("validate", "--batch", str(self.batch), "--ledger", str(ledger)).stdout)
        retry = json.loads(self.run_script("validate", "--batch", str(self.batch), "--ledger", str(ledger)).stdout)
        self.assertFalse(first["transport_retry"])
        self.assertTrue(retry["transport_retry"])
        ledger.write_text(ledger.read_text() + "\n", encoding="utf-8")
        failure = self.run_script("validate", "--batch", str(self.batch), "--ledger", str(ledger), ok=False)
        self.assertIn("different complete collection", failure.stderr)
        recreate = self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD, "--evidence-digest", EVIDENCE,
            "--slice", "qa_engineer=slice-qa", "--slice", "repository_health_engineer=slice-health",
            "--out", str(self.batch), ok=False,
        )
        self.assertIn("complete collection", recreate.stderr)

    def test_rejects_missing_duplicate_stale_epoch_and_digest(self) -> None:
        cases = [
            ({"omit_health": True}, "missing expected returns"),
            ({"duplicate": True}, "duplicate returned role"),
            ({"wrong_head": True}, "wrong head"),
            ({"wrong_epoch": True}, "wrong epoch"),
            ({"bad_digest": True}, "artifact digest mismatch"),
        ]
        for index, (options, message) in enumerate(cases):
            with self.subTest(message=message):
                batch = self.root / f"batch-{index}.json"
                self.batch = batch
                epoch = str(self.create()["epoch"])
                ledger = self.ledger(epoch, **options)
                result = self.run_script("validate", "--batch", str(batch), "--ledger", str(ledger), ok=False)
                self.assertIn(message, result.stderr)

    def test_rejects_duplicate_expected_role_or_slice_id(self) -> None:
        for slices in [
            ["qa_engineer=one", "qa_engineer=two"],
            ["qa_engineer=one", "repository_health_engineer=one"],
        ]:
            args = ["create", "--task-uid", TASK, "--head", HEAD, "--evidence-digest", EVIDENCE]
            for value in slices:
                args += ["--slice", value]
            result = self.run_script(*args, "--out", str(self.root / f"{len(slices)}-{slices[1]}.json"), ok=False)
            self.assertIn("duplicate expected", result.stderr)


if __name__ == "__main__":
    unittest.main()
