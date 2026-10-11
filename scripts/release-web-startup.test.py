#!/usr/bin/env python3
"""Behavioral startup regressions; no Cargo or browser dependencies."""
import pathlib
import subprocess
import time
import tempfile
import json
import os
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

class StartupTests(unittest.TestCase):
    def test_launcher_exit_interrupts_primary_port_wait(self):
        source = (ROOT / "scripts/viewer-primary-web-entry-regression.sh").read_text()
        functions = source[source.index("wait_for_port() {"):source.index("normalize_eval_token() {")]
        library = ROOT / "scripts/release-web-fixture-lib.sh"
        script = f"source {library};\n" if library.exists() else ""
        script += functions + "\n( exit 37 ) & launcher_pid=$!; launcher_log=/dev/null; sleep .1; wait_for_port 127.0.0.1 1 5"
        started = time.monotonic()
        result = subprocess.run(["bash", "-c", script], capture_output=True, text=True, timeout=8)
        self.assertLess(time.monotonic() - started, 3, "dead launcher consumed readiness deadline")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("status=37", result.stderr)

    def test_fixture_early_exit_is_reported(self):
        library = ROOT / "scripts/release-web-fixture-lib.sh"
        result = subprocess.run(["bash", "-c", f"source {library}; (exit 23) & release_web_fixture_pid=$!; sleep .1; release_web_check_startup"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("fixture process exited: status=23", result.stderr)

    def test_configuration_rejects_unsafe_scope_and_permissions_without_key_leak(self):
        library = ROOT / "scripts/release-web-fixture-lib.sh"
        names = ('ENDPOINT', 'PUBLIC_KEY', 'WORLD_ID', 'GENESIS_DIGEST', 'SCOPE', 'READ_PRIVATE_KEY')
        data = {'OASIS7_WORLD_SERVICE_' + key: 'secret-test-value' for key in names}
        data['OASIS7_WORLD_SERVICE_ENDPOINT'] = 'http://127.0.0.1:1234'
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / 'config.json'
            for mode, scope in [(0o644, 'public'), (0o600, 'engineering')]:
                data['OASIS7_WORLD_SERVICE_SCOPE'] = scope
                path.write_text(json.dumps(data))
                path.chmod(mode)
                result = subprocess.run(['bash', '-c', f'source {library}; release_web_fixture_import "$1"', 'bash', str(path)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn('secret-test-value', result.stdout + result.stderr)
            data['OASIS7_WORLD_SERVICE_SCOPE'] = 'public'
            path.write_text(json.dumps(data))
            path.chmod(0o600)
            result = subprocess.run(['bash', '-c', f'source {library}; release_web_fixture_import "$1"; [[ "$OASIS7_WORLD_SERVICE_SCOPE" == public ]]', 'bash', str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, '')

    def test_owned_process_deadline_terminates_hung_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = pathlib.Path(directory) / 'pid'
            stop = pathlib.Path(directory) / 'stop'
            code = "import os,signal,time,pathlib; pathlib.Path(%r).write_text(str(os.getpid())); signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(60)" % str(pidfile)
            started = time.monotonic()
            result = subprocess.run(['python3', str(ROOT / 'scripts/release-web-fixture-process.py'), str(stop), '1', 'python3', '-c', code], capture_output=True, text=True, timeout=12)
            self.assertLess(time.monotonic() - started, 10)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('process deadline exceeded', result.stderr)
            with self.assertRaises(ProcessLookupError):
                os.kill(int(pidfile.read_text()), 0)

    def test_deadline_is_failure_even_when_child_stops_cleanly(self):
        with tempfile.TemporaryDirectory() as directory:
            stop = pathlib.Path(directory) / 'stop'
            code = "import pathlib,time; p=pathlib.Path(%r); exec('while not p.exists(): time.sleep(.05)')" % str(stop)
            result = subprocess.run(['python3', str(ROOT / 'scripts/release-web-fixture-process.py'), str(stop), '1', 'python3', '-c', code], capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 124)

    def test_fixture_rejects_external_url(self):
        result = subprocess.run(['bash', str(ROOT / 'scripts/viewer-software-safe-step-regression.sh'), '--release-web-fixture', '--url', 'http://127.0.0.1:1'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('cannot be used with --url', result.stderr)

    def test_fixture_rejects_external_chain_before_runtime(self):
        result = subprocess.run(['bash', str(ROOT / 'scripts/viewer-software-safe-step-regression.sh'), '--release-web-fixture', '--chain-status-bind', '127.0.0.1:1'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('incompatible with the isolated signed Web QA fixture', result.stderr)

    def test_primary_rejects_scenario_before_runtime(self):
        result = subprocess.run(['bash', str(ROOT / 'scripts/viewer-primary-web-entry-regression.sh'), '--scenario', 'llm_bootstrap'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('unknown option: --scenario', result.stderr)

if __name__ == "__main__":
    unittest.main()
