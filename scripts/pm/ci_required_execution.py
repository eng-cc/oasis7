#!/usr/bin/env python3
"""Trusted required execution definitions and same-attempt scheduling (not v2 evidence)."""
from __future__ import annotations
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

LAYOUTS = ('required-parallel/v1', 'required-serial/v1')
EXECUTION_BRIDGE_ARGV = ['env', '-u', 'RUSTC_WRAPPER', 'cargo', 'test', '-p', 'oasis7', '--bin', 'oasis7_chain_runtime', '--features', 'test_tier_required', 'execution_bridge_real_tests::real_execution_bridge::tests']
CAPABILITY_SELECTORS = {"oasis7_required":"run_oasis7_required_tests","consensus":"run_consensus_tests","distfs":"run_distfs_tests","node":"run_oasis7_node_tests","net":"run_oasis7_net_tests","viewer_js_required":"run_viewer_contract_tests","viewer_performance_report":"run_viewer_perf_smoke","pixel_world_bridge":"run_pixel_world_bridge_lib_tests","launcher_web":"run_launcher_web_build","workspace_support":"run_oasis7_workspace_support_crate_tests","scenario_regression":"run_scenario_regression","operational_contracts":"run_operational_contracts","packaging_contracts":"run_packaging_contracts","workflow_governance":"run_workflow_governance_contracts","codex_agent_config_validation":"run_codex_agent_config_validation","compile_metrics":"run_compile_metrics_contract_tests","site_quality":"run_site_contract_tests","doc_checker_contracts":"run_doc_checker_contracts","cargo_tooling_contracts":"run_cargo_tooling_contracts"}

def selection_from_planner(planner_output, event_name, run_mode):
    units = sorted(['required_gate_baseline'] + [cap for cap, field in CAPABILITY_SELECTORS.items() if planner_output.get(field) == 'true'])
    return validate_selection({'unit_ids': units, 'planner_output': planner_output, 'event_name': event_name, 'run_mode': run_mode})
ROOT = Path(__file__).resolve().parents[2]

class ExecutionError(ValueError):
    pass

def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()

def config():
    return json.loads((ROOT / 'scripts/ci-required-scope.v2.json').read_text())

def validate_selection(selection):
    if not isinstance(selection, dict) or not {'unit_ids', 'planner_output', 'event_name', 'run_mode'} <= set(selection):
        raise ExecutionError('selection needs complete unit/selector/resource/event context')
    units = selection['unit_ids']
    if not isinstance(units, list) or not units or units != sorted(set(units)) or not set(units) <= set(config()['capabilities']):
        raise ExecutionError('unknown, duplicate or unsorted units')
    output = selection['planner_output']
    fields = [x['planner_field'] for x in config()['selector_ownership'] if x['mode'] == 'planner-owned']
    fields += ['needs_' + x for x in ('python', 'markdown', 'rust_toolchain', 'node', 'system_deps', 'trunk', 'wasm_target')]
    if not isinstance(output, dict) or any(output.get(x) not in ('true', 'false') for x in fields):
        raise ExecutionError('missing or malformed selector/resource input')
    expected_units = sorted(['required_gate_baseline'] + [cap for cap, field in CAPABILITY_SELECTORS.items() if output[field] == 'true'])
    if units != expected_units:
        raise ExecutionError('units do not match complete trusted selector context')
    if any(output['needs_' + resource] != 'true' for resource in config()['baseline_resources']):
        raise ExecutionError('missing baseline resources')
    for unit in units:
        if any(output['needs_' + resource] != 'true' for resource in config()['resource_requirements'][unit]):
            raise ExecutionError('selected capability resource contradiction')
    for x in ('event_name', 'run_mode'):
        if not isinstance(selection[x], str) or not selection[x]:
            raise ExecutionError('missing event/run mode')
    return selection

def enabled(selection, field):
    return selection['planner_output'][field] == 'true'

