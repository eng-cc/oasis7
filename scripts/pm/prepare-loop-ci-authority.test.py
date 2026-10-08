#!/usr/bin/env python3
"""Hosted isolated bootstrap tests; only external GitHub service is simulated."""
import contextlib
import io
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('bootstrap_completion_fixture', HERE / 'task-primary-package.test.py')
fixture_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture_module)


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixture_module.CompletionTests(methodName='runTest')
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        f = self.fixture
        # Preserve canonical GitHub identity while real Git transport stays
        # inside the fixture; effective-policy identity reads inspect origin.
        f.git('config', f'url.{f.root}.insteadOf', 'git@github.com:eng-cc/oasis7.git')
        policy = (f.root / 'scripts/pm/loop-policy.v1.json').read_bytes()
        self.binding = {'schema': 'oasis7.loop-task/v1', 'task_uid': fixture_module.UID,
            'owner_role': 'runtime_engineer', 'bootstrap_epoch': 1, 'loop': 'code',
            'change_id': 'alpha', 'manual_request_ref': 'existing-request', 'request_key': 'alpha',
            'target_delivery': 'alpha', 'policy_commit': f.base,
            'policy_digest': 'sha256:' + hashlib.sha256(policy).hexdigest(),
            'write_scope': ['crates/alpha/src/lib.rs'], 'out_of_scope': [],
            'acceptance_refs': ['existing acceptance'], 'dependencies': [], 'input_contracts': []}
        f.record['loop_binding'] = self.binding
        self.save()
        self.bundle = f.root / '.pm/trusted-B'
        self.bundle.mkdir()
        archive = subprocess.Popen(['git', '-C', str(f.root), 'archive', f.base, 'scripts'], stdout=subprocess.PIPE)
        subprocess.run(['tar', '-x', '-C', str(self.bundle)], stdin=archive.stdout, check=True)
        archive.stdout.close()
        self.assertEqual(archive.wait(), 0)
        self.bin = f.root / '.pm/bin'; self.bin.mkdir()
        gh = self.bin / 'gh'
        gh.write_text('#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\ns=json.loads(Path(os.environ["BOOTSTRAP_SERVICES"]).read_text())\nif sys.argv[1]=="api" and sys.argv[2] in s["_auth"]:print(json.dumps(s["_auth"][sys.argv[2]]))\nelif sys.argv[1:3]==["issue","list"]:print(json.dumps([{"number":12}]))\nelif sys.argv[1:3]==["project","view"]:print(json.dumps({"id":"project"}))\nelif sys.argv[1:3]==["api","graphql"]:print(json.dumps(s["_graphql"]))\nelif "/issues/comments/" in sys.argv[2]:print(json.dumps(next(c for c in s["_comments"] if c["id"]==int(sys.argv[2].rsplit("/",1)[1]))))\nelif "/collaborators/" in sys.argv[2]:print(json.dumps({"permission":"admin"}))\nelif "/comments" in sys.argv[2]:\n if s.get("_history_error"):raise SystemExit(55)\n print(json.dumps([[{"body":"oasis7-loop-binding-history"}]] if s.get("_history") else [[]]))\nelse:print(json.dumps(s))\n')
        gh.chmod(0o755)

    def save(self):
        f = self.fixture
        c = fixture_module.contract
        if not f.comments:
            f.record['primary_package'] = 'alpha'
            proof = {'schema': c.LOOP_PROOF_SCHEMA, 'loop_binding': self.binding, 'primary_package': 'alpha'}
            proof['proof_digest'] = c.digest(proof)
            before = {'identity': c.immutable_identity(f.record), 'primary_present': False, 'primary_package': None}
            payload = {'schema': c.SCOPE_SCHEMA, 'before': before, 'primary_package': 'alpha', 'scope_evidence': proof}
            payload['action_id'] = c.digest({key: payload[key] for key in ('schema', 'before', 'primary_package')})
            f.record[c.FIELD] = {'payload': payload, 'server': {'repository': fixture_module.REPO, 'issue_number': 12,
                'comment_id': 99, 'comment_url': f.record['issue_url'] + '#issuecomment-99',
                'body_sha256': 'sha256:' + hashlib.sha256(c.completion_body(payload).encode()).hexdigest()}}
        f.save()
        body = fixture_module.facade.issue_body(fixture_module.facade.task_from_record(fixture_module.UID, f.record))
        f.body = body
        self.services = f.root / '.pm/services.json'
        fields = {'Task UID': fixture_module.UID, 'Owner Role': f.record['owner_role'],
                  'Canonical Worktree': f.record['canonical_worktree'],
                  'Loop': self.binding['loop'], 'Change ID': self.binding['change_id']}
        item = {'id': 'item', 'project': {'id': 'project', 'number': 1}, 'fieldValues': {
            'pageInfo': {'hasNextPage': False}, 'nodes': [{'text': value, 'field': {'name': key}} for key, value in fields.items()]}}
        selected = {'number': 12, 'url': f.record['issue_url'], 'body': body,
                    'projectItems': {'pageInfo': {'hasNextPage': False}, 'nodes': [item]}}
        self.services.write_text(json.dumps({'number': 12, 'html_url': f.record['issue_url'], 'body': body,
            '_auth': {f'repos/{fixture_module.REPO}/actions/runs/1': {'event':'workflow_dispatch','head_branch':'main','head_sha':f.base,'path':'.github/workflows/rust.yml'}, f'repos/{fixture_module.REPO}': {'default_branch':'main'}, f'repos/{fixture_module.REPO}/git/ref/heads/main': {'object':{'sha':f.base}}, f'repos/{fixture_module.REPO}/environments/oasis7-project-read': {'deployment_branch_policy':{'custom_branch_policies':True,'protected_branches':False}}, f'repos/{fixture_module.REPO}/environments/oasis7-project-read/deployment-branch-policies': {'branch_policies':[{'name':'main','type':'branch'}]}}, '_comments': f.comments, '_graphql': {'data': {'repository': {'issue': selected}}}}))

        services = json.loads(self.services.read_text())
        for endpoint in (f'repos/{fixture_module.REPO}/branches/main',
                f'repos/{fixture_module.REPO}/contents/scripts/pm/loop-policy.v1.json?ref={f.base}',
                f'repos/{fixture_module.REPO}/contents/doc/engineering/workflow/source-of-truth.md?ref={f.base}'):
            services['_auth'][endpoint] = json.loads(f.policy_authority_response(['gh', 'api', endpoint]).stdout)
        self.services.write_text(json.dumps(services))

    def run_bootstrap(self, override=None):
        f = self.fixture
        env = {**os.environ, 'PATH': str(self.bin) + os.pathsep + os.environ['PATH'],
               'BOOTSTRAP_SERVICES': str(self.services), **self.auth_env()}
        env.pop('OASIS7_LOOP_TOOL_ROOT', None)
        env.update(override or {})
        return subprocess.run([sys.executable, '-I', str(self.bundle / 'scripts/pm/prepare-loop-ci-authority.py'),
            '--repo-root', str(f.root), '--repository', fixture_module.REPO,
            '--task-uid', fixture_module.UID, '--authority-base', f.base, '--destination', str(f.root / '.pm/pinned-policy')],
            cwd=f.root, env=env, text=True, capture_output=True)

    def auth_env(self):
        return {'GITHUB_EVENT_NAME': 'workflow_dispatch', 'INTEGRATION_MODE': 'integration_revalidation',
            'GITHUB_REF': 'refs/heads/main', 'GITHUB_WORKFLOW_SHA': self.fixture.base,
            'GITHUB_WORKFLOW_REF': fixture_module.REPO + '/.github/workflows/rust.yml@refs/heads/main',
            'GITHUB_RUN_ID': '1', 'OASIS7_PROJECT_READ_TOKEN': 'external-service-simulation'}

    def test_actual_isolated_bootstrap_pins_same_repository_before_consumer(self):
        # Candidate helper poison must not become executable authority.
        (self.fixture.root / 'scripts/pm/prepare-loop-ci-authority.py').write_text('raise SystemExit(99)\n')
        result = self.run_bootstrap()
        self.assertEqual(result.returncode, 0, result.stderr)
        tool = Path(result.stdout.strip())
        self.assertEqual(self.fixture.git('-C', str(tool), 'rev-parse', 'HEAD'), self.fixture.base)
        self.assertEqual(self.fixture.git('-C', str(tool), 'rev-parse', '--path-format=absolute', '--git-common-dir'),
                         self.fixture.git('rev-parse', '--path-format=absolute', '--git-common-dir'))

    def test_executable_hosted_bootstrap_exports_authority_before_first_consumer(self):
        f = self.fixture
        workflow = (HERE.parents[1] / '.github/workflows/rust.yml').read_text()
        body = workflow.split('          unset OASIS7_LOOP_TOOL_ROOT\n', 1)[1].split('          "${planner[@]}"', 1)[0]
        runner = f.root / '.pm/runner'
        runner.mkdir()
        env = {**os.environ, 'PATH': str(self.bin) + os.pathsep + os.environ['PATH'],
            'BOOTSTRAP_SERVICES': str(self.services), 'RUNNER_TEMP': str(runner),
            'GITHUB_ENV': str(runner / 'env'), 'GITHUB_REPOSITORY': fixture_module.REPO,
            'task_uid': fixture_module.UID, 'base_ref': f.base}
        env.update(self.auth_env())
        consumer = '\n[[ -n "$OASIS7_LOOP_TOOL_ROOT" ]]\n' \
            '[[ "$(git -C "$OASIS7_LOOP_TOOL_ROOT" rev-parse HEAD)" == "$base_ref" ]]\n' \
            'grep -Fq "OASIS7_LOOP_TOOL_ROOT=$OASIS7_LOOP_TOOL_ROOT" "$GITHUB_ENV"\n' \
            'echo first-consumer-has-pinned-authority\n'
        result = subprocess.run(['bash', '-euo', 'pipefail', '-c', 'unset OASIS7_LOOP_TOOL_ROOT\n' + body + consumer],
            cwd=f.root, env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('first-consumer-has-pinned-authority', result.stdout)

    def test_executable_workflow_bootstrap_then_actual_current_completion_consumer(self):
        f = self.fixture
        f.record.pop(fixture_module.contract.FIELD)
        f.record.pop('primary_package')
        f.save()
        f.body = fixture_module.facade.issue_body(fixture_module.facade.task_from_record(fixture_module.UID, f.record))
        initial_tool = f.root / '.pm/writer-policy'
        f.git('update-ref', 'refs/remotes/origin/main', f.base)
        f.git('worktree', 'add', '--detach', str(initial_tool), f.base)
        f.args.scope_evidence_json = None
        original_output = subprocess.check_output
        original_run = subprocess.run
        def external_output(command, **kwargs):
            if command[0] == 'gh':
                if command[1:3] == ['api', f'repos/{fixture_module.REPO}/issues/12/comments?per_page=100']:
                    self.assertEqual(command[3:], ['--paginate', '--slurp'])
                    return json.dumps([f.comments])
                return json.dumps(f.github(fixture_module.REPO, command[2]))
            return original_output(command, **kwargs)
        def external_run(command, **kwargs):
            if command[0] == 'gh':
                return f.policy_authority_response(command, **kwargs)
            if command[0] == 'git' and 'fetch' in command:
                return subprocess.CompletedProcess(command, 0, b'', b'')
            return original_run(command, **kwargs)
        with patch.dict(os.environ, {'OASIS7_LOOP_TOOL_ROOT': str(initial_tool)}), \
                patch.object(subprocess, 'check_output', side_effect=external_output), \
                patch.object(subprocess, 'run', side_effect=external_run):
            self.assertEqual(f.complete(), 0)
        f.record = f.current()
        self.save()
        f.git('worktree', 'remove', '--force', str(initial_tool))
        workflow = (HERE.parents[1] / '.github/workflows/rust.yml').read_text()
        body = workflow.split('          unset OASIS7_LOOP_TOOL_ROOT\n', 1)[1].split('          "${planner[@]}"', 1)[0]
        runner = f.root / '.pm/current-consumer-runner'; runner.mkdir()
        consumer = runner / 'consume.py'
        consumer.write_text('import importlib.util,json,sys,os\nfrom pathlib import Path\n'
            'assert "OASIS7_PROJECT_READ_TOKEN" not in os.environ\n'
            'spec=importlib.util.spec_from_file_location("actual_primary",sys.argv[1]);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)\n'
            'root=Path(sys.argv[2]);task=json.loads((root/".pm/github-project-sync/tasks.json").read_text())["tasks"][sys.argv[3]]\n'
            'm.validate_current_completion(root,task)\n'
            'assert m.effective_primary_package(task)=="alpha"\nprint("actual-current-loop-completion-consumed")\n')
        env = {**os.environ, 'PATH': str(self.bin) + os.pathsep + os.environ['PATH'],
            'BOOTSTRAP_SERVICES': str(self.services), 'RUNNER_TEMP': str(runner),
            'GITHUB_ENV': str(runner / 'env'), 'GITHUB_REPOSITORY': fixture_module.REPO,
            'task_uid': fixture_module.UID, 'base_ref': f.base, 'ACTUAL_CONSUMER': str(consumer),
            'PYTHON_EXECUTABLE': sys.executable}
        env.update(self.auth_env())
        command = 'unset OASIS7_LOOP_TOOL_ROOT\n' + body + \
            '\n"$PYTHON_EXECUTABLE" -I "$ACTUAL_CONSUMER" "$loop_authority_dir/scripts/pm/task_primary_package.py" "$PWD" "$task_uid"\n'
        result = subprocess.run(['bash', '-euo', 'pipefail', '-c', command], cwd=f.root,
                                env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('actual-current-loop-completion-consumed', result.stdout)

    def legacy_no_cache(self):
        # Use the real completion writer and independently authenticated Issue;
        # the receipt is never a source of missing Project identity.
        f = self.fixture
        f.record.pop(fixture_module.contract.FIELD)
        f.record.pop('primary_package')
        f.record.pop('loop_binding')
        f.save()
        f.body = fixture_module.facade.issue_body(fixture_module.facade.task_from_record(fixture_module.UID, f.record))
        with contextlib.redirect_stdout(io.StringIO()):
            f.complete()
        f.record = f.current()
        self.save()
        self.assertIsNotNone(fixture_module.contract.validate_completion(f.record))
        f.mapping.unlink()
        return f.body, json.dumps(f.comments, sort_keys=True)

    def test_no_cache_legacy_completion_reports_missing_independent_Project_authority(self):
        body, comments = self.legacy_no_cache()
        result = self.run_bootstrap({'GITHUB_EVENT_NAME': 'pull_request',
            'GITHUB_REF': 'refs/pull/7/merge', 'OASIS7_PROJECT_READ_TOKEN': ''})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Project-read setup:', result.stderr)
        self.assertNotIn('completion immutable Task identity differs', result.stderr)
        self.assertEqual(self.fixture.body, body)
        self.assertEqual(json.dumps(self.fixture.comments, sort_keys=True), comments)
        self.assertFalse(self.fixture.mapping.exists())
        self.assertFalse((self.fixture.root / '.pm/policy').exists())

    def test_no_cache_candidate_cannot_use_broader_Project_token(self):
        self.legacy_no_cache()
        result = self.run_bootstrap({'GITHUB_EVENT_NAME': 'pull_request',
            'GITHUB_REF': 'refs/pull/7/merge', 'OASIS7_PROJECT_READ_TOKEN': 'candidate-must-not-use'})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Project-read setup:', result.stderr)
        self.assertNotIn('completion immutable Task identity differs', result.stderr)
        self.assertFalse(self.fixture.mapping.exists())

    def test_missing_policy_object_rejects_without_candidate_fallback(self):
        self.binding['policy_commit'] = 'f' * 40
        self.save()
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.fixture.root / '.pm/pinned-policy').exists())

    def test_foreign_policy_commit_not_in_origin_main_rejects(self):
        f = self.fixture
        self.binding['policy_commit'] = f.git('commit-tree', f.git('rev-parse', 'HEAD^{tree}'), '-m', 'foreign policy')
        self.save()
        self.assertNotEqual(self.run_bootstrap().returncode, 0)

    def test_task_binding_owner_identity_mismatch_rejects(self):
        self.binding['owner_role'] = 'qa_engineer'
        self.save()
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('loop/task identity mismatch: owner_role', result.stderr)

    def test_no_live_Project_backing_rejects_unique_Issue(self):
        services = json.loads(self.services.read_text())
        services['_graphql']['data']['repository']['issue']['projectItems']['nodes'] = []
        self.services.write_text(json.dumps(services))
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Project-backed Task missing', result.stderr)

    def test_absent_epoch_cannot_self_satisfy_migrated_binding(self):
        self.fixture.record.pop('bootstrap_epoch')
        self.binding['bootstrap_epoch'] = 2
        self.save()
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('loop/task identity mismatch: bootstrap_epoch', result.stderr)

    def clear_completion(self, clear_binding=False):
        f = self.fixture
        f.record.pop(fixture_module.contract.FIELD, None)
        f.record.pop('primary_package', None)
        if clear_binding:
            f.record.pop('loop_binding', None)
        f.save()
        f.body = fixture_module.facade.issue_body(fixture_module.facade.task_from_record(fixture_module.UID, f.record))
        services = json.loads(self.services.read_text())
        services['body'] = f.body
        services['_graphql']['data']['repository']['issue']['body'] = f.body
        self.services.write_text(json.dumps(services))

    def test_live_no_loop_proof_bypass_does_not_write_mapping_or_require_project_auth(self):
        self.clear_completion(clear_binding=True)
        before = self.fixture.mapping.read_bytes()
        result = self.run_bootstrap({'GITHUB_REF': 'refs/pull/7/merge', 'OASIS7_PROJECT_READ_TOKEN': ''})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), '')
        self.assertEqual(before, self.fixture.mapping.read_bytes())
        self.assertFalse((self.fixture.root / '.pm/pinned-policy').exists())

    def test_live_deleted_binding_cannot_take_no_proof_bypass(self):
        self.clear_completion()
        services = json.loads(self.services.read_text())
        services['body'] = '\n'.join(line for line in services['body'].splitlines() if not line.startswith('- loop_binding_b64:'))
        self.services.write_text(json.dumps(services))
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('live loop binding disappeared', result.stderr)

    def test_malformed_completion_cannot_take_no_proof_bypass(self):
        self.clear_completion(clear_binding=True)
        services = json.loads(self.services.read_text())
        services['body'] += '\n- primary_package_completion_b64: `broken`\n'
        self.services.write_text(json.dumps(services))
        self.assertNotEqual(self.run_bootstrap().returncode, 0)

    def test_missing_B_parser_dependency_does_not_execute_candidate_fallback(self):
        (self.bundle / 'scripts/pm/workflow-durable-store.py').unlink()
        (self.fixture.root / 'scripts/pm/workflow-durable-store.py').write_text('raise SystemExit("candidate-fallback-executed")\n')
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('trusted bootstrap dependency unavailable: workflow-durable-store', result.stderr)
        self.assertNotIn('candidate-fallback-executed', result.stderr)

    def test_deleted_immutable_history_cannot_take_no_proof_bypass(self):
        self.clear_completion(clear_binding=True)
        services = json.loads(self.services.read_text()); services['_history'] = True
        self.services.write_text(json.dumps(services))
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('deleted after immutable binding history', result.stderr)

    def test_history_API_failure_cannot_take_no_proof_bypass(self):
        self.clear_completion(clear_binding=True)
        services = json.loads(self.services.read_text()); services['_history_error'] = True
        self.services.write_text(json.dumps(services))
        self.assertNotEqual(self.run_bootstrap().returncode, 0)

    def test_arbitrary_dispatch_ref_rejected_before_project_credential_use(self):
        result = self.run_bootstrap({'GITHUB_REF': 'refs/heads/candidate'})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('trusted default-B integration context required', result.stderr)

    def test_unprotected_reader_surface_rejected(self):
        services = json.loads(self.services.read_text())
        services['_auth'][f'repos/{fixture_module.REPO}/environments/oasis7-project-read']['deployment_branch_policy'] = None
        self.services.write_text(json.dumps(services))
        result = self.run_bootstrap()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('protected default-main-only reader surface required', result.stderr)

    def test_missing_project_credential_is_setup_blocker_not_bypass(self):
        result = self.run_bootstrap({'OASIS7_PROJECT_READ_TOKEN': ''})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('approved environment-scoped read credential missing', result.stderr)

    def test_actual_workflow_bootstrap_precedes_planner_and_old_B_is_explicit(self):
        workflow = (HERE.parents[1] / '.github/workflows/rust.yml').read_text()
        scope = workflow.split('      - id: scope\n', 1)[1].split('      - name: Report planned scope', 1)[0]
        self.assertLess(scope.index('--destination "${RUNNER_TEMP}/loop-policy-authority"'), scope.index('"${planner[@]}"'))
        self.assertIn('git archive "${base_ref}" scripts', scope)
        self.assertIn('trusted B loop completion bootstrap dependency is unavailable', scope)
        self.assertNotIn('python3 -I scripts/pm/prepare-loop-ci-authority.py', scope)

    def workflow_historical_bundle(self, declares_loop):
        workflow = (HERE.parents[1] / '.github/workflows/rust.yml').read_text()
        body = workflow.split('          unset OASIS7_LOOP_TOOL_ROOT\n', 1)[1].split('          "${planner[@]}"', 1)[0]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q'); git('config', 'user.email', 'test@example.invalid'); git('config', 'user.name', 'Test')
            (root / 'scripts/pm').mkdir(parents=True)
            (root / 'scripts/legacy.py').write_text('# actual old B\n')
            if declares_loop:
                (root / 'scripts/pm/task_primary_package.py').write_text('LOOP_PROOF_SCHEMA="declared"\n')
            git('add', '.'); git('commit', '-qm', 'historical B')
            base = git('rev-parse', 'HEAD')
            # A candidate helper is never used to repair missing B authority.
            (root / 'scripts/pm/prepare-loop-ci-authority.py').write_text('raise SystemExit(99)\n')
            env = {**os.environ, 'RUNNER_TEMP': str(root / 'runner'), 'GITHUB_ENV': str(root / 'env'),
                'GITHUB_REPOSITORY': fixture_module.REPO, 'base_ref': base, 'task_uid': fixture_module.UID}
            return subprocess.run(['bash', '-euo', 'pipefail', '-c', 'unset OASIS7_LOOP_TOOL_ROOT\n' + body + '\necho actual-legacy-planner'],
                                  cwd=root, env=env, capture_output=True, text=True)

    def test_old_B_without_completion_preserves_actual_legacy(self):
        result = self.workflow_historical_bundle(False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('actual-legacy-planner', result.stdout)

    def test_declared_B_completion_missing_bootstrap_rejects(self):
        result = self.workflow_historical_bundle(True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('bootstrap dependency is unavailable', result.stderr)
        self.assertNotIn('actual-legacy-planner', result.stdout)


if __name__ == '__main__':
    unittest.main()
