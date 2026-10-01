#!/usr/bin/env python3
"""Isolated provider readbacks; these fixtures are not hosted acceptance."""
import copy
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('codeql_advisory', Path(__file__).with_name('codeql_advisory.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class AdvisoryTests(unittest.TestCase):
    def setUp(self):
        self.data = {'repository': 'owner/repo', 'number': 7, 'headRefOid': 'a'*40,
                     'headRefName': 'codex/task', 'mergeStateStatus': 'UNSTABLE',
                     'policy_discovery': {'status': 'resolved', 'required_status_checks': [], 'active_rule_types': []},
                     'statusCheckRollup': [{'databaseId': 10, 'name': 'CodeQL / plan', 'conclusion': 'FAILURE',
                                            'checkSuite': {'app': {'databaseId': 9}}}]}
        self.calls = []
        self.responses = {
            'apps/github-actions': {'id': 9, 'slug': 'github-actions', 'owner': {'login': 'github', 'type': 'Organization'}},
            'repos/owner/repo/check-runs/10': {'id': 10, 'name': 'CodeQL / plan', 'head_sha': 'a'*40,
                                             'conclusion': 'failure', 'app': {'id': 9}, 'check_suite': {'id': 20}},
            'repos/owner/repo/actions/workflows/codeql.yml': {'id': 30, 'path': '.github/workflows/codeql.yml', 'state': 'active'},
            'repos/owner/repo/actions/workflows/30/runs?head_sha='+'a'*40+'&event=pull_request&per_page=100': {
                'total_count': 1, 'workflow_runs': [{'id': 40, 'workflow_id': 30, 'path': '.github/workflows/codeql.yml',
                    'head_sha': 'a'*40, 'head_branch': 'codex/task', 'event': 'pull_request', 'check_suite_id': 20,
                    'run_attempt': 1, 'repository': {'full_name': 'owner/repo'}, 'pull_requests': [{'number': 7}]}]},
            'repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100': {
                'total_count': 1, 'jobs': [{'id': 50, 'name': 'CodeQL / plan', 'run_id': 40, 'run_attempt': 1,
                    'head_sha': 'a'*40, 'conclusion': 'failure', 'check_run_url': 'https://api.github.com/repos/owner/repo/check-runs/10'}]},
        }

    def read(self, endpoint):
        self.calls.append(endpoint)
        return copy.deepcopy(self.responses[endpoint])

    def test_CQ_T24_exact_actions_failure(self):
        proof = module.explain_unstable(self.data, self.read)
        self.assertTrue(proof['explained'], proof)
        self.assertEqual(proof['checks'][0]['run_attempt'], 1)
        self.assertEqual(self.data['mergeStateStatus'], 'UNSTABLE')

    def test_CQ_T23_T38_healthy_pending_needs_no_readback(self):
        self.data['mergeStateStatus'] = 'CLEAN'
        self.data['statusCheckRollup'][0]['conclusion'] = None
        self.assertFalse(module.explain_unstable(self.data, self.read)['explained'])
        self.assertEqual(self.calls, [])

    def test_CQ_T25_T26_T30_fail_closed(self):
        mutations = [
            lambda: self.data['statusCheckRollup'].append({'context': 'unknown', 'state': 'FAILURE'}),
            lambda: self.responses['apps/github-actions'].update(id=999),
            lambda: self.responses['repos/owner/repo/check-runs/10'].update(head_sha='b'*40),
            lambda: self.responses['repos/owner/repo/actions/workflows/codeql.yml'].update(path='.github/workflows/fake.yml'),
            lambda: self.responses['repos/owner/repo/actions/workflows/30/runs?head_sha='+'a'*40+'&event=pull_request&per_page=100']['workflow_runs'][0].update(head_branch='other'),
            lambda: self.responses['repos/owner/repo/actions/workflows/30/runs?head_sha='+'a'*40+'&event=pull_request&per_page=100']['workflow_runs'][0].update(head_sha='b'*40),
            lambda: self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['jobs'][0].update(run_attempt=2),
            lambda: self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['jobs'][0].update(check_run_url='https://api.github.com/repos/owner/repo/check-runs/999'),
            lambda: self.responses['repos/owner/repo/check-runs/10'].update(conclusion='success'),
            lambda: self.data['statusCheckRollup'].append(copy.deepcopy(self.data['statusCheckRollup'][0])),
            lambda: self.data['statusCheckRollup'].append({**self.data['statusCheckRollup'][0], 'databaseId': 11, 'checkSuite': {'app': {'databaseId': 99}}}),
            lambda: self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100'].update(total_count=101),
            lambda: self.responses.clear(),
        ]
        for mutate in mutations:
            self.setUp()
            mutate()
            with self.subTest(mutation=mutate):
                self.assertFalse(module.explain_unstable(self.data, self.read)['explained'])

    def test_CQ_T27_T28_required_or_scanning_rule(self):
        for policy in ({'required_status_checks': [{'context': 'CodeQL / plan', 'app_id': 9}]},
                       {'active_rule_types': ['code_scanning']}, {'status': 'capability_blocked'}):
            self.setUp()
            self.data['policy_discovery'].update(policy)
            self.assertFalse(module.explain_unstable(self.data, self.read)['explained'])
            self.assertEqual(self.calls, [])

    def test_CQ_T27_required_failure_missing_pending_or_ambiguous(self):
        for checks in ([], [{'name': 'required-gate', 'conclusion': 'FAILURE'}],
                       [{'name': 'required-gate', 'status': 'IN_PROGRESS'}],
                       [{'name': 'required-gate', 'conclusion': 'SUCCESS'}]*2):
            self.setUp()
            self.data['policy_discovery']['required_status_checks'] = [{'context': 'required-gate', 'app_id': None}]
            self.data['statusCheckRollup'].extend(checks)
            self.assertFalse(module.explain_unstable(self.data, self.read)['explained'])
            self.assertEqual(self.calls, [])

    def test_native_provider_claims_cannot_define_production_trust(self):
        self.data['statusCheckRollup'][0].update(
            name='Code scanning results / CodeQL',
            details_url='https://github.com/owner/repo/security/code-scanning',
            analysis={'commit_sha': 'a'*40, 'ref': 'refs/pull/7/merge',
                      'tool': {'name': 'CodeQL'}, 'category': 'oasis7/python-repo/default',
                      'analysis_key': '.github/workflows/codeql.yml:analyze'})
        proof = module.explain_unstable(self.data, self.read)
        self.assertFalse(proof['explained'])
        self.assertIn('hosted association required', proof['reason'])
        self.assertFalse(any('code-scanning/analyses' in endpoint for endpoint in self.calls))

    def test_shared_same_run_provenance_is_read_once(self):
        self.data['statusCheckRollup'].append({'databaseId': 11, 'name': 'CodeQL / summary', 'conclusion': 'FAILURE',
                                             'checkSuite': {'app': {'databaseId': 9}}})
        self.responses['repos/owner/repo/check-runs/11'] = {
            **self.responses['repos/owner/repo/check-runs/10'], 'id': 11, 'name': 'CodeQL / summary'}
        self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['jobs'].append({
            **self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['jobs'][0],
            'id': 51, 'name': 'CodeQL / summary', 'check_run_url': 'https://api.github.com/repos/owner/repo/check-runs/11'})
        self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['total_count'] = 2
        self.assertTrue(module.explain_unstable(self.data, self.read)['explained'])
        self.assertEqual(len(self.calls), len(set(self.calls)))

    def test_CQ_T29_T31_lifecycle_protections_and_raw_state(self):
        spec = importlib.util.spec_from_file_location('gate', Path(__file__).with_name('pr-lifecycle-gate.py'))
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        self.data.update(state='OPEN', mergeable='MERGEABLE', reviewDecision='APPROVED',
                         merge_hold={'kind': 'normal_pr_ci_watch', 'active': False})
        ready = gate.decision(self.data, False, evidence_mode='fixture', advisory_reader=self.read)
        self.assertTrue(ready['ready_for_merge'], ready)
        self.assertEqual(ready['mergeStateStatus'], 'UNSTABLE')
        self.assertFalse(ready['use_admin_merge'])
        self.assertNotIn('readiness_receipt', ready)
        for changes in ({'mergeStateStatus': 'DIRTY'}, {'mergeStateStatus': 'UNKNOWN'},
                        {'mergeStateStatus': 'BLOCKED'}, {'reviewDecision': 'REVIEW_REQUIRED'},
                        {'reviewDecision': 'CHANGES_REQUESTED'}, {'threads': [{'isResolved': False}]},
                        {'merge_hold': {'kind': 'user_requested_merge_hold', 'active': True,
                                        'requester': 'human', 'reason': 'pause', 'resume_authority': 'human'}}):
            with self.subTest(changes=changes):
                result = gate.decision({**self.data, **changes}, True, evidence_mode='fixture', advisory_reader=self.read)
                self.assertFalse(result['ready_for_merge'], result)
                self.assertFalse(result['use_admin_merge'])

    def test_CQ_T38_production_cache_and_business_digest_separation(self):
        spec = importlib.util.spec_from_file_location('gate', Path(__file__).with_name('pr-lifecycle-gate.py'))
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        self.data.update(state='OPEN', isDraft=False, body='', baseRefOid='b'*40, baseRefName='main',
                         mergeable='MERGEABLE', reviewDecision='APPROVED',
                         merge_hold={'kind': 'normal_pr_ci_watch', 'active': False})
        def api(command):
            self.assertEqual(command[:2], ['gh', 'api'])
            return self.read(command[2])
        with patch.object(gate, '_run_json', side_effect=api), \
                patch.object(gate, 'local_loop_admission', return_value={'status': 'passed'}), \
                patch.object(gate, 'live_integration_admission', return_value=None), \
                patch.object(gate, 'read_pr_identity', side_effect=lambda *_: copy.deepcopy(self.data)):
            result = gate.production_decision(self.data, False, Path('/canonical'), 'task_fixture', None)
            self.assertTrue(result['ready_for_merge'], result)
            self.assertEqual(len(self.calls), len(set(self.calls)))
            before = result['readiness_receipt']['gate_epoch']
            # Different scan job instances must not alter business readiness.
            self.responses['repos/owner/repo/actions/runs/40/attempts/1/jobs?per_page=100']['jobs'][0]['id'] = 51
            result = gate.production_decision(self.data, False, Path('/canonical'), 'task_fixture', None)
            self.assertEqual(before, result['readiness_receipt']['gate_epoch'])
            self.calls.clear()
            self.data['mergeStateStatus'] = 'CLEAN'
            self.data['statusCheckRollup'][0].update(conclusion=None, status='IN_PROGRESS')
            self.assertTrue(gate.production_decision(self.data, False, Path('/canonical'), 'task_fixture', None)['ready_for_merge'])
            self.assertEqual(self.calls, [])


if __name__ == '__main__':
    unittest.main()