NATIVE_FIELDS = ('run_rust_baseline', 'run_oasis7_required_tests', 'run_scenario_regression', 'run_consensus_tests', 'run_distfs_tests', 'run_oasis7_node_tests', 'run_oasis7_net_tests', 'run_oasis7_net_libp2p_tests', 'run_pixel_world_bridge_lib_tests', 'run_oasis7_workspace_support_crate_tests')
WEB_FIELDS = ('run_viewer_contract_tests', 'run_viewer_wasm_check', 'run_viewer_perf_smoke', 'run_pixel_world_bridge_wasm_check', 'run_launcher_web_build')
CONTRACT_FIELDS = ('run_doc_checker_contracts', 'run_cargo_tooling_contracts', 'run_packaging_contracts', 'run_site_contract_tests', 'run_codex_agent_config_validation', 'run_compile_metrics_contract_tests')

def governance_worker(command):
    name = command['id']
    if 'terminal-readiness-recovery' in name or 'task-primary-package' in name:
        return 'governance-1'
    if 'ci-reuse-acceptance-qa' in name or 'prepare-loop-ci-authority' in name:
        return 'governance-2'
    if 'pr-projection-publication' in name or 'trusted-cargo-scope' in name:
        return 'governance-3'
    if any(x in name for x in ('pr-projection-publish-cli', 'loop-bootstrap', 'terminal-delivery', 'required-scope-routing')):
        return 'governance-4'
    # Fixed offline partition; does not read runtime timing data.
    return 'governance-' + str(1 + int(hashlib.sha256(name.encode()).hexdigest()[:8], 16) % 4)

def worker_assignments(selection, layout='required-parallel/v1'):
    validate_selection(selection)
    if layout not in LAYOUTS:
        raise ExecutionError('unknown execution layout')
    if layout == 'required-serial/v1':
        return {}
    workers = {}
    if enabled(selection, 'run_workflow_governance_contracts'):
        workers.update({f'governance-{i}': [] for i in range(1, 5)})
        for c in COMMANDS:
            if c['id'].startswith('governance:'):
                workers[governance_worker(c)].append(c['id'])
    if enabled(selection, 'run_operational_contracts'):
        workers.update({'operational-1': [], 'operational-2': []})
        for c in COMMANDS:
            if c['id'].startswith('operational:'):
                worker = 'operational-1' if 'full-network-clean-room-adapter' in c['id'] else 'operational-2'
                workers[worker].append(c['id'])
    for worker, fields in [('native', NATIVE_FIELDS), ('web', WEB_FIELDS), ('contracts', CONTRACT_FIELDS)]:
        ids = ['component:' + f for f in fields if enabled(selection, f)]
        if ids:
            if worker == 'native' and enabled(selection, 'run_oasis7_required_tests'):
                ids.append('native:execution-bridge')
            workers[worker] = ids
    return workers

def gate_job_requirements(selection, layout='required-parallel/v1'):
    return sorted(['required-plan'] + [f'required-work ({x})' for x in worker_assignments(selection, layout)])

def worker_resources(selection, worker):
    assignments = worker_assignments(selection)
    if worker not in assignments:
        raise ExecutionError('resources requested for unknown/unselected worker')
    keys = ('python', 'markdown', 'rust_toolchain', 'node', 'system_deps', 'trunk', 'wasm_target')
    resources = {f'needs_{x}': 'false' for x in keys}
    resources['needs_python'] = 'true'
    if worker.startswith('governance-'):
        # Acceptance fixtures run nested required tiers; declare the real closure.
        resources.update({f'needs_{x}': 'true' for x in keys})
    elif worker.startswith('operational-'):
        resources['needs_rust_toolchain'] = 'true'
    else:
        for command_id in assignments[worker]:
            field = command_id.removeprefix('component:')
            capability = next((cap for cap, selector in CAPABILITY_SELECTORS.items() if selector == field), None)
            requirements = config()['resource_requirements'].get(capability, [])
            if field == 'run_rust_baseline': requirements = ['rust_toolchain', 'system_deps']
            if worker == 'web': requirements = [x for x in keys if enabled(selection, 'needs_' + x)]
            for resource in requirements: resources['needs_' + resource] = 'true'
    resources['cache_write'] = 'true' if worker == 'native' else 'false'
    return resources

