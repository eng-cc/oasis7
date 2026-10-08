#!/usr/bin/env python3
import importlib.util
import pathlib
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
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.com')
        self.git('config', 'user.name', 'test')
        (self.repo / 'file').write_text('base')
        self.git('add', '.')
        self.git('commit', '-qm', 'base')
        self.head = self.git('rev-parse', 'HEAD')
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
        def run(args, check=True):
            output = ps if args[0] == 'ps' else lsof
            return subprocess.CompletedProcess(args, 0 if args[0] == 'ps' else code, output, '')
        with patch.object(cleanup, '_run', side_effect=run):
            return cleanup._process_mentions_path(pathlib.Path('/tmp/retained-worktree'))

    def test_foreign_uid_cwd_use_is_detected_without_argv_path(self):
        self.assertTrue(self.scan('202 1 999 shell\n', 'p202\nfcwd\ntDIR\nn/tmp/retained-worktree/subdir\n'))

    def test_foreign_uid_open_file_use_is_detected(self):
        self.assertTrue(self.scan('202 1 999 editor\n', 'p202\nfcwd\ntDIR\nn/tmp\nf3\ntREG\nn/tmp/retained-worktree/file\n'))

    def test_incomplete_process_coverage_retains_resources(self):
        with self.assertRaises(cleanup.CleanupError):
            self.scan('202 1 999 editor\n203 1 999 shell\n', 'p202\nfcwd\ntDIR\nn/tmp\n')

    def test_ambiguous_filesystem_descriptor_retains_resources(self):
        with self.assertRaises(cleanup.CleanupError):
            self.scan('202 1 999 editor\n', 'p202\nfcwd\ntDIR\nn/tmp\nf3\ntREG\nn??\n')

    def test_complete_unrelated_processes_allow_inspection(self):
        self.assertFalse(self.scan('202 1 999 editor\n', 'p202\nfcwd\ntDIR\nn/tmp/retained-worktree-sibling\nf3\ntREG\nn/tmp/unrelated\n'))


if __name__ == '__main__':
    unittest.main()
