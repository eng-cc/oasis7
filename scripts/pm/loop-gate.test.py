import sys
from pathlib import Path
import unittest
sys.path.insert(0, str(Path(__file__).parent))
from loop_gate import admission, live_binding
from unittest.mock import patch
import json


class GateTests(unittest.TestCase):
    def test_live_legacy_passes(self):
        self.assertEqual(admission(Path('.'), {}, 'base', 'head', reader=lambda _: None)['status'], 'legacy')

    def test_cache_deletion_cannot_downgrade_live_loop(self):
        with self.assertRaisesRegex(ValueError, 'cache differs'):
            admission(Path('.'), {}, 'base', 'head', reader=lambda _: {'loop': 'code'})

    def test_missing_effective_helper_blocks_loop(self):
        binding = {'loop': 'code'}
        with self.assertRaisesRegex(ValueError, 'trusted'):
            admission(Path('.'), {'loop_binding': binding}, 'base', 'head', reader=lambda _: binding)


    def test_legacy_live_identity_requires_one_exact_canonical_field(self):
        uid='task_'+'a'*32
        for body in ['previous '+uid,'task_uid: task_'+'b'*32+'\nprevious '+uid,'task_uid: '+uid+'\ntask_uid: '+uid,'task_uid: '+uid+'\ntask_uid: malformed']:
            with self.subTest(body=body), patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':body}),'[]']):
                with self.assertRaisesRegex(ValueError,'UID mismatch'):
                    live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid})
        with patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':'task_uid: '+uid}),'[]']):
            self.assertIsNone(live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid}))

if __name__ == '__main__': unittest.main()
