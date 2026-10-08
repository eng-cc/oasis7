#!/usr/bin/env python3
"""Execution-boundary witnesses and protected exception producer refusals."""
import copy
import base64
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import sys
import unittest
from pathlib import Path
from unittest.mock import patch
from unittest.mock import Mock
from types import SimpleNamespace

import ci_ready_receipt_identity as identity
facts = identity._strict_exception_module()
ROOT = Path(__file__).resolve().parents[2]

def comment(ident, body, actor='coordinator'):
    return {'id': ident, 'body': body, 'user': {'login': actor, 'type': 'User'},
            'issue_url': 'https://api.github.com/repos/eng-cc/oasis7/issues/12',
            'html_url': f'https://github.com/eng-cc/oasis7/issues/12#issuecomment-{ident}',
            'created_at': '2026-10-08T12:00:00Z', 'updated_at': '2026-10-08T12:00:00Z'}

def fixture():
    uid, h, q = 'task_' + 'a' * 32, 'b' * 40, 'c' * 40
    workflow = subprocess.check_output(['git', '-C', str(ROOT), 'show',
        '94dda07f00ee0ffe9171d3b9fcd25df45680e262:' + facts.WORKFLOW])
    # Candidate adds a gate to the ordinary protected driver's execution. Its
    # underlying driver remains protected; workflow control is the residual.
    source = workflow.replace(b'elif [[ "${GITHUB_EVENT_NAME}" == "pull_request" && -f "${RUNNER_TEMP}/impact-projection.json" ]]; then',
        b'elif [[ "${GITHUB_EVENT_NAME}" == "pull_request" && -f "${RUNNER_TEMP}/impact-projection.json" && "${CANDIDATE_GATE}" == "true" ]]; then')
    common = {'repository': 'eng-cc/oasis7', 'task_uid': uid, 'issue_number': 12,
              'pr_number': 7, 'source_head_oid': h, 'target_oid': q}
    record = {**common, 'exception_rule': 'trusted_executor_isolation',
        'constraint': 'candidate_required_gate_orchestration',
        'obligation': 'independent_workflow_orchestration', 'ordinary_supplements': facts.SUPPLEMENTS,
        'required_check_scope': copy.deepcopy(facts.SCOPE), 'review_comment_ids': [2, 3]}
    reviews, bindings = [], {}
    for ident, role in ((2, 'repository_health_engineer'), (3, 'qa_engineer')):
        review = {**common, 'role': role, 'conclusion': 'independent_orchestration_required',
            'constraint_digest': facts.constraint_digest(record),
            'obligation': record['obligation'], 'ordinary_supplements': facts.SUPPLEMENTS,
            'required_check_scope': copy.deepcopy(facts.SCOPE),
            'review_plan_path': 'plan.json', 'review_plan_sha256': '1' * 64,
            'role_return_path': role + '.json', 'role_return_sha256': str(ident) * 64,
            'packet_path': role + '-packet.json', 'packet_digest': '4' * 64,
            'bootstrap_snapshot_path': 'snapshot.json'}
        reviews.append((comment(ident, facts.REVIEW_MARKER + '\n' + json.dumps(review)),
                        {'user': {'login': 'coordinator'}, 'permission': 'admin'}))
        bindings[ident] = {'role': role, 'head': h, 'task_uid': uid, 'packet_digest': '4' * 64,
                          'review_plan_sha256': '1' * 64, 'role_return_sha256': str(ident) * 64}
    f = {'record_comment': comment(1, facts.MARKER + '\n' + json.dumps(record)),
        'record_permission': {'user': {'login': 'coordinator'}, 'permission': 'admin'},
        'reviews': reviews, 'role_bindings': bindings,
        'maintenance_scope': {'subject_head_oid': h, 'allowed_write_paths': [facts.WORKFLOW]},
        'repository': common['repository'], 'task_uid': uid, 'issue': 12,
        'pr': {'number': 7, 'head': {'sha': h}, 'user': {'login': 'candidate-author'}},
        'target_oid': q, 'target_workflow': workflow, 'source_workflow': source}
    for binding in bindings.values():
        binding['projection_digest'] = projection(f)['projection_digest']
    return f

