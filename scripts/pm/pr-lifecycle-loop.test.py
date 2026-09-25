#!/usr/bin/env python3
"""Production receipts require fresh local admission, independently of CI."""
import importlib.util
from pathlib import Path
import unittest
import base64
import json
import tempfile
import subprocess
import os
import shutil
import sys
from unittest.mock import patch


_APPLICABILITY_FIXTURE_SPEC = importlib.util.spec_from_file_location(
    'ci_evidence_applicability_fixtures',
    Path(__file__).with_name('ci-evidence-applicability.test.py'),
)
_APPLICABILITY_FIXTURES = importlib.util.module_from_spec(_APPLICABILITY_FIXTURE_SPEC)
_APPLICABILITY_FIXTURE_SPEC.loader.exec_module(_APPLICABILITY_FIXTURES)

spec = importlib.util.spec_from_file_location('pr_gate', Path(__file__).with_name('pr-lifecycle-gate.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ProductionLoopTests(unittest.TestCase):
    def test_final_live_pr_admission_rejects_same_oid_drift(self):
        uid = 'task_' + '1' * 32
        data = {'number': 12, 'repository': 'owner/repo', 'state': 'OPEN', 'isDraft': False,
                'body': 'Task: ' + uid + '\nRefs #1', 'baseRefName': 'main', 'headRefName': 'codex/task',
                'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40,
                'mergeable': 'MERGEABLE', 'mergeStateStatus': 'CLEAN', 'reviewDecision': 'APPROVED',
                'merge_hold': {'kind': 'normal_pr_ci_watch', 'active': False},
                'policy_discovery': {'status': 'resolved', 'required_status_checks': []}}
        cases = [('unchanged', {}, True), ('closed', {'state': 'CLOSED'}, False),
                 ('merged', {'state': 'MERGED'}, False), ('draft', {'isDraft': True}, False),
                 ('task', {'body': 'Task: task_' + '2' * 32 + '\nRefs #1'}, False),
                 ('refs', {'body': 'Task: ' + uid + '\nRefs #2'}, False),
                 ('target', {'baseRefName': 'release'}, False),
                 ('source', {'headRefName': 'codex/replaced'}, False)]
        for label, changes, ready in cases:
            fresh = {**data, **changes}
            def read(command, **kwargs):
                self.assertEqual(command[:3], ['gh', 'pr', 'view'])
                return json.dumps({key: fresh[key] for key in command[command.index('--json') + 1].split(',')})
            with self.subTest(label=label), patch.object(gate, 'local_loop_admission', return_value={'status': 'legacy'}), \
                 patch.object(gate, 'live_integration_admission', return_value=None), \
                 patch.object(gate.subprocess, 'check_output', side_effect=read):
                result = gate.production_decision(data, False, Path('/canonical'), uid, None)
                self.assertEqual(result['ready_for_merge'], ready, result)
                self.assertEqual('readiness_receipt' in result, ready, result)

    def test_advanced_target_with_old_green_run_cannot_mint_receipt(self):
        with patch.object(gate,'live_integration_admission',side_effect=ValueError('stale integration base'),create=True):
            result, _ = self.run_gate()
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt',result)

    def test_fresh_integration_run_keeps_original_source_head(self):
        with patch.object(gate,'live_integration_admission',return_value={'head_oid':'b'*40,'integration_base_oid':'a'*40},create=True) as checked:
            result, _ = self.run_gate()
        self.assertIn('readiness_receipt',result)
        self.assertEqual(result['readiness_receipt']['head_oid'],'b'*40)
        checked.assert_called_once()

    def test_legacy_admission_keeps_strict_integration_contract(self):
        with patch.object(gate, 'live_integration_admission', return_value=None) as checked:
            self.run_gate()
        self.assertTrue(checked.call_args.kwargs['require_strict'])

    def run_gate(self, admission=None, fresh=None, ready=True, admission_result=None):
        data = {'number': 12, 'repository': 'owner/repo', 'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40,
                'state': 'OPEN', 'isDraft': False, 'body': 'Task: task_uid\nRefs #1',
                'baseRefName': 'main', 'headRefName': 'codex/task',
                'policy_discovery': {'status':'resolved','required_status_checks':[]}}
        def decide(data, admin, *, evidence_mode):
            result = {'ready_for_merge': ready, 'status': 'ready' if ready else 'held', 'blockers': [] if ready else ['hold']}
            if ready and evidence_mode == 'production': result['readiness_receipt'] = {'head_oid': data['headRefOid']}
            return result
        with patch.object(gate, 'decision', side_effect=decide), patch.object(gate, 'local_loop_admission', side_effect=admission,return_value=admission_result if admission_result is not None else {'status':'legacy'}) as check, patch.object(gate, 'read_pr_identity', return_value=fresh or data):
            result = gate.production_decision(data, False, Path('/canonical'), 'task_uid', None)
        return result, check

    def test_current_admission_uses_classifier_auto_mode(self):
        with patch.object(gate, 'live_integration_admission', return_value=None) as checked:
            self.run_gate(admission_result={'status': 'passed'})
        self.assertEqual(checked.call_args.kwargs['require_strict'], 'auto')

    def test_revoked_contract_cannot_mint_receipt(self):
        result, _ = self.run_gate(admission=ValueError('contract revoked'))
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt', result)
        self.assertIn('contract revoked', ' '.join(result['blockers']))

    def test_failed_authority_cannot_mint_receipt(self):
        result, _ = self.run_gate(admission=ValueError('effective tool root unavailable'))
        self.assertFalse(result['ready_for_merge'])
        self.assertNotIn('readiness_receipt', result)

    def test_head_or_base_drift_cannot_mint_receipt(self):
        for key in ('headRefOid', 'baseRefOid'):
            fresh = {'number': 12, 'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40, key: 'c' * 40}
            result, _ = self.run_gate(fresh=fresh)
            self.assertFalse(result['ready_for_merge'])
            self.assertNotIn('readiness_receipt', result)

    def test_admission_receives_exact_pr_identity(self):
        result, check = self.run_gate()
        self.assertIn('readiness_receipt', result)
        check.assert_called_once_with(Path('/canonical'), 'task_uid', 'a' * 40, 'b' * 40, None)

    def test_hold_preserved_without_admission(self):
        result, check = self.run_gate(ready=False)
        self.assertEqual(result['status'], 'held')
        check.assert_not_called()
        self.assertNotIn('readiness_receipt', result)


class TrustedIngressTests(unittest.TestCase):
    def test_candidate_policy_commit_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common']), patch.object(gate.subprocess, 'run', side_effect=[None, subprocess.CalledProcessError(1, ['git', 'merge-base'])]) as execute:
                with self.assertRaises(subprocess.CalledProcessError):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertEqual(execute.call_count, 2)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))


class IntegrationAuthorityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        directory = self.root / 'scripts/pm'; directory.mkdir(parents=True)
        for name in ('ci-ready-receipt.py','ci_ready_receipt_identity.py','integration_ci.py','integration_executor_contract.py'):
            shutil.copy2(Path(__file__).with_name(name),directory/name)
        def git(*args): return subprocess.check_output(['git','-C',str(self.root),*args],text=True).strip()
        git('init','-q'); git('config','user.email','fixture@example.invalid'); git('config','user.name','Fixture')
        git('add','.'); git('commit','-qm','trusted')
        self.commit = git('rev-parse','HEAD')
        self.uid = 'task_'+'a'*32
        self.data = {'repository':'owner/repo','number':12,'baseRefName':'main','baseRefOid':'a'*40,'headRefOid':'b'*40,
                     'policy_discovery':{'status':'resolved','required_status_checks':[{'context':'required-gate','app_id':None},{'context':'required-gate','app_id':42}]}}
        self.context = {'status':'passed','tool_root':str(self.root),'policy_commit':self.commit,'task':{'repository':'owner/repo','issue_number':1}}
        spec = importlib.util.spec_from_file_location('fixture_ci',directory/'ci-ready-receipt.py')
        import sys
        sys.path.insert(0,str(directory))
        try:
            module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        finally: sys.path.pop(0)
        self.plan = dict(scope='test',selected_capabilities='',reason_summary='fixture',changed_path_count=0,planner_config_sha256='sha256:'+'c'*64)
        self.plan.update({name:'false' for name in module.RUN_FIELDS})
        self.state = self.root/'github.json'
        binary=self.root/'bin'; binary.mkdir()
        gh=binary/'gh'
        gh.write_text('''#!/usr/bin/env python3
import io,json,os,sys,zipfile
s=json.load(open(os.environ['CI_FIXTURE'])); path=sys.argv[2]
if '/workflows/rust.yml/runs?' in path: result={'workflow_runs':[]}
elif '/pulls/' in path: result=s['pr']
elif path == 'repos/owner/repo': result={'full_name':'owner/repo','default_branch':'main'}
elif path == 'repos/owner/repo/git/ref/heads/main': result={'object':{'sha':'c'*40}}
elif '/check-runs?' in path: result={'check_runs':[s['run']]}
elif '/artifacts?' in path: result={'artifacts':[{'id':3,'name':'oasis7-required-plan-v1','expired':False,'workflow_run':{'id':8}}]}
elif path.endswith('/artifacts/3/zip'):
 data=io.BytesIO()
 with zipfile.ZipFile(data,'w') as z: z.writestr('oasis7-required-plan-v1.json',json.dumps(s['artifact']))
 sys.stdout.buffer.write(data.getvalue()); raise SystemExit(0)
else: raise SystemExit('unexpected '+path)
print(json.dumps(result))
'''); gh.chmod(0o755)
        self.env = dict(os.environ,PATH=str(binary)+os.pathsep+os.environ['PATH'],CI_FIXTURE=str(self.state))

    def check(self, run_base='a', artifact_base='a', integration_run_id=None, require_strict=None):
        pr={'draft':False,'state':'open','merged':False,'body':f'Task: {self.uid}\nRefs #1','head':{'sha':'b'*40},'base':{'ref':'main','sha':'a'*40}}
        run={'id':9,'name':'required-gate','app':{'id':42},'head_sha':'b'*40,'status':'completed','conclusion':'success','completed_at':'2026-01-01','details_url':'https://github.com/owner/repo/actions/runs/8','pull_requests':[{'number':12,'head':{'sha':'b'*40},'base':{'ref':'main','sha':run_base*40}}]}
        artifact={'schema':'oasis7-required-plan-v1','repository':'owner/repo','workflow_run_id':8,'head_oid':'b'*40,'base_oid':artifact_base*40,'check_name':'required-gate','planner':self.plan}
        self.state.write_text(json.dumps({'pr':pr,'run':run,'artifact':artifact}))
        with patch.dict(os.environ,self.env):
            return gate.live_integration_admission(
                self.data, self.root, self.uid, self.root, self.context,
                integration_run_id, require_strict=require_strict,
            )

    def test_explicit_locator_is_forwarded_and_never_bypasses_live_discovery(self):
        with self.assertRaisesRegex(ValueError,"locator absent"):
            self.check(integration_run_id=8)

    def test_old_green_integration_run_blocks(self):
        with self.assertRaisesRegex(ValueError,'stale integration base'): self.check(run_base='c',artifact_base='c')

    def test_mutable_check_metadata_cannot_replace_frozen_artifact(self):
        with self.assertRaisesRegex(ValueError,'artifact identity mismatch: base_oid'): self.check(artifact_base='c')

    def test_fresh_run_and_artifact_allow_unchanged_source(self):
        proof=self.check()
        self.assertEqual(proof['head_oid'],'b'*40)
        self.assertEqual(proof['integration_base_oid'],'a'*40)
        self.assertEqual(proof['check_run_id'],9)
        self.assertEqual(proof['assessed_target_oid'],'c'*40)

    def test_keyed_context_binds_task_epoch_source_and_check(self):
        task = {'bootstrap_epoch': 3, 'loop_binding': {'bootstrap_epoch': 3}}
        data = {**self.data, 'number': 12, 'repository': 'owner/repo', 'headRefOid': 'b' * 40}
        effective_identity = {'schema': 'oasis7-ci-effective-policy-identity/v1', 'digest': 'sha256:' + '1' * 64}
        planner_authority = {'planner_authority_oid': '2' * 40}
        request_identity = {
            'repository': 'owner/repo', 'task_uid': self.uid, 'pr_number': 12,
            'bootstrap_epoch': 3, 'source_head_oid': 'b' * 40,
            'source_projection_digest': 'sha256:' + 'e' * 64,
            'effective_policy_digest': effective_identity['digest'],
        }
        plan = {
            'schema': 'oasis7-required-plan-v2',
            'request_key': 'sha256:' + 'd' * 64,
            'request_identity': request_identity,
            'repository': 'owner/repo', 'task_uid': self.uid, 'pr_number': 12,
            'bootstrap_epoch': 3, 'source_head_oid': 'b' * 40,
            'source_scope_oid': 'f' * 40,
            'effective_policy_identity': effective_identity,
            'planner_inventory_authority': planner_authority,
            'check_name': 'required-gate', 'check_app_id': 42, 'check_run_id': 9,
            'workflow_run_id': 10, 'run_attempt': 1,
            'planner_output': {
                'source_scope_base': 'f' * 40,
                'impact_projection_digest': request_identity['source_projection_digest'],
            },
        }
        proof = {
            'request_key': 'sha256:' + 'd' * 64,
            'request_identity': request_identity,
            'assessed_target_oid': 'c' * 40,
            'source_scope_oid': 'f' * 40,
            'required_plan_v2_payload': plan,
            'check_name': 'required-gate', 'check_app_id': 42, 'check_run_id': 9,
            'effective_policy_identity': effective_identity,
            'planner_inventory_authority': planner_authority,
            'workflow_run_id': 10, 'run_id': 10, 'request_id': 10, 'run_attempt': 1,
            'trusted_policy_context': {
                'effective_policy_identity': effective_identity,
                'planner_inventory_authority': planner_authority,
            },
        }
        self.assertTrue(gate.validate_keyed_integration_identity(proof, data, self.uid, task))
        for label, changed_proof, changed_task in (
            ('epoch', proof, {**task, 'bootstrap_epoch': 4}),
            ('head', {**proof, 'request_identity': {**proof['request_identity'], 'source_head_oid': '3' * 40}}, task),
            ('scope', {**proof, 'required_plan_v2_payload': {**proof['required_plan_v2_payload'], 'source_scope_oid': '4' * 40}}, task),
            ('app', {**proof, 'check_app_id': 43}, task),
            ('policy', {**proof, 'effective_policy_identity': {'digest': 'sha256:' + '3' * 64}}, task),
            ('target', {**proof, 'assessed_target_oid': 'invalid'}, task),
        ):
            with self.subTest(label=label), self.assertRaises(ValueError):
                gate.validate_keyed_integration_identity(changed_proof, data, self.uid, changed_task)
        bool_epoch_identity = {**proof['request_identity'], 'bootstrap_epoch': 1}
        bool_epoch_plan = {
            **proof['required_plan_v2_payload'],
            'bootstrap_epoch': True,
            'request_identity': bool_epoch_identity,
        }
        bool_epoch_proof = {
            **proof,
            'request_identity': bool_epoch_identity,
            'required_plan_v2_payload': bool_epoch_plan,
        }
        with self.assertRaisesRegex(ValueError, 'required plan bootstrap_epoch'):
            gate.validate_keyed_integration_identity(
                bool_epoch_proof, data, self.uid,
                {'bootstrap_epoch': 1, 'loop_binding': {'bootstrap_epoch': 1}},
            )

    def test_unkeyed_legacy_context_keeps_legacy_lane(self):
        self.assertFalse(gate.validate_keyed_integration_identity(
            {'ci_validation_mode': 'trusted_integration'}, self.data, self.uid, {'status': 'legacy'},
        ))
        for invalid in (True, '3', 0, -1):
            with self.subTest(invalid_epoch=invalid), self.assertRaises(ValueError):
                gate._positive_bootstrap_epoch(invalid)

    def test_ordinary_mode_reuses_source_bound_ci_after_unrelated_target_advance(self):
        proof = self.check(run_base='c', artifact_base='c', require_strict=False)
        self.assertEqual(proof['ci_validation_mode'], 'ordinary_pr')
        self.assertEqual(proof['integration_base_oid'], 'c' * 40)
        self.assertEqual(proof['head_oid'], 'b' * 40)

    def test_missing_or_ambiguous_app_pin_blocks(self):
        for pins in ([None],[42,43]):
            self.data['policy_discovery']['required_status_checks']=[{'context':'required-gate','app_id':p} for p in pins]
            with self.assertRaisesRegex(ValueError,'unambiguous app pin'): self.check()

    def test_tampered_ci_helper_cannot_execute(self):
        sentinel=self.root/'executed'
        (self.root/'scripts/pm/ci-ready-receipt.py').write_text('from pathlib import Path\nPath('+repr(str(sentinel))+').touch()')
        with self.assertRaisesRegex(ValueError,'helper bytes differ'): self.check()
        self.assertFalse(sentinel.exists())

    def test_tampered_helper_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            helper = root / 'scripts/pm/loop-local-gate.py'
            helper.parent.mkdir(parents=True)
            helper.write_text('candidate code')
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common', b'trusted code']), patch.object(gate.subprocess, 'run') as execute:
                with self.assertRaisesRegex(ValueError, 'bytes differ'):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))

    def test_canonical_head_drift_cannot_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.pm/github-project-sync').mkdir(parents=True)
            helper = root / 'scripts/pm/loop-local-gate.py'
            helper.parent.mkdir(parents=True)
            helper.write_text('trusted code')
            binding = {'policy_commit': 'c' * 40}
            task = {'repository': 'owner/repo', 'issue_number': 3, 'loop_binding': binding}
            (root / '.pm/github-project-sync/tasks.json').write_text(json.dumps({'tasks': {'uid': task}}))
            encoded = base64.urlsafe_b64encode(json.dumps(binding).encode()).decode()
            issue = json.dumps({'body': f'- loop_binding_b64: `{encoded}`'})
            with patch.object(gate.subprocess, 'check_output', side_effect=[issue, 'c' * 40, '/common', '/common', b'trusted code', 'd' * 40]), patch.object(gate.subprocess, 'run') as execute:
                with self.assertRaisesRegex(ValueError, 'worktree HEAD'):
                    gate.local_loop_admission(root, 'uid', 'a' * 40, 'b' * 40, root)
                self.assertTrue(all(call.args[0][0] == 'git' for call in execute.call_args_list))


