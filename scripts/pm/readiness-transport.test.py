#!/usr/bin/env python3
"""RED: real claim/finalizer entrypoints must bind native readiness artifacts.

The live-gate boundary and GitHub transport are deterministic offline fixtures.
Claim publication, result serialization and delivery effects are production code.
Existing protocol fixtures are imported, never edited or upgraded in place.
"""
from __future__ import annotations

import base64
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import readiness_transport
import loop_terminal

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    "readiness_delivery_fixture", ROOT / "scripts/pm/terminal-delivery-protocol.test.py")
protocol = importlib.util.module_from_spec(spec)
spec.loader.exec_module(protocol)


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


class TypedProjectReadinessRed(unittest.TestCase):
    def test_actual_query_projection_reaches_missing_readiness_after_repository_readback(self):
        with tempfile.TemporaryDirectory() as temp:
            fixture = protocol.DeliveryFixture(Path(temp))
            record, issue, item, pr, live_repo, comments = fixture.live_inputs()
            item = copy.deepcopy(item)
            item['fieldValues']['nodes'] = [node for node in item['fieldValues']['nodes']
                if (node.get('field') or {}).get('name') != 'Repository']
            for node in item['fieldValues']['nodes']:
                node['__typename'] = ('ProjectV2ItemFieldTextValue' if 'text' in node
                                      else 'ProjectV2ItemFieldSingleSelectValue')
            repository_node = {'__typename': 'ProjectV2ItemFieldRepositoryValue',
                'field': {'name': 'Repository'}, 'repository': {'nameWithOwner': protocol.REPOSITORY}}
            queries = []
            def graphql(*args):
                if args[0] == 'project':
                    return {'id': item['project']['id']}
                query = next(arg for arg in args if arg.startswith('query='))
                queries.append(query)
                projected = copy.deepcopy(item)
                # Official union projection: an unsupported branch yields {}.
                projected['fieldValues']['nodes'].append(repository_node if
                    'ProjectV2ItemFieldRepositoryValue' in query else {})
                return {'data': {'repository': {'issue': {'projectItems': {
                    'pageInfo': {'hasNextPage': False}, 'nodes': [projected]}}}}}
            def api(repo, endpoint):
                if endpoint.endswith('/issues/11'): return issue
                if endpoint.endswith('/pulls/12'): return pr
                self.fail(endpoint)
            before = (fixture.mapping_path.read_bytes(), fixture.state_path.read_bytes(),
                {str(p.relative_to(fixture.receipt_root)): p.read_bytes()
                 for p in fixture.receipt_root.rglob('*') if p.is_file()})
            with patch.object(readiness_transport, 'query', side_effect=api), \
                    patch.object(readiness_transport, 'read_comments', return_value=comments), \
                    patch.object(loop_terminal, '_json', side_effect=graphql), \
                    patch('terminal_proof.read_live_repository', return_value=live_repo):
                with self.assertRaisesRegex(ValueError,
                        'required readiness proof/native artifacts/unique migration unavailable'):
                    readiness_transport.create_readiness_proof(fixture.root, protocol.UID, write=True)
            self.assertTrue(queries)
            self.assertIn('__typename', queries[0])
            self.assertIn('repository', queries[0])
            self.assertIn('nameWithOwner', queries[0])
            self.assertEqual(before, (fixture.mapping_path.read_bytes(), fixture.state_path.read_bytes(),
                {str(p.relative_to(fixture.receipt_root)): p.read_bytes()
                 for p in fixture.receipt_root.rglob('*') if p.is_file()}))


