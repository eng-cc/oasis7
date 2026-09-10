import base64
import importlib.util
import io
import json
from pathlib import Path
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


if __name__ == '__main__': unittest.main()
