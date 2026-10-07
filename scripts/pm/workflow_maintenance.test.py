"""Human maintenance authorization and immutable code-under-test boundaries."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import workflow_maintenance as maintenance

UID = 'task_' + 'a' * 32
HEAD = 'b' * 40

class MaintenanceTests(unittest.TestCase):
    def fixture(self):
        scope = dict(repository='fixture/repo', task_uid=UID, issue_number=1, pr_number=2,
                     purpose='candidate-tool-verification',
                     allowed_write_paths=['scripts/pm/loop-ci.py'],
                     allowed_tool_paths=list(maintenance.TOOL_PATHS))
        comment = dict(id=7, body=maintenance.MARKER + '\n```json\n' + json.dumps(scope) + '\n```',
                       issue_url='https://api.github.com/repos/fixture/repo/issues/1',
                       html_url='https://github.com/fixture/repo/issues/1#issuecomment-7',
                       user=dict(login='human', type='User'), created_at='2026-10-07T00:00:00Z',
                       updated_at='2026-10-07T00:00:00Z')
        permission = dict(permission='admin', user=dict(login='human'))
        task = dict(number=1, state='open', html_url='https://github.com/fixture/repo/issues/1',
                    body=f'task_uid: {UID}\n- merge_hold_active: `false`\n')
        pr = dict(number=2, state='open', merged_at=None, draft=True,
                  html_url='https://github.com/fixture/repo/pull/2',
                  body=f'Task: {UID}\nRefs #1\nWorkflow Maintenance Authority: 7\n',
                  head=dict(sha=HEAD, ref='task/repair', repo=dict(full_name='fixture/repo')),
                  base=dict(ref='main', repo=dict(full_name='fixture/repo')))
        return scope, comment, permission, task, pr

    def validate(self, comment, permission, task, pr, head=HEAD):
        return maintenance.validate_maintenance_authority(comment, permission, task, pr, head,
            ('scripts/pm/loop-ci.py',), maintenance.TOOL_PATHS)

    def test_admin_scope_selects_exact_event_head_with_repeated_matching_uid(self):
        _, comment, permission, task, pr = self.fixture()
        pr['body'] += '\nEvidence: ' + UID
        result = self.validate(comment, permission, task, pr)
        self.assertEqual(HEAD, result['tool_revision'])
        self.assertEqual(7, result['comment_id'])
        self.assertEqual(7, maintenance.maintenance_comment_id(pr['body']))
        self.assertEqual('main', pr['base']['ref'])

    def test_live_authority_drift_and_holds_fail_closed(self):
        scope, comment, permission, task, pr = self.fixture()
        mutations = (
            lambda c,p,t,r: c.update(updated_at='edited'),
            lambda c,p,t,r: c['user'].update(type='Bot'),
            lambda c,p,t,r: c.update(issue_url='https://api.github.com/repos/fixture/repo/issues/9'),
            lambda c,p,t,r: p.update(permission='write'),
            lambda c,p,t,r: p['user'].update(login='other'),
            lambda c,p,t,r: t.update(body=f'task_uid: {UID}\n- merge_hold_active: `true`'),
            lambda c,p,t,r: t.update(state='closed'),
            lambda c,p,t,r: r['head'].update(sha='c' * 40),
            lambda c,p,t,r: r['head']['repo'].update(full_name='other/repo'),
            lambda c,p,t,r: r.update(body=r['body'] + '\nTask: malformed'),
            lambda c,p,t,r: r.update(body=r['body'] + '\nTask: ' + UID),
            lambda c,p,t,r: r.update(body=r['body'] + '\nEvidence: task_' + 'c' * 32),
            lambda c,p,t,r: r.update(body=r['body'].replace('Refs #1', 'Refs #9')),
        )
        for mutate in mutations:
            values = copy.deepcopy((comment, permission, task, pr))
            mutate(*values)
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                self.validate(*values)
        for key in ('allowed_write_paths', 'allowed_tool_paths'):
            changed = copy.deepcopy(scope); changed[key] = ['doc/unrelated.md']
            changed_comment = {**comment, 'body': maintenance.MARKER + json.dumps(changed)}
            with self.subTest(scope=key), self.assertRaises(ValueError):
                self.validate(changed_comment, permission, task, pr)

    def test_locator_and_scope_payload_cannot_supply_permission(self):
        for body in ('Workflow Maintenance Authority: 0',
                     'Workflow Maintenance Authority: 7\nWorkflow Maintenance Authority: 7'):
            with self.subTest(body=body), self.assertRaises(ValueError):
                maintenance.maintenance_comment_id(body)
        scope, comment, permission, task, pr = self.fixture()
        for payload in ({**scope, 'authority_oid': HEAD},
                        {**scope, 'allowed_write_paths': ['scripts/pm/*']},
                        {**scope, 'allowed_tool_paths': ['../loop-ci.py']}):
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                maintenance.parse_maintenance_authority(maintenance.MARKER + json.dumps(payload))
        duplicate = json.dumps(scope)[:-1] + ', "task_uid": "' + UID + '"}'
        with self.assertRaises(ValueError):
            maintenance.parse_maintenance_authority(maintenance.MARKER + duplicate)

    def test_immutable_tool_bytes_and_import_shadow_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q'); git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            for relative in maintenance.TOOL_PATHS:
                path = root / relative; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('# immutable fixture\n')
            git('add', '.'); git('commit', '-qm', 'immutable helper closure')
            authority = dict(tool_revision=git('rev-parse', 'HEAD'),
                             allowed_tool_paths=list(maintenance.TOOL_PATHS))
            maintenance.validate_candidate_tool_root(root, root, authority)
            helper = root / 'scripts/pm/loop-ci.py'
            helper.write_text('# drift\n')
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, authority)
            helper.write_text('# immutable fixture\n')
            (root / 'scripts/pm/json.py').write_text('raise RuntimeError("shadow")\n')
            with self.assertRaises(ValueError):
                maintenance.validate_candidate_tool_root(root, root, authority)

if __name__ == '__main__': unittest.main()
