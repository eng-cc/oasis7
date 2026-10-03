#!/usr/bin/env python3
"""Behavior RED for schema-directed helper review archive closure selection.

The temporary roots model already-validated evidence only. They grant no review
or recovery authority; the public recovery route remains separately required.
"""
from __future__ import annotations
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
HELPER_PATH = ROOT / "scripts" / "pm" / "github-project-task.py"
SPEC = importlib.util.spec_from_file_location("github_project_task_archive_closure", HELPER_PATH)
assert SPEC is not None and SPEC.loader is not None
HELPER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = HELPER
SPEC.loader.exec_module(HELPER)
TASK_UID = "task_33f5586aac054487998a3584ac1d8143"
HEAD = "a" * 40
PRIOR_HEAD = "b" * 40
ROLES = ["repository_health_engineer", "qa_engineer"]
SLICES = ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"]
PRIOR_ROLE = "runtime_engineer"
PRIOR_SLICE = "33333333-3333-4333-8333-333333333333"
SCRATCH = ROOT / ".pm" / "scratch" / TASK_UID / "archive-closure-red"


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


class ArchiveClosureFixture:
    """A finite, schema-shaped evidence graph; no live authority is represented."""
    def __init__(self, root: Path) -> None:
        self.root = root
        self.scratch = root / ".pm" / "scratch" / TASK_UID
        self.plan_path = self.scratch / "review-plans" / "current.json"
        self.current_context, self.expected_paths = self._make_graph()

    def rel(self, path: Path) -> str:
        return path.relative_to(self.root).as_posix()

    def put(self, path: Path, raw: bytes) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def put_json(self, path: Path, value: object) -> Path:
        return self.put(path, canonical(value) + b"\n")

    def _batch(self, head: str, epoch_roles: list[dict[str, str]], evidence: str) -> tuple[str, dict[str, object]]:
        identity = {
            "task_uid": TASK_UID,
            "frozen_head": head,
            "relevant_evidence_digest": evidence,
            "expected_slices": epoch_roles,
        }
        epoch = digest(canonical(identity))
        return epoch, {
            "schema": "oasis7-review-batch/v1", "epoch": epoch,
            **identity,
        }

    def _make_graph(self) -> tuple[dict[str, object], set[str]]:
        expected: set[str] = set()
        current_slices = [{"role": role, "slice_id": slice_id} for role, slice_id in zip(ROLES, SLICES)]
        prior_slices = [{"role": PRIOR_ROLE, "slice_id": PRIOR_SLICE}]
        prior_evidence = "d" * 64
        prior_epoch, prior_batch = self._batch(PRIOR_HEAD, prior_slices, prior_evidence)
        prior_batch_path = self.scratch / "review-batches" / f"{prior_epoch}.json"
        self.put_json(prior_batch_path, prior_batch)
        prior_ledger_path = self.scratch / "review-plans" / "preflight" / f"{prior_epoch}.jsonl"
        prior_return_path = self.scratch / "review-returns" / f"{PRIOR_SLICE}.json"
        prior_return = {
            "schema": "oasis7-review-return/v1", "task_uid": TASK_UID,
            "role": PRIOR_ROLE, "slice_id": PRIOR_SLICE, "head": PRIOR_HEAD,
            "epoch": prior_epoch, "status": "completed", "disposition": "no_findings",
            "findings": [], "residual_risk": "fixture only",
        }
        prior_return_raw = canonical(prior_return) + b"\n"
        self.put(prior_return_path, prior_return_raw)
        prior_row = {
            "role": PRIOR_ROLE, "slice_id": PRIOR_SLICE, "task_uid": TASK_UID,
            "head": PRIOR_HEAD, "epoch": prior_epoch, "status": "completed",
            "activation": "message-assigned", "context_delivery": "minimal-task-packet",
            "actual_runtime": "inherit current parent selection", "scope_verdict": "approved",
            "risk_verdict": "approved", "findings": "no_findings", "residual_risk": "fixture only",
            "artifact_digest": digest(prior_return_raw), "artifacts": [self.rel(prior_return_path)],
        }
        prior_ledger_raw = (json.dumps(prior_row, sort_keys=True) + "\n").encode("utf-8")
        self.put(prior_ledger_path, prior_ledger_raw)
        prior_collection_path = prior_batch_path.with_name(f"{prior_epoch}.collection.json")
        prior_collection = {
            "schema": "oasis7-review-collection/v1", "status": "passed",
            "epoch": prior_epoch, "task_uid": TASK_UID, "frozen_head": PRIOR_HEAD,
            "ledger_digest": digest(prior_ledger_raw), "roles": [PRIOR_ROLE],
        }
        prior_collection_raw = canonical(prior_collection) + b"\n"
        self.put(prior_collection_path, prior_collection_raw)
        prior_plan_path = self.scratch / "review-plans" / f"{prior_epoch}.json"
        prior_plan = {
            "schema": "oasis7-review-plan/v2", "task_uid": TASK_UID,
            "frozen_head": PRIOR_HEAD, "epoch": prior_epoch, "roles": [PRIOR_ROLE],
            "expected_slices": prior_slices, "batch_path": self.rel(prior_batch_path),
            "preflight": {"status": "incomplete", "ledger_path": self.rel(prior_ledger_path)},
            "source_review_digest": prior_evidence, "integration_ci_digest": "e" * 64,
        }
        prior_plan_raw = canonical(prior_plan) + b"\n"
        self.put(prior_plan_path, prior_plan_raw)

        context: dict[str, object] = {
            "schema": "oasis7-review-context/v1", "authority": "context_only",
            "task_uid": TASK_UID, "current_head_oid": HEAD, "prior_head_oid": PRIOR_HEAD,
            "prior_plan_path": self.rel(prior_plan_path), "prior_plan_digest": digest(prior_plan_raw),
            "prior_epoch": prior_epoch, "prior_source_review_digest": prior_evidence,
            "prior_integration_ci_digest": "e" * 64, "prior_roles": [PRIOR_ROLE],
            "prior_collection_path": self.rel(prior_collection_path),
            "prior_collection_digest": digest(prior_collection_raw),
            "prior_collection_ledger_digest": digest(prior_ledger_raw),
            "delta_paths": [], "delta_paths_digest": digest(canonical([])),
            "delta_patch_digest": "f" * 64,
        }

        current_evidence = "c" * 64
        current_epoch, current_batch = self._batch(HEAD, current_slices, current_evidence)
        current_batch_path = self.scratch / "review-batches" / f"{current_epoch}.json"
        current_batch_raw = canonical(current_batch) + b"\n"
        self.put(current_batch_path, current_batch_raw)
        snapshot_path = self.scratch / "bootstrap-task-snapshot.json"
        snapshot = {
            "schema": "oasis7-bootstrap-task-snapshot/v1", "task": {"task_uid": TASK_UID},
            "git": {"head": HEAD},
        }
        snapshot_raw = canonical(snapshot) + b"\n"
        self.put(snapshot_path, snapshot_raw)
        current_ledger_path = self.scratch / "review-plans" / "preflight" / f"{current_epoch}.jsonl"
        current_rows = []
        current_return_paths = []
        packets = []
        for role, slice_id in zip(ROLES, SLICES):
            packet_path = self.scratch / "slice-packets" / f"{slice_id}.json"
            packet = {
                "schema": "oasis7-subagent-task-packet/v1",
                "identity": {"task_uid": TASK_UID, "head": HEAD, "base_sha": PRIOR_HEAD},
                "slice": {"role": role, "slice_id": slice_id},
                "review_context": context,
            }
            packet["packet_digest"] = digest(canonical(packet))
            packet_raw = canonical(packet) + b"\n"
            self.put(packet_path, packet_raw)
            packets.append((role, slice_id, packet_path, packet, packet_raw))
            return_path = self.scratch / "review-returns" / f"{slice_id}.json"
            returned = {
                "schema": "oasis7-review-return/v1", "task_uid": TASK_UID,
                "role": role, "slice_id": slice_id, "head": HEAD, "epoch": current_epoch,
                "status": "completed", "activation": "message-assigned",
                "context_delivery": "minimal-task-packet", "actual_runtime": "inherit current parent selection",
                "scope_verdict": "approved", "risk_verdict": "approved",
                "disposition": "no_findings", "findings": [], "residual_risk": "fixture only",
            }
            return_raw = canonical(returned) + b"\n"
            self.put(return_path, return_raw)
            current_return_paths.append(return_path)
            current_rows.append({
                "role": role, "slice_id": slice_id, "task_uid": TASK_UID,
                "head": HEAD, "epoch": current_epoch, "status": "completed",
                "activation": "message-assigned", "context_delivery": "minimal-task-packet",
                "actual_runtime": "inherit current parent selection", "scope_verdict": "approved",
                "risk_verdict": "approved", "findings": "no_findings", "residual_risk": "fixture only",
                "artifact_digest": digest(return_raw), "artifacts": [self.rel(return_path)],
            })
        current_ledger_raw = b"".join((json.dumps(row, sort_keys=True) + "\n").encode("utf-8") for row in current_rows)
        self.put(current_ledger_path, current_ledger_raw)
        collection_path = current_batch_path.with_name(f"{current_epoch}.collection.json")
        collection = {
            "schema": "oasis7-review-collection/v1", "status": "passed",
            "epoch": current_epoch, "task_uid": TASK_UID, "frozen_head": HEAD,
            "ledger_digest": digest(current_ledger_raw), "roles": sorted(ROLES),
        }
        collection_raw = canonical(collection) + b"\n"
        self.put(collection_path, collection_raw)
        self.plan_path.parent.mkdir(parents=True, exist_ok=True)
        packet_refs = []
        handoff_rows = []
        for role, slice_id, packet_path, packet, packet_raw in packets:
            return_path = self.scratch / "review-returns" / f"{slice_id}.json"
            packet_ref = self.rel(packet_path)
            packet_refs.append({"role": role, "slice_id": slice_id, "packet_ref": packet_ref})
            handoff_rows.append({
                "role": role, "slice_id": slice_id, "packet_path": packet_ref,
                "packet_digest": packet["packet_digest"], "artifact_path": self.rel(return_path),
                "return_sha256": digest(return_path.read_bytes()), "findings_digest": digest(canonical([])),
            })
        plan = {
            "schema": "oasis7-review-plan/v2", "task_uid": TASK_UID, "frozen_head": HEAD,
            "comparison_ref": "refs/heads/main", "comparison_oid": PRIOR_HEAD,
            "source_scope_oid": PRIOR_HEAD, "source_review_identity": {"task_uid": TASK_UID},
            "source_review_digest": current_evidence, "relevant_evidence_digest": current_evidence,
            "epoch": current_epoch, "batch_path": self.rel(current_batch_path),
            "collection_path": self.rel(collection_path), "roles": ROLES,
            "expected_slices": current_slices, "packet_refs": packet_refs,
            "preflight": {"status": "incomplete", "ledger_path": self.rel(current_ledger_path),
                          "artifact_paths": [self.rel(path) for path in current_return_paths]},
            "incremental_review_context": context,
        }
        plan_raw = canonical(plan) + b"\n"
        self.put(self.plan_path, plan_raw)
        handoff_payload = {
            "schema": "oasis7-review-handoff/v2", "repository": "eng-cc/oasis7",
            "task_uid": TASK_UID, "pr_number": 1, "comparison_ref": "refs/heads/main",
            "comparison_oid": PRIOR_HEAD, "frozen_head": HEAD,
            "source_review_identity": {"task_uid": TASK_UID}, "source_review_digest": current_evidence,
            "epoch": current_epoch, "plan_path": self.rel(self.plan_path), "plan_sha256": digest(plan_raw),
            "batch_path": self.rel(current_batch_path), "batch_sha256": digest(current_batch_raw),
            "preflight_ledger_path": self.rel(current_ledger_path),
            "preflight_ledger_sha256": digest(current_ledger_raw),
            "dispatch_evidence": {"issue_number": 1, "issue_url": "https://example.invalid/issues/1",
                                  "comment_id": 1, "author": "fixture", "body_digest": "1" * 64},
            "rows": handoff_rows,
        }
        handoff_payload["handoff_digest"] = digest(canonical(handoff_payload))
        handoff_path = self.scratch / "review-handoffs" / f"{current_epoch}.json"
        self.put_json(handoff_path, handoff_payload)
        origin_path = self.scratch / "publication-helper-review-origin.json"
        origin = {
            "schema": "oasis7-publication-helper-review-origin/v1", "task_uid": TASK_UID,
            "bootstrap_snapshot": {"value": snapshot, "raw_sha256": digest(snapshot_raw)},
            "review_plan": {"value": plan, "raw_sha256": digest(plan_raw)},
            "review_batch": {"value": current_batch, "raw_sha256": digest(current_batch_raw)},
            "dispatch_readback": {"issue_number": 1, "comment_id": 1, "author": "fixture",
                                  "body_digest": "1" * 64, "payload": {}},
            "packets": [{"role": role, "slice_id": slice_id, "repo_path": self.rel(packet_path),
                         "raw_sha256": digest(packet_raw), "value": packet}
                        for role, slice_id, packet_path, packet, packet_raw in packets],
            "reviewed_git": {"comparison_oid": PRIOR_HEAD, "source_scope_oid": PRIOR_HEAD, "head_oid": HEAD},
        }
        self.put_json(origin_path, origin)
        for path in (self.plan_path, current_batch_path, snapshot_path, current_ledger_path,
                     collection_path, handoff_path, origin_path, prior_plan_path, prior_batch_path,
                     prior_collection_path, prior_ledger_path, prior_return_path):
            expected.add(self.rel(path))
        expected.update(self.rel(packet_path) for _r, _s, packet_path, _p, _raw in packets)
        expected.update(self.rel(path) for path in current_return_paths)
        return context, expected


