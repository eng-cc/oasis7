#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import tempfile
import unittest
import subprocess
import contextlib
import hashlib
import io
import json
import sys
from types import SimpleNamespace
from unittest.mock import patch

PATH = Path(__file__).with_name('loop-bootstrap.py')

class ManualBase(unittest.TestCase):
    def test_two_requests_and_historical_contract_fetch_pin_their_advertised_main(self):
        spec = importlib.util.spec_from_file_location('loop_bootstrap', PATH)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        contract_spec = importlib.util.spec_from_file_location('loop_contracts', PATH.with_name('loop_contracts.py'))
        contracts = importlib.util.module_from_spec(contract_spec); contract_spec.loader.exec_module(contracts)
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory); author = temp/'author'; author.mkdir()
            def git(root,*args): return subprocess.check_output(['git','-C',str(root),*args],text=True,stderr=subprocess.PIPE).strip()
            git(author,'init','-q','-b','main'); git(author,'config','user.email','fixture@example.invalid'); git(author,'config','user.name','Fixture')
            (author/'base.txt').write_text('base'); git(author,'add','.'); git(author,'commit','-qm','base')
            git(author,'switch','-c','historical')
            (author/'contract.md').write_text('approved'); git(author,'add','.'); git(author,'commit','-qm','historical contract')
            source = git(author,'rev-parse','HEAD')
            git(author,'switch','main'); git(author,'merge','--squash','historical'); git(author,'commit','-qm','squashed contract')
            initial = git(author,'rev-parse','HEAD')
            remote=temp/'origin.git'; subprocess.run(['git','init','--bare','-q',str(remote)],check=True)
            git(author,'remote','add','origin',str(remote)); git(author,'push','origin','main',source+':refs/pull/7/head')
            subprocess.run(['git','--git-dir',str(remote),'symbolic-ref','HEAD','refs/heads/main'],check=True)
            git(author,'branch','-D','historical')
            root=temp/'shared-source'
            subprocess.run(['git','clone','--quiet','--no-local','--single-branch',str(remote),str(root)],check=True)
            canonical='https://github.com/eng-cc/oasis7.git'
            git(root,'remote','set-url','origin',canonical); git(root,'config','url.'+str(remote)+'.insteadOf',canonical)
            self.assertNotEqual(subprocess.run(['git','-C',str(root),'cat-file','-e',source+'^{commit}'],capture_output=True).returncode,0)
            git(root,'fetch','origin','main')
            fetch_head_before=(root/'.git/FETCH_HEAD').read_bytes()
            journal=temp/'requests'; original=module.git; nested=[]; state={'fetches':0}
            request=lambda key: {'request_key':key,'binding':{'loop':'code'},'worktree':'/'+key,'branch':'codex/'+key}
            def interleave(checkout,*args):
                result=original(checkout,*args)
                if args[0]=='fetch':
                    state['fetches']+=1
                    if state['fetches']==1:
                        (author/'next.txt').write_text('new default head'); git(author,'add','.'); git(author,'commit','-qm','main advances'); git(author,'push','origin','main')
                        state['next']=git(author,'rev-parse','HEAD')
                        nested.append(module.prepare_request(journal,request('second'),root))
                    elif state['fetches']==2:
                        contracts.ensure_contract_objects(root,{'source_head':source,'merged_head':initial,'approval_ref':{'pr_number':7}})
                return result
            with patch.object(module,'git',side_effect=interleave):
                first=module.prepare_request(journal,request('first'),root)
            self.assertEqual(first,initial,'first request must retain its advertised main, not shared FETCH_HEAD')
            self.assertEqual(nested,[state['next']],'second request must pin its own advertised main')
            self.assertEqual((root/'.git/FETCH_HEAD').read_bytes(),fetch_head_before,'bootstrap and contract fetches must not mutate FETCH_HEAD')
            self.assertEqual(module.prepare_request(journal,request('first'),root),initial)
            self.assertEqual(git(root,'rev-parse',source+'^{commit}'),source)

    def test_same_request_reuses_fetched_base_and_rejects_scope_drift(self):
        spec = importlib.util.spec_from_file_location('loop_bootstrap', PATH)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            request = {'request_key': 'one', 'binding': {'loop': 'code'}, 'worktree': '/task', 'branch': 'codex/task'}
            with patch.object(module, 'fetch_base', return_value='a' * 40) as fetch:
                self.assertEqual(module.prepare_request(root, request, root), 'a' * 40)
                self.assertEqual(module.prepare_request(root, request, root), 'a' * 40)
                fetch.assert_called_once()
                with self.assertRaises(ValueError):
                    module.prepare_request(root, dict(request, branch='different'), root)

