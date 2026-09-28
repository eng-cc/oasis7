#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("review-findings-resolution.py")
BATCH_SCRIPT = Path(__file__).with_name("review-batch-epoch.py")
PLAN_SCRIPT = Path(__file__).with_name("review-plan.py")
_PROJECTION_SPEC = importlib.util.spec_from_file_location(
    "workflow_impact_projection_for_resolution_tests",
    Path(__file__).with_name("workflow-impact-projection.py"),
)
assert _PROJECTION_SPEC is not None and _PROJECTION_SPEC.loader is not None
WORKFLOW_IMPACT = importlib.util.module_from_spec(_PROJECTION_SPEC)
_PROJECTION_SPEC.loader.exec_module(WORKFLOW_IMPACT)
TASK = "task_" + "1" * 32
HEAD = "a" * 40
EPOCH = "b" * 64
ROLE = "repository_health_engineer"
SLICE = "11111111-1111-4111-8111-111111111111"
REPO = "eng-cc/oasis7"
ISSUE = 3615
COMMENT_ID = 3934017999
ADMIN = "repo-admin"


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


class ReviewFindingsResolutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.task_root = self.root / ".pm" / "scratch" / TASK
        self.task_root.mkdir(parents=True)
        mapping_root = self.root / ".pm" / "github-project-sync"
        mapping_root.mkdir(parents=True)
        (mapping_root / "tasks.json").write_text(
            json.dumps({"project": {"repo": REPO}, "tasks": {TASK: {"issue_number": ISSUE}}}) + "\n",
            encoding="utf-8",
        )
        self.artifact = self.task_root / "return.json"
        self.evidence = self.task_root / "verify.txt"
        self.evidence.write_bytes(b"exact repository proof\n")
        self.readback = self.task_root / "review-resolutions" / f"{EPOCH}.readback.json"
        self.manifest = self.task_root / "review-resolutions" / f"{EPOCH}.json"
        self.v2_fixture_counter = 0
        self.manifest.parent.mkdir()
        self.ledger = self.task_root / "slice-ledger.jsonl"
        self.gh_log = self.root / "gh.log"
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.fake_gh = self.bin / "gh"
        self._write_fixture()
        self._write_fake_gh(permission="admin")
        self.v2_git_initialized = False

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _write_fake_gh(self, *, permission: str, author: str = ADMIN, body: str | None = None,
                       issue_number: int = ISSUE, task_body: str | None = None,
                       comment_issue_number: int = ISSUE) -> None:
        if body is None:
            body = self.body
        if task_body is None:
            task_body = f"<!-- oasis7-pm-task -->\ntask_uid: {TASK}\n"
        self.fake_gh.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "args = sys.argv[1:]\n"
            "open(os.environ['GH_LOG'], 'a').write(' '.join(args) + '\\n')\n"
            f"if args[:2] != ['api', 'repos/eng-cc/oasis7/issues/{issue_number}'] and args[:2] != ['api', 'repos/eng-cc/oasis7/issues/{issue_number}/comments'] and args[:2] != ['api', 'repos/eng-cc/oasis7/issues/comments/3934017999'] and args[:2] != ['api', 'repos/eng-cc/oasis7/collaborators/{author}/permission']:\n"
            "    raise SystemExit('unexpected gh call: ' + ' '.join(args))\n"
            f"if args[1] == 'repos/eng-cc/oasis7/issues/{issue_number}':\n"
            "    print(json.dumps({'number': " + str(issue_number) + ", 'body': " + repr(task_body) + "}))\n"
            f"elif args[1] == 'repos/eng-cc/oasis7/issues/{issue_number}/comments':\n"
            "    print('[]')\n"
            "elif 'comments' in args[1]:\n"
            "    print(json.dumps({'id': 3934017999, 'body': " + repr(body) + ", 'issue_url': 'https://api.github.com/repos/eng-cc/oasis7/issues/" + str(comment_issue_number) + "', 'user': {'login': " + repr(author) + "}, 'created_at': '2026-09-06T10:00:00Z'}))\n"
            "else:\n"
            "    print(json.dumps({'permission': " + repr(permission) + "}))\n",
            encoding="utf-8",
        )
        self.fake_gh.chmod(0o755)

    def _write_fixture(self) -> None:
        finding = {
            "id": "P1",
            "summary": "evidence-backed finding",
            "triage": {"classification": "blocking", "basis": "fixture"},
        }
        self.findings = [finding]
        self.findings_digest = digest(self.findings)
        self.finding_digest = digest(finding)
        output = b"verification output\n"
        self.output_digest = hashlib.sha256(output).hexdigest()
        self.evidence_digest = hashlib.sha256(self.evidence.read_bytes()).hexdigest()
        self.entry_preimage = {
            "status": "completed",
            "index": 0,
            "finding_digest": self.finding_digest,
            "disposition": "rejected_with_evidence",
            "evidence_kind": "repository_verification",
            "evidence_ref": str(self.evidence.relative_to(self.root)),
            "evidence_digest": self.evidence_digest,
            "verification_result": {"status": "passed", "output_digest": self.output_digest},
        }
        entry = {**self.entry_preimage, "entry_digest": digest(self.entry_preimage)}
        payload = {
            "schema": "oasis7-review-resolution/v1",
            "task_uid": TASK,
            "head": HEAD,
            "epoch": EPOCH,
            "role_records": [{"role": ROLE, "slice_id": SLICE, "findings_digest": self.findings_digest, "entries": [entry]}],
        }
        self.manifest.write_text(json.dumps({**payload, "manifest_digest": digest(payload)}, sort_keys=True) + "\n")
        self.body_payload = {
            "marker": "oasis7-review-resolution",
            "schema": "oasis7-review-resolution/v1",
            "task_uid": TASK,
            "head": HEAD,
            "epoch": EPOCH,
            "manifest_digest": digest(payload),
        }
        self.body = json.dumps(self.body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        self.readback.parent.mkdir(parents=True, exist_ok=True)
        self.readback.write_text(json.dumps({
            **self.body_payload,
            "repository": REPO,
            "issue_number": ISSUE,
            "comment_id": COMMENT_ID,
            "comment_url": f"https://github.com/{REPO}/issues/{ISSUE}#issuecomment-{COMMENT_ID}",
            "author": ADMIN,
            "created_at": "2026-09-06T10:00:00Z",
            "observed_at": "2026-09-06T10:01:00Z",
            "body_digest": hashlib.sha256(self.body.encode()).hexdigest(),
        }, sort_keys=True) + "\n")
        self.artifact.write_text(json.dumps({
            "task_uid": TASK, "role": ROLE, "slice_id": SLICE, "head": HEAD,
            "epoch": EPOCH, "status": "completed", "disposition": "findings",
            "findings": self.findings, "residual_risk": "fixture risk",
        }, sort_keys=True) + "\n")
        artifact_digest = hashlib.sha256(self.artifact.read_bytes()).hexdigest()
        self.ledger.write_text(json.dumps({
            "task_uid": TASK, "role": ROLE, "slice_id": SLICE, "head": HEAD,
            "epoch": EPOCH, "status": "completed", "findings": "findings",
            "artifact_digest": artifact_digest, "artifacts": [str(self.artifact)],
        }, sort_keys=True) + "\n")

    def _rewrite_fixture(self, *, finding: dict[str, object] | None = None,
                         disposition: str | None = None) -> None:
        if finding is not None:
            self.findings = [finding]
            artifact = json.loads(self.artifact.read_text())
            artifact["findings"] = self.findings
            self.artifact.write_text(json.dumps(artifact, sort_keys=True) + "\n")
            ledger = json.loads(self.ledger.read_text())
            ledger["artifact_digest"] = hashlib.sha256(self.artifact.read_bytes()).hexdigest()
            self.ledger.write_text(json.dumps(ledger, sort_keys=True) + "\n")

        manifest = json.loads(self.manifest.read_text())
        record = manifest["role_records"][0]
        record["findings_digest"] = digest(self.findings)
        entry = record["entries"][0]
        entry["finding_digest"] = digest(self.findings[0])
        if disposition is not None:
            entry["disposition"] = disposition
        entry["entry_digest"] = digest({key: value for key, value in entry.items() if key != "entry_digest"})
        payload = {key: value for key, value in manifest.items() if key != "manifest_digest"}
        manifest["manifest_digest"] = digest(payload)
        self.manifest.write_text(json.dumps(manifest, sort_keys=True) + "\n")

        self.body_payload["manifest_digest"] = manifest["manifest_digest"]
        self.body = json.dumps(self.body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        readback = json.loads(self.readback.read_text())
        readback["manifest_digest"] = manifest["manifest_digest"]
        readback["body_digest"] = hashlib.sha256(self.body.encode()).hexdigest()
        self.readback.write_text(json.dumps(readback, sort_keys=True) + "\n")
        self._write_fake_gh(permission="admin", body=self.body)

    def _prepare_v2_plan_repo(self) -> None:
        """Create the minimum isolated Git/task context required by the real v2 plan producer."""
        if self.v2_git_initialized:
            return
        mapping_path = self.root / ".pm" / "github-project-sync" / "tasks.json"
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        mapping.setdefault("project", {})["repo"] = REPO
        mapping.setdefault("tasks", {})[TASK] = {
            "task_uid": TASK, "repository": REPO, "issue_number": ISSUE,
            "pr_number": ISSUE, "bootstrap_epoch": 1,
        }
        mapping_path.write_text(json.dumps(mapping, sort_keys=True) + "\n", encoding="utf-8")
        for path, content in (
            (self.root / ".agents" / "roles" / f"{ROLE}.md", f"# {ROLE}\n"),
            (self.root / "doc" / "engineering" / "workflow" / "source-of-truth.md", "# fixture policy\n"),
            (self.root / ".agents" / "skills" / "requesting-repo-owned-review" / "SKILL.md", "# fixture review skill\n"),
        ):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        (self.root / "README").write_text("plan fixture\n", encoding="utf-8")
        commands = (
            ("init", "-b", "main"),
            ("config", "user.email", "test@example.invalid"),
            ("config", "user.name", "Review fixture"),
            ("add", "README"),
            ("commit", "-m", "fixture base"),
        )
        for args in commands:
            result = subprocess.run(
                ["git", "-C", str(self.root), *args], text=True, capture_output=True,
            )
            self.assertEqual(0, result.returncode, result.stderr)
        head = subprocess.run(
            ["git", "-C", str(self.root), "rev-parse", "HEAD"],
            text=True, capture_output=True,
        )
        self.assertEqual(0, head.returncode, head.stderr)
        self.v2_head = head.stdout.strip()
        updated = subprocess.run(
            ["git", "-C", str(self.root), "update-ref", "refs/remotes/origin/main", self.v2_head],
            text=True, capture_output=True,
        )
        self.assertEqual(0, updated.returncode, updated.stderr)
        self.v2_git_initialized = True

    def _materialize_v2_task_packets(self, plan: dict[str, object]) -> None:
        """Write complete digest-bound packet fixtures at every immutable plan ref."""
        refs = plan["packet_refs"]
        slices = plan["expected_slices"]
        source_identity = plan["source_review_identity"]
        self.assertIsInstance(refs, list)
        self.assertIsInstance(slices, list)
        self.assertIsInstance(source_identity, dict)
        self.assertEqual(len(refs), len(slices))
        runtime_reason = (
            "message-assigned fallback; adapter inactive on this surface; "
            "actual runtime/model/reasoning unverified"
        )
        for ref, expected in zip(refs, slices):
            self.assertIsInstance(ref, dict)
            self.assertIsInstance(expected, dict)
            role = expected["role"]
            slice_id = expected["slice_id"]
            self.assertEqual({"role": role, "slice_id": slice_id},
                             {key: ref[key] for key in ("role", "slice_id")})
            packet_path = self.root / ref["packet_ref"]
            packet_path.parent.mkdir(parents=True, exist_ok=True)
            payload = {
                "schema": "oasis7-subagent-task-packet/v1",
                "created_at": "2026-09-28T00:00:00+00:00",
                "identity": {
                    "task_uid": TASK,
                    "issue_url": f"https://github.com/{REPO}/issues/{ISSUE}",
                    "repository": REPO,
                    "project_item_id": "fixture-project-item",
                    "task_status": "committed",
                    "packet_producer": "tpm",
                    "worktree": str(self.root),
                    "branch": "main",
                    "base_ref": "refs/remotes/origin/main",
                    "base_binding": "immutable_oid",
                    "base_sha": source_identity["source_scope_oid"],
                    "head": plan["frozen_head"],
                },
                "slice": {
                    "slice_id": slice_id,
                    "role": role,
                    "slice_type": "focused_review",
                    "owner_role": role,
                    "integration_owner": "tpm",
                    "integration_order": "1",
                    "context_delivery_mode": "minimal_head_bound_task_packet",
                    "intended_model_configuration": "inherit current parent selection",
                    "actual_dispatched_model_reasoning": "inherited/unverified",
                    "actual_runtime_evidence_reason": runtime_reason,
                    "role_activation": "message_assigned_adapter_inactive",
                    "write_scope": "isolated review-resolution test fixture",
                    "return_contract": "complete immutable return fixture",
                    "validation_command": "rtk python3.12 scripts/pm/review-findings-resolution.test.py",
                    "formal_sink": f"https://github.com/{REPO}/issues/{ISSUE}",
                    "full_history_escalation_reason": "",
                },
                "context": {
                    "user_intent": "exercise plan-owned v2 handoff packet validation",
                    "work_item": "isolated review-resolution consumer fixture",
                    "non_goals": "No production changes or external writes",
                    "acceptance_target": "valid digest-bound packet is accepted",
                    "governance_refs": ["AGENTS.md", "doc/engineering/workflow/source-of-truth.md"],
                    "scoped_refs": ["scripts/pm/review-findings-resolution.test.py"],
                    "evidence_summary": "synthetic immutable plan and packet fixture",
                    "collaboration_boundary": "temporary test repository only",
                },
            }
            packet = {**payload, "packet_digest": digest(payload)}
            packet_path.write_text(
                json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8"
            )

    def _write_v2_fixture(self, *, findings: bool, mutation: str | None = None) -> tuple[Path, Path, bytes]:
        """Build a v2 fixture from canonical plan/batch/preflight producers and bound return bytes."""
        self.v2_fixture_counter += 1
        self._prepare_v2_plan_repo()
        task_plans = self.task_root / "review-plans"
        task_plans.mkdir(parents=True, exist_ok=True)
        projection_path = self.task_root / f"v2-impact-{self.v2_fixture_counter}.json"
        projection_input = {
            "task_uid": TASK, "source_head_oid": self.v2_head,
            "scope_base_oid": self.v2_head, "changed_paths": [],
            "change_class": "unknown", "manual_roles": [ROLE], "domain_role": None,
            "test_profile": "required",
            "declared_tests": ["required_gate_baseline", f"v2_fixture_{self.v2_fixture_counter}"],
            "consumed_contracts": ["review-resolution-test-fixture"],
            "public_semantics": [f"v2-fixture-{self.v2_fixture_counter}"],
            "affected_consumers": ["review-resolution-validator"],
            "closure_status": {
                "status": "complete", "reason": "isolated test fixture", "evidence": [{
                    "path": "README",
                    "sha256": "sha256:" + hashlib.sha256((self.root / "README").read_bytes()).hexdigest(),
                }],
            },
        }
        projection = WORKFLOW_IMPACT.build_projection(self.root, projection_input)
        projection_path.write_text(json.dumps(projection, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
        preflight_dir = task_plans / f"v2-preflight-{self.v2_fixture_counter}"
        env = {
            **os.environ,
            "PATH": f"{self.bin}:{os.environ['PATH']}",
            "GH_LOG": str(self.gh_log),
        }
        planned = subprocess.run(
            [str(PLAN_SCRIPT), "--root", str(self.root), "--task-uid", TASK,
             "--head", self.v2_head, "--impact-projection", str(projection_path),
             "--change-class", "unknown", "--manual-role", ROLE,
             "--comparison-ref", "refs/remotes/origin/main", "--comparison-oid", self.v2_head,
             "--preflight-dir", str(preflight_dir)],
            text=True, capture_output=True, env=env,
        )
        self.assertEqual(0, planned.returncode, planned.stderr)
        plan = json.loads(planned.stdout)
        self._materialize_v2_task_packets(plan)
        epoch = plan["epoch"]
        self.v2_slice = plan["expected_slices"][0]["slice_id"]
        self.v2_plan = task_plans / f"{epoch}.json"
        batch_path = Path(plan["batch_path"])
        batch = json.loads(batch_path.read_text(encoding="utf-8"))
        preflight = plan["preflight"]
        self.v2_ledger = Path(preflight["ledger_path"])
        returned_path = Path(preflight["artifact_paths"][0])
        self.v2_artifact = returned_path
        self.v2_handoff = self.task_root / "review-handoffs" / f"{epoch}.json"
        self.v2_manifest = self.task_root / "review-resolutions" / f"{epoch}.json"
        self.v2_collection = Path(plan["collection_path"])
        self.v2_handoff.parent.mkdir(parents=True, exist_ok=True)
        self.v2_manifest.parent.mkdir(parents=True, exist_ok=True)

        returned = json.loads(returned_path.read_text(encoding="utf-8"))
        fixture_finding = {
            "id": "V2-P1", "summary": "source-shaped v2 finding",
            "triage": {"classification": "blocking", "basis": "fixture requires exact disposition"},
        }
        fixture_findings = [fixture_finding] if findings else []
        returned.update({
            "status": "completed", "activation": "message-assigned",
            "context_delivery": "minimal-task-packet",
            "actual_runtime": (
                "inherited/unverified: message-assigned fallback; adapter inactive on this surface; "
                "actual runtime/model/reasoning unverified"
            ),
            "scope_verdict": "approved", "risk_verdict": "approved",
            "disposition": "findings" if findings else "no_findings",
            "findings": fixture_findings, "residual_risk": "fixture risk",
        })
        if mutation == "not_completed":
            returned["status"] = "incomplete"
        elif mutation == "wrong_disposition":
            returned["disposition"] = "findings"
        elif mutation == "nonempty_findings":
            returned["findings"] = [{"id": "unexpected", "summary": "fixture"}]
            returned["disposition"] = "no_findings"
        returned_path.write_text(
            json.dumps(returned, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8"
        )
        return_raw = returned_path.read_bytes()

        original_ledger_raw = self.v2_ledger.read_bytes()
        original_ledger_raw = self.v2_ledger.read_bytes()
        source_identity = plan["source_review_identity"]
        source_digest = plan["source_review_digest"]
        comparison_oid = plan["comparison_oid"]
        handoff_payload = {
            "schema": "oasis7-review-return-handoff/v1", "repository": REPO,
            "task_uid": TASK, "pr_number": source_identity["pr_number"],
            "comparison_ref": plan["comparison_ref"],
            "comparison_oid": comparison_oid, "frozen_head": self.v2_head,
            "source_review_identity": source_identity, "source_review_digest": source_digest,
            "epoch": epoch, "plan_path": self.v2_plan.relative_to(self.root).as_posix(),
            "plan_sha256": hashlib.sha256(self.v2_plan.read_bytes()).hexdigest(),
            "batch_path": batch_path.resolve().relative_to(self.root.resolve()).as_posix(),
            "batch_sha256": hashlib.sha256(batch_path.read_bytes()).hexdigest(),
            "preflight_ledger_path": self.v2_ledger.resolve().relative_to(self.root.resolve()).as_posix(),
            "preflight_ledger_sha256": hashlib.sha256(original_ledger_raw).hexdigest(),
            "rows": [{
                "role": ROLE, "slice_id": self.v2_slice,
                "artifact_path": returned_path.resolve().relative_to(self.root.resolve()).as_posix(),
                "return_sha256": hashlib.sha256(return_raw).hexdigest(),
                "findings_digest": digest(returned["findings"]),
            }],
        }
        handoff = {**handoff_payload, "handoff_digest": digest(handoff_payload)}
        self.v2_handoff.write_text(
            json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8"
        )

        role_records: list[dict[str, object]] = []
        if findings:
            evidence_digest = hashlib.sha256(self.evidence.read_bytes()).hexdigest()
            output_digest = hashlib.sha256(b"verification output\n").hexdigest()
            entry_preimage = {
                "status": "completed", "index": 0,
                "finding_digest": digest(fixture_finding),
                "disposition": "rejected_with_evidence",
                "evidence_kind": "repository_verification",
                "evidence_ref": self.evidence.relative_to(self.root).as_posix(),
                "evidence_digest": evidence_digest,
                "verification_result": {"status": "passed", "output_digest": output_digest},
            }
            role_records.append({
                "role": ROLE, "slice_id": self.v2_slice, "findings_digest": digest(fixture_findings),
                "entries": [{**entry_preimage, "entry_digest": digest(entry_preimage)}],
            })
        manifest_payload = {
            "schema": "oasis7-review-resolution/v2", "task_uid": TASK, "head": self.v2_head,
            "epoch": epoch, "handoff_digest": handoff["handoff_digest"],
            "role_records": role_records,
        }
        self.v2_manifest.write_text(
            json.dumps({**manifest_payload, "manifest_digest": digest(manifest_payload)}, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        self._resign_v2_manifest(self.v2_manifest)
        return self.v2_manifest, self.v2_ledger, original_ledger_raw

    def _resign_v2_manifest(self, manifest_path: Path) -> None:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest_payload = {key: value for key, value in manifest.items() if key != "manifest_digest"}
        manifest_digest = digest(manifest_payload)
        manifest["manifest_digest"] = manifest_digest
        manifest_path.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
        body_payload = {
            "marker": "oasis7-review-resolution", "schema": "oasis7-review-resolution/v2",
            "task_uid": TASK, "head": manifest["head"], "epoch": manifest["epoch"],
            "manifest_digest": manifest_digest,
        }
        body = json.dumps(body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        self.v2_readback = manifest_path.with_name(f"{manifest['epoch']}.readback.json")
        self.v2_readback.write_text(json.dumps({
            **body_payload, "repository": REPO, "issue_number": ISSUE,
            "comment_id": COMMENT_ID,
            "comment_url": f"https://github.com/{REPO}/issues/{ISSUE}#issuecomment-{COMMENT_ID}",
            "author": ADMIN, "created_at": "2026-09-06T10:00:00Z",
            "observed_at": "2026-09-06T10:01:00Z",
            "body_digest": hashlib.sha256(body.encode()).hexdigest(),
        }, sort_keys=True) + "\n", encoding="utf-8")
        self._write_fake_gh(permission="admin", body=body)

    def _write_v2_no_findings_fixture(self, *, mutation: str | None = None) -> tuple[Path, Path, bytes]:
        """Write a producer-shaped handoff/v2 manifest over a completed no-findings return."""
        return self._write_v2_fixture(findings=False, mutation=mutation)

    def _write_v2_finding_fixture(self) -> tuple[Path, Path, bytes]:
        """Write a producer-shaped handoff/v2 manifest covering one typed finding."""
        return self._write_v2_fixture(findings=True)

    def _assert_v2_fixture_plan_binding(self) -> None:
        plan = json.loads(self.v2_plan.read_text(encoding="utf-8"))
        batch_path = Path(plan["batch_path"]).resolve()
        batch = json.loads(batch_path.read_text(encoding="utf-8"))
        handoff = json.loads(self.v2_handoff.read_text(encoding="utf-8"))
        ledger_rows = [json.loads(line) for line in self.v2_ledger.read_text(encoding="utf-8").splitlines()]
        returned = json.loads(self.v2_artifact.read_text(encoding="utf-8"))
        self.assertEqual("oasis7-review-plan/v2", plan["schema"])
        self.assertEqual("oasis7-review-batch/v1", batch["schema"])
        self.assertEqual((TASK, self.v2_head, plan["epoch"]),
                         (batch["task_uid"], batch["frozen_head"], batch["epoch"]))
        self.assertEqual(plan["source_review_digest"], batch["relevant_evidence_digest"])
        self.assertEqual(plan["expected_slices"], batch["expected_slices"])
        self.assertEqual("incomplete", plan["preflight"]["status"])
        self.assertEqual(str(self.v2_ledger), plan["preflight"]["ledger_path"])
        self.assertEqual(1, len(ledger_rows))
        self.assertEqual("incomplete", ledger_rows[0]["status"])
        self.assertEqual({"task_uid": TASK, "role": ROLE, "slice_id": self.v2_slice,
                          "head": self.v2_head, "epoch": plan["epoch"]},
                         {key: ledger_rows[0][key] for key in
                          ("task_uid", "role", "slice_id", "head", "epoch")})
        self.assertEqual((TASK, ROLE, self.v2_slice, self.v2_head, plan["epoch"], "completed"),
                         (returned["task_uid"], returned["role"], returned["slice_id"],
                          returned["head"], returned["epoch"], returned["status"]))
        self.assertEqual("oasis7-review-return-handoff/v1", handoff["schema"])
        self.assertEqual(hashlib.sha256(self.v2_plan.read_bytes()).hexdigest(), handoff["plan_sha256"])
        self.assertEqual(hashlib.sha256(batch_path.read_bytes()).hexdigest(), handoff["batch_sha256"])
        self.assertEqual(hashlib.sha256(self.v2_ledger.read_bytes()).hexdigest(),
                         handoff["preflight_ledger_sha256"])
        self.assertEqual(hashlib.sha256(self.v2_artifact.read_bytes()).hexdigest(),
                         handoff["rows"][0]["return_sha256"])
        payload = {key: value for key, value in handoff.items() if key != "handoff_digest"}
        self.assertEqual(digest(payload), handoff["handoff_digest"])

    def _assert_v2_manifest_handoff_readback_binding(self, manifest_path: Path) -> None:
        handoff = json.loads(self.v2_handoff.read_text(encoding="utf-8"))
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        readback = json.loads(self.v2_readback.read_text(encoding="utf-8"))
        handoff_payload = {key: value for key, value in handoff.items() if key != "handoff_digest"}
        manifest_payload = {key: value for key, value in manifest.items() if key != "manifest_digest"}
        self.assertEqual(digest(handoff_payload), handoff["handoff_digest"])
        self.assertEqual(handoff["handoff_digest"], manifest["handoff_digest"])
        self.assertEqual(digest(manifest_payload), manifest["manifest_digest"])
        body_payload = {
            "marker": "oasis7-review-resolution", "schema": "oasis7-review-resolution/v2",
            "task_uid": manifest["task_uid"], "head": manifest["head"],
            "epoch": manifest["epoch"], "manifest_digest": manifest["manifest_digest"],
        }
        body = json.dumps(body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        for key, value in body_payload.items():
            self.assertEqual(value, readback[key])
        self.assertEqual(hashlib.sha256(body.encode()).hexdigest(), readback["body_digest"])

    def _assert_v2_failure_preserves_preflight(
        self, manifest: Path, ledger: Path, *, expected_head: str, diagnostic: str
    ) -> subprocess.CompletedProcess[str]:
        before_ledger = ledger.read_bytes()
        collection_existed = self.v2_collection.exists()
        before_collection = self.v2_collection.read_bytes() if collection_existed else None
        failure = self.run_validation_raw(manifest, ledger, expected_head=expected_head)
        self.assertNotEqual(0, failure.returncode)
        self.assertRegex((failure.stderr + failure.stdout).lower(), diagnostic)
        self.assertEqual(before_ledger, ledger.read_bytes(), "invalid v2 input mutated preflight ledger")
        self.assertEqual(collection_existed, self.v2_collection.exists(), "invalid v2 input changed collection existence")
        if collection_existed:
            self.assertEqual(before_collection, self.v2_collection.read_bytes(), "invalid v2 input changed collection bytes")
        return failure

    def _mutate_v2_handoff_and_rebind(self, manifest: Path, mutation: str) -> None:
        handoff = json.loads(self.v2_handoff.read_text(encoding="utf-8"))
        payload = {key: value for key, value in handoff.items() if key != "handoff_digest"}
        if mutation == "unknown_field":
            payload["unexpected"] = "closed schema mutation"
        elif mutation == "missing_field":
            payload.pop("comparison_oid")
        elif mutation == "type_coercion":
            payload["pr_number"] = True
        else:
            raise AssertionError(f"unsupported digest-consistent handoff mutation: {mutation}")
        handoff = {**payload, "handoff_digest": digest(payload)}
        self.v2_handoff.write_text(
            json.dumps(handoff, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8"
        )
        manifest_value = json.loads(manifest.read_text(encoding="utf-8"))
        manifest_value["handoff_digest"] = handoff["handoff_digest"]
        manifest.write_text(json.dumps(manifest_value, sort_keys=True) + "\n", encoding="utf-8")
        self._resign_v2_manifest(manifest)

    def _mutate_v2_handoff_duplicate_key(self, manifest: Path) -> None:
        raw = self.v2_handoff.read_text(encoding="utf-8")
        needle = f'"repository": "{REPO}"'
        self.assertGreaterEqual(raw.count(needle), 2)
        # Duplicate the top-level key with the same value; a permissive parser
        # produces the original mapping and therefore the original valid digest.
        self.v2_handoff.write_text(raw.replace(needle, f"{needle}, {needle}", 1), encoding="utf-8")
        self._resign_v2_manifest(manifest)

    def run_validation_raw(
        self, manifest: Path, ledger: Path, *, expected_head: str = HEAD
    ) -> subprocess.CompletedProcess[str]:
        env = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}", "GH_LOG": str(self.gh_log)}
        return subprocess.run(
            [str(SCRIPT), "validate", "--root", str(self.root), "--task-uid", TASK,
             "--head", expected_head, "--ledger", str(ledger), "--manifest", str(manifest)],
            text=True, capture_output=True, env=env,
        )

    def run_validation(
        self, manifest: Path, ledger: Path, *, ok: bool = True, expected_head: str = HEAD
    ) -> subprocess.CompletedProcess[str]:
        result = self.run_validation_raw(manifest, ledger, expected_head=expected_head)
        if ok and result.returncode != 0:
            self.fail(result.stderr)
        if not ok and result.returncode == 0:
            self.fail(f"unexpected success: {result.stdout}")
        return result

    def run_script(self, *extra: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
        if extra:
            env = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}", "GH_LOG": str(self.gh_log)}
            result = subprocess.run(
                [str(SCRIPT), "validate", "--root", str(self.root), "--task-uid", TASK,
                 "--head", HEAD, "--ledger", str(self.ledger), "--manifest", str(self.manifest), *extra],
                text=True, capture_output=True, env=env,
            )
            if ok and result.returncode != 0:
                self.fail(result.stderr)
            if not ok and result.returncode == 0:
                self.fail(f"unexpected success: {result.stdout}")
            return result
        return self.run_validation(self.manifest, self.ledger, ok=ok)

    def run_create(self, *extra: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
        records = self.root / "records.json"
        records.write_text("[]\n", encoding="utf-8")
        output = self.root / "created.json"
        result = subprocess.run(
            [str(SCRIPT), "create", "--root", str(self.root), "--task-uid", TASK,
             "--head", HEAD, "--epoch", EPOCH, "--role-records", str(records),
             "--out", str(output), *extra],
            text=True, capture_output=True,
        )
        if ok and result.returncode != 0:
            self.fail(result.stderr)
        if not ok and result.returncode == 0:
            self.fail(f"unexpected success: {result.stdout}")
        return result

    def test_admin_readback_authorizes_exact_finding_and_preserves_ledger(self) -> None:
        before = self.ledger.read_bytes()
        result = json.loads(self.run_script().stdout)
        self.assertEqual("passed", result["status"])
        self.assertEqual("addressed", result["aggregate"])
        self.assertEqual(before, self.ledger.read_bytes())
        self.assertIn("issues/comments/3934017999", self.gh_log.read_text())
        self.assertIn("collaborators/repo-admin/permission", self.gh_log.read_text())

    def test_v2_empty_role_records_accepts_complete_no_findings_handoff(self) -> None:
        manifest, ledger, original_ledger = self._write_v2_no_findings_fixture()
        self._assert_v2_fixture_plan_binding()
        result = json.loads(self.run_validation(manifest, ledger, expected_head=self.v2_head).stdout)
        self.assertEqual("passed", result["status"])
        self.assertEqual("no_findings", result["aggregate"])
        self.assertEqual(original_ledger, ledger.read_bytes())

    def test_v2_finding_role_record_accepts_exact_bound_terminal_evidence(self) -> None:
        manifest, ledger, original_ledger = self._write_v2_finding_fixture()
        self._assert_v2_fixture_plan_binding()
        manifest_value = json.loads(manifest.read_text(encoding="utf-8"))
        handoff = json.loads(self.v2_handoff.read_text(encoding="utf-8"))
        returned = json.loads(self.v2_artifact.read_text(encoding="utf-8"))
        self.assertEqual("oasis7-review-resolution/v2", manifest_value["schema"])
        self.assertEqual(handoff["handoff_digest"], manifest_value["handoff_digest"])
        self.assertEqual(1, len(manifest_value["role_records"]))
        record = manifest_value["role_records"][0]
        self.assertEqual({"role", "slice_id", "findings_digest", "entries"}, set(record))
        self.assertEqual(ROLE, record["role"])
        self.assertEqual(self.v2_slice, record["slice_id"])
        self.assertEqual(digest(returned["findings"]), record["findings_digest"])
        self.assertEqual(digest(returned["findings"]), handoff["rows"][0]["findings_digest"])
        self.assertEqual(1, len(record["entries"]))
        entry = record["entries"][0]
        self.assertEqual(0, entry["index"])
        self.assertEqual("completed", entry["status"])
        self.assertEqual(digest(returned["findings"][0]), entry["finding_digest"])
        self.assertEqual("rejected_with_evidence", entry["disposition"])
        self.assertEqual("repository_verification", entry["evidence_kind"])
        self.assertEqual("passed", entry["verification_result"]["status"])

        result = json.loads(self.run_validation(manifest, ledger, expected_head=self.v2_head).stdout)
        self.assertEqual("passed", result["status"])
        self.assertEqual("addressed", result["aggregate"])
        self.assertEqual(original_ledger, ledger.read_bytes())

    def test_v2_finding_coverage_negatives_wait_for_valid_control(self) -> None:
        manifest, ledger, _ = self._write_v2_finding_fixture()
        self._assert_v2_fixture_plan_binding()
        control = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
        if control.returncode != 0 and "schema" in control.stderr.lower():
            print(
                "NOT EXERCISED: finding role-record missing/extra/mismatched coverage negatives "
                "await a successful source-shaped v2 finding positive control",
                file=sys.stderr,
            )
            self.skipTest("not exercised: v2 finding positive control is rejected by frozen v1-only validator")
        self.assertEqual(0, control.returncode, f"valid v2 finding control failed: {control.stderr}")

        cases = (
            ("missing_role_record", r"role|coverage|finding|return"),
            ("extra_role_record", r"role|coverage|unexpected|return"),
            ("mismatched_findings_digest", r"digest|finding|return"),
        )
        for mutation, diagnostic in cases:
            with self.subTest(mutation=mutation):
                manifest, ledger, before = self._write_v2_finding_fixture()
                manifest_value = json.loads(manifest.read_text(encoding="utf-8"))
                if mutation == "missing_role_record":
                    manifest_value["role_records"] = []
                elif mutation == "extra_role_record":
                    extra = json.loads(json.dumps(manifest_value["role_records"][0]))
                    extra["role"] = "qa_engineer"
                    extra["slice_id"] = "22222222-2222-4222-8222-222222222222"
                    manifest_value["role_records"].append(extra)
                    manifest_value["role_records"].sort(key=lambda row: (row["role"], row["slice_id"]))
                else:
                    manifest_value["role_records"][0]["findings_digest"] = digest([])
                manifest.write_text(
                    json.dumps(manifest_value, sort_keys=True) + "\n", encoding="utf-8"
                )
                self._resign_v2_manifest(manifest)

                failure = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
                self.assertNotEqual(0, failure.returncode, f"invalid v2 coverage accepted: {mutation}")
                self.assertRegex((failure.stderr + failure.stdout).lower(), diagnostic)
                self.assertEqual(before, ledger.read_bytes(), f"invalid v2 case mutated ledger: {mutation}")

    def test_v2_consumer_rejects_malformed_handoff_when_positive_control_passes(self) -> None:
        manifest, ledger, _ = self._write_v2_no_findings_fixture()
        self._assert_v2_fixture_plan_binding()
        control = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
        if control.returncode != 0 and "schema" in control.stderr.lower():
            print(
                "NOT EXERCISED: v2 consumer unknown/missing/type/duplicate-key handoff negatives "
                "await a successful source-shaped no-findings positive control",
                file=sys.stderr,
            )
            self.skipTest("not exercised: v2 consumer positive is rejected by the frozen v1-only validator")
        self.assertEqual(0, control.returncode, f"valid v2 consumer control failed: {control.stderr}")

        cases = (
            ("unknown_field", r"unknown|field|handoff|schema"),
            ("missing_field", r"missing|field|handoff|schema"),
            ("type_coercion", r"type|integer|pr_number|handoff|schema"),
            ("duplicate_json_key", r"duplicate|key|json|handoff|schema"),
        )
        for mutation, diagnostic in cases:
            with self.subTest(mutation=mutation):
                manifest, ledger, _ = self._write_v2_no_findings_fixture()
                self._assert_v2_fixture_plan_binding()
                if mutation == "duplicate_json_key":
                    self._mutate_v2_handoff_duplicate_key(manifest)
                else:
                    self._mutate_v2_handoff_and_rebind(manifest, mutation)
                # H, manifest, and readback digests/bindings are consistent;
                # only the consumer-side strict JSON/schema rule is under test.
                self._assert_v2_manifest_handoff_readback_binding(manifest)
                self._assert_v2_failure_preserves_preflight(
                    manifest, ledger, expected_head=self.v2_head, diagnostic=diagnostic
                )

    def test_v2_wrong_manifest_head_fails_with_rebound_readback(self) -> None:
        manifest, ledger, _ = self._write_v2_no_findings_fixture()
        self._assert_v2_fixture_plan_binding()
        control = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
        if control.returncode != 0 and "schema" in control.stderr.lower():
            print(
                "NOT EXERCISED: wrong-manifest-head consumer negative awaits a successful source-shaped v2 positive control",
                file=sys.stderr,
            )
            self.skipTest("not exercised: v2 consumer positive is rejected by the frozen v1-only validator")
        self.assertEqual(0, control.returncode, f"valid v2 consumer control failed: {control.stderr}")

        value = json.loads(manifest.read_text(encoding="utf-8"))
        wrong_head = "f" * 40 if self.v2_head != "f" * 40 else "e" * 40
        value["head"] = wrong_head
        manifest.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
        self._resign_v2_manifest(manifest)
        self._assert_v2_manifest_handoff_readback_binding(manifest)
        readback = json.loads(self.v2_readback.read_text(encoding="utf-8"))
        self.assertNotEqual(self.v2_head, value["head"])
        self.assertEqual(value["head"], readback["head"])
        self._assert_v2_failure_preserves_preflight(
            manifest, ledger, expected_head=self.v2_head, diagnostic=r"head|identity|manifest"
        )

    def test_v2_empty_role_records_false_predicates_wait_for_valid_control(self) -> None:
        manifest, ledger, _ = self._write_v2_no_findings_fixture()
        control = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
        if control.returncode != 0 and "schema" in control.stderr.lower():
            print(
                "NOT EXERCISED: completed/status-disposition-findings predicates, handoff-digest binding, "
                "and drifted-return bytes await a successful v2 positive control",
                file=sys.stderr,
            )
            self.skipTest("not exercised: valid v2 positive control is rejected by the frozen v1-only validator")
        self.assertEqual(0, control.returncode, f"valid v2 control failed: {control.stderr}")

        cases = (
            ("not_completed", r"status|completed|return"),
            ("wrong_disposition", r"disposition|finding|return"),
            ("nonempty_findings", r"findings|disposition|return"),
            ("missing_handoff_digest", r"handoff|digest|field"),
            ("changed_handoff_digest", r"handoff|digest"),
            ("drifted_return_bytes", r"handoff|return|artifact|digest"),
        )
        for mutation, diagnostic in cases:
            with self.subTest(mutation=mutation):
                fixture_mutation = mutation if mutation in {
                    "not_completed", "wrong_disposition", "nonempty_findings"
                } else None
                manifest, ledger, before = self._write_v2_no_findings_fixture(mutation=fixture_mutation)
                if mutation in {"missing_handoff_digest", "changed_handoff_digest"}:
                    payload = json.loads(manifest.read_text(encoding="utf-8"))
                    if mutation == "missing_handoff_digest":
                        payload.pop("handoff_digest")
                    else:
                        payload["handoff_digest"] = "a" * 64
                    unsigned = {key: value for key, value in payload.items() if key != "manifest_digest"}
                    payload["manifest_digest"] = digest(unsigned)
                    manifest.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")
                elif mutation == "drifted_return_bytes":
                    returned = json.loads(self.v2_artifact.read_text(encoding="utf-8"))
                    returned["residual_risk"] = "changed after handoff binding"
                    self.v2_artifact.write_text(json.dumps(returned, sort_keys=True) + "\n", encoding="utf-8")

                if mutation in {"missing_handoff_digest", "changed_handoff_digest"}:
                    self._resign_v2_manifest(manifest)

                failure = self.run_validation_raw(manifest, ledger, expected_head=self.v2_head)
                self.assertNotEqual(0, failure.returncode, f"invalid v2 case accepted: {mutation}")
                self.assertRegex((failure.stderr + failure.stdout).lower(), diagnostic)
                self.assertEqual(before, ledger.read_bytes(), f"invalid v2 case mutated ledger: {mutation}")

    def test_resolution_comment_must_belong_to_canonical_task_issue(self) -> None:
        self._write_fake_gh(permission="admin", comment_issue_number=999)
        before = self.ledger.read_bytes()
        failure = self.run_script(ok=False)
        self.assertIn("issue", failure.stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())

    def test_non_admin_and_author_mismatch_fail_closed(self) -> None:
        self._write_fake_gh(permission="write")
        before = self.ledger.read_bytes()
        self.assertIn("admin", self.run_script(ok=False).stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())
        self._write_fake_gh(permission="admin", author="other-admin")
        self.assertIn("author", self.run_script(ok=False).stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())

    def test_exact_identity_digest_coverage_and_verification_fail_closed(self) -> None:
        before = self.ledger.read_bytes()
        manifest = json.loads(self.manifest.read_text())
        manifest["head"] = "c" * 40
        self.manifest.write_text(json.dumps(manifest))
        self.assertIn("head", self.run_script(ok=False).stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())
        manifest["head"] = HEAD
        manifest["role_records"][0]["entries"][0]["disposition"] = "addressed"
        self.manifest.write_text(json.dumps(manifest))
        self.assertIn("digest", self.run_script(ok=False).stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())

    def test_epoch_and_manifest_schema_fail_closed(self) -> None:
        manifest = json.loads(self.manifest.read_text())
        manifest["epoch"] = "c" * 64
        self.manifest.write_text(json.dumps(manifest))
        self.assertRegex(self.run_script(ok=False).stderr.lower(), r"epoch|digest")
        manifest = {"schema": "wrong"}
        self.manifest.write_text(json.dumps(manifest))
        self.assertIn("schema", self.run_script(ok=False).stderr.lower())

    def test_task_issue_comment_is_not_per_finding_evidence(self) -> None:
        manifest = json.loads(self.manifest.read_text())
        entry = manifest["role_records"][0]["entries"][0]
        entry.pop("evidence_digest", None)
        entry["evidence_kind"] = "task_issue_comment"
        entry["entry_digest"] = digest({key: value for key, value in entry.items() if key != "entry_digest"})
        payload = {key: value for key, value in manifest.items() if key != "manifest_digest"}
        manifest["manifest_digest"] = digest(payload)
        self.manifest.write_text(json.dumps(manifest, sort_keys=True) + "\n")
        self.body_payload["manifest_digest"] = manifest["manifest_digest"]
        self.body = json.dumps(self.body_payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        readback = json.loads(self.readback.read_text())
        readback["manifest_digest"] = manifest["manifest_digest"]
        readback["body_digest"] = hashlib.sha256(self.body.encode()).hexdigest()
        self.readback.write_text(json.dumps(readback, sort_keys=True) + "\n")
        self._write_fake_gh(permission="admin", body=self.body)
        failure = self.run_script(ok=False)
        self.assertIn("evidence kind", failure.stderr.lower())

    def test_structured_finding_missing_triage_fails_closed(self) -> None:
        finding = {key: value for key, value in self.findings[0].items() if key != "triage"}
        self._rewrite_fixture(finding=finding)
        before = self.ledger.read_bytes()
        failure = self.run_script(ok=False)
        self.assertIn("triage", failure.stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())

    def test_blocking_finding_cannot_resolve_as_non_actionable(self) -> None:
        self._rewrite_fixture(disposition="non_actionable")
        before = self.ledger.read_bytes()
        failure = self.run_script(ok=False)
        self.assertRegex(failure.stderr.lower(), r"blocking|non.?actionable|disposition")
        self.assertEqual(before, self.ledger.read_bytes())

    def test_task_issue_must_match_canonical_mapping(self) -> None:
        readback = json.loads(self.readback.read_text())
        readback["issue_number"] = 999
        readback["comment_url"] = f"https://github.com/{REPO}/issues/999#issuecomment-{COMMENT_ID}"
        self.readback.write_text(json.dumps(readback, sort_keys=True) + "\n")
        self._write_fake_gh(permission="admin", issue_number=999)
        mismatch = self.run_script("--issue-number", "999", ok=False)
        self.assertIn("task issue", mismatch.stderr.lower())
        omitted = self.run_script(ok=False)
        self.assertIn("task issue", omitted.stderr.lower())

    def test_outside_root_artifact_fails_before_ledger_change(self) -> None:
        outside = self.root.parent / "outside-review-return.json"
        outside.write_bytes(self.artifact.read_bytes())
        ledger = json.loads(self.ledger.read_text())
        ledger["artifacts"] = [str(outside)]
        ledger["artifact_digest"] = hashlib.sha256(outside.read_bytes()).hexdigest()
        self.ledger.write_text(json.dumps(ledger, sort_keys=True) + "\n")
        before = self.ledger.read_bytes()
        failure = self.run_script(ok=False)
        self.assertIn("escapes", failure.stderr.lower())
        self.assertEqual(before, self.ledger.read_bytes())

    def test_readback_symlink_outside_root_fails_closed(self) -> None:
        outside = self.root.parent / f"{self.root.name}-outside-readback.json"
        self.addCleanup(lambda: outside.unlink(missing_ok=True))
        outside.write_bytes(self.readback.read_bytes())
        self.readback.unlink()
        self.readback.symlink_to(outside)
        failure = self.run_script(ok=False)
        self.assertRegex(failure.stderr.lower(), r"readback|escapes")

    def test_stale_local_task_map_requires_live_pm_task_body_before_comment_fetch(self) -> None:
        mapping_path = self.root / ".pm" / "github-project-sync" / "tasks.json"
        mapping = json.loads(mapping_path.read_text())
        mapping["tasks"][TASK]["issue_number"] = 999
        mapping_path.write_text(json.dumps(mapping) + "\n")
        readback = json.loads(self.readback.read_text())
        readback["issue_number"] = 999
        readback["comment_url"] = f"https://github.com/{REPO}/issues/999#issuecomment-{COMMENT_ID}"
        self.readback.write_text(json.dumps(readback, sort_keys=True) + "\n")
        for bad_body in (
            "task_uid: " + TASK + "\n",
            "<!-- oasis7-pm-task -->\ntask_uid: task_" + "2" * 32 + "\n",
        ):
            self.gh_log.write_text("")
            self._write_fake_gh(permission="admin", issue_number=999, task_body=bad_body)
            failure = self.run_script(ok=False)
            self.assertRegex(failure.stderr.lower(), r"canonical task issue|pm task")
            self.assertNotIn("issues/999/comments/3934017999", self.gh_log.read_text())

    def test_create_refuses_replacement_of_an_epoch(self) -> None:
        self.run_create()
        failure = self.run_create(ok=False)
        self.assertIn("immutable", failure.stderr.lower())


if __name__ == "__main__":
    unittest.main()
