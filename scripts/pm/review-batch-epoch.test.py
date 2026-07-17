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
QA_SLICE = "11111111-1111-4111-8111-111111111111"
HEALTH_SLICE = "22222222-2222-4222-8222-222222222222"


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
            "--evidence-digest", EVIDENCE, "--slice", f"qa_engineer={QA_SLICE}",
            "--slice", f"repository_health_engineer={HEALTH_SLICE}", "--out", str(self.batch),
        )
        return json.loads(result.stdout)

    def ledger(self, epoch: str, *, omit_health: bool = False, duplicate: bool = False,
               wrong_head: bool = False, wrong_epoch: bool = False, bad_digest: bool = False,
               invalid_return: bool = False) -> Path:
        ledger = self.root / "slice-ledger.jsonl"
        rows = []
        roles = [("qa_engineer", QA_SLICE)]
        if not omit_health:
            roles.append(("repository_health_engineer", HEALTH_SLICE))
        for role, slice_id in roles:
            artifact = self.root / f"{slice_id}.json"
            returned = {
                "task_uid": TASK, "role": role, "slice_id": slice_id,
                "status": "completed", "head": "c" * 40 if wrong_head else HEAD,
                "epoch": "d" * 64 if wrong_epoch else epoch,
                "disposition": "no_findings", "findings": [], "residual_risk": "none",
            }
            if invalid_return:
                returned.pop("residual_risk")
            artifact.write_text(json.dumps(returned) + "\n", encoding="utf-8")
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
            "--evidence-digest", EVIDENCE, "--slice", f"qa_engineer={QA_SLICE}",
            "--slice", f"repository_health_engineer={HEALTH_SLICE}", "--out", str(self.batch), ok=False,
        )
        self.assertIn("immutable", failure.stderr)

    def test_complete_collection_is_idempotent_only_for_same_ledger(self) -> None:
        epoch = str(self.create()["epoch"])
        ledger = self.ledger(epoch)
        first = json.loads(self.run_script("collect", "--batch", str(self.batch), "--ledger", str(ledger)).stdout)
        retry = json.loads(self.run_script("collect", "--batch", str(self.batch), "--ledger", str(ledger)).stdout)
        self.assertFalse(first["transport_retry"])
        self.assertTrue(retry["transport_retry"])
        ledger.write_text(ledger.read_text() + "\n", encoding="utf-8")
        failure = self.run_script("collect", "--batch", str(self.batch), "--ledger", str(ledger), ok=False)
        self.assertIn("different complete collection", failure.stderr)
        recreate = self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD, "--evidence-digest", EVIDENCE,
            "--slice", f"qa_engineer={QA_SLICE}", "--slice", f"repository_health_engineer={HEALTH_SLICE}",
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
            ({"invalid_return": True}, "residual_risk is missing"),
        ]
        for index, (options, message) in enumerate(cases):
            with self.subTest(message=message):
                batch = self.root / f"batch-{index}.json"
                self.batch = batch
                epoch = str(self.create()["epoch"])
                ledger = self.ledger(epoch, **options)
                result = self.run_script("collect", "--batch", str(batch), "--ledger", str(ledger), ok=False)
                self.assertIn(message, result.stderr)

    def test_rejects_duplicate_expected_role_or_slice_id(self) -> None:
        for slices in [
            [f"qa_engineer={QA_SLICE}", f"qa_engineer={HEALTH_SLICE}"],
            [f"qa_engineer={QA_SLICE}", f"repository_health_engineer={QA_SLICE}"],
        ]:
            args = ["create", "--task-uid", TASK, "--head", HEAD, "--evidence-digest", EVIDENCE]
            for value in slices:
                args += ["--slice", value]
            result = self.run_script(*args, "--out", str(self.root / f"{len(slices)}-{slices[1]}.json"), ok=False)
            self.assertIn("duplicate expected", result.stderr)

    def test_create_rejects_non_uuid_slice_identity(self) -> None:
        result = self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", EVIDENCE, "--slice", "qa_engineer=slice-qa",
            "--out", str(self.root / "invalid.json"), ok=False,
        )
        self.assertIn("UUID", result.stderr)

    def test_preflight_emits_incomplete_collector_valid_skeletons_without_pass_receipt(self) -> None:
        created = self.create()
        out_dir = self.root / "preflight"
        result = json.loads(self.run_script(
            "preflight", "--batch", str(self.batch), "--out-dir", str(out_dir)
        ).stdout)
        self.assertEqual("incomplete", result["status"])
        self.assertFalse(self.batch.with_name(f"{self.batch.stem}.collection.json").exists())
        ledger = Path(result["ledger_path"])
        failure = self.run_script("collect", "--batch", str(self.batch), "--ledger", str(ledger), ok=False)
        self.assertIn("not completed", failure.stderr)
        for expected in created["expected_slices"]:
            artifact = out_dir / f'{expected["slice_id"]}.json'
            payload = json.loads(artifact.read_text(encoding="utf-8"))
            self.assertEqual("incomplete", payload["status"])
            self.assertEqual(created["epoch"], payload["epoch"])
            self.assertEqual(expected["role"], payload["role"])
            self.assertEqual(expected["slice_id"], payload["slice_id"])
            self.assertEqual([], payload["findings"])
            self.assertNotEqual("passed", payload.get("disposition"))

    def test_reconcile_validates_completed_artifacts_rewrites_ledger_and_enables_collect(self) -> None:
        created = self.create()
        out_dir = self.root / "preflight-reconcile"
        preflight = json.loads(self.run_script(
            "preflight", "--batch", str(self.batch), "--out-dir", str(out_dir)
        ).stdout)
        ledger = Path(preflight["ledger_path"])
        old_ledger = ledger.read_bytes()
        expected_verdicts = {}
        for index, expected in enumerate(created["expected_slices"]):
            artifact = out_dir / f'{expected["slice_id"]}.json'
            payload = json.loads(artifact.read_text(encoding="utf-8"))
            scope_verdict = "approved" if index == 0 else "bounded"
            risk_verdict = "approved" if index == 0 else "accepted_with_residual"
            payload.update({
                "status": "completed", "disposition": "no_findings",
                "findings": [], "residual_risk": "none",
                "scope_verdict": scope_verdict, "risk_verdict": risk_verdict,
            })
            expected_verdicts[expected["role"]] = (scope_verdict, risk_verdict)
            artifact.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")

        reconciled = json.loads(self.run_script(
            "reconcile", "--batch", str(self.batch), "--ledger", str(ledger)
        ).stdout)
        self.assertEqual("completed", reconciled["status"])
        self.assertNotEqual(old_ledger, ledger.read_bytes())
        for row in ledger.read_text(encoding="utf-8").splitlines():
            entry = json.loads(row)
            artifact = Path(entry["artifacts"][0])
            self.assertEqual(hashlib.sha256(artifact.read_bytes()).hexdigest(), entry["artifact_digest"])
            self.assertEqual("completed", entry["status"])
            self.assertEqual(expected_verdicts[entry["role"]],
                             (entry["scope_verdict"], entry["risk_verdict"]))
            self.assertEqual("no_findings", entry["findings"])
        collected = json.loads(self.run_script(
            "collect", "--batch", str(self.batch), "--ledger", str(ledger)
        ).stdout)
        self.assertEqual("passed", collected["status"])

    def test_reconcile_fails_closed_for_incomplete_or_identity_mismatched_artifacts(self) -> None:
        for case in ("incomplete", "mismatched"):
            with self.subTest(case=case):
                self.batch = self.root / f"batch-{case}.json"
                self.create()
                out_dir = self.root / f"preflight-{case}"
                preflight = json.loads(self.run_script(
                    "preflight", "--batch", str(self.batch), "--out-dir", str(out_dir)
                ).stdout)
                ledger = Path(preflight["ledger_path"])
                if case == "mismatched":
                    artifact = next(out_dir.glob("*.json"))
                    payload = json.loads(artifact.read_text(encoding="utf-8"))
                    payload.update({"status": "completed", "head": "c" * 40,
                                    "disposition": "no_findings", "residual_risk": "none"})
                    artifact.write_text(json.dumps(payload) + "\n", encoding="utf-8")
                result = self.run_script(
                    "reconcile", "--batch", str(self.batch), "--ledger", str(ledger), ok=False
                )
                self.assertRegex(result.stderr.lower(), r"incomplete|mismatch|wrong head|invalid choice")
                self.assertFalse(self.batch.with_name(f"{self.batch.stem}.collection.json").exists())


if __name__ == "__main__":
    unittest.main()