def execution_job_requirements(selection, layout='required-parallel/v1'):
    assignments = worker_assignments(selection, layout)
    mapping = {'workflow_governance': ['governance-1', 'governance-2', 'governance-3', 'governance-4'], 'operational_contracts': ['operational-1', 'operational-2'], 'viewer_js_required': ['web'], 'viewer_performance_report': ['web'], 'launcher_web': ['web'], 'pixel_world_bridge': ['native', 'web']}
    native_units = {'oasis7_required', 'consensus', 'distfs', 'node', 'net', 'workspace_support', 'scenario_regression'}
    result = {}
    for unit in selection['unit_ids']:
        workers = mapping.get(unit, ['native'] if unit in native_units else ['contracts'])
        if unit == 'required_gate_baseline':
            workers = ['native'] if enabled(selection, 'run_rust_baseline') else []
        jobs = ['required-gate']
        if layout == 'required-parallel/v1':
            jobs += ['required-plan'] + [f'required-work ({w})' for w in workers if w in assignments]
        # Original platform conditions are applied only for events that start them.
        if selection['event_name'] in ('pull_request', 'push', 'workflow_dispatch'):
            if unit == 'packaging_contracts':
                jobs.append('testnet-packages-macos-arm64-contract')
            if unit == 'operational_contracts':
                jobs += ['windows-package-rollout-behavior'] + [f'public-testnet-fleet-health-contract ({r})' for r in ('macos-14', 'ubuntu-24.04', 'windows-2022')]
        result[unit] = sorted(jobs)
    return result

IDENTITY_FIELDS = ('repository', 'run_id', 'run_attempt', 'event_name', 'run_mode', 'base_sha', 'source_head_sha', 'tested_sha', 'tested_tree', 'workflow_sha', 'run_head_sha', 'task_uid', 'pr_number', 'request_key', 'planner_digest', 'config_digest', 'executor_digest', 'source_scope', 'impact_projection_digest')

def validate_identity(identity):
    if not isinstance(identity, dict) or set(identity) != set(IDENTITY_FIELDS):
        raise ExecutionError('frozen identity fields incomplete or unknown')
    for field in ('run_id', 'run_attempt'):
        if str(identity[field]).isdigit() is False or int(identity[field]) <= 0:
            raise ExecutionError('invalid run/attempt')
    for field in ('base_sha', 'source_head_sha', 'tested_sha', 'tested_tree', 'workflow_sha', 'run_head_sha'):
        value = identity[field]
        if not isinstance(value, str) or len(value) != 40 or any(c not in '0123456789abcdef' for c in value):
            raise ExecutionError('invalid frozen Git identity')
    for field in ('planner_digest', 'config_digest', 'executor_digest', 'impact_projection_digest'):
        value = identity[field]
        if not isinstance(value, str) or len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
            raise ExecutionError('invalid frozen digest')
    if not all(isinstance(identity[x], str) and identity[x] for x in ('repository', 'event_name', 'run_mode', 'source_scope')):
        raise ExecutionError('invalid frozen context')
    return identity

def make_plan(selection, identity, layout='required-parallel/v1'):
    validate_identity(identity)
    validate_selection(selection)
    if any(selection[x] != identity[x] for x in ('event_name', 'run_mode')):
        raise ExecutionError('event selection differs from frozen identity')
    body = {'schema': 'oasis7-required-execution-plan/v1', 'layout': layout, 'identity': identity, 'selection': selection, 'selection_digest': digest(selection), 'workers': worker_assignments(selection, layout), 'command_definition_digest': digest(COMMANDS)}
    return {**body, 'plan_digest': digest(body)}

def validate_plan(plan):
    expected = make_plan(plan['selection'], plan['identity'], plan['layout'])
    if plan != expected:
        raise ExecutionError('plan differs from trusted frozen recomputation')
    return plan

def checkout_identity(root, identity):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
    if git('rev-parse', 'HEAD') != identity['tested_sha'] or git('rev-parse', 'HEAD^{tree}') != identity['tested_tree']:
        raise ExecutionError('worker checkout does not match exact frozen M/tree')
    if git('status', '--porcelain', '--untracked-files=no'):
        raise ExecutionError('tested checkout is dirty')

