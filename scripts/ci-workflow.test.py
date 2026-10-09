#!/usr/bin/env python3
"""Regress the final DAG and execution boundaries, without Actions credentials."""
import json
import os
import subprocess
import tempfile
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = (ROOT / '.github/workflows/rust.yml').read_text()
CONFIG = json.loads((ROOT / 'scripts/ci-required-scope.json').read_text())
JOBS = dict(re.findall(r'^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)',
                       WORKFLOW.split('\njobs:\n', 1)[1], re.M | re.S))


def validate_candidate_graph(workflow, config):
    jobs = dict(re.findall(r'^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)',
                           workflow.split('\njobs:\n', 1)[1], re.M | re.S))
    gate = jobs['required-gate']
    dependencies = re.search(r'    needs: \[(.*)\]', gate).group(1).split(', ')
    expected = {'select', *(g.replace('_', '-') for g in config['groups'])}
    if set(dependencies) != expected or len(dependencies) != len(expected):
        raise ValueError('gate dependencies must exactly cover candidate groups')
    if 'always()' not in gate or 'continue-on-error:' in workflow:
        raise ValueError('failure propagation must remain intact')
    fleet = jobs['fleet-health']
    if ('os: [ubuntu-24.04, windows-2022, macos-15]' not in fleet
            or 'runs-on: ${{ matrix.os }}' not in fleet
            or 'fail-fast: false' not in fleet or 'include:' in fleet or 'exclude:' in fleet
            or "if: runner.os != 'Windows'" not in fleet or '--host-smoke' not in fleet):
        raise ValueError('fleet must run all three hosts and POSIX cleanup smoke')
    for group in config['groups']:
        job = jobs[group.replace('_', '-')]
        if ("needs.select.result == 'success'" not in job
                or "fromJSON(needs.select.outputs.plan).scope == 'full'" not in job):
            raise ValueError('group requires successful select and full migration fallback')
        if group != 'fleet_health' and 'strategy:' in job:
            raise ValueError('ordinary groups must not have singleton matrices')
        if 'ci-authority' in job:
            raise ValueError('execution must use candidate checkout')
    return True


