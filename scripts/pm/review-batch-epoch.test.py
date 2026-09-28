#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("review-batch-epoch.py")
TASK = "task_" + "1" * 32
HEAD = "a" * 40
EVIDENCE = "b" * 64
QA_SLICE = "11111111-1111-4111-8111-111111111111"
HEALTH_SLICE = "22222222-2222-4222-8222-222222222222"
REPOSITORY = "eng-cc/oasis7"


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


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

    def handoff_fixture(self, root: Path | None = None) -> dict[str, object]:
        """Create a complete isolated v2 plan whose preflight bytes stay original."""
        if root is not None:
            self.root = root
        task_root = self.root / ".pm" / "scratch" / TASK
        batch_path = task_root / "review-batches" / "batch.json"
        batch_path.parent.mkdir(parents=True, exist_ok=True)
        self.batch = batch_path
        expected_slices = [
            {"role": "qa_engineer", "slice_id": QA_SLICE},
            {"role": "repository_health_engineer", "slice_id": HEALTH_SLICE},
        ]
        ordered_roles = [str(item["role"]) for item in expected_slices]
        source_identity = {
            "task_uid": TASK, "bootstrap_epoch": 1, "repository": REPOSITORY, "pr_number": 1,
            "source_head_oid": HEAD, "source_scope_oid": "c" * 40,
            "changed_paths_digest": EVIDENCE, "ordered_role_ids": ordered_roles,
            "role_contract_digest": "d" * 64, "review_policy_digest": "e" * 64,
            "input_contract_digest": "f" * 64,
        }
        source_digest = digest(source_identity)
        batch = json.loads(self.run_script(
            "create", "--task-uid", TASK, "--head", HEAD,
            "--evidence-digest", source_digest,
            "--slice", f"qa_engineer={QA_SLICE}",
            "--slice", f"repository_health_engineer={HEALTH_SLICE}",
            "--out", str(batch_path),
        ).stdout)
        epoch = str(batch["epoch"])
        preflight_dir = task_root / "review-plans" / "preflight"
        preflight = json.loads(self.run_script(
            "preflight", "--batch", str(batch_path), "--out-dir", str(preflight_dir)
        ).stdout)
        ledger_path = Path(str(preflight["ledger_path"]))
        original_ledger = ledger_path.read_bytes()
        returned_paths: list[Path] = []
        expected_slices = batch["expected_slices"]
        for expected in expected_slices:
            artifact = preflight_dir / f'{expected["slice_id"]}.json'
            returned = json.loads(artifact.read_text(encoding="utf-8"))
            returned.update({
                "status": "completed", "activation": "message-assigned",
                "context_delivery": "minimal-task-packet",
                "actual_runtime": "inherited/unverified: human-operated",
                "scope_verdict": "approved", "risk_verdict": "approved",
                "disposition": "no_findings", "findings": [],
                "residual_risk": "fixture risk",
            })
            artifact.write_text(json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
            returned_paths.append(artifact)

        expected_slices = batch["expected_slices"]
        applicability = {
            "changed_paths_digest": source_identity["changed_paths_digest"],
            "input_contract_digest": source_identity["input_contract_digest"],
            "ordered_role_ids": ordered_roles,
            "role_contract_digest": source_identity["role_contract_digest"],
            "review_policy_digest": source_identity["review_policy_digest"],
        }
        plan_path = task_root / "review-plans" / "plan.json"
        plan_path.parent.mkdir(parents=True, exist_ok=True)
        plan = {
            "schema": "oasis7-review-plan/v2", "task_uid": TASK, "frozen_head": HEAD,
            "comparison_ref": "refs/heads/main", "comparison_oid": "c" * 40,
            "source_scope_oid": "c" * 40, "source_review_identity": source_identity,
            "source_review_digest": digest(source_identity),
            "relevant_evidence_digest": digest(source_identity),
            "professional_review_applicability": {
                "identity": applicability, "identity_digest": digest(applicability), "verified": True,
            },
            "epoch": epoch, "batch_path": str(batch_path),
            "preflight": {"status": "incomplete", "ledger_path": str(ledger_path)},
            "roles": ordered_roles, "expected_slices": expected_slices,
        }
        plan_path.write_text(json.dumps(plan, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json")
        handoff_path = task_root / "review-handoffs" / f"{epoch}.json"
        return {
            "task_root": task_root, "batch_path": batch_path, "batch": batch,
            "plan_path": plan_path, "plan": plan, "ledger_path": ledger_path,
            "original_ledger": original_ledger, "returns": returned_paths,
            "collection_path": collection_path, "handoff_path": handoff_path,
        }

    def handoff_command(self, plan_path: Path, *, ok: bool = True) -> subprocess.CompletedProcess[str]:
        return self.run_script("handoff", "--plan", str(plan_path), ok=ok)

    def test_handoff_cli_route_is_recognized(self) -> None:
        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "handoff", "--help"],
            text=True, capture_output=True,
        )
        self.assertEqual(
            0, result.returncode,
            "handoff capability sentinel: expected the adopted handoff --plan route to exist; "
            f"stderr={result.stderr.strip()!r}",
        )

    def test_handoff_create_once_binds_plan_and_completed_returns(self) -> None:
        route = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "handoff", "--help"],
            text=True, capture_output=True,
        )
        if route.returncode != 0:
            print("NOT EXERCISED: valid handoff digest/order/input-invariance and repeat-create controls await route recognition", file=sys.stderr)
            self.skipTest("not exercised: handoff route is unsupported on frozen source")
        fixture = self.handoff_fixture()
        snapshots = {
            Path(fixture["plan_path"]): Path(fixture["plan_path"]).read_bytes(),
            Path(fixture["batch_path"]): Path(fixture["batch_path"]).read_bytes(),
            Path(fixture["ledger_path"]): Path(fixture["ledger_path"]).read_bytes(),
            **{path: path.read_bytes() for path in fixture["returns"]},
        }
        created = json.loads(self.handoff_command(Path(fixture["plan_path"])).stdout)
        handoff_path = Path(fixture["handoff_path"])
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        payload = {key: value for key, value in handoff.items() if key != "handoff_digest"}
        self.assertEqual("created", created["status"])
        self.assertEqual("oasis7-review-return-handoff/v1", handoff["schema"])
        self.assertEqual({
            "schema", "repository", "task_uid", "pr_number", "comparison_ref",
            "comparison_oid", "frozen_head", "source_review_identity",
            "source_review_digest", "epoch", "plan_path", "plan_sha256",
            "batch_path", "batch_sha256", "preflight_ledger_path",
            "preflight_ledger_sha256", "rows", "handoff_digest",
        }, set(handoff))
        self.assertEqual({
            "task_uid", "bootstrap_epoch", "repository", "pr_number",
            "source_head_oid", "source_scope_oid", "changed_paths_digest",
            "ordered_role_ids", "role_contract_digest", "review_policy_digest",
            "input_contract_digest",
        }, set(handoff["source_review_identity"]))
        self.assertEqual(digest(payload), handoff["handoff_digest"])
        self.assertEqual(hashlib.sha256(snapshots[Path(fixture["plan_path"])]).hexdigest(), handoff["plan_sha256"])
        self.assertEqual(hashlib.sha256(snapshots[Path(fixture["batch_path"])]).hexdigest(), handoff["batch_sha256"])
        self.assertEqual(hashlib.sha256(fixture["original_ledger"]).hexdigest(), handoff["preflight_ledger_sha256"])
        plan = fixture["plan"]
        batch = fixture["batch"]
        source_identity = plan["source_review_identity"]
        self.assertEqual(plan["task_uid"], handoff["task_uid"])
        self.assertEqual(source_identity["pr_number"], handoff["pr_number"])
        self.assertEqual(plan["comparison_ref"], handoff["comparison_ref"])
        self.assertEqual(plan["comparison_oid"], handoff["comparison_oid"])
        self.assertEqual(plan["frozen_head"], handoff["frozen_head"])
        self.assertEqual(plan["epoch"], handoff["epoch"])
        self.assertEqual(REPOSITORY, handoff["repository"])
        self.assertEqual(source_identity, handoff["source_review_identity"])
        self.assertEqual(TASK, source_identity["task_uid"])
        self.assertEqual(REPOSITORY, source_identity["repository"])
        self.assertEqual(source_identity["pr_number"], handoff["pr_number"])
        self.assertEqual(HEAD, source_identity["source_head_oid"])
        self.assertEqual(plan["comparison_oid"], source_identity["source_scope_oid"])
        self.assertEqual(plan["source_review_digest"], handoff["source_review_digest"])
        self.assertEqual(source_identity["task_uid"], batch["task_uid"])
        self.assertEqual(source_identity["source_head_oid"], batch["frozen_head"])
        self.assertEqual(plan["epoch"], batch["epoch"])
        self.assertEqual(batch["relevant_evidence_digest"], handoff["source_review_digest"])
        self.assertEqual(plan["roles"], source_identity["ordered_role_ids"])
        self.assertEqual(plan["expected_slices"], batch["expected_slices"])
        self.assertEqual(
            Path(fixture["plan_path"]).relative_to(self.root).as_posix(), handoff["plan_path"]
        )
        self.assertEqual(
            Path(fixture["batch_path"]).relative_to(self.root).as_posix(), handoff["batch_path"]
        )
        self.assertEqual(
            Path(fixture["ledger_path"]).resolve().relative_to(self.root.resolve()).as_posix(),
            handoff["preflight_ledger_path"],
        )
        expected_rows = []
        for item, path in zip(fixture["batch"]["expected_slices"], fixture["returns"], strict=True):
            returned = json.loads(path.read_text(encoding="utf-8"))
            expected_rows.append({
                "role": item["role"], "slice_id": item["slice_id"],
                "artifact_path": path.relative_to(self.root).as_posix(),
                "return_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "findings_digest": digest(returned["findings"]),
            })
        expected_rows.sort(key=lambda row: (row["role"].encode(), row["slice_id"].encode()))
        self.assertTrue(all(set(row) == {
            "role", "slice_id", "artifact_path", "return_sha256", "findings_digest"
        } for row in handoff["rows"]))
        self.assertEqual(expected_rows, handoff["rows"])
        for path, before in snapshots.items():
            self.assertEqual(before, path.read_bytes(), f"handoff mutated input {path}")
        self.assertFalse(Path(fixture["collection_path"]).exists())

        first_handoff = handoff_path.read_bytes()
        retry = self.handoff_command(Path(fixture["plan_path"]), ok=False)
        self.assertRegex(retry.stderr.lower(), r"immutable|exists|replace")
        self.assertEqual(first_handoff, handoff_path.read_bytes())
        for path, before in snapshots.items():
            self.assertEqual(before, path.read_bytes(), f"repeat-create mutated input {path}")

    def test_handoff_invalid_inputs_are_gated_on_valid_route_and_control(self) -> None:
        route = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "handoff", "--help"],
            text=True, capture_output=True,
        )
        if route.returncode != 0:
            print("NOT EXERCISED: malformed/identity/coverage/containment handoff matrix awaits route recognition and valid create control", file=sys.stderr)
            self.skipTest("not exercised: handoff malformed matrix awaits route recognition")
        control = self.handoff_fixture()
        self.handoff_command(Path(control["plan_path"]))
        self.assertTrue(Path(control["handoff_path"]).is_file(), "valid handoff fixture control did not create output")

        original_root, original_batch = self.root, self.batch
        cases = (
            ("plan_identity", r"task|identity"),
            ("plan_unknown_key", r"plan|field|unknown"),
            ("plan_duplicate_key", r"duplicate json key"),
            ("plan_type_coercion", r"epoch|type|plan"),
            ("plan_source_digest", r"source|digest|review"),
            ("batch_identity", r"head|batch"),
            ("batch_unknown_key", r"batch|field|unknown"),
            ("ledger_identity", r"head|ledger|identity"),
            ("ledger_missing_row", r"slice|row|ledger|identity"),
            ("ledger_duplicate_row", r"duplicate|slice|ledger"),
            ("ledger_unexpected_row", r"slice|row|ledger|identity"),
            ("incomplete_return", r"status|completed|return"),
            ("malformed_return", r"disposition|findings|return"),
            ("return_identity", r"task|identity|return"),
            ("outside_symlink", r"symlink|outside|repository|return"),
        )
        try:
            for case, diagnostic in cases:
                with self.subTest(case=case), tempfile.TemporaryDirectory() as tmp:
                    self.root = Path(tmp)
                    fixture = self.handoff_fixture()
                    plan_path = Path(fixture["plan_path"])
                    batch_path = Path(fixture["batch_path"])
                    ledger_path = Path(fixture["ledger_path"])
                    return_paths = fixture["returns"]
                    if case == "plan_identity":
                        plan = json.loads(plan_path.read_text(encoding="utf-8"))
                        plan["task_uid"] = "task_" + "2" * 32
                        plan_path.write_text(json.dumps(plan, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "plan_unknown_key":
                        plan = json.loads(plan_path.read_text(encoding="utf-8"))
                        plan["unexpected"] = "closed schema mutation"
                        plan_path.write_text(json.dumps(plan, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "plan_duplicate_key":
                        plan = json.loads(plan_path.read_text(encoding="utf-8"))
                        self.assertEqual(TASK, plan["task_uid"])
                        top_level_task_sentinel = "task_" + "9" * 32
                        plan["task_uid"] = top_level_task_sentinel
                        raw = json.dumps(plan, sort_keys=True) + "\n"
                        key = f'"task_uid": "{top_level_task_sentinel}"'
                        self.assertEqual(1, raw.count(key))
                        duplicate_original_task_keys = f'"task_uid": "{TASK}", "task_uid": "{TASK}"'
                        plan_path.write_text(
                            raw.replace(key, duplicate_original_task_keys, 1), encoding="utf-8"
                        )
                    elif case == "plan_type_coercion":
                        plan = json.loads(plan_path.read_text(encoding="utf-8"))
                        plan["epoch"] = 7
                        plan_path.write_text(json.dumps(plan, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "plan_source_digest":
                        plan = json.loads(plan_path.read_text(encoding="utf-8"))
                        plan["source_review_digest"] = "a" * 64
                        plan_path.write_text(json.dumps(plan, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "batch_identity":
                        batch = json.loads(batch_path.read_text(encoding="utf-8"))
                        batch["frozen_head"] = "c" * 40
                        batch_path.write_text(json.dumps(batch, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "batch_unknown_key":
                        batch = json.loads(batch_path.read_text(encoding="utf-8"))
                        batch["unexpected"] = True
                        batch_path.write_text(json.dumps(batch, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "ledger_identity":
                        rows = [json.loads(line) for line in ledger_path.read_text().splitlines() if line.strip()]
                        rows[0]["head"] = "c" * 40
                        ledger_path.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
                    elif case == "ledger_missing_row":
                        rows = [line for line in ledger_path.read_text().splitlines() if line.strip()]
                        ledger_path.write_text(rows[0] + "\n", encoding="utf-8")
                    elif case == "ledger_duplicate_row":
                        rows = [line for line in ledger_path.read_text().splitlines() if line.strip()]
                        ledger_path.write_text("\n".join([rows[0], rows[0]]) + "\n", encoding="utf-8")
                    elif case == "ledger_unexpected_row":
                        rows = [json.loads(line) for line in ledger_path.read_text().splitlines() if line.strip()]
                        rows[1]["slice_id"] = "33333333-3333-4333-8333-333333333333"
                        ledger_path.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
                    elif case == "incomplete_return":
                        returned = json.loads(return_paths[0].read_text(encoding="utf-8"))
                        returned["status"] = "incomplete"
                        return_paths[0].write_text(json.dumps(returned, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "malformed_return":
                        returned = json.loads(return_paths[0].read_text(encoding="utf-8"))
                        returned["findings"] = "not a structured list"
                        return_paths[0].write_text(json.dumps(returned, sort_keys=True) + "\n", encoding="utf-8")
                    elif case == "return_identity":
                        returned = json.loads(return_paths[0].read_text(encoding="utf-8"))
                        returned["task_uid"] = "task_" + "2" * 32
                        return_paths[0].write_text(json.dumps(returned, sort_keys=True) + "\n", encoding="utf-8")
                    else:
                        outside = self.root.parent / f"{self.root.name}-outside-return.json"
                        self.addCleanup(lambda path=outside: path.unlink(missing_ok=True))
                        outside.write_bytes(return_paths[0].read_bytes())
                        return_paths[0].unlink()
                        return_paths[0].symlink_to(outside)
                        outside_link = os.readlink(return_paths[0])

                    tracked = [plan_path, batch_path, ledger_path, *return_paths]
                    before = {path: path.read_bytes() for path in tracked}
                    failure = self.handoff_command(plan_path, ok=False)
                    self.assertNotEqual(0, failure.returncode)
                    self.assertRegex((failure.stderr + failure.stdout).lower(), diagnostic)
                    self.assertFalse(Path(fixture["handoff_path"]).exists())
                    self.assertFalse(Path(fixture["collection_path"]).exists())
                    for path, raw in before.items():
                        self.assertEqual(raw, path.read_bytes(), f"invalid {case} mutated {path}")
                    if case == "outside_symlink":
                        self.assertTrue(return_paths[0].is_symlink())
                        self.assertEqual(outside_link, os.readlink(return_paths[0]))
        finally:
            self.root, self.batch = original_root, original_batch

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

    def test_direct_reconcile_fails_closed_for_plan_owned_preflight(self) -> None:
        fixture = self.handoff_fixture()
        plan_path = Path(fixture["plan_path"])
        batch_path = Path(fixture["batch_path"])
        ledger_path = Path(fixture["ledger_path"])
        original_ledger = bytes(fixture["original_ledger"])
        plan_before = plan_path.read_bytes()
        batch_before = batch_path.read_bytes()
        return_before = {path: path.read_bytes() for path in fixture["returns"]}
        collection_path = Path(fixture["collection_path"])
        self.assertFalse(collection_path.exists())

        plan = json.loads(plan_before)
        self.assertEqual("oasis7-review-plan/v2", plan["schema"])
        self.assertEqual(ledger_path.resolve(), Path(plan["preflight"]["ledger_path"]).resolve())
        self.assertEqual("incomplete", plan["preflight"]["status"])

        result = subprocess.run(
            [str(SCRIPT), "--root", str(self.root), "reconcile",
             "--batch", str(batch_path), "--ledger", str(ledger_path)],
            text=True, capture_output=True,
        )
        problems: list[str] = []
        ledger_after = ledger_path.read_bytes()
        if result.returncode == 0:
            problems.append(
                "direct reconcile unexpectedly succeeded without v2 promotion proof "
                f"(stdout={result.stdout.strip()!r}, ledger_changed={ledger_after != original_ledger})"
            )
        elif not re.search(r"handoff|promotion|plan-owned|preflight", result.stderr, re.IGNORECASE):
            problems.append(f"rejection was not attributable to the missing v2 proof: {result.stderr.strip()!r}")
        if ledger_after != original_ledger:
            problems.append("direct reconcile changed the exact plan-owned preflight ledger bytes")
        if plan_path.read_bytes() != plan_before or batch_path.read_bytes() != batch_before:
            problems.append("direct reconcile changed the immutable plan or batch bytes")
        if {path: path.read_bytes() for path in fixture["returns"]} != return_before:
            problems.append("direct reconcile changed one or more bound return artifacts")
        if collection_path.exists():
            problems.append("direct reconcile created a collection receipt")
        self.assertFalse(problems, "plan-owned direct reconcile contract violated: " + "; ".join(problems))

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
