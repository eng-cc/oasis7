import importlib.util
from pathlib import Path
import tempfile
import unittest
import subprocess
import sys
import os
import json
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('recovery', Path(__file__).with_name('loop_recovery.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class RecoveryTests(unittest.TestCase):
    def test_same_task_and_overlapping_scope_are_busy(self):
        with tempfile.TemporaryDirectory() as tmp:
            with module.Reservation(Path(tmp), 'task_a', ['src/']):
                with self.assertRaises(module.Busy):
                    with module.Reservation(Path(tmp), 'task_a', ['doc/']): pass
                with self.assertRaises(module.Busy):
                    with module.Reservation(Path(tmp), 'task_b', ['src/a.rs']): pass
                with module.Reservation(Path(tmp), 'task_c', ['doc/']): pass
            with module.Reservation(Path(tmp), 'task_a', ['src/']): pass

    def test_pending_actions_require_reconciliation(self):
        with tempfile.TemporaryDirectory() as tmp:
            module.record_action(Path(tmp), 'task_a', {'action_id': 'a', 'kind': 'push', 'expected': 'abc'})
            self.assertEqual(module.recovery_status(Path(tmp), 'task_a')['status'], 'reconcile_required')

    def test_all_glob_metacharacters_reserve_static_parent(self):
        with tempfile.TemporaryDirectory() as tmp:
            for pattern in ('src/a?.rs', 'src/a*.rs', 'src/a[b].rs'):
                with module.Reservation(Path(tmp), 'task_a', [pattern]):
                    with self.assertRaises(module.Busy):
                        with module.Reservation(Path(tmp), 'task_b', ['src/ab.rs']): pass

    def test_os_lock_blocks_another_process_then_releases_without_unlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            code = "import sys; from pathlib import Path; from loop_recovery import Reservation; r=Reservation(Path(sys.argv[1]),'task_a',['src/']); r.__enter__(); print('locked',flush=True); sys.stdin.read()"
            proc = subprocess.Popen([sys.executable, '-c', code, tmp], cwd=Path(__file__).parent, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
            try:
                self.assertEqual(proc.stdout.readline().strip(), 'locked')
                with self.assertRaises(module.Busy):
                    with module.Reservation(Path(tmp), 'task_b', ['src/a.rs']): pass
                before = {p: p.stat().st_ino for p in Path(tmp).rglob('*.lock')}
                proc.communicate('')
                with module.Reservation(Path(tmp), 'task_b', ['src/a.rs']): pass
                self.assertTrue(all(p.stat().st_ino == inode for p, inode in before.items()))
            finally:
                if proc.poll() is None: proc.kill(); proc.wait()

    def test_live_child_blocks_manual_recovery(self):
        with tempfile.TemporaryDirectory() as tmp:
            module.record_action(Path(tmp), 'task_a', {'action_id': 'child', 'kind': 'child_process', 'expected': str(os.getpid())})
            result = module.reconcile(Path(tmp), 'task_a', Path(tmp))
            self.assertEqual(result['status'], 'reconcile_required')
            self.assertEqual(len(result['active_or_unverified_children']), 1)

    def test_pending_effect_blocks_writer_but_allows_reserved_recovery(self):
        with tempfile.TemporaryDirectory() as tmp:
            with module.Reservation(Path(tmp), 'task_a', ['src/**']):
                module.record_action(Path(tmp), 'task_a', {'action_id': 'lost', 'kind': 'push', 'expected': 'abc'})
            for uid, scope in [('task_a', ['doc/**']), ('task_b', ['src/x.rs'])]:
                with self.assertRaises(module.Busy):
                    with module.Reservation(Path(tmp), uid, scope): pass
            with module.Reservation(Path(tmp), 'task_c', ['doc/**']): pass
            with module.Reservation(Path(tmp), 'task_a', ['src/**'], recovery=True):
                self.assertEqual(module.recovery_status(Path(tmp), 'task_a')['status'], 'reconcile_required')

    def test_real_child_survives_invocation_then_explicit_recovery(self):
        with tempfile.TemporaryDirectory() as tmp:
            child = subprocess.Popen([sys.executable, '-c', 'import sys; sys.stdin.read()'], stdin=subprocess.PIPE)
            try:
                with module.Reservation(Path(tmp), 'task_a', ['src/**']):
                    module.record_action(Path(tmp), 'task_a', {'action_id': 'child', 'kind': 'child_process', 'expected': str(child.pid)})
                observed = module.recovery_status(Path(tmp), 'task_a')
                self.assertEqual(observed['pending_actions'][0]['process_identity']['pid'], child.pid)
                with self.assertRaises(module.Busy):
                    with module.Reservation(Path(tmp), 'task_a', ['src/**']): pass
                with module.Reservation(Path(tmp), 'task_a', ['src/**'], recovery=True):
                    self.assertEqual(module.reconcile(Path(tmp), 'task_a', Path(tmp))['status'], 'reconcile_required')
                child.communicate(b'')
                with module.Reservation(Path(tmp), 'task_a', ['src/**'], recovery=True):
                    self.assertEqual(module.reconcile(Path(tmp), 'task_a', Path(tmp))['status'], 'can_continue')
                with module.Reservation(Path(tmp), 'task_a', ['src/**']): pass
            finally:
                if child.poll() is None: child.kill(); child.wait()

    def test_lost_push_response_reconciles_only_exact_remote_readback(self):
        with tempfile.TemporaryDirectory() as tmp:
            action = {'action_id': 'push', 'kind': 'push', 'expected': 'a' * 40, 'remote_ref': 'refs/heads/test'}
            module.record_action(Path(tmp), 'task_a', action)
            with patch.object(module.subprocess, 'check_output', return_value='b' * 40 + '\trefs/heads/test\n'):
                self.assertEqual(module.reconcile(Path(tmp), 'task_a', Path(tmp))['status'], 'reconcile_required')
            with patch.object(module.subprocess, 'check_output', return_value='a' * 40 + '\trefs/heads/test\n'):
                self.assertEqual(module.reconcile(Path(tmp), 'task_a', Path(tmp))['status'], 'can_continue')

    def test_partial_journal_and_forged_resolution_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            action = {'action_id': 'push', 'kind': 'push', 'expected': 'abc'}
            module.record_action(Path(tmp), 'task_a', action)
            module.record_action(Path(tmp), 'task_a', {**action, 'expected': 'other', 'reconciled': True, 'readback_evidence': 'wrong'})
            with self.assertRaises(module.Busy): module.recovery_status(Path(tmp), 'task_a')
        with tempfile.TemporaryDirectory() as tmp:
            directory = module._directory(Path(tmp))
            (directory / (module._key('task_a') + '.actions.jsonl')).write_text('{')
            with self.assertRaises(module.Busy):
                with module.Reservation(Path(tmp), 'task_a', []): pass

    def test_binding_issue_only_readback_does_not_clear_uncertain_project(self):
        with tempfile.TemporaryDirectory() as tmp:
            binding = {'task_uid': 'task_a'}
            module.record_action(Path(tmp), 'task_a', {'action_id': 'bind', 'kind': 'bind_loop', 'expected': json.dumps(binding), 'repository': 'eng-cc/oasis7', 'issue_number': 1})
            import base64
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            with patch.object(module.subprocess, 'check_output', return_value=json.dumps({'body': '- loop_binding_b64: `' + encoded + '`'})):
                self.assertEqual(module.reconcile(Path(tmp), 'task_a', Path(tmp))['status'], 'reconcile_required')

    def test_publication_lost_response_requires_unique_validated_readback(self):
        import loop_contracts
        uid = 'task_' + 'a' * 32
        contract = {'contract_id': 'c', 'revision': 1, 'content_refs': [{'clauses': ['one']}]}
        payload = {'marker': loop_contracts.MARKER, 'task_uid': uid, 'contract': contract, 'contract_digest': loop_contracts.contract_digest(contract)}
        comment = {'id': 42, 'body': json.dumps(payload)}
        with tempfile.TemporaryDirectory() as tmp:
            action = {'action_id': 'publication', 'kind': 'publish_contract', 'expected': json.dumps(contract),
                      'repository': 'eng-cc/oasis7', 'issue_number': 1, 'binding': {'task_uid': uid}}
            module.record_action(Path(tmp), uid, action)
            with patch.object(loop_contracts, 'GitHubAuthority') as authority, patch.object(loop_contracts, 'validate_contracts') as validate:
                authority.return_value.api.return_value = [[comment, comment]]
                validate.return_value = {'status': 'passed'}
                self.assertEqual(module.reconcile(Path(tmp), uid, Path(tmp))['status'], 'reconcile_required')
                validate.assert_not_called()
                authority.return_value.api.return_value = [[comment]]
                validate.return_value = {'status': 'blocked'}
                self.assertEqual(module.reconcile(Path(tmp), uid, Path(tmp))['status'], 'reconcile_required')
                validate.return_value = {'status': 'passed'}
                self.assertEqual(module.reconcile(Path(tmp), uid, Path(tmp))['status'], 'can_continue')
                self.assertTrue(all(call.kwargs == {'paginate': True} for call in authority.return_value.api.call_args_list))

if __name__ == '__main__': unittest.main()
