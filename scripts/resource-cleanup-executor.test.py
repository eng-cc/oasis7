#!/usr/bin/env python3
import importlib.util
import pathlib
import os
import sys
import time
import json
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('cleanup', pathlib.Path(__file__).with_name('resource-cleanup-executor.py'))
cleanup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleanup)

class SafetyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.email', 'test@example.com')
        self.git('config', 'user.name', 'test')
        (self.repo / 'file').write_text('base')
        self.git('add', '.')
        self.git('commit', '-qm', 'base')
        self.head = self.git('rev-parse', 'HEAD')
        self.git('update-ref', 'refs/remotes/origin/main', self.head)
        self.path = self.root / 'worktree'
        self.git('worktree', 'add', '-qb', 'feature', str(self.path))
        self.git('update-ref', 'refs/remotes/origin/feature', self.head)
        self.git('config', 'branch.feature.remote', 'origin')
        self.git('config', 'branch.feature.merge', 'refs/heads/feature')
        self.git('config', 'remote.origin.url', str(self.root / 'remote'))
        self.git('config', 'remote.origin.fetch', '+refs/heads/*:refs/remotes/origin/*')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], text=True, stderr=subprocess.DEVNULL).strip()

    def inspect(self):
        return cleanup.inspect(self.repo, self.path, 'feature', self.head)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_clean_exact_resource_is_inspectable(self, _):
        self.assertEqual(self.inspect()['head'], self.head)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_dirty_and_unknown_ignored_material_are_retained(self, _):
        (self.path / 'file').write_text('user work')
        with self.assertRaisesRegex(cleanup.CleanupError, 'uncommitted'):
            self.inspect()
        self.assertEqual((self.path / 'file').read_text(), 'user work')
        (self.path / 'file').write_text('base')
        (self.path / '.gitignore').write_text('secret\n')
        with self.assertRaises(cleanup.CleanupError):
            self.inspect()

    @patch.object(cleanup, '_process_mentions_path', return_value=True)
    def test_active_resource_is_retained(self, _):
        with self.assertRaisesRegex(cleanup.CleanupError, 'in use'):
            self.inspect()
        self.assertTrue(self.path.exists())

    def test_primary_branch_is_retained(self):
        with self.assertRaisesRegex(cleanup.CleanupError, 'primary branch'):
            cleanup.inspect(self.repo, self.path, 'main', self.head)

    def test_wrong_head_and_primary_are_retained(self):
        with self.assertRaises(cleanup.CleanupError):
            cleanup.inspect(self.repo, self.path, 'feature', '0' * 40)
        with self.assertRaises(cleanup.CleanupError):
            cleanup.inspect(self.repo, self.repo, 'feature', self.head)

    def test_changed_branch_survives_expected_old_deletion(self):
        changed = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'concurrent work')
        self.git('update-ref', 'refs/heads/feature', changed)
        with self.assertRaises(cleanup.CleanupError):
            cleanup.git(self.repo, 'update-ref', '-d', 'refs/heads/feature', self.head)
        self.assertEqual(self.git('rev-parse', 'refs/heads/feature'), changed)

