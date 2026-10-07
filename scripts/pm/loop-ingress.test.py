"""Effective ingress regression: ordinary PR wrappers retain trusted base checks."""
from pathlib import Path
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest


class IngressTests(unittest.TestCase):
    @staticmethod
    def workflow_run_body(workflow: str, step_marker: str, end_marker: str) -> str:
        start = workflow.index(step_marker)
        run = workflow.index("        run: |\n", start) + len("        run: |\n")
        end = workflow.index(end_marker, run)
        lines = workflow[run:end].splitlines()
        return "\n".join(line[10:] if line.startswith("          ") else line for line in lines)

    def test_current_target_source_ci_executes_protected_q_driver_on_actual_checkout(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        script = self.workflow_run_body(workflow, '      - name: Run required test tier',
            '      - name: Verify final task and PR binding before required-gate success')
        with tempfile.TemporaryDirectory() as tmp:
            fixture=Path(tmp); repo=fixture/'repo'; repo.mkdir()
            def git(*args): return subprocess.check_output(['git','-C',str(repo),*args],text=True).strip()
            git('init','-q','-b','main');git('config','user.name','Fixture');git('config','user.email','fixture@example.invalid')
            driver=repo/'scripts/ci-tests.sh';driver.parent.mkdir()
            driver.write_text('#!/bin/bash\nprintf "protected-Q:%s" "$*" > "$OBSERVED"\n')
            driver.chmod(0o755)
            git('add','.');git('commit','-qm','protected Q driver');q=git('rev-parse','HEAD')
            driver.write_text('#!/bin/bash\nprintf "candidate-H:%s" "$*" > "$OBSERVED"\n')
            git('add','.');git('commit','-qm','candidate H');h=git('rev-parse','HEAD')
            runner=fixture/'runner';runner.mkdir();(runner/'impact-projection.json').write_text('{}')
            observed=fixture/'observed'
            import re
            script=re.sub(r'\$\{\{[^}]+\}\}', q, script)
            env={**os.environ,'GITHUB_EVENT_NAME':'pull_request','INTEGRATION_MODE':'',
                 'GITHUB_WORKSPACE':str(repo),'RUNNER_TEMP':str(runner),'OBSERVED':str(observed),
                 'OASIS7_CARGO_SCOPE_BASE':'','OASIS7_PRODUCT_DOC_BASE':q,'OASIS7_PRODUCT_DOC_HEAD':h,
                 'CURRENT_TARGET_OID':q,'GITHUB_SHA':h}
            result=subprocess.run(['bash','-euo','pipefail','-c',script],cwd=repo,env=env,text=True,capture_output=True)
            self.assertEqual(0,result.returncode,result.stdout+result.stderr)
            self.assertTrue(observed.read_text().startswith('protected-Q:'),observed.read_text())
            self.assertIn('--repo-root '+str(repo),observed.read_text())

    def run_trusted_helper_route(self, phase_helper: bool, *, final: bool = False):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        if final:
            script = self.workflow_run_body(
                workflow,
                '      - name: Verify final task and PR binding before required-gate success',
                '      - name: Upload Cargo package profile plan',
            )
        else:
            script = self.workflow_run_body(
                workflow,
                '      - id: loop-ci-admission',
                '      - name: Report planned scope',
            )
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Path(tmp)
            repo = fixture / 'repo'
            repo.mkdir()
            git = lambda *args: subprocess.check_output(
                ['git', '-C', str(repo), *args], text=True,
            ).strip()
            git('init', '-q', '-b', 'main')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            helper = repo / 'scripts/pm/loop-ci.py'
            helper.parent.mkdir(parents=True)
            trusted = '# TRUSTED_PHASE_HELPER\n' if phase_helper else '# TRUSTED_LEGACY_HELPER\n'
            helper.write_text(trusted, encoding='utf-8')
            git('add', '.')
            git('commit', '-qm', 'trusted base helper')
            base = git('rev-parse', 'HEAD')
            helper.write_text('# CANDIDATE_UNTRUSTED_PHASE_HELPER\n', encoding='utf-8')

            log = fixture / 'python-args.jsonl'
            bin_dir = fixture / 'bin'
            bin_dir.mkdir()
            python_shim = bin_dir / 'python3'
            shim_code = (
                "import json,os,pathlib,sys\n"
                "args=sys.argv[1:]\n"
                "script_index=next((index for index,arg in enumerate(args) if arg=='-'),None)\n"
                "if script_index is not None:\n"
                "    sys.argv=['-']+args[script_index+1:]\n"
                "    source=sys.stdin.read()\n"
                "    exec(compile(source,'<stdin>','exec'),{'__name__':'__main__'})\n"
                "    raise SystemExit(0)\n"
                "if args and args[0]=='-I': args=args[1:]\n"
                "script=pathlib.Path(args[0])\n"
                "body=script.read_text(encoding='utf-8')\n"
                "if '--help' in args[1:]:\n"
                " print('usage loop-ci.py' + (' --phase' if 'TRUSTED_PHASE_HELPER' in body else ''))\n"
                " raise SystemExit(0)\n"
                "with open(os.environ['WORKFLOW_SHIM_LOG'],'a',encoding='utf-8') as out:\n"
                " out.write(json.dumps({'argv':args,'candidate': 'CANDIDATE_UNTRUSTED' in body})+'\\n')\n"
                "raise SystemExit(0)\n"
            )
            python_shim.write_text(
                '#!/bin/sh\nexec ' + shlex.quote(sys.executable) + ' -I -c '
                + shlex.quote(shim_code) + ' "$@"\n', encoding='utf-8',
            )
            python_shim.chmod(0o755)
            replacements = {
                '${{ github.event.pull_request.base.sha || inputs.integration_base }}': base,
                '${{ steps.pr_target.outputs.oid || inputs.integration_base }}': base,
                '${{ github.event.pull_request.head.sha || inputs.expected_head }}': 'b' * 40,
                '${{ github.repository }}': 'fixture/repo',
                '${{ github.event.pull_request.number || inputs.pr_number }}': '2',
                '${{ steps.loop-ci-admission.outputs.start_only }}': 'true',
                '${{ steps.scope.outputs.source_scope_base }}': base,
                '${{ steps.scope.outputs.maintenance_authority_comment_id }}': '',
                '${{ steps.scope.outputs.planner_config_sha256 }}': 'sha256:' + '0' * 64,
                '${{ steps.scope.outputs.planner_digest }}': 'sha256:' + '1' * 64,
                '${{ steps.scope.outputs.impact_projection_digest }}': 'sha256:' + '2' * 64,
            }
            for needle, value in replacements.items():
                script = script.replace(needle, value)
            environment = dict(os.environ)
            environment.update({
                'PATH': str(bin_dir) + os.pathsep + environment.get('PATH', ''),
                'RUNNER_TEMP': str(fixture / 'runner'),
                'GITHUB_OUTPUT': str(fixture / 'github-output'),
                'GITHUB_EVENT_NAME': 'pull_request',
                'GITHUB_SHA': 'b' * 40,
                'WORKFLOW_SHIM_LOG': str(log),
            })
            (fixture / 'runner').mkdir()
            (fixture / 'github-output').write_text('', encoding='utf-8')
            # Extracted hosted workflow uses modern Ubuntu Bash; prefer installed
            # Homebrew Bash over macOS system Bash 3.2 for this shell fixture.
            workflow_bash = '/opt/homebrew/bin/bash' if (
                sys.platform == 'darwin' and Path('/opt/homebrew/bin/bash').is_file()
            ) else (shutil.which('bash') or 'bash')
            completed = subprocess.run(
                [workflow_bash, '-euo', 'pipefail', '-c', script], cwd=repo,
                env=environment, text=True, capture_output=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr + completed.stdout)
            invocations = [json.loads(line) for line in log.read_text().splitlines()]
            self.assertEqual(len(invocations), 1, invocations)
            self.assertTrue(all(not item['candidate'] for item in invocations), invocations)
            return [item['argv'][1:] for item in invocations]

    def test_workflow_selects_base_script_and_candidate_stub_is_ignored(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        self.assertIn('tool_ref="${base_ref}"', workflow)
        self.assertIn('git show "${tool_ref}:scripts/pm/loop-ci.py"', workflow)
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

    def test_final_binding_is_a_required_gate_barrier_before_artifact_uploads(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        validation = workflow.index('      - id: loop-ci-admission')
        tests = workflow.index('      - name: Run required test tier')
        final = workflow.index('      - name: Verify final task and PR binding before required-gate success')
        uploads = workflow.index('      - name: Upload Cargo package profile plan')
        self.assertLess(validation, tests)
        self.assertLess(tests, final)
        self.assertLess(final, uploads)
        final_step = workflow[final:uploads]
        self.assertIn('tool_ref="${base_ref}"', final_step)
        self.assertIn('git show "${tool_ref}:scripts/pm/loop-ci.py"', final_step)
        self.assertIn('--phase final --tests-passed', final_step)
        self.assertIn("if python3 -I \"${RUNNER_TEMP}/oasis7-loop-ci-final.py\" --help 2>&1 | grep -Fq -- '--phase'; then", final_step)
        self.assertNotIn('continue-on-error', final_step)
        self.assertNotIn('always()', final_step)

    def test_same_pr_uses_old_trusted_helper_until_base_exposes_phase_api(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / '.github/workflows/rust.yml').read_text()
        validation = workflow[workflow.index('      - id: loop-ci-admission'):]
        validation = validation[:validation.index('      - name: Report planned scope')]
        self.assertIn('tool_ref="${base_ref}"', validation)
        self.assertIn('git show "${tool_ref}:scripts/pm/loop-ci.py"', validation)
        self.assertIn("if python3 -I \"${RUNNER_TEMP}/oasis7-loop-ci.py\" --help 2>&1 | grep -Fq -- '--phase'; then", validation)
        self.assertIn('python3 -I "${RUNNER_TEMP}/oasis7-loop-ci.py" --phase start', validation)
        self.assertNotIn('python3 scripts/pm/loop-ci.py', validation)

    def test_old_trusted_helper_gets_legacy_args_even_with_phase_candidate(self):
        before = self.run_trusted_helper_route(phase_helper=False)
        after = self.run_trusted_helper_route(phase_helper=False, final=True)
        self.assertEqual(before[0], ['--repository', 'fixture/repo', '--pr-number', '2', '--base', self._oid_from_args(before[0]), '--head', 'b' * 40])
        self.assertEqual(after[0][:2], ['--repository', 'fixture/repo'])
        self.assertNotIn('--phase', before[0] + after[0])
        self.assertNotIn('--tests-passed', after[0])
        self.assertNotIn('--start-only', after[0])

    def test_phase_args_are_enabled_only_by_the_trusted_helper_capability(self):
        before = self.run_trusted_helper_route(phase_helper=True)
        after = self.run_trusted_helper_route(phase_helper=True, final=True)
        self.assertEqual(before[0][:2], ['--phase', 'start'])
        self.assertEqual(after[0][:3], ['--phase', 'final', '--tests-passed'])
        self.assertIn('--start-only', after[0])

    @staticmethod
    def _oid_from_args(arguments):
        return arguments[arguments.index('--base') + 1]

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

    def test_local_promotion_gate_reuses_bound_traceability_admission(self):
        source = Path(__file__).with_name('loop-local-gate.py').read_text()
        self.assertIn("module.pre_mutation_admission(\n                'promotion'", source)


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

def load_tests(loader, tests, pattern):
    """Include maintenance authority checks in the registered ingress test entrypoint."""
    import importlib.util
    path = Path(__file__).with_name('workflow_maintenance.test.py')
    spec = importlib.util.spec_from_file_location('workflow_maintenance_ingress_tests', path)
    if spec is None or spec.loader is None:
        raise ImportError('maintenance authority test loader is unavailable')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    tests.addTests(loader.loadTestsFromModule(module))
    return tests


if __name__ == '__main__': unittest.main()