def projection(f):
    value = {'schema': identity.PROJECTION_SCHEMA, 'task_uid': f['task_uid'],
        'ci_scope': 'full', 'test_profile': 'full',
        'source_head_oid': f['pr']['head']['sha'], 'scope_base_oid': 'd' * 40,
        'changed_paths': [facts.WORKFLOW]}
    digest = lambda v: 'sha256:' + hashlib.sha256(json.dumps(v, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    value['changed_paths_digest'] = digest(value['changed_paths'])
    value['projection_digest'] = digest(value)
    return value

class ProducerTests(unittest.TestCase):
    def test_real_boundary_reaches_shared_required_with_all_facts(self):
        f = fixture()
        verified = facts.verify(**f)
        route = identity.evaluate_strict_integration_requirement(
            trusted_projection=projection(f), expected_task_uid=f['task_uid'], verified_exception=verified)
        self.assertEqual(route['status'], 'required')
        self.assertEqual(route['exception_rule'], 'trusted_executor_isolation')
        self.assertIn('candidate-controlled', route['ordinary_limitation'])
        self.assertEqual(route['constraint_evidence']['protected_workflow_sha256'], facts.EXECUTOR_SHA256)
        self.assertEqual(route['required_check_scope'], facts.SCOPE)
        self.assertEqual(route['strict_capability']['execution'], 'protected_Q_workflow_driver_on_merge_Q_H')

    def test_paths_comments_and_helper_edits_are_not_execution_constraints(self):
        for replacement in ('unchanged', 'comment', 'echo'):
            with self.subTest(replacement=replacement):
                f = fixture()
                f['source_workflow'] = f['target_workflow']
                if replacement != 'unchanged':
                    f['source_workflow'] = f['source_workflow'].replace(b'      - name: Run required test tier\n',
                        b'      - name: Run required test tier\n' +
                        (b'        # candidate documentation\n' if replacement == 'comment' else b'        description: candidate echo change\n'))
                with self.assertRaises(ValueError):
                    facts.verify(**f)

    def test_whitespace_guard_change_does_not_create_exception(self):
        f = fixture()
        f['source_workflow'] = f['target_workflow'].replace(
            b'impact-projection.json" ]]; then', b'impact-projection.json"   ]]; then')
        with self.assertRaisesRegex(ValueError, 'did not change'):
            facts.verify(**f)

    def test_verified_facts_cannot_be_mutated_through_nested_values(self):
        value = facts.verify(**fixture())
        with self.assertRaises(TypeError): value.facts['constraint_evidence']['comment_id'] = 99
        with self.assertRaises(AttributeError): value.facts['required_check_scope']['checks'].append('new')
        copy = value.as_dict()
        copy['constraint_evidence']['review_comment_ids'].append(99)
        self.assertEqual([2, 3], value.as_dict()['constraint_evidence']['review_comment_ids'])

    def test_missing_ordinary_driver_is_coverage_gap(self):
        f = fixture()
        f['source_workflow'] = f['source_workflow'].replace(b'${protected_driver}/scripts/ci-tests.sh', b'candidate.sh')
        with self.assertRaisesRegex(ValueError, 'protected ordinary driver'):
            facts.verify(**f)

    def test_unreviewed_strings_wrong_head_scope_or_edited_authority_refused(self):
        for change in ('no_bindings', 'no_reviews', 'edited', 'permission', 'head', 'unsupported_Q'):
            with self.subTest(change=change):
                f = fixture()
                if change == 'no_bindings': f['role_bindings'] = {}
                if change == 'no_reviews': f['reviews'] = []
                if change == 'edited': f['record_comment']['updated_at'] = '2026-10-09T12:00:00Z'
                if change == 'permission': f['record_permission']['permission'] = 'write'
                if change == 'head': f['pr']['head']['sha'] = 'f' * 40
                if change == 'unsupported_Q': f['target_workflow'] += b'\n# unrecognized executor\n'
                with self.assertRaises(ValueError): facts.verify(**f)

    def test_candidate_context_has_no_existing_object_increment(self):
        f = fixture()
        record = facts._record(f['record_comment']['body'], facts.MARKER)
        record['exception_rule'] = 'candidate_context_unrepresentable'
        f['record_comment']['body'] = facts.MARKER + '\n' + json.dumps(record)
        with self.assertRaisesRegex(ValueError, 'only constructs Q/H merge'):
            facts.verify(**f)

    def test_caller_dictionary_cannot_mint_required(self):
        f = fixture()
        route = identity.evaluate_strict_integration_requirement(trusted_projection=projection(f),
            expected_task_uid=f['task_uid'], verified_exception=dict(facts.verify(**f).facts))
        self.assertEqual(route['status'], 'blocked')
        self.assertEqual(route['reason'], 'strict_exception_facts_not_verified')

    def test_required_scope_cannot_be_replaced_with_targeted_checks(self):
        f = fixture()
        projected = projection(f)
        projected['ci_scope'] = 'targeted'
        projected['projection_digest'] = 'sha256:' + hashlib.sha256(json.dumps(
            {key: value for key, value in projected.items() if key != 'projection_digest'},
            sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        route = identity.evaluate_strict_integration_requirement(trusted_projection=projected,
            expected_task_uid=f['task_uid'], verified_exception=facts.verify(**f))
        self.assertEqual('blocked', route['status'])
        self.assertEqual('strict_exception_required_scope_not_in_trusted_plan', route['reason'])

    def test_current_candidate_cannot_activate_uninstalled_producer(self):
        f = fixture()
        f['pr']['body'] = 'Strict Integration Exception: 1'
        with patch.object(facts.subprocess, 'check_output', return_value=b'old protected policy without producer'):
            with self.assertRaisesRegex(ValueError, 'not installed protected Q'):
                facts.read_protected(f['repository'], f['task_uid'], f['issue'], f['pr'], f['target_oid'], ROOT)

    def test_ordinary_never_loads_exception_producer(self):
        f = fixture()
        with patch.object(identity, '_strict_exception_module', side_effect=AssertionError('ordinary loaded strict facts')):
            result = identity.evaluate_strict_integration_requirement(trusted_projection=projection(f), expected_task_uid=f['task_uid'])
        self.assertEqual(result['status'], 'not_required')

    def test_optional_plan_locator_omission_and_tamper_refused_after_binding(self):
        f = fixture()
        plan = {'strict_exception_comment_id': 1, 'source_review_identity': {
            'repository': f['repository'], 'pr_number': 7, 'source_head_oid': f['pr']['head']['sha'], 'task_uid': f['task_uid']}}
        for body in ('Task: ' + f['task_uid'], 'Strict Integration Exception: 2'):
            maintenance = type('Maintenance', (), {'_gh': staticmethod(lambda *args: {**f['pr'], 'body': body})})()
            with patch.object(facts, '_maintenance', return_value=maintenance):
                with self.assertRaisesRegex(ValueError, 'locator or head differs'):
                    facts.read_plan(plan)

    def test_adjudication_can_be_published_before_admin_selection(self):
        f = fixture()
        # Reviews contain proposal semantics, not a nonexistent future admin ID.
        record = facts._record(f['record_comment']['body'], facts.MARKER)
        for item, _ in f['reviews']:
            review = facts._record(item['body'], facts.REVIEW_MARKER)
            self.assertNotIn('exception_comment_id', review)
            self.assertEqual(facts.constraint_digest(record), review['constraint_digest'])
        f['record_comment']['id'] = 4
        f['record_comment']['html_url'] = 'https://github.com/eng-cc/oasis7/issues/12#issuecomment-4'
        f['record_comment']['created_at'] = f['record_comment']['updated_at'] = '2026-10-08T12:01:00Z'
        value = facts.verify(**f)
        self.assertEqual(4, value.facts['constraint_evidence']['comment_id'])

    def test_live_task_selector_prevents_hidden_or_stale_retained_boundary(self):
        f = fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', '-C', directory, 'init', '-q'], check=True)
            subprocess.run(['git', '-C', directory, 'config', 'user.name', 'QA'], check=True)
            subprocess.run(['git', '-C', directory, 'config', 'user.email', 'qa@example.invalid'], check=True)
            file = root / facts.WORKFLOW
            file.parent.mkdir(parents=True)
            def commit(content, message):
                file.write_bytes(content)
                subprocess.run(['git', '-C', directory, 'add', '.'], check=True)
                subprocess.run(['git', '-C', directory, 'commit', '-qm', message], check=True)
                return subprocess.check_output(['git', '-C', directory, 'rev-parse', 'HEAD'], text=True).strip()
            q = commit(f['target_workflow'], 'protected ordinary and strict executor')
            h = commit(f['source_workflow'], 'candidate orchestration guard')
            record = facts._record(f['record_comment']['body'], facts.MARKER)
            record.update(source_head_oid=h, target_oid=q)
            authority = comment(4, facts.MARKER + '\n' + json.dumps(record))
            def read(*args):
                endpoint = args[-1]
                if endpoint.endswith('/issues/12'): return {'number': 12, 'state': 'open', 'body': 'task_uid: ' + f['task_uid']}
                if '/comments?' in endpoint: return [[authority]]
                if endpoint.endswith('/permission'): return f['record_permission']
                raise AssertionError(endpoint)
            context = dict(root=root, repository=f['repository'], task_uid=f['task_uid'], issue=12,
                pr_number=7, source_head_oid=h, target_oid=q, changed_paths=[facts.WORKFLOW], github=read)
            self.assertEqual(4, identity.select_strict_exception_locator(**context, pr_body='Strict Integration Exception: 4'))
            for body in ('', 'Strict Integration Exception: 2'):
                with self.subTest(body=body), self.assertRaisesRegex(ValueError, 'hides or changes'):
                    identity.select_strict_exception_locator(**context, pr_body=body)
            h2 = commit(f['source_workflow'] + b'\n# unrelated documentation addition\n', 'amended source retains boundary')
            context['source_head_oid'] = h2
            with self.assertRaisesRegex(ValueError, 'stale H/Q'):
                identity.select_strict_exception_locator(**context, pr_body='')
            h3 = commit(f['target_workflow'], 'genuine boundary reverted')
            context['source_head_oid'] = h3
            self.assertIsNone(identity.select_strict_exception_locator(**context, pr_body=''))
            with self.assertRaisesRegex(ValueError, 'stale H/Q'):
                identity.select_strict_exception_locator(**context, pr_body='', plan_locator=4)

    def test_ordinary_real_changed_file_reader_does_not_treat_paths_as_exception(self):
        spec = importlib.util.spec_from_file_location('boundary_receipt', Path(__file__).with_name('ci-ready-receipt.py'))
        receipt = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(receipt)
        with patch.object(receipt.subprocess, 'check_output', side_effect=subprocess.CalledProcessError(1, 'git')):
            with patch.object(receipt, 'gh', return_value=[[{'filename': facts.WORKFLOW}]]):
                self.assertTrue(receipt._pr_workflow_changed('eng-cc/oasis7', 7, {'base': {'sha': 'a'}, 'head': {'sha': 'b'}}, ROOT))
            with patch.object(receipt, 'gh', return_value=[[{'filename': 'scripts/pm/helper.py'}]]):
                self.assertFalse(receipt._pr_workflow_changed('eng-cc/oasis7', 7, {'base': {'sha': 'a'}, 'head': {'sha': 'b'}}, ROOT))
            with patch.object(receipt, 'gh', return_value=None):
                with self.assertRaisesRegex(ValueError, 'inventory unreadable'):
                    receipt._pr_workflow_changed('eng-cc/oasis7', 7, {'base': {'sha': 'a'}, 'head': {'sha': 'b'}}, ROOT)

    def test_production_ensure_effect_counts_and_uncertain_dispatch_barrier(self):
        store = facts._adjacent('strict_exception_test_store', 'workflow-durable-store.py')
        verified = facts.verify(**fixture())
        selected = {'id': 44, 'run_attempt': 2}
        gate = SimpleNamespace(discover_required_policy=lambda *args: {'status': 'resolved', 'required_status_checks': [{'context': 'required-gate', 'app_id': 42}]})
        for case in ('pending', 'failed', 'success', 'absent', 'uncertain', 'unreadable'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                subprocess.run(['git', '-C', directory, 'init', '-q'], check=True)
                integration = SimpleNamespace(current_request=Mock(return_value=None if case in ('absent', 'uncertain') else selected),
                    dispatch=Mock(), verified_run=Mock(return_value=({'id': 9}, {'verified': True})))
                receipt = SimpleNamespace(planner_for_run=Mock(return_value={'scope': 'full', 'impact_projection_test_profile': 'full'}))
                def github(endpoint):
                    if '/actions/runs/' not in endpoint: return {'default_branch': 'main'}
                    if case == 'unreadable': raise ValueError('actual run read unavailable')
                    return {'run_attempt': 2, 'status': 'in_progress' if case == 'pending' else 'completed',
                            'conclusion': 'failure' if case == 'failed' else 'success'}
                call = lambda: facts.ensure_verified(Path(directory), verified, 'projection.json',
                    integration=integration, receipt=receipt, store=store, github=github)
                with patch.object(facts, '_adjacent', return_value=gate):
                    if case == 'unreadable':
                        with self.assertRaisesRegex(ValueError, 'read unavailable'): call()
                    elif case == 'uncertain':
                        integration.dispatch.side_effect = OSError('ambiguous workflow write')
                        with self.assertRaisesRegex(OSError, 'ambiguous'): call()
                        with self.assertRaisesRegex(ValueError, 'never resend'): call()
                    else:
                        result = call()
                        self.assertEqual({'pending': 'waiting', 'failed': 'blocked', 'success': 'reused', 'absent': 'requested'}[case], result['status'])
                        if case == 'absent':
                            with self.assertRaisesRegex(ValueError, 'never resend'): call()
                            # Exact later Actions readback reconciles the reserved
                            # intent and waits without another remote side effect.
                            integration.current_request.return_value = selected
                            result = call()
                            self.assertEqual('reused', result['status'])
                    self.assertEqual(1 if case in ('absent', 'uncertain') else 0, integration.dispatch.call_count)
                    if integration.dispatch.called:
                        kwargs = integration.dispatch.call_args.kwargs
                        self.assertEqual(verified.facts['source_head_oid'], kwargs['expected_head'])
                        self.assertEqual(verified.facts['target_oid'], kwargs['expected_target'])

    def test_ensure_calls_real_dispatch_with_exact_subject_and_one_outbound_write(self):
        integration = facts._adjacent('strict_exception_real_dispatch', 'integration_ci.py')
        store = facts._adjacent('strict_exception_real_dispatch_store', 'workflow-durable-store.py')
        f = fixture()
        verified = facts.verify(**f)
        h, q = f['pr']['head']['sha'], f['target_oid']
        pr = {'number': 7, 'state': 'open', 'merged': False, 'draft': True,
            'body': 'Task: ' + f['task_uid'] + '\nRefs #12\nStrict Integration Exception: 1',
            'head': {'sha': h, 'repo': {'full_name': f['repository']}},
            'base': {'sha': q, 'ref': 'main', 'repo': {'full_name': f['repository']}}}
        def github(*args):
            endpoint = args[-1]
            if endpoint == 'repos/eng-cc/oasis7': return {'default_branch': 'main'}
            if endpoint.endswith('/pulls/7'): return pr
            if endpoint.endswith('/git/ref/heads/main'): return {'object': {'sha': q}}
            if '/contents/' in endpoint: return {'content': base64.b64encode(f['target_workflow']).decode()}
            raise AssertionError(endpoint)
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['git', '-C', directory, 'init', '-q'], check=True)
            projection_path = Path(directory) / 'projection.json'
            projection_path.write_text(json.dumps(projection(f)))
            actual_run, writes = integration.subprocess.run, []
            def outbound(argv, **kwargs):
                if argv[:3] == ['gh', 'workflow', 'run']:
                    writes.append(argv)
                    return subprocess.CompletedProcess(argv, 0)
                return actual_run(argv, **kwargs)
            with patch.object(integration, 'gh', side_effect=github), patch.object(integration, 'current_request', return_value=None), patch.object(integration.subprocess, 'run', side_effect=outbound):
                result = facts.ensure_verified(Path(directory), verified, projection_path,
                    integration=integration, receipt=None, store=store, github=github)
                self.assertEqual('requested', result['status'])
                self.assertEqual(1, len(writes))
                argv = writes[0]
                self.assertIn('expected_head=' + h, argv)
                self.assertIn('integration_base=' + q, argv)
                self.assertEqual(['gh', 'workflow', 'run', 'rust.yml'], argv[:4])
                with self.assertRaisesRegex(ValueError, 'never resend'):
                    facts.ensure_verified(Path(directory), verified, projection_path,
                        integration=integration, receipt=None, store=store, github=github)
                self.assertEqual(1, len(writes))

    def test_initial_receipt_main_dispatches_once_and_validation_stays_read_only(self):
        receipt = facts._adjacent('strict_exception_acquisition_receipt', 'ci-ready-receipt.py')
        import ci_ready_receipt_identity as identity
        import integration_ci as integration
        store = facts._adjacent('strict_exception_acquisition_store', 'workflow-durable-store.py')
        f = fixture()
        verified = facts.verify(**f)
        h, q = f['pr']['head']['sha'], f['target_oid']
        raw = json.dumps(projection(f)).encode()
        pr = {'number': 7, 'state': 'open', 'merged': False, 'draft': True,
            'body': 'Task: ' + f['task_uid'] + '\nRefs #12\nStrict Integration Exception: 1\n'
                + '<!-- oasis7-impact-projection-b64: ' + base64.b64encode(raw).decode() + ' -->',
            'head': {'sha': h, 'repo': {'full_name': f['repository']}},
            'base': {'sha': q, 'ref': 'main', 'repo': {'full_name': f['repository']}}}
        def github(*args):
            endpoint = args[-1]
            if endpoint == 'repos/eng-cc/oasis7': return {'default_branch': 'main'}
            if endpoint.endswith('/pulls/7'): return pr
            if endpoint.endswith('/git/ref/heads/main'): return {'object': {'sha': q}}
            if '/contents/' in endpoint: return {'content': base64.b64encode(f['target_workflow']).decode()}
            raise AssertionError(endpoint)
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(['git', '-C', directory, 'init', '-q'], check=True)
            argv = ['ci-ready-receipt.py', '--repository', f['repository'], '--task-uid', f['task_uid'],
                '--task-issue-number', '12', '--pr-number', '7', '--check-app-id', '42',
                '--planner-digest', 'unused-until-proof', '--root', directory]
            writes = []
            actual_run = subprocess.run
            def outbound(command, **kwargs):
                if command[:3] == ['gh', 'workflow', 'run']:
                    writes.append(command)
                    return subprocess.CompletedProcess(command, 0)
                return actual_run(command, **kwargs)
            def ensure(repo, uid, issue, number, target, root, path):
                self.assertEqual((repo, uid, issue, number, target), (f['repository'], f['task_uid'], 12, 7, q))
                self.assertEqual(Path(path).read_bytes(), raw)
                return facts.ensure_verified(root, verified, path, integration=integration,
                    receipt=receipt, store=store, github=github)
            with patch.object(receipt, 'gh', side_effect=github), patch.object(integration, 'gh', side_effect=github), \
                    patch.object(integration, 'current_request', return_value=None), \
                    patch.object(identity, 'select_strict_exception_locator', return_value=1), \
                    patch.object(identity, '_strict_exception_module', return_value=facts), \
                    patch.object(facts, 'read_protected', return_value=verified), \
                    patch.object(facts, 'ensure_protected', side_effect=ensure) as producer, \
                    patch.object(subprocess, 'run', side_effect=outbound), patch.object(sys, 'argv', argv):
                with self.assertRaisesRegex(SystemExit, 'requested'): receipt.main()
                self.assertEqual(1, len(writes))
                self.assertIn('expected_head=' + h, writes[0])
                self.assertIn('integration_base=' + q, writes[0])
                with self.assertRaisesRegex(ValueError, 'never resend'): receipt.main()
                self.assertEqual(1, len(writes))
                producer.reset_mock()
                existing = Path(directory) / 'receipt.json'
                existing.write_text('{}')
                with patch.object(sys, 'argv', argv + ['--receipt', str(existing)]):
                    with self.assertRaisesRegex(SystemExit, 'request is absent'): receipt.main()
                with self.assertRaisesRegex(SystemExit, 'request is absent'):
                    receipt.selected_live(f['repository'], f['task_uid'], 12, 7, 'required-gate', 42, canonical_root=directory)
                producer.assert_not_called()
                self.assertEqual(1, len(writes))
                selected = {'id': 44, 'run_attempt': 2, 'requested_at': 1780000001.0}
                with patch.object(integration, 'current_request', return_value=selected):
                    for reason in ('pending', 'latest failed', 'unreadable'):
                        with patch.object(integration, 'verified_run', side_effect=ValueError(reason)):
                            with self.assertRaisesRegex(SystemExit, reason): receipt.main()
                    with patch.object(integration, 'verified_run', return_value=({'id': 9}, {'scope': 'full', 'impact_projection_test_profile': 'full'})), \
                            patch.object(receipt, 'planner_for_run', return_value={'scope': 'full', 'impact_projection_test_profile': 'full'}):
                        result = receipt.acquire_selected_live(f['repository'], f['task_uid'], 12, 7,
                            'required-gate', 42, canonical_root=directory)
                        self.assertEqual(44, result[1]['_integration']['request_id'])
                producer.assert_not_called()
                self.assertEqual(1, len(writes))
                ordinary = {**pr, 'body': 'Task: ' + f['task_uid'] + '\nRefs #12'}
                with patch.object(receipt, 'gh', return_value=ordinary), \
                        patch.object(receipt, '_pr_workflow_changed', return_value=False), \
                        patch.object(receipt, 'live', return_value=(ordinary, {'id': 9}, q, h)):
                    self.assertEqual(ordinary, receipt.acquire_selected_live(f['repository'], f['task_uid'], 12, 7,
                        'required-gate', 42, canonical_root=directory)[0])
                producer.assert_not_called()

if __name__ == '__main__':
    unittest.main()