class ProcessReadbackTests(unittest.TestCase):
    def scan(self, ps, lsof, code=0):
        def run(args, check=True, timeout=10):
            output = ps if args[0] == 'ps' else lsof
            return subprocess.CompletedProcess(args, 0 if args[0] == 'ps' else code, output, '')
        with patch.object(cleanup, '_run', side_effect=run), patch.object(cleanup, 'process_identity', side_effect=lambda pid: (pid, 1, 'R')):
            return cleanup._process_mentions_path(pathlib.Path('/tmp/retained-worktree'))

    def test_foreign_uid_cwd_use_is_detected_without_argv_path(self):
        self.assertTrue(self.scan('202\n', 'p202\nfcwd\ntDIR\nn/tmp/retained-worktree/subdir\n'))

    def test_foreign_uid_open_file_use_is_detected(self):
        self.assertTrue(self.scan('202\n', 'p202\nfcwd\ntDIR\nn/tmp\nf3\ntREG\nn/tmp/retained-worktree/file\n'))

    def test_incomplete_process_coverage_retains_resources(self):
        with self.assertRaises(cleanup.CleanupError):
            self.scan('202\n203\n', 'p202\nfcwd\ntDIR\nn/tmp\n')

    def test_ambiguous_filesystem_descriptor_retains_resources(self):
        with self.assertRaises(cleanup.CleanupError):
            self.scan('202\n', 'p202\nfcwd\ntDIR\nn/tmp\nf3\ntREG\nn??\n')

    def test_sibling_boundary_and_nonfiles(self):
        self.assertFalse(self.scan('202\n', 'p202\nfcwd\ntDIR\nn/tmp/retained-worktree-sibling\nf3\ntPIPE\nn\n'))

    def test_exited_collector_and_reused_pid(self):
        snapshots = [{202: (202, 1, 'R'), 203: (203, 1, 'R')}, {202: (202, 2, 'R')}, {202: (202, 2, 'R')}]
        with patch.object(cleanup, 'snapshot_processes', side_effect=snapshots), patch.object(cleanup, 'read_open_files', return_value=({202}, set())) as scan:
            self.assertEqual(cleanup.inspect_process_use(pathlib.Path('/tmp'))['state'], 'clear_observed')
            self.assertEqual(scan.call_count, 2)

    def test_new_process_after_second_round_is_unknown(self):
        snapshots = [{202: (202, 1, 'R')}, {203: (203, 1, 'R')}, {204: (204, 1, 'R')}]
        with patch.object(cleanup, 'snapshot_processes', side_effect=snapshots), patch.object(cleanup, 'read_open_files', return_value=({202, 203}, set())):
            self.assertEqual(cleanup.inspect_process_use(pathlib.Path('/tmp'))['state'], 'unknown')

    def test_permission_error_never_means_gone(self):
        with patch.object(cleanup.pathlib.Path, 'read_text', side_effect=OSError('unreadable')), patch.object(cleanup.ctypes, 'CDLL', side_effect=OSError('unreadable')), patch.object(cleanup.os, 'kill', side_effect=PermissionError(1, 'denied')):
            with self.assertRaises(cleanup.CleanupError):
                cleanup.process_identity(202)

    def test_unknown_descriptor_type_is_unknown(self):
        with self.assertRaisesRegex(cleanup.CleanupError, 'unknown descriptor type'):
            self.scan('202\n', 'p202\nfcwd\ntUNKNOWN\nn/tmp/unrelated\n')

    def test_state_change_preserves_native_start_identity(self):
        snapshots = [{202: (202, 1, 'R')}, {202: (202, 1, 'S')}]
        with patch.object(cleanup, 'snapshot_processes', side_effect=snapshots), patch.object(cleanup, 'read_open_files', return_value=({202}, set())) as scan:
            self.assertEqual(cleanup.inspect_process_use(pathlib.Path('/tmp'))['state'], 'clear_observed')
            self.assertEqual(scan.call_count, 1)

    def test_exact_github_origin_identity(self):
        for remote in ['https://github.com/example/repo.git', 'git@github.com:example/repo.git', 'ssh://git@github.com/example/repo.git']:
            self.assertEqual(cleanup.github_repository(remote), 'example/repo')
        for remote in ['https://notgithub.com/example/repo.git', 'https://github.com.evil/example/repo.git', 'git@notgithub.com:example/repo.git', 'https://github.com/example/repo.git?x=1', 'https://github.com/example/repo/extra']:
            with self.assertRaises(cleanup.CleanupError):
                cleanup.github_repository(remote)

    def test_timeout_is_unknown(self):
        with patch.object(cleanup, 'snapshot_processes', side_effect=cleanup.CleanupError('timeout')):
            self.assertEqual(cleanup.inspect_process_use(pathlib.Path('/tmp'))['state'], 'unknown')

