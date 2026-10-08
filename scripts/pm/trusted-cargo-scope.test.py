#!/usr/bin/env python3
"""Trusted checker endpoint tests using real Git and Cargo metadata."""
from pathlib import Path
import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("trusted_scope", Path(__file__).with_name("trusted_cargo_scope.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
ROOT = Path(__file__).resolve().parents[2]

CHECKER = '''import argparse,json,subprocess
p=argparse.ArgumentParser()
for name in ("repo-root","base","head","primary-package","policy"): p.add_argument("--"+name)
p.add_argument("--json",action="store_true")
p.add_argument("--trusted-full-plan")
p.add_argument("--trusted-planner-base")
a=p.parse_args()
try: metadata=json.loads(subprocess.check_output(["cargo","metadata","--no-deps","--format-version","1"],cwd=a.repo_root,text=True))
except FileNotFoundError: raise SystemExit("metadata resource missing: pinned Cargo toolchain")
print(json.dumps({"schema":"oasis7-cargo-package-scope/v1","primary_package":metadata["packages"][0]["name"],"authority":"SOURCE"}))
'''


class TrustedScopeTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="trusted-scope-test-")
        self.root = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.com")
        self.write("Cargo.toml", '[package]\nname="fixture"\nversion="0.1.0"\nedition="2021"\n')
        self.write("src/lib.rs", "pub fn value() {}\n")
        self.write(".pm/cargo-package-scope-policy.json", "{}\n")
        self.write("scripts/pm/check-cargo-package-scope", CHECKER)
        self.source = self.commit("source")
        self.git("branch", "target")

    def tearDown(self):
        self.temporary.cleanup()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def commit(self, label):
        self.git("add", ".")
        self.git("commit", "-qm", label)
        return self.git("rev-parse", "HEAD")

    def test_unique_s_authority_when_b_differs_and_candidate_is_hostile(self):
        self.write("scripts/pm/check-cargo-package-scope", "raise SystemExit('candidate must not run')\n")
        self.write("doc/example.md", "# ordinary document\n")
        head = self.commit("candidate")
        self.git("checkout", "-q", "target")
        self.write("scripts/pm/check-cargo-package-scope", "raise SystemExit('B checker must not run')\n")
        base = self.commit("target advances")
        result = MODULE.run_scope(self.root, base, head, json_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["authority"], "SOURCE")
        self.assertEqual(json.loads(result.stdout)["primary_package"], "fixture")

    def test_candidate_cannot_fill_missing_source_dependency(self):
        missing = '''from pathlib import Path
import importlib.util
p=Path(__file__).with_name("cargo_package_change_classification.py")
s=importlib.util.spec_from_file_location("classifier",p)
s.loader.exec_module(importlib.util.module_from_spec(s))
'''
        self.write("scripts/pm/check-cargo-package-scope", missing)
        source = self.commit("missing trusted module")
        self.write("scripts/pm/cargo_package_change_classification.py", "print('candidate forged success')\n")
        head = self.commit("candidate module")
        result = MODULE.run_scope(self.root, source, head, json_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("candidate forged success", result.stdout)

    def test_missing_metadata_resource_is_visible(self):
        self.write("doc/example.md", "# document\n")
        head = self.commit("document")
        with tempfile.TemporaryDirectory() as directory:
            bindir = Path(directory)
            (bindir / "git").symlink_to(shutil.which("git"))
            old = os.environ["PATH"]
            try:
                os.environ["PATH"] = str(bindir)
                result = MODULE.run_scope(self.root, self.source, head, json_output=True)
            finally:
                os.environ["PATH"] = old
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("metadata resource missing", result.stderr)

    def test_real_checker_accepts_minimal_cargo_package_and_ordinary_document(self):
        for filename in ("scripts/pm/check-cargo-package-scope",
                         "scripts/pm/cargo_package_change_classification.py",
                         "scripts/document_corpus.py", ".pm/cargo-package-scope-policy.json"):
            self.write(filename, (ROOT / filename).read_text())
        self.write(".pm/cargo-package-auxiliary-files.json",
                   '{"schema":"oasis7-cargo-package-auxiliary-files/v1","auxiliary_files":[]}\n')
        base = self.commit("real trusted checker")
        self.write("src/lib.rs", "pub fn value() -> u32 { 7 }\n")
        self.write("README.md", "# Fixture\nOrdinary package documentation.\n")
        head = self.commit("package and documentation")
        result = MODULE.run_scope(self.root, base, head, json_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["primary_package"], "fixture")

    def full_fixture(self):
        for relative in ("scripts/plan-rust-required-scope.py", "scripts/ci-required-scope.v2.json", "scripts/ci-tests.sh"):
            self.write(relative, (ROOT / relative).read_text())
        self.write(".pm/cargo-package-auxiliary-files.json", "{}\n")
        return self.commit("trusted full planner")

    def full_plan(self, base, head):
        paths = self.git("diff", "--name-only", base, head).splitlines()
        with tempfile.TemporaryDirectory() as directory:
            authority = Path(directory)
            for relative in ("scripts/plan-rust-required-scope.py", "scripts/ci-required-scope.v2.json", "scripts/ci-tests.sh"):
                target = authority / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(self.git("show", base + ":" + relative) + "\n")
            command = [os.sys.executable, "-I", str(authority / "scripts/plan-rust-required-scope.py"),
                       "--event-name", "pull_request", "--run-mode", "full_escalation"]
            for path in paths:
                command.extend(("--changed-path", path))
            output = subprocess.check_output(command, cwd=self.root, text=True)
        plan = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
        plan.update(integration_base=base, source_head=head, source_scope_base=base)
        path = self.root / "full-plan.json"
        path.write_text(json.dumps(plan))
        environment = {"OASIS7_CI_" + key.upper(): value for key, value in plan.items()
                       if key.startswith(("run_", "needs_")) or key == "execution_contract"}
        environment["OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS"] = environment.pop("OASIS7_CI_RUN_OASIS7_WORKSPACE_SUPPORT_CRATE_TESTS")
        return path, environment

    def test_exact_maintenance_selects_isolated_candidate_only_with_verified_full_runner(self):
        base = self.full_fixture()
        self.write("scripts/pm/check-cargo-package-scope", CHECKER.replace('"SOURCE"', '"CANDIDATE"'))
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        head = self.commit("bounded maintenance")
        path, environment = self.full_plan(base, head)
        with patch.dict(os.environ, environment):
            result = MODULE.run_scope(self.root, base, head, json_output=True, trusted_full_plan=str(path))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["authority"], "CANDIDATE")

    def test_outside_path_cannot_select_candidate_even_with_complete_full_plan(self):
        base = self.full_fixture()
        self.write("scripts/pm/check-cargo-package-scope", "raise SystemExit('candidate must not run')\n")
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        self.write("src/lib.rs", "pub fn changed() {}\n")
        head = self.commit("crossing boundary")
        path, environment = self.full_plan(base, head)
        with patch.dict(os.environ, environment):
            result = MODULE.run_scope(self.root, base, head, json_output=True, trusted_full_plan=str(path))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["authority"], "SOURCE")

    def test_full_plan_rejects_incomplete_actual_runner(self):
        base = self.full_fixture()
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        head = self.commit("maintenance")
        path, environment = self.full_plan(base, head)
        environment["OASIS7_CI_NEEDS_NODE"] = "false"
        with patch.dict(os.environ, environment), self.assertRaisesRegex(ValueError, "actual runner"):
            MODULE.run_scope(self.root, base, head, trusted_full_plan=str(path))

    def test_boolean_marker_cannot_select_candidate(self):
        base = self.full_fixture()
        self.write("scripts/pm/check-cargo-package-scope", "raise SystemExit('candidate must not run')\n")
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        head = self.commit("maintenance without plan")
        with patch.dict(os.environ, {"OASIS7_CARGO_SCOPE_TRUSTED_FULL_PLAN": "true"}):
            result = MODULE.run_scope(self.root, base, head, json_output=True)
        self.assertEqual(json.loads(result.stdout)["authority"], "SOURCE")

    def test_historical_checker_receives_verified_marker_without_new_cli(self):
        base = self.full_fixture()
        historical = CHECKER.replace('p.add_argument("--trusted-full-plan")\n', '').replace('p.add_argument("--trusted-planner-base")\n', '')
        self.write("scripts/pm/check-cargo-package-scope", historical)
        base = self.commit("historical checker CLI")
        self.write("Cargo.lock", '# lock-only execution fixture\n')
        head = self.commit("full path")
        path, environment = self.full_plan(base, head)
        with patch.dict(os.environ, environment):
            result = MODULE.run_scope(self.root, base, head, json_output=True, trusted_full_plan=str(path))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["authority"], "SOURCE")

    def test_full_plan_rejects_forged_identity_and_scope(self):
        base = self.full_fixture()
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        head = self.commit("maintenance")
        path, environment = self.full_plan(base, head)
        original = json.loads(path.read_text())
        for key, value in (("scope", "targeted"), ("source_head", base),
                           ("planner_config_sha256", "sha256:" + "0" * 64),
                           ("run_rust_baseline", "false")):
            with self.subTest(key=key):
                path.write_text(json.dumps({**original, key: value}))
                with patch.dict(os.environ, environment), self.assertRaises(ValueError):
                    MODULE.run_scope(self.root, base, head, trusted_full_plan=str(path))

    def test_target_only_integration_paths_do_not_enable_candidate_maintenance(self):
        source = self.full_fixture()
        self.git("branch", "integration-target")
        self.write("scripts/pm/check-cargo-package-scope", "raise SystemExit('candidate must not run')\n")
        self.write(".pm/cargo-package-auxiliary-files.json", '{"changed":true}\n')
        head = self.commit("source maintenance")
        self.git("checkout", "-q", "integration-target")
        self.write("site.txt", "target only execution change\n")
        base = self.commit("target advancement")
        path, environment = self.full_plan(base, head)
        plan = json.loads(path.read_text())
        plan["source_scope_base"] = source
        path.write_text(json.dumps(plan))
        with patch.dict(os.environ, environment):
            result = MODULE.run_scope(self.root, base, head, json_output=True, trusted_full_plan=str(path))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["authority"], "SOURCE")


if __name__ == "__main__":
    unittest.main()
