#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("review-findings-resolution.py")
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
        self.manifest.parent.mkdir()
        self.ledger = self.task_root / "slice-ledger.jsonl"
        self.gh_log = self.root / "gh.log"
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.fake_gh = self.bin / "gh"
        self._write_fixture()
        self._write_fake_gh(permission="admin")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _write_fake_gh(self, *, permission: str, author: str = ADMIN, body: str | None = None,
                       issue_number: int = ISSUE, task_body: str | None = None) -> None:
        if body is None:
            body = self.body
        if task_body is None:
            task_body = f"<!-- oasis7-pm-task -->\ntask_uid: {TASK}\n"
        self.fake_gh.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "args = sys.argv[1:]\n"
            "open(os.environ['GH_LOG'], 'a').write(' '.join(args) + '\\n')\n"
            f"if args[:2] != ['api', 'repos/eng-cc/oasis7/issues/{issue_number}'] and args[:2] != ['api', 'repos/eng-cc/oasis7/issues/{issue_number}/comments/3934017999'] and args[:2] != ['api', 'repos/eng-cc/oasis7/collaborators/{author}/permission']:\n"
            "    raise SystemExit('unexpected gh call: ' + ' '.join(args))\n"
            f"if args[1] == 'repos/eng-cc/oasis7/issues/{issue_number}':\n"
            "    print(json.dumps({'number': " + str(issue_number) + ", 'body': " + repr(task_body) + "}))\n"
            "elif 'comments' in args[1]:\n"
            "    print(json.dumps({'id': 3934017999, 'body': " + repr(body) + ", 'user': {'login': " + repr(author) + "}, 'created_at': '2026-09-06T10:00:00Z'}))\n"
            "else:\n"
            "    print(json.dumps({'permission': " + repr(permission) + "}))\n",
            encoding="utf-8",
        )
        self.fake_gh.chmod(0o755)

    def _write_fixture(self) -> None:
        finding = {"id": "P1", "summary": "evidence-backed finding"}
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

    def run_script(self, *extra: str, ok: bool = True) -> subprocess.CompletedProcess[str]:
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
        self.assertIn("issues/3615/comments/3934017999", self.gh_log.read_text())
        self.assertIn("collaborators/repo-admin/permission", self.gh_log.read_text())

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
