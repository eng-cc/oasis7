import base64
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('loop_ci', Path(__file__).with_name('loop-ci.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
UID = 'task_' + 'a' * 32


class CIGateTests(unittest.TestCase):
    def test_hosted_path_does_not_require_project_token_or_local_admission(self):
        source = Path(module.__file__).read_text()
        self.assertNotIn('OASIS7_LOOP_READ_TOKEN', source)
        self.assertNotIn('from loop import validate_task', source)
        self.assertIn('validate_ci_content', source)

    def invoke(self, body, head='b' * 40, history=None):
        issue = json.dumps({'body': 'task_uid: ' + UID + '\n- pr_number: `2`\n' + body})
        responses = [json.dumps({'body': UID, 'head': {'sha': head}}), json.dumps([{'number': 1}]), issue, issue, json.dumps(history or [])]
        with patch.object(module, 'run', side_effect=responses), patch('sys.argv', ['loop-ci.py', '--repository', 'fixture/repo', '--pr-number', '2', '--base', 'a' * 40, '--head', 'b' * 40]), patch('sys.stdout', new_callable=io.StringIO):
            return module.main()

    def test_live_legacy_passes(self):
        self.assertEqual(self.invoke(''), 0)

    def test_exact_uid_fields_across_search_refs_and_legacy(self):
        for pr_body in (UID, UID+'\nRefs #1', 'Refs #1'):
            for extra in ('', 'task_uid: malformed', 'task_uid: '+UID):
                with self.subTest(pr_body=pr_body,extra=extra):
                    def live(*args):
                        if args[1:3] == ('issue','list'): return json.dumps([{'number':1}])
                        if '/pulls/' in args[2]: return json.dumps({'body':pr_body,'head':{'sha':'b'*40}})
                        if args[2].endswith('/comments'): return '[]'
                        return json.dumps({'body':f'task_uid: {UID}\n{extra}\n- pr_number: `2`'})
                    with patch.object(module,'run',side_effect=live), patch('sys.argv',['loop-ci.py','--repository','fixture/repo','--pr-number','2','--base','a'*40,'--head','b'*40]), patch('sys.stdout',new_callable=io.StringIO):
                        self.assertEqual(module.main(),2 if extra else 0)

    def test_direct_refs_identity_cases(self):
        for second_uid, reverse, expected in [('task_' + 'c' * 32, '2', 0), (UID, '2', 2), ('task_' + 'c' * 32, '3', 2)]:
            with self.subTest(second_uid=second_uid, reverse=reverse):
                def live(*args):
                    if args[1:3] == ('issue', 'list'):
                        raise AssertionError('direct Refs must not require search')
                    endpoint = args[2]
                    if '/pulls/' in endpoint:
                        return json.dumps({'body': UID + '\nRefs #1\nRefs #9', 'head': {'sha': 'b' * 40}})
                    if endpoint.endswith('/comments'): return '[]'
                    selected = UID if endpoint.endswith('/1') else second_uid
                    return json.dumps({'body': f'task_uid: {selected}\n- pr_number: `{reverse}`'})
                with patch.object(module, 'run', side_effect=live), patch('sys.argv', ['loop-ci.py', '--repository', 'fixture/repo', '--pr-number', '2', '--base', 'a' * 40, '--head', 'b' * 40]), patch('sys.stdout', new_callable=io.StringIO):
                    self.assertEqual(module.main(), expected)

    def test_live_head_drift_blocks(self):
        self.assertEqual(self.invoke('', head='c' * 40), 2)

    def test_deleted_binding_cannot_become_legacy(self):
        self.assertEqual(self.invoke('', history=[[{'body': '<!-- oasis7-loop-binding-history -->'}]]), 2)

    def test_malformed_binding_cannot_pass_as_legacy(self):
        self.assertEqual(self.invoke('loop_binding_b64: malformed'), 2)

    def test_loop_without_effective_policy_fails_closed(self):
        encoded = base64.urlsafe_b64encode(json.dumps({'task_uid': UID, 'loop': 'code'}).encode()).decode()
        self.assertEqual(self.invoke('- loop_binding_b64: `' + encoded + '`'), 2)

    def test_bound_hosted_caller_supplies_verified_record_projection(self):
        """Run the real CI caller through a bound task's hosted path."""
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Path(tmp)
            root = fixture / "repo"
            root.mkdir()
            helpers = root / "scripts" / "pm"
            helpers.mkdir(parents=True)
            source_helpers = Path(module.__file__).parent
            for name in (
                "loop-ci.py",
                "loop_ci_content.py",
                "loop_contracts.py",
                "loop_policy.py",
                "loop_traceability.py",
                "loop-policy.v1.json",
            ):
                shutil.copy2(source_helpers / name, helpers / name)
            (root / "doc" / "engineering").mkdir(parents=True)
            (root / "doc" / "engineering" / "spec.md").write_text(
                "<a id=\"acceptance\"></a>\n# Acceptance\n", encoding="utf-8"
            )

            def git(*args):
                return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

            git("init", "-q", "-b", "main")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.invalid")
            git("add", ".")
            git("commit", "-qm", "bound hosted fixture")
            head = git("rev-parse", "HEAD")
            policy_digest = "sha256:" + hashlib.sha256(
                (helpers / "loop-policy.v1.json").read_bytes()
            ).hexdigest()

            record = {
                "schema": "oasis7.loop-change/v1",
                "marker": "oasis7-loop-change-record",
                "task_uid": UID,
                "change_id": "hosted-record",
                "coordination_ref": {
                    "repository": "eng-cc/oasis7",
                    "issue_number": 3671,
                    "comment_id": 5636938574,
                    "record_digest": "",
                    "source_commit": head,
                },
                "required_obligations": [{
                    "obligation_id": "hosted-obligation",
                    "mapping_slot": "hosted-slot",
                    "required": True,
                    "acceptance_refs": [{
                        "repository": "eng-cc/oasis7",
                        "path": "doc/engineering/spec.md",
                        "fragment": "acceptance",
                        "contract_id": "engineering-workflow",
                        "revision": "v1.15.2",
                        "contract_digest": "sha256:" + "1" * 64,
                        "publication_ref": {
                            "repository": "eng-cc/oasis7",
                            "issue_number": 3671,
                            "comment_id": 5636639918,
                        },
                    }],
                    "owner_loop": "code",
                    "owner_role": "repository_health_engineer",
                }],
                "mapping_slots": [{
                    "slot_id": "hosted-slot",
                    "owner_loop": "code",
                    "owner_role": "repository_health_engineer",
                }],
                "candidate_selection": {
                    "comparable_fields": [
                        "change_id", "source_head_oid", "integration_base_oid", "tested_tree_oid",
                        "configuration_digest", "entry", "environment", "evidence_window",
                        "effective_policy_identity", "effective_helper_identity",
                        "effective_workflow_identity", "consumed_contracts",
                    ],
                    "required_integration_base_oid": head,
                    "required_tested_tree_oid": head,
                },
                "feedback": [],
            }
            canonical = lambda value: json.dumps(
                value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
            )
            unsigned = json.loads(json.dumps(record))
            unsigned["coordination_ref"]["record_digest"] = ""
            record["coordination_ref"]["record_digest"] = "sha256:" + hashlib.sha256(
                canonical(unsigned).encode("utf-8")
            ).hexdigest()
            encoded_binding = base64.urlsafe_b64encode(json.dumps({
                "schema": "oasis7.loop-task/v1",
                "task_uid": UID,
                "change_id": "hosted-record",
                "loop": "code",
                "owner_role": "repository_health_engineer",
                "bootstrap_epoch": 1,
                "manual_request_ref": "issuecomment-5636938574",
                "request_key": "hosted-record/hosted-slot",
                "write_scope": ["scripts/pm/**"],
                "out_of_scope": [],
                "acceptance_refs": ["hosted-obligation"],
                "dependencies": [],
                "input_contracts": [],
                "target_delivery": "pilot",
                "policy_commit": head,
                "policy_digest": policy_digest,
                "coordination_ref": record["coordination_ref"],
            }, sort_keys=True).encode()).decode().rstrip("=")
            issue = {
                "number": 1,
                "body": "<!-- oasis7-pm-task -->\n"
                        f"task_uid: {UID}\n- pr_number: `2`\n"
                        f"- loop_binding_b64: `{encoded_binding}`\n",
            }
            publication = {
                "id": 5636938574,
                "issue_url": "https://api.github.com/repos/eng-cc/oasis7/issues/3671",
                "user": {"login": "coordinator"},
                "created_at": "2026-09-12T00:00:00Z",
                "body": canonical({
                    "marker": "oasis7-loop-change-record",
                    "schema": "oasis7.loop-change/v1",
                    "task_uid": UID,
                    "change_id": record["change_id"],
                    "record_digest": record["coordination_ref"]["record_digest"],
                    "record": record,
                }),
            }
            coordination_issue = {
                "number": 3671,
                "html_url": "https://github.com/eng-cc/oasis7/issues/3671",
                "body": "<!-- oasis7-pm-task -->\n"
                        f"task_uid: {UID}\n",
            }
            pr = {"number": 2, "head": {"sha": head}, "body": f"task_uid: {UID}\nRefs #1"}
            (fixture / "issue.json").write_text(json.dumps(issue), encoding="utf-8")
            (fixture / "coordination-issue.json").write_text(
                json.dumps(coordination_issue), encoding="utf-8"
            )
            (fixture / "coordination-comments.json").write_text(
                json.dumps([[publication]]), encoding="utf-8"
            )
            (fixture / "pr.json").write_text(json.dumps(pr), encoding="utf-8")
            (fixture / "publication.json").write_text(json.dumps(publication), encoding="utf-8")
            calls = fixture / "authority-calls.log"
            fake_bin = fixture / "bin"
            fake_bin.mkdir()
            fake_gh = fake_bin / "gh"
            fake_gh.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, sys\n"
                "from pathlib import Path\n"
                "fixture = Path(os.environ['GH_FIXTURE'])\n"
                "path = sys.argv[2] if len(sys.argv) > 2 else ''\n"
                "with (fixture / 'authority-calls.log').open('a', encoding='utf-8') as handle:\n"
                "    handle.write(path + '\\n')\n"
                "if path.endswith('/pulls/2'):\n"
                "    name = 'pr.json'\n"
                "elif path.endswith('/issues/1'):\n"
                "    name = 'issue.json'\n"
                "elif path.endswith('/issues/3671'):\n"
                "    name = 'coordination-issue.json'\n"
                "elif path.endswith('/issues/comments/5636938574'):\n"
                "    name = 'publication.json'\n"
                "elif path.endswith('/issues/3671/comments?per_page=100'):\n"
                "    name = 'coordination-comments.json'\n"
                "else:\n"
                "    raise SystemExit('unexpected gh path: ' + path)\n"
                "print((fixture / name).read_text())\n",
                encoding="utf-8",
            )
            fake_gh.chmod(0o755)
            remote = fixture / "remote.git"
            subprocess.run(["git", "init", "-q", "--bare", str(remote)], check=True)
            git("remote", "add", "origin", str(remote))
            git("push", "-q", "origin", "HEAD:main")
            result = subprocess.run(
                [
                    sys.executable,
                    str(source_helpers / "loop-ci.py"),
                    "--repo-root", str(root),
                    "--repository", "eng-cc/oasis7",
                    "--pr-number", "2",
                    "--base", head,
                    "--head", head,
                ],
                env={
                    **os.environ,
                    "PATH": str(fake_bin) + os.pathsep + os.environ.get("PATH", ""),
                    "GH_FIXTURE": str(fixture),
                    "PYTHONPYCACHEPREFIX": "/",
                },
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            observed_calls = calls.read_text(encoding="utf-8").splitlines()
            self.assertEqual(
                observed_calls,
                [
                    "repos/eng-cc/oasis7/pulls/2",
                    "repos/eng-cc/oasis7/issues/1",
                    "repos/eng-cc/oasis7/issues/3671",
                    "repos/eng-cc/oasis7/issues/comments/5636938574",
                    "repos/eng-cc/oasis7/issues/3671/comments?per_page=100",
                ],
            )

            caller_source = (source_helpers / "loop-ci.py").read_text(encoding="utf-8")
            self.assertIn("validate_ci_content", caller_source)
            for prohibited in (
                "OASIS7_LOOP_READ_TOKEN",
                "tasks.json",
                "FixtureReaders",
                "validate_task",
                "validate_aggregate",
                "Project API",
                "merge authorization",
            ):
                self.assertNotIn(prohibited, caller_source)

            original_publication = json.loads(
                (fixture / "publication.json").read_text(encoding="utf-8")
            )
            original_coordination_issue = json.loads(
                (fixture / "coordination-issue.json").read_text(encoding="utf-8")
            )

            def run_bound_case():
                calls.unlink(missing_ok=True)
                return subprocess.run(
                    [
                        sys.executable,
                        str(source_helpers / "loop-ci.py"),
                        "--repo-root", str(root),
                        "--repository", "eng-cc/oasis7",
                        "--pr-number", "2",
                        "--base", head,
                        "--head", head,
                    ],
                    env={
                        **os.environ,
                        "PATH": str(fake_bin) + os.pathsep + os.environ.get("PATH", ""),
                        "GH_FIXTURE": str(fixture),
                        "PYTHONPYCACHEPREFIX": "/",
                    },
                    text=True,
                    capture_output=True,
                )

            def write_publication(value):
                (fixture / "publication.json").write_text(
                    json.dumps(value), encoding="utf-8"
                )

            def write_coordination_issue(value):
                (fixture / "coordination-issue.json").write_text(
                    json.dumps(value), encoding="utf-8"
                )

            cases = {
                "missing comment": lambda: write_publication({
                    "id": 0,
                    "issue_url": publication["issue_url"],
                    "body": "",
                }),
                "foreign Issue URL": lambda: write_publication({
                    **original_publication,
                    "issue_url": "https://api.github.com/repos/foreign/repo/issues/3671",
                }),
                "wrong comment ID": lambda: write_publication({
                    **original_publication,
                    "id": 999,
                }),
                "task UID mismatch": lambda: write_coordination_issue({
                    **original_coordination_issue,
                    "body": "<!-- oasis7-pm-task -->\n"
                            "task_uid: task_" + "f" * 32 + "\n",
                }),
                "changed record body": lambda: write_publication({
                    **original_publication,
                    "body": canonical({
                        **json.loads(original_publication["body"]),
                        "record_digest": "sha256:" + "2" * 64,
                    }),
                }),
                "malformed record": lambda: write_publication({
                    **original_publication,
                    "body": "not-json",
                }),
                "no usable projection": lambda: write_publication({
                    **original_publication,
                    "body": canonical({
                        "marker": "oasis7-loop-change-record",
                        "schema": "oasis7.loop-change/v1",
                        "task_uid": UID,
                        "change_id": record["change_id"],
                        "record_digest": record["coordination_ref"]["record_digest"],
                        "record": None,
                    }),
                }),
            }
            for label, mutate in cases.items():
                with self.subTest(case=label):
                    (fixture / "coordination-issue.json").write_text(
                        json.dumps(original_coordination_issue), encoding="utf-8"
                    )
                    write_publication(original_publication)
                    mutate()
                    negative = run_bound_case()
                    output = negative.stdout + negative.stderr
                    self.assertEqual(negative.returncode, 2, output)
                    self.assertTrue(output.strip(), output)
                    observed = calls.read_text(encoding="utf-8").splitlines()
                    self.assertTrue(observed, output)
                    self.assertTrue(
                        set(observed).issubset({
                            "repos/eng-cc/oasis7/pulls/2",
                            "repos/eng-cc/oasis7/issues/1",
                            "repos/eng-cc/oasis7/issues/3671",
                            "repos/eng-cc/oasis7/issues/comments/5636938574",
                            "repos/eng-cc/oasis7/issues/3671/comments?per_page=100",
                        }),
                        observed,
                    )


if __name__ == '__main__': unittest.main()