class PublicationHelperArchiveClosureTests(unittest.TestCase):
    def setUp(self) -> None:
        SCRATCH.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="archive-closure-", dir=SCRATCH)
        self.fixture = ArchiveClosureFixture(Path(self.temp.name) / "task")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def collect(self) -> list[dict[str, object]]:
        collector = getattr(HELPER, "_collect_review_archive_closure", None)
        self.assertTrue(
            callable(collector),
            "RED: canonical helper must expose _collect_review_archive_closure(task_root, task_uid, source_oid, plan_path)",
        )
        return collector(self.fixture.root, TASK_UID, HEAD, self.fixture.plan_path)

    def rejected(self) -> None:
        with self.assertRaises((ValueError, OSError, SystemExit)):
            self.collect()

    def test_selects_schema_references_and_ignores_unrelated_count_and_size(self) -> None:
        unrelated = self.fixture.scratch / "unrelated"
        unrelated.mkdir()
        for index in range(520):
            (unrelated / f"ignored-{index:04d}.bin").write_bytes(b"")
        oversized = unrelated / "ignored-large.bin"
        with oversized.open("wb") as handle:
            handle.truncate(50 * 1024 * 1024 + 1)
        rows = self.collect()
        paths = [str(row["path"]) for row in rows]
        self.assertEqual(paths, sorted(self.fixture.expected_paths))
        self.assertEqual(len(paths), len(set(paths)))
        for row in rows:
            raw = (self.fixture.root / str(row["path"])).read_bytes()
            self.assertEqual(row, {"path": str(row["path"]), "sha256": digest(raw), "size": len(raw)})
        self.assertNotIn(self.fixture.rel(oversized), paths)
        self.assertTrue(any("review-plans" in path and "current" in path for path in paths))
        self.assertTrue(any("review-batches" in path and ".collection.json" in path for path in paths))
        self.assertTrue(any("publication-helper-review-origin.json" in path for path in paths))

    def test_rejects_missing_required_packet_and_nested_prior_return(self) -> None:
        packet = self.fixture.scratch / "slice-packets" / f"{SLICES[0]}.json"
        packet.unlink()
        self.rejected()
        self.temp.cleanup()
        self.temp = tempfile.TemporaryDirectory(prefix="archive-closure-", dir=SCRATCH)
        self.fixture = ArchiveClosureFixture(Path(self.temp.name) / "task")
        prior_return = self.fixture.scratch / "review-returns" / f"{PRIOR_SLICE}.json"
        prior_return.unlink()
        self.rejected()

    def test_rejects_task_root_escape_and_symlinked_required_component(self) -> None:
        plan = json.loads(self.fixture.plan_path.read_text(encoding="utf-8"))
        outside = self.fixture.root.parent / "outside-batch.json"
        outside.write_text("{}\n", encoding="utf-8")
        plan["batch_path"] = "../../outside-batch.json"
        self.fixture.plan_path.write_bytes(canonical(plan) + b"\n")
        self.rejected()
        self.temp.cleanup()
        self.temp = tempfile.TemporaryDirectory(prefix="archive-closure-", dir=SCRATCH)
        self.fixture = ArchiveClosureFixture(Path(self.temp.name) / "task")
        packet_dir = self.fixture.scratch / "slice-packets"
        packet_dir.rename(packet_dir.with_name("slice-packets-original"))
        outside_packets = self.fixture.root.parent / "outside-packets"
        outside_packets.mkdir()
        packet_name = f"{SLICES[0]}.json"
        (outside_packets / packet_name).write_text("{}\n", encoding="utf-8")
        packet_dir.symlink_to(outside_packets, target_is_directory=True)
        self.rejected()

    def test_rejects_distinct_lexical_alias_for_same_return_member(self) -> None:
        plan = json.loads(self.fixture.plan_path.read_text(encoding="utf-8"))
        canonical_path = plan["preflight"]["artifact_paths"][0]
        self.fixture.scratch.joinpath("review-returns", "alias-hop").mkdir()
        plan["preflight"]["artifact_paths"][0] = canonical_path.replace("review-returns/", "review-returns/alias-hop/../")
        self.fixture.plan_path.write_bytes(canonical(plan) + b"\n")
        self.rejected()

    def test_rejects_unknown_reference_bearing_schema_field(self) -> None:
        plan = json.loads(self.fixture.plan_path.read_text(encoding="utf-8"))
        plan["unrecognized_artifact_refs"] = [".pm/scratch/elsewhere/secret.json"]
        self.fixture.plan_path.write_bytes(canonical(plan) + b"\n")
        self.rejected()

    def test_rejects_required_closure_over_byte_limit(self) -> None:
        return_path = self.fixture.scratch / "review-returns" / f"{SLICES[0]}.json"
        huge = b'{"padding":"' + b"x" * (50 * 1024 * 1024) + b'"}\n'
        return_path.write_bytes(huge)
        rows_path = self.fixture.scratch / "review-plans" / "preflight" / f"{json.loads(self.fixture.plan_path.read_text())['epoch']}.jsonl"
        rows = [json.loads(line) for line in rows_path.read_text(encoding="utf-8").splitlines()]
        rows[0]["artifact_digest"] = digest(huge)
        rows[0]["artifacts"] = [self.fixture.rel(return_path)]
        rows_path.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows), encoding="utf-8")
        self.rejected()

    def test_revalidates_task_uid_and_current_packet_head_binding(self) -> None:
        collector = getattr(HELPER, "_collect_review_archive_closure", None)
        self.assertTrue(callable(collector), "RED: canonical archive closure collector is missing")
        with self.assertRaises((ValueError, OSError, SystemExit)):
            collector(self.fixture.root, "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", HEAD, self.fixture.plan_path)
        packet_path = self.fixture.scratch / "slice-packets" / f"{SLICES[0]}.json"
        packet = json.loads(packet_path.read_text(encoding="utf-8"))
        packet["identity"]["head"] = "f" * 40
        packet_path.write_bytes(canonical(packet) + b"\n")
        with self.assertRaises((ValueError, OSError, SystemExit)):
            collector(self.fixture.root, TASK_UID, HEAD, self.fixture.plan_path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