class Workflow(unittest.TestCase):
    def test_pinned_trunk_helper_failure_and_success_paths(self):
        # The protected baseline already executes this suite during migration.
        subprocess.run(['bash',str(ROOT/'scripts/install-ci-trunk.test.sh')],cwd=ROOT,check=True)

    def test_viewer_security_smoke_uses_installed_browser_after_formal_build(self):
        job = JOBS['viewer-js-required']
        self.assertIn('agent-browser@0.37.1', job)
        self.assertIn('--prefix "$RUNNER_TEMP/viewer-security-browser"', job)
        self.assertIn('"$browser_bin" install --with-deps', job)
        self.assertIn("printf 'AGENT_BROWSER_BIN=%s\\n'", job)
        self.assertNotIn('cache-mode: write', job)
        group = re.search(r'^    viewer_js_required\) (.*?) ;;$',
                          (ROOT / 'scripts/ci-tests.sh').read_text(), re.M).group(1)
        self.assertLess(group.index('run_oasis7_viewer_software_safe_build'),
                        group.index('viewer-auth-browser-security-smoke.mjs'))

    def test_additive_security_steps_preserve_candidate_groups(self):
        for name in ('net', 'viewer-js-required', 'viewer-performance-report', 'workflow-governance'):
            self.assertEqual(JOBS[name].count('name: Execute selected group'), 1)
            self.assertIn(f'bash ./scripts/ci-tests.sh required --group "{name.replace(chr(45), chr(95))}"', JOBS[name])
        performance = JOBS['viewer-performance-report']
        self.assertIn('agent-browser@0.37.1', performance)
        self.assertIn('--prefix "$RUNNER_TEMP/viewer-performance-browser"', performance)
        self.assertIn('"$browser_bin" install --with-deps', performance)
        self.assertIn("printf 'AGENT_BROWSER_BIN=%s\\n'", performance)
        self.assertLess(performance.index('Install pinned performance browser'), performance.index('Execute selected group'))
        self.assertLess(performance.index('Build performance test artifact'), performance.index('Execute selected group'))
        self.assertIn('viewer_bindgen_bin="$(./scripts/ensure-wasm-bindgen-cli.sh --print-bin)"', performance)
        self.assertIn('WASM_BINDGEN_BIN="$viewer_bindgen_bin" npm', performance)
        viewer = JOBS['viewer-js-required']
        self.assertGreater(viewer.index('Verify browser authentication security'), viewer.index('Execute selected group'))
        net = JOBS['net']
        self.assertGreater(net.index('Verify pinned network source'), net.index('Execute selected group'))
        self.assertLess(net.index('Install WASM C compiler'), net.index('Execute selected group'))
        for check in ('scripts/libp2p-security-source.test.py', 'scripts/libp2p-compat.test.py',
                      'clang --print-targets | grep -w wasm32', 'CC_wasm32_unknown_unknown: clang',
                      'cargo check -p oasis7_net --no-default-features --target wasm32-unknown-unknown --locked',
                      'cargo check -p oasis7_node --features libp2p --target wasm32-unknown-unknown --locked'):
            self.assertIn(check, net)
        governance = JOBS['workflow-governance']
        self.assertGreater(governance.index('Verify new source archive'), governance.index('Execute selected group'))
        for test in ('package-source-plan.test.py', 'safe-git-archive.test.py', 'cache-permission-probe.test.cjs'):
            self.assertIn(test, governance)
        self.assertNotIn('run: python3 scripts/ci-workflow.test.py', governance) # already in candidate dispatcher

    def test_trusted_writer_only_runs_on_protected_main(self):
        writer = JOBS['full-regression']
        self.assertIn("if: github.ref == 'refs/heads/main' && (github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' && inputs.run_mode == 'full'))", writer)
        self.assertIn('cache-mode: write', writer)

    def test_readers_share_compatible_trusted_writer_identity(self):
        writer = JOBS['full-regression']
        self.assertIn('cache-mode: write', writer)
        self.assertIn('shared-key: ci-full-regression-trusted-v2', writer)
        self.assertIn('env-vars: CARGO CC CFLAGS CXX CMAKE RUST OASIS7_WASM', writer)
        for name, job in JOBS.items():
            if 'save-if: false' in job and 'Swatinem/rust-cache@' in job:
                self.assertIn('shared-key: ci-full-regression-trusted-v2', job, name)
                self.assertIn('env-vars: CARGO CC CFLAGS CXX CMAKE RUST OASIS7_WASM', job, name)
        self.assertNotIn('ordinary-required-v2', WORKFLOW)

    def test_download_caches_follow_actual_worksets(self):
        node_jobs={name for name,job in JOBS.items() if 'uses: actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38' in job}
        self.assertEqual(node_jobs,{'viewer-js-required','viewer-performance-report','launcher-web','full-regression'})
        for name in node_jobs:
            self.assertIn('cache: npm',JOBS[name])
            self.assertIn('cache-dependency-path: crates/oasis7_viewer/package-lock.json',JOBS[name])
            self.assertIn('npm ci --prefix crates/oasis7_viewer',JOBS[name])
        for name,job in JOBS.items():
            if 'Swatinem/rust-cache@' in job and name in {g.replace('_', '-') for g in CONFIG['groups']}:
                self.assertIn('shared-key: ci-full-regression-trusted-v2',job)
                self.assertIn('add-rust-environment-hash-key: true',job)
                self.assertIn('env-vars: CARGO CC CFLAGS CXX CMAKE RUST OASIS7_WASM',job)
                self.assertLess(job.index('rustup default'),job.index('uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2'))
        for name in ('launcher-web','full-regression'):
            job=JOBS[name]
            self.assertIn('uses: actions/cache@caa296126883cff596d87d8935842f9db880ef25',job)
            self.assertIn("hashFiles('scripts/install-ci-trunk.sh')",job)
            self.assertNotIn('cargo install trunk',job)
            self.assertNotIn('restore-keys:',job)
            self.assertLess(job.index('Cache pinned Trunk release archive'),job.index('bash scripts/install-ci-trunk.sh'))
            invocation='Execute selected group' if name=='launcher-web' else 'Run full test tier'
            self.assertLess(job.index('bash scripts/install-ci-trunk.sh'),job.index(invocation))

    def test_gate_is_last_and_all_groups_are_dependencies(self):
        gate = JOBS['required-gate']
        dependencies = re.search(r'    needs: \[(.*)\]', gate).group(1).split(', ')
        self.assertEqual(set(dependencies), {'select', *(g.replace('_', '-') for g in CONFIG['groups'])})
        self.assertIn('always()', gate)
        self.assertIn('ci-required-result.py', gate)
        for group in CONFIG['groups']:
            job = JOBS[group.replace('_', '-')]
            self.assertIn('    needs: select\n', job)
            self.assertIn(f"needs.select.outputs.run_{group} == 'true'", job)
            self.assertIn("needs.select.result == 'success'", job)
            self.assertIn("fromJSON(needs.select.outputs.plan).scope == 'full'", job)
            if group != 'fleet_health':
                self.assertNotIn('strategy:', job)
            self.assertNotIn('ci-authority', job)
        self.assertNotIn('needs: required-gate', WORKFLOW)
        self.assertNotIn('continue-on-error:', WORKFLOW)
        self.assertNotIn('paths-ignore:', WORKFLOW)

    def test_platform_product_jobs_still_gate(self):
        self.assertIn('runs-on: windows-2022', JOBS['windows-rollout'])
        self.assertIn('p2p-public-testnet-package-rollout.test.sh', JOBS['windows-rollout'])
        self.assertIn('testnet-packages-macos-arm64-contract.test.sh', JOBS['macos-package-contract'])
        self.assertIn('p2p-public-testnet-fleet-health.test.py', JOBS['fleet-health'])
        self.assertIn('os: [ubuntu-24.04, windows-2022, macos-15]', JOBS['fleet-health'])
        self.assertIn('--host-smoke', JOBS['fleet-health'])
        self.assertEqual(WORKFLOW.count('run: bash ./scripts/testnet-packages-macos-arm64-contract.test.sh'), 1)
        self.assertIn('./scripts/ci-tests.sh full', JOBS['full-regression'])
        self.assertIn('package-newapi-bridge-service.sh', JOBS['newapi-bridge-package'])
        report = JOBS['viewer-performance-report']
        self.assertIn('rustup default', report)
        self.assertIn('rustup target add wasm32-unknown-unknown', report)
        self.assertIn('path: output/ci/viewer-performance/**', report)
        driver = (ROOT / 'scripts/ci-tests.sh').read_text()
        self.assertIn('viewer-performance-report-only-contract.test.sh', driver)
        full = driver.split('run_full_required_superset() {', 1)[1].split('\n}', 1)[0]
        self.assertIn('testnet-packages-macos-arm64-contract.test.sh', full)

    def test_trusted_closure_and_safe_checkout(self):
        select = JOBS['select']
        self.assertIn('git show "${BASE_SHA}:scripts/${file}"', select)
        self.assertIn('--test-ref "$tested_sha"', select)
        self.assertIn('for file in plan-rust-required-scope.py ci-required-scope.json ci-required-result.py;', select)
        self.assertNotIn('cp scripts/', select)
        self.assertEqual(WORKFLOW.count('name: ci-authority'), 2)
        self.assertNotIn('matrix.group', WORKFLOW)
        self.assertEqual(WORKFLOW.count('uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803'), WORKFLOW.count('persist-credentials: false'))
        self.assertNotIn('secrets.', WORKFLOW)
        self.assertNotIn('GH_TOKEN:', WORKFLOW)
        self.assertNotIn('scripts/pm/', WORKFLOW)
        self.assertNotIn('task_uid', WORKFLOW)
        self.assertIn('permissions:\n  contents: read', WORKFLOW)

    def test_runner_discovers_supported_python_when_default_is_unsupported(self):
        with tempfile.TemporaryDirectory() as temporary:
            for name in ('python', 'python3'):
                path = Path(temporary) / name
                path.write_text('#!/usr/bin/env bash\nexit 91\n')
                path.chmod(0o755)
            environment = dict(os.environ, PATH=temporary + os.pathsep + os.environ['PATH'])
            result = subprocess.run(['bash', str(ROOT / 'scripts/ci-tests.sh'), 'required',
                                     '--group', 'codex_agent_config_validation'],
                                    cwd=ROOT, env=environment, text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('validated agent configurations:', result.stdout)

    def test_native_tool_suite_does_not_inherit_nightly_build_std(self):
        driver = (ROOT / 'scripts/ci-tests.sh').read_text()
        function = re.search(r'^run_oasis7_workspace_support_crate_tests\(\) \{\n.*?^\}', driver, re.M | re.S).group()
        script = """set -euo pipefail
run_cargo() { printf '%s|%s\\n' "$OASIS7_WASM_BUILD_STD" "$*"; }
""" + function + "\nrun_oasis7_workspace_support_crate_tests\nprintf 'after|%s\\n' \"$OASIS7_WASM_BUILD_STD\"\n"
        result = subprocess.run(['bash', '-c', script], text=True, capture_output=True,
                                env=dict(os.environ, OASIS7_WASM_BUILD_STD='1'))
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = result.stdout.splitlines()
        self.assertEqual(calls[-2], '0|test -p wasm_build_suite -p wasm_module_observe')
        self.assertTrue(all(call.startswith('1|') for call in calls[:-2]))
        self.assertEqual(calls[-1], 'after|1')

    def test_invalid_selected_group_cannot_silently_skip_command(self):
        driver = (ROOT / 'scripts/ci-tests.sh').read_text()
        self.assertIn('Unknown CI group:', driver)
        self.assertNotIn('disabled_by_scope_planner', driver)
        self.assertNotIn('required_gate_execution_contract', driver)
        self.assertNotIn('scripts/pm/', driver)

    def test_candidate_graph_rejects_missing_dependencies_platforms_and_failure_swallowing(self):
        self.assertTrue(validate_candidate_graph(WORKFLOW, CONFIG))
        mutations = [WORKFLOW.replace(', fleet-health]', ']'),
                     WORKFLOW.replace('windows-2022, macos-15]', 'windows-2022]'),
                     WORKFLOW.replace('    runs-on: ${{ matrix.os }}', '    runs-on: windows-2022'),
                     WORKFLOW.replace('      fail-fast: false', '      continue-on-error: true'),
                     WORKFLOW.replace(' --host-smoke', ''),
                     WORKFLOW.replace('    if: always()', '    if: success()'),
                     WORKFLOW.replace("fromJSON(needs.select.outputs.plan).scope == 'full'", 'false', 1)]
        for workflow in mutations:
            with self.subTest(workflow=workflow[-100:]), self.assertRaises(ValueError):
                validate_candidate_graph(workflow, CONFIG)
        new_config = dict(CONFIG, groups=[*CONFIG['groups'], 'new_group'])
        with self.assertRaises(ValueError):
            validate_candidate_graph(WORKFLOW, new_config)

    def test_candidate_driver_executes_atomic_rename_and_new_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            scripts = root / 'scripts'
            scripts.mkdir()
            for filename in ('ci-tests.sh', 'find-python-with-module.sh', 'viewer-dependency-preflight.sh'):
                (scripts / filename).write_bytes((ROOT / 'scripts' / filename).read_bytes())
            driver = scripts / 'ci-tests.sh'
            driver.write_text(driver.read_text().replace('validate-codex-agent-config.py', 'renamed-validator.py'))
            command = ['bash', str(driver), 'required', '--group', 'codex_agent_config_validation', '--repo-root', str(root)]
            entry = scripts / 'renamed-validator.py'
            entry.write_text('print("candidate entry executed")\n')
            passed = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(passed.returncode, 0, passed.stderr)
            self.assertIn('candidate entry executed', passed.stdout)
            entry.write_text('raise SystemExit(42)\n')
            failed = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(failed.returncode, 42, failed.stderr)
            self.assertFalse((scripts / 'validate-codex-agent-config.py').exists())

    def test_concurrency_is_per_pr_and_per_non_pr_run(self):
        self.assertIn("format('pr-{0}', github.event.pull_request.number)", WORKFLOW)
        self.assertIn("format('run-{0}', github.run_id)", WORKFLOW)
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", WORKFLOW)
        concurrency = WORKFLOW.split('concurrency:', 1)[1].split('permissions:', 1)[0]
        self.assertNotIn('github.sha', concurrency)
        self.assertIn('github.workflow', concurrency)

    def test_clippy_precedes_heavy_group_tests(self):
        driver = (ROOT / 'scripts/ci-tests.sh').read_text()
        for group, prefix in [('oasis7_required', 'required_tier'), ('consensus', 'consensus'),
                              ('distfs', 'distfs'), ('node', 'node'), ('net', 'net')]:
            case = re.search(r'^    ' + group + r'\).*?;;', driver, re.M).group()
            self.assertLess(case.index('run_oasis7_' + prefix + '_clippy'),
                            case.index('run_oasis7_' + prefix + '_tests'))


if __name__ == '__main__':
    unittest.main()
