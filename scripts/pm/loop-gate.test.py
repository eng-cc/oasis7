import sys
from pathlib import Path
import unittest
from types import SimpleNamespace
import importlib.util
import tempfile
sys.path.insert(0, str(Path(__file__).parent))
from loop_gate import admission, live_binding
from unittest.mock import Mock, patch
import json
import subprocess


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

    def test_candidate_loop_cannot_admit_itself_over_the_effective_git_pin(self):
        canonical_root = Path(__file__).resolve().parents[2]
        frozen_head = subprocess.check_output(
            ['git', '-C', str(canonical_root), 'rev-parse', 'HEAD'], text=True,
        ).strip()
        with tempfile.TemporaryDirectory() as directory:
            fixture_root = Path(directory)
            origin = fixture_root / 'origin.git'
            subprocess.run(['git', 'init', '--bare', str(origin)], check=True,
                           capture_output=True, text=True)
            subprocess.run(
                ['git', '--git-dir', str(origin), 'fetch', '--no-tags',
                 str(canonical_root), f'{frozen_head}:refs/heads/main'],
                check=True, capture_output=True, text=True,
            )
            subprocess.run(
                ['git', '--git-dir', str(origin), 'symbolic-ref', 'HEAD', 'refs/heads/main'],
                check=True, capture_output=True, text=True,
            )
            trusted_root = fixture_root / 'trusted'
            candidate_root = fixture_root / 'candidate'
            subprocess.run(
                ['git', 'clone', '--no-hardlinks', str(origin), str(trusted_root)],
                check=True, capture_output=True, text=True,
            )
            subprocess.run(
                ['git', '-C', str(trusted_root), 'worktree', 'add', '--detach',
                 str(candidate_root), frozen_head],
                check=True, capture_output=True, text=True,
            )
            pinned_commit = subprocess.check_output(
                ['git', '-C', str(trusted_root), 'rev-parse', 'HEAD'], text=True,
            ).strip()
            candidate_head = subprocess.check_output(
                ['git', '-C', str(candidate_root), 'rev-parse', 'HEAD'], text=True,
            ).strip()
            trusted_common_dir = subprocess.check_output(
                ['git', '-C', str(trusted_root), 'rev-parse', '--path-format=absolute',
                 '--git-common-dir'], text=True,
            ).strip()
            candidate_common_dir = subprocess.check_output(
                ['git', '-C', str(candidate_root), 'rev-parse', '--path-format=absolute',
                 '--git-common-dir'], text=True,
            ).strip()
            self.assertEqual(frozen_head, pinned_commit)
            self.assertEqual(frozen_head, candidate_head)
            self.assertEqual(trusted_common_dir, candidate_common_dir)

            candidate_pm = candidate_root / 'scripts' / 'pm'
            candidate_gate = candidate_pm / 'loop_gate.py'
            candidate_gate.write_bytes(Path(__file__).with_name('loop_gate.py').read_bytes())
            candidate_marker = fixture_root / 'candidate-loop-was-called'
            (candidate_pm / 'loop.py').write_text(
                "from pathlib import Path\n"
                "def validate_task(*args, **kwargs):\n"
                f"    Path({str(candidate_marker)!r}).write_text('called', encoding='utf-8')\n"
                "    return {'status': 'passed', 'blockers': []}\n",
                encoding='utf-8',
            )
            spec = importlib.util.spec_from_file_location('candidate_loop_gate', candidate_gate)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)

            # This resolves and executes the actual loop.py blob at the frozen Git pin.
            pinned = module._pinned_module(trusted_root, pinned_commit, 'loop')
            self.assertEqual(
                f'{trusted_root}/scripts/pm/loop.py@{pinned_commit}', pinned.__file__,
            )
            binding = {'policy_commit': pinned_commit}
            task = {'task_uid': 'task_' + 'a' * 32, 'loop_binding': binding}
            try:
                result = module.admission(
                    candidate_root, task, frozen_head, candidate_head,
                    tool_root=trusted_root, reader=lambda _task: binding,
                )
            except ValueError as exc:
                self.assertIn('loop admission', str(exc))
            else:
                self.assertNotEqual('passed', result.get('status'), result)
            self.assertFalse(candidate_marker.exists(), 'candidate loop helper must not execute')

    def test_configured_trusted_root_is_required_before_facade_load(self):
        canonical_root = Path(__file__).resolve().parents[2]
        frozen_head = subprocess.check_output(
            ['git', '-C', str(canonical_root), 'rev-parse', 'HEAD'], text=True,
        ).strip()
        with tempfile.TemporaryDirectory() as directory:
            fixture_root = Path(directory)
            origin = fixture_root / 'origin.git'
            subprocess.run(['git', 'init', '--bare', str(origin)], check=True,
                           capture_output=True, text=True)
            subprocess.run(
                ['git', '--git-dir', str(origin), 'fetch', '--no-tags',
                 str(canonical_root), f'{frozen_head}:refs/heads/main'],
                check=True, capture_output=True, text=True,
            )
            subprocess.run(
                ['git', '--git-dir', str(origin), 'symbolic-ref', 'HEAD', 'refs/heads/main'],
                check=True, capture_output=True, text=True,
            )
            trusted_root = fixture_root / 'trusted'
            candidate_root = fixture_root / 'candidate'
            subprocess.run(
                ['git', 'clone', '--no-hardlinks', str(origin), str(trusted_root)],
                check=True, capture_output=True, text=True,
            )
            subprocess.run(
                ['git', '-C', str(trusted_root), 'worktree', 'add', '--detach',
                 str(candidate_root), frozen_head],
                check=True, capture_output=True, text=True,
            )
            pinned_commit = subprocess.check_output(
                ['git', '-C', str(trusted_root), 'rev-parse', 'HEAD'], text=True,
            ).strip()
            candidate_head = subprocess.check_output(
                ['git', '-C', str(candidate_root), 'rev-parse', 'HEAD'], text=True,
            ).strip()
            trusted_common_dir = subprocess.check_output(
                ['git', '-C', str(trusted_root), 'rev-parse', '--path-format=absolute',
                 '--git-common-dir'], text=True,
            ).strip()
            candidate_common_dir = subprocess.check_output(
                ['git', '-C', str(candidate_root), 'rev-parse', '--path-format=absolute',
                 '--git-common-dir'], text=True,
            ).strip()
            self.assertEqual(frozen_head, pinned_commit)
            self.assertEqual(frozen_head, candidate_head)
            self.assertEqual(trusted_common_dir, candidate_common_dir)

            candidate_pm = candidate_root / 'scripts' / 'pm'
            candidate_gate = candidate_pm / 'loop_gate.py'
            candidate_gate.write_bytes(Path(__file__).with_name('loop_gate.py').read_bytes())
            candidate_marker = fixture_root / 'candidate-loop-was-called'
            (candidate_pm / 'loop.py').write_text(
                "from pathlib import Path\n"
                "def validate_task(*args, **kwargs):\n"
                f"    Path({str(candidate_marker)!r}).write_text('called', encoding='utf-8')\n"
                "    return {'status': 'passed', 'blockers': []}\n",
                encoding='utf-8',
            )
            spec = importlib.util.spec_from_file_location('candidate_loop_gate', candidate_gate)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)

            # The configured root is a real checkout of the exact frozen helper commit.
            pinned = module._pinned_module(trusted_root, pinned_commit, 'loop')
            self.assertEqual(
                f'{trusted_root}/scripts/pm/loop.py@{pinned_commit}', pinned.__file__,
            )
            binding = {'policy_commit': pinned_commit}
            task = {'task_uid': 'task_' + 'a' * 32, 'loop_binding': binding}
            trusted_result = pinned.validate_task(
                candidate_root, task, trusted_root, frozen_head, candidate_head,
            )
            self.assertEqual('blocked', trusted_result['status'])
            self.assertIn(
                'active policy resolution requires canonical task repository and UID',
                trusted_result['blockers'],
            )
            with self.assertRaisesRegex(
                    ValueError, 'loop admission: active policy resolution requires canonical task repository and UID'):
                module.admission(
                    candidate_root, task, frozen_head, candidate_head,
                    tool_root=trusted_root, reader=lambda _task: binding,
                )
            self.assertFalse(candidate_marker.exists(), 'candidate loop helper must not execute')


    def test_legacy_live_identity_requires_one_exact_canonical_field(self):
        uid='task_'+'a'*32
        for body in ['previous '+uid,'task_uid: task_'+'b'*32+'\nprevious '+uid,'task_uid: '+uid+'\ntask_uid: '+uid,'task_uid: '+uid+'\ntask_uid: malformed']:
            with self.subTest(body=body), patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':body}),'[]']):
                with self.assertRaisesRegex(ValueError,'UID mismatch'):
                    live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid})
        with patch('loop_gate.subprocess.check_output',side_effect=[json.dumps({'body':'task_uid: '+uid}),'[]']):
            self.assertIsNone(live_binding({'repository':'owner/repo','issue_number':1,'task_uid':uid}))

if __name__ == '__main__': unittest.main()
