#!/usr/bin/env python3
"""Execute the real profile dispatcher with recording Cargo functions."""
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DRIVER = (ROOT / 'scripts/ci-tests.sh').read_text()
FUNCTION = DRIVER.split('run_selected_workspace_support() {', 1)[1].split(
    '\nrun_rustsec_advisory_check()', 1)[0]


class Profiles(unittest.TestCase):
    def invoke(self, profiles, fail=False):
        script = '''set -euo pipefail
ci_python="$1"; driver_dir="$2"; support_profiles="$3"
run_cargo() { echo "cargo $*"; }
run_cargo_clippy() { echo "clippy $*"; }
run_oasis7_workspace_support_crate_tests() { echo full-inventory; }
'''
        if fail:
            script += 'run_cargo() { return 17; }\n'
        script += 'run_selected_workspace_support() {' + FUNCTION
        script += '\nrun_selected_workspace_support\n'
        return subprocess.run(['bash', '-c', script, 'test', sys.executable,
                               str(ROOT / 'scripts'), profiles],
                              text=True, capture_output=True)

    def test_harness_isolated_and_strict(self):
        result = self.invoke(json.dumps(['oasis7_agent_harness']))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('clippy -p oasis7_agent_harness --all-targets', result.stdout)
        self.assertIn('cargo test -p oasis7_agent_harness --lib', result.stdout)
        self.assertNotIn('oasis7_local_signer', result.stdout)
        self.assertNotIn('full-inventory', result.stdout)

    def test_invalid_lists_fail_before_execution(self):
        for value in ['[]', '["unknown"]', '["oasis7_agent_api", "oasis7_agent_api"]',
                      'null', '{}', '[1]', 'invalid']:
            with self.subTest(value=value):
                result = self.invoke(value)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('cargo test', result.stdout)

    def test_failure_is_not_completed(self):
        result = self.invoke('["oasis7_agent_api"]', fail=True)
        self.assertEqual(result.returncode, 17)
        self.assertNotIn('support profile completed', result.stdout)

    def test_legacy_full_default_preserved(self):
        result = self.invoke('')
        self.assertEqual(result.returncode, 0)
        self.assertIn('full-inventory', result.stdout)

    def test_plan_omissions_and_unknown_versions_rejected(self):
        for plan in [dict(groups=['workspace_support'], scope='full',
                          workspace_support_profile_version=1,
                          workspace_support_profiles=['oasis7_agent_api']),
                     dict(groups=['workspace_support'], scope='targeted',
                          workspace_support_profile_version=1),
                     dict(groups=['workspace_support'], workspace_support_profile_version=2)]:
            result = self.invoke('plan:' + json.dumps(plan))
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertNotIn('cargo test', result.stdout)

    def test_full_inventory_preserves_packages_and_special_commands(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location('scope', ROOT / 'scripts/plan-rust-required-scope.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        legacy = DRIVER.split('run_oasis7_workspace_support_crate_tests() {', 1)[1].split('\n}', 1)[0]
        # Independent old full entry is retained: profiles must cover every package
        # in it, not merely agree with their own inventory constant.
        self.assertEqual(set(module.SUPPORT_PROFILES),
                         set(re.findall(r'-p\s+([a-zA-Z0-9_]+)', legacy)))
        plan = dict(groups=['workspace_support'], scope='full',
                    workspace_support_profile_version=1,
                    workspace_support_profiles=list(module.SUPPORT_PROFILES))
        result = self.invoke('plan:' + json.dumps(plan))
        self.assertEqual(result.returncode, 0, result.stderr)
        for profile in module.SUPPORT_PROFILES:
            self.assertIn('+ support profile completed: ' + profile, result.stdout)
        for command in ['oasis7_wasm_executor --features wasmtime --lib',
                        'oasis7_context --all-targets --locked',
                        'oasis7_secure_envelope --all-targets --locked',
                        'oasis7_local_signer --test context_cli --locked',
                        'oasis7_client_launcher --bin oasis7_client_launcher']:
            self.assertIn(command, result.stdout)

    def test_old_trusted_plan_cannot_narrow_support(self):
        result = self.invoke('plan:' + json.dumps(dict(groups=['workspace_support'], scope='targeted')))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('support profile completed: wasm_module_observe', result.stdout)


if __name__ == '__main__':
    unittest.main()
