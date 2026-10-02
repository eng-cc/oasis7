import sys
from pathlib import Path
import unittest
from types import SimpleNamespace
sys.path.insert(0, str(Path(__file__).parent))
from loop_gate import admission, live_binding
from unittest.mock import Mock, patch
import json


class GateTests(unittest.TestCase):
    def test_live_legacy_passes(self):
        self.assertEqual(admission(Path('.'), {}, 'base', 'head', reader=lambda _: None)['status'], 'legacy')

    def test_cache_deletion_cannot_downgrade_live_loop(self):
        with self.assertRaisesRegex(ValueError, 'cache differs'):
            admission(Path('.'), {}, 'base', 'head', reader=lambda _: {'loop': 'code'})

    def test_missing_effective_helper_blocks_loop(self):
        binding = {'loop': 'code'}
        with patch('loop_gate.importlib.util.spec_from_file_location') as load_candidate:
            with self.assertRaisesRegex(ValueError, 'trusted'):
                admission(Path('.'), {'loop_binding': binding}, 'base', 'head', reader=lambda _: binding)
            load_candidate.assert_not_called()

    def test_configured_trusted_root_is_required_before_facade_load(self):
        binding = {'loop': 'code'}
        facade = SimpleNamespace(validate_task=Mock(return_value={'status': 'passed', 'blockers': []}))
        loader = SimpleNamespace(exec_module=Mock())
        spec = SimpleNamespace(loader=loader)
        with patch('loop_gate.importlib.util.spec_from_file_location', return_value=spec) as load_candidate, \
             patch('loop_gate.importlib.util.module_from_spec', return_value=facade):
            result = admission(Path('/repo'), {'loop_binding': binding}, 'base', 'head',
                               tool_root=Path('/trusted'), reader=lambda _: binding)
        self.assertEqual(result['status'], 'passed')
        load_candidate.assert_called_once()
        facade.validate_task.assert_called_once_with(
            Path('/repo'), {'loop_binding': binding}, Path('/trusted'), 'base', 'head')


    def test_legacy_live_identity_requires_one_exact_canonical_field(self):
        uid='task_'+'a'*32
        for body in ['previous '+uid,'task_uid: task_'+'b'*32+'\nprevious '+uid,'task_uid: '+uid+'\ntask_uid: '+uid,'task_uid: '+uid+'\ntask_uid: malformed']:
            with self.subTest(body=body), patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':body}),'[]']):
                with self.assertRaisesRegex(ValueError,'UID mismatch'):
                    live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid})
        with patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':'task_uid: '+uid}),'[]']):
            self.assertIsNone(live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid}))

if __name__ == '__main__': unittest.main()