class NativeClaimFixture:
    """Same offline external-reader seam used by claim-ready-ready-pr.test.sh."""

    def __init__(self, parent: Path):
        self.root = parent / "native"
        self.root.mkdir()
        self.bin = parent / "native-bin"
        self.bin.mkdir()
        self.state_path = parent / "native-server.json"
        self.uid = protocol.UID
        self.repo = protocol.REPOSITORY
        self.issue = protocol.ISSUE
        self.pr = protocol.PR
        self.tools = self.root / "scripts/pm"
        shutil.copytree(ROOT / "scripts/pm", self.tools)
        # Stub only the live-gate reader. The production claim script remains
        # byte-identical and runs its real supplied/live comparison and writer.
        (self.tools / "pr-lifecycle-gate.py").write_text(
            "import os,pathlib\nprint(pathlib.Path(os.environ['READINESS_LIVE_GATE']).read_text(),end='')\n")
        (self.root / ".gitignore").write_text(".pm/\n")
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Readiness Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "remote.origin.url", f"https://github.com/{self.repo}.git")
        self.git("add", ".")
        self.git("commit", "-qm", "immutable fixture tool source")
        self.head = self.git("rev-parse", "HEAD")
        self.tree = self.git("rev-parse", "HEAD^{tree}")
        mapping_path = self.root / ".pm/github-project-sync/tasks.json"
        mapping_path.parent.mkdir(parents=True)
        record = {"task_uid": self.uid, "repository": self.repo, "status": "ready",
                  "issue_number": self.issue, "issue_url": protocol.ISSUE_URL,
                  "pr_number": self.pr, "pr_url": protocol.PR_URL,
                  "canonical_worktree": str(self.root), "task_branch": "main",
                  "default_branch": "main", "owner_role": "repository_health_engineer"}
        mapping_path.write_text(json.dumps({"project": {"repo": self.repo}, "tasks": {self.uid: record}}))
        self.mapping_path = mapping_path
        self.state_path.write_text(json.dumps({"comments": [], "issue": {
            "number": self.issue, "html_url": protocol.ISSUE_URL, "state": "OPEN",
            "body": f"task_uid: {self.uid}\n- pr_number: `{self.pr}`\n- pr_url: `{protocol.PR_URL}`\n"},
            "pr": {"number": self.pr, "html_url": protocol.PR_URL,
                   "body": f"Task: {self.uid}\nRefs #{self.issue}", "state": "OPEN", "merged": False,
                   "head": {"sha": self.head, "ref": "main", "repo": {"full_name": self.repo}},
                   "base": {"sha": self.head, "ref": "main", "repo": {"full_name": self.repo}}}}))
        # No real gh/network path can be reached: unknown calls fail closed.
        gh = self.bin / "gh"
        gh.write_text('''#!/usr/bin/env python3
import datetime as dt,json,os,pathlib,re,sys
a=sys.argv[1:]; p=pathlib.Path(os.environ['READINESS_SERVER']); s=json.loads(p.read_text())
def out(x): print(json.dumps(x))
if a[:2]==['issue','comment']:
    body=pathlib.Path(a[a.index('--body-file')+1]).read_text()
    now=dt.datetime.now(dt.timezone.utc).isoformat().replace('+00:00','Z')
    c={'id':901,'body':body,'user':{'login':'fixture'},'created_at':now,'updated_at':now,
       'html_url':'https://github.com/fixture/repo/issues/11#issuecomment-901'}
    if os.environ.get('READINESS_NO_READBACK')!='1':
        s['comments'].append(c); p.write_text(json.dumps(s))
    print(c['html_url'])
elif a[:1]==['api']:
    e=next((x for x in a[1:] if x.startswith('repos/') or x=='user'), '')
    if e.endswith('/collaborators/fixture/permission'): out({'permission':'admin','user':{'login':'fixture'}})
    elif e=='repos/fixture/repo': out({'full_name':'fixture/repo','default_branch':'main','permissions':{'admin':True}})
    elif e.endswith('/issues/11/comments'): out([s['comments']] if '--slurp' in a else s['comments'])
    elif e.endswith('/issues/comments/901'): out(s['comments'][-1])
    elif e.endswith('/issues/11'): out(s['issue'])
    elif e.endswith('/pulls/12'): out(s['pr'])
    elif e.endswith('/git/ref/heads/main'): out({'object':{'sha':s['pr']['head']['sha']}})
    elif e=='user': out({'login':'fixture'})
    else: raise SystemExit('unsupported offline gh: '+repr(a))
elif a[:2]==['auth','status']: print('fixture authenticated')
elif a[:2]==['issue','view']: out(s['issue'])
elif a[:2]==['pr','view']: out(s['pr'])
else: raise SystemExit('unsupported offline gh: '+repr(a))
''')
        gh.chmod(0o755)
        self.input_path = self.root / ".pm/input-gate.json"
        self.live_path = self.root / ".pm/live-gate.json"

    def git(self, *args: str) -> str:
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def gate(self, age: int) -> dict:
        now = dt.datetime.now(dt.timezone.utc)
        policy = {"status": "resolved", "label": "审查", "ci_digest": "sha256:" + "a" * 64}
        preimage = {"repository": self.repo, "pr_number": self.pr, "head_oid": self.head,
                    "blockers": [], "policy": policy, "hold": None}
        # Match the legacy serializer, including non-ASCII escaping and nested
        # prefix preservation; raw stdout below deliberately uses another form.
        epoch = digest(json.dumps(preimage, sort_keys=True, separators=(",", ":")).encode())
        return {"evidence_mode": "production", "ready_for_merge": True, "status": "ready",
                "blockers": [], "policy_discovery": policy, "merge_hold": None,
                "pr_number": self.pr, "pr_url": protocol.PR_URL,
                "readiness_receipt": {"receipt_type": "oasis7_pr_lifecycle_ready",
                    "issuer": "oasis7_pr_lifecycle_gate/v1", "repository": self.repo,
                    "pr_number": self.pr, "head_oid": self.head,
                    "observed_at": (now-dt.timedelta(seconds=age)).isoformat(), "gate_epoch": epoch}}

    def run(self, age: int = -20, no_readback: bool = False) -> tuple[subprocess.CompletedProcess, bytes, dict]:
        supplied = self.gate(age)
        raw = (json.dumps(supplied, ensure_ascii=False, indent=2) + "\n").encode()
        self.input_path.write_bytes(raw)
        live = self.gate(0)
        self.live_path.write_text(json.dumps(live))
        env = dict(os.environ)
        for name in ("GH_TOKEN", "GITHUB_TOKEN", "GITHUB_ENTERPRISE_TOKEN"):
            env.pop(name, None)
        env.update(PATH=str(self.bin)+os.pathsep+env.get("PATH", ""), PM_ROOT_DIR=str(self.root),
                   READINESS_SERVER=str(self.state_path), READINESS_LIVE_GATE=str(self.live_path), TZ="Asia/Shanghai")
        env["READINESS_NO_READBACK"] = "1" if no_readback else "0"
        proc = subprocess.run(["bash", str(self.tools / "claim-ready.sh"), "--claim-type", "ready_for_merge",
                "--verification-profile", "repository_required", "--task-uid", self.uid,
                "--pr-gate-json", str(self.input_path), "--json"],
                cwd=self.root, env=env, text=True, capture_output=True)
        return proc, raw, supplied


