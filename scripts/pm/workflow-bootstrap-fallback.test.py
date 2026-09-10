#!/usr/bin/env python3
"""Execute the actual old-base workflow inline fallback without candidate imports."""
import contextlib
import io
import json
from pathlib import Path
import re
import sys
import os
import subprocess
import tempfile
import unittest
from unittest.mock import patch

UID = 'task_' + 'a' * 32
OTHER = 'task_' + 'b' * 32
WORKFLOW = Path(__file__).resolve().parents[2] / '.github/workflows/rust.yml'


class BootstrapFallback(unittest.TestCase):
    def execute(self, pr_body, *, hits=None, issue_body=None, comments=None, second_uid=OTHER):
        source = WORKFLOW.read_text()
        code = re.search(r"<<'PY'\n(.*?)\n          PY", source, re.S)[1]
        code = '\n'.join(line[10:] for line in code.splitlines())
        calls = []

        def gh(args, **kwargs):
            calls.append(args)
            if args[1:3] == ['issue', 'list']:
                return json.dumps([{'number': 3644}] if hits is None else hits)
            endpoint = args[2]
            if endpoint.endswith('/pulls/3645'):
                return json.dumps({'body': pr_body})
            if endpoint.endswith('/comments'):
                return json.dumps([comments or []])
            if endpoint.endswith('/issues/3647'):
                return json.dumps({'body': f'task_uid: {second_uid}\n- pr_number: `3648`'})
            if endpoint.endswith('/issues/3644'):
                return json.dumps({'body': issue_body if issue_body is not None else
                                   f'task_uid: {UID}\n- pr_number: `3645`'})
            raise AssertionError(f'unexpected request: {args}')

        with patch.object(sys, 'argv', ['-', 'eng-cc/oasis7', '3645']), \
                patch('subprocess.check_output', side_effect=gh), \
                contextlib.redirect_stdout(io.StringIO()):
            exec(compile(code, str(WORKFLOW), 'exec'), {})
        return calls

    def test_uid_allows_absorbed_refs(self):
        self.execute(f'Task UID: {UID}\nRefs #3647\nRefs #3644')

    def test_multiple_uids_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID} {OTHER}\nRefs #3644')

    def test_unresolved_and_ambiguous_uid_rejected(self):
        for hits in ([], [{'number': 3644}, {'number': 3647}]):
            with self.subTest(hits=hits), self.assertRaises(SystemExit):
                self.execute(f'{UID}', hits=hits, second_uid=UID)

    def test_two_canonical_refs_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID}\nRefs #3644\nRefs #3647', second_uid=UID)

    def test_search_incidental_reference_filtered(self):
        self.execute(UID, hits=[{'number': 3647}, {'number': 3644}])

    def test_search_budget_exhaustion_blocks(self):
        with self.assertRaises(SystemExit):
            self.execute(UID, hits=[{'number': 3644}] * 5)

    def test_wrong_uid_reverse_binding_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID}\nRefs #3644', issue_body=f'task_uid: {OTHER}\n- pr_number: `3645`')

    def test_wrong_pr_reverse_binding_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID}\nRefs #3644', issue_body=f'task_uid: {UID}\n- pr_number: `36450`')

    def test_legacy_unique_ref_preserved(self):
        self.execute('Refs #3644')

    def test_malformed_second_uid_blocks_all_identity_routes(self):
        for pr_body in (UID, UID+'\nRefs #3644', 'Refs #3644'):
            with self.subTest(pr_body=pr_body), self.assertRaises(SystemExit):
                self.execute(pr_body,issue_body=f'task_uid: {UID}\ntask_uid: malformed\n- pr_number: `3645`')

    def test_legacy_multiple_refs_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute('Refs #3644\nRefs #3647')

    def test_loop_history_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID}\nRefs #3644', comments=[{'body': 'oasis7-loop-binding-history'}])

    def test_duplicate_reverse_fields_rejected(self):
        for body in (f'task_uid: {UID}\ntask_uid: {OTHER}\n- pr_number: `3645`',
                     f'task_uid: {UID}\n- pr_number: `3645`\n- pr_number: `3646`'):
            with self.subTest(body=body), self.assertRaises(SystemExit):
                self.execute(f'{UID}\nRefs #3644', issue_body=body)

    def test_existing_loop_binding_rejected(self):
        with self.assertRaises(SystemExit):
            self.execute(f'{UID}\nRefs #3644', issue_body=f'task_uid: {UID}\n- pr_number: `3645`\nloop_binding_b64: value')

    def test_read_failure_does_not_fallback_to_refs(self):
        with self.assertRaises(AssertionError):
            self.execute(f'{UID}', hits=[{'number': 9999}])

    def test_refs_work_when_search_empty(self):
        calls = self.execute(f'{UID}\nRefs #3647\nRefs #3644', hits=[])
        self.assertFalse(any(call[1:3] == ['issue', 'list'] for call in calls))

    def test_fallback_has_no_candidate_execution(self):
        calls = self.execute(f'{UID}\nRefs #3644')
        self.assertTrue(all(call[0] == 'gh' for call in calls))

    def test_run_title_exact_binding_format(self):
        self.assertIn('run-name: oasis7-ci|${{ github.event_name }}|${{ inputs.run_mode }}|${{ inputs.task_uid }}|${{ inputs.pr_number }}|${{ inputs.integration_base }}|${{ inputs.expected_head }}', WORKFLOW.read_text())

    def test_actual_interpreter_excludes_candidate_stdlib_shadow(self):
        source = WORKFLOW.read_text()
        invocation, block = re.search(r"(python3 [^\n]*) <<'PY'\n(.*?)\n          PY", source, re.S).groups()
        self.assertTrue(invocation.startswith('python3 -I - '))
        code = '\n'.join(line[10:] for line in block.splitlines())
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'json.py').write_text("raise RuntimeError('CANDIDATE EXECUTED')\n")
            gh = root / 'gh'
            gh.write_text('#!/bin/sh\ncase "$2" in\n*/pulls/*) echo \'{"body":"Refs #3644"}\';;\n*/comments) echo \'[[]]\';;\n*) cat <<\'JSON\'\n' + json.dumps({'body': f'task_uid: {UID}\n- pr_number: `3645`'}) + '\nJSON\n;;\nesac\n')
            gh.chmod(0o755)
            result = subprocess.run([sys.executable, '-I', '-', 'eng-cc/oasis7', '3645'], input=code,
                                    text=True, capture_output=True, cwd=root,
                                    env={**os.environ, 'PATH': str(root) + os.pathsep + os.environ['PATH']})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('live legacy bootstrap', result.stdout)


if __name__ == '__main__':
    unittest.main()