def selector_environment(selection):
    output = selection['planner_output']
    env = {'OASIS7_CI_EXECUTION_CONTRACT': 'required-domain-split/v2'}
    for spec in config()['selector_ownership']:
        if spec['mode'] == 'planner-owned':
            env[spec['name']] = output[spec['planner_field']]
    for key in ('python', 'markdown', 'rust_toolchain', 'node', 'system_deps', 'trunk', 'wasm_target'):
        env['OASIS7_CI_NEEDS_' + key.upper()] = output['needs_' + key]
    env.update(OASIS7_CI_RUN_PROVIDER_LIVE_GATE='false', OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE='false')
    return env

def execute_group(group, root):
    if group not in {x['group'] for x in COMMANDS}:
        raise ExecutionError('unknown trusted command group')
    for c in COMMANDS:
        if c['group'] == group:
            print('+ ' + ' '.join(c['argv']), flush=True)
            subprocess.run(c['argv'], cwd=root, check=True)

def run_worker(plan, worker, root):
    validate_plan(plan)
    checkout_identity(root, plan['identity'])
    for variable, key in [('GITHUB_REPOSITORY', 'repository'), ('GITHUB_RUN_ID', 'run_id'), ('GITHUB_RUN_ATTEMPT', 'run_attempt')]:
        if variable in os.environ and str(os.environ[variable]) != str(plan['identity'][key]):
            raise ExecutionError('worker platform run/attempt differs from frozen identity')
    if worker not in plan['workers']:
        raise ExecutionError('unknown or unselected worker')
    completed = []
    records = {c['id']: c for c in COMMANDS}
    env = {**os.environ, **selector_environment(plan['selection'])}
    driver = ROOT / 'scripts/ci-tests.sh'
    for command_id in plan['workers'][worker]:
        if command_id in records:
            argv = records[command_id]['argv']
        elif command_id.startswith('component:'):
            argv = ['bash', str(driver), 'required-component', '--component', command_id.split(':', 1)[1], '--repo-root', str(root)]
        elif command_id == 'native:execution-bridge':
            argv = EXECUTION_BRIDGE_ARGV
        else:
            raise ExecutionError('unknown trusted command id')
        print('+ ' + ' '.join(argv), flush=True)
        subprocess.run(argv, cwd=root, env=env, check=True)
        completed.append(command_id)
    return {'schema': 'oasis7-required-worker-result/v1', 'worker': worker, 'identity': plan['identity'], 'plan_digest': plan['plan_digest'], 'command_definition_digest': plan['command_definition_digest'], 'completed': completed}