class NativeBindingRed(unittest.TestCase):
    def test_publication_url_without_server_comment_cannot_report_success(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-readiness-unreadable-") as temp:
            fixture = NativeClaimFixture(Path(temp))
            proc, _, _ = fixture.run(no_readback=True)
            self.assertEqual(json.loads(fixture.state_path.read_text())["comments"], [])
            self.assertNotEqual(proc.returncode, 0,
                "native claim reports success from publication URL without any server comment readback: "+proc.stdout)
            self.assertNotIn('"allowed_to_claim": true', proc.stdout)

    def test_successful_manual_claim_persists_exact_binding_and_server_readback(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-readiness-native-") as temp:
            fixture = NativeClaimFixture(Path(temp))
            proc, raw, gate = fixture.run()
            self.assertEqual(proc.returncode, 0, proc.stdout+proc.stderr)
            result = json.loads(proc.stdout)
            # Successful existing manual baseline, with offset time and a fresh
            # same-epoch internal rerun whose timestamp/raw bytes differ.
            self.assertEqual(result["status"], "verified")
            self.assertTrue(result["allowed_to_claim"])
            self.assertEqual(result["verification_exit_code"], 0)
            self.assertTrue(result["verified_at"].endswith("+08:00"), result)
            self.assertEqual(fixture.input_path.read_bytes(), raw)
            self.assertNotEqual(raw, fixture.live_path.read_bytes())
            server = json.loads(fixture.state_path.read_text())
            self.assertEqual(len(server["comments"]), 1, server)
            binding = result.get("readiness_binding")
            self.assertIsInstance(binding, dict,
                "successful native ready_for_merge has no persisted exact raw-gate readiness_binding")
            self.assertEqual(set(binding), {"schema", "task_uid", "repository", "issue_number", "pr_number",
                "pr_url", "head_oid", "claim_type", "status", "exit_code", "verified_at", "gate_raw_sha256",
                "gate_epoch", "gate_observed_at", "source"})
            self.assertEqual(binding["gate_raw_sha256"], digest(raw))
            self.assertEqual(binding["gate_epoch"], gate["readiness_receipt"]["gate_epoch"])
            self.assertEqual(binding["gate_observed_at"], gate["readiness_receipt"]["observed_at"])
            self.assertEqual(binding["head_oid"], fixture.head)
            self.assertEqual(binding["task_uid"], fixture.uid)
            self.assertEqual(binding["repository"], fixture.repo)
            self.assertEqual(binding["issue_number"], fixture.issue)
            self.assertEqual(binding["pr_number"], fixture.pr)
            self.assertEqual(binding["pr_url"], protocol.PR_URL)
            self.assertEqual((binding["claim_type"], binding["status"], binding["exit_code"]),
                             ("ready_for_merge", "verified", 0))
            self.assertEqual(binding["schema"], "oasis7-native-readiness/v2")
            self.assertEqual(binding["verified_at"], result["verified_at"])
            self.assertEqual(binding["source"], {"oid": fixture.head, "tree_oid": fixture.tree,
                "gate_entry_sha256": digest((fixture.tools/"pr-lifecycle-gate.py").read_bytes()),
                "claim_entry_sha256": digest((fixture.tools/"claim-ready.sh").read_bytes())})
            comment = server["comments"][0]
            expected_body = "<!-- oasis7-native-readiness/v2 -->\n" + protocol.canonical(binding).decode()
            self.assertEqual(comment["body"], expected_body)
            readback = result.get("readiness_comment")
            self.assertIsInstance(readback, dict, "successful native claim lacks unique server readback")
            self.assertEqual(set(readback), {"id", "body_b64", "body_sha256", "author", "created_at", "updated_at"})
            self.assertEqual(readback["id"], comment["id"])
            self.assertEqual(base64.b64decode(readback["body_b64"]), comment["body"].encode())
            self.assertEqual(readback["body_sha256"], digest(comment["body"].encode()))
            self.assertEqual(readback["author"], comment["user"]["login"])
            self.assertEqual(readback["created_at"], comment["created_at"])
            self.assertEqual(readback["updated_at"], comment["updated_at"])


class DeliveryBindingRed(unittest.TestCase):
    def test_missing_readiness_proof_rejects_delivery_before_any_terminal_effect(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-readiness-delivery-") as temp:
            fixture = protocol.DeliveryFixture(Path(temp))
            def snapshot():
                return {"mapping": fixture.mapping_path.read_bytes(), "server": fixture.state_path.read_bytes(),
                        "receipts": {p.name:p.read_bytes() for p in fixture.receipt_root.iterdir() if p.is_file()}}
            before = snapshot()
            self.assertFalse((fixture.receipt_root / "readiness-proof.json").exists())
            proc = fixture.run_producer()
            self.assertNotEqual(proc.returncode, 0,
                "v2 terminal producer accepted absent required readiness proof and wrote terminal delivery: "+proc.stdout)
            self.assertIn("readiness", (proc.stdout+proc.stderr).lower())
            self.assertEqual(snapshot(), before,
                "missing readiness proof changed Task/Project/comments/receipt/ledger/tombstone sinks")


class TaskCompleteMarkerRed(unittest.TestCase):
    def _reject_without_effects(self, naked_marker: bool, terminal_phase: bool = False):
        with tempfile.TemporaryDirectory(prefix="oasis7-complete-marker-") as temp:
            fixture = NativeClaimFixture(Path(temp))
            # Commit accepted H in the canonical disposable task checkout;
            # failure must come from readiness, never a dirty-source refusal.
            (fixture.root / "accepted-task.txt").write_text("accepted task change\n")
            fixture.git("add", "accepted-task.txt")
            fixture.git("commit", "-qm", "accepted task head")
            fixture.head = fixture.git("rev-parse", "HEAD")
            server = json.loads(fixture.state_path.read_text())
            server["pr"]["head"]["sha"] = fixture.head
            server["pr"].update(state="CLOSED", merged=True)
            fixture.state_path.write_text(json.dumps(server))
            mapping = json.loads(fixture.mapping_path.read_text())
            record = mapping["tasks"][fixture.uid]
            record.update(status="done", workflow_phase="task_done", completion_mode="single_pr")
            if terminal_phase:
                record["workflow_phase"] = "post_merge_done"
            if naked_marker:
                record["phase_receipts"] = {"post_merge_done": {
                    "receipt_type": "oasis7_terminal_cleanup"}}
            fixture.mapping_path.write_text(json.dumps(mapping))
            receipt_root = Path(subprocess.check_output([sys.executable,
                str(fixture.tools / "canonical-receipt-root.py"),
                "--default-worktree", str(fixture.root), "--task-uid", fixture.uid,
                "--create"], text=True).strip())
            self.assertEqual(fixture.git("status", "--porcelain"), "")
            self.assertFalse((receipt_root / "readiness-proof.json").exists())
            self.assertFalse((receipt_root / "terminal-cleanup-receipt.json").exists())
            def snapshot():
                return (fixture.mapping_path.read_bytes(), fixture.state_path.read_bytes(),
                    {str(p.relative_to(receipt_root)): p.read_bytes()
                     for p in receipt_root.rglob("*") if p.is_file()})
            before = snapshot()
            env = dict(os.environ)
            for name in ("GH_TOKEN", "GITHUB_TOKEN", "GITHUB_ENTERPRISE_TOKEN"):
                env.pop(name, None)
            env.update(PATH=str(fixture.bin)+os.pathsep+env.get("PATH", ""),
                PM_ROOT_DIR=str(fixture.root), READINESS_SERVER=str(fixture.state_path))
            result = subprocess.run(["bash", str(fixture.tools / "claim-ready.sh"),
                "--claim-type", "task_complete", "--verification-profile", "repository_required",
                "--task-uid", fixture.uid, "--json"], cwd=fixture.root,
                env=env, text=True, capture_output=True)
            with self.subTest(invariant="readiness refusal"):
                self.assertNotEqual(result.returncode, 0,
                    "naked v1 marker bypassed readiness and published accepted-H completion: "
                    + result.stdout + result.stderr)
                self.assertIn("readiness", (result.stdout + result.stderr).lower())
                self.assertNotIn('"allowed_to_claim": true', result.stdout)
            with self.subTest(invariant="zero terminal effects"):
                self.assertEqual(snapshot(), before,
                    "unproved v1 marker changed mapping, server comments, or receipt sinks")

    def test_no_marker_rejects_missing_readiness_before_publication(self):
        self._reject_without_effects(False)

    def test_naked_v1_marker_rejects_missing_readiness_before_publication(self):
        self._reject_without_effects(True)

    def test_naked_v1_marker_with_forged_terminal_phase_rejects_before_publication(self):
        self._reject_without_effects(True, terminal_phase=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
