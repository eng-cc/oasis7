#!/usr/bin/env python3
"""Regression witnesses for exact partition coverage and same-attempt barriers."""
import copy
from collections import Counter
import json
from pathlib import Path
import re
import shlex
import subprocess
import unittest
import ci_required_execution as execution


class ExecutionTests(unittest.TestCase):
    def test_shared_execution_and_workflow_paths_conservatively_route_full(self):
        for path in ('scripts/pm/ci_required_execution.py', 'scripts/pm/ci-required-execution.test.py', 'scripts/pm/ci_required_workflow.py', 'scripts/pm/ci-required-workflow.test.py'):
            with self.subTest(path=path):
                output = subprocess.check_output([execution.sys.executable, '-I', str(execution.ROOT/'scripts/plan-rust-required-scope.py'), '--event-name', 'pull_request', '--changed-path', path], cwd=execution.ROOT, text=True)
                actual = dict(line.split('=', 1) for line in output.splitlines())
                self.assertEqual(actual['scope'], 'full')
                self.assertEqual(actual['run_workflow_governance_contracts'], 'true')
                self.assertEqual(actual['run_doc_checker_contracts'], 'true')
                self.assertEqual(actual['run_operational_contracts'], 'true')
                self.assertEqual(actual['run_oasis7_required_tests'], 'true')
                self.assertIn('shared_required_gate:' + path, actual['reason_summary'])

    def selection(self, all_selected=True):
        policy = execution.config()
        output = {s['planner_field']: 'true' if all_selected else 'false' for s in policy['selector_ownership'] if s['mode'] == 'planner-owned'}
        output.update({'needs_' + x: 'true' for x in ('python', 'markdown', 'rust_toolchain', 'node', 'system_deps', 'trunk', 'wasm_target')})
        return execution.selection_from_planner(output, 'workflow_dispatch', 'integration_revalidation')

    def identity(self):
        identity = {x: '' for x in execution.IDENTITY_FIELDS}
        identity.update(repository='eng-cc/oasis7', run_id='123', run_attempt='2', event_name='workflow_dispatch', run_mode='integration_revalidation', source_scope='full')
        for x in ('base_sha', 'source_head_sha', 'tested_sha', 'tested_tree', 'workflow_sha', 'run_head_sha'): identity[x] = 'a' * 40
        for x in ('planner_digest', 'config_digest', 'executor_digest', 'impact_projection_digest'): identity[x] = 'b' * 64
        return identity

    def fixtures(self):
        selection, identity = self.selection(), self.identity()
        plan = execution.make_plan(selection, identity)
        results = [{'schema': 'oasis7-required-worker-result/v1', 'worker': worker, 'identity': identity, 'plan_digest': plan['plan_digest'], 'command_definition_digest': plan['command_definition_digest'], 'completed': commands} for worker, commands in plan['workers'].items()]
        jobs = [{'name': f'required-work ({worker})', 'run_id': '123', 'run_attempt': '2', 'head_sha': 'a'*40, 'status': 'completed', 'conclusion': 'success'} for worker in plan['workers']]
        return plan, selection, identity, results, jobs

    def test_exact_partition_and_duplicate_environment(self):
        selected = execution.worker_assignments(self.selection())
        governance = [x for worker, commands in selected.items() if worker.startswith('governance-') for x in commands]
        expected = [c['id'] for c in execution.COMMANDS if c['id'].startswith('governance:')]
        self.assertEqual(Counter(governance), Counter(expected))
        self.assertEqual(len(expected), 115)  # 113 original calls, shared regression, main strict-exception obligation.
        publication = [c for c in execution.COMMANDS if c['id'].startswith('governance:pm-pr-projection-publication-test-py')]
        self.assertEqual(len(publication), 2)
        self.assertNotEqual(publication[0]['argv'], publication[1]['argv'])
        self.assertTrue(all(execution.governance_worker(c) == 'governance-3' for c in publication))

    def test_original_command_argv_multiset(self):
        original = subprocess.check_output(['git', '-C', str(execution.ROOT), 'show', 'dd04c28ab9aa850a6123d11544f1790717fb741c:scripts/ci-tests.sh'], text=True)
        functions = dict(re.findall(r'(?m)^(run_\w+)\(\) \{\n(.*?)^\}', original, re.S))
        def expand(name):
            commands = []
            for line in functions[name].splitlines():
                line = line.strip()
                if line.startswith('run '): commands.append(shlex.split(line[4:]))
                elif line in functions: commands.extend(expand(line))
                elif line: self.fail('unexpected historical command: ' + line)
            return commands
        for group in {c['group'] for c in execution.COMMANDS}:
            preserved=expand(group)
            if group=='run_workflow_governance_operational_contract_tests':
                index=preserved.index(['python3','./scripts/pm/ci-ready-receipt.test.py'])+1
                preserved.insert(index,['python3','./scripts/pm/strict_exception_facts.test.py'])
            self.assertEqual([c['argv'] for c in execution.COMMANDS if c['group'] == group and c['id'] != 'governance:ci-required-execution'], preserved)

    def test_missing_selector_resource_event_rejected(self):
        for location, field in [('planner_output', 'run_rust_baseline'), ('planner_output', 'needs_node'), (None, 'event_name')]:
            selection = self.selection()
            del (selection[location] if location else selection)[field]
            with self.assertRaises(execution.ExecutionError): execution.worker_assignments(selection)

    def test_native_web_and_contract_function_bodies_and_bridge_preserved(self):
        original = subprocess.check_output(['git', '-C', str(execution.ROOT), 'show', 'dd04c28ab9aa850a6123d11544f1790717fb741c:scripts/ci-tests.sh'], text=True)
        current = (execution.ROOT / 'scripts/ci-tests.sh').read_text()
        def bodies(source): return dict(re.findall(r'(?m)^(run_\w+)\(\) \{\n(.*?)^\}', source, re.S))
        before, after = bodies(original), bodies(current)
        coarse = ['run_newapi_bridge_service_accounting_tests', 'run_rustsec_advisory_check', 'run_oasis7_required_tier_tests', 'run_oasis7_required_tier_clippy', 'run_scenario_regression_tests', 'run_oasis7_consensus_tests', 'run_oasis7_consensus_clippy', 'run_oasis7_distfs_tests', 'run_oasis7_distfs_clippy', 'run_oasis7_node_tests', 'run_oasis7_node_clippy', 'run_oasis7_net_tests', 'run_oasis7_net_clippy', 'run_oasis7_net_libp2p_tests', 'run_oasis7_net_libp2p_clippy', 'run_pixel_world_bridge_lib_tests', 'run_oasis7_workspace_support_crate_tests', 'run_pixel_world_bridge_wasm_check', 'run_oasis7_viewer_software_safe_feedback_contract_tests', 'run_oasis7_viewer_software_safe_build', 'run_oasis7_viewer_performance_smoke_report_only', 'run_oasis7_client_launcher_web_build', 'run_doc_checker_contract_tests', 'run_cargo_tooling_contract_tests', 'run_packaging_contract_tests', 'run_site_contract_tests', 'run_codex_agent_config_validation', 'run_compile_metrics_contract_tests']
        for name in coarse: self.assertEqual(after[name], before[name], name)
        workflow = subprocess.check_output(['git', '-C', str(execution.ROOT), 'show', 'dd04c28ab9aa850a6123d11544f1790717fb741c:.github/workflows/rust.yml'], text=True)
        bridge = re.search(r'(env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime.*?execution_bridge_real_tests::real_execution_bridge::tests)', workflow, re.S).group(1)
        self.assertEqual(shlex.split(bridge.replace('\\\n', ' ')), execution.EXECUTION_BRIDGE_ARGV)

    def test_docs_has_no_native_or_web_and_baseline_ownership(self):
        selection = self.selection(False)
        self.assertEqual(execution.worker_assignments(selection), {})
        selection['planner_output']['run_doc_checker_contracts'] = 'true'
        selection = execution.selection_from_planner(selection['planner_output'], selection['event_name'], selection['run_mode'])
        self.assertEqual(set(execution.worker_assignments(selection)), {'contracts'})
        self.assertEqual(execution.worker_resources(selection, 'contracts')['needs_rust_toolchain'], 'false')
        selection['planner_output']['run_rust_baseline'] = 'true'
        self.assertIn('required-work (native)', execution.execution_job_requirements(selection)['required_gate_baseline'])

    def test_same_attempt_and_command_negative_paths(self):
        fixtures = self.fixtures()
        self.assertTrue(execution.verify(*fixtures))
        for mutate in (lambda f: f[3].pop(), lambda f: f[3].append(f[3][0]), lambda f: f[3][0]['completed'].pop(), lambda f: f[4][0].update(run_attempt='1'), lambda f: f[4][0].update(status='in_progress'), lambda f: f[4][0].update(conclusion='skipped'), lambda f: f[4].append(f[4][0]), lambda f: f[3][0].update(plan_digest='c'*64)):
            bad = copy.deepcopy(fixtures)
            mutate(bad)
            with self.assertRaises(execution.ExecutionError): execution.verify(*bad)

    def test_plan_tampering_and_unknown_layout(self):
        plan, *_ = self.fixtures()
        plan['workers']['unknown'] = ['arbitrary shell']
        with self.assertRaises(execution.ExecutionError): execution.validate_plan(plan)
        with self.assertRaises(execution.ExecutionError): execution.worker_assignments(self.selection(), 'unknown')
        self.assertEqual(execution.worker_assignments(self.selection(), 'required-serial/v1'), {})

    def test_unknown_or_unselected_actual_worker_job_rejected(self):
        fixtures = list(self.fixtures())
        for name in ('required-work (unknown)', 'required-work (not-selected)'):
            bad = copy.deepcopy(fixtures)
            bad[4].append({**bad[4][0], 'name': name})
            with self.assertRaises(execution.ExecutionError): execution.verify(*bad)

    def test_actual_run_head_is_distinct_from_workflow_authority(self):
        plan, selection, identity, results, jobs = self.fixtures()
        identity['run_head_sha'] = 'c' * 40
        plan = execution.make_plan(selection, identity)
        for result in results: result.update(identity=identity, plan_digest=plan['plan_digest'])
        with self.assertRaises(execution.ExecutionError): execution.verify(plan, selection, identity, results, jobs)
        for job in jobs: job['head_sha'] = identity['run_head_sha']
        self.assertTrue(execution.verify(plan, selection, identity, results, jobs))

    def test_platform_requirements_are_not_gate_prerequisites(self):
        selection = self.selection()
        gate = execution.gate_job_requirements(selection)
        units = execution.execution_job_requirements(selection)
        self.assertNotIn('windows-package-rollout-behavior', gate)
        self.assertIn('windows-package-rollout-behavior', units['operational_contracts'])
        self.assertNotIn('required-gate', gate)

    def test_inventory_bidirectional_shared_commands(self):
        rows = [line.split('\t') for line in (execution.ROOT/'scripts/ci-required-capability-test-inventory.tsv').read_text().splitlines()[1:]]
        for capability, prefix in [('workflow_governance', 'governance:'), ('operational_contracts', 'operational:')]:
            declared = {path for row in rows if row[2] == capability for path in row[1].split(',')}
            actual = {arg.removeprefix('./') for c in execution.COMMANDS if c['id'].startswith(prefix) for arg in c['argv'] if arg.startswith('./scripts/')}
            self.assertEqual(actual, declared)

    def test_main_strict_exception_obligation_is_in_shared_governance_group(self):
        commands=[c for c in execution.COMMANDS if c['group']=='run_workflow_governance_operational_contract_tests']
        ids=[c['id'] for c in commands]
        ident='governance:pm-strict-exception-facts-test-py'
        self.assertEqual(ids.count(ident),1)
        self.assertEqual(ids.index(ident),ids.index('governance:pm-ci-ready-receipt-test-py')+1)
        self.assertEqual(commands[ids.index(ident)]['argv'],['python3','./scripts/pm/strict_exception_facts.test.py'])


if __name__ == '__main__':
    unittest.main()