def verify(plan, selection, identity, results, jobs):
    validate_plan(plan)
    if plan != make_plan(selection, identity, plan['layout']):
        raise ExecutionError('gate frozen input recomputation mismatch')
    expected = plan['workers']
    expected_names = {f'required-work ({worker})' for worker in expected}
    actual_names = [job.get('name') for job in jobs if isinstance(job.get('name'), str) and job['name'].startswith('required-work')]
    if len(actual_names) != len(expected_names) or set(actual_names) != expected_names:
        raise ExecutionError('unknown, unselected, missing or duplicate actual worker job/check; Re-run all jobs')
    if len(results) != len(expected) or {r.get('worker') for r in results} != set(expected):
        raise ExecutionError('missing, unknown or duplicate workers; Re-run all jobs')
    seen = []
    for result in results:
        worker = result['worker']
        if set(result) != {'schema', 'worker', 'identity', 'plan_digest', 'command_definition_digest', 'completed'} or result['schema'] != 'oasis7-required-worker-result/v1':
            raise ExecutionError('malformed worker result')
        if any(result[k] != plan[k] for k in ('identity', 'plan_digest', 'command_definition_digest')) or result['completed'] != expected[worker]:
            raise ExecutionError('worker identity or exact ordered command coverage mismatch')
        matches = [j for j in jobs if j.get('name') == f'required-work ({worker})']
        if len(matches) != 1:
            raise ExecutionError('worker job/check is absent or duplicated')
        job = matches[0]
        if str(job.get('run_id')) != str(identity['run_id']) or str(job.get('run_attempt')) != str(identity['run_attempt']) or job.get('status') != 'completed' or job.get('conclusion') != 'success' or job.get('head_sha') != identity['run_head_sha']:
            raise ExecutionError('worker job/check is not exact same-attempt completed/success')
        seen.extend(result['completed'])
    if Counter(seen) != Counter(c for commands in expected.values() for c in commands):
        raise ExecutionError('command multiset mismatch')
    return True

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    group = sub.add_parser('run-group'); group.add_argument('--group', required=True); group.add_argument('--root', default='.')
    plan = sub.add_parser('plan'); plan.add_argument('--selection', required=True); plan.add_argument('--identity', required=True); plan.add_argument('--output', required=True); plan.add_argument('--layout', choices=LAYOUTS, default=LAYOUTS[0])
    worker = sub.add_parser('run-worker'); worker.add_argument('--plan', required=True); worker.add_argument('--worker', required=True); worker.add_argument('--root', required=True); worker.add_argument('--result', required=True)
    gate = sub.add_parser('verify'); gate.add_argument('--plan', required=True); gate.add_argument('--selection', required=True); gate.add_argument('--identity', required=True); gate.add_argument('--results', required=True); gate.add_argument('--jobs', required=True); gate.add_argument('--root')
    args = parser.parse_args()
    def read(path): return json.loads(Path(path).read_text())
    def write(path, value): Path(path).write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')
    try:
        if args.action == 'run-group': execute_group(args.group, args.root)
        elif args.action == 'plan': write(args.output, make_plan(read(args.selection), read(args.identity), args.layout))
        elif args.action == 'run-worker': write(args.result, run_worker(read(args.plan), args.worker, args.root))
        else:
            if args.root: checkout_identity(args.root, read(args.identity))
            verify(read(args.plan), read(args.selection), read(args.identity), [read(p) for p in Path(args.results).glob('*.json')], read(args.jobs))
    except (ExecutionError, KeyError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.exit(1, f'required execution: {error}\n')

# One executable definition; argv, env wrappers and original order are retained.
COMMANDS = [
  {
    "id": "governance:testing-manual-active-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/testing-manual-active-contract.test.sh"
    ]
  },
  {
    "id": "governance:ci-tests-argument-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/ci-tests-argument-contract.test.sh"
    ]
  },
  {
    "id": "governance:ci-tests-full-superset-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/ci-tests-full-superset-contract.test.sh"
    ]
  },
  {
    "id": "governance:rust-required-gate-apt-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/rust-required-gate-apt-contract.test.sh"
    ]
  },
  {
    "id": "governance:plan-rust-required-scope-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "./scripts/plan-rust-required-scope.test.sh"
    ]
  },
  {
    "id": "governance:pm-workflow-impact-projection-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-impact-projection.test.py"
    ]
  },
  {
    "id": "governance:pm-workflow-impact-consumers-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-impact-consumers.test.py"
    ]
  },
  {
    "id": "governance:pm-check-cargo-package-scope-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/check-cargo-package-scope.test.py"
    ]
  },
  {
    "id": "governance:pm-trusted-cargo-scope-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/trusted-cargo-scope.test.py"
    ]
  },
  {
    "id": "governance:pm-prepare-loop-ci-authority-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/prepare-loop-ci-authority.test.py"
    ]
  },
  {
    "id": "governance:pm-required-scope-routing-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/required-scope-routing.test.py"
    ]
  },
  {
    "id": "governance:pm-task-primary-package-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/task-primary-package.test.py"
    ]
  },
  {
    "id": "governance:pm-task-primary-package-consumers-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/task-primary-package-consumers.test.py"
    ]
  },
  {
    "id": "governance:pm-cargo-checker-route-retirement-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/cargo-checker-route-retirement.test.py"
    ]
  },
  {
    "id": "governance:pm-cargo-package-profile-planner-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/cargo-package-profile-planner.test.py"
    ]
  },
  {
    "id": "governance:pm-cargo-package-profile-driver-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/cargo-package-profile-driver.test.py"
    ]
  },
  {
    "id": "governance:workflow-process-identity-check-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/workflow-process-identity-check.test.py"
    ]
  },
  {
    "id": "governance:rust-required-gate-compile-command-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "./scripts/rust-required-gate-compile-command-contract.test.sh"
    ]
  },
  {
    "id": "governance:rust-full-tier-trunk-prerequisite-contract-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/rust-full-tier-trunk-prerequisite-contract.test.sh"
    ]
  },
  {
    "id": "governance:ci-required-baseline-routing-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/ci-required-baseline-routing.test.sh"
    ]
  },
  {
    "id": "governance:ci-required-domain-isolation-test-sh",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "bash",
      "./scripts/ci-required-domain-isolation.test.sh"
    ]
  },
  {
    "id": "governance:pm-ci-required-inventory-test-py",
    "group": "run_workflow_governance_baseline_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci-required-inventory.test.py"
    ]
  },
  {
    "id": "governance:document-corpus-inventory-workflow-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/document-corpus-inventory-workflow.test.py"
    ]
  },
  {
    "id": "governance:security-codeql-plan-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/security/codeql-plan.test.py"
    ]
  },
  {
    "id": "governance:security-codeql-workflow-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/security/codeql-workflow.test.py"
    ]
  },
  {
    "id": "governance:security-codeql-health-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/security/codeql-health.test.py"
    ]
  },
  {
    "id": "governance:security-codeql-upload-association-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/security/codeql-upload-association.test.py"
    ]
  },
  {
    "id": "governance:security-codeql-acceptance-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/security/codeql-acceptance.test.py"
    ]
  },
  {
    "id": "governance:pm-codeql-advisory-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/codeql-advisory.test.py"
    ]
  },
  {
    "id": "governance:pm-github-api-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-api.test.py"
    ]
  },
  {
    "id": "governance:pm-github-api-concurrency-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-api-concurrency.test.py"
    ]
  },
  {
    "id": "governance:pm-github-project-api-budget-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-project-api-budget.test.py"
    ]
  },
  {
    "id": "governance:pm-github-observation-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-observation.test.py"
    ]
  },
  {
    "id": "governance:pm-github-pr-snapshot-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github_pr_snapshot.test.py"
    ]
  },
  {
    "id": "governance:pm-portable-file-lock-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/portable-file-lock.test.py"
    ]
  },
  {
    "id": "governance:pm-graphql-budget-red-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/graphql-budget-red.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-graphql-call-budget-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/pr-graphql-call-budget.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-lifecycle-gate-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/pr-lifecycle-gate.test.sh"
    ]
  },
  {
    "id": "governance:pm-github-project-task-lifecycle-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-project-task-lifecycle.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-lifecycle-trust-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/pr-lifecycle-trust.test.sh"
    ]
  },
  {
    "id": "governance:pm-pr-watch-loop-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/pr-watch-loop.test.sh"
    ]
  },
  {
    "id": "governance:pr-review-thread-closeout-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pr-review-thread-closeout.test.sh"
    ]
  },
  {
    "id": "governance:pm-pr-projection-publication-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/pr_projection_publication.test.py"
    ]
  },
  {
    "id": "governance:pm-lint-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/lint.test.sh"
    ]
  },
  {
    "id": "governance:pm-github-project-workflow-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/github-project-workflow.test.sh"
    ]
  },
  {
    "id": "governance:ci-required-scope-audit-contract-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "./scripts/ci-required-scope-audit-contract.test.sh"
    ]
  },
  {
    "id": "governance:pm-workflow-process-exception-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-process-exception.test.py"
    ]
  },
  {
    "id": "governance:pm-workflow-next-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-next.test.py"
    ]
  },
  {
    "id": "governance:pm-workflow-delivery-readiness-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-delivery-readiness.test.py"
    ]
  },
  {
    "id": "governance:pm-aggregate-task-completion-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/aggregate-task-completion.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-delivery-protocol-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal-delivery-protocol.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-readiness-recovery-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal-readiness-recovery.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-recovery-guards-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal-recovery-guards.test.py"
    ]
  },
  {
    "id": "governance:pm-resource-cleanup-safety-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/resource-cleanup-safety.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-proof-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal_proof.test.py"
    ]
  },
  {
    "id": "governance:pm-post-merge-finalize-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-finalize.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-finalizer-ledger-red-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-finalizer-ledger-red.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-finalizer-comment-readback-red-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-finalizer-comment-readback-red.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-finalizer-project-ledger-red-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-finalizer-project-ledger-red.test.sh"
    ]
  },
  {
    "id": "governance:pm-finalize-task-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/finalize-task.test.sh"
    ]
  },
  {
    "id": "governance:pm-finalize-task-red-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/finalize-task-red.test.sh"
    ]
  },
  {
    "id": "governance:pm-finalize-task-remote-branch-mismatch-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/finalize-task-remote-branch-mismatch.test.sh"
    ]
  },
  {
    "id": "governance:pm-recover-terminal-task-mapping-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/recover-terminal-task-mapping.test.py"
    ]
  },
  {
    "id": "governance:pm-readiness-transport-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/readiness-transport.test.py"
    ]
  },
  {
    "id": "governance:pm-readiness-repeat-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/readiness-repeat.test.py"
    ]
  },
  {
    "id": "governance:pm-readiness-prior-receipt-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/readiness-prior-receipt.test.py"
    ]
  },
  {
    "id": "governance:pm-readiness-legacy-repeat-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/readiness-legacy-repeat.test.py"
    ]
  },
  {
    "id": "governance:pm-post-merge-cleanup-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-cleanup.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-cleanup-trust-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-cleanup-trust.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-cleanup-fault-isolation-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-cleanup-fault-isolation.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-cleanup-crash-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-cleanup-crash.test.sh"
    ]
  },
  {
    "id": "governance:pm-post-merge-cleanup-resume-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/post-merge-cleanup-resume.test.sh"
    ]
  },
  {
    "id": "governance:pm-ordered-aggregate-closeout-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ordered-aggregate-closeout.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-task-audit-aggregate-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal-task-audit-aggregate.test.py"
    ]
  },
  {
    "id": "governance:pm-terminal-task-audit-project-semantics-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/terminal-task-audit-project-semantics.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-ready-receipt-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci-ready-receipt.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-validation-readback-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci_reuse_validation_readback.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-validation-readback-pagination-adversarial-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci_reuse_validation_readback_pagination_adversarial.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-validation-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci-reuse-validation.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-validation-contract-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci_reuse_validation_contract.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-validation-unicode-history-adversarial-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci_reuse_validation_unicode_history_adversarial.test.py"
    ]
  },
  {
    "id": "governance:pm-ci-reuse-acceptance-qa-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/ci_reuse_acceptance_qa.test.py"
    ]
  },
  {
    "id": "governance:pm-review-plan-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/review-plan.test.py"
    ]
  },
  {
    "id": "governance:pm-subagent-task-packet-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/subagent-task-packet.test.py"
    ]
  },
  {
    "id": "governance:pm-bootstrap-task-snapshot-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/bootstrap-task-snapshot.test.py"
    ]
  },
  {
    "id": "governance:pm-integration-ci-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/integration-ci.test.py"
    ]
  },
  {
    "id": "governance:pm-integration-selection-regression-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/integration-selection-regression.test.py"
    ]
  },
  {
    "id": "governance:pm-workflow-bootstrap-fallback-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-bootstrap-fallback.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-policy-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-policy.test.py"
    ]
  },
  {
    "id": "governance:pm-github-project-task-policy-adoption-integration-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/github-project-task-policy-adoption.integration.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-projection-publication-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/pr_projection_publication.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-projection-record-pr-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/pr-projection-record-pr.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-projection-transition-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/pr-projection-transition.test.py"
    ]
  },
  {
    "id": "governance:pm-review-closeout-publication-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/review_closeout_publication.test.py"
    ]
  },
  {
    "id": "governance:pm-review-closeout-facade-test-sh",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "bash",
      "./scripts/pm/review-closeout-facade.test.sh"
    ]
  },
  {
    "id": "governance:pm-pr-projection-publish-cli-integration-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/pr-projection-publish-cli.integration.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-projection-publish-concurrency-integration-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/pr-projection-publish-concurrency.integration.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-contracts-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-contracts.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-traceability-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-traceability.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-terminal-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop_terminal.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-gate-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-gate.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-ci-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-ci.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-ci-content-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-ci-content.test.py"
    ]
  },
  {
    "id": "governance:pm-pr-lifecycle-loop-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/pr-lifecycle-loop.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-ingress-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-ingress.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-publication-integration-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/loop-publication.integration.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-recovery-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-recovery.test.py"
    ]
  },
  {
    "id": "governance:pm-github-project-loop-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-project-loop.test.py"
    ]
  },
  {
    "id": "governance:pm-github-project-admission-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/github-project-admission.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-bootstrap-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/loop-bootstrap.test.py"
    ]
  },
  {
    "id": "governance:pm-loop-bootstrap-integration-test-py:env",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "env",
      "PYTHONDONTWRITEBYTECODE=1",
      "python3",
      "./scripts/pm/loop-bootstrap.integration.test.py"
    ]
  },
  {
    "id": "governance:pm-workflow-simplification-test-py",
    "group": "run_workflow_governance_operational_contract_tests",
    "argv": [
      "python3",
      "./scripts/pm/workflow-simplification.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-full-network-clean-room-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-full-network-clean-room.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-full-network-clean-room-adapter-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-full-network-clean-room-adapter.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-identity-v2-signing-tool-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-identity-v2-signing-tool.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-identity-v2-cli-bridge-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-identity-v2-cli-bridge.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-identity-v2-evidence-aggregate-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-identity-v2-evidence-aggregate.test.py"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-peer-registry-test-py",
    "group": "run_operational_identity_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-public-testnet-peer-registry.test.py"
    ]
  },
  {
    "id": "operational:game-world-state-sync-commit-module-required-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/game-world-state-sync-commit-module-required.test.sh"
    ]
  },
  {
    "id": "operational:state-sync-closure-evidence-template-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/state-sync-closure-evidence-template.test.sh"
    ]
  },
  {
    "id": "operational:s10-five-node-game-soak-summary-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/s10-five-node-game-soak-summary.test.sh"
    ]
  },
  {
    "id": "operational:release-gate-bash-preflight-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/release-gate-bash-preflight.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-local-observer-sync-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-local-observer-sync.test.sh"
    ]
  },
  {
    "id": "operational:build-game-launcher-bundle-ops-default-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/build-game-launcher-bundle-ops-default.test.sh"
    ]
  },
  {
    "id": "operational:build-game-launcher-bundle-macos-bash3-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/build-game-launcher-bundle-macos-bash3.test.sh"
    ]
  },
  {
    "id": "operational:testnet-packages-linux-bundle-bootstrap-contract-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/testnet-packages-linux-bundle-bootstrap-contract.test.sh"
    ]
  },
  {
    "id": "operational:testnet-packages-windows-governed-closure-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/testnet-packages-windows-governed-closure.test.sh"
    ]
  },
  {
    "id": "operational:run-local-letai-game-test-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/run-local-letai-game-test.test.sh"
    ]
  },
  {
    "id": "operational:provider-remote-https-letai-provider-cli-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/provider-remote-https/letai-provider-cli.test.sh"
    ]
  },
  {
    "id": "operational:provider-remote-https-provider-bridge-contract-smoke-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "./scripts/provider-remote-https/provider-bridge-contract-smoke.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-bootstrap-fresh-validator-host-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-service-readback-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-service-readback.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-package-node-upgrade-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-package-node-upgrade.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-package-node-upgrade-health-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-package-node-upgrade-health.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-package-node-upgrade-order-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-package-node-upgrade-order.test.sh"
    ]
  },
  {
    "id": "operational:p2p-public-testnet-package-node-upgrade-rollback-contract-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-public-testnet-package-node-upgrade-rollback-contract.test.sh"
    ]
  },
  {
    "id": "operational:p2p-observer-checkpoint-closure-probe-test-sh",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "bash",
      "./scripts/p2p-observer-checkpoint-closure-probe.test.sh"
    ]
  },
  {
    "id": "operational:p2p-observer-checkpoint-closure-probe-safety-test-py",
    "group": "run_operational_node_contract_tests",
    "argv": [
      "python3",
      "./scripts/p2p-observer-checkpoint-closure-probe-safety.test.py"
    ]
  }
]

COMMANDS.append({'id': 'governance:ci-required-execution', 'group': 'run_workflow_governance_baseline_contract_tests', 'argv': ['python3', './scripts/pm/ci-required-execution.test.py']})

if __name__ == '__main__':
    main()