class KeyedQApplicabilityTests(unittest.TestCase):
    """Adversarial lifecycle-boundary checks for Q1 keyed-evidence consumption."""

    def evaluator_inputs(self):
        fixtures = _APPLICABILITY_FIXTURES
        plan = fixtures.source_plan()
        plan.update({
            'workflow_run_id': 10,
            'run_attempt': 1,
            'check_app_id': 42,
            'check_run_id': 20,
        })
        target = fixtures.target_snapshot()
        results = []
        for item in fixtures.evidence_set()['tests']:
            results.append({
                'artifact_id': item['artifact_id'],
                'name': f"oasis7-required-result-v2-{item['unit_id']}",
                'payload': {
                    'unit_id': item['unit_id'],
                    'obligation_ids': item.get('obligation_ids'),
                    'status': item['status'],
                    'input_digest': item['input_digest'],
                    'planner_inventory_digest': item['inventory_digest'],
                    'effective_policy_identity': item['effective_policy_identity'],
                    'repository': item['repository'],
                    'task_uid': item['task_uid'],
                    'pr_number': item['pr_number'],
                    'source_head_oid': item['source_head_oid'],
                    'source_scope_oid': item['source_scope_oid'],
                    'workflow_run_id': item['run_id'],
                    'run_attempt': item['run_attempt'],
                    'check_app_id': item['check_app_id'],
                    'check_run_id': item['check_run_id'],
                },
            })
        source_proof = {
            'request_key': 'sha256:' + '8' * 64,
            'request_identity': {
                'repository': fixtures.REPOSITORY,
                'task_uid': fixtures.UID,
                'pr_number': 7,
                'source_head_oid': fixtures.HEAD,
                'source_scope_oid': fixtures.SOURCE_SCOPE,
            },
            'required_plan_v2_payload': plan,
            'trusted_planner_inventory': fixtures.trusted_inventory_readback(
                plan['planner_inventory_issuer'], artifact_id=30,
            ),
            'integration_base_oid': plan['integration_base_oid'],
            'source_scope_oid': plan['source_scope_oid'],
            'assessed_target_oid': target['target_oid'],
            'workflow_run_id': 10,
            'run_attempt': 1,
            'check_app_id': 42,
            'check_run_id': 20,
            'required_result_v2_artifacts': results,
        }
        target_inventory = {
            'schema': gate.LOCAL_TARGET_INVENTORY_SCHEMA,
            'repository': target['repository'],
            'task_uid': target['task_uid'],
            'pr_number': target['pr_number'],
            'source_head_oid': target['source_head_oid'],
            'source_scope_oid': target['source_scope_oid'],
            'assessed_target_oid': target['target_oid'],
            'input_scope_commit_oid': target['input_scope_commit_oid'],
            'input_scope_tree_oid': target['input_scope_tree_oid'],
            'planner_authority_oid': target['target_oid'],
            'planner_config_sha256': 'sha256:' + 'f' * 64,
            'effective_policy_identity': target['input_scope']['target_observation']['effective_policy_identity'],
            'inventory_digest': target['input_scope']['target_observation']['inventory_digest'],
            'required_test_units': target['required_test_units'],
            'unit_specs': target['unit_specs'],
            'product_corpus': target['product_corpus'],
            'closure_status': 'complete',
            'target_observation': target['input_scope']['target_observation'],
            'input_scope': target['input_scope'],
            'effective_policy': fixtures.enabled_policy(),
        }
        live_pr = {
            'repository': fixtures.REPOSITORY,
            'number': 7,
            'headRefOid': fixtures.HEAD,
            'baseRefOid': target['target_oid'],
        }
        module_name = 'ci_evidence_applicability_under_test'
        applicability = sys.modules.get(module_name)
        if applicability is None:
            spec = importlib.util.spec_from_file_location(
                module_name, Path(__file__).with_name('ci_evidence_applicability.py'),
            )
            applicability = importlib.util.module_from_spec(spec)
            sys.modules[module_name] = applicability
            try:
                spec.loader.exec_module(applicability)
            except Exception:
                sys.modules.pop(module_name, None)
                raise
        return source_proof, target_inventory, applicability, live_pr

    def test_keyed_q_rejects_wrong_source_B_current_Q_and_W_authority(self):
        cases = (
            ('B', lambda proof, inventory, live: proof.update(integration_base_oid='d' * 40), 'immutable B'),
            ('Q', lambda proof, inventory, live: inventory.update(assessed_target_oid='d' * 40), 'immutable B or live PR Q'),
            ('W', lambda proof, inventory, live: inventory.update(planner_authority_oid='d' * 40), 'W authority'),
        )
        for label, mutate, message in cases:
            with self.subTest(label=label):
                proof, inventory, applicability, live = self.evaluator_inputs()
                mutate(proof, inventory, live)
                with self.assertRaisesRegex(ValueError, message):
                    gate.evaluate_keyed_q_applicability(proof, inventory, applicability, live)

    def test_keyed_q_requires_result_artifacts_and_cannot_promote_plan_to_success(self):
        proof, inventory, applicability, live = self.evaluator_inputs()
        proof.pop('required_result_v2_artifacts')
        with self.assertRaisesRegex(ValueError, 'source result artifact set is missing'):
            gate.evaluate_keyed_q_applicability(proof, inventory, applicability, live)

        proof, inventory, applicability, live = self.evaluator_inputs()
        proof['required_result_v2_artifacts'] = []
        with self.assertRaisesRegex(ValueError, 'requires revalidation or is blocked'):
            gate.evaluate_keyed_q_applicability(proof, inventory, applicability, live)

    def test_keyed_q_rejects_result_artifacts_from_wrong_R_A_or_check(self):
        for field, value in (
            ('workflow_run_id', 9),
            ('run_attempt', 0),
            ('check_app_id', 43),
            ('check_run_id', 19),
        ):
            with self.subTest(field=field):
                proof, inventory, applicability, live = self.evaluator_inputs()
                proof['required_result_v2_artifacts'][0]['payload'][field] = value
                with self.assertRaisesRegex(ValueError, 'differs from the exact R/A/check'):
                    gate.evaluate_keyed_q_applicability(
                        proof, inventory, applicability, live,
                    )

    def test_keyed_q_unknown_closure_and_post_assessment_target_drift_block(self):
        proof, inventory, applicability, live = self.evaluator_inputs()
        inventory['closure_status'] = 'unknown'
        with self.assertRaisesRegex(ValueError, 'closure is unknown or partial'):
            gate.evaluate_keyed_q_applicability(proof, inventory, applicability, live)

        proof, inventory, applicability, live = self.evaluator_inputs()
        assessment = gate.evaluate_keyed_q_applicability(
            proof, inventory, applicability, live,
        )
        self.assertEqual(assessment['test_evidence'], 'reusable')
        self.assertNotEqual(assessment['integration_base_oid'], assessment['assessed_target_oid'])
        proof['keyed_q_applicability'] = assessment
        for field, value in (
            ('workflow_run_id', 11),
            ('run_attempt', 2),
            ('check_run_id', 21),
        ):
            with self.subTest(field=field):
                stale = {**proof, field: value}
                with self.assertRaisesRegex(ValueError, 'latest run/check identity mismatch'):
                    gate._validate_keyed_q_applicability(stale, live)
        drifted_live = {**live, 'baseRefOid': 'd' * 40}
        with self.assertRaisesRegex(ValueError, 'target Q identity drift'):
            gate._validate_keyed_q_applicability(proof, drifted_live)

    def test_unkeyed_v1_proof_keeps_legacy_readiness_path(self):
        uid = 'task_' + '1' * 32
        data = {
            'number': 12, 'repository': 'owner/repo', 'state': 'OPEN', 'isDraft': False,
            'body': f'Task: {uid}\nRefs #1', 'baseRefName': 'main', 'headRefName': 'codex/task',
            'baseRefOid': 'a' * 40, 'headRefOid': 'b' * 40,
            'mergeable': 'MERGEABLE', 'mergeStateStatus': 'CLEAN', 'reviewDecision': 'APPROVED',
            'merge_hold': {'kind': 'normal_pr_ci_watch', 'active': False},
            'policy_discovery': {'status': 'resolved', 'required_status_checks': [
                {'context': 'required-gate', 'app_id': 42},
            ]},
            'statusCheckRollup': [{'name': 'required-gate', 'app_id': 42,
                                   'status': 'COMPLETED', 'conclusion': 'SUCCESS'}],
            'comments': [], 'reviews': [], 'threads': [],
        }
        v1_proof = {'ci_validation_mode': 'trusted_integration', 'head_oid': 'b' * 40,
                    'integration_base_oid': 'a' * 40, 'check_name': 'required-gate'}
        admission = {'status': 'passed', 'task': {'repository': 'owner/repo', 'issue_number': 1},
                     'tool_root': '/trusted', 'policy_commit': 'c' * 40}
        with patch.object(gate, 'local_loop_admission', return_value=admission), \
             patch.object(gate, 'live_integration_admission', return_value=v1_proof) as live_check, \
             patch.object(gate, 'read_pr_identity', return_value={
                 key: data[key] for key in (
                     'number', 'state', 'isDraft', 'body', 'baseRefName', 'headRefName',
                     'baseRefOid', 'headRefOid',
                 )
             }), patch.object(gate, '_validate_keyed_q_applicability') as keyed_validator:
            result = gate.production_decision(data, False, Path('/canonical'), uid, None)

        self.assertTrue(result['ready_for_merge'], result)
        self.assertEqual(result['readiness_receipt']['integration_ci'], v1_proof)
        self.assertNotIn('keyed_target_applicability_epoch', result['readiness_receipt']['integration_ci'])
        live_check.assert_called_once()
        keyed_validator.assert_not_called()


if __name__ == '__main__': unittest.main()
