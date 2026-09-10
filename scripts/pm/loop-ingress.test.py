"""Effective ingress regression: candidate wrappers cannot replace base checks."""
from pathlib import Path
import subprocess
import tempfile
import unittest


class IngressTests(unittest.TestCase):
    def test_workflow_selects_base_script_and_candidate_stub_is_ignored(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        self.assertIn('git show "${base_ref}:scripts/pm/loop-ci.py"', workflow)
        self.assertNotIn('python3 scripts/pm/loop-ci.py', workflow)
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            git = lambda *args: subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()
            git('init', '-q')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            helper = repo / 'scripts/pm/loop-ci.py'
            helper.parent.mkdir(parents=True)
            helper.write_text('raise SystemExit(2)\n')
            git('add', '.')
            git('commit', '-qm', 'effective gate')
            base = git('rev-parse', 'HEAD')
            helper.write_text('raise SystemExit(0)\n')
            selected = repo / 'effective.py'
            selected.write_text(git('show', base + ':scripts/pm/loop-ci.py'))
            self.assertEqual(subprocess.run(['python3', str(selected)]).returncode, 2)

    def test_prepare_does_not_import_candidate_gate(self):
        script = (Path(__file__).resolve().parents[1] / 'prepare-task-pr.sh').read_text()
        self.assertNotIn('from loop_gate import admission', script)
        self.assertIn("'trusted helper bytes mismatch'", script)

    def test_hosted_checks_use_only_repository_token(self):
        workflow = (Path(__file__).resolve().parents[2] / '.github/workflows/rust.yml').read_text()
        admission, planner = workflow.split('      - id: scope', 1)
        self.assertNotIn('OASIS7_LOOP_READ_TOKEN', workflow)
        self.assertNotIn('secrets.', workflow)
        self.assertIn('GH_TOKEN: ${{ github.token }}', admission)
        self.assertIn('issues: read', admission)
        self.assertIn('pull-requests: read', admission)

    def test_promotion_rechecks_local_admission_before_ready(self):
        source = (Path(__file__).resolve().parents[1] / 'prepare-task-pr.sh').read_text()
        promotion = source[source.index('promote_draft ci_ready_receipt authority does not match'):]
        self.assertLess(promotion.index('loop-local-gate.py'), promotion.index('gh pr ready'))


    def test_direct_local_legacy_gate_requires_canonical_uid(self):
        import importlib.util, json, sys, io
        from contextlib import redirect_stdout
        from unittest.mock import patch
        path=Path(__file__).with_name('loop-local-gate.py')
        spec=importlib.util.spec_from_file_location('local_gate_test',path)
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        uid='task_'+'a'*32
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);mapping=root/'.pm/github-project-sync/tasks.json';mapping.parent.mkdir(parents=True)
            mapping.write_text(json.dumps({'tasks':{uid:{'task_uid':uid,'repository':'owner/repo','issue_number':1}}}))
            for body in ['old '+uid,'task_uid: task_'+'b'*32+'\nold '+uid,'task_uid: '+uid+'\ntask_uid: '+uid,'task_uid: '+uid]:
                with self.subTest(body=body),patch.object(sys,'argv',['gate','--root',str(root),'--task-uid',uid,'--base','base','--head','head']),patch.object(module,'run',side_effect=[json.dumps({'body':body}),'[]']),redirect_stdout(io.StringIO()):
                    self.assertEqual(module.main(),0 if body=='task_uid: '+uid else 2)

if __name__ == '__main__': unittest.main()
