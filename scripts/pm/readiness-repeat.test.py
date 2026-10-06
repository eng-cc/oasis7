#!/usr/bin/env python3
"""RED: repeatable native claims and publication-scoped publisher permission.

Only offline GitHub seams are extended; producer and validator are real.
The original readiness-transport.test.py remains byte-immutable.
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    "immutable_readiness_fixture", ROOT / "scripts/pm/readiness-transport.test.py")
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
protocol = original.protocol


class RepeatFixture(original.NativeClaimFixture):
    def __init__(self, parent: Path):
        super().__init__(parent)
        gh = self.bin / "gh"
        text = gh.read_text()
        text = text.replace("c={'id':901,", "c={'id':901+len(s['comments']),")
        text = text.replace(
            "'html_url':'https://github.com/fixture/repo/issues/11#issuecomment-901'",
            "'html_url':f\"https://github.com/fixture/repo/issues/11#issuecomment-{901+len(s['comments'])}\"")
        text = text.replace(
            "elif e.endswith('/issues/comments/901'): out(s['comments'][-1])",
            "elif '/issues/comments/' in e:\n"
            "        matches=[c for c in s['comments'] if c['id']==int(e.rsplit('/',1)[1])]\n"
            "        if len(matches)!=1: raise SystemExit('missing exact comment ID')\n"
            "        out(matches[0])")
        gh.write_text(text)

    def receipt_root(self) -> Path:
        raw = subprocess.check_output([sys.executable, str(self.tools / "canonical-receipt-root.py"),
            "--default-worktree", str(self.root), "--task-uid", self.uid], text=True)
        return Path(raw.strip())

    def artifacts(self) -> dict[str, bytes]:
        root = self.receipt_root()
        return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}

    def advance_head(self):
        (self.root / "new-source.txt").write_text("accepted source changes\n")
        self.git("add", "new-source.txt")
        self.git("commit", "-qm", "new accepted head")
        self.head = self.git("rev-parse", "HEAD")
        self.tree = self.git("rev-parse", "HEAD^{tree}")
        state = json.loads(self.state_path.read_text())
        state["pr"]["head"]["sha"] = self.head
        self.state_path.write_text(json.dumps(state))


class PublisherPermissionFixture(protocol.DeliveryFixture):
    def _install_gh_stub(self):
        super()._install_gh_stub()
        gh = self.bin / "gh"
        text = gh.read_text().replace(
            'out({"permission":"admin","user":{"login":"fixture"}})',
            'out({"permission":os.environ.get("BOUND_PUBLISHER_PERMISSION","admin"),'
            '"user":{"login":"fixture"}})')
        gh.write_text(text)


class RepeatClaimRed(unittest.TestCase):
    def exercise_repeat(self, *, changed_head: bool):
        with tempfile.TemporaryDirectory(prefix="oasis7-repeat-readiness-") as temp:
            fixture = RepeatFixture(Path(temp))
            first, first_raw, _ = fixture.run()
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            first_result = json.loads(first.stdout)
            self.assertEqual(first_result["status"], "verified")
            before = fixture.artifacts()
            self.assertTrue(before, "successful first claim did not persist native artifacts")
            first_binding = first_result["readiness_binding"]
            if changed_head:
                fixture.advance_head()
            second, second_raw, _ = fixture.run(age=0)
            self.assertNotEqual(first_raw, second_raw)
            self.assertEqual(second.returncode, 0,
                "repeatable native ready_for_merge was blocked by an older Task binding: " + second.stderr)
            second_result = json.loads(second.stdout)
            second_binding = second_result["readiness_binding"]
            self.assertNotEqual(first_binding, second_binding)
            self.assertEqual(first_binding["head_oid"] != second_binding["head_oid"], changed_head)
            after = fixture.artifacts()
            for name, raw in before.items():
                self.assertEqual(after[name], raw, "older native artifact was overwritten: " + name)
            server = json.loads(fixture.state_path.read_text())
            self.assertEqual(len(server["comments"]), 2)
            self.assertNotEqual(server["comments"][0]["id"], server["comments"][1]["id"])
            for result, raw, stdout in ((first_result, first_raw, first.stdout),
                                        (second_result, second_raw, second.stdout)):
                binding = result["readiness_binding"]
                key = hashlib.sha256(protocol.canonical(binding)).hexdigest()
                path = fixture.receipt_root() / "native-readiness" / key
                self.assertEqual((path / "gate.stdout").read_bytes(), raw)
                self.assertEqual((path / "result.stdout").read_bytes(), stdout.encode())
                self.assertEqual((path / "comment.json").read_bytes(),
                                 protocol.canonical(result["readiness_comment"]))

    def test_second_same_head_claim_keeps_earlier_binding_bytes(self):
        self.exercise_repeat(changed_head=False)

    def test_new_head_claim_keeps_earlier_binding_bytes(self):
        self.exercise_repeat(changed_head=True)


class PublisherPermissionRed(unittest.TestCase):
    def test_postmerge_exact_proof_survives_original_publisher_permission_loss(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-revoked-") as temp:
            fixture = PublisherPermissionFixture(Path(temp))
            fixture.prepare_native_v2_readiness()
            proof = fixture.receipt_root / "readiness-proof.json"
            proof_before = proof.read_bytes()
            server_before = fixture.state_path.read_bytes()
            mapping_before = fixture.mapping_path.read_bytes()
            env = fixture.env()
            # The original publisher's collaborator permission changes. Exact
            # author/content/server merge do not; this read-only consumer is not
            # a terminal write or a waiver of current terminal-writer authority.
            env["BOUND_PUBLISHER_PERMISSION"] = "read"
            proc = subprocess.run([sys.executable, str(fixture.pm_tools / "readiness_transport.py"),
                "--repo-root", str(fixture.root), "--task-uid", protocol.UID, "--create"],
                text=True, capture_output=True, env=env)
            self.assertEqual(proc.returncode, 0,
                "postmerge artifact consumer added an enduring original-publisher permission gate: " + proc.stderr)
            self.assertEqual(json.loads(proc.stdout)["status"], "passed")
            self.assertEqual(proof.read_bytes(), proof_before)
            self.assertEqual(fixture.state_path.read_bytes(), server_before)
            self.assertEqual(fixture.mapping_path.read_bytes(), mapping_before)


class OpenPreflightRed(unittest.TestCase):
    def test_open_pr_finalizer_preflight_is_read_only_and_ready(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-open-preflight-") as temp:
            fixture = protocol.DeliveryFixture(Path(temp))
            fixture.prepare_native_v2_readiness()
            # Start from independently validated native artifacts, then project
            # the server's premerge phase. A terminal proof cannot exist yet.
            (fixture.receipt_root / "readiness-proof.json").unlink()
            state = json.loads(fixture.state_path.read_text())
            state["pr"].update(state="OPEN", merged=False, merged_at=None,
                               merge_commit_sha=None)
            fixture.state_path.write_text(json.dumps(state))
            before_server = fixture.state_path.read_bytes()
            before_mapping = fixture.mapping_path.read_bytes()
            before_artifacts = {str(p.relative_to(fixture.receipt_root)): p.read_bytes()
                                for p in fixture.receipt_root.rglob("*") if p.is_file()}
            proc = fixture.run_finalizer("--preflight")
            self.assertEqual(proc.returncode, 0,
                "OPEN premerge finalizer preflight incorrectly requires terminal merged readiness: " + proc.stderr)
            self.assertEqual(json.loads(proc.stdout)["status"], "ready")
            self.assertEqual(fixture.state_path.read_bytes(), before_server)
            self.assertEqual(fixture.mapping_path.read_bytes(), before_mapping)
            self.assertEqual({str(p.relative_to(fixture.receipt_root)): p.read_bytes()
                              for p in fixture.receipt_root.rglob("*") if p.is_file()}, before_artifacts)


if __name__ == "__main__":
    unittest.main(verbosity=2)