class BootstrapPurpose(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'repo'
        self.root.mkdir()
        spec = importlib.util.spec_from_file_location('loop_bootstrap', PATH)
        self.module = importlib.util.module_from_spec(spec); spec.loader.exec_module(self.module)
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('config', 'user.name', 'Fixture')
        (self.root / 'file').write_text('base')
        self.git('add', '.'); self.git('commit', '-qm', 'base')
        self.base = self.git('rev-parse', 'HEAD')
        remote = Path(self.temp.name) / 'origin.git'
        subprocess.run(['git', 'clone', '--bare', '-q', str(self.root), str(remote)], check=True)
        self.git('remote', 'add', 'origin', str(remote))
        self.target = (Path(self.temp.name) / 'task').resolve()
        self.binding = {'task_uid': 'task_' + 'a' * 32, 'loop': 'code', 'request_key': 'request:1', 'manual_request_ref': 'message:1'}
        self.source = Path(self.temp.name) / 'binding.json'
        self.source.write_text(json.dumps(self.binding))
        self.purposes = []
        self.eligible = {'new_tasks'}
        self.live = None
        def validate(root, binding, repository, purpose):
            self.purposes.append(purpose)
            if purpose not in self.eligible:
                raise ValueError('contract withdrawn or ineligible for ' + purpose)
        task = SimpleNamespace(validate_loop_inputs=validate, github_issue_record=lambda *args: self.live)
        original = self.module.load
        self.patch = patch.object(self.module, 'load', side_effect=lambda name: task if name == 'github-project-task' else original(name))
        self.patch.start(); self.addCleanup(self.patch.stop)

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True, stderr=subprocess.PIPE).strip()

    def prepare(self):
        args = [str(PATH), 'prepare', '--root', str(self.root), '--binding', str(self.source), '--loop', 'code', '--request-key', 'request:1', '--manual-request-ref', 'message:1', '--worktree', str(self.target), '--branch', 'codex/task']
        output = io.StringIO()
        with patch.object(sys, 'argv', args), contextlib.redirect_stdout(output):
            self.module.main()
        return output.getvalue().strip()

    def creation(self, state):
        key = hashlib.sha256('\0'.join(('eng-cc/oasis7', 'manual-request', 'request:1')).encode()).hexdigest()
        path = self.root / '.git/oasis7-bootstrap-journal' / (key + '.json')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({'state': state, 'task_uid': self.binding['task_uid'], 'immutable_request': {'repo': 'eng-cc/oasis7', 'request_key': 'request:1', 'loop_binding': self.binding, 'worktree_hint': str(self.target)}}))
        return path

    def test_new_only_input_retries_after_worktree_creation_failure(self):
        self.assertEqual(self.prepare(), self.base)
        self.target.mkdir(); (self.target / 'obstruction').write_text('occupied')
        failed = subprocess.run(['git', '-C', str(self.root), 'worktree', 'add', '--detach', str(self.target), self.base], capture_output=True)
        self.assertNotEqual(failed.returncode, 0)
        self.assertFalse((self.root / '.git/oasis7-bootstrap-journal').exists())
        (self.target / 'obstruction').unlink(); self.target.rmdir()
        self.assertEqual(self.prepare(), self.base)
        subprocess.run(['git', '-C', str(self.root), 'worktree', 'add', '--detach', str(self.target), self.base], check=True, capture_output=True)
        self.assertEqual(self.purposes, ['new_tasks', 'new_tasks'])

    def test_uncertain_creation_keeps_new_task_admission_and_journal(self):
        self.prepare()
        path = self.creation('planned')
        journal = json.loads(path.read_text()); journal['creation_outcome'] = 'uncertain'
        path.write_text(json.dumps(journal)); before = path.read_bytes()
        self.assertEqual(self.prepare(), self.base)
        self.assertEqual(self.purposes, ['new_tasks', 'new_tasks'])
        self.assertEqual(path.read_bytes(), before)

    def test_completed_creation_uses_live_existing_task_admission(self):
        self.creation('completed')
        self.live = {'loop_binding': self.binding, 'worktree_hint': str(self.target)}
        self.eligible = {'in_flight'}
        self.assertEqual(self.prepare(), self.base)
        self.assertEqual(self.purposes, ['in_flight'])

    def test_completed_creation_requires_matching_live_binding(self):
        self.prepare(); self.creation('completed')
        self.eligible = {'new_tasks', 'in_flight'}
        for live in (None, {'loop_binding': {}, 'worktree_hint': str(self.target)}, {'loop_binding': self.binding, 'worktree_hint': '/other'}):
            self.live = live
            with self.subTest(live=live), self.assertRaisesRegex(ValueError, 'live.*mismatch'):
                self.prepare()

if __name__ == '__main__': unittest.main()
