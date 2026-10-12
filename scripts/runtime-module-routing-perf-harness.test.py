#!/usr/bin/env python3
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

class Tests(unittest.TestCase):
    def test_dev_wrapper_release_raw_and_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            scripts = root / "scripts"
            scripts.mkdir()
            source = Path(__file__).with_name("runtime-module-routing-perf-harness.sh")
            shutil.copy(source, scripts / source.name)
            (scripts / "cargo-dev-lib.sh").write_text('oasis7_cargo_dev() { echo "wrapper called"; cargo "$@"; }\n')
            binary = root / "bin"
            binary.mkdir()
            cargo = binary / "cargo"
            cargo.write_text('#!/bin/sh\necho "args=$*"\n[ "${FAIL:-0}" = 1 ] && exit 7\necho "perf_probe_runtime_module_routing_with_many_active_manifests: modules=1 iterations=1 event_total_ms=1 event_avg_ms=1 action_total_ms=1 action_avg_ms=1 event_invoked=1 action_invoked=1"\n')
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ["PATH"])
            for mode in ("dev", "release"):
                command = ["bash", str(scripts / source.name), "--profile", mode, "--out-dir", str(root / mode)]
                result = subprocess.run(command, env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual("wrapper called" in result.stdout, mode == "dev")
                self.assertEqual("--release" in result.stdout, mode == "release")
                failed = subprocess.run(command, env=dict(env, FAIL="1"), capture_output=True)
                self.assertEqual(failed.returncode, 7)

if __name__ == "__main__":
    unittest.main()