class DeliveryTests(SafetyTests):
    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_pruned_upstream_and_main_advance_are_allowed(self, _):
        self.git('update-ref', '-d', 'refs/remotes/origin/feature')
        self.inspect()
        self.git('config', 'branch.feature.merge', 'refs/heads/main')
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'advance')
        self.git('update-ref', 'refs/remotes/origin/main', new)
        self.inspect()

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_pushed_only_and_unpublished_local_main_retained(self, _):
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'unmerged')
        self.git('update-ref', 'refs/heads/feature', new)
        self.git('update-ref', 'refs/remotes/origin/feature', new)
        with self.assertRaisesRegex(cleanup.CleanupError, 'no main ancestry'):
            cleanup.inspect(self.repo, self.path, 'feature', new)
        self.git('update-ref', 'refs/heads/main', new)
        with self.assertRaisesRegex(cleanup.CleanupError, 'not been delivered'):
            cleanup.delivery(self.repo, new, 'refs/heads/main', None)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_ordinary_remove_and_transaction(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertTrue(report['worktree_removed'])
        self.assertTrue(report['branch_removed'])
        self.assertFalse(self.path.exists())

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_after_remove_new_branch_tip_survives(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        original = cleanup.git
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'concurrent')
        def race(repo, *args):
            result = original(repo, *args)
            if args[:2] == ('worktree', 'remove'):
                self.git('update-ref', 'refs/heads/feature', new)
            return result
        with patch.object(cleanup, 'git', side_effect=race), self.assertRaisesRegex(cleanup.CleanupError, 'transaction failed'):
            cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertTrue(report['worktree_removed'])
        self.assertFalse(report['branch_removed'])
        self.assertEqual(self.git('rev-parse', 'feature'), new)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_other_checkout_after_remove_retains_branch(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        original = cleanup.git
        other = self.root / 'other'
        def race(repo, *args):
            result = original(repo, *args)
            if args[:2] == ('worktree', 'remove'):
                self.git('worktree', 'add', str(other), 'feature')
            return result
        with patch.object(cleanup, 'git', side_effect=race), self.assertRaisesRegex(cleanup.CleanupError, 'checked out elsewhere'):
            cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertEqual(self.git('rev-parse', 'feature'), self.head)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_ignored_config_and_cache_are_verified(self, _):
        self.git('config', 'core.excludesFile', str(self.root / 'ignore'))
        (self.root / 'ignore').write_text('config.toml\ntarget\nsecret\n')
        (self.repo / 'config.toml').write_text('canonical')
        (self.path / 'config.toml').write_text('canonical')
        cache = self.root / '.oasis7-cache' / 'cargo-target' / 'dev'
        cache.mkdir(parents=True)
        (self.path / 'target').symlink_to(cache)
        self.inspect()
        (self.path / 'config.toml').write_text('user change')
        with self.assertRaisesRegex(cleanup.CleanupError, 'config retained'):
            self.inspect()
        (self.path / 'config.toml').write_text('canonical')
        (self.path / 'target').unlink()
        (self.path / 'target').mkdir()
        (self.path / 'target' / 'file').write_text('user material')
        with self.assertRaisesRegex(cleanup.CleanupError, 'ignored user material'):
            self.inspect()

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_squash_pr_native_proof_and_mismatches(self, _):
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-m', 'topic root')
        self.git('remote', 'set-url', 'origin', 'https://github.com/example/repo.git')
        data = {'merged': True, 'head': {'sha': new}, 'base': {'ref': 'main', 'repo': {'full_name': 'example/repo'}}, 'merge_commit_sha': self.head}
        original = cleanup._run
        def api(args, **kwargs):
            if args[0] == 'gh':
                return subprocess.CompletedProcess(args, 0, json.dumps(data), '')
            return original(args, **kwargs)
        with patch.object(cleanup, '_run', side_effect=api):
            cleanup.delivery(self.repo, new, 'refs/remotes/origin/main', 42)
            for key, value in [('merged', False), ('head', {'sha': self.head}), ('base', {'ref': 'other', 'repo': {'full_name': 'example/repo'}}), ('merge_commit_sha', new)]:
                old = data[key]
                data[key] = value
                with self.assertRaises(cleanup.CleanupError):
                    cleanup.delivery(self.repo, new, 'refs/remotes/origin/main', 42)
                data[key] = old

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_locked_flag_without_value_retained(self, _):
        self.git('worktree', 'lock', str(self.path))
        with self.assertRaisesRegex(cleanup.CleanupError, 'locked'):
            self.inspect()

    def test_unknown_inspection_retains_every_resource(self):
        with patch.object(cleanup, 'inspect_process_use', return_value={'state': 'unknown', 'reason': 'PID 123 permission denied'}), self.assertRaisesRegex(cleanup.CleanupError, 'permission denied'):
            self.inspect()
        self.assertTrue(self.path.exists())
        self.assertEqual(self.git('rev-parse', 'feature'), self.head)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_material_changed_during_last_process_scan_retained(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        def mutate(path):
            (path / 'file').write_text('concurrent user work')
            return False
        with patch.object(cleanup, '_process_mentions_path', side_effect=mutate), self.assertRaisesRegex(cleanup.CleanupError, 'uncommitted'):
            cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertTrue(self.path.exists())

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_after_remove_changed_base_fails_transaction(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        original = cleanup.git
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'new base')
        def race(repo, *args):
            result = original(repo, *args)
            if args[:2] == ('worktree', 'remove'):
                self.git('update-ref', 'refs/remotes/origin/main', new)
            return result
        with patch.object(cleanup, 'git', side_effect=race), self.assertRaisesRegex(cleanup.CleanupError, 'transaction failed'):
            cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertTrue(report['worktree_removed'])
        self.assertEqual(self.git('rev-parse', 'feature'), self.head)

    @patch.object(cleanup, '_process_mentions_path', return_value=False)
    def test_recreated_ref_is_reported_without_second_delete(self, _):
        report = dict(self.inspect(), worktree_removed=False, branch_removed=False)
        original = subprocess.run
        new = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-p', self.head, '-m', 'recreated')
        def race(args, **kwargs):
            result = original(args, **kwargs)
            if 'update-ref' in args and '--stdin' in args and result.returncode == 0:
                original(['git', '-C', str(self.repo), 'update-ref', 'refs/heads/feature', new], check=True, capture_output=True)
            return result
        with patch.object(cleanup.subprocess, 'run', side_effect=race), self.assertRaisesRegex(cleanup.CleanupError, 'retained/recreated'):
            cleanup.execute(self.repo, self.path, 'feature', self.head, 'refs/remotes/origin/main', None, report)
        self.assertEqual(report['branch_state'], 'retained/recreated')
        self.assertFalse(report['branch_removed'])
        self.assertEqual(self.git('rev-parse', 'feature'), new)

    def test_invalid_base_cli_is_argument_error(self):
        result = subprocess.run([sys.executable, str(pathlib.Path(cleanup.__file__)), '--worktree', str(self.path), '--branch', 'feature', '--expected-head', self.head, '--base-ref', 'refs/heads/feature'], capture_output=True)
        self.assertEqual(result.returncode, 2)

class HostSmokeTests(unittest.TestCase):
    def test_native_identity_collector_busy_then_exit(self):
        self.assertIsNotNone(cleanup.process_identity(os.getpid()))
        with tempfile.TemporaryDirectory() as directory:
            target = pathlib.Path(directory)
            proc = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'], cwd=target)
            try:
                identity = cleanup.process_identity(proc.pid)
                self.assertIsNotNone(identity)
                # Real census, lsof and native identity, using our own process
                # slice to avoid unrelated host permission limits hiding busy proof.
                deadline = time.monotonic() + 30
                try:
                    snapshot = cleanup.snapshot_processes(deadline)
                    self.assertIn(proc.pid, snapshot)
                except cleanup.CleanupError as exc:
                    self.assertIn('PID', str(exc))
                covered, busy = cleanup.read_open_files({proc.pid}, target, deadline)
                self.assertIn(proc.pid, covered)
                self.assertIn(proc.pid, busy)
                observed = cleanup.inspect_process_use(target)
                self.assertIn(observed['state'], ('busy', 'unknown'))
            finally:
                proc.terminate()
                proc.wait(timeout=5)
            self.assertIsNone(cleanup.process_identity(proc.pid))
            after = cleanup.inspect_process_use(target)
            self.assertIn(after['state'], ('clear_observed', 'unknown'))
            if after['state'] == 'unknown':
                self.assertTrue(after['reason'])
            # Real ps collector has exited by the time identities are read;
            # gone identities cannot stay in a census forever.
            dead = subprocess.Popen(['ps', '-axo', 'pid='], stdout=subprocess.DEVNULL)
            dead.wait(timeout=2)
            self.assertIsNone(cleanup.process_identity(dead.pid))

if __name__ == '__main__':
    if '--host-smoke' in sys.argv:
        sys.argv.remove('--host-smoke')
        unittest.main(defaultTest='HostSmokeTests')
    else:
        unittest.main(defaultTest=['SafetyTests', 'ProcessReadbackTests', 'DeliveryTests'])
